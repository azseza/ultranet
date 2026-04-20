# Prior Art

**Status:** Draft v0.1 — 2026-04-20.
**Purpose:** Survey the projects that occupy adjacent territory to Ultranet. For each: what it does well, where it falls short for Ultranet's target audience, and what Ultranet inherits from it vs. diverges from it. This document exists so that knowledgeable readers can see we have done the homework, and so that contributors know which wheels we are not reinventing.

The survey is deliberately critical — not because these projects are bad (they are mostly excellent), but because the gaps are the justification for Ultranet's existence.

---

## Tor

**What it is.** The original low-latency anonymity network. Onion-routed circuits over TLS, a published directory of volunteer relays, and hidden services (`.onion` addresses) that accept inbound connections without revealing their location.

**What it does well.**
- Proven deployment scale (thousands of relays, millions of users).
- `arti` — the modern Rust rewrite — is the most serious anonymous-network codebase in existence.
- Hidden services are the correct primitive for rendezvous without IP exposure.

**Where it falls short for our audience.**
- **Directory-authority trust.** A small, static set of directory authorities signs the consensus. If they are compromised or coerced, the network's routing is compromised. Ultranet does not inherit this trust root.
- **Does not defend against a global passive adversary.** Tor is explicit about this. For the journalist-vs-signals-intelligence threat model, bare Tor is insufficient.
- **Exit-node ecosystem is operationally fragile.** Run by volunteers under legal exposure in many jurisdictions. For Ultranet, this is a non-problem because we do not ship a clearnet exit as a core feature.
- **No trustless compute story.** Tor carries bytes; it does not execute workloads.

**What Ultranet inherits.**
- Onion-routed circuits (via `arti` as a library).
- Hidden-service rendezvous descriptors.
- The general shape of the threat model documentation.

**What Ultranet diverges on.**
- Directory authorities are replaced by a BFT gossip layer among bootstrap peers (L3), themselves reachable only via L2.
- Trustless compute is first-class (L4), not an afterthought.
- No clearnet exit. No "browse the web anonymously" use case.

---

## I2P

**What it is.** A peer-to-peer anonymous overlay, garlic-routed (multi-message bundles per encrypted envelope), with a strong bias toward in-network services rather than clearnet exit.

**What it does well.**
- Garlic routing is a real traffic-analysis defense we should study and likely adopt at L2.
- Native bias toward in-network services matches Ultranet's philosophy better than Tor's clearnet-exit focus.
- Fully decentralized netDB; no directory authorities.

**Where it falls short for our audience.**
- Development velocity has slowed; the ecosystem is small.
- UX for non-technical users is rough enough that our personas would not adopt it unassisted.
- No hardware-attested compute layer.

**What Ultranet inherits.**
- The conceptual shape of garlic bundling for cover traffic.
- The "services in-network, clearnet is the exception" philosophy.

