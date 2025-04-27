use core::time;
use std::str::FromStr;
use std::time::Duration;

use crate::models::additional_data::{AdditonalData, SignableAdditionalData};
use crate::models::order::{ApiResponse, AttestedResponse, Order, Status};
use crate::models::quote::{
    Initiate, InitiateRequest, QuoteRequest, QuoteResponse, RedeemRequest, StarkInitiateRequst,
};
use crate::services::starknet_services::{get_intiate_signature, get_signer_and_account};
use crate::utils::file_utils::{self};
use alloy::{
    hex::FromHex,
    network::EthereumWallet,
    signers::{
        Signer,
        k256::ecdsa::SigningKey,
        local::{LocalSigner, PrivateKeySigner},
    },
    sol_types::eip712_domain,
};
use alloy_primitives::FixedBytes;
use bigdecimal::BigDecimal;
use chrono::TimeDelta;
use eyre::Result;
use hex;
use reqwest::Client;
use serde_json;
use sha2::Digest;
use starknet::macros::felt;
use starknet_crypto::Felt;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct OrderService {
    client: Client,
    api_url: String,
    api_key: String,
}

impl OrderService {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            api_url: String::from("https://evm-relay-stage.hashira.io"),
            api_key: String::from(
                "AAAAAGnnHDw6OuFGHwJsgTQFutrJ4__itFiulz-_iRsD-c-OBaOhqZ4KWhB98QhtVYybp2UFWcv8O2LIoB7EmL77dkeQZ_4isbYc",
            ),
        }
    }

    // Generate a secret and its hash for the order
    fn gen_secret(&self) -> (String, String) {
        use rand::{RngCore, rngs::OsRng};

        let mut secret = [0u8; 32];
        OsRng.fill_bytes(&mut secret);

        let hash = sha2::Sha256::digest(&secret);
        let hash_str = hex::encode(hash);
        (hex::encode(secret), hash_str)
    }

    // Create an order using the attested quote
    pub async fn create_order(
        &self,
        strategy_id: String,
        input_token_price: f64,
        output_token_price: f64,
        order_pair: &str,
        amount: &str,
        exact_out: bool,
        destination_amount: String,
    ) -> Result<(String, String)> {
        info!("🎯 Creating new order with strategy ID: {}", strategy_id);
        let (secret, secret_hash) = self.gen_secret();
        info!("🔑 Generated secret and hash for order");

        // Parse the order pair to extract chain and asset information
        info!("🔍 Parsing order pair: {}", order_pair);
        let parts: Vec<&str> = order_pair.split("::").collect();
        if parts.len() != 2 {
            return Err(eyre::eyre!("Invalid order pair format: {}", order_pair));
        }

        let (src, dst) = (parts[0], parts[1]);
        let src_parts: Vec<&str> = src.split(':').collect();
        let dst_parts: Vec<&str> = dst.split(':').collect();

        if src_parts.len() != 2 || dst_parts.len() != 2 {
            return Err(eyre::eyre!(
                "Invalid chain:asset format in order pair: {}",
                order_pair
            ));
        }

        let source_chain = src_parts[0];
        let source_asset = src_parts[1];
        let destination_chain = dst_parts[0];
        let destination_asset = dst_parts[1];

        info!("📊 Parsed order details:");
        info!("  🔹 Source chain: {}", source_chain);
        info!("  🔹 Source asset: {}", source_asset);
        info!("  🔹 Destination chain: {}", destination_chain);
        info!("  🔹 Destination asset: {}", destination_asset);

        // Set the correct initiator addresses based on source chain
        let (initiator_source_address, initiator_destination_address) = if source_chain
            .starts_with("starknet")
        {
            // If source is Starknet, use Starknet address as source and EVM address as destination
            (
                "0x056b3ebec13503cb1e1d9691f13fdc9b4ae7015765113345a7355add1e29d7dc".to_string(),
                "0x3E53d785995bb74C0B9ba8F71D0d6a0c4d9E6901".to_string(),
            )
        } else {
            // Otherwise (e.g., for Arbitrum), use EVM address as source and Starknet address as destination
            (
                "0x3E53d785995bb74C0B9ba8F71D0d6a0c4d9E6901".to_string(),
                "0x056b3ebec13503cb1e1d9691f13fdc9b4ae7015765113345a7355add1e29d7dc".to_string(),
            )
        };

        info!("👤 Using initiator addresses:");
        info!("  🔹 Source: {}", initiator_source_address);
        info!("  🔹 Destination: {}", initiator_destination_address);

        // Log all key parameters before parsing amount
        info!("[ORDER PARAMS] amount: {}", amount);
        info!("[ORDER PARAMS] order_pair: {}", order_pair);
        info!("[ORDER PARAMS] strategy_id: {}", strategy_id);
        info!("[ORDER PARAMS] input_token_price: {}", input_token_price);
        info!("[ORDER PARAMS] output_token_price: {}", output_token_price);
        info!("[ORDER PARAMS] exact_out: {}", exact_out);
        // Parse the amount as a BigDecimal
        let source_amount = match BigDecimal::from_str(amount) {
            Ok(amount) => amount,
            Err(_) => return Err(eyre::eyre!("Failed to parse amount: {}", amount)),
        };

        info!("📦 Building order parameters...");
        let mut order = Order {
            source_chain: source_chain.to_string(),
            destination_chain: destination_chain.to_string(),
            source_asset: source_asset.to_string(),
            destination_asset: destination_asset.to_string(),
            initiator_source_address,
            initiator_destination_address,
            source_amount,
            // Sets the destination amount from the quote response
            destination_amount: BigDecimal::from_str(&destination_amount)
                .unwrap_or(BigDecimal::from(0)),
            fee: BigDecimal::from(1),
            nonce: BigDecimal::from_str(&chrono::Utc::now().timestamp_millis().to_string())
                .unwrap(),
            min_destination_confirmations: 2,
            timelock: 7300,
            secret_hash: secret_hash,
            additional_data: AdditonalData {
                deadline: chrono::Utc::now()
                    .checked_add_signed(TimeDelta::minutes(10))
                    .unwrap()
                    .timestamp(),
                input_token_price: input_token_price,
                output_token_price: output_token_price,
                strategy_id: strategy_id,
                sig: None,
                bitcoin_optional_recipient: None,
                instant_refund_tx_bytes: None,
                redeem_tx_bytes: None,
                tx_hash: None,
            },
        };
        info!("✅ Order parameters built successfully");

        info!("📝 Creating signable order...");
        let signable_order = order.signable_order();
        info!("✅ Signable order created");

        // Log the destination amount being used
        info!("💰 Using destination amount: {}", order.destination_amount);

        info!("🔍 Getting attested quote...");
        let attested = self.fetch_attested_quote(&order).await?;
        info!("✅ Received attested quote :{:#?}", attested);

        info!("📝 Updating order with attested data...");
        order.additional_data.sig = Some(attested.result.additional_data.sig);
        order.additional_data.input_token_price = attested.result.additional_data.input_token_price;
        order.additional_data.output_token_price =
            attested.result.additional_data.output_token_price;
        info!("✅ Order updated with attested data");

        info!("⏳ Adding small delay before create order request...");
        // tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        info!("📤 Sending create order request...");
        let res = self
            .client
            .post(format!("{}/create-order", self.api_url))
            .header("accept", "application/json")
            .header("Content-Type", "application/json")
            .header("api-key", &self.api_key)
            .json(&order)
            .send()
            .await?;

        let response_status = res.status();
        let response_text = res.text().await?;
        info!(
            "📥 Received create order response: Status {}",
            response_status
        );

        if !response_status.is_success() {
            warn!(
                "❌ Failed to create order: {} - {}",
                response_status, response_text
            );
            return Err(eyre::eyre!(
                "Failed to create order: {} - {}",
                response_status,
                response_text
            ));
        }

        if response_text.trim().is_empty() {
            warn!("❌ Empty response body received");
            return Err(eyre::eyre!("Empty response body"));
        }

        let response: Result<ApiResponse<String>, _> = serde_json::from_str(&response_text);
        match response {
            Ok(response) => {
                info!("✅ Successfully parsed API response");
                match response.status {
                    Status::Ok => {
                        if let Some(order_id) = response.data {
                            info!("💾 Saving order data to file...");
                            file_utils::save_order_data(&order_id, &secret)?;
                            let mut order_ids =
                                file_utils::load_order_ids().unwrap_or_else(|_| Vec::new());
                            order_ids.push(order_id.clone());
                            file_utils::save_order_ids(&order_ids)?;
                            info!(
                                "✅ Successfully saved order data: order_id={}, secret={}",
                                order_id, secret
                            );
                            Ok((order_id, secret))
                        } else {
                            warn!("❌ No order ID in response");
                            Err(eyre::eyre!("No order ID in response"))
                        }
                    }
                    Status::Error => {
                        let error_msg = response.error.clone().unwrap_or_default();
                        warn!("❌ API returned error status: {}", error_msg);
                        Err(eyre::eyre!("API returned error status: {}", error_msg))
                    }
                }
            }
            Err(e) => {
                warn!("❌ Failed to decode response body: {}", e);
                Err(eyre::eyre!(
                    "Failed to decode response body: {}. Raw body: {}",
                    e,
                    response_text
                ))
            }
        }
    }

    // Get quote for an order
    pub async fn get_quote(
        &self,
        order_pair: &str,
        amount: &str,
        exact_out: bool,
    ) -> Result<(String, f64, f64, String)> {
        info!("💱 Fetching quote for order pair: {}", order_pair);
        info!("📊 Amount: {}, Exact Out: {}", amount, exact_out);

        let quote_request = QuoteRequest {
            order_pair: order_pair.to_string(),
            amount: amount.to_string(),
            exact_out,
        };

        let url = format!(
            "https://testnet.api.hashira.io/quote?order_pair={}&amount={}&exact_out={}",
            quote_request.order_pair, quote_request.amount, quote_request.exact_out
        );

        info!("📤 Sending quote request...");
        let response = self
            .client
            .get(&url)
            .header("accept", "application/json")
            .header("api-key", &self.api_key)
            .send()
            .await?;

        let response_status = response.status();
        let response_text = response.text().await?;
        info!("📥 Received quote response: Status {}", response_status);

        if !response_status.is_success() {
            warn!(
                "❌ Failed to get quote: {} - {}",
                response_status, response_text
            );
            return Err(eyre::eyre!(
                "Failed to get quote: {} - {}",
                response_status,
                response_text
            ));
        }

        info!("📝 Parsing quote response...");
        let quote_response: QuoteResponse = serde_json::from_str(&response_text)?;

        if quote_response.status != "Ok" {
            warn!(
                "❌ Quote response status is not Ok: {}",
                quote_response.status
            );
            return Err(eyre::eyre!("Quote response status is not Ok"));
        }

        if let Some((strategy_id, _)) = quote_response.result.quotes.iter().next() {
            info!("✅ Successfully retrieved quote:");
            info!("  📊 Strategy ID: {}", strategy_id);
            info!(
                "  💰 Input token price: {}",
                quote_response.result.input_token_price
            );
            info!(
                "  💰 Output token price: {}",
                quote_response.result.output_token_price
            );
            // Get the destination amount for this strategy
            let destination_amount = quote_response
                .result
                .quotes
                .get(strategy_id)
                .ok_or_else(|| eyre::eyre!("Strategy ID not found in quotes"))?;

            info!("  💰 Destination amount: {}", destination_amount);

            return Ok((
                strategy_id.clone(),
                quote_response.result.input_token_price,
                quote_response.result.output_token_price,
                destination_amount.clone(),
            ));
        }

        warn!("❌ No quotes found in response");
        Err(eyre::eyre!("No quotes found in response"))
    }

    // Fetch attested quote from the API
    pub async fn fetch_attested_quote(
        &self,
        order_params: &Order<AdditonalData>,
    ) -> Result<AttestedResponse> {
        info!("🔍 Getting attested quote for order...");
        info!("📝 Building attestation payload...");

        info!("HERE ARE THE PARAMS");
        info!("order_params:{:#?}", order_params);
        info!(
            "Source Amount: {:#?}",
            order_params.source_amount.to_string()
        );
        info!(
            "Destination Amount: {:#?}",
            order_params.destination_amount.to_string()
        );

        let payload = serde_json::json!({
            "source_chain": order_params.source_chain,
            "destination_chain": order_params.destination_chain,
            "source_asset": order_params.source_asset,
            "destination_asset": order_params.destination_asset,
            "initiator_source_address": order_params.initiator_source_address,
            "initiator_destination_address": order_params.initiator_destination_address,
            "source_amount": order_params.source_amount.to_string(),
            "destination_amount": order_params.destination_amount.to_string(),
            "fee": order_params.fee.to_string(),
            "nonce": order_params.nonce.to_string(),
            "min_destination_confirmations": order_params.min_destination_confirmations,
            "timelock": order_params.timelock,
            "secret_hash": order_params.secret_hash,
            "additional_data": {
                "strategy_id": order_params.additional_data.strategy_id,
                "bitcoin_optional_recipient": order_params.additional_data.bitcoin_optional_recipient,
            }
        });
        info!("✅ Payload built successfully");

        info!("📤 Sending attestation request...");
        let response = self
            .client
            .post("https://testnet.api.hashira.io/quote/attested")
            .header("accept", "application/json")
            .header("Content-Type", "application/json")
            .header("api-key", &self.api_key)
            .json(&payload)
            .send()
            .await?;

        let response_status = response.status();
        let response_text = response.text().await?;

        info!(
            "📥 Received attestation response: Status {}",
            response_status
        );
        info!("📝 Response body: {}", response_text);

        if !response_status.is_success() {
            warn!(
                "❌ Failed to get attested quote: {} - {}",
                response_status, response_text
            );
            return Err(eyre::eyre!(
                "Failed to get attested quote: {} - {}",
                response_status,
                response_text
            ));
        }

        info!("📝 Parsing attestation response...");
        let attestation: AttestedResponse = serde_json::from_str(&response_text)?;
        info!("✅ Successfully parsed attestation response");
        Ok(attestation)
    }

    // Initiate an order with custom signing
    // Helper function to retry an async operation with exponential backoff
    async fn retry_with_backoff<F, Fut, T>(
        &self,
        operation: F,
        max_retries: usize,
        order_id: &str,
    ) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut retries = 0;
        let mut delay = 1; // Start with 1 second delay

        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    if retries >= max_retries {
                        warn!(
                            "❌ Max retries ({}) reached for order {}: {}",
                            max_retries, order_id, e
                        );
                        return Err(e);
                    }

                    retries += 1;
                    warn!(
                        "⚠️ Attempt {} failed for order {}: {}. Retrying in {} seconds...",
                        retries, order_id, e, delay
                    );

                    // Sleep with Additional backoff
                    tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                    delay += 2;
                }
            }
        }
    }

    pub async fn initiate_order(&self, order_id: &str, private_key: &str) -> Result<String> {
        info!("🚀 Initiating order {}...", order_id);
        info!("📝 Getting order details for order {}", order_id);

        // Get order details
        let order_details = self.get_order_details(order_id).await?;
        info!(
            "✅ Successfully retrieved order details for order {}",
            order_id
        );

        // Check if the source chain is starknet
        let is_starknet_source = order_details
            .result
            .create_order
            .source_chain
            .starts_with("starknet");
        info!(
            "🔍 Source chain: {}, is starknet: {}",
            order_details.result.create_order.source_chain, is_starknet_source
        );

        // Construct the order pair for signature generation
        let order_pair = format!(
            "{}:{}::{}:{}",
            order_details.result.create_order.source_chain,
            order_details.result.create_order.source_asset,
            order_details.result.create_order.destination_chain,
            order_details.result.create_order.destination_asset
        );
        info!("🔗 Order pair: {}", order_pair);

        // Get wallet for signing
        info!("🔑 Creating wallet from private key");

        // Different signing process based on source chain
        // let signature_str =
        // if is_starknet_source {
        // Use starknet signing
        info!("🔐 Using Starknet signing method");
        let (signer, account) = get_signer_and_account(
            Felt::from_hex(private_key).unwrap(),
            felt!("0x056b3ebec13503cb1e1d9691f13fdc9b4ae7015765113345a7355add1e29d7dc"),
        )
        .await;

        info!("✅ Starknet wallet created successfully");

        // Get the redeemer, amount, timelock, and secret_hash from order details
        let redeemer = &order_details.result.source_swap.redeemer;
        let amount = order_details
            .result
            .source_swap
            .amount
            .parse::<BigDecimal>()
            .expect("Failed to parse amount");
        let timelock = order_details.result.source_swap.timelock as u128;
        let secret_hash = &order_details.result.source_swap.secret_hash;

        info!("📦 Preparing Starknet signature parameters");
        info!("  🔹 Redeemer: {}", redeemer);
        info!("  🔹 Amount: {}", amount);
        info!("  🔹 Timelock: {}", timelock);
        info!("  🔹 Secret Hash: {}", secret_hash);

        // Call the starknet signature function
        info!("✍️ Signing with Starknet for order {}", order_id);
        let signature = get_intiate_signature(
            signer,
            account,
            amount,
            redeemer.to_string(),
            secret_hash.to_string(),
            timelock.to_string(),
        )
        .await
        .unwrap();

        info!("✅ Successfully signed with Starknet");

        // } else {
        //     // Use EVM signing (original implementation)
        //     info!("🔐 Using EVM signing method");
        //     let (wallet, signer) = self.get_default_wallet(private_key.to_string())?;
        //     info!("✅ EVM wallet created successfully");

        //     // Create the Initiate struct
        //     info!("📦 Creating initiate struct for order {}", order_id);
        //     let initiate = Initiate {
        //         redeemer: alloy::primitives::Address::from_hex(
        //             &order_details.result.source_swap.redeemer,
        //         )
        //         .unwrap(),
        //         timelock: alloy_primitives::Uint::from(
        //             order_details.result.source_swap.timelock as u64,
        //         ),
        //         amount: order_details.result.source_swap.amount.parse().unwrap(),
        //         secretHash: FixedBytes::from_hex(&order_details.result.source_swap.secret_hash)
        //             .unwrap(),
        //     };
        //     info!("✅ Initiate struct created successfully");

        //     // Create domain for EIP-712 signing
        //     info!("📝 Creating EIP-712 domain for signing");
        //     let domain = eip712_domain! {
        //         name: "HTLC".to_string(),
        //         version: "1".to_string(),
        //         chain_id: 421614u64,
        //         verifying_contract: alloy::primitives::Address::from_hex("0x795Dcb58d1cd4789169D5F938Ea05E17ecEB68cA").unwrap(),
        //     };
        //     info!("✅ EIP-712 domain created successfully");

        //     // Sign the initiate data
        //     info!("✍️ Signing initiate data for order {}", order_id);
        //     let signature = signer.sign_typed_data(&initiate, &domain).await?;
        //     info!("✅ Successfully signed initiate data");

        //     signature.to_string()
        // };

        // Create initiate request
        info!("📦 Creating initiate request for order {}", order_id);
        let initiate_request = StarkInitiateRequst {
            order_id: order_id.to_string(),
            signature: vec![signature.r.to_string(), signature.s.to_string()],
            perform_on: "Source".to_string(),
        };
        info!("✅ Initiate request created successfully");

        // Send initiate request with retry
        info!("📤 Sending initiate request for order {}", order_id);
        // tokio::time::sleep(Duration::from_secs(1)).await;

        // Use retry with backoff for the API call
        self.retry_with_backoff(
            || async {
                // Use different URL for Starknet source chains
                let url = if is_starknet_source {
                    "https://starknet-relayer.hashira.io/initiate".to_string()
                } else {
                    format!("{}/initiate", self.api_url)
                };

                info!("🔗 Using initiate URL: {}", url);

                let response = self
                    .client
                    .post(url)
                    .timeout(Duration::from_secs(10))
                    .header("api-key", &self.api_key)
                    .json(&initiate_request)
                    .send()
                    .await?;

                let response_status = response.status();
                let response_text = response.text().await?;
                info!(
                    "📥 Received initiate response for order {}: Status {}",
                    order_id, response_status
                );

                if !response_status.is_success() {
                    warn!(
                        "❌ Failed to initiate order {}: {} - {}",
                        order_id, response_status, response_text
                    );
                    return Err(eyre::eyre!(
                        "Failed to initiate order: {} - {}",
                        response_status,
                        response_text
                    ));
                }

                let response: ApiResponse<String> = serde_json::from_str(&response_text)?;
                match response.status {
                    Status::Ok => {
                        if let Some(tx_hash) = response.data {
                            info!(
                                "✅ Successfully initiated order {} with tx hash: {}",
                                order_id, tx_hash
                            );
                            Ok(tx_hash)
                        } else {
                            warn!("❌ No transaction hash in response for order {}", order_id);
                            Err(eyre::eyre!("No transaction hash in response"))
                        }
                    }
                    Status::Error => {
                        let error_msg = response.error.unwrap_or_default();
                        warn!("❌ API error for order {}: {}", order_id, error_msg);
                        Err(eyre::eyre!("API error: {}", error_msg))
                    }
                }
            },
            5,
            order_id,
        )
        .await
    }

    // Get order details for initiation
    async fn get_order_details(&self, order_id: &str) -> Result<OrderDetails> {
        info!("📝 Fetching order details for order {}", order_id);
        let url = format!(
            "https://testnet.api.hashira.io/orders/id/matched/{}",
            order_id
        );

        let response = self
            .client
            .get(&url)
            .header("accept", "application/json")
            .header("api-key", &self.api_key)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;
        info!(
            "📥 Received order details response for order {}: Status {}",
            order_id, status
        );

        if !status.is_success() {
            warn!(
                "❌ Failed to get order details for order {}: {} - {}",
                order_id, status, text
            );
            return Err(eyre::eyre!(
                "Failed to get order details: {} - {}",
                status,
                text
            ));
        }

        let order_details: OrderDetails = serde_json::from_str(&text)?;
        info!(
            "✅ Successfully parsed order details for order {}",
            order_id
        );
        Ok(order_details)
    }

    // Helper function to create a wallet from a private key
    fn get_default_wallet(
        &self,
        private_key: String,
    ) -> Result<(EthereumWallet, LocalSigner<SigningKey>)> {
        let signer = PrivateKeySigner::from_bytes(
            &FixedBytes::from_hex(private_key).expect("Invalid private key"),
        )?;
        Ok((EthereumWallet::from(signer.clone()), signer))
    }

    // Redeem an order
    pub async fn redeem_order(&self, order_id: &str, secret: &str) -> Result<String> {
        info!("🎁 Redeeming order {}...", order_id);

        // Get order details to determine destination chain
        info!("📝 Getting order details for order {}", order_id);
        let order_details = self.get_order_details(order_id).await?;

        // Check if the destination chain is starknet
        let is_starknet_destination = order_details
            .result
            .create_order
            .destination_chain
            .starts_with("starknet");
        info!(
            "🔍 Destination chain: {}, is starknet: {}",
            order_details.result.create_order.destination_chain, is_starknet_destination
        );

        info!("📦 Creating redeem request for order {}", order_id);
        let redeem_request = RedeemRequest {
            order_id: order_id.to_string(),
            secret: secret.to_string(),
            perform_on: "Destination".to_string(),
        };
        info!("✅ Redeem request created successfully");

        // Use different URL based on destination chain
        let url = if is_starknet_destination {
            "https://starknet-relayer.hashira.io/redeem"
        } else {
            "https://evm-relay-stage.hashira.io/redeem"
        };
        info!("🔗 Using redeem URL: {}", url);

        info!("📤 Sending redeem request for order {}", order_id);
        let response = self
            .client
            .post(url)
            .header("accept", "application/json")
            .header("Content-Type", "application/json")
            .header("api-key", &self.api_key)
            .json(&redeem_request)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;
        info!(
            "📥 Received redeem response for order {}: Status {}",
            order_id, status
        );

        if !status.is_success() {
            warn!(
                "❌ Failed to redeem order {}: {} - {}",
                order_id, status, text
            );
            return Err(eyre::eyre!("Failed to redeem order: {} - {}", status, text));
        }

        info!(
            "✅ Successfully redeemed order {} with secret {}",
            order_id, secret
        );
        Ok(text)
    }

    // Check if an order is ready for redemption
    pub async fn is_order_ready_for_redemption(&self, order_id: &str) -> Result<bool> {
        // Get the order details
        let order_details = self.get_order_details(order_id).await?;

        // Just check if the destination swap has been initiated
        let has_initiate_tx = !order_details
            .result
            .destination_swap
            .initiate_tx_hash
            .trim()
            .is_empty()
            && order_details.result.destination_swap.initiate_tx_hash != "0x";

        Ok(has_initiate_tx)
    }

    // Retry redeeming an order up to max_attempts times
    pub async fn retry_redeem_order(
        &self,
        order_id: &str,
        secret: &str,
        max_attempts: usize,
    ) -> Result<String> {
        let max_attempts = if max_attempts == 0 { 10 } else { max_attempts };
        for attempt in 1..=max_attempts {
            info!(
                "🔄 Redemption attempt {}/{} for order {}",
                attempt, max_attempts, order_id
            );

            // First check if the order is ready for redemption
            match self.is_order_ready_for_redemption(order_id).await {
                Ok(true) => {
                    // Order is ready, attempt to redeem it
                    match self.redeem_order(order_id, secret).await {
                        Ok(result) => {
                            info!(
                                "✅ Successfully redeemed order {} on attempt {}/{}",
                                order_id, attempt, max_attempts
                            );
                            return Ok(result);
                        }
                        Err(e) => {
                            warn!(
                                "❌ Redemption attempt {}/{} failed for order {}: {}",
                                attempt, max_attempts, order_id, e
                            );
                            if attempt == max_attempts {
                                return Err(eyre::eyre!(
                                    "Failed to redeem order after {} attempts: {}",
                                    max_attempts,
                                    e
                                ));
                            }
                        }
                    }
                }
                Ok(false) => {
                    info!(
                        "⏳ Order {} is not yet ready for redemption (attempt {}/{})",
                        order_id, attempt, max_attempts
                    );
                    if attempt == max_attempts {
                        return Err(eyre::eyre!(
                            "Order not ready for redemption after {} attempts",
                            max_attempts
                        ));
                    }
                }
                Err(e) => {
                    warn!(
                        "❌ Failed to check if order {} is ready for redemption (attempt {}/{}): {}",
                        order_id, attempt, max_attempts, e
                    );
                    if attempt == max_attempts {
                        return Err(eyre::eyre!(
                            "Failed to check order redemption status after {} attempts: {}",
                            max_attempts,
                            e
                        ));
                    }
                }
            }

            // Wait before the next attempt (exponential backoff)
            let wait_time = std::time::Duration::from_secs(2u64.pow(attempt as u32));
            info!("⏱️ Waiting {:?} before next redemption attempt", wait_time);
            tokio::time::sleep(wait_time).await;
        }

        // This should never be reached due to the returns in the loop, but just in case
        Err(eyre::eyre!(
            "Failed to redeem order after {} attempts",
            max_attempts
        ))
    }

    pub async fn redeem_all_orders(&self, private_key: &str) -> Result<()> {
        info!("🔄 Starting to redeem all stored orders...");

        let order_data = file_utils::load_order_data()?;
        if order_data.is_empty() {
            warn!("❌ No orders found to redeem");
            return Err(eyre::eyre!("No orders found to redeem"));
        }

        let total_orders = order_data.len();
        info!("📊 Found {} orders to redeem", total_orders);
        let mut successful_redemptions = 0;
        let mut failed_redemptions = 0;

        for data in order_data {
            info!("🔄 Processing order {}...", data.order_id);
            match self.redeem_order(&data.order_id, &data.secret).await {
                Ok(tx_hash) => {
                    info!(
                        "✅ Successfully redeemed order {} with tx hash: {}",
                        data.order_id, tx_hash
                    );
                    successful_redemptions += 1;
                }
                Err(e) => {
                    warn!("❌ Failed to redeem order {}: {}", data.order_id, e);
                    failed_redemptions += 1;
                }
            }
            // Small delay between redemptions
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        info!("📊 Redemption Summary:");
        info!("  📈 Total orders: {}", total_orders);
        info!("  ✅ Successful: {}", successful_redemptions);
        info!("  ❌ Failed: {}", failed_redemptions);
        info!(
            "  📊 Success rate: {:.2}%",
            (successful_redemptions as f64 / total_orders as f64) * 100.0
        );

        Ok(())
    }
}

