use super::error::SystemSetTimeError;
use chrono::*;

pub fn set_system_time_linux(date_time: DateTime<Utc>) -> Result<(), SystemSetTimeError> {
    let timeval = libc::timeval {
        tv_sec: date_time.timestamp(),
        tv_usec: date_time.timestamp_subsec_micros() as i64,
    };

    let res = unsafe { libc::settimeofday(&timeval, std::ptr::null_mut()) };

    if res == 0 {
        return Ok(())
    }
    if res == libc::EFAULT {
        return Err(SystemSetTimeError::PermissionError);
    }
    Err(SystemSetTimeError::OtherError(format!(
        "Error setting system time settimeofday returned {}", res)
    ))
}
