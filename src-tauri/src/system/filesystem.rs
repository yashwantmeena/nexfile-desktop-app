use sysinfo::{DiskKind, Disks};

use crate::models::storage::{StorageDevice, StorageKind};

/// Detects all mounted storage volumes currently reported by the operating
/// system. Results are sorted by mount point to keep the response stable.
pub fn detect_storage_devices() -> Vec<StorageDevice> {
    let disks = Disks::new_with_refreshed_list();
    let mut devices = disks
        .list()
        .iter()
        .map(|disk| {
            let total_bytes = disk.total_space();
            let available_bytes = disk.available_space();
            let mount_point = disk.mount_point().to_string_lossy().into_owned();
            let reported_name = disk.name().to_string_lossy();
            let name = if reported_name.is_empty() {
                mount_point.clone()
            } else {
                reported_name.into_owned()
            };

            StorageDevice {
                volume_id: mount_point.clone(),
                name,
                mount_point,
                file_system: disk.file_system().to_string_lossy().into_owned(),
                physical_disk_id: None,
                hardware_model: None,
                volume_label: None,
                kind: map_disk_kind(disk.kind()),
                total_bytes,
                available_bytes,
                used_bytes: total_bytes.saturating_sub(available_bytes),
                is_removable: disk.is_removable(),
                is_read_only: disk.is_read_only(),
            }
        })
        .collect::<Vec<_>>();

    #[cfg(target_os = "windows")]
    enrich_windows_storage_metadata(&mut devices);

    devices.sort_by_cached_key(|device| device.mount_point.to_lowercase());
    devices
}

pub fn map_disk_kind(kind: DiskKind) -> StorageKind {
    match kind {
        DiskKind::SSD => StorageKind::Ssd,
        DiskKind::HDD => StorageKind::Hdd,
        DiskKind::Unknown(_) => StorageKind::Unknown,
    }
}

