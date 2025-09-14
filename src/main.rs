pub mod csv_utils;
pub mod errors;
pub mod history;
pub mod storage;
pub mod transactions;
pub mod transactions_processor;

use transactions_processor::InMemoryTransactionProcessor;

use crate::transactions_processor::TransactionProcessor;

const CHANNEL_SIZE: usize = 4096;

#[tokio::main]
async fn main() {
    let file_path = std::env::args()
        .nth(1)
        .expect("Usage: cargo run -- <input.csv> > <output.csv>");

    let (sender, mut receiver) = tokio::sync::mpsc::channel(CHANNEL_SIZE);

    tokio::spawn(csv_utils::read_data(file_path, sender));

    let transactions_processor = InMemoryTransactionProcessor::new();
    while let Some(tx) = receiver.recv().await {
        transactions_processor.process(tx).ok();
    }

    csv_utils::output_data(&transactions_processor).await;
}
