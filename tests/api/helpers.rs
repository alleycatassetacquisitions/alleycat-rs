use alleycat_rs::configuration::{DatabaseSettings, get_configuration};
use alleycat_rs::startup::{Application, get_connection_pool};
use alleycat_rs::telemetry::{get_subscriber, init_subscriber};
use prost::Message;
use reqwest::{Client, Method, RequestBuilder, Response};
use secrecy::Secret;
use serde_json::Value;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::sync::LazyLock;
use uuid::Uuid;

static TRACING: LazyLock<()> = LazyLock::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    };
});

pub struct TestApp {
    address: String,
    client: Client,
    pub db_pool: PgPool,
}

impl TestApp {
    pub fn request(&self, method: Method, path: &str) -> RequestBuilder {
        self.client.request(
            method,
            format!("{}/{}", self.address, path.trim_start_matches('/')),
        )
    }

    pub async fn request_json(&self, method: Method, path: &str, body: Value) -> Response {
        self.request(method, path)
            .json(&body)
            .send()
            .await
            .expect("Failed to execute JSON request.")
    }

    pub async fn get(&self, path: &str) -> Response {
        self.request(Method::GET, path)
            .send()
            .await
            .expect("Failed to execute GET request.")
    }

    pub async fn post_players(&self, body: String) -> Response {
        self.post_bytes(
            "/players",
            "application/x-www-form-urlencoded",
            body.into_bytes(),
        )
        .await
    }

    pub async fn get_players(&self) -> Response {
        self.get("/players").await
    }

    // Create an event through the API without changing the active selection.
    pub async fn create_event(&self, body: Value) -> Value {
        let response = self.request_json(Method::POST, "/events", body).await;
        assert_eq!(response.status(), 201, "Failed to create fixture event.");
        response
            .json()
            .await
            .expect("Event response was not valid JSON.")
    }

    // Seed an active event directly for tests that need one as a prerequisite.
    pub async fn activate_new_event(&self) -> Uuid {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO events (id, name, created_at) VALUES ($1, 'Test event', now())")
            .bind(id)
            .execute(&self.db_pool)
            .await
            .unwrap();
        sqlx::query("UPDATE app_state SET active_event_id = $1 WHERE id = 1")
            .bind(id)
            .execute(&self.db_pool)
            .await
            .unwrap();
        id
    }

    pub async fn post_protobuf<M>(&self, path: &str, message: &M) -> Response
    where
        M: Message,
    {
        self.post_bytes(path, "application/protobuf", message.encode_to_vec())
            .await
    }

    pub async fn post_bytes(&self, path: &str, content_type: &str, body: Vec<u8>) -> Response {
        self.request(Method::POST, path)
            .header("Content-Type", content_type)
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }
}

// A freshly migrated database has no events or active selection.
pub async fn spawn_app() -> TestApp {
    LazyLock::force(&TRACING);

    let configuration = {
        let mut c = get_configuration().expect("Failed to read configuration.");
        c.database.database_name = format!("alleycat_test_{}", Uuid::new_v4());
        c.application.port = 0;
        c
    };

    configure_database(&configuration.database).await;

    let application = Application::build(configuration.clone())
        .await
        .expect("Failed to build application.");
    let address = format!("http://127.0.0.1:{}", application.port());
    let _ = tokio::spawn(application.run_until_stopped());

    TestApp {
        address,
        client: Client::new(),
        db_pool: get_connection_pool(&configuration.database),
    }
}

async fn configure_database(config: &DatabaseSettings) -> PgPool {
    // Create database
    let maintenance_settings = DatabaseSettings {
        database_name: "postgres".to_string(),
        username: "postgres".to_string(),
        password: Secret::new("password".to_string()),
        ..config.clone()
    };

    let mut connection = PgConnection::connect_with(&maintenance_settings.connection_options())
        .await
        .expect("Failed to connect to Postgres");

    let DatabaseSettings { database_name, .. } = config;
    connection
        .execute(format!(r#"CREATE DATABASE "{database_name}";"#).as_str())
        .await
        .expect("Failed to create database.");

    // Migrate database
    let connection_pool = PgPool::connect_with(config.connection_options())
        .await
        .expect("Failed to connect to Postgres.");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");

    connection_pool
}

// Tests that require an active event opt in explicitly.
pub async fn spawn_app_with_event() -> TestApp {
    let app = spawn_app().await;
    app.activate_new_event().await;
    app
}
