use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, thiserror::Error)]
pub enum SubTokenValidationError {
    #[error("Token must be 25 characters long")]
    InvalidLength,
    #[error("Token may only be ASCII alphanumeric characters")]
    NotAlphanumeric,
}

pub struct SubscriptionToken(String);

impl SubscriptionToken {
    pub fn parse(s: String) -> Result<Self, SubTokenValidationError> {
        if s.graphemes(true).count() != 25 {
            Err(SubTokenValidationError::InvalidLength)
        } else if s.chars().any(|c| !c.is_alphanumeric()) {
            Err(SubTokenValidationError::NotAlphanumeric)
        } else {
            Ok(Self(s))
        }
    }
}

impl AsRef<str> for SubscriptionToken {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
