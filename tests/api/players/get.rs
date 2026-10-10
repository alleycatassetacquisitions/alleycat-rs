use crate::helpers::spawn_app_with_event as spawn_app;
use reqwest::header::CONTENT_TYPE;
use serde_json::Value;

#[tokio::test]
async fn player_is_retrieved_with_a_200() {
    let app = spawn_app().await;
    let body = "name=Martha%20Wells&email=martha%40example.com";
    let post_response = app.post_players(body.into()).await;
    assert_eq!(200, post_response.status().as_u16());

    let get_response = app.get_players().await;

    assert_eq!(200, get_response.status().as_u16());
    assert_eq!(
        get_response.headers().get(CONTENT_TYPE).unwrap(),
        "application/json"
    );

    let body = get_response
        .json::<Value>()
        .await
        .expect("Response was not valid JSON.");

    assert_eq!(body["pagination"]["page"], 1);
    assert_eq!(body["pagination"]["per_page"], 20);

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
    let app = spawn_app().await;
    assert_eq!(app.post_players("name=Nyx".into()).await.status(), 200);
    let event_id = crate::helpers::activate_new_event(&app).await;
    let empty: Value = app.get_players().await.json().await.unwrap();
    assert_eq!(empty["players"].as_array().unwrap().len(), 0);
    assert_eq!(app.post_players("name=Nyx".into()).await.status(), 200);
    let body: Value = app.get_players().await.json().await.unwrap();
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
    let empty: Value = app.get_players().await.json().await.unwrap();
    assert_eq!(empty["players"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn development_seed_is_repeatable_and_registration_works_afterward() {
    let app = crate::helpers::spawn_app().await;
    for _ in 0..2 {
        sqlx::raw_sql(include_str!("../../../xtask/seed_db.sql"))
            .execute(&app.db_pool)
            .await
            .unwrap();
    }
    assert_eq!(app.post_players("name=Newcomer".into()).await.status(), 200);
    let code: String = sqlx::query_scalar("SELECT pdn_code FROM players WHERE name = 'Newcomer'")
        .fetch_one(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(code, "0506");
    let other_event = crate::helpers::activate_new_event(&app).await;
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
    let app = crate::helpers::spawn_app().await;
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
