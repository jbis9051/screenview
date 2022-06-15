// thanks to https://github.com/maddymakesgames for the ground work on the windows api
mod screen_captuer_win_gdi;
mod util;
mod window_captuer_win_gdi;

use crate::{
    api::*,
    windows::{
        screen_captuer_win_gdi::ScreenCapturerWinGdi,
        window_captuer_win_gdi::WindowCapturerWinGdi,
    },
};
use image::RgbImage;
use std::{collections::HashMap, string::FromUtf16Error};
use windows::{
    core::{PCWSTR, PSTR},
    Win32::{
        Foundation::{BOOL, HANDLE, HWND, LPARAM, POINT, RECT},
        Graphics::Gdi::{
            EnumDisplayDevicesW,
            EnumDisplayMonitors,
            EnumDisplaySettingsExW,
            GetDC,
            GetMonitorInfoA,
            DEVMODEW,
            DISPLAY_DEVICEW,
            DISPLAY_DEVICE_ACTIVE,
            ENUM_CURRENT_SETTINGS,
            HDC,
            HMONITOR,
            MONITORINFO,
            MONITORINFOEXA,
        },
        System::{
            DataExchange::{
                CloseClipboard,
                EmptyClipboard,
                GetClipboardData,
                OpenClipboard,
                SetClipboardData,
            },
            Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE},
            SystemServices::{CF_TEXT, CF_UNICODETEXT, CLIPBOARD_FORMATS},
        },
        UI::{
            Input::KeyboardAndMouse::{
                SendInput,
                SetActiveWindow,
                INPUT,
                INPUT_MOUSE,
                MOUSEEVENTF_ABSOLUTE,
                MOUSEEVENTF_HWHEEL,
                MOUSEEVENTF_LEFTDOWN,
                MOUSEEVENTF_LEFTUP,
                MOUSEEVENTF_MIDDLEDOWN,
                MOUSEEVENTF_MIDDLEUP,
                MOUSEEVENTF_MOVE,
                MOUSEEVENTF_RIGHTDOWN,
                MOUSEEVENTF_RIGHTUP,
                MOUSEEVENTF_VIRTUALDESK,
                MOUSEEVENTF_WHEEL,
                MOUSEEVENTF_XDOWN,
                MOUSEEVENTF_XUP,
            },
            WindowsAndMessaging::{
                EnumWindows,
                GetClassNameW,
                GetCursorPos,
                GetSystemMetrics,
                GetWindow,
                GetWindowInfo,
                GetWindowLongW,
                GetWindowRect,
                GetWindowTextA,
                GetWindowTextLengthA,
                GetWindowTextLengthW,
                GetWindowTextW,
                GetWindowThreadProcessId,
                IsIconic,
                IsWindowVisible,
                SetForegroundWindow,
                GWL_EXSTYLE,
                GW_OWNER,
                SM_CXVIRTUALSCREEN,
                SM_CYVIRTUALSCREEN,
                WHEEL_DELTA,
                WINDOWINFO,
                WS_EX_APPWINDOW,
                XBUTTON1,
                XBUTTON2,
            },
        },
    },
};


struct WindowsMonitor {
    device: DISPLAY_DEVICEW,
    device_mode: DEVMODEW,
    device_index: u32,
    name: String,
}

struct WindowsWindow {
    handle: HWND,
    name: String,
    rect: RECT,
}

pub struct WindowsApi {
    monitors_key_cache: Vec<[u16; 128]>,
    monitor_capturers: Vec<(MonitorId, ScreenCapturerWinGdi)>,
    window_capturers: Vec<(WindowId, WindowCapturerWinGdi)>,
}

impl WindowsApi {
    pub fn new() -> Result<Self, Error> {
        Ok(WindowsApi {
            monitors_key_cache: Vec::new(),
            monitor_capturers: Vec::new(),
            window_capturers: Vec::new(),
        })
    }

    fn clipboard_map(format: &ClipboardType) -> CLIPBOARD_FORMATS {
        match format {
            ClipboardType::Text => CF_TEXT,
            _ => panic!("unsupported clipboard type"),
        }
    }

