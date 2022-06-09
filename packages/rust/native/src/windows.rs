// thanks to https://github.com/maddymakesgames for the ground work on the windows api

use crate::api::*;
use ::windows::{
    core::PSTR,
    Win32::{
        Foundation::{BOOL, HANDLE, HWND, LPARAM, POINT, RECT},
        Graphics::Gdi::{
            EnumDisplayMonitors,
            GetDC,
            GetMonitorInfoA,
            HDC,
            HMONITOR,
            MONITORINFO,
            MONITORINFOEXA,
        },
        UI::WindowsAndMessaging::{
            EnumWindows,
            GetCursorPos,
            GetWindowInfo,
            GetWindowTextA,
            GetWindowTextLengthA,
            GetWindowTextW,
            WINDOWINFO,
        },
    },
};

pub struct WindowsApi;

impl WindowsApi {
    pub fn new() -> Result<Self, Error> {
        Ok(WindowsApi)
    }
}

impl NativeApiTemplate for WindowsApi {
    type Error = Error;

    fn key_toggle(&mut self, _key: Key, _down: bool) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn pointer_position(&mut self) -> Result<MousePosition, Self::Error> {
        let mut point = POINT { x: 0, y: 0 };
        unsafe { GetCursorPos(&mut point) }
            .as_bool()
            .then(|| MousePosition {
                x: point.x as u32,
                y: point.y as u32,
                monitor_id: 0,
                window_relatives: vec![],
            })
            .ok_or(Error::WindowsApiError)
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

    fn monitors(&mut self) -> Result<Vec<Monitor>, Self::Error> {
        let mut monitor_info: Vec<HMONITOR> = Vec::new();

        if !unsafe {
            EnumDisplayMonitors(
                std::ptr::null(),
                std::ptr::null(),
                Some(monitor_callback),
                LPARAM(&mut monitor_info as *mut Vec<_> as isize),
            )
        }
        .as_bool()
        {
            return Err(Error::WindowsApiError);
        }


        monitor_vec.filter_map(|monitor_handle| {
            // we use the a(nsi) version so we don't have to deal with converting unicode bytes to char
            let mut monitor_info = MONITORINFOEXA::default();
            monitor_info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXA>() as u32;

            if !unsafe {
                GetMonitorInfoA(
                    monitor_handle,
                    &mut monitor_info as *mut MONITORINFOEXA as *mut MONITORINFO,
                )
            }
            .as_bool()
            {
                return None;
            }

            let monitor_size = monitor_info.monitorInfo.rcMonitor;

            Some(Monitor {
                // we can just cast the bytes to char because we know its using ansi encoding
                name: monitor_info
                    .szDevice
                    .iter()
                    .map(|s| s.0 as char)
                    .collect::<String>(),
                height: (monitor_size.bottom - monitor_size.top) as u32,
                width: (monitor_size.right - monitor_size.left) as u32,
                id: 0,
            })
        });

        Ok(monitor_info)
    }

    fn windows(&mut self) -> Result<Vec<Window>, Self::Error> {
        let mut window_info = Vec::<Window>::new();

        unsafe {
            if !EnumWindows(
                Some(window_callback),
                LPARAM(&mut window_info as *mut Vec<_> as isize),
            )
            .as_bool()
            {
                return Err(Error::WindowsApiError);
            }
        }

        Ok(window_info
            .into_iter()
            .filter(|w| w.width != 0 && w.height != 0 && w.name != "")
            .collect())
    }

    fn capture_monitor_frame(&mut self, _monitor_id: MonitorId) -> Result<Frame, Self::Error> {
        unimplemented!()
    }

    fn capture_window_frame(&mut self, _window_id: WindowId) -> Result<Frame, Self::Error> {
        unimplemented!()
    }
}


unsafe extern "system" fn monitor_callback(
    monitor_handle: HMONITOR,
    hdc: HDC,
    rect: *mut RECT,
    data_ptr: LPARAM,
) -> BOOL {
    let monitor_vec = &mut *(data_ptr.0 as *mut Vec<_>);
    monitor_vec.push(monitor_handle);
    true.into()
}

unsafe extern "system" fn window_callback(window_handle: HWND, data_ptr: LPARAM) -> BOOL {
    let window_vec = &mut *(data_ptr.0 as *mut Vec<Window>);

    let mut window_info = WINDOWINFO::default();
    window_info.cbSize = std::mem::size_of::<WINDOWINFO>() as u32;

    if !GetWindowInfo(window_handle, &mut window_info as *mut _).as_bool() {
        return BOOL(1);
    }

    let dialog_len = GetWindowTextLengthA(window_handle) + 1;
    let mut str_bytes = vec![0_u8; dialog_len as usize];
    GetWindowTextA(window_handle, &mut str_bytes);

    let window_size = window_info.rcWindow;

    window_vec.push(Window {
        id: 0,
        name: str_bytes[.. str_bytes.len().saturating_sub(1)]
            .iter()
            .map(|b| *b as char)
            .collect(),
        width: (window_size.bottom - window_size.top) as u32,
        height: (window_size.right - window_size.left) as u32,
    });

    BOOL(1)
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("windows api error occured")]
    WindowsApiError,
}
