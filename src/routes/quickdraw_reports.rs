use crate::events::get_active_event_id;
use crate::proto::alleycat::device::QuickdrawMatch;
use crate::routes::helpers::{
    errors::database_error,
    pagination::{Pagination, PaginationQuery},
};
use actix_web::{error, get, web};
use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ReportFilter {
    /// Optional shared match UUID. Omit to include all submissions, including malformed IDs.
    match_id: Option<Uuid>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct RawMatchReport {
    /// Server-generated submission ID. Every retry has a separate ID.
    id: Uuid,
    event_id: Uuid,
    /// Parsed match UUID, or null for malformed input.
    match_id: Option<Uuid>,
    /// Reporting MAC exactly as submitted, or null when absent or blank.
    mac_address: Option<String>,
    received_at: Timestamp,
    /// Accepted outcome: completed, tie or failed. Null means the entry was rejected.
    outcome: Option<String>,
    /// All known submitted fields, including invalid values, without normalization.
    report: QuickdrawMatch,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct RawMatchReportsResponse {
    reports: Vec<RawMatchReport>,
    pagination: Pagination,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct MatchReportSummary {
    match_id: Uuid,
    /// All raw submissions for this ID, including retries and rejected entries.
    report_count: i64,
    /// Distinct nonblank MAC strings, compared exactly.
    reporting_device_count: i64,
    last_received_at: Timestamp,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct MatchReportSummariesResponse {
    matches: Vec<MatchReportSummary>,
    pagination: Pagination,
}

// numeric(20, 0) and the database range checks guarantee an exact uint64.
// Reading decimal text avoids adding a decimal library solely for conversion.
fn stored_draw_time(value: Option<String>) -> actix_web::Result<Option<u64>> {
    value
        .map(|value| {
            value.parse().map_err(|_| {
                error::ErrorInternalServerError(
                    "Stored draw time is outside the unsigned 64-bit range",
                )
            })
        })
        .transpose()
}

fn report_error(err: sqlx::Error) -> actix_web::Error {
    database_error(err, "Failed to read match reports")
}

async fn active_event(pool: &PgPool) -> actix_web::Result<Uuid> {
    get_active_event_id(pool)
        .await
        .map_err(report_error)?
        .ok_or_else(|| error::ErrorConflict("No active event"))
}

#[utoipa::path(
    tag = "Quickdraw", summary = "List raw match submissions",
    description = "Returns every retained submission for the active event, including failures, duplicates and rejected entries. Ordered by received_at descending then submission ID descending. Filter by match_id to inspect a reported group. Invalid match IDs appear only in this unfiltered listing. Uses offset pagination; new uploads may shift pages. Returns 409 without an active event.",
    params(PaginationQuery, ReportFilter),
    responses((status = 200, body = RawMatchReportsResponse), (status = 400, description = "Invalid query"), (status = 409, description = "No active event"), (status = 500, description = "Database operation failed"))
)]
#[get("/quickdraw-matches/reports")]
pub async fn get_match_reports(
    query: web::Query<PaginationQuery>,
    filter: web::Query<ReportFilter>,
    pool: web::Data<PgPool>,
) -> actix_web::Result<web::Json<RawMatchReportsResponse>> {
    let event = active_event(&pool).await?;
    let pagination = query.normalize();
    let (limit, offset) = pagination.limit_offset();
    let rows = sqlx::query!(
        "SELECT id, event_id, match_id, mac_address, received_at AS \"received_at: jiff_sqlx::Timestamp\", outcome,
        submitted_match_id, player_a_id, player_b_id, player_a_team_id, player_b_team_id,
        player_a_draw_time_ms::text AS player_a_draw_time_ms,
        player_b_draw_time_ms::text AS player_b_draw_time_ms,
        winner_player_id, occurred_at_unix_ms, failure_reason, submitted_mac_address
        FROM quickdraw_matches_raw WHERE event_id = $1 AND ($2::uuid IS NULL OR match_id = $2)
        ORDER BY received_at DESC, id DESC LIMIT $3 OFFSET $4",
        event,
        filter.match_id,
        limit,
        offset
    )
    .fetch_all(pool.get_ref())
    .await
    .map_err(report_error)?;
    let reports = rows
        .into_iter()
        .map(|row| {
            let report = QuickdrawMatch {
                id: row.submitted_match_id,
                player_a_id: row.player_a_id,
                player_b_id: row.player_b_id,
                player_a_team_id: row.player_a_team_id,
                player_b_team_id: row.player_b_team_id,
                player_a_draw_time_ms: stored_draw_time(row.player_a_draw_time_ms)?,
                player_b_draw_time_ms: stored_draw_time(row.player_b_draw_time_ms)?,
                winner_player_id: row.winner_player_id,
                occurred_at_unix_ms: row.occurred_at_unix_ms,
                failure_reason: row.failure_reason,
                mac_address: row.submitted_mac_address,
            };
            Ok(RawMatchReport {
                id: row.id,
                event_id: row.event_id,
                match_id: row.match_id,
                mac_address: row.mac_address,
                received_at: row.received_at.to_jiff(),
                outcome: row.outcome,
                report,
            })
        })
        .collect::<actix_web::Result<Vec<_>>>()?;
    Ok(web::Json(RawMatchReportsResponse {
        reports,
        pagination,
    }))
}

// One grouped query keeps classification and pagination on the same database snapshot.
// COUNT is non-null; match_id is filtered before grouping and every group has a receipt time.
async fn summaries(
    query: PaginationQuery,
    pool: &PgPool,
    kind: &str,
) -> actix_web::Result<web::Json<MatchReportSummariesResponse>> {
    let event = active_event(pool).await?;
    let pagination = query.normalize();
    let (limit, offset) = pagination.limit_offset();
    let matches = sqlx::query_as!(MatchReportSummary,
        r#"SELECT match_id AS "match_id!", COUNT(*) AS "report_count!",
            COUNT(DISTINCT mac_address) AS "reporting_device_count!",
            MAX(received_at) AS "last_received_at!: jiff_sqlx::Timestamp"
        FROM quickdraw_matches_raw WHERE event_id = $1 AND match_id IS NOT NULL
        GROUP BY match_id
        HAVING CASE $2::text
            WHEN 'orphaned' THEN COUNT(DISTINCT mac_address) = 1
            WHEN 'unassigned' THEN COUNT(DISTINCT mac_address) = 0
            WHEN 'disagreements' THEN COUNT(DISTINCT outcome) > 1 OR COUNT(DISTINCT comparison_key) > 1
            ELSE false END
        ORDER BY MAX(received_at) DESC, match_id DESC LIMIT $3 OFFSET $4"#,
        event, kind, limit, offset).fetch_all(pool).await.map_err(report_error)?;
    Ok(web::Json(MatchReportSummariesResponse {
        matches,
        pagination,
    }))
}

#[utoipa::path(
    tag = "Quickdraw", summary = "Find match report disagreements",
    description = "Groups raw reports by valid match UUID in the active event. Flags differing accepted outcomes (failure, tie, winner), participant sets, per-player draw times, or winners. Participant order and UUID spelling are normalized. Ignores occurrence and receipt timestamps, MAC addresses, team snapshots and failure text. Rejected entries do not supply comparison values but are included in report and device counts. Reports from the same MAC can disagree. Ordered by latest receipt descending then match UUID descending. Pagination counts match IDs; inspect entries using /quickdraw-matches/reports?match_id=UUID. New uploads can shift pages. Returns 409 without an active event.",
    params(PaginationQuery),
    responses((status = 200, body = MatchReportSummariesResponse), (status = 400, description = "Invalid query"), (status = 409, description = "No active event"), (status = 500, description = "Database operation failed"))
)]
#[get("/quickdraw-matches/reports/disagreements")]
pub async fn get_match_disagreements(
    query: web::Query<PaginationQuery>,
    pool: web::Data<PgPool>,
) -> actix_web::Result<web::Json<MatchReportSummariesResponse>> {
    summaries(query.into_inner(), &pool, "disagreements").await
}

