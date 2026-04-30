# Threat Model

**Status:** Draft v0.2 — 2026-04-30.
**Purpose:** Name the adversaries Ultranet is built to defeat, map each adversary class to the specific defenses that address it, and honestly mark the attacks we do not yet defend against. This document is checked against the design system (`05-design-system.md`) whenever either one changes.

**Honesty rule.** If an attack has no mapped defense, it is listed as *unsolved* — not *out of scope*. "Out of scope" is a phrase used in this document only for attacks that the project is deliberately not in the business of stopping (e.g., a user voluntarily disclosing their own identity).

---

## 1. Assets we protect

Before adversaries, the assets. An attack is only meaningful in reference to an asset it compromises.

| # | Asset | Why it matters |
|---|---|---|
| A1 | **Peer identity** | Linking a node to a real-world person or organization unmasks the user behind it. |
| A2 | **Peer-to-peer relationships (the social graph)** | Who talks to whom is often as incriminating as what they said. |
| A3 | **Payload contents** | The actual messages, files, and workload inputs/outputs. |
| A4 | **Workload contents at L4** | What a blind-compute node is executing, even though it holds the machine. |
| A5 | **Historical activity of a node** | What a seized or compelled node did in the past. |
| A6 | **Participation signal** | The fact that a given real-world person or machine is using Ultranet at all. |
| A7 | **Protocol correctness** | The property that Ultranet's routing, attestation, and rendezvous are not silently subverted. |

---

## 2. Adversary classes

We name adversaries so that every defense can be traced to a concrete threat.

### T1. Local passive observer
**Capabilities.** Can observe traffic on a single link adjacent to one Ultranet node. Cannot correlate across links. Legal/technical access to one ISP, one coffee-shop router, one airport wifi.
**Example.** A coworker running Wireshark on the office subnet.

### T2. Local active attacker
**Capabilities.** T1 plus can inject, modify, drop, or delay packets on that one link. May run a malicious wifi AP or compromised home router.
**Example.** An abusive partner with router admin.

### T3. Remote network attacker
**Capabilities.** Can mount attacks against a node's exposed network surface from anywhere on the internet. Cannot observe the node's physical location or local link.
**Example.** A commercial spyware vendor running exploit campaigns.

### T4. Compelled service provider
**Capabilities.** Can compel one or more intermediate parties — hosting providers, cloud vendors, CDNs, DNS registrars, root CAs — to hand over data, modify service, or plant surveillance.
**Example.** National security letter, foreign intelligence service operating with cooperation of a hosting company in its jurisdiction.

### T5. Global passive adversary (GPA)
**Capabilities.** Can observe traffic at many points across the internet backbone simultaneously. Can run traffic-correlation analysis at scale. Cannot actively modify most traffic.
**Example.** Signals-intelligence agencies.

### T6. Global active adversary
**Capabilities.** T5 plus the ability to drop, delay, modify, or inject on major links and at major exchange points.
**Example.** A sufficiently resourced state-level adversary during a targeted operation.

### T7. Physical node attacker
**Capabilities.** Has physical custody of a node. Can remove storage, decap chips, perform side-channel analysis (power, EM, timing), attempt cold-boot attacks.
**Example.** Device seized at a border crossing; subpoenaed colocated hardware.

### T8. Malicious node operator (the Sybil case)
**Capabilities.** Runs one or many Ultranet nodes, participates in protocols honestly at the wire, but attempts to abuse position as a relay, rendezvous node, or compute provider. May collude with other operators.
**Example.** State-operated fleet of "volunteer" nodes joining the overlay to map the peer graph.

### T9. Supply-chain adversary
**Capabilities.** Compromises the development, build, or distribution pipeline. Can ship signed releases with backdoors, plant dependencies, or subvert the toolchain.
**Example.** A coerced maintainer, a compromised dependency in `crates.io`, a poisoned build agent.

### T10. Social-engineering adversary
**Capabilities.** Attacks the human operator of a node or the human end-user of an application built on Ultranet. Phishing, coercion, interrogation, deception.
**Example.** Classic rubber-hose cryptanalysis.

