use serde::{ser::Serializer, Serialize};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Tauri(#[from] tauri::Error),
    #[error("Menu error: {0}")]
    Menu(String),
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("Mobile plugin error: {0}")]
    Mobile(String),
    #[error("This feature is only supported on macOS")]
    UnsupportedPlatform,
    #[error("Menubar instance not found")]
    InstanceNotFound,
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}
