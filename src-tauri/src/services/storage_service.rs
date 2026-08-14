use crate::error::{AppError, AppResult};
use crate::models::storage::{
    SelectStorageTargetRequest, SetStorageAllocationRequest, StorageAllocation, StorageDevice,
    StorageTarget,
};
use crate::repositories::storage::RedbStorageRepository;
use crate::system::filesystem::detect_storage_devices;
use crate::utils::time::current_time_millis;

pub struct StorageService {
    repository: RedbStorageRepository,
}

impl StorageService {
    pub fn new(repository: RedbStorageRepository) -> Self {
        Self { repository }
    }

    pub fn available_devices(&self) -> Vec<StorageDevice> {
        detect_storage_devices()
    }

    pub fn allocations(&self) -> AppResult<Vec<StorageAllocation>> {
        let mut allocations = self.repository.list()?;
        sort_allocations(&mut allocations);
        Ok(allocations)
    }

    pub fn set_allocation(
        &self,
        request: SetStorageAllocationRequest,
    ) -> AppResult<Vec<StorageAllocation>> {
        if request.quota_bytes == 0 {
            return Err(AppError::validation(
                "storage quota must be greater than zero",
            ));
        }
        if request.priority == 0 {
            return Err(AppError::validation(
                "storage priority must be one or greater",
            ));
        }

        let device = detect_storage_devices()
            .into_iter()
            .find(|device| device.volume_id == request.volume_id)
            .ok_or_else(|| {
                AppError::storage_unavailable("the selected storage volume is not available")
            })?;
        if device.is_read_only {
            return Err(AppError::storage_unavailable(
                "the selected storage volume is read-only",
            ));
        }

        let mut allocations = self.allocations()?;
        let existing_used_bytes = allocations
            .iter()
            .find(|allocation| allocation.volume_id == request.volume_id)
            .map_or(0, |allocation| allocation.vault_used_bytes);
        if request.quota_bytes < existing_used_bytes {
            return Err(AppError::validation(format!(
                "quota cannot be lower than the vault's {existing_used_bytes} used bytes"
            )));
        }

        let allocatable_bytes = device.available_bytes.saturating_add(existing_used_bytes);
        if request.quota_bytes > allocatable_bytes {
            return Err(AppError::validation(format!(
                "requested quota exceeds the volume's {allocatable_bytes} allocatable bytes"
            )));
        }

        allocations.retain(|allocation| allocation.volume_id != request.volume_id);
        let insert_at = usize::try_from(request.priority - 1)
            .unwrap_or(usize::MAX)
            .min(allocations.len());
        allocations.insert(
            insert_at,
            StorageAllocation {
                schema_version: 1,
                priority: request.priority,
                volume_id: device.volume_id,
                physical_disk_id: device.physical_disk_id,
                hardware_model: device.hardware_model,
                volume_label: device.volume_label,
                mount_point: device.mount_point,
                quota_bytes: request.quota_bytes,
                vault_used_bytes: existing_used_bytes,
                updated_at_unix_ms: current_time_millis()?,
            },
        );
        normalize_priorities(&mut allocations);
        self.repository.save_all(&allocations)?;
        Ok(allocations)
    }

    pub fn remove_allocation(&self, volume_id: &str) -> AppResult<Vec<StorageAllocation>> {
        let mut allocations = self.allocations()?;
        allocations.retain(|allocation| allocation.volume_id != volume_id);
        normalize_priorities(&mut allocations);
        self.repository.remove_and_save(volume_id, &allocations)?;
        Ok(allocations)
    }

    pub fn clear_allocations(&self) -> AppResult<()> {
        self.repository.clear()
    }

    pub fn select_target(&self, request: SelectStorageTargetRequest) -> AppResult<StorageTarget> {
        choose_storage_target(
            &self.allocations()?,
            &detect_storage_devices(),
            request.required_bytes,
        )
    }
}

pub fn choose_storage_target(
    allocations: &[StorageAllocation],
    devices: &[StorageDevice],
    required_bytes: u64,
) -> AppResult<StorageTarget> {
    if required_bytes == 0 {
        return Err(AppError::validation(
            "required storage must be greater than zero",
        ));
    }
    if allocations.is_empty() {
        return Err(AppError::storage_unavailable(
            "no storage volumes have been configured",
        ));
    }

    let mut ordered = allocations.to_vec();
    sort_allocations(&mut ordered);
    for allocation in ordered {
        let Some(device) = devices
            .iter()
            .find(|device| device.volume_id == allocation.volume_id)
        else {
            continue;
        };
        if device.is_read_only {
            continue;
        }

        let quota_remaining = allocation
            .quota_bytes
            .saturating_sub(allocation.vault_used_bytes);
        let writable_bytes = quota_remaining.min(device.available_bytes);
        if writable_bytes < required_bytes {
            continue;
        }

        return Ok(StorageTarget {
            volume_id: allocation.volume_id,
            physical_disk_id: allocation.physical_disk_id,
            hardware_model: allocation.hardware_model,
            mount_point: device.mount_point.clone(),
            priority: allocation.priority,
            quota_bytes: allocation.quota_bytes,
            vault_used_bytes: allocation.vault_used_bytes,
            device_available_bytes: device.available_bytes,
            writable_bytes,
        });
    }

    Err(AppError::storage_unavailable(
        "no configured storage volume has enough available quota",
    ))
}

fn sort_allocations(allocations: &mut [StorageAllocation]) {
    allocations.sort_by(|left, right| {
        left.priority
            .cmp(&right.priority)
            .then_with(|| left.volume_id.cmp(&right.volume_id))
    });
}

fn normalize_priorities(allocations: &mut [StorageAllocation]) {
    for (index, allocation) in allocations.iter_mut().enumerate() {
        allocation.priority = u32::try_from(index + 1).unwrap_or(u32::MAX);
    }
}
