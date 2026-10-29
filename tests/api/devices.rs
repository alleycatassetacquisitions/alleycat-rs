use crate::helpers::{assert_json, assert_text_error, spawn_app};
use alleycat_rs::proto::alleycat::device::{
    CreateDeviceInspectionRequest, CreateDeviceRequest, DeviceKind, PdnComponentChecks,
    WriteDeviceReportRequest, create_device_inspection_request::Checks,
};
use serde_json::{Value, json};
use uuid::Uuid;

fn device(label: u32) -> CreateDeviceRequest {
    CreateDeviceRequest {
        id: Uuid::new_v4().as_bytes().to_vec(),
        id_label: label,
        device_kind: DeviceKind::Pdn as i32,
        ..Default::default()
    }
}

fn device_path(device: &CreateDeviceRequest) -> String {
    format!("/devices/{}", Uuid::from_slice(&device.id).unwrap())
}

fn inspection(time: i64) -> CreateDeviceInspectionRequest {
    CreateDeviceInspectionRequest {
        id: Uuid::new_v4().as_bytes().to_vec(),
        inspected_at_unix_ms: Some(time),
        checks: Some(Checks::Pdn(PdnComponentChecks {
            screen: Some(true),
            primary_button: Some(false),
            secondary_button: Some(true),
            haptics: Some(false),
            leds: Some(true),
            power_switch: Some(false),
            input_jack: Some(true),
            output_jack: Some(false),
            housing: Some(true),
        })),
        firmware_commit_hash: Some("26a3bc".into()),
        notes: Some("Button sticks — inspect after repair".into()),
    }
}

#[tokio::test]
async fn devices_register_without_an_event_and_retries_preserve_identity() {
    let app = spawn_app().await;
    let mut registration = device(42);
    registration.hardware_version = Some(2);
    registration.mac_address = Some("AA:BB:CC:DD:EE:FF".into());
    let (first, retry) = tokio::join!(
        app.post_protobuf("/devices", &registration),
        app.post_protobuf("/devices", &registration)
    );
    assert_eq!(first.status(), 204);
    assert_eq!(retry.status(), 204);
    let path = device_path(&registration);
    let original: Value = assert_json(app.get(&path).await, 200).await;
    assert_eq!(original["id_label"], 42);
    assert_eq!(original["hardware_version"], 2);
    assert_eq!(original["device_kind"], "pdn");
    assert_eq!(original["mac_address"], "aa:bb:cc:dd:ee:ff");
    assert!(original["latest_inspection"].is_null());
    registration.mac_address = Some("aa:bb:cc:dd:ee:ff".into());
    assert_eq!(
        app.post_protobuf("/devices", &registration).await.status(),
        204
    );
    let again: Value = assert_json(app.get(&path).await, 200).await;
    assert_eq!(again, original);

    let mut conflicting = registration.clone();
    conflicting.hardware_version = Some(1);
    assert_text_error(app.post_protobuf("/devices", &conflicting).await, 409).await;
    conflicting = registration.clone();
    conflicting.mac_address = None;
    assert_text_error(app.post_protobuf("/devices", &conflicting).await, 409).await;
    conflicting = registration.clone();
    conflicting.id = Uuid::new_v4().as_bytes().to_vec();
    assert_text_error(app.post_protobuf("/devices", &conflicting).await, 409).await;
    conflicting.id_label = 43;
    assert_text_error(app.post_protobuf("/devices", &conflicting).await, 409).await;
    let inventory: Value = assert_json(app.get("/devices").await, 200).await;
    assert_eq!(inventory["devices"], json!([original]));
}

