use crate::{session_state::TypedSession, utils::see_other};

use {actix_web::HttpResponse, actix_web_flash_messages::FlashMessage};

#[tracing::instrument(skip(session))]
pub async fn log_out(session: TypedSession) -> Result<HttpResponse, actix_web::Error> {
    // Assume middleware will have rejected a non-logged-in user already.
    session.log_out();
    tracing::info!("User logged out");
    FlashMessage::info("You have successfully logged out.").send();
    Ok(see_other("/login"))
}
