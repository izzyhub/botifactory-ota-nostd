use crate::error::{Result, UpgradeError};
use crate::partition::find_ota_partition;
use crate::seq_crc::esp_crc;
use alloc::fmt::Display;
use core::fmt::Formatter;
use defmt::Format;
use embedded_storage::nor_flash::NorFlash;

const SECTOR_SIZE: usize = 0x1000;

/// These aren't really arbitrary/crate invented states.
/// They come from the espressive bootloader
/// [documented here](https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/ota.html)
#[derive(Debug, Format, Copy, Clone, Eq, PartialEq)]
pub enum AppOTAState {
    /// Monitor the first boot.
    /// In bootloader this state is changed to ESP_OTA_IMG_PENDING_VERIFY.
    New,
    /// First boot for this app was.
    /// If while the second boot this state is then it will be changed to ABORTED.
    PendingVerify,
    /// App was confirmed as workable.
    /// App can boot and work without limits.
    Valid,
    /// App was confirmed as non-workable.
    /// This app will not selected to boot at all.
    Invalid,
    /// App could not confirm the workable or non-workable.
    /// In bootloader PendingVerify state will be changed to
    /// Aborted. This app will not be selected to boot at all
    Aborted,
    /// Undefined. App can boot and work without limits.
    Undefined,
}

impl TryFrom<u32> for AppOTAState {
    type Error = UpgradeError;
    fn try_from(value: u32) -> Result<Self> {
        match value {
            0 => Ok(Self::New),
            1 => Ok(Self::PendingVerify),
            2 => Ok(Self::Valid),
            3 => Ok(Self::Invalid),
            4 => Ok(Self::Aborted),
            u32::MAX => Ok(Self::Undefined),
            _ => Err(UpgradeError::InvalidState),
        }
    }
}

impl From<AppOTAState> for u32 {
    fn from(value: AppOTAState) -> Self {
        match value {
            AppOTAState::New => 0,
            AppOTAState::PendingVerify => 1,
            AppOTAState::Valid => 2,
            AppOTAState::Invalid => 3,
            AppOTAState::Aborted => 4,
            AppOTAState::Undefined => u32::MAX,
        }
    }
}

/// Also not arbitrary. Based on what the esp32 bootloader uses.
/// [documented here](https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/ota.html)
#[derive(Debug, Clone, Copy)]
pub struct UpgradeInfo {
    pub seq: u32,
    pub label: [u8; 20],
    pub state: AppOTAState,
    /// CRC32 of ota_seq field only
    pub seq_crc: u32,
}

impl UpgradeInfo {
    pub fn from_flash<S: NorFlash>(storage: &mut S) -> Result<Self> {
        let ota_partition = find_ota_partition(storage)?;
        let mut buffer = [0; 32];
        storage
            .read(ota_partition.offset, &mut buffer)
            .map_err(|_| UpgradeError::StorageError)?;

        if let Ok(upgrade_info) = UpgradeInfo::try_from(buffer) {
            return Ok(upgrade_info);
        }

        storage
            .read(ota_partition.offset + SECTOR_SIZE as u32, &mut buffer)
            .map_err(|_| UpgradeError::StorageError)?;

        UpgradeInfo::try_from(buffer).map_err(|_| UpgradeError::StorageError)
    }

    pub fn new(seq: u32, label: [u8; 20]) -> Self {
        let state = AppOTAState::New;
        let seq_crc = esp_crc(&seq.to_le_bytes());
        Self {
            seq,
            label,
            state,
            seq_crc,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.state == AppOTAState::Valid || self.state == AppOTAState::Undefined
    }

    pub fn save_to_flash<S: NorFlash>(&self, storage: &mut S) -> Result<()> {
        let ota_partition = find_ota_partition(storage)?;
        let buffer: [u8; 32] = (*self).into();

        // Write sector 1
        storage
            .erase(
                ota_partition.offset,
                ota_partition.offset + SECTOR_SIZE as u32,
            )
            .map_err(|_| UpgradeError::StorageError)?;
        storage
            .write(ota_partition.offset, &buffer)
            .map_err(|_| UpgradeError::StorageError)?;

        // Write sector 2
        storage
            .erase(
                ota_partition.offset + SECTOR_SIZE as u32,
                ota_partition.offset + 2 * SECTOR_SIZE as u32,
            )
            .map_err(|_| UpgradeError::StorageError)?;
        storage
            .write(ota_partition.offset + SECTOR_SIZE as u32, &buffer)
            .map_err(|_| UpgradeError::StorageError)?;
        Ok(())
    }
}

impl Display for UpgradeInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "UpgradeInfo {{ seq: {}, label: {:x?}, state: {:?}, seq_crc: 0x{:08x} }}",
            self.seq, self.label, self.state, self.seq_crc
        )
    }
}

impl TryFrom<[u8; 32]> for UpgradeInfo {
    type Error = UpgradeError;
    fn try_from(value: [u8; 32]) -> Result<Self> {
        let seq = u32::from_le_bytes(value[0..4].try_into().unwrap());
        let label = value[4..24].try_into().unwrap();
        let state = AppOTAState::try_from(u32::from_le_bytes(value[24..28].try_into().unwrap()))?;
        let seq_crc = u32::from_le_bytes(value[28..32].try_into().unwrap());

        if seq_crc == esp_crc(&seq.to_le_bytes()) {
            Ok(Self {
                seq,
                label,
                state,
                seq_crc,
            })
        } else {
            Err(UpgradeError::InvalidCrc)
        }
    }
}

impl From<UpgradeInfo> for [u8; 32] {
    fn from(value: UpgradeInfo) -> Self {
        let mut ret = [0; 32];
        ret[0..4].copy_from_slice(&value.seq.to_le_bytes());
        ret[4..24].copy_from_slice(&value.label);
        ret[24..28].copy_from_slice(&u32::to_le_bytes(value.state.into()));
        ret[28..32].copy_from_slice(&value.seq_crc.to_le_bytes());
        ret
    }
}
