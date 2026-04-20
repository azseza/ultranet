//! Ultranet node binary.
//!
//! Subcommands:
//!   (none)                         — print banner, exit. No I/O.
//!   `--bootstrap`                  — M1: bootstrap arti and exit.
//!   `serve`                        — M3: accept one authenticated
//!                                    connection, handshake, exit.
//!   `dial <peer-id>`               — M3: dial a peer, handshake, exit.
//!   `broadcast --file F [--file F]*`
//!                                  — M4: publish as a radio station
//!                                    and stream files on loop.
//!   `tune <peer-id>`               — M4: tune in to a station.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use anyhow::{Context, Result, bail};
use tokio::io::AsyncWriteExt;
use ultranet_core::{IdentityPeerId, Invariant, Layer, PeerId, VERSION};
use ultranet_crypto::SigningKey;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    print_banner();

    match args.first().map(String::as_str) {
        None => {
            println!(
                "No network I/O. Subcommands: --bootstrap | serve | dial <peer-id> | \
                 broadcast --file F [--file F]* | tune <peer-id>.",
            );
            Ok(())
        }
        Some("--bootstrap") => run_async(run_bootstrap()),
        Some("serve") => run_async(run_serve()),
        Some("dial") => {
            let Some(onion) = args.get(1) else {
                bail!("usage: ultranet dial <peer-id>");
            };
            run_async(run_dial(PeerId::from_onion(onion.clone())))
        }
        Some("broadcast") => {
            let files = collect_file_flags(&args[1..])?;
            if files.is_empty() {
                bail!("usage: ultranet broadcast --file F [--file F ...]");
            }
            run_async(run_broadcast(files))
        }
        Some("tune") => {
            let Some(onion) = args.get(1) else {
                bail!("usage: ultranet tune <peer-id>");
            };
            run_async(run_tune(PeerId::from_onion(onion.clone())))
        }
        Some(other) => bail!("unknown subcommand: {other}"),
    }
}

fn run_async<F>(fut: F) -> Result<()>
where
    F: std::future::Future<Output = Result<()>>,
{
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    rt.block_on(fut)
}

fn collect_file_flags(args: &[String]) -> Result<Vec<String>> {
    let mut files = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--file" => {
                let Some(path) = args.get(i + 1) else {
                    bail!("--file requires an argument");
                };
                files.push(path.clone());
                i += 2;
            }
            other => bail!("unknown broadcast flag: {other}"),
        }
    }
    Ok(files)
}

fn print_banner() {
    println!("ultranet {VERSION}");
    println!();
    println!("Layers:");
    for layer in [
        Layer::Substrate,
        Layer::Transport,
        Layer::Rendezvous,
        Layer::Service,
        Layer::Application,
    ] {
        println!("  {}", layer.label());
    }
    println!();
    println!("Invariants:");
    for inv in [
        Invariant::NoAmbientAuthority,
        Invariant::NoPlaintextAtRest,
        Invariant::OnionOnlyEgress,
        Invariant::NoCentralService,
        Invariant::ReproducibleBuilds,
        Invariant::FailClosed,
        Invariant::CoverTraffic,
    ] {
        println!("  {}", inv.label());
    }
    println!();
}

async fn run_bootstrap() -> Result<()> {
    println!("M1: bootstrapping L2 (arti Tor client)...");
    let b = ultranet_transport::bootstrap().await?;
    println!(
        "  bootstrap: {} in {:?}",
        if b.report.ready { "READY" } else { "INCOMPLETE" },
        b.report.duration,
    );
    println!("M1 complete. Exiting.");
    Ok(())
}

async fn run_serve() -> Result<()> {
    let signing_key = SigningKey::generate();
    let my_identity = IdentityPeerId::from_bytes(signing_key.verifying_key().to_bytes());

    println!("M3 serve: bootstrapping L2...");
    let b = ultranet_transport::bootstrap().await?;
    println!("  bootstrap: READY in {:?}", b.report.duration);

    println!("publishing L3 hidden service...");
    let mut listener = ultranet_rendezvous::publish(&b.client, signing_key).await?;
    println!("  my service peer id:  {}", listener.service_peer_id);
    println!("  my identity peer id: {}", my_identity);
    println!("  waiting for one incoming connection...");

    let mut incoming = listener.accept().await.context("accepting peer connection")?;
    println!("  incoming peer (verified): {}", incoming.peer);

    incoming.stream.shutdown().await.ok();
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    println!("M3 serve complete. Exiting.");
    Ok(())
}

async fn run_dial(peer_id: PeerId) -> Result<()> {
    let signing_key = SigningKey::generate();
    let my_identity = IdentityPeerId::from_bytes(signing_key.verifying_key().to_bytes());

    println!("M3 dial: bootstrapping L2...");
    let b = ultranet_transport::bootstrap().await?;
    println!("  bootstrap: READY in {:?}", b.report.duration);

    println!("  my identity peer id: {}", my_identity);
    println!("dialing peer {}...", peer_id);
    let outgoing = ultranet_rendezvous::dial(&b.client, &peer_id, &signing_key).await?;
    println!("  CONNECTED");
    println!("  handshake complete; listener identity: {}", outgoing.peer);

    println!("M3 dial complete. Exiting.");
    Ok(())
}

async fn run_broadcast(files: Vec<String>) -> Result<()> {
    let signing_key = SigningKey::generate();
    let my_identity = IdentityPeerId::from_bytes(signing_key.verifying_key().to_bytes());

    println!("M4 broadcast: bootstrapping L2...");
    let b = ultranet_transport::bootstrap().await?;
    println!("  bootstrap: READY in {:?}", b.report.duration);

    println!("publishing L3 hidden service...");
    let listener = ultranet_rendezvous::publish(&b.client, signing_key).await?;
    println!("  my station:  {}", listener.service_peer_id);
    println!("  my identity: {}", my_identity);
    println!("  tune in with: ultranet tune {}", listener.service_peer_id);
    println!("  playlist ({} files):", files.len());
    for f in &files {
        println!("    - {f}");
    }
    println!();

    ultranet_radio::broadcast(listener, files).await
}

async fn run_tune(peer_id: PeerId) -> Result<()> {
    let signing_key = SigningKey::generate();
    let my_identity = IdentityPeerId::from_bytes(signing_key.verifying_key().to_bytes());

    println!("M4 tune: bootstrapping L2...");
    let b = ultranet_transport::bootstrap().await?;
    println!("  bootstrap: READY in {:?}", b.report.duration);

    println!("  my identity: {}", my_identity);
    println!("tuning in to {}...", peer_id);
    let outgoing = ultranet_rendezvous::dial(&b.client, &peer_id, &signing_key).await?;
    println!("  CONNECTED, station identity: {}", outgoing.peer);

    ultranet_radio::tune(outgoing).await
}
