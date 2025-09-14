use std::{
    collections::{HashMap, hash_map::Entry},
    error::Error,
    sync::RwLock,
};

use tracing::warn;

use crate::{
    errors::TransactionHistoryError,
    transactions::TransactionId,
    transactions_processor::{TransactionInfo, TransactionStatus},
};

pub trait TransactionHistoryStorage {
    fn add_transaction(&self, transaction_info: TransactionInfo) -> Result<(), Box<dyn Error>>;
    fn find_transaction(&self, transaction_id: TransactionId) -> Option<TransactionInfo>;
    fn update_transaction_status(
        &self,
        transaction_id: TransactionId,
        new_status: TransactionStatus,
    ) -> Result<(), Box<dyn Error>>;
}

pub struct InMemoryTransactionStorage {
    storage: RwLock<HashMap<TransactionId, TransactionInfo>>,
}

impl Default for InMemoryTransactionStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryTransactionStorage {
    pub fn new() -> Self {
        Self {
            storage: RwLock::new(HashMap::new()),
        }
    }
}

impl TransactionHistoryStorage for InMemoryTransactionStorage {
    fn add_transaction(&self, transaction_info: TransactionInfo) -> Result<(), Box<dyn Error>> {
        let mut storage = self.storage.write().unwrap();
        match storage.entry(transaction_info.transaction_id) {
            Entry::Vacant(entry) => entry.insert(transaction_info),
            Entry::Occupied(_) => {
                warn!("Attempt to add transaction, that already exists in history storage");
                return Err(Box::new(TransactionHistoryError::TransactionAlreadyExists));
            }
        };
        Ok(())
    }

    fn find_transaction(&self, transaction_id: TransactionId) -> Option<TransactionInfo> {
        let storage = self.storage.read().unwrap();
        storage.get(&transaction_id).cloned()
    }

    fn update_transaction_status(
        &self,
        transaction_id: TransactionId,
        new_status: TransactionStatus,
    ) -> Result<(), Box<dyn Error>> {
        let mut storage = self.storage.write().unwrap();
        match storage.entry(transaction_id) {
            Entry::Vacant(_) => {
                warn!("Attempt to update unknown transaction");
                return Err(Box::new(TransactionHistoryError::UnknownTransaction));
            }
            Entry::Occupied(entry) => {
                let current_status = entry.get().status;
                entry.into_mut().status = current_status.make_transition(new_status)?;
            }
        };
        Ok(())
    }
}
