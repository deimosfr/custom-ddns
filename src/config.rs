use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::str::FromStr;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum RecordType {
    A,
    Aaaa,
    Cname,
    Mx,
    Txt,
    Srv,
}

impl fmt::Display for RecordType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecordType::A => write!(f, "A"),
            RecordType::Aaaa => write!(f, "AAAA"),
            RecordType::Cname => write!(f, "CNAME"),
            RecordType::Mx => write!(f, "MX"),
            RecordType::Txt => write!(f, "TXT"),
            RecordType::Srv => write!(f, "SRV"),
        }
    }
}

impl FromStr for RecordType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "A" => Ok(RecordType::A),
            "AAAA" => Ok(RecordType::Aaaa),
            "CNAME" => Ok(RecordType::Cname),
            "MX" => Ok(RecordType::Mx),
            "TXT" => Ok(RecordType::Txt),
            "SRV" => Ok(RecordType::Srv),
            _ => Err(format!("Invalid record type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfigDnsProvider {
    #[serde(rename = "cloudflare")]
    Cloudflare,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsProviderConfig {
    pub provider: ConfigDnsProvider,
    pub api_key: Option<String>,
    pub project_id: Option<String>,
    pub rate_limit: Option<RateLimit>,
    pub retry_config: Option<RetryConfig>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RateLimit {
    pub requests_per_second: u32,
    pub burst_size: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay: u64, // seconds
    pub max_delay: u64,     // seconds
    pub backoff_factor: f64,
}

#[derive(Debug)]
pub enum ConfigError {
    IoError(std::io::Error),
    ParseError(serde_yaml::Error),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::IoError(e) => write!(f, "IO error: {}", e),
            ConfigError::ParseError(e) => write!(f, "Parse error: {}", e),
        }
    }
}

impl Error for ConfigError {}

unsafe impl Send for ConfigError {}
unsafe impl Sync for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(err: std::io::Error) -> Self {
        ConfigError::IoError(err)
    }
}

impl From<serde_yaml::Error> for ConfigError {
    fn from(err: serde_yaml::Error) -> Self {
        ConfigError::ParseError(err)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub dns_records: Vec<DnsRecordConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DnsRecordConfig {
    pub name: String,
    pub source: Source,
    pub domain: Domain,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub freebox: Option<Freebox>,
    #[serde(with = "duration_serde")]
    pub check_interval_in_seconds: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Freebox {
    #[serde(default = "default_freebox_url")]
    pub url: Option<String>,
    pub token: String,
}

fn default_freebox_url() -> Option<String> {
    Some("http://mafreebox.freebox.fr".to_string())
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Domain {
    pub provider: ConfigDnsProvider,
    pub domain_name: String,
    pub record_name: String,
    pub record_type: RecordType,
    pub record_ttl: u32,
    pub api_key: Option<String>,
    pub email: Option<String>,
    pub zone_id: Option<String>,
}

// Duration serialization/deserialization helpers
mod duration_serde {
    use super::*;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let seconds = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(seconds))
    }
}

impl Config {
    #[allow(dead_code)]
    pub fn from_file(path: &str) -> Result<Self, ConfigError> {
        let contents = std::fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&contents)?;
        Ok(config)
    }
}
