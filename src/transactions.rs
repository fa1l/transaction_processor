use enum_dispatch::enum_dispatch;
use rust_decimal::Decimal;
use std::error::Error;
use tracing::warn;

const DEPOSIT_VALUE: &str = "deposit";
const WITHDRAWAL_VALUE: &str = "withdrawal";
const DISPUTE_VALUE: &str = "dispute";
const RESOLVE_VALUE: &str = "resolve";
const CHARGEBACK_VALUE: &str = "chargeback";

use crate::{
    errors::{TransactionError, TransactionLogError},
    history::TransactionHistoryStorage,
    storage::{AccountStorage, ClientId},
    transactions_processor::{TransactionLogEntry, TransactionStatus},
};

pub type TransactionId = u64;

pub trait ExecTransaction {
    fn execute(
        &self,
        account_storage: &impl AccountStorage,
        history: &impl TransactionHistoryStorage,
    ) -> Result<(), Box<dyn Error>>;
}

#[enum_dispatch(ExecTransaction)]
#[derive(Debug, PartialEq)]
pub enum Transaction {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

impl ExecTransaction for Transaction {
    fn execute(
        &self,
        account_storage: &impl AccountStorage,
        history: &impl TransactionHistoryStorage,
    ) -> Result<(), Box<dyn Error>> {
        match self {
            Transaction::Deposit(transaction) => transaction.execute(account_storage, history),
            Transaction::Withdrawal(transaction) => transaction.execute(account_storage, history),
            Transaction::Dispute(transaction) => transaction.execute(account_storage, history),
            Transaction::Resolve(transaction) => transaction.execute(account_storage, history),
            Transaction::Chargeback(transaction) => transaction.execute(account_storage, history),
        }
    }
}

impl TryFrom<&TransactionLogEntry> for Transaction {
    type Error = TransactionLogError;

    fn try_from(value: &TransactionLogEntry) -> Result<Self, Self::Error> {
        let TransactionLogEntry {
            transaction_type,
            transaction_id,
            client_id,
            amount,
        } = value;
        match transaction_type.as_str() {
            DEPOSIT_VALUE => {
                let amount = amount.ok_or(TransactionLogError::MissingAmount)?;
                Ok(Transaction::Deposit(Deposit {
                    client_id: *client_id,
                    transaction_id: *transaction_id,
                    amount,
                }))
            }
            WITHDRAWAL_VALUE => {
                let amount = amount.ok_or(TransactionLogError::MissingAmount)?;
                Ok(Transaction::Withdrawal(Withdrawal {
                    client_id: *client_id,
                    transaction_id: *transaction_id,
                    amount,
                }))
            }
            DISPUTE_VALUE => Ok(Transaction::Dispute(Dispute {
                client_id: *client_id,
                transaction_id: *transaction_id,
            })),
            RESOLVE_VALUE => Ok(Transaction::Resolve(Resolve {
                client_id: *client_id,
                transaction_id: *transaction_id,
            })),
            CHARGEBACK_VALUE => Ok(Transaction::Chargeback(Chargeback {
                client_id: *client_id,
                transaction_id: *transaction_id,
            })),
            _ => Err(TransactionLogError::InvalidTransactionType),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Deposit {
    client_id: ClientId,
    transaction_id: TransactionId,
    amount: Decimal,
}

impl ExecTransaction for Deposit {
    fn execute(
        &self,
        account_storage: &impl AccountStorage,
        _history: &impl TransactionHistoryStorage,
    ) -> Result<(), Box<dyn Error>> {
        if self.amount.is_sign_negative() {
            return Err(Box::new(TransactionError::NegativeAmount));
        }
        account_storage.add_money(self.client_id, self.amount)
    }
}

#[derive(Debug, PartialEq)]
pub struct Withdrawal {
    client_id: ClientId,
    transaction_id: TransactionId,
    amount: Decimal,
}

impl ExecTransaction for Withdrawal {
    fn execute(
        &self,
        account_storage: &impl AccountStorage,
        _history: &impl TransactionHistoryStorage,
    ) -> Result<(), Box<dyn Error>> {
        if self.amount.is_sign_negative() {
            return Err(Box::new(TransactionError::NegativeAmount));
        }
        account_storage.withdraw_money(self.client_id, self.amount)
    }
}

#[derive(Debug, PartialEq)]
pub struct Dispute {
    client_id: ClientId,
    transaction_id: TransactionId,
}

impl ExecTransaction for Dispute {
    fn execute(
        &self,
        account_storage: &impl AccountStorage,
        history: &impl TransactionHistoryStorage,
    ) -> Result<(), Box<dyn Error>> {
        let transaction_info = match history.find_transaction(self.transaction_id) {
            Some(transaction) => transaction,
            None => {
                warn!("Can't find transaction for dispute");
                return Err(Box::new(TransactionError::OriginTransactionNotFound));
            }
        };
        if !matches!(transaction_info.status, TransactionStatus::WithoutDisputes) {
            warn!("Original transaction already have been disputed");
            return Err(Box::new(TransactionError::TransactionMultipleDispute));
        }
        history.update_transaction_status(self.transaction_id, TransactionStatus::Disputed)?;
        account_storage.hold_money(self.client_id, transaction_info.amount.unwrap())?;
        //TODO: maybe account should be blocked if it hasn't got enough money to be held
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
pub struct Resolve {
    client_id: ClientId,
    transaction_id: TransactionId,
}

impl ExecTransaction for Resolve {
    fn execute(
        &self,
        account_storage: &impl AccountStorage,
        history: &impl TransactionHistoryStorage,
    ) -> Result<(), Box<dyn Error>> {
        let transaction_info = match history.find_transaction(self.transaction_id) {
            Some(transaction) => transaction,
            None => {
                warn!("Can't find transaction for resolve");
                return Err(Box::new(TransactionError::OriginTransactionNotFound));
            }
        };
        if !matches!(transaction_info.status, TransactionStatus::Disputed) {
            warn!("Original transaction not in disputed state");
            return Err(Box::new(TransactionError::TransactionNotDisputed));
        }
        let origin_amount = match transaction_info.amount {
            Some(amount) => amount,
            None => return Err(Box::new(TransactionError::EmptyAmount)),
        };
        history.update_transaction_status(self.transaction_id, TransactionStatus::Resolved)?;
        account_storage.unhold_money(self.client_id, origin_amount)?;
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
pub struct Chargeback {
    client_id: ClientId,
    transaction_id: TransactionId,
}

impl ExecTransaction for Chargeback {
    fn execute(
        &self,
        account_storage: &impl AccountStorage,
        history: &impl TransactionHistoryStorage,
    ) -> Result<(), Box<dyn Error>> {
        let transaction_info = match history.find_transaction(self.transaction_id) {
            Some(transaction) => transaction,
            None => {
                warn!("Can't find transaction for chargeback");
                return Err(Box::new(TransactionError::OriginTransactionNotFound));
            }
        };
        if !matches!(transaction_info.status, TransactionStatus::Disputed) {
            warn!("Original transaction not in disputed state");
            return Err(Box::new(TransactionError::TransactionNotDisputed));
        }
        let origin_amount = match transaction_info.amount {
            Some(amount) => amount,
            None => return Err(Box::new(TransactionError::EmptyAmount)),
        };
        history.update_transaction_status(self.transaction_id, TransactionStatus::Chargebacked)?;
        // TODO: maybe I need to make unhold + withdraw as a one method
        account_storage.unhold_money(self.client_id, origin_amount)?;
        account_storage.withdraw_money(self.client_id, origin_amount)?;
        account_storage.block_account(self.client_id)?;
        Ok(())
    }
}
