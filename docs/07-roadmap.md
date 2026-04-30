# Roadmap

**Status:** v0.1 — 2026-04-30.
**Purpose:** A milestone-by-milestone plan with **exit conditions** that are not negotiable. A milestone is "done" when its exit conditions hold, demonstrably, on a clean machine — not when the code compiles and looks right. Soft milestones rot. Hard milestones ship.

This document is the source of truth for what is built next. The vision docs (`00-manifesto.md`, `04-differentiator.md`, `05-design-system.md`, `06-browser.md`) say *what* and *why*. This document says *in what order, with what bar*.

---

## Principles

1. **A milestone is a falsifiable claim.** Each one ends with "you can demonstrate X on a clean machine." If you cannot produce the demo, the milestone is not done.
2. **No code without a design note.** Every milestone has a `mN-*.md` design note before its first line of Rust.
3. **No design note without a threat-model entry.** If a milestone introduces new attack surface, the threat model is updated *first*.
4. **One pair of eyes minimum.** Every PR for a numbered milestone is reviewed against the design note and the design system. AI-written code counts; the review still has to happen, by a human who can read every line.
5. **The pace is "right," not "fast."** Slow shipping that holds up under threat-model review beats fast shipping that does not.
6. **Test the invariants, not just the happy path.** Every milestone adds tests that would fail if I3 (onion-only egress) or I6 (fail closed) were violated. Invariant tests are non-optional.

---

## Status legend

- **DONE** — exit conditions hold; demo reproducible.
- **CODE LANDED, NOT VERIFIED** — implementation exists but exit conditions have not been demonstrated end-to-end.
- **IN DESIGN** — design note exists; code has not started.
- **PLANNED** — placeholder; design note has not been written yet.

---

## M1 — Workspace skeleton  ·  DONE

**Claim:** A Rust workspace with the layer model encoded in the type system, building cleanly under pedantic clippy and `forbid(unsafe)`.

**Exit conditions (held).**
- `cargo build --workspace` clean.
- `cargo clippy --workspace -- -D warnings` clean under workspace pedantic lints.
- `Layer` and `Invariant` enums in `ultranet-core` reflect the design system §2 and §3.
- Apache-2.0 license, NOTICE, reproducible-style toolchain pin.

---

## M2 — L2 bootstrap (Tor / Arti)  ·  DONE

**Design note.** `m2-rendezvous.md`.

**Claim:** Two processes on one host can each bootstrap a fresh ephemeral Tor client, publish a hidden service, and dial each other.

**Exit conditions (held).**
- `ultranet-transport::bootstrap` returns a working `TorClient` with state in a `TempDir` that is wiped on drop.
- A node binary publishes a hidden service and prints its `.onion`.
- A second node binary dials the printed `.onion` and exchanges a byte.

---

## M2.5 — Ephemeral state  ·  DONE

**Design note.** `m2.5-state.md`.

**Claim:** Restarts produce a fresh peer identity by default. Persistent identity is a deliberate, explicit opt-in, not the default.

**Exit conditions (held).**
- `ultranet-transport::Bootstrapped` owns a `TempDir`; drop wipes the directory.
- No code path persists identity material outside the explicit persistence API (which does not yet exist; M2.5b).

---

## M3 — Signed handshake  ·  DONE

**Design note.** `m3-signed-handshake.md`.

**Claim:** After connecting, both sides cryptographically prove their long-term identity to each other. A man-in-the-middle on the rendezvous descriptor cannot impersonate either party.

**Exit conditions (held).**
- A challenge-response over ed25519 in `ultranet-rendezvous`: listener challenge → dialer pubkey + signature → listener accept + listener pubkey.
- Tampering with any byte of the handshake (random fuzz of the signature, wrong magic, wrong version) causes the connection to abort with a typed error.
- Both ends learn the verified `IdentityPeerId` of the other.

**Verified-by-test.** Round-trip and tamper tests in `ultranet-rendezvous`.

---

## M4 — Radio broadcast  ·  CODE LANDED, NOT VERIFIED

**Design note.** `m4-radio.md`.

**Claim:** A broadcaster station serves audio frames to many listeners over the M3-authenticated rendezvous. Listeners join mid-stream. Disconnecting one listener does not affect others.

**Exit conditions.**
- [x] `ultranet-radio` library compiles with the framing protocol from the design note.
- [ ] **Two-listener integration test on one host**: a broadcaster, two listeners, a third listener joining mid-track, one listener disconnecting cleanly. All three audio streams identical (modulo join offset). Recorded as a reproducible script.
- [ ] **One regression test for I6**: a listener that fails the M3 handshake never receives a single audio frame. Demonstrated by a hostile-listener fixture.
- [ ] **Cross-process timing test**: round-trip latency for the join handshake measured and recorded as a baseline.

