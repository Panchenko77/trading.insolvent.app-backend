#![allow(non_camel_case_types)]

use crate::model::order::{parse_binance_order_type, BinanceOrderStatus};
use eyre::Result;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use trading_exchange_core::model::{
    AccountId, ExecutionResponse, TimeInForce, UpdateOrder, UpdatePosition, UpdatePositionSetValues, UpdatePositions,
};
use trading_model::core::{Time, TimeStampMs, NANOSECONDS_PER_MILLISECOND};
use trading_model::model::{Asset, Exchange, InstrumentCode, InstrumentManagerExt, SharedInstrumentManager, Symbol};

/**
type: private-api
url: GET /fapi/v2/account
ref: https://binance-docs.github.io/apidocs/futures/cn/#v2-user_data

```json
{
    "feeTier": 0,  // 手续费等级
    "canTrade": true,  // 是否可以交易
    "canDeposit": true,  // 是否可以入金z
    "canWithdraw": true, // 是否可以出金
    "updateTime": 0,
    "totalInitialMargin": "0.00000000",  // 但前所需起始保证金总额(存在逐仓请忽略), 仅计算usdt资产
    "totalMaintMargin": "0.00000000",  // 维持保证金总额, 仅计算usdt资产
    "totalWalletBalance": "23.72469206",   // 账户总余额, 仅计算usdt资产
    "totalUnrealizedProfit": "0.00000000",  // 持仓未实现盈亏总额, 仅计算usdt资产
    "totalMarginBalance": "23.72469206",  // 保证金总余额, 仅计算usdt资产
    "totalPositionInitialMargin": "0.00000000",  // 持仓所需起始保证金(基于最新标记价格), 仅计算usdt资产
    "totalOpenOrderInitialMargin": "0.00000000",  // 当前挂单所需起始保证金(基于最新标记价格), 仅计算usdt资产
    "totalCrossWalletBalance": "23.72469206",  // 全仓账户余额, 仅计算usdt资产
    "totalCrossUnPnl": "0.00000000",    // 全仓持仓未实现盈亏总额, 仅计算usdt资产
    "availableBalance": "23.72469206",       // 可用余额, 仅计算usdt资产
    "maxWithdrawAmount": "23.72469206"     // 最大可转出余额, 仅计算usdt资产
    "assets": [
        {
            "asset": "USDT",        //资产
            "walletBalance": "23.72469206",  //余额
            "unrealizedProfit": "0.00000000",  // 未实现盈亏
            "marginBalance": "23.72469206",  // 保证金余额
            "maintMargin": "0.00000000",    // 维持保证金
            "initialMargin": "0.00000000",  // 当前所需起始保证金
            "positionInitialMargin": "0.00000000",  // 持仓所需起始保证金(基于最新标记价格)
            "openOrderInitialMargin": "0.00000000", // 当前挂单所需起始保证金(基于最新标记价格)
            "crossWalletBalance": "23.72469206",  //全仓账户余额
            "crossUnPnl": "0.00000000" // 全仓持仓未实现盈亏
            "availableBalance": "23.72469206",       // 可用余额
            "maxWithdrawAmount": "23.72469206",     // 最大可转出余额
            "marginAvailable": true    // 是否可用作联合保证金
        },
        {
            "asset": "BUSD",        //资产
            "walletBalance": "103.12345678",  //余额
            "unrealizedProfit": "0.00000000",  // 未实现盈亏
            "marginBalance": "103.12345678",  // 保证金余额
            "maintMargin": "0.00000000",    // 维持保证金
            "initialMargin": "0.00000000",  // 当前所需起始保证金
            "positionInitialMargin": "0.00000000",  // 持仓所需起始保证金(基于最新标记价格)
            "openOrderInitialMargin": "0.00000000", // 当前挂单所需起始保证金(基于最新标记价格)
            "crossWalletBalance": "103.12345678",  //全仓账户余额
            "crossUnPnl": "0.00000000" // 全仓持仓未实现盈亏
            "availableBalance": "103.12345678",       // 可用余额
            "maxWithdrawAmount": "103.12345678",     // 最大可转出余额
            "marginAvailable": true    // 否可用作联合保证金
        }
    ],
    "positions": [  // 头寸，将返回所有市场symbol。
        //根据用户持仓模式展示持仓方向，即双向模式下只返回BOTH持仓情况，单向模式下只返回 LONG 和 SHORT 持仓情况
        {
            "symbol": "BTCUSDT",  // 交易对
            "initialMargin": "0",   // 当前所需起始保证金(基于最新标记价格)
            "maintMargin": "0", //维持保证金
            "unrealizedProfit": "0.00000000",  // 持仓未实现盈亏
            "positionInitialMargin": "0",  // 持仓所需起始保证金(基于最新标记价格)
            "openOrderInitialMargin": "0",  // 当前挂单所需起始保证金(基于最新标记价格)
            "leverage": "100",  // 杠杆倍率
            "isolated": true,  // 是否是逐仓模式
            "entryPrice": "0.00000",  // 持仓成本价
            "maxNotional": "250000",  // 当前杠杆下用户可用的最大名义价值
            "positionSide": "BOTH",  // 持仓方向
            "positionAmt": "0"      // 持仓数量
        }
    ]
}
```
 */
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsdmFuturesUserData {
    // "feeTier": 0,  // 手续费等级
    fee_tier: i32,
    // "canTrade": true,  // 是否可以交易
    can_trade: bool,
    // "canDeposit": true,  // 是否可以入金
    can_deposit: bool,
    // "canWithdraw": true, // 是否可以出金
    can_withdraw: bool,
    // "updateTime": 0,
    pub update_time: TimeStampMs,
    // "totalInitialMargin": "0.00000000",  // 但前所需起始保证金总额(存在逐仓请忽略), 仅计算usdt资产
    #[serde_as(as = "DisplayFromStr")]
    total_initial_margin: f64,
    // "totalMaintMargin": "0.00000000",  // 维持保证金总额, 仅计算usdt资产
    #[serde_as(as = "DisplayFromStr")]
    total_maint_margin: f64,
    // "totalWalletBalance": "23.72469206",   // 账户总余额, 仅计算usdt资产
    #[serde_as(as = "DisplayFromStr")]
    total_wallet_balance: f64,
    // "totalUnrealizedProfit": "0.00000000",  // 持仓未实现盈亏总额, 仅计算usdt资产
    #[serde_as(as = "DisplayFromStr")]
    total_unrealized_profit: f64,
    // "totalMarginBalance": "23.72469206",  // 保证金总余额, 仅计算usdt资产
    #[serde_as(as = "DisplayFromStr")]
    total_margin_balance: f64,
    // "totalPositionInitialMargin": "0.00000000",  // 持仓所需起始保证金(基于最新标记价格), 仅计算usdt资产
    #[serde_as(as = "DisplayFromStr")]
    total_position_initial_margin: f64,
    // "totalOpenOrderInitialMargin": "0.00000000",  // 当前挂单所需起始保证金(基于最新标记价格), 仅计算usdt资产
    #[serde_as(as = "DisplayFromStr")]
    total_open_order_initial_margin: f64,
    // "totalCrossWalletBalance": "23.72469206",  // 全仓账户余额, 仅计算usdt资产
    #[serde_as(as = "DisplayFromStr")]
    total_cross_wallet_balance: f64,
    // "totalCrossUnPnl": "0.00000000",    // 全仓持仓未实现盈亏总额, 仅计算usdt资产
    #[serde_as(as = "DisplayFromStr")]
    total_cross_un_pnl: f64,
    // "availableBalance": "23.72469206",       // 可用余额, 仅计算usdt资产
    #[serde_as(as = "DisplayFromStr")]
    available_balance: f64,
    // "maxWithdrawAmount": "23.72469206"     // 最大可转出余额, 仅计算usdt资产
    #[serde_as(as = "DisplayFromStr")]
    max_withdraw_amount: f64,
    pub assets: Vec<UserDataAssetEntry>,
    pub positions: Vec<UserDataPosition>,
}
// "asset": "USDT",        //资产
// "walletBalance": "23.72469206",  //余额
// "unrealizedProfit": "0.00000000",  // 未实现盈亏
// "marginBalance": "23.72469206",  // 保证金余额
// "maintMargin": "0.00000000",    // 维持保证金
// "initialMargin": "0.00000000",  // 当前所需起始保证金
// "positionInitialMargin": "0.00000000",  // 持仓所需起始保证金(基于最新标记价格)
// "openOrderInitialMargin": "0.00000000", // 当前挂单所需起始保证金(基于最新标记价格)
// "crossWalletBalance": "23.72469206",  //全仓账户余额
// "crossUnPnl": "0.00000000" // 全仓持仓未实现盈亏
// "availableBalance": "23.72469206",       // 可用余额
// "maxWithdrawAmount": "23.72469206",     // 最大可转出余额
// "marginAvailable": true    // 是否可用作联合保证金

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UserDataAssetEntry {
    pub asset: Asset,
    #[serde_as(as = "DisplayFromStr")]
    pub wallet_balance: f64,
    #[serde_as(as = "DisplayFromStr")]
    unrealized_profit: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub margin_balance: f64,
    #[serde_as(as = "DisplayFromStr")]
    maint_margin: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub initial_margin: f64,
    #[serde_as(as = "DisplayFromStr")]
    position_initial_margin: f64,
    #[serde_as(as = "DisplayFromStr")]
    open_order_initial_margin: f64,
    #[serde_as(as = "DisplayFromStr")]
    cross_wallet_balance: f64,
    #[serde_as(as = "DisplayFromStr")]
    cross_un_pnl: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub available_balance: f64,
    #[serde_as(as = "DisplayFromStr")]
    max_withdraw_amount: f64,
    margin_available: bool,
}

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct UserDataPosition {
    pub symbol: Symbol,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "initialMargin")]
    pub initial_margin: f64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "maintMargin")]
    pub maint_margin: f64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "unrealizedProfit")]
    pub unrealized_profit: f64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "positionInitialMargin")]
    pub position_initial_margin: f64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "openOrderInitialMargin")]
    pub open_order_initial_margin: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub leverage: f64,
    pub isolated: bool,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "entryPrice")]
    pub entry_price: f64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "maxNotional")]
    pub max_notional: f64,
    #[serde(rename = "positionSide")]
    pub position_side: String,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "positionAmt")]
    pub position_amt: f64,
}

