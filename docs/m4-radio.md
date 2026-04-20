# M4 — Radio broadcast

**Status:** Design note, 2026-04-20.

---

## What M4 proves

One broadcaster, many listeners, all connected anonymously, all hearing the same live audio stream.

This is the first M5-class application milestone — the first time Ultranet *does something a person could sit down and use*. It also forces the concurrency problem we've been deferring: the broadcaster must handle multiple simultaneous listeners without collapsing, and a listener must handle a long-running stream without losing frames.

It's on-theme: pirate radio over an unstoppable, unlocatable transmitter is exactly the kind of application the manifesto is about. The broadcaster cannot be geolocated; the listeners cannot be geolocated; the station itself has no IP address to jam, no domain to seize, no Tier-1 ISP to pressure. It either works or Ultranet doesn't.

---

## What it does, end to end

**Broadcaster** side (the station):
```
$ ultranet broadcast --file track1.mp3 --file track2.flac --file track3.ogg
my station: ult1qn5k...7xez4.onion  (tune in with: ultranet tune <id>)
my identity: ult1abc...xyz
now playing: track1.mp3  (3:42)
  listener joined: ult1def...uvw   [1 listener]
  listener joined: ult1ghi...jkl   [2 listeners]
now playing: track2.flac (5:11)
  listener left: ult1def...uvw     [1 listener]
...
```

The broadcaster plays through the file list in order, then loops. New listeners tune in to *whatever is currently playing*, mid-track — just like real radio.

**Listener** side:
```
$ ultranet tune ult1qn5k...7xez4.onion
my identity: ult1mno...pqr
tuning in to ult1qn5k...7xez4.onion...
CONNECTED, station identity: ult1abc...xyz
now playing: track2.flac
  [plays audio through the default sound device]
```

`Ctrl-C` disconnects cleanly.

---

## Wire protocol

After the M3 signed handshake completes, both sides agree on each other's identity. Then the broadcaster sends a continuous stream of framed messages. The listener only reads.

Framing — one simple TLV per message, big-endian:

```
1 byte   type
4 bytes  length N
N bytes  payload
```

Message types:

| Type byte | Name     | Payload                                         |
|-----------|----------|-------------------------------------------------|
| `0x01`    | METADATA | UTF-8 string: `now playing: <filename>`         |
| `0x02`    | AUDIO    | One Opus-encoded packet                         |
| `0x03`    | END      | Empty. Broadcaster has stopped.                 |

The broadcaster sends **one METADATA when a new track starts**, then a stream of **AUDIO** frames at the Opus frame rate (one per 20 ms of audio), then repeat. When a listener joins mid-track, the broadcaster immediately sends the current track's METADATA, then starts feeding the listener the live stream (from wherever the stream currently is, not from the start of the track). "Like real radio."

Opus is configured at **48 kHz, stereo, 64 kbps, 20 ms frames**. At that rate each AUDIO payload is ~160 bytes, plus 5 bytes of framing. ~82 frames/sec × 165 bytes = ~13 KB/s. That's well within a reasonable Tor circuit's bandwidth and small enough that high-latency circuits still deliver smoothly after a small jitter buffer.

**No handshake-layer framing changes.** M3's handshake still prefaces every stream. Radio framing only kicks in after M3 completes.

---

## Audio path

**On the broadcaster:**
```
files -> symphonia (decode to f32 PCM) -> resample to 48 kHz stereo
      -> opus::Encoder (20 ms frames) -> AUDIO messages -> fan-out
                                                            |
                                     [broadcast channel] --+-- per-listener task --> stream
                                                            +-- per-listener task --> stream
```

Files are decoded **lazily** — the broadcaster decodes as it plays, not upfront. `symphonia` handles MP3, FLAC, Ogg Vorbis, Ogg Opus, WAV, AAC; we accept whatever `symphonia` accepts.

**On the listener:**
```
stream -> parse TLV -> opus::Decoder (f32 PCM, 48 kHz stereo) -> cpal output device
                                                  |
                                             [~200 ms jitter buffer]
```

`cpal` gives us the system's default audio output on Linux/macOS/Windows. The jitter buffer is a small ring buffer between the decoder and the audio callback; `cpal`'s callback pulls from it, the decoder pushes into it.

