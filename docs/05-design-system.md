# Ultranet Design System

**Status:** Draft v0.2 — 2026-04-30.
**Purpose:** The architectural "laws of physics" of Ultranet. Every module, every PR, every new subsystem is checked against this document. If a design decision conflicts with an invariant here, one of them is wrong, and the burden of proof is on the design.

This is not a style guide. It is not about UI. The "design system" here is the *architectural* design system — the set of invariants, layer boundaries, and cross-cutting rules that make Ultranet coherent across time and contributors.

---

## 1. Design axioms

These are statements we treat as true by assumption. Everything else in the system is derived from them.

### A1. The adversary is well-resourced, patient, and legal-adjacent
We design against a threat model that includes nation-state signals intelligence, coerced service providers, and adversaries who can compel hosting operators via legal process. We do not design only against "a hacker on coffee-shop wifi."

### A2. Metadata is content
Who spoke to whom, when, for how long, and at what volume is content, not metadata. The network protects this information with the same seriousness it protects the payload. A design that encrypts the payload but leaks the peer graph is not acceptable.

### A3. Convenience is the enemy of privacy
Every "fallback to clearnet," "optional encryption," "temporary debug bypass," or "just this once" flag in a privacy system eventually becomes the default, either through user behavior or through social engineering. Ultranet does not ship these. The only mode is the private mode.

### A4. Trust is minimized, not distributed
We do not solve trust by adding more trusted parties. We solve it by removing the need for trust entirely (cryptographic verification) or by making trust explicit and auditable (attestation). "Federation" is not a privacy property.

### A5. A seized node leaks nothing
If an adversary physically seizes a node, removes its storage, and decaps its chips, they should learn: that the node participated in Ultranet, and nothing else. Not who its peers were. Not what computations ran on it. Not what user identities it served.

### A6. Honesty about the unsolved
Where the state of the art cannot defend against an adversary class, we say so in the threat model, we do not paper over it, and we do not market the system as defending against it. Global passive adversary traffic correlation is the canonical example. The unsolved list is treated as a research portfolio, not a disclaimer (see threat model §6).

### A7. Substrate independence
Ultranet is decoupled from any specific physical-layer carrier. TCP/IP today, FSO mesh tomorrow, ham-radio digital modes or store-and-forward sneakernet under suppression. L1 is *replaceable*, not merely *abstractable*: the design admits substrate swaps without re-architecture above L2. This is the structural meaning of "sovereign topology."

---

## 2. Layer architecture

Ultranet has five layers, with two of them — L4 and L5 — sub-structured by role.

```
┌──────────────────────────────────────────────────────────────────┐
│  L5 — Application                                                │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │ L5-runtime — the verified hyperdocument console (browser)  │  │
│  │ Identity manager, attestation chrome, session journal      │  │
│  ├────────────────────────────────────────────────────────────┤  │
│  │ L5-app — messaging, file drop, radio, intake, archive UI   │  │
│  └────────────────────────────────────────────────────────────┘  │
├──────────────────────────────────────────────────────────────────┤
│  L4 — Service                                                    │
│  ┌──────────────┬──────────────┬─────────────────────────────┐   │
│  │ L4-compute   │ L4-state     │ L4-relay                    │   │
│  │ attested     │ encrypted    │ forwarding policy,          │   │
│  │ ephemeral    │ persistent,  │ cover-traffic shaping,      │   │
│  │ enclaves     │ replicated   │ optional mix layer          │   │
│  └──────────────┴──────────────┴─────────────────────────────┘   │
├──────────────────────────────────────────────────────────────────┤
│  L3 — Rendezvous & naming                                        │
│  Signed peer descriptors with typed capabilities, DHT-over-L2    │
├──────────────────────────────────────────────────────────────────┤
│  L2 — Anonymous transport                                        │
│  Onion circuits (Arti), stream multiplexing                      │
├──────────────────────────────────────────────────────────────────┤
│  L1 — Substrate                                                  │
│  Replaceable carrier: TCP/IP today, FSO / radio / sneakernet     │
└──────────────────────────────────────────────────────────────────┘
```

### Layer invariants

