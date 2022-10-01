use serde::de;
use serde::{Deserialize, Deserializer};

use crate::Exchange;

#[derive(Debug, Deserialize, Clone)]
pub struct OrderBookLevel {
    #[serde(deserialize_with = "de_float_from_str")]
    pub price: f64,
    #[serde(deserialize_with = "de_float_from_str")]
    pub amount: f64,
}

/// Normalized Orderbook model. Orderbook updates coming from each exchange get transformed
/// into this struct.
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct OrderBook {
    pub exchange: Exchange,
    pub sequence: usize,
    pub instrument: String,
    pub bids: Vec<OrderBookLevel>,
    pub asks: Vec<OrderBookLevel>,
}

pub fn de_float_from_str<'a, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'a>,
{
    let str_val = String::deserialize(deserializer)?;
    str_val.parse::<f64>().map_err(de::Error::custom)
}

pub fn de_usize_from_str<'a, D>(deserializer: D) -> Result<usize, D::Error>
where
    D: Deserializer<'a>,
{
    let str_val = String::deserialize(deserializer)?;
    str_val.parse::<usize>().map_err(de::Error::custom)
}
