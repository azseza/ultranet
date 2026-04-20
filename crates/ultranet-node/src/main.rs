//! Ultranet node binary.
//!
//! Subcommands:
//!   (none)            — print banner, exit. No I/O.
//!   `--bootstrap`     — M1: bootstrap arti and exit.
//!   `serve`           — M2: publish a hidden service, wait for one
//!                        incoming connection, handshake, exit.
//!   `dial <peer-id>`  — M2: dial a peer id, handshake, exit.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use anyhow::{Context, Result, bail};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use ultranet_core::{Invariant, Layer, PeerId, VERSION};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    print_banner();

    match args.first().map(String::as_str) {
        None => {
            println!("No network I/O. Subcommands: --bootstrap | serve | dial <peer-id>.");
            Ok(())
        }
        Some("--bootstrap") => run_async(run_bootstrap()),
        Some("serve") => run_async(run_serve()),
        Some("dial") => {
            let Some(onion) = args.get(1) else {
                bail!("usage: ultranet dial <peer-id>");
            };
            let peer_id = PeerId::from_onion(onion.clone());
            run_async(run_dial(peer_id))
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
    println!("M2 serve: bootstrapping L2...");
    let b = ultranet_transport::bootstrap().await?;
    println!("  bootstrap: READY in {:?}", b.report.duration);

    println!("publishing L3 hidden service...");
    let mut listener = ultranet_rendezvous::publish(&b.client).await?;
    println!("  my peer id: {}", listener.peer_id);
    println!("  waiting for one incoming connection...");

    let mut stream = listener.accept().await.context("accepting peer connection")?;
    println!("  incoming connection accepted");

    let mut buf = [0u8; 6];
    stream.read_exact(&mut buf).await.context("reading HELLO")?;
    if &buf != b"HELLO\n" {
        bail!("unexpected handshake from dialer: {:?}", &buf[..]);
    }
    println!("  received HELLO");

    stream.write_all(b"HELLO-ACK\n").await.context("writing HELLO-ACK")?;
    stream.flush().await.ok();
    println!("  sent HELLO-ACK");

    // Cleanly close the write side so the dialer sees EOF after the
    // ACK bytes, rather than an abrupt circuit teardown when the
    // hidden service drops on exit.
    stream.shutdown().await.ok();

    // Give the dialer's read a moment to complete before the runtime
    // shuts down and kills the underlying circuits. Pragmatic, not
    // principled — M2 only needs to demo the handshake.
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    println!("M2 serve complete. Exiting.");
    Ok(())
}

async fn run_dial(peer_id: PeerId) -> Result<()> {
    println!("M2 dial: bootstrapping L2...");
    let b = ultranet_transport::bootstrap().await?;
    println!("  bootstrap: READY in {:?}", b.report.duration);

    println!("dialing peer {}...", peer_id);
    let mut stream = ultranet_rendezvous::dial(&b.client, &peer_id).await?;
    println!("  CONNECTED");

    stream.write_all(b"HELLO\n").await.context("writing HELLO")?;
    stream.flush().await.ok();
    println!("  sent HELLO");

    let mut buf = [0u8; 10];
    stream.read_exact(&mut buf).await.context("reading HELLO-ACK")?;
    if &buf != b"HELLO-ACK\n" {
        bail!("unexpected handshake from listener: {:?}", &buf[..]);
    }
    println!("  received HELLO-ACK from {}", peer_id);

    println!("M2 dial complete. Exiting.");
    Ok(())
}
