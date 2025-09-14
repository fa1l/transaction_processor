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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transactions_processor::{TransactionInfo, TransactionInfoType, TransactionStatus};
    use rust_decimal::dec;

    #[test]
    fn test_add_transaction_successful() {
        let storage = InMemoryTransactionStorage::new();
        let transaction_info = TransactionInfo {
            client_id: 1,
            transaction_id: 100,
            amount: dec!(50.00),
            transaction_type: TransactionInfoType::Deposit,
            status: TransactionStatus::WithoutDisputes,
        };

        let result = storage.add_transaction(transaction_info.clone());

        assert!(result.is_ok());

        let stored_transaction = storage.find_transaction(100).unwrap();
        assert_eq!(stored_transaction.client_id, transaction_info.client_id);
        assert_eq!(
            stored_transaction.transaction_id,
            transaction_info.transaction_id
        );
        assert_eq!(stored_transaction.amount, transaction_info.amount);
        assert_eq!(stored_transaction.status, transaction_info.status);
    }

    #[test]
    fn test_add_transaction_duplicate_id_error() {
        let storage = InMemoryTransactionStorage::new();
        let transaction_id = 100;

        let first_transaction = TransactionInfo {
            client_id: 1,
            transaction_id,
            amount: dec!(50.00),
            transaction_type: TransactionInfoType::Deposit,
            status: TransactionStatus::WithoutDisputes,
        };

        let second_transaction = TransactionInfo {
            client_id: 2,
            transaction_id,
            amount: dec!(75.00),
            transaction_type: TransactionInfoType::Deposit,
            status: TransactionStatus::Disputed,
        };

        let result1 = storage.add_transaction(first_transaction.clone());
        assert!(result1.is_ok());

        let result2 = storage.add_transaction(second_transaction);
        assert!(result2.is_err());

        let error = result2.unwrap_err();
        let history_error = error.downcast_ref::<TransactionHistoryError>().unwrap();
        assert_eq!(
            *history_error,
            TransactionHistoryError::TransactionAlreadyExists
        );

        let stored_transaction = storage.find_transaction(transaction_id).unwrap();
        assert_eq!(stored_transaction.client_id, first_transaction.client_id);
        assert_eq!(stored_transaction.amount, first_transaction.amount);
    }
}
