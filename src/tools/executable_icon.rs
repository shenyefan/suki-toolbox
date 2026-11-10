//! Windows icon extraction. Raster dimensions are native icon pixels, not UI geometry.
use gpui_kit::RenderImage;
use std::{os::windows::ffi::OsStrExt, path::Path, sync::Arc};
use windows::{
    Win32::{
        Graphics::Gdi::{
            BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, CreateDIBSection,
            DIB_RGB_COLORS, DeleteDC, DeleteObject, SelectObject,
        },
        Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES,
        System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize},
        UI::{
            Shell::{SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SHGetFileInfoW},
            WindowsAndMessaging::{DI_NORMAL, DestroyIcon, DrawIconEx},
        },
    },
    core::PCWSTR,
};

pub(super) fn extract(path: &Path) -> Option<Arc<RenderImage>> {
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().ok()?;
    }
    let image = extract_initialized(path);
    unsafe {
        CoUninitialize();
    }
    image
}

fn extract_initialized(path: &Path) -> Option<Arc<RenderImage>> {
    let path: std::path::PathBuf = path.components().collect();
    let path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    const SIDE: i32 = 32;
    // Each handle is local; all GDI and icon handles are released before returning.
    unsafe {
        let mut info = SHFILEINFOW::default();
        if SHGetFileInfoW(
            PCWSTR(path.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut info),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        ) == 0
        {
            return None;
        }
        let dc = CreateCompatibleDC(None);
        let bitmap_info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: SIDE,
                biHeight: -SIDE,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let bitmap = CreateDIBSection(Some(dc), &bitmap_info, DIB_RGB_COLORS, &mut bits, None, 0);
        let pixels = match bitmap {
            Ok(bitmap) => {
                let previous = SelectObject(dc, bitmap.into());
                let len = (SIDE * SIDE * 4) as usize;
                std::ptr::write_bytes(bits, 0, len);
                let result = DrawIconEx(dc, 0, 0, info.hIcon, SIDE, SIDE, 0, None, DI_NORMAL);
                let pixels = result
                    .ok()
                    .map(|_| std::slice::from_raw_parts(bits.cast::<u8>(), len).to_vec());
                SelectObject(dc, previous);
                let _ = DeleteObject(bitmap.into());
                pixels
            }
            Err(_) => None,
        };
        let _ = DeleteDC(dc);
        let _ = DestroyIcon(info.hIcon);
        // GPUI RenderImage consumes BGRA, which matches the native DIB byte layout.
        let pixels = pixels?;
        let image = image::RgbaImage::from_raw(SIDE as u32, SIDE as u32, pixels)?;
        Some(Arc::new(RenderImage::new(vec![image::Frame::new(image)])))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn extracts_visible_pixels_from_a_windows_executable() {
        let root = std::env::var_os("WINDIR").unwrap();
        let icon =
            super::extract(&std::path::Path::new(&root).join("System32/notepad.exe")).unwrap();
        let pixels = icon.as_bytes(0).unwrap();
        assert!(pixels.as_chunks::<4>().0.iter().any(|pixel| pixel[3] > 0));
    }
}
