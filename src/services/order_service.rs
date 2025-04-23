use crate::models::additional_data::{AdditonalData, SignableAdditionalData};
use crate::models::order::{ApiResponse, AttestedResponse, Order, Status};
use alloy::{
    hex::{FromHex, ToHexExt},
    primitives::Bytes,
};
use bigdecimal::BigDecimal;
use chrono::TimeDelta;
use eyre::Result;
use rand::Rng;
use reqwest::Client;
use serde_json;
use sha2::Digest;
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

    // Create an order using the attested quote
    pub async fn create_order(&self) -> Result<(String, Bytes)> {
        let (secret, sh) = self.gen_secret();
        let secret_hex = secret;

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
            secret_hash: sh.0.encode_hex(),
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
            .post(format!("{}/create-order", self.api_url))
            .header("api-key", self.api_key.clone()) // Add .clone() here to fix the move error
            .json(&order) // Send the complete Order structure, not just the attested response
            .send()
            .await?;

        let response_text = res.text().await?;
        info!("Raw Response Body: '{}'", response_text);

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
                            Ok((order_id, secret_hex))
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

    // Initiate an order with custom signing
    pub async fn initiate_order(&self, order_id: &str, signature: &str) -> Result<()> {
        // TODO: Implement API call to initiate order
        todo!()
    }

    // Redeem an order
    pub async fn redeem_order(&self, order_id: &str) -> Result<()> {
        // TODO: Implement API call to redeem order
        todo!()
    }

    // Helper function to generate a secret and its hash
    fn gen_secret(&self) -> (Bytes, (alloy::primitives::FixedBytes<32>,)) {
        let secret = rand::thread_rng().r#gen::<[u8; 32]>(); // Using gen directly (no need for r#gen)
        let x = sha2::Sha256::digest(secret);
        (
            Bytes::from(secret),
            (alloy::primitives::FixedBytes::new(x.into()),),
        )
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
            .post("https://testnet.api.hashira.io/quote/attested")
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
}
