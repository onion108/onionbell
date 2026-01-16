use std::env::VarError;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    VarError(#[from] VarError),

    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error(transparent)]
    TomlDeserializationError(#[from] toml::de::Error),

    #[error(transparent)]
    RodioStreamError(#[from] rodio::StreamError),

    #[error(transparent)]
    RodioDecoderError(#[from] rodio::decoder::DecoderError),

    #[error(transparent)]
    JsonError(#[from] serde_json::Error)
}
