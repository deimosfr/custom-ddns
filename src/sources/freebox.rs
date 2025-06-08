use super::{IpAddress, IpSource, IpVersion, SourceError, validate_ip_address};
use async_trait::async_trait;
use hex;
use hmac::{Hmac, Mac};
use reqwest::{
    Client, StatusCode,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use std::time::SystemTime;

const FREEBOX_API_BASE_URL: &str = "/api/v13";

#[derive(Debug, Serialize, Deserialize)]
struct FreeboxApiResponse<T> {
    success: bool,
    result: T,
    error_code: Option<String>,
    msg: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FreeboxConnectionStatus {
    state: String,
    #[serde(rename = "type")]
    connection_type: String,
    ipv4: Option<String>,
    ipv6: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FreeboxConnection {
    status: FreeboxConnectionStatus,
    session_token: String,
}

impl FreeboxConnection {
    pub fn new(status: FreeboxConnectionStatus, session_token: String) -> Self {
        Self {
            status,
            session_token,
        }
    }
}
#[derive(Debug, Serialize, Deserialize)]
struct FreeboxLoginResult {
    logged_in: bool,
    challenge: Option<String>,
    password_salt: Option<String>,
    password_set: bool,
}

#[derive(Debug, Serialize)]
struct FreeboxSessionRequest {
    app_id: String,
    password: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct FreeboxSessionResult {
    session_token: String,
    challenge: Option<String>,
}

pub struct FreeboxSource {
    client: Client,
    base_url: String,
    app_token: String,
    challenge: Option<String>,
    password_salt: Option<String>,
    session_token: Option<String>,
}

impl FreeboxSource {
    pub fn new(url: Option<String>, app_token: String) -> Result<Self, SourceError> {
        Ok(Self {
            client: Client::new(),
            base_url: url.unwrap_or_else(|| "http://mafreebox.freebox.fr".to_string()),
            app_token,
            challenge: None,
            password_salt: None,
            session_token: None,
        })
    }

    fn get_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("x-fbx-app-auth"),
            HeaderValue::from_str(&self.session_token.clone().unwrap()).unwrap(),
        );
        headers
    }

    fn get_session_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("content-type"),
            HeaderValue::from_static("application/json"),
        );
        headers.insert(
            HeaderName::from_static("charset"),
            HeaderValue::from_static("utf-8"),
        );
        headers.insert(
            HeaderName::from_static("accept"),
            HeaderValue::from_static("text/plain"),
        );
        headers
    }

    // login to the freebox to get the challenge and password salt
    async fn login(&mut self) -> Result<(), SourceError> {
        let url = format!("{}{}/login/", self.base_url, FREEBOX_API_BASE_URL);

        let response = self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(3))
            .send()
            .await
            .map_err(|e| SourceError::NetworkError(e.to_string()))?;

        if response.status() != StatusCode::OK {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SourceError::ApiError(format!(
                "Freebox login error: {}",
                error_text
            )));
        }

        let api_response: FreeboxApiResponse<FreeboxLoginResult> = response
            .json()
            .await
            .map_err(|e| SourceError::ApiError(e.to_string()))?;

        if !api_response.success {
            return Err(SourceError::ApiError(format!(
                "Freebox login challenge failure: {:?}",
                api_response
            )));
        }

        let result = api_response.result;
        if !result.logged_in {
            self.challenge = result.challenge;
            self.password_salt = result.password_salt;
        }

        Ok(())
    }

    // open a session to the freebox
    async fn open_session(&mut self) -> Result<(), SourceError> {
        let challenge = self.challenge.as_ref().ok_or_else(|| {
            SourceError::ApiError("No challenge available. Call login() first.".to_string())
        })?;

        // Create HMAC-SHA1 hash of app_token with challenge as key
        let mut mac = Hmac::<Sha1>::new_from_slice(self.app_token.as_bytes())
            .map_err(|e| SourceError::ApiError(format!("HMAC creation failed: {}", e)))?;
        mac.update(challenge.as_bytes());
        let password = hex::encode(mac.finalize().into_bytes());

        let url = format!("{}{}/login/session/", self.base_url, FREEBOX_API_BASE_URL);
        let payload = FreeboxSessionRequest {
            app_id: "fr.freebox.cddns".to_string(),
            password,
        };

        let response = self
            .client
            .post(&url)
            .headers(self.get_session_headers())
            .json(&payload)
            .timeout(std::time::Duration::from_secs(3))
            .send()
            .await
            .map_err(|e| SourceError::NetworkError(e.to_string()))?;

        if response.status() != StatusCode::OK {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SourceError::ApiError(format!(
                "Freebox session error: {}",
                error_text
            )));
        }

        let api_response: FreeboxApiResponse<FreeboxSessionResult> = response
            .json()
            .await
            .map_err(|e| SourceError::ApiError(e.to_string()))?;

        if !api_response.success {
            return Err(SourceError::ApiError(format!(
                "Freebox session failure: {:?}",
                api_response
            )));
        }

        let result = api_response.result;
        self.session_token = Some(result.session_token);

        Ok(())
    }

    async fn get_connection_status(&mut self) -> Result<FreeboxConnection, SourceError> {
        self.login().await?;
        self.open_session().await?;

        let session_token = match &self.session_token {
            Some(token) => token.clone(),
            None => {
                return Err(SourceError::ApiError(
                    "No session token available. Call open_session() first.".to_string(),
                ));
            }
        };

        let url = format!("{}{}/connection/", self.base_url, FREEBOX_API_BASE_URL);

        let response = self
            .client
            .get(&url)
            .headers(self.get_headers())
            .send()
            .await
            .map_err(|e| SourceError::NetworkError(e.to_string()))?;

        match response.status() {
            StatusCode::OK => {
                let api_response: FreeboxApiResponse<FreeboxConnectionStatus> = response
                    .json()
                    .await
                    .map_err(|e| SourceError::ApiError(e.to_string()))?;

                if !api_response.success {
                    return Err(SourceError::ApiError(
                        api_response
                            .msg
                            .unwrap_or_else(|| "Unknown error".to_string()),
                    ));
                }

                let connection = FreeboxConnection::new(api_response.result, session_token);

                Ok(connection)
            }
            StatusCode::UNAUTHORIZED => Err(SourceError::AuthenticationError(
                "Invalid API key".to_string(),
            )),
            _ => Err(SourceError::ApiError(format!(
                "Failed to get connection status: {}",
                response.text().await.unwrap_or_default()
            ))),
        }
    }
}

impl Clone for FreeboxSource {
    fn clone(&self) -> Self {
        Self {
            client: Client::new(),
            base_url: self.base_url.clone(),
            app_token: self.app_token.clone(),
            challenge: self.challenge.clone(),
            password_salt: self.password_salt.clone(),
            session_token: self.session_token.clone(),
        }
    }
}

#[async_trait]
impl IpSource for FreeboxSource {
    async fn get_ip(&mut self, version: IpVersion) -> Result<IpAddress, SourceError> {
        let status = self.get_connection_status().await?;

        if status.status.state != "up" {
            return Err(SourceError::ConnectionError(format!(
                "Connection is not up (state: {})",
                status.status.state
            )));
        }

        let ip = match version {
            IpVersion::IPv4 => status.status.ipv4.ok_or_else(|| {
                SourceError::IpNotFoundError("No IPv4 address available".to_string())
            })?,
            IpVersion::IPv6 => status.status.ipv6.ok_or_else(|| {
                SourceError::IpNotFoundError("No IPv6 address available".to_string())
            })?,
        };

        validate_ip_address(&ip, &version)?;

        Ok(IpAddress {
            version,
            address: ip,
            last_updated: Some(SystemTime::now()),
        })
    }
}