---

## Concurrency model

A **single `tokio::sync::broadcast` channel** on the broadcaster. The main task encodes Opus frames and calls `.send()` into the channel. Each accepted listener spawns a task that subscribes to the channel and copies frames from the channel to its `DataStream`. If a listener's stream is slow (circuit lag), the channel's per-receiver lag behavior drops older frames for that listener, not for others. That matches "real radio" semantics — a listener with a bad connection just hears gaps, not everyone.

The listener's accept loop is finally a real loop:
```rust
loop {
    let incoming = listener.accept().await?;
    tokio::spawn(handle_listener(incoming, channel.subscribe()));
}
```

---

## Crate layout

New crate: `crates/ultranet-radio` (L5 in the design system — the first true L5 code).

Public surface:
```rust
// broadcaster side
pub async fn broadcast<P: AsRef<Path>>(
    listener: ultranet_rendezvous::Listener,
    files: Vec<P>,
) -> Result<()>;

// listener side
pub async fn tune(
    connection: ultranet_rendezvous::OutgoingConnection,
) -> Result<()>;
```

No new wire types leak into `ultranet-core`. Radio is an L5 concern; its message format does not pollute lower layers.

New subcommands on `ultranet-node`: `broadcast --file F --file F ...` and `tune <peer-id>`.

---

## Out of scope for M4 — on purpose

Things that are not wrong to want, just not M4.

- **Two-way voice / calls.** A different beast (jitter buffer on both sides, tight latency budget); M4 is one-way fan-out only.
- **Live microphone input.** The broadcaster plays files from disk. Mic input is a cleaner follow-up milestone once file-based broadcast is solid.
- **Track queue management, scheduling, cross-fades, ducking, EQ.** All things a real DJ app has. None of them are about *proving the architecture works for streamed audio*, which is M4's only job.
- **Listener count privacy.** In M4 the broadcaster knows exactly how many listeners and their identity peer IDs (from M3). A privacy-preserving listener count — where the broadcaster can prove authenticity without learning identity — is interesting and deferred.
- **DRM, rate limiting, authentication beyond M3, per-listener channels, chat backchannel.** All deferred.
- **Recording.** Listeners don't record to disk automatically. Deferred; arguably never part of Ultranet.
- **Seek / rewind.** Real radio has neither. A pre-recorded-show app would; M4 is live radio.

---

## Dependency footprint — honest note

M4 pulls in the first real native-code deps:

- `opus` crate — bindings to `libopus`. Requires the system `libopus` or `cargo:rustc-link-lib`. Well-audited upstream; the Rust bindings are thin.
- `symphonia` — pure-Rust, no native deps. Solid.
- `cpal` — Rust wrapper around platform audio APIs. Needs `libasound2-dev` on Linux.
- `tokio::sync::broadcast` — already have tokio.

This is a meaningful jump from M3's dependency closure. I'm surfacing it because every native dep is an audit obligation later. `opus` and `cpal` are not Ultranet's to own, but they *are* Ultranet's to verify before any v1.0 release. We log this as part of the M5 milestone's pre-release checklist.

---

## Demo plan

1. `cargo build --workspace` — confirm clean compile with new deps.
2. Prepare two or three audio files in `/tmp/` (any MP3/OGG/FLAC the user has lying around).
3. Terminal 1: `ultranet broadcast --file f1 --file f2 --file f3`. Capture the station peer id.
4. Terminal 2 (same or different machine): `ultranet tune <station>`. Should hear audio within ~5 seconds of connecting (bootstrap + handshake + jitter buffer fill).
5. While terminal 2 is playing, terminal 3: `ultranet tune <station>`. Should also hear audio, should be roughly in sync with terminal 2 (within circuit-latency jitter).
6. Close terminal 2. Terminal 1 should print `listener left`, terminal 3 should continue unaffected.
7. Let the broadcast cycle through all files and loop. Confirm listeners don't desync, don't crash, don't leak memory.

If any of these steps fails, that's the real content of the M4 debugging session.

---

## Ready?

New crate, ~500 lines across the crate and the node subcommands, three non-trivial dependencies, first real L5 application. Going in.