    fn monitors_impl(&mut self) -> Result<Vec<WindowsMonitor>, Error> {
        // https://github.com/mozilla/gecko-dev/blob/e1d59e5c596916b73257a2a7384fd4a2b88047e6/third_party/libwebrtc/modules/desktop_capture/win/screen_capture_utils.cc#L25
        let mut devices = Vec::new();

        let mut device_index = 0;
        let mut enum_result: BOOL = TRUE;
        loop {
            let mut device = DISPLAY_DEVICEW::default();
            device.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
            enum_result = unsafe {
                EnumDisplayDevicesW(PCWSTR(std::ptr::null_mut()), device_index, &mut device, 0)
            };

            // |enum_result| is 0 if we have enumerated all devices.
            if enum_result == FALSE {
                break;
            }

            // We only care about active displays.
            if !((device.StateFlags & DISPLAY_DEVICE_ACTIVE) == 1) {
                device_index += 1;
                continue;
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
                return Err(Error::WindowsApiError("EnumDisplaySettingsExW".to_string()));
            }
            let name = match String::from_utf16(&device.DeviceName) {
                Ok(s) => s,
                Err(_) => {
                    device_index += 1;
                    continue;
                }
            }
            .trim_end_matches('\0')
            .to_string();

            devices.push(WindowsMonitor {
                device,
                device_mode,
                device_index,
                name,
            });

            device_index += 1;
        }

        self.monitors_key_cache = devices.iter().map(|m| m.device.DeviceKey).collect();

        Ok(devices)
    }

    fn windows_impl(&mut self) -> Result<Vec<WindowsWindow>, Error> {
        let mut window_info = Vec::<HWND>::new();

        unsafe {
            if !EnumWindows(
                Some(window_callback),
                LPARAM(&mut window_info as *mut Vec<_> as isize),
            )
            .as_bool()
            {
                return Err(Error::WindowsApiError("EnumWindows".to_string()));
            }
        }

        Ok(window_info
            .into_iter()
            .filter_map(|handle| {
                // Skip invisible and minimized windows
                if unsafe { IsWindowVisible(handle) } == FALSE
                    || unsafe { IsIconic(handle) } == TRUE
                {
                    return None;
                }

                // Skip windows which are not presented in the taskbar,
                // namely owned window if they don't have the app window style set
                let owner = unsafe { GetWindow(handle, GW_OWNER) };
                let exstyle = unsafe { GetWindowLongW(handle, GWL_EXSTYLE) };
                if owner.0 != 0 && !((exstyle & (WS_EX_APPWINDOW.0 as i32)) == 1) {
                    return None;
                }

                // TODO consider skipping unresponsive windows

                // GetWindowText* are potentially blocking operations if |hwnd| is
                // owned by the current process. The APIs will send messages to the window's
                // message loop, and if the message loop is waiting on this operation we will
                // enter a deadlock.
                // https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getwin
                //
                // To help consumers avoid this, there is a DesktopCaptureOption to ignore
                // windows owned by the current process. Consumers should either ensure that
                // the thread running their message loop never waits on this operation, or use
                // the option to exclude these windows from the source list.
                let mut process_id = 0;
                unsafe { GetWindowThreadProcessId(handle, &mut process_id) };

                if process_id == std::process::id() {
                    return None;
                }

                const kTitleLength: usize = 500;
                let mut window_title = [0u16; kTitleLength];
                if unsafe { GetWindowTextLengthW(handle) } == 0
                    || unsafe { GetWindowTextW(handle, &mut window_title) } == 0
                {
                    return None;
                }

                // Capture the window class name, to allow specific window classes to be
                // skipped.
                //
                // https://docs.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-wndclassa
                // says lpszClassName field in WNDCLASS is limited by 256 symbols, so we don't
                // need to have a buffer bigger than that.
                const kMaxClassNameLength: usize = 256;
                let mut class_name = [0u16; kMaxClassNameLength];
                let class_name_length = unsafe { GetClassNameW(handle, &mut class_name) };
                if class_name_length < 1 {
                    return None;
                }

                // Skip Program Manager window.
                if compare_windows_str(&class_name, "Program") {
                    return None;
                }

                // Skip Start button window on Windows Vista, Windows 7.
                // On Windows 8, Windows 8.1, Windows 10 Start button is not a top level
                // window, so it will not be examined here.
                if compare_windows_str(&class_name, "Button") {
                    return None;
                }

                let mut rect = RECT::default();
                if unsafe { GetWindowRect(handle, &mut rect) } == FALSE {
                    return None;
                }

                // TODO more checks "IsWindowVisibleOnCurrentDesktop"
                let name = match String::from_utf16(&window_title) {
                    Ok(s) => s,
                    Err(_) => return None,
                }
                .trim_end_matches('\0')
                .to_string();

                Some(WindowsWindow { handle, name, rect })
            })
            .collect())
    }

