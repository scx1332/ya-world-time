use std::env;

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::io::Result<()> {
    env::set_var("RUST_LOG", env::var("RUST_LOG").unwrap_or("info".to_string()));
    env_logger::init();

    ya_world_time::world_time::init_world_time();
    Ok(())
}