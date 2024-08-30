#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Rustls error: {0}")]
    RustlsError(#[from] rustls::Error),
    #[error("Invalid message")]
    InvalidMessage,
    #[error("Serde JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Tungstenite error: {0}")]
    TungsteniteError(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("Error executing command: {0}")]
    AdbError(String),
    #[error("Device Error: {0}")]
    DeviceError(#[from] forensic_adb::DeviceError),
    #[error("Error in database: {0}")]
    DatabaseError(#[from] redb::DatabaseError),
    #[error("Error in transaction: {0}")]
    DatabaseTransactionError(#[from] redb::TransactionError),
    #[error("Error in storage: {0}")]
    DatabaseStorageError(#[from] redb::StorageError),
    #[error("Error in table: {0}")]
    DatabaseTableError(#[from] redb::TableError),
    #[error("Error in commit: {0}")]
    DatabaseCommitError(#[from] redb::CommitError),
    // PemFileError(rustls_pemfile::PemError),
    // RustlsError(rustls::TLSError),
    // #[error("Tokio error: {0}")]
    // TokioError(#[from] tokio::io::Error),
}
