use std::collections::hash_map::Entry;
use std::sync::RwLock;
use std::{collections::HashMap, error::Error};
use tracing::{error, warn};

use rust_decimal::Decimal;

use crate::errors::{AccountError, TransactionError};

pub type UserId = u16;

pub struct UserAccount {
    available_amount: Decimal,
    held_amount: Decimal,
    locked: bool,
}

impl UserAccount {
    pub fn total_balance(&self) -> Decimal {
        return self.available_amount + self.held_amount;
    }

    pub fn available_balance(&self) -> Decimal {
        return self.available_amount;
    }

    pub fn held_balance(&self) -> Decimal {
        return self.held_amount;
    }
}

impl Default for UserAccount {
    fn default() -> Self {
        UserAccount {
            available_amount: Decimal::ZERO,
            held_amount: Decimal::ZERO,
            locked: false,
        }
    }
}

pub trait Storage {
    fn create_user(&self, user_id: UserId);
    fn add_money(&self, user_id: UserId, amount: Decimal) -> Result<(), Box<dyn Error>>;
    fn withdraw_money(&self, user_id: UserId, amount: Decimal) -> Result<(), Box<dyn Error>>;
    fn hold_money(&self, user_id: UserId, amount: Decimal);
}

pub struct InMemoryAccountsStorage {
    accounts: RwLock<HashMap<UserId, UserAccount>>,
}

impl InMemoryAccountsStorage {
    pub fn new() -> Self {
        Self {
            accounts: RwLock::new(HashMap::new()),
        }
    }

    pub fn is_locked(&self, user_id: UserId) -> Option<bool> {
        let storage = self.accounts.read().unwrap();
        match storage.get(&user_id) {
            Some(account) => Some(account.locked),
            None => {
                warn!("Unknown account");
                None
            }
        }
    }

    pub fn get_balance(&self, user_id: UserId) -> Option<Decimal> {
        let storage = self.accounts.read().unwrap();
        match storage.get(&user_id) {
            Some(account) => {
                if account.locked {
                    warn!("Looking blocked account balance");
                }
                Some(account.available_amount)
            }
            None => {
                warn!("Unknown account");
                None
            }
        }
    }
}

impl Storage for InMemoryAccountsStorage {
    fn create_user(&self, user_id: UserId) {
        let mut storage = self.accounts.write().unwrap();
        match storage.entry(user_id) {
            Entry::Vacant(entry) => {
                entry.insert(UserAccount::default());
            }
            Entry::Occupied(_entry) => warn!("Attempting to create account which already exists"),
        };
    }

    fn add_money(&self, user_id: UserId, amount: Decimal) -> Result<(), Box<dyn Error>> {
        let mut storage = self.accounts.write().unwrap();
        match storage.entry(user_id) {
            Entry::Vacant(entry) => {
                entry.insert(UserAccount {
                    available_amount: amount,
                    held_amount: Decimal::ZERO,
                    locked: false,
                });
            }
            Entry::Occupied(mut entry) => {
                let account = entry.get_mut();
                if account.locked {
                    warn!("Trying to add money to locked account");
                    return Err(Box::new(AccountError::AccountLocked));
                }
                match account.available_amount.checked_add(amount) {
                    Some(new_balance) => account.available_amount = new_balance,
                    None => {
                        error!(
                            "Got balance overflow for account {user_id}, need to solve this manually"
                        );
                        return Err(Box::new(AccountError::BalanceOverflow));
                    }
                };
            }
        }
        Ok(())
    }

    fn withdraw_money(&self, user_id: UserId, amount: Decimal) -> Result<(), Box<dyn Error>> {
        let mut storage = self.accounts.write().unwrap();
        match storage.entry(user_id) {
            Entry::Vacant(_entry) => {
                warn!("Trying to withdraw money from unknown account");
                return Err(Box::new(AccountError::AccountNotFound));
            }
            Entry::Occupied(mut entry) => {
                let account = entry.get_mut();
                if account.locked {
                    warn!("Trying to withdraw money from locked account");
                    return Err(Box::new(AccountError::AccountLocked));
                }
                if account.available_amount < amount {
                    warn!("Trying to withdraw more money then account has");
                    return Err(Box::new(AccountError::InsufficientMoney));
                }
                match account.available_amount.checked_sub(amount) {
                    Some(new_balance) => account.available_amount = new_balance,
                    None => {
                        // kind of impossible, but let it be
                        error!(
                            "Got balance overflow for account {user_id}, need to solve this manually"
                        );
                        return Err(Box::new(AccountError::BalanceOverflow));
                    }
                };
            }
        }
        Ok(())
    }

    fn hold_money(&self, user_id: UserId, amount: Decimal) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::dec;

    #[test]
    fn test_add_money_creates_new_user_account() {
        let storage = InMemoryAccountsStorage::new();
        let user_id = 1;
        let amount = dec!(100.500);
        let result = storage.add_money(user_id, amount);

        assert!(result.is_ok());
        assert_eq!(storage.get_balance(user_id), Some(amount));
        assert_eq!(storage.is_locked(user_id), Some(false));
    }

