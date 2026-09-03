use super::*;
use serde_json::json;

#[test]
fn test_json_rpc_response_constructors() {
    let succ = JsonRpcResponse::success(Some(json!(1)), json!({ "status": "ok" }));
    assert_eq!(succ.jsonrpc, "2.0");
    assert!(succ.error.is_none());
    assert_eq!(succ.result.unwrap(), json!({ "status": "ok" }));

    let err = JsonRpcResponse::error(Some(json!(2)), -32600, "Invalid request");
    assert_eq!(err.jsonrpc, "2.0");
    assert!(err.result.is_none());
    assert_eq!(err.error.unwrap().code, -32600);
}
