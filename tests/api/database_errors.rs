use actix_web::{App, http::header::CONTENT_TYPE, test, web};
use alleycat_rs::routes;
use serde_json::json;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

#[tokio::test]
async fn unexpected_constraints_are_not_reported_as_name_conflicts_or_exposed() {
    use crate::helpers::{assert_text_error, spawn_app_with_event};
    use reqwest::Method;

    let app = spawn_app_with_event().await;
    let event = alleycat_rs::events::get_active_event_id(&app.db_pool)
        .await
        .unwrap()
        .unwrap();
    // These constraints exist only in this test's isolated database. Their
    // details must stay server-side, unlike the expected name conflicts.
    for statement in [
        "ALTER TABLE teams ADD CONSTRAINT private_team_detail CHECK (name <> 'Rejected')",
        "ALTER TABLE players ADD CONSTRAINT private_player_detail CHECK (name <> 'Rejected')",
    ] {
        sqlx::query(statement).execute(&app.db_pool).await.unwrap();
    }
    let team_error = assert_text_error(
        app.request_json(
            Method::POST,
            &format!("/events/{event}/teams"),
            json!({"name": "Rejected"}),
        )
        .await,
        500,
    )
    .await;
    let player_error = assert_text_error(app.post_players("name=Rejected".into()).await, 500).await;
    // Assert against private fixture details, not the public message's wording.
    for body in [team_error, player_error] {
        for private_detail in [
            "private_team_detail",
            "private_player_detail",
            "Rejected",
            "23514",
        ] {
            assert!(
                !body.contains(private_detail),
                "Database details leaked: {body}"
            );
        }
    }
}

#[actix_web::test]
async fn representative_routes_return_safe_text_on_database_failure() {
    // Fail deterministically at the database boundary without requiring a server.
    let pool = PgPoolOptions::new().connect_lazy_with(PgConnectOptions::new());
    pool.close().await;
    // Cover both response paths: Actix converts an error for create_event;
    // registration converts it explicitly. New endpoints need no entry here.
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool))
            .service(routes::create_event)
            .service(routes::register_player),
    )
    .await;
    for request in [
        test::TestRequest::post()
            .uri("/events")
            .set_json(json!({"name": "Game"})),
        test::TestRequest::post()
            .uri("/players")
            .insert_header((CONTENT_TYPE, "application/x-www-form-urlencoded"))
            .set_payload("name=Nyx"),
    ] {
        let response = test::call_service(&app, request.to_request()).await;
        assert_eq!(response.status(), 500);
        assert_eq!(
            response
                .headers()
                .get(CONTENT_TYPE)
                .map(|h| h.to_str().unwrap()),
            Some("text/plain; charset=utf-8"),
        );
        let body = String::from_utf8(test::read_body(response).await.to_vec()).unwrap();
        assert!(!body.trim().is_empty());
        for private_detail in [
            sqlx::Error::PoolClosed.to_string(),
            format!("{:?}", sqlx::Error::PoolClosed),
        ] {
            assert!(
                !body.contains(&private_detail),
                "Database details leaked: {body}"
            );
        }
    }
}