#[cfg(target_os = "windows")]
fn enrich_windows_storage_metadata(devices: &mut [StorageDevice]) {
    use std::ffi::OsStr;
    use std::mem::size_of;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, GetVolumeInformationW, GetVolumeNameForVolumeMountPointW,
        FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE,
        IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS, OPEN_EXISTING,
    };
    use windows::Win32::System::Ioctl::{
        PropertyStandardQuery, StorageDeviceProperty, IOCTL_STORAGE_QUERY_PROPERTY,
        STORAGE_DEVICE_DESCRIPTOR, STORAGE_PROPERTY_QUERY, VOLUME_DISK_EXTENTS,
    };
    use windows::Win32::System::IO::DeviceIoControl;

    struct OwnedHandle(HANDLE);

    impl Drop for OwnedHandle {
        fn drop(&mut self) {
            let _ = unsafe { CloseHandle(self.0) };
        }
    }

    fn wide(value: &str) -> Vec<u16> {
        OsStr::new(value).encode_wide().chain(Some(0)).collect()
    }

    fn string_from_wide(buffer: &[u16]) -> Option<String> {
        let length = buffer
            .iter()
            .position(|character| *character == 0)
            .unwrap_or(buffer.len());
        let value = String::from_utf16_lossy(&buffer[..length]);
        (!value.trim().is_empty()).then(|| value.trim().to_owned())
    }

    fn open_device(path: &str) -> Option<OwnedHandle> {
        let path = wide(path);
        let handle = unsafe {
            CreateFileW(
                PCWSTR(path.as_ptr()),
                0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_FLAGS_AND_ATTRIBUTES::default(),
                None,
            )
        }
        .ok()?;

        (!handle.is_invalid()).then_some(OwnedHandle(handle))
    }

    fn volume_id_and_label(mount_point: &str) -> (Option<String>, Option<String>) {
        let mount_point = wide(mount_point);
        let mut volume_id = [0u16; 64];
        let mut volume_label = [0u16; 261];

        let id = unsafe {
            GetVolumeNameForVolumeMountPointW(PCWSTR(mount_point.as_ptr()), &mut volume_id)
        }
        .ok()
        .and_then(|_| string_from_wide(&volume_id));
        let label = unsafe {
            GetVolumeInformationW(
                PCWSTR(mount_point.as_ptr()),
                Some(&mut volume_label),
                None,
                None,
                None,
                None,
            )
        }
        .ok()
        .and_then(|_| string_from_wide(&volume_label));

        (id, label)
    }

    fn physical_disk_number(volume_path: &str) -> Option<u32> {
        let handle = open_device(volume_path)?;
        let mut output = vec![0u64; 128];
        let mut bytes_returned = 0u32;

        unsafe {
            DeviceIoControl(
                handle.0,
                IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS,
                None,
                0,
                Some(output.as_mut_ptr().cast()),
                (output.len() * size_of::<u64>()) as u32,
                Some(&mut bytes_returned),
                None,
            )
        }
        .ok()?;

        let extents = unsafe { &*(output.as_ptr().cast::<VOLUME_DISK_EXTENTS>()) };
        (extents.NumberOfDiskExtents > 0).then_some(extents.Extents[0].DiskNumber)
    }

    fn descriptor_string(buffer: &[u8], offset: u32) -> Option<String> {
        let start = usize::try_from(offset).ok()?;
        if start == 0 || start >= buffer.len() {
            return None;
        }

        let bytes = &buffer[start..];
        let end = bytes
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(bytes.len());
        let value = String::from_utf8_lossy(&bytes[..end]);
        (!value.trim().is_empty()).then(|| value.trim().to_owned())
    }

    fn physical_disk_model(disk_number: u32) -> Option<String> {
        let handle = open_device(&format!(r"\\.\PhysicalDrive{disk_number}"))?;
        let query = STORAGE_PROPERTY_QUERY {
            PropertyId: StorageDeviceProperty,
            QueryType: PropertyStandardQuery,
            AdditionalParameters: [0],
        };
        let mut output = vec![0u64; 512];
        let mut bytes_returned = 0u32;

        unsafe {
            DeviceIoControl(
                handle.0,
                IOCTL_STORAGE_QUERY_PROPERTY,
                Some((&query as *const STORAGE_PROPERTY_QUERY).cast()),
                size_of::<STORAGE_PROPERTY_QUERY>() as u32,
                Some(output.as_mut_ptr().cast()),
                (output.len() * size_of::<u64>()) as u32,
                Some(&mut bytes_returned),
                None,
            )
        }
        .ok()?;

        let bytes = unsafe {
            std::slice::from_raw_parts(output.as_ptr().cast::<u8>(), bytes_returned as usize)
        };
        if bytes.len() < size_of::<STORAGE_DEVICE_DESCRIPTOR>() {
            return None;
        }
        let descriptor = unsafe { &*(bytes.as_ptr().cast::<STORAGE_DEVICE_DESCRIPTOR>()) };
        let vendor = descriptor_string(bytes, descriptor.VendorIdOffset);
        let product = descriptor_string(bytes, descriptor.ProductIdOffset);

        match (vendor, product) {
            (Some(vendor), Some(product)) if !product.starts_with(&vendor) => {
                Some(format!("{vendor} {product}"))
            }
            (_, Some(product)) => Some(product),
            (Some(vendor), None) => Some(vendor),
            (None, None) => None,
        }
    }

    for device in devices {
        let drive_letter = device
            .mount_point
            .trim_end_matches(['\\', '/'])
            .to_uppercase();
        let (volume_id, volume_label) = volume_id_and_label(&device.mount_point);

        device.volume_id = volume_id.unwrap_or_else(|| device.mount_point.clone());
        device.volume_label = volume_label;
        device.name = device
            .volume_label
            .clone()
            .unwrap_or_else(|| "Local Disk".to_owned());

        let Some(disk_number) = physical_disk_number(&format!(r"\\.\{drive_letter}")) else {
            continue;
        };
        device.physical_disk_id = Some(format!("PHYSICALDRIVE{disk_number}"));
        device.hardware_model = physical_disk_model(disk_number);
    }
}
