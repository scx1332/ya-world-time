mod error;
#[cfg(all(unix))]
mod linux;
#[cfg(target_os = "windows")]
mod windows;

pub use error::SystemSetTimeError;

/// Sets system time (on Windows with ms precision)
pub fn set_system_time(date_time: chrono::DateTime<chrono::Utc>) -> Result<(), SystemSetTimeError> {
    #[cfg(target_os = "windows")]
    return windows::set_system_time_windows(date_time);
    #[cfg(all(unix))]
    return linux::set_system_time_linux(date_time);

    #[allow(unreachable_code)]
    Err(SystemSetTimeError::OperatingSystemNotSupported)
}
