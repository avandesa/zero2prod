use crate::{domain::SubscriberEmail, email_client::EmailClient};

use {
    actix_web::{web, HttpResponse, ResponseError},
    anyhow::Context,
    sqlx::PgPool,
};

#[derive(Debug, serde::Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
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
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

impl ResponseError for PublishError {}

#[tracing::instrument(
    name = "Publish a newsletter issue",
    skip(body, pool, email_client),
    fields(title = %body.title)
)]
pub async fn publish_newsletter(
    body: web::Json<BodyData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
) -> Result<HttpResponse, PublishError> {
    dbg!(&body);
    let subscribers = get_confirmed_subscribers(&pool)
        .await
        .context("Failed to get list of confirmed subscribers")?;
    dbg!(&subscribers);

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