**L1 (Substrate).** Assumed-hostile. Every bit leaving L2 into L1 is encrypted, padded, and indistinguishable from noise. L1 is *replaceable* by design (axiom A7): TCP/IP is the current substrate; substrate swaps must not require changes above L2. Nothing above L1 may assume properties of the substrate beyond "delivers bytes, sometimes."

**L2 (Anonymous transport).** Single responsibility: build circuits that unlink source from destination. Built on Arti. No application-layer logic lives here. No naming. No service discovery. Circuits in, circuits out.

**L3 (Rendezvous & naming).** How peers find each other without a central directory. Hidden-service-style descriptors signed by the owning peer, published to a DHT that itself runs over L2. Descriptors carry **typed capabilities** — `compute`, `state`, `relay`, `intake`, `feed` — so a peer can advertise *what it offers* and a dialer can resolve *what it needs* without leaking the choice to the substrate. The only globally-visible name for a peer is its public key.

**L4 (Service).** Stateful and stateless services exposed over L3, sub-structured by lifetime and trust model:

- **L4-compute.** Ephemeral, hardware-attested enclaves (SEV-SNP). Workload in, attested result out, no persistence visible to the operator. Headline use case: anonymous LLM inference (M5).
- **L4-state.** Persistent, encrypted-at-rest, replication-aware. The operator stores ciphertext and metadata they cannot interpret. Use cases: P3's archive, P4's audit log, KV cache durability across volunteer churn.
- **L4-relay.** Pure forwarding and shaping. Cover-traffic generators (I7), policy-bound relays, and — if/when adopted — an opt-in mixnet hop for message-class traffic. Holds no payload state across requests.

A single binary may implement more than one of these roles, but the roles are conceptually distinct, advertised separately at L3, and tested against separate threat models.

**L5 (Application).** Everything users touch, sub-structured into:

- **L5-runtime — the verified hyperdocument console.** The single user-facing surface through which all L5-apps are addressed, all L4 services are dispatched, and all attestation chrome is rendered. Identity management, plural-persona enforcement, session journaling, and the `caps://` resolver live here. See `06-browser.md`.
- **L5-app.** Specific workflows: messaging, file drop, radio, intake forms, archive UIs, compute marketplace UI. Apps run *inside* the L5-runtime sandbox; they never reach the substrate, never see raw network sockets, and access L4 only through capabilities the runtime hands them.

Applications may never reach below L4. An L5-app that opens a raw socket has violated the design system. An L5-app that bypasses the runtime to reach L4 directly has violated the design system.

### Cross-layer rules

- **No layer may expose the presence or identity of a layer above it.** L2 does not know what service is at the other end of a circuit. L3 does not know which applications care about a given descriptor.
- **Layers communicate via narrow, typed interfaces.** A new transport at L1 should require zero changes above L2. A new application at L5 should require zero changes below L4.
- **Layer crossings are authorization boundaries.** Anything crossing from L5 down is untrusted input from the perspective of lower layers and is validated accordingly.

---

## 3. Cross-cutting invariants

Rules that apply to all layers and all components. These are the ones a reviewer checks on every PR.

### I1. No ambient authority
No code anywhere in Ultranet may act on the identity of "the current user" or "this machine" without an explicit capability being passed in. Capabilities are per-operation, time-bounded, and unforgeable.

### I2. No plaintext identifiers on disk
Peer identities, onion keys, routing state, and application state are all encrypted at rest with keys held in a TPM (or equivalent enclave). Wiping the TPM wipes the node.

### I3. No outbound connection that isn't an onion circuit
If a component at L2 or above makes a network call that is not through an onion circuit — including DNS, NTP, telemetry, crash reporting, or update checks — it is a bug. Time sync and software updates have Ultranet-native designs (see §5).

### I4. No dependency on a central service
There is no "Ultranet server." There is no "official directory." There is no "default relay." Components that need bootstrap information (e.g., an initial peer list) embed it at compile time and treat it as a hint, not a requirement.

### I5. Deterministic, reproducible builds
A release binary can be reproduced bit-for-bit from the source tree by any third party. Non-reproducibility is a release blocker, not a nice-to-have — because supply-chain attacks are in the threat model.

