use crate::helpers::{TestApp, assert_text_error, spawn_app};
use chrono::{DateTime, Utc};
use reqwest::Method;
use serde_json::{Value, json};
use uuid::Uuid;

async fn set(app: &TestApp, event: Uuid, enabled: bool) -> Value {
    let response = app
        .request_json(
            Method::PUT,
            &format!("/events/{event}/scoring"),
            json!({"enabled": enabled}),
        )
        .await;
    assert_eq!(response.status(), 200);
    response.json().await.unwrap()
}

#[tokio::test]
async fn scoring_controls_preserve_history_and_retries_are_noops() {
    let app = spawn_app().await;
    let event = app.activate_new_event().await;
    let path = format!("/events/{event}/scoring");
    let disabled = json!({"enabled": false, "started_at": null});
    assert_eq!(
        app.get(&path).await.json::<Value>().await.unwrap(),
        disabled
    );
    assert_eq!(set(&app, event, false).await, disabled);
    let opened = set(&app, event, true).await;
    assert_eq!(opened["enabled"], true);
    assert!(opened["started_at"].is_string());
    assert_eq!(set(&app, event, true).await, opened);
    assert_eq!(app.get(&path).await.json::<Value>().await.unwrap(), opened);
    assert_eq!(set(&app, event, false).await, disabled);
    let first: (Uuid, DateTime<Utc>, Option<DateTime<Utc>>) = sqlx::query_as(
        "SELECT id, started_at, closes_at FROM event_scoring_periods WHERE event_id = $1",
    )
    .bind(event)
    .fetch_one(&app.db_pool)
    .await
    .unwrap();
    assert!(first.2.unwrap() > first.1);
    assert_eq!(set(&app, event, false).await, disabled);
    set(&app, event, true).await;
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
    let (a, b, c) = tokio::join!(
        set(&app, event, true),
        set(&app, event, true),
        set(&app, event, true),
    );
    assert_eq!(a, b);
    assert_eq!(b, c);
    let (a, b, c) = tokio::join!(
        set(&app, event, false),
        set(&app, event, false),
        set(&app, event, false),
    );
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
    let other = app.activate_new_event().await;
    set(&app, event, true).await;
    let other_state = app
        .get(&format!("/events/{other}/scoring"))
        .await
        .json::<Value>()
        .await
        .unwrap();
    assert_eq!(other_state["enabled"], false);
    let path = format!("/events/{event}/scoring");
    for body in [
        json!({}),
        json!({"enabled": null}),
        json!({"enabled": "true"}),
        json!({"enabled": true, "started_at": "2026-10-10T00:00:00Z"}),
    ] {
        assert_text_error(app.request_json(Method::PUT, &path, body).await, 400).await;
    }
    assert_eq!(
        app.get(&path).await.json::<Value>().await.unwrap()["enabled"],
        true
    );
    for event in [Uuid::new_v4().to_string(), "invalid".to_string()] {
        let path = format!("/events/{event}/scoring");
        assert_text_error(app.get(&path).await, 404).await;
        assert_text_error(
            app.request_json(Method::PUT, &path, json!({"enabled": true}))
                .await,
            404,
        )
        .await;
    }
}
