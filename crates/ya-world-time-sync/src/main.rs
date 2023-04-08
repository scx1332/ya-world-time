extern crate core;

mod system_time;

use crate::system_time::set_system_time;

use std::env;

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
    let world_timer = world_time();
    if let Err(err) = res {
        log::error!("Error occurred when settings system time: {}", err);
    } else {
        log::info!("System time set. Current time: {}", world_timer.utc_time());
    }
    if let Some(precision) = world_timer.precision {
        println!("{},{},{}", world_timer.utc_time().format("%Y-%m-%d %H:%M:%S"), world_timer.offset, precision);
    }
    Ok(())
}
