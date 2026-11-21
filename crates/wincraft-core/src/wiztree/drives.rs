use super::models::DriveInfo;

#[cfg(windows)]
pub fn list_fixed_drives() -> Vec<DriveInfo> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{GetDriveTypeW, GetLogicalDrives};

    const DRIVE_FIXED: u32 = 3;

    let mut drives = Vec::new();
    let mask = unsafe { GetLogicalDrives() };

    for i in 0..26 {
        if (mask & (1 << i)) != 0 {
            let letter = (b'A' + i) as char;
            let root_str = format!("{}:\\", letter);
            let wide: Vec<u16> = OsStr::new(&root_str).encode_wide().chain(Some(0)).collect();

            let drive_type = unsafe { GetDriveTypeW(wide.as_ptr()) };
            let is_fixed = drive_type == DRIVE_FIXED;

            if is_fixed {
                drives.push(DriveInfo {
                    root: root_str,
                    is_fixed: true,
                });
            }
        }
    }

    if drives.is_empty() {
        drives.push(DriveInfo {
            root: "C:\\".to_string(),
            is_fixed: true,
        });
    }

    drives
}

#[cfg(windows)]
pub fn is_elevated() -> bool {
    use std::mem::MaybeUninit;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let mut elevation = MaybeUninit::<TOKEN_ELEVATION>::uninit();
        let mut return_length = 0u32;
        let success = GetTokenInformation(
            token,
            TokenElevation,
            elevation.as_mut_ptr() as *mut _,
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut return_length,
        );
        CloseHandle(token);
        if success != 0 {
            elevation.assume_init().TokenIsElevated != 0
        } else {
            false
        }
    }
}

#[cfg(windows)]
pub fn can_read_mft(drive: &str) -> bool {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };

    const GENERIC_READ: u32 = 0x80000000;

    let letter = drive.chars().next().unwrap_or('C').to_ascii_uppercase();
    let dev_path = format!("\\\\.\\{letter}:");
    let wide: Vec<u16> = OsStr::new(&dev_path).encode_wide().chain(Some(0)).collect();

    unsafe {
        let handle = CreateFileW(
            wide.as_ptr(),
            GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        );

        if handle != INVALID_HANDLE_VALUE && !handle.is_null() {
            CloseHandle(handle);
            true
        } else {
            false
        }
    }
}

#[cfg(not(windows))]
pub fn can_read_mft(_drive: &str) -> bool {
    true
}

#[cfg(not(windows))]
pub fn is_elevated() -> bool {
    false
}

#[cfg(not(windows))]
pub fn list_fixed_drives() -> Vec<DriveInfo> {
    vec![DriveInfo {
        root: "/".to_string(),
        is_fixed: true,
    }]
}
