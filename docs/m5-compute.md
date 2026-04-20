# M5 — Trustless compute

**Status:** Design note, 2026-04-20.

---

## What M5 proves

One user. One untrusted compute node. One LLM inference request. Zero leaked prompts.

The user sends a prompt over Tor. The prompt runs inside an attested SEV-SNP enclave on hardware the user does not own. *Before sending any sensitive bytes*, the user cryptographically verifies that the code which will handle the prompt is the exact code they reviewed — not a modified version, not a keylogger, not a different model. The operator of the machine — even with root, hypervisor access, or physical access to the box — cannot see the prompt, the response, or the tokens streaming in between.

This is the first time Ultranet delivers on its headline promise: **confidential compute on someone else's hardware**.

---

## What "trustless" actually means here

"Trustless" is a loaded word. Pinning it down so we don't oversell:

- **We trust the hardware vendor.** SEV-SNP relies on AMD's root-of-trust certificate chain. If AMD's signing key leaks or AMD colludes with an adversary, the guarantee evaporates. This is the price of not building our own silicon. Intel TDX is a symmetric alternative with the same structural caveat.
- **We trust the code we reviewed.** SEV-SNP attestation proves "this machine is running a binary with this SHA-256 measurement." It says nothing about whether that binary is *correct*. A backdoor compiled into the published binary would be happily attested to.
- **We trust the model weights only if we check them.** Attestation proves which *binary* runs; it does **not** prove which *weights* are loaded. A malicious provider could attest to the correct inference binary and silently load poisoned weights. We handle this by committing a weights-hash in the attestation report (see TODOs).
- **We do NOT trust the operator.** Root on the host, the hypervisor, co-tenant VMs, physical access — SEV-SNP is designed to resist all of them.
- **We do NOT trust the network path.** The onion circuit handles that already; the end-to-end channel binds to the enclave's attestation key, not the node's onion identity.

What we claim, precisely: **the user's prompt is legible only to the measured binary, and the user can verify which binary will see it before they send it**. Nothing more. Call it *confidential inference* if "trustless" grates.

---

## What it does, end to end

**Provider** side (the compute node):
```
$ ultranet serve-inference --model deepseek-v2-lite --weights-hash sha256:abcd...
loading model (2.4 GB into encrypted VM memory)...
  weights SHA-256: abcd...  OK
  measurement:    7e3f...
my provider:  ult1zzz...7xez4.onion  (dial with: ultranet infer <id>)
my identity: ult1abc...xyz
attestation report ready, waiting for client...
```

**Client** side:
```
$ ultranet infer ult1zzz...7xez4.onion --expect-measurement 7e3f... --expect-weights sha256:abcd...
  dialing...
  handshake (M3)... peer identity ult1abc...xyz
  requesting attestation...
  verifying AMD cert chain...                OK
  verifying measurement matches expected...  OK
  verifying weights hash matches expected... OK
  establishing channel bound to enclave key...
> What's the capital of France?
Paris.
> (Ctrl-D to end)
```

Any verification failure aborts the connection *before* any prompt leaves the client. Fail-closed (I6).

---

## Architecture in one picture

```
  CLIENT                                 TOR CIRCUIT                   PROVIDER HOST
                                                                       ┌────────────────┐
                                                                       │ untrusted OS   │
                                                                       │  ┌──────────┐  │
  prompt ────────── M3 handshake ─────────────────────────────────────▶│  │ SEV-SNP  │  │
  (stored in RAM)                                                      │  │ enclave  │  │
                                                                       │  │          │  │
  attestation   ◀── attestation report ────────────────────────────────│  │  model   │  │
  verifier                                                             │  │  weights │  │
      │                                                                │  │  runtime │  │
      ▼                                                                │  │          │  │
  ECDH ──────────── channel bound to enclave pubkey ───────────────────│  │          │  │
                                                                       │  └──────────┘  │
  encrypted prompt ───────────────────────────────────────────────────▶│       ↕        │
  streamed tokens ◀────────────────────────────────────────────────────│   plaintext    │
                                                                       │   only inside  │
                                                                       │   the enclave  │
                                                                       └────────────────┘
```

Everything outside the dashed box is ciphertext or onion-wrapped. The host OS, hypervisor, and any process with root sees only encrypted memory and encrypted network bytes.

---

## Milestones within M5

M5 is big enough that we stage it. Each substage is independently runnable and testable.

- **M5a — Non-attested inference.** Client dials a provider over L3, sends a prompt, receives streamed tokens. No enclave. Lets us run a real model end-to-end, proves the service protocol, exercises the KV-cache design. Explicitly not private: the operator can read everything. Works on any hardware, including consumer GPUs.
- **M5b — Mock attestation.** Full attestation verifier on the client side, full key-binding flow, self-signed quote on the provider side. Runs on any hardware. Lets us debug the protocol without blocking on SEV-SNP machines.
- **M5c — Real SEV-SNP.** Swap the simulator for `/dev/sev-guest`. Run on EPYC Milan or newer. First moment Ultranet is actually confidential.

