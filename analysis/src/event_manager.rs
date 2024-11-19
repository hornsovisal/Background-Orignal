use std::{cmp, u128};

use crate::{background_work::BackgroundWorks, db::NetworkConnection, events::Events};

pub struct EventManager {
    pub events: Vec<Events>,
}

impl EventManager {
    pub fn new(mut events: Vec<Events>) -> Self {
        events.sort_by(|a, b| a.timestamp().cmp(&b.timestamp()));
        Self { events }
    }
    pub fn get_inscope_events_by_ptid(&self, pid: i64, tid: i64, ts: u128) -> Vec<Events> {
        let mut parents = Vec::new();
        let mut exits = Vec::new();
        let mut e = Vec::new();
        for event in &self.events {
            if event.get_ptid() == (pid, tid) {
                e.push(event.clone());
            }
            if let Events::SchedProcessFork(data) = event {
                if data.parent_pid == pid && data.child_pid == tid {
                    parents.push(event.clone());
                }
            }
            if let Events::SchedProcessExit(data) = event {
                if data.parent_pid == pid && data.parent_tid == tid {
                    exits.push(event.clone());
                }
            }
        }
        e.retain(|_e| {
            Self::find_closest_event(ts, &parents).unwrap_or(0) < _e.timestamp()
                && Self::find_closest_event(ts, &exits).unwrap_or(u128::MAX) > _e.timestamp()
        });
        e
    }
    pub fn get_parent(&self, event: &Events) -> Option<Events> {
        let mut i = self.events.iter().position(|e| e == event)?;
        loop {
            if let Events::SchedProcessFork(e) = self.events.get(i)? {
                let (pid, tid) = event.get_ptid();
                if e.parent_pid == pid && e.child_pid == tid {
                    // dbg!(e);
                    // dbg!(event);
                    return Some(self.events.get(i)?.clone());
                }
            }
            if i == 0 {
                return None;
            }
            i -= 1;
        }
    }

    pub fn find_closest_event(ts: u128, events: &[Events]) -> Option<u128> {
        let mut closest = Option::None;
        let mut closest_diff = u128::MAX;
        for e in events {
            let diff = e.timestamp().abs_diff(ts);
            if diff < closest_diff {
                closest = Some(e.timestamp());
                closest_diff = diff;
            }
        }
        closest
    }
    pub fn get_network_connections(&self) -> Vec<NetworkConnection> {
        let mut connections = Vec::new();
        let mut native_connections = Vec::new();
        for e in &self.events {
            match e {
                Events::SocketHook(e) => {
                    let Some(src_addr) =
                        e.get_local_addr().map(|(ip, port)| format!("{ip}:{port}"))
                    else {
                        continue;
                    };

                    // let Some(ts) = ts_int_to_naive_datetime(e.ts) else {
                    //     continue;
                    // };

                    if connections
                        .iter()
                        .any(|x: &NetworkConnection| x.src_addr == src_addr && x.ts == e.ts)
                    {
                        continue;
                    }
                    let mut target = None;
                    if !e.canary.is_empty() {
                        for event in &self.events {
                            if let Events::Canary(c) = event {
                                if c.canary.starts_with(&e.canary) {
                                    target = Some(c.target.clone());
                                    break;
                                }
                            }
                        }
                    }
                    connections.push(NetworkConnection {
                        src_addr,
                        destination_addr: None,
                        importance: e.importance as i32,
                        tid: e.tid as i32,
                        pid: e.pid as i32,
                        target,
                        ts: e.ts,
                        leaks: Default::default(),
                    });
                    // let con = ;
                }
                Events::SocketAddrV4(e) => {
                    let Some(src_addr) =
                        e.get_local_addr().map(|(ip, port)| format!("{ip}:{port}"))
                    else {
                        continue;
                    };
                    native_connections.push((src_addr, e.tid, e.pid, e.ts));
                }
                Events::SocketAddrV6(e) => {
                    let Some(src_addr) =
                        e.get_local_addr().map(|(ip, port)| format!("{ip}:{port}"))
                    else {
                        continue;
                    };
                    native_connections.push((src_addr, e.tid, e.pid, e.ts));
                }
                _ => {}
            }
        }
        for (src_addr, tid, pid, ts) in native_connections {
            if connections
                .iter()
                .any(|x: &NetworkConnection| x.src_addr == src_addr)
            {
                continue;
            }
            // let Some(naive_ts) = ts_int_to_naive_datetime(ts) else {
            //     continue;
            // };
            let mut target = None;
            if pid != tid {
                let mut inscope_events = self.get_inscope_events_by_ptid(pid, tid, ts);
                'outer: loop {
                    if inscope_events.is_empty() {
                        break;
                    }
                    for e in &inscope_events {
                        if let Some(rw) = BackgroundWorks::from_event(e)
                            .map(|x| x.get_work_name())
                            .map(|(c, m)| format!("{}.{}", c, m))
                        {
                            target = Some(rw);
                            break 'outer;
                        }
                        if let Some(canary) =
                            e.get_stacktrace().extract_canary_info().and_then(|x| x.0)
                        {
                            for event in &self.events {
                                if let Events::Canary(c) = event {
                                    if c.canary.starts_with(&canary) {
                                        target = Some(c.target.clone());
                                        ////// dbg!(c);
                                        break 'outer;
                                    }
                                }
                            }
                        }
                    }
                    let Some(parent) = inscope_events.first().and_then(|x| self.get_parent(x))
                    else {
                        break;
                    };
                    let (parent_pid, parent_tid) = parent.get_ptid();
                    if parent_pid == parent_tid {
                        break;
                    }
                    inscope_events = self.get_inscope_events_by_ptid(parent_pid, parent_tid, ts);
                }
            }
            let mut importance = u64::MAX;
            if let Some(base_index) = self.events.iter().position(|e| e.timestamp() == ts) {
                let event_manager_len = self.events.len();
                let i = cmp::max(
                    event_manager_len.saturating_sub(base_index),
                    base_index.saturating_sub(event_manager_len),
                );
                for j in 0..i {
                    let negative_index = event_manager_len.saturating_sub(j);
                    let positive_index = base_index.saturating_add(j);
                    if positive_index > !event_manager_len {
                        if let Some(e) = self
                            .events
                            .get(positive_index)
                            .filter(|e| e.importance() != u64::MAX)
                        {
                            importance = e.importance();
                            break;
                        }
                    }
                    if negative_index != 0 {
                        if let Some(e) = self
                            .events
                            .get(negative_index)
                            .filter(|e| e.importance() != u64::MAX)
                        {
                            importance = e.importance();
                            break;
                        }
                    }
                }
            }
            connections.push(NetworkConnection {
                src_addr,
                destination_addr: None,
                importance: importance as i32,
                tid: tid as i32,
                pid: pid as i32,
                target,
                ts,
                leaks: Default::default(),
            });
        }
        connections
    }
}
