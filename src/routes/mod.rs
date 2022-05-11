mod admin;
mod health_check;
mod home;
mod login;
mod newsletters;
mod subscriptions;
mod subscriptions_confirm;

pub use {
    admin::*, health_check::*, home::*, login::*, newsletters::*, subscriptions::*,
    subscriptions_confirm::*,
};
