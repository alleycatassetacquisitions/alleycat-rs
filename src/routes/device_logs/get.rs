use crate::routes::errors::database_error;
use crate::routes::pagination::{Pagination, PaginationQuery};
use actix_web::get;
use actix_web::web;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

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
pub struct GetDeviceLogsResponse {
    device_logs: Vec<DeviceLogResponse>,
    pagination: Pagination,
}

#[utoipa::path(
    tag = "Device logs", summary = "List device crash reports",
    description = "Returns reports across all devices, newest first.",
    params(PaginationQuery),
    responses((status = 200, description = "Device logs and effective pagination", body = GetDeviceLogsResponse),
        (status = 400, description = "Invalid pagination query"),
        (status = 500, description = "Database operation failed", body = String, content_type = "text/plain"))
)]
#[get("/device-logs")]
#[tracing::instrument(name = "Retrieve device logs", skip(parameters, pool))]
pub async fn get_device_logs(
    parameters: web::Query<PaginationQuery>,
    pool: web::Data<PgPool>,
) -> actix_web::Result<web::Json<GetDeviceLogsResponse>> {
    let pagination = parameters.normalize();

    let device_logs = fetch_device_logs(&pool, &pagination)
        .await
        .map_err(|error| database_error(error, "Failed to retrieve device logs"))?;

    Ok(web::Json(GetDeviceLogsResponse {
        device_logs,
        pagination,
    }))
}

#[tracing::instrument(name = "Fetch device logs", skip(pool))]
async fn fetch_device_logs(
    pool: &PgPool,
    pagination: &Pagination,
) -> Result<Vec<DeviceLogResponse>, sqlx::Error> {
    let (limit, offset) = pagination.limit_offset();

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
