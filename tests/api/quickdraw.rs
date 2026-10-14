use crate::helpers::{TestApp, assert_text_error, spawn_app};
use alleycat_rs::proto::alleycat::device::{QuickdrawMatch, WriteQuickdrawMatchesRequest};
use chrono::{DateTime, Utc};
use prost::Message;
use reqwest::Method;
use serde_json::{Value, json};
use uuid::Uuid;

struct Game {
    event: Uuid,
    players: [Uuid; 2],
    teams: [Uuid; 2],
}

impl Game {
    async fn create(app: &TestApp) -> Self {
        let event = app.create_event(json!({"name": "Quickdraw"})).await;
        let game = Self {
            event: Uuid::parse_str(event["id"].as_str().unwrap()).unwrap(),
            players: [Uuid::new_v4(), Uuid::new_v4()],
            teams: [Uuid::new_v4(), Uuid::new_v4()],
        };
        for i in 0..2 {
            sqlx::query("INSERT INTO teams (id, event_id, name) VALUES ($1, $2, $3)")
                .bind(game.teams[i])
                .bind(game.event)
                .bind(format!("Team {i}"))
                .execute(&app.db_pool)
                .await
                .unwrap();
            // Current membership is intentionally absent. Ingestion must use
            // the reported snapshots, including for delayed matches.
            sqlx::query(
                "INSERT INTO players (id, event_id, name, pdn_code, created_at)
                         VALUES ($1, $2, $3, $4, now())",
            )
            .bind(game.players[i])
            .bind(game.event)
            .bind(format!("Player {i}"))
            .bind(format!("{i:04}"))
            .execute(&app.db_pool)
            .await
            .unwrap();
        }
        game
    }

    fn path(&self) -> String {
        format!("/events/{}/quickdraw-matches", self.event)
    }

    fn report(&self, id: u128, timestamp: i64) -> QuickdrawMatch {
        QuickdrawMatch {
            id: Uuid::from_u128(id).to_string(),
            player_a_id: self.players[0].to_string(),
            player_b_id: self.players[1].to_string(),
            player_a_team_id: Some(self.teams[0].to_string()),
            player_b_team_id: Some(self.teams[1].to_string()),
            player_a_draw_time_ms: Some(0),
            player_b_draw_time_ms: Some(310),
            // Explicit winner is authoritative, even if their time is slower.
            winner_player_id: self.players[1].to_string(),
            occurred_at_unix_ms: Some(timestamp),
        }
    }

    async fn upload(&self, app: &TestApp, matches: Vec<QuickdrawMatch>) -> reqwest::Response {
        app.post_protobuf(&self.path(), &WriteQuickdrawMatchesRequest { matches })
            .await
    }

    async fn list(&self, app: &TestApp) -> Value {
        let response = app.get(&self.path()).await;
        assert_eq!(response.status(), 200);
        response.json().await.unwrap()
    }

    async fn period(&self, app: &TestApp, start: i64, end: Option<i64>) {
        sqlx::query("INSERT INTO event_scoring_periods (event_id, started_at, closes_at) VALUES ($1, $2, $3)")
            .bind(self.event).bind(DateTime::from_timestamp_millis(start).unwrap())
            .bind(end.map(|ms| DateTime::from_timestamp_millis(ms).unwrap()))
            .execute(&app.db_pool).await.unwrap();
    }
}

