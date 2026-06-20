use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Serialize)]
#[serde(tag = "kind", content = "msg")]
pub enum AppError {
    #[error("no SD root selected")]
    NoSdRoot,
    #[error("{0}")]
    Other(String),
}
