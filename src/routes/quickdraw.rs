use crate::events::{get_active_event_id, lock_active_event};
use crate::proto::alleycat::device::{
    QuickdrawMatch, QuickdrawMatchErrorCode, QuickdrawMatchRejection, WriteQuickdrawMatchesRequest,
    WriteQuickdrawMatchesResponse,
};
use crate::routes::{
    errors::database_error,
    pagination::{Pagination, PaginationQuery},
    protobuf::{self, ProtobufBody},
};
use actix_web::{HttpRequest, HttpResponse, error, get, post, web};
use chrono::{DateTime, Utc};
use prost::Message;
use serde::Serialize;
use sqlx::{Acquire, PgPool};
use uuid::Uuid;

struct NewMatch {
    id: Uuid,
    player_a_id: Uuid,
    player_b_id: Uuid,
    player_a_team_id: Option<Uuid>,
    player_b_team_id: Option<Uuid>,
    player_a_draw_time_ms: i64,
    player_b_draw_time_ms: i64,
    winner_player_id: Uuid,
    occurred_at: DateTime<Utc>,
}

struct MatchValidationError {
    fields: Vec<String>,
    code: QuickdrawMatchErrorCode,
}

impl MatchValidationError {
    fn new(fields: &[&str], code: QuickdrawMatchErrorCode) -> Self {
        Self {
            fields: fields.iter().map(|field| (*field).to_owned()).collect(),
            code,
        }
    }

    fn rejection(self, index: usize, match_id: String) -> QuickdrawMatchRejection {
        QuickdrawMatchRejection {
            match_index: index as u32,
            match_id,
            fields: self.fields,
            code: self.code.into(),
        }
    }
}

fn parse_id(value: &str, field: &str) -> Result<Uuid, MatchValidationError> {
    Uuid::parse_str(value)
        .map_err(|_| MatchValidationError::new(&[field], QuickdrawMatchErrorCode::InvalidUuid))
}

fn draw_time(value: Option<u64>, field: &str) -> Result<i64, MatchValidationError> {
    let value = value.ok_or_else(|| {
        MatchValidationError::new(&[field], QuickdrawMatchErrorCode::MissingField)
    })?;
    i64::try_from(value)
        .map_err(|_| MatchValidationError::new(&[field], QuickdrawMatchErrorCode::OutOfRange))
}

impl TryFrom<QuickdrawMatch> for NewMatch {
    type Error = MatchValidationError;

    fn try_from(value: QuickdrawMatch) -> Result<Self, Self::Error> {
        let id = parse_id(&value.id, "id")?;
        let player_a_id = parse_id(&value.player_a_id, "player_a_id")?;
        let player_b_id = parse_id(&value.player_b_id, "player_b_id")?;
        let winner_player_id = parse_id(&value.winner_player_id, "winner_player_id")?;
        if player_a_id == player_b_id {
            return Err(MatchValidationError::new(
                &["player_a_id", "player_b_id"],
                QuickdrawMatchErrorCode::SamePlayer,
            ));
        }
        if winner_player_id != player_a_id && winner_player_id != player_b_id {
            return Err(MatchValidationError::new(
                &["winner_player_id"],
                QuickdrawMatchErrorCode::InvalidWinner,
            ));
        }
        let occurred_at_ms = value.occurred_at_unix_ms.ok_or_else(|| {
            MatchValidationError::new(
                &["occurred_at_unix_ms"],
                QuickdrawMatchErrorCode::MissingField,
            )
        })?;
        let occurred_at = Some(occurred_at_ms)
            .filter(|ms| (0..=253_402_300_799_999).contains(ms))
            .and_then(DateTime::from_timestamp_millis)
            .ok_or_else(|| {
                MatchValidationError::new(
                    &["occurred_at_unix_ms"],
                    QuickdrawMatchErrorCode::OutOfRange,
                )
            })?;
        Ok(Self {
            id,
            player_a_id,
            player_b_id,
            player_a_team_id: value
                .player_a_team_id
                .as_deref()
                .map(|v| parse_id(v, "player_a_team_id"))
                .transpose()?,
            player_b_team_id: value
                .player_b_team_id
                .as_deref()
                .map(|v| parse_id(v, "player_b_team_id"))
                .transpose()?,
            player_a_draw_time_ms: draw_time(value.player_a_draw_time_ms, "player_a_draw_time_ms")?,
            player_b_draw_time_ms: draw_time(value.player_b_draw_time_ms, "player_b_draw_time_ms")?,
            winner_player_id,
            occurred_at,
        })
    }
}

