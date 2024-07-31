use agent_common::ProcessForkEvent;
use aya_ebpf::{
    helpers::{bpf_get_current_pid_tgid, bpf_get_current_uid_gid, bpf_send_signal, bpf_send_signal_thread},
    macros::{map, tracepoint},
    maps::{HashMap, PerfEventArray},
    programs::TracePointContext,
    EbpfContext,
};

use crate::SCOPED_APPS;
// use aya_log_ebpf::{info, warn};

#[map]
pub static SCHED_PROCESS_FORK_EVENT: PerfEventArray<ProcessForkEvent> =
    PerfEventArray::with_max_entries(128, 0);

// #[map]
// pub static RUNNING_PIDS: HashMap<u32, u32> = HashMap::with_max_entries(1024 * 8, 0);

#[tracepoint]
pub fn sched_process_fork(ctx: TracePointContext) -> u32 {
    let uid = bpf_get_current_uid_gid() as u32;
    if SCOPED_APPS.get_ptr(&uid).is_some() {
        return 0;
    }
    // let pid = bpf_get_current_pid_tgid(); // ctx.read_at::<u32>(24).unwrap_or((bpf_get_current_pid_tgid() >> 32) as u32)
    let pid = (bpf_get_current_pid_tgid() >> 32) as u32;
    let tid = (bpf_get_current_pid_tgid()) as u32;
    let cpid = unsafe { ctx.read_at::<u32>(44).unwrap_or(u32::MAX) };
    SCHED_PROCESS_FORK_EVENT.output(&ctx, &ProcessForkEvent::new(pid,tid, cpid, uid), 0);
    // RUNNING_PIDS.insert(&pid, &0, 0).unwrap();

    0
}

#[tracepoint]
pub fn task_newtask(ctx: TracePointContext) -> u32 {
    let uid = bpf_get_current_uid_gid() as u32;
    if SCOPED_APPS.get_ptr(&uid).is_some() {
        return 0;
    }
    let pid = (bpf_get_current_pid_tgid() >> 32) as u32;
    let tid = (bpf_get_current_pid_tgid()) as u32;
    SCHED_PROCESS_FORK_EVENT.output(&ctx, &ProcessForkEvent::new(pid,tid, u32::MAX - 1, uid), 0);

    0
}

// #[tracepoint]
// pub fn sched_process_exit(ctx: TracePointContext) -> u32 {
//     // let uid = ctx.uid();
//     let pid = unsafe {
//         ctx.read_at::<u32>(24)
//             .unwrap_or((bpf_get_current_pid_tgid() >> 32) as u32)
//     };
//     _ = RUNNING_PIDS.remove(&pid);
//     // warn!(&ctx, "exit: pid: {} , uid: {}", pid, uid);
//     0
// }

// sched_process_exec
