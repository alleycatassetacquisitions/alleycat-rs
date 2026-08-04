use crate::proto::alleycat::device::WriteDeviceLogRequest;
use actix_web::http::header::CONTENT_TYPE;
use actix_web::{HttpRequest, HttpResponse, error, web};
use prost::Message;
use sqlx::PgPool;

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
        .map_err(|error| {
            tracing::error!(?error, "Failed to insert device log");
            error::ErrorInternalServerError("Failed to insert device log")
        })?;

    Ok(HttpResponse::NoContent().finish())
}

#[tracing::instrument(name = "Insert device log", skip(pool, device_log))]
async fn insert_device_log(pool: &PgPool, device_log: &NewDeviceLog) -> Result<(), sqlx::Error> {
    sqlx::query(
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
    )
    .bind(device_log.device_mac)
    .bind(device_log.crash_number)
    .bind(device_log.uptime_ms)
    .bind(device_log.software_version.as_deref())
    .bind(device_log.reset_reason)
    .bind(device_log.program_counter)
    .bind(device_log.exception_cause)
    .bind(device_log.task_name.as_deref())
    .execute(pool)
    .await?;

    Ok(())
}
