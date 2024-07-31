use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::net::SocketAddrV4;
use std::pin::Pin;
use std::str::Bytes;
use std::task::{Context, Poll};
use std::time::{UNIX_EPOCH,SystemTime};

use agent_common::{AyaSocketAddrV4, ProcessForkEvent, SocketAddrRecordV4, SocketAddrRecordV6};
use aya::maps::perf::{AsyncPerfEventArrayBuffer, PerfBufferError};
use aya::maps::{AsyncPerfEventArray, HashMap, MapData};
use aya::programs::{CgroupSockAddr, KProbe, TracePoint};
use aya::util::online_cpus;
use aya::{include_bytes_aligned, Ebpf};
use aya_log::EbpfLogger;
use bytes::BytesMut;
use futures::{FutureExt, Stream, StreamExt};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use tokio::{select, task};

pub struct EbpfManager {
    pub receiver: tokio::sync::mpsc::UnboundedReceiver<Event>,
    bpf: Ebpf,
}

impl EbpfManager {
    pub fn new(apps_uid: Vec<u32>, forward_addr: SocketAddrV4) -> Self {
        let cgroup_path = "/sys/fs/cgroup";
        let ip = AyaSocketAddrV4::new(*forward_addr.ip(), forward_addr.port());
        rlimit::increase_nofile_limit(rlimit::INFINITY).ok();
        rlimit::setrlimit(
            rlimit::Resource::MEMLOCK,
            rlimit::INFINITY,
            rlimit::INFINITY,
        );

        #[cfg(debug_assertions)]
        let mut bpf = Ebpf::load(include_bytes_aligned!(
            "../../target/bpfel-unknown-none/debug/agent"
        ))
        .unwrap();
        #[cfg(not(debug_assertions))]
        let mut bpf = Ebpf::load(include_bytes_aligned!(
            "../../target/bpfel-unknown-none/release/agent"
        ))
        .unwrap();
        if let Err(e) = EbpfLogger::init(&mut bpf) {
            // This can happen if you remove all log statements from your eBPF program.
            warn!("failed to initialize eBPF logger: {}", e);
        }
        let mut sched_process_fork_event =
            AsyncPerfEventArray::try_from(bpf.take_map("SCHED_PROCESS_FORK_EVENT").unwrap())
                .unwrap();
        let mut scoped_apps = HashMap::try_from(bpf.map_mut("SCOPED_APPS").unwrap()).unwrap();
        for uid in apps_uid {
            scoped_apps.insert(uid, 0, 0).unwrap();
        }
        let mut forward_addrs: HashMap<&mut MapData, i32, [u8; 6]> =
            HashMap::try_from(bpf.map_mut("FORWARD_ADDR").unwrap()).unwrap();
        forward_addrs.insert(80, Into::<[u8; 6]>::into(ip), 0);
        forward_addrs.insert(443, Into::<[u8; 6]>::into(ip), 0);

        let mut ipv4_event: AsyncPerfEventArray<aya::maps::MapData> =
            AsyncPerfEventArray::try_from(bpf.take_map("IPV4_EVENT").unwrap()).unwrap();
        let mut ipv6_event: AsyncPerfEventArray<aya::maps::MapData> =
            AsyncPerfEventArray::try_from(bpf.take_map("IPV6_EVENT").unwrap()).unwrap();
        let program: &mut TracePoint = bpf
            .program_mut("sched_process_fork")
            .unwrap()
            .try_into()
            .unwrap();
        program.load().unwrap();
        program.attach("sched", "sched_process_fork").unwrap();
        // let program: &mut TracePoint = bpf
        //     .program_mut("sched_process_exit")
        //     .unwrap()
        //     .try_into()
        //     .unwrap();
        // program.load().unwrap();
        // program.attach("sched", "sched_process_exit").unwrap();
        let program: &mut TracePoint = bpf.program_mut("task_newtask").unwrap().try_into().unwrap();
        program.load().unwrap();
        program.attach("task", "task_rename").unwrap();

        for prog in vec!["connect4", "connect6"] {
            let program: &mut CgroupSockAddr = bpf.program_mut(prog).unwrap().try_into().unwrap();
            let cgroup = std::fs::File::open(&cgroup_path).unwrap();
            program.load().unwrap();
            program.attach(cgroup).unwrap();
        }

        let program: &mut KProbe = bpf.program_mut("tcp_connect").unwrap().try_into().unwrap();
        program.load().unwrap();
        // program.attach("__sys_setuid", 0).unwrap();
        program.attach("tcp_connect", 0).unwrap();
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

        for cpu_id in online_cpus().unwrap() {
            let mut sched_process_fork_event_pref_buffer =
                sched_process_fork_event.open(cpu_id, None).unwrap();
            let mut ipv4_event_pref_buffer = ipv4_event.open(cpu_id, None).unwrap();
            let mut ipv6_event_pref_buffer = ipv6_event.open(cpu_id, None).unwrap();
            task::spawn({
                let sender = sender.clone();
                async move {
                    let mut sched_process_fork_event_buffer = (0..10)
                        .map(|_| BytesMut::with_capacity(16))
                        .collect::<Vec<_>>();
                    let mut ipv4_event_buffer = (0..10)
                        .map(|_| BytesMut::with_capacity(32))
                        .collect::<Vec<_>>();
                    let mut ipv6_event_buffer = (0..10)
                        .map(|_| BytesMut::with_capacity(64))
                        .collect::<Vec<_>>();
                    loop {
                        let start = SystemTime::now();
                        let ts = start
                            .duration_since(UNIX_EPOCH)
                            .expect("Time went backwards").as_secs();
                        select! {
                            e = sched_process_fork_event_pref_buffer.read_events(&mut sched_process_fork_event_buffer).fuse() => {
                                for i in 0..e.unwrap().read {
                                    sender.send(Event::ProcessFork(ProcessForkEvent::from(sched_process_fork_event_buffer[i].as_ref()),ts));
                                }
                            },
                            e = ipv4_event_pref_buffer.read_events(&mut ipv4_event_buffer).fuse() => {
                                for i in 0..e.unwrap().read {
                                    sender.send(Event::SocketAddrV4(SocketAddrRecordV4::from(ipv4_event_buffer[i].as_ref()),ts));
                                }
                            },
                            e = ipv6_event_pref_buffer.read_events(&mut ipv6_event_buffer).fuse() => {
                                for i in 0..e.unwrap().read {
                                    sender.send(Event::SocketAddrV6(SocketAddrRecordV6::from(ipv6_event_buffer[i].as_ref()),ts));
                                }
                            },
                        }
                    }

                    Ok::<_, PerfBufferError>(())
                }
            });
        }

        Self { bpf, receiver }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Event {
    ProcessFork(ProcessForkEvent, u64),
    SocketAddrV4(SocketAddrRecordV4, u64),
    SocketAddrV6(SocketAddrRecordV6, u64),
}
