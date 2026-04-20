//! Ultranet L2 — anonymous transport.
//!
//! Wraps `arti` (Rust Tor) to provide onion-routed circuits. This crate
//! is the only place in Ultranet that speaks to the Tor directory
//! system; every layer above L2 consumes circuits through the API here.
//!
//! State model (M2.5a): every call to [`bootstrap`] creates a fresh
//! `TempDir` for arti's state and cache. The directory is deleted on
//! drop of the returned [`Bootstrapped`]. Peer identity is therefore
//! ephemeral across process restarts; see `docs/m2.5-state.md` for the
//! design of persistent identity (implemented when a workflow needs
//! it).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::time::Duration;

use anyhow::{Context, Result};
use arti_client::{TorClient, TorClientConfig};
use tor_config_path::CfgPath;
use tor_rtcompat::PreferredRuntime;

/// Which layer this crate implements.
pub const LAYER: ultranet_core::Layer = ultranet_core::Layer::Transport;

/// Result of a bootstrap attempt, summarised for the caller.
#[derive(Debug, Clone)]
pub struct BootstrapReport {
    /// Wall-clock time the bootstrap took.
    pub duration: Duration,
    /// Whether the client considers itself fully ready.
    pub ready: bool,
}

/// A bootstrapped Tor client together with the ephemeral state
/// directory backing it.
///
/// The [`tempfile::TempDir`] is held internally so that it is wiped
/// from disk when the `Bootstrapped` is dropped. Do not leak the
/// client outside of this wrapper if you rely on the ephemeral-state
/// guarantee.
pub struct Bootstrapped {
    /// The live Tor client. Layers above L2 consume this.
    pub client: TorClient<PreferredRuntime>,
    /// Timing and readiness report for the bootstrap itself.
    pub report: BootstrapReport,
    _state_dir: tempfile::TempDir,
}

/// Bootstrap a Tor client with a fresh, ephemeral on-disk state
/// directory.
///
/// # Errors
/// Returns an error if the runtime cannot be obtained, the state
/// directory cannot be created, the config cannot be built, or
/// bootstrap fails to complete.
pub async fn bootstrap() -> Result<Bootstrapped> {
    let started = std::time::Instant::now();

    let runtime =
        PreferredRuntime::current().context("obtaining the preferred async runtime for arti")?;

    let state_dir = tempfile::Builder::new()
        .prefix("ultranet-")
        .tempdir()
        .context("creating ephemeral state dir")?;

    let state_path = state_dir.path().join("state");
    let cache_path = state_dir.path().join("cache");
    create_private_dir(&state_path).context("creating state subdir")?;
    create_private_dir(&cache_path).context("creating cache subdir")?;

    let mut builder = TorClientConfig::builder();
    builder
        .storage()
        .state_dir(CfgPath::new_literal(state_path))
        .cache_dir(CfgPath::new_literal(cache_path));
    let config = builder.build().context("building Tor client config")?;

    let client = TorClient::with_runtime(runtime)
        .config(config)
        .create_bootstrapped()
        .await
        .context("bootstrapping Tor client")?;

    let ready = client.bootstrap_status().ready_for_traffic();

    Ok(Bootstrapped {
        client,
        report: BootstrapReport { duration: started.elapsed(), ready },
        _state_dir: state_dir,
    })
}

/// Create a directory with 0700 permissions (owner-only). Arti
/// refuses to use a state directory that is group- or world-readable.
#[cfg(unix)]
fn create_private_dir(path: &std::path::Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(path)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
}

#[cfg(not(unix))]
fn create_private_dir(path: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(path)
}
