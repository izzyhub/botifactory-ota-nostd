use crate::error::{Result, UpgradeError};
use crate::partition::find_inactive_partition;
use crate::upgrade_data::{AppOTAState, UpgradeInfo};
use core::sync::atomic::Ordering;
use defmt::{debug, error, info, warn};
use embedded_io_async::Read;
use embedded_storage::nor_flash::NorFlash;
use portable_atomic::AtomicBool;

/// Size of a flash sector
const SECTOR_SIZE: usize = 4096;

static IS_SAVING: AtomicBool = AtomicBool::new(false);

pub async fn save_new_fw<S: NorFlash, R: Read>(
    storage: &mut S,
    mut binary_reader: R,
) -> Result<()> {
    if IS_SAVING.swap(true, Ordering::SeqCst) {
        info!("download already in progress");
        return Err(UpgradeError::DLInProgress);
    }

    let upgrade_info = UpgradeInfo::from_flash(storage)?;

    if !upgrade_info.is_valid() {
        info!("booting into new fw.");
        return Err(UpgradeError::BootingIntoNewFW);
    }
    let inactive_partition = find_inactive_partition(storage, upgrade_info.seq)?;

    storage
        .erase(
            inactive_partition.offset,
            inactive_partition.offset + inactive_partition.size as u32,
        )
        .map_err(|_| UpgradeError::StorageError)?;

    upgrade_info.save_to_flash(storage)?;

    let mut write_buffer = [0; SECTOR_SIZE];
    let mut saved_len = 0;
    let mut done_reading = false;

    while !done_reading {
        let mut amount_read = 0;
        while amount_read < SECTOR_SIZE {
            let size = binary_reader
                .read(&mut write_buffer[amount_read..])
                .await
                .map_err(|_| UpgradeError::StorageError)?;
            if size == 0 {
                done_reading = true;
            }
            amount_read += size;
        }
        if amount_read + saved_len > inactive_partition.size {
            return Err(UpgradeError::OutOfSpace);
        }

        storage
            .write(
                inactive_partition.offset + amount_read as u32,
                &write_buffer[0..amount_read],
            )
            .map_err(|_| UpgradeError::StorageError)?;
        saved_len += amount_read;
    }

    let new_upgrade_info = UpgradeInfo::new(upgrade_info.seq + 1, [0xFF; 20]);
    new_upgrade_info.save_to_flash(storage)
}

pub fn accept_fw<S: NorFlash>(storage: &mut S) -> Result<()> {
    let mut upgrade_info = UpgradeInfo::from_flash(storage)?;
    let mut should_write = true;

    match upgrade_info.state {
        AppOTAState::PendingVerify => {
            info!("Accepted upgrade.");
        }
        AppOTAState::New | AppOTAState::Undefined => {
            warn!("Accepted upgrade from state {:?}.", upgrade_info.state);
        }
        AppOTAState::Invalid | AppOTAState::Aborted => {
            warn!("Rolled back but not marked by bootloader. Saving manually");
            upgrade_info.seq -= 1;
        }
        AppOTAState::Valid => {
            should_write = false;
            debug!("state already valid");
        }
    }
    if should_write {
        upgrade_info.state = AppOTAState::Valid;
        upgrade_info.save_to_flash(storage)?
    }
    Ok(())
}

pub fn reject_fw<S: NorFlash>(storage: &mut S) -> Result<()> {
    let mut upgrade_info = UpgradeInfo::from_flash(storage)?;
    let mut should_write = false;

    match upgrade_info.state {
        AppOTAState::PendingVerify => {
            info!("rejecting pending upgrade")
        }
        AppOTAState::New | AppOTAState::Undefined => {
            warn!("rejected upgrade from {:?} state", upgrade_info.state);
            should_write = true;
        }
        AppOTAState::Valid => {
            error!("tried to rejct upgrade that has already been accepted, ignoring request.")
        }
        AppOTAState::Invalid => {
            error!("tried to rejct upgrade that has already been rejected, ignoring request.")
        }
        AppOTAState::Aborted => {
            error!("tried to reject upgrade from aborted state, ignoring request.")
        }
    }

    if should_write {
        upgrade_info.state = AppOTAState::Invalid;
        upgrade_info.save_to_flash(storage)?;
    }
    Ok(())
}
