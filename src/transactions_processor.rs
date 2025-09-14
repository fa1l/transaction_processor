use std::error::Error;

use rust_decimal::Decimal;
use serde::Deserialize;

use crate::{
    errors::TransactionHistoryError,
    history::InMemoryTransactionStorage,
    storage::{ClientId, InMemoryAccountsStorage},
    transactions::{ExecTransaction, Transaction, TransactionId},
};

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum TransactionStatus {
    WithoutDisputes,
    Resolved,
    Disputed,
    Chargebacked,
}

impl TransactionStatus {
    fn is_transition_available(self, new_status: &TransactionStatus) -> bool {
        matches!(
            (self, new_status),
            (
                TransactionStatus::WithoutDisputes,
                TransactionStatus::Disputed
            ) | (TransactionStatus::Disputed, TransactionStatus::Chargebacked)
                | (TransactionStatus::Disputed, TransactionStatus::Resolved)
        )
    }

    pub fn make_transition(
        self,
        new_status: TransactionStatus,
    ) -> Result<TransactionStatus, Box<dyn Error>> {
        if self.is_transition_available(&new_status) {
            Ok(new_status)
        } else {
            Err(Box::new(TransactionHistoryError::InvalidStatusTransition))
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum TransactionInfoType {
    Deposit,
    Withdrawal,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct TransactionLogEntry {
    #[serde(rename = "type")]
    pub transaction_type: String,
    #[serde(rename = "client")]
    pub client_id: ClientId,
    #[serde(rename = "tx")]
    pub transaction_id: TransactionId,
    #[serde(default)]
    pub amount: Option<Decimal>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct TransactionInfo {
    pub client_id: ClientId,
    pub transaction_id: TransactionId,
    pub transaction_type: TransactionInfoType,
    pub amount: Decimal,
    pub status: TransactionStatus,
}

pub trait TransactionProcessor {
    fn process(&self, transaction_entry: TransactionLogEntry) -> Result<(), Box<dyn Error>>;
}

pub struct InMemoryTransactionProcessor {
    storage: InMemoryAccountsStorage,
    history: InMemoryTransactionStorage,
}

impl InMemoryTransactionProcessor {
    pub fn new() -> Self {
        Self {
            storage: InMemoryAccountsStorage::new(),
            history: InMemoryTransactionStorage::new(),
        }
    }
}

impl TransactionProcessor for InMemoryTransactionProcessor {
    fn process(&self, transaction_entry: TransactionLogEntry) -> Result<(), Box<dyn Error>> {
        let transaction = Transaction::try_from(&transaction_entry)?;
        transaction.execute(&self.storage, &self.history)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::TransactionHistoryStorage;
    use rstest::rstest;
    use rust_decimal::dec;

    #[test]
    fn test_process_deposit_successful() {
        let processor = InMemoryTransactionProcessor::new();
        let client_id = 1;
        let transaction_id = 100;
        let amount = dec!(50.00);

        let entry = TransactionLogEntry {
            transaction_type: "deposit".to_string(),
            client_id,
            transaction_id,
            amount: Some(amount),
        };

        let result = processor.process(entry);

        assert!(result.is_ok());

        let transaction_info = processor.history.find_transaction(transaction_id).unwrap();
        assert_eq!(transaction_info.client_id, client_id);
        assert_eq!(transaction_info.transaction_id, transaction_id);
        assert_eq!(transaction_info.amount, amount);
        assert_eq!(transaction_info.status, TransactionStatus::WithoutDisputes);
        assert_eq!(
            transaction_info.transaction_type,
            TransactionInfoType::Deposit
        );
    }

    #[rstest]
    #[case(TransactionLogEntry{transaction_type: "deposit".to_string(), client_id: 1, transaction_id: 2, amount: Some(dec!(10))}, false, TransactionStatus::WithoutDisputes)]
    #[case(TransactionLogEntry{transaction_type: "withdrawal".to_string(), client_id: 1, transaction_id: 2, amount: Some(dec!(10))}, false, TransactionStatus::WithoutDisputes)]
    #[case(TransactionLogEntry{transaction_type: "dispute".to_string(), client_id: 1, transaction_id: 1, amount: None}, false, TransactionStatus::Disputed)]
    #[case(TransactionLogEntry{transaction_type: "resolve".to_string(), client_id: 1, transaction_id: 1, amount: None}, true, TransactionStatus::Resolved)]
    #[case(TransactionLogEntry{transaction_type: "chargeback".to_string(), client_id: 1, transaction_id: 1, amount: None}, true, TransactionStatus::Chargebacked)]
    fn test_transaction_status(
        #[case] transaction_log: TransactionLogEntry,
        #[case] need_dispute: bool,
        #[case] expected_status: TransactionStatus,
    ) {
        let processor = InMemoryTransactionProcessor::new();

        // additional transactions for disputes-like cases
        let deposit_entry = TransactionLogEntry {
            transaction_type: "deposit".to_string(),
            client_id: 1,
            transaction_id: 1,
            amount: Some(dec!(100)),
        };
        let result = processor.process(deposit_entry);

        if need_dispute {
            let dispute_entry = TransactionLogEntry {
                transaction_type: "dispute".to_string(),
                client_id: 1,
                transaction_id: 1,
                amount: None,
            };
            let result = processor.process(dispute_entry);
        }

        let transaction_id = transaction_log.transaction_id;
        let result = processor.process(transaction_log);
        let transaction_info = processor.history.find_transaction(transaction_id).unwrap();
        assert_eq!(transaction_info.status, expected_status);
    }
}
