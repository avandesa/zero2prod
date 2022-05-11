use crate::helpers::{assert_is_redirected_to, spawn_app};

#[tokio::test]
async fn error_flash_message_set_on_failure() {
    let app = spawn_app().await;

    let login_body = serde_json::json!({
        "username": "random-username",
        "password": "random-password"
    });
    let response = app.post_login(&login_body).await;

    // Part 1 - We're redirected to the login page
    assert_is_redirected_to(&response, "/login");

    // Part 2 - When we get the login page, the error message is displayed
    let html_page = dbg!(app.get_login_html().await);
    assert!(html_page.contains(r#"<p><i>Authentication failed</i></p>"#));

    // Part 3 - When we refresh the login page, the error message is no longer displayed
    let html_page = dbg!(app.get_login_html().await);
    assert!(!html_page.contains(r#"<p><i>Authentication failed</i></p>"#));
}

#[tokio::test]
async fn redirect_to_admin_after_login_success() {
    let app = spawn_app().await;

    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    });
    let response = app.post_login(&login_body).await;
    assert_is_redirected_to(&response, "/admin/dashboard");

    let html_page = app.get_admin_dashboard_html().await;
    assert!(html_page.contains(&format!("Welcome {}", app.test_user.username)));
}

#[tokio::test]
async fn require_login_to_access_admin() {
    let app = spawn_app().await;

    let response = app.get_admin_dashboard().await;

    assert_is_redirected_to(&response, "/login");
}