    #[inline]
    fn mouse_coord_to_abs(coord: i32, width_or_height: i32) -> i32 {
        (65536 * (coord) / width_or_height) + (if coord < 0 { -1 } else { 1 })
    }

    fn set_pointer_absolute_impl(&self, x: i32, y: i32) -> Result<(), Error> {
        let virtual_width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
        let virtual_height = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };
        let absolute_x = Self::mouse_coord_to_abs(x, virtual_width);
        let absolute_y = Self::mouse_coord_to_abs(y, virtual_height);

        let mut input = INPUT::default();
        input.r#type = INPUT_MOUSE;
        input.Anonymous.mi.dx = absolute_x;
        input.Anonymous.mi.dy = absolute_y;
        input.Anonymous.mi.dwFlags =
            MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_MOVE | MOUSEEVENTF_VIRTUALDESK;

        unsafe {
            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        };

        Ok(())
    }

    fn open_clipboard() -> Result<(), Error> {
        let mut clipboard = FALSE;
        for _ in 0 .. 5 {
            clipboard = unsafe { OpenClipboard(HWND::default()) };
            if clipboard == TRUE {
                break;
            }
        }
        if clipboard == FALSE {
            return Err(Error::UnableToOpenClipboard);
        }
        Ok(())
    }
}

fn compare_windows_str(a: &[u16], b: &str) -> bool {
    a.iter()
        .copied()
        .eq(b.encode_utf16().chain(core::iter::once(0)))
}

impl NativeApiTemplate for WindowsApi {
    type Error = Error;

    fn key_toggle(&mut self, _key: Key, _down: bool) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn pointer_position(&mut self, windows: &[WindowId]) -> Result<MousePosition, Self::Error> {
        let mut point = POINT { x: 0, y: 0 };
        if unsafe { GetCursorPos(&mut point) } == FALSE {
            return Err(Error::WindowsApiError("GetCursorPos".to_string()));
        }

        let monitor = self.monitors_impl()?.into_iter().find(|monitor| {
            let position = unsafe { monitor.device_mode.Anonymous1.Anonymous2.dmPosition };
            if point.x < position.x || point.x > position.x + monitor.device_mode.dmPelsWidth as i32
            {
                return false;
            }
            if point.y < position.y
                || point.y > position.y + monitor.device_mode.dmPelsHeight as i32
            {
                return false;
            }
            true
        });

        let monitor = monitor.ok_or(Error::WindowsApiError(
            "logical error pointer not on monitor".to_string(),
        ))?; // TODO should be different error type

        let window_relatives = windows
            .into_iter()
            .filter_map(|w| {
                let mut rect = RECT::default();
                if unsafe { GetWindowRect(HWND(*w as isize), &mut rect) } == FALSE {
                    return None;
                }
                if point.x < rect.left || point.x > rect.right {
                    return None;
                }
                if point.y < rect.top || point.y > rect.bottom {
                    return None;
                }
                Some(PointerPositionRelative {
                    x: (point.x - rect.left) as u32,
                    y: (point.y - rect.top) as u32,
                    window_id: *w,
                })
            })
            .collect();


        Ok(MousePosition {
            x: point.x as u32,
            y: point.y as u32,
            monitor_id: monitor.device_index,
            window_relatives,
        })
    }

    fn set_pointer_position_absolute(
        &mut self,
        x: u32,
        y: u32,
        monitor_id: MonitorId,
    ) -> Result<(), Self::Error> {
        let monitors = self.monitors_impl()?;
        let monitor = monitors
            .get(monitor_id as usize)
            .ok_or(Error::MonitorNotFound)?;

        let monitor_key = self
            .monitors_key_cache
            .get(monitor_id as usize)
            .ok_or(Error::MonitorNotFound)?;

        // Verifies the device index still maps to the same display device, to make
        // sure we are referencing the same device when devices are added or removed.
        // DeviceKey is documented as reserved, but it actually contains the registry
        // key for the device and is unique for each monitor, while DeviceID is not.
        if monitor_key != &monitor.device.DeviceKey {
            return Err(Error::MonitorNotFound);
        }
        let position = unsafe { monitor.device_mode.Anonymous1.Anonymous2.dmPosition };

        self.set_pointer_absolute_impl(x as i32 + position.x, y as i32 + position.y)
    }

