use lib::error_code::ErrorCode;
use lib::types::*;
use lib::ws::*;
use num_derive::FromPrimitive;
use rust_decimal::Decimal;
use serde::*;
use strum_macros::{Display, EnumString};
use tokio_postgres::types::*;

#[derive(
    Debug,
    Clone,
    Copy,
    ToSql,
    FromSql,
    Serialize,
    Deserialize,
    FromPrimitive,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    EnumString,
    Display,
    Hash,
)]
#[postgres(name = "enum_role")]
pub enum EnumRole {
    ///
    #[postgres(name = "guest")]
    Guest = 0,
    ///
    #[postgres(name = "user")]
    User = 1,
    ///
    #[postgres(name = "trader")]
    Trader = 2,
    ///
    #[postgres(name = "developer")]
    Developer = 3,
    ///
    #[postgres(name = "admin")]
    Admin = 4,
}
#[derive(
    Debug,
    Clone,
    Copy,
    ToSql,
    FromSql,
    Serialize,
    Deserialize,
    FromPrimitive,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    EnumString,
    Display,
    Hash,
)]
#[postgres(name = "enum_block_chain")]
pub enum EnumBlockChain {
    ///
    #[postgres(name = "EthereumMainnet")]
    EthereumMainnet = 0,
    ///
    #[postgres(name = "EthereumGoerli")]
    EthereumGoerli = 1,
    ///
    #[postgres(name = "BscMainnet")]
    BscMainnet = 2,
    ///
    #[postgres(name = "BscTestnet")]
    BscTestnet = 3,
    ///
    #[postgres(name = "LocalNet")]
    LocalNet = 4,
    ///
    #[postgres(name = "EthereumSepolia")]
    EthereumSepolia = 5,
}
#[derive(
    Debug,
    Clone,
    Copy,
    ToSql,
    FromSql,
    Serialize,
    Deserialize,
    FromPrimitive,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    EnumString,
    Display,
    Hash,
)]
#[postgres(name = "enum_dex")]
pub enum EnumDex {
    ///
    #[postgres(name = "UniSwap")]
    UniSwap = 0,
    ///
    #[postgres(name = "PancakeSwap")]
    PancakeSwap = 1,
    ///
    #[postgres(name = "SushiSwap")]
    SushiSwap = 2,
}
#[derive(
    Debug,
    Clone,
    Copy,
    ToSql,
    FromSql,
    Serialize,
    Deserialize,
    FromPrimitive,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    EnumString,
    Display,
    Hash,
)]
#[postgres(name = "enum_dex_path_format")]
pub enum EnumDexPathFormat {
    ///
    #[postgres(name = "Json")]
    Json = 0,
    ///
    #[postgres(name = "TransactionData")]
    TransactionData = 1,
    ///
    #[postgres(name = "TransactionHash")]
    TransactionHash = 2,
}
#[derive(
    Debug,
    Clone,
    Copy,
    ToSql,
    FromSql,
    Serialize,
    Deserialize,
    FromPrimitive,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    EnumString,
    Display,
    Hash,
)]
#[postgres(name = "enum_service")]
pub enum EnumService {
    ///
    #[postgres(name = "auth")]
    Auth = 1,
    ///
    #[postgres(name = "user")]
    User = 2,
}
#[derive(
    Debug,
    Clone,
    Copy,
    ToSql,
    FromSql,
    Serialize,
    Deserialize,
    FromPrimitive,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    EnumString,
    Display,
    Hash,
)]
#[postgres(name = "enum_Endpoint")]
pub enum EnumEndpoint {
    ///
    #[postgres(name = "Login")]
    Login = 10020,
    ///
    #[postgres(name = "Signup")]
    Signup = 10010,
    ///
    #[postgres(name = "Authorize")]
    Authorize = 10030,
    ///
    #[postgres(name = "Logout")]
    Logout = 10040,
    ///
    #[postgres(name = "UserStatus")]
    UserStatus = 20000,
    ///
    #[postgres(name = "UserSubLogs")]
    UserSubLogs = 20010,
    ///
    #[postgres(name = "UserSubEvents")]
    UserSubEvents = 20020,
    ///
    #[postgres(name = "UserSubPosition")]
    UserSubPosition = 20030,
    ///
    #[postgres(name = "UserCancelOrClosePosition")]
    UserCancelOrClosePosition = 20031,
    ///
    #[postgres(name = "UserSubOrders")]
    UserSubOrders = 20040,
    ///
    #[postgres(name = "UserListStrategy")]
    UserListStrategy = 20100,
    ///
    #[postgres(name = "UserInitStrategy")]
    UserInitStrategy = 20110,
    ///
    #[postgres(name = "UserSubPrice0")]
    UserSubPrice0 = 20120,
    ///
    #[postgres(name = "UserGetPrice0")]
    UserGetPrice0 = 20130,
    ///
    #[postgres(name = "UserControlStrategy")]
    UserControlStrategy = 20140,
    ///
    #[postgres(name = "UserGetStrategyZeroSymbol")]
    UserGetStrategyZeroSymbol = 20150,
    ///
    #[postgres(name = "UserSubSignal0")]
    UserSubSignal0 = 20160,
    ///
    #[postgres(name = "UserGetSignal0")]
    UserGetSignal0 = 20170,
    ///
    #[postgres(name = "UserGetDebugLog")]
    UserGetDebugLog = 20180,
    ///
    #[postgres(name = "UserSetEncryptedKey")]
    UserSetEncryptedKey = 21000,
    ///
    #[postgres(name = "UserStartService")]
    UserStartService = 21010,
    ///
    #[postgres(name = "UserSetStrategyStatus")]
    UserSetStrategyStatus = 21020,
    ///
    #[postgres(name = "UserGetStrategyOneSymbol")]
    UserGetStrategyOneSymbol = 20200,
    ///
    #[postgres(name = "UserSetSymbolFlag1")]
    UserSetSymbolFlag1 = 20210,
    ///
    #[postgres(name = "UserGetEvent1")]
    UserGetEvent1 = 20240,
    ///
    #[postgres(name = "UserSubEvent1")]
    UserSubEvent1 = 20250,
    ///
    #[postgres(name = "UserGetStrategyOneAccuracy")]
    UserGetStrategyOneAccuracy = 20260,
    ///
    #[postgres(name = "UserGetAccuracy")]
    UserGetAccuracy = 20261,
    ///
    #[postgres(name = "UserGetOrdersPerStrategy")]
    UserGetOrdersPerStrategy = 20271,
    ///
    #[postgres(name = "UserSubStrategyOneOrder")]
    UserSubStrategyOneOrder = 20280,
    ///
    #[postgres(name = "UserGetLedger")]
    UserGetLedger = 20291,
    ///
    #[postgres(name = "UserGetHedgedOrders")]
    UserGetHedgedOrders = 20292,
    ///
    #[postgres(name = "UserSubLedgerStrategyOne")]
    UserSubLedgerStrategyOne = 20300,
    ///
    #[postgres(name = "UserSubLedger")]
    UserSubLedger = 20301,
    ///
    #[postgres(name = "UserGetLiveTestAccuracyLog")]
    UserGetLiveTestAccuracyLog = 20310,
    ///
    #[postgres(name = "UserGetSignal1")]
    UserGetSignal1 = 20320,
    ///
    #[postgres(name = "UserSubSignal1")]
    UserSubSignal1 = 20330,
    ///
    #[postgres(name = "UserGetEncryptedKey")]
    UserGetEncryptedKey = 20340,
    ///
    #[postgres(name = "UserDeleteEncryptedKey")]
    UserDeleteEncryptedKey = 20350,
    ///
    #[postgres(name = "UserDecryptEncryptedKey")]
    UserDecryptEncryptedKey = 20360,
    ///
    #[postgres(name = "UserGetPriceDifference")]
    UserGetPriceDifference = 20370,
    ///
    #[postgres(name = "UserSubPriceDifference")]
    UserSubPriceDifference = 20380,
    ///
    #[postgres(name = "UserSubFundingRates")]
    UserSubFundingRates = 20390,
    ///
    #[postgres(name = "UserAddBlacklist")]
    UserAddBlacklist = 20400,
    ///
    #[postgres(name = "UserRemoveBlacklist")]
    UserRemoveBlacklist = 20410,
    ///
    #[postgres(name = "UserGetBlacklist")]
    UserGetBlacklist = 20420,
    ///
    #[postgres(name = "UserGetSymbol2")]
    UserGetSymbol2 = 20430,
    ///
    #[postgres(name = "UserGetBestBidAskAcrossExchanges")]
    UserGetBestBidAskAcrossExchanges = 20440,
    ///
    #[postgres(name = "UserSubBestBidAskAcrossExchanges")]
    UserSubBestBidAskAcrossExchanges = 20450,
    ///
    #[postgres(name = "UserGetSignal2")]
    UserGetSignal2 = 20460,
    ///
    #[postgres(name = "UserSubSignal2")]
    UserSubSignal2 = 20470,
    ///
    #[postgres(name = "UserPlaceOrderMarket")]
    UserPlaceOrderMarket = 20520,
    ///
    #[postgres(name = "UserPlaceOrderLimit")]
    UserPlaceOrderLimit = 20521,
    ///
    #[postgres(name = "UserS3CaptureEvent")]
    UserS3CaptureEvent = 20522,
    ///
    #[postgres(name = "UserS3ReleasePosition")]
    UserS3ReleasePosition = 20523,
    ///
    #[postgres(name = "UserSubStrategy3PositionsOpening")]
    UserSubStrategy3PositionsOpening = 20524,
    ///
    #[postgres(name = "UserSubStrategy3PositionsClosing")]
    UserSubStrategy3PositionsClosing = 20525,
    ///
    #[postgres(name = "UserCancelOrder")]
    UserCancelOrder = 20530,
    ///
    #[postgres(name = "UserListTradingSymbols")]
    UserListTradingSymbols = 20540,
    ///
    #[postgres(name = "UserGetLiveTestCloseOrder1")]
    UserGetLiveTestCloseOrder1 = 20550,
    ///
    #[postgres(name = "UserSubExchangeLatency")]
    UserSubExchangeLatency = 20560,
    ///
    #[postgres(name = "SubS3TerminalBestAskBestBid")]
    SubS3TerminalBestAskBestBid = 20610,
    ///
    #[postgres(name = "UserGetBestBidAskAcrossExchangesWithPositionEvent")]
    UserGetBestBidAskAcrossExchangesWithPositionEvent = 20620,
    ///
    #[postgres(name = "UserSubBestBidAskAcrossExchangesWithPositionEvent")]
    UserSubBestBidAskAcrossExchangesWithPositionEvent = 20630,
    ///
    #[postgres(name = "UserGet5MinSpreadMean")]
    UserGet5MinSpreadMean = 20640,
    ///
    #[postgres(name = "UserSetS2Configure")]
    UserSetS2Configure = 20650,
}

