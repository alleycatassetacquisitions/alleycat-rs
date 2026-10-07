use crate::helpers::{TestApp, spawn_app};
use reqwest::{Client, Method, Response};
use serde_json::{Value, json};
use uuid::Uuid;

async fn request(app: &TestApp, method: Method, path: &str, body: Value) -> Response {
    Client::new()
        .request(method, format!("{}{path}", app.address))
        .json(&body)
        .send()
        .await
        .unwrap()
}

async fn create(app: &TestApp, name: &str) -> Value {
    let response = request(
        app,
        Method::POST,
        "/events",
        json!({"name": name, "venue_name": "Hall"}),
    )
    .await;
    assert_eq!(response.status(), 201);
    response.json().await.unwrap()
}

async fn active(app: &TestApp) -> Option<Uuid> {
    alleycat_rs::events::get_active_event_id(&app.db_pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn events_can_be_created_edited_and_switched_without_losing_players_or_codes() {
    let app = spawn_app().await;
    let first = create(&app, " First ").await;
    assert_eq!(first["name"], "First");
    assert!(first["created_at"].is_string());
    assert_eq!(active(&app).await, None);
    let id = first["id"].as_str().unwrap();
    let path = format!("/events/{id}");
    assert_eq!(
        request(&app, Method::PUT, &format!("{path}/active"), json!(null))
            .await
            .status(),
        204
    );
    assert_eq!(app.post_players("name=Nyx".into()).await.status(), 200);

    let updated = request(&app, Method::PATCH, &path, json!({"name": "Renamed"})).await;
    assert_eq!(updated.status(), 200);
    let updated: Value = updated.json().await.unwrap();
    assert_eq!(updated["name"], "Renamed");
    assert_eq!(updated["venue_name"], "Hall");
    assert_eq!(updated["created_at"], first["created_at"]);
    let cleared = request(&app, Method::PATCH, &path, json!({"venue_name": null})).await;
    assert_eq!(cleared.status(), 200);
    let cleared: Value = cleared.json().await.unwrap();
    assert_eq!(cleared["name"], "Renamed");
    assert_eq!(cleared["venue_name"], Value::Null);

    let second = create(&app, "Second").await;
    assert_eq!(active(&app).await, Some(Uuid::parse_str(id).unwrap()));
    let second_id = second["id"].as_str().unwrap();
    assert_eq!(
        request(
            &app,
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
    assert_eq!(app.post_players("name=Nyx".into()).await.status(), 200);
    for _ in 0..2 {
        assert_eq!(
            request(&app, Method::PUT, &format!("{path}/active"), json!(null))
                .await
                .status(),
            204
        );
    }
    assert_eq!(app.post_players("name=Echo".into()).await.status(), 200);
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
            request(&app, Method::POST, "/events", body).await.status(),
            400
        );
    }
    let event = create(&app, "Original").await;
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
            request(&app, Method::PATCH, &format!("/events/{id}"), body)
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
    let event = create(&app, "Original").await;
    let id = event["id"].as_str().unwrap();
    assert_eq!(
        request(
            &app,
            Method::PUT,
            &format!("/events/{id}/active"),
            json!(null)
        )
        .await
        .status(),
        204
    );
    let missing = Uuid::new_v4();
    assert_eq!(
        request(
            &app,
            Method::PUT,
            &format!("/events/{missing}/active"),
            json!(null)
        )
        .await
        .status(),
        404
    );
    assert_eq!(
        request(
            &app,
            Method::PATCH,
            &format!("/events/{missing}"),
            json!({"name": "Missing"})
        )
        .await
        .status(),
        404
    );
    assert_eq!(active(&app).await, Some(Uuid::parse_str(id).unwrap()));
}

#[tokio::test]
async fn activation_waits_for_registration_selection_lock() {
    let app = spawn_app().await;
    let first = crate::helpers::activate_new_event(&app).await;
    let second = create(&app, "Second").await;
    let id = second["id"].as_str().unwrap();
    let mut transaction = app.db_pool.begin().await.unwrap();
    assert_eq!(
        alleycat_rs::events::lock_active_event(&mut transaction)
            .await
            .unwrap(),
        Some(first)
    );
    let url = format!("{}/events/{id}/active", app.address);
    let mut switch = tokio::spawn(async move { Client::new().put(url).send().await.unwrap() });
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
