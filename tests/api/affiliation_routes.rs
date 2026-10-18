use crate::helpers::{TestApp, assert_json, assert_text_error, spawn_app, spawn_app_with_event};
use alleycat_rs::proto::alleycat::device::{DevicePlayerAffiliation, DevicePlayerResponse};
use prost::Message;
use reqwest::Method;
use serde_json::{Value, json};
use uuid::Uuid;

fn categories_path() -> String {
    "/affiliation-categories".to_owned()
}

fn category_path(category: &Value) -> String {
    format!("{}/{}", categories_path(), category["id"].as_str().unwrap())
}

fn affiliations_path(category: &Value) -> String {
    format!("{}/affiliations", category_path(category))
}

fn player_path(player: &Value) -> String {
    format!("/players/{}", player["id"].as_str().unwrap())
}

fn affiliation_edit(category: &Value, id: Value) -> Value {
    json!({"affiliations": {(category["id"].as_str().unwrap()): id}})
}

async fn category(app: &TestApp, name: &str) -> Value {
    let response = app
        .request_json(Method::POST, &categories_path(), json!({"name": name}))
        .await;
    assert_json(response, 201).await
}

async fn affiliation(app: &TestApp, category: &Value, name: &str) -> Value {
    let response = app
        .request_json(
            Method::POST,
            &affiliations_path(category),
            json!({"name": name}),
        )
        .await;
    assert_json(response, 201).await
}

async fn register_player(app: &TestApp, name: &str) -> Value {
    let response = app.post_players(json!({"name": name})).await;
    assert_json(response, 200).await
}

async fn assign(app: &TestApp, player: &Value, category: &Value, id: Value) -> Value {
    let response = app
        .request_json(
            Method::PATCH,
            &player_path(player),
            affiliation_edit(category, id),
        )
        .await;
    assert_json(response, 200).await
}

fn membership(category: &Value, affiliation: &Value) -> Value {
    json!({
        "category_id": category["id"], "category_name": category["name"],
        "affiliation_id": affiliation["id"], "affiliation_name": affiliation["name"],
    })
}

