use validator::ValidateEmail;

#[derive(Debug)]
pub struct PlayerEmail(String);

impl PlayerEmail {
    pub fn parse(s: String) -> Result<PlayerEmail, String> {
        if s.validate_email() {
            Ok(Self(s))
        } else {
            Err(format!("{s} is not a valid subscriber email."))
        }
    }
}

impl AsRef<str> for PlayerEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::PlayerEmail;
    use claims::assert_err;
    #[test]
    fn empty_string_is_rejected() {
        let email = "".to_string();
        assert_err!(PlayerEmail::parse(email));
    }
    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "ursuladomain.com".to_string();
        assert_err!(PlayerEmail::parse(email));
    }
    #[test]
    fn email_missing_subject_is_rejected() {
        let email = "@domain.com".to_string();
        assert_err!(PlayerEmail::parse(email));
    }
}
