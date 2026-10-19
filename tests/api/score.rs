use crate::helpers::{TestApp, assert_json, assert_text_error, spawn_app};
use alleycat_rs::proto::alleycat::device::{QuickdrawMatch, WriteQuickdrawMatchesRequest};
use reqwest::Method;
use serde_json::{Value, json};
use uuid::Uuid;

struct Game {
    event: Uuid,
    players: [Uuid; 4],
    teams: [Uuid; 2],
}

impl Game {
    async fn create(app: &TestApp) -> Self {
        let game = Self {
            event: app.activate_new_event().await,
            players: std::array::from_fn(|_| Uuid::new_v4()),
            teams: std::array::from_fn(|_| Uuid::new_v4()),
        };
        for (i, team) in game.teams.iter().enumerate() {
            sqlx::query("INSERT INTO teams (id, event_id, name) VALUES ($1, $2, $3)")
                .bind(team)
                .bind(game.event)
                .bind(format!("Team {i}"))
                .execute(&app.db_pool)
                .await
                .unwrap();
        }
        for (i, name) in ["Alex", "Blair", "Casey", "Drew"].iter().enumerate() {
            sqlx::query(
                "INSERT INTO players (id, event_id, name, pdn_code, created_at, team_id)
                         VALUES ($1, $2, $3, $4, now(), $5)",
            )
            .bind(game.players[i])
            .bind(game.event)
            .bind(name)
            .bind(format!("{i:04}"))
            .bind(if i == 3 {
                None
            } else {
                Some(game.teams[i % 2])
            })
            .execute(&app.db_pool)
            .await
            .unwrap();
        }
        game
    }

    async fn save_match(
        &self,
        app: &TestApp,
        participants: [usize; 2],
        times: [i64; 2],
        winner: usize,
        status: &str,
    ) {
        let teams = match status {
            "same_team" => [Some(self.teams[0]), Some(self.teams[0])],
            "unassigned_team" => [None, Some(self.teams[1])],
            _ => [Some(self.teams[0]), Some(self.teams[1])],
        };
        sqlx::query("INSERT INTO quickdraw_matches
            (id, event_id, player_a_id, player_b_id, player_a_team_id, player_b_team_id,
             player_a_draw_time_ms, player_b_draw_time_ms, winner_player_id, occurred_at, scoring_status)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, now(), $10::quickdraw_scoring_status)")
            .bind(Uuid::new_v4()).bind(self.event)
            .bind(self.players[participants[0]]).bind(self.players[participants[1]])
            .bind(teams[0]).bind(teams[1]).bind(times[0]).bind(times[1])
            .bind(self.players[winner]).bind(status)
            .execute(&app.db_pool).await.unwrap();
    }
}

async fn player_scores(app: &TestApp, query: &str) -> Value {
    assert_json(app.get(&format!("/score/players{query}")).await, 200).await
}

#[tokio::test]
async fn scoring_results_determine_totals_averages_and_shared_ranks_before_pagination() {
    let app = spawn_app().await;
    let game = Game::create(&app).await;
    game.save_match(&app, [0, 1], [0, 100], 0, "scoring").await;
    // Winner is authoritative even when slower; both participant slots count.
    game.save_match(&app, [1, 0], [200, 301], 0, "scoring")
        .await;
    game.save_match(&app, [0, 1], [602, 300], 1, "scoring")
        .await;
    game.save_match(&app, [1, 2], [402, 500], 1, "scoring")
        .await;
    for status in [
        "same_team",
        "unassigned_team",
        "scoring_disabled",
        "delayed_scores_disabled",
    ] {
        game.save_match(&app, [0, 2], [9999, 9999], 2, status).await;
    }
    // A player with only non-scoring history still has no measured average here.
    game.save_match(&app, [0, 3], [9999, 9999], 3, "scoring_disabled")
        .await;

    let body = player_scores(&app, "").await;
    assert_eq!(body["event_id"], game.event.to_string());
    let players = body["players"].as_array().unwrap();
    assert_eq!(players.len(), 4);
    for (i, (score, count, average, rank)) in [
        (2, 3, json!(301.0), 1),
        (2, 4, json!(250.5), 1),
        (0, 1, json!(500.0), 3),
        (0, 0, Value::Null, 3),
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(players[i]["id"], game.players[i].to_string());
        assert_eq!(players[i]["score"], score);
        assert_eq!(players[i]["matches_played"], count);
        assert_eq!(players[i]["average_draw_time_ms"], average);
        assert_eq!(players[i]["rank"], rank);
    }
    assert_eq!(players[0]["team_id"], game.teams[0].to_string());
    assert_eq!(players[0]["team_name"], "Team 0");
    assert_eq!(players[3]["team_id"], Value::Null);
    assert_eq!(players[3]["team_name"], Value::Null);

    // A faster average does not break a score tie, including across page boundaries.
    let page = player_scores(&app, "?page=2&per_page=1").await;
    assert_eq!(page["players"], json!([players[1]]));
    assert_eq!(page["pagination"], json!({"page": 2, "per_page": 1}));
    let page = player_scores(&app, "?page=2&per_page=2").await;
    assert_eq!(page["players"], json!(players[2..]));
    assert_eq!(
        player_scores(&app, "?page=3&per_page=2").await["players"],
        json!([])
    );
}

