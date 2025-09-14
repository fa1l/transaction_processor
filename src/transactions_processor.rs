use std::error::Error;

use rust_decimal::Decimal;
use serde::Deserialize;

use crate::{
    errors::TransactionHistoryError,
    history::{InMemoryTransactionStorage, TransactionHistoryStorage},
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
    pub user_id: ClientId,
    pub transaction_id: TransactionId,
    pub amount: Option<Decimal>,
    pub status: TransactionStatus,
}

pub trait TransactionProcessor {
    fn process(&self, transaction_entry: TransactionLogEntry) -> Result<(), Box<dyn Error>>;
}

pub struct InMemoryTransactionProcessor {
    storage: InMemoryAccountsStorage,
    transaction_history: InMemoryTransactionStorage,
}

impl TransactionProcessor for InMemoryTransactionProcessor {
    fn process(&self, transaction_entry: TransactionLogEntry) -> Result<(), Box<dyn Error>> {
        let transaction = Transaction::try_from(&transaction_entry)?;
        transaction.execute(&self.storage, &self.transaction_history)?;
        let status = match transaction {
            Transaction::Dispute(_) => TransactionStatus::Disputed,
            Transaction::Resolve(_) => TransactionStatus::Resolved,
            Transaction::Chargeback(_) => TransactionStatus::Chargebacked,
            _ => TransactionStatus::WithoutDisputes,
        };
        let info = TransactionInfo {
            user_id: transaction_entry.client_id,
            transaction_id: transaction_entry.transaction_id,
            amount: transaction_entry.amount,
            status,
        };
        self.transaction_history.add_transaction(info)?;
        Ok(())
    }
}
