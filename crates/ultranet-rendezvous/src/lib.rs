//! Ultranet L3 — rendezvous by key, with signed peer authentication.
//!
//! Two entry points:
//!   - [`publish`] — start a hidden service. Returns a [`Listener`]
//!     whose [`Listener::accept`] performs the M3 signed handshake
//!     and returns both the stream and the verified peer identity.
//!   - [`dial`] — open a stream to a peer and perform the M3 signed
//!     handshake, proving the dialer's identity to the listener and
//!     learning the listener's identity in return.
//!
//! Wire protocol (M3), big-endian, fixed-size messages:
//!
//!   L -> D  (37 B):   "ULT1" | 0x00 | 32-byte random challenge
//!   D -> L (101 B):   "ULT1" | 0x00 | 32-byte dialer pubkey |
//!                     64-byte sig over (DOMAIN || challenge)
//!   L -> D  (38 B):   "ULT1" | 0x00 | 0x01 (accept) |
//!                     32-byte listener pubkey
//!                     — or 0x00 (reject) | 32 zero bytes
//!
//! `DOMAIN` is the 12-byte string `"ultranet-m3\0"`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::sync::Arc;

use anyhow::{Context, Result, bail};
use arti_client::{DataStream, StreamPrefs, TorClient};
use futures::{StreamExt, stream::BoxStream};
use rand_core::{OsRng, RngCore};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tor_cell::relaycell::msg::Connected;
use tor_hsservice::{HsNickname, RendRequest, RunningOnionService, config::OnionServiceConfigBuilder};
use tor_rtcompat::PreferredRuntime;
use ultranet_core::{IdentityPeerId, Layer, PeerId};
use ultranet_crypto::{Signature, SigningKey, VerifyingKey};

/// Which layer this crate implements.
pub const LAYER: Layer = Layer::Rendezvous;

/// Virtual port used by M2/M3. See design note.
pub const ULTRANET_PORT: u16 = 1;

const MAGIC: [u8; 4] = *b"ULT1";
const PROTO_VERSION: u8 = 0x00;
const DOMAIN: &[u8; 12] = b"ultranet-m3\0";
const RESULT_ACCEPT: u8 = 0x01;
const RESULT_REJECT: u8 = 0x00;

/// A running hidden service that accepts incoming connections.
pub struct Listener {
    /// The service peer id (`.onion`) other peers dial.
    pub service_peer_id: PeerId,
    /// The identity peer id, proven to dialers in the handshake.
    pub identity_peer_id: IdentityPeerId,
    signing_key: SigningKey,
    _service: Arc<RunningOnionService>,
    requests: BoxStream<'static, RendRequest>,
}

/// An incoming connection whose dialer has been cryptographically
/// authenticated.
pub struct IncomingConnection {
    /// The live bidirectional stream.
    pub stream: DataStream,
    /// The verified identity of the remote peer.
    pub peer: IdentityPeerId,
}

/// Outcome of a successful dial.
pub struct OutgoingConnection {
    /// The live bidirectional stream.
    pub stream: DataStream,
    /// The listener's identity as proven in the handshake response.
    pub peer: IdentityPeerId,
}

/// Publish ourselves as a hidden service and return a [`Listener`].
///
/// The caller provides the [`SigningKey`] that acts as the node's
/// long-term identity for the handshake. The public counterpart is
/// returned as [`Listener::identity_peer_id`].
///
/// # Errors
/// Returns an error if the service cannot be launched or the onion
/// address is not yet available.
pub async fn publish(
    tor_client: &TorClient<PreferredRuntime>,
    signing_key: SigningKey,
) -> Result<Listener> {
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

    let identity_peer_id = IdentityPeerId::from_bytes(signing_key.verifying_key().to_bytes());

    Ok(Listener {
        service_peer_id: PeerId::from_onion(onion_address.to_string()),
        identity_peer_id,
        signing_key,
        _service: service,
        requests: Box::pin(requests),
    })
}

impl Listener {
    /// Wait for the next incoming stream and perform the M3 handshake.
    ///
    /// Returns once a dialer has successfully proven their identity.
    /// Malformed or unverifiable handshake attempts close the stream
    /// and the loop continues.
    ///
    /// # Errors
    /// Returns an error if the underlying request stream ends.
    pub async fn accept(&mut self) -> Result<IncomingConnection> {
        loop {
            let rend = self
                .requests
                .next()
                .await
                .context("hidden service request stream ended unexpectedly")?;
            let mut stream_requests =
                rend.accept().await.context("accepting rendezvous request")?;
            let Some(stream_req) = stream_requests.next().await else {
                continue;
            };
            let mut stream = stream_req
                .accept(Connected::new_empty())
                .await
                .context("accepting stream request")?;

            match self.perform_handshake(&mut stream).await {
                Ok(peer) => return Ok(IncomingConnection { stream, peer }),
                Err(err) => {
                    eprintln!("handshake rejected: {err}");
                    let _ = stream.shutdown().await;
                    // fall through and wait for the next request
                }
            }
        }
    }

