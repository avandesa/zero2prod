use crate::helpers::{spawn_app, ConfirmationLinks, TestApp};

use {
    uuid::Uuid,
    wiremock::{
        matchers::{any, method, path},
        Mock, ResponseTemplate,
    },
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
        "content": {
            "text": "Newsletter text body",
            "html": "<p>Newsletter HTML body</p>"
        }
    });

    let response = app.post_newsletters(&newsletter_requst_body).await;

    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn newsletters_delivered_to_confirmed_subscribers() {
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let newsletter_requst_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter text body",
            "html": "<p>Newsletter HTML body</p>"
        }
    });

    let response = app.post_newsletters(&newsletter_requst_body).await;

    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn newsletters_returns_400_for_invalid_body() {
    let app = spawn_app().await;
    let test_cases = vec![
        (
            serde_json::json!({
                "content": {
                    "text": "Newsletter body as plain text",
                    "html": "<p>Newsletter body as HTML</p>",
                }
            }),
            "missing title",
        ),
        (
            serde_json::json!({"title": "Newsletter!"}),
            "missing content",
        ),
        (
            serde_json::json!({
                "title": "Newsletter!",
                "content": { "html": "<p>Newsletter body as HTML</p>" }
            }),
            "missing text",
        ),
        (
            serde_json::json!({
                "title": "Newsletter!",
                "content": { "text": "Newsletter body as HTML" }
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
async fn unauthorized_requests_are_rejected() {
    let app = spawn_app().await;

    let newsletter_requst_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter text body",
            "html": "<p>Newsletter HTML body</p>"
        }
    });

    let response = reqwest::Client::new()
        .post(&format!("{}/newsletters", &app.address))
        .json(&newsletter_requst_body)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status().as_u16(), 401);
    assert_eq!(
        r#"Basic realm="publish""#,
        response.headers()["WWW-Authenticate"]
    );
}

#[tokio::test]
async fn nonexistent_user_is_rejected() {
    let app = spawn_app().await;
    let username = Uuid::new_v4().to_string();
    let password = Uuid::new_v4().to_string();

    let response = reqwest::Client::new()
        .post(&format!("{}/newsletters", &app.address))
        .basic_auth(username, Some(password))
        .json(&serde_json::json!({
            "title": "Newsletter title",
            "content": {
                "text": "Newsletter body as plain text",
                "html": "<p>Newsletter body as HTML</p>",
            }
        }))
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(response.status().as_u16(), 401);
    assert_eq!(
        response.headers()["WWW-Authenticate"],
        r#"Basic realm="publish""#
    )
}

#[tokio::test]
async fn invalid_password_rejected() {
    let app = spawn_app().await;
    let username = &app.test_user.username;
    let password = Uuid::new_v4().to_string();

    let response = reqwest::Client::new()
        .post(&format!("{}/newsletters", &app.address))
        .basic_auth(username, Some(password))
        .json(&serde_json::json!({
            "title": "Newsletter title",
            "content": {
                "text": "Newsletter body as plain text",
                "html": "<p>Newsletter body as HTML</p>",
            }
        }))
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(response.status().as_u16(), 401);
    assert_eq!(
        response.headers()["WWW-Authenticate"],
        r#"Basic realm="publish""#
    )
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
