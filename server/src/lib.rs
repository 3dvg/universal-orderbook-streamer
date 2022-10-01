
use std::{collections::HashMap, pin::Pin, vec};
use data_streamer::{model::OrderBook, Exchange};
use float_ord::FloatOrd;
use futures::{Stream};
use log::*;
use tokio::sync::{mpsc, broadcast};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
pub mod orderbook {
    tonic::include_proto!("orderbook");
}

use orderbook::orderbook_aggregator_server::OrderbookAggregator;
use orderbook::{Level, Summary, Empty};

/// Orderbook updates get stored in this struct
#[derive(Debug, Clone)]
pub struct OrderbookStorage {
    pub data: HashMap<Exchange, OrderBook>
}

impl OrderbookStorage {
    pub fn new() -> Self {
        Self { data: HashMap::new(),}
    }

    pub fn update(&mut self, exchange: Exchange, orderbook: OrderBook) {
        self.data.insert(exchange, orderbook);
    }

    pub fn get(&self, key: &Exchange) -> Option<&OrderBook> {
        self.data.get(key)
    }

    pub fn merge(&self) -> (f64, Vec<Level>, Vec<Level>){
        let mut merged_bids: Vec<Level> = vec![];
        let mut merged_asks: Vec<Level> = vec![];

        for (ex, ob) in self.data.clone().into_iter() {
            let mut bids: Vec<Level> = ob
                .bids
                .into_iter()
                .map(|level| Level {
                    exchange: ex.to_string(),
                    price: level.price,
                    amount: level.amount,
                })
                .collect();
            merged_bids.append(&mut bids);

            let mut asks: Vec<Level> = ob
                .asks
                .into_iter()
                .map(|level| Level {
                    exchange: ex.to_string(),
                    price: level.price,
                    amount: level.amount,
                })
                .collect();
            merged_asks.append(&mut asks);
        }
        merged_bids.sort_unstable_by_key(|level| (FloatOrd(-level.price), FloatOrd(-level.amount)));
        merged_asks.sort_unstable_by_key(|level| (FloatOrd(level.price), FloatOrd(-level.amount)));
        merged_bids.truncate(10);
        merged_asks.truncate(10);

        let spread = merged_asks[0].price - merged_bids[0].price;

        (spread, merged_bids, merged_asks)

    }
}


/// The gRPC service
#[derive(Debug)]
pub struct OrderbookAggregatorService {
    pub sender: broadcast::Sender<Summary>,
    pub receiver: broadcast::Receiver<Summary>,
}
/// Handling incoming requests for the defined gRPC endpoint
#[tonic::async_trait]
impl OrderbookAggregator for OrderbookAggregatorService {
    type BookSummaryStream = Pin<Box<dyn Stream<Item = Result<Summary, Status>> + Send + 'static>>;
    async fn book_summary(
        &self,
        request: Request<Empty>,
    ) -> Result<Response<Self::BookSummaryStream>, Status> {
        let remote_addr = request.remote_addr();
        info!("Received a new request from {:?}", remote_addr);
        let (response_tx, response_rs) = mpsc::channel(16);
        let mut watcher = self.sender.subscribe();
        tokio::spawn(async move {
            while let Ok(message) = watcher.recv().await {
                if response_tx.send(Ok(message)).await.is_err() {
                    info!("Client {:?} disconnected", remote_addr);
                    break
                }
            }
        });
        Ok(Response::new(Box::pin(ReceiverStream::new(response_rs))))
    }
}

/// Response sent to the client. Transform the current `OrderbookStorage` into the desired format `Summary`
impl From<(f64, Vec<Level>, Vec<Level>)> for Summary {
    fn from((spread, merged_bids, merged_asks): (f64, Vec<Level>, Vec<Level>)) -> Self {
        Self {
            spread,
            bids: merged_bids,
            asks: merged_asks,
        }
    }
}