#[utoipa::path(
    tag = "Quickdraw", summary = "Find orphaned match reports",
    description = "Groups all raw reports with valid match UUIDs in the active event and returns IDs reported by exactly one distinct nonblank MAC string, compared exactly as submitted. Repeated uploads from that MAC still count as one reporter. Absent or blank MACs do not count. Includes failures and rejected entries. No waiting period applies: a newly uploaded match can be orphaned until its second device reports. Ordered by latest receipt descending then match UUID descending; pagination counts match IDs and new uploads can shift pages. Inspect entries using /quickdraw-matches/reports?match_id=UUID. Returns 409 without an active event.",
    params(PaginationQuery),
    responses((status = 200, body = MatchReportSummariesResponse), (status = 400, description = "Invalid query"), (status = 409, description = "No active event"), (status = 500, description = "Database operation failed"))
)]
#[get("/quickdraw-matches/reports/orphaned")]
pub async fn get_orphaned_match_reports(
    query: web::Query<PaginationQuery>,
    pool: web::Data<PgPool>,
) -> actix_web::Result<web::Json<MatchReportSummariesResponse>> {
    summaries(query.into_inner(), &pool, "orphaned").await
}

#[utoipa::path(
    tag = "Quickdraw", summary = "Find match reports with no assigned device",
    description = "Groups all raw reports with valid match UUIDs in the active event and returns IDs with no nonblank reporting MAC. This describes missing device identity, not missing team assignments. Includes failures and rejected entries. A single known MAC moves a match to orphaned, even when other submissions have no MAC. Ordered by latest receipt descending then match UUID descending; pagination counts match IDs and new uploads can shift pages. Inspect entries using /quickdraw-matches/reports?match_id=UUID. Returns 409 without an active event.",
    params(PaginationQuery),
    responses((status = 200, body = MatchReportSummariesResponse), (status = 400, description = "Invalid query"), (status = 409, description = "No active event"), (status = 500, description = "Database operation failed"))
)]
#[get("/quickdraw-matches/reports/unassigned")]
pub async fn get_unassigned_match_reports(
    query: web::Query<PaginationQuery>,
    pool: web::Data<PgPool>,
) -> actix_web::Result<web::Json<MatchReportSummariesResponse>> {
    summaries(query.into_inner(), &pool, "unassigned").await
}