fn match_error(error: sqlx::Error) -> actix_web::Error {
    database_error(error, "Quickdraw operation failed")
}

fn reference_error(error: &sqlx::Error) -> Option<MatchValidationError> {
    let field = match error.as_database_error().and_then(|e| e.constraint()) {
        Some("quickdraw_matches_player_a_fkey") => "player_a_id",
        Some("quickdraw_matches_player_b_fkey") => "player_b_id",
        Some("quickdraw_matches_team_a_fkey") => "player_a_team_id",
        Some("quickdraw_matches_team_b_fkey") => "player_b_team_id",
        _ => return None,
    };
    Some(MatchValidationError::new(
        &[field],
        QuickdrawMatchErrorCode::InvalidEventReference,
    ))
}

#[utoipa::path(
    tag = "Quickdraw", summary = "Upload completed quickdraw matches",
    description = r##"Send `alleycat.device.WriteQuickdrawMatchesRequest` encoded as binary protobuf.
Use the [protobuf definition](/docs/device_api.proto) to generate your client messages.
Supply **1–100 matches**, with a maximum request body of **256 KiB**.
Uses the active event and holds that selection until the batch commits. Returns 409 when no event is selected.
After an event switch, references to previous-event players or teams are rejected per match; select that event again before retrying.

See [Device APIs: WriteQuickdrawMatchesRequest](/docs/device-apis#alleycat.device.WriteQuickdrawMatchesRequest)
for message fields, protobuf types, and validation requirements.

### Validation and scoring

- Players must be distinct, and the explicit winner must be a participant.
- New matches must reference players and teams in this event.
- Teams are device-supplied match-time snapshots, never inferred from current membership.
- Scoring uses historical periods at occurrence time. The first applicable outcome wins:
  outside a scoring period, closed period with delayed scores disabled, unassigned team,
  same team, otherwise scoring. The delayed-score switch never changes saved results.

### Partial acceptance and duplicate reports

- Valid matches commit together; invalid matches return one field-level rejection per entry.
- 204 means all entries were accepted. 200 returns binary
  [WriteQuickdrawMatchesResponse](/docs/device-apis#alleycat.device.WriteQuickdrawMatchesResponse),
  including when every entry was rejected. Entries not listed were saved or already stored.
- Request-wide errors and unexpected database failures save no new matches.
- Valid duplicate reports in the same event succeed without changing stored fields,
  including receipt time and scoring.
- For repeated IDs within a batch, the first accepted occurrence wins; rejections identify request indices.
- An ID already stored in another event rejects the batch.
"##,
    request_body(content = inline(ProtobufBody), content_type = "application/protobuf"),
    responses(
        (status = 204, description = "Entire batch accepted or already stored; empty body"),
        (status = 200, description = "Batch processed; protobuf identifies rejected entries, all other entries accepted", body = inline(ProtobufBody), content_type = "application/protobuf"),
        (status = 400, description = "Invalid protobuf or batch size; no matches saved", body = String, content_type = "text/plain"),
        (status = 409, description = "No active event or match ID belongs to another event; batch rejected", body = String, content_type = "text/plain"),
        (status = 413, description = "Request body exceeds 256 KiB"),
        (status = 415, description = "Expected application/protobuf"),
        (status = 500, description = "Database operation failed", body = String, content_type = "text/plain")
    )
)]
#[post("/quickdraw-matches")]
pub async fn write_quickdraw_matches(
    request: HttpRequest,
    body: web::Bytes,
    pool: web::Data<PgPool>,
) -> actix_web::Result<HttpResponse> {
    let batch = protobuf::decode::<WriteQuickdrawMatchesRequest>(&request, &body)?;
    if !(1..=100).contains(&batch.matches.len()) {
        return Err(error::ErrorBadRequest("Supply 1–100 matches"));
    }
    // Check ownership even for reports with other invalid fields. A known
    // cross-event match ID remains a request-wide configuration failure.
    let ids: Vec<Uuid> = batch
        .matches
        .iter()
        .filter_map(|report| Uuid::parse_str(&report.id).ok())
        .collect();
    let mut rejected = Vec::new();
    let mut transaction = pool.begin().await.map_err(match_error)?;
    let event_id = lock_active_event(&mut transaction)
        .await
        .map_err(match_error)?
        .ok_or_else(|| error::ErrorConflict("No active event"))?;
    // Share the scoring controls' per-event lock. A separate INSERT statement
    // then observes any period change committed while this request waited.
    sqlx::query_scalar!("SELECT id FROM events WHERE id = $1 FOR UPDATE", event_id)
        .fetch_one(&mut *transaction)
        .await
        .map_err(match_error)?;
    let conflicting_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM quickdraw_matches WHERE id = ANY($1) AND event_id <> $2 LIMIT 1",
    )
    .bind(&ids)
    .bind(event_id)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(match_error)?;
    if conflicting_id.is_some() {
        return Err(error::ErrorConflict(
            "Match ID already belongs to another event",
        ));
    }
    for (index, raw_report) in batch.matches.into_iter().enumerate() {
        let match_id = raw_report.id.clone();
        let report = match NewMatch::try_from(raw_report) {
            Ok(report) => report,
            Err(error) => {
                rejected.push(error.rejection(index, match_id));
                continue;
            }
        };
        // PostgreSQL aborts a transaction after a constraint violation. A
        // savepoint isolates expected reference failures from valid matches.
        let mut entry = transaction.begin().await.map_err(match_error)?;

        let inserted = sqlx::query!(
            "INSERT INTO quickdraw_matches
             (id, event_id, player_a_id, player_b_id, player_a_team_id, player_b_team_id,
              player_a_draw_time_ms, player_b_draw_time_ms, winner_player_id, occurred_at, scoring_status)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                (CASE
                    WHEN NOT EXISTS (SELECT 1 FROM event_scoring_periods
                        WHERE event_id = $2 AND started_at <= $10
                        AND (closes_at IS NULL OR $10 < closes_at)) THEN 'scoring_disabled'
                    WHEN NOT (SELECT accepting_delayed_scores FROM events WHERE id = $2)
                        AND EXISTS (SELECT 1 FROM event_scoring_periods
                            WHERE event_id = $2 AND started_at <= $10 AND $10 < closes_at)
                        THEN 'delayed_scores_disabled'
                    WHEN $5::uuid IS NULL OR $6::uuid IS NULL THEN 'unassigned_team'
                    WHEN $5 = $6 THEN 'same_team'
                    ELSE 'scoring'
                 END)::quickdraw_scoring_status)
             ON CONFLICT (id) DO NOTHING",
            report.id, event_id, report.player_a_id, report.player_b_id,
            report.player_a_team_id, report.player_b_team_id,
            report.player_a_draw_time_ms, report.player_b_draw_time_ms,
            report.winner_player_id, report.occurred_at,
        ).execute(&mut *entry).await;
        let inserted = match inserted {
            Ok(inserted) => inserted,
            Err(error) => {
                if let Some(validation) = reference_error(&error) {
                    entry.rollback().await.map_err(match_error)?;
                    rejected.push(validation.rejection(index, match_id));
                    continue;
                }
                return Err(match_error(error));
            }
        };
        if inserted.rows_affected() == 0 {
            // Retries must belong to the current event too, even though the
            // duplicate insert leaves the original record untouched.
            let owner = sqlx::query_scalar!(
                "SELECT event_id FROM quickdraw_matches WHERE id = $1",
                report.id
            )
            .fetch_one(&mut *entry)
            .await
            .map_err(match_error)?;
            if owner != event_id {
                return Err(error::ErrorConflict(
                    "Match ID already belongs to another event",
                ));
            }
        }
        entry.commit().await.map_err(match_error)?;
    }
    transaction.commit().await.map_err(match_error)?;
    if rejected.is_empty() {
        Ok(HttpResponse::NoContent().finish())
    } else {
        Ok(HttpResponse::Ok()
            .content_type("application/protobuf")
            .body(WriteQuickdrawMatchesResponse { rejected }.encode_to_vec()))
    }
}

