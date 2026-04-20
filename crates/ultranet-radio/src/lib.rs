//! Ultranet L5 — one-to-many anonymous radio broadcast.
//!
//! The broadcaster reads audio files from disk, encodes them to Opus,
//! and fans the Opus packets out to every connected listener via a
//! single `tokio::sync::broadcast` channel. The listener reads those
//! packets, decodes them, and plays the PCM through the system's
//! default audio output.
//!
//! M4 requires input files to already be **48 kHz stereo**.
//! Resampling is out of scope for this milestone; convert with
//! `ffmpeg -i in.xxx -ar 48000 -ac 2 out.ogg` if your sources aren't.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, anyhow, bail};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::{
    HeapRb,
    traits::{Consumer, Producer, Split},
};
use symphonia::core::{
    audio::SampleBuffer,
    codecs::DecoderOptions,
    formats::FormatOptions,
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{Mutex, broadcast},
};
use ultranet_rendezvous::{IncomingConnection, Listener, OutgoingConnection};

/// 48 kHz fixed. Matches Opus's native rate; avoids resampling.
pub const SAMPLE_RATE: u32 = 48_000;
/// Stereo only.
pub const CHANNELS: u16 = 2;
/// 20 ms frames, the Opus sweet spot for streaming.
pub const FRAME_MS: u32 = 20;
/// Interleaved sample count per Opus frame (2 channels × 960 samples).
pub const SAMPLES_PER_FRAME: usize = (SAMPLE_RATE as usize / 1000 * FRAME_MS as usize) * 2;
/// Target bitrate.
pub const BITRATE: i32 = 64_000;

const TYPE_METADATA: u8 = 0x01;
const TYPE_AUDIO: u8 = 0x02;
const TYPE_END: u8 = 0x03;

/// Max frame payload we will accept on the wire. Guards against a
/// malicious or confused peer asking for a huge allocation.
const MAX_FRAME_BYTES: u32 = 1 << 20; // 1 MiB

/// Broadcast-channel capacity — roughly this many Opus frames of
/// fan-out buffer per listener. 500 × 20 ms = 10 s of slack.
const CHANNEL_CAPACITY: usize = 500;

/// A frame on the wire. `kind` is the TLV type byte.
#[derive(Clone)]
struct Frame {
    kind: u8,
    payload: Vec<u8>,
}

/// Run the broadcaster: fan audio files out to every accepted
/// listener. Runs until interrupted.
///
/// # Errors
/// Returns an error if Opus encoding, file decoding, or the
/// rendezvous accept loop fails unrecoverably.
pub async fn broadcast<P: AsRef<Path>>(mut listener: Listener, files: Vec<P>) -> Result<()> {
    if files.is_empty() {
        bail!("broadcast needs at least one --file");
    }
    let files: Vec<PathBuf> = files.iter().map(|p| p.as_ref().to_path_buf()).collect();

    let (tx, _rx) = broadcast::channel::<Frame>(CHANNEL_CAPACITY);
    let current_metadata: Arc<Mutex<Option<Vec<u8>>>> = Arc::new(Mutex::new(None));

    // Encoder task: file loop → Opus frames → channel.
    let encoder_tx = tx.clone();
    let encoder_meta = current_metadata.clone();
    let encoder_task = tokio::spawn(async move {
        if let Err(e) = run_encoder(files, encoder_tx, encoder_meta).await {
            eprintln!("encoder error: {e:#}");
        }
    });

    // Acceptor loop: each listener spawns its own forwarding task.
    let listener_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    loop {
        let incoming = match listener.accept().await {
            Ok(c) => c,
            Err(e) => {
                eprintln!("accept error: {e:#}");
                continue;
            }
        };

        let rx = tx.subscribe();
        let meta = current_metadata.clone();
        let count = listener_count.clone();

        tokio::spawn(async move {
            let n = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            println!("  listener joined: {}   [{} listener{}]", incoming.peer, n, if n == 1 { "" } else { "s" });
            let peer_label = incoming.peer;
            let err = handle_listener(incoming, rx, meta).await.err();
            let n = count.fetch_sub(1, std::sync::atomic::Ordering::SeqCst) - 1;
            println!(
                "  listener left: {}     [{} listener{}]{}",
                peer_label,
                n,
                if n == 1 { "" } else { "s" },
                if let Some(e) = err { format!(" ({e:#})") } else { String::new() },
            );
        });

        if encoder_task.is_finished() {
            break;
        }
    }

    encoder_task.abort();
    Ok(())
}

