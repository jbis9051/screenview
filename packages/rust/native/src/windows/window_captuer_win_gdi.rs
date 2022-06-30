use crate::{
    api::{MonitorId, WindowId},
    windows::{util::is_windows8or_later, FALSE, TRUE},
};
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{HWND, RECT},
        Graphics::{
            Dwm::DwmIsCompositionEnabled,
            Gdi::{
                BitBlt,
                CreateCompatibleBitmap,
                CreateCompatibleDC,
                CreatedHDC,
                DeleteDC,
                DeleteObject,
                EnumDisplayDevicesW,
                EnumDisplaySettingsExW,
                GetDC,
                GetDIBits,
                GetWindowDC,
                ReleaseDC,
                SelectObject,
                BITMAPINFO,
                BITMAPINFOHEADER,
                BI_RGB,
                CAPTUREBLT,
                DEVMODEW,
                DIB_RGB_COLORS,
                DISPLAY_DEVICEW,
                ENUM_CURRENT_SETTINGS,
                HBITMAP,
                HDC,
                RGBQUAD,
                ROP_CODE,
                SRCCOPY,
            },
        },
        Storage::Xps::{PrintWindow, PRINT_WINDOW_FLAGS},
        UI::WindowsAndMessaging::{GetWindowRect, PW_RENDERFULLCONTENT},
    },
};

#[derive(Debug)]
struct Rect {
    width: u32,
    height: u32,
    x: i32,
    y: i32,
}

pub struct WindowCapturerWinGdi {
    window_id: MonitorId,
}

impl WindowCapturerWinGdi {
    pub fn new(window_id: WindowId) -> Result<Self, ()> {
        Ok(Self { window_id })
    }

    fn get_window_rect(id: WindowId) -> Option<Rect> {
        let mut rect = RECT::default();

        if unsafe { GetWindowRect(HWND(id as isize), &mut rect) } == FALSE {
            return None;
        }

        Some(Rect {
            width: (rect.right - rect.left) as u32,
            height: (rect.bottom - rect.top) as u32,
            x: rect.left,
            y: rect.top,
        })
    }

    pub fn capture(&self) -> Result<(Vec<u8>, u32, u32), ()> {
        let window_dc = unsafe { GetWindowDC(HWND(self.window_id as isize)) };
        if window_dc == HDC::default() {
            return Err(());
        }
        let memory_dc = unsafe { CreateCompatibleDC(window_dc) };
        let rect = Self::get_window_rect(self.window_id).ok_or(())?;
        let bitmap =
            unsafe { CreateCompatibleBitmap(window_dc, rect.width as _, rect.height as _) };
        let prev = unsafe { SelectObject(memory_dc, bitmap) };

        let mut result = FALSE;


        // When desktop composition (Aero) is enabled each window is rendered to a
        // private buffer allowing BitBlt() to get the window content even if the
        // window is occluded. PrintWindow() is slower but lets rendering the window
        // contents to an off-screen device context when Aero is not available.
        // PrintWindow() is not supported by some applications.
        //
        // If Aero is enabled, we prefer BitBlt() because it's faster and avoids
        // window flickering. Otherwise, we prefer PrintWindow() because BitBlt() may
        // render occluding windows on top of the desired window.
        //
        // When composition is enabled the DC returned by GetWindowDC() doesn't always
        // have window frame rendered correctly. Windows renders it only once and then
        // caches the result between captures. We hack it around by calling
        // PrintWindow() whenever window size changes, including the first time of
        // capturing - it somehow affects what we get from BitBlt() on the subsequent
        // captures.
        //
        // For Windows 8.1 and later, we want to always use PrintWindow when the
        // cropping screen capturer falls back to the window capturer. I.e.
        // on Windows 8.1 and later, PrintWindow is only used when the window is
        // occluded. When the window is not occluded, it is much faster to capture
        // the screen and to crop it to the window position and size.
        if is_windows8or_later() {
            // Special flag that makes PrintWindow to work on Windows 8.1 and later.
            // Indeed certain apps (e.g. those using DirectComposition rendering) can't
            // be captured using BitBlt or PrintWindow without this flag. Note that on
            // Windows 8.0 this flag is not supported so the block below will fallback
            // to the other call to PrintWindow. It seems to be very tricky to detect
            // Windows 8.0 vs 8.1 so a try/fallback is more approriate here.
            let flags = PW_RENDERFULLCONTENT;
            result = unsafe {
                PrintWindow(
                    HWND(self.window_id as isize),
                    memory_dc,
                    PRINT_WINDOW_FLAGS(flags),
                )
            };
        }

        if result == FALSE && unsafe { DwmIsCompositionEnabled() } != Ok(TRUE) {
            result = unsafe {
                PrintWindow(
                    HWND(self.window_id as isize),
                    memory_dc,
                    PRINT_WINDOW_FLAGS(0),
                )
            }
        }

        // Aero is enabled or PrintWindow() failed, use BitBlt.
        if result == FALSE {
            result = unsafe {
                BitBlt(
                    memory_dc,
                    0,
                    0,
                    rect.width as _,
                    rect.height as _,
                    window_dc,
                    0,
                    0,
                    SRCCOPY,
                )
            };
        }

        if result == FALSE {
            unsafe {
                DeleteDC(memory_dc);
                ReleaseDC(HWND(self.window_id as isize), window_dc);
            }
            return Err(());
        }

        unsafe { SelectObject(memory_dc, prev) };

        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as _,
                biWidth: rect.width as _,
                biHeight: -(rect.height as i32), // apparently it needs to be negative or it's upside down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB as _,
                biSizeImage: 0, // as per docs
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [RGBQUAD {
                rgbBlue: 0,
                rgbGreen: 0,
                rgbRed: 0,
                rgbReserved: 0,
            }],
        };

        let mut bgra = Vec::with_capacity(rect.width as usize * rect.height as usize * 4);

        let res = unsafe {
            GetDIBits(
                window_dc,
                bitmap,
                0,
                rect.height,
                &mut bgra[0] as *mut u8 as _,
                &mut bmi as _,
                DIB_RGB_COLORS,
            )
        };

        unsafe {
            DeleteDC(memory_dc);
            ReleaseDC(HWND(self.window_id as isize), window_dc);
        }

        if res == 0 {
            return Err(());
        }

        unsafe { bgra.set_len(rect.width as usize * rect.height as usize * 4) };

        Ok((bgra, rect.width, rect.height))
    }
}
