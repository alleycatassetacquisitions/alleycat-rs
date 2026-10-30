use crate::proto::alleycat::device::{
    CreateDeviceInspectionRequest, CreateDeviceRequest, DeviceKind, PdnComponentChecks,
    WriteDeviceReportRequest, create_device_inspection_request::Checks,
};
use crate::routes::helpers::errors::database_error;
use crate::routes::helpers::protobuf::{self, ProtobufBody};
use actix_web::{HttpRequest, HttpResponse, error, post, web};
use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool, types::mac_address::MacAddress};
use uuid::Uuid;

fn record_id(bytes: &[u8]) -> actix_web::Result<Uuid> {
    Uuid::from_slice(bytes)
        .ok()
        .filter(|id| !id.is_nil())
        .ok_or_else(|| error::ErrorBadRequest("id must be a non-nil 16-byte UUID"))
}

fn validate_text(
    value: Option<&str>,
    field: &str,
    limit: usize,
    nonblank: bool,
) -> actix_web::Result<()> {
    if value.is_some_and(|text| {
        text.len() > limit || text.contains('\0') || (nonblank && text.trim().is_empty())
    }) {
        return Err(error::ErrorBadRequest(format!(
            "{field} must fit {limit} UTF-8 bytes, contain no null characters{}",
            if nonblank { ", and be nonblank" } else { "" }
        )));
    }
    Ok(())
}

fn write_error(err: sqlx::Error) -> actix_web::Error {
    if let Some(db) = err.as_database_error() {
        if db.is_unique_violation() {
            return error::ErrorConflict("Device ID, label, or MAC address is already in use");
        }
        if db.is_foreign_key_violation() {
            return error::ErrorNotFound("Device not found");
        }
    }
    database_error(err, "Failed to save device inventory record")
}

struct DeviceRegistration {
    id: Uuid,
    label: i32,
    hardware: Option<i32>,
    mac: Option<MacAddress>,
}

impl TryFrom<CreateDeviceRequest> for DeviceRegistration {
    type Error = actix_web::Error;

    fn try_from(value: CreateDeviceRequest) -> Result<Self, Self::Error> {
        if value.device_kind != DeviceKind::Pdn as i32 {
            return Err(error::ErrorBadRequest("device_kind must be PDN"));
        }
        let id = record_id(&value.id)?;
        if !(1..=9999).contains(&value.id_label) {
            return Err(error::ErrorBadRequest(
                "id_label must be between 1 and 9999",
            ));
        }
        let label = value.id_label as i32;
        let hardware = value
            .hardware_version
            .map(|v| {
                i32::try_from(v).ok().filter(|v| *v > 0).ok_or_else(|| {
                    error::ErrorBadRequest("hardware_version must be between 1 and 2147483647")
                })
            })
            .transpose()?;
        let mac: Option<MacAddress> = value
            .mac_address
            .as_deref()
            .map(|text| {
                if text.len() != 17
                    || !text
                        .split(':')
                        .all(|pair| pair.len() == 2 && pair.bytes().all(|b| b.is_ascii_hexdigit()))
                {
                    return Err(error::ErrorBadRequest(
                        "mac_address must contain six colon-separated hexadecimal byte pairs",
                    ));
                }
                text.parse()
                    .map_err(|_| error::ErrorBadRequest("Invalid mac_address"))
            })
            .transpose()?;
        Ok(Self {
            id,
            label,
            hardware,
            mac,
        })
    }
}

struct NewInspection {
    id: Uuid,
    inspected_at: DateTime<Utc>,
    firmware_commit_hash: Option<String>,
    notes: Option<String>,
    powers_on: Option<bool>,
    checks: PdnComponentChecks,
}

impl TryFrom<CreateDeviceInspectionRequest> for NewInspection {
    type Error = actix_web::Error;

    fn try_from(value: CreateDeviceInspectionRequest) -> Result<Self, Self::Error> {
        let id = record_id(&value.id)?;
        let ms = value
            .inspected_at_unix_ms
            .ok_or_else(|| error::ErrorBadRequest("inspected_at_unix_ms is required"))?;
        if !(0..=253_402_300_799_999).contains(&ms) || ms > Utc::now().timestamp_millis() {
            return Err(error::ErrorBadRequest(
                "inspected_at_unix_ms must be between the Unix epoch and the current time",
            ));
        }
        let inspected_at: DateTime<Utc> =
            DateTime::from_timestamp_millis(ms).expect("validated timestamp range");
        validate_text(
            value.firmware_commit_hash.as_deref(),
            "firmware_commit_hash",
            64,
            true,
        )?;
        validate_text(value.notes.as_deref(), "notes", 2048, false)?;
        let Some(Checks::Pdn(checks)) = value.checks else {
            return Err(error::ErrorBadRequest(
                "A supported inspection payload is required (pdn)",
            ));
        };
        Ok(Self {
            id,
            inspected_at,
            checks,
            firmware_commit_hash: value.firmware_commit_hash,
            notes: value.notes,
            powers_on: value.powers_on,
        })
    }
}

