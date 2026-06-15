use actix_web::{HttpResponse, web};
use chrono::Utc;
use sqlx::PgPool;
use tracing::Instrument;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct FormData {
    name: String,
}

pub async fn register_player(form: web::Form<FormData>, pool: web::Data<PgPool>) -> HttpResponse {
    let request_id = Uuid::new_v4();
    let request_span =
        tracing::info_span!("Registering a new player", %request_id, player_name = form.name);
    let _request_span_guard = request_span.enter();
    let query_span = tracing::info_span!("Adding player to the databse",);
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
    .instrument(query_span)
    .await
    {
        Ok(result) => {
            if result.rows_affected() == 0 {
                tracing::error!("request_id {} - PDN code pool exhausted", request_id);
                return HttpResponse::InternalServerError().finish();
            }

            tracing::info!("request_id {} - New player registered", request_id);
            HttpResponse::Ok().finish()
        }

        Err(e) => {
            tracing::error!(
                "request_id {} - Failed to execute query: {:?}",
                request_id,
                e
            );
            HttpResponse::InternalServerError().finish()
        }
    }
}
