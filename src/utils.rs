use crate::{config::RecordType, dns::DnsError, sources::IpVersion};

pub fn get_ip_version(record_type: &RecordType) -> Result<IpVersion, DnsError> {
    match record_type {
        RecordType::A => Ok(IpVersion::IPv4),
        RecordType::Aaaa => Ok(IpVersion::IPv6),
        _ => Err(DnsError::ValidationError(
            "Unsupported record type for IP retrieval".to_string(),
        )),
    }
}
