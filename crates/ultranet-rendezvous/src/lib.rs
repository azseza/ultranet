//! Ultranet L3 — rendezvous by key.
//!
//! Two entry points:
//!   - [`publish`] — start a hidden service. Returns a [`Listener`]
//!     that exposes the assigned [`PeerId`] and an `accept` loop.
//!   - [`dial`] — open a stream to a peer identified by [`PeerId`].
//!
//! M2 deviates slightly from `docs/m2-rendezvous.md` in one detail:
//! the handshake exchanges only the strings `HELLO\n` and
//! `HELLO-ACK\n`, with no trailing 32-byte payload. The payload was a
//! placeholder for "dialer's PeerId bytes", but in M2 the dialer does
//! not run a hidden service and therefore has no PeerId of its own.
//! Real signed peer authentication lands in M3.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::sync::Arc;

use anyhow::{Context, Result};
use arti_client::{DataStream, StreamPrefs, TorClient};
use futures::{StreamExt, stream::BoxStream};
use tor_cell::relaycell::msg::Connected;
use tor_hsservice::{HsNickname, RendRequest, RunningOnionService, config::OnionServiceConfigBuilder};
use tor_rtcompat::PreferredRuntime;
use ultranet_core::{Layer, PeerId};

/// Which layer this crate implements.
pub const LAYER: Layer = Layer::Rendezvous;

/// Virtual port used by M2. Tor hidden services are addressed by a
/// virtual port number that is carried inside the circuit, not a
/// real TCP port; the number itself is a convention shared between
/// listener and dialer.
pub const ULTRANET_PORT: u16 = 1;

/// A running hidden service that accepts incoming streams from peers
/// who know its [`PeerId`].
pub struct Listener {
    /// The peer id — the `.onion` string — that other peers dial.
    pub peer_id: PeerId,
    /// Kept alive for the lifetime of the listener. Dropping it tears
    /// down the hidden service.
    _service: Arc<RunningOnionService>,
    requests: BoxStream<'static, RendRequest>,
}

impl Listener {
    /// Wait for, and accept, the next incoming stream.
    ///
    /// Blocks until a peer completes the rendezvous handshake and
    /// opens a stream, then returns that stream.
    ///
    /// # Errors
    /// Returns an error if the hidden service's request stream ends
    /// or if accepting the stream fails.
    pub async fn accept(&mut self) -> Result<DataStream> {
        loop {
            let rend = self
                .requests
                .next()
                .await
                .context("hidden service request stream ended unexpectedly")?;
            let mut stream_requests =
                rend.accept().await.context("accepting rendezvous request")?;
            if let Some(stream_req) = stream_requests.next().await {
                let stream = stream_req
                    .accept(Connected::new_empty())
                    .await
                    .context("accepting stream request")?;
                return Ok(stream);
            }
        }
    }
}

/// Publish ourselves as a hidden service and return a [`Listener`].
///
/// The peer id is the `.onion` address assigned to the newly-generated
/// ed25519 service key. It is ephemeral: a fresh key is minted by
/// arti on each run.
///
/// # Errors
/// Returns an error if the service cannot be launched, or if the
/// onion name is not yet available when we ask for it.
pub async fn publish(tor_client: &TorClient<PreferredRuntime>) -> Result<Listener> {
    let nickname: HsNickname = "ultranet-m2".parse().context("parsing HS nickname")?;
    let config = OnionServiceConfigBuilder::default()
        .nickname(nickname)
        .build()
        .context("building onion service config")?;
    let (service, requests) = tor_client
        .launch_onion_service(config)
        .context("launching onion service")?;
    let onion_address = service
        .onion_address()
        .context("onion address not available immediately after launch")?;
    Ok(Listener {
        peer_id: PeerId::from_onion(onion_address.to_string()),
        _service: service,
        requests: Box::pin(requests),
    })
}

/// Open a stream to a peer by id.
///
/// # Errors
/// Returns an error if the peer cannot be reached or the stream
/// cannot be opened.
pub async fn dial(
    tor_client: &TorClient<PreferredRuntime>,
    peer_id: &PeerId,
) -> Result<DataStream> {
    let addr = format!("{}:{}", peer_id.as_onion(), ULTRANET_PORT);
    let prefs = StreamPrefs::default();
    tor_client
        .connect_with_prefs(addr.as_str(), &prefs)
        .await
        .context("dialing peer")
}
