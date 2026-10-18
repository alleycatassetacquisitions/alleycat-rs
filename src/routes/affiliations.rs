use crate::events::lock_active_event;
use crate::routes::errors;
use actix_web::{HttpResponse, error, get, patch, post, web};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AffiliationName {
    /// Trimmed before saving. Requires 1–256 Unicode scalar values, without null characters.
    /// Names are case-sensitive and unique within their event (categories) or category (affiliations).
    name: String,
}

impl AffiliationName {
    fn validated(&self) -> actix_web::Result<&str> {
        let name = self.name.trim();
        if name.is_empty() || name.chars().count() > 256 || name.contains('\0') {
            return Err(error::ErrorBadRequest(
                "Name must contain 1–256 Unicode scalar values after trimming and no null characters",
            ));
        }
        Ok(name)
    }
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct AffiliationCategoryResponse {
    id: Uuid,
    event_id: Uuid,
    name: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct AffiliationResponse {
    id: Uuid,
    event_id: Uuid,
    category_id: Uuid,
    name: String,
}

fn database_error(error: sqlx::Error) -> actix_web::Error {
    match error
        .as_database_error()
        .and_then(|error| error.constraint())
    {
        Some("affiliation_categories_event_name_key") => {
            error::ErrorConflict("Category name already exists in this event")
        }
        Some("affiliations_category_name_key") => {
            error::ErrorConflict("Affiliation name already exists in this category")
        }
        _ => errors::database_error(error, "Affiliation operation failed"),
    }
}

// Keep one event selection for the entire operation, including parent checks.
async fn begin_active_event(pool: &PgPool) -> actix_web::Result<(Transaction<'_, Postgres>, Uuid)> {
    let mut transaction = pool.begin().await.map_err(database_error)?;
    let event_id = lock_active_event(&mut transaction)
        .await
        .map_err(database_error)?
        .ok_or_else(|| error::ErrorConflict("No active event"))?;
    Ok((transaction, event_id))
}

#[utoipa::path(
    tag = "Affiliations", summary = "Create an affiliation category",
    description = "Creates an operator-defined category in the active event. Returns 409 when no event is active. Uses AffiliationName validation. Creates no player assignments and grants no gameplay behavior.",
    request_body(content = AffiliationName, example = json!({"name": "Faction"})),
    responses(
        (status = 201, description = "Saved category", body = AffiliationCategoryResponse),
        (status = 400, description = "Invalid JSON, unknown fields, or invalid name"),
        (status = 409, description = "No active event or name already exists within this parent"),
        (status = 500, description = "Database operation failed", body = String, content_type = "text/plain")
    )
)]
#[post("/affiliation-categories")]
pub async fn create_affiliation_category(
    body: web::Json<AffiliationName>,
    pool: web::Data<PgPool>,
) -> actix_web::Result<HttpResponse> {
    let (mut transaction, event_id) = begin_active_event(pool.get_ref()).await?;
    let name = body.validated()?;
    let result = sqlx::query_as!(
        AffiliationCategoryResponse,
        "INSERT INTO affiliation_categories (event_id, name) SELECT id, $2 FROM events WHERE id = $1 RETURNING id, event_id, name",
        event_id, name
    )
    .fetch_one(&mut *transaction)
    .await
    .map_err(database_error)?;
    transaction.commit().await.map_err(database_error)?;
    Ok(HttpResponse::Created().json(result))
}

