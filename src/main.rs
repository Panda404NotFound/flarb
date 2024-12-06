// main.rs
mod macros;
mod config;
mod jupiter;
mod test_param;

use log::info;
use jupiter::quote::test_valid_pools;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    config::initialize_http_client();

    env_logger::init();
    info!("Testing Jupiter Pools...");
    test_valid_pools().await?;
    Ok(())

}