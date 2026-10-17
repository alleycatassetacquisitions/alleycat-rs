use crate::helpers::{assert_json, spawn_app, spawn_app_with_event};
use serde_json::json;

async fn assert_registration_conflict(response: reqwest::Response, message: &str) {
    assert_eq!(response.status(), 409);
    assert!(response.headers().get("content-type").is_none());
    assert_eq!(response.text().await.unwrap(), message);
}

#[tokio::test]
async fn register_player_returns_a_200_for_valid_json() {
    let app = spawn_app_with_event().await;
    let body = json!({"name": "le guin", "email": "ursula_le_guin@gmail.com"});
    let response = app.post_players(body).await;

    let created: serde_json::Value = assert_json(response, 200).await;
    let saved = sqlx::query!("SELECT name, email, pdn_code FROM players",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.email, Some("ursula_le_guin@gmail.com".to_string()));
    assert_eq!(saved.pdn_code.len(), 4);
    assert!(saved.pdn_code.chars().all(|c| c.is_ascii_digit()));
    assert_eq!(created["name"], saved.name);
    assert_eq!(created["email"], saved.email.unwrap());
    assert_eq!(created["pdn_code"], saved.pdn_code);
    assert_eq!(created["active"], true);
    assert!(created["id"].is_string());
    assert!(created["event_id"].is_string());
    assert!(created["created_at"].is_string());
    assert_eq!(created.get("team_id"), Some(&serde_json::Value::Null));
    assert_eq!(created.get("team_name"), Some(&serde_json::Value::Null));
    let response = app.get_players().await;
    let roster: serde_json::Value = assert_json(response, 200).await;
    assert_eq!(roster["players"][0], created);
}

#[tokio::test]
async fn register_player_accepts_an_omitted_or_null_email() {
    let app = spawn_app_with_event().await;
    for body in [
        json!({"name": "Ursula"}),
        json!({"name": "Octavia", "email": null}),
    ] {
        let response = app.post_players(body.clone()).await;
        let player: serde_json::Value = assert_json(response, 200).await;
        assert_eq!(player["name"], body["name"]);
        assert_eq!(player.get("email"), Some(&serde_json::Value::Null));
    }

    let saved = sqlx::query!("SELECT name, email FROM players")
        .fetch_all(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(saved.len(), 2);
    for player in saved {
        assert!(["Ursula", "Octavia"].contains(&player.name.as_str()));
        assert_eq!(player.email, None);
    }
}

#[tokio::test]
async fn registration_rejects_form_data_and_malformed_json_without_creating_players() {
    let app = spawn_app_with_event().await;
    for (content_type, body) in [
        ("application/x-www-form-urlencoded", "name=Alex"),
        ("text/plain", r#"{"name":"Alex"}"#),
        ("application/json", "{broken"),
        ("application/json", ""),
    ] {
        let response = app
            .post_bytes("/players", content_type, body.as_bytes().to_vec())
            .await;
        assert_eq!(response.status(), 400, "{content_type}: {body}");
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM players")
        .fetch_one(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn register_player_returns_a_400_when_fields_are_present_but_invalid() {
    let app = spawn_app_with_event().await;
    let test_cases = vec![
        (
            json!({"name": "", "email": "ursula_le_guin@example.com"}),
            "empty name",
        ),
        (
            json!({"name": "Ursula", "email": "definitely-not-an-email"}),
            "invalid email",
        ),
        (json!({"name": null}), "null name"),
        (json!({"name": 42}), "numeric name"),
        (json!({"name": "Ursula", "email": 42}), "numeric email"),
    ];
    for (body, description) in test_cases {
        let response = app.post_players(body).await;
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 400 Bad Request when the payload was {}.",
            description
        );
    }
}

#[tokio::test]
async fn register_player_returns_a_400_when_data_is_missing() {
    let app = spawn_app_with_event().await;
    let test_cases = vec![
        (json!({}), "missing the name"),
        (json!({"score": 100}), "invalid parameter"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = app.post_players(invalid_body).await;
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}

#[tokio::test]
async fn registration_requires_an_active_event() {
    let app = spawn_app().await;
    assert_registration_conflict(
        app.post_players(json!({"name": "Nyx"})).await,
        "No active event",
    )
    .await;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM players")
        .fetch_one(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn allocation_skips_reserved_and_used_codes_and_rolls_back_on_duplicate_name() {
    let app = spawn_app_with_event().await;
    sqlx::query("INSERT INTO reserved_pdn_codes (code) VALUES ('0002')")
        .execute(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(app.post_players(json!({"name": "Nyx"})).await.status(), 200);
    assert_registration_conflict(
        app.post_players(json!({"name": "Nyx"})).await,
        "Player name already exists in the active event",
    )
    .await;
    assert_eq!(
        app.post_players(json!({"name": "Rook"})).await.status(),
        200
    );
    // Simulate an imported player ahead of the counter.
    sqlx::query("UPDATE events SET next_pdn_code = 1")
        .execute(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(
        app.post_players(json!({"name": "Echo"})).await.status(),
        200
    );
    let codes: Vec<String> = sqlx::query_scalar("SELECT pdn_code FROM players ORDER BY pdn_code")
        .fetch_all(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(codes, ["0001", "0003", "0004"]);
}

#[tokio::test]
async fn exhausted_event_returns_conflict_without_creating_a_player() {
    let app = spawn_app_with_event().await;
    sqlx::query("UPDATE events SET next_pdn_code = 9998")
        .execute(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(
        app.post_players(json!({"name": "Last"})).await.status(),
        200
    );
    // 9999 is reserved.
    assert_registration_conflict(
        app.post_players(json!({"name": "Overflow"})).await,
        "No PDN codes available for the active event",
    )
    .await;
    sqlx::query("UPDATE events SET next_pdn_code = 10000")
        .execute(&app.db_pool)
        .await
        .unwrap();
    assert_registration_conflict(
        app.post_players(json!({"name": "Overflow"})).await,
        "No PDN codes available for the active event",
    )
    .await;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM players")
        .fetch_one(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn concurrent_registrations_allocate_distinct_sequential_codes() {
    let app = spawn_app_with_event().await;
    let (first, second) = tokio::join!(
        app.post_players(json!({"name": "Nyx"})),
        app.post_players(json!({"name": "Rook"}))
    );
    assert_eq!(first.status(), 200);
    assert_eq!(second.status(), 200);
    let first: serde_json::Value = first.json().await.unwrap();
    let second: serde_json::Value = second.json().await.unwrap();
    assert_eq!(first["name"], "Nyx");
    assert_eq!(second["name"], "Rook");
    assert_ne!(first["id"], second["id"]);
    assert_ne!(first["pdn_code"], second["pdn_code"]);
    let codes: Vec<String> = sqlx::query_scalar("SELECT pdn_code FROM players ORDER BY pdn_code")
        .fetch_all(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(codes, ["0001", "0002"]);
}
