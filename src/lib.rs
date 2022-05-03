pub mod configuration;
pub mod domain;
pub mod email_client;
pub mod routes;
pub mod startup;
pub mod telemetry;

pub use {
    configuration::get_configuration,
    email_client::EmailClient,
    startup::run,
    telemetry::{get_subscriber, init_subscriber},
};
