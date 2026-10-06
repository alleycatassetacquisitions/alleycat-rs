use crate::helpers::spawn_app_with_event as spawn_app;

#[tokio::test]
async fn register_player_returns_a_200_for_valid_form_data() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = app.post_players(body.into()).await;

    assert_eq!(200, response.status().as_u16());
    let saved = sqlx::query!("SELECT name, email, pdn_code FROM players",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.email, Some("ursula_le_guin@gmail.com".to_string()));
    assert_eq!(saved.pdn_code.len(), 4);
    assert!(saved.pdn_code.chars().all(|c| c.is_ascii_digit()));
}

#[tokio::test]
async fn register_player_accepts_a_missing_email() {
    let app = spawn_app().await;
    let body = "name=Ursula";
    let response = app.post_players(body.into()).await;

    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("SELECT name, email FROM players")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved player.");

    assert_eq!(saved.name, "Ursula");
    assert_eq!(saved.email, None);
}

#[tokio::test]
async fn register_player_returns_a_400_when_fields_are_present_but_invalid() {
    let app = spawn_app().await;
    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gamil.com", "empty name"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email"),
    ];
    for (body, description) in test_cases {
        let response = app.post_players(body.into()).await;
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 400 Bad Request when the payload was {}.",
            description
        );
    }
}

#[tokio::test]
async fn register_player_returns_a_400_when_data_is_missing() {
    let app = spawn_app().await;
    let test_cases = vec![("", "missing the name"), ("score=100", "invalid parameter")];

    for (invalid_body, error_message) in test_cases {
        let response = app.post_players(invalid_body.into()).await;
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}

#[tokio::test]
async fn registration_requires_an_active_event() {
    let app = crate::helpers::spawn_app().await;
    assert_eq!(app.post_players("name=Nyx".into()).await.status(), 409);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM players")
        .fetch_one(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn allocation_skips_reserved_and_used_codes_and_rolls_back_on_duplicate_name() {
    let app = spawn_app().await;
    sqlx::query("INSERT INTO reserved_pdn_codes (code) VALUES ('0002')")
        .execute(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(app.post_players("name=Nyx".into()).await.status(), 200);
    assert_eq!(app.post_players("name=Nyx".into()).await.status(), 409);
    assert_eq!(app.post_players("name=Rook".into()).await.status(), 200);
    // Simulate an imported player ahead of the counter.
    sqlx::query("UPDATE events SET next_pdn_code = 1")
        .execute(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(app.post_players("name=Echo".into()).await.status(), 200);
    let codes: Vec<String> = sqlx::query_scalar("SELECT pdn_code FROM players ORDER BY pdn_code")
        .fetch_all(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(codes, ["0001", "0003", "0004"]);
}

#[tokio::test]
async fn exhausted_event_returns_conflict_without_creating_a_player() {
    let app = spawn_app().await;
    sqlx::query("UPDATE events SET next_pdn_code = 9998")
        .execute(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(app.post_players("name=Last".into()).await.status(), 200);
    // 9999 is reserved.
    assert_eq!(app.post_players("name=Overflow".into()).await.status(), 409);
    sqlx::query("UPDATE events SET next_pdn_code = 10000")
        .execute(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(app.post_players("name=Overflow".into()).await.status(), 409);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM players")
        .fetch_one(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn concurrent_registrations_allocate_distinct_sequential_codes() {
    let app = spawn_app().await;
    let (first, second) = tokio::join!(
        app.post_players("name=Nyx".into()),
        app.post_players("name=Rook".into())
    );
    assert_eq!(first.status(), 200);
    assert_eq!(second.status(), 200);
    let codes: Vec<String> = sqlx::query_scalar("SELECT pdn_code FROM players ORDER BY pdn_code")
        .fetch_all(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(codes, ["0001", "0002"]);
}
