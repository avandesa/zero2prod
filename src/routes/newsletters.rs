use crate::{
    domain::SubscriberEmail, email_client::EmailClient, telemetry::spawn_blocking_with_tracing,
};

use {
    actix_web::{
        http::{
            header::{self, HeaderMap, HeaderValue},
            StatusCode,
        },
        web, HttpRequest, HttpResponse, ResponseError,
    },
    anyhow::{Context, Result},
    argon2::{Argon2, PasswordHash, PasswordVerifier},
    secrecy::{ExposeSecret, Secret},
    sqlx::PgPool,
    uuid::Uuid,
};

#[derive(Debug, serde::Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(Debug)]
struct Credentials {
    username: String,
    password: Secret<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

#[derive(Debug)]
struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[derive(Debug, thiserror::Error)]
pub enum PublishError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

impl ResponseError for PublishError {
    fn error_response(&self) -> HttpResponse {
        match self {
            PublishError::AuthError(_) => {
                let mut response = HttpResponse::new(StatusCode::UNAUTHORIZED);
                let header_value = HeaderValue::from_str(r#"Basic realm="publish""#).unwrap();
                response
                    .headers_mut()
                    .insert(header::WWW_AUTHENTICATE, header_value);

                response
            }
            PublishError::Unexpected(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }
}

#[tracing::instrument(
    name = "Publish a newsletter issue",
    skip(body, pool, email_client),
    fields(title = %body.title)
)]
pub async fn publish_newsletter(
    body: web::Json<BodyData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    request: HttpRequest,
) -> Result<HttpResponse, PublishError> {
    let credentials = basic_authentication(request.headers()).map_err(PublishError::AuthError)?;
    dbg!(&credentials);
    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));

    let user_id = validate_credentials(credentials, &pool).await?;
    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));

    let subscribers = get_confirmed_subscribers(&pool)
        .await
        .context("Failed to get list of confirmed subscribers")?;

    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => email_client
                .send_email(
                    &subscriber.email,
                    &body.title,
                    &body.content.html,
                    &body.content.text,
                )
                .await
                .with_context(|| {
                    format!("Failed to send newsletter issue to {}", &subscriber.email)
                })?,
            Err(error) => {
                tracing::warn!(error.cause_chain = ?error, "Skipping a confirmed subscriber. Their stored contact details are invalid")
            }
        }
    }

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(name = "Get confirmed subscribers", skip(pool))]
async fn get_confirmed_subscribers(
    pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, sqlx::Error> {
    let confirmed_subscribers: Vec<_> =
        sqlx::query!(r#"SELECT email FROM subscriptions WHERE status = 'confirmed'"#)
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(|r| {
                SubscriberEmail::parse(r.email.clone())
                    .map(|email| ConfirmedSubscriber { email })
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "A confirmed subscriber is using an invalid email address: {}",
                            r.email
                        )
                    })
            })
            .collect();

    tracing::debug!(
        "Found {} confirmed subscribers",
        confirmed_subscribers.len()
    );

    Ok(confirmed_subscribers)
}

fn basic_authentication(headers: &HeaderMap) -> Result<Credentials, anyhow::Error> {
    let header_value = headers
        .get("Authorization")
        .context("The `Authorization` header was missing")?
        .to_str()
        .context("The `Authorization` header was not a valid UTF8 string")?;

    let base64_encoded_segment = header_value
        .strip_prefix("Basic ")
        .context("The authorization scheme was not `Basic`")?;

    let decoded_bytes = base64::decode_config(base64_encoded_segment, base64::STANDARD)
        .context("Failed to decode-base64 `Basic` credentials")?;

    let decoded_credentials = String::from_utf8(decoded_bytes)
        .context("The decoded credential string is not valid UTF8")?;

    let mut credentials = decoded_credentials.splitn(2, ':');
    let username = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A username must be provided in `Basic` auth"))?
        .to_string();
    let password = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A password must be provided in `Basic` auth"))?
        .to_string();

    Ok(Credentials {
        username,
        password: Secret::new(password),
    })
}

#[tracing::instrument(name = "Validate credentials", skip(credentials, pool))]
async fn validate_credentials(
    credentials: Credentials,
    pool: &PgPool,
) -> Result<uuid::Uuid, PublishError> {
    let mut user_id = None;
    let mut expected_password_hash = Secret::new(
        "$argon2id$v=19$m=15000,t=2,p=1$\
                gZiV/M1gPc22ElAH/Jh1Hw$\
                CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
            .to_string(),
    );

    if let Some((stored_user_id, stored_password_hash)) =
        get_stored_credentials(&credentials.username, pool).await?
    {
        user_id = Some(stored_user_id);
        expected_password_hash = stored_password_hash
    }

    spawn_blocking_with_tracing(move || {
        verify_password_hash(expected_password_hash, credentials.password)
    })
    .await
    .context("Failed to spawn blocking task")??;

    user_id.ok_or_else(|| PublishError::AuthError(anyhow::anyhow!("Unknown username")))
}

#[tracing::instrument(name = "Get stored credentials", skip(username, pool))]
async fn get_stored_credentials(
    username: &str,
    pool: &PgPool,
) -> Result<Option<(Uuid, Secret<String>)>> {
    let row: Option<_> = sqlx::query!(
        r#"SELECT user_id, password_hash FROM users WHERE username = $1"#,
        username
    )
    .fetch_optional(pool)
    .await
    .context("Failed to retrieve stored credentials")?
    .map(|r| (r.user_id, Secret::new(r.password_hash)));

    Ok(row)
}

#[tracing::instrument(
    "Verify password hash",
    skip(expected_password_hash, password_candidate)
)]
fn verify_password_hash(
    expected_password_hash: Secret<String>,
    password_candidate: Secret<String>,
) -> Result<(), PublishError> {
    let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .context("Failed to parse hash in PHC string format")?;

    Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .context("Invalid password")
        .map_err(PublishError::AuthError)
}
