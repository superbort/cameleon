//! This module contains low level streaming implementation for `U3V` device.

use std::{
    convert::TryInto,
    sync::{Arc, Mutex, MutexGuard},
    time::Duration,
};

use async_std::task;
use cameleon_device::u3v::{self, protocol::stream as u3v_stream};
use futures::channel::oneshot;
use tracing::{error, info, warn};

use crate::{
    camera::PayloadStream,
    payload::{ImageInfo, Payload, PayloadSender, PayloadType},
    ControlResult, StreamError, StreamResult,
};

use super::control_handle::U3VDeviceControl;

/// This type is used to receive stream packets from the device.
pub struct StreamHandle {
    inner: Arc<Mutex<u3v::ReceiveChannel>>,
    params: StreamParams,
    cancellation_tx: Option<oneshot::Sender<()>>,
    completion_rx: Option<oneshot::Receiver<()>>,
}

macro_rules! unwrap_or_poisoned {
    ($res:expr) => {{
        let error = "panics has occurred in running loop";
        $res.map_err(|cause| {
            error!(error, ?cause);
            StreamError::Device(error.into())
        })
    }};
}

impl StreamHandle {
    /// Read leader of a stream packet.
    ///
    /// Buffer size must be equal or larger than [`StreamParams::leader_size`].
    pub fn read_leader<'a>(&self, buf: &'a mut [u8]) -> StreamResult<u3v_stream::Leader<'a>> {
        if self.is_loop_running() {
            Err(StreamError::InStreaming)
        } else {
            read_leader(
                &mut unwrap_or_poisoned!(self.inner.lock())?,
                &self.params,
                buf,
            )
        }
    }

    /// Read payload of a stream packet.
    pub fn read_payload(&self, buf: &mut [u8]) -> StreamResult<usize> {
        if self.is_loop_running() {
            Err(StreamError::InStreaming)
        } else {
            read_payload(
                &mut unwrap_or_poisoned!(self.inner.lock())?,
                &self.params,
                buf,
            )
        }
    }

    /// Read trailer of a stream packet.
    ///
    /// Buffer size must be equal of larger than [`StreamParams::trailer_size`].
    pub fn read_trailer<'a>(&self, buf: &'a mut [u8]) -> StreamResult<u3v_stream::Trailer<'a>> {
        if self.is_loop_running() {
            Err(StreamError::InStreaming)
        } else {
            read_trailer(
                &mut unwrap_or_poisoned!(self.inner.lock())?,
                &self.params,
                buf,
            )
        }
    }

    /// Return params.
    #[must_use]
    pub fn params(&self) -> &StreamParams {
        &self.params
    }

    ///  Return mutable params.
    pub fn params_mut(&mut self) -> &mut StreamParams {
        &mut self.params
    }

    pub(super) fn new(device: &u3v::Device) -> ControlResult<Option<Self>> {
        let inner = device.stream_channel()?;
        Ok(inner.map(|inner| Self {
            inner: Arc::new(Mutex::new(inner)),
            params: StreamParams::default(),
            cancellation_tx: None,
            completion_rx: None,
        }))
    }
}

impl PayloadStream for StreamHandle {
    type StrmParams = StreamParams;

    fn open(&mut self) -> StreamResult<()> {
        unwrap_or_poisoned!(self.inner.lock())?.open().map_err(|e| {
            error!(?e);
            StreamError::Device(format!("{}", e).into())
        })
    }

    fn close(&mut self) -> StreamResult<()> {
        if self.is_loop_running() {
            self.stop_streaming_loop()?;
        }
        unwrap_or_poisoned!(self.inner.lock())?
            .close()
            .map_err(|e| {
                error!(?e);
                StreamError::Device(format!("{}", e).into())
            })
    }

    fn run_streaming_loop(
        &mut self,
        sender: PayloadSender,
        params: Self::StrmParams,
    ) -> StreamResult<()> {
        self.params = params;
        if self.is_loop_running() {
            return Err(StreamError::InStreaming);
        }

        let (cancellation_tx, cancellation_rx) = oneshot::channel();
        let (completion_tx, completion_rx) = oneshot::channel();
        self.cancellation_tx = Some(cancellation_tx);
        self.completion_rx = Some(completion_rx);

        let strm_loop = StreamingLoop {
            inner: self.inner.clone(),
            params: self.params.clone(),
            sender,
            completion_tx,
            cancellation_rx,
        };
        std::thread::spawn(|| {
            strm_loop.run();
        });

        info!("start streaming loop successfully");
        Ok(())
    }