async fn handle_listener(
    mut incoming: IncomingConnection,
    mut rx: broadcast::Receiver<Frame>,
    current_metadata: Arc<Mutex<Option<Vec<u8>>>>,
) -> Result<()> {
    // Send the current metadata up front so the listener knows what's
    // playing right now, not just whatever the next track is.
    if let Some(meta) = current_metadata.lock().await.clone() {
        write_frame(&mut incoming.stream, TYPE_METADATA, &meta).await?;
    }

    loop {
        match rx.recv().await {
            Ok(frame) => {
                if let Err(e) = write_frame(&mut incoming.stream, frame.kind, &frame.payload).await
                {
                    return Err(e);
                }
            }
            Err(broadcast::error::RecvError::Lagged(_n)) => {
                // This listener fell behind. Radio semantics: just keep
                // going; they'll resynchronize at the next frame they
                // actually receive. An alternative is to disconnect.
                continue;
            }
            Err(broadcast::error::RecvError::Closed) => return Ok(()),
        }
    }
}

async fn run_encoder(
    files: Vec<PathBuf>,
    tx: broadcast::Sender<Frame>,
    current_metadata: Arc<Mutex<Option<Vec<u8>>>>,
) -> Result<()> {
    let mut encoder = opus::Encoder::new(SAMPLE_RATE, opus::Channels::Stereo, opus::Application::Audio)
        .map_err(|e| anyhow!("opus encoder: {e}"))?;
    encoder.set_bitrate(opus::Bitrate::Bits(BITRATE)).map_err(|e| anyhow!("opus bitrate: {e}"))?;

    let mut frame_index: u64 = 0;
    let start = Instant::now();

    loop {
        for file in &files {
            let name = file.file_name().and_then(|s| s.to_str()).unwrap_or("?");
            let meta_bytes = format!("now playing: {name}").into_bytes();
            println!("now playing: {name}");
            *current_metadata.lock().await = Some(meta_bytes.clone());
            let _ = tx.send(Frame { kind: TYPE_METADATA, payload: meta_bytes });

            if let Err(e) =
                encode_file(file, &mut encoder, &tx, &mut frame_index, start).await
            {
                eprintln!("  skipping {name}: {e:#}");
            }
        }
    }
}

async fn encode_file(
    path: &Path,
    encoder: &mut opus::Encoder,
    tx: &broadcast::Sender<Frame>,
    frame_index: &mut u64,
    start: Instant,
) -> Result<()> {
    // Symphonia wants a sync file handle; that's fine inside this
    // spawned encoder task, it already runs off the main flow.
    let src = std::fs::File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let mss = MediaSourceStream::new(Box::new(src), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .context("probing format")?;
    let mut format = probed.format;
    let track = format
        .default_track()
        .context("no default track")?
        .clone();

    let source_rate = track.codec_params.sample_rate.context("no sample rate in codec params")?;
    let source_channels = track
        .codec_params
        .channels
        .context("no channel info in codec params")?;
    if source_rate != SAMPLE_RATE {
        bail!("sample rate is {source_rate}, expected 48000 — reencode source to 48 kHz stereo");
    }
    if source_channels.count() != 2 {
        bail!("channels = {}, expected 2 (stereo)", source_channels.count());
    }

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .context("making decoder")?;

    let mut pcm_buf: Vec<f32> = Vec::new();
    let mut opus_out = vec![0u8; 4000];

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break
            }
            Err(e) => return Err(e).context("reading packet"),
        };

        let decoded = match decoder.decode(&packet) {
            Ok(buf) => buf,
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(e) => return Err(e).context("decoding packet"),
        };

        let spec = *decoded.spec();
        let mut sample_buf =
            SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
        sample_buf.copy_interleaved_ref(decoded);
        pcm_buf.extend_from_slice(sample_buf.samples());

        while pcm_buf.len() >= SAMPLES_PER_FRAME {
            let frame_slice: Vec<f32> = pcm_buf.drain(..SAMPLES_PER_FRAME).collect();
            let len = encoder
                .encode_float(&frame_slice, &mut opus_out)
                .map_err(|e| anyhow!("opus encode: {e}"))?;
            let payload = opus_out[..len].to_vec();

            *frame_index += 1;
            let deadline = start + Duration::from_millis(*frame_index * u64::from(FRAME_MS));
            let now = Instant::now();
            if deadline > now {
                tokio::time::sleep(deadline - now).await;
            }

            let _ = tx.send(Frame { kind: TYPE_AUDIO, payload });
        }
    }

    Ok(())
}

