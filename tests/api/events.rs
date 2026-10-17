use crate::helpers::{TestApp, assert_json, assert_text_error, spawn_app};
use reqwest::Method;
use serde_json::{Value, json};
use uuid::Uuid;

async fn active(app: &TestApp) -> Option<Uuid> {
    alleycat_rs::events::get_active_event_id(&app.db_pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn listing_events_returns_an_empty_array_when_none_exist() {
    let app = spawn_app().await;
    let response = app.get("/events").await;
    let listed: Value = assert_json(response, 200).await;
    assert_eq!(listed, json!([]));
}

#[tokio::test]
async fn listing_events_orders_by_creation_time_then_id() {
    let app = spawn_app().await;
    // The newest event has the lowest ID so time must take precedence.
    let newest = json!({
        "id": Uuid::from_u128(1),
        "name": "Newest",
        "venue_name": "Main Hall",
        "created_at": "2026-01-02T00:00:00Z"
    });
    let older_higher_id = json!({
        "id": Uuid::from_u128(3),
        "name": "Older, higher ID",
        "venue_name": null,
        "created_at": "2026-01-01T00:00:00Z"
    });
    let older_lower_id = json!({
        "id": Uuid::from_u128(2),
        "name": "Older, lower ID",
        "venue_name": "Side Hall",
        "created_at": "2026-01-01T00:00:00Z"
    });
    // Insert out of order; the listing must establish its own order.
    for event in [&older_lower_id, &newest, &older_higher_id] {
        let id = Uuid::parse_str(event["id"].as_str().unwrap()).unwrap();
        let created_at: chrono::DateTime<chrono::Utc> =
            event["created_at"].as_str().unwrap().parse().unwrap();
        sqlx::query(
            "INSERT INTO events (id, name, venue_name, created_at) VALUES ($1, $2, $3, $4)",
        )
        .bind(id)
        .bind(event["name"].as_str().unwrap())
        .bind(event["venue_name"].as_str())
        .bind(created_at)
        .execute(&app.db_pool)
        .await
        .unwrap();
    }

    let response = app.get("/events").await;
    let listed: Value = assert_json(response, 200).await;

    assert_eq!(listed, json!([newest, older_higher_id, older_lower_id]));
}

#[tokio::test]
async fn a_listed_event_can_be_selected_for_registration() {
    let app = spawn_app().await;
    let first = app
        .create_event(json!({"name": "First", "venue_name": "Hall"}))
        .await;
    let second = app.create_event(json!({"name": "Second"})).await;

    let response = app.get("/events").await;
    let listed: Vec<Value> = assert_json(response, 200).await;
    assert_eq!(listed.len(), 2);
    assert!(listed.contains(&first));
    assert!(listed.contains(&second));
    assert_eq!(active(&app).await, None);

    let selected_id = Uuid::parse_str(listed[0]["id"].as_str().unwrap()).unwrap();
    let response = app
        .request(Method::PUT, &format!("/events/{selected_id}/active"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 204);

    // Listing still includes every event and preserves the active selection.
    let response = app.get("/events").await;
    let listed_again: Vec<Value> = assert_json(response, 200).await;
    assert_eq!(listed_again, listed);
    assert_eq!(active(&app).await, Some(selected_id));

    let response = app.post_players(json!({"name": "Nyx"})).await;
    assert_eq!(response.status(), 200);
    let registered_event: Uuid =
        sqlx::query_scalar("SELECT event_id FROM players WHERE name = 'Nyx'")
            .fetch_one(&app.db_pool)
            .await
            .unwrap();
    assert_eq!(registered_event, selected_id);
}

#[tokio::test]
async fn events_can_be_created_edited_and_switched_without_losing_players_or_codes() {
    let app = spawn_app().await;
    let first = app
        .create_event(json!({"name": " First ", "venue_name": "Hall"}))
        .await;
    assert_eq!(first["name"], "First");
    assert!(first["created_at"].is_string());
    assert_eq!(active(&app).await, None);
    let id = first["id"].as_str().unwrap();
    let path = format!("/events/{id}");
    assert_eq!(
        app.request_json(Method::PUT, &format!("{path}/active"), json!(null))
            .await
            .status(),
        204
    );
    assert_eq!(app.post_players(json!({"name": "Nyx"})).await.status(), 200);

    let updated = app
        .request_json(Method::PATCH, &path, json!({"name": "Renamed"}))
        .await;
    let updated: Value = assert_json(updated, 200).await;
    assert_eq!(updated["name"], "Renamed");
    assert_eq!(updated["venue_name"], "Hall");
    assert_eq!(updated["created_at"], first["created_at"]);
    let cleared = app
        .request_json(Method::PATCH, &path, json!({"venue_name": null}))
        .await;
    let cleared: Value = assert_json(cleared, 200).await;
    assert_eq!(cleared["name"], "Renamed");
    assert_eq!(cleared["venue_name"], Value::Null);

    let second = app
        .create_event(json!({"name": "Second", "venue_name": "Hall"}))
        .await;
    assert_eq!(active(&app).await, Some(Uuid::parse_str(id).unwrap()));
    let second_id = second["id"].as_str().unwrap();
    assert_eq!(
        app.request_json(
            Method::PUT,
            &format!("/events/{second_id}/active"),
            json!(null)
        )
        .await
        .status(),
        204
    );
    let players: Value = app.get_players().await.json().await.unwrap();
    assert_eq!(players["players"], json!([]));
    assert_eq!(app.post_players(json!({"name": "Nyx"})).await.status(), 200);
    for _ in 0..2 {
        assert_eq!(
            app.request_json(Method::PUT, &format!("{path}/active"), json!(null))
                .await
                .status(),
            204
        );
    }
    assert_eq!(
        app.post_players(json!({"name": "Echo"})).await.status(),
        200
    );
    let players: Value = app.get_players().await.json().await.unwrap();
    assert_eq!(players["players"].as_array().unwrap().len(), 2);
    assert_eq!(players["players"][0]["pdn_code"], "0002");
}

#[tokio::test]
async fn event_validation_rejects_invalid_input_without_mutating_data() {
    let app = spawn_app().await;
    for body in [
        json!({}),
        json!({"name": " \t"}),
        json!({"name": null}),
        json!({"name": 1}),
        json!({"name": "a\u{0000}"}),
        json!({"name": "Valid", "active": true}),
    ] {
        assert_eq!(
            app.request_json(Method::POST, "/events", body)
                .await
                .status(),
            400
        );
    }
    let event = app
        .create_event(json!({"name": "Original", "venue_name": "Hall"}))
        .await;
    let id = event["id"].as_str().unwrap();
    for body in [
        json!({}),
        json!({"name": ""}),
        json!({"name": null}),
        json!({"venue_name": 1}),
        json!({"venue_name": "a\u{0000}"}),
        json!({"next_pdn_code": 1}),
    ] {
        assert_eq!(
            app.request_json(Method::PATCH, &format!("/events/{id}"), body)
                .await
                .status(),
            400
        );
    }
    let row: (String, Option<String>, i64) = sqlx::query_as(
        "SELECT name, venue_name, (SELECT count(*) FROM events) FROM events WHERE id = $1",
    )
    .bind(Uuid::parse_str(id).unwrap())
    .fetch_one(&app.db_pool)
    .await
    .unwrap();
    assert_eq!(row, ("Original".into(), Some("Hall".into()), 1));
}

#[tokio::test]
async fn missing_events_do_not_change_the_active_selection() {
    let app = spawn_app().await;
    let event = app
        .create_event(json!({"name": "Original", "venue_name": "Hall"}))
        .await;
    let id = event["id"].as_str().unwrap();
    assert_eq!(
        app.request_json(Method::PUT, &format!("/events/{id}/active"), json!(null))
            .await
            .status(),
        204
    );
    let missing = Uuid::new_v4();
    assert_text_error(
        app.request_json(
            Method::PUT,
            &format!("/events/{missing}/active"),
            json!(null),
        )
        .await,
        404,
    )
    .await;
    assert_text_error(
        app.request_json(
            Method::PATCH,
            &format!("/events/{missing}"),
            json!({"name": "Missing"}),
        )
        .await,
        404,
    )
    .await;
    assert_eq!(active(&app).await, Some(Uuid::parse_str(id).unwrap()));
}

#[tokio::test]
async fn activation_waits_for_registration_selection_lock() {
    let app = spawn_app().await;
    let first = app.activate_new_event().await;
    let second = app
        .create_event(json!({"name": "Second", "venue_name": "Hall"}))
        .await;
    let id = second["id"].as_str().unwrap();
    let mut transaction = app.db_pool.begin().await.unwrap();
    assert_eq!(
        alleycat_rs::events::lock_active_event(&mut transaction)
            .await
            .unwrap(),
        Some(first)
    );
    let request = app.request(Method::PUT, &format!("/events/{id}/active"));
    let mut switch = tokio::spawn(async move { request.send().await.unwrap() });
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(150), &mut switch)
            .await
            .is_err()
    );
    assert_eq!(active(&app).await, Some(first));
    transaction.commit().await.unwrap();
    assert_eq!(switch.await.unwrap().status(), 204);
    assert_eq!(active(&app).await, Some(Uuid::parse_str(id).unwrap()));
}
