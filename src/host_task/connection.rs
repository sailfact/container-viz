//! Turning a `HostConfig` into a live, pinged `Docker` client.

use anyhow::{Context, Result};
use bollard::Docker;

use crate::model::{ConnectionType, HostConfig, TlsConfig};

const CONNECT_TIMEOUT_S: u64 = 30;

pub(super) async fn connect(config: &HostConfig) -> Result<Docker> {
    let docker = match &config.connection {
        ConnectionType::UnixSocket(path) => {
            Docker::connect_with_unix(
                &path.to_string_lossy(),
                CONNECT_TIMEOUT_S,
                bollard::API_DEFAULT_VERSION,
            )
            .context("constructing unix socket client")?
        }
        ConnectionType::Tcp { host, port, tls } => {
            let addr = format!("tcp://{host}:{port}");
            match tls {
                Some(tls_cfg) => connect_tls(&addr, tls_cfg)?,
                None => Docker::connect_with_http(
                    &addr,
                    CONNECT_TIMEOUT_S,
                    bollard::API_DEFAULT_VERSION,
                )
                .context("constructing http client")?,
            }
        }
    };

    // Constructors above don't touch the network; ping forces a round-trip
    // so a misconfigured host fails here, not on the first poll.
    docker.ping().await.context("ping")?;
    Ok(docker)
}

/// `cert_path` is a directory holding `cert.pem`/`key.pem`; CA defaults to
/// `ca.pem` alongside them unless overridden.
fn connect_tls(addr: &str, tls: &TlsConfig) -> Result<Docker> {
    let cert = tls.cert_path.join("cert.pem");
    let key = tls.cert_path.join("key.pem");
    let ca = tls
        .ca_cert
        .clone()
        .unwrap_or_else(|| tls.cert_path.join("ca.pem"));

    // bollard always verifies the peer cert; TlsConfig.verify is not yet honoured.
    Docker::connect_with_ssl(
        addr,
        &key,
        &cert,
        &ca,
        CONNECT_TIMEOUT_S,
        bollard::API_DEFAULT_VERSION,
    )
    .context("constructing TLS client")
}