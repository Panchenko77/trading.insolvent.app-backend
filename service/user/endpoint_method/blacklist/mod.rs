use crate::endpoint_method::blacklist::add_blacklist::MethodUserAddBlacklist;
use crate::endpoint_method::blacklist::get_blacklist::MethodUserGetBlacklist;
use crate::endpoint_method::blacklist::remove_blacklist::MethodUserRemoveBlacklist;
use crate::main_core::MainStruct;
use lib::ws::WebsocketServer;

mod add_blacklist;
mod get_blacklist;
mod remove_blacklist;

pub fn init_endpoints(server: &mut WebsocketServer, main_struct: &mut MainStruct) {
    server.add_handler(MethodUserAddBlacklist {
        table_symbol_flag: main_struct.table_map.persistent.symbol_flag.clone(),
    });
    server.add_handler(MethodUserGetBlacklist {
        table_symbol_flag: main_struct.table_map.persistent.symbol_flag.clone(),
    });
    server.add_handler(MethodUserRemoveBlacklist {
        table_symbol_flag: main_struct.table_map.persistent.symbol_flag.clone(),
    });
}
