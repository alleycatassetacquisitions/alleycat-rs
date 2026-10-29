use crate::helpers::{assert_json, spawn_app};
use serde_json::Value;
use std::collections::BTreeMap;
use uuid::Uuid;

#[tokio::test]
async fn development_seed_provides_repeatable_metagame_matches_and_scores() {
    let app = spawn_app().await;
    let seed = include_str!("../../xtask/seed_db.sql");
    sqlx::raw_sql(seed).execute(&app.db_pool).await.unwrap();

    let counts: Vec<(Uuid, i64)> = sqlx::query_as(
        "SELECT p.id, count(m.id) FROM players p
         LEFT JOIN quickdraw_matches m ON p.id IN (m.player_a_id, m.player_b_id)
         GROUP BY p.id",
    )
    .fetch_all(&app.db_pool)
    .await
    .unwrap();
    assert_eq!(counts.len(), 50);
    let mut distribution = BTreeMap::new();
    for (_, count) in &counts {
        *distribution.entry(*count).or_insert(0) += 1;
    }
    assert_eq!(
        distribution,
        BTreeMap::from([(0, 10), (3, 1), (4, 32), (15, 5), (20, 2)])
    );

    let (total, unique_pairs, scoring): (i64, i64, i64) = sqlx::query_as(
        "SELECT count(*),
                count(DISTINCT (LEAST(player_a_id, player_b_id),
                                GREATEST(player_a_id, player_b_id))),
                count(*) FILTER (WHERE scoring_status = 'scoring')
         FROM quickdraw_matches",
    )
    .fetch_one(&app.db_pool)
    .await
    .unwrap();
    assert_eq!((total, unique_pairs, scoring), (123, 123, 123));

    let body: Value = assert_json(app.get("/score/players?per_page=50").await, 200).await;
    let players = body["players"].as_array().unwrap();
    assert_eq!(players.len(), 50);
    assert_eq!(
        players
            .iter()
            .map(|p| p["score"].as_i64().unwrap())
            .sum::<i64>(),
        123
    );
    for (id, count) in counts {
        let player = players.iter().find(|p| p["id"] == id.to_string()).unwrap();
        assert_eq!(player["matches_played"], count);
        if count == 0 {
            assert_eq!(player["average_draw_time_ms"], Value::Null);
        } else {
            assert!(player["average_draw_time_ms"].as_f64().unwrap() > 0.0);
        }
    }

    let snapshot = "SELECT jsonb_agg(to_jsonb(m) ORDER BY id) FROM quickdraw_matches m";
    let original: Value = sqlx::query_scalar(snapshot)
        .fetch_one(&app.db_pool)
        .await
        .unwrap();
    // Membership and scoring edits must not reinterpret historical fixtures.
    sqlx::raw_sql(
        "UPDATE players SET team_id = NULL;
         UPDATE events SET accepting_delayed_scores = false;",
    )
    .execute(&app.db_pool)
    .await
    .unwrap();
    for _ in 0..2 {
        sqlx::raw_sql(seed).execute(&app.db_pool).await.unwrap();
        let actual: Value = sqlx::query_scalar(snapshot)
            .fetch_one(&app.db_pool)
            .await
            .unwrap();
        assert_eq!(actual, original);
    }
    let periods: i64 = sqlx::query_scalar("SELECT count(*) FROM event_scoring_periods")
        .fetch_one(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(periods, 0);
    let accepting_delayed_scores: bool =
        sqlx::query_scalar("SELECT accepting_delayed_scores FROM events")
            .fetch_one(&app.db_pool)
            .await
            .unwrap();
    assert!(!accepting_delayed_scores);
}
