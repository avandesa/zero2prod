use crate::domain::{SubTokenValidationError, SubscriptionToken};

use {
    actix_web::{http::StatusCode, web, HttpResponse, ResponseError},
    anyhow::Context,
    serde::Deserialize,
    sqlx::PgPool,
    uuid::Uuid,
};

#[derive(Debug, Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

impl TryFrom<Parameters> for SubscriptionToken {
    type Error = SubTokenValidationError;

    fn try_from(params: Parameters) -> Result<Self, Self::Error> {
        SubscriptionToken::parse(params.subscription_token)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SubConfirmationError {
    #[error("{0}")]
    MalformedToken(#[from] SubTokenValidationError),
    #[error("Token is not valid")]
    InvalidToken,
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

impl ResponseError for SubConfirmationError {
    fn status_code(&self) -> StatusCode {
        match self {
            SubConfirmationError::MalformedToken(_) => StatusCode::BAD_REQUEST,
            SubConfirmationError::InvalidToken => StatusCode::UNAUTHORIZED,
            SubConfirmationError::Unexpected(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(params, pool))]
pub async fn confirm(
    params: web::Query<Parameters>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, SubConfirmationError> {
    let subscription_token: SubscriptionToken = params.0.try_into()?;

    let sub_id = get_subscriber_id_from_token(&subscription_token, &pool)
        .await
        .context("Failed to get subscriber ID from token")?
        .ok_or(SubConfirmationError::InvalidToken)?;

    confirm_subscriber(&sub_id, &pool)
        .await
        .context("Failed to mark subscriber as confirmed")?;

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(name = "Mark a subscriber as confirmed", skip(sub_id, pool))]
async fn confirm_subscriber(sub_id: &Uuid, pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        sub_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

#[tracing::instrument(name = "Get subscriber_id from token", skip(token, pool))]
async fn get_subscriber_id_from_token(
    token: &SubscriptionToken,
    pool: &PgPool,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        r#"SELECT subscriber_id, is_valid FROM subscription_tokens WHERE subscription_token = $1 AND is_valid = true"#,
        token.as_ref(),
    )
    .fetch_optional(pool)
    .await?;

    Ok(result.map(|r| r.subscriber_id))
}
