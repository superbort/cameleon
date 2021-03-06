/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use futures::channel::oneshot;

use super::IfaceKind;

/// Signal sent to control module.
pub(super) enum ControlSignal {
    /// Signal notifying that the device has received data from the host.
    ReceiveData(Vec<u8>),

    /// Signal to cancel all jobs running on ContolModule.
    CancelJobs(oneshot::Sender<()>),

    /// Signal to clear Ei register.
    ClearEiRegister,

    /// Signal to clear Si register.
    ClearSiRegister,

    /// Signal to shutdown.
    Shutdown,
}

/// Signal sent to event module.
pub(super) enum EventSignal {
    /// Signal to send event data to tha host.
    _EventData {
        event_id: u16,
        data: Vec<u8>,
        request_id: u16,
    },

    /// Signal to update timestamp
    UpdateTimestamp(u64),

    /// signal to enable event module.
    _Enable,

    /// signal to disable event module.
    Disable(oneshot::Sender<()>),

    /// Signal to shutdown.
    Shutdown,
}

/// Signal sent to stream module.
pub(super) enum StreamSignal {
    /// Signal to enable stream module.
    Enable,

    /// Signal to disable stream module.
    Disable(oneshot::Sender<()>),

    /// Signal to shutdown.
    Shutdown,
}

/// Signal sent from each module to interface.
pub(super) enum InterfaceSignal {
    ToControl(ControlSignal),
    ToEvent(EventSignal),
    ToStream(StreamSignal),
    Halt(IfaceKind),
}

impl From<ControlSignal> for InterfaceSignal {
    fn from(val: ControlSignal) -> Self {
        Self::ToControl(val)
    }
}

impl From<EventSignal> for InterfaceSignal {
    fn from(val: EventSignal) -> Self {
        Self::ToEvent(val)
    }
}

impl From<StreamSignal> for InterfaceSignal {
    fn from(val: StreamSignal) -> Self {
        Self::ToStream(val)
    }
}