#[tokio::test]
async fn catalogs_are_event_scoped_sorted_and_names_are_unique_within_their_parent() {
    let app = spawn_app().await;
    let first_event = app.activate_new_event().await;
    let response = app.get(&categories_path()).await;
    assert_eq!(assert_json::<Value>(response, 200).await, json!([]));
    let faction = category(&app, " Faction ").await;
    let allegiance = category(&app, "Allegiance").await;
    assert_eq!(faction["name"], "Faction");
    let response = app.get(&categories_path()).await;
    assert_eq!(
        assert_json::<Value>(response, 200).await,
        json!([allegiance, faction])
    );
    let response = app.get(&affiliations_path(&faction)).await;
    assert_eq!(assert_json::<Value>(response, 200).await, json!([]));
    let sentinels = affiliation(&app, &faction, "Sentinels").await;
    let corvid = affiliation(&app, &faction, " Corvid Alliance ").await;
    assert_eq!(corvid["name"], "Corvid Alliance");
    let response = app.get(&affiliations_path(&faction)).await;
    assert_eq!(
        assert_json::<Value>(response, 200).await,
        json!([corvid, sentinels])
    );

    affiliation(&app, &allegiance, "Sentinels").await;
    for (path, name) in [
        (categories_path(), "Faction"),
        (affiliations_path(&faction), "Sentinels"),
    ] {
        let response = app
            .request_json(Method::POST, &path, json!({"name": name}))
            .await;
        assert_text_error(response, 409).await;
    }

    let second_event = app.activate_new_event().await;
    assert_eq!(
        assert_json::<Value>(app.get(&categories_path()).await, 200).await,
        json!([])
    );
    let other_category = category(&app, "Faction").await;
    affiliation(&app, &other_category, "Sentinels").await;
    assert_eq!(other_category["event_id"], second_event.to_string());
    assert_ne!(other_category["id"], faction["id"]);
    // Every resource lookup is constrained to the selected event, including writes.
    for (method, path) in [
        (Method::GET, affiliations_path(&faction)),
        (Method::POST, affiliations_path(&faction)),
        (Method::PATCH, category_path(&faction)),
        (
            Method::PATCH,
            format!(
                "{}/{}",
                affiliations_path(&faction),
                corvid["id"].as_str().unwrap()
            ),
        ),
    ] {
        let response = app
            .request_json(method, &path, json!({"name": "Wrong event"}))
            .await;
        assert_eq!(response.status(), 404);
    }
    let response = app
        .request(Method::PUT, &format!("/events/{first_event}/active"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 204);
    assert_eq!(
        assert_json::<Value>(app.get(&categories_path()).await, 200).await,
        json!([allegiance, faction])
    );
    assert_eq!(
        assert_json::<Value>(app.get(&affiliations_path(&faction)).await, 200).await,
        json!([corvid, sentinels])
    );
}

#[tokio::test]
async fn catalog_operations_require_an_active_event_without_falling_back() {
    let app = spawn_app_with_event().await;
    let category = category(&app, "Faction").await;
    for remove_state in [false, true] {
        sqlx::query(if remove_state {
            "DELETE FROM app_state"
        } else {
            "UPDATE app_state SET active_event_id = NULL"
        })
        .execute(&app.db_pool)
        .await
        .unwrap();
        // Representative read/write callers of the shared selection helper.
        assert_text_error(app.get(&categories_path()).await, 409).await;
        let response = app
            .request_json(
                Method::POST,
                &affiliations_path(&category),
                json!({"name": "Must not save"}),
            )
            .await;
        assert_text_error(response, 409).await;
    }
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM affiliations")
        .fetch_one(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn event_switch_waits_for_a_catalog_write_to_commit() {
    let app = spawn_app_with_event().await;
    let category = category(&app, "Faction").await;
    let category_id = Uuid::parse_str(category["id"].as_str().unwrap()).unwrap();
    let other_event = app.create_event(json!({"name": "Other"})).await;
    let mut transaction = app.db_pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM affiliation_categories WHERE id = $1 FOR UPDATE")
        .bind(category_id)
        .fetch_one(&mut *transaction)
        .await
        .unwrap();
    let request = app
        .request(Method::PATCH, &category_path(&category))
        .json(&json!({"name": "Renamed"}));
    let mut edit = tokio::spawn(async move { request.send().await.unwrap() });
    // The edit acquires the selection lock before it waits on our category row.
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let locked: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM pg_locks WHERE relation = 'app_state'::regclass
                 AND mode = 'RowShareLock' AND granted)",
            )
            .fetch_one(&app.db_pool)
            .await
            .unwrap();
            if locked {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let request = app.request(
        Method::PUT,
        &format!("/events/{}/active", other_event["id"].as_str().unwrap()),
    );
    let mut switch = tokio::spawn(async move { request.send().await.unwrap() });
    for task in [&mut edit, &mut switch] {
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(100), task)
                .await
                .is_err()
        );
    }
    transaction.commit().await.unwrap();
    let updated: Value = assert_json(edit.await.unwrap(), 200).await;
    assert_eq!(updated["name"], "Renamed");
    assert_eq!(updated["event_id"], category["event_id"]);
    assert_eq!(switch.await.unwrap().status(), 204);
    assert_eq!(
        assert_json::<Value>(app.get(&categories_path()).await, 200).await,
        json!([])
    );
    let saved: String = sqlx::query_scalar("SELECT name FROM affiliation_categories WHERE id = $1")
        .bind(category_id)
        .fetch_one(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(saved, "Renamed");
}

#[tokio::test]
async fn catalog_edits_reject_invalid_fields_missing_parents_and_name_conflicts() {
    let app = spawn_app().await;
    app.activate_new_event().await;
    let faction = category(&app, "Faction").await;
    category(&app, "Taken").await;
    let first = affiliation(&app, &faction, "First").await;
    affiliation(&app, &faction, "Taken").await;
    let affiliation_path = format!(
        "{}/{}",
        affiliations_path(&faction),
        first["id"].as_str().unwrap()
    );
    // Shared name validation has boundary coverage in its unit test. Here cover
    // JSON extraction and immutable parent fields through the management routes.
    for (method, path, body, status) in [
        (Method::POST, categories_path(), json!({"name": " "}), 400),
        (
            Method::POST,
            affiliations_path(&faction),
            json!({"name": null}),
            400,
        ),
        (
            Method::PATCH,
            category_path(&faction),
            json!({"name": "New", "event_id": Uuid::new_v4()}),
            400,
        ),
        (
            Method::PATCH,
            affiliation_path.clone(),
            json!({"name": "New", "category_id": Uuid::new_v4()}),
            400,
        ),
        (
            Method::PATCH,
            category_path(&faction),
            json!({"name": "Taken"}),
            409,
        ),
        (
            Method::PATCH,
            affiliation_path,
            json!({"name": "Taken"}),
            409,
        ),
        (
            Method::POST,
            format!("/affiliation-categories/{}/affiliations", Uuid::new_v4()),
            json!({"name": "New"}),
            404,
        ),
        (
            Method::PATCH,
            format!("{}/{}", categories_path(), Uuid::new_v4()),
            json!({"name": "New"}),
            404,
        ),
        (
            Method::PATCH,
            format!("{}/{}", affiliations_path(&faction), Uuid::new_v4()),
            json!({"name": "New"}),
            404,
        ),
    ] {
        let response = app.request_json(method, &path, body).await;
        assert_eq!(response.status(), status, "{path}");
    }
    assert_eq!(
        app.get(&format!(
            "/affiliation-categories/{}/affiliations",
            Uuid::new_v4()
        ))
        .await
        .status(),
        404
    );
    let response = app.get(&affiliations_path(&faction)).await;
    let listed: Vec<Value> = assert_json(response, 200).await;
    assert!(listed.contains(&first));
}

#[tokio::test]
async fn assignments_are_optional_replace_one_category_and_can_be_cleared_repeatedly() {
    let app = spawn_app_with_event().await;
    let player = register_player(&app, "Nyx").await;
    assert_eq!(player["affiliations"], json!([]));
    let faction = category(&app, "Faction").await;
    let allegiance = category(&app, "Allegiance").await;
    let first = affiliation(&app, &faction, "Corvid Alliance").await;
    let second = affiliation(&app, &faction, "Sentinels").await;
    let helix = affiliation(&app, &allegiance, "Helix Industries").await;
    let path = format!("/players/{}", player["id"].as_str().unwrap());
    let response = app
        .request_json(Method::PATCH, &path, json!({"active": false}))
        .await;
    let inactive: Value = assert_json(response, 200).await;
    assign(&app, &player, &faction, first["id"].clone()).await;
    assign(&app, &player, &allegiance, helix["id"].clone()).await;
    for _ in 0..2 {
        let actual = assign(&app, &player, &faction, second["id"].clone()).await;
        let mut expected = inactive.clone();
        expected["affiliations"] = json!([
            membership(&allegiance, &helix),
            membership(&faction, &second)
        ]);
        assert_eq!(actual, expected);
    }
    for _ in 0..2 {
        let actual = assign(&app, &player, &faction, Value::Null).await;
        assert_eq!(
            actual["affiliations"],
            json!([membership(&allegiance, &helix)])
        );
    }
}

#[tokio::test]
async fn roster_pagination_counts_players_and_keeps_all_their_affiliations() {
    let app = spawn_app_with_event().await;
    let unassigned = register_player(&app, "Rook").await;
    let assigned = register_player(&app, "Nyx").await;
    let faction = category(&app, "Faction").await;
    let allegiance = category(&app, "Allegiance").await;
    let corvid = affiliation(&app, &faction, "Corvid Alliance").await;
    let helix = affiliation(&app, &allegiance, "Helix Industries").await;
    assign(&app, &assigned, &faction, corvid["id"].clone()).await;
    let assigned = assign(&app, &assigned, &allegiance, helix["id"].clone()).await;
    assert_eq!(
        assigned["affiliations"],
        json!([
            membership(&allegiance, &helix),
            membership(&faction, &corvid)
        ])
    );
    for (page, expected) in [
        (1, json!([assigned])),
        (2, json!([unassigned])),
        (3, json!([])),
    ] {
        let response = app.get(&format!("/players?page={page}&per_page=1")).await;
        let result: Value = assert_json(response, 200).await;
        assert_eq!(result["players"], expected);
    }
}

#[tokio::test]
async fn current_assignments_and_names_reach_player_json_and_devices() {
    let app = spawn_app_with_event().await;
    let player = register_player(&app, "Nyx").await;
    let allegiance = category(&app, "Allegiance").await;
    let helix = affiliation(&app, &allegiance, "Helix Industries").await;
    let category = category(&app, "Faction").await;
    let affiliation = affiliation(&app, &category, "Corvid Alliance").await;
    let device_path = format!(
        "/device/players/by-pin/{}",
        player["pdn_code"].as_str().unwrap()
    );
    assign(&app, &player, &allegiance, helix["id"].clone()).await;
    assign(&app, &player, &category, affiliation["id"].clone()).await;
    let response = app
        .request_json(
            Method::PATCH,
            &category_path(&category),
            json!({"name": "Community"}),
        )
        .await;
    let renamed_category: Value = assert_json(response, 200).await;
    let path = format!(
        "{}/{}",
        affiliations_path(&category),
        affiliation["id"].as_str().unwrap()
    );
    let response = app
        .request_json(Method::PATCH, &path, json!({"name": "Renamed"}))
        .await;
    let renamed: Value = assert_json(response, 200).await;
    assert_eq!(renamed["id"], affiliation["id"]);
    assert_eq!(renamed["category_id"], category["id"]);
    let expected = json!([
        membership(&allegiance, &helix),
        membership(&renamed_category, &renamed)
    ]);
    let player_path = format!("/players/{}", player["id"].as_str().unwrap());
    for path in [
        &player_path,
        &format!("/players/by-pin/{}", player["pdn_code"].as_str().unwrap()),
    ] {
        let response = app.get(path).await;
        let found: Value = assert_json(response, 200).await;
        assert_eq!(found["affiliations"], expected);
    }
    let response = app
        .request_json(Method::PATCH, &player_path, json!({"name": "Nyx Updated"}))
        .await;
    let updated: Value = assert_json(response, 200).await;
    assert_eq!(updated["affiliations"], expected);
    let mut expected_affiliations: Vec<_> = [&expected[0], &expected[1]]
        .into_iter()
        .map(|assignment| DevicePlayerAffiliation {
            category_id: assignment["category_id"].as_str().unwrap().to_owned(),
            category_name: assignment["category_name"].as_str().unwrap().to_owned(),
            affiliation_id: assignment["affiliation_id"].as_str().unwrap().to_owned(),
            affiliation_name: assignment["affiliation_name"].as_str().unwrap().to_owned(),
        })
        .collect();
    let device =
        DevicePlayerResponse::decode(app.get(&device_path).await.bytes().await.unwrap()).unwrap();
    assert_eq!(device.name, "Nyx Updated");
    assert_eq!(device.affiliations, expected_affiliations);

    assign(&app, &player, &category, Value::Null).await;
    expected_affiliations.pop();
    let device =
        DevicePlayerResponse::decode(app.get(&device_path).await.bytes().await.unwrap()).unwrap();
    assert_eq!(device.affiliations, expected_affiliations);
    assign(&app, &player, &allegiance, Value::Null).await;
    let device =
        DevicePlayerResponse::decode(app.get(&device_path).await.bytes().await.unwrap()).unwrap();
    assert!(device.affiliations.is_empty());
}

#[tokio::test]
async fn invalid_assignments_leave_existing_membership_unchanged() {
    let app = spawn_app_with_event().await;
    let player = register_player(&app, "Nyx").await;
    let faction = category(&app, "Faction").await;
    let original = affiliation(&app, &faction, "Corvid Alliance").await;
    let allegiance = category(&app, "Allegiance").await;
    let wrong_category = affiliation(&app, &allegiance, "Helix Industries").await;
    app.activate_new_event().await;
    let other_category = category(&app, "Faction").await;
    let wrong_event = affiliation(&app, &other_category, "Corvid Alliance").await;
    let response = app
        .request(
            Method::PUT,
            &format!("/events/{}/active", player["event_id"].as_str().unwrap()),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 204);
    let expected = assign(&app, &player, &faction, original["id"].clone()).await;
    let path = player_path(&player);
    for body in [
        json!({"affiliations": null}),
        json!({"affiliations": []}),
        json!({"affiliations": {"not-a-uuid": null}}),
        json!({"affiliations": {}}),
        affiliation_edit(&faction, json!("bad")),
        affiliation_edit(&faction, json!(42)),
        affiliation_edit(&faction, json!(Uuid::new_v4())),
        affiliation_edit(&faction, wrong_category["id"].clone()),
        affiliation_edit(&faction, wrong_event["id"].clone()),
        affiliation_edit(&other_category, Value::Null),
        json!({"affiliations": {(Uuid::new_v4().to_string()): null}}),
    ] {
        let response = app.request_json(Method::PATCH, &path, body).await;
        assert_eq!(response.status(), 400);
    }
    let response = app.get(&path).await;
    assert_eq!(assert_json::<Value>(response, 200).await, expected);
    app.activate_new_event().await;
    assert_eq!(
        app.request_json(
            Method::PATCH,
            &path,
            affiliation_edit(&faction, Value::Null)
        )
        .await
        .status(),
        404
    );
    sqlx::query("UPDATE app_state SET active_event_id = NULL")
        .execute(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(
        app.request_json(
            Method::PATCH,
            &path,
            affiliation_edit(&faction, Value::Null)
        )
        .await
        .status(),
        409
    );
}

#[tokio::test]
async fn player_and_multiple_affiliation_edits_commit_or_roll_back_together() {
    let app = spawn_app_with_event().await;
    let player = register_player(&app, "Nyx").await;
    let mut categories = [
        category(&app, "Faction").await,
        category(&app, "Allegiance").await,
    ];
    // PATCH processes category UUIDs in order. Fail the second change to check
    // rollback after both a player edit and an earlier assignment have executed.
    categories.sort_by_key(|category| category["id"].as_str().unwrap().to_owned());
    let [first_category, second_category] = &categories;
    let first = affiliation(&app, first_category, "First").await;
    let replacement = affiliation(&app, first_category, "Replacement").await;
    let second = affiliation(&app, second_category, "Second").await;
    let first_id = first_category["id"].as_str().unwrap();
    let second_id = second_category["id"].as_str().unwrap();
    let path = player_path(&player);
    let response = app
        .request_json(
            Method::PATCH,
            &path,
            json!({
                "name": "Nyx Updated", "active": false,
                "affiliations": {(first_id): first["id"], (second_id): second["id"]}
            }),
        )
        .await;
    let saved: Value = assert_json(response, 200).await;
    assert_eq!(saved["name"], "Nyx Updated");
    assert_eq!(saved["active"], false);
    let assignments = saved["affiliations"].as_array().unwrap();
    assert_eq!(assignments.len(), 2);
    assert!(assignments.contains(&membership(first_category, &first)));
    assert!(assignments.contains(&membership(second_category, &second)));

    let response = app
        .request_json(
            Method::PATCH,
            &path,
            json!({
                "name": "Must roll back", "active": true,
                "affiliations": {(first_id): replacement["id"], (second_id): Uuid::new_v4()}
            }),
        )
        .await;
    assert_eq!(response.status(), 400);
    assert_eq!(assert_json::<Value>(app.get(&path).await, 200).await, saved);

    let response = app
        .request_json(
            Method::PATCH,
            &path,
            json!({
                "name": "Nyx", "active": true,
                "affiliations": {(first_id): replacement["id"], (second_id): null}
            }),
        )
        .await;
    let updated: Value = assert_json(response, 200).await;
    assert_eq!(updated["name"], "Nyx");
    assert_eq!(updated["active"], true);
    assert_eq!(
        updated["affiliations"],
        json!([membership(first_category, &replacement)])
    );

    let response = app
        .request_json(
            Method::PATCH,
            &path,
            json!({
                "name": "Only the name", "affiliations": {}
            }),
        )
        .await;
    let updated_name: Value = assert_json(response, 200).await;
    assert_eq!(updated_name["name"], "Only the name");
    assert_eq!(updated_name["affiliations"], updated["affiliations"]);
}

#[tokio::test]
async fn concurrent_assignments_preserve_other_categories_and_one_choice_per_category() {
    let app = spawn_app_with_event().await;
    let player = register_player(&app, "Nyx").await;
    let faction = category(&app, "Faction").await;
    let allegiance = category(&app, "Allegiance").await;
    let first = affiliation(&app, &faction, "First").await;
    let second = affiliation(&app, &faction, "Second").await;
    let helix = affiliation(&app, &allegiance, "Helix").await;
    tokio::join!(
        assign(&app, &player, &faction, first["id"].clone()),
        assign(&app, &player, &allegiance, helix["id"].clone())
    );
    tokio::join!(
        assign(&app, &player, &faction, first["id"].clone()),
        assign(&app, &player, &faction, second["id"].clone())
    );
    let response = app
        .get(&format!("/players/{}", player["id"].as_str().unwrap()))
        .await;
    let actual: Value = assert_json(response, 200).await;
    assert_eq!(actual["affiliations"].as_array().unwrap().len(), 2);
    assert_eq!(actual["affiliations"][0], membership(&allegiance, &helix));
    assert!(
        [first["id"].clone(), second["id"].clone()]
            .contains(&actual["affiliations"][1]["affiliation_id"])
    );
}
