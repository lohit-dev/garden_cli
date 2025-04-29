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
    
    pub order_id: String,
    
    pub secret: String,
    
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


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InitiateRequest {
    
    pub order_id: String,
    
    pub signature: String,
    
    pub perform_on: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StarkInitiateRequst {
    pub order_id: String,
    pub signature: Vec<String>,
    pub perform_on: String,
}
