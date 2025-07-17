pub mod analysis;
pub mod structs;

use crate::analysis::AnalysisResult;
use crate::structs::{
    ContractSources, EtherscanApiResponse, EtherscanApiResult, OpenAiMessage, OpenAiRequest,
    OpenAiResponse, ScanResult,
};
use analysis::RiskLevel::{Critical, High, Info, Low, Medium};
use candid::Nat;
use ic_cdk::management_canister::{
    http_request, HttpHeader, HttpMethod, HttpRequestArgs, HttpRequestResult,
};
use ic_cdk::update;

const ETHERSCAN_API_KEY: &str = match option_env!("ETHERSCAN_API_KEY") {
    Some(key) => key,
    None => "demo_key_for_testing",
};

const OPENAI_API_KEY: &str = match option_env!("OPENAI_API_KEY") {
    Some(key) => key,
    None => "demo_key_for_testing",
};

const UNVERIFIED_CONTRACT_NEUTRAL_SCORE: u8 = 50;

#[update]
async fn analyze_address(address: String) -> ScanResult {
    if !is_valid_ethereum_address(&address) {
        return ScanResult::new_error("Error: Invalid Ethereum address format.", vec![]);
    }

    let request = build_etherscan_request(&address);

    match http_request(&request).await {
        Ok(response) => process_response(response).await,
        Err(error) => ScanResult::new_error(
            "HTTP request to Etherscan failed.",
            vec![format!("Error: {}", error.to_string())],
        ),
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
async fn process_response(response: HttpRequestResult) -> ScanResult {
    if response.status >= Nat::from(200u32) && response.status < Nat::from(300u32) {
        match serde_json::from_slice::<EtherscanApiResponse>(&response.body) {
            Ok(etherscan_data) => process_etherscan_data(etherscan_data).await,
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
pub fn extract_true_source_code(etherscan_source: &str) -> String {
    if is_double_encoded_json(etherscan_source) {
        let inner_json_str = &etherscan_source[1..etherscan_source.len() - 1];
        if let Ok(sources) = ContractSources::from_string(inner_json_str) {
            return sources.to_string();
        }
    }

    if let Ok(sources) = ContractSources::from_string(etherscan_source) {
        return sources.to_string();
    }

    not_json_to_string(etherscan_source)
}

fn not_json_to_string(etherscan_source: &str) -> String {
    etherscan_source.to_string()
}

fn is_double_encoded_json(etherscan_source: &str) -> bool {
    etherscan_source.starts_with("{{") && etherscan_source.ends_with("}}")
}

/// Processes the parsed data from the Etherscan API and builds the final ScanResult.
async fn process_etherscan_data(etherscan_data: EtherscanApiResponse) -> ScanResult {
    if is_scan_sucessful(&etherscan_data) {
        let contract_info = &etherscan_data.result[0];

        if contract_info.source_code.is_empty() {
            // Create an AnalysisResult for unverified contract
            let unverified_analysis = analysis::analyze_source_code_with_verification("", false);
            
            let ai_summary = match get_ai_summary(&unverified_analysis, &contract_info.contract_name).await {
                Ok(summary) => summary,
                Err(_e) => {
                    format!(
                        "Contract '{}' source code is NOT verified on Etherscan. \
                        Without verified source code, it's impossible to audit the contract for security vulnerabilities. \
                        This is a major red flag for transparency and security.",
                        contract_info.contract_name
                    )
                }
            };
            
            ScanResult {
                score: UNVERIFIED_CONTRACT_NEUTRAL_SCORE,
                summary: ai_summary,
                risks: vec!["The contract source code is not verified on Etherscan.".to_string()],
            }
        } else {
            let true_source_code = extract_true_source_code(&contract_info.source_code);
            let analysis_result = analysis::analyze_source_code_with_verification(&true_source_code, true);

            let ai_summary = match get_ai_summary(&analysis_result, &contract_info.contract_name).await {
                Ok(summary) => summary,
                Err(e) => {
                    ic_cdk::println!("AI summary failed: {}", e);
                    create_summary_for_verified(contract_info, &analysis_result)
                }
            };

            let risks = analysis_result
                .risks
                .iter()
                .map(|r| r.to_string())
                .collect();

            ScanResult {
                score: analysis_result.score,
                summary: ai_summary,
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

fn is_scan_sucessful(etherscan_data: &EtherscanApiResponse) -> bool {
    etherscan_data.status == "1" && !etherscan_data.result.is_empty()
}

fn create_summary_for_verified(
    contract_info: &EtherscanApiResult,
    analysis_result: &AnalysisResult,
) -> String {
    let mut critical_risks = 0;
    let mut high_risks = 0;
    let mut medium_risks = 0;
    let mut low_risks = 0;
    let mut info_risks = 0;

    for risk in &analysis_result.risks {
        match risk.risk_level {
            Critical => critical_risks += 1,
            High => high_risks += 1,
            Medium => medium_risks += 1,
            Low => low_risks += 1,
            Info => info_risks += 1,
        }
    }

    format!(
        "Analysis of '{}' complete. Found {} critical, {} high, {} medium, {} low, and {} informational risks. Final Score: {}",
        contract_info.contract_name,
        critical_risks,
        high_risks,
        medium_risks,
        low_risks,
        info_risks,
        analysis_result.score
    )
}

async fn get_ai_summary(
    analysis_result: &AnalysisResult,
    contract_name: &str,
) -> Result<String, String> {
    let _risks_json = serde_json::to_string_pretty(&analysis_result.risks)
        .map_err(|e| format!("Failed to serialize risks: {}", e))?;

    let risks_formatted = analysis_result.risks.iter()
        .map(|r| format!("{}: {}", r.check_name, r.description))
        .collect::<Vec<String>>()
        .join("\n");

    let prompt = format!(
        "You are a Web3 security analyst reviewing smart contract risks.\n\
        Your task is to analyze the given smart contract, explain the security issues in simple terms, and assign a final trust score between 0 and 100. Return your response in strict JSON format.\n\n\
        Input:\n\
        - Contract Name: {}\n\
        - Risks Detected:\n{}\n\n\
        - Contract Traits:\n\
          - Verified Source Code: {}\n\
          - Good Token Distribution: {}\n\
          - Contract Type: {}\n\n\
        Please analyze these risks and provide a comprehensive summary with recommendations in valid JSON format. \
        Focus on explaining technical risks in simple terms for non-technical users.\n\
        The response should include verdict, summary, and recommendations fields.",
        contract_name,
        risks_formatted,
        analysis_result.contract_traits.verified,
        analysis_result.contract_traits.good_distribution,
        analysis_result.contract_traits.contract_type
    );

    let request_body = OpenAiRequest {
        model: "gpt-3.5-turbo".to_string(),
        messages: vec![
            OpenAiMessage {
                role: "system".to_string(),
                content: "You are a helpful web3 security assistant. Always respond with valid JSON.".to_string(),
            },
            OpenAiMessage {
                role: "user".to_string(),
                content: prompt,
            },
        ],
    };

    let request_body_bytes = serde_json::to_vec(&request_body)
        .map_err(|e| format!("Failed to serialize OpenAI request: {}", e))?;

    let request = HttpRequestArgs {
        url: "https://api.openai.com/v1/chat/completions".to_string(),
        method: HttpMethod::POST,
        body: Some(request_body_bytes),
        max_response_bytes: Some(4096),
        transform: None,
        headers: vec![
            HttpHeader {
                name: "Authorization".to_string(),
                value: format!("Bearer {}", OPENAI_API_KEY),
            },
            HttpHeader {
                name: "Content-Type".to_string(),
                value: "application/json".to_string(),
            },
        ],
    };

    match http_request(&request).await {
        Ok(response) => {
            if response.status >= Nat::from(200u32) && response.status < Nat::from(300u32) {
                let response_body: OpenAiResponse = serde_json::from_slice(&response.body)
                    .map_err(|e| format!("Failed to parse OpenAI response: {}", e))?;
                if let Some(choice) = response_body.choices.into_iter().next() {
                    Ok(choice.message.content)
                } else {
                    Err("OpenAI response contained no choices.".to_string())
                }
            } else {
                Err(format!(
                    "OpenAI API returned an error. Status: {}, Body: {}",
                    response.status,
                    String::from_utf8_lossy(&response.body)
                ))
            }
        }
        Err(error) => Err(format!(
            "HTTP request to OpenAI failed. Error {}",
            error.to_string()
        )),
    }
}

ic_cdk::export_candid!();
