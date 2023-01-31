pub mod store;

use clap::Parser;
use futures::StreamExt;

use store::inventory_client::InventoryClient;
use store::{
    Item, ItemIdentifier, ItemInformation, ItemStock, PriceChangeRequest, QuantityChangeRequest,
};

// -----------------------------------------------------------------------------
// Base Command
// -----------------------------------------------------------------------------

#[derive(Debug, Parser)]
struct Options {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Parser)]
enum Command {
    Add(AddOptions),
    Remove(RemoveOptions),
    Get(GetOptions),
    UpdateQuantity(UpdateQuantityOptions),
    UpdatePrice(UpdatePriceOptions),
    Watch(GetOptions),
}

// -----------------------------------------------------------------------------
// Add Command
// -----------------------------------------------------------------------------

#[derive(Debug, Parser)]
struct AddOptions {
    #[clap(long)]
    sku: String,
    #[clap(long)]
    price: f32,
    #[clap(default_value = "0", long)]
    quantity: u32,
    #[clap(long)]
    name: Option<String>,
    #[clap(long)]
    description: Option<String>,
}

async fn add(opts: AddOptions) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = InventoryClient::connect("http://127.0.0.1:9001").await?;

    let id = ItemIdentifier { sku: opts.sku };

    let stock = ItemStock {
        price: opts.price,
        quantity: opts.quantity,
    };

    let info = ItemInformation {
        name: opts.name,
        description: opts.description,
    };

    let item = Item {
        identifier: Some(id),
        stock: Some(stock),
        information: Some(info),
    };

    let request = tonic::Request::new(item);
    let response = client.add(request).await?;
    assert_eq!(response.into_inner().status, "success");
    println!("success: item was added to the inventory.");

    Ok(())
}

// -----------------------------------------------------------------------------
// Remove Command
// -----------------------------------------------------------------------------

#[derive(Debug, Parser)]
struct RemoveOptions {
    #[clap(long)]
    sku: String,
}

async fn remove(opts: RemoveOptions) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = InventoryClient::connect("http://127.0.0.1:9001").await?;

    let request = tonic::Request::new(ItemIdentifier { sku: opts.sku });
    let response = client.remove(request).await?;
    let msg = response.into_inner().status;
    assert!(msg.starts_with("success"));
    println!("{}", msg);

    Ok(())
}

// -----------------------------------------------------------------------------
// Get Command
// -----------------------------------------------------------------------------

#[derive(Debug, Parser)]
struct GetOptions {
    #[clap(long)]
    sku: String,
}

async fn get(opts: GetOptions) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = InventoryClient::connect("http://127.0.0.1:9001").await?;

    let request = tonic::Request::new(ItemIdentifier { sku: opts.sku });
    let item = client.get(request).await?.into_inner();
    println!("found item: {:?}", item);

    Ok(())
}

// -----------------------------------------------------------------------------
// UpdateQuantity Command
// -----------------------------------------------------------------------------

#[derive(Debug, Parser)]
struct UpdateQuantityOptions {
    #[clap(long)]
    sku: String,
    #[clap(allow_hyphen_values = true, long)]
    change: i32,
}

async fn update_quantity(opts: UpdateQuantityOptions) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = InventoryClient::connect("http://127.0.0.1:9001").await?;

    let request = tonic::Request::new(QuantityChangeRequest {
        sku: opts.sku,
        change: opts.change,
    });

    let message = client.update_quantity(request).await?.into_inner();
    assert_eq!(message.status, "success");
    println!(
        "success: quantity was updated. Quantity: {} Price: {}",
        message.quantity, message.price
    );

    Ok(())
}

// -----------------------------------------------------------------------------
// UpdatePrice Command
// -----------------------------------------------------------------------------

#[derive(Debug, Parser)]
struct UpdatePriceOptions {
    #[clap(long)]
    sku: String,
    #[clap(long)]
    price: f32,
}

async fn update_price(opts: UpdatePriceOptions) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = InventoryClient::connect("http://127.0.0.1:9001").await?;

    let request = tonic::Request::new(PriceChangeRequest {
        sku: opts.sku,
        price: opts.price,
    });

    let message = client.update_price(request).await?.into_inner();
    assert_eq!(message.status, "success");
    println!(
        "success: price was updated. Quantity: {} Price: {}",
        message.quantity, message.price
    );

    Ok(())
}

// -----------------------------------------------------------------------------
// Watch Command
// -----------------------------------------------------------------------------

async fn watch(opts: GetOptions) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = InventoryClient::connect("http://127.0.0.1:9001").await?;

    let mut stream = client
        .watch(ItemIdentifier {
            sku: opts.sku.clone(),
        })
        .await?
        .into_inner();

    println!("streaming changes to item {}", opts.sku);
    while let Some(item) = stream.next().await {
        match item {
            Ok(item) => println!("item was updated: {:?}", item),
            Err(err) => {
                if err.code() == tonic::Code::NotFound {
                    println!("watched item has been removed from the inventory.");
                    break;
                } else {
                    return Err(err.into());
                }
            }
        };
    }
    println!("stream closed");

    Ok(())
}

// -----------------------------------------------------------------------------
// Main
// -----------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts = Options::parse();

    use Command::*;
    match opts.command {
        Add(opts) => add(opts).await?,
        Remove(opts) => remove(opts).await?,
        Get(opts) => get(opts).await?,
        UpdateQuantity(opts) => update_quantity(opts).await?,
        UpdatePrice(opts) => update_price(opts).await?,
        Watch(opts) => watch(opts).await?,
    };

    Ok(())
}
