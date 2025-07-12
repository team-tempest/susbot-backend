pub mod structs;

pub use crate::structs::{EtherscanApiResponse, ScanResult, EtherscanApiResult};
use candid::{CandidType, Deserialize};
use ic_cdk::api::management_canister::http_request::{
    http_request, CanisterHttpRequestArgument, HttpMethod,
};
use ic_cdk::update;

// TODO: Replace this with your own Etherscan API key
const ETHERSCAN_API_KEY: &str = "DFB3ZHKRG2PZYCB8M4I6EQS15NQQ638PBJ";

// --- Main Logic ---

#[update]
async fn analyze_address(address: String) -> ScanResult {
    if !address.starts_with("0x") || address.len() != 42 {
        return ScanResult {
            score: 0,
            summary: "Error: Invalid Ethereum address format.".to_string(),
            risks: vec![],
        };
    }

    let url = format!(
        "https://api.etherscan.io/api?module=contract&action=getsourcecode&address={}&apikey={}",
        address, ETHERSCAN_API_KEY
    );

    let request = CanisterHttpRequestArgument {
        url: url.clone(),
        method: HttpMethod::GET,
        body: None,
        max_response_bytes: Some(2_000_000),
        transform: None,                     
        headers: vec![],
    };

    match http_request(request, 21000000000).await {
        Ok((response,)) => {
            let body_str = String::from_utf8(response.body)
                .unwrap_or_else(|_| "Invalid UTF-8 in response body".to_string());

            // Try to parse the JSON response
            let parsed_response: Result<EtherscanApiResponse, _> = serde_json::from_str(&body_str);

            match parsed_response {
                Ok(etherscan_data) => {
                    if etherscan_data.status == "1" && !etherscan_data.result.is_empty() {
                        let contract_info = &etherscan_data.result[0];
                        let summary = if contract_info.source_code.is_empty() {
                            format!(
                                "Contract '{}' source code is NOT verified.",
                                contract_info.contract_name
                            )
                        } else {
                            format!(
                                "Successfully fetched source code for contract '{}'.",
                                contract_info.contract_name
                            )
                        };
                        ScanResult {
                            score: 50, // Placeholder score
                            summary,
                            risks: vec![format!(
                                "Source Code Length: {}",
                                contract_info.source_code.len()
                            )],
                        }
                    } else {
                        ScanResult {
                            score: 0,
                            summary: format!(
                                "Etherscan API returned an error: {}",
                                etherscan_data.message
                            ),
                            risks: vec![],
                        }
                    }
                }
                Err(e) => ScanResult {
                    score: 0,
                    summary: "Failed to parse Etherscan API response.".to_string(),
                    risks: vec![e.to_string()],
                },
            }
        }
        Err((code, message)) => ScanResult {
            score: 0,
            summary: "HTTP request to Etherscan failed.".to_string(),
            risks: vec![format!("Rejection Code: {:?}, Message: {}", code, message)],
        },
    }
}

ic_cdk::export_candid!();