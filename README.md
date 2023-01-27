# gRPC Rust Demo

This is an example of creating [gRPC][grpc] services in [Rust][rust] using the
[Tonic][tonic] framework.

[grpc]:https://grpc.io
[rust]:https://www.rust-lang.org/
[tonic]:https://github.com/hyperium/tonic

## Background

In recent decades it's become common for communication between backend services
to employ [HTTP][http] [APIs][apis] with [JSON][json] payloads. Many HTTP APIs
adhere (or at least aspire) to [REST Principles][rest], though many fall into a
category we'll call "REST-like". Some "REST-like" APIs ultimately end up
operating like [Remote Procedure Call (RPC)][rpc] APIs in that they are less
concerned with [CRUD Operations][crud] and operate more as if they're calling
general procedures on the API endpoint.

RPC APIs can be a great alternative to "REST-like" APIs and operate as a set of
[functions/subroutines][subs] which can be called over a network. RPC APIs are
often more lightweight and performant than HTTP APIs, but can also be a little
more burdensome to set up initially. The [gRPC][grpc] project was introduced to
improve the set up and tooling experience for creating and maintaining RPC APIs,
providing a "batteries included" experience.

[gRPC][grpc] is a modern, [open source][oss] high performance RPC framework
which is a [Cloud Native Compute Foundation (CNCF)][cncf] project that operates
on top of [HTTP/2][http2] and provides automatic code generation. gRPC can be a
helpful solution for your projects to enable quick setup, lighter data transfers
and better maintenance costs. Utilizing [Protocol Buffers][protoc] to define
services with callable methods, code for servers _and_ clients can be
[automatically generated][codegen] in a number of programming languages which
reduces the time it takes to build API clients and to provide updates and
improvements to them over time.

The [Rust Programming Language][rust] is a great compliment to our gRPC story
so far as we're already thinking about performance. Rust provides [excellent
execution performance][rust-perf] as well as [state-of-the-art memory safety
guarantees][rust-memsafe] for developing your applications. Rust is _not yet_
considered one of the [core languages supported by the gRPC project][grpclang],
but in this walkthrough we will demonstrate how developers can get started
building gRPC services in Rust _today_ using the highly capable [Tonic][tonic]
framework. We will demonstrate setting up a new project, creating our services
with Protocol Buffers, generating client and server code and then ultimately
using the API to call methods over the network and stream data.

[http]:https://developer.mozilla.org/docs/Web/HTTP
[apis]:https://www.redhat.com/topics/api
[rest]:https://www.redhat.com/topics/api/what-is-a-rest-api
[json]:https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/JSON
[rpc]:https://wikipedia.org/wiki/Remote_procedure_call
[crud]:https://wikipedia.org/wiki/Create,_read,_update_and_delete
[subs]:https://wikipedia.org/wiki/Function_(computer_programming)
[grpc]:https://grpc.io
[oss]:https://www.redhat.com/topics/open-source/what-is-open-source-software
[cncf]:https://cncf.io/
[http2]:https://wikipedia.org/wiki/HTTP/2
[protoc]:https://wikipedia.org/wiki/Protocol_Buffers
[codegen]:https://wikipedia.org/wiki/Code_generation_%28compiler%29
[rust]:https://www.rust-lang.org/
[rust-perf]:https://en.wikipedia.org/wiki/Rust_(programming_language)#Performance
[rust-memsafe]:https://en.wikipedia.org/wiki/Rust_(programming_language)#Memory_safety
[grpclang]:https://grpc.io/docs/languages/
[tonic]:https://github.com/hyperium/tonic

# Demo

## Prerequisites

Ensure that you have [Rust installed on your system][rust-install] and select an
editor of your choice for creating the code.

> **Note**: This demo expects _some_ familiarity with Rust, though all the code
> is provided so in theory this could be helpful for newcomers. If you're brand
> new to Rust however, we would recommend checking out the excellent official
> [Learn Rust][rust-learn] documentation before continuing, in order to get your
> bearings.

> **Note**: Tonic requires Rust `v1.60.x`+. This demo was originally written
> using `v1.65.0`, so if you run into trouble with other releases, you might
> consider giving that specific version a try.

Install the [Protocol Buffers Compiler][protoc-install] for your system, as
this will be needed to generate our server and client code.

