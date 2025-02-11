#[cfg(test)]
mod tests {
    use claim::{assert_err, assert_ok};
    use fake::faker::lorem::en::Sentence;
    use fake::{Fake, Faker};
    use reqwest::Url;
    use secrecy::{ExposeSecret, SecretString};
    use twilio_client::sms::{Client, SendSmsResponse};
    use twilio_client::Phone;
    use wiremock::matchers::{any, header, method, path};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    fn generate_phone() -> (String, String) {
        (String::from("0700782326"), String::from("KE"))
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

    fn sms_client(base_url: &str) -> (Client, SecretString) {
        let base_url = Url::parse(base_url).expect("Failed to parse base uri");
        let auth_token = 13.fake::<String>();
        let auth_token = SecretString::from(auth_token);
        let account_sid = SecretString::from(Faker.fake::<String>());

        (
            Client::builder()
                .base_url(base_url)
                .sender(phone())
                .account_sid(account_sid.clone())
                .auth_token(auth_token)
                .timeout(std::time::Duration::from_secs(1))
                .build()
                .unwrap(),
            account_sid,
        )
    }

    #[tokio::test]
    async fn send_sms_sends_expected_request() {
        let mock_server = MockServer::start().await;
        let (sms_client, account_sid) = sms_client(&mock_server.uri());

        Mock::given(header("Content-Type", "application/x-www-form-urlencoded"))
            .and(method("POST"))
            .and(path(format!(
                "/2010-04-01/Accounts/{}/Messages.json",
                account_sid.expose_secret()
            )))
            // Use SendSmsBodyMatcher!
            .and(SendSmsBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let _ = sms_client
            .send(&phone(), content().as_ref(), None, None)
            .await;
    }

    #[tokio::test]
    async fn send_sms_succeeds_if_the_server_returns_200() {
        let mock_server = MockServer::start().await;
        let (sms_client, _) = sms_client(&mock_server.uri());
        let template = ResponseTemplate::new(200).set_body_json(SendSmsResponse::default());

        Mock::given(any())
            .respond_with(template)
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = sms_client
            .send(&phone(), content().as_ref(), None, None)
            .await;

        assert_ok!(outcome);
    }

    #[tokio::test]
    async fn send_sms_fails_if_the_server_returns_500() {
        let mock_server = MockServer::start().await;
        let (sms_client, _) = sms_client(&mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = sms_client
            .send(&phone(), content().as_ref(), None, None)
            .await;

        assert_err!(outcome);
    }

    #[tokio::test]
    async fn send_sms_times_out_if_the_server_takes_too_long() {
        let mock_server = MockServer::start().await;
        let (sms_client, _) = sms_client(&mock_server.uri());

        let response = ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(180));

        Mock::given(any())
            .respond_with(response)
            .expect(1)
            .mount(&mock_server)
            .await;
        let outcome = sms_client
            .send(&phone(), content().as_ref(), None, None)
            .await;

        assert_err!(outcome);
    }

    struct SendSmsBodyMatcher;

    impl wiremock::Match for SendSmsBodyMatcher {
        fn matches(&self, request: &Request) -> bool {
            // Try to parse the body as a JSON value
            let result: Result<serde_json::Value, _> = serde_urlencoded::from_bytes(&request.body);
            if let Ok(body) = result {
                // Check that all the mandatory fields are populated
                body.get("From").is_some() && body.get("To").is_some() && body.get("Body").is_some()
            } else {
                // If parsing failed, do not match the request
                false
            }
        }
    }
}
