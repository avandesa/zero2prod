use crate::routes;

use std::net::TcpListener;

use {
    actix_web::{dev::Server, web, App, HttpServer},
    sqlx::PgPool,
};

pub fn run(listener: TcpListener, pool: PgPool) -> Result<Server, std::io::Error> {
    let pool = web::Data::new(pool);

    let server = HttpServer::new(move || {
        App::new()
            .route("/health_check", web::get().to(routes::health_check))
            .route("/subscriptions", web::post().to(routes::subscribe))
            .app_data(pool.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}
