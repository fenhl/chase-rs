use std::error::Error;
use std::io;
use std::sync::mpsc as channel_mpsc;

use futures::sync::mpsc as stream_mpsc;

#[derive(Debug, thiserror::Error)]
pub enum ChaseError {
    #[error(transparent)] IoError(#[from] io::Error),
    #[error(transparent)] ChannelSendError(#[from] channel_mpsc::SendError<String>),
    #[error(transparent)] StreamSendError(#[from] stream_mpsc::SendError<String>),
    #[error(transparent)] Custom(Box<dyn Error + Send + Sync>),
}