#[utoipa::path(
    tag = "Affiliations", summary = "Rename an affiliation category",
    description = "Uses the active event; returns 409 when none is selected. Requires name, following AffiliationName validation. Preserves the ID and player assignments. Event and category cannot be changed; unknown fields are rejected. Renaming to the current name succeeds. No deletion endpoint is provided.",
    params(("category_id" = Uuid, Path, description = "Category ID")),
    request_body(content = AffiliationName, example = json!({"name": "Faction"})),
    responses(
        (status = 200, description = "Saved category", body = AffiliationCategoryResponse),
        (status = 400, description = "Invalid JSON, unknown fields, or invalid name"),
        (status = 404, description = "Resource not found in the active event or invalid ID"),
        (status = 409, description = "No active event or name already exists within this parent"),
        (status = 500, description = "Database operation failed", body = String, content_type = "text/plain")
    )
)]
#[patch("/affiliation-categories/{category_id}")]
pub async fn rename_affiliation_category(
    path: web::Path<Uuid>,
    body: web::Json<AffiliationName>,
    pool: web::Data<PgPool>,
) -> actix_web::Result<HttpResponse> {
    let category_id = *path;
    let (mut transaction, event_id) = begin_active_event(pool.get_ref()).await?;
    let name = body.validated()?;
    let result = sqlx::query_as!(
        AffiliationCategoryResponse,
        "UPDATE affiliation_categories SET name = $3 WHERE event_id = $1 AND id = $2 RETURNING id, event_id, name",
        event_id, category_id, name
    )
    .fetch_optional(&mut *transaction)
    .await
    .map_err(database_error)?
    .ok_or_else(|| error::ErrorNotFound("Category or its parent not found in this event"))?;
    transaction.commit().await.map_err(database_error)?;
    Ok(HttpResponse::Ok().json(result))
}

#[utoipa::path(
    tag = "Affiliations", summary = "List affiliation categories",
    description = "Returns every category in the active event, ordered by name then ID. Returns an empty array when this event has no categories, or 409 when no event is active.",
    responses(
        (status = 200, description = "Matching entries", body = Vec<AffiliationCategoryResponse>),
        (status = 409, description = "No active event", body = String, content_type = "text/plain"),
        (status = 500, description = "Database operation failed", body = String, content_type = "text/plain")
    )
)]
#[get("/affiliation-categories")]
pub async fn list_affiliation_categories(
    pool: web::Data<PgPool>,
) -> actix_web::Result<HttpResponse> {
    let (mut transaction, event_id) = begin_active_event(pool.get_ref()).await?;
    let result = sqlx::query_as!(
        AffiliationCategoryResponse,
        "SELECT id, event_id, name FROM affiliation_categories WHERE event_id = $1 ORDER BY name, id",
        event_id
    )
    .fetch_all(&mut *transaction)
    .await
    .map_err(database_error)?;
    transaction.commit().await.map_err(database_error)?;
    Ok(HttpResponse::Ok().json(result))
}

#[utoipa::path(
    tag = "Affiliations", summary = "Create an affiliation",
    description = "Creates an operator-defined affiliation in a category belonging to the active event. Returns 409 when no event is active. Uses AffiliationName validation. Creates no player assignments and grants no gameplay behavior.",
    params(("category_id" = Uuid, Path, description = "Category ID")),
    request_body(content = AffiliationName, example = json!({"name": "Corvid Alliance"})),
    responses(
        (status = 201, description = "Saved affiliation", body = AffiliationResponse),
        (status = 400, description = "Invalid JSON, unknown fields, or invalid name"),
        (status = 404, description = "Resource not found in the active event or invalid ID"),
        (status = 409, description = "No active event or name already exists within this parent"),
        (status = 500, description = "Database operation failed", body = String, content_type = "text/plain")
    )
)]
#[post("/affiliation-categories/{category_id}/affiliations")]
pub async fn create_affiliation(
    path: web::Path<Uuid>,
    body: web::Json<AffiliationName>,
    pool: web::Data<PgPool>,
) -> actix_web::Result<HttpResponse> {
    let category_id = *path;
    let (mut transaction, event_id) = begin_active_event(pool.get_ref()).await?;
    let name = body.validated()?;
    let result = sqlx::query_as!(
        AffiliationResponse,
        "INSERT INTO affiliations (event_id, category_id, name) SELECT event_id, id, $3 FROM affiliation_categories WHERE event_id = $1 AND id = $2 RETURNING id, event_id, category_id, name",
        event_id, category_id, name
    )
    .fetch_optional(&mut *transaction)
    .await
    .map_err(database_error)?
    .ok_or_else(|| error::ErrorNotFound("Affiliation or its parent not found in this event"))?;
    transaction.commit().await.map_err(database_error)?;
    Ok(HttpResponse::Created().json(result))
}

