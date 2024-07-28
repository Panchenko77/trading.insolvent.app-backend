# User Service
the main service which has a few functional units running at [main.rs](./main.rs)

## Code Structure
- [/main_core.rs](./main_core.rs) main_core spawns threads and  returns `MainStruct` to user service
- [/db](./db/README.md) database, both persistent and in-memory
- [/event](./event/README.md) event to be sent/received acrosss threads
- [/lib](./lib/README.md) common library across project
- [/service](./service/README.md) functional unit within user core
- [/strategy](./strategy/README.md) strategy to be ran
- [/test](./test/README.md) unit test following the above code structure
- [/benches](./benches/README.md) benchmark test powered by criterion
- [/endpoint_method](./endpoint_method/README.md) method to be called by endpoint handler

## Threads Spawned by Main Core 
| Thread             | Feature                                                |
| ------------------ | ------------------------------------------------------ |
| client_hyper_rest  | publishes oracle/mark price to price updater           |
| client_hyper_ws    | publishes hyper top bid to price updater               |
| client_binance     | publishes top 5 bid average to market data parser      |
| price_parser       | parses market data into in-memory database             |
| strategy           | run strategy                                           |
| yield_monitor      | monitors data yield from exchanges                     |
| price_table_limit  | maintains volatile price table limit size to 1 hour    |
| signal_table_limit | maintains persistent signal table limit size to 1 week |
