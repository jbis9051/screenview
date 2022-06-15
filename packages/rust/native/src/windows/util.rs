use crate::windows::FALSE;
use windows::Win32::System::SystemInformation::{GetVersionExA, OSVERSIONINFOA};

enum WindowsMajorVersions {
    kWindows2000 = 5,
    kWindowsVista = 6,
    kWindows10 = 10,
}

//bool GetOsVersion(int* major, int* minor, int* build) {
//   OSVERSIONINFO info = {0};
//   info.dwOSVersionInfoSize = sizeof(info);
//   if (GetVersionEx(&info)) {
//     if (major)
//       *major = info.dwMajorVersion;
//     if (minor)
//       *minor = info.dwMinorVersion;
//     if (build)
//       *build = info.dwBuildNumber;
//     return true;
//   }
//   return false;
// }

fn get_os_version() -> Result<(u32, u32, u32), ()> {
    let mut info = OSVERSIONINFOA::default();
    info.dwOSVersionInfoSize = std::mem::size_of::<OSVERSIONINFOA>() as u32;
    let res = unsafe { GetVersionExA(&mut info) };
    if res == FALSE {
        return Err(());
    }
    Ok((info.dwMajorVersion, info.dwMinorVersion, info.dwBuildNumber))
}

// inline bool is_windows8or_later() {
//   int major, minor;
//   return (GetOsVersion(&major, &minor, nullptr) &&
//           (major > kWindowsVista || (major == kWindowsVista && minor >= 2)));
// }

pub fn is_windows8or_later() -> bool {
    match get_os_version() {
        Ok((major, minor, _)) =>
            (major > WindowsMajorVersions::kWindowsVista as u32)
                || ((major == WindowsMajorVersions::kWindowsVista as u32) && (minor >= 2)),
        Err(_) => false,
    }
}
