use eyre::Result;

use trading_model::model::{Exchange, MarketEvent, SharedInstrumentManager, Symbol};
use trading_model::wire::PacketStr;

use crate::market::depth_futures::BinanceFuturesDepthChannel;
use crate::market::depth::{KucoinSpotDepthChannel, KucoinSpotDepthMessage};
use crate::market::msg::KucoinMarketFeedMessage;
use crate::market::ticker::{KucoinBookTicker, KucoinBookTickerChannel};


pub struct KucoinMarketParser {
    pub(crate) symbol: Option<Symbol>,
    pub(crate) depth: KucoinSpotDepthChannel,
    pub(crate) futures: KucoinFuturesDepthChannel,
    //pub(crate) trade: TradeChannel,
    pub(crate) book_ticker: KucoinBookTickerChannel,
}
impl KucoinMarketParser {
    pub fn new(exchange: Exchange, manager: Option<SharedInstrumentManager>) -> Self {
        Self {
            symbol: None,
            depth: KucoinSpotDepthChannel::new(exchange, manager.clone()),
            futures: KucoinFuturesDepthChannel::new(exchange, manager.clone()),
            //trade: TradeChannel::new(exchange, manager.clone()),
            book_ticker: KucoinBookTickerChannel::new(exchange, manager.clone()),
        }
    }
    pub fn set_symbol(&mut self, symbol: Symbol) {
        self.symbol = Some(symbol);
    }
    pub fn parse_message(&self, pkt: PacketStr) -> Result<Option<MarketEvent>> {
        if pkt.starts_with("{\"lastUpdateId\":") {
            let msg: KucoinSpotDepthMessage = serde_json::from_str(&pkt)?;

            let quotes = self.depth_spot.parse_kucoin_spot_depth_update(
                self.symbol.as_ref().unwrap(),
                msg,
                pkt.received_time,
            )?;

            return Ok(Some(MarketEvent::Quotes(quotes)));
        }
        if pkt.starts_with("{\"u\":") {
            let msg: KucoinBookTicker = serde_json::from_str(&pkt)?;
            let tob = self.book_ticker.parse_kucoion_book_ticker(msg, pkt.received_time)?;

            return Ok(Some(MarketEvent::BookTicker(tob)));
        }
        let msg: KucoinMarketFeedMessage = serde_json::from_str(&pkt)?;
        match msg {
            KucoinMarketFeedMessage::DepthUpdateFutures(update) => {
                let quotes = self.futures.parse_kucoin_futures_depth_update(update)?;

                Ok(Some(MarketEvent::Quotes(quotes)))
            }
           // KucoinMarketFeedMessage::Trade(trade) => {
            //    let trade = self.trade.parse_kucoin_trade_update(trade, pkt.received_time)?;
             //   Ok(Some(MarketEvent::Trade(trade)))
            //}
            KucoinMarketFeedMessage::BookTicker(ticker) => {
                let tob = self.book_ticker.parse_kucoin_book_ticker(ticker, pkt.received_time)?;
                Ok(Some(MarketEvent::BookTicker(tob)))
            }
        }
    }
}
