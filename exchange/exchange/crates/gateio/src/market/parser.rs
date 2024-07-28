use eyre::Result;

use trading_model::model::{Exchange, MarketEvent, SharedInstrumentManager, Symbol};
use trading_model::wire::PacketStr;

use crate::market::depth::GateioDepthChannel;
use crate::market::msg::{GateioMarketFeedMessage, GateioMarketFeedMessageOuter};
use crate::market::ticker::GateioBookTickerChannel;
use crate::market::trade::GateioTradeChannel;

pub struct GateioMarketParser {
    pub(crate) symbol: Option<Symbol>,
    pub(crate) depth_spot: GateioDepthChannel,

    pub(crate) trade: GateioTradeChannel,
    pub(crate) book_ticker: GateioBookTickerChannel,
}
impl GateioMarketParser {
    pub fn new(exchange: Exchange, manager: SharedInstrumentManager) -> Self {
        Self {
            symbol: None,
            depth_spot: GateioDepthChannel::new(exchange, manager.clone()),
            trade: GateioTradeChannel::new(exchange, manager.clone()),
            book_ticker: GateioBookTickerChannel::new(exchange, Some(manager.clone())),
        }
    }
    pub fn set_symbol(&mut self, symbol: Symbol) {
        self.symbol = Some(symbol);
    }
    pub fn parse_message(&self, pkt: PacketStr) -> Result<Option<MarketEvent>> {
        let msg: GateioMarketFeedMessageOuter = serde_json::from_str(&pkt)?;
        match msg.result {
            GateioMarketFeedMessage::SpotOrderBook(update) => {
                let quotes = self.depth_spot.parse_spot_depth_update(update, pkt.received_time)?;

                Ok(Some(MarketEvent::Quotes(quotes)))
            }
            GateioMarketFeedMessage::PerpetualOrderBook(update) => {
                let quotes = self
                    .depth_spot
                    .parse_perpetual_depth_update(update, pkt.received_time)?;

                Ok(Some(MarketEvent::Quotes(quotes)))
            }
            GateioMarketFeedMessage::SpotTrade(trade) => {
                let trade = self.trade.parse_spot_trade(trade, pkt.received_time)?;
                Ok(Some(MarketEvent::Trade(trade)))
            }
            GateioMarketFeedMessage::PerpetualTrade(trade) => {
                let trade = self.trade.parse_perpetual_trade(trade, pkt.received_time)?;
                Ok(Some(MarketEvent::Trade(trade)))
            }
            GateioMarketFeedMessage::BookTicker(ticker) => {
                let tob = self.book_ticker.parse_binance_book_ticker(ticker, pkt.received_time)?;
                Ok(Some(MarketEvent::BookTicker(tob)))
            }
        }
    }
}
