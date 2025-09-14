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
    transactions_processor::{
        TransactionInfo, TransactionInfoType, TransactionLogEntry, TransactionStatus,
    },
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
        history: &impl TransactionHistoryStorage,
    ) -> Result<(), Box<dyn Error>> {
        if self.amount.is_sign_negative() {
            return Err(Box::new(TransactionError::NegativeAmount));
        }
        account_storage.add_money(self.client_id, self.amount)?;
        let transaction_info = TransactionInfo {
            client_id: self.client_id,
            transaction_id: self.transaction_id,
            amount: self.amount,
            status: TransactionStatus::WithoutDisputes,
            transaction_type: TransactionInfoType::Deposit,
        };
        history.add_transaction(transaction_info)?;
        Ok(())
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
        account_storage.withdraw_money(self.client_id, self.amount)?;
        let transaction_info = TransactionInfo {
            client_id: self.client_id,
            transaction_id: self.transaction_id,
            amount: self.amount,
            status: TransactionStatus::WithoutDisputes,
            transaction_type: TransactionInfoType::Withdrawal,
        };
        history.add_transaction(transaction_info)?;
        Ok(())
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
        match transaction_info.transaction_type {
            TransactionInfoType::Deposit => {
                //TODO: maybe account should be blocked if it hasn't got enough money to be held
                account_storage.hold_money(self.client_id, transaction_info.amount)?;
            }
            TransactionInfoType::Withdrawal => {
                account_storage.add_money(self.client_id, transaction_info.amount)?;
                account_storage.hold_money(self.client_id, transaction_info.amount)?;
            }
        };
        history.update_transaction_status(self.transaction_id, TransactionStatus::Disputed)?;
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
        history.update_transaction_status(self.transaction_id, TransactionStatus::Resolved)?;
        account_storage.unhold_money(self.client_id, transaction_info.amount)?;
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
        history.update_transaction_status(self.transaction_id, TransactionStatus::Chargebacked)?;
        // TODO: maybe I need to make unhold + withdraw as a one method
        account_storage.unhold_money(self.client_id, transaction_info.amount)?;
        account_storage.withdraw_money(self.client_id, transaction_info.amount)?;
        account_storage.block_account(self.client_id)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        errors::{AccountError, TransactionError, TransactionHistoryError},
        history::{InMemoryTransactionStorage, TransactionHistoryStorage},
        storage::{AccountStorage, InMemoryAccountsStorage},
        transactions_processor::{TransactionInfo, TransactionInfoType, TransactionStatus},
    };
    use rstest::rstest;
    use rust_decimal::dec;

    fn create_transaction_in_history(
        history: &InMemoryTransactionStorage,
        transaction_id: u64,
        client_id: u16,
        amount: Decimal,
        transaction_type: TransactionInfoType,
        status: TransactionStatus,
    ) {
        let transaction_info = TransactionInfo {
            client_id,
            transaction_id,
            amount,
            status,
            transaction_type,
        };
        history.add_transaction(transaction_info).unwrap();
    }

    #[rstest]
    #[case("deposit", Some(dec!(100.00)), 1, 100)]
    #[case("withdrawal", Some(dec!(50.25)), 2, 200)]
    #[case("dispute", None, 3, 300)]
    #[case("resolve", None, 4, 400)]
    #[case("chargeback", None, 5, 500)]
    fn test_try_from_all_valid_types(
        #[case] transaction_type: &str,
        #[case] amount: Option<Decimal>,
        #[case] client_id: u16,
        #[case] transaction_id: u64,
    ) {
        let entry = TransactionLogEntry {
            transaction_type: transaction_type.to_string(),
            client_id,
            transaction_id,
            amount,
        };

        let result = Transaction::try_from(&entry);

        assert!(result.is_ok());
        let transaction = result.unwrap();

        match (transaction_type, &transaction) {
            ("deposit", Transaction::Deposit(_)) => {}
            ("withdrawal", Transaction::Withdrawal(_)) => {}
            ("dispute", Transaction::Dispute(_)) => {}
            ("resolve", Transaction::Resolve(_)) => {}
            ("chargeback", Transaction::Chargeback(_)) => {}
            _ => panic!("Unexpected transaction type for {}", transaction_type),
        }
    }

    #[test]
    fn test_deposit_execute_successful() {
        let account_storage = InMemoryAccountsStorage::new();
        let history = InMemoryTransactionStorage::new();
        let client_id = 1;
        let transaction_id = 100;
        let amount = dec!(50.00);

        let deposit = Deposit {
            client_id,
            transaction_id,
            amount,
        };

        assert_eq!(account_storage.get_balance(client_id), None);
        assert!(history.find_transaction(transaction_id).is_none());

        let result = deposit.execute(&account_storage, &history);

        assert!(result.is_ok());

        assert_eq!(account_storage.get_balance(client_id), Some(amount));

        let transaction_info = history.find_transaction(transaction_id).unwrap();
        assert_eq!(transaction_info.client_id, client_id);
        assert_eq!(transaction_info.transaction_id, transaction_id);
        assert_eq!(transaction_info.amount, amount);
        assert_eq!(transaction_info.status, TransactionStatus::WithoutDisputes);
        assert_eq!(
            transaction_info.transaction_type,
            TransactionInfoType::Deposit
        );
    }

    #[test]
    fn test_deposit_negative_amount_error() {
        let storage = InMemoryAccountsStorage::new();
        let history = InMemoryTransactionStorage::new();
        let client_id = 1;
        let transaction_id = 100;
        let negative_amount = dec!(-50.0);

        storage.create_user(client_id);

        let deposit = Deposit {
            client_id,
            transaction_id,
            amount: negative_amount,
        };
        let result = deposit.execute(&storage, &history);

        assert!(result.is_err());
        let error = result.unwrap_err();
        let transaction_error = error.downcast_ref::<TransactionError>().unwrap();
        assert_eq!(*transaction_error, TransactionError::NegativeAmount);

        let accounts = storage.accounts.read().unwrap();
        let account = accounts.get(&client_id).unwrap();
        assert_eq!(account.available_balance(), dec!(0));

        assert!(history.find_transaction(transaction_id).is_none());
    }

    #[test]
    fn test_deposit_execute_locked_account() {
        let account_storage = InMemoryAccountsStorage::new();
        let history = InMemoryTransactionStorage::new();
        let client_id = 1;
        let amount = dec!(50.00);

        account_storage.add_money(client_id, dec!(100.00)).unwrap();
        account_storage.block_account(client_id).unwrap();

        let deposit = Deposit {
            client_id,
            transaction_id: 100,
            amount,
        };

        let result = deposit.execute(&account_storage, &history);

        assert!(result.is_err());
        let error = result.unwrap_err();
        let account_error = error.downcast_ref::<AccountError>().unwrap();
        assert_eq!(*account_error, AccountError::AccountLocked);
        assert!(history.find_transaction(100).is_none());
    }

    #[test]
    fn test_withdrawal_execute_successful() {
        let account_storage = InMemoryAccountsStorage::new();
        let history = InMemoryTransactionStorage::new();
        let client_id = 1;
        let transaction_id = 100;
        let initial_amount = dec!(100.00);
        let withdrawal_amount = dec!(30.00);
        let expected_balance = dec!(70.00);

        account_storage
            .add_money(client_id, initial_amount)
            .unwrap();

        let withdrawal = Withdrawal {
            client_id,
            transaction_id,
            amount: withdrawal_amount,
        };

        assert_eq!(account_storage.get_balance(client_id), Some(initial_amount));
        assert!(history.find_transaction(transaction_id).is_none());

        let result = withdrawal.execute(&account_storage, &history);
        assert!(result.is_ok());

        assert_eq!(
            account_storage.get_balance(client_id),
            Some(expected_balance)
        );

        let transaction_info = history.find_transaction(transaction_id).unwrap();
        assert_eq!(transaction_info.client_id, client_id);
        assert_eq!(transaction_info.transaction_id, transaction_id);
        assert_eq!(transaction_info.amount, withdrawal_amount);
        assert_eq!(transaction_info.status, TransactionStatus::WithoutDisputes);
        assert_eq!(
            transaction_info.transaction_type,
            TransactionInfoType::Withdrawal
        );
    }

    #[test]
    fn test_withdrawal_negative_amount_error() {
        let storage = InMemoryAccountsStorage::new();
        let history = InMemoryTransactionStorage::new();
        let client_id = 1;
        let transaction_id = 100;
        let negative_amount = dec!(-30.0);

        storage.create_user(client_id);
        storage.add_money(client_id, dec!(100.0)).unwrap();

        let withdrawal = Withdrawal {
            client_id,
            transaction_id,
            amount: negative_amount,
        };
        let result = withdrawal.execute(&storage, &history);

        assert!(result.is_err());
        let error = result.unwrap_err();
        let transaction_error = error.downcast_ref::<TransactionError>().unwrap();
        assert_eq!(*transaction_error, TransactionError::NegativeAmount);

        let accounts = storage.accounts.read().unwrap();
        let account = accounts.get(&client_id).unwrap();
        assert_eq!(account.available_balance(), dec!(100.0));

        assert!(history.find_transaction(transaction_id).is_none());
    }

    #[test]
    fn test_withdrawal_execute_insufficient_funds() {
        let account_storage = InMemoryAccountsStorage::new();
        let history = InMemoryTransactionStorage::new();
        let client_id = 1;
        let transaction_id = 100;
        let initial_amount = dec!(50.00);
        let withdrawal_amount = dec!(100.00);

        account_storage
            .add_money(client_id, initial_amount)
            .unwrap();

        let withdrawal = Withdrawal {
            client_id,
            transaction_id,
            amount: withdrawal_amount,
        };

        let result = withdrawal.execute(&account_storage, &history);

        assert!(result.is_err());
        let error = result.unwrap_err();
        let account_error = error.downcast_ref::<AccountError>().unwrap();
        assert_eq!(*account_error, AccountError::InsufficientMoney);

        assert_eq!(account_storage.get_balance(client_id), Some(initial_amount));
        assert!(history.find_transaction(transaction_id).is_none());
    }

    #[test]
    fn test_withdrawal_execute_nonexistent_account() {
        let account_storage = InMemoryAccountsStorage::new();
        let history = InMemoryTransactionStorage::new();
        let client_id = 999;
        let transaction_id = 100;
        let amount = dec!(50.00);

        let withdrawal = Withdrawal {
            client_id,
            transaction_id,
            amount,
        };

        let result = withdrawal.execute(&account_storage, &history);

        assert!(result.is_err());
        let error = result.unwrap_err();
        let account_error = error.downcast_ref::<AccountError>().unwrap();
        assert_eq!(*account_error, AccountError::AccountNotFound);

        assert!(history.find_transaction(transaction_id).is_none());
    }

    #[test]
    fn test_withdrawal_execute_locked_account() {
        let account_storage = InMemoryAccountsStorage::new();
        let history = InMemoryTransactionStorage::new();
        let client_id = 1;
        let transaction_id = 100;
        let amount = dec!(50.00);

        account_storage.add_money(client_id, dec!(100.00)).unwrap();
        account_storage.block_account(client_id).unwrap();

        let withdrawal = Withdrawal {
            client_id,
            transaction_id,
            amount,
        };

        let result = withdrawal.execute(&account_storage, &history);

        assert!(result.is_err());
        let error = result.unwrap_err();
        let account_error = error.downcast_ref::<AccountError>().unwrap();
        assert_eq!(*account_error, AccountError::AccountLocked);

        assert!(history.find_transaction(transaction_id).is_none());
    }

    #[test]
    fn test_dispute_execute_deposit_successful() {
        let account_storage = InMemoryAccountsStorage::new();
        let history = InMemoryTransactionStorage::new();
        let client_id = 1;
        let transaction_id = 100;
        let amount = dec!(50.00);

        account_storage.add_money(client_id, amount).unwrap();
        create_transaction_in_history(
            &history,
            transaction_id,
            client_id,
            amount,
            TransactionInfoType::Deposit,
            TransactionStatus::WithoutDisputes,
        );

        let dispute = Dispute {
            client_id,
            transaction_id,
        };

        assert_eq!(account_storage.get_balance(client_id), Some(amount));
        let initial_transaction = history.find_transaction(transaction_id).unwrap();
        assert_eq!(
            initial_transaction.status,
            TransactionStatus::WithoutDisputes
        );

        let result = dispute.execute(&account_storage, &history);

        assert!(result.is_ok());

        let accounts = account_storage.accounts.read().unwrap();
        let account = accounts.get(&client_id).unwrap();
        assert_eq!(account.available_balance(), Decimal::ZERO);
        assert_eq!(account.held_balance(), amount);
        assert_eq!(account.total_balance(), amount);

        let updated_transaction = history.find_transaction(transaction_id).unwrap();
        assert_eq!(updated_transaction.status, TransactionStatus::Disputed);
    }

    #[test]
    fn test_dispute_execute_withdrawal_successful() {
        let account_storage = InMemoryAccountsStorage::new();
        let history = InMemoryTransactionStorage::new();
        let client_id = 1;
        let transaction_id = 100;
        let withdrawal_amount = dec!(30.00);
        let initial_balance = dec!(70.00);

        account_storage
            .add_money(client_id, initial_balance)
            .unwrap();
        create_transaction_in_history(
            &history,
            transaction_id,
            client_id,
            withdrawal_amount,
            TransactionInfoType::Withdrawal,
            TransactionStatus::WithoutDisputes,
        );

        let dispute = Dispute {
            client_id,
            transaction_id,
        };

        assert_eq!(
            account_storage.get_balance(client_id),
            Some(initial_balance)
        );

        let result = dispute.execute(&account_storage, &history);

        assert!(result.is_ok());

        let accounts = account_storage.accounts.read().unwrap();
        let account = accounts.get(&client_id).unwrap();
        assert_eq!(account.available_balance(), initial_balance);
        assert_eq!(account.held_balance(), withdrawal_amount);
        assert_eq!(account.total_balance(), initial_balance + withdrawal_amount);

        let updated_transaction = history.find_transaction(transaction_id).unwrap();
        assert_eq!(updated_transaction.status, TransactionStatus::Disputed);
    }

    #[test]
    fn test_dispute_execute_transaction_not_found() {
        let account_storage = InMemoryAccountsStorage::new();
        let history = InMemoryTransactionStorage::new();
        let client_id = 1;
        let nonexistent_transaction_id = 999;

        let dispute = Dispute {
            client_id,
            transaction_id: nonexistent_transaction_id,
        };

        let result = dispute.execute(&account_storage, &history);

        assert!(result.is_err());
        let error = result.unwrap_err();
        let transaction_error = error.downcast_ref::<TransactionError>().unwrap();
        assert_eq!(
            *transaction_error,
            TransactionError::OriginTransactionNotFound
        );
    }

    #[rstest]
    #[case(TransactionStatus::Disputed)]
    #[case(TransactionStatus::Resolved)]
    #[case(TransactionStatus::Chargebacked)]
    fn test_dispute_execute_already_disputed(#[case] existing_status: TransactionStatus) {
        let account_storage = InMemoryAccountsStorage::new();
        let history = InMemoryTransactionStorage::new();
        let client_id = 1;
        let transaction_id = 100;
        let amount = dec!(50.00);

        create_transaction_in_history(
            &history,
            transaction_id,
            client_id,
            amount,
            TransactionInfoType::Deposit,
            existing_status,
        );

        let dispute = Dispute {
            client_id,
            transaction_id,
        };

        let result = dispute.execute(&account_storage, &history);

        assert!(result.is_err());
        let error = result.unwrap_err();
        let transaction_error = error.downcast_ref::<TransactionError>().unwrap();
        assert_eq!(
            *transaction_error,
            TransactionError::TransactionMultipleDispute
        );

        let transaction_info = history.find_transaction(transaction_id).unwrap();
        assert_eq!(transaction_info.status, existing_status);
    }

    #[test]
    fn test_dispute_execute_deposit_insufficient_funds_to_hold() {
        let account_storage = InMemoryAccountsStorage::new();
        let history = InMemoryTransactionStorage::new();
        let client_id = 1;
        let transaction_id = 100;
        let deposit_amount = dec!(100.00);
        let current_balance = dec!(50.00);

        account_storage
            .add_money(client_id, current_balance)
            .unwrap();
        create_transaction_in_history(
            &history,
            transaction_id,
            client_id,
            deposit_amount,
            TransactionInfoType::Deposit,
            TransactionStatus::WithoutDisputes,
        );

        let dispute = Dispute {
            client_id,
            transaction_id,
        };

        let result = dispute.execute(&account_storage, &history);

        assert!(result.is_err());
        let error = result.unwrap_err();
        let account_error = error.downcast_ref::<AccountError>().unwrap();
        assert_eq!(*account_error, AccountError::InsufficientMoney);

        let transaction_info = history.find_transaction(transaction_id).unwrap();
        assert_eq!(transaction_info.status, TransactionStatus::WithoutDisputes);
    }

    #[test]
    fn test_dispute_execute_locked_account() {
        let account_storage = InMemoryAccountsStorage::new();
        let history = InMemoryTransactionStorage::new();
        let client_id = 1;
        let transaction_id = 100;
        let amount = dec!(50.00);

        account_storage.add_money(client_id, amount).unwrap();
        account_storage.block_account(client_id).unwrap();
        create_transaction_in_history(
            &history,
            transaction_id,
            client_id,
            amount,
            TransactionInfoType::Deposit,
            TransactionStatus::WithoutDisputes,
        );

        let dispute = Dispute {
            client_id,
            transaction_id,
        };

        let result = dispute.execute(&account_storage, &history);

        assert!(result.is_err());
        let error = result.unwrap_err();
        let account_error = error.downcast_ref::<AccountError>().unwrap();
        assert_eq!(*account_error, AccountError::AccountLocked);
    }

    #[test]
    fn test_resolve_successful_for_disputed_deposit() {
        let storage = InMemoryAccountsStorage::new();
        let history = InMemoryTransactionStorage::new();
        let client_id = 1;
        let transaction_id = 100_u64;
        let amount = dec!(50.0);

        storage.create_user(client_id);
        storage.add_money(client_id, amount).unwrap();
        storage.hold_money(client_id, amount).unwrap();
        create_transaction_in_history(
            &history,
            transaction_id,
            client_id,
            amount,
            TransactionInfoType::Deposit,
            TransactionStatus::Disputed,
        );

        let resolve = Resolve {
            client_id,
            transaction_id,
        };
        let result = resolve.execute(&storage, &history);

        assert!(result.is_ok());

        let transaction_info = history.find_transaction(transaction_id).unwrap();
        assert_eq!(transaction_info.status, TransactionStatus::Resolved);

        let accounts = storage.accounts.read().unwrap();
        let account = accounts.get(&client_id).unwrap();
        assert_eq!(account.available_balance(), amount);
        assert_eq!(account.held_balance(), dec!(0));
    }

    #[test]
    fn test_resolve_successful_for_disputed_withdrawal() {
        let storage = InMemoryAccountsStorage::new();
        let history = InMemoryTransactionStorage::new();
        let client_id = 1;
        let transaction_id = 100;
        let amount = dec!(30.0);

        storage.create_user(client_id);
        storage.add_money(client_id, amount * dec!(2)).unwrap();
        storage.hold_money(client_id, amount).unwrap();

        create_transaction_in_history(
            &history,
            transaction_id,
            client_id,
            amount,
            TransactionInfoType::Withdrawal,
            TransactionStatus::Disputed,
        );
        let resolve = Resolve {
            client_id,
            transaction_id,
        };
        let result = resolve.execute(&storage, &history);

        assert!(result.is_ok());

        let transaction_info = history.find_transaction(transaction_id).unwrap();
        assert_eq!(transaction_info.status, TransactionStatus::Resolved);

        let accounts = storage.accounts.read().unwrap();
        let account = accounts.get(&client_id).unwrap();
        assert_eq!(account.held_balance(), dec!(0));
        assert_eq!(account.available_balance(), amount * dec!(2));
    }

    #[test]
    fn test_resolve_transaction_not_found() {
        let storage = InMemoryAccountsStorage::new();
        let history = InMemoryTransactionStorage::new();
        let client_id = 1;
        let transaction_id = 999;

        storage.create_user(client_id);

        let resolve = Resolve {
            client_id,
            transaction_id,
        };
        let result = resolve.execute(&storage, &history);

        assert!(result.is_err());
        let error = result.unwrap_err();
        let transaction_error = error.downcast_ref::<TransactionError>().unwrap();
        assert_eq!(
            *transaction_error,
            TransactionError::OriginTransactionNotFound
        );
    }

    #[test]
    fn test_resolve_transaction_not_disputed() {
        let storage = InMemoryAccountsStorage::new();
        let history = InMemoryTransactionStorage::new();
        let client_id = 1;
        let transaction_id = 100;
        let amount = dec!(25.0);

        storage.create_user(client_id);
        let transaction_info = TransactionInfo {
            client_id,
            transaction_id,
            amount,
            status: TransactionStatus::WithoutDisputes,
            transaction_type: TransactionInfoType::Deposit,
        };
        history.add_transaction(transaction_info).unwrap();

        let resolve = Resolve {
            client_id,
            transaction_id,
        };
        let result = resolve.execute(&storage, &history);

        assert!(result.is_err());
        let error = result.unwrap_err();
        let transaction_error = error.downcast_ref::<TransactionError>().unwrap();
        assert_eq!(*transaction_error, TransactionError::TransactionNotDisputed);
    }

    #[test]
    fn test_resolve_transaction_already_resolved() {
        let storage = InMemoryAccountsStorage::new();
        let history = InMemoryTransactionStorage::new();
        let client_id = 1;
        let transaction_id = 100;
        let amount = dec!(40.0);

        storage.create_user(client_id);
        let transaction_info = TransactionInfo {
            client_id,
            transaction_id,
            amount,
            status: TransactionStatus::Resolved,
            transaction_type: TransactionInfoType::Deposit,
        };
        history.add_transaction(transaction_info).unwrap();

        let resolve = Resolve {
            client_id,
            transaction_id,
        };
        let result = resolve.execute(&storage, &history);

        assert!(result.is_err());

        let error = result.unwrap_err();
        let transaction_error = error.downcast_ref::<TransactionError>().unwrap();
        assert_eq!(*transaction_error, TransactionError::TransactionNotDisputed);
    }

    #[test]
    fn test_resolve_transaction_chargebacked() {
        let storage = InMemoryAccountsStorage::new();
        let history = InMemoryTransactionStorage::new();
        let client_id = 1;
        let transaction_id = 100;
        let amount = dec!(60.0);

        storage.create_user(client_id);
        let transaction_info = TransactionInfo {
            client_id,
            transaction_id,
            amount,
            status: TransactionStatus::Chargebacked,
            transaction_type: TransactionInfoType::Deposit,
        };
        history.add_transaction(transaction_info).unwrap();

        let resolve = Resolve {
            client_id,
            transaction_id,
        };
        let result = resolve.execute(&storage, &history);

        assert!(result.is_err());
        let error = result.unwrap_err();
        let transaction_error = error.downcast_ref::<TransactionError>().unwrap();
        assert_eq!(*transaction_error, TransactionError::TransactionNotDisputed);
    }

    #[test]
    fn test_resolve_unhold_money_error_propagation() {
        let storage = InMemoryAccountsStorage::new();
        let history = InMemoryTransactionStorage::new();
        let client_id = 1;
        let transaction_id = 100;
        let amount = dec!(100.0);

        storage.create_user(client_id);
        create_transaction_in_history(
            &history,
            transaction_id,
            client_id,
            amount,
            TransactionInfoType::Deposit,
            TransactionStatus::Disputed,
        );

        let resolve = Resolve {
            client_id,
            transaction_id,
        };
        let result = resolve.execute(&storage, &history);

        assert!(result.is_err());
        let error = result.unwrap_err();
        let transaction_error = error.downcast_ref::<AccountError>().unwrap();
        assert_eq!(*transaction_error, AccountError::InsufficientMoney);
    }

    #[test]
    fn test_chargeback_successful_for_disputed_deposit() {
        let storage = InMemoryAccountsStorage::new();
        let history = InMemoryTransactionStorage::new();
        let client_id = 1;
        let transaction_id = 100;
        let amount = dec!(50.0);

        storage.create_user(client_id);
        storage.add_money(client_id, amount * dec!(2)).unwrap();
        storage.hold_money(client_id, amount).unwrap();
        create_transaction_in_history(
            &history,
            transaction_id,
            client_id,
            amount,
            TransactionInfoType::Deposit,
            TransactionStatus::Disputed,
        );

        let chargeback = Chargeback {
            client_id,
            transaction_id,
        };
        let result = chargeback.execute(&storage, &history);

        assert!(result.is_ok());

        let transaction_info = history.find_transaction(transaction_id).unwrap();
        assert_eq!(transaction_info.status, TransactionStatus::Chargebacked);

        let accounts = storage.accounts.read().unwrap();
        let account = accounts.get(&client_id).unwrap();
        assert_eq!(account.held_balance(), dec!(0));
        assert_eq!(account.available_balance(), amount);
        assert!(account.is_locked());
    }

    #[test]
    fn test_chargeback_successful_for_disputed_withdrawal() {
        let storage = InMemoryAccountsStorage::new();
        let history = InMemoryTransactionStorage::new();
        let client_id = 1;
        let transaction_id = 100;
        let amount = dec!(30.0);

        storage.create_user(client_id);
        storage.add_money(client_id, amount * dec!(3)).unwrap();
        storage.hold_money(client_id, amount).unwrap();
        create_transaction_in_history(
            &history,
            transaction_id,
            client_id,
            amount,
            TransactionInfoType::Withdrawal,
            TransactionStatus::Disputed,
        );

        let chargeback = Chargeback {
            client_id,
            transaction_id,
        };
        let result = chargeback.execute(&storage, &history);

        assert!(result.is_ok());

        let transaction_info = history.find_transaction(transaction_id).unwrap();
        assert_eq!(transaction_info.status, TransactionStatus::Chargebacked);

        let accounts = storage.accounts.read().unwrap();
        let account = accounts.get(&client_id).unwrap();
        assert_eq!(account.held_balance(), dec!(0));
        assert_eq!(account.available_balance(), amount * dec!(2));
        assert!(account.is_locked());
    }

    #[test]
    fn test_chargeback_transaction_not_found() {
        let storage = InMemoryAccountsStorage::new();
        let history = InMemoryTransactionStorage::new();
        let client_id = 1;
        let transaction_id = 999;

        storage.create_user(client_id);

        let chargeback = Chargeback {
            client_id,
            transaction_id,
        };
        let result = chargeback.execute(&storage, &history);

        assert!(result.is_err());
        let error = result.unwrap_err();
        let transaction_error = error.downcast_ref::<TransactionError>().unwrap();
        assert_eq!(
            *transaction_error,
            TransactionError::OriginTransactionNotFound
        );
    }

    #[test]
    fn test_chargeback_transaction_not_disputed() {
        let storage = InMemoryAccountsStorage::new();
        let history = InMemoryTransactionStorage::new();
        let client_id = 1;
        let transaction_id = 100;
        let amount = dec!(25.0);

        storage.create_user(client_id);
        create_transaction_in_history(
            &history,
            transaction_id,
            client_id,
            amount,
            TransactionInfoType::Deposit,
            TransactionStatus::WithoutDisputes,
        );

        let chargeback = Chargeback {
            client_id,
            transaction_id,
        };
        let result = chargeback.execute(&storage, &history);

        assert!(result.is_err());
        let error = result.unwrap_err();
        let transaction_error = error.downcast_ref::<TransactionError>().unwrap();
        assert_eq!(*transaction_error, TransactionError::TransactionNotDisputed);
    }

    #[test]
    fn test_chargeback_transaction_already_resolved() {
        let storage = InMemoryAccountsStorage::new();
        let history = InMemoryTransactionStorage::new();
        let client_id = 1;
        let transaction_id = 100;
        let amount = dec!(40.0);

        storage.create_user(client_id);
        create_transaction_in_history(
            &history,
            transaction_id,
            client_id,
            amount,
            TransactionInfoType::Deposit,
            TransactionStatus::Resolved,
        );

        let chargeback = Chargeback {
            client_id,
            transaction_id,
        };
        let result = chargeback.execute(&storage, &history);

        assert!(result.is_err());
        let error = result.unwrap_err();
        let transaction_error = error.downcast_ref::<TransactionError>().unwrap();
        assert_eq!(*transaction_error, TransactionError::TransactionNotDisputed);
    }

    #[test]
    fn test_chargeback_transaction_already_chargebacked() {
        let storage = InMemoryAccountsStorage::new();
        let history = InMemoryTransactionStorage::new();
        let client_id = 1;
        let transaction_id = 100;
        let amount = dec!(60.0);

        storage.create_user(client_id);
        create_transaction_in_history(
            &history,
            transaction_id,
            client_id,
            amount,
            TransactionInfoType::Deposit,
            TransactionStatus::Chargebacked,
        );

        let chargeback = Chargeback {
            client_id,
            transaction_id,
        };
        let result = chargeback.execute(&storage, &history);

        assert!(result.is_err());
        let error = result.unwrap_err();
        let transaction_error = error.downcast_ref::<TransactionError>().unwrap();
        assert_eq!(*transaction_error, TransactionError::TransactionNotDisputed);
    }

    #[test]
    fn test_chargeback_unhold_money_error_propagation() {
        let storage = InMemoryAccountsStorage::new();
        let history = InMemoryTransactionStorage::new();
        let client_id = 1;
        let transaction_id = 100;
        let amount = dec!(100.0);

        storage.create_user(client_id);
        storage.add_money(client_id, amount).unwrap();
        create_transaction_in_history(
            &history,
            transaction_id,
            client_id,
            amount,
            TransactionInfoType::Deposit,
            TransactionStatus::Disputed,
        );

        let chargeback = Chargeback {
            client_id,
            transaction_id,
        };
        let result = chargeback.execute(&storage, &history);

        assert!(result.is_err());
        let error = result.unwrap_err();
        let transaction_error = error.downcast_ref::<AccountError>().unwrap();
        assert_eq!(*transaction_error, AccountError::InsufficientMoney);
    }

    #[test]
    fn test_chargeback_withdraw_money_error_propagation() {
        let storage = InMemoryAccountsStorage::new();
        let history = InMemoryTransactionStorage::new();
        let client_id = 1;
        let transaction_id = 100;
        let amount = dec!(100.0);

        storage.create_user(client_id);
        storage.add_money(client_id, amount / dec!(2)).unwrap();
        storage.hold_money(client_id, amount / dec!(2)).unwrap();
        create_transaction_in_history(
            &history,
            transaction_id,
            client_id,
            amount,
            TransactionInfoType::Deposit,
            TransactionStatus::Disputed,
        );

        let chargeback = Chargeback {
            client_id,
            transaction_id,
        };
        let result = chargeback.execute(&storage, &history);

        assert!(result.is_err());
        let error = result.unwrap_err();
        let transaction_error = error.downcast_ref::<AccountError>().unwrap();
        assert_eq!(*transaction_error, AccountError::InsufficientMoney);
    }
}
