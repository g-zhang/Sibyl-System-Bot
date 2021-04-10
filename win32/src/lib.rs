use std::ffi::CStr;
use std::os::raw::c_char;
use tracing::error;

#[no_mangle]
pub extern "C" fn log_error_in_rust(err_string: *const c_char) {
    let c_str: &CStr = unsafe { CStr::from_ptr(err_string) };
    let str_slice: &str = c_str.to_str().unwrap();
    let mut strclean = str_slice.split('\r');
    error!("{}", strclean.next().unwrap());
}

#[cfg(target_os = "windows")]
extern "C" {
    fn Win32_EnableTerminalAnsiSupport() -> bool;
    fn Win32_EnableMitigations() -> bool;
}

pub fn enable_ansi_support() {
    #[cfg(target_os = "windows")]
    unsafe {
        if !Win32_EnableTerminalAnsiSupport() {
            panic!("Failed to enable ANSI support.");
        }
    }
}

pub fn enable_mitigations() {
    #[cfg(target_os = "windows")]
    unsafe {
        if !Win32_EnableMitigations() {
            panic!("Failed to enable mitigations.");
        }
    }
}
