use std::{collections::HashSet, sync::Arc};

use redb::{Database, TableDefinition};

const ACTIONS_TABLE: TableDefinition<String, Vec<String>> = TableDefinition::new("actions");

use crate::error::Error;

#[derive(Debug, Clone)]
pub struct ActionDb {
    db: Arc<Database>,
}

impl ActionDb {
    pub fn new(path: &str) -> Result<Self, Error> {
        let db = Arc::new(Database::create(path)?);

        Ok(Self { db })
    }
    pub fn add_action(&self, action: String, args: String) -> Result<(), Error> {
        let read = self.db.begin_read()?;
        let mut args_set = HashSet::new();
        args_set.insert(args);
        // let mut args = vec![args];
        if let Ok(table) = read.open_table(ACTIONS_TABLE) {
            if let Some(stored_args) = table.get(&action)? {
                args_set.extend(stored_args.value());
            }
        }

        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(ACTIONS_TABLE)?;
            table.insert(action, args_set.into_iter().collect::<Vec<String>>())?;
        }
        write_txn.commit()?;
        Ok(())
    }
    pub fn get_action_args(&self, action: String) -> Result<Vec<String>, Error> {
        let mut args = Vec::new();
        let read = self.db.begin_read()?;
        if let Ok(table) = read.open_table(ACTIONS_TABLE) {
            if let Some(stored_args) = table.get(action)? {
                args.extend(stored_args.value());
            }
        }
        Ok(args)
    }
}
