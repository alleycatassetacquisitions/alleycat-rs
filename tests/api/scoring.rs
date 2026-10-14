//! Database-boundary tests for the scoring migrations. Runtime SQL here exercises
//! constraints and the ingestion contract; scoring controls have separate API tests.
use crate::helpers::spawn_app;
use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

struct Game {
    event: Uuid,
    players: [Uuid; 2],
    teams: [Uuid; 2],
}

impl Game {
    async fn create(pool: &PgPool) -> Self {
        let game = Self {
            event: Uuid::new_v4(),
            players: [Uuid::new_v4(), Uuid::new_v4()],
            teams: [Uuid::new_v4(), Uuid::new_v4()],
        };
        sqlx::query("INSERT INTO events (id, name, created_at) VALUES ($1, 'Scoring test', now())")
            .bind(game.event)
            .execute(pool)
            .await
            .unwrap();
        for i in 0..2 {
            sqlx::query("INSERT INTO teams (id, event_id, name) VALUES ($1, $2, $3)")
                .bind(game.teams[i])
                .bind(game.event)
                .bind(format!("Team {i}"))
                .execute(pool)
                .await
                .unwrap();
            sqlx::query(
                "INSERT INTO players (id, event_id, name, pdn_code, team_id, created_at)
                 VALUES ($1, $2, $3, $4, $5, now())",
            )
            .bind(game.players[i])
            .bind(game.event)
            .bind(format!("Player {i}"))
            .bind(format!("{i:04}"))
            .bind(game.teams[i])
            .execute(pool)
            .await
            .unwrap();
        }
        game
    }
}

fn start() -> DateTime<Utc> {
    "2026-10-10T15:00:00Z".parse().unwrap()
}

async fn add_period(
    pool: &PgPool,
    event: Uuid,
    started_at: DateTime<Utc>,
    closes_at: Option<DateTime<Utc>>,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO event_scoring_periods (event_id, started_at, closes_at)
         VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(event)
    .bind(started_at)
    .bind(closes_at)
    .fetch_one(pool)
    .await
}

async fn period_at(pool: &PgPool, event: Uuid, at: DateTime<Utc>) -> Option<Uuid> {
    sqlx::query_scalar(
        "SELECT id FROM event_scoring_periods
         WHERE event_id = $1 AND tstzrange(started_at, closes_at, '[)') @> $2::timestamptz",
    )
    .bind(event)
    .bind(at)
    .fetch_optional(pool)
    .await
    .unwrap()
}

struct Match {
    id: Uuid,
    event: Uuid,
    players: [Uuid; 2],
    teams: [Option<Uuid>; 2],
    winner: Uuid,
    occurred_at: DateTime<Utc>,
    scoring_status: &'static str,
}

impl Match {
    fn new(game: &Game, scoring_status: &'static str) -> Self {
        Self {
            id: Uuid::new_v4(),
            event: game.event,
            players: game.players,
            teams: game.teams.map(Some),
            winner: game.players[0],
            occurred_at: start() + Duration::minutes(5),
            scoring_status,
        }
    }