    async fn perform_handshake(&self, stream: &mut DataStream) -> Result<IdentityPeerId> {
        // 1. L -> D: magic | version | 32-byte challenge
        let mut challenge = [0u8; 32];
        OsRng.fill_bytes(&mut challenge);

        let mut msg1 = [0u8; 4 + 1 + 32];
        msg1[..4].copy_from_slice(&MAGIC);
        msg1[4] = PROTO_VERSION;
        msg1[5..].copy_from_slice(&challenge);
        stream.write_all(&msg1).await.context("writing challenge")?;
        stream.flush().await.ok();

        // 2. D -> L: magic | version | 32-byte pubkey | 64-byte signature
        let mut msg2 = [0u8; 4 + 1 + 32 + 64];
        stream.read_exact(&mut msg2).await.context("reading dialer auth")?;
        if msg2[..4] != MAGIC {
            bail!("bad magic in dialer message");
        }
        if msg2[4] != PROTO_VERSION {
            bail!("unsupported protocol version: {:#x}", msg2[4]);
        }
        let mut pubkey_bytes = [0u8; 32];
        pubkey_bytes.copy_from_slice(&msg2[5..37]);
        let mut sig_bytes = [0u8; 64];
        sig_bytes.copy_from_slice(&msg2[37..101]);

        let verifying_key = VerifyingKey::from_bytes(&pubkey_bytes)
            .map_err(|e| anyhow::anyhow!("invalid dialer pubkey: {e}"))?;
        let sig = Signature::from_bytes(&sig_bytes);

        let mut signed = [0u8; DOMAIN.len() + 32];
        signed[..DOMAIN.len()].copy_from_slice(DOMAIN);
        signed[DOMAIN.len()..].copy_from_slice(&challenge);
        verifying_key
            .verify(&signed, &sig)
            .map_err(|e| anyhow::anyhow!("signature verification failed: {e}"))?;

        // 3. L -> D: magic | version | 0x01 | 32-byte listener pubkey
        let mut msg3 = [0u8; 4 + 1 + 1 + 32];
        msg3[..4].copy_from_slice(&MAGIC);
        msg3[4] = PROTO_VERSION;
        msg3[5] = RESULT_ACCEPT;
        msg3[6..].copy_from_slice(&self.signing_key.verifying_key().to_bytes());
        stream.write_all(&msg3).await.context("writing accept")?;
        stream.flush().await.ok();

        Ok(IdentityPeerId::from_bytes(pubkey_bytes))
    }
}

/// Open a stream to a peer and perform the M3 handshake.
///
/// # Errors
/// Returns an error if the peer cannot be reached, the stream
/// cannot be opened, or the handshake fails.
pub async fn dial(
    tor_client: &TorClient<PreferredRuntime>,
    service_peer: &PeerId,
    signing_key: &SigningKey,
) -> Result<OutgoingConnection> {
    let addr = format!("{}:{}", service_peer.as_onion(), ULTRANET_PORT);
    let prefs = StreamPrefs::default();
    let mut stream = tor_client
        .connect_with_prefs(addr.as_str(), &prefs)
        .await
        .context("dialing peer")?;

    // 1. L -> D: magic | version | challenge
    let mut msg1 = [0u8; 4 + 1 + 32];
    stream.read_exact(&mut msg1).await.context("reading challenge")?;
    if msg1[..4] != MAGIC {
        bail!("bad magic in listener challenge");
    }
    if msg1[4] != PROTO_VERSION {
        bail!("unsupported protocol version: {:#x}", msg1[4]);
    }
    let mut challenge = [0u8; 32];
    challenge.copy_from_slice(&msg1[5..]);

    // 2. D -> L: magic | version | pubkey | signature over (DOMAIN || challenge)
    let mut signed = [0u8; DOMAIN.len() + 32];
    signed[..DOMAIN.len()].copy_from_slice(DOMAIN);
    signed[DOMAIN.len()..].copy_from_slice(&challenge);
    let sig = signing_key.sign(&signed);

    let pubkey_bytes = signing_key.verifying_key().to_bytes();
    let mut msg2 = [0u8; 4 + 1 + 32 + 64];
    msg2[..4].copy_from_slice(&MAGIC);
    msg2[4] = PROTO_VERSION;
    msg2[5..37].copy_from_slice(&pubkey_bytes);
    msg2[37..].copy_from_slice(&sig.to_bytes());
    stream.write_all(&msg2).await.context("writing dialer auth")?;
    stream.flush().await.ok();

    // 3. L -> D: magic | version | result | listener pubkey
    let mut msg3 = [0u8; 4 + 1 + 1 + 32];
    stream.read_exact(&mut msg3).await.context("reading listener result")?;
    if msg3[..4] != MAGIC {
        bail!("bad magic in listener response");
    }
    if msg3[4] != PROTO_VERSION {
        bail!("unsupported protocol version: {:#x}", msg3[4]);
    }
    match msg3[5] {
        RESULT_ACCEPT => {}
        RESULT_REJECT => bail!("listener rejected our handshake"),
        other => bail!("unknown handshake result byte: {other:#x}"),
    }
    let mut listener_pubkey = [0u8; 32];
    listener_pubkey.copy_from_slice(&msg3[6..]);

    Ok(OutgoingConnection {
        stream,
        peer: IdentityPeerId::from_bytes(listener_pubkey),
    })
}