#[derive(Serialize, sqlx::Type, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "quickdraw_scoring_status", rename_all = "snake_case")]
enum ScoringStatus {
    Scoring,
    SameTeam,
    ScoringDisabled,
    UnassignedTeam,
    DelayedScoresDisabled,
}

#[derive(Serialize, utoipa::ToSchema)]
struct MatchResponse {
    id: Uuid,
    event_id: Uuid,
    player_a_id: Uuid,
    player_b_id: Uuid,
    player_a_team_id: Option<Uuid>,
    player_b_team_id: Option<Uuid>,
    player_a_draw_time_ms: i64,
    player_b_draw_time_ms: i64,
    winner_player_id: Uuid,
    occurred_at: DateTime<Utc>,
    received_at: DateTime<Utc>,
    scoring_status: ScoringStatus,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct GetQuickdrawMatchesResponse {
    matches: Vec<MatchResponse>,
    pagination: Pagination,
}

#[utoipa::path(
    tag = "Quickdraw", summary = "List quickdraw matches in the active event",
    description = "Returns saved results, including non-scoring matches, ordered by occurred_at descending then ID descending. Teams and scoring status are historical snapshots. Uses the active event; returns 409 when none is selected. Offset pagination reflects current records; late uploads can shift pages.",
    params(PaginationQuery),
    responses(
        (status = 200, description = "Matches and effective pagination", body = GetQuickdrawMatchesResponse),
        (status = 400, description = "Invalid pagination query"),
        (status = 409, description = "No active event"),
        (status = 500, description = "Database operation failed", body = String, content_type = "text/plain")
    )
)]
#[get("/quickdraw-matches")]
pub async fn get_quickdraw_matches(
    parameters: web::Query<PaginationQuery>,
    pool: web::Data<PgPool>,
) -> actix_web::Result<web::Json<GetQuickdrawMatchesResponse>> {
    let event_id = get_active_event_id(pool.get_ref())
        .await
        .map_err(match_error)?
        .ok_or_else(|| error::ErrorConflict("No active event"))?;
    let pagination = parameters.normalize();
    let (limit, offset) = pagination.limit_offset();
    let matches = sqlx::query_as!(
        MatchResponse,
        r#"SELECT id, event_id, player_a_id, player_b_id, player_a_team_id, player_b_team_id,
           player_a_draw_time_ms, player_b_draw_time_ms, winner_player_id, occurred_at, received_at,
           scoring_status AS "scoring_status: ScoringStatus"
           FROM quickdraw_matches WHERE event_id = $1
           ORDER BY occurred_at DESC, id DESC LIMIT $2 OFFSET $3"#,
        event_id,
        limit,
        offset,
    )
    .fetch_all(pool.get_ref())
    .await
    .map_err(match_error)?;
    Ok(web::Json(GetQuickdrawMatchesResponse {
        matches,
        pagination,
    }))
}