**What Ultranet diverges on.**
- Rust implementation (I2P's canonical implementation is Java; `i2pd` is C++) — aligns with `arti`.
- Design system is explicit and enforced; I2P's equivalent is implicit in the code.

---

## Nym

**What it is.** A mixnet — a network that adds latency and reorders packets to defeat traffic analysis — with a cryptocurrency-based incentive layer (`NYM` token) for mix-node operators.

**What it does well.**
- Mixnet design is the strongest available defense against the global passive adversary. If we ever add a mix layer, this is the reference.
- Loopix-family research grounding is serious academic work.

**Where it falls short for our audience.**
- Token-gated operator economics. Ultranet is committed not to ship a token.
- Latency cost of mixing is high enough that interactive use cases (chat, shell) are uncomfortable. Nym's target is message-oriented workflows.
- Operationally young; production track record is thin compared to Tor.

**What Ultranet inherits.**
- Design vocabulary for cover traffic, Sphinx packet format, and loop messages.
- A clear-eyed sense of what mixing costs in latency.

**What Ultranet diverges on.**
- No token, no staking, no on-chain accounting.
- Mixnet-style defenses, if adopted, are opt-in for message-class traffic, not blanket.

---

## Session / Oxen

**What it is.** An end-to-end encrypted messenger built on the Oxen Service Node network (staked nodes, Monero-adjacent). Routes via onion circuits over Service Nodes; no phone-number identity.

**What it does well.**
- Proof that a production messenger can ship on an anonymous overlay without phone/email identifiers.
- Staked-node economics create a clearer operator accountability story than pure volunteer relays.

**Where it falls short for our audience.**
- Staking is cryptoeconomic — token-gated operator participation. Same disqualifier as Nym for our goals.
- Application is a messenger; the underlying overlay is not exposed as a general-purpose infrastructure layer that other applications can build on.

**What Ultranet inherits.**
- The operational lesson that a small, reachable set of "service nodes" can work as an L3 bootstrap layer.
- UX patterns for identifier-less onboarding.

**What Ultranet diverges on.**
- No staking, no token.
- L5 is a platform, not a single app.

---

## Veilid

**What it is.** An in-development framework from the Cult of the Dead Cow for building privacy-preserving applications. DHT-based rendezvous, route-based addressing, sealed private routes.

**What it does well.**
- Application-framework framing — treat the overlay as infrastructure, applications live above it — matches Ultranet's layer model.
- No token, no blockchain. Volunteer-operated.
- Active, reasonably transparent development.

**Where it falls short for our audience.**
- Young. Production track record is negligible as of this writing.
- Threat model is less formally documented than Tor or Nym.
- No trustless compute story.

**What Ultranet inherits.**
- The framework-not-app framing is a direct influence.
- DHT-over-anonymous-overlay pattern at L3.

**What Ultranet diverges on.**
- Hardware attestation and trustless compute at L4.
- Stricter enforcement of "dark by default" invariant.

---

## Yggdrasil

**What it is.** An experimental peer-to-peer IPv6 mesh overlay. Self-arranging, cryptographically-derived addresses, spanning-tree routing.

**What it does well.**
- Zero-configuration mesh is a real achievement.
- Cryptographic-address identity is a clean idea.
- Running code, modest scale, low friction.

**Where it falls short for our audience.**
- Not anonymous. Traffic analysis is trivial. Yggdrasil deliberately does not claim anonymity.
- No rendezvous-hiding; public-key-derived addresses are visible on the overlay.

**What Ultranet inherits.**
- Mesh-routing primitives are informative for the FSO long-term track (L1 substrate).

**What Ultranet diverges on.**
- Yggdrasil solves connectivity; Ultranet solves confidentiality. Different problems.

---

## Urbit

**What it is.** A reimagining of the personal computing stack as a deterministic, portable, cryptographically-addressed computer with a P2P networking layer (Ames).

**What it does well.**
- Cryptographic identity as a first-class primitive (`@p`) rather than bolted-on.
- "Your personal server" as a model is culturally aligned with sovereignty.

**Where it falls short for our audience.**
- Identity is effectively a scarce, tradable asset (star/planet hierarchy). This introduces a commercial/political layer Ultranet explicitly avoids.
- Not designed for anonymity. Identity persistence and traceability are features, not bugs.
- Stack complexity is high.

**What Ultranet inherits.**
- The philosophical framing that your node *is* your identity.

**What Ultranet diverges on.**
- Identity is a keypair, not a scarce resource. No hierarchy, no issuance.
- Anonymity is a first-order property, not explicitly out of scope.

---

## Phala Network

**What it is.** A blockchain-orchestrated trustless-compute network using Intel SGX (and recently SEV-SNP) enclaves. Workloads run on attested enclaves; results are verifiable.

**What it does well.**
- Production experience with hardware-attested confidential compute at meaningful scale.
- The attestation pipeline — from vendor-signed quote to on-chain verifier — is a mature reference.

**Where it falls short for our audience.**
- Blockchain-orchestrated. Workload scheduling, payment, and attestation verification run on a public chain.
- Not anonymous at the network layer. Workload submission and result retrieval go over clearnet.
- SGX availability has contracted; SEV-SNP is the right bet going forward.

**What Ultranet inherits.**
- The attestation flow for hardware-confidential compute (SEV-SNP remote attestation) at L4.
- The threat-model vocabulary for "the operator holds the machine but cannot observe the workload."

**What Ultranet diverges on.**
- No chain. Workload submission is routed over L2/L3 anonymously.
- No token.
- Attestation is a peer-to-peer verification, not an on-chain one.

---

## Freenet / Hyphanet

**What it is.** A long-running anonymous peer-to-peer content-distribution network optimized for censorship-resistant publishing. Datastore model, keys resolve to content.

**What it does well.**
- Demonstrated longevity (20+ years of continuous operation).
- Strong conceptual framing of plausible deniability for node operators.

**Where it falls short for our audience.**
- Store-and-forward, content-addressed model. Does not support low-latency interactive workflows.
- No rendezvous/compute primitives.

**What Ultranet inherits.**
- Plausible-deniability design vocabulary.
- "The operator cannot easily tell what they are hosting" is an inspiration for L4's blind-compute guarantee.

**What Ultranet diverges on.**
- Interactive, not content-only.
- L4 compute is a first-class primitive.

---

## Summary: the gap Ultranet fills

No project in this list combines all of the following in a single coherent system:

1. **Anonymous transport** with low-enough latency for interactive workflows (Tor-class).
2. **Rendezvous without directory-authority trust** (neither Tor nor Phala satisfies this alone).
3. **Hardware-attested trustless compute** available as a first-class service over the anonymous overlay (Phala has the compute, without the overlay; Tor has the overlay, without the compute).
4. **No token, no chain, no staking.** Economic incentives, if needed, are deferred deliberately.
5. **Dark by default, enforced by design system invariants**, not by user configuration.

Each property exists somewhere. Their combination does not. That combination is what this project is.

---

*When a new entrant appears in this space, it goes in this document. If it eats part of our reason-to-exist, we say so honestly and adjust.*
