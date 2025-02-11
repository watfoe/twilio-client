#[cfg(test)]
mod tests {
    use claim::{assert_err, assert_ok};
    use fake::faker::lorem::en::Sentence;
    use fake::{Fake, Faker};
    use reqwest::Url;
    use secrecy::{ExposeSecret, SecretString};
    use serde::Serialize;
    use twilio_client::verify::Client;
    use twilio_client::Phone;
    use wiremock::matchers::{any, header, method, path};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    fn generate_phone() -> (String, String) {
        (String::from("0700123456"), String::from("KE"))
    }

    // Generate a random sms content
    fn content() -> String {
        Sentence(1..2).fake()
    }

    // Generate a random user phone
    fn phone() -> Phone {
        let (number, country_id) = generate_phone();
        Phone::parse(&number, country_id.as_str()).unwrap()
    }

    fn twilio_verify_client(base_url: &str) -> (Client, SecretString) {
        let base_url = Url::parse(base_url).expect("Failed to parse base uri");
        let account_sid = SecretString::from(Faker.fake::<String>());
        let auth_token = SecretString::from(Faker.fake::<String>());
        let service_sid = SecretString::from(Faker.fake::<String>());

        (
            Client::builder()
                .base_url(base_url)
                .service_sid(service_sid.clone())
                .account_sid(account_sid.clone())
                .auth_token(auth_token)
                .timeout(std::time::Duration::from_secs(1))
                .build()
                .unwrap(),
            service_sid,
        )
    }

    #[tokio::test]
    async fn request_verify_sends_expected_request() {
        let mock_server = MockServer::start().await;
        let (client, service_sid) = twilio_verify_client(&mock_server.uri());

        Mock::given(header("Content-Type", "application/x-www-form-urlencoded"))
            .and(method("POST"))
            .and(path(format!(
                "/v2/Services/{}/Verifications",
                service_sid.expose_secret()
            )))
            .and(RequestTwilioVerifyBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let _ = client.request(&phone()).await;
    }

    #[tokio::test]
    async fn send_sms_succeeds_if_the_server_returns_200() {
        let mock_server = MockServer::start().await;
        let (client, _) = twilio_verify_client(&mock_server.uri());
        let template =
            ResponseTemplate::new(200).set_body_json(RequestTwilioVerifyBodyMatcher::new());

        Mock::given(any())
            .respond_with(template)
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = client.request(&phone()).await;

        assert_ok!(outcome);
    }

    #[tokio::test]
    async fn send_sms_fails_if_the_server_returns_500() {
        let mock_server = MockServer::start().await;
        let (client, _) = twilio_verify_client(&mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = client.request(&phone()).await;

        assert_err!(outcome);
    }

    #[tokio::test]
    async fn send_sms_times_out_if_the_server_takes_too_long() {
        let mock_server = MockServer::start().await;
        let (client, _) = twilio_verify_client(&mock_server.uri());
        let response = ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(180));

        Mock::given(any())
            .respond_with(response)
            .expect(1)
            .mount(&mock_server)
            .await;
        let outcome = client.request(&phone()).await;

        assert_err!(outcome);
    }

    #[derive(Serialize)]
    struct RequestTwilioVerifyBodyMatcher;

    impl RequestTwilioVerifyBodyMatcher {
        fn new() -> Self {
            Self
        }
    }

    impl wiremock::Match for RequestTwilioVerifyBodyMatcher {
        fn matches(&self, request: &Request) -> bool {
            // Try to parse the body as a JSON value
            let result: Result<serde_json::Value, _> = serde_urlencoded::from_bytes(&request.body);
            if let Ok(body) = result {
                // Check that all the mandatory fields are populated
                body.get("Channel").is_some() && body.get("To").is_some()
            } else {
                // If parsing failed, do not match the request
                false
            }
        }
    }
}