> **Note**: This demo was built and tested on an [Arch Linux][arch] system,
> but should work on any platform where [Tonic][tonic] and `protoc` are
> supported. If you end up having any trouble building Tonic on your system,
> check out the [getting help][tonic-help] documentation and reach out to the
> community.

[arch]:https://archlinux.org/
[tonic]:https://github.com/hyperium/tonic
[tonic-help]:https://github.com/hyperium/tonic#getting-help
[rust-install]:https://www.rust-lang.org/tools/install
[rust-learn]:https://www.rust-lang.org/learn
[protoc-install]:https://grpc.io/docs/protoc-installation/

## Step 1: Scaffolding

Choose a directory where you'll be adding code, and generate a new crate for
the code with

```
$ cargo new --bin demo
```

Next we'll define protobuf files which will generate client code for us.

## Step 2: Service Definition Protobuf

For this demo we'll be creating a service which is responsible for keeping
track of the inventory of a grocery store, with the ability to view and create
items as well as the ability to watch for changes in inventory (so that we can
try a streaming call as well as non-streaming calls).

We will build our grocery store service using [Protocol Buffers][protoc],
which are configured with `.proto` files wherein the services and types are
defined. Start by creating the `proto/` directory where our `.proto` files
will live:

```console
$ mkdir proto/
```

Then create `proto/store.proto`:

```proto
syntax = "proto3";
package store;
```

The above defines which version of protocol buffers we'll be using, and the
package name. Next we'll add our service:

```proto
service Inventory {
    // Add inserts a new Item into the inventory.
    rpc Add(Item) returns (InventoryChangeResponse);

    // Remove removes Items from the inventory.
    rpc Remove(ItemIdentifier) returns (InventoryChangeResponse);

    // Get retrieves Item information.
    rpc Get(ItemIdentifier) returns (Item);

    // UpdateQuantity increases or decreases the stock quantity of an Item.
    rpc UpdateQuantity(QuantityChangeRequest) returns (InventoryUpdateResponse);

    // UpdatePrice increases or decreases the price of an Item.
    rpc UpdatePrice(PriceChangeRequest) returns (InventoryUpdateResponse);

    // Watch streams Item updates from the inventory.
    rpc Watch(ItemIdentifier) returns (stream Item);
}
```

This service provides that calls we can use for a basic level of control over
our store's inventory, including creating and removing items, updating their
stock quantity/price and viewing or streaming item information. Next we'll
create the messages which are the types needed for these calls:

```proto
message ItemIdentifier {
    string sku = 2;
}

message ItemStock {
    float  price    = 1;
    uint32 quantity = 2;
}

message ItemInformation {
    optional string name        = 1;
    optional string description = 2;
}

message Item {
    ItemIdentifier           identifier  = 1;
    ItemStock                stock       = 2;
    optional ItemInformation information = 3;
}

message QuantityChangeRequest {
    string sku    = 1;
    int32  change = 2;
}

message PriceChangeRequest {
    string sku   = 1;
    float  price = 2;
}

message InventoryChangeResponse {
    string status = 1;
}

message InventoryUpdateResponse {
    string status   = 1;
    float price     = 2;
    uint32 quantity = 3;
}
```

Now we have the service and messages we need to ask the `protoc` compiler to
generate some of our server and our client code.

[protoc]:https://wikipedia.org/wiki/Protocol_Buffers

# Step 3: Compiling Protobuf

Now that we have our service, calls and messages all defined we should be able
to compile that into a Rust API server and client.

To do so, we'll need to add some dependencies on `tonic` and `prost` to handle
gRPC and protobufs. Update the `Cargo.toml` to include them:

```toml
[package]
name = "demo"
version = "0.1.0"
edition = "2021"
publish = false

[[bin]]
name = "server"
path = "src/main.rs"

[[bin]]
name = "cli"
path = "src/cli.rs"

[dependencies]
tonic = "0.8"
prost = "0.11"
tokio = { version = "1.24", features = ["macros", "rt-multi-thread"] }
tokio-stream = { version = "0.1", features = ["net"] }
futures = "0.3"
clap = { version = "4.1.4", features = ["derive"] }

[build-dependencies]
tonic-build = "0.8"

[dev-dependencies]
uuid = { version = "1.2.2", features = ["v4", "fast-rng"] }
futures-util = "0.3.25"
anyhow = "1"
```

