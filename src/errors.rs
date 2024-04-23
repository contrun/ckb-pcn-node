use ractor::{MessagingErr, SpawnErr};
use tentacle::{error::SendErrorKind, secio::PeerId};
use thiserror::Error;

use crate::ckb::{
    channel::{ChannelActorMessage, ProcessingChannelError},
    types::Hash256,
    NetworkActorMessage,
};

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    #[error("Peer not found error: {0:?}")]
    PeerNotFound(PeerId),
    #[error("Channel not found error: {0:?}")]
    ChannelNotFound(Hash256),
    #[error("Failed to send tentacle message: {0}")]
    TentacleSend(#[from] SendErrorKind),
    #[error("Failed to spawn actor: {0}")]
    SpawnErr(#[from] SpawnErr),
    #[error("Failed to send channel actor message: {0}")]
    ChannelMessagingErr(#[from] MessagingErr<ChannelActorMessage>),
    #[error("Failed to send network actor message: {0}")]
    NetworkMessagingErr(#[from] MessagingErr<NetworkActorMessage>),
    #[error("Failed to processing channel: {0}")]
    ChannelError(#[from] ProcessingChannelError),
}

pub type Result<T> = std::result::Result<T, Error>;
