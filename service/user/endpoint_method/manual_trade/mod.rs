mod cancel_or_close_position;
mod cancel_order;
mod place_order;

use crate::main_core::MainStruct;
pub use cancel_or_close_position::*;
pub use cancel_order::*;
use lib::ws::WebsocketServer;
pub use place_order::*;

pub fn init_endpoints(server: &mut WebsocketServer, main_struct: &mut MainStruct) {
    // manual order
    server.add_handler(MethodUserPlaceOrderMarket::new(main_struct.manual_trade.clone()));
    server.add_handler(MethodUserPlaceOrderLimit::new(main_struct.manual_trade.clone()));
    server.add_handler(MethodUserCancelOrder::new(main_struct.manual_trade.clone()));
    server.add_handler(MethodUserCancelOrClosePosition::new(
        main_struct.manual_trade.clone(),
        main_struct.table_map.volatile.position_manager.clone(),
        main_struct.table_map.volatile.price_map.clone(),
        main_struct.table_map.volatile.instruments.clone(),
    ));
}