**Until those three demos exist as scripts in `tests/` or under `crates/ultranet-radio/tests/`, M4 is not DONE.**

**Why this matters.** M4 is the first milestone where an invariant could regress silently. We need the test discipline locked in *before* M5, because M5's enclave path is far harder to retrofit tests onto.

---

## M5 — Trustless compute  ·  IN DESIGN

**Design notes.** `m5-compute.md`, `m5-kv.md`.

**Claim:** A user can submit an LLM inference request to an untrusted compute provider over Tor, verify before any prompt leaves the client that the provider is running an attested SEV-SNP enclave with a known measurement and known weights hash, and receive streamed tokens through a channel cryptographically bound to the enclave's attestation key. The provider operator — even with root, hypervisor access, and physical access — cannot see the prompt, the response, or any token.

This is the headline milestone. M5 is the first time Ultranet delivers on the post-cloud claim.

**Sub-milestones.**

### M5.0 — Layer split in core
The `Layer` enum currently has a single `Service` variant. Per `05-design-system.md` v0.2, that splits into `ComputeService`, `StateService`, `RelayService`. This is mechanical and small, perfect first M5 PR.

**Exit conditions.**
- [ ] `Layer` enum updated; existing call sites compile by choosing the right sub-variant.
- [ ] `Layer::label` covers the new variants.
- [ ] `LAYER` constants in `ultranet-rendezvous` and `ultranet-radio` re-evaluated; they probably move (rendezvous stays at L3, radio is L5-app, transport stays at L2).
- [ ] Tests updated.

### M5.1 — Local SEV-SNP attestation roundtrip (no network)
Generate, parse, and verify an SEV-SNP attestation report locally — no Tor, no peer, no enclave yet. Develops the verifier code that everything else depends on.

**Exit conditions.**
- [ ] `ultranet-attestation` crate created. Pulls `sev` or equivalent vendored crate.
- [ ] Function `verify_report(report, expected_measurement, expected_weights_hash, amd_root_certs)` returning a typed verification result.
- [ ] Test vectors: at least one valid sample report (vendor-supplied), one with a tampered measurement, one with an expired AMD chain. All three verification outcomes correct.

### M5.2 — Enclave-key channel binding
Define and implement the binding from the M3 handshake to an attested enclave key. The post-handshake session key must be derivable only inside the enclave, otherwise an attacker who has compromised the host but not the enclave can decrypt prompts.

