use crate::helpers::{assert_json, spawn_app, spawn_app_with_event};
use alleycat_rs::proto::alleycat::device::DevicePlayerResponse;
use prost::Message;
use reqwest::Method;
use serde_json::{Value, json};

#[tokio::test]
async fn lookup_by_id_or_pin_uses_the_active_event_and_device_responses_omit_email() {
    let app = spawn_app_with_event().await;
    let response = app
        .post_players(json!({"name": "Alex", "email": "private-address@example.com"}))
        .await;
    let first: Value = assert_json(response, 200).await;
    let pin = first["pdn_code"].as_str().unwrap();
    assert_eq!(pin, "0001");
    let id_path = format!("/players/{}", first["id"].as_str().unwrap());
    let pin_path = format!("/players/by-pin/{pin}");
    let device_path = format!("/device/players/by-pin/{pin}");
    for path in [&id_path, &pin_path] {
        let response = app.get(path).await;
        let player: Value = assert_json(response, 200).await;
        assert_eq!(player, first);
    }
    let response = app.get(&device_path).await;
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()["content-type"], "application/protobuf");
    let bytes = response.bytes().await.unwrap();
    assert!(
        !bytes
            .windows(b"private-address@example.com".len())
            .any(|window| window == b"private-address@example.com")
    );
    let device = DevicePlayerResponse::decode(bytes).unwrap();
    assert_eq!(device.id, first["id"].as_str().unwrap());
    assert_eq!(device.event_id, first["event_id"].as_str().unwrap());
    assert_eq!(device.pdn_code, pin);
    assert_eq!(device.name, "Alex");
    assert_eq!(device.team_id, None);
    assert_eq!(device.team_name, None);
    assert!(device.active);
    let created: chrono::DateTime<chrono::Utc> =
        first["created_at"].as_str().unwrap().parse().unwrap();
    assert_eq!(device.created_at_unix_ms, created.timestamp_millis());

    let team_response = app
        .request_json(
            Method::POST,
            &format!("/events/{}/teams", device.event_id),
            json!({"name": "Runners"}),
        )
        .await;
    let team: Value = assert_json(team_response, 201).await;
    assert_eq!(
        app.request_json(
            Method::PATCH,
            &id_path,
            json!({"team_id": team["id"], "active": false})
        )
        .await
        .status(),
        200
    );
    assert_eq!(
        app.request_json(
            Method::PATCH,
            &format!(
                "/events/{}/teams/{}",
                device.event_id,
                team["id"].as_str().unwrap()
            ),
            json!({"name": "Renamed team"})
        )
        .await
        .status(),
        200
    );
    let device =
        DevicePlayerResponse::decode(app.get(&device_path).await.bytes().await.unwrap()).unwrap();
    assert_eq!(device.team_id.as_deref(), team["id"].as_str());
    assert_eq!(device.team_name.as_deref(), Some("Renamed team"));
    assert!(!device.active);
    let response = app.get(&pin_path).await;
    let by_pin: Value = assert_json(response, 200).await;
    assert_eq!(by_pin["team_name"], "Renamed team");
    assert_eq!(by_pin["email"], first["email"]);

    app.activate_new_event().await;
    let response = app.post_players(json!({"name": "Second"})).await;
    let second: Value = assert_json(response, 200).await;
    assert_eq!(second["pdn_code"], pin);
    assert_eq!(app.get(&id_path).await.status(), 404);
    assert_eq!(
        app.get(&pin_path).await.json::<Value>().await.unwrap(),
        second
    );
    let device =
        DevicePlayerResponse::decode(app.get(&device_path).await.bytes().await.unwrap()).unwrap();
    assert_eq!(device.id, second["id"].as_str().unwrap());
    for path in ["/players/by-pin/1", "/device/players/by-pin/abcd"] {
        assert_eq!(app.get(path).await.status(), 400);
    }
    for path in ["/players/by-pin/9876", "/device/players/by-pin/9876"] {
        assert_eq!(app.get(path).await.status(), 404);
    }
    sqlx::query("UPDATE app_state SET active_event_id = NULL")
        .execute(&app.db_pool)
        .await
        .unwrap();
    for path in [&id_path, &pin_path, &device_path] {
        assert_eq!(app.get(path).await.status(), 404);
    }
}

#[tokio::test]
async fn player_is_retrieved_with_a_200() {
    let app = spawn_app_with_event().await;
    let body = json!({"name": "Martha Wells", "email": "martha@example.com"});
    let post_response = app.post_players(body).await;
    assert_eq!(200, post_response.status().as_u16());

    let get_response = app.get_players().await;

    let body: Value = assert_json(get_response, 200).await;

    let players = body["players"]
        .as_array()
        .expect("`players` was not a JSON array.");

    assert_eq!(players.len(), 1);
    let player = &players[0];

    assert_eq!(player["name"], "Martha Wells");
    assert_eq!(player.get("team_id"), Some(&Value::Null));
    assert!(player.get("team").is_none());

    assert!(player["id"].is_string());
    assert!(player["pdn_code"].is_string());
    assert!(player["created_at"].is_string());
}