    fn set_pointer_position_relative(
        &mut self,
        x: u32,
        y: u32,
        window_id: WindowId,
    ) -> Result<(), Self::Error> {
        let windows = self
            .windows_impl()?
            .into_iter()
            .find(|w| w.handle.0 == window_id as isize)
            .ok_or(Error::WindowNotFound)?;
        let absolute_x = windows.rect.left + x as i32;
        let absolute_y = windows.rect.top + y as i32;
        self.set_pointer_absolute_impl(absolute_x, absolute_y)
    }

    fn toggle_mouse(
        &mut self,
        button: MouseButton,
        down: bool,
        window_id: Option<WindowId>,
    ) -> Result<(), Self::Error> {
        if let Some(window_id) = window_id {
            // Windows does not allow us to bring any window to the foreground. There are many
            // restrictions and in most cases, we aren't allowed to. If this fails we click anyway
            // and hope for the best.
            // A possible TODO would be to, if this fails, check if the window is visible where the mouse is and if not return
            // More info https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setforegroundwindow?redirectedfrom=MSDN
            unsafe { SetForegroundWindow(HWND(window_id as isize)) };
        }
        let mut input = INPUT::default();
        input.r#type = INPUT_MOUSE;
        input.Anonymous.mi.dx = 0;
        input.Anonymous.mi.dy = 0;
        input.Anonymous.mi.dwFlags = match button {
            MouseButton::Left =>
                if down {
                    MOUSEEVENTF_LEFTDOWN
                } else {
                    MOUSEEVENTF_LEFTUP
                },
            MouseButton::Center =>
                if down {
                    MOUSEEVENTF_MIDDLEDOWN
                } else {
                    MOUSEEVENTF_MIDDLEUP
                },
            MouseButton::Right =>
                if down {
                    MOUSEEVENTF_RIGHTDOWN
                } else {
                    MOUSEEVENTF_RIGHTUP
                },
            MouseButton::ScrollUp | MouseButton::ScrollDown => MOUSEEVENTF_WHEEL,
            MouseButton::ScrollLeft | MouseButton::ScrollRight => MOUSEEVENTF_HWHEEL,
            MouseButton::Button4 | MouseButton::Button5 =>
                if down {
                    MOUSEEVENTF_XDOWN
                } else {
                    MOUSEEVENTF_XUP
                },
        };

        input.Anonymous.mi.mouseData = match button {
            MouseButton::ScrollUp | MouseButton::ScrollRight => WHEEL_DELTA as i32,
            MouseButton::ScrollDown | MouseButton::ScrollLeft => -(WHEEL_DELTA as i32),
            MouseButton::Button4 => XBUTTON1.0 as i32,
            MouseButton::Button5 => XBUTTON2.0 as i32,
            _ => 0,
        };


        unsafe {
            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        };

        Ok(())
    }

    fn clipboard_content(
        &mut self,
        type_name: &ClipboardType,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        if let ClipboardType::Custom(_) = type_name {
            return Ok(None);
        }

        let windows_type = Self::clipboard_map(type_name);

        Self::open_clipboard()?;

        let mut handle = unsafe { GetClipboardData(windows_type.0) }
            .map_err(|_| Error::WindowsApiError("GetClipboardData".to_string()))?;
        if handle.is_invalid() {
            return Ok(None);
        }
        let mut str = unsafe { GlobalLock(handle.0) } as *mut u8;

        if str.is_null() {
            return Ok(Some(Vec::new()));
        }

        // TODO: add a better way of reading data from clipboard
        let mut output = Vec::new();

        // we know text will be null terminated
        let mut char = unsafe { *str };
        while char != 0 {
            output.push(char);
            str = unsafe { str.add(1) };
            char = unsafe { *str };
        }

        unsafe {
            GlobalUnlock(handle.0);
            CloseClipboard();
        }


        Ok(Some(output))
    }

