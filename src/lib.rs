use tokio::{
    io,
    sync::mpsc,
};
pub use crate::data::{
    Chaser,
    Line,
};

mod r#async;
mod data;
mod sync;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)] Io(#[from] io::Error),
    #[error(transparent)] Send(#[from] mpsc::error::SendError<String>),
    #[error(transparent)] Custom(Box<dyn std::error::Error + Send + Sync>),
}
