// use alloy::{
//     hex::{self, FromHex},
//     network::EthereumWallet,
//     primitives::FixedBytes,
//     signers::{
//         k256::ecdsa::SigningKey,
//         local::{LocalSigner, PrivateKeySigner},
//         Signer,
//     },
//     sol,
//     sol_types::eip712_domain,
// };
// use bigdecimal::BigDecimal;
// use chrono::{TimeDelta, Utc};
// use clap::{Parser, Subcommand};
// use eyre::Result;
// use futures::{stream::FuturesUnordered, StreamExt};
// use rand::Rng;
// use serde::{Deserialize, Serialize};
// use sha2::Digest;
// use std::{
//     fs::{self, File},
//     io::{BufReader, BufWriter},
//     sync::Arc,
//     time::Duration,
// };
// use tokio::sync::Mutex as AsyncMutex;
// use tokio::{
//     sync::Semaphore,
//     time::{sleep, timeout},
// };
// use tracing::{info, warn};

// // API Constants
// const API_URL: &str = "https://evm-relay-stage.hashira.io";
// const API_KEY: &str = "AAAAAGnnHDw6OuFGHwJsgTQFutrJ4__itFiulz-_iRsD-c-OBaOhqZ4KWhB98QhtVYybp2UFWcv8O2LIoB7EmL77dkeQZ_4isbYc";

// // Test Configuration Constants
// const TOTAL_COROUTINES: usize = 2;
// const ORDERS_PER_COROUTINE: usize = 2;
// const MAX_CONCURRENT_REQUESTS: usize = 250;

// // Timing Constants
// const INITIAL_VERIFICATION_DELAY: Duration = Duration::from_secs(8);
// const VERIFICATION_BATCH_SIZE: usize = 100;

// sol! {
//     struct Initiate {
//         address redeemer;
//         uint256 timelock;
//         uint256 amount;
//         bytes32 secretHash;
//     }
// }