/**
GET /fapi/v2/positionRisk (HMAC SHA256)

{
    "entryPrice": "0.00000", // 开仓均价
    "marginType": "isolated", // 逐仓模式或全仓模式
    "isAutoAddMargin": "false",
    "isolatedMargin": "0.00000000", // 逐仓保证金
    "leverage": "10", // 当前杠杆倍数
    "liquidationPrice": "0", // 参考强平价格
    "markPrice": "6679.50671178",   // 当前标记价格
    "maxNotionalValue": "20000000", // 当前杠杆倍数允许的名义价值上限
    "positionAmt": "0.000", // 头寸数量，符号代表多空方向, 正数为多，负数为空
    "symbol": "BTCUSDT", // 交易对
    "unRealizedProfit": "0.00000000", // 持仓未实现盈亏
    "positionSide": "BOTH", // 持仓方向
}
 */
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PositionRisk {
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "entryPrice")]
    pub entry_price: f64,
    #[serde(rename = "marginType")]
    pub margin_type: String,
    #[serde(rename = "isAutoAddMargin")]
    pub is_auto_add_margin: String,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "isolatedMargin")]
    pub isolated_margin: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub leverage: f64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "liquidationPrice")]
    pub liquidation_price: f64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "markPrice")]
    pub mark_price: f64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "maxNotionalValue")]
    pub max_notional_value: f64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "positionAmt")]
    pub position_amt: f64,
    pub symbol: String,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "unRealizedProfit")]
    pub un_realized_profit: f64,
    #[serde(rename = "positionSide")]
    pub position_side: String,
}

