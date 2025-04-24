use crate::models::additional_data::{AdditonalData, SignableAdditionalData};
use alloy::primitives::{Address, FixedBytes, Uint};
use alloy::sol;
use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};

sol! {
    struct Initiate {
        address redeemer;
        uint256 timelock;
        uint256 amount;
        bytes32 secretHash;
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InitiateRequest {
    pub order_id: String,
    pub signature: String,
    pub perform_on: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Status {
    Ok,
    Error,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub status: Status,
    #[serde(rename = "result")]
    pub data: Option<T>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AttestedData {
    pub sig: String,
    pub input_token_price: f64,
    pub output_token_price: f64,
    pub bitcoin_optional_recipient: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AttestedResponse {
    pub result: AtttestedResult,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AtttestedResult {
    pub additional_data: AttestedData,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Order<T = AdditonalData> {
    pub source_chain: String,
    pub destination_chain: String,
    pub source_asset: String,
    pub destination_asset: String,
    pub initiator_source_address: String,
    pub initiator_destination_address: String,
    pub source_amount: BigDecimal,
    pub destination_amount: BigDecimal,
    pub fee: BigDecimal,
    pub nonce: BigDecimal,
    pub min_destination_confirmations: i64,
    pub timelock: i64,
    pub secret_hash: String,
    pub additional_data: T,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoadOrder<T = AdditonalData> {
    pub source_chain: String,
    pub destination_chain: String,
    pub source_asset: String,
    pub destination_asset: String,
    pub initiator_source_address: String,
    pub initiator_destination_address: String,
    pub source_amount: BigDecimal,
    pub destination_amount: BigDecimal,
    pub fee: BigDecimal,
    pub nonce: BigDecimal,
    pub min_destination_confirmations: i64,
    pub timelock: i64,
    pub additional_data: T,
}
impl Order<AdditonalData> {
    pub fn signable_order(&self) -> Order<SignableAdditionalData> {
        let o = self.clone();
        let signable_additional_data = SignableAdditionalData {
            strategy_id: o.additional_data.strategy_id.clone(),
            bitcoin_optional_recipient: o.additional_data.bitcoin_optional_recipient.clone(),
        };
        Order {
            source_chain: o.source_chain,
            destination_chain: o.destination_chain,
            source_asset: o.source_asset,
            destination_asset: o.destination_asset,
            initiator_source_address: o.initiator_source_address,
            initiator_destination_address: o.initiator_destination_address,
            source_amount: o.source_amount,
            destination_amount: o.destination_amount,
            fee: o.fee,
            nonce: o.nonce,
            min_destination_confirmations: o.min_destination_confirmations,
            timelock: o.timelock,
            secret_hash: o.secret_hash,
            additional_data: signable_additional_data,
        }
    }
}
