mod sync_time;

use crate::sync_time::sync_time;
use sntpc::utils::update_system_time;
use std::env;
use std::ops::Sub;
use chrono::{Datelike, Timelike, Utc};
use winapi::shared::minwindef::{BOOL, FALSE, WORD};
use ya_world_time::world_time::world_time;

fn set_system_time_windows(date_time: chrono::DateTime<Utc>){
    //fix rounding to millis
    date_time.sub(chrono::Duration::microseconds(500));
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
    let res = unsafe {
        winapi::um::sysinfoapi::SetSystemTime(&s)
    };
    if res == 0 {
        panic!("Set system time failed.");
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::io::Result<()> {
    env::set_var(
        "RUST_LOG",
        env::var("RUST_LOG").unwrap_or("info".to_string()),
    );
    env_logger::init();

    ya_world_time::world_time::init_world_time();
    let current_time = world_time().utc_time();

    set_system_time_windows(current_time);
    log::info!("Current time: {}", world_time().utc_time());
    Ok(())
}
