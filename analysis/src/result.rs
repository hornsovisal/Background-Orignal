use std::{
    collections::{HashMap, HashSet},
    env,
};

use db::{
    BackgroundWork, Leak, NetworkConnection, RuntimeRegisteredReceiver, ServiceMethod, Trigger,
};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
mod db;
mod system_intents;
const LEAK_ITEMS: [(&str, &str); 29] = [
    // ("imei", "IMEI"),
    // ("serial", "Device Serial"),
    ("device_id", "Device ID"),
    ("adid", "Advertising ID"),
    ("bootloader", "Bootloader"),
    ("finger_print", "Fingerprint"),
    ("cpu", "CPU Model"),
    ("displayid", "Display ID"),
    ("device_name", "Device Name"),
    ("resolution", "Device Resolution"),
    ("abi", "Device ABI"),
    ("model", "Device Model"),
    ("timezone", "Device Timezone"),
    ("operator", "Operator"),
    // ("iccid", "ICCID"),
    // ("imsi", "IMSI"),
    ("wifi_ip", "Device WiFi IP"),
    ("wifi_ipv6", "Device WiFi IPv6"),
    ("gateway", "Default Gateway IP"),
    ("proxy_addr", "On Device Proxy Address"),
    ("my_wifi_essid", "Router ESSID"),
    ("my_wifi_bssid", "Router BSSID"),
    ("other_wifi_essid", "Neighbor Router ESSID"),
    ("other_wifi_bssid", "Neighbor Router BSSID"),
    ("gps4", "GPS ($\\leq$7 meter accuracy)"),
    ("gps3", "GPS (78 meter accuracy)"),
    ("gps2", "GPS (787 meter accuracy)"),
    ("device_apps", "List of Device Apps"),
    ("phone_number", "Phone Number"),
    ("call_history_number", "Call History Number"),
    ("contact_name", "Contacts Name"),
    ("contact_phone", "Contacts Number"),
    ("email", "Device Email"),
];

const DEVICE_INFO: [&str; 13] = [
    "imei",
    "serial",
    "device_id",
    "adid",
    "bootloader",
    "finger_print",
    "cpu",
    "displayid",
    "device_name",
    "resolution",
    "abi",
    "model",
    "timezone",
];

const NETWORK_INFO: [&str; 7] = [
    "operator",
    "iccid",
    "imsi",
    "wifi_ip",
    "wifi_ipv6",
    "gateway",
    "proxy_addr",
];

const NETWORK_LOCATION_INFO: [&str; 4] = [
    "my_wifi_essid",
    "my_wifi_bssid",
    "other_wifi_essid",
    "other_wifi_bssid",
];

const LOCATION_INFO: [&str; 3] = ["gps4", "gps3", "gps2"];

const USER_ASSETS: [&str; 6] = [
    "device_apps",
    "phone_number",
    "call_history_number",
    "contact_name",
    "contact_phone",
    "email",
];

const NEW_IMPLICIT: [&str; 13] = [
    "android.app.action.APP_BLOCK_STATE_CHANGED",
    "android.hardware.usb.action.USB_STATE",
    "android.intent.action.ACTION_DEFAULT_DATA_SUBSCRIPTION_CHANGED",
    "android.intent.action.ACTION_DEFAULT_VOICE_SUBSCRIPTION_CHANGED",
    "android.intent.action.MY_PACKAGE_REPLACED",
    "android.intent.action.SERVICE_STATE",
    "android.intent.action.SIM_STATE_CHANGED",
    "android.intent.action.SUBSCRIPTION_PHONE_STATE",
    "android.intent.action.UID_REMOVED",
    "android.telephony.action.DEFAULT_SMS_SUBSCRIPTION_CHANGED",
    "android.telephony.action.DEFAULT_SUBSCRIPTION_CHANGED",
    "com.google.android.checkin.CHECKIN_COMPLETE",
    "com.google.gservices.intent.action.GSERVICES_CHANGED",
];
const IMPLICIT: [&str; 38] = [
    "android.intent.action.LOCKED_BOOT_COMPLETED",
    "android.intent.action.BOOT_COMPLETED",
    "android.intent.action.USER_INITIALIZE",
    "android.intent.action.USER_ADDED",
    "android.intent.action.USER_REMOVED",
    "android.intent.action.TIME_SET",
    "android.intent.action.TIMEZONE_CHANGED",
    "android.app.action.NEXT_ALARM_CLOCK_CHANGED",
    "TelephonyIntents.SECRET_CODE_ACTION",
    "TelephonyIntents.ACTION_*_SUBSCRIPTION_CHANGED",
    "android.intent.action.LOCALE_CHANGED",
    "android.hardware.usb.action.USB_ACCESSORY_ATTACHED",
    "android.hardware.usb.action.USB_ACCESSORY_DETACHED",
    "android.hardware.usb.action.USB_DEVICE_ATTACHED",
    "android.hardware.usb.action.USB_DEVICE_DETACHED",
    "android.bluetooth.headset.profile.action.CONNECTION_STATE_CHANGED",
    "android.bluetooth.a2dp.profile.action.CONNECTION_STATE_CHANGED",
    "android.bluetooth.device.action.ACL_DISCONNECTED",
    "android.bluetooth.device.action.ACL_CONNECTED",
    "android.telephony.action.CARRIER_CONFIG_CHANGED",
    "android.intent.action.PHONE_STATE",
    "android.telecom.action.PHONE_ACCOUNT_REGISTERED",
    "android.telecom.action.PHONE_ACCOUNT_UNREGISTERED",
    "android.accounts.action.ACCOUNT_REMOVED",
    "android.intent.action.PACKAGE_DATA_CLEARED",
    "android.intent.action.PACKAGE_FULLY_REMOVED",
    "android.intent.action.NEW_OUTGOING_CALL",
    "android.app.action.DEVICE_OWNER_CHANGED",
    "android.intent.action.EVENT_REMINDER",
    "android.intent.action.MEDIA_MOUNTED",
    "android.intent.action.MEDIA_CHECKING",
    "android.intent.action.MEDIA_UNMOUNTED",
    "android.intent.action.MEDIA_EJECT",
    "android.intent.action.MEDIA_BAD_REMOVAL",
    "android.intent.action.MEDIA_REMOVED",
    "android.intent.action.MEDIA_UNMOUNTABLE",
    "android.provider.Telephony.SMS_RECEIVED",
    "android.provider.Telephony.WAP_PUSH_RECEIVED",
];

