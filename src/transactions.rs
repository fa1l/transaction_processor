use rust_decimal::Decimal;
use std::error::Error;

use crate::{
    errors::TransactionError,
    storage::{self, InMemoryAccountsStorage, Storage, UserId},
};

pub type TransactionId = u64;

pub trait TransactionExecutor {
    fn execute(self, storage: &mut impl Storage) -> Result<(), Box<dyn Error>>;
}

#[derive(Debug)]
pub enum Transaction {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

pub struct Deposit {
    client_id: UserId,
    transaction_id: TransactionId,
    amount: Decimal,
}

impl TransactionExecutor for Deposit {
    fn execute(self, storage: &mut impl Storage) -> Result<(), Box<dyn Error>> {
        if self.amount.is_sign_negative() {
            return Err(Box::new(TransactionError::NegativeAmount));
        }
        storage.add_money(self.client_id, self.amount)
    }
}

pub struct Withdrawal {
    client_id: UserId,
    transaction_id: TransactionId,
    amount: Decimal,
}

impl TransactionExecutor for Withdrawal {
    fn execute(self, storage: &mut impl Storage) -> Result<(), Box<dyn Error>> {
        if self.amount.is_sign_negative() {
            return Err(Box::new(TransactionError::NegativeAmount));
        }
        storage.withdraw_money(self.client_id, self.amount)
    }
}