Once the dependencies are updated, we'll need to add build tooling that will
hook the `cargo build` step to compile our `.proto` file during every build.
We can do that by creating `build.rs`:

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_file = "./proto/store.proto";

    tonic_build::configure()
        .protoc_arg("--experimental_allow_proto3_optional") // for older systems
        .build_client(true)
        .build_server(true)
        .out_dir("./src")
        .compile(&[proto_file], &["."])?;

    Ok(())
}
```

> **Note**: the `--experimental_allow_proto3_optional` argument isn't strictly
> necessary on newer systems with `protoc` version `3.21.x`+, but it wont hurt
> anything either. This is particularly helpful for users of Ubuntu LTS or other
> systems where the packaged `protoc` is significantly older.

We've indicated that we want both client and server built, and the output
directory for the generated code should be the `src/` directory. Now we should
be able to run:

```console
$ cargo build
```

There should now be a `src/store.rs` created for us with our client and server
code conveniently generated.

## Step 4: Implementing The Server

Now that we've generated the code for our service, we'll need to add our
implementation of the server methods for the client to call.

Start by creating a new `src/server.rs` file and we'll begin with the imports
we'll need:

```rust
use futures::Stream;
use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tonic::{Request, Response, Status};

use crate::store::inventory_server::Inventory;
use crate::store::{
    InventoryChangeResponse, InventoryUpdateResponse, Item, ItemIdentifier, PriceChangeRequest,
    QuantityChangeRequest,
};
```

We'll also provide some helpful error messages for a variety of failure
conditions which our API can reach related to inventory management:

```rust
const BAD_PRICE_ERR: &str = "provided PRICE was invalid";
const DUP_PRICE_ERR: &str = "item is already at this price";
const DUP_ITEM_ERR: &str = "item already exists in inventory";
const EMPTY_QUANT_ERR: &str = "invalid quantity of 0 provided";
const EMPTY_SKU_ERR: &str = "provided SKU was empty";
const NO_ID_ERR: &str = "no ID or SKU provided for item";
const NO_ITEM_ERR: &str = "the item requested was not found";
const NO_STOCK_ERR: &str = "no stock provided for item";
const UNSUFF_INV_ERR: &str = "not enough inventory for quantity change";
```

Next up we're going to implement the `Inventory` trait which was generated for
us from the `proto/store.proto` file in the last step. For each of the methods
we added to our `Inventory` service, we'll write our own implementation.

We'll create a `StoreInventory` object to implement our inventory service:

```rust
#[derive(Debug)]
pub struct StoreInventory {
    inventory: Arc<Mutex<HashMap<String, Item>>>,
}

impl Default for StoreInventory {
    fn default() -> Self {
        StoreInventory {
            inventory: Arc::new(Mutex::new(HashMap::<String, Item>::new())),
        }
    }
}

#[tonic::async_trait]
impl Inventory for StoreInventory {}
```

> **Note**: you may notice we're using lots of [async Rust][rust-async]
> terminology, and imports from the [Tokio Runtime][tokio]. If you're newer to
> Rust, and the use of these things are a bit confusing, don't worry! There's
> some great material out there to get you caught up on async Rust: check out
> the [Rust Async Book][rust-async] and the [Tokio Runtime Tutorial][tokio-tut]
> first and get your bearings.

Our `StoreInventory` will have an `inventory` field which contains a threadsafe
hashmap, which will be the in-memory storage for our inventory system. We
implement the `Default` trait for convenience, and then we provide the
`impl Inventory` block. Now we can start adding our method implementations for
`add`, `remove`, `update_price` and so forth, so the following code blocks
should be placed nested inside that `impl Inventory for StoreInventory` block.

Let's start with adding the `add` method:

```rust
    async fn add(
        &self,
        request: Request<Item>,
    ) -> Result<Response<InventoryChangeResponse>, Status> {
        let item = request.into_inner();

        // validate SKU, verify that it's present and not empty
        let sku = match item.identifier.as_ref() {
            Some(id) if id.sku == "" => return Err(Status::invalid_argument(EMPTY_SKU_ERR)),
            Some(id) => id.sku.to_owned(),
            None => return Err(Status::invalid_argument(NO_ID_ERR)),
        };

        // validate stock, verify its present and price is not negative or $0.00
        match item.stock.as_ref() {
            Some(stock) if stock.price <= 0.00 => {
                return Err(Status::invalid_argument(BAD_PRICE_ERR))
            }
            Some(_) => {}
            None => return Err(Status::invalid_argument(NO_STOCK_ERR)),
        };

        // if the item is already present don't allow the duplicate
        let mut map = self.inventory.lock().await;
        if let Some(_) = map.get(&sku) {
            return Err(Status::already_exists(DUP_ITEM_ERR));
        }

        // add the item to the inventory
        map.insert(sku.into(), item);

        Ok(Response::new(InventoryChangeResponse {
            status: "success".into(),
        }))
    }
