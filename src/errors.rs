use tokio::{
    io,
    sync::mpsc,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)] Io(#[from] io::Error),
    #[error(transparent)] Send(#[from] mpsc::error::SendError<String>),
    #[error(transparent)] Custom(Box<dyn std::error::Error + Send + Sync>),
}
