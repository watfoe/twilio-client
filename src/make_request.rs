use std::collections::HashMap;
use std::time::Duration;

use crate::error::ClientError;
use crate::sms::SendSmsResponse;
use crate::Phone;
use reqwest::Url;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

pub(crate) async fn make_request<T>(
    http_client: &reqwest::Client,
    urls: (&Url, &str),
    account_sid: &SecretString,
    auth_token: &SecretString,
    timeout: Duration,
    body: &HashMap<&str, String>,
    service_name: &str,
) -> Result<T, ClientError> {
    let account_sid = account_sid.expose_secret();

    let url = urls
        .0
        .join(urls.1)
        .map_err(|e| ClientError::Configuration(format!("{service_name}: invalid URL: {}", e)))?;

    let req = http_client
        .post(url.as_str())
        .basic_auth(account_sid, Some(auth_token.expose_secret()))
        .form(body)
        .build()?;

    let resp = http_client.execute(req).await.map_err(|err| {
        tracing::error!("{service_name}: failed to send sms: {}", err);
        if err.is_timeout() {
            ClientError::Timeout(timeout.as_secs())
        } else {
            ClientError::Reqwest(err)
        }
    })?;

    let status_code = resp.status();
    let message = resp.text().await.map_err(|err| {
        tracing::error!("{service_name}: failed to read response body: {}", err);
        ClientError::Reqwest(err)
    })?;

    if status_code.is_success() {
        serde_json::from_str(&message).map_err(|err| {
            tracing::error!("{service_name}: failed to parse response: {}", err);
            ClientError::Serde(err)
        })
    } else if status_code.as_str() == "401" {
        Err(ClientError::Authentication(message))
    } else {
        Err(ClientError::ServerResponse {
            status_code,
            message,
        })
    }
}

fn urlencode_from_string<T: AsRef<str>>(s: T) -> String {
    url::form_urlencoded::byte_serialize(s.as_ref().as_bytes()).collect()
}
