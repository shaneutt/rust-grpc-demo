use tonic::transport::Server;

use server::StoreInventory;
use store::inventory_server::InventoryServer;

pub mod server;
pub mod store;

mod store_proto {
    include!("store.rs");

    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("store_descriptor");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:9001".parse()?;
    let inventory = StoreInventory::default();

    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(store_proto::FILE_DESCRIPTOR_SET)
        .build()
        .unwrap();
    
    Server::builder()
        .add_service(InventoryServer::new(inventory))
        .add_service(reflection_service)
        .serve(addr)
        .await?;
    Ok(())
}
