use crate::model::agent::{l1, mainnet, testnet};
use crate::model::exchange::request::HyperliquidChain;
use ethers::prelude::{LocalWallet, Signature, Signer, H256};

/// Create a signature for the given connection id
pub async fn sign_l1_action(
    chain: HyperliquidChain,
    wallet: &LocalWallet,
    connection_id: H256,
) -> crate::error::Result<Signature> {
    // This is weird, but it's running ok
    let (chain, source) = match chain {
        HyperliquidChain::Arbitrum => (HyperliquidChain::Dev, "a".to_string()),
        HyperliquidChain::Dev | HyperliquidChain::ArbitrumGoerli => {
            (HyperliquidChain::Dev, "b".to_string())
        }
    };
    sign_l1_action_inner(chain, source, wallet, connection_id).await
}
pub async fn sign_l1_action_inner(
    chain: HyperliquidChain,
    source: String,
    wallet: &LocalWallet,
    connection_id: H256,
) -> crate::error::Result<Signature> {
    let sig = match chain {
        HyperliquidChain::Arbitrum => {
            let payload = mainnet::Agent {
                source: source.to_string(),
                connection_id,
            };
            wallet.sign_typed_data(&payload).await?
        }
        HyperliquidChain::ArbitrumGoerli => {
            let payload = testnet::Agent {
                source: source.to_string(),
                connection_id,
            };
            wallet.sign_typed_data(&payload).await?
        }
        HyperliquidChain::Dev => {
            let payload = l1::Agent {
                source: source.to_string(),
                connection_id,
            };

            wallet.sign_typed_data(&payload).await?
        }
    };
    Ok(sig)
}