async fn ensure_device(
    connection: &mut PgConnection,
    device: &DeviceRegistration,
) -> actix_web::Result<()> {
    let DeviceRegistration {
        id,
        label,
        hardware,
        mac,
    } = *device;
    // Concurrent identical inserts can collide on any unique index. Check the
    // saved fields below to distinguish retries from conflicting registrations.
    let inserted = sqlx::query_scalar!(
        "INSERT INTO devices (id, id_label, hardware_version, mac_address) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING RETURNING id",
        id, label, hardware, mac
    ).fetch_optional(&mut *connection).await.map_err(write_error)?;
    if inserted.is_none() {
        let identical = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM devices WHERE id = $1 AND device_kind = 'pdn' AND id_label = $2 AND hardware_version IS NOT DISTINCT FROM $3 AND mac_address IS NOT DISTINCT FROM $4) AS \"identical!\"",
            id, label, hardware, mac
        ).fetch_one(&mut *connection).await.map_err(write_error)?;
        if !identical {
            return Err(error::ErrorConflict(
                "Device ID already has different registration fields",
            ));
        }
    }
    Ok(())
}

async fn require_pdn(connection: &mut PgConnection, device_id: Uuid) -> actix_web::Result<()> {
    let kind = sqlx::query_scalar!("SELECT device_kind FROM devices WHERE id = $1", device_id)
        .fetch_optional(&mut *connection)
        .await
        .map_err(write_error)?
        .ok_or_else(|| error::ErrorNotFound("Device not found"))?;
    if kind != "pdn" {
        return Err(error::ErrorBadRequest(
            "Inspection payload does not match device kind",
        ));
    }
    Ok(())
}

async fn save_inspection(
    connection: &mut PgConnection,
    device_id: Uuid,
    value: &NewInspection,
) -> actix_web::Result<()> {
    let id = value.id;
    let inspected_at = value.inspected_at;
    let checks = &value.checks;
    // The (id, device_kind) unique index must also accept concurrent retries.
    let inserted = sqlx::query_scalar!(
        "INSERT INTO device_inspections (id, device_id, device_kind, inspected_at, firmware_commit_hash, notes, powers_on)
         VALUES ($1, $2, 'pdn', $3, $4, $5, $6) ON CONFLICT DO NOTHING RETURNING id",
        id, device_id, inspected_at, value.firmware_commit_hash.as_deref(), value.notes.as_deref(), value.powers_on
    ).fetch_optional(&mut *connection).await.map_err(write_error)?;
    if inserted.is_some() {
        sqlx::query!(
            "INSERT INTO pdn_inspection_checks (inspection_id, screen, primary_button, secondary_button, haptics, leds, power_switch, input_jack, output_jack, housing)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            id, checks.screen, checks.primary_button, checks.secondary_button, checks.haptics,
            checks.leds, checks.power_switch, checks.input_jack, checks.output_jack, checks.housing
        ).execute(&mut *connection).await.map_err(write_error)?;
    }
    if inserted.is_none() {
        let identical = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM device_inspections i JOIN pdn_inspection_checks p ON p.inspection_id = i.id WHERE i.id = $1 AND i.device_id = $2 AND i.device_kind = 'pdn' AND i.inspected_at = $3
             AND firmware_commit_hash IS NOT DISTINCT FROM $4 AND notes IS NOT DISTINCT FROM $5
             AND screen IS NOT DISTINCT FROM $6
             AND primary_button IS NOT DISTINCT FROM $7
             AND secondary_button IS NOT DISTINCT FROM $8
             AND haptics IS NOT DISTINCT FROM $9
             AND leds IS NOT DISTINCT FROM $10
             AND power_switch IS NOT DISTINCT FROM $11
             AND input_jack IS NOT DISTINCT FROM $12
             AND output_jack IS NOT DISTINCT FROM $13
             AND housing IS NOT DISTINCT FROM $14
             AND powers_on IS NOT DISTINCT FROM $15) AS \"identical!\"",
            id, device_id, inspected_at, value.firmware_commit_hash.as_deref(), value.notes.as_deref(),
            checks.screen, checks.primary_button, checks.secondary_button, checks.haptics, checks.leds, checks.power_switch, checks.input_jack, checks.output_jack, checks.housing, value.powers_on
        ).fetch_one(&mut *connection).await.map_err(write_error)?;
        if !identical {
            return Err(error::ErrorConflict(
                "Inspection ID already has different fields or device",
            ));
        }
    }
    Ok(())
}