```

In the above you will find that a `Request<Item>` is provided (from our client
when called) which includes the entire item that we need to store in the
inventory. Some validation is performed to ensure data-integrity, we lock the
`Mutex` on our `HashMap` to ensure thread safety and integrity and then
ultimately the item is stored by SKU into the `HashMap`.

We'll add the `remove` counterpart as well, which is more simple:

```rust
    async fn remove(
        &self,
        request: Request<ItemIdentifier>,
    ) -> Result<Response<InventoryChangeResponse>, Status> {
        let identifier = request.into_inner();

        // don't allow empty SKU
        if identifier.sku == "" {
            return Err(Status::invalid_argument(EMPTY_SKU_ERR));
        }

        // remove the item (if present)
        let mut map = self.inventory.lock().await;
        let msg = match map.remove(&identifier.sku) {
            Some(_) => "success: item was removed",
            None => "success: item didn't exist",
        };

        Ok(Response::new(InventoryChangeResponse {
            status: msg.into(),
        }))
    }
```

> **Note**: this method returns success for removal of items that didn't exist,
> but informs the user of that circumstance.

Now that items can be added and removed, they also need to be retrieve-able,
let's add our `get` implementation:

```rust
    async fn get(&self, request: Request<ItemIdentifier>) -> Result<Response<Item>, Status> {
        let identifier = request.into_inner();

        // don't allow empty SKU
        if identifier.sku == "" {
            return Err(Status::invalid_argument(EMPTY_SKU_ERR));
        }

        // retrieve the item if it exists
        let map = self.inventory.lock().await;
        let item = match map.get(&identifier.sku) {
            Some(item) => item,
            None => return Err(Status::not_found(NO_ITEM_ERR)),
        };

        Ok(Response::new(item.clone()))
    }
```

The `get` implementation is small and simple, validating input and returning
the inventory `Item` if present.

We can add and retrieve, but we also need to be able to update in place. Let's
add our `update_quantity` implementation:

```rust
    async fn update_quantity(
        &self,
        request: Request<QuantityChangeRequest>,
    ) -> Result<Response<InventoryUpdateResponse>, Status> {
        let change = request.into_inner();

        // don't allow empty SKU
        if change.sku == "" {
            return Err(Status::invalid_argument(EMPTY_SKU_ERR));
        }

        // quantity changes with no actual change don't make sense, inform user
        if change.change == 0 {
            return Err(Status::invalid_argument(EMPTY_QUANT_ERR));
        }

        // retrieve the current inventory item data
        let mut map = self.inventory.lock().await;
        let item = match map.get_mut(&change.sku) {
            Some(item) => item,
            None => return Err(Status::not_found(NO_ITEM_ERR)),
        };

        // retrieve the stock mutable so we can update the quantity
        let mut stock = match item.stock.borrow_mut() {
            Some(stock) => stock,
            None => return Err(Status::internal(NO_STOCK_ERR)),
        };

        // validate and then handle the quantity change
        stock.quantity = match change.change {
            // handle negative numbers as stock reduction
            change if change < 0 => {
                if change.abs() as u32 > stock.quantity {
                    return Err(Status::resource_exhausted(UNSUFF_INV_ERR));
                }
                stock.quantity - change.abs() as u32
            }
            // handle positive numbers as stock increases
            change => stock.quantity + change as u32,
        };

        Ok(Response::new(InventoryUpdateResponse {
            status: "success".into(),
            price: stock.price,
            quantity: stock.quantity,
        }))
    }
