use crate::proto::alleycat::device::WriteDeviceLogRequest;
use crate::routes::errors::database_error;
use actix_web::http::header::CONTENT_TYPE;
use actix_web::post;
use actix_web::{HttpRequest, HttpResponse, error, web};
use prost::Message;
use sqlx::PgPool;

// Describe the wire payload as binary, rather than a JSON array of byte values.
struct ProtobufBody;

impl utoipa::PartialSchema for ProtobufBody {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        use utoipa::openapi::schema::{KnownFormat, ObjectBuilder, SchemaFormat, Type};
        ObjectBuilder::new()
            .schema_type(Type::String)
            .format(Some(SchemaFormat::KnownFormat(KnownFormat::Binary)))
            .into()
    }
}

impl utoipa::ToSchema for ProtobufBody {}

struct NewDeviceLog {
    device_mac: sqlx::types::mac_address::MacAddress,
    crash_number: i64,
    uptime_ms: i64,
    software_version: Option<String>,
    reset_reason: i32,
    program_counter: Option<i64>,
    exception_cause: Option<i64>,
    task_name: Option<String>,
}

impl TryFrom<WriteDeviceLogRequest> for NewDeviceLog {
    type Error = String;

    fn try_from(value: WriteDeviceLogRequest) -> Result<Self, Self::Error> {
        let device_mac = value
            .device_mac
            .ok_or("device_mac is required")?
            .parse()
            .map_err(|_| "device_mac is invalid")?;

        let crash_number = i64::try_from(value.crash_number.ok_or("crash_number is required")?)
            .map_err(|_| "crash_number is too large")?;

        let uptime_ms = i64::try_from(value.uptime_ms.ok_or("uptime_ms is required")?)
            .map_err(|_| "uptime_ms is too large")?;

        let reset_reason = i32::try_from(value.reset_reason.ok_or("reset_reason is required")?)
            .map_err(|_| "reset_reason is too large")?;

        Ok(Self {
            device_mac,
            crash_number,
            uptime_ms,
            software_version: value.software_version,
            reset_reason,
            program_counter: value.program_counter.map(i64::from),
            exception_cause: value.exception_cause.map(i64::from),
            task_name: value.task_name,
        })
    }
}

#[utoipa::path(
    tag = "Device logs", summary = "Submit a device crash report",
    description = "Send a binary alleycat.device.WriteDeviceLogRequest encoded using proto/device_api.proto. Required fields: device_mac (48-bit MAC address), crash_number, uptime_ms, reset_reason. crash_number and uptime_ms must fit a signed 64-bit integer; reset_reason must fit a signed 32-bit integer. Optional fields: software_version, program_counter, exception_cause, task_name. Repeated (device_mac, crash_number) reports are ignored and still return 204.",
    request_body(content = inline(ProtobufBody), content_type = "application/protobuf"),
    responses((status = 204, description = "Report accepted; empty body"),
        (status = 400, description = "Invalid protobuf or missing/invalid report fields"),
        (status = 415, description = "Expected application/protobuf"),
        (status = 500, description = "Database operation failed", body = String, content_type = "text/plain"))
)]
#[post("/device-logs")]
pub async fn write_device_log(
    http_request: HttpRequest,
    body: web::Bytes,
    pool: web::Data<PgPool>,
) -> actix_web::Result<HttpResponse> {
    let content_type = http_request
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(str::trim);

    if !content_type.is_some_and(|value| value.eq_ignore_ascii_case("application/protobuf")) {
        return Err(error::ErrorUnsupportedMediaType(
            "Expected application/protobuf",
        ));
    }

    let report = WriteDeviceLogRequest::decode(body.as_ref()).map_err(error::ErrorBadRequest)?;
    let new_device_log = NewDeviceLog::try_from(report).map_err(error::ErrorBadRequest)?;

    insert_device_log(&pool, &new_device_log)
        .await
        .map_err(|error| database_error(error, "Failed to insert device log"))?;

    Ok(HttpResponse::NoContent().finish())
}

#[tracing::instrument(name = "Insert device log", skip(pool, device_log))]
async fn insert_device_log(pool: &PgPool, device_log: &NewDeviceLog) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO device_logs (
            device_mac,
            crash_number,
            uptime_ms,
            software_version,
            reset_reason,
            program_counter,
            exception_cause,
            task_name
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (device_mac, crash_number) DO NOTHING
        "#,
        device_log.device_mac,
        device_log.crash_number,
        device_log.uptime_ms,
        device_log.software_version.as_deref(),
        device_log.reset_reason,
        device_log.program_counter,
        device_log.exception_cause,
        device_log.task_name.as_deref()
    )
    .execute(pool)
    .await?;

    Ok(())
}
