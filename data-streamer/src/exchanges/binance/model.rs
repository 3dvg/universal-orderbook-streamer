use serde::Deserialize;

use crate::{
    model::{OrderBook, OrderBookLevel},
    Exchange,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BinanceOrderBook {
    pub last_update_id: usize,
    pub bids: Vec<OrderBookLevel>,
    pub asks: Vec<OrderBookLevel>,
}

impl From<(Exchange, String, BinanceOrderBook)> for OrderBook {
    fn from((exchange, instrument, book): (Exchange, String, BinanceOrderBook)) -> Self {
        Self {
            exchange,
            sequence: book.last_update_id,
            instrument,
            bids: book.bids,
            asks: book.asks,
        }
    }
}
