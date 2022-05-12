use crate::helpers::{assert_is_redirected_to, spawn_app};

use uuid::Uuid;

#[tokio::test]
async fn only_logged_in_can_see_change_password_form() {
    let app = spawn_app().await;
    let response = app.get_change_password().await;
    assert_is_redirected_to(&response, "/login");
}

#[tokio::test]
async fn only_logged_in_can_change_password() {
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let body = serde_json::json!({
        "current_password": Uuid::new_v4().to_string(), // incorrect
        "new_password": &new_password,
        "new_password_check": &new_password,
    });

    let response = app.post_change_password(&body).await;

    assert_is_redirected_to(&response, "/login");
}

#[tokio::test]
async fn new_password_fields_must_match() {
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let another_new_password = Uuid::new_v4().to_string();

    app.post_login(&serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    }))
    .await;

    let body = serde_json::json!({
        "current_password": Uuid::new_v4().to_string(), // incorrect
        "new_password": &new_password,
        "new_password_check": &another_new_password,
    });
    let response = app.post_change_password(&body).await;
    assert_is_redirected_to(&response, "/admin/password");

    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains(
        "<p><i>You entered two different new passwords - the field values must match.</i></p>"
    ));
}

#[tokio::test]
async fn current_password_must_be_valid() {
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let wrong_password = Uuid::new_v4().to_string();

    app.post_login(&serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    }))
    .await;

    let body = serde_json::json!({
        "current_password": &wrong_password,
        "new_password": &new_password,
        "new_password_check": &new_password,
    });
    let response = app.post_change_password(&body).await;
    assert_is_redirected_to(&response, "/admin/password");

    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains("<p><i>The current password is incorrect.</i></p>"));
}

#[tokio::test]
async fn new_password_must_be_at_least_12_chars() {
    let app = spawn_app().await;
    let new_password = "Too short".to_string();

    app.post_login(&serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    }))
    .await;

    let body = serde_json::json!({
        "current_password": &app.test_user.password,
        "new_password": &new_password,
        "new_password_check": &new_password,
    });
    let response = app.post_change_password(&body).await;
    assert_is_redirected_to(&response, "/admin/password");

    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains("<p><i>The new password must be at least 12 characters</i></p>"));
}

#[tokio::test]
async fn new_password_must_be_no_more_than_128_chars() {
    let app = spawn_app().await;
    let new_password = "a".repeat(129);

    app.post_login(&serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    }))
    .await;

    let body = serde_json::json!({
        "current_password": &app.test_user.password,
        "new_password": &new_password,
        "new_password_check": &new_password,
    });
    let response = app.post_change_password(&body).await;
    assert_is_redirected_to(&response, "/admin/password");

    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains("<p><i>The new password must be no more than 128 chars</i></p>"));
}

#[tokio::test]
async fn logout_clears_session_state() {
    let app = spawn_app().await;

    // Login and verify we can access the dashboard
    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    });
    let response = app.post_login(&login_body).await;
    assert_is_redirected_to(&response, "/admin/dashboard");

    let html_page = app.get_admin_dashboard_html().await;
    assert!(html_page.contains(&format!("Welcome {}", app.test_user.username)));

    // Logout and verify we can no longer access the dashboard
    let response = app.post_logout().await;
    assert_is_redirected_to(&response, "/login");

    let html_page = app.get_login_html().await;
    assert!(html_page.contains(r#"<p><i>You have successfully logged out.</i></p>"#));

    let response = app.get_admin_dashboard().await;
    assert_is_redirected_to(&response, "/login");
}

#[tokio::test]
async fn change_password_works() {
    let app = spawn_app().await;

    // Login and verify we can access the dashboard
    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    });
    let response = app.post_login(&login_body).await;
    assert_is_redirected_to(&response, "/admin/dashboard");

    // Change password
    let new_password = Uuid::new_v4().to_string();
    let change_password_body = serde_json::json!({
        "current_password": &app.test_user.password,
        "new_password": &new_password,
        "new_password_check": &new_password,
    });
    let response = app.post_change_password(&change_password_body).await;
    assert_is_redirected_to(&response, "/admin/password");
    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains("<p><i>Your password has been changed.</i></p>"));

    // Log out
    let response = app.post_logout().await;
    assert_is_redirected_to(&response, "/login");
    let html_page = app.get_login_html().await;
    assert!(html_page.contains("<p><i>You have successfully logged out.</i></p>"));

    // Log in again using the new password
    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &new_password,
    });
    let response = app.post_login(&login_body).await;
    assert_is_redirected_to(&response, "/admin/dashboard");
}
