use std::path::{Path, PathBuf};
use egui::ColorImage;

use super::model::InstalledApp;

/// Generates a clean 2-letter monogram for applications (e.g. "VS" for Visual Studio Code).
pub fn app_monogram(name: &str) -> String {
    let clean = name.trim();
    if clean.is_empty() {
        return "AP".to_string();
    }
    let words: Vec<&str> = clean
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();

    if words.len() >= 2 {
        let first = words[0].chars().next().unwrap_or('A');
        let second = words[1].chars().next().unwrap_or('P');
        format!("{}{}", first.to_ascii_uppercase(), second.to_ascii_uppercase())
    } else {
        let mut chars = clean.chars().filter(|c| c.is_alphanumeric());
        let first = chars.next().unwrap_or('A').to_ascii_uppercase();
        let second = chars.next().unwrap_or('P').to_ascii_uppercase();
        format!("{first}{second}")
    }
}

/// Extracts the icon image for an installed application, running safely on background threads.
pub fn extract_for_app(app: &InstalledApp) -> Option<std::sync::Arc<ColorImage>> {
    let (path, idx) = resolve_icon_path(app)?;
    extract_app_icon(&path, idx).map(std::sync::Arc::new)
}

/// Resolves the candidate icon file path and index for an installed application.
pub fn resolve_icon_path(app: &InstalledApp) -> Option<(PathBuf, i32)> {
    if let Some(raw_icon) = &app.display_icon {
        if let Some(res) = parse_icon_target(raw_icon) {
            if res.0.exists() {
                return Some(res);
            }
        }
    }

    if let Some(loc) = &app.install_location {
        if loc.is_dir() {
            // Check direct .ico files
            if let Ok(entries) = std::fs::read_dir(loc) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                        if ext == "ico" || ext == "exe" {
                            return Some((path, 0));
                        }
                    }
                }
            }
        } else if loc.is_file() {
            return Some((loc.clone(), 0));
        }
    }

    // Try parsing executable from uninstall_string
    if let Some(res) = parse_icon_target(&app.uninstall_string) {
        if res.0.exists() && res.0.is_file() {
            return Some(res);
        }
    }

    None
}

fn parse_icon_target(raw: &str) -> Option<(PathBuf, i32)> {
    let trimmed = raw.trim().trim_matches('"');
    if trimmed.is_empty() {
        return None;
    }

    if let Some((path_part, idx_part)) = trimmed.rsplit_once(',') {
        let idx = idx_part.trim().parse::<i32>().unwrap_or(0);
        let clean_path = path_part.trim().trim_matches('"');
        if !clean_path.is_empty() {
            return Some((PathBuf::from(clean_path), idx));
        }
    }

    // If no comma, check if there are arguments separated by space
    let path = PathBuf::from(trimmed);
    if path.exists() {
        return Some((path, 0));
    }

    // Try splitting at first quote or space for paths like `C:\Program Files\app.exe /uninstall`
    if let Some(idx) = trimmed.find(".exe") {
        let candidate = &trimmed[..idx + 4].trim_matches('"');
        let p = PathBuf::from(candidate);
        if p.exists() {
            return Some((p, 0));
        }
    }

    None
}

