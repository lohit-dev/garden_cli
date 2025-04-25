use reqwest::Url;
use starknet::{
    accounts::{Account, ExecutionEncoding, SingleOwnerAccount},
    core::types::TypedData,
    providers::{JsonRpcClient, Provider, jsonrpc::HttpTransport},
    signers::{LocalWallet, Signer, SigningKey},
};
use starknet_crypto::Felt;

pub async fn get_signer_and_account(
    priv_key: Felt,
    address: Felt,
) -> (
    LocalWallet,
    SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>,
) {
    let provider = JsonRpcClient::new(HttpTransport::new(Url::parse("https://starknet-sepolia.g.alchemy.com/starknet/version/rpc/v0_8/zN3JM2LnBeD4lFHMlO_iA8IoQA8Ws9_r").unwrap()));
    let signer = LocalWallet::from(SigningKey::from_secret_scalar(priv_key));

    let chain_id = provider.chain_id().await.unwrap();
    let account: SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet> =
        SingleOwnerAccount::new(
            provider.clone(),
            signer.clone(),
            address,
            chain_id,
            ExecutionEncoding::New,
        );
    (signer, account)
}
pub async fn get_starknet_signature(
    signer: LocalWallet,
    account: SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>,
    redeemer: &str,
    amount: &str,
    timelock: u128,
    secret_hash: &str,
    order_pair: &str,
) -> eyre::Result<starknet_crypto::Signature> {
    use eyre::Context;
    let provider = JsonRpcClient::new(HttpTransport::new(Url::parse("https://starknet-sepolia.g.alchemy.com/starknet/version/rpc/v0_8/zN3JM2LnBeD4lFHMlO_iA8IoQA8Ws9_r").unwrap()));

    // Parse chain ID from the order_pair
    let chain_parts: Vec<&str> = order_pair.split("::").collect();
    if chain_parts.len() != 2 {
        return Err(eyre::eyre!("Invalid order_pair format: {}", order_pair));
    }

    // Parse the source chain (first part of order_pair)
    let source_parts: Vec<&str> = chain_parts[0].split(":").collect();
    if source_parts.len() != 2 {
        return Err(eyre::eyre!(
            "Invalid source chain format in order_pair: {}",
            chain_parts[0]
        ));
    }

    // Get the chain ID from the provider
    let chain_id = provider
        .chain_id()
        .await
        .wrap_err("Failed to get chain ID")?;
    let chain_id_str = chain_id.to_string();

    let raw = format!(
        r#"{{
        "types": {{
            "StarknetDomain": [
                {{ "name": "name", "type": "shortstring" }},
                {{ "name": "version", "type": "shortstring" }},
                {{ "name": "chainId", "type": "shortstring" }},
                {{ "name": "revision", "type": "shortstring" }}
            ],
        "Initiate": [
            {{ "name": "redeemer", "type": "ContractAddress" }},
            {{ "name": "amount", "type": "u256" }},
            {{ "name": "timelock", "type": "u128" }},
            {{ "name": "secretHash", "type": "u128*" }}
        ]
        }},
        "primaryType": "Initiate",
        "domain": {{
            "name": "HTLC",
            "version": "49",
            "chainId": "{}",
            "revision": "1"
        }},
        "message": {{
            "redeemer": "{}",
            "amount": "{}",
            "timelock": "{}",
            "secretHash": "{}"
        }}
    }}"#,
        chain_id_str, redeemer, amount, timelock, secret_hash
    );

    // Get the account address
    let account_address = account.address();

    let typed_data =
        serde_json::from_str::<TypedData>(&raw).wrap_err("Failed to parse message data")?;

    let message_hash = typed_data
        .message_hash(account_address)
        .wrap_err("Failed to get message hash")?;

    let signature = signer
        .sign_hash(&message_hash)
        .await
        .wrap_err("Failed to sign message hash")?;

    Ok(signature)
}
