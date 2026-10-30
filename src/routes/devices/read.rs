use super::{DeviceRecord, Inspection, InspectionRow};
use crate::routes::helpers::errors::database_error;
use crate::routes::helpers::pagination::{Pagination, PaginationQuery};
use actix_web::{error, get, web};
use serde::Serialize;
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Serialize, utoipa::ToSchema)]
pub struct DeviceResponse {
    #[serde(flatten)]
    device: DeviceRecord,
    /// Most recent check by inspection time; null when the device has never been checked.
    latest_inspection: Option<Inspection>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct DeviceList {
    devices: Vec<DeviceResponse>,
    pagination: Pagination,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct InspectionList {
    inspections: Vec<Inspection>,
    pagination: Pagination,
}

async fn latest_inspections(
    pool: &PgPool,
    ids: &[Uuid],
) -> actix_web::Result<HashMap<Uuid, Inspection>> {
    let rows = sqlx::query_as!(InspectionRow,
        "SELECT DISTINCT ON (i.device_id) i.id, i.device_id, i.inspected_at, i.received_at, i.firmware_commit_hash, i.notes, i.powers_on, p.screen, p.primary_button, p.secondary_button, p.haptics, p.leds, p.power_switch, p.input_jack, p.output_jack, p.housing
         FROM device_inspections i JOIN pdn_inspection_checks p ON p.inspection_id = i.id WHERE i.device_id = ANY($1)
         ORDER BY i.device_id, i.inspected_at DESC, i.received_at DESC, i.id DESC", ids
    ).fetch_all(pool).await.map_err(|err| database_error(err, "Failed to read device inspections"))?;
    Ok(rows
        .into_iter()
        .map(|row| (row.device_id, row.into()))
        .collect())
}

#[utoipa::path(
    tag = "Device inventory", summary = "List devices with their latest condition checks",
    description = "Inventory across events, ordered by id_label ascending; no active event required. Latest inspection is selected by inspected_at, then received_at and inspection UUID, all descending. Check age and unknown components remain visible; no readiness verdict is inferred.",
    params(PaginationQuery),
    responses((status = 200, description = "Device inventory page", body = DeviceList),
        (status = 400, description = "Invalid pagination"),
        (status = 500, description = "Database operation failed"))
)]
#[get("/devices")]
pub async fn list_devices(
    parameters: web::Query<PaginationQuery>,
    pool: web::Data<PgPool>,
) -> actix_web::Result<web::Json<DeviceList>> {
    let pagination = parameters.normalize();
    let (limit, offset) = pagination.limit_offset();
    let rows = sqlx::query_as!(DeviceRecord,
        "SELECT id, id_label, device_kind, hardware_version, mac_address::text AS mac_address, created_at
         FROM devices ORDER BY id_label LIMIT $1 OFFSET $2", limit, offset
    ).fetch_all(pool.get_ref()).await.map_err(|err| database_error(err, "Failed to read devices"))?;
    let ids: Vec<_> = rows.iter().map(|device| device.id).collect();
    let mut latest = latest_inspections(pool.get_ref(), &ids).await?;
    let devices = rows
        .into_iter()
        .map(|device| DeviceResponse {
            latest_inspection: latest.remove(&device.id),
            device,
        })
        .collect();
    Ok(web::Json(DeviceList {
        devices,
        pagination,
    }))
}

#[utoipa::path(
    tag = "Device inventory", summary = "Get a device and its latest condition check",
    description = "Returns one device using the same latest-inspection ordering as GET /devices. No active event required.",
    params(("device_id" = Uuid, Path, description = "Permanent device UUID")),
    responses((status = 200, description = "Device with latest inspection", body = DeviceResponse),
        (status = 404, description = "Device not found or invalid path UUID"),
        (status = 500, description = "Database operation failed"))
)]
#[get("/devices/{device_id}")]
pub async fn get_device(
    device_id: web::Path<Uuid>,
    pool: web::Data<PgPool>,
) -> actix_web::Result<web::Json<DeviceResponse>> {
    let device = sqlx::query_as!(DeviceRecord,
        "SELECT id, id_label, device_kind, hardware_version, mac_address::text AS mac_address, created_at FROM devices WHERE id = $1", *device_id
    ).fetch_optional(pool.get_ref()).await.map_err(|err| database_error(err, "Failed to read device"))?
        .ok_or_else(|| error::ErrorNotFound("Device not found"))?;
    let latest_inspection = latest_inspections(pool.get_ref(), &[*device_id])
        .await?
        .remove(&device_id);
    Ok(web::Json(DeviceResponse {
        device,
        latest_inspection,
    }))
}

#[utoipa::path(
    tag = "Device inventory", summary = "List a device's inspection history",
    description = "Complete snapshots ordered by inspected_at, received_at, then inspection UUID, all descending. Existing devices without inspections return an empty page. No active event required.",
    params(("device_id" = Uuid, Path, description = "Permanent device UUID"), PaginationQuery),
    responses((status = 200, description = "Inspection history page", body = InspectionList),
        (status = 400, description = "Invalid pagination"),
        (status = 404, description = "Device not found or invalid path UUID"),
        (status = 500, description = "Database operation failed"))
)]
#[get("/devices/{device_id}/inspections")]
pub async fn list_device_inspections(
    device_id: web::Path<Uuid>,
    parameters: web::Query<PaginationQuery>,
    pool: web::Data<PgPool>,
) -> actix_web::Result<web::Json<InspectionList>> {
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM devices WHERE id = $1) AS \"exists!\"",
        *device_id
    )
    .fetch_one(pool.get_ref())
    .await
    .map_err(|err| database_error(err, "Failed to read device"))?;
    if !exists {
        return Err(error::ErrorNotFound("Device not found"));
    }
    let pagination = parameters.normalize();
    let (limit, offset) = pagination.limit_offset();
    let inspections = sqlx::query_as!(InspectionRow,
        "SELECT i.id, i.device_id, i.inspected_at, i.received_at, i.firmware_commit_hash, i.notes, i.powers_on, p.screen, p.primary_button, p.secondary_button, p.haptics, p.leds, p.power_switch, p.input_jack, p.output_jack, p.housing FROM device_inspections i JOIN pdn_inspection_checks p ON p.inspection_id = i.id WHERE i.device_id = $1
         ORDER BY i.inspected_at DESC, i.received_at DESC, i.id DESC LIMIT $2 OFFSET $3", *device_id, limit, offset
    ).fetch_all(pool.get_ref()).await.map_err(|err| database_error(err, "Failed to read device inspections"))?
        .into_iter().map(Inspection::from).collect();
    Ok(web::Json(InspectionList {
        inspections,
        pagination,
    }))
}
