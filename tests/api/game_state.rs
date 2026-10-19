use crate::helpers::{assert_json, assert_text_error, spawn_app};
use chrono::{DateTime, Utc};
use reqwest::Method;
use serde_json::{Value, json};

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
        .request_json(Method::PUT, "/scoring", json!({"enabled": true}))
        .await;
    let scoring: Value = assert_json(response, 200).await;
    let delayed = "/delayed-scores";
    for _ in 0..2 {
        let response = app
            .request_json(Method::PUT, delayed, json!({"enabled": false}))
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
    // Changing the second event leaves the first event's settings intact.
    for enabled in [false, true] {
        let response = app
            .request_json(Method::PUT, delayed, json!({"enabled": enabled}))
            .await;
        assert_eq!(
            assert_json::<Value>(response, 200).await,
            json!({"enabled": enabled})
        );
    }
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
    assert_eq!(state["accepting_delayed_scores"], false);
}

#[tokio::test]
async fn delayed_score_settings_validate_requests_and_require_an_active_event() {
    let app = spawn_app().await;
    let event = app.activate_new_event().await;
    let path = "/delayed-scores";
    for body in [
        json!({}),
        json!({"enabled": null}),
        json!({"enabled": "false"}),
        json!({"enabled": false, "finalize": true}),
        json!({"enabled": false, "event_id": event}),
    ] {
        assert_eq!(
            app.request_json(Method::PUT, path, body).await.status(),
            400
        );
    }
    sqlx::query("UPDATE app_state SET active_event_id = NULL")
        .execute(&app.db_pool)
        .await
        .unwrap();
    assert_text_error(
        app.request_json(Method::PUT, path, json!({"enabled": false}))
            .await,
        409,
    )
    .await;
    app.select_event(event).await;
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