```

Again we provide some validation, and enable the two ways the caller can update
the quantity: positive or negative changes. Ultimately the validated change is
updated in place in memory for subsequent calls.

The `update_price` method will be similar:

```rust
    async fn update_price(
        &self,
        request: Request<PriceChangeRequest>,
    ) -> Result<Response<InventoryUpdateResponse>, Status> {
        let change = request.into_inner();

        // don't allow empty SKU
        if change.sku == "" {
            return Err(Status::invalid_argument(EMPTY_SKU_ERR));
        }

        // $0.00 disallowed and negatives don't make sense, inform the user
        if change.price <= 0.0 {
            return Err(Status::invalid_argument(BAD_PRICE_ERR));
        }

        // retrieve the current inventory item data
        let mut map = self.inventory.lock().await;
        let item = match map.get_mut(&change.sku) {
            Some(item) => item,
            None => return Err(Status::not_found(NO_ITEM_ERR)),
        };

        // retrieve the stock mutable so we can update the quantity
        let mut stock = match item.stock.borrow_mut() {
            Some(stock) => stock,
            None => return Err(Status::internal(NO_STOCK_ERR)),
        };

        // let the client know if they requested to change the price to the
        // price that is already currently set
        if stock.price == change.price {
            return Err(Status::invalid_argument(DUP_PRICE_ERR));
        }

        // update the item unit price
        stock.price = change.price;

        Ok(Response::new(InventoryUpdateResponse {
            status: "success".into(),
            price: stock.price,
            quantity: stock.quantity,
        }))
    }
```

The main differences in `update_price` from `update_quantity` are the
validation rules about price: `$0.00` priced items are not allowed, and we
guard against negative prices.

Now as we add our `watch` implementation things will get a little bit more
interesting, as we have to provide the mechanism to stream updates back out to
the client. First we'll define a return type for our stream which will utilize
the `Stream` type from [Rust's futures library][rust-futures]:

```rust
    type WatchStream = Pin<Box<dyn Stream<Item = Result<Item, Status>> + Send>>;
```

Our streams will consist of a `Result<Item, Status>` where each update the
client receives will either contain a new copy of the `Item` which has changed,
or a `Status` indicating any problems that were encountered (and corresponding
with the error messages we placed in constants in a previous step).

With that we can define our `watch` implementation:

```rust
    async fn watch(
        &self,
        request: Request<ItemIdentifier>,
    ) -> Result<Response<Self::WatchStream>, Status> {
        // retrieve the relevant item and get a baseline
        let id = request.into_inner();
        let mut item = self.get(Request::new(id.clone())).await?.into_inner();

        // the channel will be our stream back to the client, we'll send copies
        // of the requested item any time we notice a change to it in the
        // inventory.
        let (tx, rx) = mpsc::unbounded_channel();

        // we'll loop and poll new copies of the item until either the client
        // closes the connection, or an error occurs.
        let inventory = self.inventory.clone();
        tokio::spawn(async move {
            loop {
                // it's somewhat basic, but for this demo we'll just check the
                // item every second for any changes.
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;

                // pull a fresh copy of the item in the inventory
                let map = inventory.lock().await;
                let item_refresh = match map.get(&id.sku) {
                    Some(item) => item,
                    // the item has been removed from the inventory. Let the
                    // client know, and stop the stream.
                    None => {
                        if let Err(err) = tx.send(Err(Status::not_found(NO_ITEM_ERR))) {
                            println!("ERROR: failed to update stream client: {:?}", err);
                        }
                        return;
                    }
                };

                // check to see if the item has changed since we last saw it,
                // and if it has inform the client via the stream.
                if item_refresh != &item {
                    if let Err(err) = tx.send(Ok(item_refresh.clone())) {
                        println!("ERROR: failed to update stream client: {:?}", err);
                        return;
                    }
                }

                // cache the most recent copy of the item
                item = item_refresh.clone()
            }
        });

        let stream = UnboundedReceiverStream::new(rx);
        Ok(Response::new(Box::pin(stream) as Self::WatchStream))
    }
