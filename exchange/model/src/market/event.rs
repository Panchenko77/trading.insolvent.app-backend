use crate::{
    BookTicker, FundingRateEvent, InstrumentCode, Market, MarketTrade, MarketUniversal, PriceEvent, Quotes, Time,
    OHLCVT,
};
use derive_from_one::FromOne;
use tracing::warn;

#[derive(Clone, PartialEq, Debug, FromOne)]
pub enum MarketEvent {
    String(String),
    Trade(MarketTrade),
    Trades(Vec<MarketTrade>),
    Quotes(Quotes),
    BookTicker(BookTicker),
    OHLCVT(OHLCVT),
    Price(PriceEvent),
    FundingRate(FundingRateEvent),
    FundingRates(Vec<FundingRateEvent>),
}

impl MarketEvent {
    pub fn get_instrument(&self) -> Option<InstrumentCode> {
        match self {
            Self::String(_) => None,
            Self::Trade(trade) => Some(trade.instrument.clone()),
            Self::Trades(trades) => trades.first().map(|trade| trade.instrument.clone()),
            Self::Quotes(quotes) => Some(quotes.instrument.clone()),
            Self::BookTicker(top_of_book) => Some(top_of_book.instrument.clone()),
            Self::OHLCVT(ohlcv) => Some(ohlcv.instrument.clone()),
            Self::Price(price) => Some(price.instrument.clone()),
            Self::FundingRate(funding_rate) => Some(funding_rate.instrument.clone()),
            Self::FundingRates(funding_rates) => funding_rates
                .first()
                .map(|funding_rate| funding_rate.instrument.clone()),
        }
    }

    pub fn get_timestamp(&self) -> Time {
        match self {
            Self::String(_) => Time::NULL,
            Self::Trade(trade) => trade.exchange_time,
            Self::Trades(trades) => trades.first().map(|trade| trade.exchange_time).unwrap_or(Time::NULL),
            Self::Quotes(quotes) => quotes.exchange_time,
            Self::BookTicker(top_of_book) => top_of_book.exchange_time,
            Self::OHLCVT(ohlcv) => ohlcv.exchange_time,
            Self::Price(price) => price.exchange_time,
            Self::FundingRate(funding_rate) => funding_rate.exchange_time,
            Self::FundingRates(funding_rates) => funding_rates
                .first()
                .map(|funding_rate| funding_rate.exchange_time)
                .unwrap_or(Time::NULL),
        }
    }

    pub fn update_market(&self, market: &mut Market) {
        match self {
            Self::Trade(trade) => {
                market.trades.trades.push(trade.clone());
            }
            Self::Trades(trades) => {
                market.trades.trades.extend(trades.iter().cloned());
            }
            Self::Quotes(quotes) => market.orderbook.update_quotes(quotes.get_quotes()),
            Self::BookTicker(top_of_book) => market.orderbook.update_top_of_book(top_of_book),
            Self::String(_) => {}
            _ => {
                warn!("unhandled market event: {:?}", self);
            }
        }
    }

    pub fn update_universe(&self, universe: &mut MarketUniversal) {
        if let Some(exchange) = self.get_instrument() {
            self.update_market(universe.ensure_market(exchange))
        }
    }
}
