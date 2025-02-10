use std::time::Duration;

use crate::error::ClientError;
use crate::models::{Phone};
use crate::sms::DEFAULT_TIMEOUT;
use reqwest::Url;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use crate::make_request::make_request;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwilioRequestResponse {
    pub status: Option<Status>,
    pub send_code_attempts: Option<Vec<SendCodeAttempt>>,
    pub to: Option<String>,
    pub valid: Option<bool>,
    pub date_created: Option<String>,
    pub date_updated: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendCodeAttempt {
    pub attempt_sid: String,
    pub channel: Channel,
    pub time: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TwilioVerifyResponse {
    pub status: Status,
    pub payee: Option<String>,
    pub date_updated: String,
    pub account_sid: String,
    pub to: String,
    pub amount: Option<f32>,
    pub valid: bool,
    pub sid: String,
    pub date_created: String,
    pub service_sid: String,
    pub channel: Channel,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    Sms
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Pending,
    Approved,
}

#[derive(Debug, Clone, Default)]
pub struct ClientBuilder {
    base_url: Option<Url>,
    service_sid: Option<SecretString>,
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

    pub fn account_sid(mut self, account_sid: SecretString) -> Self {
        self.account_sid = Some(account_sid);
        self
    }

    pub fn service_sid(mut self, service_sid: SecretString) -> Self {
        self.service_sid = Some(service_sid);
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
            ClientError::Configuration("Twilio verify base_url is required".to_string())
        })?;
        let account_sid = self.account_sid.ok_or_else(|| {
            ClientError::Configuration("Twilio verify account_sid is required".to_string())
        })?;
        let service_sid = self.service_sid.ok_or_else(|| {
            ClientError::Configuration("Twilio verify service_sid is required".to_string())
        })?;
        let auth_token = self.auth_token.ok_or_else(|| {
            ClientError::Configuration("Twilio verify auth_token is required".to_string())
        })?;

        let timeout = self.timeout.unwrap_or(DEFAULT_TIMEOUT);

        let http_client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(ClientError::Reqwest)?;

        Ok(Client {
            http_client,
            base_url,
            service_sid,
            account_sid,
            auth_token,
            timeout,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Client {
    http_client: reqwest::Client,
    base_url: Url,
    account_sid: SecretString,
    auth_token: SecretString,
    service_sid: SecretString,
    timeout: Duration,
}

impl Client {
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    #[tracing::instrument(name = "Twilio Verify: Request OTP to phone", skip(self, to))]
    pub async fn request(&self, to: &Phone) -> Result<TwilioRequestResponse, ClientError> {
        let service_sid = self.service_sid.expose_secret();
        let url = format!("/v2/Services/{service_sid}/Verifications");

        let mut body = std::collections::HashMap::new();
        body.insert("To", to.e164_number());
        body.insert("Channel", "sms".to_string());

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

    #[tracing::instrument(name = "Twilio Verify: Verify OTP", skip(self, to, code))]
    pub async fn verify(
        &self,
        to: &Phone,
        code: SecretString,
    ) -> Result<TwilioVerifyResponse, ClientError> {
        let service_sid = self.service_sid.expose_secret();
        let url = format!("/v2/Services/{service_sid}/VerificationCheck");

        let mut body = std::collections::HashMap::new();
        body.insert("To", to.e164_number());
        body.insert("Code", code.expose_secret().to_string());

        make_request(
            &self.http_client,
            (&self.base_url, &url),
            &self.account_sid,
            &self.auth_token,
            self.timeout,
            &body,
            "Twilio Verify",
        )
            .await
    }
}
