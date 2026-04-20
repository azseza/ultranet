# Personas

**Status:** Draft v0.1 — 2026-04-20.
**Purpose:** Concrete users whose needs drive design decisions. When a design question comes up, the answer is: *does this make life better or worse for the five people below?* If a proposal does not help any of them, or actively harms any of them, that is a strong signal to reject the proposal.

These are not market-research archetypes. They are representative cases drawn from the real populations Ultranet is for. Every persona below maps to a real profession, a real threat, and real workflows that currently happen on infrastructure that does not protect them.

---

## P1. Leyla — the investigative journalist

**Location.** Istanbul, Turkey.
**Role.** Senior investigative reporter at an independent outlet. Works on accountability stories involving ministries, state-linked contractors, and organized crime.
**Technical literacy.** Intermediate. She uses Signal fluently, knows what a VPN is, can follow a GPG tutorial with help. She is not a security engineer.

### What she needs
- Talk to a source inside a ministry without the source's employer learning the source talked to a journalist.
- Receive leaked documents (PDFs, spreadsheets, recordings) that cannot be traced to their origin through metadata in the files.
- Collaborate with two colleagues abroad on a story without the collaboration itself being visible to anyone.
- Keep working when her outlet's domain is blocked and her phone number is added to a state watchlist.

### What she fears
- Her device is confiscated at the airport.
- Her source is identified by the pattern of Leyla's network activity, not by what she said.
- A "secure" tool she uses has a clearnet fallback that she didn't know about.
- A tool goes out of business, or complies with a takedown request, and her archive becomes evidence.

### What Ultranet provides her
- No domain to block. No phone number to watchlist. Her Ultranet identity is a keypair she controls.
- Her conversations with a source are onion-routed; the source's location and employer are hidden from her, and hers from them.
- "Dark by default" means there is no fallback path that can be tricked into exposing her. She cannot accidentally send plaintext.
- If her device is seized, the TPM wipes on intrusion and storage is encrypted at rest.

### What Ultranet does *not* provide her
- Protection against her own device being already-compromised (T11). If Pegasus is on her phone before she installs Ultranet, Ultranet cannot help. We tell her this directly in the onboarding material.
- Protection against her source voluntarily identifying themselves in the content of a message. That is a training issue, not a protocol issue.

### What she signals for design
- The onboarding flow must work on a laptop inside a hostile network without phoning home to update servers.
- The UI cannot assume that she can safely link her Ultranet identity to her email.
- Recovery of a lost key cannot be "click here to reset" — there is nowhere safe to reset through.

---

## P2. Dr. Mathieu — the cross-border clinician

**Location.** French physician consulting on rare-disease cases with colleagues in countries where the patient's condition is stigmatized or politically sensitive.
**Role.** Senior specialist who participates in an informal international review network. The network shares anonymized case files for second opinions.
**Technical literacy.** Low. He uses whatever his institution provides. He trusts his hospital IT department less than his medical-board ethics.

### What he needs
- Share case files (images, labs) with named peers abroad, where the peer's jurisdiction treats the condition or the patient class as grounds for state interest.
- Know, with confidence, that the file he sends cannot be retrieved by the hosting infrastructure in either country.
- A workflow that does not require him to become a security engineer.

### What he fears
- The hospital IT department logs, or is compelled to log, his communications.
- A file sitting on a "cloud drive" — even an encrypted one — being seized at the provider's legal edge.
- Being the weakest link in his peer's operational security because he misconfigured a tool.

### What Ultranet provides him
- A peer-to-peer rendezvous that has no "cloud drive" in the middle. The file goes from his machine to his peer's machine through onion circuits; no hosting provider holds an encrypted-or-otherwise copy.
- An L5 application, built on the Ultranet platform, designed for exactly this workflow: drop a file on a named peer, receive confirmation, done.
- A design that treats his technical naïveté as a requirement, not a problem. Failure modes are fail-closed; there is no way for him to "accidentally" leak.

### What Ultranet does *not* provide him
- Anonymity from his named peers. He is *identified* to his colleagues; he is *unobservable* to everyone else. This is a deliberate design property: he needs verifiable peer identity, not pseudonymity.
- Legal cover. If law enforcement in his country compels him personally, no protocol can help.

