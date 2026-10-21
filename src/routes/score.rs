use crate::events::get_active_event_id;
use crate::routes::helpers::errors::database_error;
use crate::routes::helpers::pagination::{Pagination, PaginationQuery};
use actix_web::{error, get, web};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Serialize, utoipa::ToSchema)]
pub struct RankedPlayer {
    pub id: Uuid,
    pub name: String,
    /// Current team ID, or null when unassigned. Does not change earned points.
    pub team_id: Option<Uuid>,
    /// Current team name, or null when unassigned.
    pub team_name: Option<String>,
    /// Rank by score alone. Ties share a rank and leave gaps, for example 1, 1, 3.
    pub rank: i64,
    /// One point per win in a match saved with scoring_status = scoring.
    pub score: i64,
    /// Number of scoring matches played, including wins and losses.
    pub matches_played: i64,
    /// Mean of this player's draw times in scoring matches, in milliseconds.
    /// Includes losses and zero measurements; null when matches_played is zero.
    pub average_draw_time_ms: Option<f64>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct PlayerScoresResponse {
    event_id: Uuid,
    players: Vec<RankedPlayer>,
    pagination: Pagination,
}

#[utoipa::path(
    tag = "Scores", summary = "Rank players in the active event",
    description = "Returns individual quickdraw scores across the active event's full history. Includes all registered players, including inactive players and players with no scoring matches. Only matches saved with scoring_status = scoring contribute: each recorded winner earns one point, and both players contribute their own draw times and match counts. Current team membership and scoring switches do not recalculate past results. Team ID and name describe current membership and are both null when unassigned. Highest score first; equal scores share a rank (1, 1, 3). Tied rows are ordered by name using PostgreSQL C collation, then player UUID; draw time does not break ties. Ranks are calculated before pagination. Returns 409 without an active event. Late uploads can change scores, averages, ranks, and page contents between requests.",
    params(PaginationQuery),
    responses(
        (status = 200, description = "Player scores and effective pagination", body = PlayerScoresResponse),
        (status = 400, description = "Invalid pagination query"),
        (status = 409, description = "No active event"),
        (status = 500, description = "Database operation failed", body = String, content_type = "text/plain")
    )
)]
#[get("/score/players")]
pub async fn get_player_scores(
    parameters: web::Query<PaginationQuery>,
    pool: web::Data<PgPool>,
) -> actix_web::Result<web::Json<PlayerScoresResponse>> {
    let event_id = get_active_event_id(&pool)
        .await
        .map_err(|error| database_error(error, "Failed to retrieve player scores"))?
        .ok_or_else(|| error::ErrorConflict("No active event"))?;
    let pagination = parameters.normalize();
    let (limit, offset) = pagination.limit_offset();

    // Resolve the event once. All scores and current team labels below share a
    // single statement snapshot, even if the active selection changes meanwhile.
    // COUNT, RANK and the COALESCE expressions cannot be null; AVG stays nullable
    // for players without matches. AVG(bigint) uses numeric before conversion.
    let players = sqlx::query_as!(
        RankedPlayer,
        r#"WITH totals AS (
            SELECT participant.player_id,
                   COUNT(*) FILTER (WHERE m.winner_player_id = participant.player_id) AS score,
                   COUNT(*) AS matches_played,
                   AVG(participant.draw_time_ms)::double precision AS average_draw_time_ms
            FROM quickdraw_matches m
            CROSS JOIN LATERAL (VALUES
                (m.player_a_id, m.player_a_draw_time_ms),
                (m.player_b_id, m.player_b_draw_time_ms)
            ) AS participant(player_id, draw_time_ms)
            WHERE m.event_id = $1 AND m.scoring_status = 'scoring'
            GROUP BY participant.player_id
        )
        SELECT p.id, p.name, p.team_id, t.name AS "team_name?",
               RANK() OVER (ORDER BY COALESCE(totals.score, 0) DESC) AS "rank!",
               COALESCE(totals.score, 0) AS "score!",
               COALESCE(totals.matches_played, 0) AS "matches_played!",
               totals.average_draw_time_ms AS "average_draw_time_ms?"
        FROM players p
        LEFT JOIN teams t ON t.id = p.team_id AND t.event_id = p.event_id
        LEFT JOIN totals ON totals.player_id = p.id
        WHERE p.event_id = $1
        ORDER BY COALESCE(totals.score, 0) DESC, p.name COLLATE "C", p.id
        LIMIT $2 OFFSET $3"#,
        event_id,
        limit,
        offset,
    )
    .fetch_all(pool.get_ref())
    .await
    .map_err(|error| database_error(error, "Failed to retrieve player scores"))?;

    Ok(web::Json(PlayerScoresResponse {
        event_id,
        players,
        pagination,
    }))
}