///
/// GET /fapi/v1/openOrders
///   {
///     "avgPrice": "0.00000",              // 平均成交价
///     "clientOrderId": "abc",             // 用户自定义的订单号
///     "cumQuote": "0",                        // 成交金额
///     "executedQty": "0",                 // 成交量
///     "orderId": 1917641,                 // 系统订单号
///     "origQty": "0.40",                  // 原始委托数量
///     "origType": "TRAILING_STOP_MARKET", // 触发前订单类型
///     "price": "0",                   // 委托价格
///     "reduceOnly": false,                // 是否仅减仓
///     "side": "BUY",                      // 买卖方向
///     "positionSide": "SHORT", // 持仓方向
///     "status": "NEW",                    // 订单状态
///     "stopPrice": "9300",                    // 触发价，对`TRAILING_STOP_MARKET`无效
///     "closePosition": false,             // 是否条件全平仓
///     "symbol": "BTCUSDT",                // 交易对
///     "time": 1579276756075,              // 订单时间
///     "timeInForce": "GTC",               // 有效方法
///     "type": "TRAILING_STOP_MARKET",     // 订单类型
///     "activatePrice": "9020", // 跟踪止损激活价格, 仅`TRAILING_STOP_MARKET` 订单返回此字段
///     "priceRate": "0.3", // 跟踪止损回调比例, 仅`TRAILING_STOP_MARKET` 订单返回此字段
///     "updateTime": 1579276756075,        // 更新时间
///     "workingType": "CONTRACT_PRICE", // 条件价格触发类型
///     "priceProtect": false            // 是否开启条件单触发保护
///   }
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct HttpLiveOrder {
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "avgPrice")]
    pub avg_price: f64,
    #[serde(rename = "clientOrderId")]
    pub client_order_id: String,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "cumQuote")]
    pub cum_quote: f64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "executedQty")]
    pub executed_qty: f64,
    #[serde(rename = "orderId")]
    pub order_id: i64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "origQty")]
    pub orig_qty: f64,
    #[serde(rename = "origType")]
    pub orig_type: String,
    #[serde_as(as = "DisplayFromStr")]
    pub price: f64,
    #[serde(rename = "reduceOnly")]
    pub reduce_only: bool,
    pub side: String,
    #[serde(rename = "positionSide")]
    pub position_side: String,
    pub status: String,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "stopPrice")]
    pub stop_price: f64,
    #[serde(rename = "closePosition")]
    pub close_position: bool,
    pub symbol: String,
    pub time: TimeStampMs,
    #[serde(rename = "timeInForce")]
    pub time_in_force: String,
    #[serde(rename = "type")]
    pub ty: String,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "activatePrice", default = "Default::default")]
    pub activate_price: f64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "priceRate", default = "Default::default")]
    pub price_rate: f64,
    #[serde(rename = "updateTime")]
    pub update_time: TimeStampMs,
    #[serde(rename = "workingType")]
    pub working_type: String,
    #[serde(rename = "priceProtect")]
    pub price_protect: bool,
}

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "e")]
enum USDMFuturesWebsocketData {
    listenKeyExpired {},
    /**
    ```json
        {
        "e":"MARGIN_CALL",      // 事件类型
        "E":1587727187525,      // 事件时间
        "cw":"3.16812045",      // 除去逐仓仓位保证金的钱包余额, 仅在全仓 margin call 情况下推送此字段
        "p":[                   // 涉及持仓
          {
            "s":"ETHUSDT",      // symbol
            "ps":"LONG",        // 持仓方向
            "pa":"1.327",       // 仓位
            "mt":"CROSSED",     // 保证金模式
            "iw":"0",           // 若为逐仓，仓位保证金
            "mp":"187.17127",   // 标记价格
            "up":"-1.166074",   // 未实现盈亏
            "mm":"1.614445"     // 持仓需要的维持保证金
          }
        ]
    }
    ```
     */
    MARGIN_CALL {
        #[serde(rename = "E")]
        e: TimeStampMs,
        #[serde_as(as = "DisplayFromStr")]
        cw: f64,
        p: Vec<MarginCallPositionInfo>,
    },
    /**
    ```json
    {
      "e": "ACCOUNT_UPDATE",                // 事件类型
      "E": 1564745798939,                   // 事件时间
      "T": 1564745798938 ,                  // 撮合时间
      "a":                                  // 账户更新事件
        {
          "m":"ORDER",                      // 事件推出原因
          "B":[                             // 余额信息
            {
              "a":"USDT",                   // 资产名称
              "wb":"122624.12345678",       // 钱包余额
              "cw":"100.12345678",          // 除去逐仓仓位保证金的钱包余额
              "bc":"50.12345678"            // 除去盈亏与交易手续费以外的钱包余额改变量
            }
          ],
          "P":[
           {
              "s":"BTCUSDT",            // 交易对
              "pa":"0",                 // 仓位
              "ep":"0.00000",            // 入仓价格
              "cr":"200",               // (费前)累计实现损益
              "up":"0",                     // 持仓未实现盈亏
              "mt":"isolated",              // 保证金模式
              "iw":"0.00000000",            // 若为逐仓，仓位保证金
              "ps":"BOTH"                   // 持仓方向
           }
          ]
        }
    }
    ```
     */
    ACCOUNT_UPDATE {
        #[serde(rename = "E")]
        event_time: TimeStampMs,
        #[serde(rename = "T")]
        transaction_time: TimeStampMs,
        #[serde(rename = "a")]
        event: AccountUpdateEvent,
    },