#[tokio::test]
async fn inspection_history_retains_failures_and_uses_check_time_not_upload_order() {
    let app = spawn_app().await;
    let registration = device(42);
    assert_eq!(
        app.post_protobuf("/devices", &registration).await.status(),
        204
    );
    let path = device_path(&registration);
    let history_path = format!("{path}/inspections");
    let failed = inspection(1_700_000_000_000);
    assert_eq!(
        app.post_protobuf(&history_path, &failed).await.status(),
        204
    );
    let history: Value = assert_json(app.get(&history_path).await, 200).await;
    let saved = &history["inspections"][0];
    assert_eq!(saved["checks"]["device_kind"], "pdn");
    assert_eq!(saved["firmware_commit_hash"], "26a3bc");
    assert_eq!(saved["notes"], failed.notes.as_ref().unwrap().as_str());
    assert_eq!(
        saved["device_id"],
        Uuid::from_slice(&registration.id).unwrap().to_string()
    );
    assert_eq!(
        saved["checks"]["components"],
        json!({
            "screen": true, "primary_button": false, "secondary_button": true,
            "haptics": false, "leds": true, "power_switch": false,
            "input_jack": true, "output_jack": false, "housing": true
        })
    );
    let repaired = CreateDeviceInspectionRequest {
        id: Uuid::new_v4().as_bytes().to_vec(),
        inspected_at_unix_ms: Some(1_700_000_001_000),
        checks: Some(Checks::Pdn(PdnComponentChecks {
            primary_button: Some(true),
            ..Default::default()
        })),
        ..Default::default()
    };
    assert_eq!(
        app.post_protobuf(&history_path, &repaired).await.status(),
        204
    );
    let older = inspection(1_600_000_000_000);
    assert_eq!(app.post_protobuf(&history_path, &older).await.status(), 204);
    let latest: Value = assert_json(app.get(&path).await, 200).await;
    assert_eq!(
        latest["latest_inspection"]["id"],
        Uuid::from_slice(&repaired.id).unwrap().to_string()
    );
    assert_eq!(
        latest["latest_inspection"]["checks"]["components"]["primary_button"],
        true
    );
    assert!(latest["latest_inspection"]["checks"]["components"]["screen"].is_null());
    assert!(latest["latest_inspection"]["firmware_commit_hash"].is_null());
    let history: Value = assert_json(app.get(&history_path).await, 200).await;
    assert_eq!(history["inspections"].as_array().unwrap().len(), 3);
    assert_eq!(&history["inspections"][1], saved);
    let page: Value = assert_json(
        app.get(&format!("{history_path}?page=2&per_page=1")).await,
        200,
    )
    .await;
    assert_eq!(&page["inspections"][0], saved);
    assert_eq!(page["pagination"], json!({"page": 2, "per_page": 1}));
    app.activate_new_event().await;
    app.activate_new_event().await;
    let after_switch: Value = assert_json(app.get(&path).await, 200).await;
    assert_eq!(after_switch, latest);
}

#[tokio::test]
async fn inspection_retries_are_idempotent_but_conflicting_content_is_rejected() {
    let app = spawn_app().await;
    let registration = device(1);
    assert_eq!(
        app.post_protobuf("/devices", &registration).await.status(),
        204
    );
    let path = format!("{}/inspections", device_path(&registration));
    let report = inspection(0);
    let (first, retry) = tokio::join!(
        app.post_protobuf(&path, &report),
        app.post_protobuf(&path, &report)
    );
    assert_eq!(first.status(), 204);
    assert_eq!(retry.status(), 204);
    let original: Value = assert_json(app.get(&path).await, 200).await;
    assert_eq!(original["inspections"].as_array().unwrap().len(), 1);
    assert_eq!(
        original["inspections"][0]["inspected_at"],
        "1970-01-01T00:00:00Z"
    );
    let mut conflict = report.clone();
    let Checks::Pdn(checks) = conflict.checks.as_mut().unwrap();
    checks.primary_button = Some(true);
    assert_text_error(app.post_protobuf(&path, &conflict).await, 409).await;
    conflict = report.clone();
    conflict.notes = None;
    assert_text_error(app.post_protobuf(&path, &conflict).await, 409).await;
    let another = device(2);
    assert_eq!(app.post_protobuf("/devices", &another).await.status(), 204);
    assert_text_error(
        app.post_protobuf(&format!("{}/inspections", device_path(&another)), &report)
            .await,
        409,
    )
    .await;
    let after: Value = assert_json(app.get(&path).await, 200).await;
    assert_eq!(after, original);
}

