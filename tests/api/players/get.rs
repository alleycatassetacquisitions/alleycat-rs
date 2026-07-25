use crate::helpers::spawn_app;
use reqwest::header::CONTENT_TYPE;
use serde_json::Value;

#[tokio::test]
async fn player_is_retrieved_with_a_200() {
    let app = spawn_app().await;
    let body = "name=Martha%20Wells&email=martha%40example.com";
    let post_response = app.post_players(body.into()).await;
    assert_eq!(200, post_response.status().as_u16());

    let get_response = app.get_players().await;

    assert_eq!(200, get_response.status().as_u16());
    assert_eq!(
        get_response.headers().get(CONTENT_TYPE).unwrap(),
        "application/json"
    );

    let body = get_response
        .json::<Value>()
        .await
        .expect("Response was not valid JSON.");

    assert_eq!(body["pagination"]["page"], 1);
    assert_eq!(body["pagination"]["per_page"], 20);

    let players = body["players"]
        .as_array()
        .expect("`players` was not a JSON array.");

    assert_eq!(players.len(), 1);
    let player = &players[0];

    assert_eq!(player["name"], "Martha Wells");
    assert_eq!(player["mode"], "unassigned");

    assert!(player["id"].is_string());
    assert!(player["pdn_code"].is_string());
    assert!(player["created_at"].is_string());
}