#[utoipa::path(
    tag = "Affiliations", summary = "Rename an affiliation",
    description = "Uses the active event; returns 409 when none is selected. Requires name, following AffiliationName validation. Preserves the ID and player assignments. Event and category cannot be changed; unknown fields are rejected. Renaming to the current name succeeds. No deletion endpoint is provided.",
    params(("category_id" = Uuid, Path, description = "Category ID"),
        ("id" = Uuid, Path, description = "Affiliation ID")),
    request_body(content = AffiliationName, example = json!({"name": "Corvid Alliance"})),
    responses(
        (status = 200, description = "Saved affiliation", body = AffiliationResponse),
        (status = 400, description = "Invalid JSON, unknown fields, or invalid name"),
        (status = 404, description = "Resource not found in the active event or invalid ID"),
        (status = 409, description = "No active event or name already exists within this parent"),
        (status = 500, description = "Database operation failed", body = String, content_type = "text/plain")
    )
)]
#[patch("/affiliation-categories/{category_id}/affiliations/{id}")]
pub async fn rename_affiliation(
    path: web::Path<(Uuid, Uuid)>,
    body: web::Json<AffiliationName>,
    pool: web::Data<PgPool>,
) -> actix_web::Result<HttpResponse> {
    let (category_id, id) = path.into_inner();
    let (mut transaction, event_id) = begin_active_event(pool.get_ref()).await?;
    let name = body.validated()?;
    let result = sqlx::query_as!(
        AffiliationResponse,
        "UPDATE affiliations SET name = $4 WHERE event_id = $1 AND category_id = $2 AND id = $3 RETURNING id, event_id, category_id, name",
        event_id, category_id, id, name
    )
    .fetch_optional(&mut *transaction)
    .await
    .map_err(database_error)?
    .ok_or_else(|| error::ErrorNotFound("Affiliation or its parent not found in this event"))?;
    transaction.commit().await.map_err(database_error)?;
    Ok(HttpResponse::Ok().json(result))
}

#[utoipa::path(
    tag = "Affiliations", summary = "List affiliations",
    description = "Returns every affiliation in this category in the active event, ordered by name then ID. Returns an empty array for a category with no entries; an unknown category or one from another event returns 404. Returns 409 when no event is active.",
    params(("category_id" = Uuid, Path, description = "Category ID")),
    responses(
        (status = 200, description = "Matching entries", body = Vec<AffiliationResponse>),
        (status = 404, description = "Resource not found in the active event or invalid ID"),
        (status = 409, description = "No active event", body = String, content_type = "text/plain"),
        (status = 500, description = "Database operation failed", body = String, content_type = "text/plain")
    )
)]
#[get("/affiliation-categories/{category_id}/affiliations")]
pub async fn list_affiliations(
    path: web::Path<Uuid>,
    pool: web::Data<PgPool>,
) -> actix_web::Result<HttpResponse> {
    let category_id = *path;
    let (mut transaction, event_id) = begin_active_event(pool.get_ref()).await?;
    let exists = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM affiliation_categories WHERE event_id = $1 AND id = $2) AS "exists!""#,
        event_id, category_id
    )
    .fetch_one(&mut *transaction)
    .await
    .map_err(database_error)?;
    if !exists {
        return Err(error::ErrorNotFound("Category in this event not found"));
    }
    let result = sqlx::query_as!(
        AffiliationResponse,
        "SELECT id, event_id, category_id, name FROM affiliations WHERE event_id = $1 AND category_id = $2 ORDER BY name, id",
        event_id, category_id
    )
    .fetch_all(&mut *transaction)
    .await
    .map_err(database_error)?;
    transaction.commit().await.map_err(database_error)?;
    Ok(HttpResponse::Ok().json(result))
}

#[cfg(test)]
mod tests {
    use super::AffiliationName;

    #[test]
    fn names_are_trimmed_and_bounded_in_unicode_scalars() {
        for name in [" Faction ".to_owned(), "🦾".repeat(256)] {
            let body = AffiliationName { name };
            assert_eq!(body.validated().unwrap(), body.name.trim());
        }
        for name in [
            "".to_owned(),
            " \t\n".to_owned(),
            "a\0b".to_owned(),
            "🦾".repeat(257),
        ] {
            assert!(AffiliationName { name }.validated().is_err());
        }
    }
}
