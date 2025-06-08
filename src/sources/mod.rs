use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use thiserror::Error;

pub mod freebox;

#[derive(Debug, Error)]
pub enum SourceError {
    #[error("Authentication error: {0}")]
    AuthenticationError(String),
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Connection error: {0}")]
    ConnectionError(String),
    #[error("IP not found: {0}")]
    IpNotFoundError(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IpVersion {
    IPv4,
    IPv6,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpAddress {
    pub version: IpVersion,
    pub address: String,
    pub last_updated: Option<SystemTime>,
}

#[async_trait]
pub trait IpSource: Send + Sync {
    /// Get the current IP address for the specified version
    async fn get_ip(&mut self, version: IpVersion) -> Result<IpAddress, SourceError>;
}

/// Validate an IP address based on its version
pub fn validate_ip_address(ip: &str, version: &IpVersion) -> Result<(), SourceError> {
    match version {
        IpVersion::IPv4 => {
            if !ip.split('.').all(|octet| octet.parse::<u8>().is_ok()) {
                return Err(SourceError::ValidationError(
                    "Invalid IPv4 address format".to_string(),
                ));
            }
        }
        IpVersion::IPv6 => {
            // Basic IPv6 validation - could be made more strict
            if !ip.contains(':') || ip.split(':').count() > 8 {
                return Err(SourceError::ValidationError(
                    "Invalid IPv6 address format".to_string(),
                ));
            }
        }
    }
    Ok(())
}
