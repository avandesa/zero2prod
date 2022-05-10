use crate::helpers::spawn_app;

use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

#[tokio::test]
async fn confirmations_without_tokens_rejected_400() {
    let app = spawn_app().await;

    let response = reqwest::get(&format!("{}/subscriptions/confirm", app.address))
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 400);
}

#[tokio::test]
async fn link_returned_by_subscribe_returns_200_when_called() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(email_request);

    let response = reqwest::get(confirmation_links.html).await.unwrap();

    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn clicking_confirmation_link_confirms_subscriber() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(email_request);

    reqwest::get(confirmation_links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.status, "confirmed");
}

#[tokio::test]
async fn using_nonexistent_token_returns_401() {
    let app = spawn_app().await;
    let phony_token = "0".repeat(25);
    let phony_link = format!(
        "http://127.0.0.1:{}/subscriptions/confirm?subscription_token={}",
        app.port, phony_token,
    );

    let response = reqwest::get(phony_link).await.unwrap();

    assert_eq!(response.status().as_u16(), 401);
}

#[tokio::test]
async fn using_invalidated_token_returns_401() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;

    // Get the first confirmation links
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(email_request);

    // Subscribe again, invalidating the previous links
    app.post_subscriptions(body.into()).await;

    let response = reqwest::get(confirmation_links.html).await.unwrap();
    assert_eq!(response.status().as_u16(), 401);

    let saved = sqlx::query!("SELECT status FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription");
    assert_eq!(saved.status, "pending_confirmation");
}

#[tokio::test]
async fn malformed_token_returns_400() {
    let app = spawn_app().await;

    let test_cases = vec![
        ("tooshort", "too short"),
        ("too_long__________________", "too long"),
        ("has spaces_______________", "not alphanumeric"),
    ];

    for (phony_token, description) in test_cases {
        let phony_link = format!(
            "http://127.0.0.1:{}/subscriptions/confirm?subscription_token={}",
            app.port, phony_token,
        );
        let response = reqwest::get(phony_link).await.unwrap();

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 400 Bad Request when the paylod had an {}",
            description
        );
    }
}
