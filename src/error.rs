use thiserror::Error;

#[derive(Error, Debug)]
pub enum EncodeError {
    #[error("bad/unrecognized deserialize format")]
    Serialize(#[from] serde_cbor::Error),
}

#[derive(Error, Debug)]
pub enum DecodeError {
    #[error("unsupported report format")]
    Unsupported,
    #[error("bad/unrecognized format")]
    BadFormat,
    #[error("bad/unrecognized deserialize format")]
    Deserialize(#[from] serde_cbor::Error),
    #[cfg(feature = "compress")]
    #[error("bad lzma compressed format")]
    BadLzma,
}