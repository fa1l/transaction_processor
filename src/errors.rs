use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum AccountError {
    BalanceOverflow,
    InsufficientMoney,
    AccountLocked,
    AccountNotFound,
}

impl fmt::Display for AccountError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccountError::BalanceOverflow => write!(f, "Balance overflow happened"),
            AccountError::InsufficientMoney => write!(f, "Insufficient money"),
            AccountError::AccountLocked => write!(f, "Account is locked"),
            &AccountError::AccountNotFound => write!(f, "Account not found"),
        }
    }
}

impl std::error::Error for AccountError {}

#[derive(Debug, Clone, PartialEq)]
pub enum TransactionError {
    NegativeAmount,
    OriginTransactionNotFound,
    TransactionNotDisputed,
    MultipleTransactionDispute,
}

impl fmt::Display for TransactionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransactionError::NegativeAmount => write!(f, "Transaction provides negative amount"),
            TransactionError::OriginTransactionNotFound => {
                write!(f, "Origin transaction not found")
            }
            TransactionError::TransactionNotDisputed => write!(f, "Transaction not disputed"),
            TransactionError::MultipleTransactionDispute => {
                write!(f, "Multiple transaction dispute")
            }
        }
    }
}

impl std::error::Error for TransactionError {}
