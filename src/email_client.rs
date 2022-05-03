use crate::domain::SubscriberEmail;

use std::time::Duration;

use {
    reqwest::{Client, Url},
    secrecy::{ExposeSecret, Secret},
    serde::Serialize,
};

pub struct EmailClient {
    sender: SubscriberEmail,
    http_client: Client,
    base_url: Url,
    authorization_token: Secret<String>,
}

impl EmailClient {
    pub fn new(
        base_url: String,
        sender: SubscriberEmail,
        authorization_token: Secret<String>,
        timeout: Duration,
    ) -> Result<Self, String> {
        let http_client = Client::builder().timeout(timeout).build().unwrap();
        Ok(Self {
            http_client,
            base_url: Url::parse(&base_url).map_err(|_| "Invalid base url")?,
            sender,
            authorization_token,
        })
    }

    pub async fn send_email(
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), reqwest::Error> {
        let url = self
            .base_url
            .join("email")
            .expect("Failed to join base URL with `/email` endpoint");

        let request_body = SendEmailRequest {
            from: self.sender.as_ref(),
            to: recipient.as_ref(),
            subject,
            html_body: html_content,
            text_body: text_content,
        };

        self.http_client
            .post(url)
            .header(
                "X-Postmark-Server-Token",
                self.authorization_token.expose_secret(),
            )
            .json(&request_body)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    html_body: &'a str,
    text_body: &'a str,
}

#[cfg(test)]
mod test {
    use super::EmailClient;
    use crate::domain::SubscriberEmail;
    use std::time::Duration;
    use {
        claim::{assert_err, assert_ok},
        fake::{
            faker::{
                internet::en::SafeEmail,
                lorem::en::{Paragraph, Sentence},
            },
            Fake, Faker,
        },
        secrecy::Secret,
        wiremock::{matchers, Mock, MockServer, ResponseTemplate},
    };

    #[tokio::test]
    async fn send_email_sends_expected_request() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());
        let (subscriber_email, subject, content) = mock_content();

        Mock::given(matchers::header_exists("X-Postmark-Server-Token"))
            .and(matchers::header("Content-Type", "application/json"))
            .and(matchers::path("/email"))
            .and(matchers::method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;

        assert_ok!(outcome);
    }

    #[tokio::test]
    async fn send_email_fails_on_server_500() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());
        let (subscriber_email, subject, content) = mock_content();

        Mock::given(matchers::any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;

        assert_err!(outcome);
    }

    #[tokio::test]
    async fn send_email_times_out() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());
        let (subscriber_email, subject, content) = mock_content();

        let response = ResponseTemplate::new(200).set_delay(Duration::from_secs(180));

        Mock::given(matchers::any())
            .respond_with(response)
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;

        assert_err!(outcome);
    }

    fn email_client(base_url: String) -> EmailClient {
        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        EmailClient::new(
            base_url,
            sender,
            Secret::new(Faker.fake()),
            Duration::from_millis(200),
        )
        .unwrap()
    }

    fn mock_content() -> (SubscriberEmail, String, String) {
        let subscriber_email = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let subject: String = Sentence(1..2).fake();
        let content: String = Paragraph(1..10).fake();
        (subscriber_email, subject, content)
    }

    struct SendEmailBodyMatcher;
    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &wiremock::Request) -> bool {
            match serde_json::from_slice::<serde_json::Value>(&request.body) {
                Ok(body) => {
                    body.get("From").is_some()
                        && body.get("To").is_some()
                        && body.get("Subject").is_some()
                        && body.get("HtmlBody").is_some()
                        && body.get("TextBody").is_some()
                }
                Err(_) => false,
            }
        }
    }
}
