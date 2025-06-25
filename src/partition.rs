use crate::error::{Result, UpgradeError};
use embedded_storage::nor_flash::NorFlash;
use esp_partition_table::{
    AppPartitionType, DataPartitionType, PartitionEntry, PartitionTable, PartitionType,
};
use log::{debug, info};

pub fn find_ota_partition<S: NorFlash>(storage: &mut S) -> Result<PartitionEntry> {
    let table = PartitionTable::default();

    for partition in table.iter_nor_flash(storage, false).flatten() {
        if let PartitionType::Data(DataPartitionType::Ota) = partition.type_ {
            return Ok(partition);
        }
    }

    Err(UpgradeError::PartitionNotFound)
}

pub fn find_running_partition<S: NorFlash>(storage: &mut S, seq: u32) -> Result<PartitionEntry> {
    let partition_number = ((seq + 1) % 2) as u8;
    find_partition_by_type(
        storage,
        PartitionType::App(AppPartitionType::Ota(partition_number)),
    )
}

pub fn find_inactive_partition<S: NorFlash>(storage: &mut S, seq: u32) -> Result<PartitionEntry> {
    let partition_number = (seq % 2) as u8;
    find_partition_by_type(
        storage,
        PartitionType::App(AppPartitionType::Ota(partition_number)),
    )
}

/// Find partition entry by type
pub fn find_partition_by_type<S: NorFlash>(
    storage: &mut S,
    typ: PartitionType,
) -> Result<PartitionEntry> {
    let table = PartitionTable::default();

    for entry in table.iter_nor_flash(storage, false) {
        let entry = entry?;
        if entry.type_ == typ {
            return Ok(entry);
        }
    }

    Err(UpgradeError::PartitionNotFound)
}

/// Find partition entry by name
pub fn find_partition_by_name<S: NorFlash>(storage: &mut S, name: &str) -> Result<PartitionEntry> {
    let table = PartitionTable::default();

    for entry in table.iter_nor_flash(storage, false) {
        let ok_entry = entry?;
        if ok_entry.name() == name {
            return Ok(ok_entry);
        }
    }
    Err(UpgradeError::PartitionNotFound)
}
