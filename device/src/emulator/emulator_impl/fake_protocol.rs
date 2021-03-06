/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

/// This packet is sent from a host.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct FakeReqPacket {
    pub(super) iface: IfaceKind,
    pub(super) kind: FakeReqKind,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum FakeReqKind {
    Send(Vec<u8>),
    Recv,
    SetHalt,
    ClearHalt,
}

impl FakeReqPacket {
    pub(super) fn new(iface: IfaceKind, kind: FakeReqKind) -> Self {
        Self { iface, kind }
    }
}

impl FakeReqKind {
    pub(super) fn is_set_halt(&self) -> bool {
        matches!(self, Self::SetHalt)
    }

    pub(super) fn is_clear_halt(&self) -> bool {
        matches!(self, Self::ClearHalt)
    }
}

/// This packet is sent from a device.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct FakeAckPacket {
    pub(super) iface: IfaceKind,
    pub(super) kind: FakeAckKind,
}

impl FakeAckPacket {
    pub(super) fn new(iface: IfaceKind, kind: FakeAckKind) -> Self {
        Self { iface, kind }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum FakeAckKind {
    SendAck,
    RecvAck(Vec<u8>),
    RecvNak,
    IfaceHalted,
    SetHaltAck,
    ClearHaltAck,
    BrokenReq,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum IfaceKind {
    Control,
    Event,
    Stream,
}
