use eyre::Result;

use trading_model::model::{Exchange, MarketEvent, SharedInstrumentManager, Symbol};
use trading_model::wire::PacketStr;

use crate::market::depth_futures::BinanceFuturesDepthChannel;
use crate::market::depth_spot::{BinanceSpotDepthChannel, BinanceSpotDepthMessage};
use crate::market::msg::BinanceMarketFeedMessage;
use crate::market::ticker::{BinanceBookTicker, BinanceBookTickerChannel};
use crate::market::trade::BinanceTradeChannel;

pub struct BinanceMarketParser {
    pub(crate) symbol: Option<Symbol>,
    pub(crate) depth_spot: BinanceSpotDepthChannel,
    pub(crate) depth_futures: BinanceFuturesDepthChannel,
    pub(crate) trade: BinanceTradeChannel,
    pub(crate) book_ticker: BinanceBookTickerChannel,
}
impl BinanceMarketParser {
    pub fn new(exchange: Exchange, manager: Option<SharedInstrumentManager>) -> Self {
        Self {
            symbol: None,
            depth_spot: BinanceSpotDepthChannel::new(exchange, manager.clone()),
            depth_futures: BinanceFuturesDepthChannel::new(exchange, manager.clone()),
            trade: BinanceTradeChannel::new(exchange, manager.clone()),
            book_ticker: BinanceBookTickerChannel::new(exchange, manager.clone()),
        }
    }
    pub fn set_symbol(&mut self, symbol: Symbol) {
        self.symbol = Some(symbol);
    }
    pub fn parse_message(&self, pkt: PacketStr) -> Result<Option<MarketEvent>> {
        // Binance Spot Depth Update
        if pkt.starts_with("{\"lastUpdateId\":") {
            let msg: BinanceSpotDepthMessage = serde_json::from_str(&pkt)?;

            let quotes = self.depth_spot.parse_binance_spot_depth_update(
                self.symbol.as_ref().unwrap(),
                msg,
                pkt.received_time,
            )?;

            return Ok(Some(MarketEvent::Quotes(quotes)));
        }
        // Binance Spot BookTicker
        if pkt.starts_with("{\"u\":") {
            let msg: BinanceBookTicker = serde_json::from_str(&pkt)?;
            let tob = self.book_ticker.parse_binance_book_ticker(msg, pkt.received_time)?;

            return Ok(Some(MarketEvent::BookTicker(tob)));
        }
        let msg: BinanceMarketFeedMessage = serde_json::from_str(&pkt)?;
        match msg {
            BinanceMarketFeedMessage::DepthUpdateFutures(update) => {
                let quotes = self.depth_futures.parse_binance_futures_depth_update(update)?;

                Ok(Some(MarketEvent::Quotes(quotes)))
            }
            BinanceMarketFeedMessage::Trade(trade) => {
                let trade = self.trade.parse_binance_trade_update(trade, pkt.received_time)?;
                Ok(Some(MarketEvent::Trade(trade)))
            }
            BinanceMarketFeedMessage::BookTicker(ticker) => {
                let tob = self.book_ticker.parse_binance_book_ticker(ticker, pkt.received_time)?;
                Ok(Some(MarketEvent::BookTicker(tob)))
            }
        }
    }
}
