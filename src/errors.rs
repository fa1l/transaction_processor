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
    TransactionMultipleDispute,
    EmptyAmount,
}

impl fmt::Display for TransactionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransactionError::NegativeAmount => write!(f, "Transaction provides negative amount"),
            TransactionError::OriginTransactionNotFound => {
                write!(f, "Origin transaction not found")
            }
            TransactionError::TransactionNotDisputed => write!(f, "Transaction not disputed"),
            TransactionError::TransactionMultipleDispute => {
                write!(f, "Multiple transaction dispute")
            }
            TransactionError::EmptyAmount => {
                write!(f, "Transaction goes with empty amount but it shouldn't")
            }
        }
    }
}

impl std::error::Error for TransactionError {}

#[derive(Debug, Clone, PartialEq)]
pub enum TransactionLogError {
    InvalidTransactionType,
    MissingAmount,
}

impl fmt::Display for TransactionLogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransactionLogError::InvalidTransactionType => {
                write!(f, "Invalid transaction type in entry")
            }
            TransactionLogError::MissingAmount => write!(f, "Missing amount in entry"),
        }
    }
}

impl std::error::Error for TransactionLogError {}

#[derive(Debug, Clone, PartialEq)]
pub enum TransactionHistoryError {
    TransactionAlreadyExists,
    UnknownTransaction,
    InvalidStatusTransition,
}

impl fmt::Display for TransactionHistoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransactionHistoryError::TransactionAlreadyExists => write!(
                f,
                "Trying to add transaction that already exists in history"
            ),
            TransactionHistoryError::UnknownTransaction => write!(f, "Unknown transaction ID"),
            TransactionHistoryError::InvalidStatusTransition => {
                write!(f, "Can't complete transaction status update")
            }
        }
    }
}

impl std::error::Error for TransactionHistoryError {}