fn get_leak_category(leak: &Leak) -> &'static str {
    let value = leak.leak.as_str();
    if DEVICE_INFO.contains(&value) {
        "Device"
    } else if NETWORK_INFO.contains(&value) {
        "Network"
    } else if NETWORK_LOCATION_INFO.contains(&value)
        || [
            "latitude4",
            "langitude4",
            "latitude3",
            "langitude3",
            "latitude2",
            "langitude2",
        ]
        .contains(&value)
    {
        "Location"
    } else if USER_ASSETS.contains(&value) {
        "User Assets"
    } else {
        "Other"
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let Some(path) = args.get(1) else {
        return Err("Please provide the path to the data directory".into());
    };
    let cat = args.get(2);
    // let mut connections: HashMap<String, Vec<NetworkConnection>> = HashMap::new();
    // let mut works: HashMap<String, Vec<BackgroundWork>> = HashMap::new();
    // let mut runtime_registered: HashMap<_, Vec<>> = HashMap::new();
    let mut result_data: HashMap<String, ResultData> = HashMap::new();
    let mut dirs = tokio::fs::read_dir(path).await?;

    while let Ok(Some(entity)) = dirs.next_entry().await {
        if entity
            .metadata()
            .await
            .ok()
            .filter(|p| p.is_dir())
            .is_some()
        {
            if let Some(c) = cat.and_then(|c| entity.file_name().to_str().map(|s| s.ends_with(c))) {
                if !c {
                    continue;
                }
            }
            let mut files = tokio::fs::read_dir(entity.path()).await?;
            while let Ok(Some(file)) = files.next_entry().await {
                if file.metadata().await.ok().filter(|p| p.is_dir()).is_some() {
                    let Some(app_name) = file.file_name().to_str().map(|s| s.to_string()) else {
                        continue;
                    };
                    let mut app_log_files_path = file.path().clone();
                    app_log_files_path.push("analysis_result");
                    let mut app_log_files_path = tokio::fs::read_dir(app_log_files_path).await?;
                    while let Ok(Some(file)) = app_log_files_path.next_entry().await {
                        if let Some(f) = file.file_name().to_str() {
                            if f.ends_with(".json") && f.starts_with("network-") {
                                let file = tokio::fs::read_to_string(file.path()).await?;
                                let app_connections: Vec<NetworkConnection> =
                                    serde_json::from_str(&file)?;
                                result_data
                                    .entry(app_name.to_string())
                                    .and_modify(|v| {
                                        v.connections.extend(app_connections.clone());
                                    })
                                    .or_insert(ResultData {
                                        connections: app_connections.clone(),
                                        ..Default::default()
                                    });
                            } else if f.ends_with(".json") && f.starts_with("works-") {
                                let file_content = tokio::fs::read_to_string(file.path()).await?;
                                let app_works = match serde_json::from_str::<Vec<BackgroundWork>>(
                                    &file_content,
                                ) {
                                    Ok(app_work) => app_work,
                                    Err(e) => {
                                        println!("Failed to parse works file: {} - {:?}", e, file);
                                        continue;
                                    }
                                };
                                result_data
                                    .entry(app_name.to_string())
                                    .and_modify(|v| {
                                        v.works.extend(app_works.clone());
                                    })
                                    .or_insert(ResultData {
                                        works: app_works.clone(),
                                        ..Default::default()
                                    });
                            } else if f.ends_with(".json") && f.starts_with("runtime_receivers-") {
                                let file_content = tokio::fs::read_to_string(file.path()).await?;
                                let app_runtime_registered =
                                    match serde_json::from_str::<Vec<RuntimeRegisteredReceiver>>(
                                        &file_content,
                                    ) {
                                        Ok(app_runtime_registered) => app_runtime_registered,
                                        Err(e) => {
                                            println!(
                                                "Failed to parse runtime_receivers file: {} - {:?}",
                                                e, file
                                            );
                                            continue;
                                        }
                                    };
                                result_data
                                    .entry(app_name.to_string())
                                    .and_modify(|v| {
                                        v.runtime_registered.extend(app_runtime_registered.clone());
                                    })
                                    .or_insert(ResultData {
                                        runtime_registered: app_runtime_registered.clone(),
                                        ..Default::default()
                                    });
                            }
                        }
                    }
                }
            }
        }
    }
    let mut apps_establishing_connection: HashSet<String> = HashSet::new();
    let mut apps_establishing_connection_visible: HashSet<String> = HashSet::new();
    let mut apps_establishing_connection_invisible: HashSet<String> = HashSet::new();
    let mut apps_establishing_connection_broadcasts: HashSet<String> = HashSet::new();
    let mut apps_establishing_connection_services: HashSet<String> = HashSet::new();
    let mut any_overal_any_plus_invisible: HashSet<String> = HashSet::new();
    let mut apps_explicit_intent_from_implicit: HashSet<String> = HashSet::new();
    let mut apps_explicit_intent_from_any: HashSet<String> = HashSet::new();
    let mut apps_explicit_intent_from_service: HashSet<String> = HashSet::new();
    let mut apps_explicit_intent_from_jobs: HashSet<String> = HashSet::new();

    let mut domains_apps_visible: HashMap<String, HashSet<String>> = HashMap::new();
    let mut domains_apps_invisible: HashMap<String, HashSet<String>> = HashMap::new();
    let mut domains_apps_works: HashMap<String, HashSet<String>> = HashMap::new();
    let mut domains_apps_any: HashMap<String, HashSet<String>> = HashMap::new();

    let mut apps_successfuly_established_at_least_one_conenction: HashSet<String> = HashSet::new();
    // return Ok(());
    let mut visible_overal_leaks: HashMap<String, HashSet<String>> = HashMap::new();
    let mut visible_encrypted_leaks: HashMap<String, HashSet<String>> = HashMap::new();
    let mut visible_plain_leaks: HashMap<String, HashSet<String>> = HashMap::new();

    let mut invisible_overal_leaks: HashMap<String, HashSet<String>> = HashMap::new();
    let mut invisible_encrypted_leaks: HashMap<String, HashSet<String>> = HashMap::new();
    let mut invisible_plain_leaks: HashMap<String, HashSet<String>> = HashMap::new();

    let mut background_overal_leaks: HashMap<String, HashSet<String>> = HashMap::new();
    let mut background_encrypted_leaks: HashMap<String, HashSet<String>> = HashMap::new();
    let mut background_plain_leaks: HashMap<String, HashSet<String>> = HashMap::new();

    let mut overall_overal_leaks: HashMap<String, HashSet<String>> = HashMap::new();
    let mut overall_encrypted_leaks: HashMap<String, HashSet<String>> = HashMap::new();
    let mut overall_plain_leaks: HashMap<String, HashSet<String>> = HashMap::new();
    /*
    let mut persistent_overal_leaks: HashMap<String, HashSet<String>> = HashMap::new();
    let mut persistent_encrypted_leaks: HashMap<String, HashSet<String>> = HashMap::new();
    let mut persistent_plain_leaks: HashMap<String, HashSet<String>> = HashMap::new();

    let mut event_overal_leaks: HashMap<String, HashSet<String>> = HashMap::new();
    let mut event_encrypted_leaks: HashMap<String, HashSet<String>> = HashMap::new();
    let mut event_plain_leaks: HashMap<String, HashSet<String>> = HashMap::new();

    let mut time_overal_leaks: HashMap<String, HashSet<String>> = HashMap::new();
    let mut time_encrypted_leaks: HashMap<String, HashSet<String>> = HashMap::new();
    let mut time_plain_leaks: HashMap<String, HashSet<String>> = HashMap::new();

    let mut event_time_overal_leaks: HashMap<String, HashSet<String>> = HashMap::new();
    let mut event_time_encrypted_leaks: HashMap<String, HashSet<String>> = HashMap::new();
    let mut event_time_plain_leaks: HashMap<String, HashSet<String>> = HashMap::new();

    let mut attributed_overal_leaks: HashMap<String, HashSet<String>> = HashMap::new();
    let mut attributed_encrypted_leaks: HashMap<String, HashSet<String>> = HashMap::new();
    let mut attributed_plain_leaks: HashMap<String, HashSet<String>> = HashMap::new();
    */
    let mut visible_foreground: HashSet<(String, bool)> = HashSet::new();
    let mut invisible_foreground: HashSet<(String, bool)> = HashSet::new();
    let mut visible_background: HashSet<(String, bool)> = HashSet::new();
    let mut invisible_background: HashSet<(String, bool)> = HashSet::new();
    let mut visible_works: HashSet<(String, bool)> = HashSet::new();
    let mut invisible_works: HashSet<(String, bool)> = HashSet::new();
    let mut visible_explicit_background: HashSet<(String, bool)> = HashSet::new();
    let mut invisible_explicit_background: HashSet<(String, bool)> = HashSet::new();
    let mut visible_implicit_background: HashSet<(String, bool)> = HashSet::new();
    let mut invisible_implicit_background: HashSet<(String, bool)> = HashSet::new();
    let mut system_broadcasts: HashSet<String> = HashSet::new();
    let mut jobs_table: HashMap<&str, [HashSet<String>; 9]> = HashMap::new();
    let mut receivers_table: HashMap<String, HashSet<(String, String)>> = HashMap::new();
    let mut work_parent: HashMap<&str, [HashSet<String>; 4]> = HashMap::new();
    let mut parents_same_parent = HashSet::new();

    let mut job_network_leak: HashMap<String, HashSet<String>> = HashMap::new();
    let mut job_battery_leak: HashMap<String, HashSet<String>> = HashMap::new();
    let mut job_charging_leak: HashMap<String, HashSet<String>> = HashMap::new();
    let mut job_device_idel_leak: HashMap<String, HashSet<String>> = HashMap::new();
    let mut job_storage_leak: HashMap<String, HashSet<String>> = HashMap::new();
    let mut job_expediate_leak: HashMap<String, HashSet<String>> = HashMap::new();
    let mut job_important_leak: HashMap<String, HashSet<String>> = HashMap::new();
    // let mut job_overal_leak: HashMap<String, HashSet<String>> = HashMap::new();

    let mut broadcast_explicit_leak: HashMap<String, HashSet<String>> = HashMap::new();
    let mut broadcast_implicit_leak: HashMap<String, HashSet<String>> = HashMap::new();
    let mut broadcast_undocumented_implicit_leak: HashMap<String, HashSet<String>> = HashMap::new();

    let mut service_foreground_leak: HashMap<String, HashSet<String>> = HashMap::new();
    let mut service_background_leak: HashMap<String, HashSet<String>> = HashMap::new();

    // let mut on_create_classes: HashSet<String> = HashSet::new();
    let mut leak_items = HashSet::new();

    for (app_name, data) in result_data {
        let mut foreground_classes = HashSet::new();
        // for w in &data.works {
        //     if let Some(Trigger::Job(j)) = &w.trigger {
        //         if let Some(t) = &j.time_driven_job {
        //             println!("time slot {}", t.time_slot());
        //         }
        //     }
        // }
        for connection in data.connections {
            apps_establishing_connection.insert(app_name.clone());
            if connection.destination_addr.is_some() {
                apps_successfuly_established_at_least_one_conenction.insert(app_name.clone());
                domains_apps_any
                    .entry(connection.destination_addr.clone().unwrap())
                    .or_insert(HashSet::new())
                    .insert(app_name.clone());
            }
            // if app_name == "com.xenstudio.love.photoframes" && connection.target.is_some() {
            //     dbg!(&connection.target);

            //     for work in &data.works {
            //         // dbg!(&target);
            //         dbg!(&format!("{}.{}", work.class, work.method));
            //         // if connection.target == Some(format!("{}.{}", work.class, work.method)) {
            //         //     dbg!(&app_name);
            //         //     break;
            //         // }
            //     }
            // }
            let visibility = match connection.importance {
                100 | 200 => {
                    apps_establishing_connection_visible.insert(app_name.clone());
                    if connection.destination_addr.is_some() {
                        if connection.target.is_some() {
                            domains_apps_works
                                .entry(connection.destination_addr.clone().unwrap())
                                .or_insert(HashSet::new())
                                .insert(app_name.clone());
                        } else {
                            domains_apps_visible
                                .entry(connection.destination_addr.clone().unwrap())
                                .or_insert(HashSet::new())
                                .insert(app_name.clone());
                        }
                    }
                    Some(true)
                }
                400 | 125 | 230 | 300 | 325 => {
                    apps_establishing_connection_invisible.insert(app_name.clone());
                    if connection.destination_addr.is_some() {
                        if connection.target.is_some() {
                            domains_apps_works
                                .entry(connection.destination_addr.clone().unwrap())
                                .or_insert(HashSet::new())
                                .insert(app_name.clone());
                        } else {
                            domains_apps_invisible
                                .entry(connection.destination_addr.clone().unwrap())
                                .or_insert(HashSet::new())
                                .insert(app_name.clone());
                        }
                    }
                    Some(false)
                }
                _ => None,
            };
            if let (Some(visible), Some(target)) = (visibility, &connection.target) {
                let has_dst = connection.destination_addr.is_some();

                // if has_dst == false {
                //     dbg!(&app_name, &target, &connection.src_addr);
                // }
                if let Some((class, method)) = target.rsplit_once(".") {
                    match method {
                        "onCreate" | "onStartCommand" | "onBind" | "onRebind" => {
                            for bw in data
                                .works
                                .par_iter()
                                .filter(|w| w.class == class)
                                .collect::<Vec<_>>()
                            {
                                if let Some((_class, _method)) = &bw.parent {
                                    if class == _class {
                                        // dbg!(&app_name, &class, &method);
                                        parents_same_parent.insert((
                                            app_name.clone(),
                                            class.to_string(),
                                            method.to_string(),
                                            _method.to_string(),
                                        ));
                                    }
                                    match _method.as_str() {
                                        "onCreate" | "onStartCommand" | "onBind" | "onRebind" => {
                                            work_parent
                                                .entry("Service")
                                                .or_insert(Default::default())[0]
                                                .insert(app_name.clone());
                                            work_parent
                                                .entry("Overal")
                                                .or_insert(Default::default())[0]
                                                .insert(app_name.clone());
                                        }
                                        "onReceive" => {
                                            work_parent
                                                .entry("Service")
                                                .or_insert(Default::default())[1]
                                                .insert(app_name.clone());
                                            work_parent
                                                .entry("Overal")
                                                .or_insert(Default::default())[1]
                                                .insert(app_name.clone());
                                        }
                                        "doWork" | "onStartJob" => {
                                            work_parent
                                                .entry("Service")
                                                .or_insert(Default::default())[2]
                                                .insert(app_name.clone());
                                            work_parent
                                                .entry("Overal")
                                                .or_insert(Default::default())[2]
                                                .insert(app_name.clone());
                                        }
                                        _ => {}
                                    }
                                    work_parent.entry("Service").or_insert(Default::default())[3]
                                        .insert(app_name.clone());
                                    work_parent.entry("Overal").or_insert(Default::default())[3]
                                        .insert(app_name.clone());
                                }
                                let foreground = match &bw.trigger {
                                    Some(Trigger::Service(_, ServiceMethod::OnRebind(t))) => t,
                                    Some(Trigger::Service(_, ServiceMethod::OnStartCommand(t))) => {
                                        t
                                    }
                                    Some(Trigger::Service(_, ServiceMethod::OnBind(t))) => t,

                                    _ => {
                                        continue;
                                    }
                                };
                                if *foreground {
                                    foreground_classes.insert(class.to_string());
                                    println!(
                                        "{} - {:?} - {:?} - {:?} ",
                                        app_name,
                                        class,
                                        connection.destination_addr,
                                        connection.leaks
                                    );
                                }

                                for l in &connection.leaks {
                                    let leak_type = get_leak_category(&l);
                                    if leak_type == "Other" {
                                        continue;
                                    }
                                    if *foreground {
                                        service_foreground_leak
                                            .entry(leak_type.to_string())
                                            .and_modify(|leak| {
                                                leak.insert(app_name.clone());
                                            })
                                            .or_insert_with(|| {
                                                let mut set = HashSet::new();
                                                set.insert(app_name.clone());
                                                set
                                            });
                                    } else {
                                        service_background_leak
                                            .entry(leak_type.to_string())
                                            .and_modify(|leak| {
                                                leak.insert(app_name.clone());
                                            })
                                            .or_insert_with(|| {
                                                let mut set = HashSet::new();
                                                set.insert(app_name.clone());
                                                set
                                            });
                                    };

                                    // let leak_entry = service_leak_entry
                                    //     .or_insert(HashSet::new())
                                    //     .entry(leak_type.to_string())
                                    //     .or_insert(HashSet::new());
                                    // leak_entry.insert(l.leak.clone());
                                }

                                match (visible, foreground) {
                                    (true, true) => {
                                        visible_foreground.insert((app_name.clone(), has_dst));
                                    }
                                    (true, false) => {
                                        visible_background.insert((app_name.clone(), has_dst));
                                    }
                                    (false, true) => {
                                        invisible_foreground.insert((app_name.clone(), has_dst));
                                    }
                                    (false, false) => {
                                        invisible_background.insert((app_name.clone(), has_dst));
                                    }
                                }
                            }
                        }
                        "onReceive" => {
                            for bw in data
                                .works
                                .par_iter()
                                .filter(|w| &format!("{}.{}", w.class, w.method) == target)
                                // .filter_map(|f| f.trigger.clone())
                                .collect::<Vec<_>>()
                            {
                                if let Some((_, method)) = &bw.parent {
                                    match method.as_str() {
                                        "onCreate" | "onStartCommand" | "onBind" | "onRebind" => {
                                            work_parent
                                                .entry("Broadcasts")
                                                .or_insert(Default::default())[0]
                                                .insert(app_name.clone());
                                            work_parent
                                                .entry("Overal")
                                                .or_insert(Default::default())[0]
                                                .insert(app_name.clone());
                                        }
                                        "onReceive" => {
                                            work_parent
                                                .entry("Broadcasts")
                                                .or_insert(Default::default())[1]
                                                .insert(app_name.clone());
                                            work_parent
                                                .entry("Overal")
                                                .or_insert(Default::default())[1]
                                                .insert(app_name.clone());
                                        }
                                        "doWork" | "onStartJob" => {
                                            work_parent
                                                .entry("Broadcasts")
                                                .or_insert(Default::default())[2]
                                                .insert(app_name.clone());
                                            work_parent
                                                .entry("Overal")
                                                .or_insert(Default::default())[2]
                                                .insert(app_name.clone());
                                        }
                                        _ => {}
                                    }
                                    work_parent
                                        .entry("Broadcasts")
                                        .or_insert(Default::default())[3]
                                        .insert(app_name.clone());
                                    work_parent.entry("Overal").or_insert(Default::default())[3]
                                        .insert(app_name.clone());
                                }
                                if let Some(Trigger::BroadcastReceiver(ref b, _)) = &bw.trigger {
                                    receivers_table
                                        .entry(b.action.clone())
                                        .or_insert(HashSet::new())
                                        .insert((app_name.clone(), class.to_string()));
                                }
                            }
                            for intent in data
                                .works
                                .par_iter()
                                .filter_map(|w| {
                                    if w.class == class {
                                        w.trigger.as_ref().and_then(|m| match m {
                                            Trigger::BroadcastReceiver(t, _) => {
                                                // if let Some(p) = &w.parent{
                                                //     dbg!(&p);
                                                // }
                                                Some(t)
                                            }
                                            _ => None,
                                        })
                                    } else {
                                        None
                                    }
                                })
                                .collect::<Vec<_>>()
                            {
                                for l in &connection.leaks {
                                    let leak_type = get_leak_category(&l);
                                    if leak_type == "Other" {
                                        continue;
                                    }
                                    if NEW_IMPLICIT.contains(&intent.action.as_str()) {
                                        broadcast_undocumented_implicit_leak
                                            .entry(leak_type.to_string())
                                            .and_modify(|leak| {
                                                leak.insert(app_name.clone());
                                            })
                                            .or_insert_with(|| {
                                                let mut set = HashSet::new();
                                                set.insert(app_name.clone());
                                                set
                                            });
                                    } else if IMPLICIT.contains(&intent.action.as_str()) {
                                        broadcast_implicit_leak
                                            .entry(leak_type.to_string())
                                            .and_modify(|leak| {
                                                leak.insert(app_name.clone());
                                            })
                                            .or_insert_with(|| {
                                                let mut set = HashSet::new();
                                                set.insert(app_name.clone());
                                                set
                                            });
                                    } else {
                                        broadcast_explicit_leak
                                            .entry(leak_type.to_string())
                                            .and_modify(|leak| {
                                                leak.insert(app_name.clone());
                                            })
                                            .or_insert_with(|| {
                                                let mut set = HashSet::new();
                                                set.insert(app_name.clone());
                                                set
                                            });
                                    }
                                }

                                if NEW_IMPLICIT.contains(&intent.action.as_str())
                                    || IMPLICIT.contains(&intent.action.as_str())
                                // intent
                                //     .action
                                //     .as_str()
                                //     .starts_with("com.google.gservices.intent.action.GSERVICES_CHANGED")
                                // .starts_with("android.intent.action.PACKAGE")
                                // && 1==2
                                {
                                    // if !connection.leaks.is_empty() {
                                    //     println!(
                                    //         "{} - {} - {:?} - {:?} ",
                                    //         app_name,
                                    //         intent.action.as_str(),
                                    //         connection.destination_addr,
                                    //         connection.leaks
                                    //     );
                                    //     // dbg!(
                                    //     //     &app_name,
                                    //     //     &connection.destination_addr,
                                    //     //     &connection.leaks
                                    //     // );
                                    // }

                                    // if let Some(ref t) = connection.destination_addr {
                                    //     println!(
                                    //         "{} - {} - {} - {} ",
                                    //         app_name,
                                    //         intent.action.as_str(),
                                    //         t,
                                    //         t
                                    //     );
                                    // } else {
                                    //     println!(
                                    //         "{} - {} - {} - {} ",
                                    //         app_name,
                                    //         intent.action.as_str(),
                                    //         target,
                                    //         connection.src_addr
                                    //     );
                                    // }
                                }
                                // for c in &connection.leaks {
                                //     if c.leak.starts_with("proxy_addr")  {
                                //         println!("{} - {} - {} - {} ", app_name, intent.action.as_str(), target, connection.src_addr);
                                //     }

                                // }
                                if NEW_IMPLICIT.contains(&intent.action.as_str())
                                    || IMPLICIT.contains(&intent.action.as_str())
                                {
                                    if visible {
                                        visible_implicit_background
                                            .insert((app_name.clone(), has_dst));
                                    } else {
                                        invisible_implicit_background
                                            .insert((app_name.clone(), has_dst));
                                    }
                                } else {
                                    if has_dst
                                        && system_intents::SYSTEM_INTENTS
                                            .contains(&intent.action.as_str())
                                    {
                                        system_broadcasts.insert(app_name.clone());
                                    }
                                    for r in &data.runtime_registered {
                                        let Some((parent_class, parent_method)) = &r.parent else {
                                            continue;
                                        };
                                        let class_cond = (!r.class.is_empty() && r.class == class)
                                            || r.class.is_empty();

                                        if r.action.iter().any(|a| a == &intent.action)
                                            && class_cond
                                        {
                                            apps_explicit_intent_from_any.insert(app_name.clone());
                                            match parent_method.as_str() {
                                                "onCreate" | "onStartCommand" | "onBind"
                                                | "onRebind" => {
                                                    apps_explicit_intent_from_service
                                                        .insert(app_name.clone());
                                                }
                                                "onReceive" => {
                                                    for w in data
                                                        .works
                                                        .iter()
                                                        .filter(|w| &w.class == parent_class)
                                                    {
                                                        if let Some(Trigger::BroadcastReceiver(
                                                            ref b,
                                                            _,
                                                        )) = &w.trigger
                                                        {
                                                            if NEW_IMPLICIT
                                                                .contains(&b.action.as_str())
                                                                || IMPLICIT
                                                                    .contains(&b.action.as_str())
                                                            {
                                                            }
                                                            apps_explicit_intent_from_implicit
                                                                .insert(app_name.clone());
                                                        }
                                                    }
                                                }
                                                "doWork" | "onStartJob" => {
                                                    apps_explicit_intent_from_jobs
                                                        .insert(app_name.clone());
                                                }
                                                _ => {}
                                            }
                                            // if parent_method == "onReceive" {
                                            //     // dbg!(&app_name, &r.class, &intent.action);

                                            //     // dbg!(&app_name, &r.class, &intent.action);
                                            // }
                                        }
                                    }
                                    if visible {
                                        visible_explicit_background
                                            .insert((app_name.clone(), has_dst));
                                    } else {
                                        invisible_explicit_background
                                            .insert((app_name.clone(), has_dst));
                                    }
                                }
                            }
                        }
                        // pub required_network_capabilities: Vec<i32>, // network capabilities required to run the job
                        // pub required_network_transport_types: Vec<i32>, // network transport types required to run the job
                        // pub is_require_battery_not_low: bool,           // is battery not low
                        // pub is_require_charging: bool,                  // is charging
                        // pub is_require_device_idle: bool,               // is device idle
                        // pub is_require_storage_not_low: bool,           // is storage not low
                        // pub trigger_content_max_delay: i64,             // maximum delay before running the job
                        // pub trigger_content_update_delay: i64,          // delay before running the job
                        // pub trigger_content_uris: String,               // content uris
                        // pub is_expedited: bool,                         // is expedited
                        // pub is_important_while_foreground: bool,        // is important while foreground
                        "doWork" | "onStartJob" => {
                            for bw in data
                                .works
                                .par_iter()
                                .filter(|w| &format!("{}.{}", w.class, w.method) == target)
                                // .filter_map(|f| f.trigger.clone())
                                .collect::<Vec<_>>()
                            {
                                if let Some((_, method)) = &bw.parent {
                                    // let row = work_parent.entry("Jobs").or_insert(Default::default());
                                    match method.as_str() {
                                        "onCreate" | "onStartCommand" | "onBind" | "onRebind" => {
                                            work_parent.entry("Jobs").or_insert(Default::default())
                                                [0]
                                            .insert(app_name.clone());
                                            work_parent
                                                .entry("Overal")
                                                .or_insert(Default::default())[0]
                                                .insert(app_name.clone());
                                        }
                                        "onReceive" => {
                                            work_parent.entry("Jobs").or_insert(Default::default())
                                                [1]
                                            .insert(app_name.clone());
                                            work_parent
                                                .entry("Overal")
                                                .or_insert(Default::default())[1]
                                                .insert(app_name.clone());
                                        }
                                        "doWork" | "onStartJob" => {
                                            work_parent.entry("Jobs").or_insert(Default::default())
                                                [2]
                                            .insert(app_name.clone());
                                            work_parent
                                                .entry("Overal")
                                                .or_insert(Default::default())[2]
                                                .insert(app_name.clone());
                                        }
                                        _ => {}
                                    }
                                    work_parent.entry("Jobs").or_insert(Default::default())[3]
                                        .insert(app_name.clone());
                                    work_parent.entry("Overal").or_insert(Default::default())[3]
                                        .insert(app_name.clone());
                                }
                                if let Some(Trigger::Job(j)) = &bw.trigger {
                                    if let Some(t) = &j.time_driven_job {
                                        if let Some(e) = &j.event_driven_job {
                                            let mut job_type = Vec::new();
                                            let time_type = t.time_slot() as usize;
                                            if !e.required_network_transport_types.is_empty() {
                                                let r = jobs_table
                                                    .entry("Network Type")
                                                    .or_insert(Default::default());
                                                job_type.push("Network Type");
                                                r[time_type].insert(app_name.clone());
                                                r[8].insert(app_name.clone());
                                            }
                                            if !e.required_network_capabilities.is_empty() {
                                                // if !connection.leaks.is_empty() && (time_type == 6 || time_type == 7) {
                                                //     println!(
                                                //         "{} - {:?} - {:?} - {:?} ",
                                                //         app_name,
                                                //         j,
                                                //         connection.destination_addr,
                                                //         connection.leaks
                                                //     );
                                                //     // dbg!(
                                                //     //     &app_name,
                                                //     //     &connection.destination_addr,
                                                //     //     &connection.leaks
                                                //     // );
                                                // }
                                                let r = jobs_table
                                                    .entry("Network Capability")
                                                    .or_insert(Default::default());
                                                job_type.push("Network Capability");
                                                r[time_type].insert(app_name.clone());
                                                // for l in &connection.leaks{
                                                //     let o = get_leak_category(&l);
                                                //     dbg!(&app_name, &o, &l.leak);
                                                // }
                                                // dbg!(&connection.leaks);
                                                r[8].insert(app_name.clone());
                                            }
                                            if e.is_require_battery_not_low {
                                                let r = jobs_table
                                                    .entry("Battery")
                                                    .or_insert(Default::default());
                                                job_type.push("Battery");
                                                r[time_type].insert(app_name.clone());
                                                r[8].insert(app_name.clone());
                                            }
                                            if e.is_require_charging {
                                                let r = jobs_table
                                                    .entry("Charging")
                                                    .or_insert(Default::default());
                                                job_type.push("Charging");
                                                r[time_type].insert(app_name.clone());
                                                r[8].insert(app_name.clone());
                                            }
                                            if e.is_require_device_idle {
                                                let r = jobs_table
                                                    .entry("Device Idle")
                                                    .or_insert(Default::default());
                                                job_type.push("Device Idle");
                                                r[time_type].insert(app_name.clone());
                                                r[8].insert(app_name.clone());
                                            }
                                            if e.is_require_storage_not_low {
                                                let r = jobs_table
                                                    .entry("Storage")
                                                    .or_insert(Default::default());
                                                job_type.push("Storage");
                                                r[time_type].insert(app_name.clone());
                                                r[8].insert(app_name.clone());
                                            }
                                            if !e.trigger_content_uris.is_empty() {
                                                let r = jobs_table
                                                    .entry("Content")
                                                    .or_insert(Default::default());
                                                // job_type.push("Content");
                                                r[time_type].insert(app_name.clone());
                                                r[8].insert(app_name.clone());
                                            }
                                            if e.is_expedited {
                                                let r = jobs_table
                                                    .entry("Expedited")
                                                    .or_insert(Default::default());
                                                job_type.push("Expedited");
                                                r[time_type].insert(app_name.clone());
                                                r[8].insert(app_name.clone());
                                            }
                                            if e.is_important_while_foreground {
                                                let r = jobs_table
                                                    .entry("Important")
                                                    .or_insert(Default::default());
                                                job_type.push("Important");
                                                r[time_type].insert(app_name.clone());
                                                r[8].insert(app_name.clone());
                                            }
                                            let r = jobs_table
                                                .entry("Overall")
                                                .or_insert(Default::default());
                                            r[time_type].insert(app_name.clone());
                                            r[8].insert(app_name.clone());
                                            for l in &connection.leaks {
                                                let leak_type = get_leak_category(&l);
                                                if leak_type == "Other" {
                                                    continue;
                                                }

                                                for j in &job_type {
                                                    // if *j == "Overall" {
                                                    //     job_overal_leak
                                                    //         .entry(leak_type.to_string())
                                                    //         .and_modify(|e| {
                                                    //             e.insert(app_name.clone());
                                                    //         })
                                                    //         .or_insert_with(|| {
                                                    //             let mut s = HashSet::new();
                                                    //             s.insert(app_name.clone());
                                                    //             s
                                                    //         });
                                                    // }
                                                    if *j == "Network Type"
                                                        || *j == "Network Capability"
                                                    {
                                                        job_network_leak
                                                            .entry(leak_type.to_string())
                                                            .and_modify(|e| {
                                                                e.insert(app_name.clone());
                                                            })
                                                            .or_insert_with(|| {
                                                                let mut s = HashSet::new();
                                                                s.insert(app_name.clone());
                                                                s
                                                            });
                                                    }

                                                    if *j == "Battery" {
                                                        job_battery_leak
                                                            .entry(leak_type.to_string())
                                                            .and_modify(|e| {
                                                                e.insert(app_name.clone());
                                                            })
                                                            .or_insert_with(|| {
                                                                let mut s = HashSet::new();
                                                                s.insert(app_name.clone());
                                                                s
                                                            });
                                                    }
                                                    if *j == "Charging" {
                                                        job_charging_leak
                                                            .entry(leak_type.to_string())
                                                            .and_modify(|e| {
                                                                e.insert(app_name.clone());
                                                            })
                                                            .or_insert_with(|| {
                                                                let mut s = HashSet::new();
                                                                s.insert(app_name.clone());
                                                                s
                                                            });
                                                    }
                                                    if *j == "Device Idle" {
                                                        job_device_idel_leak
                                                            .entry(leak_type.to_string())
                                                            .and_modify(|e| {
                                                                e.insert(app_name.clone());
                                                            })
                                                            .or_insert_with(|| {
                                                                let mut s = HashSet::new();
                                                                s.insert(app_name.clone());
                                                                s
                                                            });
                                                    }
                                                    if *j == "Storage" {
                                                        job_storage_leak
                                                            .entry(leak_type.to_string())
                                                            .and_modify(|e| {
                                                                e.insert(app_name.clone());
                                                            })
                                                            .or_insert_with(|| {
                                                                let mut s = HashSet::new();
                                                                s.insert(app_name.clone());
                                                                s
                                                            });
                                                    }
                                                    if *j == "Expedited" {
                                                        job_expediate_leak
                                                            .entry(leak_type.to_string())
                                                            .and_modify(|e| {
                                                                e.insert(app_name.clone());
                                                            })
                                                            .or_insert_with(|| {
                                                                let mut s = HashSet::new();
                                                                s.insert(app_name.clone());
                                                                s
                                                            });
                                                    }
                                                    if *j == "Important" {
                                                        job_important_leak
                                                            .entry(leak_type.to_string())
                                                            .and_modify(|e| {
                                                                e.insert(app_name.clone());
                                                            })
                                                            .or_insert_with(|| {
                                                                let mut s = HashSet::new();
                                                                s.insert(app_name.clone());
                                                                s
                                                            });
                                                    }
                                                }
                                            }
                                        } else {
                                            dbg!("1111");
                                        }
                                    } else {
                                        dbg!("2222");
                                    }
                                }
                            }

                            if visible {
                                visible_works.insert((app_name.clone(), has_dst));
                            } else {
                                invisible_works.insert((app_name.clone(), has_dst));
                            }
                        }
                        _ => {}
                    }
                    // if method == "onCreate" {
                    //     //     if let Some(work) = data
                    //     //     .works
                    //     //     .par_iter()
                    //     //     .find_first(|w| &format!("{}.{}", w.class, w.method) == target)
                    //     // || target == "com.google.android.datatransport.runtime.scheduling.jobscheduling.JobInfoSchedulerService.onStartJob" || target == "androidx.work.impl.background.systemjob.SystemJobService.onStartJob")
                    //     // {

                    //     if let Some(c) = data.works
                    //         .par_iter()
                    //         .find_first(|w| w.class == class && ["onStartCommand","onBind","onRebind"].iter().any(|v|*v==w.method)){
                    //             // dbg!(&app_name, &c.class, &c.method);
                    //         }else{
                    //             dbg!(&app_name, &class, &method);
                    //         }

                    //     // }
                    //     // println!(
                    //     //     "{} - {} - {} - {} ",
                    //     //     app_name, visible, target, connection.src_addr
                    //     // );
                    // }
                }
                // println!(
                //     "{} - {} - {} - {} ",
                //     app_name, visible, target, connection.src_addr
                // );
            }
            for leak in &connection.leaks {
                if !leak.is_send {
                    continue;
                }
                let leak_item = match leak.leak.as_str() {
                    "latitude4" | "langitude4" => "gps4",
                    "latitude3" | "langitude3" => "gps3",
                    "latitude2" | "langitude2" => "gps2",
                    _ => leak.leak.as_str(),
                };

                leak_items.insert(leak_item.to_string());

                // let has_target = connection.target.is_some();

                //destination_addr
                let without_explicit = if let Some(t) = connection.target.as_ref() {
                    if t.ends_with("onReceive") {
                        let mut o = receivers_table.iter().filter_map(|(k, v)| {
                            let mut classes = v.iter().filter_map(|(app, class)| {
                                if app == &app_name {
                                    Some(class)
                                } else {
                                    None
                                }
                            });

                            if classes.any(|c| t.starts_with(c)) {
                                Some(k)
                            } else {
                                None
                            }
                        });
                        // NEW_IMPLICIT.contains(&intent.action.as_str())
                        // || IMPLICIT.contains(&intent.action.as_str())
                        o.any(|x| {
                            NEW_IMPLICIT.contains(&x.as_str()) || IMPLICIT.contains(&x.as_str())
                        })
                    }
                    /*else if ["onCreate", "onStartCommand", "onBind", "onRebind"]
                        .iter()
                        .any(|v| t.ends_with(v))
                    {
                        let x= foreground_classes.iter().any(|c| t.starts_with(c));
                        // dbg!(&app_name, &t, &x);
                        x
                        // dbg!(&app_name, &t);
                        // true
                    } */
                    else {
                        true
                    }
                } else {
                    false
                };
                if without_explicit {
                    /*
                    if let Some(work) = data
                        .works
                        .par_iter()
                        .find_first(|w| &format!("{}.{}", w.class, w.method) == target)
                    // || target == "com.google.android.datatransport.runtime.scheduling.jobscheduling.JobInfoSchedulerService.onStartJob" || target == "androidx.work.impl.background.systemjob.SystemJobService.onStartJob")
                    {
                        let work_type = match work.trigger.as_ref() {
                            Some(Trigger::BroadcastReceiver(info)) => {
                                if info.alarm_info.is_some() {
                                    WorkType::Time
                                } else {
                                    WorkType::Event
                                }
                            }
                            Some(Trigger::Job(info)) => {
                                if info.event_driven_job.is_some() && info.time_driven_job.is_some()
                                {
                                    WorkType::EventTime
                                } else if info.event_driven_job.is_some() {
                                    WorkType::Event
                                } else {
                                    WorkType::Time
                                }
                            }
                            Some(Trigger::Service(info, service_method)) => WorkType::Persistent,
                            None => {
                                // oncreate
                                WorkType::Persistent
                            }
                        };
                        if leak.is_encrypted {
                            attributed_encrypted_leaks
                                .entry(leak_item.to_string())
                                .or_insert(HashSet::new())
                                .insert(app_name.clone());
                            match work_type {
                                WorkType::Time => {
                                    time_encrypted_leaks
                                        .entry(leak_item.to_string())
                                        .or_insert(HashSet::new())
                                        .insert(app_name.clone());
                                }
                                WorkType::Event => {
                                    event_encrypted_leaks
                                        .entry(leak_item.to_string())
                                        .or_insert(HashSet::new())
                                        .insert(app_name.clone());
                                }
                                WorkType::EventTime => {
                                    event_time_encrypted_leaks
                                        .entry(leak_item.to_string())
                                        .or_insert(HashSet::new())
                                        .insert(app_name.clone());
                                }
                                WorkType::Persistent => {
                                    persistent_encrypted_leaks
                                        .entry(leak_item.to_string())
                                        .or_insert(HashSet::new())
                                        .insert(app_name.clone());
                                }
                            }
                        } else {
                            attributed_plain_leaks
                                .entry(leak_item.to_string())
                                .or_insert(HashSet::new())
                                .insert(app_name.clone());
                            match work_type {
                                WorkType::Time => {
                                    time_plain_leaks
                                        .entry(leak_item.to_string())
                                        .or_insert(HashSet::new())
                                        .insert(app_name.clone());
                                }
                                WorkType::Event => {
                                    event_plain_leaks
                                        .entry(leak_item.to_string())
                                        .or_insert(HashSet::new())
                                        .insert(app_name.clone());
                                }
                                WorkType::EventTime => {
                                    event_time_plain_leaks
                                        .entry(leak_item.to_string())
                                        .or_insert(HashSet::new())
                                        .insert(app_name.clone());
                                }
                                WorkType::Persistent => {
                                    persistent_plain_leaks
                                        .entry(leak_item.to_string())
                                        .or_insert(HashSet::new())
                                        .insert(app_name.clone());
                                }
                            }
                        }
                        attributed_overal_leaks
                            .entry(leak_item.to_string())
                            .or_insert(HashSet::new())
                            .insert(app_name.clone());
                        match work_type {
                            WorkType::Time => {
                                time_overal_leaks
                                    .entry(leak_item.to_string())
                                    .or_insert(HashSet::new())
                                    .insert(app_name.clone());
                            }
                            WorkType::Event => {
                                event_overal_leaks
                                    .entry(leak_item.to_string())
                                    .or_insert(HashSet::new())
                                    .insert(app_name.clone());
                            }
                            WorkType::EventTime => {
                                event_time_overal_leaks
                                    .entry(leak_item.to_string())
                                    .or_insert(HashSet::new())
                                    .insert(app_name.clone());
                            }
                            WorkType::Persistent => {
                                persistent_overal_leaks
                                    .entry(leak_item.to_string())
                                    .or_insert(HashSet::new())
                                    .insert(app_name.clone());
                            }
                        }
                    } else {
                        dbg!(&target, &app_name);
                    }
                    */
                    if leak.is_encrypted {
                        background_encrypted_leaks
                            .entry(leak_item.to_string())
                            .or_insert(HashSet::new())
                            .insert(app_name.clone());
                    } else {
                        background_plain_leaks
                            .entry(leak_item.to_string())
                            .or_insert(HashSet::new())
                            .insert(app_name.clone());
                    }
                    background_overal_leaks
                        .entry(leak_item.to_string())
                        .or_insert(HashSet::new())
                        .insert(app_name.clone());
                } else {
                    match visibility {
                        Some(true) => {
                            if leak.is_encrypted {
                                visible_encrypted_leaks
                                    .entry(leak_item.to_string())
                                    .or_insert(HashSet::new())
                                    .insert(app_name.clone());
                            } else {
                                visible_plain_leaks
                                    .entry(leak_item.to_string())
                                    .or_insert(HashSet::new())
                                    .insert(app_name.clone());
                            }
                            visible_overal_leaks
                                .entry(leak_item.to_string())
                                .or_insert(HashSet::new())
                                .insert(app_name.clone());
                        }
                        Some(false) => {
                            if leak.is_encrypted {
                                invisible_encrypted_leaks
                                    .entry(leak_item.to_string())
                                    .or_insert(HashSet::new())
                                    .insert(app_name.clone());
                            } else {
                                invisible_plain_leaks
                                    .entry(leak_item.to_string())
                                    .or_insert(HashSet::new())
                                    .insert(app_name.clone());
                            }
                            invisible_overal_leaks
                                .entry(leak_item.to_string())
                                .or_insert(HashSet::new())
                                .insert(app_name.clone());
                        }
                        None => {}
                    }
                }

                if leak.is_encrypted {
                    overall_encrypted_leaks
                        .entry(leak_item.to_string())
                        .or_insert(HashSet::new())
                        .insert(app_name.clone());
                } else {
                    overall_plain_leaks
                        .entry(leak_item.to_string())
                        .or_insert(HashSet::new())
                        .insert(app_name.clone());
                }
                overall_overal_leaks
                    .entry(leak_item.to_string())
                    .or_insert(HashSet::new())
                    .insert(app_name.clone());
            }
        }
    }

    println!("++++++++++New Table++++++++++");
    println!(
        "&Network&{}&{}&{}&{}&{}\\\\",
        job_network_leak
            .get("Device")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        job_network_leak
            .get("Network")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        job_network_leak
            .get("Location")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        job_network_leak
            .get("User Assets")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        job_network_leak
            .values()
            .into_iter()
            .flatten()
            .collect::<HashSet<&String>>()
            .iter()
            .count()
    );
    println!(
        "&Battery&{}&{}&{}&{}&{}\\\\",
        job_battery_leak
            .get("Device")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        job_battery_leak
            .get("Network")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        job_battery_leak
            .get("Location")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        job_battery_leak
            .get("User Assets")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        job_battery_leak
            .values()
            .into_iter()
            .flatten()
            .collect::<HashSet<&String>>()
            .iter()
            .count()
    );

    println!(
        "&Charging&{}&{}&{}&{}&{}\\\\",
        job_charging_leak
            .get("Device")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        job_charging_leak
            .get("Network")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        job_charging_leak
            .get("Location")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        job_charging_leak
            .get("User Assets")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        job_charging_leak
            .values()
            .into_iter()
            .flatten()
            .collect::<HashSet<&String>>()
            .iter()
            .count()
    );

    println!(
        "&Device Idle&{}&{}&{}&{}&{}\\\\",
        job_device_idel_leak
            .get("Device")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        job_device_idel_leak
            .get("Network")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        job_device_idel_leak
            .get("Location")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        job_device_idel_leak
            .get("User Assets")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        job_device_idel_leak
            .values()
            .into_iter()
            .flatten()
            .collect::<HashSet<&String>>()
            .iter()
            .count()
    );

    println!(
        "&Storage&{}&{}&{}&{}&{}\\\\",
        job_storage_leak
            .get("Device")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        job_storage_leak
            .get("Network")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        job_storage_leak
            .get("Location")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        job_storage_leak
            .get("User Assets")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        job_storage_leak
            .values()
            .into_iter()
            .flatten()
            .collect::<HashSet<&String>>()
            .iter()
            .count()
    );

    println!(
        "&Expedited&{}&{}&{}&{}&{}\\\\",
        job_expediate_leak
            .get("Device")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        job_expediate_leak
            .get("Network")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        job_expediate_leak
            .get("Location")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        job_expediate_leak
            .get("User Assets")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        job_expediate_leak
            .values()
            .into_iter()
            .flatten()
            .collect::<HashSet<&String>>()
            .iter()
            .count()
    );

    println!(
        "&Important&{}&{}&{}&{}&{}\\\\",
        job_important_leak
            .get("Device")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        job_important_leak
            .get("Network")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        job_important_leak
            .get("Location")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        job_important_leak
            .get("User Assets")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        job_important_leak
            .values()
            .into_iter()
            .flatten()
            .collect::<HashSet<&String>>()
            .iter()
            .count()
    );

    let job_overal_leak = job_network_leak
        .iter()
        .chain(job_battery_leak.iter())
        .chain(job_charging_leak.iter())
        .chain(job_device_idel_leak.iter())
        .chain(job_storage_leak.iter())
        .chain(job_expediate_leak.iter())
        .chain(job_important_leak.iter())
        .fold(HashMap::new(), |mut acc, (k, v)| {
            acc.entry(k.clone())
                .or_insert_with(HashSet::new)
                .extend(v.iter().cloned());
            acc
        });

    println!(
        "&Overall&{}&{}&{}&{}&{}\\\\",
        job_overal_leak
            .get("Device")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        job_overal_leak
            .get("Network")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        job_overal_leak
            .get("Location")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        job_overal_leak
            .get("User Assets")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        job_overal_leak
            .values()
            .into_iter()
            .flatten()
            .collect::<HashSet<&String>>()
            .iter()
            .count()
    );

    // broadcasts

    // println!("Implicit&{}&{}&{}&{}&{}\\\\", broadcast_implicit_leak
    //     .get("Device")
    //     .map(|apps| apps.iter().count())
    //     .unwrap_or(0),broadcast_implicit_leak.get("Network")

    println!(
        "&Implicit&{}&{}&{}&{}&{}\\\\",
        broadcast_implicit_leak
            .get("Device")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        broadcast_implicit_leak
            .get("Network")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        broadcast_implicit_leak
            .get("Location")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        broadcast_implicit_leak
            .get("User Assets")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        broadcast_implicit_leak
            .values()
            .into_iter()
            .flatten()
            .collect::<HashSet<&String>>()
            .iter()
            .count()
    );

    println!(
        "&Undocumented Implicit&{}&{}&{}&{}&{}\\\\",
        broadcast_undocumented_implicit_leak
            .get("Device")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        broadcast_undocumented_implicit_leak
            .get("Network")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        broadcast_undocumented_implicit_leak
            .get("Location")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        broadcast_undocumented_implicit_leak
            .get("User Assets")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        broadcast_undocumented_implicit_leak
            .values()
            .into_iter()
            .flatten()
            .collect::<HashSet<&String>>()
            .iter()
            .count()
    );

    println!(
        "&Explicit&{}&{}&{}&{}&{}\\\\",
        broadcast_explicit_leak
            .get("Device")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        broadcast_explicit_leak
            .get("Network")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        broadcast_explicit_leak
            .get("Location")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        broadcast_explicit_leak
            .get("User Assets")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        broadcast_explicit_leak
            .values()
            .into_iter()
            .flatten()
            .collect::<HashSet<&String>>()
            .iter()
            .count()
    );

    let broadcast_overal_leak = broadcast_implicit_leak
        .iter()
        .chain(broadcast_undocumented_implicit_leak.iter())
        .chain(broadcast_explicit_leak.iter())
        .fold(HashMap::new(), |mut acc, (k, v)| {
            acc.entry(k.clone())
                .or_insert_with(HashSet::new)
                .extend(v.iter().cloned());
            acc
        });

        let broadcast_overal_leak_without_explicit = broadcast_implicit_leak
        .iter()
        .chain(broadcast_undocumented_implicit_leak.iter())
        .fold(HashMap::new(), |mut acc, (k, v)| {
            acc.entry(k.clone())
                .or_insert_with(HashSet::new)
                .extend(v.iter().cloned());
            acc
        });

    println!(
        "&Overall&{}&{}&{}&{}&{}\\\\",
        broadcast_overal_leak
            .get("Device")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        broadcast_overal_leak
            .get("Network")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        broadcast_overal_leak
            .get("Location")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        broadcast_overal_leak
            .get("User Assets")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        broadcast_overal_leak
            .values()
            .into_iter()
            .flatten()
            .collect::<HashSet<&String>>()
            .iter()
            .count()
    );

    // service

    println!(
        "&Foreground&{}&{}&{}&{}&{}\\\\",
        service_foreground_leak
            .get("Device")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        service_foreground_leak
            .get("Network")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        service_foreground_leak
            .get("Location")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        service_foreground_leak
            .get("User Assets")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        service_foreground_leak
            .values()
            .into_iter()
            .flatten()
            .collect::<HashSet<&String>>()
            .iter()
            .count()
    );
    println!(
        "&Background&{}&{}&{}&{}&{}\\\\",
        service_background_leak
            .get("Device")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        service_background_leak
            .get("Network")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        service_background_leak
            .get("Location")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        service_background_leak
            .get("User Assets")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        service_background_leak
            .values()
            .into_iter()
            .flatten()
            .collect::<HashSet<&String>>()
            .iter()
            .count()
    );

    let service_overal_leak = service_foreground_leak
        .iter()
        .chain(service_background_leak.iter())
        .fold(HashMap::new(), |mut acc, (k, v)| {
            acc.entry(k.clone())
                .or_insert_with(HashSet::new)
                .extend(v.iter().cloned());
            acc
        });

    println!(
        "&Overall&{}&{}&{}&{}&{}\\\\",
        service_overal_leak
            .get("Device")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        service_overal_leak
            .get("Network")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        service_overal_leak
            .get("Location")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        service_overal_leak
            .get("User Assets")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        service_overal_leak
            .values()
            .into_iter()
            .flatten()
            .collect::<HashSet<&String>>()
            .iter()
            .count()
    );

    let backgroundwork_leak_overall = service_foreground_leak//service_overal_leak
        .iter()
        .chain(broadcast_overal_leak_without_explicit.iter())
        .chain(job_overal_leak.iter())
        .fold(HashMap::new(), |mut acc, (k, v)| {
            acc.entry(k.clone())
                .or_insert_with(HashSet::new)
                .extend(v.iter().cloned());
            acc
        });

    println!(
        "&Overall&{}&{}&{}&{}&{}\\\\",
        backgroundwork_leak_overall
            .get("Device")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        backgroundwork_leak_overall
            .get("Network")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        backgroundwork_leak_overall
            .get("Location")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        backgroundwork_leak_overall
            .get("User Assets")
            .map(|apps| apps.iter().count())
            .unwrap_or(0),
        backgroundwork_leak_overall
            .values()
            .into_iter()
            .flatten()
            .collect::<HashSet<&String>>()
            .iter()
            .count()
    );
    println!("----------New Table----------");

    let mut leaks_visible_categories: HashMap<String, HashSet<String>> = HashMap::new();
    let mut leaks_invisible_categories: HashMap<String, HashSet<String>> = HashMap::new();
    let mut leaks_works_categories: HashMap<String, HashSet<String>> = HashMap::new();
    let mut leaks_works_invisible_categories: HashMap<String, HashSet<String>> = HashMap::new();
    let mut leaks_any_categories: HashMap<String, HashSet<String>> = HashMap::new();
    let mut exclusive_leakage_visible: HashSet<String> = HashSet::new();
    let mut exclusive_leakage_invisible: HashSet<String> = HashSet::new();
    let mut exclusive_leakage_works: HashSet<String> = HashSet::new();
    for (item, item_name) in LEAK_ITEMS {
        let mut leak_type = String::new();
        if DEVICE_INFO.contains(&item) {
            leak_type = "Device".to_string();
        } else if NETWORK_INFO.contains(&item) {
            leak_type = "Network".to_string();
        } else if NETWORK_LOCATION_INFO.contains(&item) {
            leak_type = "Network Location".to_string();
        } else if LOCATION_INFO.contains(&item) {
            leak_type = "GPS".to_string();
        } else if USER_ASSETS.contains(&item) {
            leak_type = "User Assets".to_string();
        };

        let num_visible_overal_leaks = visible_overal_leaks
            .get(item)
            .map(|i| i.len())
            .unwrap_or_default();
        let num_visible_encrypted_leaks = visible_encrypted_leaks
            .get(item)
            .map(|i| i.len())
            .unwrap_or_default();
        let num_visible_plain_leaks = visible_plain_leaks
            .get(item)
            .map(|i| i.len())
            .unwrap_or_default();
        if let Some(l) = visible_overal_leaks.get(item) {
            leaks_visible_categories
                .entry(leak_type.clone())
                .or_insert(HashSet::new())
                .extend(l.iter().cloned());
            leaks_any_categories
                .entry(leak_type.clone())
                .or_insert(HashSet::new())
                .extend(l.iter().cloned());
        };

        let num_invisible_overal_leaks = invisible_overal_leaks
            .get(item)
            .map(|i| i.len())
            .unwrap_or_default();
        let num_invisible_encrypted_leaks = invisible_encrypted_leaks
            .get(item)
            .map(|i| i.len())
            .unwrap_or_default();
        let num_invisible_plain_leaks = invisible_plain_leaks
            .get(item)
            .map(|i| i.len())
            .unwrap_or_default();

        if let Some(l) = invisible_overal_leaks.get(item) {
            leaks_invisible_categories
                .entry(leak_type.clone())
                .or_insert(HashSet::new())
                .extend(l.iter().cloned());
            leaks_works_invisible_categories
                .entry(leak_type.clone())
                .or_insert(HashSet::new())
                .extend(l.iter().cloned());
            leaks_any_categories
                .entry(leak_type.clone())
                .or_insert(HashSet::new())
                .extend(l.iter().cloned());
        };

        let num_background_overal_leaks = background_overal_leaks
            .get(item)
            .map(|i| i.len())
            .unwrap_or_default();
        let num_background_encrypted_leaks = background_encrypted_leaks
            .get(item)
            .map(|i| i.len())
            .unwrap_or_default();
        let num_background_plain_leaks = background_plain_leaks
            .get(item)
            .map(|i| i.len())
            .unwrap_or_default();

        if let Some(l) = background_overal_leaks.get(item) {
            leaks_works_categories
                .entry(leak_type.clone())
                .or_insert(HashSet::new())
                .extend(l.iter().cloned());
            leaks_works_invisible_categories
                .entry(leak_type.clone())
                .or_insert(HashSet::new())
                .extend(l.iter().cloned());
            leaks_any_categories
                .entry(leak_type.clone())
                .or_insert(HashSet::new())
                .extend(l.iter().cloned());
        };

        let num_overall_overal_leaks = overall_overal_leaks
            .get(item)
            .map(|i| i.len())
            .unwrap_or_default();
        let num_overall_encrypted_leaks = overall_encrypted_leaks
            .get(item)
            .map(|i| i.len())
            .unwrap_or_default();
        let num_overall_plain_leaks = overall_plain_leaks
            .get(item)
            .map(|i| i.len())
            .unwrap_or_default();
        let empty_set = HashSet::new();
        // let mut exclusive_visible_leaks: HashSet<String> = HashSet::new();
        // let mut exclusive_invisible_leaks: HashSet<String> = HashSet::new();
        // let mut exclusive_work_leaks: HashSet<String> = HashSet::new();
        let num_exclusive_visible_leaks = visible_overal_leaks
            .get(item)
            .unwrap_or(&empty_set)
            .difference(invisible_overal_leaks.get(item).unwrap_or(&empty_set))
            .cloned()
            .collect::<HashSet<String>>()
            .difference(background_overal_leaks.get(item).unwrap_or(&empty_set))
            .cloned()
            .collect::<HashSet<String>>();
        exclusive_leakage_visible.extend(num_exclusive_visible_leaks.iter().cloned());
        let num_exclusive_visible_leaks = num_exclusive_visible_leaks.len();
        let num_exclusive_invisible_leaks = invisible_overal_leaks
            .get(item)
            .unwrap_or(&empty_set)
            .difference(visible_overal_leaks.get(item).unwrap_or(&empty_set))
            .cloned()
            .collect::<HashSet<String>>()
            .difference(background_overal_leaks.get(item).unwrap_or(&empty_set))
            .cloned()
            .collect::<HashSet<String>>();
        exclusive_leakage_invisible.extend(num_exclusive_invisible_leaks.iter().cloned());
        let num_exclusive_invisible_leaks = num_exclusive_invisible_leaks.len();
        let num_exclusive_works_leaks = background_overal_leaks
            .get(item)
            .unwrap_or(&empty_set)
            .difference(visible_overal_leaks.get(item).unwrap_or(&empty_set))
            .cloned()
            .collect::<HashSet<String>>()
            .difference(invisible_overal_leaks.get(item).unwrap_or(&empty_set))
            .into_iter()
            .cloned()
            .collect::<HashSet<String>>();
        exclusive_leakage_works.extend(num_exclusive_works_leaks.iter().cloned());
        // if NETWORK_LOCATION_INFO.contains(&item)
        //     || USER_ASSETS.contains(&item)
        //     || LOCATION_INFO.contains(&item)
        // {
        //     for a in &num_exclusive_works_leaks {
        //         println!("{} - {} - {}", item, a, leak_type);
        //     }
        // }
        // "&{}&{}&{}&{}&{}&{}&{}&{}&{}&{}&{}&{}&{}&{}&{}&{}\\\\",
        println!(
            "&{}&{}&{}&{}&{}&{}&{}&{}\\\\",
            item_name,
            // num_visible_plain_leaks,
            // num_visible_encrypted_leaks,
            num_exclusive_visible_leaks,
            num_visible_overal_leaks,
            // num_invisible_plain_leaks,
            // num_invisible_encrypted_leaks,
            num_exclusive_invisible_leaks,
            num_invisible_overal_leaks,
            // num_background_plain_leaks,
            // num_background_encrypted_leaks,
            num_exclusive_works_leaks.len(),
            num_background_overal_leaks,
            // num_overall_plain_leaks,
            // num_overall_encrypted_leaks,
            num_overall_overal_leaks
        );
    }
    println!("-------------------");
    println!(
        "Total Exclusive Leakage Visible: {}",
        exclusive_leakage_visible.len()
    );
    println!(
        "Total Exclusive Leakage Invisible: {}",
        exclusive_leakage_invisible.len()
    );
    println!(
        "Total Exclusive Leakage Works: {}",
        exclusive_leakage_works.len()
    );
    println!("-------------------");
    for (k, v) in leaks_visible_categories.iter() {
        println!("leak visible = {}: {}", k, v.len());
    }
    println!(
        "overal leaks visible = {}",
        leaks_visible_categories
            .values()
            .flatten()
            .cloned()
            .collect::<HashSet<String>>()
            .len()
    );
    for (k, v) in leaks_invisible_categories.iter() {
        println!("leak invisible = {}: {}", k, v.len());
    }
    println!(
        "overal leaks invisible = {}",
        leaks_invisible_categories
            .values()
            .flatten()
            .cloned()
            .collect::<HashSet<String>>()
            .len()
    );
    for (k, v) in leaks_works_categories.iter() {
        println!("leak works = {}: {}", k, v.len());
    }
    println!(
        "overal leaks works = {}",
        leaks_works_categories
            .values()
            .flatten()
            .cloned()
            .collect::<HashSet<String>>()
            .len()
    );
    for (k, v) in leaks_works_invisible_categories.iter() {
        println!("leak works invisible = {}: {}", k, v.len());
    }
    println!(
        "overal leaks works invisible = {}",
        leaks_works_invisible_categories
            .values()
            .flatten()
            .cloned()
            .collect::<HashSet<String>>()
            .len()
    );
    for (k, v) in leaks_any_categories.iter() {
        println!("leak any = {}: {}", k, v.len());
    }
    println!(
        "overal any = {}",
        leaks_any_categories
            .values()
            .flatten()
            .cloned()
            .collect::<HashSet<String>>()
            .len()
    );
    println!("-------------------");
    let visible_total_leaks = visible_overal_leaks
        .values()
        .flatten()
        .cloned()
        .collect::<HashSet<String>>()
        .len();
    let invisible_total_leaks = invisible_overal_leaks
        .values()
        .flatten()
        .cloned()
        .collect::<HashSet<String>>()
        .len();
    let background_total_leaks = background_overal_leaks
        .values()
        .flatten()
        .cloned()
        .collect::<HashSet<String>>()
        .len();
    let overall_total_leaks = overall_overal_leaks
        .values()
        .flatten()
        .cloned()
        .collect::<HashSet<String>>()
        .len();
    println!(
        "Visible Total: {}, Invisible Total: {}, Background Total: {}, Overall Total: {}",
        visible_total_leaks, invisible_total_leaks, background_total_leaks, overall_total_leaks
    );
    {
        let visible_established_foreground_service = visible_foreground
            .iter()
            .filter_map(|(n, b)| if *b { Some(n) } else { None })
            .collect::<HashSet<_>>();
        let visible_interupted_foreground_service = visible_foreground
            .iter()
            .filter_map(|(n, b)| if !*b { Some(n) } else { None })
            .collect::<HashSet<_>>();
        let visible_common_foreground_service = visible_established_foreground_service
            .intersection(&visible_interupted_foreground_service)
            .collect::<HashSet<_>>();
        let visible_overal_foreground_service = visible_foreground
            .iter()
            .map(|(n, _)| n)
            .collect::<HashSet<_>>();

        let invisible_established_foreground_service = invisible_foreground
            .iter()
            .filter_map(|(n, b)| if *b { Some(n) } else { None })
            .collect::<HashSet<_>>();
        let invisible_interupted_foreground_service = invisible_foreground
            .iter()
            .filter_map(|(n, b)| if !*b { Some(n) } else { None })
            .collect::<HashSet<_>>();
        let invisible_common_foreground_service = invisible_established_foreground_service
            .intersection(&invisible_interupted_foreground_service)
            .collect::<HashSet<_>>();
        let invisible_overal_foreground_service = invisible_foreground
            .iter()
            .map(|(n, _)| n)
            .collect::<HashSet<_>>();

        let any_established_foreground_service = visible_established_foreground_service
            .union(&invisible_established_foreground_service)
            .collect::<HashSet<_>>();
        let any_interupted_foreground_service = visible_interupted_foreground_service
            .union(&invisible_interupted_foreground_service)
            .collect::<HashSet<_>>();
        let any_common_foreground_service = visible_common_foreground_service
            .union(&invisible_common_foreground_service)
            .collect::<HashSet<_>>();
        let any_overal_foreground_service = visible_overal_foreground_service
            .union(&invisible_overal_foreground_service)
            .collect::<HashSet<_>>();

        let visible_established_background_service = visible_background
            .iter()
            .filter_map(|(n, b)| if *b { Some(n) } else { None })
            .collect::<HashSet<_>>();
        let visible_interupted_background_service = visible_background
            .iter()
            .filter_map(|(n, b)| if !*b { Some(n) } else { None })
            .collect::<HashSet<_>>();
        let visible_common_background_service = visible_established_background_service
            .intersection(&visible_interupted_background_service)
            .collect::<HashSet<_>>();
        let visible_overal_background_service = visible_background
            .iter()
            .map(|(n, _)| n)
            .collect::<HashSet<_>>();

        let invisible_established_background_service = invisible_background
            .iter()
            .filter_map(|(n, b)| if *b { Some(n) } else { None })
            .collect::<HashSet<_>>();
        let invisible_interupted_background_service = invisible_background
            .iter()
            .filter_map(|(n, b)| if !*b { Some(n) } else { None })
            .collect::<HashSet<_>>();
        let invisible_common_background_service = invisible_established_background_service
            .intersection(&invisible_interupted_background_service)
            .collect::<HashSet<_>>();
        let invisible_overal_background_service = invisible_background
            .iter()
            .map(|(n, _)| n)
            .collect::<HashSet<_>>();

        let any_established_background_service = visible_established_background_service
            .union(&invisible_established_background_service)
            .collect::<HashSet<_>>();
        let any_interupted_background_service = visible_interupted_background_service
            .union(&invisible_interupted_background_service)
            .collect::<HashSet<_>>();
        let any_common_background_service = visible_common_background_service
            .union(&invisible_common_background_service)
            .collect::<HashSet<_>>();
        let any_overal_background_service = visible_overal_background_service
            .union(&invisible_overal_background_service)
            .collect::<HashSet<_>>();

        let visible_established_works = visible_works
            .iter()
            .filter_map(|(n, b)| if *b { Some(n) } else { None })
            .collect::<HashSet<_>>();
        let visible_interupted_works = visible_works
            .iter()
            .filter_map(|(n, b)| if !*b { Some(n) } else { None })
            .collect::<HashSet<_>>();
        let visible_common_works = visible_established_works
            .intersection(&visible_interupted_works)
            .collect::<HashSet<_>>();
        let visible_overal_works = visible_works.iter().map(|(n, _)| n).collect::<HashSet<_>>();

        let invisible_established_works = invisible_works
            .iter()
            .filter_map(|(n, b)| if *b { Some(n) } else { None })
            .collect::<HashSet<_>>();
        let invisible_interupted_works = invisible_works
            .iter()
            .filter_map(|(n, b)| if !*b { Some(n) } else { None })
            .collect::<HashSet<_>>();
        let invisible_common_works = invisible_established_works
            .intersection(&invisible_interupted_works)
            .collect::<HashSet<_>>();
        let invisible_overal_works = invisible_works
            .iter()
            .map(|(n, _)| n)
            .collect::<HashSet<_>>();

        let any_established_works = visible_established_works
            .union(&invisible_established_works)
            .collect::<HashSet<_>>();
        let any_interupted_works = visible_interupted_works
            .union(&invisible_interupted_works)
            .collect::<HashSet<_>>();
        let any_common_works = visible_common_works
            .union(&invisible_common_works)
            .collect::<HashSet<_>>();
        let any_overal_works = visible_overal_works
            .union(&invisible_overal_works)
            .collect::<HashSet<_>>();

        let visible_established_implicit_broadcasts = visible_implicit_background
            .iter()
            .filter_map(|(n, b)| if *b { Some(n) } else { None })
            .collect::<HashSet<_>>();
        let visible_interupted_implicit_broadcasts = visible_implicit_background
            .iter()
            .filter_map(|(n, b)| if !*b { Some(n) } else { None })
            .collect::<HashSet<_>>();
        let visible_common_implicit_broadcasts = visible_established_implicit_broadcasts
            .intersection(&visible_interupted_implicit_broadcasts)
            .collect::<HashSet<_>>();
        let visible_overal_implicit_broadcasts = visible_implicit_background
            .iter()
            .map(|(n, _)| n)
            .collect::<HashSet<_>>();

        let invisible_established_implicit_broadcasts = invisible_implicit_background
            .iter()
            .filter_map(|(n, b)| if *b { Some(n) } else { None })
            .collect::<HashSet<_>>();
        let invisible_interupted_implicit_broadcasts = invisible_implicit_background
            .iter()
            .filter_map(|(n, b)| if !*b { Some(n) } else { None })
            .collect::<HashSet<_>>();
        let invisible_common_implicit_broadcasts = invisible_established_implicit_broadcasts
            .intersection(&invisible_interupted_implicit_broadcasts)
            .collect::<HashSet<_>>();
        let invisible_overal_implicit_broadcasts = invisible_implicit_background
            .iter()
            .map(|(n, _)| n)
            .collect::<HashSet<_>>();

        let any_established_implicit_broadcasts = visible_established_implicit_broadcasts
            .union(&invisible_established_implicit_broadcasts)
            .collect::<HashSet<_>>();
        let any_interupted_implicit_broadcasts = visible_interupted_implicit_broadcasts
            .union(&invisible_interupted_implicit_broadcasts)
            .collect::<HashSet<_>>();
        let any_common_implicit_broadcasts = visible_common_implicit_broadcasts
            .union(&invisible_common_implicit_broadcasts)
            .collect::<HashSet<_>>();
        let any_overal_implicit_broadcasts = visible_overal_implicit_broadcasts
            .union(&invisible_overal_implicit_broadcasts)
            .collect::<HashSet<_>>();

        let visible_established_explicit_broadcasts = visible_explicit_background
            .iter()
            .filter_map(|(n, b)| if *b { Some(n) } else { None })
            .collect::<HashSet<_>>();
        let visible_interupted_explicit_broadcasts = visible_explicit_background
            .iter()
            .filter_map(|(n, b)| if !*b { Some(n) } else { None })
            .collect::<HashSet<_>>();
        let visible_common_explicit_broadcasts = visible_established_explicit_broadcasts
            .intersection(&visible_interupted_explicit_broadcasts)
            .collect::<HashSet<_>>();
        let visible_overal_explicit_broadcasts = visible_explicit_background
            .iter()
            .map(|(n, _)| n)
            .collect::<HashSet<_>>();

        let invisible_established_explicit_broadcasts = invisible_explicit_background
            .iter()
            .filter_map(|(n, b)| if *b { Some(n) } else { None })
            .collect::<HashSet<_>>();
        let invisible_interupted_explicit_broadcasts = invisible_explicit_background
            .iter()
            .filter_map(|(n, b)| if !*b { Some(n) } else { None })
            .collect::<HashSet<_>>();
        let invisible_common_explicit_broadcasts = invisible_established_explicit_broadcasts
            .intersection(&invisible_interupted_explicit_broadcasts)
            .collect::<HashSet<_>>();
        let invisible_overal_explicit_broadcasts = invisible_explicit_background
            .iter()
            .map(|(n, _)| n)
            .collect::<HashSet<_>>();

        let any_established_explicit_broadcasts = visible_established_explicit_broadcasts
            .union(&invisible_established_explicit_broadcasts)
            .collect::<HashSet<_>>();
        let any_interupted_explicit_broadcasts = visible_interupted_explicit_broadcasts
            .union(&invisible_interupted_explicit_broadcasts)
            .collect::<HashSet<_>>();
        let any_common_explicit_broadcasts = visible_common_explicit_broadcasts
            .union(&invisible_common_explicit_broadcasts)
            .collect::<HashSet<_>>();
        let any_overal_explicit_broadcasts = visible_overal_explicit_broadcasts
            .union(&invisible_overal_explicit_broadcasts)
            .collect::<HashSet<_>>();

        let visible_established_any = visible_established_foreground_service
            .union(&visible_established_background_service)
            .cloned()
            .collect::<HashSet<_>>()
            .union(&visible_established_works.clone())
            .cloned()
            .collect::<HashSet<_>>()
            .union(&visible_established_implicit_broadcasts)
            .cloned()
            .collect::<HashSet<_>>()
            .union(&visible_established_explicit_broadcasts)
            .cloned()
            .collect::<HashSet<_>>();
        println!(
            "&(E)stablished&{}&{}&{}&{}&{}&{}\\\\",
            visible_established_foreground_service.len(),
            visible_established_background_service.len(),
            visible_established_works.len(),
            visible_established_implicit_broadcasts.len(),
            visible_established_explicit_broadcasts.len(),
            visible_established_any.len()
        );

        let visible_interupted_any = visible_interupted_foreground_service
            .union(&visible_interupted_background_service)
            .cloned()
            .collect::<HashSet<_>>()
            .union(&visible_interupted_works.clone())
            .cloned()
            .collect::<HashSet<_>>()
            .union(&visible_interupted_implicit_broadcasts)
            .cloned()
            .collect::<HashSet<_>>()
            .union(&visible_interupted_explicit_broadcasts)
            .cloned()
            .collect::<HashSet<_>>();

        println!(
            "&(D)ropped&{}&{}&{}&{}&{}&{}\\\\",
            visible_interupted_foreground_service.len(),
            visible_interupted_background_service.len(),
            visible_interupted_works.len(),
            visible_interupted_implicit_broadcasts.len(),
            visible_interupted_explicit_broadcasts.len(),
            visible_interupted_any.len()
        );

        let visible_common_any = visible_established_any
            .intersection(&visible_interupted_any)
            .cloned()
            .collect::<HashSet<_>>();

        println!(
            "&Common&{}&{}&{}&{}&{}&{}\\\\",
            visible_common_foreground_service.len(),
            visible_common_background_service.len(),
            visible_common_works.len(),
            visible_common_implicit_broadcasts.len(),
            visible_common_explicit_broadcasts.len(),
            visible_common_any.len()
        );

        let visible_overal_any = visible_established_any
            .union(&visible_interupted_any)
            .cloned()
            .collect::<HashSet<_>>();

        println!(
            "&Total&{}&{}&{}&{}&{}&{}\\\\",
            visible_overal_foreground_service.len(),
            visible_overal_background_service.len(),
            visible_overal_works.len(),
            visible_overal_implicit_broadcasts.len(),
            visible_overal_explicit_broadcasts.len(),
            visible_overal_any.len()
        );

        let invisible_established_any = invisible_established_foreground_service
            .union(&invisible_established_background_service)
            .cloned()
            .collect::<HashSet<_>>()
            .union(&invisible_established_works.clone())
            .cloned()
            .collect::<HashSet<_>>()
            .union(&invisible_established_implicit_broadcasts)
            .cloned()
            .collect::<HashSet<_>>()
            .union(&invisible_established_explicit_broadcasts)
            .cloned()
            .collect::<HashSet<_>>();

        println!(
            "&(E)stablished&{}&{}&{}&{}&{}&{}\\\\",
            invisible_established_foreground_service.len(),
            invisible_established_background_service.len(),
            invisible_established_works.len(),
            invisible_established_implicit_broadcasts.len(),
            invisible_established_explicit_broadcasts.len(),
            invisible_established_any.len()
        );

        let invisible_interupted_any = invisible_interupted_foreground_service
            .union(&invisible_interupted_background_service)
            .cloned()
            .collect::<HashSet<_>>()
            .union(&invisible_interupted_works.clone())
            .cloned()
            .collect::<HashSet<_>>()
            .union(&invisible_interupted_implicit_broadcasts)
            .cloned()
            .collect::<HashSet<_>>()
            .union(&invisible_interupted_explicit_broadcasts)
            .cloned()
            .collect::<HashSet<_>>();

        println!(
            "&(D)ropped&{}&{}&{}&{}&{}&{}\\\\",
            invisible_interupted_foreground_service.len(),
            invisible_interupted_background_service.len(),
            invisible_interupted_works.len(),
            invisible_interupted_implicit_broadcasts.len(),
            invisible_interupted_explicit_broadcasts.len(),
            invisible_interupted_any.len()
        );

        let invisible_common_any = invisible_established_any
            .intersection(&invisible_interupted_any)
            .cloned()
            .collect::<HashSet<_>>();

        println!(
            "&Common&{}&{}&{}&{}&{}&{}\\\\",
            invisible_common_foreground_service.len(),
            invisible_common_background_service.len(),
            invisible_common_works.len(),
            invisible_common_implicit_broadcasts.len(),
            invisible_common_explicit_broadcasts.len(),
            invisible_common_any.len()
        );

        let invisible_overal_any = invisible_established_any
            .union(&invisible_interupted_any)
            .cloned()
            .collect::<HashSet<_>>();

        println!(
            "&Total&{}&{}&{}&{}&{}&{}\\\\",
            invisible_overal_foreground_service.len(),
            invisible_overal_background_service.len(),
            invisible_overal_works.len(),
            invisible_overal_implicit_broadcasts.len(),
            invisible_overal_explicit_broadcasts.len(),
            invisible_overal_any.len()
        );

        let any_established_any = any_established_foreground_service
            .union(&any_established_background_service)
            .cloned()
            .collect::<HashSet<_>>()
            .union(&any_established_works.clone())
            .cloned()
            .collect::<HashSet<_>>()
            .union(&any_established_implicit_broadcasts)
            .cloned()
            .collect::<HashSet<_>>()
            .union(&any_established_explicit_broadcasts)
            .cloned()
            .collect::<HashSet<_>>();

        println!(
            "&(E)stablished&{}&{}&{}&{}&{}&{}\\\\",
            any_established_foreground_service.len(),
            any_established_background_service.len(),
            any_established_works.len(),
            any_established_implicit_broadcasts.len(),
            any_established_explicit_broadcasts.len(),
            any_established_any.len()
        );

        let any_interupted_any = any_interupted_foreground_service
            .union(&any_interupted_background_service)
            .cloned()
            .collect::<HashSet<_>>()
            .union(&any_interupted_works.clone())
            .cloned()
            .collect::<HashSet<_>>()
            .union(&any_interupted_implicit_broadcasts)
            .cloned()
            .collect::<HashSet<_>>()
            .union(&any_interupted_explicit_broadcasts)
            .cloned()
            .collect::<HashSet<_>>();

        println!(
            "&(D)ropped&{}&{}&{}&{}&{}&{}\\\\",
            any_interupted_foreground_service.len(),
            any_interupted_background_service.len(),
            any_interupted_works.len(),
            any_interupted_implicit_broadcasts.len(),
            any_interupted_explicit_broadcasts.len(),
            any_interupted_any.len()
        );

        let any_common_any = any_interupted_any
            .intersection(&any_established_any)
            .cloned()
            .collect::<HashSet<_>>();

        println!(
            "&Common&{}&{}&{}&{}&{}&{}\\\\",
            any_common_foreground_service.len(),
            any_common_background_service.len(),
            any_common_works.len(),
            any_common_implicit_broadcasts.len(),
            any_common_explicit_broadcasts.len(),
            any_common_any.len()
        );

        let any_overal_any = any_interupted_any
            .union(&any_established_any)
            .cloned()
            .collect::<HashSet<_>>();
        println!(
            "&Total&{}&{}&{}&{}&{}&{}\\\\",
            any_overal_foreground_service.len(),
            any_overal_background_service.len(),
            any_overal_works.len(),
            any_overal_implicit_broadcasts.len(),
            any_overal_explicit_broadcasts.len(),
            any_overal_any.len()
        );
        any_overal_any_plus_invisible.extend(any_overal_any.iter().map(|x| x.to_string()));
        apps_establishing_connection_services
            .extend(any_overal_foreground_service.iter().map(|x| x.to_string()));
        apps_establishing_connection_services
            .extend(any_overal_background_service.iter().map(|x| x.to_string()));
        apps_establishing_connection_broadcasts
            .extend(any_overal_implicit_broadcasts.iter().map(|x| x.to_string()));
        apps_establishing_connection_broadcasts
            .extend(any_overal_explicit_broadcasts.iter().map(|x| x.to_string()));

        println!(
            " system_broadcasts:{}",
            system_broadcasts
                .iter()
                .filter(|x| any_overal_explicit_broadcasts.contains(x))
                .count()
        );

        // any_overal_explicit_broadcasts.
        // println!("user_broadcasts :{}",system_broadcasts.difference(&any_overal_explicit_broadcasts.into_iter().map(|x|x.to_string()).collect::<HashSet<String>>()).count());
        // system_broadcasts.difference(&any_overal_explicit_broadcasts.into_iter().map(|x|x.to_string()).collect::<HashSet<String>>()).count();
        println!(
            "visible_overal_works:{},invisible_overal_works:{}",
            visible_overal_works.len(),
            invisible_overal_works.len()
        );
        // dbg!(visible_overal_any.len(), invisible_overal_any.len());
        println!(
            "visible_overal_any:{},invisible_overal_any:{}",
            visible_overal_any.len(),
            invisible_overal_any.len()
        );
        // dbg!(visible_overal_any.difference(&visible_overal_works).count());
        println!(
            "visible_overal_any - visible_overal_works:{}",
            visible_overal_any.difference(&visible_overal_works).count()
        );
        // dbg!(visible_overal_works.difference(&visible_overal_any).count());
        println!(
            "visible_overal_works - visible_overal_any:{}",
            visible_overal_works.difference(&visible_overal_any).count()
        );
        // dbg!(invisible_overal_any
        //     .difference(&invisible_overal_works)
        //     .count());
        println!(
            "invisible_overal_any - invisible_overal_works:{}",
            invisible_overal_any
                .difference(&invisible_overal_works)
                .count()
        );
        // dbg!(invisible_overal_works
        //     .difference(&invisible_overal_any)
        //     .count());
        println!(
            "invisible_overal_works - invisible_overal_any:{}",
            invisible_overal_works
                .difference(&invisible_overal_any)
                .count()
        );
        let visible_exclusive_foreground_service = visible_overal_foreground_service
            .difference(&invisible_overal_foreground_service)
            .collect::<HashSet<_>>();
        let visible_exclusive_background_service = visible_overal_background_service
            .difference(&invisible_overal_background_service)
            .collect::<HashSet<_>>();
        let visible_exclusive_works = visible_overal_works
            .difference(&invisible_overal_works)
            .collect::<HashSet<_>>();
        let visible_exclusive_implicit_broadcasts = visible_overal_implicit_broadcasts
            .difference(&invisible_overal_implicit_broadcasts)
            .collect::<HashSet<_>>();
        let visible_exclusive_explicit_broadcasts = visible_overal_explicit_broadcasts
            .difference(&invisible_overal_explicit_broadcasts)
            .collect::<HashSet<_>>();
        let visible_exclusive_any = visible_exclusive_background_service
            .union(&visible_exclusive_foreground_service)
            .cloned()
            .collect::<HashSet<_>>()
            .union(&visible_exclusive_works)
            .cloned()
            .collect::<HashSet<_>>()
            .union(&visible_exclusive_implicit_broadcasts)
            .cloned()
            .collect::<HashSet<_>>()
            .union(&visible_exclusive_explicit_broadcasts)
            .cloned()
            .collect::<HashSet<_>>();
        println!(
            "Visible Exclusive&{}&{}&{}&{}&{}&{}\\\\",
            visible_exclusive_foreground_service.len(),
            visible_exclusive_background_service.len(),
            visible_exclusive_works.len(),
            visible_exclusive_implicit_broadcasts.len(),
            visible_exclusive_explicit_broadcasts.len(),
            visible_exclusive_any.len()
        );

        let invisible_exclusive_foreground_service = invisible_overal_foreground_service
            .difference(&visible_overal_foreground_service)
            .collect::<HashSet<_>>();
        let invisible_exclusive_background_service = invisible_overal_background_service
            .difference(&visible_overal_background_service)
            .collect::<HashSet<_>>();
        let invisible_exclusive_works = invisible_overal_works
            .difference(&visible_overal_works)
            .collect::<HashSet<_>>();
        let invisible_exclusive_implicit_broadcasts = invisible_overal_implicit_broadcasts
            .difference(&visible_overal_implicit_broadcasts)
            .collect::<HashSet<_>>();
        let invisible_exclusive_explicit_broadcasts = invisible_overal_explicit_broadcasts
            .difference(&visible_overal_explicit_broadcasts)
            .collect::<HashSet<_>>();
        let invisible_exclusive_any = invisible_exclusive_background_service
            .union(&invisible_exclusive_foreground_service)
            .cloned()
            .collect::<HashSet<_>>()
            .union(&invisible_exclusive_works)
            .cloned()
            .collect::<HashSet<_>>()
            .union(&invisible_exclusive_implicit_broadcasts)
            .cloned()
            .collect::<HashSet<_>>()
            .union(&invisible_exclusive_explicit_broadcasts)
            .cloned()
            .collect::<HashSet<_>>();
        println!(
            "Invisible Exclusive&{}&{}&{}&{}&{}&{}\\\\",
            invisible_exclusive_foreground_service.len(),
            invisible_exclusive_background_service.len(),
            invisible_exclusive_works.len(),
            invisible_exclusive_implicit_broadcasts.len(),
            invisible_exclusive_explicit_broadcasts.len(),
            invisible_exclusive_any.len()
        );
    }
    println!("------");
    for (n, r) in jobs_table.iter() {
        println!(
            "{}&{}&{}&{}&{}&{}&{}&{}&{}&{}",
            n,
            r[0].len(),
            r[1].len(),
            r[2].len(),
            r[3].len(),
            r[4].len(),
            r[5].len(),
            r[6].len(),
            r[7].len(),
            r[8].len()
        );
    }
    for (b, n) in receivers_table.iter() {
        let unique = n.iter().map(|(x, _)| x).collect::<HashSet<_>>();
        let imp = if NEW_IMPLICIT.contains(&b.as_str()) || IMPLICIT.contains(&b.as_str()) {
            true
        } else {
            if n.len() < 10 {
                continue;
            }
            false
        };
        println!("{} - {} {}", b, unique.len(), imp);
    }
    for (work_type, apps) in work_parent {
        println!(
            "{}&{}&{}&{}&{}",
            work_type,
            apps[0].len(),
            apps[1].len(),
            apps[2].len(),
            apps[3].len()
        );
    }
    // dbg!(parents_same_parent);
    println!("Parents with the same parent {:?}", parents_same_parent);

    println!("------");
    // any_overal_foreground_service.len(),
    // any_overal_background_service.len(),
    // any_overal_works.len(),
    // any_overal_implicit_broadcasts.len(),
    // any_overal_explicit_broadcasts.len(),

    println!(
        "Apps Established Connections {}",
        apps_establishing_connection.len()
    );
    println!(
        "Apps Established Connections Visible {}",
        apps_establishing_connection_visible.len()
    );
    println!(
        "Apps Established Connections Invisible {}",
        apps_establishing_connection_invisible.len()
    );
    println!(
        "Apps Established Connections Background {}",
        any_overal_any_plus_invisible.len()
    );

    any_overal_any_plus_invisible.extend(
        apps_establishing_connection_invisible
            .iter()
            .map(|x| x.to_string()),
    );

    // let any_overal_any_plus_invisible= any_overal_any.union(&invisible_overal_any).collect::<HashSet<_>>();
    println!(
        "Apps Established Connections background + invisible {}",
        any_overal_any_plus_invisible.len()
    );
    println!(
        "Apps Established Connections Services {}",
        apps_establishing_connection_services.len()
    );
    println!(
        "Apps Established Connections Broadcasts {}",
        apps_establishing_connection_broadcasts.len()
    );

    println!(
        "Apps explicit intent from implicit {}",
        apps_explicit_intent_from_implicit.len()
    );
    println!(
        "Apps explicit intent from any {}",
        apps_explicit_intent_from_any.len()
    );
    println!(
        "Apps explicit intent from service {}",
        apps_explicit_intent_from_service.len()
    );

    println!(
        "Apps explicit intent from jobs {}",
        apps_explicit_intent_from_jobs.len()
    );

    println!(
        "Apps successfuly established at least one connection {}",
        apps_successfuly_established_at_least_one_conenction.len()
    );
    println!("Domains with at least 10 apps - Wroks",);
    let mut s = HashSet::new();
    for (k, v) in &domains_apps_works {
        if v.len() > 10 {
            if domains_apps_visible.keys().any(|x| x == k)
                || domains_apps_invisible.keys().any(|x| x == k) && k != "0.0.0.0:80"
            {
                continue;
            }
            println!("{} - {}", v.len(), k);
            s.extend(v.iter().map(|x| x.to_string()));
        }
    }
    let mut all_thrid_party = HashSet::new();
    let mut invisible_third_party = HashSet::new();
    let mut all_invisible_successful_connection = HashSet::new();
    let mut visible_third_party = HashSet::new();
    let mut all_visible_successful_connection = HashSet::new();
    let mut works_third_party = HashSet::new();
    let mut all_works_successful_connection = HashSet::new();
    let mut invisible_work_third_party = HashSet::new();
    let mut all_invisible_works_successful_connection = HashSet::new();
    for (k, v) in &domains_apps_visible {
        all_visible_successful_connection.extend(v.iter().map(|x| x.to_string()));
        if v.len() >= 10 {
            all_thrid_party.extend(v.iter().map(|x| x.to_string()));
            visible_third_party.extend(v.iter().map(|x| x.to_string()));
            // println!("{} - {}", v.len(),k);
        }
    }
    for (k, v) in &domains_apps_invisible {
        all_invisible_successful_connection.extend(v.iter().map(|x| x.to_string()));
        all_invisible_works_successful_connection.extend(v.iter().map(|x| x.to_string()));
        if v.len() >= 10 {
            all_thrid_party.extend(v.iter().map(|x| x.to_string()));
            invisible_third_party.extend(v.iter().map(|x| x.to_string()));
            invisible_work_third_party.extend(v.iter().map(|x| x.to_string()));
            // println!("{} - {}", v.len(),k);
        }
    }
    for (k, v) in &domains_apps_works {
        all_works_successful_connection.extend(v.iter().map(|x| x.to_string()));
        all_invisible_works_successful_connection.extend(v.iter().map(|x| x.to_string()));
        if v.len() >= 10 {
            all_thrid_party.extend(v.iter().map(|x| x.to_string()));
            works_third_party.extend(v.iter().map(|x| x.to_string()));
            invisible_work_third_party.extend(v.iter().map(|x| x.to_string()));
            // println!("{} - {}", v.len(),k);
        }
    }
    println!("All Third Party {}", all_thrid_party.len());
    println!("Invisible Third Party {}", invisible_third_party.len());
    println!("Visible Third Party {}", visible_third_party.len());
    println!("Works Third Party {}", works_third_party.len());
    println!(
        "Invisible works Third Party {}",
        invisible_work_third_party.len()
    );
    println!(
        "All Invisible Successful Connection {}",
        all_invisible_successful_connection.len()
    );
    println!(
        "All Visible Successful Connection {}",
        all_visible_successful_connection.len()
    );
    println!(
        "All Works Successful Connection {}",
        all_works_successful_connection.len()
    );
    println!(
        "All Invisible Works Successful Connection {}",
        all_invisible_works_successful_connection.len()
    );
    println!("Domains {}", s.len());
    println!("Domains with at least 10 apps - Invisible",);
    let mut third_party_invisible = Vec::new();
    let mut third_party_visible = Vec::new();
    let mut third_party_works = Vec::new();
    let mut third_party_any = Vec::new();

    let mut use_google_domain_visible = HashSet::new();
    let mut use_google_domain_invisible = HashSet::new();
    let mut use_google_domain_works = HashSet::new();
    let mut use_google_domain_any = HashSet::new();
    let mut use_facebook_domain_visible = HashSet::new();
    let mut use_facebook_domain_invisible = HashSet::new();
    let mut use_facebook_domain_works = HashSet::new();
    let mut use_facebook_domain_any = HashSet::new();
    let mut use_amazon_domain_visible = HashSet::new();
    let mut use_amazon_domain_invisible = HashSet::new();
    let mut use_amazon_domain_works = HashSet::new();
    let mut use_amazon_domain_any = HashSet::new();
    for (k, v) in &domains_apps_invisible {
        if v.len() > 10 {
            if k.is_empty() {
                continue;
            }
            let k = k.rsplit_once(":").map(|x| x.0).unwrap_or(k);
            if GOOGLE_DOMAINS.contains(&k)
                || GOOGLE_DOMAINS
                    .iter()
                    .any(|x| k.ends_with(format!(".{}", x).as_str()))
            {
                use_google_domain_invisible.extend(v.iter().map(|x| x.to_string()));
            }
            if FACEBOOK_DOMAINS.contains(&k)
                || FACEBOOK_DOMAINS
                    .iter()
                    .any(|x| k.ends_with(format!(".{}", x).as_str()))
            {
                use_facebook_domain_invisible.extend(v.iter().map(|x| x.to_string()));
            }
            if AMAZON_DOMAINS.contains(&k)
                || AMAZON_DOMAINS
                    .iter()
                    .any(|x| k.ends_with(format!(".{}", x).as_str()))
            {
                use_amazon_domain_invisible.extend(v.iter().map(|x| x.to_string()));
            }
            third_party_invisible.push((k, v.len()));
            // println!("{} - {}", v.len(),k);
        }
    }
    println!("Domains with at least 10 apps - Visible",);
    for (k, v) in &domains_apps_visible {
        if v.len() > 10 {
            if k.is_empty() {
                continue;
            }
            let k = k.rsplit_once(":").map(|x| x.0).unwrap_or(k);
            if GOOGLE_DOMAINS.contains(&k)
                || GOOGLE_DOMAINS
                    .iter()
                    .any(|x| k.ends_with(format!(".{}", x).as_str()))
            {
                use_google_domain_visible.extend(v.iter().map(|x| x.to_string()));
            }
            if FACEBOOK_DOMAINS.contains(&k)
                || FACEBOOK_DOMAINS
                    .iter()
                    .any(|x| k.ends_with(format!(".{}", x).as_str()))
            {
                use_facebook_domain_visible.extend(v.iter().map(|x| x.to_string()));
            }
            if AMAZON_DOMAINS.contains(&k)
                || AMAZON_DOMAINS
                    .iter()
                    .any(|x| k.ends_with(format!(".{}", x).as_str()))
            {
                use_amazon_domain_visible.extend(v.iter().map(|x| x.to_string()));
            }
            // println!("{} - {}", v.len(),k);
            third_party_visible.push((k, v.len()));
        }
    }
    let mut first_party_works = HashSet::new();
    println!("Domains with at least 10 apps - Works",);
    for (k, v) in &domains_apps_works {
        if v.len() > 10 {
            if k.is_empty() {
                continue;
            }
            let k = k.rsplit_once(":").map(|x| x.0).unwrap_or(k);
            if GOOGLE_DOMAINS.contains(&k)
                || GOOGLE_DOMAINS
                    .iter()
                    .any(|x| k.ends_with(format!(".{}", x).as_str()))
            {
                use_google_domain_works.extend(v.iter().map(|x| x.to_string()));
            }
            if FACEBOOK_DOMAINS.contains(&k)
                || FACEBOOK_DOMAINS
                    .iter()
                    .any(|x| k.ends_with(format!(".{}", x).as_str()))
            {
                use_facebook_domain_works.extend(v.iter().map(|x| x.to_string()));
            }
            if AMAZON_DOMAINS.contains(&k)
                || AMAZON_DOMAINS
                    .iter()
                    .any(|x| k.ends_with(format!(".{}", x).as_str()))
            {
                use_amazon_domain_works.extend(v.iter().map(|x| x.to_string()));
            }
            // println!("{} - {}", v.len(),k);
            third_party_works.push((k, v.len()));
        } else {
            first_party_works.insert(k);
        }
    }
    println!("Domains with at least 10 apps - Any",);
    for (k, v) in &domains_apps_any {
        if v.len() > 10 {
            if k.is_empty() {
                continue;
            }
            let k = k.rsplit_once(":").map(|x| x.0).unwrap_or(k);
            if GOOGLE_DOMAINS.contains(&k)
                || GOOGLE_DOMAINS
                    .iter()
                    .any(|x| k.ends_with(format!(".{}", x).as_str()))
            {
                use_google_domain_any.extend(v.iter().map(|x| x.to_string()));
            }
            if FACEBOOK_DOMAINS.contains(&k)
                || FACEBOOK_DOMAINS
                    .iter()
                    .any(|x| k.ends_with(format!(".{}", x).as_str()))
            {
                use_facebook_domain_any.extend(v.iter().map(|x| x.to_string()));
            }
            if AMAZON_DOMAINS.contains(&k)
                || AMAZON_DOMAINS
                    .iter()
                    .any(|x| k.ends_with(format!(".{}", x).as_str()))
            {
                use_amazon_domain_any.extend(v.iter().map(|x| x.to_string()));
            }
            // println!("{} - {}", v.len(),k);
            third_party_any.push((k, v.len()));
        }
    }
    third_party_invisible.sort_by(|a, b| b.1.cmp(&a.1));
    third_party_visible.sort_by(|a, b| b.1.cmp(&a.1));
    third_party_works.sort_by(|a, b| b.1.cmp(&a.1));
    third_party_any.sort_by(|a, b| b.1.cmp(&a.1));

    println!("work first party {}", first_party_works.len());

    // for (k,v) in third_party_invisible{

    //     println!("{} - {}", v,k);
    // }

    println!(
        "Google Domain Invisible {}",
        use_google_domain_invisible.len()
    );
    println!("Google Domain Visible {}", use_google_domain_visible.len());
    println!("Google Domain Works {}", use_google_domain_works.len());
    println!("Google Domain Any {}", use_google_domain_any.len());
    println!(
        "Facebook Domain Invisible {}",
        use_facebook_domain_invisible.len()
    );
    println!(
        "Facebook Domain Visible {}",
        use_facebook_domain_visible.len()
    );
    println!("Facebook Domain Works {}", use_facebook_domain_works.len());
    println!("Facebook Domain Any {}", use_facebook_domain_any.len());
    println!(
        "Amazon Domain Invisible {}",
        use_amazon_domain_invisible.len()
    );
    println!("Amazon Domain Visible {}", use_amazon_domain_visible.len());
    println!("Amazon Domain Works {}", use_amazon_domain_works.len());
    println!("Amazon Domain Any {}", use_amazon_domain_any.len());
    // for i in 0..21 {
    //     println!(
    //         "{}&{}&{}&{}&{}&{}&{}&{}\\\\",
    //         third_party_visible[i].0,
    //         third_party_visible[i].1,
    //         third_party_invisible[i].0,
    //         third_party_invisible[i].1,
    //         third_party_works[i].0,
    //         third_party_works[i].1,
    //         third_party_any[i].0,
    //         third_party_any[i].1
    //     )
    // }

    for i in 0..21 {
        println!(
            "{}&{}&{}&{}\\\\",
            check_google_facebook_domain_generate_output(
                third_party_visible[i].0,
                third_party_visible[i].1
            ),
            check_google_facebook_domain_generate_output(
                third_party_invisible[i].0,
                third_party_invisible[i].1
            ),
            check_google_facebook_domain_generate_output(
                third_party_works[i].0,
                third_party_works[i].1
            ),
            check_google_facebook_domain_generate_output(
                third_party_any[i].0,
                third_party_any[i].1
            )
        )
    }
    Ok(())
}

