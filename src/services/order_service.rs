use crate::models::additional_data::{AdditonalData, SignableAdditionalData};
use crate::models::order::{ApiResponse, AttestedResponse, Order, Status};
use crate::models::quote::{Initiate, InitiateRequest, QuoteRequest, QuoteResponse, RedeemRequest};
use crate::utils::file_utils::{self, OrderData};
use alloy::{
    hex::{FromHex, ToHexExt},
    network::EthereumWallet,
    primitives::Bytes,
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
use rand::Rng;
use reqwest::Client;
use serde_json;
use sha2::Digest;
use std::collections::HashMap;
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
    ) -> Result<(String, String)> {
        info!("🎯 Creating new order with strategy ID: {}", strategy_id);
        let (secret, secret_hash) = self.gen_secret();
        info!("🔑 Generated secret and hash for order");

        info!("📦 Building order parameters...");
        let mut order = Order {
            source_chain: "arbitrum_sepolia".to_string(),
            destination_chain: "starknet_sepolia".to_string(),
            source_asset: "0x795Dcb58d1cd4789169D5F938Ea05E17ecEB68cA".to_string(),
            destination_asset: "0x75cf614ce4ebce29ac622a50cd5151ddfff853159707589a85dd67b9fb1eba"
                .to_string(),
            initiator_source_address: "0x3E53d785995bb74C0B9ba8F71D0d6a0c4d9E6901".to_string(),
            initiator_destination_address:
                "0x056b3ebec13503cb1e1d9691f13fdc9b4ae7015765113345a7355add1e29d7dc".to_string(),
            source_amount: BigDecimal::from(10000),
            destination_amount: BigDecimal::from(5194207362831030i64),
            fee: BigDecimal::from(1),
            nonce: BigDecimal::from(1),
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

        info!("🔍 Getting attested quote...");
        let attested = self.fetch_attested_quote(&signable_order).await?;
        info!("✅ Received attested quote");

        info!("📝 Updating order with attested data...");
        order.additional_data.sig = Some(attested.result.additional_data.sig);
        order.additional_data.input_token_price = attested.result.additional_data.input_token_price;
        order.additional_data.output_token_price =
            attested.result.additional_data.output_token_price;
        info!("✅ Order updated with attested data");

        info!("⏳ Adding small delay before create order request...");
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

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
    ) -> Result<(String, f64, f64)> {
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
            return Ok((
                strategy_id.clone(),
                quote_response.result.input_token_price,
                quote_response.result.output_token_price,
            ));
        }

        warn!("❌ No quotes found in response");
        Err(eyre::eyre!("No quotes found in response"))
    }

    // Fetch attested quote from the API
    pub async fn fetch_attested_quote(
        &self,
        order_params: &Order<SignableAdditionalData>,
    ) -> Result<AttestedResponse> {
        info!("🔍 Getting attested quote for order...");
        info!("📝 Building attestation payload...");
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
    pub async fn initiate_order(&self, order_id: &str, private_key: &str) -> Result<String> {
        info!("🚀 Initiating order {}...", order_id);
        info!("📝 Getting order details for order {}", order_id);

        // Get order details
        let order_details = self.get_order_details(order_id).await?;
        info!(
            "✅ Successfully retrieved order details for order {}",
            order_id
        );

        // Get wallet for signing
        info!("🔑 Creating wallet from private key");
        let (wallet, signer) = self.get_default_wallet(private_key.to_string())?;
        info!("✅ Wallet created successfully");

        // Create the Initiate struct
        info!("📦 Creating initiate struct for order {}", order_id);
        let initiate = Initiate {
            redeemer: alloy::primitives::Address::from_hex(&order_details.source_swap.redeemer)
                .unwrap(),
            timelock: alloy_primitives::Uint::from(order_details.source_swap.timelock as u64),
            amount: order_details.source_swap.amount.parse().unwrap(),
            secretHash: FixedBytes::from_hex(&order_details.source_swap.secret_hash).unwrap(),
        };
        info!("✅ Initiate struct created successfully");

        // Create domain for EIP-712 signing
        info!("📝 Creating EIP-712 domain for signing");
        let domain = eip712_domain! {
            name: "Garden Finance",
            version: "1",
            chain_id: 421614u64,
            verifying_contract: alloy::primitives::Address::from_hex("0x795Dcb58d1cd4789169D5F938Ea05E17ecEB68cA").unwrap(),
        };
        info!("✅ EIP-712 domain created successfully");

        // Sign the initiate data
        info!("✍️ Signing initiate data for order {}", order_id);
        let signature = signer.sign_typed_data(&initiate, &domain).await?;
        info!("✅ Successfully signed initiate data");

        // Create initiate request
        info!("📦 Creating initiate request for order {}", order_id);
        let initiate_request = InitiateRequest {
            order_id: order_id.to_string(),
            signature: signature.to_string(),
            perform_on: "Source".to_string(),
        };
        info!("✅ Initiate request created successfully");

        // Send initiate request
        info!("📤 Sending initiate request for order {}", order_id);
        let response = self
            .client
            .post(format!("{}/initiate", self.api_url))
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
        info!("📦 Creating redeem request for order {}", order_id);

        let redeem_request = RedeemRequest {
            order_id: order_id.to_string(),
            secret: secret.to_string(),
            perform_on: "Destination".to_string(),
        };
        info!("✅ Redeem request created successfully");

        info!("📤 Sending redeem request for order {}", order_id);
        let response = self
            .client
            .post("https://starknet-relayer.hashira.io/redeem")
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
}

#[derive(Debug, serde::Deserialize)]
struct CreateOrderData {
    create_id: String,
}
