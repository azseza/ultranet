# Ultranet

**Infrastructure for work that depends on confidentiality.**

---

## The problem

There are professions where confidentiality is not a preference — it is a condition of the work itself.

A journalist talking to a source inside a ministry. A lawyer communicating privileged material with a client across a border. A doctor handling patient records in a jurisdiction where the state is not a neutral party. A researcher sharing unpublished findings with a collaborator in a country where academic speech is policed. A survivor coordinating with a shelter. An auditor inside a company that does not want to be audited.

The modern internet does not protect any of them. Every layer — DNS, BGP, TLS SNI, IP addressing, CDN telemetry, device fingerprinting, metadata logs at the carrier — leaks enough signal that a motivated adversary with ordinary resources can reconstruct who spoke to whom, when, and about what. "End-to-end encrypted" has become a marketing claim that survives the encrypted payload, and loses the metadata.

The people who most need the internet to work as advertised are the ones for whom it most visibly does not.

## What Ultranet is

Ultranet is a network in which:

1. **Identity is a cryptographic rendezvous, not a geographic address.** There is no IP to log, no DNS name to subpoena, no CDN to lean on.
2. **Compute can be delegated without trust.** You can run workloads on hardware you do not own, in a way where the hardware operator cannot see what you are computing — only that cycles are being spent.
3. **There is no "clearnet mode."** The only way to use Ultranet is the private way. Convenience escape hatches are how privacy systems die, and Ultranet does not have one.
4. **The network has no center.** No Tier-1 ISP. No DNS root. No BGP announcements. No single authority whose compromise collapses the system.

## Who this is for

Ultranet is explicitly not aimed at "everyone who values privacy." It is aimed at people whose work depends on it, and who currently have no infrastructure that isn't compromised by default. The initial target audience is small and concrete:

- Investigative journalists coordinating with sources inside institutions.
- Lawyers handling privileged cross-border matters.
- Clinicians in jurisdictions with coercive health-data regimes.
- Human-rights researchers and opposition-adjacent political actors.
- Whistleblower intake workflows run by NGOs and newsrooms.

These are the personas that drive design decisions. See `03-personas.md` for the concrete user stories each release is checked against.

## Non-negotiable principles

These are the constraints Ultranet does not relax. Every design decision is checked against them.

### 1. Sovereign topology
The network has no center. No Tier-1 ISP. No DNS root. No BGP hijacking surface. Discovery is cryptographically-assured rendezvous, not geographic broadcast.

### 2. Dark by default
There is no "clearnet mode." Ultranet is the only mode. Traffic on the wire is indistinguishable from noise to any party without the precise cryptographic handshake.

### 3. Trustless compute
You can share your hardware's cycles without sharing your hardware's memory. A node operator cannot observe the computation running on their machine, even though the machine is physically theirs.

### 4. No plaintext user data at rest on a node
A node that is seized, searched, or compelled contains nothing that can identify its users, its peers, or the workloads it ran. Key material lives in a TPM or equivalent enclave that zeroes on chassis intrusion.

### 5. Adversary honesty
The threat model is documented, named, and honest about what is unsolved. "We don't defend against X yet" beats "X isn't a realistic attack."

## What Ultranet is not

- **Not a cryptocurrency.** There is no token. There is no chain. Economic incentives for running nodes, if they become necessary, will be added deliberately and late, not as a foundational feature.
- **Not Tor with extra steps.** Ultranet inherits a lot from Tor — onion-routed circuits, hidden-service rendezvous — but adds trustless compute delegation and removes the dependency on directory-authority trust. Prior art, inheritance, and divergence are detailed in `01-prior-art.md`.
- **Not a consumer chat app.** Ultranet is infrastructure. Applications (messaging, file drop, compute marketplace) are built on top of it.
- **Not an anti-state project.** It is an anti-surveillance project. The distinction matters: the goal is confidentiality for legitimate work, not evasion for its own sake.

## The path

Ultranet is decomposed into three tracks, on very different timelines.

### Track 1 — Software fortress (shippable, months to a year+)
A pure-software network built on Arti (Rust Tor), libp2p, and a Firecracker-based blind-execution sandbox. Two nodes find each other without IP addresses, delegate work to each other without exposing memory, and reconstitute the connection if any intermediate hop is lost.

### Track 2 — Sovereign nodes (R&D, 2–3 years)
Hardware appliances — small, sealed, low-power — that run Track 1 as their only workload. Tamper-evident enclosures, TPM-backed key material, no user-accessible shell. The appliance is single-purpose: it participates in Ultranet, and nothing else.

### Track 3 — Free-space optical mesh (long-horizon research, open-ended)
The vision of geographically-ambiguous mobile nodes communicating via steered infrared lasers is real and motivating, but it is a research program, not a roadmap milestone. It lives in this document to mark the direction; it does not block Track 1 or Track 2.

## What we build first

The next concrete deliverables are all writing, not code:

1. Prior art survey — what Tor, I2P, Nym, Session, Veilid, Yggdrasil, Urbit, Phala, and Freenet do, where they fall short for this audience, and what Ultranet inherits vs. adds.
2. Threat model — named adversaries, mapped defenses, an honest "unsolved" column.
3. Personas — the 3–5 concrete users who drive design decisions.
4. Differentiator statement — the one paragraph that explains what Ultranet does that `Tor + Signal + Phala` cannot.
5. Design system — the architectural invariants, layer boundaries, and cross-cutting rules that every module must respect. (See `05-design-system.md`.)

Only after these are written do we write the first line of Rust.

---

*The original manifesto, which framed Ultranet primarily around kinetic-warfare resistance, is preserved as `appendix-a-original-manifesto.md`. The framing has been widened because the underlying architecture serves a much broader population than the kinetic frame captures, and because the kinetic frame was narrowing the project's credibility and audience. The technical principles are unchanged.*
