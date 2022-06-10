// thanks to https://github.com/maddymakesgames for the ground work on the windows api

use crate::api::*;
use ::windows::{
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
        UI::WindowsAndMessaging::{
            EnumWindows,
            GetClassNameW,
            GetCursorPos,
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
            GWL_EXSTYLE,
            GW_OWNER,
            WINDOWINFO,
            WS_EX_APPWINDOW,
        },
    },
};
use std::string::FromUtf16Error;

const TRUE: BOOL = BOOL(1);
const FALSE: BOOL = BOOL(0);

struct WindowsMonitor {
    device: DISPLAY_DEVICEW,
    device_mode: DEVMODEW,
    device_index: u32,
    name: String,
}

pub struct WindowsApi;

impl WindowsApi {
    pub fn new() -> Result<Self, Error> {
        Ok(WindowsApi)
    }

    fn monitors_impl(&self) -> Result<Vec<WindowsMonitor>, Error> {
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

        Ok(devices)
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
        ))?;

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
        _x: u32,
        _y: u32,
        _monitor_id: MonitorId,
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn set_pointer_position_relative(
        &mut self,
        _x: u32,
        _y: u32,
        _window_id: WindowId,
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn toggle_mouse(
        &mut self,
        _button: MouseButton,
        _down: bool,
        _window_id: Option<WindowId>,
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn clipboard_content(
        &mut self,
        _type_name: &ClipboardType,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        unimplemented!()
    }

    fn set_clipboard_content(
        &mut self,
        _type_name: &ClipboardType,
        _content: &[u8],
    ) -> Result<(), Self::Error> {
        unimplemented!()
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

                return Some(Window {
                    id: handle.0 as u32,
                    name,
                    width: (rect.right - rect.left) as u32,
                    height: (rect.bottom - rect.top) as u32,
                });
            })
            .collect())
    }

    fn capture_monitor_frame(&mut self, _monitor_id: MonitorId) -> Result<Frame, Self::Error> {
        unimplemented!()
    }

    fn capture_window_frame(&mut self, _window_id: WindowId) -> Result<Frame, Self::Error> {
        unimplemented!()
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
}