    /**
    ```json
        {

      "e":"ORDER_TRADE_UPDATE",         // 事件类型
      "E":1568879465651,                // 事件时间
      "T":1568879465650,                // 撮合时间
      "o":{
        "s":"BTCUSDT",                  // 交易对
        "c":"TEST",                     // 客户端自定订单ID
          // 特殊的自定义订单ID:
          // "autoclose-"开头的字符串: 系统强平订单
          // "adl_autoclose": ADL自动减仓订单
        "S":"SELL",                     // 订单方向
        "o":"TRAILING_STOP_MARKET", // 订单类型
        "f":"GTC",                      // 有效方式
        "q":"0.001",                    // 订单原始数量
        "p":"0",                        // 订单原始价格
        "ap":"0",                       // 订单平均价格
        "sp":"7103.04",                 // 条件订单触发价格，对追踪止损单无效
        "x":"NEW",                      // 本次事件的具体执行类型
        "X":"NEW",                      // 订单的当前状态
        "i":8886774,                    // 订单ID
        "l":"0",                        // 订单末次成交量
        "z":"0",                        // 订单累计已成交量
        "L":"0",                        // 订单末次成交价格
        "N": "USDT",                    // 手续费资产类型
        "n": "0",                       // 手续费数量
        "T":1568879465651,              // 成交时间
        "t":0,                          // 成交ID
        "b":"0",                        // 买单净值
        "a":"9.91",                     // 卖单净值
        "m": false,                     // 该成交是作为挂单成交吗？
        "R":false   ,                   // 是否是只减仓单
        "wt": "CONTRACT_PRICE",         // 触发价类型
        "ot": "TRAILING_STOP_MARKET",   // 原始订单类型
        "ps":"LONG"                     // 持仓方向
        "cp":false,                     // 是否为触发平仓单; 仅在条件订单情况下会推送此字段
        "AP":"7476.89",                 // 追踪止损激活价格, 仅在追踪止损单时会推送此字段
        "cr":"5.0",                     // 追踪止损回调比例, 仅在追踪止损单时会推送此字段
        "rp":"0"                            // 该交易实现盈亏

      }

    }
    ```
     */
    ORDER_TRADE_UPDATE {
        #[serde(rename = "E")]
        event_time: TimeStampMs,
        #[serde(rename = "T")]
        transaction_time: TimeStampMs,
        #[serde(rename = "o")]
        order: OrderTradeUpdateOrder,
    },
    ACCOUNT_CONFIG_UPDATE {
        #[serde(rename = "E")]
        event_time: TimeStampMs,
        #[serde(rename = "T")]
        transaction_time: TimeStampMs,
        // ai, ac
    },
}

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MarginCallPositionInfo {
    pub s: String,
    pub ps: String,
    #[serde_as(as = "DisplayFromStr")]
    pub pa: f64,
    pub mt: String,
    #[serde_as(as = "DisplayFromStr")]
    pub iw: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub mp: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub up: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub mm: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct AccountUpdateEvent {
    #[serde(rename = "m")]
    pub reason: String,
    #[serde(rename = "B")]
    pub balances: Vec<BalanceInfo>,
    #[serde(rename = "P")]
    pub positions: Vec<PositionInfo>,
}

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct BalanceInfo {
    #[serde(rename = "a")]
    pub asset: String,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "wb")]
    pub wallet_balance: f64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "cw")]
    pub available_balance: f64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "bc")]
    pub balance_change: f64,
}

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct PositionInfo {
    #[serde(rename = "s")]
    pub symbol: Symbol,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "pa")]
    pub position: f64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "ep")]
    pub entry_price: f64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "cr")]
    pub current_unrealized_profit: f64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "up")]
    pub unrealized_profit: f64,
    #[serde(rename = "mt")]
    pub margin_type: String,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "iw")]
    pub isolated_wallet: f64,
    #[serde(rename = "ps")]
    pub position_side: String,
}

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct OrderTradeUpdateOrder {
    #[serde(rename = "s")]
    pub symbol: Symbol,
    #[serde(rename = "c")]
    pub client_oid: String,
    #[serde(rename = "S")]
    pub side: String,
    #[serde(rename = "o")]
    pub order_type: String,
    #[serde(rename = "f")]
    pub valid_method: String,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "q")]
    pub size: f64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "p")]
    pub price: f64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "ap")]
    pub avg_price: f64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "sp")]
    pub stop_price: f64,
    #[serde(rename = "x")]
    pub execution_type: String,
    #[serde(rename = "X")]
    pub order_status: BinanceOrderStatus,
    #[serde(rename = "i")]
    pub server_oid: i64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "l")]
    pub last_filled: f64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "z")]
    pub filled: f64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "L")]
    pub last_price: f64,
    // #[serde(rename = "N")]
    // pub fee_symbol: String,
    // #[serde_as(as = "DisplayFromStr")]
    // #[serde(rename = "n")]
    // pub fee_size: f64,
    #[serde(rename = "T")]
    pub trans_time: i64,
    #[serde(rename = "t")]
    pub trans_id: i64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "b")]
    pub buy_size: f64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "a")]
    pub sell_size: f64,
    #[serde(rename = "m")]
    pub is_maker: bool,
    #[serde(rename = "R")]
    pub reduce_only: bool,
    #[serde(rename = "wt")]
    pub trigger_type: String,
    #[serde(rename = "ot")]
    pub orig_order_type: String,
    #[serde(rename = "ps")]
    pub position_side: String,
    #[serde(rename = "cp")]
    pub is_liquefied: bool,
    // #[serde_as(as = "DisplayFromStr")]
    // #[serde(rename = "AP")]
    // pub ap: f64,
    // #[serde_as(as = "DisplayFromStr")]
    // #[serde(rename = "cr")]
    // pub cr: f64,
    // #[serde_as(as = "DisplayFromStr")]
    // #[serde(rename = "rp")]
    // pub realized_profit: f64,
}

