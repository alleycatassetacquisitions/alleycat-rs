use actix_web::{HttpResponse, error, web};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateEvent {
    name: String,
    venue_name: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateEvent {
    #[serde(default, deserialize_with = "present")]
    name: Option<String>,
    // Missing preserves the venue; explicit null clears it.
    #[serde(default, deserialize_with = "present")]
    venue_name: Option<Option<String>>,
}

fn present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

#[derive(Serialize, sqlx::FromRow)]
struct EventResponse {
    id: Uuid,
    name: String,
    venue_name: Option<String>,
    created_at: DateTime<Utc>,
}

fn validate_name(name: &str) -> actix_web::Result<()> {
    if name.trim().is_empty() || name.contains('\0') {
        return Err(error::ErrorBadRequest(
            "Event name must be nonblank and contain no null characters",
        ));
    }
    Ok(())
}

fn validate_venue(venue: Option<&str>) -> actix_web::Result<()> {
    if venue.is_some_and(|venue| venue.contains('\0')) {
        return Err(error::ErrorBadRequest(
            "Venue must contain no null characters",
        ));
    }
    Ok(())
}

fn database_error(error: sqlx::Error) -> actix_web::Error {
    tracing::error!(?error, "Event operation failed");
    error::ErrorInternalServerError("Event operation failed")
}

pub async fn create_event(
    body: web::Json<CreateEvent>,
    pool: web::Data<PgPool>,
) -> actix_web::Result<HttpResponse> {
    validate_name(&body.name)?;
    validate_venue(body.venue_name.as_deref())?;
    let event = sqlx::query_as::<_, EventResponse>(
        "INSERT INTO events (id, name, venue_name, created_at) VALUES ($1, $2, $3, $4)
         RETURNING id, name, venue_name, created_at",
    )
    .bind(Uuid::new_v4())
    .bind(body.name.trim())
    .bind(&body.venue_name)
    .bind(Utc::now())
    .fetch_one(pool.get_ref())
    .await
    .map_err(database_error)?;
    Ok(HttpResponse::Created().json(event))
}

pub async fn update_event(
    id: web::Path<Uuid>,
    body: web::Json<UpdateEvent>,
    pool: web::Data<PgPool>,
) -> actix_web::Result<HttpResponse> {
    if body.name.is_none() && body.venue_name.is_none() {
        return Err(error::ErrorBadRequest("Supply name or venue_name"));
    }
    if let Some(name) = &body.name {
        validate_name(name)?;
    }
    validate_venue(body.venue_name.as_ref().and_then(|venue| venue.as_deref()))?;
    // Apply only supplied fields in one statement, avoiding lost updates when
    // separate requests edit the name and venue concurrently.
    let event = sqlx::query_as::<_, EventResponse>(
        "UPDATE events SET name = COALESCE($2, name),
         venue_name = CASE WHEN $3 THEN $4 ELSE venue_name END
         WHERE id = $1 RETURNING id, name, venue_name, created_at",
    )
    .bind(*id)
    .bind(body.name.as_deref().map(str::trim))
    .bind(body.venue_name.is_some())
    .bind(body.venue_name.as_ref().and_then(|venue| venue.as_deref()))
    .fetch_optional(pool.get_ref())
    .await
    .map_err(database_error)?
    .ok_or_else(|| error::ErrorNotFound("Event not found"))?;
    Ok(HttpResponse::Ok().json(event))
}

pub async fn set_active_event(
    id: web::Path<Uuid>,
    pool: web::Data<PgPool>,
) -> actix_web::Result<HttpResponse> {
    // Updating the singleton row waits for registrations holding FOR SHARE.
    // An unknown event leaves the previous selection intact.
    let selected = sqlx::query_scalar::<_, Uuid>(
        "UPDATE app_state SET active_event_id = $1
         WHERE id = 1 AND EXISTS (SELECT 1 FROM events WHERE id = $1)
         RETURNING active_event_id",
    )
    .bind(*id)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(database_error)?;
    if selected.is_none() {
        return Err(error::ErrorNotFound("Event not found"));
    }
    Ok(HttpResponse::NoContent().finish())
}
