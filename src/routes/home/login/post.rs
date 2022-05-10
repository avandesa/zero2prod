use crate::authentication::{validate_credentials, AuthError, Credentials};

use {
    actix_web::{error::InternalError, http::header::LOCATION, web, HttpResponse},
    actix_web_flash_messages::FlashMessage,
    secrecy::Secret,
    serde::Deserialize,
    sqlx::PgPool,
};

#[derive(Debug, Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

#[tracing::instrument(name = "Login", skip(form, db_pool))]
pub async fn login(
    form: web::Form<FormData>,
    db_pool: web::Data<PgPool>,
) -> Result<HttpResponse, InternalError<LoginError>> {
    let credentials = Credentials {
        username: form.0.username,
        password: form.0.password,
    };
    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));

    match validate_credentials(credentials, &db_pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
            Ok(HttpResponse::SeeOther()
                .insert_header((LOCATION, "/"))
                .finish())
        }
        Err(e) => {
            let e = match e {
                AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into()),
                AuthError::Unexpected(_) => LoginError::UnexpectedError(e.into()),
            };

            FlashMessage::error(e.to_string()).send();

            let response = HttpResponse::SeeOther()
                .insert_header((LOCATION, "/login"))
                .finish();
            Err(InternalError::from_response(e, response))
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}