```

> **Note**: keep in mind that all code is just for demonstration purposes, you
> would not necessarily want to, for instance, use unbounded channels in your
> production applications.

The comments throughout should hopefully provide a good walkthrough of how
everything works, but these are the high level steps:

1. validate the input
2. create a `Channel` which we will stream `Item` data into
3. use `tokio::spawn` to spawn a new asynchronous task in the background which
   will continue to update our client with changes to the subscribed `Item`
   until the client closes the connection, or an error occurs
4. send the `rx` portion of the `Channel` back wrapped as our `WatchStream`
   type we defined in the previous step

With that we can `add`, `remove`, `get`, `update` and `watch` items in our
inventory! We need a mechansim to _start_ this server we just created, so let's
add that to our `src/main.rs`, making the file look like this:

```rust
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
```

In the next steps we'll move on to client code so that we can see our server
in action.

[rust-async]:https://rust-lang.github.io/async-book/
[tokio]:https://tokio.rs
[tokio-tut]:https://tokio.rs/tokio/tutorial
[rust-futures]:https://github.com/rust-lang/futures-rs

## Step 5: Implementing The Client

The server is up and running, now we need to be able to use the generated API
client to view and manage our inventory. For this we will make a command-line
tool which can be used to manage the inventory using the gRPC API.

We'll use [Clap][rust-clap], which is a popular command-line toolkit for Rust
and create our CLI. Create the file `src/cli.rs` and add the required imports:

```rust
pub mod store;

use clap::Parser;
use futures::StreamExt;

use store::inventory_client::InventoryClient;
use store::{
    Item, ItemIdentifier, ItemInformation, ItemStock, PriceChangeRequest, QuantityChangeRequest,
};
```

So we've imported the `Parser` from Clap so we can construct our CLI using
structs with Clap attributes and we've imported the `InventoryClient` and some
of our other relevant types from our API, now let's add our commands:

```rust
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
```

The above instructs Clap to provide the entries in the `Command` enum as
sub-commands so that we'll be able to run `demo add`, `demo remove`, and so
forth. We'll need to add the options and the implementation for each of these,
so let's get started with `add`:

```rust
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
    let mut client = InventoryClient::connect("http://127.0.0.1:8080").await?;

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
```

The `AddOptions` struct enables us to provide all the required data to add an
item to the inventory, and includes helpful options like the ability to provide
defaults and `Options` can be used for optional parameters. With this we'll be
able to run things like `demo add --sku 87A7669F --price 1.99` to add a new
`Item` to the inventory.

Next up we'll handle `remove`, which is fairly brief:

```rust
#[derive(Debug, Parser)]
struct RemoveOptions {
    #[clap(long)]
    sku: String,
}

async fn remove(opts: RemoveOptions) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = InventoryClient::connect("http://127.0.0.1:8080").await?;

    let request = tonic::Request::new(ItemIdentifier { sku: opts.sku });
    let response = client.remove(request).await?;
    let msg = response.into_inner().status;
    assert!(msg.starts_with("success"));
    println!("{}", msg);

    Ok(())
}
```

The `get` functions's implementation is small as well:

```rust
#[derive(Debug, Parser)]
struct GetOptions {
    #[clap(long)]
    sku: String,
}

async fn get(opts: GetOptions) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = InventoryClient::connect("http://127.0.0.1:8080").await?;

    let request = tonic::Request::new(ItemIdentifier { sku: opts.sku });
    let item = client.get(request).await?.into_inner();
    println!("found item: {:?}", item);

    Ok(())
}
```

Now for the update functions, which will somewhat resemble one-another, starting
with `update_quantity`:

```rust
#[derive(Debug, Parser)]
struct UpdateQuantityOptions {
    #[clap(long)]
    sku: String,
    #[clap(allow_hyphen_values = true, long)]
    change: i32,
}

async fn update_quantity(opts: UpdateQuantityOptions) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = InventoryClient::connect("http://127.0.0.1:8080").await?;

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
```

```rust
#[derive(Debug, Parser)]
struct UpdatePriceOptions {
    #[clap(long)]
    sku: String,
    #[clap(long)]
    price: f32,
}

