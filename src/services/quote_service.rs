use crate::models::additional_data::SignableAdditionalData;
use crate::models::order::{AttestedResponse, Order};
use crate::models::quote::QuoteResponse;
use eyre::Result;
use reqwest::Client;
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

    // Fetch price quote from the API - matches reference.rs get_quote
    pub async fn fetch_price_quote(
        &self,
        order_pair: &str,
        amount: &str,
        exact_out: bool,
    ) -> Result<(String, f64, f64)> {
        info!("Fetching quote from Garden Finance API...");
        let url = format!(
            "https://testnet.api.hashira.io/quote?order_pair={}&amount={}&exact_out={}",
            order_pair, amount, exact_out
        );

        let response = self
            .client
            .get(&url)
            .header("accept", "application/json")
            .header("api-key", &self.api_key)
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

        info!("[ATTESTED QUOTE PAYLOAD] {}", payload);
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