### T11. Endpoint-compromise adversary
**Capabilities.** Has full code execution on the user's device outside the Ultranet sandbox — a compromised OS, a malicious driver, a keylogger installed physically.
**Example.** Pegasus, Predator, state-deployed implants.

---

## 3. Defenses and their mappings

The left column is a defense. The center column names the adversary classes it addresses. The right column names the design axioms or invariants it derives from.

| Defense | Adversaries addressed | Derivation |
|---|---|---|
| **Onion-routed circuits (L2)** | T1, T2, T3, T4 (partially), T5 (partially) | A2 (metadata is content), I3 (no outbound that isn't onion) |
| **Hidden-service rendezvous (L3)** | T3, T4 | A1 (well-resourced adversary), I4 (no central service) |
| **No DNS, no directory authorities** | T4 | A4 (trust minimized), I4 |
| **Traffic indistinguishable from noise on the wire** | T1, T2 | A3 (convenience = enemy), I6 (fail closed) |
| **Constant-rate cover traffic** | T5 (partial), T6 (partial) | I7 (cover traffic is a feature) |
| **TPM-backed keys, wipe-on-intrusion** | T7 | A5 (seized node leaks nothing), I2 (no plaintext identifiers on disk) |
| **No plaintext state on disk** | T7 | A5, I2 |
| **Hardware-attested enclave execution (SEV-SNP) at L4** | T7 (against the node operator), T8 | A4 (trust minimized), requirement for A4 (workload confidentiality) |
| **BFT gossip for bootstrap, no single directory** | T4, T8 (partial) | A4, I4 |
| **Reproducible builds** | T9 | I5 (reproducible builds) |
| **Signed releases with multi-party signature threshold** | T9 | I5; derives from A4 |
| **Minimized attack surface (no open ports but the onion service)** | T3 | I6 (fail closed) |
| **Sandboxed application execution (L5 cannot reach below L4)** | T11 (limit blast radius) | Layer invariants, §2 of design system |
| **Ambient-authority-free API design** | T11 (limit blast radius) | I1 (no ambient authority) |

---

## 4. Attack-by-attack walkthrough

The threats above, run against concrete attacks and their mapped defenses.

### 4.1 Traffic capture on the local link (T1, T2)
- **Attack.** Wireshark on the LAN tries to identify Ultranet traffic and extract anything useful.
- **Defense.** L1 carries only ciphertext. Packet lengths, timing, and rates are shaped to an indistinguishable-from-noise profile (I7). The initial handshake is padded so that even the *presence* of an Ultranet flow is ambiguous.
- **Residual risk.** Low. If the adversary already suspects a user of running Ultranet, confirmation is harder than refutation, but not impossible with long-term observation of the link.

### 4.2 Remote exploitation of the node (T3)
- **Attack.** Commercial spyware campaign attempts to reach the node over the network.
- **Defense.** The only network-reachable surface is the onion service. To reach it, the adversary must know the onion address and complete a cryptographic handshake. There is no port to scan, no service banner, no listening HTTP.
- **Residual risk.** A zero-day in `arti` or the handshake code itself. Mitigated by keeping the exposed code small and auditable; accepted as a known-cost of having any network presence at all.

### 4.3 Hosting provider compulsion (T4)
- **Attack.** Legal process compels a cloud provider to hand over a disk image of a node running in their infrastructure.
- **Defense.** Keys are in the TPM (I2). Storage is encrypted at rest. The seized image yields: the ciphertext of past state, and the fact that Ultranet was present. It does not yield peer identities, the social graph, or workload contents.
- **Residual risk.** If the adversary seized the running machine, including RAM, and the TPM was still unlocked, some in-flight material could be recovered. Mitigation is to use hardware with memory encryption (SEV-SNP) for any node where T4 is in scope.

### 4.4 Global-passive traffic correlation (T5)
- **Attack.** Signals-intelligence agency observes both endpoints of a circuit at the backbone level and correlates timing to de-anonymize.
- **Defense.** **Partial.** Constant-rate cover traffic (I7) raises the cost substantially but does not fully defeat long-term traffic correlation, which is the current state of the art across Tor and most of this space. For message-class traffic, an optional mixnet layer (Nym-inspired) can be enabled with a latency cost.
- **Residual risk.** **This is the most significant unsolved attack in Ultranet's threat model.** We say so plainly. We do not market the system as defending fully against T5 for interactive traffic. The roadmap for mix-layer integration is in `05-design-system.md` §5.

### 4.5 Global active tampering (T6)
- **Attack.** State adversary selectively drops or delays Ultranet-identified traffic at backbone level, degrading service until users switch to insecure alternatives.
- **Defense.** **Partial.** Traffic indistinguishability (I6, I7) makes targeted blocking hard; the adversary has to block too much collateral to isolate Ultranet. But a determined adversary controlling enough of the path can still degrade the network.
- **Residual risk.** Availability under active network suppression is *unsolved*. FSO mesh (long-horizon research track) is the architectural response, with all the caveats in the manifesto.

### 4.6 Physical seizure (T7)
- **Attack.** Node is physically taken and subjected to forensic analysis.
- **Defense.** TPM with chassis-intrusion wipe. No plaintext state on disk. SEV-SNP memory encryption for workloads at L4. The adversary learns: the node participated in Ultranet, and (if the chassis was opened correctly) nothing else.
- **Residual risk.** Side-channel attacks on the TPM itself, chip decapping, or sophisticated cold-boot attacks before the intrusion detector fires. Mitigation is hardware selection at the Track 2 stage; Track 1 software-only deployments inherit whatever the host hardware provides.

### 4.7 Sybil overlay (T8)
- **Attack.** Adversary stands up thousands of well-behaved-seeming Ultranet nodes to learn the peer graph, observe rendezvous patterns, and fingerprint users.
- **Defense.** **Partial.** Because L3 rendezvous descriptors are published to a DHT that itself runs over L2, a malicious node learns only the local slice of the DHT it is responsible for. Global graph reconstruction requires substantial Sybil presence. BFT gossip at the bootstrap layer limits influence by any one operator.
- **Residual risk.** Sybil resistance without a token economy is *partially unsolved*. We accept this tradeoff deliberately (manifesto: no token). Mitigation research is tracked in design system §5.

### 4.8 Supply-chain compromise (T9)
- **Attack.** A backdoored release is signed and shipped.
- **Defense.** Reproducible builds (I5). Multi-party release signing (a threshold of maintainer signatures required for a release to verify). Dependency audits; pinned and mirrored sources where feasible; `cargo-vet` or equivalent discipline on the Rust crate graph.
- **Residual risk.** A coordinated compromise of a threshold of maintainers. This is the hardest attack to defend against at the project-governance level; partial mitigation is governance transparency and a public changelog that any user can diff against.

### 4.9 Social engineering (T10)
- **Attack.** The user is coerced or deceived into revealing their identity or handing over keys.
- **Defense.** **None at the protocol layer.** This is explicitly out of scope for Ultranet-the-protocol and in scope for Ultranet-the-documentation: we write operational-security guidance for the personas in `03-personas.md`, and applications built on Ultranet should support features like decoy accounts and duress passwords. The protocol cannot defend against a user voluntarily compromising themselves.

### 4.10 Endpoint compromise (T11)
- **Attack.** Pegasus-class implant runs on the user's device outside any Ultranet sandbox.
- **Defense.** **Limited.** Ultranet protects the network and compute layers, not the endpoint. Invariant I1 (no ambient authority) limits blast radius *within* Ultranet code. Layer isolation (§2) limits what a compromised L5 application can reach. But if the OS is hostile, Ultranet cannot save you — and we say so, because pretending otherwise would mislead users into false confidence.
- **Residual risk.** The endpoint is the weakest link. Track 2 (sealed appliances) exists partly to address this: a single-purpose device with no userland has a radically smaller attack surface than a general-purpose laptop.

---

## 5. Explicitly out of scope

These are not threats we intend to solve. Listed here so that contributors do not mistake silence for oversight.

- **A user voluntarily disclosing their own identity.** No system can stop this.
- **Legal risk to the *user*** arising from their activity on the network. Jurisdictions differ; legal counsel is not a protocol feature.
- **Abuse and content moderation at the protocol layer.** Applications at L5 may implement policy; the protocol does not. This is a contested design decision; see design system §5.
- **Replacing the user's operating system or hardware.** Ultranet runs on what the user has. Hardening the broader stack is the user's responsibility (or, in Track 2, the project's — but only on purpose-built appliances).

---

## 6. The unsolved research portfolio

This section is not a disclaimer. It is the list of *open research problems Ultranet treats as part of its mandate*. Each item below is an attack we cannot currently defend against, and a track of work that — when it produces a defense — moves the item up into §4. Honesty about the unsolved (axiom A6) means more than admitting the gaps; it means owning the research that closes them.

### 6.1 Global passive adversary traffic correlation (interactive traffic)
**State of the art.** Tor, I2P, Veilid all share this gap. Mixnet-class systems (Nym, Loopix) close it at a latency cost that rules out interactive workflows.
**Ultranet partial defense.** Constant-rate cover traffic (I7) raises the cost; an opt-in mix layer at L4-relay is on the roadmap for message-class traffic.
**Open questions we own.** Can a per-circuit traffic-shaping policy at L4-relay defeat correlation for short-burst interactive traffic without paying full mixnet latency? What is the latency/anonymity-set Pareto frontier for the persona set in `03-personas.md`?

### 6.2 Sustained global active suppression
**State of the art.** No deployed network survives a sufficiently resourced state-level suppression campaign at the IP layer. Tor's bridge ecosystem partially mitigates censorship but not bandwidth-class active interference.
**Ultranet partial defense.** Wire indistinguishability (I6) raises the collateral cost of blocking. Substrate replaceability (A7) provides a structural escape: when the public internet is hostile, Ultranet can move to FSO mesh, ham-radio digital modes, or sneakernet with no architectural change above L2.
**Open questions we own.** What is the operational hand-off between substrates — automatic, manual, or graceful-degradation? How does the L5-runtime represent "we are now operating on a degraded substrate" to users without breaking I6?

### 6.3 Sybil resistance without a token economy
**State of the art.** Every production overlay either tolerates Sybil (Tor) or token-gates participation (Nym, Session). No deployed system has solved this without economic stakes.
**Ultranet partial defense.** BFT gossip among bootstrap peers limits any single operator's influence. L3 descriptor schema can carry capability *types* but not *reputation*.
**Open questions we own.** Can social-graph-bound credentials (web-of-trust signatures from existing peers) provide Sybil resistance for sensitive capabilities (intake, archive replication) without becoming a deanonymization vector? Is there a viable "proof of legitimate participation" that is not a token?

### 6.4 Endpoint compromise
**State of the art.** Pegasus-class implants defeat any application-layer privacy measure. Track 2 (sealed appliances) is the structural answer.
**Ultranet partial defense.** No-ambient-authority (I1), capability-language sandboxing in L5-runtime, layer isolation prevents L5-app compromise from reaching L4.
**Open questions we own.** What is the minimal Track 2 hardware specification that meaningfully resists T11? Can the L5-runtime detect tampering with the host OS (e.g., via TPM measured boot) and fail closed (I6)?

### 6.5 Coordinated supply-chain compromise
**State of the art.** SolarWinds, xz, event-stream — supply-chain attacks above maintainer thresholds are an unsolved class for the entire software industry.
**Ultranet partial defense.** Reproducible builds (I5), multi-party release signing, dependency vetting.
**Open questions we own.** What is the threshold? How is the threshold *publicly verifiable* by users without a central trust anchor? Is there a tractable "diverse-double-compilation" workflow for an `arti`-sized dependency graph?

### 6.6 Plural identity unlinkability across colluding peers
**State of the art.** Anonymous credentials (BBS+, IRMA, recent BBS-23) provide selective disclosure but lack ergonomics for end users.
**Ultranet partial defense.** None yet — plural identity is a vision-doc commitment, not an implemented primitive.
**Open questions we own.** How does the L5-runtime expose persona-switching such that a user cannot accidentally cross-link two personas? Can the threat model rule out *all* observer-side correlation, or only most?

---

These are not "things we don't do." They are the project's research surface. When an item closes, it moves into §4 with the defense documented. When an item proves intractable, we say so explicitly and revise the manifesto's claims accordingly.