    fn set_clipboard_content(
        &mut self,
        type_name: &ClipboardType,
        content: &[u8],
    ) -> Result<(), Self::Error> {
        if let ClipboardType::Custom(_) = type_name {
            // windows doesn't support custom clipboard types by string
            return Ok(());
        }

        let windows_type = Self::clipboard_map(type_name);

        Self::open_clipboard()?;
        fn close_clipboard() {
            unsafe {
                CloseClipboard();
            }
        }
        unsafe {
            EmptyClipboard();
        }
        let alloc = unsafe { GlobalAlloc(GMEM_MOVEABLE, content.len()) } as *const u8;
        if alloc.is_null() {
            close_clipboard();
            return Err(Error::WindowsApiError("GlobalAlloc".to_string()));
        }
        let mut ptr = unsafe { GlobalLock(alloc as isize) } as *mut u8;
        unsafe {
            ptr.copy_from_nonoverlapping(content.as_ptr(), content.len());
        }
        unsafe {
            GlobalUnlock(alloc as isize);
        }
        let handle = unsafe { SetClipboardData(windows_type.0, HANDLE(alloc as isize)) };
        close_clipboard();
        match handle {
            Ok(h) =>
                if h.is_invalid() {
                    Err(Error::WindowsApiError("SetClipboardData".to_string()))
                } else {
                    Ok(())
                },
            Err(_) => Err(Error::WindowsApiError("SetClipboardData".to_string())),
        }
    }

    /// Note: The device id returned by this method is not guaranteed to be consistant
    fn monitors(&mut self) -> Result<Vec<Monitor>, Self::Error> {
        Ok(self
            .monitors_impl()?
            .into_iter()
            .map(|m| Monitor {
                id: m.device_index,
                name: m.name,
                width: m.device_mode.dmPelsWidth,
                height: m.device_mode.dmPelsHeight,
            })
            .collect())
    }

    fn windows(&mut self) -> Result<Vec<Window>, Self::Error> {
        let mut window_info = self.windows_impl()?;

        Ok(window_info
            .into_iter()
            .map(|ww| Window {
                id: ww.handle.0 as u32,
                name: ww.name,
                width: (ww.rect.right - ww.rect.left) as u32,
                height: (ww.rect.bottom - ww.rect.top) as u32,
            })
            .collect())
    }

    fn capture_monitor_frame(&mut self, monitor_id: MonitorId) -> Result<Frame, Self::Error> {
        let capturer = self
            .monitor_capturers
            .iter()
            .find(|(id, _)| id == &monitor_id);

        let capturer = match capturer {
            None => {
                self.monitor_capturers.push((
                    monitor_id,
                    ScreenCapturerWinGdi::new(
                        monitor_id,
                        self.monitors_key_cache.get(monitor_id as usize).copied(),
                    )
                    .map_err(|()| Error::WindowsApiError("GDI".to_string()))?,
                ));
                &self.monitor_capturers.last_mut().unwrap().1
            }
            Some(c) => &c.1,
        };

        let (data, width, height) = capturer
            .capture()
            .map_err(|()| Error::WindowsApiError("GDI Capture".to_string()))?;

        Ok(RgbImage::from_vec(width, height, data).expect("Failed to create image"))
    }

    fn capture_window_frame(&mut self, window_id: WindowId) -> Result<Frame, Self::Error> {
        let capturer = self
            .window_capturers
            .iter()
            .find(|(id, _)| id == &window_id);

        let capturer = match capturer {
            None => {
                self.window_capturers.push((
                    window_id,
                    WindowCapturerWinGdi::new(window_id)
                        .map_err(|()| Error::WindowsApiError("GDI".to_string()))?,
                ));
                &self.window_capturers.last_mut().unwrap().1
            }
            Some(c) => &c.1,
        };

        let (data, width, height) = capturer
            .capture()
            .map_err(|()| Error::WindowsApiError("GDI Capture".to_string()))?;

        Ok(RgbImage::from_vec(width, height, data).expect("Failed to create image"))
    }
}

unsafe extern "system" fn window_callback(window_handle: HWND, data_ptr: LPARAM) -> BOOL {
    let window_vec = &mut *(data_ptr.0 as *mut Vec<_>);
    window_vec.push(window_handle);

    TRUE
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("windows api error occured when calling {0}")]
    WindowsApiError(String),
    #[error("monitor not found")]
    MonitorNotFound,
    #[error("window not found")]
    WindowNotFound,
    #[error("unable to open clipboard")]
    UnableToOpenClipboard,
    #[error("unsupported mouse button")]
    UnsupportedMouseButton,
}
