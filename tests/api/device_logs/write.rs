use crate::helpers::spawn_app;
use alleycat_rs::proto::alleycat::device::WriteDeviceLogRequest;
use sqlx::Row;

#[tokio::test]
async fn write_device_log_returns_a_204_and_persists_a_valid_report() {
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

    let response = app.post_protobuf("/device-logs", &report).await;

    assert_eq!(204, response.status().as_u16());

    let saved = sqlx::query(
        r#"
        SELECT
            device_mac::text AS device_mac,
            crash_number,
            uptime_ms,
            software_version,
            reset_reason,
            program_counter,
            exception_cause,
            task_name
        FROM device_logs
        "#,
    )
    .fetch_one(&app.db_pool)
    .await
    .expect("Failed to fetch saved device log.");

    assert_eq!(saved.get::<String, _>("device_mac"), "aa:bb:cc:dd:ee:ff");
    assert_eq!(saved.get::<i64, _>("crash_number"), 42);
    assert_eq!(saved.get::<i64, _>("uptime_ms"), 123_456);
    assert_eq!(
        saved.get::<Option<String>, _>("software_version"),
        Some("0.1.0".to_string())
    );
    assert_eq!(saved.get::<i32, _>("reset_reason"), 1);
    assert_eq!(
        saved.get::<Option<i64>, _>("program_counter"),
        Some(i64::from(0x4008_1234_u32))
    );
    assert_eq!(saved.get::<Option<i64>, _>("exception_cause"), Some(6));
    assert_eq!(
        saved.get::<Option<String>, _>("task_name"),
        Some("main".to_string())
    );
}

#[tokio::test]
async fn omitted_report_fields_remain_null() {
    let app = spawn_app().await;
    let report = WriteDeviceLogRequest {
        device_mac: Some("aa:bb:cc:dd:ee:ff".to_string()),
        crash_number: Some(42),
        uptime_ms: Some(123_456),
        reset_reason: Some(1),
        ..Default::default()
    };
    assert_eq!(
        app.post_protobuf("/device-logs", &report).await.status(),
        204
    );

    let response = app.get("/device-logs").await;
    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    let logs = body["device_logs"].as_array().unwrap();
    assert_eq!(logs.len(), 1);
    for field in [
        "software_version",
        "program_counter",
        "exception_cause",
        "task_name",
    ] {
        assert_eq!(logs[0][field], serde_json::Value::Null, "{field}");
    }
}

#[tokio::test]
async fn report_integer_fields_preserve_maximum_values() {
    let app = spawn_app().await;
    let report = WriteDeviceLogRequest {
        device_mac: Some("aa:bb:cc:dd:ee:ff".to_string()),
        crash_number: Some(i64::MAX as u64),
        uptime_ms: Some(i64::MAX as u64),
        reset_reason: Some(i32::MAX as u32),
        program_counter: Some(u32::MAX),
        exception_cause: Some(u32::MAX),
        ..Default::default()
    };
    assert_eq!(
        app.post_protobuf("/device-logs", &report).await.status(),
        204
    );

    let response = app.get("/device-logs").await;
    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    let logs = body["device_logs"].as_array().unwrap();
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0]["crash_number"], i64::MAX);
    assert_eq!(logs[0]["uptime_ms"], i64::MAX);
    assert_eq!(logs[0]["reset_reason"], i32::MAX);
    assert_eq!(logs[0]["program_counter"], u32::MAX);
    assert_eq!(logs[0]["exception_cause"], u32::MAX);
}

#[tokio::test]
async fn duplicate_submission_preserves_original_report() {
    let app = spawn_app().await;
    let report = WriteDeviceLogRequest {
        device_mac: Some("aa:bb:cc:dd:ee:ff".to_string()),
        crash_number: Some(42),
        uptime_ms: Some(123_456),
        reset_reason: Some(1),
        ..Default::default()
    };
    assert_eq!(
        app.post_protobuf("/device-logs", &report).await.status(),
        204
    );
    let response = app.get("/device-logs").await;
    assert_eq!(response.status(), 200);
    let original: serde_json::Value = response.json().await.unwrap();
    assert_eq!(original["device_logs"].as_array().unwrap().len(), 1);

    let duplicate = WriteDeviceLogRequest {
        uptime_ms: Some(1),
        software_version: Some("replacement".to_string()),
        reset_reason: Some(2),
        program_counter: Some(3),
        exception_cause: Some(4),
        task_name: Some("replacement".to_string()),
        ..report
    };
    assert_eq!(
        app.post_protobuf("/device-logs", &duplicate).await.status(),
        204
    );

    let response = app.get("/device-logs").await;
    assert_eq!(response.status(), 200);
    let after_duplicate: serde_json::Value = response.json().await.unwrap();
    assert_eq!(after_duplicate["device_logs"], original["device_logs"]);
}
