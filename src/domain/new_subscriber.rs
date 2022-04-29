use super::{SubscriberEmail, SubscriberName};

use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct NewSubscriber {
    pub email: SubscriberEmail,
    pub name: SubscriberName,
}
