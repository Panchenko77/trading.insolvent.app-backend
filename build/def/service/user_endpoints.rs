use endpoint_gen::model::{EndpointSchema, Field, Type};
/// concat N Vec<T> into Vec<T>
macro_rules! concat {
    // Base case: just one vector, return it directly.
    ($x:expr) => ($x);

    // Recursive case: more than one vector.
    ($x:expr, $($y:expr),+ $(,)?) => {{
        [concat!($($y),+), $x].concat()
    }};
}

fn symbol_list() -> Type {
    Type::datatable(
        "UserSymbolList",
        vec![
            // symbol name "BTC", without USD/USDT suffix
            Field::new("symbol", Type::String),
            // either "active" or "inactive" for now
            Field::new("status", Type::String),
            // flag (for the purpose of manually filtering out "bad" symbols)
            Field::new("flag", Type::Boolean),
        ],
    )
}
fn trading_symbol_list() -> Type {
    Type::datatable(
        "UserTradingSymbol",
        vec![
            Field::new("exchange", Type::String),
            // symbol name "BTC", without USD/USDT suffix
            Field::new("symbol", Type::String),
            Field::new("base", Type::String),
            Field::new("lot_size", Type::Numeric),
            Field::new("base_decimals", Type::Int),
            Field::new("quote", Type::String),
            Field::new("tick_size", Type::Numeric),
            Field::new("quote_decimals", Type::Int),
        ],
    )
}
fn strategy_status() -> Type {
    Type::datatable(
        "UserStrategyStatus",
        vec![Field::new("id", Type::Int), Field::new("status", Type::String)],
    )
}
fn live_test_close_order_price() -> Type {
    Type::datatable(
        "UserLiveTestPrice",
        vec![
            Field::new("symbol", Type::String),
            Field::new("datetime", Type::BigInt),
            Field::new("target_price", Type::Numeric),
            Field::new("last_price", Type::Numeric),
            Field::new("trend_prediction", Type::String),
            Field::new("price_event", Type::Numeric),
            Field::new("price_actual_filled", Type::Numeric),
            Field::new("price_market_when_filled", Type::Numeric),
            Field::new("pass_actual_filled", Type::Boolean),
            Field::new("pass_market_when_filled", Type::Boolean),
            Field::new("last_open_price", Type::Numeric),
            Field::new("last_close_price", Type::Numeric),
            Field::new("last_high_price", Type::Numeric),
            Field::new("last_low_price", Type::Numeric),
        ],
    )
}

