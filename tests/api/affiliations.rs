use crate::helpers::spawn_app;
use uuid::Uuid;

const SEED: &str = include_str!("../../xtask/seed_db.sql");

#[tokio::test]
async fn development_seed_adds_affiliation_choices_without_assigning_players() {
    let app = spawn_app().await;
    sqlx::raw_sql(SEED).execute(&app.db_pool).await.unwrap();

    let categories: Vec<String> =
        sqlx::query_scalar("SELECT name FROM affiliation_categories ORDER BY name")
            .fetch_all(&app.db_pool)
            .await
            .unwrap();
    assert_eq!(categories, ["Allegiance", "Faction"]);

    let allegiances: Vec<String> = sqlx::query_scalar(
        "SELECT a.name FROM affiliations a
         JOIN affiliation_categories c ON c.id = a.category_id
         WHERE c.name = 'Allegiance' ORDER BY a.name",
    )
    .fetch_all(&app.db_pool)
    .await
    .unwrap();
    assert_eq!(
        allegiances,
        ["Endline Solutions", "Helix Industries", "Reboot Syndicate"]
    );
    let faction_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM affiliations a
         JOIN affiliation_categories c ON c.id = a.category_id WHERE c.name = 'Faction'",
    )
    .fetch_one(&app.db_pool)
    .await
    .unwrap();
    assert_eq!(faction_count, 10);
    let assignments: i64 = sqlx::query_scalar("SELECT count(*) FROM player_affiliations")
        .fetch_one(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(assignments, 0);
}

#[tokio::test]
async fn reseeding_affiliations_preserves_edits_assignments_and_event_selection() {
    let app = spawn_app().await;
    sqlx::raw_sql(SEED).execute(&app.db_pool).await.unwrap();

    sqlx::query("UPDATE affiliation_categories SET name = name || ' renamed'")
        .execute(&app.db_pool)
        .await
        .unwrap();
    sqlx::query("UPDATE affiliations SET name = name || ' renamed'")
        .execute(&app.db_pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO player_affiliations (event_id, player_id, category_id, affiliation_id)
         SELECT p.event_id, p.id, a.category_id, a.id
         FROM players p JOIN affiliations a ON a.event_id = p.event_id
         WHERE p.name = 'Nyx Voltage' AND a.name = 'Helix Industries renamed'",
    )
    .execute(&app.db_pool)
    .await
    .unwrap();

    let categories_query = "SELECT id, name FROM affiliation_categories ORDER BY id";
    let affiliations_query = "SELECT id, name FROM affiliations ORDER BY id";
    let assignments_query = "SELECT player_id, category_id, affiliation_id FROM player_affiliations ORDER BY player_id, category_id";
    let categories: Vec<(Uuid, String)> = sqlx::query_as(categories_query)
        .fetch_all(&app.db_pool)
        .await
        .unwrap();
    let affiliations: Vec<(Uuid, String)> = sqlx::query_as(affiliations_query)
        .fetch_all(&app.db_pool)
        .await
        .unwrap();
    let assignments: Vec<(Uuid, Uuid, Uuid)> = sqlx::query_as(assignments_query)
        .fetch_all(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(assignments.len(), 1);

    let other_event = app.activate_new_event().await;
    for _ in 0..2 {
        sqlx::raw_sql(SEED).execute(&app.db_pool).await.unwrap();
    }

    assert_eq!(
        sqlx::query_as::<_, (Uuid, String)>(categories_query)
            .fetch_all(&app.db_pool)
            .await
            .unwrap(),
        categories
    );
    assert_eq!(
        sqlx::query_as::<_, (Uuid, String)>(affiliations_query)
            .fetch_all(&app.db_pool)
            .await
            .unwrap(),
        affiliations
    );
    assert_eq!(
        sqlx::query_as::<_, (Uuid, Uuid, Uuid)>(assignments_query)
            .fetch_all(&app.db_pool)
            .await
            .unwrap(),
        assignments
    );
    let active = alleycat_rs::events::get_active_event_id(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(active, Some(other_event));
    let other_categories: i64 =
        sqlx::query_scalar("SELECT count(*) FROM affiliation_categories WHERE event_id = $1")
            .bind(other_event)
            .fetch_one(&app.db_pool)
            .await
            .unwrap();
    assert_eq!(other_categories, 0);
}