#[utoipa::path(
    tag = "Device inventory", summary = "Register a physical device (protobuf)",
    description = "Register a device without an inspection. No active event is required. Send [CreateDeviceRequest](/docs/device-apis#alleycat.device.CreateDeviceRequest); identical normalized registrations are safe to retry. Firmware should use POST /device-reports to register and inspect together.",
    request_body(content = inline(ProtobufBody), content_type = "application/protobuf"),
    responses((status = 204, description = "Device saved or identical registration already exists; empty body"),
        (status = 400, description = "Invalid protobuf or fields"),
        (status = 409, description = "ID has different registration fields, or label/MAC is already in use"),
        (status = 413, description = "Body exceeds 256 KiB"),
        (status = 415, description = "Expected application/protobuf"),
        (status = 500, description = "Database operation failed"))
)]
#[post("/devices")]
pub async fn create_device(
    request: HttpRequest,
    body: web::Bytes,
    pool: web::Data<PgPool>,
) -> actix_web::Result<HttpResponse> {
    let value = protobuf::decode::<CreateDeviceRequest>(&request, &body)?;
    let device = DeviceRegistration::try_from(value)?;
    let mut connection = pool.acquire().await.map_err(write_error)?;
    ensure_device(&mut connection, &device).await?;
    Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(
    tag = "Device inventory", summary = "Record a device inspection (protobuf)",
    description = "Save [CreateDeviceInspectionRequest](/docs/device-apis#alleycat.device.CreateDeviceInspectionRequest) for an existing device. No active event is required. Identical retries preserve the original snapshot and receipt time. Firmware can use POST /device-reports without registering first.",
    params(("device_id" = Uuid, Path, description = "Permanent device UUID")),
    request_body(content = inline(ProtobufBody), content_type = "application/protobuf"),
    responses((status = 204, description = "Inspection saved or identical inspection already exists; empty body"),
        (status = 400, description = "Invalid protobuf or fields"),
        (status = 404, description = "Device not found or invalid path UUID"),
        (status = 409, description = "Inspection ID already has different fields or device"),
        (status = 413, description = "Body exceeds 256 KiB"),
        (status = 415, description = "Expected application/protobuf"),
        (status = 500, description = "Database operation failed"))
)]
#[post("/devices/{device_id}/inspections")]
pub async fn create_device_inspection(
    device_id: web::Path<Uuid>,
    request: HttpRequest,
    body: web::Bytes,
    pool: web::Data<PgPool>,
) -> actix_web::Result<HttpResponse> {
    let value = protobuf::decode::<CreateDeviceInspectionRequest>(&request, &body)?;
    let inspection = NewInspection::try_from(value)?;
    let mut tx = pool.begin().await.map_err(write_error)?;
    require_pdn(&mut tx, *device_id).await?;
    save_inspection(&mut tx, *device_id, &inspection).await?;
    tx.commit().await.map_err(write_error)?;
    Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(
    tag = "Device inventory", summary = "Register if needed and record an inspection (protobuf)",
    description = r##"Upload [WriteDeviceReportRequest](/docs/device-apis#alleycat.device.WriteDeviceReportRequest) as binary protobuf with `Content-Type: application/protobuf`. No registration check or active event is needed. Registration and inspection commit together; errors save nothing.

Keep one device UUID and stable registration details. Generate a new inspection UUID for each check, and retain the complete report until acknowledged. On connection failure or 5xx, retry the same payload with backoff. Successful retries preserve the original records and receipt times.

See the message reference for field rules. Errors are plain text. Separate administrative writes use POST /devices and POST /devices/{device_id}/inspections."##,
    request_body(content = inline(ProtobufBody), content_type = "application/protobuf"),
    responses((status = 204, description = "Saved or already saved; empty body. Stop retrying this report"),
        (status = 400, description = "Invalid or incomplete message; correct it before retrying"),
        (status = 409, description = "Conflicting identity, label, MAC, registration fields, or inspection contents; resolve before retrying"),
        (status = 413, description = "Body exceeds 256 KiB"),
        (status = 415, description = "Expected application/protobuf"),
        (status = 500, description = "Database operation failed; no partial report saved"))
)]
#[post("/device-reports")]
pub async fn write_device_report(
    request: HttpRequest,
    body: web::Bytes,
    pool: web::Data<PgPool>,
) -> actix_web::Result<HttpResponse> {
    let report = protobuf::decode::<WriteDeviceReportRequest>(&request, &body)?;
    let device = DeviceRegistration::try_from(
        report
            .device
            .ok_or_else(|| error::ErrorBadRequest("device is required"))?,
    )?;
    let inspection = NewInspection::try_from(
        report
            .inspection
            .ok_or_else(|| error::ErrorBadRequest("inspection is required"))?,
    )?;
    let mut tx = pool.begin().await.map_err(write_error)?;
    ensure_device(&mut tx, &device).await?;
    save_inspection(&mut tx, device.id, &inspection).await?;
    tx.commit().await.map_err(write_error)?;
    Ok(HttpResponse::NoContent().finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn record_ids_require_non_nil_network_order_uuid_bytes() {
        for bytes in [vec![], vec![0; 16], vec![1; 15], vec![1; 17]] {
            assert_eq!(
                record_id(&bytes)
                    .unwrap_err()
                    .as_response_error()
                    .status_code(),
                400
            );
        }
        let id = Uuid::parse_str("00112233-4455-6677-8899-aabbccddeeff").unwrap();
        assert_eq!(record_id(id.as_bytes()).unwrap(), id);
    }

    #[test]
    fn registration_validates_field_limits_and_formats() {
        let valid = CreateDeviceRequest {
            id: Uuid::new_v4().as_bytes().to_vec(),
            id_label: 1,
            device_kind: DeviceKind::Pdn as i32,
            ..Default::default()
        };
        let mut invalid = Vec::new();
        for device_kind in [0, 2, -1, i32::MAX] {
            invalid.push(CreateDeviceRequest {
                device_kind,
                ..valid.clone()
            });
        }
        for id_label in [0, 10000, u32::MAX] {
            invalid.push(CreateDeviceRequest {
                id_label,
                ..valid.clone()
            });
        }
        for hardware_version in [0, i32::MAX as u32 + 1] {
            invalid.push(CreateDeviceRequest {
                hardware_version: Some(hardware_version),
                ..valid.clone()
            });
        }
        for mac in [
            "",
            "aa:bb:cc:dd",
            "aa-bb-cc-dd-ee-ff",
            "xx:bb:cc:dd:ee:ff",
            " aa:bb:cc:dd:ee:ff",
        ] {
            invalid.push(CreateDeviceRequest {
                mac_address: Some(mac.into()),
                ..valid.clone()
            });
        }
        for value in invalid {
            assert!(DeviceRegistration::try_from(value).is_err());
        }
        assert!(DeviceRegistration::try_from(valid.clone()).is_ok());
        assert!(
            DeviceRegistration::try_from(CreateDeviceRequest {
                id_label: 9999,
                hardware_version: Some(i32::MAX as u32),
                mac_address: Some("AA:BB:CC:DD:EE:FF".into()),
                ..valid
            })
            .is_ok()
        );
    }

    #[test]
    fn inspection_validates_required_fields_and_text_limits() {
        let valid = CreateDeviceInspectionRequest {
            id: Uuid::new_v4().as_bytes().to_vec(),
            inspected_at_unix_ms: Some(0),
            checks: Some(Checks::Pdn(PdnComponentChecks::default())),
            ..Default::default()
        };
        let mut invalid = Vec::new();
        for time in [
            None,
            Some(-1),
            Some(i64::MAX),
            Some(chrono::Utc::now().timestamp_millis() + 60_000),
        ] {
            invalid.push(CreateDeviceInspectionRequest {
                inspected_at_unix_ms: time,
                ..valid.clone()
            });
        }
        for text in ["".into(), "  ".into(), "a\0b".into(), "é".repeat(33)] {
            invalid.push(CreateDeviceInspectionRequest {
                firmware_commit_hash: Some(text),
                ..valid.clone()
            });
        }
        for text in ["a\0b".into(), "é".repeat(1025)] {
            invalid.push(CreateDeviceInspectionRequest {
                notes: Some(text),
                ..valid.clone()
            });
        }
        invalid.push(CreateDeviceInspectionRequest {
            checks: None,
            ..valid.clone()
        });
        for value in invalid {
            assert!(NewInspection::try_from(value).is_err());
        }
        assert!(NewInspection::try_from(valid.clone()).is_ok());
        assert!(
            NewInspection::try_from(CreateDeviceInspectionRequest {
                firmware_commit_hash: Some("a".repeat(64)),
                notes: Some("é".repeat(1024)),
                ..valid
            })
            .is_ok()
        );
    }
}
