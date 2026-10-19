use crate::helpers::{TestApp, assert_json, assert_text_error, spawn_app};
use reqwest::Method;
use serde_json::{Value, json};
use uuid::Uuid;

async fn teams(app: &TestApp) -> Vec<Value> {
    let response = app.get("/teams").await;
    assert_json(response, 200).await
}

#[tokio::test]
async fn teams_start_empty_and_are_created_renamed_and_isolated_by_event() {
    let app = spawn_app().await;
    let first = app.activate_new_event().await;
    assert!(teams(&app).await.is_empty());
    let path = "/teams";
    assert_eq!(
        app.request_json(Method::POST, path, json!({"name": "Hunter"}))
            .await
            .status(),
        201
    );
    let response = app
        .request_json(Method::POST, path, json!({"name": "  Runners  "}))
        .await;
    let created: Value = assert_json(response, 201).await;
    let id = created["id"].as_str().unwrap();
    Uuid::parse_str(id).unwrap();
    assert_eq!(created["name"], "Runners");
    assert_eq!(created["event_id"], first.to_string());
    assert_text_error(
        app.request_json(Method::POST, path, json!({"name": " Runners "}))
            .await,
        409,
    )
    .await;
    let rename = format!("{path}/{id}");
    for _ in 0..2 {
        let response = app
            .request_json(Method::PATCH, &rename, json!({"name": " Sprinters "}))
            .await;
        let renamed: Value = assert_json(response, 200).await;
        assert_eq!(renamed["id"], id);
        assert_eq!(renamed["name"], "Sprinters");
    }
    assert_text_error(
        app.request_json(Method::PATCH, &rename, json!({"name": "Hunter"}))
            .await,
        409,
    )
    .await;
    let original = teams(&app).await;
    assert_eq!(
        original
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["Hunter", "Sprinters"]
    );

    let second = app.activate_new_event().await;
    assert!(teams(&app).await.is_empty());
    let response = app
        .request_json(Method::POST, path, json!({"name": "Runners"}))
        .await;
    let other: Value = assert_json(response, 201).await;
    assert_eq!(other["event_id"], second.to_string());
    assert_text_error(
        app.request_json(Method::PATCH, &rename, json!({"name": "Wrong event"}))
            .await,
        404,
    )
    .await;
    // Query parameters cannot override the active event either.
    let response = app.get(&format!("/teams?event_id={first}")).await;
    assert_eq!(assert_json::<Vec<Value>>(response, 200).await, vec![other]);
    app.select_event(first).await;
    assert_eq!(teams(&app).await, original);
}

#[tokio::test]
async fn invalid_names_and_missing_resources_do_not_mutate_teams() {
    let app = spawn_app().await;
    let event = app.activate_new_event().await;
    let path = "/teams";
    assert_eq!(
        app.request_json(Method::POST, path, json!({"name": "Runners"}))
            .await
            .status(),
        201
    );
    let initial = teams(&app).await;
    let rename = format!("{path}/{}", initial[0]["id"].as_str().unwrap());
    for body in [
        json!({}),
        json!({"name": null}),
        json!({"name": 1}),
        json!({"name": ""}),
        json!({"name": " \t\n\u{2003}"}),
        json!({"name": "bad\u{0000}name"}),
        json!({"name": "Valid", "event_id": event}),
    ] {
        assert_eq!(
            app.request_json(Method::POST, path, body.clone())
                .await
                .status(),
            400
        );
        assert_eq!(
            app.request_json(Method::PATCH, &rename, body)
                .await
                .status(),
            400
        );
    }
    for missing in [Uuid::new_v4().to_string(), "not-a-uuid".into()] {
        assert_eq!(
            app.request_json(
                Method::PATCH,
                &format!("/teams/{missing}"),
                json!({"name": "Valid"})
            )
            .await
            .status(),
            404
        );
    }
    assert_eq!(teams(&app).await, initial);
}

