// Copyright (c) SimpleStaking and Tezedge Contributors
// SPDX-License-Identifier: MIT

use core::{mem, ptr, convert::TryFrom, fmt};

#[cfg(feature = "probes")]
use redbpf_probes::helpers;

#[repr(C)]
pub struct DataDescriptor {
    pub id: EventId,
    pub tag: DataTag,
    pub size: i32,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct EventId {
    pub socket_id: SocketId,
    //ts: Range<u64>,
    ts: u64,
}

impl EventId {
    #[cfg(feature = "probes")]
    fn new(socket_id: SocketId, _ts_start: u64, ts_finish: u64) -> Self {
        EventId {
            socket_id: socket_id,
            ts: ts_finish,
        }
    }

    #[cfg(feature = "probes")]
    pub fn unknown_fd() -> Self {
        let ts = helpers::bpf_ktime_get_ns();
        let id = helpers::bpf_get_current_pid_tgid();
        let socket_id = SocketId {
            pid: (id >> 32) as u32,
            fd: 0,
        };
        // same timestamp because event is instant
        Self::new(socket_id, ts, ts)
    }

    #[cfg(feature = "probes")]
    pub fn now(fd: u32, ts0: u64) -> EventId {
        Self::new(SocketId::this(fd), ts0, helpers::bpf_ktime_get_ns())
    }

    pub fn ts_start(&self) -> u64 {
        0
    }

    pub fn ts_finish(&self) -> u64 {
        self.ts
    }
}

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.socket_id, self.ts)
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct SocketId {
    pub pid: u32,
    pub fd: u32,
}

impl SocketId {
    #[cfg(feature = "probes")]
    pub fn this(fd: u32) -> SocketId {
        let id = helpers::bpf_get_current_pid_tgid();

        SocketId {
            pid: (id >> 32) as u32,
            fd: fd,
        }
    }
}

impl fmt::Display for SocketId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.pid, self.fd)
    }
}

impl TryFrom<&[u8]> for DataDescriptor {
    type Error = ();

    // TODO: rewrite safe
    fn try_from(v: &[u8]) -> Result<Self, Self::Error> {
        if v.len() >= mem::size_of::<Self>() {
            /*Ok(DataDescriptor {
                id: EventId {
                    socket_id: SocketId {
                        pid: u32::from_le_bytes(TryFrom::try_from(&v[0..4]).unwrap()),
                        fd: u32::from_le_bytes(TryFrom::try_from(&v[4..8]).unwrap()),
                    },
                    ts: u64::from_le_bytes(TryFrom::try_from(&v[8..16]).unwrap()),
                },
                tag: DataTag::try_from_u32(u32::from_le_bytes(TryFrom::try_from(&v[16..20]).unwrap())).ok_or(())?,
                size: i32::from_le_bytes(TryFrom::try_from(&v[20..24]).unwrap()),
            })*/
            Ok(unsafe { ptr::read(v.as_ptr() as *const Self) })
        } else {
            Err(())
        }
    }
}

#[repr(u32)]
#[derive(Debug)]
pub enum DataTag {
    Write,
    SendTo,

    Read,
    RecvFrom,

    Connect,
    Bind,
    Listen,
    Accept,
    Close,

    Debug,
}

impl DataTag {
    #[allow(dead_code)]
    fn try_from_u32(v: u32) -> Option<Self> {
        if v == Self::Write as u32 {
            Some(Self::Write)
        } else if v == Self::SendTo as u32 {
            Some(Self::SendTo)
        } else if v == Self::Read as u32 {
            Some(Self::Read)
        } else if v == Self::RecvFrom as u32 {
            Some(Self::RecvFrom)
        } else if v == Self::Connect as u32 {
            Some(Self::Connect)
        } else if v == Self::Bind as u32 {
            Some(Self::Bind)
        } else if v == Self::Listen as u32 {
            Some(Self::Listen)
        } else if v == Self::Accept as u32 {
            Some(Self::Accept)
        } else if v == Self::Close as u32 {
            Some(Self::Close)
        } else if v == Self::Debug as u32 {
            Some(Self::Debug)
        } else {
            None
        }
    }
}
