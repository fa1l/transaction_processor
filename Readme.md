# Transaction Service

A high-performance, asynchronous transaction processing system built in Rust that handles financial transactions with support for deposits, withdrawals, disputes, resolutions, and chargebacks.

## Features

- **Transaction Processing**: Support for deposits, withdrawals, disputes, resolutions, and chargebacks
- **Account Management**: Client account creation with balance tracking (available, held, total)
- **Account Locking**: Automatic account locking on chargebacks for fraud prevention
- **Dispute Resolution**: Complete dispute workflow with proper state transitions
- **CSV I/O**: Asynchronous CSV file processing for input and output
- **Error Handling**: Comprehensive error handling with detailed error types
- **High Performance**: Built with async/await and tokio for concurrent processing
- **Memory Safety**: Written in Rust with zero-cost abstractions

## Architecture

The system is built with a modular architecture:

- **`transactions.rs`**: Core transaction types and execution logic
- **`storage.rs`**: Account storage with thread-safe in-memory implementation
- **`history.rs`**: Transaction history tracking and status management
- **`transactions_processor.rs`**: Main transaction processing engine
- **`csv_utils.rs`**: Asynchronous CSV reading and writing utilities
- **`errors.rs`**: Comprehensive error type definitions
- **`main.rs`**: Application entry point and async runtime setup

## Transaction Types

### Basic Transactions
- **Deposit**: Add funds to a client account
- **Withdrawal**: Remove funds from a client account (with balance validation)

### Dispute Management
- **Dispute**: Challenge a previous transaction, holds the disputed amount
- **Resolve**: Resolve a dispute in favor of the client, releases held funds
- **Chargeback**: Resolve a dispute against the client, withdraws funds and locks account

## Transaction State Machine

```
WithoutDisputes → Disputed → Resolved
                           → Chargebacked
```

- Transactions start in `WithoutDisputes` state
- Only `WithoutDisputes` transactions can be disputed
- Disputed transactions can be either resolved or charged back
- State transitions are strictly validated

## Usage

### Running the Application

```bash
cargo run -- input.csv > output.csv
```

### Input Format (CSV)

```csv
type,client,tx,amount
deposit,1,1,1.0
withdrawal,1,2,0.5
dispute,1,1,
resolve,1,1,
chargeback,1,1,
```

### Output Format (CSV)

```csv
client,available,held,total,locked
1,1.5,0.0,1.5,false
2,2.0,0.0,2.0,false
```

## Development

### Prerequisites

- Rust 1.89+ (2024 edition)
- Cargo

### Setup

```bash
# Install development tools
make init

# Format code
make pretty

# Run linting
make lint

# Run tests with coverage
make tests

# Generate HTML coverage report
make codecov
```

### Dependencies

- **tokio**: Async runtime and utilities
- **csv-async**: Asynchronous CSV processing
- **rust_decimal**: Precise decimal arithmetic for financial calculations
- **serde**: Serialization/deserialization
- **tracing**: Structured logging
- **rstest**: Parameterized testing

## Testing

The project includes comprehensive test coverage with:

- Unit tests for all transaction types
- Integration tests for the transaction processor
- Error handling validation
- Edge case testing (boundary values, precision handling)
- State transition validation

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with coverage (minimum 75%)
make tests

# Generate HTML coverage report
make codecov
```

## Integration Testing

The project includes a comprehensive integration test file `test_transactions.csv` that can be used to verify the correctness of the transaction processing system.

### Running Integration Tests

```bash
# Run the test file and output results
cargo run -- test_transactions.csv > test_results.csv
```

### Test File Contents

The `test_transactions.csv` file contains various transaction scenarios including:

- **Basic Operations**: Deposits and withdrawals with various amounts
- **Error Cases**: Negative amounts, insufficient funds
- **Dispute Workflows**: Multiple disputes, resolves, and chargebacks
- **Edge Cases**: Very small amounts, boundary conditions
- **Invalid Operations**: Non-existent transactions, blocked account operations

### Expected Test Results

When processing `test_transactions.csv`, the system should produce the following account states:

```csv
client,available,held,total,locked
1,175,0,175,false
2,40,0,40,false
3,150,0,150,true
4,300,0,300,false
5,800,200.0,1000,false
6,0.005,0,0.005,false
7,25.5,0,25.5,false
```

## Code Quality & Linting

The project maintains high code quality standards through comprehensive automated checks:

### Linting Tools
- **Clippy**: Advanced Rust linter for catching common mistakes and suggesting improvements
- **rustfmt**: Automatic code formatting to ensure consistent style
- **cargo check**: Fast compilation checks for early error detection

### Security & Vulnerability Scanning
- **cargo audit**: Scans dependencies for known security vulnerabilities
- **Security-focused linting**: Clippy rules configured to catch potential security issues

### Code Quality Checks
- **Typos detection**: Automated spell checking across the entire codebase using `typos-cli`
- **Dead code detection**: Identifies unused code and imports
- **Performance linting**: Clippy rules for performance optimization suggestions

### Automated Quality Pipeline

```bash
# Run all quality checks
make lint

# Individual checks
cargo fmt -- --check          # Format checking
cargo check --all-targets     # Compilation check
cargo clippy -- -D warnings   # Linting with warnings as errors
cargo audit                    # Security vulnerability scan
typos .                       # Typo detection
```

## Error Handling

The system provides detailed error handling for:

### Account Errors
- `BalanceOverflow`: Arithmetic overflow in balance calculations
- `InsufficientMoney`: Insufficient funds for operations
- `AccountLocked`: Operations on locked accounts
- `AccountNotFound`: Operations on non-existent accounts

### Transaction Errors
- `NegativeAmount`: Negative amounts in deposits/withdrawals
- `OriginTransactionNotFound`: Referenced transaction doesn't exist
- `TransactionNotDisputed`: Invalid state for dispute operations
- `TransactionMultipleDispute`: Attempting to dispute already disputed transaction
- `EmptyAmount`: Missing required amount field

## Performance Characteristics

- **Asynchronous I/O**: Non-blocking CSV processing
- **Memory Efficient**: In-memory storage with RwLock for concurrent access
- **Zero-Copy**: Minimal data copying with efficient data structures
- **Concurrent Processing**: Multi-threaded transaction processing

## Security Considerations

- **Account Locking**: Automatic account locking on chargebacks
- **State Validation**: Strict transaction state machine enforcement
- **Balance Validation**: Comprehensive balance checking before operations
- **Precision Arithmetic**: Uses `rust_decimal` for accurate financial calculations

## AI Assistance Acknowledgment

This project utilized AI assistance for:
- **Test Generation**: Comprehensive test suites were generated with AI assistance to ensure thorough coverage of all transaction types, error cases, and edge conditions. Tests were taken and reviewed manually.
- **Error Display Formatting**: AI was used to generate consistent and user-friendly error message formatting in the `Display` implementations
- **Readme Generation**: AI was used to generate content for this Readme file

