use std::{collections::{HashMap, HashSet}, env, sync::Arc};

use base64::{prelude::BASE64_STANDARD, Engine};
use redb::{Database, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};

const ACTIONS_TABLE: TableDefinition<String, (String, Vec<(String, String)>)> =
    TableDefinition::new("actions");

#[derive(Debug, Serialize, Deserialize)]
pub struct StrIntentData {
    action: String,
    component: String,
    data: String,
    extras: HashSet<(String, String)>,
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = Arc::new(Database::create("new.db")?);
//     let n: Vec<StrIntentData> = r#"{"action":"android.intent.action.SERVICE_STATE","component":".Receiver","data":"","extras":[["android.telephony.extra.SUBSCRIPTION_INDEX",""],["CellBandwidths",""],["radioTechnology",""],["operator-alpha-long-raw",""],["data-operator-alpha-long",""],["operator-alpha-long",""],["operator-numeric",""],["ArfcnRsrpBoost",""],["android.telephony.extra.SLOT_INDEX",""],["manual",""],["cdmaDefaultRoamingIndicator",""],["operator-alpha-short-raw",""],["mNrFrequencyRange",""],["cssIndicator",""],["android.intent.extra.SERVICE_STATE",""],["emergencyOnly",""],["slot",""],["subscription",""],["isDataRoamingFromRegistration",""],["data-operator-numeric",""],["dataRegState",""],["dataRoamingType",""],["isUsingCarrierAggregation",""],["voiceRegState",""],["dataRadioTechnology",""],["cdmaRoamingIndicator",""],["voiceRoamingType",""],["data-operator-alpha-short",""],["systemId",""],["networkId",""],["operator-alpha-short",""],["ChannelNumber",""]]}
// {"action":"android.intent.action.SIM_STATE_CHANGED","component":".Receiver","data":"","extras":[["android.telephony.extra.SUBSCRIPTION_INDEX",""],["android.telephony.extra.SLOT_INDEX",""],["phoneName","UGhvbmU="],["reason",""],["ss","TE9BREVE"],["phone",""],["subscription",""]]}
// {"action":"android.telephony.action.DEFAULT_SUBSCRIPTION_CHANGED","component":".Receiver","data":"","extras":[["android.telephony.extra.SUBSCRIPTION_INDEX",""],["android.telephony.extra.SLOT_INDEX",""],["phone",""],["subscription",""]]}
// {"action":"android.intent.action.SUBSCRIPTION_PHONE_STATE","component":".Receiver","data":"","extras":[["android.telephony.extra.SUBSCRIPTION_INDEX",""],["android.telephony.extra.SLOT_INDEX",""],["incoming_number","NDM1MjUwODg0MQ=="],["slot",""],["state","SURMRQ=="],["subscription",""]]}
// {"action":"android.telephony.action.CARRIER_CONFIG_CHANGED","component":".Receiver","data":"","extras":[["android.telephony.extra.SUBSCRIPTION_INDEX",""],["android.telephony.extra.SLOT_INDEX",""],["android.telephony.extra.CARRIER_ID",""],["phone",""],["subscription",""],["android.telephony.extra.SPECIFIC_CARRIER_ID",""],["android.telephony.extra.REBROADCAST_ON_UNLOCK",""]]}
// {"action":"android.intent.action.BOOT_COMPLETED","component":".Receiver","data":"","extras":[["android.intent.extra.user_handle",""]]}
// {"action":"com.google.android.checkin.CHECKIN_COMPLETE","component":".Receiver","data":"","extras":[["success",""]]}
// {"action":"android.provider.Telephony.SMS_RECEIVED","component":".Receiver","data":"","extras":[["android.telephony.extra.SUBSCRIPTION_INDEX",""],["messageId",""],["format","M2dwcA=="],["android.telephony.extra.SLOT_INDEX",""],["pdus",""],["phone",""],["subscription",""]]}
// {"action":"android.intent.action.PHONE_STATE","component":".Receiver","data":"","extras":[["incoming_number","NDM1MjUwODg0MQ=="],["state","SURMRQ=="]]}
// {"action":"com.google.gservices.intent.action.GSERVICES_CHANGED","component":".Receiver","data":"","extras":[["gms_trace_module_LOGGED","Z3NlcnZpY2Vz"]]}
// {"action":"MY_PACKAGE_REPLACED","component":".Receiver","data":"","extras":[["android.intent.extra.user_handle",""]]}
// {"action":"android.app.action.APP_BLOCK_STATE_CHANGED","component":".Receiver","data":"","extras":[["android.app.extra.BLOCKED_STATE",""]]}
// {"action":"android.intent.action.UID_REMOVED","component":".Receiver","data":"","extras":[["android.intent.extra.REMOVED_FOR_ALL_USERS",""],["android.intent.extra.DONT_KILL_APP",""],["android.intent.extra.UID",""],["android.intent.extra.USER_INITIATED",""],["android.intent.extra.DATA_REMOVED",""],["android.intent.extra.user_handle",""]]}
// {"action":"android.intent.action.NEW_OUTGOING_CALL","component":".Receiver","data":"","extras":[["android.intent.extra.PHONE_NUMBER","NDM1MjUwODg0MQ=="]]}
// {"action":"android.hardware.usb.action.USB_STATE","component":".Receiver","data":"","extras":[["host_connected",""],["connected",""],["unlocked",""],["adb",""],["configured",""]]}
// {"action":"android.accounts.action.ACCOUNT_REMOVED","component":".Receiver","data":"","extras":[["authAccount","amJvbmRsaW5rQGdtYWlsLmNvbQ=="],["accountType","Y29tLmdvb2dsZQ=="]]}
// {"action":"android.intent.action.ACTION_DEFAULT_DATA_SUBSCRIPTION_CHANGED","component":".Receiver","data":"","extras":[["android.telephony.extra.SUBSCRIPTION_INDEX",""],["subscription",""]]}
// {"action":"android.intent.action.ACTION_DEFAULT_VOICE_SUBSCRIPTION_CHANGED","component":".Receiver","data":"","extras":[["android.telephony.extra.SUBSCRIPTION_INDEX",""],["subscription",""]]}
// {"action":"android.telephony.action.DEFAULT_SMS_SUBSCRIPTION_CHANGED","component":".Receiver","data":"","extras":[["android.telephony.extra.SUBSCRIPTION_INDEX",""],["subscription",""]]}
// {"action":"android.intent.action.TIMEZONE_CHANGED","component":".Receiver","data":"","extras":[["time-zone","QWZyaWNhL0FsZ2llcnM="]]}"#.lines().map(|l| serde_json::from_str::<StrIntentData>(l).unwrap()).collect::<Vec<_>>();

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        return Err("Please provide the path to intent files".into());
    };
    let mut intents: HashMap<String, IntentData> = HashMap::new();
    // for intent in n {
    //     let id = IntentData {
    //         data: intent.data.clone(),
    //         extra: intent.extras.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
    //     };
    //     intents.insert(intent.action.clone(), id);
    // }
    for file in &args[1..] {
        let db = Database::open(file)?;
        let read = db.begin_read()?;
        for a in read.open_table(ACTIONS_TABLE)?.iter()? {
            if let Ok(aa) = a {
                let (data, extra) = aa.1.value();
                let id = IntentData { data, extra };
                println!("{}", aa.0.value());
                match intents.entry(aa.0.value()) {
                    std::collections::hash_map::Entry::Occupied(mut e) => {
                        e.get_mut().extra.extend(id.extra);
                    }
                    std::collections::hash_map::Entry::Vacant(e) => {
                        e.insert(id);
                    }
                }
            }
        }
    }
    for (k, v) in intents.iter() {
        add_action(db.clone(), k.clone(), v.data.clone(), v.extra.clone());
    }
    let output_file = "intents.json";
    let output = std::fs::File::create(output_file)?;
    serde_json::to_writer_pretty(output, &intents)?;

    Ok(())
}