### What he signals for design
- L5 applications must be usable by someone who has never heard of onion routing.
- Peer identity (P2P, verifiable) is a first-class concept distinct from anonymity (network, unobservable).
- The "named peer" concept is a core L5 primitive, not an application-specific hack.

---

## P3. Aisha — the human-rights researcher

**Location.** Nairobi, Kenya. Works on documentation of abuses in several African states.
**Role.** Research lead at a regional NGO. Compiles case files from field investigators, stores them, shares redacted versions with international partners.
**Technical literacy.** Intermediate-high. She manages keys, runs a local server, understands threat modeling well enough to participate in it.

### What she needs
- Intake channel for field investigators to submit reports with minimal technical requirements on the sender.
- Storage of case files in a form that survives seizure of her office.
- Selective sharing with named international partners, with no persistent cloud footprint.
- A workflow for high-volume storage and retrieval that doesn't require her to be online 24/7.

### What she fears
- Her office is raided; the server is taken; her years of archive are the evidence used against the people in it.
- A field investigator's submission is tied back to them through the metadata of the submission itself.
- One of her partners turns out to be compromised, and she did not know until case files started appearing in the wrong places.

### What Ultranet provides her
- Intake is a published rendezvous descriptor. Investigators reach the intake via onion circuit; the intake node does not learn the investigator's IP.
- The local server encrypts at rest; the TPM wipes on seizure; backup is to other Ultranet peers she trusts, also encrypted and authenticated.
- Named-peer primitives (inherited from P2) let her share with partners individually; compromise of one partner's keys does not expose her other partners.

### What Ultranet does *not* provide her
- Forensic reconstruction of who leaked if a partner is compromised. "Who had which file" is traceable to peers; beyond the peer boundary, there is no visibility. That is a feature for the investigator and a limitation for Aisha — and she understands the tradeoff.
- Anonymity *from* her partners. Again, named-peer identity is a feature here.

### What she signals for design
- L4 capabilities are not only "compute" — storage and intake workflows are also served here.
- Long-term archive durability is a real requirement; the system must support multi-peer replication with strong integrity guarantees.
- Operational tooling (monitoring, backup verification, peer-health checks) is a non-optional part of what a real user runs.

---

## P4. Jin — the privileged-communications lawyer

**Location.** Seoul-based international law partner handling cross-border disputes.
**Role.** Represents corporate clients in litigation that includes jurisdictions where the opposing party has state-adjacent leverage. Sends privileged material to co-counsel, experts, and clients.
**Technical literacy.** Low-intermediate. His firm has an IT security team; he mostly uses what they provide.

### What he needs
- Send privileged material to named collaborators abroad without the material transiting through US cloud providers or Chinese cloud providers or any cloud provider.
- A paper trail of *that the communication happened*, for firm compliance, without a paper trail of *what was in it* for anyone outside the privilege.
- Deniability for casual observation: his firm's IT should not be able to tell, from network traffic alone, which of his matters he is working on.

### What he fears
- A discovery motion in a foreign court that reaches an intermediary cloud provider.
- A leak that embarrasses a client because a jurisdiction he didn't expect could subpoena the data.
- Professional liability if a "secure messenger" turns out to have been less secure than marketed.

### What Ultranet provides him
- Direct peer-to-peer delivery of privileged material. No cloud. No intermediary. No jurisdiction to subpoena.
- Firm-compliant audit logs at the L5 application layer (his machine records what he sent to whom and when; no one else sees it).
- Indistinguishable-from-noise wire traffic: his network usage does not reveal which client he is working on, because all his network usage looks the same.

### What Ultranet does *not* provide him
- A legal determination that communications over Ultranet qualify for privilege in any given jurisdiction. That is a legal question, not a technical one; the protocol supports the technical predicates (direct delivery, no third-party access), and the legal characterization is up to his jurisdiction.

