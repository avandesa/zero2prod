use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    startup::ApplicationBaseUrl,
    EmailClient,
};

use {
    actix_web::{web, HttpResponse},
    chrono::Utc,
    rand::{distributions::Alphanumeric, thread_rng, Rng},
    sqlx::{PgPool, Postgres, Transaction},
    uuid::Uuid,
};

type Trans<'c> = Transaction<'c, Postgres>;

#[derive(serde::Deserialize, Debug)]
pub struct FormData {
    email: String,
    name: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(form: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(form.name)?;
        let email = SubscriberEmail::parse(form.email)?;
        Ok(NewSubscriber { email, name })
    }
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pool, email_client, base_url),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
) -> HttpResponse {
    let new_sub: NewSubscriber = match form.0.try_into() {
        Ok(new_sub) => new_sub,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    let subscription_token = match match match find_existing_subscriber(&new_sub.email, &pool).await
    {
        Ok(maybe_token) => maybe_token,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    } {
        // The user has already subscribed, so we refresh their subscription
        Some(user_id) => refresh_existing_subscription(user_id, &pool).await,
        // This is a new user, so store their info as a new subscription
        None => create_new_subscription(&new_sub, &pool).await,
    } {
        Ok(subscription_token) => subscription_token,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    // Send a confirmation email to the new subscriber
    if send_confirmation_email(&new_sub, &email_client, &base_url.0, &subscription_token)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Ok().finish()
}

#[tracing::instrument(name = "Creating a new subscription", skip(new_sub, pool))]
async fn create_new_subscription(
    new_sub: &NewSubscriber,
    pool: &PgPool,
) -> Result<String, sqlx::Error> {
    // Transaction start
    let mut transaction = pool.begin().await?;

    // Add the subscriber to the database
    let subscriber_id = insert_subscriber(new_sub, &mut transaction).await?;

    // Store the subscriber's token
    let subscription_token = generate_subscription_token();
    store_token(&subscriber_id, &subscription_token, &mut transaction).await?;

    // Transaction commit
    transaction.commit().await?;

    Ok(subscription_token)
}

#[tracing::instrument(
    name = "Refreshing an existing subscription",
    skip(subscriber_id, pool)
)]
async fn refresh_existing_subscription(
    subscriber_id: Uuid,
    pool: &PgPool,
) -> Result<String, sqlx::Error> {
    // Transaction start
    let mut transaction = pool.begin().await?;

    // Reset the subscription status
    reset_subscription_status(&subscriber_id, &mut transaction).await?;

    // Create a new subscription token
    let subscription_token = generate_subscription_token();
    store_token(&subscriber_id, &subscription_token, &mut transaction).await?;

    // Transaction commit
    transaction.commit().await?;

    Ok(subscription_token)
}

#[tracing::instrument("Find existing subscription by email", skip(email, pool))]
async fn find_existing_subscriber(
    email: &SubscriberEmail,
    pool: &PgPool,
) -> Result<Option<Uuid>, sqlx::Error> {
    let existing = sqlx::query!(
        r#"SELECT id FROM subscriptions WHERE email = $1"#,
        email.as_ref()
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to find existing subscription: {:?}", e);
        e
    })?
    .map(|r| r.id);

    if existing.is_some() {
        tracing::info!("Existing subscription found for email {}", email.as_ref());
    }

    Ok(existing)
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(new_sub, trans)
)]
async fn insert_subscriber(
    new_sub: &NewSubscriber,
    trans: &mut Trans<'_>,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();

    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, 'pending_confirmation')
        "#,
        subscriber_id,
        new_sub.email.as_ref(),
        new_sub.name.as_ref(),
        Utc::now()
    )
    .execute(trans)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(subscriber_id)
}

#[tracing::instrument(name = "Reset subscription status", skip(subscriber_id, trans))]
async fn reset_subscription_status(
    subscriber_id: &Uuid,
    trans: &mut Trans<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'pending_confirmation' WHERE id = $1"#,
        subscriber_id,
    )
    .execute(trans)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(())
}

#[tracing::instrument(
    name = "Store subscription token in the database",
    skip(subscription_token, trans)
)]
async fn store_token(
    subscriber_id: &Uuid,
    subscription_token: &str,
    trans: &mut Trans<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"INSERT INTO subscription_tokens (subscription_token, subscriber_id)
        VALUES ($1, $2)"#,
        subscription_token,
        subscriber_id,
    )
    .execute(trans)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(())
}

#[tracing::instrument(
    name = "Sending confirmation email to new subscriber",
    skip(email_client, new_sub, base_url, subscription_token)
)]
async fn send_confirmation_email(
    new_sub: &NewSubscriber,
    email_client: &EmailClient,
    base_url: &str,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!(
        "{}/subscriptions/confirm?subscription_token={}",
        base_url, subscription_token
    );
    let html_body = format!(
        "Welcome to our newsletter!<br />\
        Click <a href=\"{}\"> here to confirm your subscription.",
        confirmation_link
    );
    let text_body = format!(
        "Welcome to our newsletter!\nVisit {} to confirm your subscription.",
        confirmation_link
    );

    email_client
        .send_email(new_sub.email.clone(), "Welcome!", &html_body, &text_body)
        .await
        .map_err(|e| {
            tracing::error!("Failed to send confirmation email: {:?}", e);
            e
        })?;

    Ok(())
}

fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}
