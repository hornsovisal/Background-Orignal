#![no_std]

use core::{
    fmt::Debug,
    net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6},
    num::NonZeroUsize,
};

use aya_log_common::WriteToBuf;
#[cfg(feature = "user")]
use serde::{Deserialize, Serialize};

pub enum Transport {
    TCP,
    UDP,
}
pub enum AyaSocketAddr {
    V4(AyaSocketAddrV4),
    V6(AyaSocketAddrV6),
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "user", derive(Serialize, Deserialize))]
pub struct AyaSocketAddrV4 {
    pub ip: Ipv4Addr,
    pub port: u16,
}

impl WriteToBuf for AyaSocketAddrV4 {
    fn write(self, buf: &mut [u8]) -> Option<NonZeroUsize> {
        #[allow(unused_mut)]
        let mut ip_bytes = self.ip.octets();
        buf[0..4].copy_from_slice(&ip_bytes);
        buf[4..6].copy_from_slice(&self.port.to_ne_bytes());
        NonZeroUsize::new(6)
    }
}

impl From<SocketAddrV4> for AyaSocketAddrV4 {
    fn from(value: SocketAddrV4) -> Self {
        AyaSocketAddrV4 {
            ip: *value.ip(),
            port: value.port(),
        }
    }
}

impl Into<SocketAddrV4> for AyaSocketAddrV4 {
    fn into(self) -> SocketAddrV4 {
        SocketAddrV4::new(self.ip, self.port)
    }
}

impl AyaSocketAddrV4 {
    pub fn new(ip: Ipv4Addr, port: u16) -> AyaSocketAddrV4 {
        AyaSocketAddrV4 { ip, port }
    }
    pub fn port(&self) -> u16 {
        self.port
    }
    pub fn ip(&self) -> &Ipv4Addr {
        &self.ip
    }
}

impl Into<[u8; 6]> for AyaSocketAddrV4 {
    fn into(self) -> [u8; 6] {
        let mut buf = [0; 6];
        self.write(&mut buf);
        buf
    }
}

impl From<&[u8]> for AyaSocketAddrV4 {
    fn from(buf: &[u8]) -> AyaSocketAddrV4 {
        let ip = Ipv4Addr::from([buf[0], buf[1], buf[2], buf[3]]);
        // let ip = Ipv4Addr::new(buf[0], buf[1], buf[2], buf[3]);
        let port = u16::from_be_bytes([buf[4], buf[5]]);
        AyaSocketAddrV4 { ip, port }
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "user", derive(Serialize, Deserialize))]
pub struct SocketAddrRecordV4 {
    pub uid: u32,
    pub pid: u32,
    pub tid: u32,
    pub protocol: u16,
    pub r#type: u16,
    pub left_socket_addr: AyaSocketAddrV4,
    pub right_socket_addr: AyaSocketAddrV4,
}

impl SocketAddrRecordV4 {
    pub fn new(
        uid: u32,
        pid: u32,
        tid: u32,
        protocol: u16,
        r#type: u16,
        left_socket_addr: AyaSocketAddrV4,
        right_socket_addr: AyaSocketAddrV4,
    ) -> SocketAddrRecordV4 {
        SocketAddrRecordV4 {
            uid,
            pid,
            tid,
            protocol,
            r#type,
            left_socket_addr,
            right_socket_addr,
        }
    }
    pub fn right_socket_addr(&self) -> &AyaSocketAddrV4 {
        &self.right_socket_addr
    }
    pub fn left_socket_addr(&self) -> &AyaSocketAddrV4 {
        &self.left_socket_addr
    }
    pub fn protocol(&self) -> u16 {
        self.protocol
    }
    pub fn r#type(&self) -> u16 {
        self.r#type
    }
    pub fn uid(&self) -> u32 {
        self.uid
    }
    pub fn pid(&self) -> u32 {
        self.pid
    }
    pub fn tid(&self) -> u32 {
        self.tid
    }
}

impl From<&[u8]> for SocketAddrRecordV4 {
    fn from(buf: &[u8]) -> SocketAddrRecordV4 {
        let uid = u32::from_ne_bytes([buf[0], buf[1], buf[2], buf[3]]);
        let pid = u32::from_ne_bytes([buf[4], buf[5], buf[6], buf[7]]);
        let tid = u32::from_ne_bytes([buf[8], buf[9], buf[10], buf[11]]);
        let protocol = u16::from_ne_bytes([buf[12], buf[13]]);
        let r#type = u16::from_ne_bytes([buf[14], buf[15]]);
        let left_socket_addr = AyaSocketAddrV4::new(
            Ipv4Addr::from(u32::from_be_bytes([buf[16], buf[17], buf[18], buf[19]])),
            u16::from_be_bytes([buf[20], buf[21]]),
        );
        let right_socket_addr = AyaSocketAddrV4::new(
            Ipv4Addr::from(u32::from_be_bytes([buf[22], buf[23], buf[24], buf[25]])),
            u16::from_be_bytes([buf[26], buf[27]]),
        );
        SocketAddrRecordV4 {
            uid,
            pid,
            tid,
            protocol,
            r#type,
            left_socket_addr,
            right_socket_addr,
        }
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "user", derive(Serialize, Deserialize))]
pub struct AyaSocketAddrV6 {
    pub ip: Ipv6Addr,
    pub port: u16,
}

impl AyaSocketAddrV6 {
    pub fn new(ip: Ipv6Addr, port: u16) -> AyaSocketAddrV6 {
        AyaSocketAddrV6 { ip, port }
    }
    pub fn port(&self) -> u16 {
        self.port
    }
    pub fn ip(&self) -> &Ipv6Addr {
        &self.ip
    }
}

