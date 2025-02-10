use std::time::Duration;

use crate::error::ClientError;
use crate::make_request::make_request;
use crate::Phone;
use reqwest::Url;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SendSmsResponse {
    #[serde(skip_serializing_if = "Option::is_none", alias = "Body")]
    pub body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_created: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_sent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_updated: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<Status>,
    #[serde(skip_serializing_if = "Option::is_none", alias = "To")]
    pub to: Option<String>,
}

/// The status of the message
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Queued,
    Sending,
    Sent,
    Failed,
    Delivered,
    Undelivered,
    Receiving,
    Received,
    Accepted,
    Scheduled,
    Read,
    #[serde(rename = "partially_delivered")]
    PartiallyDelivered,
    Canceled,
}

#[derive(Debug, Clone, Default)]
pub struct ClientBuilder {
    base_url: Option<Url>,
    sender: Option<Phone>,
    account_sid: Option<SecretString>,
    auth_token: Option<SecretString>,
    timeout: Option<Duration>,
}

impl ClientBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn base_url(mut self, url: Url) -> Self {
        self.base_url = Some(url);
        self
    }

    pub fn sender(mut self, sender: Phone) -> Self {
        self.sender = Some(sender);
        self
    }

    pub fn account_sid(mut self, account_sid: SecretString) -> Self {
        self.account_sid = Some(account_sid);
        self
    }

    pub fn auth_token(mut self, token: SecretString) -> Self {
        self.auth_token = Some(token);
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn build(self) -> Result<Client, ClientError> {
        let base_url = self.base_url.ok_or_else(|| {
            ClientError::Configuration("Twilio sms base_url is required".to_string())
        })?;
        let sender = self.sender.ok_or_else(|| {
            ClientError::Configuration("Twilio sms sender phone is required".to_string())
        })?;
        let account_sid = self.account_sid.ok_or_else(|| {
            ClientError::Configuration("Twilio sms account_sid is required".to_string())
        })?;
        let auth_token = self.auth_token.ok_or_else(|| {
            ClientError::Configuration("Twilio sms auth_token is required".to_string())
        })?;

        let timeout = self.timeout.unwrap_or(DEFAULT_TIMEOUT);

        let http_client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(ClientError::Reqwest)?;

        Ok(Client {
            http_client,
            base_url,
            account_sid,
            sender,
            auth_token,
            timeout,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Client {
    http_client: reqwest::Client,
    base_url: Url,
    sender: Phone,
    account_sid: SecretString,
    auth_token: SecretString,
    timeout: Duration,
}

impl Client {
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    #[tracing::instrument(name = "Twilio SMS: Send sms", skip(self, body))]
    pub async fn send(
        &self,
        to: &Phone,
        content: &str,
        send_as_mms: Option<bool>,
        media_url: Option<Vec<String>>,
    ) -> Result<SendSmsResponse, ClientError> {
        let account_sid = self.account_sid.expose_secret();
        let url = format!(
            "/2010-04-01/Accounts/{AccountSid}/Messages.json",
            AccountSid = urlencode_from_string(account_sid)
        );

        let mut body = std::collections::HashMap::new();
        body.insert("From", self.sender.e164_number());
        body.insert("To", to.e164_number());
        body.insert("Body", content.to_string());

        if let Some(urls) = media_url {
            body.insert(
                "MediaUrl",
                urls.into_iter().collect::<Vec<String>>().join(","),
            );
        }
        if let Some(param_value) = send_as_mms {
            body.insert("SendAsMms", param_value.to_string());
        }

        make_request(
            &self.http_client,
            (&self.base_url, &url),
            &self.account_sid,
            &self.auth_token,
            self.timeout,
            &body,
            "Twilio SMS",
        )
        .await
    }
}

fn urlencode_from_string<T: AsRef<str>>(s: T) -> String {
    url::form_urlencoded::byte_serialize(s.as_ref().as_bytes()).collect()
}