fn check_google_facebook_domain_generate_output(domain: &str, len: usize) -> String {
    if GOOGLE_DOMAINS.contains(&domain)
        || GOOGLE_DOMAINS
            .iter()
            .any(|x| domain.ends_with(format!(".{}", x).as_str()))
    {
        return format!("\\google{{{}}}&\\google{{{}}}", domain, len);
    }
    if FACEBOOK_DOMAINS.contains(&domain)
        || FACEBOOK_DOMAINS
            .iter()
            .any(|x| domain.ends_with(format!(".{}", x).as_str()))
    {
        return format!("\\facebook{{{}}}&\\facebook{{{}}}", domain, len);
    }
    if AMAZON_DOMAINS.contains(&domain)
        || AMAZON_DOMAINS
            .iter()
            .any(|x| domain.ends_with(format!(".{}", x).as_str()))
    {
        return format!("\\amazon{{{}}}&\\amazon{{{}}}", domain, len);
    }
    return format!("{}&{}", domain, len);
}

#[derive(Debug, Clone, Default)]
pub struct ResultData {
    connections: Vec<NetworkConnection>,
    works: Vec<BackgroundWork>,
    runtime_registered: Vec<RuntimeRegisteredReceiver>,
}

pub enum WorkType {
    Time,
    Event,
    EventTime,
    Persistent,
}

