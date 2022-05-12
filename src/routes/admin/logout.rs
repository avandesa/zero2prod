use crate::{
    session_state::TypedSession,
    utils::{e500, see_other},
};

use {actix_web::HttpResponse, actix_web_flash_messages::FlashMessage};

#[tracing::instrument(skip(session))]
pub async fn log_out(session: TypedSession) -> Result<HttpResponse, actix_web::Error> {
    if session.get_user_id().map_err(e500)?.is_none() {
        Ok(see_other("/login"))
    } else {
        session.log_out();
        tracing::info!("User logged out");
        FlashMessage::info("You have successfully logged out.").send();
        Ok(see_other("/login"))
    }
}