// #[derive(Debug, Serialize, Deserialize, Clone)]
// pub struct Order<T = AdditonalData> {
//     pub source_chain: String,
//     pub destination_chain: String,
//     pub source_asset: String,
//     pub destination_asset: String,
//     pub initiator_source_address: String,
//     pub initiator_destination_address: String,
//     pub source_amount: BigDecimal,
//     pub destination_amount: BigDecimal,
//     pub fee: BigDecimal,
//     pub nonce: BigDecimal,
//     pub min_destination_confirmations: i64,
//     pub timelock: i64,
//     pub secret_hash: String,
//     pub additional_data: T,
// }

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct SignableAdditionalData {
//     pub strategy_id: String,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub bitcoin_optional_recipient: Option<String>,
// }

// #[derive(Debug, Deserialize, Serialize, Clone)]
// pub struct InitiateRequest {
//     /// Unique identifier of the order
//     order_id: String,
//     /// EIP-712 signature for the initiate request
//     signature: String,
//     /// Which chain to perform the swap on
//     perform_on: String,
// }

// #[derive(Debug, Serialize, Deserialize)]
// pub struct RedeemRequest {
//     /// Unique identifier of the order
//     order_id: String,
//     /// Secret used to redeem the order
//     secret: String,
//     /// Which chain to perform the redemption on
//     perform_on: String,
// }

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct AdditonalData {
//     pub strategy_id: String,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub bitcoin_optional_recipient: Option<String>,
//     pub input_token_price: f64,
//     pub output_token_price: f64,
//     pub sig: Option<String>,
//     pub deadline: i64,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub instant_refund_tx_bytes: Option<String>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub redeem_tx_bytes: Option<String>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub tx_hash: Option<String>,
// }

// #[derive(Debug, Serialize, Deserialize)]
// pub struct ApiResponse<T> {
//     pub status: Status,
//     #[serde(rename = "result")]
//     pub data: Option<T>,
//     pub error: Option<String>,
// }

// #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
// pub enum Status {
//     Ok,
//     Error,
// }

// impl Order<AdditonalData> {
//     pub fn signable_order(&self) -> Order<SignableAdditionalData> {
//         let o = self.clone();
//         let signable_additional_data = SignableAdditionalData {
//             strategy_id: o.additional_data.strategy_id.clone(),
//             bitcoin_optional_recipient: o.additional_data.bitcoin_optional_recipient.clone(),
//         };
//         Order {
//             source_chain: o.source_chain,
//             destination_chain: o.destination_chain,
//             source_asset: o.source_asset,
//             destination_asset: o.destination_asset,
//             initiator_source_address: o.initiator_source_address,
//             initiator_destination_address: o.initiator_destination_address,
//             source_amount: o.source_amount,
//             destination_amount: o.destination_amount,
//             fee: o.fee,
//             nonce: o.nonce,
//             min_destination_confirmations: o.min_destination_confirmations,
//             timelock: o.timelock,
//             secret_hash: o.secret_hash,
//             additional_data: signable_additional_data,
//         }
//     }
// }

// pub fn get_default_wallet(private_key: String) -> (EthereumWallet, LocalSigner<SigningKey>) {
//     let signer = PrivateKeySigner::from_bytes(
//         &FixedBytes::from_hex(private_key).expect("Invalid private key"),
//     )
//     .unwrap();
//     (EthereumWallet::from(signer.clone()), signer)
// }

// pub fn gen_secret() -> (String, String) {
//     use rand::{rngs::OsRng, RngCore};

//     let mut secret = [0u8; 32];
//     OsRng.fill_bytes(&mut secret);

//     let hash = sha2::Sha256::digest(&secret);
//     let hash_str = hex::encode(hash);
//     (hex::encode(secret), hash_str)
// }

// #[derive(Debug, Serialize, Deserialize)]
// struct AttestedResponse {
//     status: String,
//     result: AtttestedData,
// }

// #[derive(Debug, Serialize, Deserialize)]
// struct AtttestedData {
//     source_chain: String,
//     destination_chain: String,
//     source_asset: String,
//     destination_asset: String,
//     initiator_source_address: String,
//     initiator_destination_address: String,
//     source_amount: String,
//     destination_amount: String,
//     fee: String,
//     nonce: String,
//     min_destination_confirmations: i32,
//     timelock: i64,
//     secret_hash: String,
//     additional_data: NewAdditionalData,
// }

// #[derive(Debug, Serialize, Deserialize, Clone)]
// pub struct QuoteRequest {
//     order_pair: String,
//     amount: String,
//     exact_out: bool,
// }

// #[derive(Debug, Serialize, Deserialize, Clone)]
// pub struct QuoteResponse {
//     pub status: String,
//     pub result: QuoteResult,
// }

// #[derive(Debug, Serialize, Deserialize, Clone)]
// pub struct QuoteResult {
//     pub quotes: std::collections::HashMap<String, String>,
//     pub input_token_price: f64,
//     pub output_token_price: f64,
// }

// async fn get_quote(
//     client: &reqwest::Client,
//     order_params: QuoteRequest,
// ) -> Result<(String, f64, f64)> {
//     info!("Fetching quote from Garden Finance API...");
//     let url = format!(
//         "https://testnet.api.hashira.io/quote?order_pair={}&amount={}&exact_out={}",
//         order_params.order_pair, order_params.amount, order_params.exact_out
//     );

//     let response = client
//         .get(&url)
//         .header("accept", "application/json")
//         .send()
//         .await?;

//     let response_status = response.status();
//     let response_text = response.text().await?;

//     info!("Quote API Status: {}", response_status);
//     // info!("Quote API Response: {}", response_text);

//     if !response_status.is_success() {
//         return Err(eyre::eyre!(
//             "Failed to get quote: {} - {}",
//             response_status,
//             response_text
//         ));
//     }

//     // Parse the response using the QuoteResponse struct
//     let quote_response: QuoteResponse = serde_json::from_str(&response_text)?;

//     if quote_response.status != "Ok" {
//         return Err(eyre::eyre!("Quote response status is not Ok"));
//     }

//     // Extract the first strategy_id and its value from the quotes map
//     if let Some((strategy_id, _)) = quote_response.result.quotes.iter().next() {
//         return Ok((
//             strategy_id.clone(),
//             quote_response.result.input_token_price,
//             quote_response.result.output_token_price,
//         ));
//     }

//     Err(eyre::eyre!("No quotes found in response"))
// }

// async fn get_attested_quote(
//     client: &reqwest::Client,
//     order_params: &Order<SignableAdditionalData>,
// ) -> Result<AttestedResponse> {
//     info!("Getting attested quote from Garden Finance API...");
//     let payload = serde_json::json!({
//         "source_chain": order_params.source_chain,
//         "destination_chain": order_params.destination_chain,
//         "source_asset": order_params.source_asset,
//         "destination_asset": order_params.destination_asset,
//         "initiator_source_address": order_params.initiator_source_address,
//         "initiator_destination_address": order_params.initiator_destination_address,
//         "source_amount": order_params.source_amount.to_string(),
//         "destination_amount": order_params.destination_amount.to_string(),
//         "fee": order_params.fee.to_string(),
//         "nonce": order_params.nonce.to_string(),
//         "min_destination_confirmations": order_params.min_destination_confirmations,
//         "timelock": order_params.timelock,
//         "secret_hash": order_params.secret_hash,
//         "additional_data": {
//             "strategy_id": order_params.additional_data.strategy_id,
//             "bitcoin_optional_recipient": order_params.additional_data.bitcoin_optional_recipient,
//         }
//     });

//     let response = client
//         .post("https://testnet.api.hashira.io/quote/attested")
//         .header("accept", "application/json")
//         .header("Content-Type", "application/json")
//         .json(&payload)
//         .send()
//         .await?;

//     let response_status = response.status();
//     let response_text = response.text().await?;

//     info!("Attestation API Status: {}", response_status);
//     // info!("Attestation API Response: {}", response_text);

//     if !response_status.is_success() {
//         return Err(eyre::eyre!(
//             "Failed to get attested quote: {} - {}",
//             response_status,
//             response_text
//         ));
//     }

//     let attestation: AttestedResponse = serde_json::from_str(&response_text)?;
//     Ok(attestation)
// }

// async fn create_order(client: &reqwest::Client) -> Result<(String, String)> {
//     let (secret, sh) = gen_secret();
//     info!("Generated Secret: {secret}");

//     // 1. First fetch a quote with the specified parameters
//     let quote_request = QuoteRequest {
//         order_pair: "arbitrum_sepolia:0x795Dcb58d1cd4789169D5F938Ea05E17ecEB68cA::starknet_sepolia:0x75cf614ce4ebce29ac622a50cd5151ddfff853159707589a85dd67b9fb1eba".to_string(),
//         amount: "10000".to_string(),
//         exact_out: false,
//     };

//     let (strategy_id, input_token_price, output_token_price) =
//         get_quote(client, quote_request).await?;
//     // info!("Selected strategy_id: {}", strategy_id);
//     // info!("Input token price: {}", input_token_price);
//     // info!("Output token price: {}", output_token_price);

//     let mut order = Order {
//         source_chain: "arbitrum_sepolia".to_string(),
//         destination_chain: "starknet_sepolia".to_string(),
//         source_asset: "0x795Dcb58d1cd4789169D5F938Ea05E17ecEB68cA".to_string(),
//         destination_asset: "0x75cf614ce4ebce29ac622a50cd5151ddfff853159707589a85dd67b9fb1eba"
//             .to_string(),
//         initiator_source_address: "0x3E53d785995bb74C0B9ba8F71D0d6a0c4d9E6901".to_string(),
//         initiator_destination_address:
//             "0x56b3ebec13503cb1e1d9691f13fdc9b4ae7015765113345a7355add1e29d7dc".to_string(),
//         source_amount: BigDecimal::from(10000),
//         destination_amount: BigDecimal::from(5194207362831030i64),
//         fee: BigDecimal::from(1),
//         nonce: BigDecimal::from(Utc::now().timestamp()),
//         min_destination_confirmations: 0,
//         timelock: 7300,
//         secret_hash: sh,
//         additional_data: AdditonalData {
//             deadline: chrono::Utc::now()
//                 .checked_add_signed(TimeDelta::minutes(10))
//                 .unwrap()
//                 .timestamp(),
//             input_token_price: input_token_price,
//             output_token_price: output_token_price,
//             strategy_id: strategy_id,
//             sig: None,
//             bitcoin_optional_recipient: None,
//             instant_refund_tx_bytes: None,
//             redeem_tx_bytes: None,
//             tx_hash: None,
//         },
//     };

//     // let mut order = Order {
//     //     source_chain: "arbitrum_sepolia".to_string(),
//     //     destination_chain: "ethereum_sepolia".to_string(),
//     //     source_asset: "0x795Dcb58d1cd4789169D5F938Ea05E17ecEB68cA".to_string(),
//     //     destination_asset: "0xfAd20D08e79c41deE4501648E2EF182965CCFBb4".to_string(),
//     //     initiator_source_address: "0x3E53d785995bb74C0B9ba8F71D0d6a0c4d9E6901".to_string(),
//     //     initiator_destination_address: "0x3E53d785995bb74C0B9ba8F71D0d6a0c4d9E6901".to_string(),
//     //     source_amount: BigDecimal::from(50000),
//     //     destination_amount: BigDecimal::from(49000),
//     //     fee: BigDecimal::from(1),
//     //     nonce: BigDecimal::from(1),
//     //     min_destination_confirmations: 4,
//     //     timelock: 7300,
//     //     secret_hash: sh,
//     //     additional_data: AdditonalData {
//     //         deadline: chrono::Utc::now()
//     //             .checked_add_signed(TimeDelta::minutes(10))
//     //             .unwrap()
//     //             .timestamp(),
//     //         input_token_price: 1.0,
//     //         output_token_price: 1.0,
//     //         strategy_id: "asaces4b".to_string(),
//     //         sig: None,
//     //         bitcoin_optional_recipient: None,
//     //         instant_refund_tx_bytes: None,
//     //         redeem_tx_bytes: None,
//     //         tx_hash: None,
//     //     },
//     // };

//     let signable_order = order.signable_order();

//     let attested = get_attested_quote(client, &signable_order).await?;

//     // populating unique values
//     order.additional_data.sig = Some(attested.result.additional_data.sig);
//     order.additional_data.input_token_price = attested.result.additional_data.input_token_price;
//     order.additional_data.output_token_price = attested.result.additional_data.output_token_price;

//     // Add a small delay before sending the create order request
//     tokio::time::sleep(std::time::Duration::from_secs(1)).await;

//     let res = client
//         .post(format!("{}/create-order", API_URL))
//         .header("api-key", API_KEY)
//         .json(&order) // Send the complete Order structure, not just the attested response
//         .send()
//         .await?;

//     let response_text = res.text().await?;
//     // info!("Raw Response Body: '{}'", response_text);

//     if response_text.trim().is_empty() {
//         warn!("Response body is empty, cannot decode as JSON");
//         return Err(eyre::eyre!("Empty response body"));
//     }

//     let response: Result<ApiResponse<String>, _> = serde_json::from_str(&response_text);
//     match response {
//         Ok(response) => {
//             // info!("Decoded Response: {:?}", response);
//             match response.status {
//                 Status::Ok => {
//                     if let Some(order_id) = response.data {
//                         Ok((order_id, secret))
//                     } else {
//                         Err(eyre::eyre!("No order ID in response"))
//                     }
//                 }
//                 Status::Error => Err(eyre::eyre!("API returned error status: {:?}", response)),
//             }
//         }
//         Err(e) => {
//             warn!("Failed to decode response body: {}", e);
//             Err(eyre::eyre!(
//                 "Failed to decode response body: {}. Raw body: {}",
//                 e,
//                 response_text
//             ))
//         }
//     }
// }

// async fn initiate_order(client: &reqwest::Client, order_id: &str) -> Result<String, eyre::Error> {
//     // 1. Fetch the order details
//     let order = get_order(order_id).await?;
//     info!(
//         "Fetched order for initiation: {}",
//         order.create_order.create_id
//     );

//     // Create the EIP-712 domain
//     let domain = eip712_domain! {
//     name: "HTLC".to_string(),
//     version: "1".to_string(),
//     chain_id: 421614u64,
//         verifying_contract: alloy::primitives::Address::from_hex("0x795Dcb58d1cd4789169D5F938Ea05E17ecEB68cA").unwrap(),    };

//     let redeemer = alloy::primitives::Address::from_hex(&order.source_swap.redeemer).unwrap();
//     let amount_str = order.source_swap.amount.split('.').next().unwrap_or("0");
//     let amount = alloy::primitives::U256::from_str_radix(amount_str, 10).unwrap();
//     let timelock_value = alloy::primitives::U256::from(order.source_swap.timelock as u64);
//     let secret_hash =
//         alloy::primitives::FixedBytes::<32>::from_hex(&order.source_swap.secret_hash).unwrap();

//     let initiate = Initiate {
//         redeemer: redeemer,
//         timelock: timelock_value,
//         amount: amount,
//         secretHash: secret_hash,
//     };

//     let (_w, wallet_signer) = get_default_wallet(
//         "7c7f9b2aac806a014c9a26d31d1c21a123aa6e8c130374369b4b5365e7bc347b".to_string(),
//     );
//     let wallet_signer = Arc::new(wallet_signer);
//     let signature = wallet_signer.sign_typed_data(&initiate, &domain).await?;
//     let signature_hex = signature;
//     // info!("Generated signature: {}", signature_hex);

//     // 4. Send to initiate endpoint
//     let res = client
//         .post(format!("{}/initiate", API_URL))
//         .header("api-key", API_KEY)
//         .json(&InitiateRequest {
//             order_id: order_id.to_string(),
//             signature: signature_hex.to_string(),
//             perform_on: "Source".to_string(),
//         })
//         .send()
//         .await?;

//     let response_text = res.text().await?;
//     // info!("Raw Response: {}", response_text);

//     if response_text.trim().is_empty() {
//         warn!("Response body is empty, cannot decode as JSON");
//         return Err(eyre::eyre!("Empty response body"));
//     }

//     let response: ApiResponse<String> = serde_json::from_str(&response_text)?;
//     match response.status {
//         Status::Ok => {
//             if let Some(tx_hash) = response.data {
//                 info!("Successfully initiated order with tx hash: {}", tx_hash);
//                 Ok(tx_hash)
//             } else {
//                 Err(eyre::eyre!("No transaction hash in response"))
//             }
//         }
//         Status::Error => Err(eyre::eyre!(
//             "API error: {}",
//             response.error.unwrap_or_default()
//         )),
//     }
// }

// async fn verify_order(client: &reqwest::Client, order_id: &str) -> Result<bool> {
//     let response = client
//         .get(format!(
//             "https://testnet.api.hashira.io/orders/id/matched/{}",
//             order_id
//         ))
//         .header("api-key", API_KEY)
//         .send()
//         .await?;

//     // Any successful response is valid
//     if response.status().is_success() {
//         match response.json::<ApiResponse<serde_json::Value>>().await {
//             Ok(api_response) => {
//                 Ok(api_response.status == Status::Ok && api_response.data.is_some())
//             }
//             Err(_e) => {
//                 info!("Some error occurred: {:?}", _e);
//                 Ok(false)
//             }
//         }
//     } else {
//         info!("Some error occurred: {:?}", response);
//         Ok(false)
//     }
// }

// async fn verify_order_with_retry(client: &reqwest::Client, order_id: &str) -> Result<bool> {
//     let mut attempts = 0;
//     const MAX_VERIFY_RETRIES: u32 = 3;
//     const VERIFY_TIMEOUT: Duration = Duration::from_millis(700);
//     const BASE_DELAY: Duration = Duration::from_millis(200);

//     while attempts < MAX_VERIFY_RETRIES {
//         match timeout(VERIFY_TIMEOUT, verify_order(client, order_id)).await {
//             Ok(Ok(true)) => return Ok(true),
//             Ok(Ok(false)) | Ok(Err(_)) | Err(_) => {
//                 attempts += 1;
//                 if attempts < MAX_VERIFY_RETRIES {
//                     let jitter = rand::thread_rng().gen_range(0..=200);
//                     sleep(BASE_DELAY + Duration::from_millis(jitter)).await;
//                 }
//             }
//         }
//     }

//     Ok(false)
// }

// // async fn process_creation_batch(
// //     client: Arc<reqwest::Client>,
// //     wallet_signer: Arc<LocalSigner<SigningKey>>,
// //     batch_size: usize,
// //     semaphore: Arc<Semaphore>,
// // ) -> Vec<Result<String>> {
// //     let mut futures = FuturesUnordered::new();
// //     let mut results = Vec::with_capacity(batch_size);

// //     // Create orders in parallel with no retries
// //     for _ in 0..batch_size {
// //         let client = client.clone();
// //         let wallet_signer = wallet_signer.clone();
// //         let semaphore = semaphore.clone();

// //         futures.push(async move {
// //             let _permit = semaphore.acquire().await.unwrap();
// //             match create_order(&client, &wallet_signer).await {
// //                 Ok(order_id) => {
// //                     info!("✅ Order created in DB: {}", order_id);
// //                     Ok(order_id)
// //                 }
// //                 Err(e) => {
// //                     warn!("❌ Order creation failed: {:?}", e);
// //                     Err(e)
// //                 }
// //             }
// //         });
// //     }

// //     while let Some(result) = futures.next().await {
// //         results.push(result);
// //     }

// //     results
// // }

// async fn process_verification_batch(
//     client: Arc<reqwest::Client>,
//     order_ids: Vec<String>,
//     semaphore: Arc<Semaphore>,
// ) -> Vec<Result<String>> {
//     let mut futures = FuturesUnordered::new();

//     // 100 workers each worker in 100 does 10  -> 100 * 10 = 1000       (now changed btw this is the approach)
//     for chunk in order_ids.chunks(10) {
//         let chunk = chunk.to_vec();
//         let client = client.clone();
//         let semaphore = semaphore.clone();

//         futures.push(async move {
//             let _permit = semaphore.acquire().await.unwrap();
//             let mut results = Vec::new();
//             for order_id in chunk {
//                 match verify_order_with_retry(&client, &order_id).await {
//                     Ok(true) => {
//                         info!("Order verified: {}", order_id);
//                         results.push(Ok(order_id));
//                     }
//                     Ok(false) | Err(_) => {
//                         warn!("Order verification failed for {}", order_id);
//                     }
//                 }
//                 // very very small delay
//                 sleep(Duration::from_millis(10)).await;
//             }
//             results
//         });
//     }

//     let mut results = Vec::new();
//     while let Some(batch_results) = futures.next().await {
//         results.extend(batch_results);
//     }
//     results
// }

// #[derive(Parser)]
// #[command(name = "order-processor")]
// #[command(about = "Creates, verifies and initiates orders")]
// struct Cli {
//     #[command(subcommand)]
//     command: Commands,
// }

// #[derive(Subcommand)]
// enum Commands {
//     Create,
//     Verify,
//     Initiate,
//     Get,
//     Redeem,
//     Status,
//     GardenFlow,
// }
// async fn is_order_ready_for_redemption(order_id: &str) -> Result<bool> {
//     // Get the order details
//     let order = get_order(order_id).await?;

//     // Just check if the destination swap has been initiated
//     let has_initiate_tx = !order.destination_swap.initiate_tx_hash.trim().is_empty()
//         && order.destination_swap.initiate_tx_hash != "0x";

//     Ok(has_initiate_tx)
// }

// async fn redeem_order(
//     client: &reqwest::Client,
//     order_id: &str,
//     mut secret: String,
// ) -> Result<String> {
//     info!("Attempting to redeem order: {}", order_id);

//     if !secret.starts_with("0x") {
//         secret = format!("0x{}", secret);
//     }

//     info!("Using secret: {secret}");
//     let payload = &RedeemRequest {
//         order_id: order_id.to_string(),
//         perform_on: "Destination".to_string(),
//         secret: secret,
//     };

//     // info!("The Payload: {:?}", payload);

//     let res = client
//         .post(format!("https://starknet-relayer.hashira.io/redeem"))
//         .header("api-key", API_KEY)
//         .json(payload)
//         .send()
//         .await?;

//     let response_text = res.text().await?;
//     // info!("Raw Response: {}", response_text);

//     if response_text.trim().is_empty() {
//         warn!("Response body is empty, cannot decode as JSON");
//         return Err(eyre::eyre!("Empty response body"));
//     }

//     let response: ApiResponse<String> = serde_json::from_str(&response_text)?;
//     match response.status {
//         Status::Ok => {
//             if let Some(tx_hash) = response.data {
//                 info!("Successfully redeemed order with tx hash: {}", tx_hash);
//                 Ok(tx_hash)
//             } else {
//                 Err(eyre::eyre!("No transaction hash in response"))
//             }
//         }
//         Status::Error => Err(eyre::eyre!(
//             "API error: {}",
//             response.error.unwrap_or_default()
//         )),
//     }
// }

// async fn redeem_orders() -> Result<()> {
//     info!("Starting order redemption process...");
//     let start = std::time::Instant::now();

//     // Read the initiated orders from file
//     let file = File::open("initiated_orders.json")?;
//     let reader = BufReader::new(file);
//     let order_ids: Vec<String> = serde_json::from_reader(reader)?;

//     let file = File::open("order_secrets.json")?;
//     let reader = BufReader::new(file);
//     let order_data: Vec<(String, String)> = serde_json::from_reader(reader)?;

//     if order_data.is_empty() {
//         return Err(eyre::eyre!(
//             "No initiated orders found in order_secrets.json"
//         ));
//     }

//     if order_ids.is_empty() {
//         return Err(eyre::eyre!(
//             "No initiated orders found in initiated_orders.json"
//         ));
//     }

//     info!("Found {} initiated orders to process", order_ids.len());

//     let client = Arc::new(reqwest::Client::builder().build()?);
//     let semaphore = Arc::new(Semaphore::new(TOTAL_COROUTINES));

//     let mut futures = FuturesUnordered::new();
//     let mut redeemed_orders = Vec::new();
//     let mut failed_orders = Vec::new();

//     // Process each order
//     for (order_id, encoded_secret) in order_data {
//         let client = client.clone();
//         let semaphore = semaphore.clone();

//         futures.push(async move {
//             let _permit = semaphore.acquire().await.unwrap();

//             match is_order_ready_for_redemption(&order_id).await {
//                 Ok(true) => match redeem_order(&client, &order_id, encoded_secret).await {
//                     Ok(tx_hash) => Ok((order_id, tx_hash)),
//                     Err(e) => Err((order_id, e.to_string())),
//                 },
//                 Ok(false) => Err((order_id, "Order not ready for redemption".to_string())),
//                 Err(e) => Err((order_id, e.to_string())),
//             }
//         });
//     }

//     // Collect results
//     while let Some(result) = futures.next().await {
//         match result {
//             Ok((order_id, tx_hash)) => {
//                 redeemed_orders.push((order_id, tx_hash));
//             }
//             Err((order_id, error)) => {
//                 failed_orders.push((order_id, error));
//             }
//         }
//     }

//     // Save results to file
//     let file = File::create("redeemed_orders.json")?;
//     let writer = BufWriter::new(file);
//     serde_json::to_writer(writer, &redeemed_orders)?;

//     let file = File::create("failed_redemptions.json")?;
//     let writer = BufWriter::new(file);
//     serde_json::to_writer(writer, &failed_orders)?;

//     let elapsed = start.elapsed();
//     info!("Redemption process completed in {:?}", elapsed);
//     info!("Successfully redeemed: {} orders", redeemed_orders.len());
//     info!("Failed to redeem: {} orders", failed_orders.len());

//     Ok(())
// }

// async fn verify_orders() -> Result<()> {
//     info!("Starting order verification...");
//     let start = std::time::Instant::now();

//     let client = Arc::new(reqwest::Client::builder().build()?);
//     let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS));

//     // getting id from the file
//     let file = File::open("order_ids.json")?;
//     let reader = BufReader::new(file);
//     let order_ids: Vec<String> = serde_json::from_reader(reader)?;

//     info!("Loaded {} order IDs for verification", order_ids.len());

//     // Delay just in case creating didn't go fast
//     info!(
//         "Waiting {} seconds for initial database processing...",
//         INITIAL_VERIFICATION_DELAY.as_secs()
//     );
//     info!("run verify 2-3 times if db write speeds are slow");
//     sleep(INITIAL_VERIFICATION_DELAY).await;

//     let mut verified_count = 0;
//     let mut remaining_orders = order_ids;
//     let mut attempt = 1;
//     let max_attempts = 3;

//     while !remaining_orders.is_empty() && attempt <= max_attempts {
//         info!(
//             "Verification attempt {}/{} for {} orders",
//             attempt,
//             max_attempts,
//             remaining_orders.len()
//         );

//         // parallel batcher
//         let mut futures = FuturesUnordered::new();

//         for chunk in remaining_orders.chunks(VERIFICATION_BATCH_SIZE) {
//             let client = client.clone();
//             let semaphore = semaphore.clone();
//             let chunk = chunk.to_vec();

//             futures.push(async move { process_verification_batch(client, chunk, semaphore).await });
//         }

//         let mut new_remaining = Vec::new();
//         while let Some(results) = futures.next().await {
//             for result in results {
//                 match result {
//                     Ok(_) => verified_count += 1,
//                     Err(_) => new_remaining.push(result.unwrap_err().to_string()),
//                 }
//             }
//         }

//         remaining_orders = new_remaining;

//         if !remaining_orders.is_empty() && attempt < max_attempts {
//             info!(
//                 "{} orders pending, waiting before next attempt...",
//                 remaining_orders.len()
//             );
//             sleep(Duration::from_secs(2)).await;
//         }

//         attempt += 1;
//     }

//     let failed_count = remaining_orders.len();

//     // Failed things different place
//     if !remaining_orders.is_empty() {
//         let file = File::create("failed_verifications.json")?;
//         let writer = BufWriter::new(file);
//         serde_json::to_writer(writer, &remaining_orders)?;
//     }

//     let elapsed = start.elapsed();
//     info!("Verification completed in {:?}", elapsed);
//     info!("Total orders verified: {}", verified_count);
//     info!("Total orders failed: {}", failed_count);
//     // percentage calculation just for fun
//     info!(
//         "Success rate: {:.2}%",
//         (verified_count as f64 / (verified_count + failed_count) as f64) * 100.0
//     );

//     Ok(())
// }

// #[tokio::main]
// async fn main() -> Result<()> {
//     tracing_subscriber::fmt::init();
//     let cli = Cli::parse();

//     match cli.command {
//         Commands::Create => {
//             info!("Starting order creation process...");
//             create_orders().await?;
//         }
//         Commands::Verify => {
//             info!("Starting order verification process...");
//             verify_orders().await?;
//         }
//         Commands::Initiate => {
//             info!("Starting order initiation process...");
//             initiate_orders().await?;
//         }
//         Commands::Get => {
//             info!("Starting fetching dummy order");
//             get_order("5469f5ccac4f33acdb4197aabd71a70bcc77d7c68910ae9b04df1d675bc90066").await?;
//         }
//         Commands::Redeem => {
//             info!("Starting Redeem...");
//             redeem_orders().await?;
//         }
//         Commands::Status => {
//             info!("Checking order status...");
//             let order_id = "5469f5ccac4f33acdb4197aabd71a70bcc77d7c68910ae9b04df1d675bc90066";
//             let order = get_order(order_id).await?;
//             info!("Order: {:#?}", order);
//         }
//         Commands::GardenFlow => {
//             info!("Starting Garden Finance flow...");
//             execute_garden_flow().await?;
//         }
//     }
//     Ok(())
// }

// // Function to execute the complete Garden Finance flow
// async fn execute_garden_flow() -> Result<()> {
//     info!("Starting Garden Finance flow demonstration");

//     // Create a reusable HTTP client
//     let client = Arc::new(reqwest::Client::builder().build()?);

//     // Step 1: Fetch a quote
//     info!("Step 1: Fetching a quote");
//     let quote_request = QuoteRequest {
//         order_pair: "arbitrum_sepolia:0x795Dcb58d1cd4789169D5F938Ea05E17ecEB68cA::starknet_sepolia:0x75cf614ce4ebce29ac622a50cd5151ddfff853159707589a85dd67b9fb1eba".to_string(),
//         amount: "10000".to_string(),
//         exact_out: false,
//     };

//     let (strategy_id, input_price, output_price) = get_quote(&client, quote_request).await?;
//     info!("Successfully obtained strategy_id: {}", strategy_id);
//     info!(
//         "Token prices - Input: {}, Output: {}",
//         input_price, output_price
//     );

//     // Step 2: Create an order
//     info!("Step 2: Creating an order");
//     let (order_id, secret) = create_order(&client).await?;
//     info!("Successfully created order with ID: {}", order_id);

//     // Step 3: Get order details
//     info!("Step 3: Retrieving order details");
//     let order = get_order(&order_id).await?;
//     info!("Order details: {:#?}", order);

//     // Step 4: Initiate the order (optional)
//     info!("Step 4: Initiating the order");
//     let initiate_result = initiate_order(&client, &order_id).await;
//     match initiate_result {
//         Ok(tx_hash) => {
//             info!(
//                 "Successfully initiated order with transaction hash: {}",
//                 tx_hash
//             );

//             // Step 5: Wait and retry checking if the order is ready for redemption
//             info!("Step 5: Waiting for order to be ready for redemption");

//             // Try up to 6 times with a delay between attempts
//             let max_attempts = 20;
//             let mut is_ready = false;
//             let mut redemption_error = None;

//             for attempt in 1..=max_attempts {
//                 info!("Redemption check attempt {}/{}", attempt, max_attempts);

//                 // Wait between attempts
//                 if attempt > 1 {
//                     let delay = std::time::Duration::from_secs(5);
//                     info!("Waiting {} seconds before next attempt...", delay.as_secs());
//                     tokio::time::sleep(delay).await;
//                 }

//                 // Check if order is ready for redemption
//                 match is_order_ready_for_redemption(&order_id).await {
//                     Ok(true) => {
//                         is_ready = true;
//                         info!("Order is ready for redemption!");
//                         break;
//                     }
//                     Ok(false) => {
//                         info!(
//                             "Order not yet ready for redemption (attempt {}/{})",
//                             attempt, max_attempts
//                         );
//                     }
//                     Err(e) => {
//                         redemption_error = Some(e);
//                         warn!(
//                             "Error checking if order is ready for redemption: {}",
//                             redemption_error.as_ref().unwrap()
//                         );
//                     }
//                 }
//             }

//             // Step 6: Redeem the order if it's ready
//             if is_ready {
//                 info!("Step 6: Redeeming the order");
//                 match redeem_order(&client, &order_id, secret).await {
//                     Ok(redeem_tx_hash) => {
//                         info!(
//                             "Successfully redeemed order with transaction hash: {}",
//                             redeem_tx_hash
//                         );
//                     }
//                     Err(e) => warn!("Failed to redeem order: {}", e),
//                 }
//             } else if let Some(e) = redemption_error {
//                 warn!(
//                     "Failed to verify if order is ready for redemption after multiple attempts: {}",
//                     e
//                 );
//             } else {
//                 warn!(
//                     "Order not ready for redemption after {} attempts",
//                     max_attempts
//                 );
//             }
//         }
//         Err(e) => warn!("Failed to initiate order: {}", e),
//     }

//     info!("Garden Finance flow demonstration completed successfully");
//     Ok(())
// }

// #[derive(Debug, Serialize, Deserialize, Clone)]
// pub struct ResultData {
//     pub created_at: String,
//     pub updated_at: String,
//     pub deleted_at: Option<String>,
//     pub source_swap: SwapData,
//     pub destination_swap: SwapData,
//     pub create_order: CreateOrderData,
// }

// #[derive(Debug, Serialize, Deserialize, Clone)]
// pub struct SwapData {
//     pub created_at: String,
//     pub updated_at: String,
//     pub deleted_at: Option<String>,
//     pub swap_id: String,
//     pub chain: String,
//     pub asset: String,
//     pub initiator: String,
//     pub redeemer: String,
//     pub timelock: i64,
//     pub filled_amount: String,
//     pub amount: String,
//     pub secret_hash: String,
//     pub secret: String,
//     pub initiate_tx_hash: String,
//     pub redeem_tx_hash: String,
//     pub refund_tx_hash: String,
//     pub initiate_block_number: String,
//     pub redeem_block_number: String,
//     pub refund_block_number: String,
//     pub required_confirmations: i64,
//     pub current_confirmations: i64,
// }

// #[derive(Debug, Serialize, Deserialize, Clone)]
// pub struct CreateOrderData {
//     pub created_at: String,
//     pub updated_at: String,
//     pub deleted_at: Option<String>,
//     pub create_id: String,
//     pub block_number: String,
//     pub source_chain: String,
//     pub destination_chain: String,
//     pub source_asset: String,
//     pub destination_asset: String,
//     pub initiator_source_address: String,
//     pub initiator_destination_address: String,
//     pub source_amount: String,
//     pub destination_amount: String,
//     pub fee: String,
//     pub nonce: String,
//     pub min_destination_confirmations: i64,
//     pub timelock: i64,
//     pub secret_hash: String,
//     pub user_id: String,
//     pub additional_data: NewAdditionalData,
// }

// #[derive(Debug, Serialize, Deserialize, Clone)]
// pub struct NewAdditionalData {
//     pub strategy_id: String,
//     pub input_token_price: f64,
//     pub output_token_price: f64,
//     pub sig: String,
//     pub deadline: i64,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub tx_hash: Option<String>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub is_blacklisted: Option<bool>,
// }

// async fn get_order(order_id: &str) -> Result<ResultData> {
//     let client = Arc::new(reqwest::Client::builder().build()?);
//     let url = format!(
//         "https://testnet.api.hashira.io/orders/id/matched/{}",
//         order_id
//     );

//     info!("Fetching order details for ID: {}", order_id);
//     let res = client
//         .get(&url)
//         .header("accept", "application/json")
//         .send()
//         .await?;

//     let response_status = res.status();

//     if !response_status.is_success() {
//         let error_text = res.text().await?;
//         return Err(eyre::eyre!(
//             "Failed to get order: {} - {}",
//             response_status,
//             error_text
//         ));
//     }

//     let order: ApiResponse<ResultData> = res.json().await?;
//     // info!("Order: {:#?}", serde_json::json!(order));
//     order.data.ok_or_else(|| eyre::eyre!("Order not found"))
// }

// async fn initiate_orders() -> Result<()> {
//     info!(
//         "Starting order creation with all 100 coroutines at once, 1-second gap between orders..."
//     );
//     let start = std::time::Instant::now();

//     let client = Arc::new(reqwest::Client::builder().build()?);
//     let semaphore = Arc::new(Semaphore::new(TOTAL_COROUTINES));

//     // getting id from the file
//     let file = File::open("order_ids.json")?;
//     let reader = BufReader::new(file);
//     let order_ids: Vec<String> = serde_json::from_reader(reader)?;
//     if order_ids.is_empty() {
//         return Err(eyre::eyre!(
//             "No order IDs found in order_ids.json. Please create orders first."
//         ));
//     }
//     let mut initiate_ids: Vec<String> = Vec::new();
//     let mut total_created = 0;
//     let mut total_failed = 0;
//     let mut error_counts: std::collections::HashMap<String, usize> =
//         std::collections::HashMap::new();

//     let mut futures = FuturesUnordered::new();

//     // Clone order_ids once before the loop to avoid the 'use of moved value' error
//     let order_ids = Arc::new(order_ids);

//     for coroutine_id in 0..TOTAL_COROUTINES {
//         let client = client.clone();
//         let semaphore: Arc<Semaphore> = semaphore.clone();
//         let orders = order_ids.clone();

//         futures.push(async move {
//             let _permit = semaphore.acquire().await.unwrap();
//             let coroutine_start = std::time::Instant::now();

//             info!(
//                 "Coroutine {} starting to process {} orders",
//                 coroutine_id + 1,
//                 ORDERS_PER_COROUTINE
//             );

//             let mut results = Vec::new();

//             // Calculate the starting index for this coroutine to avoid overlap
//             let start_idx = coroutine_id as usize * ORDERS_PER_COROUTINE;
//             for i in 0..ORDERS_PER_COROUTINE {
//                 // Skip if we're out of bounds
//                 if start_idx + i >= orders.len() {
//                     info!(
//                         "Coroutine {}: Skipping order {} (out of bounds)",
//                         coroutine_id + 1,
//                         i + 1
//                     );
//                     continue;
//                 }

//                 match initiate_order(&client, &orders[start_idx + i]).await {
//                     Ok(order_id) => {
//                         results.push(Ok(order_id.clone()));
//                         info!(
//                             "Coroutine {}: Initiated order {} of {} (order_id: {})",
//                             coroutine_id + 1,
//                             i + 1,
//                             ORDERS_PER_COROUTINE,
//                             order_id
//                         );
//                     }
//                     Err(e) => {
//                         results.push(Err(e));
//                         info!(
//                             "Coroutine {}: Failed to initiate order {} of {} (order_id: {})",
//                             coroutine_id + 1,
//                             i + 1,
//                             ORDERS_PER_COROUTINE,
//                             orders[start_idx + i],
//                         );
//                     }
//                 }

//                 // Add a 1-second gap after each coroutine
//                 // if i < ORDERS_PER_COROUTINE - 1 {
//                 //     tokio::time::sleep(std::time::Duration::from_secs(2)).await;
//                 // }
//             }

//             info!(
//                 "Coroutine {} completed {} orders in {:?}",
//                 coroutine_id + 1,
//                 ORDERS_PER_COROUTINE,
//                 coroutine_start.elapsed()
//             );

//             results
//         });
//     }

//     // Collect results from all coroutines
//     while let Some(results) = futures.next().await {
//         for result in results {
//             match result {
//                 Ok(order_id) => {
//                     total_created += 1;
//                     initiate_ids.push(order_id);

//                     // Log progress periodically
//                     if total_created % 50 == 0 {
//                         info!("Progress: {} orders created so far", total_created);
//                     }
//                 }
//                 Err(e) => {
//                     total_failed += 1;
//                     let error_type = e.to_string();
//                     *error_counts.entry(error_type).or_insert(0) += 1;
//                 }
//             }
//         }
//     }

//     info!(
//         "Order creation completed in {:?}. Created: {}, Failed: {}, Total attempted: {}",
//         start.elapsed(),
//         total_created,
//         total_failed,
//         total_created + total_failed
//     );

//     if !error_counts.is_empty() {
//         info!("Creation error distribution:");
//         for (error_type, count) in error_counts {
//             info!("  {}: {} occurrences", error_type, count);
//         }
//     }

//     // Saving the initiated order IDs to a separate file
//     let file = fs::OpenOptions::new()
//         .write(true)
//         .create(true)
//         .truncate(true)
//         .open("initiated_orders.json")?;
//     let writer = BufWriter::new(file);
//     serde_json::to_writer(writer, &initiate_ids)?;

//     info!(
//         "Successfully saved {} initiated orders to file",
//         initiate_ids.len()
//     );

//     Ok(())
// }

// async fn create_orders() -> Result<()> {
//     info!(
//         "Starting order creation with all 100 coroutines at once, 1-second gap between orders..."
//     );
//     let start = std::time::Instant::now();

//     let client = Arc::new(reqwest::Client::builder().build()?);
//     let semaphore = Arc::new(Semaphore::new(TOTAL_COROUTINES));

//     // Change this line to use Arc<Mutex>
//     let order_data = Arc::new(AsyncMutex::new(Vec::new()));
//     let mut total_created = 0;
//     let mut total_failed = 0;
//     let mut error_counts: std::collections::HashMap<String, usize> =
//         std::collections::HashMap::new();

//     let mut futures = FuturesUnordered::new();

//     for coroutine_id in 0..TOTAL_COROUTINES {
//         let client = client.clone();
//         let semaphore: Arc<Semaphore> = semaphore.clone();
//         let order_data = order_data.clone(); // Clone the Arc for each coroutine

//         futures.push(async move {
//             let _permit = semaphore.acquire().await.unwrap();
//             let coroutine_start = std::time::Instant::now();

//             info!(
//                 "Coroutine {} starting to process {} orders",
//                 coroutine_id + 1,
//                 ORDERS_PER_COROUTINE
//             );

//             let mut results = Vec::new();

//             // Process 5 orders sequentially within this coroutine
//             for i in 0..ORDERS_PER_COROUTINE {
//                 match create_order(&client).await {
//                     Ok((order_id, secret)) => {
//                         results.push(Ok((order_id.clone(), secret.clone())));
//                         // Lock and push to the shared vector
//                         order_data.lock().await.push((order_id, secret));
//                         info!(
//                             "Coroutine {}: Created order {} of {}",
//                             coroutine_id + 1,
//                             i + 1,
//                             ORDERS_PER_COROUTINE
//                         );
//                     }
//                     Err(e) => {
//                         results.push(Err(e));
//                         info!(
//                             "Coroutine {}: Failed to create order {} of {}",
//                             coroutine_id + 1,
//                             i + 1,
//                             ORDERS_PER_COROUTINE
//                         );
//                     }
//                 }
//             }

//             info!(
//                 "Coroutine {} completed {} orders in {:?}",
//                 coroutine_id + 1,
//                 ORDERS_PER_COROUTINE,
//                 coroutine_start.elapsed()
//             );

//             results
//         });
//     }

//     // Collect results from all coroutines
//     while let Some(results) = futures.next().await {
//         for result in results {
//             match result {
//                 Ok((_order_id, _encoded_secret)) => {
//                     total_created += 1;
//                     // Log progress periodically
//                     if total_created % 50 == 0 {
//                         info!("Progress: {} orders created so far", total_created);
//                     }
//                 }
//                 Err(e) => {
//                     total_failed += 1;
//                     let error_type = e.to_string();
//                     *error_counts.entry(error_type).or_insert(0) += 1;
//                 }
//             }
//         }
//     }

//     // Now you can access the order_data after all futures complete
//     let order_data = order_data.lock().await;
//     let order_ids: Vec<String> = order_data.iter().map(|(id, _)| id.clone()).collect();

//     // Rest of your code remains the same...
//     info!(
//         "Order creation completed in {:?}. Created: {}, Failed: {}, Total attempted: {}",
//         start.elapsed(),
//         total_created,
//         total_failed,
//         total_created + total_failed
//     );

//     if !error_counts.is_empty() {
//         info!("Creation error distribution:");
//         for (error_type, count) in error_counts {
//             info!("  {}: {} occurrences", error_type, count);
//         }
//     }

//     // Saving the order IDs
//     let file = fs::OpenOptions::new()
//         .write(true)
//         .create(true)
//         .truncate(true)
//         .open("order_ids.json")?;
//     let writer = BufWriter::new(file);
//     serde_json::to_writer(writer, &order_ids)?;

//     // For saving full datas
//     let full_data = fs::OpenOptions::new()
//         .write(true)
//         .create(true)
//         .truncate(true)
//         .open("order_secrets.json")?;

//     let writer = BufWriter::new(full_data);
//     serde_json::to_writer(writer, &*order_data)?;

//     info!("Successfully saved {} order IDs to file", order_ids.len());
//     info!(
//         "Successfully saved {} order id with secret",
//         order_data.len()
//     );

//     Ok(())
// }
