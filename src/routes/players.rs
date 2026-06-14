use actix_web::{HttpResponse, web};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct FormData {
    name: String,
}

pub async fn register_player(form: web::Form<FormData>, pool: web::Data<PgPool>) -> HttpResponse {
    log::info!("Registering '{}' as a new player.", form.name);
    match sqlx::query!(
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
    .execute(pool.get_ref())
    .await
    {
        Ok(result) => {
            if result.rows_affected() == 0 {
                log::error!("PDN code pool exhausted");
                return HttpResponse::InternalServerError().finish();
            }

            log::info!("New player registered");
            HttpResponse::Ok().finish()
        }

        Err(e) => {
            log::error!("Failed to execute query: {:?}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}