    fn stop_streaming_loop(&mut self) -> StreamResult<()> {
        if self.is_loop_running() {
            let (cancellation_tx, completion_rx) = (
                self.cancellation_tx.take().unwrap(),
                self.completion_rx.take().unwrap(),
            );
            unwrap_or_poisoned!(cancellation_tx.send(()))?;
            unwrap_or_poisoned!(task::block_on(completion_rx))?;
        }

        info!("stop streaming loop successfully");
        Ok(())
    }

    fn is_loop_running(&self) -> bool {
        debug_assert_eq!(self.completion_rx.is_some(), self.cancellation_tx.is_some());
        self.completion_rx.is_some()
    }
}

impl Drop for StreamHandle {
    fn drop(&mut self) {
        if let Err(e) = self.close() {
            error!(?e)
        }
    }
}

struct StreamingLoop {
    inner: Arc<Mutex<u3v::ReceiveChannel>>,
    params: StreamParams,
    sender: PayloadSender,
    completion_tx: oneshot::Sender<()>,
    cancellation_rx: oneshot::Receiver<()>,
}

impl StreamingLoop {
    fn run(mut self) {
        let mut trailer_buf = vec![0; self.params.trailer_size];
        let mut payload_buf_opt = None;
        let mut leader_buf = vec![0; self.params.leader_size];
        let mut inner = self.inner.lock().unwrap();

        loop {
            macro_rules! unwrap_or_continue {
                ($result:expr, $payload_buf:expr) => {
                    match $result {
                        Ok(v) => v,
                        Err(e) => {
                            warn!(?e);
                            // Reuse `payload_buf`.
                            payload_buf_opt = $payload_buf;
                            self.sender.try_send(Err(e)).ok();
                            continue;
                        }
                    }
                };
            }

            // Stop the loop when
            // 1. `cancellation_tx` sends signal.
            // 2. `cancellation_tx` is dropped.
            if self.cancellation_rx.try_recv().transpose().is_some() {
                break;
            }

            let maximum_payload_size = self.params.maximum_payload_size();
            let mut payload_buf = match payload_buf_opt.take() {
                Some(payload_buf) => payload_buf,
                None => match self.sender.try_recv() {
                    Ok(mut payload) => {
                        if payload.payload.len() != maximum_payload_size {
                            payload.payload.resize(maximum_payload_size, 0);
                        }
                        payload.payload
                    }
                    Err(_) => {
                        vec![0; maximum_payload_size]
                    }
                },
            };

            // We don't use `unwrap_or_continue` here not to emit so much logs.
            let leader = match read_leader(&mut inner, &self.params, &mut trailer_buf) {
                Ok(leader) => leader,
                Err(_) => {
                    payload_buf_opt = Some(payload_buf);
                    continue;
                }
            };
            unwrap_or_continue!(
                read_payload(&mut inner, &self.params, &mut payload_buf),
                Some(payload_buf)
            );
            let trailer = unwrap_or_continue!(
                read_trailer(&mut inner, &self.params, &mut leader_buf),
                Some(payload_buf)
            );

            let payload = unwrap_or_continue!(
                PayloadBuilder {
                    leader,
                    payload_buf,
                    trailer
                }
                .build(),
                None
            );
            if let Err(err) = self.sender.try_send(Ok(payload)) {
                warn!(?err);
            }
        }

        if let Err(e) = self.completion_tx.send(()) {
            error!(?e);
        }
    }
}

struct PayloadBuilder<'a> {
    leader: u3v_stream::Leader<'a>,
    payload_buf: Vec<u8>,
    trailer: u3v_stream::Trailer<'a>,
}

impl<'a> PayloadBuilder<'a> {
    fn build(self) -> StreamResult<Payload> {
        let payload_status = self.trailer.payload_status();
        if payload_status != u3v_stream::PayloadStatus::Success {
            return Err(StreamError::InvalidPayload(
                format!("trailer status indicates error: {:?}", payload_status).into(),
            ));
        }

        match self.leader.payload_type() {
            u3v_stream::PayloadType::Image => self.build_image_payload(),
            u3v_stream::PayloadType::ImageExtendedChunk => self.build_image_extended_payload(),
            u3v_stream::PayloadType::Chunk => self.build_chunk_payload(),
        }
    }

