use std::ffi::OsStr;
use std::mem::size_of;
use std::os::windows::ffi::OsStrExt;

use sysinfo::Disks;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, GetVolumeInformationW, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_READ,
    FILE_SHARE_WRITE, IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS, OPEN_EXISTING,
};
use windows::Win32::System::Ioctl::{
    PropertyStandardQuery, StorageDeviceProperty, IOCTL_STORAGE_QUERY_PROPERTY,
    STORAGE_DEVICE_DESCRIPTOR, STORAGE_PROPERTY_QUERY, VOLUME_DISK_EXTENTS,
};
use windows::Win32::System::IO::DeviceIoControl;

use crate::models::storage::DriveInfo;

/// Returns every mounted drive currently connected to this Windows machine.
pub fn get_drives() -> Vec<DriveInfo> {
    let disks = Disks::new_with_refreshed_list();
    let system_drive = system_drive();
    let mut drives = disks
        .list()
        .iter()
        .map(|disk| {
            let mount_point = disk.mount_point().to_string_lossy().into_owned();
            let drive_letter = mount_point.trim_end_matches(['\\', '/']).to_owned();
            let volume_label = volume_label(&mount_point);
            let reported_name = disk.name().to_string_lossy();
            let drive_name = physical_disk_number(&format!(r"\\.\{drive_letter}"))
                .and_then(physical_disk_model)
                .or_else(|| volume_label.clone())
                .or_else(|| (!reported_name.trim().is_empty()).then(|| reported_name.into_owned()))
                .unwrap_or_else(|| "Local Disk".to_owned());
            let is_system = system_drive
                .as_deref()
                .is_some_and(|system_drive| drive_letter.eq_ignore_ascii_case(system_drive));
            let partition_name = match volume_label.as_deref() {
                Some(label) => format!("{label} ({drive_letter})"),
                None => drive_letter,
            };
            let total_bytes = disk.total_space();

            DriveInfo {
                drive_id: String::new(),
                drive_name,
                partition_name,
                file_system: disk.file_system().to_string_lossy().into_owned(),
                total_bytes,
                system_used_bytes: total_bytes.saturating_sub(disk.available_space()),
                is_system,
                mount_point: disk.mount_point().to_path_buf(),
            }
        })
        .collect::<Vec<_>>();

    drives.sort_by(|left, right| left.drive_id.cmp(&right.drive_id));
    drives
}

fn system_drive() -> Option<String> {
    let system_root = std::env::var("SystemRoot")
        .or_else(|_| std::env::var("windir"))
        .ok()?;
    let drive = system_root.get(..2)?;

    drive.ends_with(':').then(|| drive.to_owned())
}

struct OwnedHandle(HANDLE);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        // SAFETY: `OwnedHandle` is only created from a successful `CreateFileW`
        // call and owns the handle until it is dropped.
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
    // SAFETY: `path` is a null-terminated UTF-16 buffer that remains alive for
    // the duration of the call. The returned handle is wrapped immediately.
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

fn volume_label(mount_point: &str) -> Option<String> {
    let mount_point = wide(mount_point);
    let mut volume_label = [0u16; 261];

    // SAFETY: the mount point is null terminated and the output buffer is valid
    // for the duration of the call.
    unsafe {
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
    .and_then(|_| string_from_wide(&volume_label))
}

fn physical_disk_number(volume_path: &str) -> Option<u32> {
    let handle = open_device(volume_path)?;
    // A u64-backed buffer provides sufficient alignment for the Windows
    // `VOLUME_DISK_EXTENTS` structure and room for multi-extent volumes.
    let mut output = vec![0u64; 128];
    let mut bytes_returned = 0u32;

    // SAFETY: the handle is valid and both input/output pointers and byte sizes
    // match the buffers passed to `DeviceIoControl`.
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

    if (bytes_returned as usize) < size_of::<VOLUME_DISK_EXTENTS>() {
        return None;
    }
    // SAFETY: the aligned output buffer contains at least one complete
    // `VOLUME_DISK_EXTENTS` value after the successful control call.
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
    // A u64-backed buffer provides the alignment required when reading the
    // descriptor header from the returned bytes.
    let mut output = vec![0u64; 512];
    let mut bytes_returned = 0u32;

    // SAFETY: the query and output buffers remain valid for the call and their
    // byte sizes are passed exactly to `DeviceIoControl`.
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

    let returned_length = (bytes_returned as usize).min(output.len() * size_of::<u64>());
    if returned_length < size_of::<STORAGE_DEVICE_DESCRIPTOR>() {
        return None;
    }
    // SAFETY: `output` is aligned for the descriptor and `returned_length` was
    // checked to contain the complete header.
    let bytes =
        unsafe { std::slice::from_raw_parts(output.as_ptr().cast::<u8>(), returned_length) };
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
