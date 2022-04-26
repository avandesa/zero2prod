use zero2prod::{get_configuration, run};

use sqlx::PgPool;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let configuration = get_configuration().expect("Failed to read configuration");

    let connection_pool = PgPool::connect(&configuration.database.connection_string())
        .await
        .expect("Failed to conenct to Postgres");

    let address = format!("127.1:{}", configuration.application_port);
    let listener = std::net::TcpListener::bind(address)?;

    run(listener, connection_pool)?.await
}
