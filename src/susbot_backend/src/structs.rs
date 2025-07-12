use candid::CandidType;
use serde_derive::{Deserialize, Serialize};

#[derive(CandidType, Deserialize, Clone)]
pub struct ScanResult {
    pub score: u8,
    pub summary: String,
    pub risks: Vec<String>,
}

impl ScanResult {
    /// Creates a new error ScanResult.
    pub fn new_error(summary: &str, risks: Vec<String>) -> Self {
        Self {
            score: 0,
            summary: summary.to_string(),
            risks,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct EtherscanApiResponse {
    pub status: String,
    pub message: String,
    pub result: Vec<EtherscanApiResult>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct EtherscanApiResult {
    #[serde(rename = "SourceCode")]
    pub source_code: String,
    #[serde(rename = "ContractName")]
    pub contract_name: String,
}
