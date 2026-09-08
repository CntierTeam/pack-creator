use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("yaml: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("zip: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("parse: {0}")]
    ParseInt(#[from] std::num::ParseIntError),
    #[error("build.pk: {0}")]
    BuildPk(String),
    #[error("project: {0}")]
    Project(String),
    #[error("pack: {0}")]
    Pack(String),
    #[error("{0}")]
    Msg(String),
}

pub type Result<T> = std::result::Result<T, Error>;
