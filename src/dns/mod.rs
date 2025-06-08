use crate::{
    config::{ConfigDnsProvider, RecordType},
    dns::cloudflare::CloudflareDns,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod cloudflare;

#[derive(Debug, Error)]
pub enum DnsError {
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Zone not found: {0}")]
    ZoneNotFound(String),
}

pub enum DnsClient {
    Cloudflare(CloudflareDns),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecordCloudflare {
    pub id: Option<String>,
    pub name: String,
    pub content: String,
    pub record_type: RecordType,
    pub ttl: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsZone {
    pub name: String,
    pub provider: ConfigDnsProvider,
    pub records: Vec<DnsRecordCloudflare>,
}

#[async_trait]
pub trait DnsProvider: Send + Sync {
    async fn update_record(&self, zone: &str, record: &DnsRecordCloudflare)
    -> Result<(), DnsError>;
    async fn get_record_content(
        &self,
        zone: &str,
        record_name: &str,
        record_type: &RecordType,
    ) -> Result<Option<String>, DnsError>;
    fn validate_record(&self, record: &DnsRecordCloudflare) -> Result<(), DnsError>;
}

// Validation functions
pub fn validate_record_name(name: &str) -> Result<(), DnsError> {
    if name.is_empty() {
        return Err(DnsError::ValidationError(
            "Record name cannot be empty".to_string(),
        ));
    }
    if !name.ends_with('.') {
        return Err(DnsError::ValidationError(
            "Record name must end with a dot".to_string(),
        ));
    }
    Ok(())
}

pub fn validate_record_data(content: &str, record_type: &RecordType) -> Result<(), DnsError> {
    if content.is_empty() {
        return Err(DnsError::ValidationError(
            "Record content cannot be empty".to_string(),
        ));
    }

    match record_type {
        RecordType::A => {
            if !content.split('.').all(|octet| octet.parse::<u8>().is_ok()) {
                return Err(DnsError::ValidationError(
                    "Invalid IPv4 address format".to_string(),
                ));
            }
        }
        RecordType::Aaaa => {
            if !content.split(':').all(|segment| {
                segment.is_empty()
                    || segment.len() <= 4 && segment.chars().all(|c| c.is_ascii_hexdigit())
            }) {
                return Err(DnsError::ValidationError(
                    "Invalid IPv6 address format".to_string(),
                ));
            }
        }
        RecordType::Cname | RecordType::Mx => {
            if !content.ends_with('.') {
                return Err(DnsError::ValidationError(
                    "CNAME and MX records must end with a dot".to_string(),
                ));
            }
        }
        RecordType::Txt => {
            // TXT records can contain any printable ASCII characters
            if !content
                .chars()
                .all(|c| c.is_ascii() && !c.is_ascii_control())
            {
                return Err(DnsError::ValidationError(
                    "TXT record contains invalid characters".to_string(),
                ));
            }
        }
        RecordType::Srv => {
            // SRV record format: priority weight port target
            let parts: Vec<&str> = content.split_whitespace().collect();
            if parts.len() != 4 {
                return Err(DnsError::ValidationError(
                    "SRV record must have format: priority weight port target".to_string(),
                ));
            }
            if parts[0].parse::<u16>().is_err()
                || parts[1].parse::<u16>().is_err()
                || parts[2].parse::<u16>().is_err()
            {
                return Err(DnsError::ValidationError(
                    "SRV record priority, weight, and port must be numbers".to_string(),
                ));
            }
            if !parts[3].ends_with('.') {
                return Err(DnsError::ValidationError(
                    "SRV record target must end with a dot".to_string(),
                ));
            }
        }
    }

    Ok(())
}

pub fn validate_ttl(ttl: u32) -> Result<(), DnsError> {
    if !(60..=86400).contains(&ttl) {
        return Err(DnsError::ValidationError(
            "TTL must be between 60 and 86400 seconds".to_string(),
        ));
    }
    Ok(())
}
