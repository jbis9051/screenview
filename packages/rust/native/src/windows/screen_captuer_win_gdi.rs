use crate::{
    api::{MonitorId, WindowId},
    windows::FALSE,
};
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::HWND,
        Graphics::Gdi::{
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
};

#[derive(Debug)]
struct Rect {
    width: u32,
    height: u32,
    x: i32,
    y: i32,
}

pub struct ScreenCapturerWinGdi {
    desktop_dc: HDC,
    memory_dc: CreatedHDC,
    bitmap: HBITMAP,
    monitor_id: MonitorId,
    monitor_key: Option<[u16; 128]>,
}

impl Drop for ScreenCapturerWinGdi {
    fn drop(&mut self) {
        unsafe {
            ReleaseDC(HWND::default(), self.desktop_dc);
            DeleteDC(self.memory_dc);
            DeleteObject(self.bitmap);
        }
    }
}

impl ScreenCapturerWinGdi {
    pub fn new(monitor_id: MonitorId, monitor_key: Option<[u16; 128]>) -> Result<Self, ()> {
        let desktop_dc = unsafe { GetDC(HWND::default()) };
        let memory_dc = unsafe { CreateCompatibleDC(desktop_dc) };
        let rect = Self::get_screen_rect(monitor_id, &monitor_key).ok_or(())?;
        let bitmap =
            unsafe { CreateCompatibleBitmap(desktop_dc, rect.width as _, rect.height as _) };

        Ok(Self {
            desktop_dc,
            memory_dc,
            bitmap,
            monitor_id,
            monitor_key,
        })
    }

    fn get_screen_rect(id: MonitorId, key: &Option<[u16; 128]>) -> Option<Rect> {
        let mut device = DISPLAY_DEVICEW::default();
        device.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
        let result =
            unsafe { EnumDisplayDevicesW(PCWSTR(std::ptr::null_mut()), id, &mut device, 0) };

        if result == FALSE {
            return None;
        }

        if let Some(key) = key {
            if key != &device.DeviceKey {
                return None;
            }
        }

        let mut device_mode = DEVMODEW::default();
        device_mode.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
        device_mode.dmDriverExtra = 0;

        let result = unsafe {
            EnumDisplaySettingsExW(
                PCWSTR(device.DeviceName.as_ptr()),
                ENUM_CURRENT_SETTINGS,
                &mut device_mode,
                0,
            )
        };

        if result == FALSE {
            return None;
        }

        let position = unsafe { device_mode.Anonymous1.Anonymous2.dmPosition };

        Some(Rect {
            width: device_mode.dmPelsWidth,
            height: device_mode.dmPelsHeight,
            x: position.x,
            y: position.y,
        })
    }

    pub fn capture(&self) -> Result<(Vec<u8>, u32, u32), ()> {
        let rect = Self::get_screen_rect(self.monitor_id, &self.monitor_key).ok_or(())?;

        let prev = unsafe { SelectObject(self.memory_dc, self.bitmap) };

        if prev.is_invalid() {
            return Err(());
        }

        let result = unsafe {
            BitBlt(
                self.memory_dc,
                0,
                0,
                rect.width as _,
                rect.height as _,
                self.desktop_dc,
                rect.x,
                rect.y,
                ROP_CODE(SRCCOPY.0 | CAPTUREBLT.0),
            )
        };

        if result == FALSE {
            return Err(());
        }

        unsafe { SelectObject(self.memory_dc, prev) };

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
                self.desktop_dc,
                self.bitmap,
                0,
                rect.height,
                &mut bgra[0] as *mut u8 as _,
                &mut bmi as _,
                DIB_RGB_COLORS,
            )
        };

        if res == 0 {
            return Err(());
        }

        unsafe { bgra.set_len(rect.width as usize * rect.height as usize * 4) };

        Ok((bgra, rect.width, rect.height))
    }
}
