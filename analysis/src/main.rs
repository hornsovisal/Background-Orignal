use std::{
    collections::{HashMap, HashSet},
    env,
    fs::File,
    path::Path,
};

pub mod event_manager;
// mod graph;
pub mod events;
use data::find_closest_network_connection;
use db::{
    AlarmInfo, BackgroundWork, IntentInfo, JobScheduleInfo, RuntimeRegisteredReceiver,
    ServiceMethod, Trigger,
};
use events::{Events, PendingIntentGet, SetImpl, StackTrace};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
// use regex::Regex;
mod background_work;
mod data;
mod db;

mod leak;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let Some(path) = args.get(1) else {
        return Err("Please provide the path to the data directory".into());
    };
    let Some(leak_file) = args.get(2) else {
        return Err("Please provide the path to the leak file".into());
    };
    // let db = Arc::new(db::DB::open("result.db").await?);
    let leak_item_file = File::open(leak_file)?;
    let leak_table = serde_json::from_reader::<_, HashMap<String, Vec<String>>>(leak_item_file).unwrap();

    let mut d = data::DataProvider::new(path.to_owned());
    d.load().await.unwrap();

    d.apps.par_iter().for_each(|((app, session), data)| {
        // if app != "com.instabridge.android" {
        //     return;
        // }
        let mut leak_table = leak_table.clone();
        if let Some(device_id) = &data.device_id {
            leak_table.insert("device_id".to_string(), vec![device_id.clone()]);
        }
        
        let leak_items = leak::Leaks::from(leak_table);
        let event_manager = event_manager::EventManager::new(data.events.clone());

        if !Path::new(&format!(
            "{}/{}/analysis_result/network-{}.json",
            path, app, session
        ))
        .exists()
        {
            let (mut network_connections, network_leaks) = rayon::join(
                || event_manager.get_network_connections(),
                || leak_items.extract_leaks(&data.data, data.cryptographic.clone()),
            );

            for (leaks, leak_network_info) in network_leaks {
                let candidates = network_connections
                    .iter()
                    .filter(|c| c.src_addr == leak_network_info.source_address)
                    .cloned()
                    .collect::<Vec<_>>();
                if let Some(item_index) =
                    find_closest_network_connection(leak_network_info.timestamp as u128, candidates)
                        .and_then(|closest| network_connections.iter().position(|c| c == &closest))
                {
                    if let Some(entry) = network_connections.get_mut(item_index) {
                        entry.leaks.extend(leaks);
                        entry.destination_addr = Some(leak_network_info.destination_address);
                    }
                }
            }
            println!("Network done {}:{}", app, session);
            serde_json::to_writer(
                File::create(format!(
                    "{}/{}/analysis_result/network-{}.json",
                    path, app, session
                ))
                .unwrap(),
                &network_connections,
            )
            .unwrap();
        }
        if !Path::new(&format!(
            "{}/{}/analysis_result/works-{}.json",
            path, app, session
        ))
        .exists()
        {
            let mut alarms: Vec<SetImpl> = Vec::new();
            let mut pending_intents: Vec<PendingIntentGet> = Vec::new();
            let mut work: HashSet<BackgroundWork> = HashSet::new();
            let mut foreground_service_classes: HashSet<String> = HashSet::new();
            let mut runtime_registered_receiver = HashSet::new();
            let mut job_parent_map = HashMap::new();
            let mut setter_parent_map = HashMap::new();
            let mut doworks = Vec::new();
            let mut onjobstart = Vec::new();
            let mut work_job_map = Vec::new();
            let mut job_scheduler = Vec::new();
            for e in event_manager.events {
                match e {
                    Events::ScheduleInternal(sci) => {
                        work_job_map.push((sci.job_id, sci.work_id));
                    }
                    Events::DoWork(dw) => {
                        doworks.push(dw.clone());
                    }
                    Events::OnStartJob(oj) => {
                        onjobstart.push(oj.clone());
                    }
                    Events::SetImpl(e) => {
                        alarms.push(e.clone());
                    }
                    Events::JobScheduler(ref j) => {
                        if let Some(x) = j.get_parent_work(){
                            job_parent_map.insert(j.job_id.clone(), x);
                        }
                        // 
                        job_scheduler.push(j.clone());
                    }
                    Events::PendingIntentGet(e) => {
                        pending_intents.push(e.clone());
                    }
                    Events::OnStartCommand(e) => {
                        work.insert(BackgroundWork {
                            class: e.class_name,
                            method: e.method_name,
                            ts: e.ts,
                            parent: None,
                            trigger: Some(Trigger::Service(
                                IntentInfo {
                                    action: e.action,
                                    component: e.component,
                                    data: e.data,
                                    extras: e.extras,
                                    intent_package: e.intent_package,
                                    alarm_info: None,
                                },
                                ServiceMethod::OnStartCommand(false),
                            )),
                        });
                    }
                    Events::OnBind(e) => {
                        let service_method = if e.method_name == "onBind" {
                            ServiceMethod::OnBind(false)
                        } else if e.method_name == "onRebind" {
                            ServiceMethod::OnRebind(false)
                        } else {
                            continue;
                        };
                        work.insert(BackgroundWork {
                            class: e.class_name,
                            method: e.method_name,
                            ts: e.ts,
                            parent: None,
                            trigger: Some(Trigger::Service(
                                IntentInfo {
                                    action: e.action,
                                    component: e.component,
                                    data: e.data,
                                    extras: e.extras,
                                    intent_package: e.intent_package,
                                    alarm_info: None,
                                },
                                service_method,
                            )),
                        });
                    }
                    Events::GenericHook(e) => {
                        if e.method_name == "onCreate" {
                            work.insert(BackgroundWork {
                                class: e.class_name,
                                method: e.method_name,
                                ts: e.ts,
                                parent: None,
                                trigger: None,
                            });
                        }
                    }

                    Events::OnReceiveHook(e) => {
                        work.insert(BackgroundWork {
                            class: e.class_name,
                            method: e.method_name,
                            ts: e.ts,
                            parent: None,
                            trigger: Some(Trigger::BroadcastReceiver(IntentInfo {
                                action: e.action,
                                component: e.component,
                                data: e.data,
                                extras: e.extras,
                                intent_package: e.intent_package,
                                alarm_info: None,
                            },false)),
                        });
                    }
                    Events::RegisterReceiver(ref r) => {
                        println!("{:?}", r);
                        // who_register_what.insert(r.register_class.clone());
                        // r.importance
                        runtime_registered_receiver.insert(RuntimeRegisteredReceiver {
                            class: r.register_class.clone(),
                            action: r.actions.clone(),
                            parent: r.get_parent_work(),
                            importance: r.importance,
                        });
                    }
                    Events::Intent(intent) => {
                        if let Some(parent) = (&intent).get_parent_work() {
                            let is_broadcast = match intent.method_name.as_str() {
                                "startService" | "startForegroundService" => false,
                                "bindService" | "bindServiceAsUser" | "bindIsolatedService" => {
                                    false
                                }
                                "sendBroadcast"
                                | "sendOrderedBroadcast"
                                | "sendOrderedBroadcastAsUser"
                                | "sendStickyBroadcast"
                                | "sendStickyBroadcastAsUser"
                                | "sendStickyOrderedBroadcast"
                                | "sendStickyOrderedBroadcastAsUser"
                                | "sendBroadcastAsUser"
                                | "sendBroadcastWithMultiplePermissions" => true,
                                _ => {
                                    continue;
                                }
                            };
                            setter_parent_map.insert(
                                (
                                    IntentInfo {
                                        action: intent.action,
                                        component: intent.component,
                                        data: intent.data,
                                        extras: intent.extras,
                                        intent_package: intent.intent_package,
                                        alarm_info: None,
                                    },
                                    is_broadcast,
                                ),
                                parent,
                            );
                        }
                    }
                    Events::StartForeground(ref e) => {
                        if let Some(classes) = e.get_stack_trace() {
                            for class in classes {
                                if class.starts_with("LSPHooker_") {
                                    if let Some((c, m)) = class[10..]
                                        .rsplit_once("(")
                                        .and_then(|x| x.0.rsplit_once("."))
                                    {
                                        if ["onStartCommand", "onBind", "onStart", "onRebind"]
                                            .contains(&m)
                                        {
                                            foreground_service_classes.insert(c.to_string());
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            let mut alarm_intents = HashSet::new();
            if !alarms.is_empty() {
                for p_intent in pending_intents {
                    for alarm in &alarms {
                        if alarm.operation_hashcode == p_intent.hashcode {
                            alarm_intents.insert((
                                IntentInfo {
                                    action: p_intent.action.clone(),
                                    component: p_intent.component.clone(),
                                    data: p_intent.data.clone(),
                                    extras: p_intent.extras.clone(),
                                    intent_package: p_intent.intent_package.clone(),
                                    alarm_info: Some(AlarmInfo {
                                        trigger_at_millis: alarm.trigger_at_millis,
                                        window_millis: alarm.window_millis,
                                        interval_millis: alarm.interval_millis,
                                    }),
                                },
                                p_intent.method_name == "getBroadcast",
                            ));
                        }
                    }
                }
            }

            'outer: for js in &job_scheduler {
                for (job_id, work_id) in &work_job_map {
                    if js.job_id == *job_id {
                        for dw in &doworks {
                            if dw.work_id == *work_id {
                                work.insert(BackgroundWork {
                                    class: dw.class_name.clone(),
                                    method: dw.method_name.clone(),
                                    ts: dw.ts,
                                    parent: js.get_parent_work(),
                                    trigger: Some(Trigger::Job(JobScheduleInfo {
                                        id: js.job_id,
                                        time_driven_job: js.into_time(),
                                        event_driven_job: js.into_event(),
                                    })),
                                });
                                break 'outer;
                            }
                        }
                    }
                }
                for oj in &onjobstart {
                    if js.job_id == oj.job_id {
                        work.insert(BackgroundWork {
                            class: oj.class_name.clone(),
                            method: oj.method_name.clone(),
                            ts: oj.ts,
                            parent: js.get_parent_work(),
                            trigger: Some(Trigger::Job(JobScheduleInfo {
                                id: js.job_id,
                                time_driven_job: js.into_time(),
                                event_driven_job: js.into_event(),
                            })),
                        });
                        break 'outer;
                    }
                }
            }
            // for (job_id, work_id) in work_job_map {
            //     for dw in &doworks {
            //         if dw.work_id == work_id {
            //             for js in &job_scheduler {
            //                 if js.job_id == job_id {
            //                     work.insert(BackgroundWork {
            //                         class: dw.class_name.clone(),
            //                         method: dw.method_name.clone(),
            //                         ts: dw.ts,
            //                         parent: js.get_parent_work(),
            //                         trigger: Some(Trigger::Job(JobScheduleInfo {
            //                             time_driven_job: js.into_time(),
            //                             event_driven_job: js.into_event(),
            //                         })),
            //                     });
            //                 }
            //             }
            //         }
            //     }
            // }
            // for oj in onjobstart {
            //     for js in &job_scheduler {
            //         if js.job_id == oj.job_id {
            //             work.insert(BackgroundWork {
            //                 class: oj.class_name.clone(),
            //                 method: oj.method_name.clone(),
            //                 ts: oj.ts,
            //                 parent: js.get_parent_work(),
            //                 trigger: Some(Trigger::Job(JobScheduleInfo {
            //                     time_driven_job: js.into_time(),
            //                     event_driven_job: js.into_event(),
            //                 })),
            //             });
            //         }
            //     }
            // }
            let mut work = work.into_iter().collect::<Vec<_>>();
            for w in &mut work {
                for (intent, is_broadcast) in &alarm_intents {
                    if *is_broadcast {
                        // w.trigger = Some(Trigger::BroadcastReceiver(intent.clone()));
                        if let Some(Trigger::BroadcastReceiver(trigger_intent,_)) = w.trigger.as_mut()
                        {
                            if (&trigger_intent).eq_no_alarm(intent) {
                                trigger_intent.alarm_info = intent.alarm_info.clone();
                            }
                        }
                    } else {
                        if let Some(Trigger::Service(trigger_intent, _)) = w.trigger.as_mut() {
                            if (&trigger_intent).eq_no_alarm(intent) {
                                trigger_intent.alarm_info = intent.alarm_info.clone();
                            }
                        }
                    }
                }
                for ((intent_info, is_broadcast), parent) in &setter_parent_map {
                    if *is_broadcast {
                        if let Some(Trigger::BroadcastReceiver(trigger_intent,_)) = w.trigger.as_mut()
                        {
                            if (&trigger_intent).eq_no_alarm(intent_info) {
                                w.parent = Some(parent.clone());
                            }
                        }
                    } else {
                        if let Some(Trigger::Service(trigger_intent, _)) = w.trigger.as_mut() {
                            if (&trigger_intent).eq_no_alarm(intent_info) {
                                w.parent = Some(parent.clone());
                            }
                        }
                    }
                }
                for (job_id, parent) in &job_parent_map {
                    if let Some(Trigger::Job(js)) = w.trigger.as_mut() {
                        if js.id == *job_id {
                            w.parent = Some(parent.clone());
                        }
                    }
                }
                if let Some(Trigger::Service(_, method)) = w.trigger.as_mut() {
                    if foreground_service_classes.contains(&w.class) {
                        match method {
                            ServiceMethod::OnStartCommand(_) => {
                                *method = ServiceMethod::OnStartCommand(true);
                            }
                            ServiceMethod::OnBind(_) => {
                                *method = ServiceMethod::OnBind(true);
                            }
                            ServiceMethod::OnRebind(_) => {
                                *method = ServiceMethod::OnRebind(true);
                            }
                        }
                    }
                }
                if let Some(Trigger::BroadcastReceiver(intent, _)) = w.trigger.as_mut() {
                    for r in &runtime_registered_receiver {
                        if r.class == w.class || r.class.is_empty(){
                            if r.action.contains(&intent.action) {
                                w.trigger = Some(Trigger::BroadcastReceiver(intent.clone(),true));
                                break;
                            }
                        }
                    }
                }
            }
            // runtime_registered_receiver
            drop(alarms);
            drop(alarm_intents);
            drop(setter_parent_map);

            serde_json::to_writer(
                File::create(format!(
                    "{}/{}/analysis_result/works-{}.json",
                    path, app, session
                ))
                .unwrap(),
                &work,
            )
            .unwrap();
            serde_json::to_writer(
                File::create(format!(
                    "{}/{}/analysis_result/runtime_receivers-{}.json",
                    path, app, session
                ))
                .unwrap(),
                &runtime_registered_receiver.into_iter().collect::<Vec<_>>(),
            )
            .unwrap();
        }
        // let value = db.clone();
        // let app_name = app.clone();
        // let rt  = Runtime::new().unwrap();
        // rt.block_on(async move {
        //     value.insert_network_connections(&app_name, network_connections).await.unwrap();
        // });

        // dbg!(app,&network_connections);
        // fn find_closes_time(base: NaiveDateTime, times: &[NaiveDateTime]) -> Option<NaiveDateTime> {
        //     let mut closest = None;
        //     let mut delta = TimeDelta::max_value();
        //     for t in times {
        //         let diff = t.signed_duration_since(base);
        //         if diff < delta {
        //             delta = diff;
        //             closest = Some(*t);
        //         }
        //     }
        //     closest
        // }

        // for record in network_data.records{
        //     if let Some(req) = record.request{
        //         let leaks = leak_items.contains(req.body);
        //         if leaks.len() > 0 {
        //             println!("{}: {:?}", app, leaks);
        //         }

        //     }
        // }
        /*
        let mut socket_info = HashMap::new();
        let mut canary_class = HashMap::new();
        let mut pending_service = HashMap::new();
        let mut pending_broadcast = HashMap::new();
        let mut imples = HashMap::new();
        for e in &event_manager.events {
            match e {
                Events::SocketHook(e) => {
                    if let Some(si) = e.get_local_addr().map(|s| (s, e.ts)) {
                        socket_info.insert(
                            si,
                            (
                                e.importance,
                                if e.canary.is_empty() {
                                    None
                                } else {
                                    Some(e.canary.clone())
                                },
                            ),
                        );
                    }
                }
                Events::Canary(e) => {
                    canary_class.insert(e.canary.clone(), e.get_work_name());
                }
                Events::SetImpl(e) => {
                    imples.insert(e.operation_hashcode, e.trigger_at_millis);
                }
                Events::PendingIntentGet(e) => match e.method_name.as_str() {
                    "getBroadcast" => {
                        pending_broadcast
                            .insert(e.hashcode, (e.action.clone(), e.component.clone()));
                    }
                    "getService" => {
                        pending_service.insert(e.hashcode, (e.action.clone(), e.component.clone()));
                    }
                    _ => continue,
                },
                _ => {}
            }
        }
        let mut alarm_info = HashSet::new();
        for (h, m) in imples.iter() {
            if let Some((action, component)) = pending_broadcast.remove(h) {
                alarm_info.insert((action, component, *m, true));
            }
            if let Some((action, component)) = pending_service.remove(h) {
                alarm_info.insert((action, component, *m, false));
            }
        }
        drop(imples);
        drop(pending_broadcast);
        drop(pending_service);
        let mut work_map_to_canary = HashMap::new();
        let mut socket_to_work_event = HashMap::new();
        for event in &event_manager.events {
            if let Some(e) = BackgroundWork::from_event(event) {
                let (class_name, method_name) = e.get_work_name();
                for (canary, object) in canary_class.iter() {
                    for (c, m) in object {
                        if c == &class_name && m == &method_name {
                            work_map_to_canary.insert(canary, e.clone());
                        }
                    }
                }
                continue;
            };
            let (pid, tid) = event.get_ptid();
            if pid == tid {
                continue;
            }
            let mut related_work = None;
            let mut canary_info = None;
            let mut importance = u64::MAX;
            let Some(si) = (match event {
                Events::SocketAddrV4(e) => e.get_local_addr().map(|s| (s, e.ts)),
                Events::SocketAddrV6(e) => e.get_local_addr().map(|s| (s, e.ts)),
                _ => {
                    continue;
                }
            }) else {
                continue;
            };
            let mut inscope = event_manager.get_inscope_events_by_ptid(event);
            'outer: loop {
                if inscope.is_empty() {
                    break;
                }
                for e in &inscope {
                    if let Some(rw) = BackgroundWork::from_event(e) {
                        related_work = Some(rw);
                        importance = e.importance();
                        break 'outer;
                    }
                    canary_info = e.get_stacktrace().extract_canary_info().map(|c| {
                        format!(
                            "{}{:_>15}",
                            c.0.unwrap_or("________".to_string()),
                            format!("{}_{}", c.1, c.2)
                        )
                    });
                    importance = e.importance();
                    if canary_info.is_some() {
                        break 'outer;
                    }
                }
                // StackTracer_2MtIGmjU____32437_32533.run(StackTracer_2MtIGmjU____32437_32533.java:11)
                let Some(parent) = inscope.first().and_then(|x| event_manager.get_parent(x)) else {
                    break;
                };
                let (parent_pid, parent_tid) = parent.get_ptid();
                if parent_pid == parent_tid {
                    break;
                }
                inscope = event_manager.get_inscope_events_by_ptid(&parent);
            }
            socket_info.entry(si).or_insert((importance, canary_info));
            if let Some(rw) = related_work {
                socket_to_work_event.insert(si, rw);
            }
        }
        for (si, (imp, canary)) in socket_info {
            let Some(c) = canary else {
                continue;
            };
            // dbg!(&c);
            // for x in &work_map_to_canary{
            //     dbg!(&x);
            //     break;
            // }
            let Some(o) = work_map_to_canary.get(&c) else {
                continue;
            };

            // dbg!(o);
        }*/
        // for ((pid, tid), node) in &data.flows {
        //     for e in &node.events {
        //         match e {
        //             events::Events::SocketHook(_)|events::Events::SocketAddrV4(_)|events::Events::SocketAddrV6(_) => {
        //                 let mut earlier_events = node
        //                     .events
        //                     .iter()
        //                     // .filter(|p| s.ts > p.timestamp())
        //                     .collect::<Vec<&Events>>();
        //                 let mut parent = node.parent.clone();
        //                 let mut pending_parents = Vec::new();
        //                 'outer: loop {

        //                     if let Some((pending_pid, pending_tid, pending_ts)) =
        //                         pending_parents.pop()
        //                     {
        //                         let Some(new_node) = data.flows.get(&(pending_pid, pending_tid))
        //                         else {
        //                             continue;
        //                         };
        //                         earlier_events = new_node
        //                             .events
        //                             .iter()
        //                             // .filter(|p| s.ts > p.timestamp())
        //                             .collect::<Vec<&Events>>();
        //                         parent = new_node.parent.clone();
        //                     }

        //                     for e in &earlier_events {
        //                         match e {
        //                             Events::DoWork(e) => {
        //                                 if e.pid != e.tid {
        //                                     println!("DoWork = {}, tid = {}", app, tid);
        //                                     break 'outer;
        //                                 }
        //                             }
        //                             Events::OnStartJob(e) => {
        //                                 if e.pid != e.tid {
        //                                     println!("OnStartJob = {}, tid = {}", app, tid);
        //                                     break 'outer;
        //                                 }
        //                             }
        //                             Events::OnBind(e) => {
        //                                 if e.pid != e.tid {
        //                                     println!("OnBind = {}, tid = {}", app, tid);
        //                                     break 'outer;
        //                                 }
        //                             }
        //                             Events::OnReceiveHook(e) => {
        //                                 if e.pid != e.tid {
        //                                     println!("OnReceiveHook = {}, tid = {}", app, tid);
        //                                     break 'outer;
        //                                 }
        //                             }
        //                             Events::OnStartCommand(e) => {
        //                                 if e.pid != e.tid {
        //                                     println!("OnStartCommand = {}, tid = {}", app, tid);
        //                                     break 'outer;
        //                                 }
        //                             }
        //                             _ => {}
        //                         }
        //                     }

        //                     for n in parent.clone() {
        //                         pending_parents.push(n);
        //                     }
        //                     // dbg!(&node.parent, &parent, &pending_parents);
        //                     if parent.is_empty() && pending_parents.is_empty() {
        //                         break;
        //                     }

        //                 }
        //             }
        //             _ => {}
        //         }
        //     }
        // }
    });

    // let mut file = File::open("/Users/sajjad/i/archive/com.sec.android.app.sbrowser/analysis_result/network-1723344124.pb")?;
    // let mut buf = Vec::new();
    // file.read_to_end(&mut buf)?;

    // Deserialize the buffer into a Person struct
    // let person = data::NetworkData::decode(buf.as_slice())?;
    // dbg!(person.records.first());

    // /Users/sajjad/i/archive/com.sec.android.app.sbrowser/analysis_result/network-1723344124.pb
    println!("Hello, world!");
    Ok(())
}
