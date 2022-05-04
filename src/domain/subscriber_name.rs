use {serde::Deserialize, unicode_segmentation::UnicodeSegmentation};

#[derive(Debug, thiserror::Error)]
pub enum SubscriberNameValidationError {
    #[error("Name cannot be empty")]
    EmptyOrWhitespace,
    #[error("Name must be shorter than 256 characters")]
    TooLong,
    #[error("Name may not contain any of the following characters: /()\"<>\\{{}}")]
    ForbiddenCharacters,
}

#[derive(Deserialize, Debug)]
pub struct SubscriberName(String);

impl SubscriberName {
    pub fn parse(s: String) -> Result<Self, SubscriberNameValidationError> {
        let forbidden_chars = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];

        if s.trim().is_empty() {
            Err(SubscriberNameValidationError::EmptyOrWhitespace)
        } else if s.graphemes(true).count() > 256 {
            Err(SubscriberNameValidationError::TooLong)
        } else if s.chars().any(|g| forbidden_chars.contains(&g)) {
            Err(SubscriberNameValidationError::ForbiddenCharacters)
        } else {
            Ok(Self(s))
        }
    }
}

impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::SubscriberName;
    use claim::{assert_err, assert_ok};

    #[test]
    fn a_256_grapheme_long_name_is_valid() {
        let name = "ё".repeat(256);
        assert_ok!(SubscriberName::parse(name));
    }

    #[test]
    fn a_name_longer_than_256_graphemes_is_rejected() {
        let name = "ё".repeat(257);
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn whitespace_only_name_rejected() {
        let name = " ".to_string();
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn invalid_chars_rejected() {
        for name in &['/', '(', ')', '"', '<', '>', '\\', '{', '}'] {
            let name = name.to_string();
            assert_err!(SubscriberName::parse(name));
        }
    }

    #[test]
    fn a_valid_name_is_parsed_successfully() {
        let name = "Ursula Le Guin".to_string();
        assert_ok!(SubscriberName::parse(name));
    }
}