impl Into<[u8; 18]> for AyaSocketAddrV6 {
    fn into(self) -> [u8; 18] {
        let mut buf = [0; 18];
        self.write(&mut buf);
        buf
    }
}

impl From<&[u8]> for AyaSocketAddrV6 {
    fn from(buf: &[u8]) -> AyaSocketAddrV6 {
        let ip = Ipv6Addr::new(
            u16::from_ne_bytes([buf[0], buf[1]]),
            u16::from_ne_bytes([buf[2], buf[3]]),
            u16::from_ne_bytes([buf[4], buf[5]]),
            u16::from_ne_bytes([buf[6], buf[7]]),
            u16::from_ne_bytes([buf[8], buf[9]]),
            u16::from_ne_bytes([buf[10], buf[11]]),
            u16::from_ne_bytes([buf[12], buf[13]]),
            u16::from_ne_bytes([buf[14], buf[15]]),
        );
        let port = u16::from_be_bytes([buf[16], buf[17]]);
        AyaSocketAddrV6 { ip, port }
    }
}

impl WriteToBuf for AyaSocketAddrV6 {
    fn write(self, buf: &mut [u8]) -> Option<NonZeroUsize> {
        buf[0..16].copy_from_slice(&self.ip.octets());
        buf[16..18].copy_from_slice(&self.port.to_ne_bytes());
        NonZeroUsize::new(18)
    }
}

impl From<SocketAddrV6> for AyaSocketAddrV6 {
    fn from(value: SocketAddrV6) -> Self {
        AyaSocketAddrV6 {
            ip: *value.ip(),
            port: value.port(),
        }
    }
}

impl Into<SocketAddrV6> for AyaSocketAddrV6 {
    fn into(self) -> SocketAddrV6 {
        SocketAddrV6::new(self.ip, self.port, 0, 0)
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "user", derive(Serialize, Deserialize))]
pub struct SocketAddrRecordV6 {
    pub uid: u32,
    pub pid: u32,
    pub tid: u32,
    pub protocol: u16,
    r#type: u16,
    pub left_socket_addr: AyaSocketAddrV6,
    pub right_socket_addr: AyaSocketAddrV6,
}

impl SocketAddrRecordV6 {
    pub fn new(
        uid: u32,
        pid: u32,
        tid: u32,
        protocol: u16,
        r#type: u16,
        left_socket_addr: AyaSocketAddrV6,
        right_socket_addr: AyaSocketAddrV6,
    ) -> SocketAddrRecordV6 {
        SocketAddrRecordV6 {
            uid,
            pid,
            tid,
            protocol,
            r#type,
            left_socket_addr,
            right_socket_addr,
        }
    }
    pub fn right_socket_addr(&self) -> &AyaSocketAddrV6 {
        &self.right_socket_addr
    }
    pub fn left_socket_addr(&self) -> &AyaSocketAddrV6 {
        &self.left_socket_addr
    }
    pub fn protocol(&self) -> u16 {
        self.protocol
    }
    pub fn r#type(&self) -> u16 {
        self.r#type
    }
    pub fn uid(&self) -> u32 {
        self.uid
    }
    pub fn pid(&self) -> u32 {
        self.pid
    }
    pub fn tid(&self) -> u32 {
        self.tid
    }
}

impl From<&[u8]> for SocketAddrRecordV6 {
    fn from(buf: &[u8]) -> SocketAddrRecordV6 {
        let uid = u32::from_ne_bytes([buf[0], buf[1], buf[2], buf[3]]);
        let pid = u32::from_ne_bytes([buf[4], buf[5], buf[6], buf[7]]);
        let tid = u32::from_ne_bytes([buf[8], buf[9], buf[10], buf[11]]);
        let protocol = u16::from_ne_bytes([buf[12], buf[13]]);
        let r#type = u16::from_ne_bytes([buf[14], buf[15]]);
        let left_socket_addr = AyaSocketAddrV6::from(&buf[16..34]);
        let right_socket_addr = AyaSocketAddrV6::from(&buf[34..52]);
        SocketAddrRecordV6 {
            uid,
            pid,
            tid,
            protocol,
            r#type,
            left_socket_addr,
            right_socket_addr,
        }
    }
}
#[derive(Debug)]
#[cfg_attr(feature = "user", derive(Serialize, Deserialize))]
pub struct ProcessForkEvent {
    pub parent_pid: u32,
    pub parent_tid: u32,
    pub child_pid: u32,
    pub uid: u32,
}

impl ProcessForkEvent {
    pub fn new(parent_pid: u32, parent_tid: u32, child_pid: u32, uid: u32) -> Self {
        Self {
            parent_pid,
            parent_tid,
            child_pid,
            uid,
        }
    }
}

impl From<&[u8]> for ProcessForkEvent {
    fn from(buf: &[u8]) -> ProcessForkEvent {
        let parent_pid = u32::from_ne_bytes([buf[0], buf[1], buf[2], buf[3]]);
        let parent_tid = u32::from_ne_bytes([buf[4], buf[5], buf[6], buf[7]]);
        let child_pid = u32::from_ne_bytes([buf[8], buf[9], buf[10], buf[11]]);
        let uid = u32::from_ne_bytes([buf[12], buf[13], buf[14], buf[15]]);
        ProcessForkEvent {
            parent_pid,
            parent_tid,
            child_pid,
            uid,
        }
    }
}
