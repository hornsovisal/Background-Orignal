use core::net::Ipv6Addr;

use crate::{address_family, SCOPED_APPS};
use agent_common::{AyaSocketAddrV4, AyaSocketAddrV6, SocketAddrRecordV4, SocketAddrRecordV6};
use aya_ebpf::{
    helpers::{bpf_get_current_pid_tgid, bpf_get_current_uid_gid, bpf_probe_read_kernel},
    macros::{kprobe, map},
    maps::{HashMap, PerfEventArray},
    programs::ProbeContext,
};

use crate::{
    binding::{sock, sock_common},
    transport_protocol,
};

#[map]
pub static IPV4_EVENT: PerfEventArray<SocketAddrRecordV4> =
    PerfEventArray::with_max_entries(128, 0);
#[map]
pub static IPV6_EVENT: PerfEventArray<SocketAddrRecordV6> =
    PerfEventArray::with_max_entries(128, 0);

#[kprobe]
pub fn tcp_connect(ctx: ProbeContext) -> u32 {
    let uid = bpf_get_current_uid_gid() as u32;
    if SCOPED_APPS.get_ptr(&uid).is_some() {
        return 0;
    }
    let sock: *mut sock = ctx.arg(0).ok_or(1i64).unwrap();
    let sk_common =
        unsafe { bpf_probe_read_kernel(&(*sock).__sk_common as *const sock_common).unwrap() };
    let skc_portpair = unsafe { sk_common.__bindgen_anon_3.skc_portpair };
    let pid = (bpf_get_current_pid_tgid() >> 32) as u32;
    let tid = (bpf_get_current_pid_tgid()) as u32;
    match sk_common.skc_family {
        address_family::AF_INET => {
            let skc_addrpair = unsafe { sk_common.__bindgen_anon_1.skc_addrpair };
            let local_addr = AyaSocketAddrV4::new(
                ((skc_addrpair >> 32) as u32).to_ne_bytes().into(),
                (skc_portpair >> 16) as u16,
            );
            let peer_addr = AyaSocketAddrV4::new(
                (skc_addrpair as u32).to_ne_bytes().into(),
                skc_portpair as u16,
            );
            let record = SocketAddrRecordV4::new(
                uid,
                pid,
                tid,
                transport_protocol::TCP,
                1,
                local_addr,
                peer_addr,
            );
            IPV4_EVENT.output(&ctx, &record, 0);
        }
        address_family::AF_INET6 => {
            let local_addr = AyaSocketAddrV6::new(
                Ipv6Addr::from(unsafe { sk_common.skc_v6_rcv_saddr.in6_u.u6_addr16 }),
                (skc_portpair >> 16) as u16,
            );
            let peer_addr = AyaSocketAddrV6::new(
                Ipv6Addr::from(unsafe { sk_common.skc_v6_daddr.in6_u.u6_addr16 }),
                skc_portpair as u16,
            );

            let record = SocketAddrRecordV6::new(
                uid,
                pid,
                tid,
                transport_protocol::TCP,
                1,
                local_addr,
                peer_addr,
            );
            IPV6_EVENT.output(&ctx, &record, 0);
            // warn!(
            //     &ctx,
            //     "ipv6 - {} - {}",
            //     (skc_portpair >> 16) as u16,
            //     (skc_portpair) as u16
            // );
        }
        _ => {}
    }

    // info!(&ctx, "local addr {} - peer addr {}", local_addr, peer_addr);
    // info!(&ctx, "tcp_connect pro {}", sk_common.skc_family);
    0
}

// #[kretprobe]
// pub fn sys_setuid(ctx: ProbeContext) -> u32 {
//     // let uid = ctx.uid();
//     let ret: i32 = 0; //ctx.ret().ok_or(1_u32).unwrap();

//     // unsafe {
//     //     bpf_send_signal(4);
//     //     bpf_send_signal_thread(4);
//     // };
//     warn!(
//         &ctx,
//         "!!: pid: {},uid: {},ret: {}",
//         (bpf_get_current_pid_tgid() >> 32),
//         (bpf_get_current_uid_gid() >> 32),
//         ret
//     );

//     warn!(&ctx, "setuid");
//     0
// }
// #[kprobe]
// pub fn udp_sendmsg(ctx: ProbeContext) -> u32 {
//     let sock: *mut sock = ctx.arg(0).ok_or(1i64).unwrap();
//     let sk_common =
//         unsafe { bpf_probe_read_kernel(&(*sock).__sk_common as *const sock_common).unwrap() };
//     let pid = bpf_get_current_pid_tgid() as u32;
//     let uid = bpf_get_current_uid_gid() as u32;
//     let skc_portpair = unsafe { sk_common.__bindgen_anon_3.skc_portpair };
//     let skc_addrpair = unsafe { sk_common.__bindgen_anon_1.skc_addrpair };
//     let local_addr = AyaSocketAddrV4::new(
//         ((skc_addrpair >> 32) as u32).to_ne_bytes().into(),
//         (skc_portpair >> 16) as u16,
//     );
//     let peer_addr = AyaSocketAddrV4::new(
//         (skc_addrpair as u32).to_ne_bytes().into(),
//         skc_portpair as u16,
//     );
//     let record =
//         SocketAddrRecordV4::new(uid, pid, transport_protocol::UDP, 1, local_addr, peer_addr);
//     IPV4_EVENT.output(&ctx, &record, 0);
//     0
// }
