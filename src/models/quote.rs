use alloy::sol;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QuoteRequest {
    pub order_pair: String,
    pub amount: String,
    pub exact_out: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QuoteResponse {
    pub status: String,
    pub result: QuoteResult,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QuoteResult {
    pub quotes: HashMap<String, String>,
    pub input_token_price: f64,
    pub output_token_price: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RedeemRequest {
    /// Unique identifier of the order
    pub order_id: String,
    /// Secret used to redeem the order
    pub secret: String,
    /// Which chain to perform the redemption on
    pub perform_on: String,
}

sol! {
    struct Initiate {
        address redeemer;
        uint256 timelock;
        uint256 amount;
        bytes32 secretHash;
    }
}

// Keep the original Initiate struct but rename it to avoid conflicts
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InitiateRequest {
    /// Unique identifier of the order
    pub order_id: String,
    /// EIP-712 signature for the initiate request
    pub signature: String,
    /// Which chain to perform the swap on
    pub perform_on: String,
}