/// price data
struct Price;
impl Price {
    fn price() -> Type {
        Type::datatable(
            "Price",
            vec![
                Field::new("datetime", Type::BigInt),
                Field::new("symbol", Type::String),
                Field::new("price", Type::Numeric),
            ],
        )
    }
    fn price_0() -> Type {
        Type::datatable(
            "Price0",
            vec![
                Field::new("datetime", Type::BigInt),
                // binance top 5 bid average
                Field::new("binance_price", Type::Numeric),
                // hyper top bid
                Field::new("hyper_bid_price", Type::Numeric),
                // hyper oracle
                Field::new("hyper_oracle", Type::Numeric),
                // hyper mark
                Field::new("hyper_mark", Type::Numeric),
                // hyper_bid_price - hyper_mark
                Field::new("difference_in_usd", Type::Numeric),
                // (hyper_bid_price - hyper_mark) * 10000 / hyper_mark
                Field::new("difference_in_basis_points", Type::Numeric),
            ],
        )
    }
    fn price_1() -> Type {
        Type::datatable(
            "PriceDifference",
            vec![
                Field::new("datetime", Type::BigInt),
                // binance top 5 bid average
                Field::new("binance_price", Type::Numeric),
                // hyper best price
                Field::new("hyper_ask_price", Type::Numeric),
                Field::new("hyper_bid_price", Type::Numeric),
                // hyper_bid_price - hyper_mark
                Field::new("difference_in_usd", Type::Numeric),
                // (hyper_bid_price - hyper_mark) * 10000 / hyper_mark
                Field::new("difference_in_basis_points", Type::Numeric),
            ],
        )
    }
    fn price_spread() -> Type {
        Type::datatable(
            "BestBidAskAcrossExchanges",
            vec![
                Field::new("datetime", Type::BigInt),
                Field::new("binance_ask_price", Type::Numeric),
                Field::new("binance_ask_volume", Type::Numeric),
                Field::new("binance_bid_price", Type::Numeric),
                Field::new("binance_bid_volume", Type::Numeric),
                // hyper best price
                Field::new("hyper_ask_price", Type::Numeric),
                Field::new("hyper_ask_volume", Type::Numeric),
                Field::new("hyper_bid_price", Type::Numeric),
                Field::new("hyper_bid_volume", Type::Numeric),
                // bp(binance bid, hyper ask)
                Field::new("bb_ha", Type::Numeric),
                // bp(binance ask, hyper bid)
                Field::new("ba_hb", Type::Numeric),
            ],
        )
    }
    fn price_spread_with_position() -> Type {
        Type::datatable(
            "BestBidAskAcrossExchangesWithPosition",
            vec![
                Field::new("id", Type::BigInt),
                Field::new("opening_id", Type::BigInt),
                Field::new("datetime", Type::BigInt),
                Field::new("expiry", Type::BigInt),
                Field::new("symbol", Type::String),
                Field::new("ba_bn", Type::Numeric),
                Field::new("bb_bn", Type::Numeric),
                Field::new("ba_amount_bn", Type::Numeric),
                Field::new("bb_amount_bn", Type::Numeric),
                Field::new("ba_hp", Type::Numeric),
                Field::new("bb_hp", Type::Numeric),
                Field::new("ba_amount_hp", Type::Numeric),
                Field::new("bb_amount_hp", Type::Numeric),
                Field::new("hl_balance_coin", Type::Numeric),
                Field::new("ba_balance_coin", Type::Numeric),
                Field::new("opportunity_size", Type::Numeric),
                Field::new("expired", Type::Boolean),
                Field::new("action", Type::String),
            ],
        )
    }
    fn spread() -> Type {
        Type::datatable(
            "PriceSpread",
            vec![
                Field::new("datetime", Type::BigInt),
                Field::new("exchange_1", Type::String),
                Field::new("exchange_2", Type::String),
                Field::new("asset", Type::String),
                Field::new("spread_buy_1", Type::Numeric),
                Field::new("spread_sell_1", Type::Numeric),
            ],
        )
    }
}

/// signal data
struct Signal;
impl Signal {
    /// header
    fn header() -> Vec<Field> {
        vec![
            // unique incremental ID
            Field::new("id", Type::BigInt),
            // timestamp when event is detected
            Field::new("datetime", Type::BigInt),
            // symbol name "BTC", without USD/USDT suffix
            Field::new("symbol", Type::String),
            // "Normal"/"High"/"Critical"
            Field::new("level", Type::String),
        ]
    }
    /// signal in strategy 0
    fn signal_0() -> Type {
        Type::datatable(
            "Signal0",
            concat!(
                Signal::header(),
                vec![
                    // 0 for lowest, higher value higher priority
                    Field::new("priority", Type::Int),
                    //basis point
                    Field::new("bp", Type::Numeric),
                ]
            ),
        )
    }

    /// difference in binance bid and hyper bid
    fn bb_hb_diff() -> Type {
        Type::struct_(
            "SignalPriceDifference",
            vec![
                Field::new("price_binance", Type::Numeric),
                Field::new("price_hyper", Type::Numeric),
                Field::new("bp", Type::Numeric),
                Field::new("used", Type::Boolean),
            ],
        )
    }

