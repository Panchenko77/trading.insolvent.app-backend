CREATE TYPE enum_role AS ENUM ('guest', 'user', 'trader', 'developer', 'admin');
CREATE TYPE enum_block_chain AS ENUM ('EthereumMainnet', 'EthereumGoerli', 'BscMainnet', 'BscTestnet', 'LocalNet', 'EthereumSepolia');
CREATE TYPE enum_dex AS ENUM ('UniSwap', 'PancakeSwap', 'SushiSwap');
CREATE TYPE enum_dex_path_format AS ENUM ('Json', 'TransactionData', 'TransactionHash');
CREATE TYPE enum_service AS ENUM ('auth', 'user');
