use crate::helpers::{TestApp, assert_json, assert_text_error, spawn_app_with_event};
use reqwest::Method;
use serde_json::{Value, json};
use uuid::Uuid;

async fn register(app: &TestApp, name: &str) -> Value {
    let response = app
        .post_players(format!("name={name}&email=alex%40example.com"))
        .await;
    assert_json(response, 200).await
}

async fn team(app: &TestApp, event: &str, name: &str) -> Value {
    let response = app
        .request_json(
            Method::POST,
            &format!("/events/{event}/teams"),
            json!({"name": name}),
        )
        .await;
    assert_json(response, 201).await
}

#[tokio::test]
async fn players_can_edit_clear_and_restore_fields_without_changing_identity() {
    let app = spawn_app_with_event().await;
    let created = register(&app, "Alex").await;
    let id = created["id"].as_str().unwrap();
    let event = created["event_id"].as_str().unwrap();
    let path = format!("/players/{id}");
    let first_team = team(&app, event, "Runners").await;
    let second_team = team(&app, event, "Couriers").await;
    let response = app
        .request_json(
            Method::PATCH,
            &path,
            json!({"name": "Alex Updated", "team_id": first_team["id"], "active": false}),
        )
        .await;
    let updated: Value = assert_json(response, 200).await;
    assert_eq!(updated["name"], "Alex Updated");
    assert_eq!(updated["email"], created["email"]);
    assert_eq!(updated["team_id"], first_team["id"]);
    assert_eq!(updated["team_name"], "Runners");
    assert_eq!(updated["active"], false);
    for field in ["id", "event_id", "pdn_code", "created_at"] {
        assert_eq!(updated[field], created[field]);
    }
    let response = app.get_players().await;
    let roster: Value = assert_json(response, 200).await;
    assert_eq!(roster["players"][0], updated);
    assert_eq!(app.get(&path).await.json::<Value>().await.unwrap(), updated);
    let replacement = app
        .request_json(
            Method::PATCH,
            &path,
            json!({"team_id": second_team["id"], "email": "new@example.com"}),
        )
        .await;
    let replacement: Value = assert_json(replacement, 200).await;
    assert_eq!(replacement["team_name"], "Couriers");
    assert_eq!(replacement["email"], "new@example.com");
    assert_eq!(replacement["active"], false);
    let response = app
        .request_json(
            Method::PATCH,
            &path,
            json!({"email": null, "team_id": null, "active": true}),
        )
        .await;
    let cleared: Value = assert_json(response, 200).await;
    assert_eq!(cleared["email"], Value::Null);
    assert_eq!(cleared["team_id"], Value::Null);
    assert_eq!(cleared["team_name"], Value::Null);
    assert_eq!(cleared["active"], true);
    assert_eq!(cleared["name"], updated["name"]);
    assert_eq!(cleared["pdn_code"], created["pdn_code"]);
}

