use crate::helpers::{spawn_app, spawn_app_with_event};
use alleycat_rs::events::lock_active_event;
use std::time::Duration;

const SQLSTATE_LOCK_NOT_AVAILABLE: &str = "55P03";

#[tokio::test]
async fn api_connections_receive_timeout_defaults() {
    let app = spawn_app().await;
    // Holding both at once ensures we inspect two different connections.
    let mut first = app.db_pool.acquire().await.unwrap();
    let mut second = app.db_pool.acquire().await.unwrap();
    for connection in [&mut first, &mut second] {
        let settings: (String, String, String) = sqlx::query_as(
            "SELECT current_setting('lock_timeout'), current_setting('statement_timeout'), \
             current_setting('idle_in_transaction_session_timeout')",
        )
        .fetch_one(&mut **connection)
        .await
        .unwrap();
        assert_eq!(settings, ("3s".into(), "10s".into(), "30s".into()));
    }
}

#[tokio::test]
async fn event_switch_times_out_when_registration_holds_the_selection_lock() {
    let app = spawn_app_with_event().await;
    let mut registration = app.db_pool.begin().await.unwrap();
    assert!(
        lock_active_event(&mut registration)
            .await
            .unwrap()
            .is_some()
    );

    let error = tokio::time::timeout(
        Duration::from_secs(8),
        sqlx::query("UPDATE app_state SET active_event_id = NULL WHERE id = 1")
            .execute(&app.db_pool),
    )
    .await
    .expect("The database lock timeout should fire before the test deadline")
    .expect_err("The event switch should time out while registration holds the lock");
    assert_eq!(
        error.as_database_error().unwrap().code().as_deref(),
        Some(SQLSTATE_LOCK_NOT_AVAILABLE)
    );

    registration.rollback().await.unwrap();
    sqlx::query("UPDATE app_state SET active_event_id = NULL WHERE id = 1")
        .execute(&app.db_pool)
        .await
        .expect("The event switch should succeed after the lock is released");
}