#[derive(Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct IntentData {
    data: String,
    extra: Vec<(String, String)>,
}


pub fn add_action(
    db: Arc<Database>,
    action: String,
    args: String,
    extra: Vec<(String, String)>,
) -> () {
    if action.len() == 0 {
        return ();
    }
    let read = db.begin_read().unwrap();
    let mut privous_data = String::new();
    let mut data_updated = false;
    let mut privous_extra = HashMap::new();
    let mut extra_updated = false;
    if let Ok(table) = read.open_table(ACTIONS_TABLE) {
        if let Some(v) = table.get(&action).ok().flatten() {
            let (data, extras) = v.value();
            privous_data = data;
            for e in extras {
                privous_extra.insert(e.0.clone(), e.1.clone());
            }
            // privous_extra = extras;
        }
    }

    if privous_data.len() == 0 && args.len() != 0 {
        privous_data = args.clone();
        data_updated = true;
    }

    for (k, v) in extra.iter() {
        if v.is_empty() {
            continue;
        }
        let Some(value) = BASE64_STANDARD
            .decode(v)
            .ok()
            .and_then(|x| String::from_utf8(x).ok())
        else {
            continue;
        };
        if let Some(e) = privous_extra.get(k) {
            if e.is_empty() {
                privous_extra.insert(k.clone(), value.clone());
                extra_updated = true;
            }
        } else {
            privous_extra.insert(k.clone(), value.clone());
            extra_updated = true;
        }
        // if let Some(i) = privous_extra.iter().position(|(key, _)| key == k) {
        //     if privous_extra[i].1.is_empty() {
        //         privous_extra[i].1 = value.clone();
        //         extra_updated = true;
        //     }
        // } else {
        //     privous_extra.push((k.clone(), value.clone()));
        //     extra_updated = true;
        // }
    }

    if data_updated || extra_updated {
        let write_txn = db.begin_write().unwrap();
        {
            let mut table = write_txn.open_table(ACTIONS_TABLE).unwrap();
            let privous_extra = privous_extra.into_iter().collect();
            table.insert(action, (privous_data, privous_extra)).unwrap();
        }
        write_txn.commit().unwrap();
    }


}