#[tokio::test]
async fn invalid_updates_and_conflicts_leave_the_player_unchanged() {
    let app = spawn_app_with_event().await;
    let created = register(&app, "Alex").await;
    register(&app, "Taken").await;
    let path = format!("/players/{}", created["id"].as_str().unwrap());
    let other_event = app.create_event(json!({"name": "Other"})).await;
    let other_team = team(&app, other_event["id"].as_str().unwrap(), "Other team").await;
    for body in [
        json!({}),
        json!({"name": null}),
        json!({"name": " "}),
        json!({"name": "a\u{0000}"}),
        json!({"name": "a".repeat(257)}),
        json!({"email": "bad"}),
        json!({"active": null}),
        json!({"active": "false"}),
        json!({"team_id": "bad"}),
        json!({"team_id": Uuid::new_v4()}),
        json!({"team_id": other_team["id"]}),
        json!({"id": Uuid::new_v4()}),
        json!({"event_id": other_event["id"]}),
        json!({"pdn_code": "1234"}),
        json!({"created_at": "2026-01-01T00:00:00Z"}),
        json!({"mode": "hunter"}),
    ] {
        assert_eq!(
            app.request_json(Method::PATCH, &path, body.clone())
                .await
                .status(),
            400,
            "{body}"
        );
        assert_eq!(app.get(&path).await.json::<Value>().await.unwrap(), created);
    }
    let error = assert_text_error(
        app.request_json(
            Method::PATCH,
            &path,
            json!({"name": "Taken", "email": null}),
        )
        .await,
        409,
    )
    .await;
    assert_eq!(error, "Player name already exists in the active event");
    assert_eq!(app.get(&path).await.json::<Value>().await.unwrap(), created);
    assert_text_error(
        app.request_json(
            Method::PATCH,
            &format!("/players/{}", Uuid::new_v4()),
            json!({"active": false}),
        )
        .await,
        404,
    )
    .await;

    app.activate_new_event().await;
    assert_eq!(
        app.request_json(Method::PATCH, &path, json!({"active": false}))
            .await
            .status(),
        404
    );
    sqlx::query("UPDATE app_state SET active_event_id = NULL")
        .execute(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(
        app.request_json(Method::PATCH, &path, json!({"active": false}))
            .await
            .status(),
        409
    );
}

#[tokio::test]
async fn concurrent_partial_edits_preserve_each_others_fields_and_inactive_identity() {
    let app = spawn_app_with_event().await;
    let created = register(&app, "Alex").await;
    let path = format!("/players/{}", created["id"].as_str().unwrap());
    let (name, email) = tokio::join!(
        app.request_json(Method::PATCH, &path, json!({"name": "Renamed"})),
        app.request_json(
            Method::PATCH,
            &path,
            json!({"email": "new@example.com", "active": false})
        )
    );
    assert_eq!(name.status(), 200);
    assert_eq!(email.status(), 200);
    let response = app.get(&path).await;
    let updated: Value = assert_json(response, 200).await;
    assert_eq!(updated["name"], "Renamed");
    assert_eq!(updated["email"], "new@example.com");
    assert_eq!(updated["active"], false);
    assert_eq!(app.post_players("name=Renamed".into()).await.status(), 409);
    let response = app.post_players("name=Next".into()).await;
    let next: Value = assert_json(response, 200).await;
    assert_ne!(next["pdn_code"], created["pdn_code"]);
}

#[tokio::test]
async fn active_event_switch_waits_for_a_player_edit_to_commit() {
    let app = spawn_app_with_event().await;
    let created = register(&app, "Alex").await;
    let id = Uuid::parse_str(created["id"].as_str().unwrap()).unwrap();
    let second = app.create_event(json!({"name": "Second"})).await;
    // Hold the player row so PATCH acquires its active-event share lock and waits.
    let mut transaction = app.db_pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM players WHERE id = $1 FOR UPDATE")
        .bind(id)
        .fetch_one(&mut *transaction)
        .await
        .unwrap();
    let edit_request = app
        .request(Method::PATCH, &format!("/players/{id}"))
        .json(&json!({"name": "Updated"}));
    let mut edit = tokio::spawn(async move { edit_request.send().await.unwrap() });
    // Wait for the edit's lock on app_state, rather than relying on task timing.
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let locked: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_locks WHERE relation = 'app_state'::regclass AND mode = 'RowShareLock' AND granted)")
                .fetch_one(&app.db_pool).await.unwrap();
            if locked { break; }
            tokio::task::yield_now().await;
        }
    }).await.unwrap();
    let switch_request = app.request(
        Method::PUT,
        &format!("/events/{}/active", second["id"].as_str().unwrap()),
    );
    let mut switch = tokio::spawn(async move { switch_request.send().await.unwrap() });
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), &mut edit)
            .await
            .is_err()
    );
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), &mut switch)
            .await
            .is_err()
    );
    transaction.commit().await.unwrap();
    assert_eq!(edit.await.unwrap().status(), 200);
    assert_eq!(switch.await.unwrap().status(), 204);
    let name: String = sqlx::query_scalar("SELECT name FROM players WHERE id = $1")
        .bind(id)
        .fetch_one(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(name, "Updated");
}