    #[test]
    fn test_add_money_to_existing_account() {
        let storage = InMemoryAccountsStorage::new();
        let user_id = 1;
        let initial_amount = dec!(50.25);
        let additional_amount = dec!(25.75);
        let expected_total = dec!(76.00);

        storage.add_money(user_id, initial_amount).unwrap();
        let result = storage.add_money(user_id, additional_amount);

        assert!(result.is_ok());
        assert_eq!(storage.get_balance(user_id), Some(expected_total));
    }

    #[test]
    fn test_add_money_to_locked_account_returns_error() {
        let mut storage = InMemoryAccountsStorage::new();
        let user_id = 1;
        let amount = dec!(100.00);

        storage.add_money(user_id, amount).unwrap();
        storage
            .accounts
            .get_mut()
            .unwrap()
            .get_mut(&user_id)
            .unwrap()
            .locked = true;

        let result = storage.add_money(user_id, dec!(50.00));
        assert!(result.is_err());

        let error = result.unwrap_err();
        let account_error = error.downcast_ref::<AccountError>().unwrap();
        assert_eq!(*account_error, AccountError::AccountLocked);
        assert_eq!(storage.get_balance(user_id), Some(amount));
    }

    #[test]
    fn test_add_money_multiple_users() {
        let storage = InMemoryAccountsStorage::new();
        let user1_id = 1;
        let user2_id = 2;
        let amount1 = dec!(100.00);
        let amount2 = dec!(200.50);

        let result1 = storage.add_money(user1_id, amount1);
        let result2 = storage.add_money(user2_id, amount2);

        assert!(result1.is_ok());
        assert!(result2.is_ok());
        assert_eq!(storage.get_balance(user1_id), Some(amount1));
        assert_eq!(storage.get_balance(user2_id), Some(amount2));
    }

    #[test]
    fn test_add_money_overflow_protection() {
        let storage = InMemoryAccountsStorage::new();
        let user_id = 1;
        let max_decimal = Decimal::MAX;

        storage.add_money(user_id, max_decimal).unwrap();
        let result = storage.add_money(user_id, dec!(1.00));

        assert!(result.is_err());
        let error = result.unwrap_err();
        let account_error = error.downcast_ref::<AccountError>().unwrap();
        assert_eq!(*account_error, AccountError::BalanceOverflow);
        assert_eq!(storage.get_balance(user_id), Some(max_decimal));
    }

    #[test]
    fn test_withdraw_money_successful() {
        let storage = InMemoryAccountsStorage::new();
        let user_id = 1;
        let initial_amount = dec!(100.00);
        let withdraw_amount = dec!(30.00);
        let expected_balance = dec!(70.00);

        storage.add_money(user_id, initial_amount).unwrap();
        let result = storage.withdraw_money(user_id, withdraw_amount);

        assert!(result.is_ok());
        assert_eq!(storage.get_balance(user_id), Some(expected_balance));
    }

    #[test]
    fn test_withdraw_money_insufficient_funds() {
        let storage = InMemoryAccountsStorage::new();
        let user_id = 1;
        let initial_amount = dec!(50.00);
        let withdraw_amount = dec!(100.00);

        storage.add_money(user_id, initial_amount).unwrap();
        let result = storage.withdraw_money(user_id, withdraw_amount);

        assert!(result.is_err());
        let error = result.unwrap_err();
        let account_error = error.downcast_ref::<AccountError>().unwrap();
        assert_eq!(*account_error, AccountError::InsufficientMoney);
        assert_eq!(storage.get_balance(user_id), Some(initial_amount));
    }

    #[test]
    fn test_withdraw_money_from_nonexistent_account() {
        let storage = InMemoryAccountsStorage::new();
        let user_id = 999;
        let withdraw_amount = dec!(50.00);

        let result = storage.withdraw_money(user_id, withdraw_amount);

        assert!(result.is_err());
        let error = result.unwrap_err();
        let account_error = error.downcast_ref::<AccountError>().unwrap();
        assert_eq!(*account_error, AccountError::AccountNotFound);
    }

    #[test]
    fn test_withdraw_money_from_locked_account() {
        let storage = InMemoryAccountsStorage::new();
        let user_id = 1;
        let initial_amount = dec!(100.00);
        let withdraw_amount = dec!(30.00);

        storage.add_money(user_id, initial_amount).unwrap();
        {
            let mut accounts = storage.accounts.write().unwrap();
            accounts.get_mut(&user_id).unwrap().locked = true;
        }
        let result = storage.withdraw_money(user_id, withdraw_amount);

        assert!(result.is_err());
        let error = result.unwrap_err();
        let account_error = error.downcast_ref::<AccountError>().unwrap();
        assert_eq!(*account_error, AccountError::AccountLocked);
        assert_eq!(storage.get_balance(user_id), Some(initial_amount));
    }
}
