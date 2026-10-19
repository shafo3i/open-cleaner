use std::fs::{self, OpenOptions};
use std::io::{self, ErrorKind, Write};
use std::path::Path;

use super::{CleanMode, ModuleReport};

const CHUNK_SIZE: usize = 64 * 1024; // 64 KB overwrite buffer

#[cfg(windows)]
fn probe_delete_access(path: &Path, is_readonly: bool) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateFileW(
            lpFileName: *const u16,
            dwDesiredAccess: u32,
            dwShareMode: u32,
            lpSecurityAttributes: *const std::ffi::c_void,
            dwCreationDisposition: u32,
            dwFlagsAndAttributes: u32,
            hTemplateFile: isize,
        ) -> isize;
        fn CloseHandle(hObject: isize) -> i32;
    }

    const DELETE: u32 = 0x0001_0000;
    const FILE_WRITE_ATTRIBUTES: u32 = 0x0000_0100;
    const FILE_SHARE_READ: u32 = 0x0000_0001;
    const FILE_SHARE_WRITE: u32 = 0x0000_0002;
    const FILE_SHARE_DELETE: u32 = 0x0000_0004;
    const OPEN_EXISTING: u32 = 3;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    const INVALID_HANDLE_VALUE: isize = -1;

    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let desired_access = if is_readonly {
        FILE_WRITE_ATTRIBUTES
    } else {
        DELETE
    };

    unsafe {
        let handle = CreateFileW(
            wide.as_ptr(),
            desired_access,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_OPEN_REPARSE_POINT,
            0,
        );
        if handle == INVALID_HANDLE_VALUE {
            Err(io::Error::last_os_error())
        } else {
            let _ = CloseHandle(handle);
            Ok(())
        }
    }
}

#[cfg(windows)]
fn try_posix_unlink(path: &Path) -> bool {
    use std::os::windows::ffi::OsStrExt;

    #[repr(C)]
    struct FileDispositionInfoEx {
        flags: u32,
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateFileW(
            lpFileName: *const u16,
            dwDesiredAccess: u32,
            dwShareMode: u32,
            lpSecurityAttributes: *const std::ffi::c_void,
            dwCreationDisposition: u32,
            dwFlagsAndAttributes: u32,
            hTemplateFile: isize,
        ) -> isize;
        fn SetFileInformationByHandle(
            hFile: isize,
            FileInformationClass: u32,
            lpFileInformation: *const std::ffi::c_void,
            dwBufferSize: u32,
        ) -> i32;
        fn CloseHandle(hObject: isize) -> i32;
    }

    const DELETE: u32 = 0x0001_0000;
    const FILE_SHARE_READ: u32 = 0x0000_0001;
    const FILE_SHARE_WRITE: u32 = 0x0000_0002;
    const FILE_SHARE_DELETE: u32 = 0x0000_0004;
    const OPEN_EXISTING: u32 = 3;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    const INVALID_HANDLE_VALUE: isize = -1;
    const FILE_DISPOSITION_INFO_EX_CLASS: u32 = 21;
    const FILE_DISPOSITION_DELETE: u32 = 0x0000_0001;
    const FILE_DISPOSITION_POSIX_SEMANTICS: u32 = 0x0000_0002;
    const FILE_DISPOSITION_IGNORE_READONLY_ATTRIBUTE: u32 = 0x0000_0010;

    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        let handle = CreateFileW(
            wide.as_ptr(),
            DELETE,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            0,
        );
        if handle == INVALID_HANDLE_VALUE {
            return false;
        }
        let info = FileDispositionInfoEx {
            flags: FILE_DISPOSITION_DELETE
                | FILE_DISPOSITION_POSIX_SEMANTICS
                | FILE_DISPOSITION_IGNORE_READONLY_ATTRIBUTE,
        };
        let ok = SetFileInformationByHandle(
            handle,
            FILE_DISPOSITION_INFO_EX_CLASS,
            (&info as *const FileDispositionInfoEx).cast(),
            std::mem::size_of::<FileDispositionInfoEx>() as u32,
        );
        let _ = CloseHandle(handle);
        ok != 0
    }
}

/// Result of attempting to clean a single file
pub enum CleanResult {
    Cleaned(u64),
    DryRunFound(u64),
    SkippedLocked(u64),
    SkippedPermission(u64),
    Failed(u64, String),
}

