use zero2prod::*;

use {secrecy::ExposeSecret, sqlx::PgPool};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read configuration");

    let connection_pool =
        PgPool::connect(configuration.database.connection_string().expose_secret())
            .await
            .expect("Failed to conenct to Postgres");

    let address = format!("127.1:{}", configuration.application_port);
    let listener = std::net::TcpListener::bind(address)?;

    run(listener, connection_pool)?.await
}
