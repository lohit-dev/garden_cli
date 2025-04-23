use crate::models::additional_data::SignableAdditionalData;
use crate::models::order::{ApiResponse, AttestedResponse, Order, Status};
use eyre::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::info;

pub struct QuoteService {
    client: Client,
    api_url: String,
    api_key: String,
}

impl QuoteService {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            api_url: String::from("https://evm-relay-stage.hashira.io"),
            api_key: String::from(
                "AAAAAGnnHDw6OuFGHwJsgTQFutrJ4__itFiulz-_iRsD-c-OBaOhqZ4KWhB98QhtVYybp2UFWcv8O2LIoB7EmL77dkeQZ_4isbYc",
            ),
        }
    }

    // Fetch price quote from the API
    pub async fn fetch_price_quote(
        &self,
        order_pair: &str,
        amount: &str,
        exact_out: bool,
    ) -> Result<String> {
        let url = format!("{}/quote", self.api_url);

        let response = self
            .client
            .post(&url)
            .header("api-key", &self.api_key)
            .json(&json!({
                "order_pair": order_pair,
                "amount": amount,
                "exact_out": exact_out
            }))
            .send()
            .await?;

        let quote_response: ApiResponse<serde_json::Value> = response.json().await?;

        match quote_response.status {
            crate::models::order::Status::Ok => {
                let quote_data = serde_json::to_string(&quote_response.data.unwrap())?;
                Ok(quote_data)
            }
            crate::models::order::Status::Error => Err(eyre::eyre!(
                "Error fetching price quote: {}",
                quote_response
                    .error
                    .unwrap_or_else(|| "Unknown error".to_string())
            )),
        }
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
