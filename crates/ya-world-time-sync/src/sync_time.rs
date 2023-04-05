use std::process::Command;

use chrono::{DateTime, Datelike, Local, Timelike};

/// Synchronize system time with the platform specific
/// command line tool
pub fn sync_time(time: DateTime<Local>) {
    let cmd = Command::new("cmd")
        .args(&[
            "/C",
            format!(
                "powershell Set-Date -Date \"{:02}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}\"",
                time.year(),
                time.month(),
                time.day(),
                time.hour(),
                time.minute(),
                time.second(),
                time.nanosecond() / 1000000
            )
            .as_str(),
        ])
        .spawn();

    match cmd {
        Ok(mut child) => {
            child
                .wait()
                .expect("Time synchronization finished incorrectly");
        }
        Err(e) => {
            eprintln!("Error occurred: {}", e.to_string());
        }
    };
}
