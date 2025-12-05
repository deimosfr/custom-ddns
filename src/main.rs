use clap::Parser;
use custom_ddns::config::{Config, ConfigDnsProvider, DnsRecordConfig};
use custom_ddns::dns::cloudflare::CloudflareDns;
use custom_ddns::dns::{DnsClient, DnsProvider, DnsRecordCloudflare};
use custom_ddns::router::start_health_server;
use custom_ddns::sources::IpSource;
use custom_ddns::sources::freebox::FreeboxSource;
use custom_ddns::utils::get_ip_version;
use tracing::{debug, error, info};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the configuration file
    #[arg(short, long, default_value = "config.yaml")]
    config: String,
    /// Port for the health check server
    #[arg(long, default_value = "8080")]
    health_port: u16,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    info!("Starting Custom DDNS");

    // Start health check server
    let args = Args::parse();
    let mut handles = Vec::new();
    let health_handle = tokio::spawn(start_health_server(args.health_port));
    handles.push(health_handle);

    debug!("Loading config from {}", args.config);

    match Config::from_file(&args.config) {
        Ok(config) => {
            info!("Configuration loaded successfully");

            for record in config.dns_records {
                let handle = tokio::spawn(process_record(record));
                handles.push(handle);
            }

            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?
                .recv()
                .await;

            info!("Received SIGTERM, shutting down");

            // Cancel all running tasks
            for handle in handles {
                handle.abort();
            }

            info!("Shutdown complete");
            Ok(())
        }
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            Err(anyhow::anyhow!("Error loading configuration: {}", e))
        }
    }
}

async fn process_record(record: DnsRecordConfig) -> Result<(), anyhow::Error> {
    let mut interval = tokio::time::interval(record.source.check_interval_in_seconds);

    info!("Starting DNS record check for `{}`", &record.name);
    let mut previous_ip_address = None;

    loop {
        interval.tick().await;

        debug!("Checking record: {}", &record.name);

        if let Some(source) = &record.source.freebox {
            let mut freebox_source = FreeboxSource::new(source.url.clone(), source.token.clone())?;

            // Determine IP version from record type (IPv4 or IPv6)
            let ip_version = match get_ip_version(&record.domain.record_type) {
                Ok(ip_version) => ip_version,
                Err(e) => {
                    error!(
                        "Failed to determine IP kind for record {}: {}",
                        record.name, e
                    );
                    continue;
                }
            };

            // perform DNS check and get client
            let dns_client = match record.domain.provider {
                ConfigDnsProvider::Cloudflare => {
                    let api_key = match record.domain.api_key.as_ref() {
                        Some(key) => key,
                        None => {
                            error!(
                                "API key is required for Cloudflare provider for record: {}",
                                record.name
                            );
                            std::process::exit(1);
                        }
                    };

                    let client = match CloudflareDns::new(api_key.clone()) {
                        Ok(dns) => dns,
                        Err(e) => {
                            error!("{}", e);
                            std::process::exit(1);
                        }
                    };
                    DnsClient::Cloudflare(client)
                }
            };

            // Get the current IP address
            let current_ip = match freebox_source.get_ip(ip_version).await {
                Ok(current_ip_address) => {
                    debug!("Detected Freebox IP: {}", current_ip_address.address);
                    current_ip_address
                }
                Err(e) => {
                    error!("Failed to get Freebox IP: {}", e);
                    continue;
                }
            };

            // compare the current ip address with the previous ip address
            let update_record = match previous_ip_address {
                None => {
                    // first run, set the previous ip address
                    previous_ip_address = Some(current_ip.clone());

                    // Check if the record already exists with DNS check
                    async {
                        match dns_client {
                            DnsClient::Cloudflare(ref cloudflare_dns) => {
                                // Construct the full qualified domain name with trailing dot
                                let full_record_name = format!(
                                    "{}.{}.",
                                    record.domain.record_name, record.domain.domain_name
                                );

                                match cloudflare_dns
                                    .get_record_content(
                                        &record.domain.domain_name,
                                        &full_record_name,
                                        &record.domain.record_type,
                                    )
                                    .await
                                {
                                    Ok(Some(existing_content)) => {
                                        if existing_content == current_ip.address {
                                            debug!(
                                                "DNS record for {} already matches current IP: {}",
                                                record.name, current_ip.address
                                            );
                                            false
                                        } else {
                                            info!(
                                                "DNS record for {} has different IP: {} -> {}",
                                                record.name, existing_content, current_ip.address
                                            );
                                            true
                                        }
                                    }
                                    Ok(None) => {
                                        info!(
                                            "DNS record for {} does not exist, will create it",
                                            record.name
                                        );
                                        true
                                    }
                                    Err(e) => {
                                        error!(
                                            "Failed to check DNS record for {}: {}",
                                            record.name, e
                                        );
                                        false
                                    }
                                }
                            }
                        }
                    }
                    .await
                }
                Some(ref last_ip_address) => match last_ip_address.address == current_ip.address {
                    true => false,
                    false => {
                        info!(
                            "Freebox IP address has changed: {} -> {}",
                            last_ip_address.address, &current_ip.address
                        );
                        true
                    }
                },
            };

            if update_record {
                match dns_client {
                    DnsClient::Cloudflare(cloudflare_dns) => {
                        // Construct the full qualified domain name with trailing dot
                        let full_record_name = format!(
                            "{}.{}.",
                            record.domain.record_name, record.domain.domain_name
                        );

                        match cloudflare_dns
                            .update_record(
                                &record.domain.domain_name,
                                &DnsRecordCloudflare {
                                    id: None,
                                    name: full_record_name,
                                    content: current_ip.address.clone(),
                                    record_type: record.domain.record_type.clone(),
                                    ttl: record.domain.record_ttl,
                                },
                            )
                            .await
                        {
                            Ok(_) => {
                                info!(
                                    "Successfully updated DNS record for {}: {}",
                                    record.name, current_ip.address
                                );
                            }
                            Err(e) => {
                                error!("Failed to update DNS record for {}: {}", record.name, e);
                            }
                        }
                    }
                }
            };

            debug!("Record check completed for {}", record.name);
        }
    }
}
