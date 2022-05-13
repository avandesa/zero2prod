use crate::helpers::{assert_is_redirected_to, spawn_app, ConfirmationLinks, TestApp};

use wiremock::{
    matchers::{any, method, path},
    Mock, ResponseTemplate,
};

#[tokio::test]
async fn newsletters_not_delivered_to_unconfirmed_subscribers() {
    let app = spawn_app().await;
    create_unconfirmed_subscriber(&app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let newsletter_requst_body = serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter text body",
        "html_content": "<p>Newsletter HTML body</p>"
    });
    app.login_test_user().await;
    let response = app.post_newsletters(&newsletter_requst_body).await;

    assert_is_redirected_to(&response, "/admin/newsletters");
    // Follow the redirect
    let html = app.get_newsletter_page().await.text().await.unwrap();
    assert!(html.contains("<p><i>Newsletter delivered successfully</i></p>"));
}

#[tokio::test]
async fn newsletters_delivered_to_confirmed_subscribers() {
    let app = spawn_app().await;
    app.login_test_user().await;
    create_confirmed_subscriber(&app).await;
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Send the newsletter
    let newsletter_requst_body = serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter text body",
        "html_content": "<p>Newsletter HTML body</p>"
    });
    let response = app.post_newsletters(&newsletter_requst_body).await;

    // Confirm we're redirected back to the newsletters page
    assert_is_redirected_to(&response, "/admin/newsletters");

    // Follow the redirect
    let html = app.get_newsletter_page().await.text().await.unwrap();
    assert!(html.contains("<p><i>Newsletter delivered successfully</i></p>"));
}

#[tokio::test]
async fn newsletters_returns_400_for_invalid_body() {
    let app = spawn_app().await;
    app.login_test_user().await;
    let test_cases = vec![
        (
            serde_json::json!({
                "text_content": "Newsletter body as plain text",
                "html_content": "<p>Newsletter body as HTML</p>",
            }),
            "missing title",
        ),
        (
            serde_json::json!({
                "title": "Newsletter!",
                "html_content": "<p>Newsletter body as HTML</p>"
            }),
            "missing text",
        ),
        (
            serde_json::json!({
                "title": "Newsletter!",
                "text_content": "Newsletter body as text"
            }),
            "missing html",
        ),
    ];

    for (invalid_body, err_message) in test_cases {
        let response = app.post_newsletters(&invalid_body).await;
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}",
            err_message
        )
    }
}

#[tokio::test]
async fn logged_out_users_redirected_post() {
    let app = spawn_app().await;
    let response = app.post_newsletters(&serde_json::json!({})).await;
    assert_is_redirected_to(&response, "/login");
}

#[tokio::test]
async fn logged_out_users_redirected_get() {
    let app = spawn_app().await;
    let response = app.get_newsletter_page().await;
    assert_is_redirected_to(&response, "/login");
}

async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;

    app.post_subscriptions(body.into())
        .await
        .error_for_status()
        .unwrap();

    let email_request = &app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();
    app.get_confirmation_links(email_request)
}

async fn create_confirmed_subscriber(app: &TestApp) {
    let confirmation_link = create_unconfirmed_subscriber(app).await;
    reqwest::get(confirmation_link.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}