pub fn decode_binance_usdm_futures_websocket_message(
    account: AccountId,
    data: &str,
    manager: Option<SharedInstrumentManager>,
) -> Result<Option<ExecutionResponse>> {
    let exchange = Exchange::BinanceFutures;
    // info!("{}", data);
    let data: USDMFuturesWebsocketData = serde_json::from_str(data)?;
    match data {
        USDMFuturesWebsocketData::listenKeyExpired { .. }
        | USDMFuturesWebsocketData::ACCOUNT_CONFIG_UPDATE { .. }
        | USDMFuturesWebsocketData::MARGIN_CALL { .. } => Ok(None),
        USDMFuturesWebsocketData::ACCOUNT_UPDATE {
            event_time,
            transaction_time,
            event,
        } => {
            let mut update = UpdatePositions::update(account, exchange);
            update.extend_updates(event.balances.into_iter().map(|b| {
                UpdatePosition {
                    account,
                    instrument: InstrumentCode::from_asset(Exchange::BinanceFutures, b.asset.into()),

                    times: (
                        event_time * NANOSECONDS_PER_MILLISECOND,
                        transaction_time * NANOSECONDS_PER_MILLISECOND,
                    )
                        .into(),
                    set_values: Some(UpdatePositionSetValues {
                        available: b.available_balance,
                        locked: b.wallet_balance - b.available_balance,
                        total: b.wallet_balance,
                    }),
                    ..UpdatePosition::empty()
                }
            }));
            update.extend_updates(event.positions.into_iter().map(|x| {
                let instrument = InstrumentCode::from_symbol(Exchange::BinanceFutures, x.symbol.as_str().into());
                UpdatePosition {
                    account,
                    instrument,
                    times: (
                        event_time * NANOSECONDS_PER_MILLISECOND,
                        transaction_time * NANOSECONDS_PER_MILLISECOND,
                    )
                        .into(),
                    set_values: Some(UpdatePositionSetValues {
                        total: x.position,
                        available: x.position,
                        locked: 0.0,
                    }),
                    entry_price: Some(x.entry_price),
                    ..UpdatePosition::empty()
                }
            }));
            Ok(Some(ExecutionResponse::UpdatePositions(update)))
        }
        USDMFuturesWebsocketData::ORDER_TRADE_UPDATE {
            event_time,
            transaction_time,
            order,
        } => {
            let instrument = manager.maybe_lookup_instrument(exchange, order.symbol);
            let update_order = UpdateOrder {
                account,
                instrument,
                tif: TimeInForce::GoodTilCancel,
                server_id: order.server_oid.to_string().as_str().into(),
                client_id: order.client_oid.into(),
                status: order.order_status.into(),
                side: order.side.parse()?,
                ty: parse_binance_order_type(exchange, &order.order_type)?,
                price: order.price,
                size: order.size,
                filled_size: order.filled,
                update_lt: Time::now(),
                update_est: Time::from_millis(event_time),
                update_tst: Time::from_millis(transaction_time),
                average_filled_price: order.avg_price,
                last_filled_size: order.last_filled,
                last_filled_price: order.last_price,
                stop_price: order.stop_price,
                ..UpdateOrder::empty()
            };
            Ok(Some(ExecutionResponse::UpdateOrder(update_order)))
        }
    }
}
