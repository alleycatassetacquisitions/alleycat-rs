use actix_web::{HttpResponse, web};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct FormData {
    name: String,
}

#[tracing::instrument(
    name = "Registering player",
    skip(form, pool),
    fields(
        name = %form.name
    )
)]
pub async fn register_player(form: web::Form<FormData>, pool: web::Data<PgPool>) -> HttpResponse {
    match insert_player(&pool, &form).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[tracing::instrument(name = "Adding a player to the database", skip(form, pool))]
pub async fn insert_player(pool: &PgPool, form: &FormData) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        WITH picked AS (
            SELECT code
            FROM available_pdn_codes
            ORDER BY random()
            LIMIT 1
            FOR UPDATE SKIP LOCKED
        ),
        created AS (
            INSERT INTO players (id, pdn_code, name, created_at)
            SELECT $1, picked.code, $2, $3
            FROM picked
            RETURNING pdn_code
        )
        DELETE FROM available_pdn_codes
        WHERE code = (SELECT pdn_code FROM created)
        "#,
        Uuid::new_v4(),
        form.name,
        Utc::now()
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}