#[tokio::test]
async fn name_length_is_checked_for_creation_and_rename() {
    let app = spawn_app().await;
    app.activate_new_event().await;
    let path = "/teams";

    // Four-byte characters also fit at the limit; count scalars, not UTF-8 bytes.
    for (created_char, renamed_char) in [("a", "b"), ("🦀", "🐈")] {
        let name = created_char.repeat(256);
        let response = app
            .request_json(Method::POST, path, json!({"name": name}))
            .await;
        let created: Value = assert_json(response, 201).await;
        assert_eq!(created["name"], name);
        let rename = format!("{path}/{}", created["id"].as_str().unwrap());
        let name = renamed_char.repeat(256);
        let response = app
            .request_json(Method::PATCH, &rename, json!({"name": name}))
            .await;
        let renamed: Value = assert_json(response, 200).await;
        assert_eq!(renamed["name"], name);

        let before = teams(&app).await;
        for (method, target) in [(Method::POST, path), (Method::PATCH, rename.as_str())] {
            let response = app
                .request_json(method, target, json!({"name": created_char.repeat(257)}))
                .await;
            assert_eq!(response.status(), 400);
        }
        assert_eq!(teams(&app).await, before);
    }
}

#[tokio::test]
async fn membership_is_nullable_event_scoped_and_stable_across_rename() {
    let app = spawn_app().await;
    let event = app.create_event(json!({"name": "Game"})).await;
    let event = event["id"].as_str().unwrap();
    app.request_json(Method::PUT, &format!("/events/{event}/active"), json!(null))
        .await
        .error_for_status()
        .unwrap();
    assert_eq!(app.post_players(json!({"name": "Nyx"})).await.status(), 200);
    let players: Value = app.get_players().await.json().await.unwrap();
    assert_eq!(players["players"][0].get("team_id"), Some(&Value::Null));
    let player = Uuid::parse_str(players["players"][0]["id"].as_str().unwrap()).unwrap();
    assert_eq!(
        app.request_json(Method::POST, "/teams", json!({"name": "Runners"}))
            .await
            .status(),
        201
    );
    let team = teams(&app).await.remove(0);
    let team_id = Uuid::parse_str(team["id"].as_str().unwrap()).unwrap();
    let saved: Option<Uuid> = sqlx::query_scalar("SELECT team_id FROM players WHERE id = $1")
        .bind(player)
        .fetch_one(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(saved, None);
    sqlx::query("UPDATE players SET team_id = $1 WHERE id = $2")
        .bind(team_id)
        .bind(player)
        .execute(&app.db_pool)
        .await
        .unwrap();
    let renamed = app
        .request_json(
            Method::PATCH,
            &format!("/teams/{team_id}"),
            json!({"name": "Renamed default"}),
        )
        .await;
    assert_eq!(renamed.status(), 200);
    let players: Value = app.get_players().await.json().await.unwrap();
    assert_eq!(players["players"][0]["team_id"], team_id.to_string());
    let other_event = app.activate_new_event().await;
    assert!(teams(&app).await.is_empty());
    assert_eq!(
        app.request_json(Method::POST, "/teams", json!({"name": "Runners"}))
            .await
            .status(),
        201
    );
    let foreign_team = teams(&app).await.remove(0);
    let foreign_id = Uuid::parse_str(foreign_team["id"].as_str().unwrap()).unwrap();
    for query in [
        "UPDATE players SET team_id = $1 WHERE id = $2",
        "UPDATE players SET event_id = $1 WHERE id = $2",
    ] {
        let value = if query.contains("SET team_id") {
            foreign_id
        } else {
            other_event
        };
        let error = sqlx::query(query)
            .bind(value)
            .bind(player)
            .execute(&app.db_pool)
            .await
            .unwrap_err();
        assert_eq!(
            error.as_database_error().unwrap().constraint(),
            Some("players_event_team_fkey")
        );
    }
}

#[tokio::test]
async fn teams_require_an_active_event_even_when_other_events_exist() {
    let app = spawn_app().await;
    app.activate_new_event().await;
    let response = app
        .request_json(Method::POST, "/teams", json!({"name": "Runners"}))
        .await;
    let created: Value = assert_json(response, 201).await;
    for statement in [
        "UPDATE app_state SET active_event_id = NULL",
        "DELETE FROM app_state",
    ] {
        sqlx::query(statement).execute(&app.db_pool).await.unwrap();
        assert_text_error(app.get("/teams").await, 409).await;
        assert_text_error(
            app.request_json(Method::POST, "/teams", json!({"name": "Stray"}))
                .await,
            409,
        )
        .await;
        assert_text_error(
            app.request_json(
                Method::PATCH,
                &format!("/teams/{}", created["id"].as_str().unwrap()),
                json!({"name": "Stray"}),
            )
            .await,
            409,
        )
        .await;
    }
    let names: Vec<String> = sqlx::query_scalar("SELECT name FROM teams")
        .fetch_all(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(names, ["Runners"]);
}
