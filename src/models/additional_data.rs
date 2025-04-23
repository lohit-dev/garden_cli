use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignableAdditionalData {
    pub strategy_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitcoin_optional_recipient: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdditonalData {
    pub strategy_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitcoin_optional_recipient: Option<String>,
    pub input_token_price: f64,
    pub output_token_price: f64,
    pub sig: Option<String>,
    pub deadline: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instant_refund_tx_bytes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redeem_tx_bytes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_hash: Option<String>,
}
