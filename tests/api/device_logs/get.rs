use crate::helpers::spawn_app;
use alleycat_rs::proto::alleycat::device::WriteDeviceLogRequest;
use reqwest::header::CONTENT_TYPE;
use serde_json::Value;

#[tokio::test]
async fn get_device_logs_returns_json_with_saved_reports() {
    let app = spawn_app().await;
    let report = WriteDeviceLogRequest {
        device_mac: Some("aa:bb:cc:dd:ee:ff".to_string()),
        crash_number: Some(42),
        uptime_ms: Some(123_456),
        software_version: Some("0.1.0".to_string()),
        reset_reason: Some(1),
        program_counter: Some(0x4008_1234),
        exception_cause: Some(6),
        task_name: Some("main".to_string()),
    };

    let write_response = app.post_protobuf("/device-logs", &report).await;
    assert_eq!(204, write_response.status().as_u16());

    let response = reqwest::Client::new()
        .get(format!("{}/device-logs", app.address))
        .send()
        .await
        .expect("Failed to execute device-log read request.");

    assert_eq!(200, response.status().as_u16());
    assert_eq!(
        response.headers().get(CONTENT_TYPE).unwrap(),
        "application/json"
    );

    let body = response
        .json::<Value>()
        .await
        .expect("Response was not valid JSON.");

    assert_eq!(body["pagination"]["page"], 1);
    assert_eq!(body["pagination"]["per_page"], 20);

    let device_logs = body["device_logs"]
        .as_array()
        .expect("`device_logs` was not a JSON array.");

    assert_eq!(device_logs.len(), 1);
    let device_log = &device_logs[0];

    assert!(device_log["id"].is_string());
    assert!(device_log["received_at"].is_string());
    assert_eq!(device_log["device_mac"], "aa:bb:cc:dd:ee:ff");
    assert_eq!(device_log["crash_number"], 42);
    assert_eq!(device_log["uptime_ms"], 123_456);
    assert_eq!(device_log["software_version"], "0.1.0");
    assert_eq!(device_log["reset_reason"], 1);
    assert_eq!(device_log["program_counter"], 0x4008_1234_u32);
    assert_eq!(device_log["exception_cause"], 6);
    assert_eq!(device_log["task_name"], "main");
}
