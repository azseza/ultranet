# M5-KV — Trustless KV cache

**Status:** Design note, 2026-04-20. Companion to `m5-compute.md`.

---

## What problem this solves

Modern LLM inference is dominated by one data structure: the **KV cache** — the per-token key and value tensors each attention layer keeps around so it doesn't have to re-read the whole prompt on every new token. For a 128k-context turn, the KV cache can be **gigabytes**. Without some form of caching, every turn of a conversation re-prefills the whole context from scratch — seconds to minutes of GPU time, every single message.

In a normal datacenter this problem is solved with fast fabrics (100 Gbps InfiniBand) and trusted shared storage. Neither exists in Ultranet: the network between nodes is Tor (≈200 ms RTT, ≈5 MB/s throughput), and no node trusts any other.

So: **how do we keep KV cache working without a fast fabric and without a trusted host?**

This doc answers that.

---

## One-page literature summary

(Full survey lives elsewhere; here's what actually lands.)

**Five primitives that port to Ultranet:**

1. **MLA** — Multi-head Latent Attention (DeepSeek-V2, 2024). Compresses the KV cache ~16× by projecting to a low-rank latent. Makes KV cache small enough to actually ship over Tor. *This is the single most important primitive in the survey.*
2. **Block + prefix index** (vLLM/PagedAttention, SGLang/RadixAttention, ChunkAttention). Right local data structure for KV. We adopt it, but the prefix *keys* must be keyed HMACs, not raw hashes (raw hashes fingerprint users).
3. **Bandwidth-aware streaming** (CacheGen, CacheBlend, LMCache). The only research family that explicitly designed for constrained WAN bandwidth. Compression-then-stream directly applies.
4. **Selective fetch** (InfiniGen). Over a 200 ms link the only affordable fetch policy is "fetch the KV you will actually attend to, not the rest." InfiniGen's cheap-rehearsal predictor is almost directly portable.
5. **Same-node prefill + decode.** Invert DistServe/Splitwise: split roles across *time* on one node, never across Tor.

**Five gaps no surveyed paper addresses:**

1. Binding the KV-encryption key to an SEV-SNP attestation measurement.
2. Verifiable inference (a node can return plausible wrong logits).
3. Traffic-analysis-resistant cache index (prefix-hash leak + block-size leak).
4. KV durability under volunteer churn.
5. Economics of KV hosting.

---

## The core bet

MLA shrinks KV ~16×. That changes what is **feasible**, not just what is fast.

Rough numbers, DeepSeek-V2 family with MLA:

| Context | MLA KV size (approx) | Shipping over 5 MB/s Tor |
|---------|----------------------|--------------------------|
| 2k      | ~2 MB                | <1 s                     |
| 8k      | ~8 MB                | ~2 s                     |
| 16k     | ~16 MB               | ~3–4 s                   |
| 32k     | ~32 MB               | ~6–8 s                   |
| 64k     | ~64 MB               | ~13 s                    |
| 128k    | ~128 MB              | ~25 s                    |

The 8–16k row is **the sweet spot**. For conversational workloads (chat, short agent turns, shortish documents) we can ship the entire KV cache in the round-trip budget of a single interactive turn. That unlocks a design **no surveyed system uses**:

> **The client holds its own encrypted KV cache, ships it with each turn, provider's enclave decrypts it inside SEV-SNP, updates it, ships the new version back.**
>
> **No persistent KV ever sits on an untrusted host.**

This is the Ultranet bet. If the bandwidth math holds, we sidestep the entire KV-hosting-economics / KV-durability-under-churn / KV-encryption-at-rest problem by default. If it doesn't hold, we fall back to persistent-mode (below), which keeps the property but costs more per turn.

---

## Two session modes

### Ephemeral mode (default for short context)

```
client                                     provider enclave
  │                                              │
  │   PROMPT: { tokens, encrypted_kv_in }        │
  │─────────────────────────────────────────────▶│
  │                                              │  decrypt KV
  │                                              │  run prefill + decode
  │                                              │  emit tokens
  │                                              │
  │   TOKEN stream (encrypted)                   │
  │◀─────────────────────────────────────────────│
  │                                              │
  │   UPDATED_KV: { encrypted_kv_out }           │
  │◀─────────────────────────────────────────────│
  │                                              │
  client stores encrypted_kv_out locally
  provider enclave wipes KV from memory on disconnect
```

Every turn: ship KV up, run inference, ship KV back. The provider's enclave never persists KV between turns. Nothing about any prior turn survives on the provider side.

Applies when: context ≤ ~16k tokens, or the user's UX budget tolerates the round-trip.

### Persistent mode (for long context)

```
client                                     provider enclave
  │                                              │
  │   PROMPT: { tokens, session_id }             │
  │─────────────────────────────────────────────▶│
  │                                              │  look up encrypted_kv in enclave memory
  │                                              │  (or fetch sharded ciphertext from peers)
  │                                              │  decrypt, run inference, emit tokens
  │                                              │
  │   TOKEN stream                               │
  │◀─────────────────────────────────────────────│
  │                                              │
                     KV stays in enclave
                     Ciphertext-at-rest: encrypted under enclave key
                     Optional: erasure-coded shards across k-of-n onion peers for durability
```

The enclave keeps KV across turns. KV at rest on the provider's storage is ciphertext — keyed to the measurement, not the machine, so migration is possible. If the provider dies, any peer holding a shard can be asked for it; the client can still recover (k-of-n) without trusting any individual shard-holder.

Applies when: context > 16k, or latency targets prefer not to round-trip 100+ MB per turn.

A client chooses per-session. Default: ephemeral. Persistent mode opts into durability risk explicitly.

---

## Architecture

### Model constraint

Commit to **MLA-family models** for the trustless path:
- DeepSeek-V2, V2-Lite, V3
- MiniMax-01 (hybrid linear/softmax, orthogonal win but compatible)
- TransMLA-retrofitted Llama/Mistral/Gemma, if and when the community delivers clean conversions

Non-MLA models still work in the non-attested fallback tier, but the headline "ship KV over Tor" guarantees apply only inside the MLA family. This is a real constraint and we surface it to users clearly.

### KV-encryption key binding

The KV-encryption key is **derived from the attestation session**, not from any persistent node identity:

```
channel_key = HKDF(ECDH(client_eph_x25519, enclave_eph_x25519))
kv_key      = HKDF(channel_key, "ultranet-kv-v1")
```

Consequences:
- Only the measured binary that ran the attestation can decrypt the KV.
- A node reboot invalidates all persistent KVs (new enclave keypair → new channel → new kv_key). This is correct behavior: re-attest, reship (ephemeral) or re-fetch + re-verify (persistent).
- A node swap (operator rebuilds their machine) invalidates all KVs. Same story.
- The host OS, even with root and disk access, sees only ciphertext.

The ciphertext format (proposal):
```
chacha20poly1305_seal(kv_key, nonce=block_index, aad=session_id || block_id, plaintext=kv_block)
```

### Block layout

Following the vLLM/PagedAttention lineage:
- KV is paged into blocks of fixed size (e.g. 16 tokens per block, per layer).
- Each block has a stable `block_id` assigned inside the enclave.
- Blocks are indexed by a per-session keyed prefix hash: `prefix_id = HMAC(session_key, prefix_token_ids)`.
- Block sizes on the wire are **padded** to a fixed size (e.g. nearest 4 KB) so size doesn't leak prefix boundaries.

### Prefix index privacy

All surveyed prefix-cache systems index by raw token-sequence hash. In a shared-tenant untrusted setting that's a fingerprint: an adversarial node can learn which system prompts or documents appear across user sessions.

Our index keys are `HMAC(session_key, prefix_tokens)`. Two consequences:
- An attacker cannot correlate prefixes across users (session keys differ).
- Cross-session prefix reuse for one user still works (same session key rebuilds the same index).
- Cross-*user* prefix sharing (the vLLM thing where many users share "You are a helpful assistant...") is gone. We trade it for privacy. See TODOs for recovery via keyed set-intersection schemes.

### Selective fetch (InfiniGen inside the enclave)

For persistent-mode long context, we don't want to decrypt the full KV per decode step. Borrowing InfiniGen's idea:
- Cheap rehearsal layer predicts which KV blocks will have significant attention weight.
- Enclave decrypts only those blocks for that decode step.
- Amortizes decryption + memory bandwidth + (in sharded mode) network fetch.

This is implemented inside the enclave, not across the wire. No new wire protocol needed.

### Erasure-coded shards for durability (persistent mode, optional)

```
encrypted_kv ──── k-of-n Reed-Solomon ──── shard_1, shard_2, ..., shard_n
                                                │
                                                ▼
                                          distributed to k-of-n
                                          volunteer onion peers
```

Any `k` of `n` shards reconstruct the full KV. Ciphertext, so no shard leaks content. Shard integrity via AEAD on each shard. Deferred to a later substage; flagged now.

---

## Wire protocol (extending m5-compute)

Reusing the M5 TLV framing. KV-specific message types:

| Type  | Direction | Name         | Payload                                           |
|-------|-----------|--------------|---------------------------------------------------|
| `0x30` | C → P    | KV_UPLOAD    | Ciphertext KV + session_id + manifest             |
| `0x31` | P → C    | KV_DOWNLOAD  | Ciphertext KV + manifest (for ephemeral reply)    |
| `0x32` | C → P    | KV_RESUME    | session_id (persistent-mode turn)                 |
| `0x33` | P → C    | KV_MISS      | session_id — provider doesn't have it, reship     |
| `0x34` | C → P    | KV_SHARD_PUT | shard + shard_id (durability layer)               |
| `0x35` | C → P    | KV_SHARD_GET | shard_id                                          |
| `0x36` | P → C    | KV_SHARD     | shard_id + ciphertext                             |

All payloads are AEAD-sealed under the channel key (for control) or the kv_key (for KV bytes themselves).

---

## Non-attested fallback tier

A user should be able to use Ultranet inference on consumer hardware (no SEV-SNP) with *weaker guarantees clearly disclosed*. In that tier:

- No attestation. The host operator can read the prompt and the KV.
- Tor transport still applies. The network path is still anonymous and unmetered.
- The protocol is otherwise identical — same block layout, same message types — so a client can switch tiers with a flag.
- UI makes the downgrade explicit: `WARNING: this provider is not attested. Your prompt is visible to the operator.` Fail-visible, not fail-closed, because the user explicitly opted in.

This matters because SEV-SNP hardware is rare. A non-attested tier lets Ultranet be *useful* before the attested tier reaches scale.

---

## Math check: is this actually feasible?

Worked example for DeepSeek-V2-Lite (our M5a workhorse), MLA:

- 27 attention layers, latent dim ≈ 576 per layer, ~64 heads × 128 per-head via MLA projection
- MLA KV per token per layer ≈ 1.1 KB (quoted values vary by release; treat as order-of-magnitude)
- Per token total ≈ 30 KB
- 8k tokens ≈ 240 MB

Hmm — this is *worse* than our earlier estimate. The full-size DeepSeek-V2 (236B MoE) has a 60-layer stack; the V2-Lite 27-layer stack actually stores fewer bytes per token but still not trivially small at 8k.

**Action: measure, don't guess.** TODO/real-kv-size-measurement below. Our design survives whether the number is 30 MB or 300 MB — but the sweet-spot context length moves, and the ephemeral/persistent crossover moves with it. Measure before we commit to a default.

Worst case — ephemeral mode is only viable up to 2–4k context, and persistent mode with selective fetch is the real workhorse. That's still a working design; just not the elegant default we'd hoped for.

---

## Out of scope for M5-KV — on purpose

- **Cross-user prefix sharing via private set intersection.** Keyed indexes break this; recovering it requires OPRF or similar. Later.
- **Oblivious RAM over KV.** Hides *which* block was fetched, not just its contents. Huge overhead. Research tier.
- **Speculative decoding on the client.** Requires a small draft model on the client side; interesting but orthogonal.
- **Quantization-aware KV compression** (CacheGen-style perplexity-guided lossy compression). Probably a win but tunable per model; defer to after we have baseline numbers.
- **Payment for KV shard hosting.** Part of the Ultranet economic layer; belongs elsewhere.

---

## TODOs — concepts to clarify in the future

- **TODO/real-kv-size-measurement.** Actually measure MLA KV bytes per token for DeepSeek-V2-Lite, V2, V3 in the runtime we pick. All sizing decisions in this doc depend on it. First M5a task.
- **TODO/mla-support-in-runtime.** Confirm candle (or whichever runtime we land on) has production-grade MLA. If not, llama.cpp via FFI or mistralrs as fallback. Binding decisions here set the audit burden for all of M5.
- **TODO/cross-user-prefix-reuse.** Whether to give up cross-user prefix cache entirely (simplest, most private), or invest in a PSI-style scheme so "everyone's system prompt" can still share a single cached prefix without leaking which users use it. PSI is a research-grade primitive — not a week's work.
- **TODO/block-size-choice.** vLLM uses 16 tokens/block. Optimal for Ultranet may differ — bigger blocks amortize AEAD overhead, smaller blocks reduce over-fetch. Measure.
- **TODO/padded-block-size.** How much fixed-size padding is enough to defeat size side-channels? Analyze information leakage explicitly, not just pick a round number.
- **TODO/session-key-rotation.** How often do we rotate the session key (and therefore invalidate the prefix index)? Tied to forward secrecy guarantees. Probably per-session, possibly per-N-turns.
- **TODO/persistent-mode-gc.** Enclaves have bounded memory. When does the provider evict a persistent-mode KV? Policy: LRU + client-notified pinning. Details TBD.
- **TODO/shard-discovery.** How does a client find peers willing to hold shards? Gossip? Pre-negotiated pool? Market? The durability layer needs a DHT-like primitive that doesn't exist in Ultranet yet. Defer deeply but flag.
- **TODO/rehearsal-attention-oracle.** InfiniGen's rehearsal predictor is model-family-specific. Needs research for MLA models specifically — may already be done in a paper we missed.
- **TODO/quantized-kv.** FP16 is the baseline; INT8 or FP8 KV halves the bytes. Compatible with MLA? Check.
- **TODO/non-attested-tier-ux.** What exactly does the CLI/UX say when the user is about to send a prompt to a non-attested provider? Hard warnings, or config-level opt-in only? Policy call.
- **TODO/session-resumption.** After a disconnect (Tor circuit dies), can a client resume the same persistent-mode session? Requires binding between attestation epochs. Design later.
- **TODO/colocated-cache-tenant-leak.** Two clients sharing the same provider enclave — can one see the other's KV via cache-timing or allocator reuse? Standard side-channel concern, needs an explicit mitigation.

---

## TODOs — making the network fast

KV cache movement is where most of the per-turn latency budget lives. This is the speed front.

- **TODO/measure-tor-throughput.** Real numbers between the user's machines over Tor. Today all sizing is "5 MB/s on a good circuit" — could be 500 KB/s or 15 MB/s in practice. First measurement before design commitment.
- **TODO/parallel-circuits-for-kv.** Stripe KV ciphertext across 4 parallel Tor circuits. ~4× throughput at the cost of more relays per session. Research politeness and consensus rules.
- **TODO/compress-before-encrypt.** Zstd on KV before AEAD. MLA KV is float tensors — zstd alone won't do much. Evaluate specialized float-tensor compressors (CacheGen's quantization-aware approach, fpzip, zfp).
- **TODO/incremental-kv-upload.** Only upload *new* KV blocks each turn, not the entire cache. The first N-1 turns' blocks are unchanged; ship only the delta. Requires client-side block-manifest tracking. High-value optimization.
- **TODO/pipeline-kv-with-prompt.** Send KV_UPLOAD frames *before* the full prompt finishes uploading. Overlap the bytes with the prompt itself. Small win but cheap.
- **TODO/kv-prefetch-on-dial.** If the client knows it's going to dial a provider for a long session, start KV upload as soon as M3 + attestation succeed, even before the user has typed the prompt. Hides round-trip latency behind UX time.
- **TODO/shard-parallel-fetch.** Persistent-mode + erasure-coded shards → fetch `k` shards in parallel from different onion peers. Bandwidth aggregation for free.
- **TODO/selective-fetch-over-wire.** If KV ships block-by-block (persistent mode, sharded), apply InfiniGen-style selective fetch at the *wire* level, not just inside the enclave. Fetches only the blocks the rehearsal predicts will matter. Dramatic bandwidth savings on long context.
- **TODO/time-to-first-token-budget.** Concrete number: v1 interactive use wants TTFT ≤ 5s. Budget that across (Tor dial) + (attestation) + (KV upload) + (prefill) + (first token generation). Each component needs a sub-budget.
- **TODO/cover-traffic-dial.** I7 says cover traffic. KV shipping already fuzzes timing patterns (big bursts). Cover traffic *on top of* KV shipping may be redundant for this use case; decide on a policy.
- **TODO/circuit-reuse-across-sessions.** One client, many provider sessions in quick succession — reuse circuits where the destination overlaps. Tor guards may make this already-happen; verify.
- **TODO/benchmark-vs-baseline.** At every milestone, compare TTFT and tokens/sec against (a) OpenAI/Anthropic API as the "fast but surveilled" baseline, (b) local inference on the client machine as the "private but limited" baseline. Ultranet's job is to approach (b)'s privacy with performance closer to (a) — numbers should tell a believable story.

---

## Demo plan — M5a-KV

Runs alongside M5a (non-attested inference), measures the KV side.

1. With M5a inference running, instrument:
   - Time to upload KV for 1k, 4k, 8k, 16k context
   - Actual MLA KV size per token in our runtime
   - TTFT contribution from KV transfer
2. Log real numbers into this doc under a "Measured values" section so future design choices have data.
3. Try ephemeral-mode round-trip for 8k context. Does it feel interactive? If yes, ephemeral is our default. If no, persistent-mode becomes the default sooner.
4. Identify the single biggest latency sink and attack it first.

No attestation, no shards, no selective fetch in M5a-KV. Just baseline numbers. Everything else builds on the measurements.

---

## Ready?

No new crate for M5-KV proper — KV lives inside `ultranet-compute` as a submodule. Additional dependencies: `chacha20poly1305`, `hkdf`, a Reed-Solomon crate (later), a keyed-hash crate (`hmac` + `sha2`, already available).

Sequence: M5a inference plumbing → M5a-KV measurements → decide default mode → M5b adds attestation binding to kv_key → M5c on real hardware → durability shards as a separate later substage.