#[tokio::test]
async fn current_team_and_registration_changes_preserve_earned_results() {
    let app = spawn_app().await;
    let game = Game::create(&app).await;
    game.save_match(&app, [0, 1], [300, 400], 0, "scoring")
        .await;
    let original = player_scores(&app, "").await["players"][0].clone();
    let response = app
        .request_json(
            Method::PATCH,
            &format!("/players/{}", game.players[0]),
            json!({"team_id": game.teams[1], "active": false}),
        )
        .await;
    assert_eq!(response.status(), 200);
    let response = app
        .request_json(
            Method::PATCH,
            &format!("/teams/{}", game.teams[1]),
            json!({"name": "Renamed"}),
        )
        .await;
    assert_eq!(response.status(), 200);
    let mut expected = original;
    expected["team_id"] = json!(game.teams[1]);
    expected["team_name"] = json!("Renamed");
    assert_eq!(player_scores(&app, "").await["players"][0], expected);

    let response = app
        .request_json(
            Method::PATCH,
            &format!("/players/{}", game.players[0]),
            json!({"team_id": null}),
        )
        .await;
    assert_eq!(response.status(), 200);
    expected["team_id"] = Value::Null;
    expected["team_name"] = Value::Null;
    assert_eq!(player_scores(&app, "").await["players"][0], expected);
}

#[tokio::test]
async fn selection_is_required_and_other_events_cannot_leak_into_the_player_scores() {
    let app = spawn_app().await;
    assert_text_error(app.get("/score/players").await, 409).await;
    let first = Game::create(&app).await;
    first.save_match(&app, [0, 1], [10, 20], 0, "scoring").await;
    let original = player_scores(&app, "").await;
    let empty = app.activate_new_event().await;
    assert_eq!(player_scores(&app, "").await["players"], json!([]));
    let second = Game::create(&app).await;
    second
        .save_match(&app, [0, 1], [1000, 2000], 1, "scoring")
        .await;
    app.select_event(first.event).await;
    assert_eq!(
        player_scores(&app, &format!("?event_id={}", second.event)).await,
        original
    );
    app.select_event(empty).await;
    assert_eq!(player_scores(&app, "").await["event_id"], empty.to_string());
    for query in ["page=abc", "per_page=-1"] {
        assert_eq!(
            app.get(&format!("/score/players?{query}")).await.status(),
            400
        );
    }
    for statement in [
        "UPDATE app_state SET active_event_id = NULL",
        "DELETE FROM app_state",
    ] {
        sqlx::query(statement).execute(&app.db_pool).await.unwrap();
        assert_text_error(app.get("/score/players").await, 409).await;
    }
}

#[tokio::test]
async fn late_uploads_refresh_scores_but_retries_and_switch_changes_do_not_rescore() {
    let app = spawn_app().await;
    let game = Game::create(&app).await;
    sqlx::query(
        "INSERT INTO event_scoring_periods (event_id, started_at, closes_at)
                 VALUES ($1, '2026-10-01T00:00:00Z', '2026-10-02T00:00:00Z')",
    )
    .bind(game.event)
    .execute(&app.db_pool)
    .await
    .unwrap();
    let mut report = QuickdrawMatch {
        id: Uuid::new_v4().to_string(),
        player_a_id: game.players[0].to_string(),
        player_b_id: game.players[1].to_string(),
        player_a_team_id: Some(game.teams[0].to_string()),
        player_b_team_id: Some(game.teams[1].to_string()),
        player_a_draw_time_ms: Some(i64::MAX as u64),
        player_b_draw_time_ms: Some(0),
        winner_player_id: game.players[0].to_string(),
        occurred_at_unix_ms: Some(1_790_812_800_000), // 2026-10-01T00:00:00Z
    };
    for _ in 0..2 {
        report.id = Uuid::new_v4().to_string();
        assert_eq!(
            app.post_protobuf(
                "/quickdraw-matches",
                &WriteQuickdrawMatchesRequest {
                    matches: vec![report.clone()],
                }
            )
            .await
            .status(),
            204
        );
    }
    let original = player_scores(&app, "").await;
    assert_eq!(original["players"][0]["score"], 2);
    assert_eq!(original["players"][0]["matches_played"], 2);
    // Averaging two maximum valid times must not overflow a bigint sum.
    assert_eq!(
        original["players"][0]["average_draw_time_ms"],
        json!(i64::MAX as f64)
    );
    assert_eq!(original["players"][1]["average_draw_time_ms"], 0.0);
    assert_eq!(
        app.request_json(Method::PUT, "/delayed-scores", json!({"enabled": false}))
            .await
            .status(),
        200
    );
    report.winner_player_id = game.players[1].to_string();
    report.player_a_draw_time_ms = Some(1);
    assert_eq!(
        app.post_protobuf(
            "/quickdraw-matches",
            &WriteQuickdrawMatchesRequest {
                matches: vec![report.clone()],
            }
        )
        .await
        .status(),
        204
    );
    report.id = Uuid::new_v4().to_string();
    assert_eq!(
        app.post_protobuf(
            "/quickdraw-matches",
            &WriteQuickdrawMatchesRequest {
                matches: vec![report],
            }
        )
        .await
        .status(),
        204
    );
    assert_eq!(player_scores(&app, "").await, original);
}
