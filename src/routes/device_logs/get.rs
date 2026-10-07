use actix_web::get;
use actix_web::{error::ErrorInternalServerError, web};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Deserialize, utoipa::ToSchema)]
pub struct GetDeviceLogsQuery {
    page: Option<u32>,
    per_page: Option<u32>,
}

#[derive(Serialize, sqlx::FromRow, utoipa::ToSchema)]
struct DeviceLogResponse {
    id: Uuid,
    device_mac: String,
    crash_number: i64,
    uptime_ms: i64,
    received_at: DateTime<Utc>,
    software_version: Option<String>,
    reset_reason: i32,
    program_counter: Option<i64>,
    exception_cause: Option<i64>,
    task_name: Option<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
struct DeviceLogPagination {
    page: u32,
    per_page: u32,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct GetDeviceLogsResponse {
    device_logs: Vec<DeviceLogResponse>,
    pagination: DeviceLogPagination,
}

#[utoipa::path(
    tag = "Device logs", summary = "List device crash reports",
    description = "Returns reports across all devices, newest first.",
    params(
        ("page" = Option<u32>, Query, description = "Page number; defaults to 1. Zero is treated as 1."),
        ("per_page" = Option<u32>, Query, description = "Page size; defaults to 20 and is clamped to 1–100.")
    ),
    responses((status = 200, description = "Device logs and effective pagination", body = GetDeviceLogsResponse),
        (status = 400, description = "Invalid pagination query"),
        (status = 500, description = "Database operation failed"))
)]
#[get("/device-logs")]
#[tracing::instrument(name = "Retrieve device logs", skip(parameters, pool))]
pub async fn get_device_logs(
    parameters: web::Query<GetDeviceLogsQuery>,
    pool: web::Data<PgPool>,
) -> actix_web::Result<web::Json<GetDeviceLogsResponse>> {
    const DEFAULT_PAGE: u32 = 1;
    const DEFAULT_PER_PAGE: u32 = 20;
    const MAX_PER_PAGE: u32 = 100;

    let page = parameters.page.unwrap_or(DEFAULT_PAGE).max(1);
    let per_page = parameters
        .per_page
        .unwrap_or(DEFAULT_PER_PAGE)
        .clamp(1, MAX_PER_PAGE);

    let device_logs = fetch_device_logs(&pool, page, per_page)
        .await
        .map_err(|error| {
            tracing::error!(?error, "Failed to retrieve device logs");
            ErrorInternalServerError("Failed to retrieve device logs")
        })?;

    Ok(web::Json(GetDeviceLogsResponse {
        device_logs,
        pagination: DeviceLogPagination { page, per_page },
    }))
}

#[tracing::instrument(name = "Fetch device logs", skip(pool))]
async fn fetch_device_logs(
    pool: &PgPool,
    page: u32,
    per_page: u32,
) -> Result<Vec<DeviceLogResponse>, sqlx::Error> {
    let limit = i64::from(per_page);
    let offset = i64::from(page - 1) * limit;

    sqlx::query_as::<_, DeviceLogResponse>(
        r#"
        SELECT
            id,
            device_mac::text AS device_mac,
            crash_number,
            uptime_ms,
            received_at,
            software_version,
            reset_reason,
            program_counter,
            exception_cause,
            task_name
        FROM device_logs
        ORDER BY received_at DESC, id DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
}
