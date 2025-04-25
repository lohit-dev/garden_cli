use bigdecimal::{BigDecimal, num_bigint};
use eyre::Result;
use reqwest::Url;
use starknet::{
    accounts::{Account, ExecutionEncoding, SingleOwnerAccount},
    core::types::TypedData,
    providers::{JsonRpcClient, Provider, jsonrpc::HttpTransport},
    signers::{LocalWallet, Signer, SigningKey},
};
use starknet_crypto::{Felt, Signature};

/// Converts a BigDecimal value into a tuple of two i128 values
///
/// # Arguments
///
/// * `value` - A reference to a BigDecimal value to be converted
///
/// # Returns
///
/// * `Result<(i128, i128)>` - A Result containing a tuple of:
///   - First element: Lower 128 bits as i128
///   - Second element: Upper 128 bits as i128
///   Or an error if the conversion fails
pub fn bigdecimal_to_i128s(value: &BigDecimal) -> eyre::Result<(i128, i128)> {
    let (bigint, scale) = value.as_bigint_and_exponent();

    let adjusted_bigint = if scale < 0 {
        bigint * num_bigint::BigInt::from(10).pow(-scale as u32)
    } else if scale > 0 {
        bigint / num_bigint::BigInt::from(10).pow(scale as u32)
    } else {
        bigint
    };

    let bytes = adjusted_bigint.to_bytes_le().1;

    let mut padded_bytes = vec![0u8; 32];
    for (i, &byte) in bytes.iter().enumerate().take(32) {
        padded_bytes[i] = byte;
    }

    let low = i128::from_le_bytes(
        padded_bytes[0..16]
            .try_into()
            .map_err(|e| eyre::eyre!("Failed to convert low bytes to array: {}", e))?,
    );
    let high = i128::from_le_bytes(
        padded_bytes[16..32]
            .try_into()
            .map_err(|e| eyre::eyre!("Failed to convert high bytes to array: {}", e))?,
    );

    Ok((low, high))
}

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

/// Convert a hexadecimal string to an array of u32 values (Big Endian)
///
/// # Arguments
///
/// * `hex_string` - A hexadecimal string with "0x" prefix
///
/// # Returns
///
/// * `Result<[u32; 8], String>` - Result containing the u32 array or an error message
pub fn hex_to_u32_array(hex_string: &str) -> Result<[u32; 8]> {
    // Remove "0x" prefix if present
    let hex_str = hex_string.strip_prefix("0x").unwrap_or(hex_string);

    // Check if the string length is correct (8 u32s = 64 hex characters)
    if hex_str.len() != 64 {
        return Err(eyre::eyre!(
            "Invalid hex string length. Expected 64 characters, got {}",
            hex_str.len()
        ));
    }

    // Create array to store results
    let mut result = [0u32; 8];

    // Process 8 characters at a time (one u32)
    for i in 0..8 {
        let start = i * 8;
        let end = start + 8;
        let chunk = &hex_str[start..end];

        // Parse the hex chunk into u32
        result[i] = u32::from_str_radix(chunk, 16)
            .map_err(|e| eyre::eyre!("Failed to parse hex chunk '{}': {}", chunk, e))?;
    }
    Ok(result)
}

pub async fn get_intiate_signature(
    signer: LocalWallet,
    account: SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>,
    amount: BigDecimal,
    redeemer: String,
    secret_hash: String,
    timelock: String,
) -> Result<Signature> {
    let (low, high) =
        bigdecimal_to_i128s(&amount).map_err(|e| eyre::eyre!("Failed to parse amount : {}", e))?;
    let secret_hash =
        hex_to_u32_array(&secret_hash).map_err(|e| eyre::eyre!("Invalid secret Hash: {}", e))?;

    let redeemer =
        Felt::from_hex(&redeemer).map_err(|e| eyre::eyre!("Invalid redeemer address: {}", e))?;
    // This is the expected format of the input
    // Do not change this
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
      "version": 49,
      "chainId": "0x534e5f5345504f4c4941",
      "revision": 1
    }},
    "message": {{
      "redeemer": "{}",
      "amount": {{ "low": "{}" , "high": "{}" }},
      "timelock": "{}",
      "secretHash": [
        {},
        {},
        {},
        {},
        {},
        {},
        {},
        {}
      ]
    }}
  }}"#,
        redeemer,
        format!("0x{:x}", low), // hex encoding
        format!("0x{:x}", high),
        timelock,
        secret_hash[0],
        secret_hash[1],
        secret_hash[2],
        secret_hash[3],
        secret_hash[4],
        secret_hash[5],
        secret_hash[6],
        secret_hash[7]
    );
    let typed_data = serde_json::from_str::<TypedData>(&raw).expect("Failed to parse typed data");
    let message_hash = typed_data
        .message_hash(account.address())
        .expect("Failed to generate message hash");
    let signature = signer.sign_hash(&message_hash).await?;
    Ok(signature)
}
