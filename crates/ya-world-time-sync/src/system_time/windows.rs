use std::ops::Sub;
use chrono::*;
use winapi::shared::minwindef::WORD;
use super::error::SystemSetTimeError;

pub fn set_system_time_windows(date_time: DateTime<Utc>) -> Result<(), SystemSetTimeError> {
    //fix rounding to millis
    date_time.sub(Duration::microseconds(500));
    let s = winapi::um::minwinbase::SYSTEMTIME {
        wYear: date_time.year() as WORD,
        wMonth: date_time.month() as WORD,
        wDayOfWeek: date_time.day0() as WORD,
        wDay: date_time.day() as WORD,
        wHour: date_time.hour() as WORD,
        wMinute: date_time.minute() as WORD,
        wSecond: date_time.second() as WORD,
        wMilliseconds: (date_time.nanosecond() / 1000000) as WORD,
    };
    let res = unsafe { winapi::um::sysinfoapi::SetSystemTime(&s) };
    if res == 0 {
        Err(SystemSetTimeError::OperatingSystemNotSupported)
    } else {
        Ok(())
    }
}
