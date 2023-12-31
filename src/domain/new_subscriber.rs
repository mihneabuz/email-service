use crate::{
    domain::{SubscriberEmail, SubscriberName},
    routes::SubscribeData,
};

pub struct NewSubscriber {
    pub email: SubscriberEmail,
    pub name: SubscriberName,
}

impl TryFrom<SubscribeData> for NewSubscriber {
    type Error = String;

    fn try_from(value: SubscribeData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;
        Ok(Self { email, name })
    }
}
