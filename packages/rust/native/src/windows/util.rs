use crate::windows::FALSE;
use windows::Win32::{
    Foundation::BOOL,
    System::SystemInformation::{GetVersionExA, OSVERSIONINFOA},
};

const TRUE: BOOL = BOOL(1);
const FALSE: BOOL = BOOL(0);

#[allow(non_camel_case_types)]
enum WindowsMajorVersions {
    kWindows2000 = 5,
    kWindowsVista = 6,
    kWindows10 = 10,
}

fn get_os_version() -> Result<(u32, u32, u32), ()> {
    let mut info = OSVERSIONINFOA::default();
    info.dwOSVersionInfoSize = std::mem::size_of::<OSVERSIONINFOA>() as u32;
    let res = unsafe { GetVersionExA(&mut info) };
    if res == FALSE {
        return Err(());
    }
    Ok((info.dwMajorVersion, info.dwMinorVersion, info.dwBuildNumber))
}

pub fn is_windows8or_later() -> bool {
    match get_os_version() {
        Ok((major, minor, _)) =>
            (major > WindowsMajorVersions::kWindowsVista as u32)
                || ((major == WindowsMajorVersions::kWindowsVista as u32) && (minor >= 2)),
        Err(_) => false,
    }
}
