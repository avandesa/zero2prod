mod admin;
mod health_check;
mod home;
mod newsletters;
mod subscriptions;
mod subscriptions_confirm;

pub use {
    admin::*, health_check::*, home::*, newsletters::*, subscriptions::*, subscriptions_confirm::*,
};
