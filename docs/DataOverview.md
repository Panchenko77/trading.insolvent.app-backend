# Detailed Document on WorkTable and GlueSQL

## 1. Introduction
This document provides an overview of the `WorkTable` and `GlueSQL` libraries in the context of the `trading.insolvent` project. It covers their purposes, usage scenarios, and key functionalities.

## 2. WorkTable

### Overview
`WorkTable` is a Rust library for managing tabular data, akin to a data table or data grid. It supports operations for inserting, updating, retrieving, and removing rows, providing a structured approach to handle rows and columns.

### Features
- **Modular Design**: Organized into several modules, including `columns`, `fields`, `rows`, `tables`, `types`, and `values`, ensuring clean and maintainable code.
- **Use Case**: Utilized for in-memory table operations, specifically for managing balances, orders, and positions.

### Directory Structure
- **Location**: `service/user/db/worktable`
- **Modules**: 
  - `balance`
  - `order_manager`
  - `orders`
  - `position_manager`
  - `positions`

### Usage
`WorkTable` is used in the `db` module, which handles various aspects of database management, including both SQL-based and in-memory operations. This module includes:
- **`gluesql`**: Defines schemas and SQL-related operations.
- **`worktable`**: Focuses on in-memory data management.

## 3. GlueSQL

### Overview
`GlueSQL` is a SQL database library for Rust that provides SQL querying capabilities.

### Components
- **Traits and Structs**:
  - `DbRow`
  - `Table<G, D>`
  - `QueryStatement<R>`

- **Key Methods**:
  - `Table::new`
  - `Table::insert`
  - `Table::insert_to`
  - `Table::upsert`
  - `Table::next_index`
  - `Table::set_index`

- **Trait Implementations**:
  - `Clone` for `Table`: Allows cloning of table instances.
  - `Deref` and `DerefMut` for `Table`: Enables dereferencing to the inner `Glue` instance.

- **Async Traits**:
  - `TableCreate`
  - `TableSelectItem`
  - `TableUpdateItem`
  - `TableOverwriteItem`

  These asynchronous traits define methods for creating tables, selecting items, updating items, and overwriting rows.

### Usage
`GlueSQL` is used in the `service/user` crate and relies on Sled Storage for database operations. The library offers strong typing and async support, leveraging Rust features like traits, generics, and async/await for powerful database interactions.

- **Location**: `lib/gluesql/common.rs`

  Provides a flexible framework for managing GlueSQL databases, facilitating robust and efficient database operations.




# Data object usage

## WorkTable
- **`tblBalances`**: Handles balance-related data operations.
- **`tblOrders`**: Manages order data including creation, updates, and retrievals.
- **`tblPositions`**: Manages position data for various entities.
- **`order_manager`**: Handles operations related to order management.
- **`position_manager`**: Manages operations related to position data.

## GlueSQL
`GlueSQL` is used for SQL querying and database interactions, primarily involving:

- **Volatile Tables**
  price_worktable: Handles temporary price-related computations and operations.
  signal_price_spread_worktable: Manages temporary data for price spread signals.
  signal_price_difference: Handles temporary computations for price differences.
  signal_price_change: Manages temporary data for price changes.
  signal_price_difference_generic: Handles generic price difference computations.
  signal_price_change_immediate: Manages immediate price change data.
  accuracy: Stores temporary accuracy-related metrics.
  price_volume: Handles temporary computations related to price and volume.
  index_price_volume: Manages index-related price and volume data.
  event_price_change: Handles temporary data for price change events.
  event_price_spread_and_position: Manages data related to price spread and position events.
  funding_rate: Stores temporary funding rate data.
  portfolios: Manages temporary portfolio data.
  livetest_fill: Handles live test data fills.
  bench: Manages benchmark-related data.
  worktable_balance: Handles balance-related temporary computations.
  worktable_filled_open_order: Manages data for filled and open orders.
  strategy_status: Stores temporary data for strategy statuses.
  order_manager: Handles temporary operations related to order management.
  position_manager: Manages temporary operations related to position data.
  candlestick: Handles temporary candlestick data.
  instruments: Manages temporary instrument-related data.
  price_map: Stores temporary mappings for price data.
  spread_table: Handles temporary data for spread calculations.
  spread_mean: Manages temporary mean spread calculations.

- **Persistent Tables**
  version: Stores version information for data management.
  symbol_flag: Manages flags related to symbols.
  key: Handles key-value pair data.
  order: Manages persistent order data including creation, updates, and retrievals.
  ledger: Stores ledger data for transactions and operations.
  trade_status: Manages the status of trades.

- **Database Schema**: Defines schemas for tables and queries.
- **`TableCreate`**: Asynchronous Trait for creating tables.
- **`TableSelectItem`**: Asynchronous Trait for selecting items from tables.
- **`TableUpdateItem`**: Asynchronous Trait for updating items in tables.
- **`TableOverwriteItem`**: Asynchronous Trait for overwriting rows in tables.
- **`TableUpdateField`**: Asynchronous Trait for updating specific fields in tables.
- **`TableDeleteItem`**: Asynchronous Trait for deleting items from tables.
- **`TableGetIndex`**: Asynchronous Trait for getting the last index in tables.

## Vectors Usage
- Dynamic Collection
- Serialization and Deserialization
- Utility and Manipulation
- Efficient Iteration
- Flexibility
