use super::{
    DnsError, DnsProvider, DnsRecordCloudflare, validate_record_data, validate_record_name,
    validate_ttl,
};
use crate::config::RecordType;
use async_trait::async_trait;
use cloudflare::endpoints::dns::dns::{
    CreateDnsRecord, ListDnsRecords, ListDnsRecordsParams, UpdateDnsRecord,
};
use cloudflare::endpoints::dns::dns::{CreateDnsRecordParams, DnsContent, UpdateDnsRecordParams};
use cloudflare::endpoints::zones::zone::ListZones;
use cloudflare::framework::{auth::Credentials, client::async_api::Client};
use tracing;

pub struct CloudflareDns {
    client: Client,
}

impl CloudflareDns {
    pub fn new(api_token: String) -> Result<Self, DnsError> {
        let credentials = Credentials::UserAuthToken { token: api_token };

        let config = cloudflare::framework::client::ClientConfig::default();
        let client = Client::new(
            credentials,
            config,
            cloudflare::framework::Environment::Production,
        )
        .map_err(|e| DnsError::ApiError(format!("Failed to create Cloudflare client: {}", e)))?;

        Ok(Self { client })
    }

    // Ensures a domain name ends with a dot for proper DNS formatting
    fn ensure_trailing_dot(domain: &str) -> String {
        if domain.ends_with('.') {
            domain.to_string()
        } else {
            format!("{}.", domain)
        }
    }

    fn record_type_to_string(record_type: &RecordType) -> String {
        match record_type {
            RecordType::A => "A",
            RecordType::Aaaa => "AAAA",
            RecordType::Cname => "CNAME",
            RecordType::Mx => "MX",
            RecordType::Txt => "TXT",
            RecordType::Srv => "SRV",
        }
        .to_string()
    }

    fn get_record_type_from_content(content: &DnsContent) -> String {
        match content {
            DnsContent::A { .. } => "A".to_string(),
            DnsContent::AAAA { .. } => "AAAA".to_string(),
            DnsContent::CNAME { .. } => "CNAME".to_string(),
            DnsContent::MX { .. } => "MX".to_string(),
            DnsContent::TXT { .. } => "TXT".to_string(),
            DnsContent::SRV { .. } => "SRV".to_string(),
            _ => "UNKNOWN".to_string(),
        }
    }

    async fn get_zone_id(&self, zone_name: &str) -> Result<String, DnsError> {
        let zones = self
            .client
            .request(&ListZones {
                params: cloudflare::endpoints::zones::zone::ListZonesParams::default(),
            })
            .await
            .map_err(|e| DnsError::ApiError(format!("Failed to list zones: {}", e)))?;

        for zone in zones.result {
            if zone.name == zone_name {
                return Ok(zone.id);
            }
        }

        Err(DnsError::ZoneNotFound(format!(
            "Zone '{}' not found",
            zone_name
        )))
    }