#[tokio::test]
async fn unchecked_snapshots_are_distinct_from_never_inspected_and_ties_are_stable() {
    let app = spawn_app().await;
    let registration = device(9999);
    assert_eq!(
        app.post_protobuf("/devices", &registration).await.status(),
        204
    );
    let path = device_path(&registration);
    let history_path = format!("{path}/inspections");
    let empty: Value = assert_json(app.get(&history_path).await, 200).await;
    assert_eq!(empty["inspections"], json!([]));
    let mut report = CreateDeviceInspectionRequest {
        id: Uuid::from_u128(1).as_bytes().to_vec(),
        inspected_at_unix_ms: Some(0),
        ..Default::default()
    };
    assert_eq!(
        app.post_protobuf(&history_path, &report).await.status(),
        400
    );
    report.checks = Some(Checks::Pdn(PdnComponentChecks::default()));
    assert_eq!(
        app.post_protobuf(&history_path, &report).await.status(),
        204
    );
    report.id = Uuid::from_u128(2).as_bytes().to_vec();
    assert_eq!(
        app.post_protobuf(&history_path, &report).await.status(),
        204
    );
    sqlx::query("UPDATE device_inspections SET received_at = '2020-01-01T00:00:00Z'")
        .execute(&app.db_pool)
        .await
        .unwrap();
    let current: Value = assert_json(app.get(&path).await, 200).await;
    assert_eq!(
        current["latest_inspection"]["id"],
        Uuid::from_u128(2).to_string()
    );
    assert!(
        current["latest_inspection"]["checks"]["components"]
            .as_object()
            .unwrap()
            .values()
            .all(Value::is_null)
    );
    assert!(current["hardware_version"].is_null());
    assert!(current["mac_address"].is_null());
    let history: Value = assert_json(app.get(&history_path).await, 200).await;
    assert_eq!(history["inspections"][0], current["latest_inspection"]);
}

#[tokio::test]
async fn inspection_validation_and_unknown_devices_leave_no_history() {
    let app = spawn_app().await;
    let registration = device(1);
    let path = format!("{}/inspections", device_path(&registration));
    let valid = inspection(0);
    assert_text_error(app.post_protobuf(&path, &valid).await, 404).await;
    assert_text_error(app.get(&path).await, 404).await;
    assert_text_error(app.get(&device_path(&registration)).await, 404).await;
    let mut invalid_registration = registration.clone();
    invalid_registration.id_label = 0;
    assert_text_error(
        app.post_protobuf("/devices", &invalid_registration).await,
        400,
    )
    .await;
    let inventory: Value = assert_json(app.get("/devices").await, 200).await;
    assert_eq!(inventory["devices"], json!([]));
    assert_eq!(
        app.post_protobuf("/devices", &registration).await.status(),
        204
    );
    let mut invalid = valid;
    invalid.inspected_at_unix_ms = None;
    assert_text_error(app.post_protobuf(&path, &invalid).await, 400).await;
    let history: Value = assert_json(app.get(&path).await, 200).await;
    assert_eq!(history["inspections"], json!([]));
}

#[tokio::test]
async fn inventory_pages_use_labels() {
    let app = spawn_app().await;
    for label in [3, 1, 2] {
        assert_eq!(
            app.post_protobuf("/devices", &device(label)).await.status(),
            204
        );
    }
    let page: Value = assert_json(app.get("/devices?page=2&per_page=1").await, 200).await;
    assert_eq!(page["devices"].as_array().unwrap().len(), 1);
    assert_eq!(page["devices"][0]["id_label"], 2);
}

