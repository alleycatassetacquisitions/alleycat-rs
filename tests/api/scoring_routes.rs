use crate::helpers::{TestApp, assert_json, assert_text_error, spawn_app};
use chrono::{DateTime, Utc};
use reqwest::Method;
use serde_json::{Value, json};
use uuid::Uuid;

async fn set(app: &TestApp, enabled: bool) -> Value {
    let response = app
        .request_json(Method::PUT, "/scoring", json!({"enabled": enabled}))
        .await;
    assert_json(response, 200).await
}

#[tokio::test]
async fn scoring_controls_preserve_history_and_retries_are_noops() {
    let app = spawn_app().await;
    let event = app.activate_new_event().await;
    let path = "/scoring";
    let disabled = json!({"enabled": false, "started_at": null});
    assert_eq!(app.get(path).await.json::<Value>().await.unwrap(), disabled);
    assert_eq!(set(&app, false).await, disabled);
    let opened = set(&app, true).await;
    assert_eq!(opened["enabled"], true);
    assert!(opened["started_at"].is_string());
    assert_eq!(set(&app, true).await, opened);
    assert_eq!(app.get(path).await.json::<Value>().await.unwrap(), opened);
    assert_eq!(set(&app, false).await, disabled);
    let first: (Uuid, DateTime<Utc>, Option<DateTime<Utc>>) = sqlx::query_as(
        "SELECT id, started_at, closes_at FROM event_scoring_periods WHERE event_id = $1",
    )
    .bind(event)
    .fetch_one(&app.db_pool)
    .await
    .unwrap();
    assert!(first.2.unwrap() > first.1);
    assert_eq!(set(&app, false).await, disabled);
    set(&app, true).await;
    let periods: Vec<(Uuid, DateTime<Utc>, Option<DateTime<Utc>>)> = sqlx::query_as(
        "SELECT id, started_at, closes_at FROM event_scoring_periods WHERE event_id = $1 ORDER BY started_at",
    )
    .bind(event)
    .fetch_all(&app.db_pool)
    .await
    .unwrap();
    assert_eq!(periods.len(), 2);
    assert_eq!(periods[0], first);
    assert!(periods[1].1 >= first.2.unwrap());
    assert_eq!(periods[1].2, None);
}

#[tokio::test]
async fn concurrent_enable_and_disable_requests_are_idempotent() {
    let app = spawn_app().await;
    let event = app.activate_new_event().await;
    let (a, b, c) = tokio::join!(set(&app, true), set(&app, true), set(&app, true),);
    assert_eq!(a, b);
    assert_eq!(b, c);
    let (a, b, c) = tokio::join!(set(&app, false), set(&app, false), set(&app, false),);
    assert_eq!(a, b);
    assert_eq!(b, c);
    let periods: Vec<Option<DateTime<Utc>>> =
        sqlx::query_scalar("SELECT closes_at FROM event_scoring_periods WHERE event_id = $1")
            .bind(event)
            .fetch_all(&app.db_pool)
            .await
            .unwrap();
    assert_eq!(periods.len(), 1);
    assert!(periods[0].is_some());
}

#[tokio::test]
async fn scoring_is_event_scoped_and_rejects_invalid_requests() {
    let app = spawn_app().await;
    let event = app.activate_new_event().await;
    set(&app, true).await;
    app.activate_new_event().await;
    let other_state: Value = assert_json(app.get("/scoring").await, 200).await;
    assert_eq!(other_state["enabled"], false);
    app.select_event(event).await;
    let path = "/scoring";
    for body in [
        json!({}),
        json!({"enabled": null}),
        json!({"enabled": "true"}),
        json!({"enabled": true, "started_at": "2026-10-10T00:00:00Z"}),
        json!({"enabled": false, "event_id": event}),
    ] {
        assert_text_error(app.request_json(Method::PUT, path, body).await, 400).await;
    }
    assert_eq!(
        app.get(path).await.json::<Value>().await.unwrap()["enabled"],
        true
    );
    sqlx::query("UPDATE app_state SET active_event_id = NULL")
        .execute(&app.db_pool)
        .await
        .unwrap();
    assert_text_error(app.get(path).await, 409).await;
    assert_text_error(
        app.request_json(Method::PUT, path, json!({"enabled": false}))
            .await,
        409,
    )
    .await;
    app.select_event(event).await;
    assert_eq!(
        assert_json::<Value>(app.get(path).await, 200).await["enabled"],
        true
    );
}

#[tokio::test]
async fn event_switch_waits_for_scoring_controls_to_commit() {
    for (path, enabled) in [("/scoring", true), ("/delayed-scores", false)] {
        let app = spawn_app().await;
        let first = app.activate_new_event().await;
        let second = app.create_event(json!({"name": "Second"})).await;
        let second = Uuid::parse_str(second["id"].as_str().unwrap()).unwrap();
        // Hold the event row as an upload would. The control request must keep
        // the selection pinned while it waits, then save to the original event.
        let mut upload = app.db_pool.begin().await.unwrap();
        sqlx::query("SELECT id FROM events WHERE id = $1 FOR UPDATE")
            .bind(first)
            .fetch_one(&mut *upload)
            .await
            .unwrap();
        let request = app
            .request(Method::PUT, path)
            .json(&json!({"enabled": enabled}));
        let edit = tokio::spawn(async move { request.send().await.unwrap() });
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                let locked: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_locks WHERE relation = 'app_state'::regclass AND mode = 'RowShareLock' AND granted)")
                    .fetch_one(&app.db_pool).await.unwrap();
                if locked { break; }
                tokio::task::yield_now().await;
            }
        }).await.unwrap();
        let request = app.request(Method::PUT, &format!("/events/{second}/active"));
        let mut switch = tokio::spawn(async move { request.send().await.unwrap() });
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(100), &mut switch)
                .await
                .is_err()
        );
        upload.commit().await.unwrap();
        assert_eq!(edit.await.unwrap().status(), 200);
        assert_eq!(switch.await.unwrap().status(), 204);

        let state: Value = assert_json(app.get("/game-state").await, 200).await;
        assert_eq!(state["active_event"]["id"], second.to_string());
        assert_eq!(state["scoring"]["enabled"], false);
        assert_eq!(state["accepting_delayed_scores"], true);
        app.select_event(first).await;
        let state: Value = assert_json(app.get("/game-state").await, 200).await;
        let setting = if path == "/scoring" {
            &state["scoring"]["enabled"]
        } else {
            &state["accepting_delayed_scores"]
        };
        assert_eq!(setting, &json!(enabled));
    }
}