#[cfg(windows)]
pub fn extract_app_icon(path: &Path, icon_index: i32) -> Option<ColorImage> {
    use std::ffi::c_void;
    use std::mem::size_of;
    use std::os::windows::ffi::OsStrExt;

    #[repr(C)]
    #[allow(non_snake_case)]
    struct SHFILEINFOW {
        hIcon: *mut c_void,
        iIcon: i32,
        dwAttributes: u32,
        szDisplayName: [u16; 260],
        szTypeName: [u16; 80],
    }

    #[repr(C)]
    #[allow(non_snake_case)]
    struct ICONINFO {
        fIcon: i32,
        xHotspot: u32,
        yHotspot: u32,
        hbmMask: *mut c_void,
        hbmColor: *mut c_void,
    }

    #[repr(C)]
    #[allow(non_snake_case)]
    struct BITMAP {
        bmType: i32,
        bmWidth: i32,
        bmHeight: i32,
        bmWidthBytes: i32,
        bmPlanes: u16,
        bmBitsPixel: u16,
        bmBits: *mut c_void,
    }

    #[repr(C)]
    #[allow(non_snake_case)]
    struct BITMAPINFOHEADER {
        biSize: u32,
        biWidth: i32,
        biHeight: i32,
        biPlanes: u16,
        biBitCount: u16,
        biCompression: u32,
        biSizeImage: u32,
        biXPelsPerMeter: i32,
        biYPelsPerMeter: i32,
        biClrUsed: u32,
        biClrImportant: u32,
    }

    #[repr(C)]
    #[allow(non_snake_case)]
    struct BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER,
        bmiColors: [u32; 1],
    }

    const SHGFI_ICON: u32 = 0x000000100;
    const SHGFI_LARGEICON: u32 = 0x000000000;
    const DIB_RGB_COLORS: u32 = 0;
    const BI_RGB: u32 = 0;

    #[link(name = "shell32")]
    unsafe extern "system" {
        fn SHGetFileInfoW(
            pszPath: *const u16,
            dwFileAttributes: u32,
            psfi: *mut SHFILEINFOW,
            cbFileInfo: u32,
            uFlags: u32,
        ) -> usize;
        fn ExtractIconExW(
            lpszFile: *const u16,
            nIconIndex: i32,
            phiconLarge: *mut *mut c_void,
            phiconSmall: *mut *mut c_void,
            nIcons: u32,
        ) -> u32;
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn DestroyIcon(hIcon: *mut c_void) -> i32;
        fn GetIconInfo(hIcon: *mut c_void, piconinfo: *mut ICONINFO) -> i32;
        fn GetDC(hWnd: *mut c_void) -> *mut c_void;
        fn ReleaseDC(hWnd: *mut c_void, hDC: *mut c_void) -> i32;
    }

    #[link(name = "gdi32")]
    unsafe extern "system" {
        fn DeleteObject(ho: *mut c_void) -> i32;
        fn GetObjectW(h: *mut c_void, c: i32, pv: *mut c_void) -> i32;
        fn GetDIBits(
            hdc: *mut c_void,
            hbm: *mut c_void,
            start: u32,
            cLines: u32,
            lpvBits: *mut c_void,
            lpbmi: *mut BITMAPINFO,
            usage: u32,
        ) -> i32;
    }

    let mut wide_path: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide_path.push(0);

    let mut hicon: *mut c_void = std::ptr::null_mut();

    // 1. Try ExtractIconExW first with requested index
    unsafe {
        let extracted = ExtractIconExW(
            wide_path.as_ptr(),
            icon_index,
            &mut hicon,
            std::ptr::null_mut(),
            1,
        );
        if extracted == 0 || hicon.is_null() {
            // 2. Fall back to Shell icon resolution
            let mut sfi: SHFILEINFOW = std::mem::zeroed();
            let ret = SHGetFileInfoW(
                wide_path.as_ptr(),
                0,
                &mut sfi,
                size_of::<SHFILEINFOW>() as u32,
                SHGFI_ICON | SHGFI_LARGEICON,
            );
            if ret != 0 && !sfi.hIcon.is_null() {
                hicon = sfi.hIcon;
            }
        }

        if hicon.is_null() {
            return None;
        }

        let mut info: ICONINFO = std::mem::zeroed();
        if GetIconInfo(hicon, &mut info) == 0 {
            DestroyIcon(hicon);
            return None;
        }

        let hdc = GetDC(std::ptr::null_mut());
        if hdc.is_null() {
            if !info.hbmColor.is_null() { DeleteObject(info.hbmColor); }
            if !info.hbmMask.is_null() { DeleteObject(info.hbmMask); }
            DestroyIcon(hicon);
            return None;
        }

        let target_bitmap = if !info.hbmColor.is_null() {
            info.hbmColor
        } else {
            info.hbmMask
        };

        let mut bmp: BITMAP = std::mem::zeroed();
        GetObjectW(target_bitmap, size_of::<BITMAP>() as i32, &mut bmp as *mut _ as *mut c_void);

        let width = bmp.bmWidth as usize;
        let height = bmp.bmHeight.abs() as usize;
        if width == 0 || height == 0 || width > 256 || height > 256 {
            if !info.hbmColor.is_null() { DeleteObject(info.hbmColor); }
            if !info.hbmMask.is_null() { DeleteObject(info.hbmMask); }
            ReleaseDC(std::ptr::null_mut(), hdc);
            DestroyIcon(hicon);
            return None;
        }

        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = width as i32;
        bmi.bmiHeader.biHeight = -(height as i32); // top-down
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB;

        let mut bgra_buf = vec![0u8; width * height * 4];
        let lines = GetDIBits(
            hdc,
            target_bitmap,
            0,
            height as u32,
            bgra_buf.as_mut_ptr() as *mut c_void,
            &mut bmi,
            DIB_RGB_COLORS,
        );

        if !info.hbmColor.is_null() { DeleteObject(info.hbmColor); }
        if !info.hbmMask.is_null() { DeleteObject(info.hbmMask); }
        ReleaseDC(std::ptr::null_mut(), hdc);
        DestroyIcon(hicon);

        if lines == 0 {
            return None;
        }

        // Convert BGRA to RGBA for egui
        let mut rgba = vec![0u8; width * height * 4];
        let mut has_nonzero_alpha = false;
        for i in 0..(width * height) {
            let b = bgra_buf[i * 4];
            let g = bgra_buf[i * 4 + 1];
            let r = bgra_buf[i * 4 + 2];
            let a = bgra_buf[i * 4 + 3];
            if a > 0 {
                has_nonzero_alpha = true;
            }
            rgba[i * 4] = r;
            rgba[i * 4 + 1] = g;
            rgba[i * 4 + 2] = b;
            rgba[i * 4 + 3] = a;
        }

        if !has_nonzero_alpha {
            for i in 0..(width * height) {
                rgba[i * 4 + 3] = 255;
            }
        }

        Some(ColorImage::from_rgba_unmultiplied([width, height], &rgba))
    }
}

#[cfg(not(windows))]
pub fn extract_app_icon(_path: &Path, _icon_index: i32) -> Option<ColorImage> {
    None
}
