use crate::helpers::spawn_app;

#[tokio::test]
async fn player_is_retrieved_with_a_200() {
    let name = "Martha_Wells";
    let email = "martha%40example.com";
    let body = format!("name={}&email={}", name, email);
    let app = spawn_app().await;
    let register_response = app.post_players(body.into()).await;
    assert_eq!(200, register_response.status().as_u16());
    let response = app.get_players().await;
    assert_eq!(200, response.status().as_u16());
    //TODO: deserialize response and check data
}
