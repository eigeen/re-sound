pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),

    #[error("Unknown HIRC entry type at offset {0}: {0}")]
    UnknownHircEntryType(u64, u8),
    #[error("Unknown SoundType at offset {0}: {0}")]
    UnknownSoundType(u64, u8),
    #[error("Unknown EventActionScope at offset {0}: {0}")]
    UnknownEventActionScope(u64, u8),
    #[error("Incorrect data size for {0}: expected {1}, got {2}.")]
    BadDataSize(String, u32, u32),
}