    fn build_image_payload(self) -> StreamResult<Payload> {
        let leader: u3v_stream::ImageLeader = self.specific_leader_as()?;
        let trailer: u3v_stream::ImageTrailer = self.specific_trailer_as()?;

        let id = self.leader.block_id();
        let valid_payload_size = self.trailer.valid_payload_size() as usize;

        let image_info = Some(ImageInfo {
            width: leader.width() as usize,
            height: trailer.actual_height() as usize,
            x_offset: leader.x_offset() as usize,
            y_offset: leader.y_offset() as usize,
            pixel_format: leader.pixel_format(),
            image_size: valid_payload_size,
        });

        Ok(Payload {
            id,
            payload_type: PayloadType::Image,
            image_info,
            payload: self.payload_buf,
            valid_payload_size,
            timestamp: leader.timestamp(),
        })
    }

    fn build_image_extended_payload(self) -> StreamResult<Payload> {
        const CHUNK_ID_LEN: usize = 4;
        const CHUNK_SIZE_LEN: usize = 4;

        let leader: u3v_stream::ImageExtendedChunkLeader = self.specific_leader_as()?;
        let trailer: u3v_stream::ImageExtendedChunkTrailer = self.specific_trailer_as()?;

        let id = self.leader.block_id();
        let valid_payload_size = self.trailer.valid_payload_size() as usize;

        // Extract image size from the first chunk of the paload data.
        // Chunk data is designed to be decoded from the last byte to the first byte.
        // Use chunk parser of `cameleon_genapi` once it gets implemented.
        let mut current_offset = valid_payload_size;
        let image_size = loop {
            current_offset = current_offset.checked_sub(CHUNK_SIZE_LEN).ok_or_else(|| {
                StreamError::InvalidPayload("failed to parse chunk data: size field missing".into())
            })?;
            let data_size = u32::from_be_bytes(
                self.payload_buf[current_offset..current_offset + CHUNK_SIZE_LEN]
                    .try_into()
                    .unwrap(),
            ) as usize;
            current_offset = current_offset.checked_sub(data_size + CHUNK_ID_LEN).ok_or_else(|| {
                StreamError::InvalidPayload(
                    "failed to parse chunk data: chunk data size is smaller than specified size".into()
                )
            })?;

            if current_offset == 0 {
                break data_size;
            }
        };

        let image_info = Some(ImageInfo {
            width: leader.width() as usize,
            height: trailer.actual_height() as usize,
            x_offset: leader.x_offset() as usize,
            y_offset: leader.y_offset() as usize,
            pixel_format: leader.pixel_format(),
            image_size,
        });

        Ok(Payload {
            id,
            payload_type: PayloadType::ImageExtendedChunk,
            image_info,
            payload: self.payload_buf,
            valid_payload_size,
            timestamp: leader.timestamp(),
        })
    }

    fn build_chunk_payload(self) -> StreamResult<Payload> {
        let leader: u3v_stream::ChunkLeader = self.specific_leader_as()?;
        let _: u3v_stream::ChunkTrailer = self.specific_trailer_as()?;

        let id = self.leader.block_id();
        let valid_payload_size = self.trailer.valid_payload_size() as usize;

        Ok(Payload {
            id,
            payload_type: PayloadType::Chunk,
            image_info: None,
            payload: self.payload_buf,
            valid_payload_size,
            timestamp: leader.timestamp(),
        })
    }

    fn specific_leader_as<T: u3v_stream::SpecificLeader>(&self) -> StreamResult<T> {
        self.leader
            .specific_leader_as()
            .map_err(|e| StreamError::InvalidPayload(format!("{}", e).into()))
    }

    fn specific_trailer_as<T: u3v_stream::SpecificTrailer>(&self) -> StreamResult<T> {
        self.trailer
            .specific_trailer_as()
            .map_err(|e| StreamError::InvalidPayload(format!("{}", e).into()))
    }
}

/// Parameters to receive stream packets.
///
/// Both [`StreamHandle`] doesn't check the integrity of the parameters. That's up to user.
#[derive(Debug, Clone, Default)]
pub struct StreamParams {
    /// Maximum leader size.
    pub leader_size: usize,

    /// Maximum trailer size.
    pub trailer_size: usize,

    /// Payload transfer size.
    pub payload_size: usize,