#[tokio::test]
async fn unsupported_inspection_payloads_are_not_saved_as_unchecked_pdns() {
    use prost::Message;
    let app = spawn_app().await;
    let registration = device(1);
    assert_eq!(
        app.post_protobuf("/devices", &registration).await.status(),
        204
    );
    let path = format!("{}/inspections", device_path(&registration));
    let mut report = inspection(0);
    report.checks = None;
    // Simulate a future oneof variant: field 6, length-delimited empty payload.
    // Prost skips unknown fields, so the handler must require a recognized variant.
    let mut encoded = report.encode_to_vec();
    encoded.extend_from_slice(&[0x32, 0x00]);
    let response = app
        .request(reqwest::Method::POST, &path)
        .header("Content-Type", "application/protobuf")
        .body(encoded)
        .send()
        .await
        .unwrap();
    assert_text_error(response, 400).await;
    let history: Value = assert_json(app.get(&path).await, 200).await;
    assert_eq!(history["inspections"], json!([]));
}

#[tokio::test]
async fn inspection_header_and_typed_checks_are_saved_atomically() {
    let app = spawn_app().await;
    let registration = device(1);
    assert_eq!(
        app.post_protobuf("/devices", &registration).await.status(),
        204
    );
    let path = format!("{}/inspections", device_path(&registration));
    sqlx::query("ALTER TABLE pdn_inspection_checks ADD CONSTRAINT test_reject_failed_screen CHECK (screen IS DISTINCT FROM false)")
        .execute(&app.db_pool).await.unwrap();
    let mut report = inspection(0);
    let Checks::Pdn(checks) = report.checks.as_mut().unwrap();
    checks.screen = Some(false);
    let message = assert_text_error(app.post_protobuf(&path, &report).await, 500).await;
    assert!(!message.contains("test_reject_failed_screen"));
    let headers: i64 = sqlx::query_scalar("SELECT count(*) FROM device_inspections")
        .fetch_one(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(headers, 0);
    let details: i64 = sqlx::query_scalar("SELECT count(*) FROM pdn_inspection_checks")
        .fetch_one(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(details, 0);
    sqlx::query("ALTER TABLE pdn_inspection_checks DROP CONSTRAINT test_reject_failed_screen")
        .execute(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(app.post_protobuf(&path, &report).await.status(), 204);
    let history: Value = assert_json(app.get(&path).await, 200).await;
    assert_eq!(history["inspections"].as_array().unwrap().len(), 1);
    assert_eq!(
        history["inspections"][0]["checks"]["components"]["screen"],
        false
    );
}

fn device_report(label: u32) -> WriteDeviceReportRequest {
    WriteDeviceReportRequest {
        device: Some(device(label)),
        inspection: Some(inspection(0)),
    }
}

#[tokio::test]
async fn device_reports_register_once_and_append_inspections_without_a_registration_step() {
    let app = spawn_app().await;
    let mut report = device_report(42);
    let (first, retry) = tokio::join!(
        app.post_protobuf("/device-reports", &report),
        app.post_protobuf("/device-reports", &report)
    );
    assert_eq!(first.status(), 204);
    assert_eq!(retry.status(), 204);
    let path = device_path(report.device.as_ref().unwrap());
    let original: Value = assert_json(app.get(&path).await, 200).await;
    app.activate_new_event().await;
    report.inspection = Some(inspection(1000));
    report.inspection.as_mut().unwrap().firmware_commit_hash = Some("new-build".into());
    assert_eq!(
        app.post_protobuf("/device-reports", &report).await.status(),
        204
    );
    let current: Value = assert_json(app.get(&path).await, 200).await;
    assert_eq!(current["id"], original["id"]);
    assert_eq!(current["created_at"], original["created_at"]);
    assert_eq!(
        current["latest_inspection"]["firmware_commit_hash"],
        "new-build"
    );
    let history: Value = assert_json(app.get(&format!("{path}/inspections")).await, 200).await;
    assert_eq!(history["inspections"].as_array().unwrap().len(), 2);
    assert_eq!(history["inspections"][1], original["latest_inspection"]);
    let inventory: Value = assert_json(app.get("/devices").await, 200).await;
    assert_eq!(inventory["devices"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn conflicting_device_reports_preserve_inventory_and_roll_back_new_registration() {
    let app = spawn_app().await;
    let original_report = device_report(1);
    assert_eq!(
        app.post_protobuf("/device-reports", &original_report)
            .await
            .status(),
        204
    );
    let original: Value = assert_json(app.get("/devices").await, 200).await;
    let mut conflicts = Vec::new();
    let mut changed = original_report.clone();
    changed.device.as_mut().unwrap().id_label = 2;
    conflicts.push(changed);
    let mut changed = original_report.clone();
    changed.inspection.as_mut().unwrap().notes = Some("Changed report".into());
    conflicts.push(changed);
    // A new device with a reused inspection ID must not remain registered.
    let mut changed = original_report.clone();
    changed.device = Some(device(2));
    let rejected_device_path = device_path(changed.device.as_ref().unwrap());
    conflicts.push(changed);
    for report in conflicts {
        assert_text_error(app.post_protobuf("/device-reports", &report).await, 409).await;
    }
    assert_text_error(app.get(&rejected_device_path).await, 404).await;
    let after: Value = assert_json(app.get("/devices").await, 200).await;
    assert_eq!(after, original);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM device_inspections")
        .fetch_one(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn incomplete_or_invalid_device_reports_do_not_register_devices() {
    let app = spawn_app().await;
    let valid = device_report(1);
    let mut invalid = vec![
        WriteDeviceReportRequest {
            device: None,
            ..valid.clone()
        },
        WriteDeviceReportRequest {
            inspection: None,
            ..valid.clone()
        },
    ];
    let mut bad_device = valid.clone();
    bad_device.device.as_mut().unwrap().id_label = 0;
    invalid.push(bad_device);
    let mut bad_inspection = valid.clone();
    bad_inspection.inspection.as_mut().unwrap().checks = None;
    invalid.push(bad_inspection);
    for report in invalid {
        assert_text_error(app.post_protobuf("/device-reports", &report).await, 400).await;
    }
    assert_text_error(
        app.request_json(reqwest::Method::POST, "/device-reports", json!({}))
            .await,
        415,
    )
    .await;
    let inventory: Value = assert_json(app.get("/devices").await, 200).await;
    assert_eq!(inventory["devices"], json!([]));
}

#[tokio::test]
async fn device_report_storage_failure_rolls_back_registration_and_inspection() {
    let app = spawn_app().await;
    let mut report = device_report(1);
    let Checks::Pdn(checks) = report.inspection.as_mut().unwrap().checks.as_mut().unwrap();
    checks.screen = Some(false);
    sqlx::query("ALTER TABLE pdn_inspection_checks ADD CONSTRAINT test_reject_report CHECK (screen IS DISTINCT FROM false)")
        .execute(&app.db_pool).await.unwrap();
    assert_text_error(app.post_protobuf("/device-reports", &report).await, 500).await;
    let counts: (i64, i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM devices), (SELECT count(*) FROM device_inspections), (SELECT count(*) FROM pdn_inspection_checks)")
        .fetch_one(&app.db_pool).await.unwrap();
    assert_eq!(counts, (0, 0, 0));
    sqlx::query("ALTER TABLE pdn_inspection_checks DROP CONSTRAINT test_reject_report")
        .execute(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(
        app.post_protobuf("/device-reports", &report).await.status(),
        204
    );
    let current: Value = assert_json(
        app.get(&device_path(report.device.as_ref().unwrap())).await,
        200,
    )
    .await;
    assert_eq!(
        current["latest_inspection"]["checks"]["components"]["screen"],
        false
    );
}
