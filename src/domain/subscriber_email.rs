use validator::validate_email;

#[derive(Debug, Clone)]
pub struct SubscriberEmail(String);

impl SubscriberEmail {
    pub fn parse(s: String) -> Result<SubscriberEmail, String> {
        if validate_email(&s) {
            Ok(Self(s))
        } else {
            Err(format!("{} is not a valid subscriber email", s))
        }
    }

    pub fn into_inner(self) -> String {
        self.0
    }

    pub fn inner_ref(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        self.inner_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::SubscriberEmail;
    use claim::assert_err;

    #[test]
    fn empty_string_is_rejected() {
        let email = "".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "ursuladomain.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_subject_is_rejected() {
        let email = "@domain.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    use quickcheck::{quickcheck, Arbitrary};

    #[derive(Clone, Debug)]
    struct ValidEmailFixture(pub String);

    impl Arbitrary for ValidEmailFixture {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            use fake::faker::internet::en::SafeEmail;
            use fake::Fake;
            let email = SafeEmail().fake_with_rng(g);
            Self(email)
        }
    }

    quickcheck! {
        fn valid_emails_are_parsed_successfully(email: ValidEmailFixture) -> bool {
            SubscriberEmail::parse(email.0).is_ok()
        }
    }
}
