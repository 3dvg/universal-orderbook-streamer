use crate::{
    model::{de_usize_from_str, OrderBook, OrderBookLevel},
    Exchange,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BitStampOrderBook {
    #[serde(deserialize_with = "de_usize_from_str")]
    pub timestamp: usize,
    #[serde(deserialize_with = "de_usize_from_str")]
    pub microtimestamp: usize,
    pub bids: Vec<OrderBookLevel>,
    pub asks: Vec<OrderBookLevel>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BitStampOrderBookWrapper {
    pub event: String,
    pub channel: String,
    pub data: BitStampOrderBook,
}

impl From<(Exchange, String, BitStampOrderBookWrapper)> for OrderBook {
    fn from((exchange, instrument, book): (Exchange, String, BitStampOrderBookWrapper)) -> Self {
        Self {
            exchange,
            sequence: book.data.timestamp,
            instrument,
            bids: book.data.bids,
            asks: book.data.asks,
        }
    }
}