### What he signals for design
- Application-layer audit logs that satisfy compliance needs must be compatible with protocol-layer confidentiality. The compliance log lives on *his* machine, not on a shared service.
- The UI must communicate clearly what is and is not recorded, and where. Lawyers care about this in a way other professionals often do not.
- Wire traffic must not distinguish between "important matter" and "routine matter." Cover traffic (I7) is the mechanism.

---

## P5. Raza — the compute-delegating researcher

**Location.** Cambridge, MA. Computational biologist at an academic lab, collaborating with a lab in a country whose government would very much like access to the pipeline they are jointly developing.
**Role.** Runs machine-learning inference on genomic data. Some of that data is patient-identifiable; some of the models are the lab's IP.
**Technical literacy.** High. He writes Python, runs Kubernetes, knows what an enclave is.

### What he needs
- Run inference on a partner lab's hardware without the partner lab seeing either the model weights or the input data.
- Return the results to the partner lab without a central orchestrator learning what was computed.
- A way to verify that the remote execution actually ran inside an attested enclave and not in a logging-enabled shim.

### What he fears
- The partner lab is compromised, or its host institution is compelled, and model weights end up with a competitor or a state lab.
- A cloud-hosted "confidential compute" offering turns out to be less confidential than advertised, either by architecture or by legal process.
- His workflow is observable enough that the *fact* of the collaboration tips off the adversary before the results do.

### What Ultranet provides him
- L4 blind-compute: his workload runs inside an SEV-SNP-attested enclave on the partner lab's hardware. The partner lab sees cycles spent; it does not see weights or inputs.
- Attestation is peer-to-peer — Raza verifies the enclave quote directly, with no on-chain or centralized verifier to observe the transaction.
- Workload submission and result retrieval are routed over L2/L3 onion circuits; the collaboration is not visible at the network layer.

### What Ultranet does *not* provide him
- Defense against a side-channel in the SEV-SNP implementation itself. That is a hardware-vendor risk he inherits from choosing the platform; Ultranet documents the assumption but cannot close the gap.
- A guarantee that the partner lab will not run the attested enclave many times and infer the model through black-box querying. That is an ML-security problem, not a protocol problem.

### What he signals for design
- L4 attestation must be user-verifiable end-to-end. There is no central attestation service Raza trusts; the protocol must let him check the enclave quote himself.
- "Workload" is a more structured concept than a byte stream. It has inputs, an attested executable, outputs, and a measurement root.
- Network-layer unobservability of the L4 traffic is a requirement, not a bonus: the *existence* of the compute flow is adversary-relevant information.

---

## What these personas rule in and rule out

Collectively, the five personas shape the scope of Ultranet's first release.

**In scope (derived from personas):**
- Peer-to-peer messaging and file transfer with named-peer semantics (P1, P2, P3, P4).
- Published rendezvous services for intake workflows (P3).
- Blind-compute workload submission and attested execution (P5).
- Multi-peer encrypted archive with durability guarantees (P3).
- Compliance-compatible local audit logs at L5 (P4).
- Fail-closed behavior on every primitive (all five).

**Out of scope for v0.1 (no persona needs it yet):**
- General-purpose clearnet-style web browsing. No persona above needs it; all five are better served by in-network services.
- Anonymous publishing to a public audience. Freenet-like broadcast is a real use case but not represented in this persona set. Defer until a sixth persona demands it.
- Payments, micropayments, or any token. No persona above benefits from a payment rail, and several are actively harmed by the legal exposure of adding one.
- Mobile-first UX. All five personas use laptops for their primary workflow. Mobile ports follow desktop, not the other way around.

**Explicit design signals:**
- **Named-peer identity is a first-class L5 concept.** Four of five personas rely on verifiable named peers, not pseudonymity.
- **L5 applications must be usable by non-technical users (P2, P4).** "It works if you configure it right" is a failure.
- **L4 is not only compute — it includes storage and intake (P3).** The service layer is broader than the manifesto originally implied.
- **Attestation is end-user-verifiable (P5).** No central attestation service lives in the architecture.

---

*When a new persona is added, it goes here. When an existing persona's needs change (a new jurisdiction, a new adversary in the wild), the persona is updated. Design decisions that cannot be justified by a persona are suspect.*