    /// Payload transfer count.
    pub payload_count: usize,

    /// Payload transfer final1 size.
    pub payload_final1_size: usize,

    /// Payload transfer final2 size.
    pub payload_final2_size: usize,

    /// Timeout duration of each transaction between device.
    pub timeout: Duration,
}

impl StreamParams {
    /// Return upper bound of payload size calculated by current `StreamParams` values.
    ///
    /// NOTE: Payload size may dynamically change according to settings of camera.
    pub fn maximum_payload_size(&self) -> usize {
        self.payload_size * self.payload_count + self.payload_final1_size + self.payload_final2_size
    }
}

impl StreamParams {
    /// Construct `StreamParams`.
    #[must_use]
    pub fn new(
        leader_size: usize,
        trailer_size: usize,
        payload_size: usize,
        payload_count: usize,
        payload_final1_size: usize,
        payload_final2_size: usize,
        timeout: Duration,
    ) -> Self {
        Self {
            leader_size,
            trailer_size,
            payload_size,
            payload_count,
            payload_final1_size,
            payload_final2_size,
            timeout,
        }
    }

    /// Build `StreamParams` from [`U3VDeviceControl`].
    pub fn from_device(device: &mut impl U3VDeviceControl) -> ControlResult<Self> {
        let sirm = device.sirm()?;
        let leader_size = sirm.maximum_leader_size(device)? as usize;
        let trailer_size = sirm.maximum_trailer_size(device)? as usize;

        let payload_size = sirm.payload_transfer_size(device)? as usize;
        let payload_count = sirm.payload_transfer_count(device)? as usize;
        let payload_final1_size = sirm.payload_final_transfer1_size(device)? as usize;
        let payload_final2_size = sirm.payload_final_transfer2_size(device)? as usize;
        let timeout = device.abrm()?.maximum_device_response_time(device)?;

        Ok(Self::new(
            leader_size,
            trailer_size,
            payload_size,
            payload_count,
            payload_final1_size,
            payload_final2_size,
            timeout,
        ))
    }
}

fn read_leader<'a>(
    inner: &mut MutexGuard<'_, u3v::ReceiveChannel>,
    params: &StreamParams,
    buf: &'a mut [u8],
) -> StreamResult<u3v_stream::Leader<'a>> {
    let leader_size = params.leader_size;
    recv(inner, params, buf, leader_size)?;

    u3v_stream::Leader::parse(buf).map_err(|e| StreamError::InvalidPayload(format!("{}", e).into()))
}

fn read_payload(
    inner: &mut MutexGuard<'_, u3v::ReceiveChannel>,
    params: &StreamParams,
    buf: &mut [u8],
) -> StreamResult<usize> {
    let mut read_len = 0;

    let payload_size = params.payload_size;
    for _ in 0..params.payload_count {
        read_len += recv(
            inner,
            params,
            &mut buf[read_len..read_len + payload_size],
            payload_size,
        )?;
    }

    let payload_final1_size = params.payload_final1_size;
    read_len += recv(
        inner,
        params,
        &mut buf[read_len..read_len + payload_final1_size],
        payload_final1_size,
    )?;

    let payload_final2_size = params.payload_final2_size;
    read_len += recv(
        inner,
        params,
        &mut buf[read_len..read_len + payload_final2_size],
        payload_final2_size,
    )?;

    Ok(read_len)
}

fn read_trailer<'a>(
    inner: &mut MutexGuard<'_, u3v::ReceiveChannel>,
    params: &StreamParams,
    buf: &'a mut [u8],
) -> StreamResult<u3v_stream::Trailer<'a>> {
    let trailer_size = params.trailer_size as usize;
    recv(inner, params, buf, trailer_size)?;

    u3v_stream::Trailer::parse(buf)
        .map_err(|e| StreamError::InvalidPayload(format!("invalid trailer: {}", e).into()))
}

fn recv(
    inner: &mut MutexGuard<'_, u3v::ReceiveChannel>,
    params: &StreamParams,
    buf: &mut [u8],
    len: usize,
) -> StreamResult<usize> {
    if len == 0 {
        return Ok(0);
    }

    if buf.len() < len {
        return Err(StreamError::BufferTooSmall);
    }

    inner
        .recv(&mut buf[..len], params.timeout)
        .map_err(|e| StreamError::Device(format!("{}", e).into()))
}