async fn update_price(opts: UpdatePriceOptions) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = InventoryClient::connect("http://127.0.0.1:8080").await?;

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
```

Our `watch` command is surprisingly simple, given that the implementation on
the server side was fairly involved, all we need to do is receive the `Stream`
from the `watch` request on the server, and iterate through it with `.next()`:

```rust
async fn watch(opts: GetOptions) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = InventoryClient::connect("http://127.0.0.1:8080").await?;

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
```

Also you'll see that we didn't bother making a specific option struct for watch
as the previously created `GetOptions` type is perfectly sufficient.

With all our API methods covered, now we just need to tie it all together with
Clap and add our `main` function:

```rust
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
```

Now we're ready to test everything out!

[rust-clap]:https://github.com/clap-rs/clap

## Step 6: Trying It Out

Everything's in place and now it's time to see our work in action!

We're going to run our server in the background, and then try a variety of
cli commands against it.

Start by creating a new separate terminal which we'll dedicate to the server
and run:

```console
$ cargo run --release --bin server
```

Then in another terminal in the work directory, let's compile the CLI and make
a copy:

```console
$ cargo build --release --bin cli
$ cp target/release/cli ./
```

Now we should be ready to start making commands.

Let's start by adding a new `Item` to the inventory:

```console
$ ./cli add --sku TESTSKU --price 1.99 --quantity 20 --name bananas --description "yellow fruit"
success: item was added to the inventory.
```

Retrieve the item to see it's contents:

```console
$ ./cli get --sku TESTSKU
found item: { sku: "TESTSKU" }, stock: { price: 1.99, quantity: 0 }, information: { name: "bananas", description: "yellow fruit" }
```

Great, and let's run the exact same thing another time, to verify that our
validation code rejects the duplicate:

```console
$ ./cli add --sku TESTSKU --price 2.99
Error: Status { code: AlreadyExists, message: "item already exists in inventory" }
```

Then we can change the quantity, as if some of the inventory had been purchased:

```console
$ ./cli update-quantity --sku TESTSKU --change -17
success: quantity was updated. Quantity: 3 Price: 1.99
```

Then update the price:

```console
$ ./cli update-price --sku TESTSKU --price 2.19
success: price was updated. Quantity: 3 Price: 2.19
```

Then we can `watch` the item as we change the inventory, in a new terminal
dedicated to running `watch`:

```console
$ ./cli watch --sku TESTSKU
streaming changes to item TESTSKU
```

Then back in our previous terminal, make several changes and even remove
the item entirely:

```console
$ ./cli update-quantity --sku TESTSKU --change +50
success: quantity was updated. Quantity: 53 Price: 2.19
$ ./cli update-price --sku TESTSKU --price 1.99
success: price was updated. Quantity: 53 Price: 1.99
$ ./cli remove --sku TESTSKU
success: item was removed
```

Over in the `watch` terminal, you should have seen a stream of all the actions:

```console
$ ./cli watch --sku TESTSKU
streaming changes to item TESTSKU
item was updated: Item { identifier: Some(ItemIdentifier { sku: "TESTSKU" }), stock: Some(ItemStock { price: 2.19, quantity: 53 }), information: Some(ItemInformation { name: Some("bananas"), description: Some("yellow fruit") }) }
item was updated: Item { identifier: Some(ItemIdentifier { sku: "TESTSKU" }), stock: Some(ItemStock { price: 1.99, quantity: 53 }), information: Some(ItemInformation { name: Some("bananas"), description: Some("yellow fruit") }) }
watched item has been removed from the inventory.
stream closed
```

We've accomplished what we set out to do, we have an API with a streaming
endpoint, and a CLI which excercises it. If you want to play around with it
more, Clap automatically generates `--help` information:

```console
$ ./cli --help
Usage: cli <COMMAND>

Commands:
  add
  remove
  get
  update-quantity
  update-price
  watch
  help             Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

At this point you should have a solid basic understanding of how to set up and
test gRPC services written in [Rust][rust] with the [Tonic][tonic] framework.

[rust]:https://www.rust-lang.org/
[tonic]:https://github.com/hyperium/tonic


## Next Steps

I hope you enjoyed this demo. If you're interested in doing more with it
the code provided here was a light touch for the purposes of demonstration
brevity, but there are certainly some follow-up tasks you could do if you like.

All the code is [available for you on Github][repo] and if you find ways you'd
like to improve it feel free to send in a pull request.

Some tasks that were not covered in this demo for time reasons were:

- adding [TLS and auth][grpc-auth] to the client and server to protect data
- add command line flags for the cli and server, enable changing `host:port`
- improve the appearance of the CLI output for better human readability

And certainly many more tasks. Or you can start your own project with what you
learned here, either way happy coding!

[repo]:https://github.com/shaneutt/rust-grpc-demo
[auth]:https://grpc.io/docs/guides/auth/

# License

This demo is distributed under the following licenses:

- this `README.md` file is licensed under the terms of the [Creative Commons CC-BY-SA v4.0][cc] license.
- all other files are licensed under the terms of the [MIT License][mit] license.

[cc]:https://github.com/shaneutt/rust-grpc-demo/blob/main/LICENSE-CC-BY-SA
[mit]:https://github.com/shaneutt/rust-grpc-demo/blob/main/LICENSE-MIT
