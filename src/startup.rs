use crate::configuration::{DatabaseSettings, Settings};
use crate::routes::{
    VersionInfo, get_device_logs, get_players, get_version, health_check, register_player,
    write_device_log,
};
use actix_web::dev::Server;
use actix_web::{App, HttpServer, web};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

pub async fn build(configuration: Settings) -> Result<Server, std::io::Error> {
    let connection_pool = get_connection_pool(&configuration.database);

    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );
    let listener = TcpListener::bind(address)?;
    run(listener, connection_pool)
}

pub fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
    // Apply these defaults to every API connection, including replacements.
    // Migration and maintenance connections use the base options separately.
    let options = configuration.connection_options().options([
        ("lock_timeout", "3s"),
        ("statement_timeout", "10s"),
        ("idle_in_transaction_session_timeout", "30s"),
    ]);
    PgPoolOptions::new().connect_lazy_with(options)
}

pub fn run(listener: TcpListener, db_pool: PgPool) -> Result<Server, std::io::Error> {
    let db_pool = web::Data::new(db_pool);
    let version = web::Data::new(VersionInfo::from_environment());
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/version", web::get().to(get_version))
            .route("/device-logs", web::get().to(get_device_logs))
            .route("/device-logs", web::post().to(write_device_log))
            .route("/players", web::get().to(get_players))
            .route("/players", web::post().to(register_player))
            .app_data(db_pool.clone())
            .app_data(version.clone())
    })
    .listen(listener)?
    .run();
    Ok(server)
}

pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, std::io::Error> {
        let connection_pool = get_connection_pool(&configuration.database);
        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );
        let listener = TcpListener::bind(address)?;
        let port = listener.local_addr().unwrap().port();
        let server = run(listener, connection_pool)?;
        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}
