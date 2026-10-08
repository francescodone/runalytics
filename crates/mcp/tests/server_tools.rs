//! End-to-end tool tests: a real rmcp client talking to the server over an
//! in-memory duplex pipe. This is the only place the wire format itself is
//! exercised; the op-level behaviour lives in `ops::tests`.

use rmcp::model::CallToolRequestParams;
use rmcp::{ClientHandler, ServiceExt};
use runalytics_mcp::{RunalyticsServer, ServerConfig, ops::Context};
use serde_json::json;

struct TestClient;

impl ClientHandler for TestClient {}

async fn handshake() -> rmcp::service::RunningService<rmcp::RoleClient, TestClient> {
    let ctx = Context::open(ServerConfig::default()).expect("in-memory context");
    let server = RunalyticsServer::new(ctx);
    let (client_io, server_io) = tokio::io::duplex(64 * 1024);
    tokio::spawn(async move {
        let service = server.serve(server_io).await.expect("server serve");
        let _ = service.waiting().await;
    });
    TestClient.serve(client_io).await.expect("client handshake")
}

async fn call(
    peer: &rmcp::service::Peer<rmcp::RoleClient>,
    tool: &str,
    args: serde_json::Value,
) -> rmcp::model::CallToolResult {
    let params = CallToolRequestParams::new(tool.to_string())
        .with_arguments(args.as_object().cloned().unwrap_or_default());
    peer.call_tool(params).await.expect("tool call roundtrip")
}

fn payload(result: &rmcp::model::CallToolResult) -> serde_json::Value {
    let text = result
        .content
        .iter()
        .filter_map(|c| match c {
            rmcp::model::ContentBlock::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .collect::<String>();
    serde_json::from_str(&text).expect("tool payload is JSON")
}

#[tokio::test]
async fn tools_are_listed_with_schemas() {
    let client = handshake().await;
    let tools = client.list_tools(None).await.expect("list_tools");
    let names: Vec<_> = tools.tools.iter().map(|t| t.name.as_ref()).collect();
    for expected in [
        "generate_plan",
        "activate_plan",
        "update_session",
        "sync_calendar",
        "get_status",
        "score_day",
    ] {
        assert!(
            names.contains(&expected),
            "missing tool {expected} in {names:?}"
        );
    }
    let generate = tools
        .tools
        .iter()
        .find(|t| t.name == "generate_plan")
        .expect("generate_plan");
    let schema = &generate.input_schema;
    assert_eq!(schema["type"], "object");
    assert!(schema["properties"]["request"].is_object());
    client.cancel().await.expect("cancel");
}

#[tokio::test]
async fn status_over_the_wire() {
    let client = handshake().await;
    let result = call(client.peer(), "get_status", json!({})).await;
    assert!(!result.is_error.unwrap_or(false), "get_status errored");
    let body = payload(&result);
    assert!(body["today"].is_string());
    assert!(body["activePlan"].is_null());
    client.cancel().await.expect("cancel");
}

#[tokio::test]
async fn generate_activate_roundtrip_over_the_wire() {
    let client = handshake().await;
    let athlete =
        runalytics_core::AthleteSnapshot::placeholder("Europe/Berlin".parse().expect("tz"));
    let request = json!({
        "goal": "five_k",
        "anchor": {"kind": "horizon", "value": {"weeks": 6}},
        "athlete": athlete,
        "constraints": {
            "blackoutWeekdays": [],
            "noQualityWeekdays": [],
            "preferredStart": "07:00",
            "maxWeeklyVolume": null,
            "qualitySessions": null,
            "blackoutDates": [],
            "longRunWeekday": null,
        },
    });
    let generated = call(
        client.peer(),
        "generate_plan",
        json!({ "request": request, "save": true }),
    )
    .await;
    assert!(!generated.is_error.unwrap_or(false));
    let plan_id = payload(&generated)["planId"]
        .as_str()
        .expect("plan id")
        .to_string();

    let activated = call(client.peer(), "activate_plan", json!({ "planId": plan_id })).await;
    assert!(!activated.is_error.unwrap_or(false));

    let status = call(client.peer(), "get_status", json!({})).await;
    assert_eq!(
        payload(&status)["activePlan"]["id"].as_str(),
        Some(plan_id.as_str())
    );
    client.cancel().await.expect("cancel");
}

#[tokio::test]
async fn tool_errors_come_back_as_error_results() {
    let client = handshake().await;
    let result = call(client.peer(), "get_plan", json!({ "planId": "not-a-uuid" })).await;
    assert_eq!(result.is_error, Some(true), "bad id must be a tool error");
    assert!(
        payload(&result)["error"]
            .as_str()
            .expect("error field")
            .contains("not a UUID")
    );
    client.cancel().await.expect("cancel");
}

#[tokio::test]
async fn initialize_reports_server_info() {
    // The handshake itself asserts the initialize result was accepted; here
    // we verify the advertised identity an agent sees on connect.
    let (client_io, server_io) = tokio::io::duplex(64 * 1024);
    let ctx = Context::open(ServerConfig::default()).expect("ctx");
    tokio::spawn(async move {
        let service = RunalyticsServer::new(ctx)
            .serve(server_io)
            .await
            .expect("serve");
        let _ = service.waiting().await;
    });
    let client = TestClient.serve(client_io).await.expect("handshake");
    let info = client
        .peer()
        .peer_info()
        .expect("peer info after initialize");
    let server_impl = info.server_info.as_ref().expect("server info advertised");
    assert_eq!(server_impl.name, "runalytics");
    assert!(
        info.instructions
            .as_deref()
            .is_some_and(|i| i.contains("generate_plan")),
        "instructions should name the entry tool"
    );
    client.cancel().await.expect("cancel");
}
