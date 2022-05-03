use {
    actix_web::{web, HttpResponse},
    serde::Deserialize,
    sqlx::PgPool,
    uuid::Uuid,
};

#[derive(Debug, Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(params, pool))]
pub async fn confirm(params: web::Query<Parameters>, pool: web::Data<PgPool>) -> HttpResponse {
    let sub_id = match match get_subscriber_id_from_token(&params.subscription_token, &pool).await {
        Ok(id) => id,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    } {
        Some(id) => id,
        None => return HttpResponse::Unauthorized().finish(),
    };

    match confirm_subscriber(&sub_id, &pool).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[tracing::instrument(name = "Mark a subscriber as confirmed", skip(sub_id, pool))]
async fn confirm_subscriber(sub_id: &Uuid, pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        sub_id
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(())
}

#[tracing::instrument(name = "Get subscriber_id from token", skip(token, pool))]
async fn get_subscriber_id_from_token(
    token: &str,
    pool: &PgPool,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        r#"SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1"#,
        token
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(result.map(|r| r.subscriber_id))
}
