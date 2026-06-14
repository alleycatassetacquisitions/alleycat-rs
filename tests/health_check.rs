use alleycat_rs::configuration::{DatabaseSettings, get_configuration};
use alleycat_rs::startup::run;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
use uuid::Uuid;

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

async fn spawn_app() -> TestApp {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{port}");
    let mut configuration = get_configuration().expect("Failed to read configuration.");
    configuration.database.database_name = Uuid::new_v4().to_string();
    let connection_pool = configure_database(&configuration.database).await;
    let server = run(listener, connection_pool.clone()).expect("Failed to bind address");
    let _ = tokio::spawn(server);
    TestApp {
        address,
        db_pool: connection_pool,
    }
}

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    // Create database
    let maintenance_settings = DatabaseSettings {
        database_name: "postgres".to_string(),
        username: "postgres".to_string(),
        password: "password".to_string(),
        ..config.clone()
    };

    let mut connection = PgConnection::connect(&maintenance_settings.connection_string())
        .await
        .expect("Failed to connect to Postgres");

    let DatabaseSettings { database_name, .. } = config;
    connection
        .execute(format!(r#"CREATE DATABASE "{database_name}";"#).as_str())
        .await
        .expect("Failed to create database.");

    // Migrate database
    let connection_pool = PgPool::connect(&config.connection_string())
        .await
        .expect("Failed to connect to Postgres.");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");

    connection_pool
}

//TODO add a test for name uniquness, maybe after a test refactor

#[tokio::test]
async fn health_check_works() {
    let TestApp { address, .. } = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{address}/health_check"))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn register_player_returns_a_200_for_valid_form_data() {
    // Arrange
    let TestApp { address, db_pool } = spawn_app().await;
    let client = reqwest::Client::new();
    // Act
    let body = "name=le%20guin";
    let response = client
        .post(&format!("{address}/players"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");
    // Assert
    assert_eq!(200, response.status().as_u16());
    let saved = sqlx::query!("SELECT name, pdn_code FROM players",)
        .fetch_one(&db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.pdn_code.len(), 4);
    assert!(saved.pdn_code.chars().all(|c| c.is_ascii_digit()));
}

#[tokio::test]
async fn register_player_returns_a_400_when_data_is_missing() {
    // Arrange
    let TestApp { address, .. } = spawn_app().await;
    let client = reqwest::Client::new();
    let test_cases = vec![("", "missing the name"), ("score=100", "invalid parameter")];

    for (invalid_body, error_message) in test_cases {
        // Act
        let response = client
            .post(&format!("{address}/players"))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request.");
        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}
