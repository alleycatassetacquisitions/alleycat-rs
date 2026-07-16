use crate::helpers::spawn_app;

//TODO add a test for name uniquness, maybe after a test refactor

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
