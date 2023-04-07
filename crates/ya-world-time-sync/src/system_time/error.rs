use std::fmt;

#[derive(Debug, Clone)]
pub enum SystemSetTimeError {
    OperatingSystemNotSupported,
    PermissionError,
    OtherError(String),
}

impl fmt::Display for SystemSetTimeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error setting system time, probably no permission for setting time")
    }
}
