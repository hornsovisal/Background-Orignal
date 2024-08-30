use std::sync::Arc;

use base64::{prelude::BASE64_STANDARD, Engine};
use redb::{Database, TableDefinition};

const ACTIONS_TABLE: TableDefinition<String, (String, Vec<(String, String)>)> =
    TableDefinition::new("actions");

use crate::error::Error;

#[derive(Debug, Clone)]
pub struct ActionDb {
    db: Arc<Database>,
}

impl ActionDb {
    pub fn new(path: &str) -> Result<Self, Error> {
        let db = Arc::new(Database::create(path)?);
        // let read = db.begin_read()?;
        // if let Ok(table) = read.open_table(ACTIONS_TABLE) {
        //     for (i,entry) in table.iter()?.enumerate(){
        //         let (key, value) = entry?;
        //         println!("{}-{}:{}",i, key.value(), value.value());
        //     }
        // }
        Ok(Self { db })
    }
    pub fn add_action(
        &self,
        action: String,
        args: String,
        extra: Vec<(String, String)>,
    ) -> Result<(), Error> {
        if action.len() == 0 {
            return Ok(());
        }
        let read = self.db.begin_read()?;
        let mut privous_data = String::new();
        let mut data_updated = false;
        let mut privous_extra = Vec::new();
        let mut extra_updated = false;
        if let Ok(table) = read.open_table(ACTIONS_TABLE) {
            if let Some(v) = table.get(&action).ok().flatten() {
                let (data, extras) = v.value();
                privous_data = data;
                privous_extra = extras;
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
            if let Some(i) = privous_extra.iter().position(|(key, _)| key == k) {
                if privous_extra[i].1.is_empty() {
                    privous_extra[i].1 = value.clone();
                    extra_updated = true;
                }
            } else {
                privous_extra.push((k.clone(), value.clone()));
                extra_updated = true;
            }
        }

        if data_updated || extra_updated {
            let write_txn = self.db.begin_write()?;
            {
                let mut table = write_txn.open_table(ACTIONS_TABLE)?;
                table.insert(action, (privous_data, privous_extra))?;
            }
            write_txn.commit()?;
        }

        Ok(())
    }
    pub fn get_action_arg(
        &self,
        action: String,
    ) -> Result<Option<(String, Vec<(String, String)>)>, Error> {
        if action.len() == 0 {
            return Ok(None);
        }
        let read = self.db.begin_read()?;
        if let Ok(table) = read.open_table(ACTIONS_TABLE) {
            if let Some(stored_args) = table.get(action)? {
                let (data, extras) = stored_args.value();
                if data.len() == 0 && extras.len() == 0 {
                    return Ok(None);
                } else {
                    return Ok(Some(stored_args.value()));
                }
            }
        }
        Ok(None)
    }
}
