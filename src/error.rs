use crate::alloc::string::ToString;
use alloc::str::Utf8Error;
use alloc::string::String;
use defmt::error;
use esp_partition_table::NorFlashOpError;
use semver::Error as SemverError;
use thiserror::Error;

pub type Result<T> = core::result::Result<T, UpgradeError>;

#[derive(Error, Debug)]
pub enum UpgradeError {
    #[error("Download already in progress")]
    DLInProgress,
    #[error("Already rebooting into new firmware")]
    BootingIntoNewFW,
    #[error("Invalid OTA state")]
    InvalidState,
    #[error("Invalid CRC")]
    InvalidCrc,
    #[error("Storage error")]
    StorageError,
    #[error("Flash error")]
    FlashError,
    #[error("Version error")]
    VersionError(String),
    #[error("Partition not found")]
    PartitionNotFound,
    #[error("Partition found twice")]
    PartitionFoundTwice,
    #[error("OTA partition corrupted")]
    OtaPartitionCorrupted,
    #[error("network error")]
    RequestError,
    #[error("UTF8error")]
    UTF8Error(#[from] Utf8Error),
    #[error("serde Error")]
    SerdeError(#[from] serde_json_core::de::Error),
    #[error("Out of space")]
    OutOfSpace,
}

impl From<reqwless::Error> for UpgradeError {
    fn from(error: reqwless::Error) -> Self {
        error!("network error: {}", error);
        Self::RequestError
    }
}
impl From<()> for UpgradeError {
    fn from(_: ()) -> Self {
        error!("unit error");
        Self::RequestError
    }
}
impl<S: embedded_storage::nor_flash::ReadNorFlash> From<NorFlashOpError<S>> for UpgradeError {
    fn from(error: NorFlashOpError<S>) -> Self {
        match error {
            NorFlashOpError::PartitionError(_internal_error) => {
                //error!("error message: {:?}", internal_error);
                Self::FlashError
            }
            NorFlashOpError::StorageError(_internal_error) => {
                //error!("error message: {:?}", internal_error);
                Self::StorageError
            }
        }
    }
}

impl From<SemverError> for UpgradeError {
    fn from(error: SemverError) -> Self {
        let error_message = error.to_string();
        error!("error message: {}", error_message);
        UpgradeError::VersionError(error_message)
    }
}
/*
impl fmt::Display for OtaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OtaError::HttpError(e) => write!(f, "HTTP error: {}", e),
            OtaError::VersionError(e) => write!(f, "Version error: {}", e),
            OtaError::PartitionError(e) => write!(f, "Partition error: {}", e),
            OtaError::IoError(e) => write!(f, "IO error: {}", e),
            OtaError::InvalidVersion => write!(f, "Invalid version format"),
            OtaError::InvalidPartition => write!(f, "Invalid partition"),
            OtaError::InsufficientSpace => write!(f, "Insufficient space for update"),
        }
    }
}
*/