const FACEBOOK_DOMAINS: [&str; 395] = [
    "aboutfacebook.com",
    "accessfacebookfromschool.com",
    "acebooik.com",
    "acebook.com",
    "advancediddetection.com",
    "askfacebook.net",
    "askfacebook.org",
    "atdmt2.com",
    "atlasdmt.com",
    "atlasonepoint.com",
    "buyingfacebooklikes.com",
    "careersatfb.com",
    "celebgramme.com",
    "china-facebook.com",
    "click-url.com",
    "como-hackearfacebook.com",
    "crowdtangle.com",
    "dacebook.com",
    "dlfacebook.com",
    "dotfacebook.com",
    "dotfacebook.net",
    "expresswifi.com",
    "faacebok.com",
    "faacebook.com",
    "faasbook.com",
    "facbebook.com",
    "facbeok.com",
    "facboo.com",
    "facbook.com",
    "facbool.com",
    "facboox.com",
    "faccebook.com",
    "faccebookk.com",
    "facdbook.com",
    "facdebook.com",
    "face-book.com",
    "faceabook.com",
    "facebboc.com",
    "facebbook.com",
    "facebboook.com",
    "facebcook.com",
    "facebdok.com",
    "facebgook.com",
    "facebhook.com",
    "facebkkk.com",
    "facebo-ok.com",
    "faceboak.com",
    "facebock.com",
    "facebocke.com",
    "facebof.com",
    "faceboik.com",
    "facebok.com",
    "facebokbook.com",
    "facebokc.com",
    "facebokk.com",
    "facebokok.com",
    "faceboks.com",
    "facebol.com",
    "facebolk.com",
    "facebomok.com",
    "faceboo.com",
    "facebooa.com",
    "faceboob.com",
    "faceboobok.com",
    "facebooc.com",
    "faceboock.com",
    "facebood.com",
    "facebooe.com",
    "faceboof.com",
    "facebooi.com",
    "facebooik.com",
    "facebooik.org",
    "facebooj.com",
    "facebook-corp.com",
    "facebook-covid-19.com",
    "facebook-ebook.com",
    "facebook-forum.com",
    "facebook-hardware.com",
    "facebook-inc.com",
    "facebook-login.com",
    "facebook-newsroom.com",
    "facebook-newsroom.org",
    "facebook-pmdcenter.com",
    "facebook-pmdcenter.net",
    "facebook-pmdcenter.org",
    "facebook-privacy.com",
    "facebook-program.com",
    "facebook-studio.com",
    "facebook-support.org",
    "facebook-texas-holdem.com",
    "facebook-texas-holdem.net",
    "facebook.br",
    "facebook.ca",
    "facebook.cc",
    "facebook.com",
    "facebook.design",
    "facebook.net",
    "facebook.hu",
    "facebook.in",
    "fecebook.net",
    "facebook.nl",
    "facebook.org",
    "facebook.se",
    "facebook.shop",
    "facebook.tv",
    "facebook.us",
    "facebook.wang",
    "facebook123.org",
    "facebook30.com",
    "facebook30.net",
    "facebook30.org",
    "facebook4business.com",
    "facebookads.com",
    "facebookadvertisingsecrets.com",
    "facebookatschool.com",
    "facebookawards.com",
    "facebookblueprint.net",
    "facebookbrand.com",
    "facebookbrand.net",
    "facebookcanadianelectionintegrityinitiative.com",
    "facebookcareer.com",
    "facebookcheats.com",
    "facebookck.com",
    "facebookclub.com",
    "facebookcom.com",
    "facebookconsultant.org",
    "facebookcoronavirus.com",
    "facebookcovers.org",
    "facebookcredits.info",
    "facebookdating.net",
    "facebookdusexe.org",
    "facebookemail.com",
    "facebookenespanol.com",
    "facebookexchange.com",
    "facebookexchange.net",
    "facebookfacebook.com",
    "facebookflow.com",
    "facebookgames.com",
    "facebookgraphsearch.com",
    "facebookgraphsearch.info",
    "facebookgroups.com",
    "facebookhome.cc",
    "facebookhome.com",
    "facebookhome.info",
    "facebookhub.com",
    "facebooki.com",
    "facebookinc.com",
    "facebookland.com",
    "facebooklikeexchange.com",
    "facebooklive.com",
    "facebooklivestaging.net",
    "facebooklivestaging.org",
    "facebooklogin.com",
    "facebooklogin.info",
    "facebookloginhelp.net",
    "facebooklogs.com",
    "facebookmail.com",
    "facebookmail.tv",
    "facebookmanager.info",
    "facebookmarketing.info",
    "facebookmarketingpartner.com",
    "facebookmarketingpartners.com",
    "facebookmobile.com",
    "facebookmsn.com",
    "facebooknews.com",
    "facebooknfl.com",
    "facebooknude.com",
    "facebookofsex.com",
    "facebookook.com",
    "facebookpaper.com",
    "facebookpay.com",
    "facebookphonenumber.net",
    "facebookphoto.com",
    "facebookphotos.com",
    "facebookpmdcenter.com",
    "facebookpoke.net",
    "facebookpoke.org",
    "facebookpoker.info",
    "facebookpokerchips.info",
    "facebookporn.net",
    "facebookporn.org",
    "facebookporno.net",
    "facebookportal.com",
    "facebooks.com",
    "facebooksafety.com",
    "facebooksecurity.net",
    "facebookshop.com",
    "facebooksignup.net",
    "facebooksite.net",
    "facebookstories.com",
    "facebookstudios.net",
    "facebookstudios.org",
    "facebooksupplier.com",
    "facebooksuppliers.com",
    "facebookswagemea.com",
    "facebookswagstore.com",
    "facebooksz.com",
    "facebookthreads.net",
    "facebooktv.net",
    "facebooktv.org",
    "facebookvacation.com",
    "facebookw.com",
    "facebookworld.com",
    "facebool.com",
    "facebool.info",
    "facebooll.com",
    "faceboom.com",
    "faceboon.com",
    "faceboonk.com",
    "faceboooik.com",
    "faceboook.com",
    "faceboop.com",
    "faceboot.com",
    "faceboox.com",
    "facebopk.com",
    "facebpook.com",
    "facebuk.com",
    "facebuok.com",
    "facebvook.com",
    "facebyook.com",
    "facebzook.com",
    "facecbgook.com",
    "facecbook.com",
    "facecbook.org",
    "facecook.com",
    "facecook.org",
    "facedbook.com",
    "faceebok.com",
    "faceebook.com",
    "faceebot.com",
    "facegbok.com",
    "facegbook.com",
    "faceobk.com",
    "faceobok.com",
    "faceobook.com",
    "faceook.com",
    "facerbooik.com",
    "facerbook.com",
    "facesbooc.com",
    "facesounds.com",
    "facetook.com",
    "facevbook.com",
    "facewbook.co",
    "facewook.com",
    "facfacebook.com",
    "facfebook.com",
    "fackebook.com",
    "facnbook.com",
    "facrbook.com",
    "facvebook.com",
    "facwebook.com",
    "facxebook.com",
    "fadebook.com",
    "faebok.com",
    "faebook.com",
    "faebookc.com",
    "faeboook.com",
    "faecebok.com",
    "faesebook.com",
    "fafacebook.com",
    "faicbooc.com",
    "fasebokk.com",
    "fasebook.com",
    "faseboox.com",
    "favebook.com",
    "faycbok.com",
    "fb.careers",
    "fb.com",
    "fb.gg",
    "fb.me",
    "fb.watch",
    "fbacebook.com",
    "fbbmarket.com",
    "fbboostyourbusiness.com",
    "fbcdn.com",
    "fbcdn.net",
    "fbfeedback.com",
    "fbhome.com",
    "fbidb.io",
    "fbinc.com",
    "fbinnovation.com",
    "fbmarketing.com",
    "fbreg.com",
    "fbrpms.com",
    "fbsbx.com",
    "fbsbx.net",
    "fbsupport-covid.net",
    "fbthirdpartypixel.com",
    "fbthirdpartypixel.net",
    "fbthirdpartypixel.org",
    "fburl.com",
    "fbwat.ch",
    "fbworkmail.com",
    "fcacebook.com",
    "fcaebook.com",
    "fcebook.com",
    "fcebookk.com",
    "fcfacebook.com",
    "fdacebook.info",
    "feacboo.com",
    "feacbook.com",
    "feacbooke.com",
    "feacebook.com",
    "fecbbok.com",
    "fecbooc.com",
    "fecbook.com",
    "feceboock.com",
    "feceboox.com",
    "fececbook.com",
    "feook.com",
    "ferabook.com",
    "fescebook.com",
    "fesebook.com",
    "ffacebook.com",
    "fgacebook.com",
    "ficeboock.com",
    "fmcebook.com",
    "fnacebook.com",
    "fosebook.com",
    "fpacebook.com",
    "fqcebook.com",
    "fracebook.com",
    "freeb.com",
    "freebasics.com",
    "freebasics.net",
    "freebs.com",
    "freefacebook.com",
    "freefacebook.net",
    "freefacebookads.net",
    "freefblikes.com",
    "freindfeed.com",
    "friendbook.info",
    "friendfed.com",
    "friendfeed-api.com",
    "friendfeed-media.com",
    "friendfeed.com",
    "friendfeedmedia.com",
    "fsacebok.com",
    "fscebook.com",
    "fundraisingwithfacebook.com",
    "funnyfacebook.org",
    "futureofbusinesssurvey.org",
    "gacebook.com",
    "gameroom.com",
    "gfacecbook.com",
    "groups.com",
    "hackerfacebook.com",
    "hackfacebook.com",
    "hackfacebookid.com",
    "hifacebook.info",
    "howtohackfacebook-account.com",
    "hsfacebook.com",
    "httpfacebook.com",
    "httpsfacebook.com",
    "httpwwwfacebook.com",
    "i.org",
    "internet.org",
    "klik.me",
    "liverail.com",
    "liverail.tv",
    "login-account.net",
    "markzuckerberg.com",
    "midentsolutions.com",
    "mobilefacebook.com",
    "moneywithfacebook.com",
    "myfbfans.com",
    "newsfeed.com",
    "nextstop.com",
    "online-deals.net",
    "opencreate.org",
    "reachtheworldonfacebook.com",
    "rocksdb.org",
    "shopfacebook.com",
    "sportsfacebook.com",
    "sportstream.com",
    "supportfacebook.com",
    "terragraph.com",
    "thefacebook.com",
    "thefacebook.net",
    "thefind.com",
    "toplayerserver.com",
    "viewpointsfromfacebook.com",
    "whyfacebook.com",
    "worldhack.com",
    "www-facebook.com",
    "wwwfacebok.com",
    "wwwfacebook.com",
    "wwwmfacebook.com",
    "zuckerberg.com",
    "zuckerberg.net",
    "facebookwork.com",
    "redkix.com",
    "workplace.com",
    "workplaceusecases.com",
    "fbcdn-a.akamaihd.net",
];
const GOOGLE_DOMAINS: [&str; 663] = [
    "google.ad",
    "google.ae",
    "google.al",
    "google.am",
    "google.as",
    "google.at",
    "google.az",
    "google.ba",
    "google.be",
    "google.bf",
    "google.bg",
    "google.bi",
    "google.bj",
    "google.bs",
    "google.bt",
    "google.by",
    "google.ca",
    "google.cat",
    "google.cd",
    "google.cf",
    "google.cg",
    "google.ch",
    "google.ci",
    "google.cl",
    "google.cm",
    "google.cn",
    "google.co",
    "google.co.ao",
    "google.co.bw",
    "google.co.ck",
    "google.co.cr",
    "google.co.id",
    "google.co.il",
    "google.co.in",
    "google.co.jp",
    "google.co.ke",
    "google.co.kr",
    "google.co.ls",
    "google.co.ma",
    "google.co.mz",
    "google.co.nz",
    "google.co.th",
    "google.co.tz",
    "google.co.ug",
    "google.co.uk",
    "google.co.uz",
    "google.co.ve",
    "google.co.vi",
    "google.co.za",
    "google.co.zm",
    "google.co.zw",
    "google.com",
    "google.com.af",
    "google.com.ag",
    "google.com.ai",
    "google.com.ar",
    "google.com.au",
    "google.com.bd",
    "google.com.bh",
    "google.com.bn",
    "google.com.bo",
    "google.com.br",
    "google.com.bz",
    "google.com.co",
    "google.com.cu",
    "google.com.cy",
    "google.com.do",
    "google.com.ec",
    "google.com.eg",
    "google.com.et",
    "google.com.fj",
    "google.com.gh",
    "google.com.gi",
    "google.com.gt",
    "google.com.hk",
    "google.com.jm",
    "google.com.kh",
    "google.com.kw",
    "google.com.lb",
    "google.com.ly",
    "google.com.mm",
    "google.com.mt",
    "google.com.mx",
    "google.com.my",
    "google.com.na",
    "google.com.ng",
    "google.com.ni",
    "google.com.np",
    "google.com.om",
    "google.com.pa",
    "google.com.pe",
    "google.com.pg",
    "google.com.ph",
    "google.com.pk",
    "google.com.pr",
    "google.com.py",
    "google.com.qa",
    "google.com.sa",
    "google.com.sb",
    "google.com.sg",
    "google.com.sl",
    "google.com.sv",
    "google.com.tj",
    "google.com.tr",
    "google.com.tw",
    "google.com.ua",
    "google.com.uy",
    "google.com.vc",
    "google.com.vn",
    "google.cv",
    "google.cz",
    "google.de",
    "google.dj",
    "google.dk",
    "google.dm",
    "google.dz",
    "google.ee",
    "google.es",
    "google.fi",
    "google.fm",
    "google.fr",
    "google.ga",
    "google.ge",
    "google.gg",
    "google.gl",
    "google.gm",
    "google.gr",
    "google.gy",
    "google.hn",
    "google.hr",
    "google.ht",
    "google.hu",
    "google.ie",
    "google.im",
    "google.iq",
    "google.is",
    "google.it",
    "google.je",
    "google.jo",
    "google.kg",
    "google.ki",
    "google.kz",
    "google.la",
    "google.li",
    "google.lk",
    "google.lt",
    "google.lu",
    "google.lv",
    "google.md",
    "google.me",
    "google.mg",
    "google.mk",
    "google.ml",
    "google.mn",
    "google.ms",
    "google.mu",
    "google.mv",
    "google.mw",
    "google.ne",
    "google.nl",
    "google.no",
    "google.nr",
    "google.nu",
    "google.pl",
    "google.pn",
    "google.ps",
    "google.pt",
    "google.ro",
    "google.rs",
    "google.ru",
    "google.rw",
    "google.sc",
    "google.se",
    "google.sh",
    "google.si",
    "google.sk",
    "google.sm",
    "google.sn",
    "google.so",
    "google.sr",
    "google.st",
    "google.td",
    "google.tg",
    "google.tl",
    "google.tm",
    "google.tn",
    "google.to",
    "google.tt",
    "google.vg",
    "google.vu",
    "google.ws",
    "0emm.com",
    "1e100.net",
    "1ucrs.com",
    "466453.com",
    "abc.xyz",
    "adgoogle.net",
    "admeld.com",
    "angulardart.org",
    "api.ai",
    "apigee.com",
    "appbridge.ca",
    "appbridge.io",
    "appbridge.it",
    "appspot.com",
    "apture.com",
    "area120.com",
    "asp-cc.com",
    "autodraw.com",
    "bandpage.com",
    "baselinestudy.com",
    "baselinestudy.org",
    "bazel.build",
    "bdn.dev",
    "beatthatquote.com",
    "blink.org",
    "brocaproject.com",
    "brotli.org",
    "bumpshare.com",
    "bumptop.ca",
    "bumptop.com",
    "bumptop.net",
    "bumptop.org",
    "bumptunes.com",
    "campuslondon.com",
    "certificate-transparency.org",
    "chrome.com",
    "chromebook.com",
    "chromecast.com",
    "chromeos.dev",
    "chromium.org",
    "chronicle.security",
    "chroniclesec.com",
    "cloudburstresearch.com",
    "cloudfunctions.net",
    "cloudproxy.app",
    "cloudrobotics.com",
    "cobrasearch.com",
    "codespot.com",
    "conscrypt.com",
    "conscrypt.org",
    "cookiechoices.org",
    "coova.com",
    "coova.net",
    "coova.org",
    "crossmediapanel.com",
    "cs4hs.com",
    "dartsearch.net",
    "dataliberation.org",
    "debug.com",
    "debugproject.com",
    "devsitetest.how",
    "dialogflow.com",
    "digitalassetlinks.org",
    "docs.new",
    "episodic.com",
    "feedburner.com",
    "fflick.com",
    "financeleadsonline.com",
    "fitbit.com",
    "foofle.com",
    "froogle.com",
    "fuchsia.dev",
    "g-tun.com",
    "g.cn",
    "g.co",
    "g.dev",
    "g.page",
    "gateway.dev",
    "gcr.io",
    "gerritcodereview.com",
    "getbumptop.com",
    "ggoogle.com",
    "gipscorp.com",
    "gkecnapps.cn",
    "globaledu.org",
    "gmail.com",
    "gmodules.com",
    "gogle.com",
    "gogole.com",
    "gonglchuangl.net",
    "goo.gl",
    "googel.com",
    "googil.com",
    "googl.com",
    "google-access.net",
    "google-syndication.com",
    "google.berlin",
    "google.dev",
    "google.net",
    "google.org",
    "google.ventures",
    "googleacquisitionmigration.com",
    "googleapis.cn",
    "googleapis.com",
    "googleapps.com",
    "googlearth.com",
    "googleblog.com",
    "googlebot.com",
    "googlecapital.com",
    "googlecert.net",
    "googlecnapps.cn",
    "googlecode.com",
    "googlecommerce.com",
    "googlecompare.co.uk",
    "googledanmark.com",
    "googledomains.com",
    "googledrive.com",
    "googlee.com",
    "googleearth.com",
    "googlefiber.com",
    "googlefiber.net",
    "googlefinland.com",
    "googlemail.com",
    "googlemaps.com",
    "googlepagecreator.com",
    "googlephotos.com",
    "googleplus.com",
    "googlesource.com",
    "googlestore.com",
    "googlesverige.com",
    "googleusercontent.com",
    "googleventures.com",
    "googlezip.net",
    "googlr.com",
    "goolge.com",
    "gooogle.com",
    "gridaware.app",
    "gsrc.io",
    "gstatic.cn",
    "gstatic.com",
    "gstaticcnapps.cn",
    "gsuite.com",
    "gv.com",
    "gvt0.com",
    "gvt1.com",
    "gvt2.com",
    "gvt3.com",
    "gvt5.com",
    "gvt6.com",
    "gvt7.com",
    "gvt9.com",
    "haplorrhini.com",
    "hdrplusdata.org",
    "hindiweb.com",
    "howtogetmo.co.uk",
    "html5rocks.com",
    "hwgo.com",
    "iamremarkable.org",
    "idx.dev",
    "igoogle.com",
    "impermium.com",
    "itasoftware.com",
    "j2objc.org",
    "jibemobile.com",
    "keyhole.com",
    "keytransparency.com",
    "keytransparency.foo",
    "keytransparency.org",
    "lanternal.com",
    "like.com",
    "madewithcode.com",
    "material.io",
    "mdialog.com",
    "meet.new",
    "mfg-inspector.com",
    "mobileview.page",
    "moodstocks.com",
    "near.by",
    "nest.com",
    "nomulus.foo",
    "oauthz.com",
    "on.here",
    "on2.com",
    "onefifteen.net",
    "onefifteen.org",
    "oneworldmanystories.com",
    "openthread.io",
    "openweave.io",
    "orbitera.com",
    "page.link",
    "pagespeedmobilizer.com",
    "pageview.mobi",
    "panoramio.com",
    "partylikeits1986.org",
    "paxlicense.org",
    "picasa.com",
    "picasaweb.com",
    "picasaweb.net",
    "picasaweb.org",
    "picnik.com",
    "pittpatt.com",
    "pixate.com",
    "postini.com",
    "privacysandbox.com",
    "projectara.com",
    "projectbaseline.com",
    "publishproxy.com",
    "questvisual.com",
    "quickoffice.com",
    "quiksee.com",
    "revolv.com",
    "ridepenguin.com",
    "run.app",
    "savethedate.foo",
    "saynow.com",
    "schemer.com",
    "screenwisetrends.com",
    "screenwisetrendspanel.com",
    "sheets.new",
    "slides.new",
    "snapseed.com",
    "solveforx.com",
    "stadia.dev",
    "stcroixmosquito.com",
    "stcroixmosquitoproject.com",
    "studywatchbyverily.com",
    "studywatchbyverily.org",
    "stxmosquito.com",
    "stxmosquitoproject.com",
    "stxmosquitoproject.net",
    "stxmosquitoproject.org",
    "synergyse.com",
    "tensorflow.org",
    "tfhub.dev",
    "thecleversense.com",
    "thegooglestore.com",
    "thinkquarterly.co.uk",
    "thinkquarterly.com",
    "thinkwithgoogle.com",
    "tiltbrush.com",
    "txcloud.net",
    "txvia.com",
    "unfiltered.news",
    "useplannr.com",
    "usvimosquito.com",
    "usvimosquitoproject.com",
    "velostrata.com",
    "verily.com",
    "verilylifesciences.com",
    "verilystudyhub.com",
    "verilystudywatch.com",
    "verilystudywatch.org",
    "wallet.com",
    "waymo.com",
    "waze.com",
    "web.app",
    "web.dev",
    "webappfieldguide.com",
    "webmproject.org",
    "webpkgcache.com",
    "webrtc.org",
    "weltweitwachsen.de",
    "whatbrowser.org",
    "whats.new",
    "widevine.com",
    "withgoogle.com",
    "womenwill.com",
    "womenwill.com.br",
    "womenwill.id",
    "womenwill.in",
    "womenwill.mx",
    "x.company",
    "x.team",
    "xn--9kr7l.com",
    "xn--9trs65b.com",
    "xn--flw351e.com",
    "xn--ggle-55da.com",
    "xn--gogl-0nd52e.com",
    "xn--gogl-1nd42e.com",
    "xplr.co",
    "zukunftswerkstatt.de",
    "google-ohttp-relay-safebrowsing.fastly-edge.com",
    "www.recaptcha.net",
    "265.com",
    "2mdn-cn.net",
    "2mdn.net",
    "admob-cn.com",
    "adservice.google.com",
    "ai.ytimg.com",
    "app-measurement-cn.com",
    "app-measurement.com",
    "apps5.oingo.com",
    "avail.googleflights.net",
    "beacons.gcp.gvt2.com",
    "beacons.gvt2.com",
    "beacons2.gvt2.com",
    "beacons3.gvt2.com",
    "borgmon.adz.google.com",
    "c.admob.com",
    "c.android.clients.google.com",
    "cache-management-prod.google.com",
    "cache.pack.google.com",
    "checkin.gstatic.com",
    "clickserve.cc-dt.com",
    "clickserve.dartsearch.net",
    "clickserver.googleads.com",
    "clientservices.googleapis.com",
    "cn.widevine.com",
    "cnappinstall.googleadapis.com",
    "connectivitycheck.gstatic.com",
    "content.googleadapis.com",
    "crashlyticsreports-pa.googleapis.com",
    "crl.pki.goog",
    "csi.gstatic.com",
    "dartsearch-cn.net",
    "dg-meta.video.google.com",
    "distribution.qatp1.net",
    "distribution.qcpp1.net",
    "distribution.qpdp1.net",
    "dl.google.com",
    "dl.l.google.com",
    "doubleclick-cn.net",
    "doubleclick.net",
    "download.mlcc.google.com",
    "download.qatp1.net",
    "download.qcpp1.net",
    "download.qpdp1.net",
    "download.tensorflow.google.com",
    "emmapplecodevice.googleapis.com",
    "firebase-settings.crashlytics.com",
    "fontfiles.googleadapis.com",
    "fontfiles.googleapis.com",
    "fonts.googleadapis.com",
    "fonts.googleapis.com",
    "fonts.gstatic.com",
    "g0.gstatic.com",
    "g1.gstatic.com",
    "g2.gstatic.com",
    "g3.gstatic.com",
    "gonglchuangl.net",
    "gongyichuangyi.net",
    "google-analytics-cn.com",
    "google-analytics.com",
    "googleadservices-cn.com",
    "googleadservices.com",
    "googleanalytics.com",
    "googleapis-cn.com",
    "googleapps-cn.com",
    "googleflights-cn.net",
    "googleoptimize-cn.com",
    "googleoptimize.com",
    "googleplex.com",
    "googlesyndication-cn.com",
    "googlesyndication.com",
    "googletagmanager-cn.com",
    "googletagmanager.com",
    "googletagservices-cn.com",
    "googletagservices.com",
    "googletraveladservices-cn.com",
    "googletraveladservices.com",
    "googlevads-cn.com",
    "goto.google.com",
    "gstatic-cn.com",
    "gstaticadssl.l.google.com",
    "gtm.oasisfeng.com",
    "imasdk.googleapis.com",
    "monitoring.qatp1.net",
    "monitoring.qcpp1.net",
    "monitoring.qpdp1.net",
    "ocsp.pki.goog",
    "pagead-googlehosted.l.google.com",
    "performanceparameters.googleapis.com",
    "pki-goog.l.google.com",
    "play.1ucrs.com",
    "prod-controlbe.floonet.goog",
    "prod-databe.floonet.goog",
    "prod.databe.floonet.goog",
    "qagpublic.qatp1.net",
    "qagpublic.qcpp1.net",
    "qagpublic.qpdp1.net",
    "qgadmin.qatp1.net",
    "qgadmin.qcpp1.net",
    "qgadmin.qpdp1.net",
    "qpx.googleflights.net",
    "qualysapi.qatp1.net",
    "qualysapi.qcpp1.net",
    "qualysapi.qpdp1.net",
    "qualysguard.qatp1.net",
    "qualysguard.qcpp1.net",
    "qualysguard.qpdp1.net",
    "recaptcha.net",
    "recaptcha-cn.net",
    "redirector.bdn.dev",
    "redirector.c.chat.google.com",
    "redirector.c.mail.google.com",
    "redirector.c.pack.google.com",
    "redirector.c.youtubeeducation.com",
    "redirector.gcpcdn.gvt1.com",
    "redirector.gvt1.com",
    "redirector.offline-maps.gvt1.com",
    "redirector.snap.gvt1.com",
    "redirector.xn--ngstr-lra8j.com",
    "regioninfo-pa.googleapis.com",
    "safebrowsing.googleapis.com",
    "scanservice1.qatp1.net",
    "scanservice1.qcpp1.net",
    "scanservice1.qpdp1.net",
    "service.urchin.com",
    "settings.xn--9trs65b.com",
    "ssl-google-analytics.l.google.com",
    "ssl.gstatic.com",
    "staging-controlbe.floonet.goog",
    "staging-databe.floonet.goog",
    "staging-regioninfo-pa.googleapis.com",
    "staging.databe.floonet.goog",
    "support.1ucrs.com",
    "tac.googleapis.com",
    "test.bugs-qa.chromium.org",
    "test.gbugs-qa.chromium.org",
    "tools.google.com",
    "tools.l.google.com",
    "update.crashlytics.com",
    "update.googleapis.com",
    "wear.googleapis.com",
    "www-google-analytics.l.google.com",
    "www-googletagmanager.l.google.com",
    "www.destinationurl.com",
    "www.googleadapis.com",
    "www.gstatic.com",
    "www.pxcc.com",
    "xn--9kr7l.com",
    "xn--flw351e.com",
    "applemdm.corp.google.com",
    "dns.corp.google.com",
    "g3doc.corp.google.com",
    "gmskiosk-ssl.corp.google.com",
    "go.corp.google.com",
    "gorom.corp.google.com",
    "goto.corp.google.com",
    "l2-uberproxy.corp.google.com",
    "logger.corp.google.com",
    "logger-dev.corp.google.com",
    "login.corp.google.com",
    "proxyconfig.corp.google.com",
    "rapture-autopush.corp.google.com",
    "rapture.corp.google.com",
    "rapture-prod.corp.google.com",
    "rapture-test.corp.google.com",
    "r.cert.corp.google.com",
    "ssh-relay.corp.google.com",
    "sslredirect.corp.google.com",
    "streaming-uberproxy.corp.google.com",
    "streaming-uberproxy-rotation.corp.google.com",
    "sup.corp.google.com",
    "supportcenter.corp.google.com",
    "sup-ssh-relay2.corp.google.com",
    "sup-ssh-relay.corp.google.com",
    "techstop.corp.google.com",
    "uberproxy4.corp.google.com",
    "uberproxy6.corp.google.com",
    "uberproxy.corp.google.com",
    "uberproxy-debug4.corp.google.com",
    "uberproxy-debug.corp.google.com",
    "uberproxy-pen-redirect.corp.google.com",
    "vext.corp.google.com",
    "sup.l.google.com",
    "uberproxy-with-cn.l.google.com",
    "uberproxy-with-cn4.l.google.com",
    "uberproxy-with-cn6.l.google.com",
    "qiao-cn.com",
    "gvt1-cn.com",
    "gvt2-cn.com",
];
const AMAZON_DOMAINS: [&str; 94] = [
    "a2z.com",
    "aboutamazon.cn",
    "aboutamazon.co.uk",
    "aboutamazon.com",
    "aboutamazon.com.au",
    "aboutamazon.de",
    "aboutamazon.es",
    "aboutamazon.eu",
    "aboutamazon.fr",
    "aboutamazon.in",
    "aboutamazon.it",
    "aboutamazon.jp",
    "aboutamazon.pl",
    "alexafund.cn",
    "alexafund.com.cn",
    "amaaozn.com",
    "amazon-fashions.com",
    "amazon-jp-recruiting.com",
    "amazon-lantern.com",
    "amazon-launchpad.com",
    "amazon.ae",
    "amazon.ca",
    "amazon.cn",
    "amazon.co.jp",
    "amazon.co.uk",
    "amazon.com",
    "amazon.com.au",
    "amazon.com.br",
    "amazon.com.mx",
    "amazon.com.tr",
    "amazon.de",
    "amazon.dev",
    "amazon.es",
    "amazon.fr",
    "amazon.in",
    "amazon.it",
    "amazon.jobs",
    "amazon.jp",
    "amazon.nl",
    "amazon.red",
    "amazon.sg",
    "amazonalexavoxcon.com",
    "amazonauthorinsights.com",
    "amazonbusiness.cn",
    "amazonbusiness.com.cn",
    "amazonbusiness.org",
    "amazonbusinessblog.com",
    "amazonchoice.cn",
    "amazonchoice.com.cn",
    "amazonchoices.cn",
    "amazonchoices.com.cn",
    "amazondevicesupport.com",
    "amazonfctours.com",
    "amazonianblog.com",
    "amazonimages.com",
    "amazoninspire.cn",
    "amazoninspire.com.cn",
    "amazonlaunchpad.cn",
    "amazonlaunchpad.com",
    "amazonlaunchpad.com.cn",
    "amazonlending.com.cn",
    "amazonliterarypartnership.com",
    "amazonlumberyard.wang",
    "amazonnow.cn",
    "amazonnow.com.cn",
    "amazonpay.com",
    "amazonpay.in",
    "amazonsdi.com",
    "amazonstudiosguilds.com",
    "amazonvideodirect.com",
    "amzn.asia",
    "amzn.com",
    "amzn.to",
    "amznl.com",
    "assoc-amazon.cn",
    "associates-amazon.com",
    "gameon-masters.com",
    "gdansk-amazon.com",
    "images-amazon.com",
    "media-amazon.com",
    "primeday.cn",
    "primeday.com.cn",
    "primeday.info",
    "siege-amazon.com",
    "ssl-images-amazon.com",
    "ueberamazon.de",
    "yamaxun.cn",
    "yamaxun.com",
    "z.cn",
    "alexa.com",
    "audible.com",
    "createspace.com",
    "full:images-cn.ssl-images-amazon.com",
    "full:images-cn-8.ssl-images-amazon.com",
];
