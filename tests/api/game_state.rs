use crate::helpers::{assert_json, assert_text_error, spawn_app};
use chrono::{DateTime, Utc};
use reqwest::Method;
use serde_json::{Value, json};
use uuid::Uuid;

#[tokio::test]
async fn game_state_without_an_active_event_returns_null_settings_and_utc_time() {
    let app = spawn_app().await;
    let before: DateTime<Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(&app.db_pool)
        .await
        .unwrap();
    let response = app.get("/game-state").await;
    let state: Value = assert_json(response, 200).await;
    let after: DateTime<Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(&app.db_pool)
        .await
        .unwrap();
    let time: DateTime<Utc> = state["server_time_utc"].as_str().unwrap().parse().unwrap();
    assert!(time >= before && time <= after);
    assert!(state["server_time_utc"].as_str().unwrap().ends_with('Z'));
    assert_eq!(state["active_event"], Value::Null);
    assert_eq!(state["scoring"], Value::Null);
    assert_eq!(state["accepting_delayed_scores"], Value::Null);
}

#[tokio::test]
async fn game_state_tracks_the_active_event_and_its_independent_settings() {
    let app = spawn_app().await;
    let first = app
        .create_event(json!({"name": "First", "venue_name": "Hall"}))
        .await;
    let id = first["id"].as_str().unwrap();
    let response = app
        .request(Method::PUT, &format!("/events/{id}/active"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 204);
    let response = app.get("/game-state").await;
    let state: Value = assert_json(response, 200).await;
    assert_eq!(state["active_event"], first);
    assert_eq!(
        state["scoring"],
        json!({"enabled": false, "started_at": null})
    );
    assert_eq!(state["accepting_delayed_scores"], true);

    let response = app
        .request_json(
            Method::PUT,
            &format!("/events/{id}/scoring"),
            json!({"enabled": true}),
        )
        .await;
    let scoring: Value = assert_json(response, 200).await;
    let delayed = format!("/events/{id}/delayed-scores");
    for _ in 0..2 {
        let response = app
            .request_json(Method::PUT, &delayed, json!({"enabled": false}))
            .await;
        let setting: Value = assert_json(response, 200).await;
        assert_eq!(setting, json!({"enabled": false}));
    }
    let response = app.get("/game-state").await;
    let state: Value = assert_json(response, 200).await;
    assert_eq!(state["scoring"], scoring);
    assert_eq!(state["accepting_delayed_scores"], false);

    let second = app.activate_new_event().await;
    let response = app.get("/game-state").await;
    let state: Value = assert_json(response, 200).await;
    assert_eq!(state["active_event"]["id"], second.to_string());
    assert_eq!(state["scoring"]["enabled"], false);
    assert_eq!(state["accepting_delayed_scores"], true);
    // An explicit-event update neither switches the active event nor its setting.
    assert_eq!(
        app.request_json(Method::PUT, &delayed, json!({"enabled": true}))
            .await
            .status(),
        200
    );
    let response = app
        .request(Method::PUT, &format!("/events/{id}/active"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 204);
    let response = app.get("/game-state").await;
    let state: Value = assert_json(response, 200).await;
    assert_eq!(state["active_event"], first);
    assert_eq!(state["scoring"], scoring);
    assert_eq!(state["accepting_delayed_scores"], true);
}

#[tokio::test]
async fn delayed_score_settings_validate_requests_and_missing_events() {
    let app = spawn_app().await;
    let event = app.activate_new_event().await;
    let path = format!("/events/{event}/delayed-scores");
    for body in [
        json!({}),
        json!({"enabled": null}),
        json!({"enabled": "false"}),
        json!({"enabled": false, "finalize": true}),
    ] {
        assert_eq!(
            app.request_json(Method::PUT, &path, body).await.status(),
            400
        );
    }
    assert_text_error(
        app.request_json(
            Method::PUT,
            &format!("/events/{}/delayed-scores", Uuid::new_v4()),
            json!({"enabled": false}),
        )
        .await,
        404,
    )
    .await;
    let response = app.get("/game-state").await;
    let state: Value = assert_json(response, 200).await;
    assert_eq!(state["accepting_delayed_scores"], true);
}

#[tokio::test]
async fn game_state_without_the_selection_row_still_returns_server_time() {
    let app = spawn_app().await;
    sqlx::query("DELETE FROM app_state")
        .execute(&app.db_pool)
        .await
        .unwrap();
    let response = app.get("/game-state").await;
    let state: Value = assert_json(response, 200).await;
    assert_eq!(state["active_event"], Value::Null);
    assert_eq!(state["scoring"], Value::Null);
    assert_eq!(state["accepting_delayed_scores"], Value::Null);
    assert!(state["server_time_utc"].is_string());
}
