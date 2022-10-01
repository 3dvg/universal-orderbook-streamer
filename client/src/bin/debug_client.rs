pub mod orderbook {
    tonic::include_proto!("orderbook");
}
use orderbook::{orderbook_aggregator_client::OrderbookAggregatorClient, Empty};
use log::*;
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::formatted_timed_builder()
        .filter_level(LevelFilter::Info)
        .init();

    let mut client = OrderbookAggregatorClient::connect("http://[::1]:10000").await?;
    info!("Connected client: {:?}", client);

    let mut stream = client.book_summary(Empty{}).await?.into_inner();

    while let Some(summary) = stream.next().await {
        info!("{:#?}", summary?);
    }
    Ok(())
}
