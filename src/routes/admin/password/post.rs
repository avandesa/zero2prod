use crate::{
    authentication::{validate_credentials, AuthError, Credentials},
    routes::admin::dashboard::get_username,
    session_state::TypedSession,
    utils::{e500, see_other},
};

use {
    actix_web::{web, HttpResponse},
    actix_web_flash_messages::FlashMessage,
    secrecy::{ExposeSecret, Secret},
    sqlx::PgPool,
};

#[derive(serde::Deserialize)]
pub struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_check: Secret<String>,
}

pub async fn change_password(
    form: web::Form<FormData>,
    session: TypedSession,
    db_pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = if let Some(user_id) = session.get_user_id().map_err(e500)? {
        user_id
    } else {
        return Ok(see_other("/login"));
    };

    if form.new_password.expose_secret() != form.new_password_check.expose_secret() {
        FlashMessage::error(
            "You entered two different new passwords - the field values must match.",
        )
        .send();
        return Ok(see_other("/admin/password"));
    }

    if form.new_password.expose_secret().len() < 12 {
        FlashMessage::error("The new password must be at least 12 characters").send();
        return Ok(see_other("/admin/password"));
    }
    if form.new_password.expose_secret().len() > 128 {
        FlashMessage::error("The new password must be no more than 128 chars").send();
        return Ok(see_other("/admin/password"));
    }

    let username = get_username(&user_id, &db_pool).await.map_err(e500)?;
    let creds = Credentials {
        username,
        password: form.0.current_password,
    };
    if let Err(e) = validate_credentials(creds, &db_pool).await {
        return match e {
            AuthError::InvalidCredentials(_) => {
                FlashMessage::error("The current password is incorrect.").send();
                Ok(see_other("/admin/password"))
            }
            AuthError::Unexpected(_) => Err(e500(e)),
        };
    }

    crate::authentication::change_password(&user_id, form.0.new_password, &db_pool)
        .await
        .map_err(e500)?;

    FlashMessage::info("Your password has been changed.").send();

    Ok(see_other("/admin/password"))
}
