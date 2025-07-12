pub mod structs;
pub mod analysis;

pub use crate::structs::{EtherscanApiResponse, ScanResult, EtherscanApiResult};
use candid::Nat;
use ic_cdk::management_canister::{
    http_request, HttpRequestArgs, HttpMethod, HttpRequestResult,
};
use ic_cdk::update;
use std::borrow::Cow;
use crate::structs::ContractSources;

// IMPORTANT: In a real-world application, this API key should be stored securely
// using a service like IC secrets management, not hardcoded.
const ETHERSCAN_API_KEY: &str = option_env!("ETHERSCAN_API_KEY")
    .expect("ETHERSCAN_API_KEY environment variable not set during build.");
const CYCLES_PER_HTTP_REQUEST: u128 = 2_000_000_000;

#[update]
async fn analyze_address(address: String) -> ScanResult {
    if !is_valid_ethereum_address(&address) {
        return ScanResult::new_error("Error: Invalid Ethereum address format.", vec![]);
    }

    let request = build_etherscan_request(&address);

    match http_request(&request).await {
        Ok(response) => process_response(response),
        Err(err) => {
            ScanResult::new_error("HTTP request to Etherscan failed. Error: {}", vec![err.to_string()])
        }
    }
}

/// Validates if the given string is a plausible Ethereum address.
fn is_valid_ethereum_address(address: &str) -> bool {
    address.starts_with("0x") && address.len() == 42
}

/// Constructs the HTTP request to query the Etherscan API.
fn build_etherscan_request(address: &str) -> HttpRequestArgs {
    let url = format!(
        "https://api.etherscan.io/api?module=contract&action=getsourcecode&address={}&apikey={}",
        address, ETHERSCAN_API_KEY
    );

    HttpRequestArgs {
        url,
        method: HttpMethod::GET,
        body: None,
        max_response_bytes: Some(2_000_000),
        transform: None,
        headers: vec![],
    }
}

/// Processes the HttpResponse from the Etherscan API.
fn process_response(response: HttpRequestResult) -> ScanResult {
    if response.status >= Nat::from(200u32) && response.status < Nat::from(300u32) {
        match serde_json::from_slice::<EtherscanApiResponse>(&response.body) {
            Ok(etherscan_data) => process_etherscan_data(etherscan_data),
            Err(e) => ScanResult::new_error(
                "Failed to parse Etherscan API response.",
                vec![e.to_string()],
            ),
        }
    } else {
        ScanResult::new_error(
            "HTTP request to Etherscan failed.",
            vec![format!(
                "HTTP Status: {}. Body: {}",
                response.status,
                String::from_utf8_lossy(&response.body)
            )],
        )
    }
}

/// Extracts and concatenates the true source code from the Etherscan API response.
/// This function handles cases where the source code is a single file, a JSON object
/// of multiple files, or a double-encoded JSON string for standard-json-input formats.
fn extract_true_source_code(etherscan_source: &str) -> String {
    // Case 1: Standard-JSON-Input format (double-encoded JSON)
    if etherscan_source.starts_with("{{") && etherscan_source.ends_with("}}") {
        let inner_json_str = &etherscan_source[1..etherscan_source.len() - 1];
        if let Ok(sources) = serde_json::from_str::<ContractSources>(inner_json_str) {
            return sources
                .sources
                .values()
                .map(|file| file.content.as_str())
                .collect::<Vec<&str>>()
                .join("\n");
        }
    }

    // Case 2: Simple JSON object with a 'sources' key
    if let Ok(sources) = serde_json::from_str::<ContractSources>(etherscan_source) {
        return sources
            .sources
            .values()
            .map(|file| file.content.as_str())
            .collect::<Vec<&str>>()
            .join("\n");
    }

    // Case 3: A single, flat source file (not JSON)
    etherscan_source.to_string()
}


/// Processes the parsed data from the Etherscan API and builds the final ScanResult.
fn process_etherscan_data(etherscan_data: EtherscanApiResponse) -> ScanResult {
    if etherscan_data.status == "1" && !etherscan_data.result.is_empty() {
        let contract_info = &etherscan_data.result[0];

        if contract_info.source_code.is_empty() {
            let summary = format!(
                "Contract '{}' source code is NOT verified.",
                contract_info.contract_name
            );
            let risks = vec![format!(
                "Unverified contract: {}",
                contract_info.contract_name
            )];
            ScanResult {
                score: 50, // Neutral score for unverified contracts
                summary,
                risks,
            }
        } else {
            let true_source_code = extract_true_source_code(&contract_info.source_code);
            let analysis_result = analysis::analyze_source_code(&true_source_code);

            let mut critical_risks = 0;
            let mut high_risks = 0;
            let mut medium_risks = 0;
            let mut low_risks = 0;
            let mut info_risks = 0;

            for risk in &analysis_result.risks {
                match risk.risk_level {
                    analysis::RiskLevel::Critical => critical_risks += 1,
                    analysis::RiskLevel::High => high_risks += 1,
                    analysis::RiskLevel::Medium => medium_risks += 1,
                    analysis::RiskLevel::Low => low_risks += 1,
                    analysis::RiskLevel::Info => info_risks += 1,
                }
            }

            let summary = format!(
                "Analysis of '{}' complete. Found {} critical, {} high, {} medium, {} low, and {} informational risks. Final Score: {}",
                contract_info.contract_name,
                critical_risks,
                high_risks,
                medium_risks,
                low_risks,
                info_risks,
                analysis_result.score
            );

            let risks = analysis_result
                .risks
                .iter()
                .map(|r| r.to_string())
                .collect();

            ScanResult {
                score: analysis_result.score,
                summary,
                risks,
            }
        }
    } else {
        ScanResult::new_error(
            "Etherscan API returned an error.",
            vec![etherscan_data.message],
        )
    }
}

ic_cdk::export_candid!();