    async fn insert(&self, pool: &PgPool) -> Result<u64, sqlx::Error> {
        sqlx::query(
            "INSERT INTO quickdraw_matches
             (id, event_id, player_a_id, player_b_id, player_a_team_id, player_b_team_id,
              player_a_draw_time_ms, player_b_draw_time_ms, winner_player_id, occurred_at,
              scoring_status)
             VALUES ($1, $2, $3, $4, $5, $6, 240, 310, $7, $8, $9::text::quickdraw_scoring_status)
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(self.id)
        .bind(self.event)
        .bind(self.players[0])
        .bind(self.players[1])
        .bind(self.teams[0])
        .bind(self.teams[1])
        .bind(self.winner)
        .bind(self.occurred_at)
        .bind(self.scoring_status)
        .execute(pool)
        .await
        .map(|result| result.rows_affected())
    }

    async fn status(&self, pool: &PgPool) -> String {
        sqlx::query_scalar("SELECT scoring_status::text FROM quickdraw_matches WHERE id = $1")
            .bind(self.id)
            .fetch_one(pool)
            .await
            .unwrap()
    }
}

fn assert_constraint(error: sqlx::Error, constraint: &str) {
    assert_eq!(
        error.as_database_error().unwrap().constraint(),
        Some(constraint),
        "{error}"
    );
}

#[tokio::test]
async fn scoring_periods_use_occurrence_time_and_include_only_the_opening_boundary() {
    let app = spawn_app().await;
    let pool = &app.db_pool;
    let game = Game::create(pool).await;
    assert_eq!(period_at(pool, game.event, start()).await, None);
    let close = start() + Duration::hours(1);
    let first = add_period(pool, game.event, start(), Some(close))
        .await
        .unwrap();
    for (at, expected) in [
        (start() - Duration::microseconds(1), None),
        (start(), Some(first)),
        (close - Duration::microseconds(1), Some(first)),
        (close, None),
    ] {
        assert_eq!(period_at(pool, game.event, at).await, expected);
    }
    let second = add_period(pool, game.event, close, None).await.unwrap();
    assert_eq!(period_at(pool, game.event, close).await, Some(second));
}

#[tokio::test]
async fn periods_reject_overlap_bad_times_and_allow_independent_events() {
    let app = spawn_app().await;
    let pool = &app.db_pool;
    let game = Game::create(pool).await;
    let other = Game::create(pool).await;
    let close = start() + Duration::hours(1);
    let first = add_period(pool, game.event, start(), Some(close))
        .await
        .unwrap();
    for (open, end) in [
        (start(), None),
        (start() - Duration::minutes(1), Some(close)),
        (start() + Duration::minutes(1), Some(close)),
        (
            start() - Duration::hours(1),
            Some(close + Duration::hours(1)),
        ),
    ] {
        assert_constraint(
            add_period(pool, game.event, open, end).await.unwrap_err(),
            "event_scoring_periods_no_overlap",
        );
    }
    for end in [start(), start() - Duration::seconds(1)] {
        assert!(
            add_period(pool, other.event, start(), Some(end))
                .await
                .is_err()
        );
    }
    add_period(pool, other.event, start(), None).await.unwrap();
    let next = add_period(pool, game.event, close, None).await.unwrap();
    assert_constraint(
        sqlx::query("UPDATE event_scoring_periods SET closes_at = NULL WHERE id = $1")
            .bind(first)
            .execute(pool)
            .await
            .unwrap_err(),
        "event_scoring_periods_no_overlap",
    );
    assert_constraint(
        sqlx::query("UPDATE event_scoring_periods SET started_at = '-infinity' WHERE id = $1")
            .bind(next)
            .execute(pool)
            .await
            .unwrap_err(),
        "event_scoring_periods_finite_times",
    );
}

#[tokio::test]
async fn concurrent_opens_cannot_create_two_periods_for_one_event() {
    let app = spawn_app().await;
    let pool = &app.db_pool;
    let game = Game::create(pool).await;
    let (a, b) = tokio::join!(
        add_period(pool, game.event, start(), None),
        add_period(pool, game.event, start() + Duration::seconds(1), None)
    );
    let results = [a, b];
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    for error in results.into_iter().filter_map(Result::err) {
        assert_constraint(error, "event_scoring_periods_no_overlap");
    }
}

#[tokio::test]
async fn explicit_scoring_decisions_preserve_history_and_must_agree_with_saved_teams() {
    let app = spawn_app().await;
    let pool = &app.db_pool;
    let game = Game::create(pool).await;
    let period = add_period(pool, game.event, start(), None).await.unwrap();
    let opposing = game.teams.map(Some);
    for (teams, expected) in [
        (opposing, "scoring"),
        ([opposing[0]; 2], "same_team"),
        ([None, opposing[1]], "unassigned_team"),
        ([None, None], "unassigned_team"),
        (opposing, "scoring_disabled"),
        ([opposing[0]; 2], "scoring_disabled"),
        ([None, None], "scoring_disabled"),
    ] {
        let mut duel = Match::new(&game, expected);
        duel.teams = teams;
        assert_eq!(duel.insert(pool).await.unwrap(), 1);
        assert_eq!(duel.status(pool).await, expected);
    }
    for (teams, invalid_status) in [
        ([opposing[0]; 2], "scoring"),
        ([None, opposing[1]], "scoring"),
        ([opposing[0], None], "scoring"),
        (opposing, "same_team"),
        (opposing, "unassigned_team"),
        ([None, None], "same_team"),
    ] {
        let mut duel = Match::new(&game, invalid_status);
        duel.teams = teams;
        assert_constraint(
            duel.insert(pool).await.unwrap_err(),
            "quickdraw_matches_scoring_teams_check",
        );
    }
    let duel = Match::new(&game, "scoring");
    duel.insert(pool).await.unwrap();
    // The writer must decide eligibility explicitly; omission must not silently
    // classify a result as scoring (or discard a legitimate score).
    let error = sqlx::query(
        "INSERT INTO quickdraw_matches
         (id, event_id, player_a_id, player_b_id, player_a_draw_time_ms,
          player_b_draw_time_ms, winner_player_id, occurred_at)
         SELECT $1, event_id, player_a_id, player_b_id, player_a_draw_time_ms,
                player_b_draw_time_ms, winner_player_id, occurred_at
         FROM quickdraw_matches WHERE id = $2",
    )
    .bind(Uuid::new_v4())
    .bind(duel.id)
    .execute(pool)
    .await
    .unwrap_err();
    assert_eq!(
        error.as_database_error().unwrap().code().as_deref(),
        Some("23502")
    );
    assert!(
        Match::new(&game, "unknown_status")
            .insert(pool)
            .await
            .is_err()
    );
    sqlx::query("UPDATE players SET team_id = $1 WHERE id = $2")
        .bind(game.teams[0])
        .bind(game.players[1])
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("UPDATE event_scoring_periods SET closes_at = $1 WHERE id = $2")
        .bind(start() + Duration::hours(1))
        .bind(period)
        .execute(pool)
        .await
        .unwrap();
    assert_eq!(duel.status(pool).await, "scoring");
    assert!(
        sqlx::query("UPDATE quickdraw_matches SET scoring_status = 'same_team' WHERE id = $1")
            .bind(duel.id)
            .execute(pool)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn first_upload_wins_despite_timestamp_variation_or_concurrent_retries() {
    let app = spawn_app().await;
    let pool = &app.db_pool;
    let game = Game::create(pool).await;
    let mut duel = Match::new(&game, "scoring");
    let occurred = duel.occurred_at;
    assert_eq!(duel.insert(pool).await.unwrap(), 1);
    let before: serde_json::Value =
        sqlx::query_scalar("SELECT to_jsonb(m) FROM quickdraw_matches m WHERE id = $1")
            .bind(duel.id)
            .fetch_one(pool)
            .await
            .unwrap();
    duel.occurred_at += Duration::milliseconds(5);
    duel.scoring_status = "scoring_disabled";
    let (a, b) = tokio::join!(duel.insert(pool), duel.insert(pool));
    assert_eq!((a.unwrap(), b.unwrap()), (0, 0));
    let after: serde_json::Value =
        sqlx::query_scalar("SELECT to_jsonb(m) FROM quickdraw_matches m WHERE id = $1")
            .bind(duel.id)
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(before, after);

    // Concurrent first uploads also produce one result, whichever arrives first.
    let a = Match::new(&game, "scoring");
    duel.id = a.id;
    let (first, second) = tokio::join!(a.insert(pool), duel.insert(pool));
    assert_eq!(first.unwrap() + second.unwrap(), 1);
    let saved: DateTime<Utc> =
        sqlx::query_scalar("SELECT occurred_at FROM quickdraw_matches WHERE id = $1")
            .bind(a.id)
            .fetch_one(pool)
            .await
            .unwrap();
    assert!(saved == occurred || saved == duel.occurred_at);
}

#[tokio::test]
async fn receipt_time_defaults_to_database_clock_and_survives_unrelated_updates() {
    let app = spawn_app().await;
    let pool = &app.db_pool;
    let game = Game::create(pool).await;
    let duel = Match::new(&game, "scoring_disabled");
    let before: DateTime<Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(pool)
        .await
        .unwrap();
    // Ingestion omits received_at so PostgreSQL supplies its clock time.
    duel.insert(pool).await.unwrap();
    let received: DateTime<Utc> =
        sqlx::query_scalar("SELECT received_at FROM quickdraw_matches WHERE id = $1")
            .bind(duel.id)
            .fetch_one(pool)
            .await
            .unwrap();
    let after: DateTime<Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(pool)
        .await
        .unwrap();
    assert!(received >= before && received <= after);

    let (unchanged, draw_time): (DateTime<Utc>, i64) = sqlx::query_as(
        "UPDATE quickdraw_matches
         SET player_a_draw_time_ms = 250
         WHERE id = $1 RETURNING received_at, player_a_draw_time_ms",
    )
    .bind(duel.id)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(unchanged, received);
    assert_eq!(draw_time, 250);
}

#[tokio::test]
async fn matches_reject_cross_event_references_self_play_and_invalid_measurements() {
    let app = spawn_app().await;
    let pool = &app.db_pool;
    let game = Game::create(pool).await;
    let other = Game::create(pool).await;
    for slot in 0..2 {
        let mut duel = Match::new(&game, "scoring");
        duel.players[slot] = other.players[slot];
        duel.winner = duel.players[0];
        assert_constraint(
            duel.insert(pool).await.unwrap_err(),
            if slot == 0 {
                "quickdraw_matches_player_a_fkey"
            } else {
                "quickdraw_matches_player_b_fkey"
            },
        );
        let mut duel = Match::new(&game, "scoring");
        duel.teams[slot] = Some(other.teams[slot]);
        assert_constraint(
            duel.insert(pool).await.unwrap_err(),
            if slot == 0 {
                "quickdraw_matches_team_a_fkey"
            } else {
                "quickdraw_matches_team_b_fkey"
            },
        );
    }
    let mut duel = Match::new(&game, "scoring");
    duel.players[1] = duel.players[0];
    assert_constraint(
        duel.insert(pool).await.unwrap_err(),
        "quickdraw_matches_distinct_players",
    );
    let mut duel = Match::new(&game, "scoring");
    duel.winner = other.players[0];
    assert_constraint(
        duel.insert(pool).await.unwrap_err(),
        "quickdraw_matches_winner_is_participant",
    );
    let duel = Match::new(&game, "scoring");
    duel.insert(pool).await.unwrap();
    for assignment in [
        "player_a_draw_time_ms = -1",
        "player_b_draw_time_ms = -1",
        "occurred_at = 'infinity'",
        "received_at = '-infinity'",
    ] {
        assert!(
            sqlx::query(&format!(
                "UPDATE quickdraw_matches SET {assignment} WHERE id = $1"
            ))
            .bind(duel.id)
            .execute(pool)
            .await
            .is_err(),
            "accepted {assignment}"
        );
    }
}