impl EnumEndpoint {
    pub fn schema(&self) -> ::endpoint_gen::model::EndpointSchema {
        let schema = match self {
            Self::Login => LoginRequest::SCHEMA,
            Self::Signup => SignupRequest::SCHEMA,
            Self::Authorize => AuthorizeRequest::SCHEMA,
            Self::Logout => LogoutRequest::SCHEMA,
            Self::UserStatus => UserStatusRequest::SCHEMA,
            Self::UserSubLogs => UserSubLogsRequest::SCHEMA,
            Self::UserSubEvents => UserSubEventsRequest::SCHEMA,
            Self::UserSubPosition => UserSubPositionRequest::SCHEMA,
            Self::UserCancelOrClosePosition => UserCancelOrClosePositionRequest::SCHEMA,
            Self::UserSubOrders => UserSubOrdersRequest::SCHEMA,
            Self::UserListStrategy => UserListStrategyRequest::SCHEMA,
            Self::UserInitStrategy => UserInitStrategyRequest::SCHEMA,
            Self::UserSubPrice0 => UserSubPrice0Request::SCHEMA,
            Self::UserGetPrice0 => UserGetPrice0Request::SCHEMA,
            Self::UserControlStrategy => UserControlStrategyRequest::SCHEMA,
            Self::UserGetStrategyZeroSymbol => UserGetStrategyZeroSymbolRequest::SCHEMA,
            Self::UserSubSignal0 => UserSubSignal0Request::SCHEMA,
            Self::UserGetSignal0 => UserGetSignal0Request::SCHEMA,
            Self::UserGetDebugLog => UserGetDebugLogRequest::SCHEMA,
            Self::UserSetEncryptedKey => UserSetEncryptedKeyRequest::SCHEMA,
            Self::UserStartService => UserStartServiceRequest::SCHEMA,
            Self::UserSetStrategyStatus => UserSetStrategyStatusRequest::SCHEMA,
            Self::UserGetStrategyOneSymbol => UserGetStrategyOneSymbolRequest::SCHEMA,
            Self::UserSetSymbolFlag1 => UserSetSymbolFlag1Request::SCHEMA,
            Self::UserGetEvent1 => UserGetEvent1Request::SCHEMA,
            Self::UserSubEvent1 => UserSubEvent1Request::SCHEMA,
            Self::UserGetStrategyOneAccuracy => UserGetStrategyOneAccuracyRequest::SCHEMA,
            Self::UserGetAccuracy => UserGetAccuracyRequest::SCHEMA,
            Self::UserGetOrdersPerStrategy => UserGetOrdersPerStrategyRequest::SCHEMA,
            Self::UserSubStrategyOneOrder => UserSubStrategyOneOrderRequest::SCHEMA,
            Self::UserGetLedger => UserGetLedgerRequest::SCHEMA,
            Self::UserGetHedgedOrders => UserGetHedgedOrdersRequest::SCHEMA,
            Self::UserSubLedgerStrategyOne => UserSubLedgerStrategyOneRequest::SCHEMA,
            Self::UserSubLedger => UserSubLedgerRequest::SCHEMA,
            Self::UserGetLiveTestAccuracyLog => UserGetLiveTestAccuracyLogRequest::SCHEMA,
            Self::UserGetSignal1 => UserGetSignal1Request::SCHEMA,
            Self::UserSubSignal1 => UserSubSignal1Request::SCHEMA,
            Self::UserGetEncryptedKey => UserGetEncryptedKeyRequest::SCHEMA,
            Self::UserDeleteEncryptedKey => UserDeleteEncryptedKeyRequest::SCHEMA,
            Self::UserDecryptEncryptedKey => UserDecryptEncryptedKeyRequest::SCHEMA,
            Self::UserGetPriceDifference => UserGetPriceDifferenceRequest::SCHEMA,
            Self::UserSubPriceDifference => UserSubPriceDifferenceRequest::SCHEMA,
            Self::UserSubFundingRates => UserSubFundingRatesRequest::SCHEMA,
            Self::UserAddBlacklist => UserAddBlacklistRequest::SCHEMA,
            Self::UserRemoveBlacklist => UserRemoveBlacklistRequest::SCHEMA,
            Self::UserGetBlacklist => UserGetBlacklistRequest::SCHEMA,
            Self::UserGetSymbol2 => UserGetSymbol2Request::SCHEMA,
            Self::UserGetBestBidAskAcrossExchanges => UserGetBestBidAskAcrossExchangesRequest::SCHEMA,
            Self::UserSubBestBidAskAcrossExchanges => UserSubBestBidAskAcrossExchangesRequest::SCHEMA,
            Self::UserGetSignal2 => UserGetSignal2Request::SCHEMA,
            Self::UserSubSignal2 => UserSubSignal2Request::SCHEMA,
            Self::UserPlaceOrderMarket => UserPlaceOrderMarketRequest::SCHEMA,
            Self::UserPlaceOrderLimit => UserPlaceOrderLimitRequest::SCHEMA,
            Self::UserS3CaptureEvent => UserS3CaptureEventRequest::SCHEMA,
            Self::UserS3ReleasePosition => UserS3ReleasePositionRequest::SCHEMA,
            Self::UserSubStrategy3PositionsOpening => UserSubStrategy3PositionsOpeningRequest::SCHEMA,
            Self::UserSubStrategy3PositionsClosing => UserSubStrategy3PositionsClosingRequest::SCHEMA,
            Self::UserCancelOrder => UserCancelOrderRequest::SCHEMA,
            Self::UserListTradingSymbols => UserListTradingSymbolsRequest::SCHEMA,
            Self::UserGetLiveTestCloseOrder1 => UserGetLiveTestCloseOrder1Request::SCHEMA,
            Self::UserSubExchangeLatency => UserSubExchangeLatencyRequest::SCHEMA,
            Self::SubS3TerminalBestAskBestBid => SubS3TerminalBestAskBestBidRequest::SCHEMA,
            Self::UserGetBestBidAskAcrossExchangesWithPositionEvent => {
                UserGetBestBidAskAcrossExchangesWithPositionEventRequest::SCHEMA
            }
            Self::UserSubBestBidAskAcrossExchangesWithPositionEvent => {
                UserSubBestBidAskAcrossExchangesWithPositionEventRequest::SCHEMA
            }
            Self::UserGet5MinSpreadMean => UserGet5MinSpreadMeanRequest::SCHEMA,
            Self::UserSetS2Configure => UserSetS2ConfigureRequest::SCHEMA,
        };
        serde_json::from_str(schema).unwrap()
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorBadRequest {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorInternalServerError {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorNotImplemented {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorNotFound {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorDatabaseError {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorInvalidService {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorUserForbidden {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorUserNotFound {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorUserMustAgreeTos {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorUserMustAgreePrivacyPolicy {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorUserNoAuthToken {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorUserInvalidAuthToken {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorTokenNotTop25 {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorImmutableStrategy {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorUserWhitelistedWalletNotSameNetworkAsStrategy {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorDuplicateRequest {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorInvalidExpression {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorInvalidEnumLevel {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorError {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorInvalidArgument {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorInvalidState {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorInvalidSeq {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorInvalidMethod {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorProtocolViolation {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorMalformedRequest {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorUnknownUser {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorBlockedUser {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorInvalidPassword {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorInvalidToken {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorTemporarilyUnavailable {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorUnexpectedException {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorBackPressureIncreased {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorInvalidPublicId {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorInvalidRange {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorBankAccountAlreadyExists {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorInsufficientFunds {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorLogicalError {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorRestrictedUserPrivileges {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorIdenticalReplacement {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorInvalidRecoveryQuestions {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorInvalidRole {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorWrongRecoveryAnswers {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorMessageNotDelivered {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorNoReply {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorNullAttribute {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorConsentMissing {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorActiveSubscriptionRequired {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorUsernameAlreadyRegistered {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorRecoveryQuestionsNotSet {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorMustSubmitAllRecoveryQuestions {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorInvalidRecoveryToken {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorRoutingError {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorUnauthorizedMessage {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorAuthError {}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorInternalError {}
#[derive(
    Debug,
    Clone,
    Copy,
    ToSql,
    FromSql,
    Serialize,
    Deserialize,
    FromPrimitive,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    EnumString,
    Display,
    Hash,
)]
#[postgres(name = "enum_ErrorCode")]
pub enum EnumErrorCode {
    /// Custom Bad Request
    #[postgres(name = "BadRequest")]
    BadRequest = 100400,
    /// Custom Internal Server Error
    #[postgres(name = "InternalServerError")]
    InternalServerError = 100500,
    /// Custom Method not implemented
    #[postgres(name = "NotImplemented")]
    NotImplemented = 100501,
    /// Custom NotFoundResource
    #[postgres(name = "NotFound")]
    NotFound = 100404,
    /// Custom Database error
    #[postgres(name = "DatabaseError")]
    DatabaseError = 100601,
    /// Custom Invalid Service
    #[postgres(name = "InvalidService")]
    InvalidService = 100602,
    /// Custom Insufficient role for user
    #[postgres(name = "UserForbidden")]
    UserForbidden = 101403,
    /// Custom User not found
    #[postgres(name = "UserNotFound")]
    UserNotFound = 101404,
    /// Custom Must agree to the terms of service
    #[postgres(name = "UserMustAgreeTos")]
    UserMustAgreeTos = 101601,
    /// Custom Must agree to the privacy policy
    #[postgres(name = "UserMustAgreePrivacyPolicy")]
    UserMustAgreePrivacyPolicy = 101602,
    /// Custom No auth token
    #[postgres(name = "UserNoAuthToken")]
    UserNoAuthToken = 101604,
    /// Custom token invalid
    #[postgres(name = "UserInvalidAuthToken")]
    UserInvalidAuthToken = 101605,
    /// Audit Token is not top 25
    #[postgres(name = "TokenNotTop25")]
    TokenNotTop25 = 102602,
    /// Audit Strategy is immutable
    #[postgres(name = "ImmutableStrategy")]
    ImmutableStrategy = 102603,
    /// Audit User whitelisted wallet not same network as strategy
    #[postgres(name = "UserWhitelistedWalletNotSameNetworkAsStrategy")]
    UserWhitelistedWalletNotSameNetworkAsStrategy = 102604,
    /// Custom Duplicate request
    #[postgres(name = "DuplicateRequest")]
    DuplicateRequest = 103001,
    /// Custom Invalid expression
    #[postgres(name = "InvalidExpression")]
    InvalidExpression = 104000,
    /// SQL 22P02 InvalidEnumLevel
    #[postgres(name = "InvalidEnumLevel")]
    InvalidEnumLevel = 3484946,
    /// SQL R0000 Error
    #[postgres(name = "Error")]
    Error = 4349632,
    /// SQL R0001 InvalidArgument
    #[postgres(name = "InvalidArgument")]
    InvalidArgument = 45349633,
    /// SQL R0002 InvalidState
    #[postgres(name = "InvalidState")]
    InvalidState = 45349634,
    /// SQL R0003 InvalidSeq
    #[postgres(name = "InvalidSeq")]
    InvalidSeq = 45349635,
    /// SQL R0004 InvalidMethod
    #[postgres(name = "InvalidMethod")]
    InvalidMethod = 45349636,
    /// SQL R0005 ProtocolViolation
    #[postgres(name = "ProtocolViolation")]
    ProtocolViolation = 45349637,
    /// SQL R0006 MalformedRequest
    #[postgres(name = "MalformedRequest")]
    MalformedRequest = 45349638,
    /// SQL R0007 UnknownUser
    #[postgres(name = "UnknownUser")]
    UnknownUser = 45349639,
    /// SQL R0008 BlockedUser
    #[postgres(name = "BlockedUser")]
    BlockedUser = 45349640,
    /// SQL R0009 InvalidPassword
    #[postgres(name = "InvalidPassword")]
    InvalidPassword = 45349641,
    /// SQL R000A InvalidToken
    #[postgres(name = "InvalidToken")]
    InvalidToken = 45349642,
    /// SQL R000B TemporarilyUnavailable
    #[postgres(name = "TemporarilyUnavailable")]
    TemporarilyUnavailable = 45349643,
    /// SQL R000C UnexpectedException
    #[postgres(name = "UnexpectedException")]
    UnexpectedException = 45349644,
    /// SQL R000D BackPressureIncreased
    #[postgres(name = "BackPressureIncreased")]
    BackPressureIncreased = 45349645,
    /// SQL R000E InvalidPublicId
    #[postgres(name = "InvalidPublicId")]
    InvalidPublicId = 45349646,
    /// SQL R000F InvalidRange
    #[postgres(name = "InvalidRange")]
    InvalidRange = 45349647,
    /// SQL R000G BankAccountAlreadyExists
    #[postgres(name = "BankAccountAlreadyExists")]
    BankAccountAlreadyExists = 45349648,
    /// SQL R000H InsufficientFunds
    #[postgres(name = "InsufficientFunds")]
    InsufficientFunds = 45349649,
    /// SQL R000M LogicalError
    #[postgres(name = "LogicalError")]
    LogicalError = 45349654,
    /// SQL R000N RestrictedUserPrivileges
    #[postgres(name = "RestrictedUserPrivileges")]
    RestrictedUserPrivileges = 45349655,
    /// SQL R000O IdenticalReplacement
    #[postgres(name = "IdenticalReplacement")]
    IdenticalReplacement = 45349656,
    /// SQL R000R InvalidRecoveryQuestions
    #[postgres(name = "InvalidRecoveryQuestions")]
    InvalidRecoveryQuestions = 45349659,
    /// SQL R000S InvalidRole
    #[postgres(name = "InvalidRole")]
    InvalidRole = 45349660,
    /// SQL R000T WrongRecoveryAnswers
    #[postgres(name = "WrongRecoveryAnswers")]
    WrongRecoveryAnswers = 45349661,
    /// SQL R000U MessageNotDelivered
    #[postgres(name = "MessageNotDelivered")]
    MessageNotDelivered = 45349662,
    /// SQL R000V NoReply
    #[postgres(name = "NoReply")]
    NoReply = 45349663,
    /// SQL R000W NullAttribute
    #[postgres(name = "NullAttribute")]
    NullAttribute = 45349664,
    /// SQL R000X ConsentMissing
    #[postgres(name = "ConsentMissing")]
    ConsentMissing = 45349665,
    /// SQL R000Y ActiveSubscriptionRequired
    #[postgres(name = "ActiveSubscriptionRequired")]
    ActiveSubscriptionRequired = 45349666,
    /// SQL R000Z UsernameAlreadyRegistered
    #[postgres(name = "UsernameAlreadyRegistered")]
    UsernameAlreadyRegistered = 45349667,
    /// SQL R0010 RecoveryQuestionsNotSet
    #[postgres(name = "RecoveryQuestionsNotSet")]
    RecoveryQuestionsNotSet = 45349668,
    /// SQL R0011 MustSubmitAllRecoveryQuestions
    #[postgres(name = "MustSubmitAllRecoveryQuestions")]
    MustSubmitAllRecoveryQuestions = 45349669,
    /// SQL R0012 InvalidRecoveryToken
    #[postgres(name = "InvalidRecoveryToken")]
    InvalidRecoveryToken = 45349670,
    /// SQL R0018 RoutingError
    #[postgres(name = "RoutingError")]
    RoutingError = 45349676,
    /// SQL R0019 UnauthorizedMessage
    #[postgres(name = "UnauthorizedMessage")]
    UnauthorizedMessage = 45349677,
    /// SQL R001B AuthError
    #[postgres(name = "AuthError")]
    AuthError = 45349679,
    /// SQL R001G InternalError
    #[postgres(name = "InternalError")]
    InternalError = 45349684,
}

impl From<EnumErrorCode> for ErrorCode {
    fn from(e: EnumErrorCode) -> Self {
        ErrorCode::new(e as _)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizeRequest {
    pub username: String,
    pub token: uuid::Uuid,
    pub service: EnumService,
    pub device_id: String,
    pub device_os: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizeResponse {
    pub success: bool,
    pub user_id: i64,
    pub role: EnumRole,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BestBidAskAcrossExchanges {
    pub datetime: i64,
    pub binance_ask_price: f64,
    pub binance_ask_volume: f64,
    pub binance_bid_price: f64,
    pub binance_bid_volume: f64,
    pub hyper_ask_price: f64,
    pub hyper_ask_volume: f64,
    pub hyper_bid_price: f64,
    pub hyper_bid_volume: f64,
    pub bb_ha: f64,
    pub ba_hb: f64,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BestBidAskAcrossExchangesWithPosition {
    pub id: i64,
    pub opening_id: i64,
    pub datetime: i64,
    pub expiry: i64,
    pub symbol: String,
    pub ba_bn: f64,
    pub bb_bn: f64,
    pub ba_amount_bn: f64,
    pub bb_amount_bn: f64,
    pub ba_hp: f64,
    pub bb_hp: f64,
    pub ba_amount_hp: f64,
    pub bb_amount_hp: f64,
    pub hl_balance_coin: f64,
    pub ba_balance_coin: f64,
    pub opportunity_size: f64,
    pub expired: bool,
    pub action: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DefaultS2Configuration {
    pub buy_exchange: String,
    pub sell_exchange: String,
    pub instrument: String,
    pub order_size: f64,
    pub max_unhedged: f64,
    pub target_spread: f64,
    pub target_position: f64,
    pub order_type: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Event1 {
    pub trend: String,
    pub binance_price: f64,
    pub hyper_price: f64,
    pub difference_in_basis_points: f64,
    pub status: String,
    pub id: i64,
    pub datetime: i64,
    pub symbol: String,
    pub level: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    pub service: EnumService,
    pub device_id: String,
    pub device_os: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub username: String,
    pub display_name: String,
    #[serde(default)]
    pub avatar: Option<String>,
    pub role: EnumRole,
    pub user_id: i64,
    pub user_token: uuid::Uuid,
    pub admin_token: uuid::Uuid,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LogoutRequest {}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LogoutResponse {}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Price {
    pub datetime: i64,
    pub symbol: String,
    pub price: f64,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Price0 {
    pub datetime: i64,
    pub binance_price: f64,
    pub hyper_bid_price: f64,
    pub hyper_oracle: f64,
    pub hyper_mark: f64,
    pub difference_in_usd: f64,
    pub difference_in_basis_points: f64,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PriceDifference {
    pub datetime: i64,
    pub binance_price: f64,
    pub hyper_ask_price: f64,
    pub hyper_bid_price: f64,
    pub difference_in_usd: f64,
    pub difference_in_basis_points: f64,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PriceSpread {
    pub datetime: i64,
    pub exchange_1: String,
    pub exchange_2: String,
    pub asset: String,
    pub spread_buy_1: f64,
    pub spread_sell_1: f64,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RequestSymbolList {
    pub symbol: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Signal0 {
    pub priority: i32,
    pub bp: f64,
    pub id: i64,
    pub datetime: i64,
    pub symbol: String,
    pub level: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Signal1 {
    #[serde(default)]
    pub difference: Option<SignalPriceDifference>,
    #[serde(default)]
    pub change: Option<SignalPriceChange>,
    pub id: i64,
    pub datetime: i64,
    pub symbol: String,
    pub level: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Signal2 {
    #[serde(default)]
    pub ba_change: Option<SignalPriceChangeImmediate>,
    #[serde(default)]
    pub bb_change: Option<SignalPriceChangeImmediate>,
    #[serde(default)]
    pub ba_hb_diff: Option<SignalBinAskHypBidDiff>,
    #[serde(default)]
    pub bb_ha_diff: Option<SignalBinBidHypAskDiff>,
    pub id: i64,
    pub datetime: i64,
    pub symbol: String,
    pub level: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SignalBinAskHypBidDiff {
    pub bin_ask: f64,
    pub hyp_bid: f64,
    pub ratio: f64,
    pub used: bool,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SignalBinBidHypAskDiff {
    pub bin_bid: f64,
    pub hyp_ask: f64,
    pub ratio: f64,
    pub used: bool,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SignalPriceChange {
    pub trend: String,
    pub time_high: i64,
    pub time_low: i64,
    pub price_high: f64,
    pub price_low: f64,
    pub bp: f64,
    pub used: bool,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SignalPriceChangeImmediate {
    pub trend: String,
    pub exchange: String,
    pub price_type: String,
    pub before: f64,
    pub after: f64,
    pub ratio: f64,
    pub used: bool,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SignalPriceDifference {
    pub price_binance: f64,
    pub price_hyper: f64,
    pub bp: f64,
    pub used: bool,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SignupRequest {
    pub username: String,
    pub password: String,
    pub email: String,
    pub phone: String,
    pub agreed_tos: bool,
    pub agreed_privacy: bool,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SignupResponse {
    pub username: String,
    pub user_id: i64,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SubS3TerminalBestAskBestBidRequest {
    #[serde(default)]
    pub unsubscribe_other_symbol: Option<bool>,
    pub symbol: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SubS3TerminalBestAskBestBidResponse {
    pub data: Vec<Price>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserAccuracyLog {
    pub datetime: i64,
    pub count_pass: i64,
    pub count_fail: i64,
    pub accuracy: f64,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserAddBlacklistRequest {
    pub strategy_id: i32,
    pub list: Vec<RequestSymbolList>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserAddBlacklistResponse {
    pub success: bool,
    #[serde(default)]
    pub reason: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserBenchmarkResult {
    pub id: i64,
    pub datetime: i64,
    pub exchange: String,
    pub latency_us: i64,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserCancelOrClosePositionRequest {
    pub id: i64,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserCancelOrClosePositionResponse {}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserCancelOrderRequest {
    pub exchange: String,
    pub symbol: String,
    pub local_id: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserCancelOrderResponse {
    pub success: bool,
    #[serde(default)]
    pub reason: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserCapturedEvent {
    pub id: i64,
    #[serde(default)]
    pub event_id: Option<i64>,
    #[serde(default)]
    pub cloid: Option<String>,
    pub exchange: String,
    pub symbol: String,
    pub status: String,
    #[serde(default)]
    pub price: Option<f64>,
    pub size: f64,
    pub filled_size: f64,
    pub cancel_or_close: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserControlStrategyRequest {
    pub strategy_id: i32,
    pub config: serde_json::Value,
    pub paused: bool,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserControlStrategyResponse {
    pub success: bool,
    #[serde(default)]
    pub reason: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserDebugLogRow {
    pub datetime: i64,
    pub level: String,
    pub thread: String,
    pub path: String,
    pub line_number: i32,
    pub message: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserDecryptEncryptedKeyRequest {
    pub encryption_key: String,
    pub exchange: String,
    pub account_id: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserDecryptEncryptedKeyResponse {
    pub success: bool,
    #[serde(default)]
    pub reason: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserDeleteEncryptedKeyRequest {
    pub exchange: String,
    pub account_id: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserDeleteEncryptedKeyResponse {
    pub success: bool,
    #[serde(default)]
    pub reason: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserEncryptedKey {
    pub id: i64,
    pub exchange: String,
    pub account_id: String,
    pub ciphertext: String,
    pub alias: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserEvent {
    pub topic: String,
    pub time: i64,
    pub content: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserFundingRates {
    pub exchange: String,
    pub symbol: String,
    pub rate: f64,
    pub datetime: i64,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGet5MinSpreadMeanRequest {}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGet5MinSpreadMeanResponse {
    pub data: Vec<PriceSpread>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetAccuracyRequest {
    #[serde(default)]
    pub symbol: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetAccuracyResponse {
    pub count_correct: i64,
    pub count_wrong: i64,
    pub accuracy: f64,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetBestBidAskAcrossExchangesRequest {
    #[serde(default)]
    pub latest: Option<bool>,
    #[serde(default)]
    pub time_start: Option<i64>,
    #[serde(default)]
    pub time_end: Option<i64>,
    #[serde(default)]
    pub symbol: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetBestBidAskAcrossExchangesResponse {
    pub data: Vec<BestBidAskAcrossExchanges>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetBestBidAskAcrossExchangesWithPositionEventRequest {
    #[serde(default)]
    pub id: Option<i64>,
    #[serde(default)]
    pub time_start: Option<i64>,
    #[serde(default)]
    pub time_end: Option<i64>,
    #[serde(default)]
    pub symbol: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetBestBidAskAcrossExchangesWithPositionEventResponse {
    pub data: Vec<BestBidAskAcrossExchangesWithPosition>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetBlacklistRequest {
    pub strategy_id: i32,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetBlacklistResponse {
    pub data: Vec<UserSymbolList>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetDebugLogRequest {
    #[serde(default)]
    pub limit: Option<i32>,
    #[serde(default)]
    pub page: Option<i32>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetDebugLogResponse {
    pub data: Vec<UserDebugLogRow>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetEncryptedKeyRequest {}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetEncryptedKeyResponse {
    pub data: Vec<UserEncryptedKey>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetEvent1Request {
    #[serde(default)]
    pub id: Option<i64>,
    #[serde(default)]
    pub time_start: Option<i64>,
    #[serde(default)]
    pub time_end: Option<i64>,
    #[serde(default)]
    pub symbol: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetEvent1Response {
    pub data: Vec<Event1>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetHedgedOrdersRequest {
    pub strategy_id: i32,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetHedgedOrdersResponse {
    pub data: Vec<UserHedgedOrders>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetLedgerRequest {
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub include_ack: Option<bool>,
    pub strategy_id: i32,
    #[serde(default)]
    pub time_start: Option<i64>,
    #[serde(default)]
    pub time_end: Option<i64>,
    #[serde(default)]
    pub symbol: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetLedgerResponse {
    pub data: Vec<UserLedger>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetLiveTestAccuracyLogRequest {
    #[serde(default)]
    pub tag: Option<String>,
    #[serde(default)]
    pub time_start: Option<i64>,
    #[serde(default)]
    pub time_end: Option<i64>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetLiveTestAccuracyLogResponse {
    pub data: Vec<UserAccuracyLog>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetLiveTestCloseOrder1Request {}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetLiveTestCloseOrder1Response {
    pub data: Vec<UserLiveTestPrice>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetOrdersPerStrategyRequest {
    #[serde(default)]
    pub event_id: Option<i64>,
    #[serde(default)]
    pub client_id: Option<String>,
    pub strategy_id: i32,
    #[serde(default)]
    pub time_start: Option<i64>,
    #[serde(default)]
    pub time_end: Option<i64>,
    #[serde(default)]
    pub symbol: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetOrdersPerStrategyResponse {
    pub data: Vec<UserOrder>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetPrice0Request {
    #[serde(default)]
    pub time_start: Option<i64>,
    #[serde(default)]
    pub time_end: Option<i64>,
    pub symbol: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetPrice0Response {
    pub data: Vec<Price0>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetPriceDifferenceRequest {
    #[serde(default)]
    pub time_start: Option<i64>,
    #[serde(default)]
    pub time_end: Option<i64>,
    pub symbol: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetPriceDifferenceResponse {
    pub data: Vec<PriceDifference>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetSignal0Request {
    #[serde(default)]
    pub min_level: Option<String>,
    #[serde(default)]
    pub time_start: Option<i64>,
    #[serde(default)]
    pub time_end: Option<i64>,
    #[serde(default)]
    pub symbol: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetSignal0Response {
    pub data: Vec<Signal0>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetSignal1Request {
    #[serde(default)]
    pub signal: Option<String>,
    #[serde(default)]
    pub min_level: Option<String>,
    #[serde(default)]
    pub symbol: Option<String>,
    #[serde(default)]
    pub time_start: Option<i64>,
    #[serde(default)]
    pub time_end: Option<i64>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetSignal1Response {
    pub data: Vec<Signal1>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetSignal2Request {
    #[serde(default)]
    pub signal: Option<String>,
    #[serde(default)]
    pub min_level: Option<String>,
    #[serde(default)]
    pub symbol: Option<String>,
    #[serde(default)]
    pub time_start: Option<i64>,
    #[serde(default)]
    pub time_end: Option<i64>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetSignal2Response {
    pub data: Vec<Signal2>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetStrategyOneAccuracyRequest {}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetStrategyOneAccuracyResponse {
    pub count_correct: i64,
    pub count_wrong: i64,
    pub accuracy: f64,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetStrategyOneSymbolRequest {
    #[serde(default)]
    pub symbol: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetStrategyOneSymbolResponse {
    pub data: Vec<UserSymbolList>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetStrategyZeroSymbolRequest {
    #[serde(default)]
    pub symbol: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetStrategyZeroSymbolResponse {
    pub data: Vec<UserSymbolList>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetSymbol2Request {
    #[serde(default)]
    pub symbol: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserGetSymbol2Response {
    pub data: Vec<UserSymbolList>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserHedgedOrders {
    pub id: i64,
    pub leg1_id: String,
    pub leg2_id: String,
    pub leg1_cloid: String,
    pub leg2_cloid: String,
    pub datetime: i64,
    pub leg1_ins: String,
    pub leg2_ins: String,
    pub leg1_side: String,
    pub leg2_side: String,
    pub leg1_price: f64,
    pub leg2_price: f64,
    pub leg1_status: String,
    pub leg2_status: String,
    pub size: f64,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserInitStrategyRequest {
    pub strategy_id: i32,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserInitStrategyResponse {
    pub success: bool,
    #[serde(default)]
    pub reason: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserKey {
    pub exchange: String,
    pub account_id: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserLedger {
    pub id: i64,
    pub open_order_id: String,
    pub close_order_id: String,
    pub open_order_cloid: String,
    pub close_order_cloid: String,
    pub datetime: i64,
    pub exchange: String,
    pub symbol: String,
    pub open_order_position_type: String,
    pub open_order_side: String,
    pub open_price_usd: f64,
    pub close_price_usd: f64,
    pub volume: f64,
    pub closed_profit: f64,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserListStrategyRequest {
    #[serde(default)]
    pub name: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserListStrategyResponse {
    pub strategies: Vec<UserStrategyRow>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserListTradingSymbolsRequest {}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserListTradingSymbolsResponse {
    pub data: Vec<UserTradingSymbol>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserLiveTestPrice {
    pub symbol: String,
    pub datetime: i64,
    pub target_price: f64,
    pub last_price: f64,
    pub trend_prediction: String,
    pub price_event: f64,
    pub price_actual_filled: f64,
    pub price_market_when_filled: f64,
    pub pass_actual_filled: bool,
    pub pass_market_when_filled: bool,
    pub last_open_price: f64,
    pub last_close_price: f64,
    pub last_high_price: f64,
    pub last_low_price: f64,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserLogEvent {
    pub level: String,
    pub time: i64,
    pub content: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserOrder {
    pub id: i64,
    pub event_id: i64,
    pub client_id: String,
    pub exchange: String,
    pub symbol: String,
    pub order_type: String,
    pub side: String,
    pub price: f64,
    pub volume: f64,
    pub strategy_id: i32,
    pub datetime: i64,
    pub effect: String,
    pub status: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserPlaceOrderLimitRequest {
    pub exchange: String,
    pub symbol: String,
    pub side: String,
    pub price: f64,
    pub size: f64,
    pub local_id: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserPlaceOrderLimitResponse {
    pub success: bool,
    pub reason: String,
    pub local_id: String,
    pub client_id: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserPlaceOrderMarketRequest {
    pub exchange: String,
    pub symbol: String,
    pub side: String,
    pub price: f64,
    pub size: f64,
    pub local_id: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserPlaceOrderMarketResponse {
    pub success: bool,
    pub reason: String,
    pub local_id: String,
    pub client_id: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserPosition {
    pub id: i64,
    #[serde(default)]
    pub cloid: Option<String>,
    pub exchange: String,
    pub symbol: String,
    pub size: f64,
    pub filled_size: f64,
    pub cancel_or_close: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserRemoveBlacklistRequest {
    pub strategy_id: i32,
    pub list: Vec<RequestSymbolList>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserRemoveBlacklistResponse {
    pub success: bool,
    #[serde(default)]
    pub reason: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserS3CaptureEventRequest {
    pub event_id: i64,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserS3CaptureEventResponse {
    pub success: bool,
    pub reason: String,
    pub local_id: String,
    pub client_id: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserS3ReleasePositionRequest {
    pub event_id: i64,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserS3ReleasePositionResponse {
    pub success: bool,
    pub reason: String,
    pub local_id: String,
    pub client_id: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSetEncryptedKey {
    pub exchange: String,
    pub account_id: String,
    pub ciphertext: String,
    pub alias: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSetEncryptedKeyRequest {
    pub key: Vec<UserSetEncryptedKey>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSetEncryptedKeyResponse {
    pub success: bool,
    #[serde(default)]
    pub reason: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSetS2ConfigureRequest {
    pub configuration: Vec<DefaultS2Configuration>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSetS2ConfigureResponse {
    pub success: bool,
    #[serde(default)]
    pub reason: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSetStrategyStatusRequest {
    #[serde(default)]
    pub set_status: Option<Vec<UserStrategyStatus>>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSetStrategyStatusResponse {
    pub data: Vec<UserStrategyStatus>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSetSymbolFlag1Request {
    pub flag: bool,
    pub symbol: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSetSymbolFlag1Response {
    pub success: bool,
    #[serde(default)]
    pub reason: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserStartServiceRequest {
    pub keys: Vec<UserKey>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserStartServiceResponse {
    pub success: bool,
    #[serde(default)]
    pub reason: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserStatusRequest {}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserStatusResponse {
    pub status: String,
    pub time: i64,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserStrategyRow {
    pub name: String,
    pub strategy_id: i32,
    pub status: String,
    pub config: serde_json::Value,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserStrategyStatus {
    pub id: i32,
    pub status: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSubBestBidAskAcrossExchangesRequest {
    #[serde(default)]
    pub unsubscribe_other_symbol: Option<bool>,
    pub symbol: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSubBestBidAskAcrossExchangesResponse {
    pub data: Vec<BestBidAskAcrossExchanges>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSubBestBidAskAcrossExchangesWithPositionEventRequest {
    #[serde(default)]
    pub symbol: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSubBestBidAskAcrossExchangesWithPositionEventResponse {
    pub data: Vec<BestBidAskAcrossExchangesWithPosition>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSubEvent1Request {
    #[serde(default)]
    pub symbol: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSubEvent1Response {
    pub data: Vec<Event1>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSubEventsRequest {
    pub topic: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSubEventsResponse {}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSubExchangeLatencyRequest {
    #[serde(default)]
    pub unsub: Option<bool>,
    #[serde(default)]
    pub time_start: Option<i64>,
    #[serde(default)]
    pub time_end: Option<i64>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSubExchangeLatencyResponse {
    pub data: Vec<UserBenchmarkResult>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSubFundingRatesRequest {
    #[serde(default)]
    pub exchange: Option<String>,
    #[serde(default)]
    pub symbol: Option<String>,
    #[serde(default)]
    pub unsub: Option<bool>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSubFundingRatesResponse {
    pub data: Vec<UserFundingRates>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSubLedgerRequest {
    pub strategy_id: i32,
    #[serde(default)]
    pub symbol: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSubLedgerResponse {
    pub data: Vec<UserLedger>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSubLedgerStrategyOneRequest {
    #[serde(default)]
    pub symbol: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSubLedgerStrategyOneResponse {
    pub data: Vec<UserLedger>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSubLogsRequest {}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSubLogsResponse {}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSubOrdersRequest {
    #[serde(default)]
    pub strategy_id: Option<i32>,
    #[serde(default)]
    pub unsubscribe: Option<bool>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSubOrdersResponse {
    pub data: Vec<UserOrder>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSubPositionRequest {
    #[serde(default)]
    pub unsubscribe: Option<bool>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSubPositionResponse {
    pub data: Vec<UserPosition>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSubPrice0Request {
    #[serde(default)]
    pub unsubscribe_other_symbol: Option<bool>,
    pub symbol: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSubPrice0Response {
    pub data: Vec<Price0>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSubPriceDifferenceRequest {
    #[serde(default)]
    pub unsubscribe_other_symbol: Option<bool>,
    pub symbol: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSubPriceDifferenceResponse {
    pub data: Vec<PriceDifference>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSubSignal0Request {
    #[serde(default)]
    pub unsubscribe_other_symbol: Option<bool>,
    #[serde(default)]
    pub symbol: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSubSignal0Response {
    pub data: Vec<Signal0>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSubSignal1Request {
    #[serde(default)]
    pub symbol: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSubSignal1Response {
    pub data: Vec<Signal1>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSubSignal2Request {
    #[serde(default)]
    pub symbol: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSubSignal2Response {
    pub data: Vec<Signal2>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSubStrategy3PositionsClosingRequest {
    #[serde(default)]
    pub unsubscribe: Option<bool>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSubStrategy3PositionsClosingResponse {
    pub data: Vec<UserCapturedEvent>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSubStrategy3PositionsOpeningRequest {
    #[serde(default)]
    pub unsubscribe: Option<bool>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSubStrategy3PositionsOpeningResponse {
    pub data: Vec<UserCapturedEvent>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSubStrategyOneOrderRequest {
    #[serde(default)]
    pub symbol: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSubStrategyOneOrderResponse {
    pub data: Vec<UserOrder>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserSymbolList {
    pub symbol: String,
    pub status: String,
    pub flag: bool,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserTradingSymbol {
    pub exchange: String,
    pub symbol: String,
    pub base: String,
    pub lot_size: f64,
    pub base_decimals: i32,
    pub quote: String,
    pub tick_size: f64,
    pub quote_decimals: i32,
}
impl WsRequest for LoginRequest {
    type Response = LoginResponse;
    const METHOD_ID: u32 = 10020;
    const SCHEMA: &'static str = r#"{
  "name": "Login",
  "code": 10020,
  "parameters": [
    {
      "name": "username",
      "ty": "String"
    },
    {
      "name": "password",
      "ty": "String"
    },
    {
      "name": "service",
      "ty": {
        "EnumRef": "service"
      }
    },
    {
      "name": "device_id",
      "ty": "String"
    },
    {
      "name": "device_os",
      "ty": "String"
    }
  ],
  "returns": [
    {
      "name": "username",
      "ty": "String"
    },
    {
      "name": "display_name",
      "ty": "String"
    },
    {
      "name": "avatar",
      "ty": {
        "Optional": "String"
      }
    },
    {
      "name": "role",
      "ty": {
        "EnumRef": "role"
      }
    },
    {
      "name": "user_id",
      "ty": "BigInt"
    },
    {
      "name": "user_token",
      "ty": "UUID"
    },
    {
      "name": "admin_token",
      "ty": "UUID"
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for LoginResponse {
    type Request = LoginRequest;
}

impl WsRequest for SignupRequest {
    type Response = SignupResponse;
    const METHOD_ID: u32 = 10010;
    const SCHEMA: &'static str = r#"{
  "name": "Signup",
  "code": 10010,
  "parameters": [
    {
      "name": "username",
      "ty": "String"
    },
    {
      "name": "password",
      "ty": "String"
    },
    {
      "name": "email",
      "ty": "String"
    },
    {
      "name": "phone",
      "ty": "String"
    },
    {
      "name": "agreed_tos",
      "ty": "Boolean"
    },
    {
      "name": "agreed_privacy",
      "ty": "Boolean"
    }
  ],
  "returns": [
    {
      "name": "username",
      "ty": "String"
    },
    {
      "name": "user_id",
      "ty": "BigInt"
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for SignupResponse {
    type Request = SignupRequest;
}

impl WsRequest for AuthorizeRequest {
    type Response = AuthorizeResponse;
    const METHOD_ID: u32 = 10030;
    const SCHEMA: &'static str = r#"{
  "name": "Authorize",
  "code": 10030,
  "parameters": [
    {
      "name": "username",
      "ty": "String"
    },
    {
      "name": "token",
      "ty": "UUID"
    },
    {
      "name": "service",
      "ty": {
        "EnumRef": "service"
      }
    },
    {
      "name": "device_id",
      "ty": "String"
    },
    {
      "name": "device_os",
      "ty": "String"
    }
  ],
  "returns": [
    {
      "name": "success",
      "ty": "Boolean"
    },
    {
      "name": "user_id",
      "ty": "BigInt"
    },
    {
      "name": "role",
      "ty": {
        "EnumRef": "role"
      }
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for AuthorizeResponse {
    type Request = AuthorizeRequest;
}

impl WsRequest for LogoutRequest {
    type Response = LogoutResponse;
    const METHOD_ID: u32 = 10040;
    const SCHEMA: &'static str = r#"{
  "name": "Logout",
  "code": 10040,
  "parameters": [],
  "returns": [],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for LogoutResponse {
    type Request = LogoutRequest;
}

impl WsRequest for UserStatusRequest {
    type Response = UserStatusResponse;
    const METHOD_ID: u32 = 20000;
    const SCHEMA: &'static str = r#"{
  "name": "UserStatus",
  "code": 20000,
  "parameters": [],
  "returns": [
    {
      "name": "status",
      "ty": "String"
    },
    {
      "name": "time",
      "ty": "TimeStampMs"
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserStatusResponse {
    type Request = UserStatusRequest;
}

impl WsRequest for UserSubLogsRequest {
    type Response = UserSubLogsResponse;
    const METHOD_ID: u32 = 20010;
    const SCHEMA: &'static str = r#"{
  "name": "UserSubLogs",
  "code": 20010,
  "parameters": [],
  "returns": [],
  "stream_response": {
    "Struct": {
      "name": "UserLogEvent",
      "fields": [
        {
          "name": "level",
          "ty": "String"
        },
        {
          "name": "time",
          "ty": "TimeStampMs"
        },
        {
          "name": "content",
          "ty": "String"
        }
      ]
    }
  },
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserSubLogsResponse {
    type Request = UserSubLogsRequest;
}

impl WsRequest for UserSubEventsRequest {
    type Response = UserSubEventsResponse;
    const METHOD_ID: u32 = 20020;
    const SCHEMA: &'static str = r#"{
  "name": "UserSubEvents",
  "code": 20020,
  "parameters": [
    {
      "name": "topic",
      "ty": "String"
    }
  ],
  "returns": [],
  "stream_response": {
    "Struct": {
      "name": "UserEvent",
      "fields": [
        {
          "name": "topic",
          "ty": "String"
        },
        {
          "name": "time",
          "ty": "TimeStampMs"
        },
        {
          "name": "content",
          "ty": "String"
        }
      ]
    }
  },
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserSubEventsResponse {
    type Request = UserSubEventsRequest;
}

impl WsRequest for UserSubPositionRequest {
    type Response = UserSubPositionResponse;
    const METHOD_ID: u32 = 20030;
    const SCHEMA: &'static str = r#"{
  "name": "UserSubPosition",
  "code": 20030,
  "parameters": [
    {
      "name": "unsubscribe",
      "ty": {
        "Optional": "Boolean"
      }
    }
  ],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "UserPosition",
          "fields": [
            {
              "name": "id",
              "ty": "BigInt"
            },
            {
              "name": "cloid",
              "ty": {
                "Optional": "String"
              }
            },
            {
              "name": "exchange",
              "ty": "String"
            },
            {
              "name": "symbol",
              "ty": "String"
            },
            {
              "name": "size",
              "ty": "Numeric"
            },
            {
              "name": "filled_size",
              "ty": "Numeric"
            },
            {
              "name": "cancel_or_close",
              "ty": "String"
            }
          ]
        }
      }
    }
  ],
  "stream_response": {
    "DataTable": {
      "name": "UserPosition",
      "fields": [
        {
          "name": "id",
          "ty": "BigInt"
        },
        {
          "name": "cloid",
          "ty": {
            "Optional": "String"
          }
        },
        {
          "name": "exchange",
          "ty": "String"
        },
        {
          "name": "symbol",
          "ty": "String"
        },
        {
          "name": "size",
          "ty": "Numeric"
        },
        {
          "name": "filled_size",
          "ty": "Numeric"
        },
        {
          "name": "cancel_or_close",
          "ty": "String"
        }
      ]
    }
  },
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserSubPositionResponse {
    type Request = UserSubPositionRequest;
}

impl WsRequest for UserCancelOrClosePositionRequest {
    type Response = UserCancelOrClosePositionResponse;
    const METHOD_ID: u32 = 20031;
    const SCHEMA: &'static str = r#"{
  "name": "UserCancelOrClosePosition",
  "code": 20031,
  "parameters": [
    {
      "name": "id",
      "ty": "BigInt"
    }
  ],
  "returns": [],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserCancelOrClosePositionResponse {
    type Request = UserCancelOrClosePositionRequest;
}

impl WsRequest for UserSubOrdersRequest {
    type Response = UserSubOrdersResponse;
    const METHOD_ID: u32 = 20040;
    const SCHEMA: &'static str = r#"{
  "name": "UserSubOrders",
  "code": 20040,
  "parameters": [
    {
      "name": "strategy_id",
      "ty": {
        "Optional": "Int"
      }
    },
    {
      "name": "unsubscribe",
      "ty": {
        "Optional": "Boolean"
      }
    }
  ],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "UserOrder",
          "fields": [
            {
              "name": "id",
              "ty": "BigInt"
            },
            {
              "name": "event_id",
              "ty": "BigInt"
            },
            {
              "name": "client_id",
              "ty": "String"
            },
            {
              "name": "exchange",
              "ty": "String"
            },
            {
              "name": "symbol",
              "ty": "String"
            },
            {
              "name": "order_type",
              "ty": "String"
            },
            {
              "name": "side",
              "ty": "String"
            },
            {
              "name": "price",
              "ty": "Numeric"
            },
            {
              "name": "volume",
              "ty": "Numeric"
            },
            {
              "name": "strategy_id",
              "ty": "Int"
            },
            {
              "name": "datetime",
              "ty": "TimeStampMs"
            },
            {
              "name": "effect",
              "ty": "String"
            },
            {
              "name": "status",
              "ty": "String"
            }
          ]
        }
      }
    }
  ],
  "stream_response": {
    "DataTable": {
      "name": "UserOrder",
      "fields": [
        {
          "name": "id",
          "ty": "BigInt"
        },
        {
          "name": "event_id",
          "ty": "BigInt"
        },
        {
          "name": "client_id",
          "ty": "String"
        },
        {
          "name": "exchange",
          "ty": "String"
        },
        {
          "name": "symbol",
          "ty": "String"
        },
        {
          "name": "order_type",
          "ty": "String"
        },
        {
          "name": "side",
          "ty": "String"
        },
        {
          "name": "price",
          "ty": "Numeric"
        },
        {
          "name": "volume",
          "ty": "Numeric"
        },
        {
          "name": "strategy_id",
          "ty": "Int"
        },
        {
          "name": "datetime",
          "ty": "TimeStampMs"
        },
        {
          "name": "effect",
          "ty": "String"
        },
        {
          "name": "status",
          "ty": "String"
        }
      ]
    }
  },
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserSubOrdersResponse {
    type Request = UserSubOrdersRequest;
}

impl WsRequest for UserListStrategyRequest {
    type Response = UserListStrategyResponse;
    const METHOD_ID: u32 = 20100;
    const SCHEMA: &'static str = r#"{
  "name": "UserListStrategy",
  "code": 20100,
  "parameters": [
    {
      "name": "name",
      "ty": {
        "Optional": "String"
      }
    }
  ],
  "returns": [
    {
      "name": "strategies",
      "ty": {
        "Vec": {
          "Struct": {
            "name": "UserStrategyRow",
            "fields": [
              {
                "name": "name",
                "ty": "String"
              },
              {
                "name": "strategy_id",
                "ty": "Int"
              },
              {
                "name": "status",
                "ty": "String"
              },
              {
                "name": "config",
                "ty": "Object"
              }
            ]
          }
        }
      }
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserListStrategyResponse {
    type Request = UserListStrategyRequest;
}

impl WsRequest for UserInitStrategyRequest {
    type Response = UserInitStrategyResponse;
    const METHOD_ID: u32 = 20110;
    const SCHEMA: &'static str = r#"{
  "name": "UserInitStrategy",
  "code": 20110,
  "parameters": [
    {
      "name": "strategy_id",
      "ty": "Int"
    }
  ],
  "returns": [
    {
      "name": "success",
      "ty": "Boolean"
    },
    {
      "name": "reason",
      "ty": {
        "Optional": "String"
      }
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserInitStrategyResponse {
    type Request = UserInitStrategyRequest;
}

impl WsRequest for UserSubPrice0Request {
    type Response = UserSubPrice0Response;
    const METHOD_ID: u32 = 20120;
    const SCHEMA: &'static str = r#"{
  "name": "UserSubPrice0",
  "code": 20120,
  "parameters": [
    {
      "name": "unsubscribe_other_symbol",
      "ty": {
        "Optional": "Boolean"
      }
    },
    {
      "name": "symbol",
      "ty": "String"
    }
  ],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "Price0",
          "fields": [
            {
              "name": "datetime",
              "ty": "BigInt"
            },
            {
              "name": "binance_price",
              "ty": "Numeric"
            },
            {
              "name": "hyper_bid_price",
              "ty": "Numeric"
            },
            {
              "name": "hyper_oracle",
              "ty": "Numeric"
            },
            {
              "name": "hyper_mark",
              "ty": "Numeric"
            },
            {
              "name": "difference_in_usd",
              "ty": "Numeric"
            },
            {
              "name": "difference_in_basis_points",
              "ty": "Numeric"
            }
          ]
        }
      }
    }
  ],
  "stream_response": {
    "DataTable": {
      "name": "Price0",
      "fields": [
        {
          "name": "datetime",
          "ty": "BigInt"
        },
        {
          "name": "binance_price",
          "ty": "Numeric"
        },
        {
          "name": "hyper_bid_price",
          "ty": "Numeric"
        },
        {
          "name": "hyper_oracle",
          "ty": "Numeric"
        },
        {
          "name": "hyper_mark",
          "ty": "Numeric"
        },
        {
          "name": "difference_in_usd",
          "ty": "Numeric"
        },
        {
          "name": "difference_in_basis_points",
          "ty": "Numeric"
        }
      ]
    }
  },
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserSubPrice0Response {
    type Request = UserSubPrice0Request;
}

impl WsRequest for UserGetPrice0Request {
    type Response = UserGetPrice0Response;
    const METHOD_ID: u32 = 20130;
    const SCHEMA: &'static str = r#"{
  "name": "UserGetPrice0",
  "code": 20130,
  "parameters": [
    {
      "name": "time_start",
      "ty": {
        "Optional": "TimeStampMs"
      }
    },
    {
      "name": "time_end",
      "ty": {
        "Optional": "TimeStampMs"
      }
    },
    {
      "name": "symbol",
      "ty": "String"
    }
  ],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "Price0",
          "fields": [
            {
              "name": "datetime",
              "ty": "BigInt"
            },
            {
              "name": "binance_price",
              "ty": "Numeric"
            },
            {
              "name": "hyper_bid_price",
              "ty": "Numeric"
            },
            {
              "name": "hyper_oracle",
              "ty": "Numeric"
            },
            {
              "name": "hyper_mark",
              "ty": "Numeric"
            },
            {
              "name": "difference_in_usd",
              "ty": "Numeric"
            },
            {
              "name": "difference_in_basis_points",
              "ty": "Numeric"
            }
          ]
        }
      }
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserGetPrice0Response {
    type Request = UserGetPrice0Request;
}

impl WsRequest for UserControlStrategyRequest {
    type Response = UserControlStrategyResponse;
    const METHOD_ID: u32 = 20140;
    const SCHEMA: &'static str = r#"{
  "name": "UserControlStrategy",
  "code": 20140,
  "parameters": [
    {
      "name": "strategy_id",
      "ty": "Int"
    },
    {
      "name": "config",
      "ty": "Object"
    },
    {
      "name": "paused",
      "ty": "Boolean"
    }
  ],
  "returns": [
    {
      "name": "success",
      "ty": "Boolean"
    },
    {
      "name": "reason",
      "ty": {
        "Optional": "String"
      }
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserControlStrategyResponse {
    type Request = UserControlStrategyRequest;
}

impl WsRequest for UserGetStrategyZeroSymbolRequest {
    type Response = UserGetStrategyZeroSymbolResponse;
    const METHOD_ID: u32 = 20150;
    const SCHEMA: &'static str = r#"{
  "name": "UserGetStrategyZeroSymbol",
  "code": 20150,
  "parameters": [
    {
      "name": "symbol",
      "ty": {
        "Optional": "String"
      }
    }
  ],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "UserSymbolList",
          "fields": [
            {
              "name": "symbol",
              "ty": "String"
            },
            {
              "name": "status",
              "ty": "String"
            },
            {
              "name": "flag",
              "ty": "Boolean"
            }
          ]
        }
      }
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserGetStrategyZeroSymbolResponse {
    type Request = UserGetStrategyZeroSymbolRequest;
}

impl WsRequest for UserSubSignal0Request {
    type Response = UserSubSignal0Response;
    const METHOD_ID: u32 = 20160;
    const SCHEMA: &'static str = r#"{
  "name": "UserSubSignal0",
  "code": 20160,
  "parameters": [
    {
      "name": "unsubscribe_other_symbol",
      "ty": {
        "Optional": "Boolean"
      }
    },
    {
      "name": "symbol",
      "ty": {
        "Optional": "String"
      }
    }
  ],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "Signal0",
          "fields": [
            {
              "name": "priority",
              "ty": "Int"
            },
            {
              "name": "bp",
              "ty": "Numeric"
            },
            {
              "name": "id",
              "ty": "BigInt"
            },
            {
              "name": "datetime",
              "ty": "BigInt"
            },
            {
              "name": "symbol",
              "ty": "String"
            },
            {
              "name": "level",
              "ty": "String"
            }
          ]
        }
      }
    }
  ],
  "stream_response": {
    "DataTable": {
      "name": "Signal0",
      "fields": [
        {
          "name": "priority",
          "ty": "Int"
        },
        {
          "name": "bp",
          "ty": "Numeric"
        },
        {
          "name": "id",
          "ty": "BigInt"
        },
        {
          "name": "datetime",
          "ty": "BigInt"
        },
        {
          "name": "symbol",
          "ty": "String"
        },
        {
          "name": "level",
          "ty": "String"
        }
      ]
    }
  },
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserSubSignal0Response {
    type Request = UserSubSignal0Request;
}

impl WsRequest for UserGetSignal0Request {
    type Response = UserGetSignal0Response;
    const METHOD_ID: u32 = 20170;
    const SCHEMA: &'static str = r#"{
  "name": "UserGetSignal0",
  "code": 20170,
  "parameters": [
    {
      "name": "min_level",
      "ty": {
        "Optional": "String"
      }
    },
    {
      "name": "time_start",
      "ty": {
        "Optional": "TimeStampMs"
      }
    },
    {
      "name": "time_end",
      "ty": {
        "Optional": "TimeStampMs"
      }
    },
    {
      "name": "symbol",
      "ty": {
        "Optional": "String"
      }
    }
  ],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "Signal0",
          "fields": [
            {
              "name": "priority",
              "ty": "Int"
            },
            {
              "name": "bp",
              "ty": "Numeric"
            },
            {
              "name": "id",
              "ty": "BigInt"
            },
            {
              "name": "datetime",
              "ty": "BigInt"
            },
            {
              "name": "symbol",
              "ty": "String"
            },
            {
              "name": "level",
              "ty": "String"
            }
          ]
        }
      }
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserGetSignal0Response {
    type Request = UserGetSignal0Request;
}

impl WsRequest for UserGetDebugLogRequest {
    type Response = UserGetDebugLogResponse;
    const METHOD_ID: u32 = 20180;
    const SCHEMA: &'static str = r#"{
  "name": "UserGetDebugLog",
  "code": 20180,
  "parameters": [
    {
      "name": "limit",
      "ty": {
        "Optional": "Int"
      }
    },
    {
      "name": "page",
      "ty": {
        "Optional": "Int"
      }
    }
  ],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "UserDebugLogRow",
          "fields": [
            {
              "name": "datetime",
              "ty": "BigInt"
            },
            {
              "name": "level",
              "ty": "String"
            },
            {
              "name": "thread",
              "ty": "String"
            },
            {
              "name": "path",
              "ty": "String"
            },
            {
              "name": "line_number",
              "ty": "Int"
            },
            {
              "name": "message",
              "ty": "String"
            }
          ]
        }
      }
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserGetDebugLogResponse {
    type Request = UserGetDebugLogRequest;
}

impl WsRequest for UserSetEncryptedKeyRequest {
    type Response = UserSetEncryptedKeyResponse;
    const METHOD_ID: u32 = 21000;
    const SCHEMA: &'static str = r#"{
  "name": "UserSetEncryptedKey",
  "code": 21000,
  "parameters": [
    {
      "name": "key",
      "ty": {
        "DataTable": {
          "name": "UserSetEncryptedKey",
          "fields": [
            {
              "name": "exchange",
              "ty": "String"
            },
            {
              "name": "account_id",
              "ty": "String"
            },
            {
              "name": "ciphertext",
              "ty": "String"
            },
            {
              "name": "alias",
              "ty": "String"
            }
          ]
        }
      }
    }
  ],
  "returns": [
    {
      "name": "success",
      "ty": "Boolean"
    },
    {
      "name": "reason",
      "ty": {
        "Optional": "String"
      }
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserSetEncryptedKeyResponse {
    type Request = UserSetEncryptedKeyRequest;
}

impl WsRequest for UserStartServiceRequest {
    type Response = UserStartServiceResponse;
    const METHOD_ID: u32 = 21010;
    const SCHEMA: &'static str = r#"{
  "name": "UserStartService",
  "code": 21010,
  "parameters": [
    {
      "name": "keys",
      "ty": {
        "Vec": {
          "Struct": {
            "name": "UserKey",
            "fields": [
              {
                "name": "exchange",
                "ty": "String"
              },
              {
                "name": "account_id",
                "ty": "String"
              }
            ]
          }
        }
      }
    }
  ],
  "returns": [
    {
      "name": "success",
      "ty": "Boolean"
    },
    {
      "name": "reason",
      "ty": {
        "Optional": "String"
      }
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserStartServiceResponse {
    type Request = UserStartServiceRequest;
}

impl WsRequest for UserSetStrategyStatusRequest {
    type Response = UserSetStrategyStatusResponse;
    const METHOD_ID: u32 = 21020;
    const SCHEMA: &'static str = r#"{
  "name": "UserSetStrategyStatus",
  "code": 21020,
  "parameters": [
    {
      "name": "set_status",
      "ty": {
        "Optional": {
          "DataTable": {
            "name": "UserStrategyStatus",
            "fields": [
              {
                "name": "id",
                "ty": "Int"
              },
              {
                "name": "status",
                "ty": "String"
              }
            ]
          }
        }
      }
    }
  ],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "UserStrategyStatus",
          "fields": [
            {
              "name": "id",
              "ty": "Int"
            },
            {
              "name": "status",
              "ty": "String"
            }
          ]
        }
      }
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserSetStrategyStatusResponse {
    type Request = UserSetStrategyStatusRequest;
}

impl WsRequest for UserGetStrategyOneSymbolRequest {
    type Response = UserGetStrategyOneSymbolResponse;
    const METHOD_ID: u32 = 20200;
    const SCHEMA: &'static str = r#"{
  "name": "UserGetStrategyOneSymbol",
  "code": 20200,
  "parameters": [
    {
      "name": "symbol",
      "ty": {
        "Optional": "String"
      }
    }
  ],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "UserSymbolList",
          "fields": [
            {
              "name": "symbol",
              "ty": "String"
            },
            {
              "name": "status",
              "ty": "String"
            },
            {
              "name": "flag",
              "ty": "Boolean"
            }
          ]
        }
      }
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserGetStrategyOneSymbolResponse {
    type Request = UserGetStrategyOneSymbolRequest;
}

impl WsRequest for UserSetSymbolFlag1Request {
    type Response = UserSetSymbolFlag1Response;
    const METHOD_ID: u32 = 20210;
    const SCHEMA: &'static str = r#"{
  "name": "UserSetSymbolFlag1",
  "code": 20210,
  "parameters": [
    {
      "name": "flag",
      "ty": "Boolean"
    },
    {
      "name": "symbol",
      "ty": "String"
    }
  ],
  "returns": [
    {
      "name": "success",
      "ty": "Boolean"
    },
    {
      "name": "reason",
      "ty": {
        "Optional": "String"
      }
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserSetSymbolFlag1Response {
    type Request = UserSetSymbolFlag1Request;
}

impl WsRequest for UserGetEvent1Request {
    type Response = UserGetEvent1Response;
    const METHOD_ID: u32 = 20240;
    const SCHEMA: &'static str = r#"{
  "name": "UserGetEvent1",
  "code": 20240,
  "parameters": [
    {
      "name": "id",
      "ty": {
        "Optional": "BigInt"
      }
    },
    {
      "name": "time_start",
      "ty": {
        "Optional": "TimeStampMs"
      }
    },
    {
      "name": "time_end",
      "ty": {
        "Optional": "TimeStampMs"
      }
    },
    {
      "name": "symbol",
      "ty": {
        "Optional": "String"
      }
    }
  ],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "Event1",
          "fields": [
            {
              "name": "trend",
              "ty": "String"
            },
            {
              "name": "binance_price",
              "ty": "Numeric"
            },
            {
              "name": "hyper_price",
              "ty": "Numeric"
            },
            {
              "name": "difference_in_basis_points",
              "ty": "Numeric"
            },
            {
              "name": "status",
              "ty": "String"
            },
            {
              "name": "id",
              "ty": "BigInt"
            },
            {
              "name": "datetime",
              "ty": "BigInt"
            },
            {
              "name": "symbol",
              "ty": "String"
            },
            {
              "name": "level",
              "ty": "String"
            }
          ]
        }
      }
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserGetEvent1Response {
    type Request = UserGetEvent1Request;
}

impl WsRequest for UserSubEvent1Request {
    type Response = UserSubEvent1Response;
    const METHOD_ID: u32 = 20250;
    const SCHEMA: &'static str = r#"{
  "name": "UserSubEvent1",
  "code": 20250,
  "parameters": [
    {
      "name": "symbol",
      "ty": {
        "Optional": "String"
      }
    }
  ],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "Event1",
          "fields": [
            {
              "name": "trend",
              "ty": "String"
            },
            {
              "name": "binance_price",
              "ty": "Numeric"
            },
            {
              "name": "hyper_price",
              "ty": "Numeric"
            },
            {
              "name": "difference_in_basis_points",
              "ty": "Numeric"
            },
            {
              "name": "status",
              "ty": "String"
            },
            {
              "name": "id",
              "ty": "BigInt"
            },
            {
              "name": "datetime",
              "ty": "BigInt"
            },
            {
              "name": "symbol",
              "ty": "String"
            },
            {
              "name": "level",
              "ty": "String"
            }
          ]
        }
      }
    }
  ],
  "stream_response": {
    "DataTable": {
      "name": "Event1",
      "fields": [
        {
          "name": "trend",
          "ty": "String"
        },
        {
          "name": "binance_price",
          "ty": "Numeric"
        },
        {
          "name": "hyper_price",
          "ty": "Numeric"
        },
        {
          "name": "difference_in_basis_points",
          "ty": "Numeric"
        },
        {
          "name": "status",
          "ty": "String"
        },
        {
          "name": "id",
          "ty": "BigInt"
        },
        {
          "name": "datetime",
          "ty": "BigInt"
        },
        {
          "name": "symbol",
          "ty": "String"
        },
        {
          "name": "level",
          "ty": "String"
        }
      ]
    }
  },
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserSubEvent1Response {
    type Request = UserSubEvent1Request;
}

impl WsRequest for UserGetStrategyOneAccuracyRequest {
    type Response = UserGetStrategyOneAccuracyResponse;
    const METHOD_ID: u32 = 20260;
    const SCHEMA: &'static str = r#"{
  "name": "UserGetStrategyOneAccuracy",
  "code": 20260,
  "parameters": [],
  "returns": [
    {
      "name": "count_correct",
      "ty": "BigInt"
    },
    {
      "name": "count_wrong",
      "ty": "BigInt"
    },
    {
      "name": "accuracy",
      "ty": "Numeric"
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserGetStrategyOneAccuracyResponse {
    type Request = UserGetStrategyOneAccuracyRequest;
}

impl WsRequest for UserGetAccuracyRequest {
    type Response = UserGetAccuracyResponse;
    const METHOD_ID: u32 = 20261;
    const SCHEMA: &'static str = r#"{
  "name": "UserGetAccuracy",
  "code": 20261,
  "parameters": [
    {
      "name": "symbol",
      "ty": {
        "Optional": "String"
      }
    }
  ],
  "returns": [
    {
      "name": "count_correct",
      "ty": "BigInt"
    },
    {
      "name": "count_wrong",
      "ty": "BigInt"
    },
    {
      "name": "accuracy",
      "ty": "Numeric"
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserGetAccuracyResponse {
    type Request = UserGetAccuracyRequest;
}

impl WsRequest for UserGetOrdersPerStrategyRequest {
    type Response = UserGetOrdersPerStrategyResponse;
    const METHOD_ID: u32 = 20271;
    const SCHEMA: &'static str = r#"{
  "name": "UserGetOrdersPerStrategy",
  "code": 20271,
  "parameters": [
    {
      "name": "event_id",
      "ty": {
        "Optional": "BigInt"
      }
    },
    {
      "name": "client_id",
      "ty": {
        "Optional": "String"
      }
    },
    {
      "name": "strategy_id",
      "ty": "Int"
    },
    {
      "name": "time_start",
      "ty": {
        "Optional": "TimeStampMs"
      }
    },
    {
      "name": "time_end",
      "ty": {
        "Optional": "TimeStampMs"
      }
    },
    {
      "name": "symbol",
      "ty": {
        "Optional": "String"
      }
    }
  ],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "UserOrder",
          "fields": [
            {
              "name": "id",
              "ty": "BigInt"
            },
            {
              "name": "event_id",
              "ty": "BigInt"
            },
            {
              "name": "client_id",
              "ty": "String"
            },
            {
              "name": "exchange",
              "ty": "String"
            },
            {
              "name": "symbol",
              "ty": "String"
            },
            {
              "name": "order_type",
              "ty": "String"
            },
            {
              "name": "side",
              "ty": "String"
            },
            {
              "name": "price",
              "ty": "Numeric"
            },
            {
              "name": "volume",
              "ty": "Numeric"
            },
            {
              "name": "strategy_id",
              "ty": "Int"
            },
            {
              "name": "datetime",
              "ty": "TimeStampMs"
            },
            {
              "name": "effect",
              "ty": "String"
            },
            {
              "name": "status",
              "ty": "String"
            }
          ]
        }
      }
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserGetOrdersPerStrategyResponse {
    type Request = UserGetOrdersPerStrategyRequest;
}

impl WsRequest for UserSubStrategyOneOrderRequest {
    type Response = UserSubStrategyOneOrderResponse;
    const METHOD_ID: u32 = 20280;
    const SCHEMA: &'static str = r#"{
  "name": "UserSubStrategyOneOrder",
  "code": 20280,
  "parameters": [
    {
      "name": "symbol",
      "ty": {
        "Optional": "String"
      }
    }
  ],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "UserOrder",
          "fields": [
            {
              "name": "id",
              "ty": "BigInt"
            },
            {
              "name": "event_id",
              "ty": "BigInt"
            },
            {
              "name": "client_id",
              "ty": "String"
            },
            {
              "name": "exchange",
              "ty": "String"
            },
            {
              "name": "symbol",
              "ty": "String"
            },
            {
              "name": "order_type",
              "ty": "String"
            },
            {
              "name": "side",
              "ty": "String"
            },
            {
              "name": "price",
              "ty": "Numeric"
            },
            {
              "name": "volume",
              "ty": "Numeric"
            },
            {
              "name": "strategy_id",
              "ty": "Int"
            },
            {
              "name": "datetime",
              "ty": "TimeStampMs"
            },
            {
              "name": "effect",
              "ty": "String"
            },
            {
              "name": "status",
              "ty": "String"
            }
          ]
        }
      }
    }
  ],
  "stream_response": {
    "DataTable": {
      "name": "UserOrder",
      "fields": [
        {
          "name": "id",
          "ty": "BigInt"
        },
        {
          "name": "event_id",
          "ty": "BigInt"
        },
        {
          "name": "client_id",
          "ty": "String"
        },
        {
          "name": "exchange",
          "ty": "String"
        },
        {
          "name": "symbol",
          "ty": "String"
        },
        {
          "name": "order_type",
          "ty": "String"
        },
        {
          "name": "side",
          "ty": "String"
        },
        {
          "name": "price",
          "ty": "Numeric"
        },
        {
          "name": "volume",
          "ty": "Numeric"
        },
        {
          "name": "strategy_id",
          "ty": "Int"
        },
        {
          "name": "datetime",
          "ty": "TimeStampMs"
        },
        {
          "name": "effect",
          "ty": "String"
        },
        {
          "name": "status",
          "ty": "String"
        }
      ]
    }
  },
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserSubStrategyOneOrderResponse {
    type Request = UserSubStrategyOneOrderRequest;
}

impl WsRequest for UserGetLedgerRequest {
    type Response = UserGetLedgerResponse;
    const METHOD_ID: u32 = 20291;
    const SCHEMA: &'static str = r#"{
  "name": "UserGetLedger",
  "code": 20291,
  "parameters": [
    {
      "name": "client_id",
      "ty": {
        "Optional": "String"
      }
    },
    {
      "name": "include_ack",
      "ty": {
        "Optional": "Boolean"
      }
    },
    {
      "name": "strategy_id",
      "ty": "Int"
    },
    {
      "name": "time_start",
      "ty": {
        "Optional": "TimeStampMs"
      }
    },
    {
      "name": "time_end",
      "ty": {
        "Optional": "TimeStampMs"
      }
    },
    {
      "name": "symbol",
      "ty": {
        "Optional": "String"
      }
    }
  ],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "UserLedger",
          "fields": [
            {
              "name": "id",
              "ty": "BigInt"
            },
            {
              "name": "open_order_id",
              "ty": "String"
            },
            {
              "name": "close_order_id",
              "ty": "String"
            },
            {
              "name": "open_order_cloid",
              "ty": "String"
            },
            {
              "name": "close_order_cloid",
              "ty": "String"
            },
            {
              "name": "datetime",
              "ty": "TimeStampMs"
            },
            {
              "name": "exchange",
              "ty": "String"
            },
            {
              "name": "symbol",
              "ty": "String"
            },
            {
              "name": "open_order_position_type",
              "ty": "String"
            },
            {
              "name": "open_order_side",
              "ty": "String"
            },
            {
              "name": "open_price_usd",
              "ty": "Numeric"
            },
            {
              "name": "close_price_usd",
              "ty": "Numeric"
            },
            {
              "name": "volume",
              "ty": "Numeric"
            },
            {
              "name": "closed_profit",
              "ty": "Numeric"
            }
          ]
        }
      }
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserGetLedgerResponse {
    type Request = UserGetLedgerRequest;
}

impl WsRequest for UserGetHedgedOrdersRequest {
    type Response = UserGetHedgedOrdersResponse;
    const METHOD_ID: u32 = 20292;
    const SCHEMA: &'static str = r#"{
  "name": "UserGetHedgedOrders",
  "code": 20292,
  "parameters": [
    {
      "name": "strategy_id",
      "ty": "Int"
    }
  ],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "UserHedgedOrders",
          "fields": [
            {
              "name": "id",
              "ty": "BigInt"
            },
            {
              "name": "leg1_id",
              "ty": "String"
            },
            {
              "name": "leg2_id",
              "ty": "String"
            },
            {
              "name": "leg1_cloid",
              "ty": "String"
            },
            {
              "name": "leg2_cloid",
              "ty": "String"
            },
            {
              "name": "datetime",
              "ty": "TimeStampMs"
            },
            {
              "name": "leg1_ins",
              "ty": "String"
            },
            {
              "name": "leg2_ins",
              "ty": "String"
            },
            {
              "name": "leg1_side",
              "ty": "String"
            },
            {
              "name": "leg2_side",
              "ty": "String"
            },
            {
              "name": "leg1_price",
              "ty": "Numeric"
            },
            {
              "name": "leg2_price",
              "ty": "Numeric"
            },
            {
              "name": "leg1_status",
              "ty": "String"
            },
            {
              "name": "leg2_status",
              "ty": "String"
            },
            {
              "name": "size",
              "ty": "Numeric"
            }
          ]
        }
      }
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserGetHedgedOrdersResponse {
    type Request = UserGetHedgedOrdersRequest;
}

impl WsRequest for UserSubLedgerStrategyOneRequest {
    type Response = UserSubLedgerStrategyOneResponse;
    const METHOD_ID: u32 = 20300;
    const SCHEMA: &'static str = r#"{
  "name": "UserSubLedgerStrategyOne",
  "code": 20300,
  "parameters": [
    {
      "name": "symbol",
      "ty": {
        "Optional": "String"
      }
    }
  ],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "UserLedger",
          "fields": [
            {
              "name": "id",
              "ty": "BigInt"
            },
            {
              "name": "open_order_id",
              "ty": "String"
            },
            {
              "name": "close_order_id",
              "ty": "String"
            },
            {
              "name": "open_order_cloid",
              "ty": "String"
            },
            {
              "name": "close_order_cloid",
              "ty": "String"
            },
            {
              "name": "datetime",
              "ty": "TimeStampMs"
            },
            {
              "name": "exchange",
              "ty": "String"
            },
            {
              "name": "symbol",
              "ty": "String"
            },
            {
              "name": "open_order_position_type",
              "ty": "String"
            },
            {
              "name": "open_order_side",
              "ty": "String"
            },
            {
              "name": "open_price_usd",
              "ty": "Numeric"
            },
            {
              "name": "close_price_usd",
              "ty": "Numeric"
            },
            {
              "name": "volume",
              "ty": "Numeric"
            },
            {
              "name": "closed_profit",
              "ty": "Numeric"
            }
          ]
        }
      }
    }
  ],
  "stream_response": {
    "DataTable": {
      "name": "UserLedger",
      "fields": [
        {
          "name": "id",
          "ty": "BigInt"
        },
        {
          "name": "open_order_id",
          "ty": "String"
        },
        {
          "name": "close_order_id",
          "ty": "String"
        },
        {
          "name": "open_order_cloid",
          "ty": "String"
        },
        {
          "name": "close_order_cloid",
          "ty": "String"
        },
        {
          "name": "datetime",
          "ty": "TimeStampMs"
        },
        {
          "name": "exchange",
          "ty": "String"
        },
        {
          "name": "symbol",
          "ty": "String"
        },
        {
          "name": "open_order_position_type",
          "ty": "String"
        },
        {
          "name": "open_order_side",
          "ty": "String"
        },
        {
          "name": "open_price_usd",
          "ty": "Numeric"
        },
        {
          "name": "close_price_usd",
          "ty": "Numeric"
        },
        {
          "name": "volume",
          "ty": "Numeric"
        },
        {
          "name": "closed_profit",
          "ty": "Numeric"
        }
      ]
    }
  },
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserSubLedgerStrategyOneResponse {
    type Request = UserSubLedgerStrategyOneRequest;
}

impl WsRequest for UserSubLedgerRequest {
    type Response = UserSubLedgerResponse;
    const METHOD_ID: u32 = 20301;
    const SCHEMA: &'static str = r#"{
  "name": "UserSubLedger",
  "code": 20301,
  "parameters": [
    {
      "name": "strategy_id",
      "ty": "Int"
    },
    {
      "name": "symbol",
      "ty": {
        "Optional": "String"
      }
    }
  ],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "UserLedger",
          "fields": [
            {
              "name": "id",
              "ty": "BigInt"
            },
            {
              "name": "open_order_id",
              "ty": "String"
            },
            {
              "name": "close_order_id",
              "ty": "String"
            },
            {
              "name": "open_order_cloid",
              "ty": "String"
            },
            {
              "name": "close_order_cloid",
              "ty": "String"
            },
            {
              "name": "datetime",
              "ty": "TimeStampMs"
            },
            {
              "name": "exchange",
              "ty": "String"
            },
            {
              "name": "symbol",
              "ty": "String"
            },
            {
              "name": "open_order_position_type",
              "ty": "String"
            },
            {
              "name": "open_order_side",
              "ty": "String"
            },
            {
              "name": "open_price_usd",
              "ty": "Numeric"
            },
            {
              "name": "close_price_usd",
              "ty": "Numeric"
            },
            {
              "name": "volume",
              "ty": "Numeric"
            },
            {
              "name": "closed_profit",
              "ty": "Numeric"
            }
          ]
        }
      }
    }
  ],
  "stream_response": {
    "DataTable": {
      "name": "UserLedger",
      "fields": [
        {
          "name": "id",
          "ty": "BigInt"
        },
        {
          "name": "open_order_id",
          "ty": "String"
        },
        {
          "name": "close_order_id",
          "ty": "String"
        },
        {
          "name": "open_order_cloid",
          "ty": "String"
        },
        {
          "name": "close_order_cloid",
          "ty": "String"
        },
        {
          "name": "datetime",
          "ty": "TimeStampMs"
        },
        {
          "name": "exchange",
          "ty": "String"
        },
        {
          "name": "symbol",
          "ty": "String"
        },
        {
          "name": "open_order_position_type",
          "ty": "String"
        },
        {
          "name": "open_order_side",
          "ty": "String"
        },
        {
          "name": "open_price_usd",
          "ty": "Numeric"
        },
        {
          "name": "close_price_usd",
          "ty": "Numeric"
        },
        {
          "name": "volume",
          "ty": "Numeric"
        },
        {
          "name": "closed_profit",
          "ty": "Numeric"
        }
      ]
    }
  },
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserSubLedgerResponse {
    type Request = UserSubLedgerRequest;
}

impl WsRequest for UserGetLiveTestAccuracyLogRequest {
    type Response = UserGetLiveTestAccuracyLogResponse;
    const METHOD_ID: u32 = 20310;
    const SCHEMA: &'static str = r#"{
  "name": "UserGetLiveTestAccuracyLog",
  "code": 20310,
  "parameters": [
    {
      "name": "tag",
      "ty": {
        "Optional": "String"
      }
    },
    {
      "name": "time_start",
      "ty": {
        "Optional": "TimeStampMs"
      }
    },
    {
      "name": "time_end",
      "ty": {
        "Optional": "TimeStampMs"
      }
    }
  ],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "UserAccuracyLog",
          "fields": [
            {
              "name": "datetime",
              "ty": "TimeStampMs"
            },
            {
              "name": "count_pass",
              "ty": "BigInt"
            },
            {
              "name": "count_fail",
              "ty": "BigInt"
            },
            {
              "name": "accuracy",
              "ty": "Numeric"
            }
          ]
        }
      }
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserGetLiveTestAccuracyLogResponse {
    type Request = UserGetLiveTestAccuracyLogRequest;
}

impl WsRequest for UserGetSignal1Request {
    type Response = UserGetSignal1Response;
    const METHOD_ID: u32 = 20320;
    const SCHEMA: &'static str = r#"{
  "name": "UserGetSignal1",
  "code": 20320,
  "parameters": [
    {
      "name": "signal",
      "ty": {
        "Optional": "String"
      }
    },
    {
      "name": "min_level",
      "ty": {
        "Optional": "String"
      }
    },
    {
      "name": "symbol",
      "ty": {
        "Optional": "String"
      }
    },
    {
      "name": "time_start",
      "ty": {
        "Optional": "TimeStampMs"
      }
    },
    {
      "name": "time_end",
      "ty": {
        "Optional": "TimeStampMs"
      }
    }
  ],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "Signal1",
          "fields": [
            {
              "name": "difference",
              "ty": {
                "Optional": {
                  "Struct": {
                    "name": "SignalPriceDifference",
                    "fields": [
                      {
                        "name": "price_binance",
                        "ty": "Numeric"
                      },
                      {
                        "name": "price_hyper",
                        "ty": "Numeric"
                      },
                      {
                        "name": "bp",
                        "ty": "Numeric"
                      },
                      {
                        "name": "used",
                        "ty": "Boolean"
                      }
                    ]
                  }
                }
              }
            },
            {
              "name": "change",
              "ty": {
                "Optional": {
                  "Struct": {
                    "name": "SignalPriceChange",
                    "fields": [
                      {
                        "name": "trend",
                        "ty": "String"
                      },
                      {
                        "name": "time_high",
                        "ty": "BigInt"
                      },
                      {
                        "name": "time_low",
                        "ty": "BigInt"
                      },
                      {
                        "name": "price_high",
                        "ty": "Numeric"
                      },
                      {
                        "name": "price_low",
                        "ty": "Numeric"
                      },
                      {
                        "name": "bp",
                        "ty": "Numeric"
                      },
                      {
                        "name": "used",
                        "ty": "Boolean"
                      }
                    ]
                  }
                }
              }
            },
            {
              "name": "id",
              "ty": "BigInt"
            },
            {
              "name": "datetime",
              "ty": "BigInt"
            },
            {
              "name": "symbol",
              "ty": "String"
            },
            {
              "name": "level",
              "ty": "String"
            }
          ]
        }
      }
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserGetSignal1Response {
    type Request = UserGetSignal1Request;
}

impl WsRequest for UserSubSignal1Request {
    type Response = UserSubSignal1Response;
    const METHOD_ID: u32 = 20330;
    const SCHEMA: &'static str = r#"{
  "name": "UserSubSignal1",
  "code": 20330,
  "parameters": [
    {
      "name": "symbol",
      "ty": {
        "Optional": "String"
      }
    }
  ],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "Signal1",
          "fields": [
            {
              "name": "difference",
              "ty": {
                "Optional": {
                  "Struct": {
                    "name": "SignalPriceDifference",
                    "fields": [
                      {
                        "name": "price_binance",
                        "ty": "Numeric"
                      },
                      {
                        "name": "price_hyper",
                        "ty": "Numeric"
                      },
                      {
                        "name": "bp",
                        "ty": "Numeric"
                      },
                      {
                        "name": "used",
                        "ty": "Boolean"
                      }
                    ]
                  }
                }
              }
            },
            {
              "name": "change",
              "ty": {
                "Optional": {
                  "Struct": {
                    "name": "SignalPriceChange",
                    "fields": [
                      {
                        "name": "trend",
                        "ty": "String"
                      },
                      {
                        "name": "time_high",
                        "ty": "BigInt"
                      },
                      {
                        "name": "time_low",
                        "ty": "BigInt"
                      },
                      {
                        "name": "price_high",
                        "ty": "Numeric"
                      },
                      {
                        "name": "price_low",
                        "ty": "Numeric"
                      },
                      {
                        "name": "bp",
                        "ty": "Numeric"
                      },
                      {
                        "name": "used",
                        "ty": "Boolean"
                      }
                    ]
                  }
                }
              }
            },
            {
              "name": "id",
              "ty": "BigInt"
            },
            {
              "name": "datetime",
              "ty": "BigInt"
            },
            {
              "name": "symbol",
              "ty": "String"
            },
            {
              "name": "level",
              "ty": "String"
            }
          ]
        }
      }
    }
  ],
  "stream_response": {
    "DataTable": {
      "name": "Signal1",
      "fields": [
        {
          "name": "difference",
          "ty": {
            "Optional": {
              "Struct": {
                "name": "SignalPriceDifference",
                "fields": [
                  {
                    "name": "price_binance",
                    "ty": "Numeric"
                  },
                  {
                    "name": "price_hyper",
                    "ty": "Numeric"
                  },
                  {
                    "name": "bp",
                    "ty": "Numeric"
                  },
                  {
                    "name": "used",
                    "ty": "Boolean"
                  }
                ]
              }
            }
          }
        },
        {
          "name": "change",
          "ty": {
            "Optional": {
              "Struct": {
                "name": "SignalPriceChange",
                "fields": [
                  {
                    "name": "trend",
                    "ty": "String"
                  },
                  {
                    "name": "time_high",
                    "ty": "BigInt"
                  },
                  {
                    "name": "time_low",
                    "ty": "BigInt"
                  },
                  {
                    "name": "price_high",
                    "ty": "Numeric"
                  },
                  {
                    "name": "price_low",
                    "ty": "Numeric"
                  },
                  {
                    "name": "bp",
                    "ty": "Numeric"
                  },
                  {
                    "name": "used",
                    "ty": "Boolean"
                  }
                ]
              }
            }
          }
        },
        {
          "name": "id",
          "ty": "BigInt"
        },
        {
          "name": "datetime",
          "ty": "BigInt"
        },
        {
          "name": "symbol",
          "ty": "String"
        },
        {
          "name": "level",
          "ty": "String"
        }
      ]
    }
  },
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserSubSignal1Response {
    type Request = UserSubSignal1Request;
}

impl WsRequest for UserGetEncryptedKeyRequest {
    type Response = UserGetEncryptedKeyResponse;
    const METHOD_ID: u32 = 20340;
    const SCHEMA: &'static str = r#"{
  "name": "UserGetEncryptedKey",
  "code": 20340,
  "parameters": [],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "UserEncryptedKey",
          "fields": [
            {
              "name": "id",
              "ty": "BigInt"
            },
            {
              "name": "exchange",
              "ty": "String"
            },
            {
              "name": "account_id",
              "ty": "String"
            },
            {
              "name": "ciphertext",
              "ty": "String"
            },
            {
              "name": "alias",
              "ty": "String"
            }
          ]
        }
      }
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserGetEncryptedKeyResponse {
    type Request = UserGetEncryptedKeyRequest;
}

impl WsRequest for UserDeleteEncryptedKeyRequest {
    type Response = UserDeleteEncryptedKeyResponse;
    const METHOD_ID: u32 = 20350;
    const SCHEMA: &'static str = r#"{
  "name": "UserDeleteEncryptedKey",
  "code": 20350,
  "parameters": [
    {
      "name": "exchange",
      "ty": "String"
    },
    {
      "name": "account_id",
      "ty": "String"
    }
  ],
  "returns": [
    {
      "name": "success",
      "ty": "Boolean"
    },
    {
      "name": "reason",
      "ty": {
        "Optional": "String"
      }
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserDeleteEncryptedKeyResponse {
    type Request = UserDeleteEncryptedKeyRequest;
}

impl WsRequest for UserDecryptEncryptedKeyRequest {
    type Response = UserDecryptEncryptedKeyResponse;
    const METHOD_ID: u32 = 20360;
    const SCHEMA: &'static str = r#"{
  "name": "UserDecryptEncryptedKey",
  "code": 20360,
  "parameters": [
    {
      "name": "encryption_key",
      "ty": "String"
    },
    {
      "name": "exchange",
      "ty": "String"
    },
    {
      "name": "account_id",
      "ty": "String"
    }
  ],
  "returns": [
    {
      "name": "success",
      "ty": "Boolean"
    },
    {
      "name": "reason",
      "ty": {
        "Optional": "String"
      }
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserDecryptEncryptedKeyResponse {
    type Request = UserDecryptEncryptedKeyRequest;
}

impl WsRequest for UserGetPriceDifferenceRequest {
    type Response = UserGetPriceDifferenceResponse;
    const METHOD_ID: u32 = 20370;
    const SCHEMA: &'static str = r#"{
  "name": "UserGetPriceDifference",
  "code": 20370,
  "parameters": [
    {
      "name": "time_start",
      "ty": {
        "Optional": "TimeStampMs"
      }
    },
    {
      "name": "time_end",
      "ty": {
        "Optional": "TimeStampMs"
      }
    },
    {
      "name": "symbol",
      "ty": "String"
    }
  ],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "PriceDifference",
          "fields": [
            {
              "name": "datetime",
              "ty": "BigInt"
            },
            {
              "name": "binance_price",
              "ty": "Numeric"
            },
            {
              "name": "hyper_ask_price",
              "ty": "Numeric"
            },
            {
              "name": "hyper_bid_price",
              "ty": "Numeric"
            },
            {
              "name": "difference_in_usd",
              "ty": "Numeric"
            },
            {
              "name": "difference_in_basis_points",
              "ty": "Numeric"
            }
          ]
        }
      }
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserGetPriceDifferenceResponse {
    type Request = UserGetPriceDifferenceRequest;
}

impl WsRequest for UserSubPriceDifferenceRequest {
    type Response = UserSubPriceDifferenceResponse;
    const METHOD_ID: u32 = 20380;
    const SCHEMA: &'static str = r#"{
  "name": "UserSubPriceDifference",
  "code": 20380,
  "parameters": [
    {
      "name": "unsubscribe_other_symbol",
      "ty": {
        "Optional": "Boolean"
      }
    },
    {
      "name": "symbol",
      "ty": "String"
    }
  ],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "PriceDifference",
          "fields": [
            {
              "name": "datetime",
              "ty": "BigInt"
            },
            {
              "name": "binance_price",
              "ty": "Numeric"
            },
            {
              "name": "hyper_ask_price",
              "ty": "Numeric"
            },
            {
              "name": "hyper_bid_price",
              "ty": "Numeric"
            },
            {
              "name": "difference_in_usd",
              "ty": "Numeric"
            },
            {
              "name": "difference_in_basis_points",
              "ty": "Numeric"
            }
          ]
        }
      }
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserSubPriceDifferenceResponse {
    type Request = UserSubPriceDifferenceRequest;
}

impl WsRequest for UserSubFundingRatesRequest {
    type Response = UserSubFundingRatesResponse;
    const METHOD_ID: u32 = 20390;
    const SCHEMA: &'static str = r#"{
  "name": "UserSubFundingRates",
  "code": 20390,
  "parameters": [
    {
      "name": "exchange",
      "ty": {
        "Optional": "String"
      }
    },
    {
      "name": "symbol",
      "ty": {
        "Optional": "String"
      }
    },
    {
      "name": "unsub",
      "ty": {
        "Optional": "Boolean"
      }
    }
  ],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "UserFundingRates",
          "fields": [
            {
              "name": "exchange",
              "ty": "String"
            },
            {
              "name": "symbol",
              "ty": "String"
            },
            {
              "name": "rate",
              "ty": "Numeric"
            },
            {
              "name": "datetime",
              "ty": "BigInt"
            }
          ]
        }
      }
    }
  ],
  "stream_response": {
    "DataTable": {
      "name": "UserFundingRates",
      "fields": [
        {
          "name": "exchange",
          "ty": "String"
        },
        {
          "name": "symbol",
          "ty": "String"
        },
        {
          "name": "rate",
          "ty": "Numeric"
        },
        {
          "name": "datetime",
          "ty": "BigInt"
        }
      ]
    }
  },
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserSubFundingRatesResponse {
    type Request = UserSubFundingRatesRequest;
}

impl WsRequest for UserAddBlacklistRequest {
    type Response = UserAddBlacklistResponse;
    const METHOD_ID: u32 = 20400;
    const SCHEMA: &'static str = r#"{
  "name": "UserAddBlacklist",
  "code": 20400,
  "parameters": [
    {
      "name": "strategy_id",
      "ty": "Int"
    },
    {
      "name": "list",
      "ty": {
        "DataTable": {
          "name": "RequestSymbolList",
          "fields": [
            {
              "name": "symbol",
              "ty": "String"
            }
          ]
        }
      }
    }
  ],
  "returns": [
    {
      "name": "success",
      "ty": "Boolean"
    },
    {
      "name": "reason",
      "ty": {
        "Optional": "String"
      }
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserAddBlacklistResponse {
    type Request = UserAddBlacklistRequest;
}

impl WsRequest for UserRemoveBlacklistRequest {
    type Response = UserRemoveBlacklistResponse;
    const METHOD_ID: u32 = 20410;
    const SCHEMA: &'static str = r#"{
  "name": "UserRemoveBlacklist",
  "code": 20410,
  "parameters": [
    {
      "name": "strategy_id",
      "ty": "Int"
    },
    {
      "name": "list",
      "ty": {
        "DataTable": {
          "name": "RequestSymbolList",
          "fields": [
            {
              "name": "symbol",
              "ty": "String"
            }
          ]
        }
      }
    }
  ],
  "returns": [
    {
      "name": "success",
      "ty": "Boolean"
    },
    {
      "name": "reason",
      "ty": {
        "Optional": "String"
      }
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserRemoveBlacklistResponse {
    type Request = UserRemoveBlacklistRequest;
}

impl WsRequest for UserGetBlacklistRequest {
    type Response = UserGetBlacklistResponse;
    const METHOD_ID: u32 = 20420;
    const SCHEMA: &'static str = r#"{
  "name": "UserGetBlacklist",
  "code": 20420,
  "parameters": [
    {
      "name": "strategy_id",
      "ty": "Int"
    }
  ],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "UserSymbolList",
          "fields": [
            {
              "name": "symbol",
              "ty": "String"
            },
            {
              "name": "status",
              "ty": "String"
            },
            {
              "name": "flag",
              "ty": "Boolean"
            }
          ]
        }
      }
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserGetBlacklistResponse {
    type Request = UserGetBlacklistRequest;
}

impl WsRequest for UserGetSymbol2Request {
    type Response = UserGetSymbol2Response;
    const METHOD_ID: u32 = 20430;
    const SCHEMA: &'static str = r#"{
  "name": "UserGetSymbol2",
  "code": 20430,
  "parameters": [
    {
      "name": "symbol",
      "ty": {
        "Optional": "String"
      }
    }
  ],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "UserSymbolList",
          "fields": [
            {
              "name": "symbol",
              "ty": "String"
            },
            {
              "name": "status",
              "ty": "String"
            },
            {
              "name": "flag",
              "ty": "Boolean"
            }
          ]
        }
      }
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserGetSymbol2Response {
    type Request = UserGetSymbol2Request;
}

impl WsRequest for UserGetBestBidAskAcrossExchangesRequest {
    type Response = UserGetBestBidAskAcrossExchangesResponse;
    const METHOD_ID: u32 = 20440;
    const SCHEMA: &'static str = r#"{
  "name": "UserGetBestBidAskAcrossExchanges",
  "code": 20440,
  "parameters": [
    {
      "name": "latest",
      "ty": {
        "Optional": "Boolean"
      }
    },
    {
      "name": "time_start",
      "ty": {
        "Optional": "TimeStampMs"
      }
    },
    {
      "name": "time_end",
      "ty": {
        "Optional": "TimeStampMs"
      }
    },
    {
      "name": "symbol",
      "ty": {
        "Optional": "String"
      }
    }
  ],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "BestBidAskAcrossExchanges",
          "fields": [
            {
              "name": "datetime",
              "ty": "BigInt"
            },
            {
              "name": "binance_ask_price",
              "ty": "Numeric"
            },
            {
              "name": "binance_ask_volume",
              "ty": "Numeric"
            },
            {
              "name": "binance_bid_price",
              "ty": "Numeric"
            },
            {
              "name": "binance_bid_volume",
              "ty": "Numeric"
            },
            {
              "name": "hyper_ask_price",
              "ty": "Numeric"
            },
            {
              "name": "hyper_ask_volume",
              "ty": "Numeric"
            },
            {
              "name": "hyper_bid_price",
              "ty": "Numeric"
            },
            {
              "name": "hyper_bid_volume",
              "ty": "Numeric"
            },
            {
              "name": "bb_ha",
              "ty": "Numeric"
            },
            {
              "name": "ba_hb",
              "ty": "Numeric"
            }
          ]
        }
      }
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserGetBestBidAskAcrossExchangesResponse {
    type Request = UserGetBestBidAskAcrossExchangesRequest;
}

impl WsRequest for UserSubBestBidAskAcrossExchangesRequest {
    type Response = UserSubBestBidAskAcrossExchangesResponse;
    const METHOD_ID: u32 = 20450;
    const SCHEMA: &'static str = r#"{
  "name": "UserSubBestBidAskAcrossExchanges",
  "code": 20450,
  "parameters": [
    {
      "name": "unsubscribe_other_symbol",
      "ty": {
        "Optional": "Boolean"
      }
    },
    {
      "name": "symbol",
      "ty": "String"
    }
  ],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "BestBidAskAcrossExchanges",
          "fields": [
            {
              "name": "datetime",
              "ty": "BigInt"
            },
            {
              "name": "binance_ask_price",
              "ty": "Numeric"
            },
            {
              "name": "binance_ask_volume",
              "ty": "Numeric"
            },
            {
              "name": "binance_bid_price",
              "ty": "Numeric"
            },
            {
              "name": "binance_bid_volume",
              "ty": "Numeric"
            },
            {
              "name": "hyper_ask_price",
              "ty": "Numeric"
            },
            {
              "name": "hyper_ask_volume",
              "ty": "Numeric"
            },
            {
              "name": "hyper_bid_price",
              "ty": "Numeric"
            },
            {
              "name": "hyper_bid_volume",
              "ty": "Numeric"
            },
            {
              "name": "bb_ha",
              "ty": "Numeric"
            },
            {
              "name": "ba_hb",
              "ty": "Numeric"
            }
          ]
        }
      }
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserSubBestBidAskAcrossExchangesResponse {
    type Request = UserSubBestBidAskAcrossExchangesRequest;
}

impl WsRequest for UserGetSignal2Request {
    type Response = UserGetSignal2Response;
    const METHOD_ID: u32 = 20460;
    const SCHEMA: &'static str = r#"{
  "name": "UserGetSignal2",
  "code": 20460,
  "parameters": [
    {
      "name": "signal",
      "ty": {
        "Optional": "String"
      }
    },
    {
      "name": "min_level",
      "ty": {
        "Optional": "String"
      }
    },
    {
      "name": "symbol",
      "ty": {
        "Optional": "String"
      }
    },
    {
      "name": "time_start",
      "ty": {
        "Optional": "TimeStampMs"
      }
    },
    {
      "name": "time_end",
      "ty": {
        "Optional": "TimeStampMs"
      }
    }
  ],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "Signal2",
          "fields": [
            {
              "name": "ba_change",
              "ty": {
                "Optional": {
                  "Struct": {
                    "name": "SignalPriceChangeImmediate",
                    "fields": [
                      {
                        "name": "trend",
                        "ty": "String"
                      },
                      {
                        "name": "exchange",
                        "ty": "String"
                      },
                      {
                        "name": "price_type",
                        "ty": "String"
                      },
                      {
                        "name": "before",
                        "ty": "Numeric"
                      },
                      {
                        "name": "after",
                        "ty": "Numeric"
                      },
                      {
                        "name": "ratio",
                        "ty": "Numeric"
                      },
                      {
                        "name": "used",
                        "ty": "Boolean"
                      }
                    ]
                  }
                }
              }
            },
            {
              "name": "bb_change",
              "ty": {
                "Optional": {
                  "Struct": {
                    "name": "SignalPriceChangeImmediate",
                    "fields": [
                      {
                        "name": "trend",
                        "ty": "String"
                      },
                      {
                        "name": "exchange",
                        "ty": "String"
                      },
                      {
                        "name": "price_type",
                        "ty": "String"
                      },
                      {
                        "name": "before",
                        "ty": "Numeric"
                      },
                      {
                        "name": "after",
                        "ty": "Numeric"
                      },
                      {
                        "name": "ratio",
                        "ty": "Numeric"
                      },
                      {
                        "name": "used",
                        "ty": "Boolean"
                      }
                    ]
                  }
                }
              }
            },
            {
              "name": "ba_hb_diff",
              "ty": {
                "Optional": {
                  "Struct": {
                    "name": "SignalBinAskHypBidDiff",
                    "fields": [
                      {
                        "name": "bin_ask",
                        "ty": "Numeric"
                      },
                      {
                        "name": "hyp_bid",
                        "ty": "Numeric"
                      },
                      {
                        "name": "ratio",
                        "ty": "Numeric"
                      },
                      {
                        "name": "used",
                        "ty": "Boolean"
                      }
                    ]
                  }
                }
              }
            },
            {
              "name": "bb_ha_diff",
              "ty": {
                "Optional": {
                  "Struct": {
                    "name": "SignalBinBidHypAskDiff",
                    "fields": [
                      {
                        "name": "bin_bid",
                        "ty": "Numeric"
                      },
                      {
                        "name": "hyp_ask",
                        "ty": "Numeric"
                      },
                      {
                        "name": "ratio",
                        "ty": "Numeric"
                      },
                      {
                        "name": "used",
                        "ty": "Boolean"
                      }
                    ]
                  }
                }
              }
            },
            {
              "name": "id",
              "ty": "BigInt"
            },
            {
              "name": "datetime",
              "ty": "BigInt"
            },
            {
              "name": "symbol",
              "ty": "String"
            },
            {
              "name": "level",
              "ty": "String"
            }
          ]
        }
      }
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserGetSignal2Response {
    type Request = UserGetSignal2Request;
}

impl WsRequest for UserSubSignal2Request {
    type Response = UserSubSignal2Response;
    const METHOD_ID: u32 = 20470;
    const SCHEMA: &'static str = r#"{
  "name": "UserSubSignal2",
  "code": 20470,
  "parameters": [
    {
      "name": "symbol",
      "ty": {
        "Optional": "String"
      }
    }
  ],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "Signal2",
          "fields": [
            {
              "name": "ba_change",
              "ty": {
                "Optional": {
                  "Struct": {
                    "name": "SignalPriceChangeImmediate",
                    "fields": [
                      {
                        "name": "trend",
                        "ty": "String"
                      },
                      {
                        "name": "exchange",
                        "ty": "String"
                      },
                      {
                        "name": "price_type",
                        "ty": "String"
                      },
                      {
                        "name": "before",
                        "ty": "Numeric"
                      },
                      {
                        "name": "after",
                        "ty": "Numeric"
                      },
                      {
                        "name": "ratio",
                        "ty": "Numeric"
                      },
                      {
                        "name": "used",
                        "ty": "Boolean"
                      }
                    ]
                  }
                }
              }
            },
            {
              "name": "bb_change",
              "ty": {
                "Optional": {
                  "Struct": {
                    "name": "SignalPriceChangeImmediate",
                    "fields": [
                      {
                        "name": "trend",
                        "ty": "String"
                      },
                      {
                        "name": "exchange",
                        "ty": "String"
                      },
                      {
                        "name": "price_type",
                        "ty": "String"
                      },
                      {
                        "name": "before",
                        "ty": "Numeric"
                      },
                      {
                        "name": "after",
                        "ty": "Numeric"
                      },
                      {
                        "name": "ratio",
                        "ty": "Numeric"
                      },
                      {
                        "name": "used",
                        "ty": "Boolean"
                      }
                    ]
                  }
                }
              }
            },
            {
              "name": "ba_hb_diff",
              "ty": {
                "Optional": {
                  "Struct": {
                    "name": "SignalBinAskHypBidDiff",
                    "fields": [
                      {
                        "name": "bin_ask",
                        "ty": "Numeric"
                      },
                      {
                        "name": "hyp_bid",
                        "ty": "Numeric"
                      },
                      {
                        "name": "ratio",
                        "ty": "Numeric"
                      },
                      {
                        "name": "used",
                        "ty": "Boolean"
                      }
                    ]
                  }
                }
              }
            },
            {
              "name": "bb_ha_diff",
              "ty": {
                "Optional": {
                  "Struct": {
                    "name": "SignalBinBidHypAskDiff",
                    "fields": [
                      {
                        "name": "bin_bid",
                        "ty": "Numeric"
                      },
                      {
                        "name": "hyp_ask",
                        "ty": "Numeric"
                      },
                      {
                        "name": "ratio",
                        "ty": "Numeric"
                      },
                      {
                        "name": "used",
                        "ty": "Boolean"
                      }
                    ]
                  }
                }
              }
            },
            {
              "name": "id",
              "ty": "BigInt"
            },
            {
              "name": "datetime",
              "ty": "BigInt"
            },
            {
              "name": "symbol",
              "ty": "String"
            },
            {
              "name": "level",
              "ty": "String"
            }
          ]
        }
      }
    }
  ],
  "stream_response": {
    "DataTable": {
      "name": "Signal2",
      "fields": [
        {
          "name": "ba_change",
          "ty": {
            "Optional": {
              "Struct": {
                "name": "SignalPriceChangeImmediate",
                "fields": [
                  {
                    "name": "trend",
                    "ty": "String"
                  },
                  {
                    "name": "exchange",
                    "ty": "String"
                  },
                  {
                    "name": "price_type",
                    "ty": "String"
                  },
                  {
                    "name": "before",
                    "ty": "Numeric"
                  },
                  {
                    "name": "after",
                    "ty": "Numeric"
                  },
                  {
                    "name": "ratio",
                    "ty": "Numeric"
                  },
                  {
                    "name": "used",
                    "ty": "Boolean"
                  }
                ]
              }
            }
          }
        },
        {
          "name": "bb_change",
          "ty": {
            "Optional": {
              "Struct": {
                "name": "SignalPriceChangeImmediate",
                "fields": [
                  {
                    "name": "trend",
                    "ty": "String"
                  },
                  {
                    "name": "exchange",
                    "ty": "String"
                  },
                  {
                    "name": "price_type",
                    "ty": "String"
                  },
                  {
                    "name": "before",
                    "ty": "Numeric"
                  },
                  {
                    "name": "after",
                    "ty": "Numeric"
                  },
                  {
                    "name": "ratio",
                    "ty": "Numeric"
                  },
                  {
                    "name": "used",
                    "ty": "Boolean"
                  }
                ]
              }
            }
          }
        },
        {
          "name": "ba_hb_diff",
          "ty": {
            "Optional": {
              "Struct": {
                "name": "SignalBinAskHypBidDiff",
                "fields": [
                  {
                    "name": "bin_ask",
                    "ty": "Numeric"
                  },
                  {
                    "name": "hyp_bid",
                    "ty": "Numeric"
                  },
                  {
                    "name": "ratio",
                    "ty": "Numeric"
                  },
                  {
                    "name": "used",
                    "ty": "Boolean"
                  }
                ]
              }
            }
          }
        },
        {
          "name": "bb_ha_diff",
          "ty": {
            "Optional": {
              "Struct": {
                "name": "SignalBinBidHypAskDiff",
                "fields": [
                  {
                    "name": "bin_bid",
                    "ty": "Numeric"
                  },
                  {
                    "name": "hyp_ask",
                    "ty": "Numeric"
                  },
                  {
                    "name": "ratio",
                    "ty": "Numeric"
                  },
                  {
                    "name": "used",
                    "ty": "Boolean"
                  }
                ]
              }
            }
          }
        },
        {
          "name": "id",
          "ty": "BigInt"
        },
        {
          "name": "datetime",
          "ty": "BigInt"
        },
        {
          "name": "symbol",
          "ty": "String"
        },
        {
          "name": "level",
          "ty": "String"
        }
      ]
    }
  },
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserSubSignal2Response {
    type Request = UserSubSignal2Request;
}

impl WsRequest for UserPlaceOrderMarketRequest {
    type Response = UserPlaceOrderMarketResponse;
    const METHOD_ID: u32 = 20520;
    const SCHEMA: &'static str = r#"{
  "name": "UserPlaceOrderMarket",
  "code": 20520,
  "parameters": [
    {
      "name": "exchange",
      "ty": "String"
    },
    {
      "name": "symbol",
      "ty": "String"
    },
    {
      "name": "side",
      "ty": "String"
    },
    {
      "name": "price",
      "ty": "Numeric"
    },
    {
      "name": "size",
      "ty": "Numeric"
    },
    {
      "name": "local_id",
      "ty": "String"
    }
  ],
  "returns": [
    {
      "name": "success",
      "ty": "Boolean"
    },
    {
      "name": "reason",
      "ty": "String"
    },
    {
      "name": "local_id",
      "ty": "String"
    },
    {
      "name": "client_id",
      "ty": "String"
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserPlaceOrderMarketResponse {
    type Request = UserPlaceOrderMarketRequest;
}

impl WsRequest for UserPlaceOrderLimitRequest {
    type Response = UserPlaceOrderLimitResponse;
    const METHOD_ID: u32 = 20521;
    const SCHEMA: &'static str = r#"{
  "name": "UserPlaceOrderLimit",
  "code": 20521,
  "parameters": [
    {
      "name": "exchange",
      "ty": "String"
    },
    {
      "name": "symbol",
      "ty": "String"
    },
    {
      "name": "side",
      "ty": "String"
    },
    {
      "name": "price",
      "ty": "Numeric"
    },
    {
      "name": "size",
      "ty": "Numeric"
    },
    {
      "name": "local_id",
      "ty": "String"
    }
  ],
  "returns": [
    {
      "name": "success",
      "ty": "Boolean"
    },
    {
      "name": "reason",
      "ty": "String"
    },
    {
      "name": "local_id",
      "ty": "String"
    },
    {
      "name": "client_id",
      "ty": "String"
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserPlaceOrderLimitResponse {
    type Request = UserPlaceOrderLimitRequest;
}

impl WsRequest for UserS3CaptureEventRequest {
    type Response = UserS3CaptureEventResponse;
    const METHOD_ID: u32 = 20522;
    const SCHEMA: &'static str = r#"{
  "name": "UserS3CaptureEvent",
  "code": 20522,
  "parameters": [
    {
      "name": "event_id",
      "ty": "BigInt"
    }
  ],
  "returns": [
    {
      "name": "success",
      "ty": "Boolean"
    },
    {
      "name": "reason",
      "ty": "String"
    },
    {
      "name": "local_id",
      "ty": "String"
    },
    {
      "name": "client_id",
      "ty": "String"
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserS3CaptureEventResponse {
    type Request = UserS3CaptureEventRequest;
}

impl WsRequest for UserS3ReleasePositionRequest {
    type Response = UserS3ReleasePositionResponse;
    const METHOD_ID: u32 = 20523;
    const SCHEMA: &'static str = r#"{
  "name": "UserS3ReleasePosition",
  "code": 20523,
  "parameters": [
    {
      "name": "event_id",
      "ty": "BigInt"
    }
  ],
  "returns": [
    {
      "name": "success",
      "ty": "Boolean"
    },
    {
      "name": "reason",
      "ty": "String"
    },
    {
      "name": "local_id",
      "ty": "String"
    },
    {
      "name": "client_id",
      "ty": "String"
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserS3ReleasePositionResponse {
    type Request = UserS3ReleasePositionRequest;
}

impl WsRequest for UserSubStrategy3PositionsOpeningRequest {
    type Response = UserSubStrategy3PositionsOpeningResponse;
    const METHOD_ID: u32 = 20524;
    const SCHEMA: &'static str = r#"{
  "name": "UserSubStrategy3PositionsOpening",
  "code": 20524,
  "parameters": [
    {
      "name": "unsubscribe",
      "ty": {
        "Optional": "Boolean"
      }
    }
  ],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "UserCapturedEvent",
          "fields": [
            {
              "name": "id",
              "ty": "BigInt"
            },
            {
              "name": "event_id",
              "ty": {
                "Optional": "BigInt"
              }
            },
            {
              "name": "cloid",
              "ty": {
                "Optional": "String"
              }
            },
            {
              "name": "exchange",
              "ty": "String"
            },
            {
              "name": "symbol",
              "ty": "String"
            },
            {
              "name": "status",
              "ty": "String"
            },
            {
              "name": "price",
              "ty": {
                "Optional": "Numeric"
              }
            },
            {
              "name": "size",
              "ty": "Numeric"
            },
            {
              "name": "filled_size",
              "ty": "Numeric"
            },
            {
              "name": "cancel_or_close",
              "ty": "String"
            }
          ]
        }
      }
    }
  ],
  "stream_response": {
    "DataTable": {
      "name": "UserCapturedEvent",
      "fields": [
        {
          "name": "id",
          "ty": "BigInt"
        },
        {
          "name": "event_id",
          "ty": {
            "Optional": "BigInt"
          }
        },
        {
          "name": "cloid",
          "ty": {
            "Optional": "String"
          }
        },
        {
          "name": "exchange",
          "ty": "String"
        },
        {
          "name": "symbol",
          "ty": "String"
        },
        {
          "name": "status",
          "ty": "String"
        },
        {
          "name": "price",
          "ty": {
            "Optional": "Numeric"
          }
        },
        {
          "name": "size",
          "ty": "Numeric"
        },
        {
          "name": "filled_size",
          "ty": "Numeric"
        },
        {
          "name": "cancel_or_close",
          "ty": "String"
        }
      ]
    }
  },
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserSubStrategy3PositionsOpeningResponse {
    type Request = UserSubStrategy3PositionsOpeningRequest;
}

impl WsRequest for UserSubStrategy3PositionsClosingRequest {
    type Response = UserSubStrategy3PositionsClosingResponse;
    const METHOD_ID: u32 = 20525;
    const SCHEMA: &'static str = r#"{
  "name": "UserSubStrategy3PositionsClosing",
  "code": 20525,
  "parameters": [
    {
      "name": "unsubscribe",
      "ty": {
        "Optional": "Boolean"
      }
    }
  ],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "UserCapturedEvent",
          "fields": [
            {
              "name": "id",
              "ty": "BigInt"
            },
            {
              "name": "event_id",
              "ty": {
                "Optional": "BigInt"
              }
            },
            {
              "name": "cloid",
              "ty": {
                "Optional": "String"
              }
            },
            {
              "name": "exchange",
              "ty": "String"
            },
            {
              "name": "symbol",
              "ty": "String"
            },
            {
              "name": "status",
              "ty": "String"
            },
            {
              "name": "price",
              "ty": {
                "Optional": "Numeric"
              }
            },
            {
              "name": "size",
              "ty": "Numeric"
            },
            {
              "name": "filled_size",
              "ty": "Numeric"
            },
            {
              "name": "cancel_or_close",
              "ty": "String"
            }
          ]
        }
      }
    }
  ],
  "stream_response": {
    "DataTable": {
      "name": "UserCapturedEvent",
      "fields": [
        {
          "name": "id",
          "ty": "BigInt"
        },
        {
          "name": "event_id",
          "ty": {
            "Optional": "BigInt"
          }
        },
        {
          "name": "cloid",
          "ty": {
            "Optional": "String"
          }
        },
        {
          "name": "exchange",
          "ty": "String"
        },
        {
          "name": "symbol",
          "ty": "String"
        },
        {
          "name": "status",
          "ty": "String"
        },
        {
          "name": "price",
          "ty": {
            "Optional": "Numeric"
          }
        },
        {
          "name": "size",
          "ty": "Numeric"
        },
        {
          "name": "filled_size",
          "ty": "Numeric"
        },
        {
          "name": "cancel_or_close",
          "ty": "String"
        }
      ]
    }
  },
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserSubStrategy3PositionsClosingResponse {
    type Request = UserSubStrategy3PositionsClosingRequest;
}

impl WsRequest for UserCancelOrderRequest {
    type Response = UserCancelOrderResponse;
    const METHOD_ID: u32 = 20530;
    const SCHEMA: &'static str = r#"{
  "name": "UserCancelOrder",
  "code": 20530,
  "parameters": [
    {
      "name": "exchange",
      "ty": "String"
    },
    {
      "name": "symbol",
      "ty": "String"
    },
    {
      "name": "local_id",
      "ty": "String"
    }
  ],
  "returns": [
    {
      "name": "success",
      "ty": "Boolean"
    },
    {
      "name": "reason",
      "ty": {
        "Optional": "String"
      }
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserCancelOrderResponse {
    type Request = UserCancelOrderRequest;
}

impl WsRequest for UserListTradingSymbolsRequest {
    type Response = UserListTradingSymbolsResponse;
    const METHOD_ID: u32 = 20540;
    const SCHEMA: &'static str = r#"{
  "name": "UserListTradingSymbols",
  "code": 20540,
  "parameters": [],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "UserTradingSymbol",
          "fields": [
            {
              "name": "exchange",
              "ty": "String"
            },
            {
              "name": "symbol",
              "ty": "String"
            },
            {
              "name": "base",
              "ty": "String"
            },
            {
              "name": "lot_size",
              "ty": "Numeric"
            },
            {
              "name": "base_decimals",
              "ty": "Int"
            },
            {
              "name": "quote",
              "ty": "String"
            },
            {
              "name": "tick_size",
              "ty": "Numeric"
            },
            {
              "name": "quote_decimals",
              "ty": "Int"
            }
          ]
        }
      }
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserListTradingSymbolsResponse {
    type Request = UserListTradingSymbolsRequest;
}

impl WsRequest for UserGetLiveTestCloseOrder1Request {
    type Response = UserGetLiveTestCloseOrder1Response;
    const METHOD_ID: u32 = 20550;
    const SCHEMA: &'static str = r#"{
  "name": "UserGetLiveTestCloseOrder1",
  "code": 20550,
  "parameters": [],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "UserLiveTestPrice",
          "fields": [
            {
              "name": "symbol",
              "ty": "String"
            },
            {
              "name": "datetime",
              "ty": "BigInt"
            },
            {
              "name": "target_price",
              "ty": "Numeric"
            },
            {
              "name": "last_price",
              "ty": "Numeric"
            },
            {
              "name": "trend_prediction",
              "ty": "String"
            },
            {
              "name": "price_event",
              "ty": "Numeric"
            },
            {
              "name": "price_actual_filled",
              "ty": "Numeric"
            },
            {
              "name": "price_market_when_filled",
              "ty": "Numeric"
            },
            {
              "name": "pass_actual_filled",
              "ty": "Boolean"
            },
            {
              "name": "pass_market_when_filled",
              "ty": "Boolean"
            },
            {
              "name": "last_open_price",
              "ty": "Numeric"
            },
            {
              "name": "last_close_price",
              "ty": "Numeric"
            },
            {
              "name": "last_high_price",
              "ty": "Numeric"
            },
            {
              "name": "last_low_price",
              "ty": "Numeric"
            }
          ]
        }
      }
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserGetLiveTestCloseOrder1Response {
    type Request = UserGetLiveTestCloseOrder1Request;
}

impl WsRequest for UserSubExchangeLatencyRequest {
    type Response = UserSubExchangeLatencyResponse;
    const METHOD_ID: u32 = 20560;
    const SCHEMA: &'static str = r#"{
  "name": "UserSubExchangeLatency",
  "code": 20560,
  "parameters": [
    {
      "name": "unsub",
      "ty": {
        "Optional": "Boolean"
      }
    },
    {
      "name": "time_start",
      "ty": {
        "Optional": "TimeStampMs"
      }
    },
    {
      "name": "time_end",
      "ty": {
        "Optional": "TimeStampMs"
      }
    }
  ],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "UserBenchmarkResult",
          "fields": [
            {
              "name": "id",
              "ty": "BigInt"
            },
            {
              "name": "datetime",
              "ty": "TimeStampMs"
            },
            {
              "name": "exchange",
              "ty": "String"
            },
            {
              "name": "latency_us",
              "ty": "BigInt"
            }
          ]
        }
      }
    }
  ],
  "stream_response": {
    "DataTable": {
      "name": "UserBenchmarkResult",
      "fields": [
        {
          "name": "id",
          "ty": "BigInt"
        },
        {
          "name": "datetime",
          "ty": "TimeStampMs"
        },
        {
          "name": "exchange",
          "ty": "String"
        },
        {
          "name": "latency_us",
          "ty": "BigInt"
        }
      ]
    }
  },
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserSubExchangeLatencyResponse {
    type Request = UserSubExchangeLatencyRequest;
}

impl WsRequest for SubS3TerminalBestAskBestBidRequest {
    type Response = SubS3TerminalBestAskBestBidResponse;
    const METHOD_ID: u32 = 20610;
    const SCHEMA: &'static str = r#"{
  "name": "SubS3TerminalBestAskBestBid",
  "code": 20610,
  "parameters": [
    {
      "name": "unsubscribe_other_symbol",
      "ty": {
        "Optional": "Boolean"
      }
    },
    {
      "name": "symbol",
      "ty": "String"
    }
  ],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "Price",
          "fields": [
            {
              "name": "datetime",
              "ty": "BigInt"
            },
            {
              "name": "symbol",
              "ty": "String"
            },
            {
              "name": "price",
              "ty": "Numeric"
            }
          ]
        }
      }
    }
  ],
  "stream_response": {
    "DataTable": {
      "name": "Price",
      "fields": [
        {
          "name": "datetime",
          "ty": "BigInt"
        },
        {
          "name": "symbol",
          "ty": "String"
        },
        {
          "name": "price",
          "ty": "Numeric"
        }
      ]
    }
  },
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for SubS3TerminalBestAskBestBidResponse {
    type Request = SubS3TerminalBestAskBestBidRequest;
}

impl WsRequest for UserGetBestBidAskAcrossExchangesWithPositionEventRequest {
    type Response = UserGetBestBidAskAcrossExchangesWithPositionEventResponse;
    const METHOD_ID: u32 = 20620;
    const SCHEMA: &'static str = r#"{
  "name": "UserGetBestBidAskAcrossExchangesWithPositionEvent",
  "code": 20620,
  "parameters": [
    {
      "name": "id",
      "ty": {
        "Optional": "BigInt"
      }
    },
    {
      "name": "time_start",
      "ty": {
        "Optional": "TimeStampMs"
      }
    },
    {
      "name": "time_end",
      "ty": {
        "Optional": "TimeStampMs"
      }
    },
    {
      "name": "symbol",
      "ty": {
        "Optional": "String"
      }
    }
  ],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "BestBidAskAcrossExchangesWithPosition",
          "fields": [
            {
              "name": "id",
              "ty": "BigInt"
            },
            {
              "name": "opening_id",
              "ty": "BigInt"
            },
            {
              "name": "datetime",
              "ty": "BigInt"
            },
            {
              "name": "expiry",
              "ty": "BigInt"
            },
            {
              "name": "symbol",
              "ty": "String"
            },
            {
              "name": "ba_bn",
              "ty": "Numeric"
            },
            {
              "name": "bb_bn",
              "ty": "Numeric"
            },
            {
              "name": "ba_amount_bn",
              "ty": "Numeric"
            },
            {
              "name": "bb_amount_bn",
              "ty": "Numeric"
            },
            {
              "name": "ba_hp",
              "ty": "Numeric"
            },
            {
              "name": "bb_hp",
              "ty": "Numeric"
            },
            {
              "name": "ba_amount_hp",
              "ty": "Numeric"
            },
            {
              "name": "bb_amount_hp",
              "ty": "Numeric"
            },
            {
              "name": "hl_balance_coin",
              "ty": "Numeric"
            },
            {
              "name": "ba_balance_coin",
              "ty": "Numeric"
            },
            {
              "name": "opportunity_size",
              "ty": "Numeric"
            },
            {
              "name": "expired",
              "ty": "Boolean"
            },
            {
              "name": "action",
              "ty": "String"
            }
          ]
        }
      }
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserGetBestBidAskAcrossExchangesWithPositionEventResponse {
    type Request = UserGetBestBidAskAcrossExchangesWithPositionEventRequest;
}

impl WsRequest for UserSubBestBidAskAcrossExchangesWithPositionEventRequest {
    type Response = UserSubBestBidAskAcrossExchangesWithPositionEventResponse;
    const METHOD_ID: u32 = 20630;
    const SCHEMA: &'static str = r#"{
  "name": "UserSubBestBidAskAcrossExchangesWithPositionEvent",
  "code": 20630,
  "parameters": [
    {
      "name": "symbol",
      "ty": {
        "Optional": "String"
      }
    }
  ],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "BestBidAskAcrossExchangesWithPosition",
          "fields": [
            {
              "name": "id",
              "ty": "BigInt"
            },
            {
              "name": "opening_id",
              "ty": "BigInt"
            },
            {
              "name": "datetime",
              "ty": "BigInt"
            },
            {
              "name": "expiry",
              "ty": "BigInt"
            },
            {
              "name": "symbol",
              "ty": "String"
            },
            {
              "name": "ba_bn",
              "ty": "Numeric"
            },
            {
              "name": "bb_bn",
              "ty": "Numeric"
            },
            {
              "name": "ba_amount_bn",
              "ty": "Numeric"
            },
            {
              "name": "bb_amount_bn",
              "ty": "Numeric"
            },
            {
              "name": "ba_hp",
              "ty": "Numeric"
            },
            {
              "name": "bb_hp",
              "ty": "Numeric"
            },
            {
              "name": "ba_amount_hp",
              "ty": "Numeric"
            },
            {
              "name": "bb_amount_hp",
              "ty": "Numeric"
            },
            {
              "name": "hl_balance_coin",
              "ty": "Numeric"
            },
            {
              "name": "ba_balance_coin",
              "ty": "Numeric"
            },
            {
              "name": "opportunity_size",
              "ty": "Numeric"
            },
            {
              "name": "expired",
              "ty": "Boolean"
            },
            {
              "name": "action",
              "ty": "String"
            }
          ]
        }
      }
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserSubBestBidAskAcrossExchangesWithPositionEventResponse {
    type Request = UserSubBestBidAskAcrossExchangesWithPositionEventRequest;
}

impl WsRequest for UserGet5MinSpreadMeanRequest {
    type Response = UserGet5MinSpreadMeanResponse;
    const METHOD_ID: u32 = 20640;
    const SCHEMA: &'static str = r#"{
  "name": "UserGet5MinSpreadMean",
  "code": 20640,
  "parameters": [],
  "returns": [
    {
      "name": "data",
      "ty": {
        "DataTable": {
          "name": "PriceSpread",
          "fields": [
            {
              "name": "datetime",
              "ty": "BigInt"
            },
            {
              "name": "exchange_1",
              "ty": "String"
            },
            {
              "name": "exchange_2",
              "ty": "String"
            },
            {
              "name": "asset",
              "ty": "String"
            },
            {
              "name": "spread_buy_1",
              "ty": "Numeric"
            },
            {
              "name": "spread_sell_1",
              "ty": "Numeric"
            }
          ]
        }
      }
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserGet5MinSpreadMeanResponse {
    type Request = UserGet5MinSpreadMeanRequest;
}

impl WsRequest for UserSetS2ConfigureRequest {
    type Response = UserSetS2ConfigureResponse;
    const METHOD_ID: u32 = 20650;
    const SCHEMA: &'static str = r#"{
  "name": "UserSetS2Configure",
  "code": 20650,
  "parameters": [
    {
      "name": "configuration",
      "ty": {
        "DataTable": {
          "name": "DefaultS2Configuration",
          "fields": [
            {
              "name": "buy_exchange",
              "ty": "String"
            },
            {
              "name": "sell_exchange",
              "ty": "String"
            },
            {
              "name": "instrument",
              "ty": "String"
            },
            {
              "name": "order_size",
              "ty": "Numeric"
            },
            {
              "name": "max_unhedged",
              "ty": "Numeric"
            },
            {
              "name": "target_spread",
              "ty": "Numeric"
            },
            {
              "name": "target_position",
              "ty": "Numeric"
            },
            {
              "name": "order_type",
              "ty": "String"
            }
          ]
        }
      }
    }
  ],
  "returns": [
    {
      "name": "success",
      "ty": "Boolean"
    },
    {
      "name": "reason",
      "ty": {
        "Optional": "String"
      }
    }
  ],
  "stream_response": null,
  "description": "",
  "json_schema": null
}"#;
}
impl WsResponse for UserSetS2ConfigureResponse {
    type Request = UserSetS2ConfigureRequest;
}
