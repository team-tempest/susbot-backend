pub mod structs;

pub use crate::structs::{EtherscanApiResponse, ScanResult, EtherscanApiResult};
use candid::Nat;
use ic_cdk::management_canister::{
    http_request, HttpRequestArgs, HttpMethod, HttpRequestResult,
};
use ic_cdk::update;

// IMPORTANT: In a real-world application, this API key should be stored securely
// using a service like IC secrets management, not hardcoded.
const ETHERSCAN_API_KEY: &str = "DFB3ZHKRG2PZYCB8M4I6EQS15NQQ638PBJ";
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

/// Processes the parsed data from the Etherscan API and builds the final ScanResult.
fn process_etherscan_data(etherscan_data: EtherscanApiResponse) -> ScanResult {
    if etherscan_data.status == "1" && !etherscan_data.result.is_empty() {
        let contract_info = &etherscan_data.result[0];
        let summary;
        let risks;

        if contract_info.source_code.is_empty() {
            summary = format!(
                "Contract '{}' source code is NOT verified.",
                contract_info.contract_name
            );
            risks = vec![format!(
                "Unverified contract: {}",
                contract_info.contract_name
            )];
        } else {
            summary = format!(
                "Successfully fetched source code for contract '{}'.",
                contract_info.contract_name
            );
            // This is a placeholder for a more sophisticated risk analysis.
            risks = vec![format!(
                "Source Code Length: {}",
                contract_info.source_code.len()
            )];
        }
        ScanResult {
            score: 50, // Placeholder score
            summary,
            risks,
        }
    } else {
        ScanResult::new_error(
            "Etherscan API returned an error.",
            vec![etherscan_data.message],
        )
    }
}

ic_cdk::export_candid!();