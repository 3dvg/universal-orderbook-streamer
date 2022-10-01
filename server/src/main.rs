use server::{
    orderbook::{orderbook_aggregator_server::OrderbookAggregatorServer, Summary}, OrderbookAggregatorService,
};
use data_streamer::{Exchange, Instrument, Subscriptions};
use log::*;
use tonic::transport::{Server};
use tokio::sync::{broadcast};
use server::{OrderbookStorage};
use clap::Parser;
#[derive(Parser, Debug, Clone)]
#[clap(author = "Eduardo Gallego", version = "0.0", about = "Universal Orderbook", long_about = None)]
struct Args {
    #[clap(short, long, help = "The base of the pair")]
    base: String,
    #[clap(short, long, help = "The quote of the pair")]
    quote: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::formatted_timed_builder()
        .filter_level(LevelFilter::Info)
        .init();
        
    let args = Args::parse();

    let address = "[::1]:10000";
    info!("Server listening on {address}");
    let subscriptions = Subscriptions {
        instrument: Instrument {
            base: args.base.to_string(),
            quote: args.quote.to_string(),
        },
        exchanges: vec![Exchange::Binance, Exchange::Bitstamp],
    };

    let mut orderbook_storage = OrderbookStorage::new();
    let (tx, receiver) = broadcast::channel(16);
    let sender = tx.clone();
    
    tokio::spawn(async move {
        let mut streams = subscriptions.clone().init().await.unwrap();
        while let Ok(orderbook) = streams.stream.recv().await {
            orderbook_storage.update(orderbook.exchange, orderbook);
            let summary = Summary::from(orderbook_storage.merge());
            if tx.send(summary).is_err() {
                warn!("Connection dropped");
            };
        }
    });

    let service = OrderbookAggregatorService {
        sender,
        receiver,
    };

    Server::builder()
        .add_service(OrderbookAggregatorServer::new(service))
        .serve(address.parse()?)
        .await?;

    Ok(())
}