// Structure to hold order details needed for initiation
#[derive(Debug, serde::Deserialize)]
struct OrderDetails {
    result: OrderResult,
}

#[derive(Debug, serde::Deserialize)]
struct OrderResult {
    source_swap: SwapData,
    destination_swap: SwapData,
    create_order: CreateOrderData,
}

#[derive(Debug, serde::Deserialize)]
struct SwapData {
    redeemer: String,
    timelock: i64,
    amount: String,
    secret_hash: String,
    chain: String,
    asset: String,
    initiator: String,
    filled_amount: String,
    secret: String,
    initiate_tx_hash: String,
    redeem_tx_hash: String,
    refund_tx_hash: String,
    initiate_block_number: String,
    redeem_block_number: String,
    refund_block_number: String,
    required_confirmations: i64,
    current_confirmations: i64,
}

#[derive(Debug, serde::Deserialize)]
struct CreateOrderData {
    create_id: String,
    block_number: String,
    source_chain: String,
    destination_chain: String,
    source_asset: String,
    destination_asset: String,
    initiator_source_address: String,
    initiator_destination_address: String,
    source_amount: String,
    destination_amount: String,
    fee: String,
    nonce: String,
    min_destination_confirmations: i64,
    timelock: i64,
    secret_hash: String,
    user_id: String,
    additional_data: AdditionalData,
}

#[derive(Debug, serde::Deserialize)]
struct AdditionalData {
    strategy_id: String,
    input_token_price: f64,
    output_token_price: f64,
    sig: String,
    deadline: i64,
    tx_hash: String,
    is_blacklisted: bool,
}
