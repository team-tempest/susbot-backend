use candid::{decode_one, encode_one, Principal};
use pocket_ic::common::rest::{
    CanisterHttpReply, CanisterHttpRequest, CanisterHttpResponse, MockCanisterHttpResponse,
    RawMessageId,
};
use pocket_ic::PocketIc;
use std::fs;
use susbot_backend::{EtherscanApiResponse, EtherscanApiResult, ScanResult};

const ANALYZE_ADDRESS: &'static str = "analyze_address";
const BACKEND_WASM: &str = "../../target/wasm32-unknown-unknown/release/susbot_backend.wasm";
const POCKET_IC_BINARY_PATH: &'static str = "pocket-ic";

fn setup() -> (PocketIc, Principal) {
    std::env::set_var("POCKET_IC_BIN", POCKET_IC_BINARY_PATH);
    let pic = PocketIc::new();

    let backend_canister = pic.create_canister();
    pic.add_cycles(backend_canister, 2_000_000_000_000);
    let wasm = fs::read(BACKEND_WASM).expect("Wasm file not found, run 'dfx build'.");
    pic.install_canister(backend_canister, wasm, vec![], None);
    (pic, backend_canister)
}

#[test]
fn test_successful_scan() {
    let input_token = "0xAb5801a7D398351b8bE11C439e05C5B3259aeC9B";
    let body = create_successful_response();
    let response_http_status_code = 200;

    let (pic, backend_canister) = setup();

    let (call_id, canister_http_request) = start_processing(&pic, backend_canister, input_token);

    mock_http_call(body, response_http_status_code, &pic, canister_http_request);

    let reply = pic.await_call(call_id);
    assert!(reply.is_ok());
    let res = decode_one::<ScanResult>(&reply.unwrap());
    assert!(res.is_ok());
    let scan_result = res.unwrap();

    assert_eq!(50, scan_result.score);
    assert!(!scan_result.risks.is_empty());
    assert!(scan_result.risks.get(0).unwrap().contains("3"));
    assert!(scan_result.summary.contains("Successfully fetched source code for contract"));
}

fn mock_http_call(
    body: Vec<u8>,
    response_http_status_code: u16,
    pic: &PocketIc,
    canister_http_request: CanisterHttpRequest,
) {
    let mock_canister_http_response = MockCanisterHttpResponse {
        subnet_id: canister_http_request.subnet_id,
        request_id: canister_http_request.request_id,
        response: CanisterHttpResponse::CanisterHttpReply(CanisterHttpReply {
            status: response_http_status_code,
            headers: vec![],
            body,
        }),
        additional_responses: vec![],
    };

    pic.mock_canister_http_response(mock_canister_http_response);
    assert_no_more_http_outcalls(pic);
}

fn assert_no_more_http_outcalls(pic: &PocketIc) {
    let canister_http_requests = pic.get_canister_http();
    assert_eq!(canister_http_requests.len(), 0);
}

fn create_successful_response() -> Vec<u8> {
    let scan_result = EtherscanApiResult {
        source_code: "ABC".to_string(),
        contract_name: "Name".to_string(),
    };

    let resp = EtherscanApiResponse {
        status: "1".to_string(),
        message: "".to_string(),
        result: vec![scan_result],
    };

    let body = serde_json::to_vec(&resp).unwrap();
    body
}

fn start_processing(
    pic: &PocketIc,
    backend_canister: Principal,
    input_token: &str,
) -> (RawMessageId, CanisterHttpRequest) {
    let call_id = pic
        .submit_call(
            backend_canister,
            Principal::anonymous(),
            ANALYZE_ADDRESS,
            encode_one(input_token).unwrap(),
        )
        .unwrap();

    let canister_http_request = wait_for_http_call(&pic);
    (call_id, canister_http_request)
}

fn wait_for_http_call(pic: &PocketIc) -> CanisterHttpRequest {
    pic.tick();
    pic.tick();
    let canister_http_requests = pic.get_canister_http();
    assert_eq!(canister_http_requests.len(), 1);
    canister_http_requests[0].clone()
}
