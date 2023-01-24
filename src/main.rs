use server::StoreInventory;
use store::inventory_server::InventoryServer;
use tonic::transport::Server;

pub mod server;
pub mod store;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:8080".parse()?;
    let inventory = StoreInventory::default();

    Server::builder()
        .add_service(InventoryServer::new(inventory))
        .serve(addr)
        .await?;

    Ok(())
}