    /// binance bid change
    fn bb_change_1() -> Type {
        Type::struct_(
            "SignalPriceChange",
            vec![
                Field::new("trend", Type::String),
                Field::new("time_high", Type::BigInt),
                Field::new("time_low", Type::BigInt),
                Field::new("price_high", Type::Numeric),
                Field::new("price_low", Type::Numeric),
                Field::new("bp", Type::Numeric),
                Field::new("used", Type::Boolean),
            ],
        )
    }
    /// price change immediate
    fn price_change_immediate() -> Type {
        Type::struct_(
            "SignalPriceChangeImmediate",
            vec![
                Field::new("trend", Type::String),
                Field::new("exchange", Type::String),
                Field::new("price_type", Type::String),
                Field::new("before", Type::Numeric),
                Field::new("after", Type::Numeric),
                Field::new("ratio", Type::Numeric),
                Field::new("used", Type::Boolean),
            ],
        )
    }
    fn bb_ha_diff() -> Type {
        Type::struct_(
            "SignalBinBidHypAskDiff",
            vec![
                Field::new("bin_bid", Type::Numeric),
                Field::new("hyp_ask", Type::Numeric),
                Field::new("ratio", Type::Numeric),
                Field::new("used", Type::Boolean),
            ],
        )
    }
    fn ba_hb_diff() -> Type {
        Type::struct_(
            "SignalBinAskHypBidDiff",
            vec![
                Field::new("bin_ask", Type::Numeric),
                Field::new("hyp_bid", Type::Numeric),
                Field::new("ratio", Type::Numeric),
                Field::new("used", Type::Boolean),
            ],
        )
    }
    // strategy 1
    fn signal_1() -> Type {
        Type::datatable(
            "Signal1",
            concat!(
                Signal::header(),
                vec![
                    Field::new("difference", Type::optional(Signal::bb_hb_diff())),
                    Field::new("change", Type::optional(Signal::bb_change_1())),
                ]
            ),
        )
    }
    // strategy 2
    fn signal_2() -> Type {
        Type::datatable(
            "Signal2",
            concat!(
                Signal::header(),
                vec![
                    Field::new("ba_change", Type::optional(Signal::price_change_immediate())),
                    Field::new("bb_change", Type::optional(Signal::price_change_immediate())),
                    Field::new("ba_hb_diff", Type::optional(Signal::ba_hb_diff())),
                    Field::new("bb_ha_diff", Type::optional(Signal::bb_ha_diff())),
                ]
            ),
        )
    }
}

struct Event;
impl Event {
    /// strategy 1
    fn event_1() -> Type {
        Type::datatable(
            "Event1",
            concat!(
                Signal::header(),
                vec![
                    // "Rising"/"Falling"
                    Field::new("trend", Type::String),
                    Field::new("binance_price", Type::Numeric),
                    Field::new("hyper_price", Type::Numeric),
                    Field::new("difference_in_basis_points", Type::Numeric),
                    Field::new("status", Type::String),
                ]
            ),
        )
    }
}

struct Request;
impl Request {
    /// request signal row for sudden change
    fn set_symbol_flag() -> Type {
        Type::datatable("RequestSymbolList", vec![Field::new("symbol", Type::String)])
    }
}

struct Order;
impl Order {
    fn orders() -> Type {
        Type::datatable(
            "UserOrder",
            vec![
                Field::new("id", Type::BigInt),
                // event ID of the event captured by the order
                Field::new("event_id", Type::BigInt),
                // client ID is generated by exchange crate to identify the ID
                Field::new("client_id", Type::String),
                Field::new("exchange", Type::String),
                Field::new("symbol", Type::String),
                Field::new("order_type", Type::String),
                Field::new("side", Type::String),
                Field::new("price", Type::Numeric),
                Field::new("volume", Type::Numeric),
                Field::new("strategy_id", Type::Int),
                Field::new("datetime", Type::TimeStampMs),
                Field::new("effect", Type::String),
                Field::new("status", Type::String),
            ],
        )
    }

    fn ledger() -> Type {
        Type::datatable(
            "UserLedger",
            vec![
                Field::new("id", Type::BigInt),
                Field::new("open_order_id", Type::String),
                Field::new("close_order_id", Type::String),
                Field::new("open_order_cloid", Type::String),
                Field::new("close_order_cloid", Type::String),
                Field::new("datetime", Type::TimeStampMs),
                Field::new("exchange", Type::String),
                Field::new("symbol", Type::String),
                Field::new("open_order_position_type", Type::String),
                Field::new("open_order_side", Type::String),
                Field::new("open_price_usd", Type::Numeric),
                Field::new("close_price_usd", Type::Numeric),
                Field::new("volume", Type::Numeric),
                Field::new("closed_profit", Type::Numeric),
            ],
        )
    }
    fn hedged_orders() -> Type {
        Type::datatable(
            "UserHedgedOrders",
            vec![
                Field::new("id", Type::BigInt),
                Field::new("leg1_id", Type::String),
                Field::new("leg2_id", Type::String),
                Field::new("leg1_cloid", Type::String),
                Field::new("leg2_cloid", Type::String),
                Field::new("datetime", Type::TimeStampMs),
                Field::new("leg1_ins", Type::String),
                Field::new("leg2_ins", Type::String),
                Field::new("leg1_side", Type::String),
                Field::new("leg2_side", Type::String),
                Field::new("leg1_price", Type::Numeric),
                Field::new("leg2_price", Type::Numeric),
                Field::new("leg1_status", Type::String),
                Field::new("leg2_status", Type::String),
                Field::new("size", Type::Numeric),
            ],
        )
    }
}
fn accuracy_log() -> Type {
    Type::datatable(
        "UserAccuracyLog",
        vec![
            Field::new("datetime", Type::TimeStampMs),
            // cumulative counts of pass and fail
            Field::new("count_pass", Type::BigInt),
            Field::new("count_fail", Type::BigInt),
            Field::new("accuracy", Type::Numeric),
        ],
    )
}
/// this is a signal row that aggregates the signals
fn set_encrypted_key() -> Type {
    Type::datatable(
        "UserSetEncryptedKey",
        vec![
            Field::new("exchange", Type::String),
            Field::new("account_id", Type::String),
            Field::new("ciphertext", Type::String),
            Field::new("alias", Type::String),
        ],
    )
}