M5a is the next milestone after M4. M5c is the first that needs hardware the user says they have but "not now."

---

## Wire protocol

After the M3 signed handshake completes (both sides know each other's identity), the inference subprotocol kicks in. Same TLV framing as M4 radio:

```
1 byte   type
4 bytes  length N
N bytes  payload
```

Message types for M5:

| Type  | Direction | Name             | Payload                                               |
|-------|-----------|------------------|-------------------------------------------------------|
| `0x10` | C → P    | REQUEST_ATTEST   | Empty. Client asks for the attestation bundle.        |
| `0x11` | P → C    | ATTESTATION      | Serialized SEV-SNP report + AMD cert chain + enclave ephemeral pubkey + weights hash |
| `0x12` | C → P    | CHANNEL_ESTABLISH | Client ephemeral ECDH pubkey (encrypted? no — ECDH is safe in the clear) |
| `0x20` | C → P    | PROMPT           | Encrypted under channel key: user's prompt tokens + session state reference |
| `0x21` | P → C    | TOKEN            | Encrypted under channel key: one or more streamed output tokens |
| `0x22` | P → C    | DONE             | Encrypted under channel key: generation complete, stats |
| `0x23` | C → P    | CANCEL           | Empty. Client aborts this generation.                 |
| `0xFF` | both     | ERROR            | UTF-8 error message                                   |

The **channel key** is derived via ECDH between the client's ephemeral X25519 key and the enclave's ephemeral X25519 key from the attestation report. Both sides forget their keys when the session ends — perfect forward secrecy by construction.

KV-cache-related messages (see `m5-kv.md`) are a separate type range (`0x30`–`0x3F`).

---

## Attestation flow in detail

1. **Provider boots the enclave.** The inference binary is loaded into an encrypted VM. SEV-SNP measures it. The binary, *inside the enclave*, generates an ephemeral X25519 keypair. It writes the pubkey into the `REPORT_DATA` field of an attestation report it requests from `/dev/sev-guest`. The report is signed by the AMD-provisioned VCEK key.
2. **Client dials (M3).** Signed handshake completes; client knows the provider's identity peer ID.
3. **Client sends REQUEST_ATTEST.** Provider replies with the attestation report + the AMD VCEK cert + the endorsement chain back to AMD's root.
4. **Client verifies:**
   - AMD cert chain valid, root matches a pinned AMD public key.
   - Platform TCB version meets our minimum.
   - Report is freshly signed (no stale replay; we include a client-generated nonce — covered by `REPORT_DATA` or a wrapper).
   - Measurement matches one of the published-and-reviewed Ultranet inference binary builds.
   - Weights hash matches what the client asked for.
   - Enclave pubkey is extracted from `REPORT_DATA`.
5. **ECDH.** Client generates its own X25519 ephemeral key, sends pubkey in CHANNEL_ESTABLISH. Both sides derive a shared secret, expand into AEAD keys (ChaCha20-Poly1305, direction-specific).
6. **Encrypted traffic begins.** All subsequent messages on `0x20`–`0x3F` are AEAD-sealed.

If step 4 fails, the client closes the connection *without sending any prompt bytes*. I6 (fail-closed).

---

## Model + runtime

Inside the enclave runs a single-process inference server. Candidates:

- **candle** (pure Rust). Plays well with `forbid(unsafe_code)` at the workspace level. Performance is behind llama.cpp on CPU, competitive on CUDA.
- **llama.cpp** via FFI. Mature, fast, wide model support. Introduces unsafe FFI and a C++ dependency tree. Audit burden.
- **mistralrs**. Rust-native, specifically targets Mistral/Llama/Gemma families. Decent performance, growing ecosystem.

**Tentative pick: candle**, on the principle that Ultranet's audit surface is already large and pure-Rust matters. We revisit if candle's MLA support lags. See TODOs.

Models we care about in order (driven by the KV design — see `m5-kv.md`):

1. **DeepSeek-V2-Lite** (15.7B, MLA, permissively licensed) — the small MLA model. Our M5a workhorse.
2. **DeepSeek-V2** / **V3** — when provider hardware can handle the full-size MoE.
3. **TransMLA retrofits** of Llama/Mistral families — if the community supplies them.

---

## Crate layout

New crate: `crates/ultranet-compute` (L4 Service in the design system).

Public surface (tentative):
```rust
// provider side
pub struct InferenceConfig { pub model: ModelChoice, pub weights_path: PathBuf, ... }
pub async fn serve(
    listener: ultranet_rendezvous::Listener,
    config: InferenceConfig,
) -> Result<()>;

// client side
pub struct ExpectedAttestation {
    pub measurement: [u8; 48],        // SEV-SNP measurement
    pub weights_hash: [u8; 32],       // SHA-256 of weights
    pub min_tcb_version: TcbVersion,
}
pub async fn infer(
    connection: ultranet_rendezvous::OutgoingConnection,
    expected: ExpectedAttestation,
    prompt: &str,
) -> impl Stream<Item = Result<String>>;
```

Separate crate `crates/ultranet-attest` for the attestation verifier so it can be reused (and unit-tested with canned SEV-SNP reports).

New node subcommands: `serve-inference` and `infer`.

---

## Trust in, trust out — what this doesn't give you

Being explicit because "confidential" gets conflated with "correct":

- **No guarantee the response is the ground truth.** The measured binary plus measured weights could still be a carefully poisoned system prompt, finetune, or adversarially trained model. Attestation only proves *what* ran.
- **No guarantee of availability.** A provider can attest, take the prompt, and never respond. Cost-free for them (they don't even need to run inference). Countermeasure: client-side timeout + reputation (see TODOs).
- **No guarantee across providers.** If the same client talks to provider A and provider B, there's no protocol-level reason A and B haven't colluded out-of-band to compare what they saw. Multi-provider private inference (split model, secret sharing) is a different and harder problem.
- **No anonymity of the provider to the client.** M3 handshake reveals each side's identity peer ID. Intended; we want client-side verification of *which* attested provider they talked to. Anonymous-to-client provider is a separate design.
- **No protection from memory side-channels.** SEV-SNP mitigates but does not eliminate cache-timing and transient-execution side-channel attacks on co-tenant workloads. Research-grade attacks published roughly yearly.

---

## Out of scope for M5 — on purpose

- **Multi-provider inference** (split model across provider A + B + C, no single one sees the full prompt). Valuable, much harder, later.
- **Verifiable inference** (proof the logits are correct given the attested weights + prompt). Open research problem; attestation-plus-redundant-execution is the path but adds 2–3× cost. See TODOs.
- **Payment / incentive layer.** Who pays whom, in what currency, with what accounting. Essential for a public network; orthogonal to the protocol.
- **Reputation.** Does this provider have a track record of actually responding, not censoring, not lying about their attestation stack? Huge question, deferred.
- **Agentic workloads / tool use.** The enclave doing network calls on behalf of the client. Explodes the threat model (can the tool call leak the prompt?). Different design note.
- **Fine-tuning / training inside the enclave.** Different performance envelope; later milestone if ever.
- **Intel TDX support.** Designed for, not implemented in M5. Parallel path once SEV-SNP is working.

---

## Dependency footprint — honest note

M5 is our biggest native-dep jump yet:

- **SEV-SNP guest tools** — `/dev/sev-guest` ioctl interface. Linux kernel 5.19+. Small surface. Direct syscall, no wrapper crate we'd trust more than writing it ourselves.
- **SEV-SNP verifier** — AMD publishes `snpguest` (Rust), we'll study it. May need to reimplement for audit reasons.
- **AMD cert chain** — one pinned root public key; the rest is fetched and verified.
- **candle** (likely) — pulls in `tch`, `cudarc`, etc. on GPU builds. Large tree; this is where the bulk of audit debt accumulates.
- **ChaCha20-Poly1305, X25519** — via `ring` or `dalek-cryptography`. Already in closure via `ed25519-dalek`.

Every one of these is an audit line item before v1.0. Logging here so we don't forget.

---

## TODOs — concepts to clarify in the future

The honest list of "we're not sure yet":

- **TODO/attest-weights-commitment.** Precise format for committing model weights into the attestation report. Options: (a) hash of the raw weight file, loaded-and-hashed inside the enclave before inference starts, written into `REPORT_DATA`; (b) Merkle root over weight blocks so partial loading is verifiable. Pick one and write the exact bytes.
- **TODO/attest-freshness.** How we prevent replay of old attestation reports. SEV-SNP reports have a `REPORT_DATA` field — use it for a client nonce. Concrete protocol to be written.
- **TODO/tcb-policy.** What SEV-SNP TCB (trusted computing base) version is our minimum? How do we publish upgrades? How does a client know its policy file is current? Reproduces the PKI problem at a smaller scale.
- **TODO/binary-distribution.** Who publishes the reference Ultranet enclave binary, with what measurement, under what signing key? Today this is "whoever compiles it on their laptop" — not acceptable for v1.0. Reproducible builds (I5) + multi-party signing to be specified.
- **TODO/runtime-choice.** candle vs llama.cpp vs mistralrs. Benchmark MLA-family models on all three once `M5a` ships. Decision criteria: correctness (does it match reference outputs?), performance on consumer GPU, audit surface.
- **TODO/verifiable-inference.** Long-term: redundant execution across providers with cross-check? Proof-of-inference schemes (e.g. zkML)? Probabilistic spot-checks? All open research. At minimum document the threat so users know.
- **TODO/availability-reputation.** A provider that accepts prompts and silently drops them wastes users' time and leaks nothing-but-much. Reputation layer. Defer to post-M5c, but flag now.
- **TODO/poisoned-weights.** Even with weights-hash commitment, the *signer* of the weights hash is trusted. Who is the signer? For HF-released models we can pin HF's published SHA-256s. For community retrofits (TransMLA variants) we need a policy.
- **TODO/intel-tdx-parity.** Document the TDX-equivalent flow so M5c's design isn't AMD-locked. Structural differences matter at the verifier level.
- **TODO/model-license-audit.** DeepSeek models are MIT-licensed for the weights. Confirm for every model we officially support. Avoid shipping something whose license forbids our deployment.
- **TODO/enclave-resource-limits.** What happens when a client sends a 128k-token prompt to a provider with 24GB VRAM? Today: we OOM. Need graceful limits and error reporting.

---

## TODOs — making the network fast

Inference traffic over Tor is the canary for whether the whole design is viable. Preliminary list of speed levers:

- **TODO/fast-circuit-pool.** Today every M1/M3 run re-bootstraps arti (~10s). A warm circuit pool per node cuts cold-start to <1s. Applies to all milestones, not just M5.
- **TODO/session-pinned-circuits.** Don't rotate circuits mid-inference session. Once we're bound to an enclave, keep the circuit; the cost of re-handshake is attestation-round-trip-sized.
- **TODO/parallel-circuits.** Run N parallel Tor circuits per session and stripe bytes across them (classic aggregation). Tor circuit throughput is ~5 MB/s; 4 circuits gives ~20 MB/s. Cost: 4× relays used. Research what's allowed by Tor consensus and what's polite.
- **TODO/real-bandwidth-telemetry.** Measure actual throughput between the user's machines over Tor. Our design calculations are "5 MB/s on a good day." Confirm or adjust.
- **TODO/token-streaming-latency.** Time-to-first-token (TTFT) dominates interactive UX. Measure end-to-end TTFT — Tor circuit setup + M3 handshake + attestation + prefill + first token — and set a budget. v1 target: <5 s.
- **TODO/avoid-directory-authorities.** Tor fetches consensus from directory authorities on startup. For known-peer dials (we already have their onion service descriptor) we may skip some of this. Research.
- **TODO/cover-traffic-vs-speed.** I7 (cover traffic) directly costs bandwidth. Quantify the trade-off so we can expose a dial — "privacy mode" vs "speed mode" — per session rather than blanket-apply.
- **TODO/kv-cache-streaming.** KV cache movement is the biggest per-turn bandwidth cost after prefill. See `m5-kv.md` — most of the speed fight lives there.
- **TODO/batch-decode.** If multiple client sessions share a provider, we can batch their decode steps GPU-side for free throughput. Compatible with attestation if all clients attest the same enclave.
- **TODO/onion-service-descriptors-caching.** `HSDir` lookup can add seconds. Cache aggressively on the client side and gossip fresh descriptors between sessions.
- **TODO/protocol-compression.** ChaCha20-Poly1305 is fast but the *plaintext* is mostly text. Optional zstd on prompts and responses. Weigh compression-oracle concerns for hostile prompt contexts.

---

## Demo plan — M5a (non-attested, works today)

1. `cargo build --workspace` with a new `ultranet-compute` crate.
2. Download DeepSeek-V2-Lite weights (~30 GB, MIT license).
3. Terminal 1: `ultranet serve-inference --model deepseek-v2-lite --weights /path/to/weights`. Capture provider peer id.
4. Terminal 2: `ultranet infer <provider>` then type a prompt. Observe tokens streaming back.
5. Measure TTFT, tokens/sec over the Tor circuit. Record actual numbers into this doc so future milestones have a baseline.
6. Repeat with a 4k-token prompt. Observe KV-prefill cost, log it.

No security properties yet. Protocol plumbing only. Everything above M5a builds on this scaffolding.

---

## Ready?

Two new crates (`ultranet-compute`, `ultranet-attest`), ~1500–2000 lines, bigger dep closure, and the first milestone that needs specialized hardware to fully validate. But M5a gets us most of the protocol on consumer GPUs today — the attestation story can land in M5b/M5c when the EPYC boxes come online.

Sequence: finish M4 → small M4.5 accept-loop cleanup in `ultranet-rendezvous` → M5a.