/// Run the listener (the "tuner"): receive frames and play audio.
///
/// # Errors
/// Returns an error if audio output cannot be set up or the stream
/// fails with an unrecoverable error.
pub async fn tune(mut outgoing: OutgoingConnection) -> Result<()> {
    let mut producer = build_audio_output()?;

    let mut decoder = opus::Decoder::new(SAMPLE_RATE, opus::Channels::Stereo)
        .map_err(|e| anyhow!("opus decoder: {e}"))?;
    let mut pcm_out = vec![0.0f32; SAMPLES_PER_FRAME * 2]; // generous

    println!("  (press Ctrl-C to disconnect)");

    loop {
        let (kind, payload) = match read_frame(&mut outgoing.stream).await {
            Ok(f) => f,
            Err(e) if is_eof(&e) => {
                println!("  station ended the stream.");
                return Ok(());
            }
            Err(e) => return Err(e),
        };

        match kind {
            TYPE_METADATA => {
                if let Ok(s) = std::str::from_utf8(&payload) {
                    println!("  {s}");
                }
            }
            TYPE_AUDIO => {
                let n = decoder
                    .decode_float(&payload, &mut pcm_out, false)
                    .map_err(|e| anyhow!("opus decode: {e}"))?;
                let total = n * usize::from(CHANNELS);
                for s in &pcm_out[..total] {
                    let _ = producer.try_push(*s);
                }
            }
            TYPE_END => {
                println!("  station sent END.");
                return Ok(());
            }
            other => {
                eprintln!("  unknown frame type {other:#x}, ignoring");
            }
        }
    }
}

type RbProducer = ringbuf::HeapProd<f32>;

/// Build a cpal output stream that pulls from a ring buffer. Returns
/// the producer side of the ring; the consumer is moved into the
/// audio callback. The cpal `Stream` is intentionally leaked for
/// the process lifetime (it is `!Send`, cannot cross task boundaries,
/// and is released implicitly on process exit).
fn build_audio_output() -> Result<RbProducer> {
    // ~1 second of stereo audio buffer. Steady-state fill target is
    // ~200 ms, which absorbs Tor-circuit jitter without noticeable
    // added latency.
    let capacity = SAMPLE_RATE as usize * CHANNELS as usize;
    let rb = HeapRb::<f32>::new(capacity);
    let (producer, mut consumer) = rb.split();

    let host = cpal::default_host();
    let device = host.default_output_device().context("no default audio output device")?;
    let config = cpal::StreamConfig {
        channels: CHANNELS,
        sample_rate: cpal::SampleRate(SAMPLE_RATE),
        buffer_size: cpal::BufferSize::Default,
    };

    let stream = device
        .build_output_stream::<f32, _, _>(
            &config,
            move |output: &mut [f32], _info: &cpal::OutputCallbackInfo| {
                for sample in output.iter_mut() {
                    *sample = consumer.try_pop().unwrap_or(0.0);
                }
            },
            move |err| eprintln!("cpal error: {err}"),
            None,
        )
        .map_err(|e| anyhow!("building audio output stream: {e}"))?;
    stream.play().map_err(|e| anyhow!("starting audio output: {e}"))?;

    Box::leak(Box::new(stream));
    Ok(producer)
}

async fn write_frame<W: tokio::io::AsyncWrite + Unpin>(
    w: &mut W,
    kind: u8,
    payload: &[u8],
) -> Result<()> {
    let len = u32::try_from(payload.len()).context("frame payload too large")?;
    w.write_all(&[kind]).await.context("writing frame kind")?;
    w.write_all(&len.to_be_bytes()).await.context("writing frame length")?;
    w.write_all(payload).await.context("writing frame payload")?;
    w.flush().await.ok();
    Ok(())
}

async fn read_frame<R: tokio::io::AsyncRead + Unpin>(r: &mut R) -> Result<(u8, Vec<u8>)> {
    let mut header = [0u8; 5];
    r.read_exact(&mut header).await.context("reading frame header")?;
    let kind = header[0];
    let len = u32::from_be_bytes([header[1], header[2], header[3], header[4]]);
    if len > MAX_FRAME_BYTES {
        bail!("frame length {len} exceeds cap {MAX_FRAME_BYTES}");
    }
    let mut payload = vec![0u8; len as usize];
    r.read_exact(&mut payload).await.context("reading frame payload")?;
    Ok((kind, payload))
}

fn is_eof(err: &anyhow::Error) -> bool {
    err.chain().any(|e| {
        e.downcast_ref::<std::io::Error>()
            .is_some_and(|ioe| ioe.kind() == std::io::ErrorKind::UnexpectedEof)
    })
}

/// Which layer this crate implements.
pub const LAYER: ultranet_core::Layer = ultranet_core::Layer::Application;
