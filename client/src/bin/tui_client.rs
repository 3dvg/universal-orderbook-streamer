pub mod orderbook {
    tonic::include_proto!("orderbook");
}
use orderbook::{orderbook_aggregator_client::OrderbookAggregatorClient, Empty, Summary};
use log::*;
use tokio_stream::StreamExt;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::formatted_timed_builder()
        .filter_level(LevelFilter::Info)
        .init();
    let mut client = OrderbookAggregatorClient::connect("http://[::1]:10000").await?;
    info!("Connected client");

    let m = MultiProgress::new();
    let mut pb_bids: Vec<ProgressBar> = Vec::with_capacity(10);
    let mut pb_asks: Vec<ProgressBar> = Vec::with_capacity(10);
    
    let header = m.add(ProgressBar::new(100));
    header.set_style(
        ProgressStyle::with_template("{prefix:.bold} {bar:} {msg}")
            .unwrap()
            .progress_chars("█  "),
    );
    header.set_prefix("  Exchange\t\t      Price\t         Amount");

    for _ in 0..10 {
        let pb_ask = m.add(ProgressBar::new(100));
        pb_ask.set_style(
            ProgressStyle::with_template("{spinner:.red} {prefix:.bold}▕{bar:.red}▏{msg:.red}")
                .unwrap()
                .progress_chars("█  "),
        );
        pb_asks.push(pb_ask);
    }
    
    let pb_spread = m.add(ProgressBar::new(100));
    pb_spread.set_style(
        ProgressStyle::with_template("{spinner:.blue} {prefix:.bold.blue} {bar:.blue} {msg:.blue}")
            .unwrap()
            .progress_chars("█  "),
    );
    pb_spread.set_prefix("Spread  ");

    for _ in 0..10 {
        let pb_bid = m.add(ProgressBar::new(100));
        pb_bid.set_style(
            ProgressStyle::with_template("{spinner:.green} {prefix:.bold}▕{bar:.green}▏{msg:.green}")
                .unwrap()
                .progress_chars("█  "),
        );
        pb_bids.push(pb_bid);
    }

    let mut stream = client.book_summary(Empty {}).await?.into_inner();

    while let Some(summary) = stream.next().await {
        let ob: Summary = summary?;
        
        // Otherwise the spinner won't work
        pb_spread.set_position(0);
        pb_spread.set_message(format!("{:.8}",ob.spread));

        let bid_max_len = ob.bids.iter().map(|l| l.amount).max_by(|a, b| a.partial_cmp(b).unwrap());
        let ask_max_len = ob.asks.iter().map(|l| l.amount).max_by(|a, b| a.partial_cmp(b).unwrap());
        
        ob.asks.iter().rev().enumerate().for_each(|(i, level)|
            pb_asks[i].update_level(ask_max_len, level)
        );

        ob.bids.iter().enumerate().for_each(|(i, level)|
            pb_bids[i].update_level(bid_max_len, level)
        );
    }
    Ok(())
}

trait UpdateLevel {
    fn update_level(&self, max_len: Option<f64>, level: &orderbook::Level);
}

impl UpdateLevel for ProgressBar {
    fn update_level(&self, max_len: Option<f64>, level: &orderbook::Level) {
        max_len.map(|len| {
            self.set_length((100.0) as u64);
            self.set_position(((level.amount / len)*100.0) as u64)
        });

        match level.exchange.as_str() {
            "Binance" => {self.set_prefix(format!("{} ", level.exchange));}
            "Bitstamp" => {self.set_prefix(format!("{}", level.exchange));}
            _ => {}
        }
        self.set_message(format!("{:.8}\t{:.8}", level.price, level.amount));
    }
}