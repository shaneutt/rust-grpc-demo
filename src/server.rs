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

// -----------------------------------------------------------------------------
// Error Messages
// -----------------------------------------------------------------------------

const BAD_PRICE_ERR: &str = "provided PRICE was invalid";
const DUP_PRICE_ERR: &str = "item is already at this price";
const DUP_ITEM_ERR: &str = "item already exists in inventory";
const EMPTY_QUANT_ERR: &str = "invalid quantity of 0 provided";
const EMPTY_SKU_ERR: &str = "provided SKU was empty";
const NO_ID_ERR: &str = "no ID or SKU provided for item";
const NO_ITEM_ERR: &str = "the item requested was not found";
const NO_STOCK_ERR: &str = "no stock provided for item";
const UNSUFF_INV_ERR: &str = "not enough inventory for quantity change";

// -----------------------------------------------------------------------------
// InventoryServer Implementation
// -----------------------------------------------------------------------------

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
impl Inventory for StoreInventory {
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

    type WatchStream = Pin<Box<dyn Stream<Item = Result<Item, Status>> + Send>>;

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
}

// -----------------------------------------------------------------------------
// Testing
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::println as info;
    use std::sync::Once;

    use anyhow::Error;
    use tonic::{
        transport::{Channel, Server},
        Request,
    };

    use uuid::Uuid;

    use crate::{
        server,
        server::StoreInventory,
        store::{
            inventory_client::InventoryClient, inventory_server::InventoryServer, Item,
            ItemIdentifier, ItemStock, PriceChangeRequest, QuantityChangeRequest,
        },
    };

    // -------------------------------------------------------------------------
    // Test Setup
    // -------------------------------------------------------------------------

    static SERVER_INIT: Once = Once::new();
    async fn get_client() -> InventoryClient<Channel> {
        SERVER_INIT.call_once(|| {
            tokio::spawn(async {
                let addr = "127.0.0.1:8080".parse().unwrap();
                let inventory = StoreInventory::default();
                Server::builder()
                    .add_service(InventoryServer::new(inventory))
                    .serve(addr)
                    .await
                    .unwrap();
            });
        });

        loop {
            match InventoryClient::connect("http://127.0.0.1:8080").await {
                Ok(client) => return client,
                Err(_) => println!("waiting for server connection"),
            };
        }
    }

    // -------------------------------------------------------------------------
    // Tests
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn inventory_management() -> Result<(), Error> {
        let mut client = get_client().await;

        // ---------------------------------------------------------------------
        // test adding items
        // ---------------------------------------------------------------------

        info!("adding a single item to the inventory");
        let sku = Uuid::new_v4().to_string();
        let item_id = ItemIdentifier { sku: sku.clone() };
        let item_stock = ItemStock {
            price: 1.79,
            quantity: 42,
        };
        let item = Item {
            identifier: Some(item_id.to_owned()),
            stock: Some(item_stock.to_owned()),
            information: None,
        };
        let request = Request::new(item.clone());
        let response = client.add(request).await?;
        assert_eq!(response.into_inner().status, "success");

        info!("verifying that items with an blank SKU are rejected");
        let bad_item = Item {
            identifier: Some(ItemIdentifier { sku: "".into() }),
            stock: Some(item_stock.clone()),
            information: None,
        };
        let request = Request::new(bad_item);
        let response = client.add(request).await;
        assert!(response.is_err());
        assert_eq!(response.err().unwrap().message(), server::EMPTY_SKU_ERR);

        info!("verifying that items with no ID are rejected");
        let bad_item = Item {
            identifier: None,
            stock: Some(item_stock.clone()),
            information: None,
        };
        let request = Request::new(bad_item);
        let response = client.add(request).await;
        assert!(response.is_err());
        assert_eq!(response.err().unwrap().message(), server::NO_ID_ERR);

        info!("verifying that items marked as $0.00 in cost are rejected");
        let bad_item = Item {
            identifier: Some(ItemIdentifier { sku: "FREE".into() }),
            stock: Some(ItemStock {
                price: 0.00,
                quantity: 42,
            }),
            information: None,
        };
        let request = Request::new(bad_item);
        let response = client.add(request).await;
        assert!(response.is_err());
        assert_eq!(response.err().unwrap().message(), server::BAD_PRICE_ERR);

        info!("verifying that items with no stock information are rejected");
        let bad_item = Item {
            identifier: Some(ItemIdentifier { sku: "NONE".into() }),
            stock: None,
            information: None,
        };
        let request = Request::new(bad_item);
        let response = client.add(request).await;
        assert!(response.is_err());
        assert_eq!(response.err().unwrap().message(), server::NO_STOCK_ERR);

        info!("verifying that duplicate items are rejected");
        let request = Request::new(item.clone());
        let response = client.add(request).await;
        assert!(response.is_err());
        assert_eq!(response.err().unwrap().message(), server::DUP_ITEM_ERR);

        info!("adding a 1000 generic items to the inventory");
        for i in 1000..2000 {
            let item_id = ItemIdentifier {
                sku: format!("SKU{}", i),
            };
            let item = Item {
                identifier: Some(item_id),
                stock: Some(item_stock.clone()),
                information: None,
            };

            let request = Request::new(item);
            let response = client.add(request).await?;
            assert_eq!(response.into_inner().status, "success");
        }

        // ---------------------------------------------------------------------
        // test updating an item's quantity
        // ---------------------------------------------------------------------

        info!("reducing item inventory by 35 units");
        let request = Request::new(QuantityChangeRequest {
            sku: sku.clone(),
            change: -35,
        });
        let response = client.update_quantity(request).await?;
        assert_eq!(response.into_inner().status, "success");

        info!("verifying quantity change");
        let request = Request::new(ItemIdentifier { sku: sku.clone() });
        let quantity = item_quantity(&client.get(request).await?.into_inner());
        assert_eq!(quantity, 7);

        info!("increasing item inventory by 7 units");
        let request = Request::new(QuantityChangeRequest {
            sku: sku.clone(),
            change: 7,
        });
        let response = client.update_quantity(request).await?;
        assert_eq!(response.into_inner().status, "success");

        info!("verifying quantity updates for no-SKU items are rejected");
        let request = Request::new(QuantityChangeRequest {
            sku: "".into(),
            change: 1024,
        });
        let response = client.update_quantity(request).await;
        assert!(response.is_err());
        assert_eq!(response.err().unwrap().message(), server::EMPTY_SKU_ERR);

        info!("verifying quantity updates that introduce no change are rejected");
        let request = Request::new(QuantityChangeRequest {
            sku: sku.clone(),
            change: 0,
        });
        let response = client.update_quantity(request).await;
        assert!(response.is_err());
        assert_eq!(response.err().unwrap().message(), server::EMPTY_QUANT_ERR);

        info!("verifying quantity updates for non-existent items are rejected");
        let request = Request::new(QuantityChangeRequest {
            sku: "DOESNTEXIST".into(),
            change: 4098,
        });
        let response = client.update_quantity(request).await;
        assert!(response.is_err());
        assert_eq!(response.err().unwrap().message(), server::NO_ITEM_ERR);

        info!("verifying quantity updates that would reduce below 0 are rejected");
        let request = Request::new(QuantityChangeRequest {
            sku: sku.clone(),
            change: -15,
        });
        let response = client.update_quantity(request).await;
        assert!(response.is_err());
        assert_eq!(response.err().unwrap().message(), server::UNSUFF_INV_ERR);

        info!("verifying current item quantity");
        let request = Request::new(ItemIdentifier { sku: sku.clone() });
        let quantity = item_quantity(&client.get(request).await?.into_inner());
        assert_eq!(quantity, 14);

        // ---------------------------------------------------------------------
        // test updating an item's price
        // ---------------------------------------------------------------------

        info!("increasing the price of an item to $2.49");
        let request = Request::new(PriceChangeRequest {
            sku: item_id.sku.clone(),
            price: 2.49,
        });
        let response = client.update_price(request).await?;
        assert_eq!(response.into_inner().status, "success");

        info!("verifying price updates for items with no SKU are rejected");
        let request = Request::new(PriceChangeRequest {
            sku: "".into(),
            price: 9.99,
        });
        let response = client.update_price(request).await;
        assert!(response.is_err());
        assert_eq!(response.err().unwrap().message(), server::EMPTY_SKU_ERR);

        info!("verifying price updates to $0.00 are rejected");
        let request = Request::new(PriceChangeRequest {
            sku: sku.clone(),
            price: 0.00,
        });
        let response = client.update_price(request).await;
        assert!(response.is_err());
        assert_eq!(response.err().unwrap().message(), server::BAD_PRICE_ERR);

        info!("verifying price updates to a negative value are rejected");
        let request = Request::new(PriceChangeRequest {
            sku: sku.clone(),
            price: -8096.64,
        });
        let response = client.update_price(request).await;
        assert!(response.is_err());
        assert_eq!(response.err().unwrap().message(), server::BAD_PRICE_ERR);

        info!("verifying price updates to a non-existent item are rejected");
        let request = Request::new(PriceChangeRequest {
            sku: "DOESNTEXIST".into(),
            price: 299.99,
        });
        let response = client.update_price(request).await;
        assert!(response.is_err());
        assert_eq!(response.err().unwrap().message(), server::NO_ITEM_ERR);

        info!("verifying price updates to the price already set are rejected");
        let request = Request::new(PriceChangeRequest {
            sku: sku.clone(),
            price: 2.49,
        });
        let response = client.update_price(request).await;
        assert!(response.is_err());
        assert_eq!(response.err().unwrap().message(), server::DUP_PRICE_ERR);

        info!("verifying current item price");
        let request = Request::new(ItemIdentifier { sku: sku.clone() });
        let price = item_price(&client.get(request).await?.into_inner());
        assert_eq!(price, 2.49);

        // ---------------------------------------------------------------------
        // test retrieving items
        // ---------------------------------------------------------------------

        info!("verifying that retrievals of items with no SKU are rejected");
        let request = Request::new(ItemIdentifier { sku: "".into() });
        let response = client.get(request).await;
        assert!(response.is_err());
        assert_eq!(response.err().unwrap().message(), server::EMPTY_SKU_ERR);

        info!("verifying that retrievals of items which don't exist are rejected");
        let request = Request::new(ItemIdentifier {
            sku: "DOESNTEXIST".into(),
        });
        let response = client.get(request).await;
        assert!(response.is_err());
        assert_eq!(response.err().unwrap().message(), server::NO_ITEM_ERR);

        // ---------------------------------------------------------------------
        // test watching items
        // ---------------------------------------------------------------------

        // TODO

        // ---------------------------------------------------------------------
        // test removing items
        // ---------------------------------------------------------------------

        info!("removing all added items");
        let request = Request::new(item_id.clone());
        let response = client.remove(request).await?;
        assert_eq!(response.into_inner().status, "success: item was removed");
        for i in 1000..2000 {
            let item_id = ItemIdentifier {
                sku: format!("SKU{}", i),
            };
            let request = Request::new(item_id);
            let response = client.remove(request).await?;
            assert_eq!(response.into_inner().status, "success: item was removed");
        }

        info!("verifying removing items with no SKU is rejected");
        let request = Request::new(ItemIdentifier { sku: "".into() });
        let response = client.remove(request).await;
        assert!(response.is_err());
        assert_eq!(response.err().unwrap().message(), server::EMPTY_SKU_ERR);

        info!("verifying removing non-existent items succeeds, but is reported");
        let request = Request::new(item_id.clone());
        let response = client.remove(request).await?;
        assert_eq!(response.into_inner().status, "success: item didn't exist");

        Ok(())
    }

    // -------------------------------------------------------------------------
    // Helper Functions
    // -------------------------------------------------------------------------

    fn item_quantity(item: &Item) -> u32 {
        item.stock.as_ref().unwrap().quantity
    }

    fn item_price(item: &Item) -> f32 {
        item.stock.as_ref().unwrap().price
    }
}