#[tokio::test]
async fn delayed_matches_use_period_boundaries_and_device_team_snapshots() {
    let app = spawn_app().await;
    let game = Game::create(&app).await;
    let start = 1_790_000_000_000;
    game.period(&app, start, Some(start + 1000)).await;
    let mut reports = vec![
        game.report(1, start - 1),
        game.report(2, start),
        game.report(3, start + 999),
        game.report(4, start + 1000),
        game.report(5, start + 100),
        game.report(6, start + 100),
        game.report(7, start + 1000),
    ];
    reports[4].player_b_team_id = reports[4].player_a_team_id.clone();
    reports[5].player_a_team_id = None;
    reports[6].player_a_team_id = None;
    let before: DateTime<Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(&app.db_pool)
        .await
        .unwrap();
    let response = game.upload(&app, reports).await;
    assert_eq!(response.status(), 204);
    assert!(response.bytes().await.unwrap().is_empty());
    let after: DateTime<Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(&app.db_pool)
        .await
        .unwrap();
    let body = game.list(&app).await;
    let matches = body["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 7);
    for (id, status) in [
        (1, "scoring_disabled"),
        (2, "scoring"),
        (3, "scoring"),
        (4, "scoring_disabled"),
        (5, "same_team"),
        (6, "unassigned_team"),
        (7, "scoring_disabled"),
    ] {
        let row = matches
            .iter()
            .find(|row| row["id"] == Uuid::from_u128(id).to_string())
            .unwrap();
        assert_eq!(row["scoring_status"], status);
        assert_eq!(row["winner_player_id"], game.players[1].to_string());
        assert_eq!(row["player_a_draw_time_ms"], 0);
        assert_eq!(row["player_b_draw_time_ms"], 310);
        let received: DateTime<Utc> = row["received_at"].as_str().unwrap().parse().unwrap();
        assert!(received >= before && received <= after);
        if id <= 4 {
            assert_eq!(row["player_a_team_id"], game.teams[0].to_string());
            assert_eq!(row["player_b_team_id"], game.teams[1].to_string());
        }
    }
    // Closing/reopening scoring and changing current membership cannot rewrite
    // the accepted results or their credit.
    sqlx::query("UPDATE players SET team_id = $1 WHERE event_id = $2")
        .bind(game.teams[0])
        .bind(game.event)
        .execute(&app.db_pool)
        .await
        .unwrap();
    let response = app
        .request_json(
            Method::PUT,
            &format!("/events/{}/scoring", game.event),
            json!({"enabled": true}),
        )
        .await;
    assert_eq!(response.status(), 200);
    assert_eq!(game.list(&app).await, body);
}

#[tokio::test]
async fn retries_preserve_the_first_report_including_receipt_time() {
    let app = spawn_app().await;
    let game = Game::create(&app).await;
    let first = game.report(1, 1_790_000_000_000);
    let mut second = first.clone();
    second.player_a_draw_time_ms = Some(50);
    second.occurred_at_unix_ms = Some(1_790_000_000_100);
    second.winner_player_id = game.players[0].to_string();
    second.player_b_team_id = None;
    assert_eq!(
        game.upload(&app, vec![first.clone(), second.clone()])
            .await
            .status(),
        204
    );
    let saved = game.list(&app).await;
    assert_eq!(saved["matches"].as_array().unwrap().len(), 1);
    assert_eq!(saved["matches"][0]["player_a_draw_time_ms"], 0);
    // A period added later must not rescore a duplicate.
    game.period(&app, 1_790_000_000_000, None).await;
    let (a, b) = tokio::join!(
        game.upload(&app, vec![second.clone()]),
        game.upload(&app, vec![first]),
    );
    assert_eq!(a.status(), 204);
    assert_eq!(b.status(), 204);
    assert_eq!(game.list(&app).await, saved);

    second.id = Uuid::from_u128(2).to_string();
    let mut competing = second.clone();
    competing.player_a_draw_time_ms = Some(99);
    let (a, b) = tokio::join!(
        game.upload(&app, vec![second]),
        game.upload(&app, vec![competing]),
    );
    assert_eq!(a.status(), 204);
    assert_eq!(b.status(), 204);
    let saved = game.list(&app).await;
    assert_eq!(saved["matches"].as_array().unwrap().len(), 2);
    assert!([json!(50), json!(99)].contains(&saved["matches"][0]["player_a_draw_time_ms"]));
}

#[tokio::test]
async fn invalid_event_references_roll_back_the_whole_batch() {
    let app = spawn_app().await;
    let game = Game::create(&app).await;
    let other = Game::create(&app).await;
    for index in 0..4 {
        let first = game.report(1, 1000);
        let mut invalid = game.report(2, 1000);
        match index {
            0 => invalid.player_a_id = other.players[0].to_string(),
            1 => {
                invalid.player_b_id = other.players[1].to_string();
                invalid.winner_player_id = invalid.player_b_id.clone();
            }
            2 => invalid.player_a_team_id = Some(other.teams[0].to_string()),
            _ => invalid.player_b_team_id = Some(Uuid::new_v4().to_string()),
        }
        assert_text_error(game.upload(&app, vec![first, invalid]).await, 400).await;
        assert!(
            game.list(&app).await["matches"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
}

#[tokio::test]
async fn invalid_fields_and_batch_sizes_are_rejected_without_writes() {
    let app = spawn_app().await;
    let game = Game::create(&app).await;
    for index in 0..15 {
        let mut report = game.report(1, 1000);
        match index {
            0 => report.id.clear(),
            1 => report.player_a_id = "invalid".into(),
            2 => report.player_b_id.clear(),
            3 => report.player_b_id = report.player_a_id.clone(),
            4 => report.winner_player_id = Uuid::new_v4().to_string(),
            5 => report.player_a_team_id = Some(String::new()),
            6 => report.player_b_team_id = Some("invalid".into()),
            7 => report.player_a_draw_time_ms = None,
            8 => report.player_b_draw_time_ms = None,
            9 => report.player_a_draw_time_ms = Some(u64::MAX),
            10 => report.player_b_draw_time_ms = Some(u64::MAX),
            11 => report.occurred_at_unix_ms = None,
            12 => report.occurred_at_unix_ms = Some(-1),
            13 => report.occurred_at_unix_ms = Some(i64::MAX),
            _ => report.winner_player_id.clear(),
        }
        assert_text_error(game.upload(&app, vec![report]).await, 400).await;
    }
    for size in [0, 101] {
        assert_text_error(
            game.upload(&app, vec![game.report(1, 1000); size]).await,
            400,
        )
        .await;
    }
    assert_text_error(
        app.post_bytes(&game.path(), "application/protobuf", vec![0xff])
            .await,
        400,
    )
    .await;
    assert_text_error(
        app.post_bytes(&game.path(), "application/json", b"{}".to_vec())
            .await,
        415,
    )
    .await;
    assert_text_error(
        app.post_bytes(&game.path(), "application/protobuf", vec![0; 262_145])
            .await,
        413,
    )
    .await;
    assert!(
        game.list(&app).await["matches"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    let mut report = game.report(1, 253_402_300_799_999);
    report.player_a_draw_time_ms = Some(i64::MAX as u64);
    report.player_b_draw_time_ms = Some(i64::MAX as u64);
    assert_eq!(game.upload(&app, vec![report; 100]).await.status(), 204);
    let saved = game.list(&app).await;
    assert_eq!(saved["matches"].as_array().unwrap().len(), 1);
    assert_eq!(saved["matches"][0]["player_a_draw_time_ms"], i64::MAX);
    assert_eq!(saved["matches"][0]["player_b_draw_time_ms"], i64::MAX);
    assert_eq!(
        saved["matches"][0]["occurred_at"],
        "9999-12-31T23:59:59.999Z"
    );
}

#[tokio::test]
async fn listing_uses_explicit_event_and_stable_timestamp_and_id_order() {
    let app = spawn_app().await;
    let game = Game::create(&app).await;
    let other = Game::create(&app).await;
    app.activate_new_event().await;
    assert_eq!(
        game.upload(
            &app,
            vec![
                game.report(1, 1000),
                game.report(3, 1000),
                game.report(2, 2000)
            ]
        )
        .await
        .status(),
        204
    );
    assert_eq!(
        other
            .upload(&app, vec![other.report(4, 3000)])
            .await
            .status(),
        204
    );
    let response = app.get(&format!("{}?per_page=2", game.path())).await;
    assert_eq!(response.status(), 200);
    let first: Value = response.json().await.unwrap();
    assert_eq!(first["pagination"], json!({"page": 1, "per_page": 2}));
    assert_eq!(first["matches"][0]["id"], Uuid::from_u128(2).to_string());
    assert_eq!(first["matches"][1]["id"], Uuid::from_u128(3).to_string());
    let second: Value = app
        .get(&format!("{}?per_page=2&page=2", game.path()))
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(second["matches"].as_array().unwrap().len(), 1);
    assert_eq!(second["matches"][0]["id"], Uuid::from_u128(1).to_string());
    assert_eq!(
        other.list(&app).await["matches"].as_array().unwrap().len(),
        1
    );
    for id in [Uuid::new_v4().to_string(), "invalid".into()] {
        let path = format!("/events/{id}/quickdraw-matches");
        assert_text_error(app.get(&path).await, 404).await;
        assert_text_error(
            app.post_protobuf(
                &path,
                &WriteQuickdrawMatchesRequest {
                    matches: vec![game.report(5, 1000)],
                },
            )
            .await,
            404,
        )
        .await;
    }
}

#[tokio::test]
async fn cross_event_duplicate_ids_conflict_and_do_not_partially_commit() {
    let app = spawn_app().await;
    let game = Game::create(&app).await;
    let other = Game::create(&app).await;
    assert_eq!(
        game.upload(&app, vec![game.report(2, 1000)]).await.status(),
        204
    );
    assert_text_error(
        other
            .upload(&app, vec![other.report(1, 1000), other.report(2, 1000)])
            .await,
        409,
    )
    .await;
    assert!(
        other.list(&app).await["matches"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    // Overlapping concurrent batches in opposite input orders must finish
    // without deadlock; only one event owns both IDs.
    let (a, b) = tokio::join!(
        game.upload(&app, vec![game.report(3, 1000), game.report(4, 1000)]),
        other.upload(&app, vec![other.report(4, 1000), other.report(3, 1000)]),
    );
    let statuses = [a.status().as_u16(), b.status().as_u16()];
    assert!(statuses == [204, 409] || statuses == [409, 204]);
    let owners: Vec<Uuid> =
        sqlx::query_scalar("SELECT DISTINCT event_id FROM quickdraw_matches WHERE id IN ($1, $2)")
            .bind(Uuid::from_u128(3))
            .bind(Uuid::from_u128(4))
            .fetch_all(&app.db_pool)
            .await
            .unwrap();
    assert_eq!(owners.len(), 1);
}

#[tokio::test]
async fn ingestion_waits_for_scoring_changes_and_reads_the_committed_period() {
    let app = spawn_app().await;
    let game = Game::create(&app).await;
    game.period(&app, 1000, None).await;
    let mut transaction = app.db_pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM events WHERE id = $1 FOR UPDATE")
        .bind(game.event)
        .execute(&mut *transaction)
        .await
        .unwrap();
    sqlx::query("UPDATE event_scoring_periods SET closes_at = $1 WHERE event_id = $2")
        .bind(DateTime::from_timestamp_millis(2000).unwrap())
        .bind(game.event)
        .execute(&mut *transaction)
        .await
        .unwrap();
    let request = app
        .request(Method::POST, &game.path())
        .header("Content-Type", "application/protobuf")
        .body(
            WriteQuickdrawMatchesRequest {
                matches: vec![game.report(1, 2000)],
            }
            .encode_to_vec(),
        );
    let mut upload = tokio::spawn(async move { request.send().await.unwrap() });
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(150), &mut upload)
            .await
            .is_err()
    );
    transaction.commit().await.unwrap();
    assert_eq!(upload.await.unwrap().status(), 204);
    assert_eq!(
        game.list(&app).await["matches"][0]["scoring_status"],
        "scoring_disabled"
    );
}
