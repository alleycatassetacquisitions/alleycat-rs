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
