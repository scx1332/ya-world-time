extern crate core;

mod system_time;

use crate::system_time::{set_system_time};
use chrono::{Datelike, Timelike, Utc};
use sntpc::utils::update_system_time;
use std::env;
use std::ops::Sub;
use ya_world_time::world_time::world_time;


#[tokio::main(flavor = "current_thread")]
async fn main() -> std::io::Result<()> {
    env::set_var(
        "RUST_LOG",
        env::var("RUST_LOG").unwrap_or("info".to_string()),
    );
    env_logger::init();

    ya_world_time::world_time::init_world_time();
    let current_time = world_time().utc_time();

    let res = set_system_time(current_time);
    if let Err(err) = res {
        log::error!("Error occurred when settings system time: {}", err);
    }
    log::info!("Current time: {}", world_time().utc_time());
    Ok(())
}
