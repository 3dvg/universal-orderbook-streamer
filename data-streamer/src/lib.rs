pub mod model;

use model::OrderBook;
use serde_enum_str::{Deserialize_enum_str, Serialize_enum_str};
use serde_json::{json, Value};
use std::{collections::HashMap, time::Duration};
use tokio::sync::{broadcast};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

pub type WebSocket = tokio_tungstenite::tungstenite::WebSocket<
    tokio_tungstenite::tungstenite::stream::MaybeTlsStream<std::net::TcpStream>,
>;
pub type SocketError = tokio_tungstenite::tungstenite::Error;

mod exchanges;
use exchanges::binance::model::BinanceOrderBook;
use exchanges::bitstamp::model::BitStampOrderBookWrapper;

use futures::{SinkExt, StreamExt};
use log::*;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Instrument {
    pub quote: String,
    pub base: String
}

impl Instrument {
    pub fn get_symbol_compact(&self) -> String {
        format!("{}{}",self.base, self.quote)
    }
    pub fn get_symbol_dash(&self) -> String {
        format!("{}-{}",self.base, self.quote)
    }
    pub fn get_symbol_slash(&self) -> String {
        format!("{}/{}",self.base, self.quote)
    }

    pub fn get_symbol_compact_usdt(&self) -> String {
        if self.base.to_lowercase() == "usd" {
            format!("{}t{}",self.base, self.quote)
        }
        else if self.quote.to_lowercase() == "usd" {
            format!("{}{}t",self.base, self.quote)
        }
        else {
            format!("{}{}",self.base, self.quote)
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize_enum_str, Serialize_enum_str)]
pub enum Exchange {
    #[default]
    Binance,
    Bitstamp,
}

#[derive(Debug)]
pub struct Streamer {
    pub stream: broadcast::Receiver<OrderBook>,
}

#[derive(Debug, Clone)]
pub struct Subscriptions {
    pub instrument: Instrument,
    pub exchanges: Vec<Exchange>,
}

/// [Subscriptions] is the core of `data-streamer`. Spawns a task for each exchange, each task sends updates
/// from the respective websocket to the channel. The client reads the messages coming from this channel
impl Subscriptions {
    pub async fn init(self) -> Result<Streamer, SocketError> {
        let (tx, rx) = broadcast::channel::<OrderBook>(16);
        for exchange in self.exchanges {
            let tx2 = tx.clone();
            match exchange {
                Exchange::Binance => {
                    tokio::spawn(consume_binance(self.instrument.clone(), tx2));
                }
                Exchange::Bitstamp => {
                    tokio::spawn(consume_bitstamp(self.instrument.clone(), tx2));
                }
            }
        }
        Ok(Streamer { stream: rx })
    }
}

/// Connects to Binance webscoket stream. Handles reconnection with exponential backoff
/// Sends messages back to the main channdel from which it's called
pub async fn consume_binance(instrument: Instrument, sender: broadcast::Sender<OrderBook>) {
    let mut sleep = 100; //ms
    loop {
        let request = format!("wss://stream.binance.com:9443/ws/{}@depth20@100ms", instrument.get_symbol_compact_usdt());
        let (mut stream, _response) = connect_async(request)
            .await
            .expect("Expected connection with Binance to work");

        info!("Initialized Binance stream");
        while let Some(event) = stream.next().await {
            match event {
                Ok(Message::Text(msg)) => {
                    let raw_ob: BinanceOrderBook = serde_json::from_str(&msg).expect("Can't parse");
                    let ob = OrderBook::from((Exchange::Binance, instrument.get_symbol_compact(), raw_ob));
                    let _ = sender.send(ob);
                    sleep = 100;
                }
                Ok(Message::Binary(msg)) => {
                    warn!("Received binary message from Binance. Msg: {:?}", msg);
                    sleep = 100;
                }
                Ok(Message::Ping(msg)) => {
                    info!("Received PING message from Binance");
                    match stream.send(Message::Pong(msg)).await {
                        Ok(()) => {
                            info!("Sent PONG message to Binance");
                            sleep = 100;
                        },
                        Err(err) => {
                            error!("Couldn't send PONG to Binance. Error: {:?}", err);
                            break;
                        },
                    };
                }
                Ok(msg) => {
                    warn!("Received a non-handled message from Binance. Msg: {:?}", msg );
                    sleep = 100;
                }
                Err(err) => {
                    error!("Error from Binance websocket: {:?}", err);
                    break;
                }
            }
        }
        // Exponential backoff
        warn!("Binance stream disconnected, re-connecting. Sleep:{}", sleep);
        tokio::time::sleep(Duration::from_millis(sleep)).await;
        sleep *= 2;
    }
}

/// Connects to Bitstamp webscoket stream. Handles reconnection with exponential backoff
/// Sends messages back to the main channdel from which it's called
pub async fn consume_bitstamp(instrument: Instrument, sender: broadcast::Sender<OrderBook>){
    let mut sleep = 100; //ms
    loop {
        let (mut ws_stream, _ws_response) = connect_async("wss://ws.bitstamp.net")
            .await
            .expect("Expected connection with Bitstamp to work");

        info!("Initialized Bitstamp stream");
        let _ = ws_stream
            .send(Message::Text(
                json!({
                "event": "bts:subscribe",
                    "data": {
                        "channel": format!("order_book_{}", instrument.get_symbol_compact())
                    }
                })
                .to_string(),
            ))
            .await;

        info!("Sent subscription message to Bitstamp (required)");
        while let Some(event) = ws_stream.next().await {
            match event {
                Ok(Message::Text(msg)) => {
                    let resp = serde_json::from_str::<HashMap<String, Value>>(&msg);
                    if resp.is_err() {
                        error!("Error from Bitstamp: {:?}", resp);
                    }
                    let obj = resp.unwrap();
                    let event = obj.get("event").unwrap().as_str().unwrap();
                    match event {
                        "data" => {
                            let raw_ob: BitStampOrderBookWrapper = serde_json::from_str(&msg).expect("Can't parse");
                            let ob = OrderBook::from((Exchange::Bitstamp, instrument.get_symbol_compact(), raw_ob));
                            let _ = sender.send(ob);
                        }
                        "bts:subscription_succeeded" => info!("Connection with Bitstamp succedded"),
                        _ =>  warn!("Received non-data message from Bitstamp. Msg: {:?}", event),
                    }
                    sleep = 100;
                }
                Ok(Message::Binary(msg)) => warn!("Received binary message from Bitstamp. Msg: {:?}", msg),
                Ok(Message::Ping(msg)) => {
                    info!("Received PING message from Bitstamp");
                    match ws_stream.send(Message::Pong(msg)).await {
                        Ok(()) => {
                            info!("Sent PONG message to Bitstamp");
                            sleep = 100;
                        },
                        Err(err) => {
                            error!("Couldn't send PONG to Bitstamp. Error: {:?}", err);
                            break;
                        },
                    };
                    
                }
                Ok(msg) => {
                    warn!("Received a non-handled message from Bitstamp, ignoring. Msg: {:?}", msg);
                    sleep = 100;
                },
                Err(err) => {
                    error!("Error from Bitstamp websocket: {:?}", err);
                    break;
                },
                
            }
        }
        // Exponential backoff
        warn!("Bitstamp stream disconnected, re-connecting. Sleep:{}", sleep);
        tokio::time::sleep(Duration::from_millis(sleep)).await;
        sleep *= 2;
    }
}