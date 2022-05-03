use crate::{email_client::EmailClient, routes};

use std::net::TcpListener;

use {
    actix_web::{dev::Server, web, App, HttpServer},
    sqlx::PgPool,
    tracing_actix_web::TracingLogger,
};

pub fn run(
    listener: TcpListener,
    pool: PgPool,
    email_client: EmailClient,
) -> Result<Server, std::io::Error> {
    let pool = web::Data::new(pool);
    let email_client = web::Data::new(email_client);

    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(routes::health_check))
            .route("/subscriptions", web::post().to(routes::subscribe))
            .app_data(pool.clone())
            .app_data(email_client.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}