/// this is a signal row that aggregates the signals
fn encrypted_key() -> Type {
    Type::datatable(
        "UserEncryptedKey",
        vec![
            Field::new("id", Type::BigInt),
            Field::new("exchange", Type::String),
            Field::new("account_id", Type::String),
            Field::new("ciphertext", Type::String),
            Field::new("alias", Type::String),
        ],
    )
}

// success and reason if failed
fn success_result() -> Vec<Field> {
    vec![
        Field::new("success", Type::Boolean),
        Field::new("reason", Type::optional(Type::String)),
    ]
}
fn success_place_order_result() -> Vec<Field> {
    vec![
        Field::new("success", Type::Boolean),
        Field::new("reason", Type::String),
        Field::new("local_id", Type::String),
        Field::new("client_id", Type::String),
    ]
}

struct Filter;
impl Filter {
    /// time query filter
    fn time() -> Vec<Field> {
        vec![
            Field::new("time_start", Type::optional(Type::TimeStampMs)),
            Field::new("time_end", Type::optional(Type::TimeStampMs)),
        ]
    }
    /// symbol query filter
    fn symbol(is_compulsory: bool) -> Vec<Field> {
        match is_compulsory {
            true => vec![Field::new("symbol", Type::String)],
            false => vec![Field::new("symbol", Type::optional(Type::String))],
        }
    }
    fn strategy_id() -> Vec<Field> {
        vec![Field::new("strategy_id", Type::Int)]
    }
}

fn get_user_debug_log_list() -> Type {
    Type::datatable(
        "UserDebugLogRow",
        vec![
            Field::new("datetime", Type::BigInt),
            Field::new("level", Type::String),
            Field::new("thread", Type::String),
            Field::new("path", Type::String),
            Field::new("line_number", Type::Int),
            Field::new("message", Type::String),
        ],
    )
}

fn funding_rates() -> Type {
    Type::datatable(
        "UserFundingRates",
        vec![
            Field::new("exchange", Type::String),
            Field::new("symbol", Type::String),
            Field::new("rate", Type::Numeric),
            Field::new("datetime", Type::BigInt),
        ],
    )
}

fn user_position_list() -> Type {
    Type::datatable(
        "UserPosition",
        vec![
            Field::new("id", Type::BigInt),
            Field::new("cloid", Type::optional(Type::String)),
            Field::new("exchange", Type::String),
            Field::new("symbol", Type::String),
            Field::new("size", Type::Numeric),
            Field::new("filled_size", Type::Numeric),
            Field::new("cancel_or_close", Type::String),
        ],
    )
}

