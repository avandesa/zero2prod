use crate::session_state::TypedSession;

use {
    actix_web::{
        http::header::{ContentType, LOCATION},
        web, HttpResponse,
    },
    anyhow::{Context, Result},
    sqlx::PgPool,
    uuid::Uuid,
};

fn e500<T>(e: T) -> actix_web::Error
where
    T: std::fmt::Debug + std::fmt::Display + 'static,
{
    actix_web::error::ErrorInternalServerError(e)
}

#[tracing::instrument(name = "Get admin dashboard", skip(session))]
pub async fn admin_dashboard(
    session: TypedSession,
    db_pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let username = if let Some(user_id) = session.get_user_id().map_err(e500)? {
        get_username(&user_id, &db_pool).await.map_err(e500)?
    } else {
        let response = HttpResponse::SeeOther()
            .insert_header((LOCATION, "/login"))
            .finish();
        return Ok(response);
    };

    let response = HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta http-equiv="content-type" content="text/html; charset=utf-8">
    <title>Admin dashboard</title>
</head>
<body>
    <p>Welcome {username}!</p>
</body>
</html>
            "#
        ));

    Ok(response)
}

async fn get_username(user_id: &Uuid, db_pool: &PgPool) -> Result<String> {
    let row = sqlx::query!(r#"SELECT username FROM users WHERE user_id = $1"#, user_id,)
        .fetch_one(db_pool)
        .await
        .context("Failed to perform a query to retrieve a username")?;

    Ok(row.username)
}