use {
    std::{
        io,
        sync::mpsc as channel_mpsc,
    },
    tokio::sync::mpsc as stream_mpsc,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)] IoError(#[from] io::Error),
    #[error(transparent)] ChannelSendError(#[from] channel_mpsc::SendError<String>),
    #[error(transparent)] StreamSendError(#[from] stream_mpsc::error::SendError<String>),
    #[error(transparent)] Custom(Box<dyn std::error::Error + Send + Sync>),
}
