use thiserror::Error;

#[derive(Debug, Error)]
pub enum XmlError {
    #[error("xml reader error: {0}")]
    Reader(#[from] quick_xml::Error),

    #[error("xml event error at byte {pos}: {msg}")]
    Event { pos: usize, msg: String },

    #[error("not a deluge kit file (expected <kit> root, found `{0}`)")]
    NotAKit(String),

    #[error("legacy v2.x kit format (multi-root XML) is not supported")]
    LegacyV2,

    #[error("invalid numeric value `{0}`")]
    InvalidNumber(String),

    #[error("utf8 error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
}

pub type XmlResult<T> = Result<T, XmlError>;
