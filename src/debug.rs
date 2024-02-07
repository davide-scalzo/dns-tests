use std::time::Duration;

use my_lib;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let handle = my_lib::fetch_and_query_futures();
    handle.await?;

    // sleep(Duration::from_secs(2)).await;

    // let handle = my_lib::fetch_and_query();
    // handle.await?;

    Ok(())
}