**Exit conditions.**
- [ ] Design extension to `m5-compute.md` specifying the binding (likely: enclave-internal ECDH key whose public part is bound into the attestation report; client ECDH against that).
- [ ] Threat-model entry: what attacks does the binding prevent, what does it not? (Specifically: a host that records ciphertext and receives the enclave's ECDH key after the fact still cannot decrypt, because session keys are forward-secret.)
- [ ] Implementation in `ultranet-rendezvous` (or a new `ultranet-channel` crate, decision in design note) with a self-test that demonstrates a host-side observer cannot derive the session key from observable bytes alone.

### M5.3 — Enclave runtime (Firecracker + measured boot)
Build the enclave-side host that runs an inference binary with a known measurement. This is the operationally hardest sub-milestone; it touches kernel, hypervisor, hardware.

**Exit conditions.**
- [ ] A Firecracker-based launcher that boots a measured Linux kernel + initramfs + inference binary into an SEV-SNP guest.
- [ ] The launcher emits an attestation report whose measurement matches the launched payload, reproducibly.
- [ ] The launched binary exposes a minimal request/response interface over a virtio channel; no listening sockets in the guest.
- [ ] **Hard requirement:** the host cannot read guest memory. Demonstrated by attempting to read `/proc/<pid>/mem` of the Firecracker process and showing the prompt is not present.

### M5.4 — Anonymously-routed inference end-to-end
Stitch M5.0–M5.3 together with M2/M3. Client over Tor; provider runs Firecracker enclave; full flow as in `m5-compute.md`.

**Exit conditions.**
- [ ] `ultranet infer` and `ultranet serve-inference` CLIs exist.
- [ ] A client can run inference against a remote provider node (different host, optionally different network) and receive correct tokens.
- [ ] **I6 regression test:** if attestation verification fails (e.g., wrong expected measurement), the client never sends the prompt. Verified by a fixture that intercepts client output before the dispatch.
- [ ] **I3 regression test:** the client makes no non-onion outbound connection during the entire flow. Verified by `strace` or equivalent on a clean machine.
- [ ] **End-to-end demo:** scripted, runs on a clean host, takes <10 minutes from `cargo build` to first token. Recorded.

### M5.5 — KV ephemeral mode
Per `m5-kv.md`, the client ships its own encrypted KV cache per turn. The simpler of the two KV modes; lands first.

**Exit conditions.**
- [ ] Inference request format includes optional encrypted KV blob.
- [ ] Enclave decrypts inside SNP, runs prefill+decode, re-encrypts the updated KV, returns it.
- [ ] An interactive multi-turn demo where the client retains continuity across turns *with a different provider node each turn*. (Demonstrates KV is portable, no provider-side persistence.)

### M5.6 — Weights commitment
The attestation must bind the weights hash, not only the binary measurement (`m5-compute.md` is explicit on this).

**Exit conditions.**
- [ ] Enclave computes weights hash on load, includes in attestation report data.
- [ ] Client verifies weights hash against expected.
- [ ] **Negative test:** swapping the weights file on the host without changing the binary causes verification to fail; the client refuses to send the prompt.

---

## M6 — The console (L5-runtime)  ·  PLANNED

**Design note.** `06-browser.md`.

This is a multi-quarter milestone. It is sequenced after M5 because the console needs the L4-compute primitive to be real before designing the "compute on" verb's UX. Sub-milestones M6.0 through M6.5 are listed in `06-browser.md`.

**M6 must not start until M5.4 is DONE.** A console without a working L4-compute is a wireframe; the value of the console is that it *renders* attestation chrome over actual attestations.

---

## M7 — L4-state (durable archive)  ·  PLANNED

**Design note.** `07-state.md` (does not exist yet).

**Claim:** Aisha (P3) can replicate her case archive across N peer-operated L4-state nodes such that loss of any K of them does not lose data, and seizure of any single one yields nothing readable.

**Exit conditions (provisional).**
- [ ] A storage-capability descriptor at L3.
- [ ] An encrypt-once, replicate-N protocol with HMAC-keyed addressing.
- [ ] Demonstrated archive and recovery against a fixture cluster.
- [ ] Threat-model entry: what does T7 (physical seizure) recover from a single L4-state node?

---

## M8 — Plural identity primitives  ·  PLANNED

**Design note.** `08-identity.md` (does not exist yet).

**Claim:** A user can operate multiple personas under a single author identity such that no remote peer or substrate observer can link two personas. Persona-switching in the console (M6.5) is the UX; M8 is the cryptography underneath.

---

## M9 — Cover traffic and constant-rate I/O  ·  PLANNED

**Design note.** `09-cover-traffic.md` (does not exist yet).

**Claim:** A node that is online generates a constant-rate traffic profile regardless of user activity. The mapping from user activity to wire activity is one-to-many or many-to-one in a way that defeats local-link analysis (T1, T2).

This closes a partial defense in §6.1. Until M9 lands, I7 is a stated invariant whose enforcement is incomplete.

---

## M10+ — substrate experiments  ·  PLANNED, RESEARCH

The first non-TCP/IP L1 substrate experiment (per A7). Candidates: a LAN-only ham-radio digital-mode bridge (most tractable) or a static FSO link between two pre-aimed nodes (most narratively-aligned). Goal is to prove the substrate-replaceability claim is real, not aspirational.

---

## Cross-cutting tracks (run in parallel with milestones)

These do not have milestone numbers because they are continuous, not discrete.

### CI and reproducible builds
- Set up CI that runs `cargo build`, `cargo test`, `cargo clippy -- -D warnings`, on every PR.
- Build a reproducible-build pipeline: same source, same compiler, same dependencies, same binary, byte-for-byte. Required by I5 before any v1.0.

### Threat-model maintenance
- Every milestone PR includes a threat-model diff: "this changes attack surface in the following way."
- §6 is reviewed quarterly; each item gets a "still unsolved / partially solved / closed" annotation.

### Documentation
- Every public API on every crate has a doc comment with at least: what it does, what invariant it depends on, what fails-closed behavior it has.
- A `BUILDING.md` for contributors, a `RUNNING.md` for users — kept up to date as milestones land.

### Audit-readiness
- Every release tag has a tag-message that lists the threat model version, the design system version, and the milestone numbers it covers.
- An external review of M5 is desirable before M5.4 is declared DONE; reviewers chosen from the Tor / Arti / SEV-SNP ecosystems.

---

## What this roadmap explicitly does not promise

- **A timeline.** Calendar dates rot. The only schedule promise is *order*.
- **Mobile.** All five personas work primarily on laptops. Mobile follows desktop; it does not lead.
- **A clearnet bridge, ever.** Not in M1, not in M11, not in v2.0. It is permanently off the table (axiom A3).
- **A token, ever.** Same reason.
- **Hosted onboarding.** The first peer is introduced out of band. There is no "ultranet.example.com" that smooths first contact.

---

## How this document changes

When a milestone moves status, this document is updated in the same PR. When a new milestone is added, it gets a number that does not break any existing reference. When a milestone is *abandoned*, it stays in this document with status `ABANDONED` and a note explaining why — because the absence of a milestone is itself information.

A roadmap that quietly forgets what it promised is not a roadmap. This one will not.
