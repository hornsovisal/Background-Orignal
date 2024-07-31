use crate::{transport_protocol, SCOPED_APPS};
use agent_common::{AyaSocketAddrV4, AyaSocketAddrV6};
use aya_ebpf::{
    helpers::bpf_get_current_uid_gid,
    macros::{cgroup_sock_addr, map},
    maps::HashMap,
    programs::SockAddrContext,
};
// use aya_log_ebpf::info;

#[map]
pub static FORWARD_ADDR: HashMap<u32, AyaSocketAddrV4> = HashMap::with_max_entries(1024, 0);

// #[map]
// pub static SCOPED_APPS: HashMap<u32, u32> = HashMap::with_max_entries(1024, 0);

#[cgroup_sock_addr(connect4)]
pub fn connect4(ctx: SockAddrContext) -> i32 {
    let uid = bpf_get_current_uid_gid() as u32;
    if SCOPED_APPS.get_ptr(&uid).is_some() {
        return 0;
    }

    let bpf_sock_addr = unsafe { *ctx.sock_addr };
    let transport_protocol = bpf_sock_addr.protocol as u16;
    let left_socket_addr = AyaSocketAddrV4::new(
        bpf_sock_addr.user_ip4.to_ne_bytes().into(),
        (bpf_sock_addr.user_port as u16).swap_bytes(),
    );

    if matches!(transport_protocol, transport_protocol::TCP) {
        if let Some(addr) = FORWARD_ADDR.get_ptr(&(left_socket_addr.port() as u32)) {
            // info!(&ctx, "TCP {}", left_socket_addr.port());
            unsafe {
                if (*addr).ip() == left_socket_addr.ip() {
                    return 0;
                }
                (*ctx.sock_addr).user_ip4 = u32::from_ne_bytes((*addr).ip().octets());
                (*ctx.sock_addr).user_port = (*addr).port().swap_bytes() as u32;
            };
        }
    };
    1
}

#[cgroup_sock_addr(connect6)]
pub fn connect6(ctx: SockAddrContext) -> i32 {
    let uid = bpf_get_current_uid_gid() as u32;
    if SCOPED_APPS.get_ptr(&uid).is_some() {
        return 0;
    }

    let bpf_sock_addr = unsafe { *ctx.sock_addr };
    let transport_protocol = bpf_sock_addr.protocol as u16;
    let mut ip = [0u8; 16];
    for u_32 in bpf_sock_addr.user_ip6.iter().enumerate() {
        for u8 in u_32.1.to_ne_bytes().iter().enumerate() {
            ip[u_32.0 * 4 + u8.0] = *u8.1;
        }
    }
    let left_socket_addr =
        AyaSocketAddrV6::new(ip.into(), (bpf_sock_addr.user_port as u16).swap_bytes());
    if matches!(transport_protocol, transport_protocol::TCP) {
        if let Some(addr) = FORWARD_ADDR.get_ptr(&(left_socket_addr.port() as u32)) {
            // info!(&ctx, "TCP6 {}", left_socket_addr.port());
            unsafe {
                if (*ctx.sock_addr).user_ip6[3] == u32::from_ne_bytes((*addr).ip().octets()) {
                    return 0;
                }
                (*ctx.sock_addr).user_ip6[3] = u32::from_ne_bytes((*addr).ip().octets());
                (*ctx.sock_addr).user_port = (*addr).port().swap_bytes() as u32;
            };
        }
    };
    1
}