#[tokio::test]
async fn listing_is_scoped_to_active_event_and_names_and_codes_can_be_reused() {
    let app = spawn_app_with_event().await;
    assert_eq!(app.post_players(json!({"name": "Nyx"})).await.status(), 200);
    let event_id = app.activate_new_event().await;
    let response = app.get_players().await;
    let empty: Value = assert_json(response, 200).await;
    assert_eq!(empty["players"].as_array().unwrap().len(), 0);
    assert_eq!(app.post_players(json!({"name": "Nyx"})).await.status(), 200);
    let response = app.get_players().await;
    let body: Value = assert_json(response, 200).await;
    assert_eq!(body["players"].as_array().unwrap().len(), 1);
    assert_eq!(body["players"][0]["pdn_code"], "0001");
    let saved_event: uuid::Uuid = sqlx::query_scalar("SELECT event_id FROM players WHERE id = $1")
        .bind(uuid::Uuid::parse_str(body["players"][0]["id"].as_str().unwrap()).unwrap())
        .fetch_one(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(saved_event, event_id);
    sqlx::query("UPDATE app_state SET active_event_id = NULL")
        .execute(&app.db_pool)
        .await
        .unwrap();
    let response = app.get_players().await;
    let empty: Value = assert_json(response, 200).await;
    assert_eq!(empty["players"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn development_seed_is_repeatable_and_registration_works_afterward() {
    let app = spawn_app().await;
    for _ in 0..2 {
        sqlx::raw_sql(include_str!("../../../xtask/seed_db.sql"))
            .execute(&app.db_pool)
            .await
            .unwrap();
    }
    assert_eq!(
        app.post_players(json!({"name": "Newcomer"})).await.status(),
        200
    );
    let code: String = sqlx::query_scalar("SELECT pdn_code FROM players WHERE name = 'Newcomer'")
        .fetch_one(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(code, "0506");
    let other_event = app.activate_new_event().await;
    sqlx::raw_sql(include_str!("../../../xtask/seed_db.sql"))
        .execute(&app.db_pool)
        .await
        .unwrap();
    let active: Option<uuid::Uuid> =
        sqlx::query_scalar("SELECT active_event_id FROM app_state WHERE id = 1")
            .fetch_one(&app.db_pool)
            .await
            .unwrap();
    assert_eq!(active, Some(other_event));
}

#[tokio::test]
async fn development_seed_preserves_membership_after_team_rename() {
    let app = spawn_app().await;
    let seed = include_str!("../../../xtask/seed_db.sql");
    sqlx::raw_sql(seed).execute(&app.db_pool).await.unwrap();

    let original_teams: Vec<(uuid::Uuid, String)> =
        sqlx::query_as("SELECT id, name FROM teams ORDER BY name")
            .fetch_all(&app.db_pool)
            .await
            .unwrap();
    assert_eq!(
        original_teams
            .iter()
            .map(|(_, name)| name.as_str())
            .collect::<Vec<_>>(),
        ["Bounty", "Hunter"]
    );
    let memberships = "SELECT id, team_id FROM players ORDER BY id";
    let original: Vec<(uuid::Uuid, Option<uuid::Uuid>)> = sqlx::query_as(memberships)
        .fetch_all(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(original.len(), 5);
    assert_eq!(
        original.iter().filter(|(_, team)| team.is_some()).count(),
        4
    );

    sqlx::query("UPDATE teams SET name = name || ' renamed'")
        .execute(&app.db_pool)
        .await
        .unwrap();

    // Neither missing seed names nor replacement teams may change membership.
    for replacements in [false, true] {
        if replacements {
            sqlx::query(
                "INSERT INTO teams (event_id, name)
                 SELECT event_id, name FROM
                 (SELECT DISTINCT event_id FROM players) events
                 CROSS JOIN (VALUES ('Hunter'), ('Bounty')) names(name)",
            )
            .execute(&app.db_pool)
            .await
            .unwrap();
        }
        sqlx::raw_sql(seed).execute(&app.db_pool).await.unwrap();
        let actual: Vec<(uuid::Uuid, Option<uuid::Uuid>)> = sqlx::query_as(memberships)
            .fetch_all(&app.db_pool)
            .await
            .unwrap();
        assert_eq!(actual, original);
        let actual_teams: Vec<(uuid::Uuid, String)> =
            sqlx::query_as("SELECT id, name FROM teams ORDER BY name")
                .fetch_all(&app.db_pool)
                .await
                .unwrap();
        assert_eq!(actual_teams.len(), if replacements { 4 } else { 2 });
        for (id, name) in &original_teams {
            assert!(actual_teams.contains(&(*id, format!("{name} renamed"))));
        }
    }
}