### I6. Fail closed
If a component cannot verify that it is operating under Ultranet invariants (e.g., onion circuit failed to establish, attestation failed, TPM is not present), it refuses to operate. It does not fall back to a "degraded but functional" mode.

### I7. Cover traffic is a feature, not a decoration
Nodes that are online generate traffic at a constant rate, regardless of user activity. Silence leaks. This has power and bandwidth costs, and we accept them.

---

## 4. Design tensions we explicitly accept

Every real system has tensions. Pretending otherwise is how architectures rot. These are the tradeoffs Ultranet is *choosing*, eyes open.

| Tension | Choice | Cost we accept |
|---|---|---|
| Latency vs. anonymity set size | Bias toward larger anonymity sets | Higher end-to-end latency than clearnet (seconds, not milliseconds, for many operations) |
| Usability vs. no-clearnet-mode | No clearnet mode, ever | Users cannot trivially link an Ultranet identity to an existing email/phone/handle; onboarding is harder |
| Attestation hardware lock-in vs. trustless compute guarantees | Require AMD SEV-SNP (or equivalent) for Track 2 hardware | Track 2 nodes cost more; Track 1 nodes can participate without hardware attestation at reduced guarantees, clearly labeled |
| Cover traffic cost vs. traffic analysis resistance | Always-on cover traffic | Continuous bandwidth and power draw; not viable on metered mobile data |
| Feature velocity vs. invariant stability | Invariants win | Many "obvious" features (clearnet bridge, email gateway, web-based UI on public DNS) are permanently off the table |
| Decentralization vs. coordinated upgrades | Decentralized, with embedded bootstrap hints | Protocol upgrades are slow and require long deprecation windows |

---

## 5. Open design questions

Things the design system does *not* yet answer. Each one is a future design doc.

- **Time synchronization without NTP.** How do nodes agree on "now" without querying a public time source that would leak participation?
- **Software updates without phoning home.** How does a node learn that a new signed release exists, and fetch it, without establishing a fingerprintable relationship with an update server?
- **Bootstrapping from zero peers.** A fresh install needs at least one peer to talk to. How is that first peer introduced, and how is that channel itself protected?
- **Key rotation and recovery.** What does it mean to "lose your keys" on a system that has no account recovery by design? How much of the answer is tooling, how much is user education?
- **Plural identity and selective disclosure.** P1, P3, P4 all need multiple personas (work vs. personal, source-handler vs. outlet-facing) that cannot be linked by an observer or by a colluding peer. What primitives — anonymous credentials, BBS+, hierarchical pseudonyms — does the L5-runtime expose, and how are they enforced? Future doc: `08-identity.md`.
- **Abuse and moderation.** A trustless compute marketplace without moderation will attract workloads nobody wants to run. What is the minimum viable abuse story that does not compromise the invariants above?
- **Sybil resistance at L3.** BFT gossip among bootstrap nodes works until it doesn't. What is the long-term Sybil story, and how close is it to requiring a token (which we are committed not to ship)?
- **Sustainability without a token.** Volunteer ideology runs out around a few hundred nodes. What is the funding and operator-incentive model that does not become a token? Future doc: `09-economics.md`.

---

## 6. How to use this document

**When designing:** before writing a proposal, re-read §§1–3. If your proposal violates an axiom, an invariant, or a layer rule, the proposal is wrong — not the document. If you believe the document is wrong, the PR is to *this file*, and it requires its own justification, threat model impact analysis, and explicit acknowledgment of what the change invalidates elsewhere.

**When reviewing:** the first review question is not "does this work?" It is "does this preserve the invariants?" A PR that works but violates I3 or I6 is rejected without further review.

**When stuck:** the tensions in §4 are the negotiable surface. The axioms in §1 and invariants in §3 are not. If a problem seems unsolvable without violating an invariant, the problem is usually mis-scoped; restate it.

---

*This document is the most important non-code artifact in the repository. It changes slowly and deliberately. Changes to §§1, 3 are version-bumping events for the project as a whole.*
