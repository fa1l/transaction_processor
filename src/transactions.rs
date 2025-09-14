use enum_dispatch::enum_dispatch;
use rust_decimal::Decimal;
use std::error::Error;

const DEPOSIT_VALUE: &str = "deposit";
const WITHDRAWAL_VALUE: &str = "withdrawal";
const DISPUTE_VALUE: &str = "dispute";
const RESOLVE_VALUE: &str = "resolve";
const CHARGEBACK_VALUE: &str = "chargeback";

use crate::{
    errors::{TransactionError, TransactionLogError},
    history::TransactionHistoryStorage,
    storage::{AccountStorage, ClientId},
    transactions_processor::TransactionLogEntry,
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
            _ => panic!("not yet"),
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
        history: &impl TransactionHistoryStorage,
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
        history: &impl TransactionHistoryStorage,
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
        Ok(())
    }
}
