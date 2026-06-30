use crate::domain::{NewPlayer, PlayerEmail, PlayerName};
use actix_web::{HttpResponse, web};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct FormData {
    name: String,
    email: Option<String>,
}

#[tracing::instrument(
    name = "Registering player",
    skip(form, pool),
    fields(
        name = %form.name
    )
)]

pub async fn register_player(form: web::Form<FormData>, pool: web::Data<PgPool>) -> HttpResponse {
    let name = match PlayerName::parse(form.0.name) {
        Ok(name) => name,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    let email = match PlayerEmail::parse(form.0.email.unwrap_or_default()) {
        Ok(email) => email,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    let new_player = NewPlayer {
        email: email,
        name: name,
    };
    match insert_player(&pool, &new_player).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[tracing::instrument(name = "Adding a player to the database", skip(new_player, pool))]
pub async fn insert_player(pool: &PgPool, new_player: &NewPlayer) -> Result<(), sqlx::Error> {
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
            INSERT INTO players (id, pdn_code, name, email, created_at)
            SELECT $1, picked.code, $2, $3, $4
            FROM picked
            RETURNING pdn_code
        )
        DELETE FROM available_pdn_codes
        WHERE code = (SELECT pdn_code FROM created)
        "#,
        Uuid::new_v4(),
        new_player.name.as_ref(),
        new_player.email.as_ref(),
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
