use std::fmt;

#[cfg(target_os = "windows")]
mod windows;
mod error;
mod linux;

pub use error::SystemSetTimeError as SystemSetTimeError;

/// Sets system time (on Windows with ms precision)
pub fn set_system_time(date_time: chrono::DateTime<chrono::Utc>) -> Result<(), SystemSetTimeError> {
    #[cfg(target_os = "windows")]
    return windows::set_system_time_windows(date_time);

    Err(SystemSetTimeError::OperatingSystemNotSupported)
}
