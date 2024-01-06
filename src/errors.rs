//! Module holding various error wrappers

use std::io;

use std::sync::mpsc as channel_mpsc;

#[cfg(feature = "stream")]
use futures::sync::mpsc as stream_mpsc;

use std::error::Error;
use crate::r#async::SendData;

#[derive(Debug, thiserror::Error)]
pub enum ChaseError {
    #[error(transparent)] IoError(#[from] io::Error),
    #[error(transparent)] ChannelSendError(#[from] channel_mpsc::SendError<SendData>),
    #[cfg(feature = "stream")] #[error(transparent)] StreamSendError(#[from] stream_mpsc::SendError<SendData>),
    #[error(transparent)] Custom(Box<dyn Error + Send + Sync>),
}
