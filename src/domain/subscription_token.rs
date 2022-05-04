use unicode_segmentation::UnicodeSegmentation;

pub struct SubscriptionToken(String);

impl SubscriptionToken {
    pub fn parse(s: String) -> Result<Self, String> {
        let has_invalid_length = s.graphemes(true).count() != 25;
        let is_not_alphanumeric = s.chars().any(|c| !c.is_alphanumeric());
        dbg!(&s, has_invalid_length, is_not_alphanumeric);

        if has_invalid_length || is_not_alphanumeric {
            Err("Invalid subscription token".to_string())
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