/// Securely deletes a single file with native permission recovery and optional zero-fill
pub fn secure_delete_file(path: &Path, mode: CleanMode) -> CleanResult {
    let metadata = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == ErrorKind::NotFound => return CleanResult::Cleaned(0),
        Err(e) => return CleanResult::Failed(0, format!("Metadata error: {e}")),
    };

    // Do not follow symlinks/junctions for file deletion to avoid escaping the target tree
    if metadata.file_type().is_symlink() {
        if mode.is_dry_run() {
            return CleanResult::DryRunFound(0);
        }
        return match fs::remove_file(path) {
            Ok(_) => CleanResult::Cleaned(0),
            Err(e) => map_io_error(e, 0),
        };
    }

    let file_size = metadata.len();
    let is_readonly = metadata.permissions().readonly();

    if mode.is_dry_run() {
        #[cfg(windows)]
        if let Err(err) = probe_delete_access(path, is_readonly) {
            return map_io_error(err, file_size);
        }
        return CleanResult::DryRunFound(file_size);
    }

    // Handle Windows Read-Only file attribute if present
    if is_readonly {
        let mut permissions = metadata.permissions();
        #[allow(clippy::permissions_set_readonly_false)]
        permissions.set_readonly(false);
        let _ = fs::set_permissions(path, permissions);
    }

    // Optional secure data sanitization (zero-fill wipe before unlink)
    if mode == CleanMode::SecureZero
        && file_size > 0
        && let Err(e) = sanitize_file_data(path, file_size)
    {
        // If sanitization fails due to access/lock, return appropriate skipped result
        return map_io_error(e, file_size);
    }

    // Native file removal with Windows NTFS POSIX unlink fallback for shared-open files
    match fs::remove_file(path) {
        Ok(_) => CleanResult::Cleaned(file_size),
        Err(e) => {
            #[cfg(windows)]
            if try_posix_unlink(path) {
                return CleanResult::Cleaned(file_size);
            }
            map_io_error(e, file_size)
        }
    }
}

/// Overwrites file content with zeros and syncs to disk before deletion
fn sanitize_file_data(path: &Path, total_bytes: u64) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).open(path)?;
    let zero_buffer = [0u8; CHUNK_SIZE];
    let mut remaining = total_bytes;

    while remaining > 0 {
        let write_len = std::cmp::min(remaining, CHUNK_SIZE as u64) as usize;
        file.write_all(&zero_buffer[..write_len])?;
        remaining -= write_len as u64;
    }

    file.sync_all()?;
    file.set_len(0)?;
    Ok(())
}

/// Translates OS I/O error into gracefully categorized CleanResult
fn map_io_error(err: io::Error, file_size: u64) -> CleanResult {
    let raw_os_err = err.raw_os_error().unwrap_or(0);

    // Windows Error 32: ERROR_SHARING_VIOLATION (file is opened/locked by an active process)
    // Windows Error 33: ERROR_LOCK_VIOLATION
    if raw_os_err == 32 || raw_os_err == 33 {
        return CleanResult::SkippedLocked(file_size);
    }

    // Windows Error 5: ERROR_ACCESS_DENIED
    if err.kind() == ErrorKind::PermissionDenied || raw_os_err == 5 {
        return CleanResult::SkippedPermission(file_size);
    }

    CleanResult::Failed(file_size, err.to_string())
}

/// Recursively cleans directory contents based on an optional filename predicate
pub fn clean_directory_contents<F>(
    dir_path: &Path,
    recursive: bool,
    preserve_root: bool,
    filter: &F,
    mode: CleanMode,
    report: &mut ModuleReport,
    category_desc: &str,
) where
    F: Fn(&Path) -> bool,
{
    if !dir_path.exists() || !dir_path.is_dir() {
        return;
    }

    let entries = match fs::read_dir(dir_path) {
        Ok(e) => e,
        Err(e) => {
            report.record_skipped_permission(
                dir_path.to_path_buf(),
                0,
                format!("{category_desc} (Dir read error: {e})"),
            );
            return;
        }
    };

    for entry_res in entries {
        let entry = match entry_res {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        // On Windows, entry.metadata() uses the WIN32_FIND_DATA already fetched by ReadDir
        // with zero additional kernel syscalls.
        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        #[cfg(windows)]
        let is_reparse = {
            use std::os::windows::fs::MetadataExt;
            const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
            metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
        };
        #[cfg(not(windows))]
        let is_reparse = metadata.file_type().is_symlink();

        if is_reparse {
            // Safely skip junctions/reparse points without marking the target as failed
            continue;
        }

        let path = entry.path();
        let file_type = metadata.file_type();

        if file_type.is_dir() {
            if recursive {
                clean_directory_contents(&path, true, false, filter, mode, report, category_desc);
                // Attempt to remove now-empty directory if allowed
                if !mode.is_dry_run() {
                    let _ = fs::remove_dir(&path);
                }
            }
        } else if (file_type.is_file() || file_type.is_symlink()) && filter(&path) {
            if mode.is_dry_run() {
                report.record_scanned(path, metadata.len(), category_desc);
                continue;
            }
            match secure_delete_file(&path, mode) {
                CleanResult::Cleaned(size) => {
                    report.record_cleaned(path, size, category_desc);
                }
                CleanResult::DryRunFound(size) => {
                    report.record_scanned(path, size, category_desc);
                }
                CleanResult::SkippedLocked(size) => {
                    report.record_skipped_locked(path, size, category_desc);
                }
                CleanResult::SkippedPermission(size) => {
                    report.record_skipped_permission(path, size, category_desc);
                }
                CleanResult::Failed(size, err) => {
                    report.record_failed(path, size, err, category_desc);
                }
            }
        }
    }

    if !preserve_root && !mode.is_dry_run() {
        let _ = fs::remove_dir(dir_path);
    }
}
