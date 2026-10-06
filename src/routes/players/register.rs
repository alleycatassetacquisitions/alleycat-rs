use crate::domain::{NewPlayer, PlayerEmail, PlayerName};
use crate::events::lock_active_event;
use actix_web::{HttpResponse, web};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct FormData {
    name: String,
    email: Option<String>,
}

impl TryFrom<FormData> for NewPlayer {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name = PlayerName::parse(value.name)?;
        let email = value.email.map(PlayerEmail::parse).transpose()?;
        Ok(NewPlayer { email, name })
    }
}

#[tracing::instrument(
    name = "Registering player",
    skip(form, pool),
    fields(
        name = %form.name,
        email = %form.email.is_some(),
    )
)]
pub async fn register_player(form: web::Form<FormData>, pool: web::Data<PgPool>) -> HttpResponse {
    let new_player = match form.0.try_into() {
        Ok(player) => player,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    match insert_player(&pool, &new_player).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(RegistrationError::NoActiveEvent) => HttpResponse::Conflict().body("No active event"),
        Err(RegistrationError::CodesExhausted) => {
            HttpResponse::Conflict().body("No PDN codes available for the active event")
        }
        Err(RegistrationError::Database(error)) => {
            if error.as_database_error().and_then(|e| e.constraint())
                == Some("players_event_name_key")
            {
                return HttpResponse::Conflict()
                    .body("Player name already exists in the active event");
            }
            tracing::error!(?error, "Failed to register player");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[derive(Debug)]
pub enum RegistrationError {
    NoActiveEvent,
    CodesExhausted,
    Database(sqlx::Error),
}

impl From<sqlx::Error> for RegistrationError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

#[tracing::instrument(name = "Adding a player to the database", skip(new_player, pool))]
pub async fn insert_player(pool: &PgPool, new_player: &NewPlayer) -> Result<(), RegistrationError> {
    let mut transaction = pool.begin().await?;
    let event_id = lock_active_event(&mut transaction)
        .await?
        .ok_or(RegistrationError::NoActiveEvent)?;
    // Serialize allocations within each event, including concurrent requests.
    let next_code = sqlx::query_scalar!(
        "SELECT next_pdn_code FROM events WHERE id = $1 FOR UPDATE",
        event_id
    )
    .fetch_one(&mut *transaction)
    .await?;
    let code = sqlx::query_scalar!(
        r#"
        SELECT lpad(n::text, 4, '0') AS "code!"
        FROM generate_series($1::integer, 9999) AS n
        WHERE NOT EXISTS (
            SELECT 1 FROM reserved_pdn_codes WHERE code = lpad(n::text, 4, '0')
        ) AND NOT EXISTS (
            SELECT 1 FROM players WHERE event_id = $2 AND pdn_code = lpad(n::text, 4, '0')
        )
        ORDER BY n
        LIMIT 1
        "#,
        next_code,
        event_id
    )
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or(RegistrationError::CodesExhausted)?;

    sqlx::query!(
        "INSERT INTO players (id, event_id, pdn_code, name, email, created_at) VALUES ($1, $2, $3, $4, $5, $6)",
        Uuid::new_v4(), event_id, code, new_player.name.as_ref(),
        new_player.email.as_ref().map(|email| email.as_ref()), Utc::now()
    )
    .execute(&mut *transaction)
    .await?;
    sqlx::query!(
        "UPDATE events SET next_pdn_code = $2::text::integer + 1 WHERE id = $1",
        event_id,
        code
    )
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(())
}