fn user_benchmark_result() -> Type {
    Type::datatable(
        "UserBenchmarkResult",
        vec![
            Field::new("id", Type::BigInt),
            Field::new("datetime", Type::TimeStampMs),
            Field::new("exchange", Type::String),
            Field::new("latency_us", Type::BigInt),
        ],
    )
}
fn user_captured_event() -> Type {
    Type::datatable(
        "UserCapturedEvent",
        vec![
            Field::new("id", Type::BigInt),
            Field::new("event_id", Type::optional(Type::BigInt)),
            Field::new("cloid", Type::optional(Type::String)),
            Field::new("exchange", Type::String),
            Field::new("symbol", Type::String),
            Field::new("status", Type::String),
            Field::new("price", Type::optional(Type::Numeric)),
            Field::new("size", Type::Numeric),
            Field::new("filled_size", Type::Numeric),
            Field::new("cancel_or_close", Type::String),
        ],
    )
}
fn user_set_s2_configure() -> Type {
    Type::datatable(
        "DefaultS2Configuration",
        vec![
            Field::new("buy_exchange", Type::String),
            Field::new("sell_exchange", Type::String),
            Field::new("instrument", Type::String),
            Field::new("order_size", Type::Numeric),
            Field::new("max_unhedged", Type::Numeric),
            Field::new("target_spread", Type::Numeric),
            Field::new("target_position", Type::Numeric),
            Field::new("order_type", Type::String),
        ],
    )
}
pub fn get_user_endpoints() -> Vec<EndpointSchema> {
    vec![
        EndpointSchema::new(
            "UserStatus",
            20000,
            vec![],
            vec![
                Field::new("status", Type::String),
                Field::new("time", Type::TimeStampMs),
            ],
        ),
        EndpointSchema::new("UserSubLogs", 20010, vec![], vec![]).with_stream_response_type(Type::struct_(
            "UserLogEvent",
            vec![
                Field::new("level", Type::String),
                Field::new("time", Type::TimeStampMs),
                Field::new("content", Type::String),
            ],
        )),
        EndpointSchema::new("UserSubEvents", 20020, vec![Field::new("topic", Type::String)], vec![])
            .with_stream_response_type(Type::struct_(
                "UserEvent",
                vec![
                    Field::new("topic", Type::String),
                    Field::new("time", Type::TimeStampMs),
                    Field::new("content", Type::String),
                ],
            )),
        EndpointSchema::new(
            "UserSubPosition",
            20030,
            vec![Field::new("unsubscribe", Type::optional(Type::Boolean))],
            vec![Field::new("data", user_position_list())],
        )
        .with_stream_response_type(user_position_list()),
        EndpointSchema::new(
            "UserCancelOrClosePosition",
            20031,
            vec![Field::new("id", Type::BigInt)],
            vec![],
        ),
        EndpointSchema::new(
            "UserSubOrders",
            20040,
            vec![
                Field::new("strategy_id", Type::optional(Type::Int)),
                Field::new("unsubscribe", Type::optional(Type::Boolean)),
            ],
            vec![Field::new("data", Order::orders())],
        )
        .with_stream_response_type(Order::orders()),
        EndpointSchema::new(
            "UserListStrategy",
            20100,
            vec![Field::new("name", Type::optional(Type::String))],
            vec![Field::new(
                "strategies",
                Type::vec(Type::struct_(
                    "UserStrategyRow",
                    vec![
                        Field::new("name", Type::String),
                        Field::new("strategy_id", Type::Int),
                        Field::new("status", Type::String),
                        Field::new("config", Type::Object),
                    ],
                )),
            )],
        ),
        EndpointSchema::new(
            "UserInitStrategy",
            20110,
            vec![Field::new("strategy_id", Type::Int)],
            success_result(),
        ),
        EndpointSchema::new(
            "UserSubPrice0",
            20120,
            concat!(
                Filter::symbol(true),
                vec![Field::new("unsubscribe_other_symbol", Type::optional(Type::Boolean)),]
            ),
            vec![Field::new("data", Price::price_0())],
        )
        .with_stream_response_type(Price::price_0()),
        EndpointSchema::new(
            "UserGetPrice0",
            20130,
            concat!(Filter::symbol(true), Filter::time()),
            vec![Field::new("data", Price::price_0())],
        ),
        EndpointSchema::new(
            "UserControlStrategy",
            20140,
            vec![
                Field::new("strategy_id", Type::Int),
                Field::new("config", Type::Object),
                Field::new("paused", Type::Boolean),
            ],
            success_result(),
        ),
        EndpointSchema::new(
            "UserGetStrategyZeroSymbol",
            20150,
            Filter::symbol(false),
            vec![Field::new("data", symbol_list())],
        ),
        EndpointSchema::new(
            "UserSubSignal0",
            20160,
            concat!(
                Filter::symbol(false),
                vec![Field::new("unsubscribe_other_symbol", Type::optional(Type::Boolean)),]
            ),
            vec![Field::new("data", Signal::signal_0())],
        )
        .with_stream_response_type(Signal::signal_0()),
        EndpointSchema::new(
            "UserGetSignal0",
            20170,
            concat!(
                Filter::symbol(false),
                Filter::time(),
                vec![Field::new("min_level", Type::optional(Type::String)),]
            ),
            vec![Field::new("data", Signal::signal_0())],
        ),
        EndpointSchema::new(
            "UserGetDebugLog",
            20180,
            vec![
                Field::new("limit", Type::optional(Type::Int)),
                Field::new("page", Type::optional(Type::Int)),
            ],
            vec![Field::new("data", get_user_debug_log_list())],
        ),
        EndpointSchema::new(
            "UserSetEncryptedKey",
            21000,
            vec![Field::new("key", set_encrypted_key())],
            success_result(),
        ),
        // TODO replace teh below with UserSetServiceStatus
        // TODO define each service into a type with init/start/pause/stop status
        EndpointSchema::new(
            "UserStartService",
            21010,
            vec![Field::new(
                "keys",
                Type::vec(Type::struct_(
                    "UserKey",
                    vec![
                        Field::new("exchange", Type::String),
                        // arbitrum address for hyper
                        Field::new("account_id", Type::String),
                    ],
                )),
            )],
            success_result(),
        ),
        EndpointSchema::new(
            "UserSetStrategyStatus",
            21020,
            vec![Field::new("set_status", Type::optional(strategy_status()))],
            vec![Field::new("data", strategy_status())],
        ),
        EndpointSchema::new(
            "UserGetStrategyOneSymbol",
            20200,
            Filter::symbol(false),
            vec![Field::new("data", symbol_list())],
        ),
        EndpointSchema::new(
            "UserSetSymbolFlag1",
            20210,
            concat!(Filter::symbol(true), vec![Field::new("flag", Type::Boolean)],),
            success_result(),
        ),
        EndpointSchema::new(
            "UserGetEvent1",
            20240,
            concat!(
                Filter::symbol(false),
                Filter::time(),
                vec![Field::new("id", Type::optional(Type::BigInt)),],
            ),
            vec![Field::new("data", Event::event_1())],
        ),
        EndpointSchema::new(
            "UserSubEvent1",
            20250,
            Filter::symbol(false),
            vec![Field::new("data", Event::event_1())],
        )
        .with_stream_response_type(Event::event_1()),
        EndpointSchema::new(
            "UserGetStrategyOneAccuracy",
            20260,
            vec![],
            vec![
                Field::new("count_correct", Type::BigInt),
                Field::new("count_wrong", Type::BigInt),
                // in percentage, correct/(correct+wrong)
                Field::new("accuracy", Type::Numeric),
            ],
        ),
        EndpointSchema::new(
            "UserGetAccuracy",
            20261,
            concat![Filter::symbol(false)],
            vec![
                Field::new("count_correct", Type::BigInt),
                Field::new("count_wrong", Type::BigInt),
                // in percentage, correct/(correct+wrong)
                Field::new("accuracy", Type::Numeric),
            ],
        ),
        EndpointSchema::new(
            "UserGetOrdersPerStrategy",
            20271,
            concat![
                Filter::symbol(false),
                Filter::time(),
                Filter::strategy_id(),
                vec![Field::new("client_id", Type::optional(Type::String))],
                vec![Field::new("event_id", Type::optional(Type::BigInt))],
            ],
            vec![Field::new("data", Order::orders())],
        ),
        EndpointSchema::new(
            "UserSubStrategyOneOrder",
            20280,
            Filter::symbol(false),
            vec![Field::new("data", Order::orders())],
        )
        .with_stream_response_type(Order::orders()),
        EndpointSchema::new(
            "UserGetLedger",
            20291,
            concat![
                Filter::symbol(false),
                Filter::time(),
                Filter::strategy_id(),
                vec![
                    Field::new("client_id", Type::optional(Type::String)),
                    Field::new("include_ack", Type::optional(Type::Boolean))
                ],
            ],
            vec![Field::new("data", Order::ledger())],
        ),
        EndpointSchema::new(
            "UserGetHedgedOrders",
            20292,
            vec![Field::new("strategy_id", Type::Int)],
            vec![Field::new("data", Order::hedged_orders())],
        ),
        EndpointSchema::new(
            "UserSubLedgerStrategyOne",
            20300,
            Filter::symbol(false),
            vec![Field::new("data", Order::ledger())],
        )
        .with_stream_response_type(Order::ledger()),
        EndpointSchema::new(
            "UserSubLedger",
            20301,
            concat![Filter::symbol(false), Filter::strategy_id()],
            vec![Field::new("data", Order::ledger())],
        )
        .with_stream_response_type(Order::ledger()),
        EndpointSchema::new(
            "UserGetLiveTestAccuracyLog",
            20310,
            concat!(Filter::time(), vec![Field::new("tag", Type::optional(Type::String))]),
            vec![Field::new("data", accuracy_log())],
        ),
        EndpointSchema::new(
            "UserGetSignal1",
            20320,
            concat!(
                Filter::time(),
                Filter::symbol(false),
                vec![
                    Field::new("signal", Type::optional(Type::String)),
                    Field::new("min_level", Type::optional(Type::String)),
                ]
            ),
            vec![Field::new("data", Signal::signal_1())],
        ),
        EndpointSchema::new(
            "UserSubSignal1",
            20330,
            Filter::symbol(false),
            vec![Field::new("data", Signal::signal_1())],
        )
        .with_stream_response_type(Signal::signal_1()),
        EndpointSchema::new(
            "UserGetEncryptedKey",
            20340,
            vec![],
            vec![Field::new("data", encrypted_key())],
        ),
        EndpointSchema::new(
            "UserDeleteEncryptedKey",
            20350,
            vec![
                Field::new("exchange", Type::String),
                Field::new("account_id", Type::String),
            ],
            success_result(),
        ),
        EndpointSchema::new(
            "UserDecryptEncryptedKey",
            20360,
            vec![
                Field::new("encryption_key", Type::String),
                Field::new("exchange", Type::String),
                Field::new("account_id", Type::String),
            ],
            success_result(),
        ),
        EndpointSchema::new(
            "UserGetPriceDifference",
            20370,
            concat!(Filter::symbol(true), Filter::time()),
            vec![Field::new("data", Price::price_1())],
        ),
        EndpointSchema::new(
            "UserSubPriceDifference",
            20380,
            concat!(
                Filter::symbol(true),
                vec![Field::new("unsubscribe_other_symbol", Type::optional(Type::Boolean)),]
            ),
            vec![Field::new("data", Price::price_1())],
        ),
        EndpointSchema::new(
            "UserSubFundingRates",
            20390,
            vec![
                Field::new("exchange", Type::optional(Type::String)),
                Field::new("symbol", Type::optional(Type::String)),
                Field::new("unsub", Type::optional(Type::Boolean)),
            ],
            vec![Field::new("data", funding_rates())],
        )
        .with_stream_response_type(funding_rates()),
        EndpointSchema::new(
            "UserAddBlacklist",
            20400,
            vec![
                Field::new("strategy_id", Type::Int),
                Field::new("list", Request::set_symbol_flag()),
            ],
            success_result(),
        ),
        EndpointSchema::new(
            "UserRemoveBlacklist",
            20410,
            vec![
                Field::new("strategy_id", Type::Int),
                Field::new("list", Request::set_symbol_flag()),
            ],
            success_result(),
        ),
        EndpointSchema::new(
            "UserGetBlacklist",
            20420,
            vec![Field::new("strategy_id", Type::Int)],
            vec![Field::new("data", symbol_list())],
        ),
        // strategy 2
        // - symbol (get)
        // - price (get/sub)
        // - signal (get/sub)
        // - event (get/sub)
        // - blacklist (add/remove/get)
        EndpointSchema::new(
            "UserGetSymbol2",
            20430,
            Filter::symbol(false),
            vec![Field::new("data", symbol_list())],
        ),
        EndpointSchema::new(
            "UserGetBestBidAskAcrossExchanges",
            20440,
            concat!(
                Filter::symbol(false),
                Filter::time(),
                vec![Field::new("latest", Type::optional(Type::Boolean))]
            ),
            vec![Field::new("data", Price::price_spread())],
        ),
        EndpointSchema::new(
            "UserSubBestBidAskAcrossExchanges",
            20450,
            concat!(
                Filter::symbol(true),
                vec![Field::new("unsubscribe_other_symbol", Type::optional(Type::Boolean)),]
            ),
            vec![Field::new("data", Price::price_spread())],
        ),
        EndpointSchema::new(
            "UserGetSignal2",
            20460,
            concat!(
                Filter::time(),
                Filter::symbol(false),
                vec![
                    Field::new("signal", Type::optional(Type::String)),
                    Field::new("min_level", Type::optional(Type::String)),
                ]
            ),
            vec![Field::new("data", Signal::signal_2())],
        ),
        EndpointSchema::new(
            "UserSubSignal2",
            20470,
            Filter::symbol(false),
            vec![Field::new("data", Signal::signal_2())],
        )
        .with_stream_response_type(Signal::signal_2()),
        EndpointSchema::new(
            "UserPlaceOrderMarket",
            20520,
            vec![
                Field::new("exchange", Type::String),
                Field::new("symbol", Type::String),
                Field::new("side", Type::String),
                Field::new("price", Type::Numeric),
                Field::new("size", Type::Numeric),
                Field::new("local_id", Type::String),
            ],
            success_place_order_result(),
        ),
        EndpointSchema::new(
            "UserPlaceOrderLimit",
            20521,
            vec![
                Field::new("exchange", Type::String),
                Field::new("symbol", Type::String),
                Field::new("side", Type::String),
                Field::new("price", Type::Numeric),
                Field::new("size", Type::Numeric),
                Field::new("local_id", Type::String),
            ],
            success_place_order_result(),
        ),
        EndpointSchema::new(
            "UserS3CaptureEvent",
            20522,
            vec![Field::new("event_id", Type::BigInt)],
            success_place_order_result(),
        ),
        EndpointSchema::new(
            "UserS3ReleasePosition",
            20523,
            vec![Field::new("event_id", Type::BigInt)],
            success_place_order_result(),
        ),
        EndpointSchema::new(
            "UserSubStrategy3PositionsOpening",
            20524,
            vec![Field::new("unsubscribe", Type::optional(Type::Boolean))],
            vec![Field::new("data", user_captured_event())],
        )
        .with_stream_response_type(user_captured_event()),
        EndpointSchema::new(
            "UserSubStrategy3PositionsClosing",
            20525,
            vec![Field::new("unsubscribe", Type::optional(Type::Boolean))],
            vec![Field::new("data", user_captured_event())],
        )
        .with_stream_response_type(user_captured_event()),
        EndpointSchema::new(
            "UserCancelOrder",
            20530,
            vec![
                Field::new("exchange", Type::String),
                Field::new("symbol", Type::String),
                Field::new("local_id", Type::String),
            ],
            success_result(),
        ),
        EndpointSchema::new(
            "UserListTradingSymbols",
            20540,
            vec![],
            vec![Field::new("data", trading_symbol_list())],
        ),
        EndpointSchema::new(
            "UserGetLiveTestCloseOrder1",
            20550,
            vec![],
            vec![Field::new("data", live_test_close_order_price())],
        ),
        EndpointSchema::new(
            "UserSubExchangeLatency",
            20560,
            vec![
                Field::new("unsub", Type::optional(Type::Boolean)),
                Field::new("time_start", Type::optional(Type::TimeStampMs)),
                Field::new("time_end", Type::optional(Type::TimeStampMs)),
            ],
            vec![Field::new("data", user_benchmark_result())],
        )
        .with_stream_response_type(user_benchmark_result()),
        EndpointSchema::new(
            "SubS3TerminalBestAskBestBid",
            20610,
            concat!(
                Filter::symbol(true),
                vec![Field::new("unsubscribe_other_symbol", Type::optional(Type::Boolean)),]
            ),
            vec![Field::new("data", Price::price())],
        )
        .with_stream_response_type(Price::price()),
        EndpointSchema::new(
            "UserGetBestBidAskAcrossExchangesWithPositionEvent",
            20620,
            concat!(
                Filter::symbol(false),
                Filter::time(),
                vec![Field::new("id", Type::optional(Type::BigInt))]
            ),
            vec![Field::new("data", Price::price_spread_with_position())],
        ),
        EndpointSchema::new(
            "UserSubBestBidAskAcrossExchangesWithPositionEvent",
            20630,
            concat!(Filter::symbol(false)),
            vec![Field::new("data", Price::price_spread_with_position())],
        ),
        EndpointSchema::new(
            "UserGet5MinSpreadMean",
            20640,
            vec![],
            vec![Field::new("data", Price::spread())],
        ),
        EndpointSchema::new(
            "UserSetS2Configure",
            20650,
            vec![Field::new("configuration", user_set_s2_configure())],
            success_result(),
        ),
    ]
}
