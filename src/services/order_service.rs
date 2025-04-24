use crate::models::additional_data::{AdditonalData, SignableAdditionalData};
use crate::models::order::{ApiResponse, AttestedResponse, Order, Status};
use crate::models::quote::{Initiate, InitiateRequest, QuoteRequest, QuoteResponse, RedeemRequest};
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
use rand::Rng;
use reqwest::Client;
use serde_json;
use sha2::Digest;
use tracing::{info, warn};
use hex;

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
        use rand::{rngs::OsRng, RngCore};

        let mut secret = [0u8; 32];
        OsRng.fill_bytes(&mut secret);

        let hash = sha2::Sha256::digest(&secret);
        let hash_str = hex::encode(hash);
        (hex::encode(secret), hash_str)
    }

    // Create an order using the attested quote
    pub async fn create_order(&self) -> Result<(String, String)> {
        let (secret, secret_hash) = self.gen_secret();
        info!("Creating order...");

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
            min_destination_confirmations: 4,
            timelock: 7300,
            secret_hash: secret_hash,
            additional_data: AdditonalData {
                deadline: chrono::Utc::now()
                    .checked_add_signed(TimeDelta::minutes(10))
                    .unwrap()
                    .timestamp(),
                input_token_price: 1.0,
                output_token_price: 1.0,
                strategy_id: "asacssab".to_string(),
                sig: None,
                bitcoin_optional_recipient: None,
                instant_refund_tx_bytes: None,
                redeem_tx_bytes: None,
                tx_hash: None,
            },
        };

        let signable_order = order.signable_order();

        let attested = self.fetch_attested_quote(&signable_order).await?;

        // populating unique values
        order.additional_data.sig = Some(attested.result.additional_data.sig);
        order.additional_data.input_token_price = attested.result.additional_data.input_token_price;
        order.additional_data.output_token_price =
            attested.result.additional_data.output_token_price;

        // Add a small delay before sending the create order request
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        let res = self
            .client
            .post("https://testnet.api.hashira.io/orders/gasless/order")
            .header("accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&order) // Send the complete Order structure, not just the attested response
            .send()
            .await?;

        let response_status = res.status();
        let response_text = res.text().await?;
        info!("Create Order API Status: {}", response_status);
        info!("Raw Response Body: '{}'", response_text);

        if !response_status.is_success() {
            return Err(eyre::eyre!(
                "Failed to create order: {} - {}",
                response_status,
                response_text
            ));
        }

        if response_text.trim().is_empty() {
            warn!("Response body is empty, cannot decode as JSON");
            return Err(eyre::eyre!("Empty response body"));
        }

        let response: Result<ApiResponse<String>, _> = serde_json::from_str(&response_text);
        match response {
            Ok(response) => {
                info!("Decoded Response: {:?}", response);
                match response.status {
                    Status::Ok => {
                        if let Some(order_id) = response.data {
                            Ok((order_id, secret))
                        } else {
                            Err(eyre::eyre!("No order ID in response"))
                        }
                    }
                    Status::Error => Err(eyre::eyre!("API returned error status: {:?}", response)),
                }
            }
            Err(e) => {
                warn!("Failed to decode response body: {}", e);
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
        info!("Fetching quote from Garden Finance API...");
        let quote_request = QuoteRequest {
            order_pair: order_pair.to_string(),
            amount: amount.to_string(),
            exact_out,
        };

        let url = format!(
            "https://testnet.api.hashira.io/prices/quote?order_pair={}&amount={}&exact_out={}",
            quote_request.order_pair, quote_request.amount, quote_request.exact_out
        );

        let response = self
            .client
            .get(&url)
            .header("accept", "application/json")
            .send()
            .await?;

        let response_status = response.status();
        let response_text = response.text().await?;

        info!("Quote API Status: {}", response_status);

        if !response_status.is_success() {
            return Err(eyre::eyre!(
                "Failed to get quote: {} - {}",
                response_status,
                response_text
            ));
        }

        // Parse the response using the QuoteResponse struct
        let quote_response: QuoteResponse = serde_json::from_str(&response_text)?;

        if quote_response.status != "Ok" {
            return Err(eyre::eyre!("Quote response status is not Ok"));
        }

        // Extract the first strategy_id and its value from the quotes map
        if let Some((strategy_id, _)) = quote_response.result.quotes.iter().next() {
            return Ok((
                strategy_id.clone(),
                quote_response.result.input_token_price,
                quote_response.result.output_token_price,
            ));
        }

        Err(eyre::eyre!("No quotes found in response"))
    }

    // Fetch attested quote from the API
    pub async fn fetch_attested_quote(
        &self,
        order_params: &Order<SignableAdditionalData>,
    ) -> Result<AttestedResponse> {
        info!("Getting attested quote from Garden Finance API...");
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

        let response = self
            .client
            .post("https://testnet.api.hashira.io/prices/quote/attested")
            .header("accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        let response_status = response.status();
        let response_text = response.text().await?;

        info!("Attestation API Status: {}", response_status);
        info!("Attestation API Response: {}", response_text);

        if !response_status.is_success() {
            return Err(eyre::eyre!(
                "Failed to get attested quote: {} - {}",
                response_status,
                response_text
            ));
        }

        let attestation: AttestedResponse = serde_json::from_str(&response_text)?;
        Ok(attestation)
    }

    // Initiate an order with custom signing
    pub async fn initiate_order(&self, order_id: &str, private_key: &str) -> Result<String> {
        info!("Initiating order {}...", order_id);
        
        // Get order details
        let order_details = self.get_order_details(order_id).await?;
        
        // Get wallet for signing
        let (wallet, signer) = self.get_default_wallet(private_key.to_string())?;
        
        // Create the Initiate struct according to the sol! macro definition
        let initiate = Initiate {
            redeemer: alloy::primitives::Address::from_hex(&order_details.redeemer).unwrap(),
            timelock: alloy_primitives::Uint::from(order_details.timelock),
            amount: order_details.amount.parse().unwrap(),
            secretHash: FixedBytes::from_hex(&order_details.secret_hash).unwrap(),
        };
        
        // Create domain for EIP-712 signing
        let domain = eip712_domain! {
            name: "Garden Finance",
            version: "1",
            chain_id: 421614u64,
            verifying_contract: alloy::primitives::Address::from_hex("0x795Dcb58d1cd4789169D5F938Ea05E17ecEB68cA").unwrap(),
        };
        
        // Sign the initiate data
        let signature = signer.sign_typed_data(&initiate, &domain).await?;
        
        // Create initiate request
        let initiate_request = InitiateRequest {
            order_id: order_id.to_string(),
            signature: signature.to_string(),
            perform_on: "Source".to_string(),
        };
        
        // Send initiate request
        let response = self
            .client
            .post(format!("{}/initiate", self.api_url))
            .header("api-key", &self.api_key)
            .json(&initiate_request)
            .send()
            .await?;
            
        let response_status = response.status();
        let response_text = response.text().await?;
        
        if !response_status.is_success() {
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
                    Ok(tx_hash)
                } else {
                    Err(eyre::eyre!("No transaction hash in response"))
                }
            }
            Status::Error => Err(eyre::eyre!(
                "API error: {}",
                response.error.unwrap_or_default()
            )),
        }
    }
    
    // Get order details for initiation
    async fn get_order_details(&self, order_id: &str) -> Result<OrderDetails> {
        let url = format!("https://testnet.api.hashira.io/orders/orders/id/matched/{}", order_id);
        
        let response = self
            .client
            .get(&url)
            .header("accept", "application/json")
            .send()
            .await?;
            
        let status = response.status();
        let text = response.text().await?;
        
        if !status.is_success() {
            return Err(eyre::eyre!("Failed to get order details: {} - {}", status, text));
        }
        
        // Parse the order details from the response
        // This is a simplified version - you'll need to adjust based on the actual API response
        let order_details: OrderDetails = serde_json::from_str(&text)?;
        
        Ok(order_details)
    }
    
    // Helper function to create a wallet from a private key
    fn get_default_wallet(&self, private_key: String) -> Result<(EthereumWallet, LocalSigner<SigningKey>)> {
        let signer = PrivateKeySigner::from_bytes(
            &FixedBytes::from_hex(private_key).expect("Invalid private key"),
        )?;
        Ok((EthereumWallet::from(signer.clone()), signer))
    }
    
    // Redeem an order
    pub async fn redeem_order(&self, order_id: &str, secret: &str) -> Result<String> {
        info!("Redeeming order {}...", order_id);
        
        let redeem_request = RedeemRequest {
            order_id: order_id.to_string(),
            secret: secret.to_string(),
            perform_on: "Destination".to_string(),
        };
        
        let response = self
            .client
            .post("https://testnet.api.hashira.io/orders/gasless/redeem")
            .header("accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&redeem_request)
            .send()
            .await?;
            
        let status = response.status();
        let text = response.text().await?;
        
        if !status.is_success() {
            return Err(eyre::eyre!("Failed to redeem order: {} - {}", status, text));
        }
        
        Ok(text)
    }
}

// Structure to hold order details needed for initiation
#[derive(Debug, serde::Deserialize)]
struct OrderDetails {
    redeemer: String,
    timelock: u64,
    amount: String,
    secret_hash: String,
}