    /// Check if a specific record exists with the correct content by filtering the API query
    async fn find_matching_record(
        &self,
        zone_id: &str,
        record_name: &str,
        record_type: &RecordType,
    ) -> Result<Option<String>, DnsError> {
        tracing::debug!(
            "Searching for existing record: {} ({})",
            record_name,
            Self::record_type_to_string(record_type)
        );

        // Create parameters that filter by record name and type to avoid TLSA records
        let params = ListDnsRecordsParams {
            per_page: Some(20),
            ..Default::default()
        };

        // Try to make a targeted request
        match self
            .client
            .request(&ListDnsRecords {
                zone_identifier: zone_id,
                params,
            })
            .await
        {
            Ok(records) => {
                let normalized_name = Self::ensure_trailing_dot(record_name);
                let target_type = Self::record_type_to_string(record_type);

                for cf_record in records.result {
                    if Self::ensure_trailing_dot(&cf_record.name) == normalized_name {
                        let cf_record_type = Self::get_record_type_from_content(&cf_record.content);
                        if cf_record_type == target_type {
                            tracing::debug!("Found matching record with ID: {}", cf_record.id);
                            return Ok(Some(cf_record.id));
                        }
                    }
                }
                tracing::debug!("No matching record found");
                Ok(None)
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to search for existing record due to API error: {:?}",
                    e
                );
                // If we can't list records due to TLSA issues, return None (assume record doesn't exist)
                Ok(None)
            }
        }
    }

    /// Update record with proper existence checking
    async fn update_record_smart(
        &self,
        zone: &str,
        record: &DnsRecordCloudflare,
    ) -> Result<(), DnsError> {
        self.validate_record(record)?;

        let zone_id = self.get_zone_id(zone).await?;

        // Check if record already exists with same name and type
        match self
            .find_matching_record(&zone_id, &record.name, &record.record_type)
            .await?
        {
            Some(record_id) => {
                // Record exists, update it
                tracing::info!("Updating existing DNS record: {}", record.name);

                let content = self.create_dns_content(record)?;

                let params = UpdateDnsRecordParams {
                    name: &Self::ensure_trailing_dot(&record.name),
                    content,
                    ttl: Some(record.ttl),
                    proxied: None,
                };

                self.client
                    .request(&UpdateDnsRecord {
                        zone_identifier: &zone_id,
                        identifier: &record_id,
                        params,
                    })
                    .await
                    .map_err(|e| {
                        DnsError::ApiError(format!("Failed to update DNS record: {}", e))
                    })?;
            }
            None => {
                // Record doesn't exist, create it
                tracing::info!("Creating new DNS record: {}", record.name);

                let content = self.create_dns_content(record)?;

                let params = CreateDnsRecordParams {
                    name: &Self::ensure_trailing_dot(&record.name),
                    content,
                    ttl: Some(record.ttl),
                    priority: None,
                    proxied: None,
                };

                self.client
                    .request(&CreateDnsRecord {
                        zone_identifier: &zone_id,
                        params,
                    })
                    .await
                    .map_err(|e| {
                        DnsError::ApiError(format!("Failed to create DNS record: {}", e))
                    })?;
            }
        }

        Ok(())
    }

    /// Helper to create DNS content from our record
    fn create_dns_content(&self, record: &DnsRecordCloudflare) -> Result<DnsContent, DnsError> {
        match &record.record_type {
            RecordType::A => {
                let ip: std::net::Ipv4Addr = record
                    .content
                    .parse()
                    .map_err(|_| DnsError::ValidationError("Invalid IPv4 address".to_string()))?;
                Ok(DnsContent::A { content: ip })
            }
            RecordType::Aaaa => {
                let ip: std::net::Ipv6Addr = record
                    .content
                    .parse()
                    .map_err(|_| DnsError::ValidationError("Invalid IPv6 address".to_string()))?;
                Ok(DnsContent::AAAA { content: ip })
            }
            RecordType::Cname => Ok(DnsContent::CNAME {
                content: Self::ensure_trailing_dot(&record.content),
            }),
            RecordType::Mx => {
                let parts: Vec<&str> = record.content.split_whitespace().collect();
                if parts.len() != 2 {
                    return Err(DnsError::ValidationError(
                        "MX record must have format: priority target".to_string(),
                    ));
                }
                let priority = parts[0]
                    .parse::<u16>()
                    .map_err(|_| DnsError::ValidationError("Invalid MX priority".to_string()))?;
                Ok(DnsContent::MX {
                    content: Self::ensure_trailing_dot(parts[1]),
                    priority,
                })
            }
            RecordType::Txt => Ok(DnsContent::TXT {
                content: record.content.clone(),
            }),
            RecordType::Srv => Ok(DnsContent::SRV {
                content: Self::ensure_trailing_dot(&record.content),
            }),
        }
    }

    /// Get the current content of a DNS record
    async fn get_record_content_impl(
        &self,
        zone_id: &str,
        record_name: &str,
        record_type: &RecordType,
    ) -> Result<Option<String>, DnsError> {
        tracing::debug!(
            "Getting content for record: {} ({})",
            record_name,
            Self::record_type_to_string(record_type)
        );

        let params = ListDnsRecordsParams {
            per_page: Some(20),
            ..Default::default()
        };

        match self
            .client
            .request(&ListDnsRecords {
                zone_identifier: zone_id,
                params,
            })
            .await
        {
            Ok(records) => {
                let normalized_name = Self::ensure_trailing_dot(record_name);
                let target_type = Self::record_type_to_string(record_type);

                for cf_record in records.result {
                    if Self::ensure_trailing_dot(&cf_record.name) == normalized_name {
                        let cf_record_type = Self::get_record_type_from_content(&cf_record.content);
                        if cf_record_type == target_type {
                            let content = match &cf_record.content {
                                DnsContent::A { content } => content.to_string(),
                                DnsContent::AAAA { content } => content.to_string(),
                                DnsContent::CNAME { content } => content.clone(),
                                DnsContent::MX { content, priority } => {
                                    format!("{} {}", priority, content)
                                }
                                DnsContent::TXT { content } => content.clone(),
                                DnsContent::SRV { content } => content.clone(),
                                _ => {
                                    return Err(DnsError::ApiError(
                                        "Unsupported record type".to_string(),
                                    ));
                                }
                            };
                            tracing::debug!("Found record content: {}", content);
                            return Ok(Some(content));
                        }
                    }
                }
                tracing::debug!("No matching record found");
                Ok(None)
            }
            Err(e) => {
                tracing::warn!("Failed to get record content due to API error: {:?}", e);
                Ok(None)
            }
        }
    }
}

#[async_trait]
impl DnsProvider for CloudflareDns {
    async fn update_record(
        &self,
        zone: &str,
        record: &DnsRecordCloudflare,
    ) -> Result<(), DnsError> {
        // Use the smart update approach that handles TLSA parsing issues
        self.update_record_smart(zone, record).await
    }

    async fn get_record_content(
        &self,
        zone: &str,
        record_name: &str,
        record_type: &RecordType,
    ) -> Result<Option<String>, DnsError> {
        let zone_id = self.get_zone_id(zone).await?;
        self.get_record_content_impl(&zone_id, record_name, record_type)
            .await
    }

    fn validate_record(&self, record: &DnsRecordCloudflare) -> Result<(), DnsError> {
        validate_record_name(&record.name)?;
        validate_record_data(&record.content, &record.record_type)?;
        validate_ttl(record.ttl)?;
        Ok(())
    }
}
