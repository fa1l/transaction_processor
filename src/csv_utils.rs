use csv_async::Trim;
use rust_decimal::Decimal;
use serde::Serialize;
use tokio::sync::mpsc::Sender;
use tokio_stream::StreamExt;
use tracing::error;

use crate::{
    storage::ClientId,
    transactions_processor::{InMemoryTransactionProcessor, TransactionLogEntry},
};

#[derive(Serialize)]
pub struct CsvAccountData {
    #[serde(rename = "client")]
    client_id: ClientId,
    available: Decimal,
    held: Decimal,
    total: Decimal,
    locked: bool,
}

pub async fn read_data(file_path: String, sender: Sender<TransactionLogEntry>) {
    let mut file = tokio::fs::File::open(&file_path)
        .await
        .expect("Can't read file");
    let mut reader = csv_async::AsyncReaderBuilder::new()
        .trim(Trim::All)
        .create_deserializer(&mut file);
    let mut records = reader.deserialize::<TransactionLogEntry>();
    while let Some(fetched_tx) = records.next().await {
        match fetched_tx {
            Ok(transaction_entry) => {
                sender.send(transaction_entry).await.ok();
            }
            Err(e) => {
                error!("Can't deserialize data into TransactionLogEntry, got: {e:#?}");
                continue;
            }
        }
    }
}

pub async fn output_data(transaction_processor: &InMemoryTransactionProcessor) {
    let accounts_storage = transaction_processor.get_accounts_storage();
    let account_logs = accounts_storage
        .accounts
        .read()
        .unwrap()
        .iter()
        .map(|(client_id, user_account)| CsvAccountData {
            client_id: *client_id,
            available: user_account.available_balance(),
            held: user_account.held_balance(),
            total: user_account.total_balance(),
            locked: user_account.is_locked(),
        })
        .collect::<Vec<CsvAccountData>>();

    let mut writer = csv_async::AsyncWriterBuilder::new().create_serializer(tokio::io::stdout());
    for log in account_logs {
        writer.serialize(log).await.ok();
    }
}
