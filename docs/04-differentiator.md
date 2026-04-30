# Differentiator

**Status:** Draft v0.2 — 2026-04-30.

---

## The one-sentence version

**Ultranet is what comes after the cloud era, for people whose work cannot be cloud-shaped.**

The cloud's economics — borrow someone else's hardware, pay only for what you use — are correct. The cloud's *price* — surrender the substance of the work to the provider, and the metadata of the work to the public internet — is unaffordable for the personas in `03-personas.md`. Ultranet keeps the economics and removes the price.

## The one-paragraph version

Ultranet is the only system that combines Tor-class anonymous transport, directory-authority-free rendezvous, and hardware-attested trustless compute into a single coherent platform — with no token, no chain, and no clearnet mode. Tor carries bytes but does not execute workloads and depends on a small trusted directory. Phala executes confidential workloads but orchestrates them over clearnet and a public blockchain. Nym defends against global passive adversaries but requires a staked token to participate. Session, Veilid, Freenet, I2P, and Urbit each occupy a corner of this space. None of them cover the diagonal. Ultranet covers the diagonal: a journalist, a lawyer, a clinician, a researcher, and a compute-delegating scientist can all do their actual work on the same infrastructure, with the same non-negotiable privacy invariants, without any one of them paying for a token or trusting a directory.

---

## Said differently, for different audiences

### For a security engineer
`Tor + Phala + Session - blockchain - directory-authority trust + SEV-SNP attestation + strict "dark by default" enforcement`. The novelty is not any one component; it is the composition and the refusal to include the escape hatches that each predecessor retained.

### For a journalist's editor
You can talk to sources, share files, and collaborate with colleagues over infrastructure that has no phone numbers, no domains, no hosting providers, no cloud accounts, and no "public web" fallback. If your reporter's laptop is taken, her archive is a brick. If your outlet's domain is blocked, the work continues.

### For an academic collaborator
You can run workloads on a partner's hardware without revealing your code or data, and the partner can accept those workloads without seeing what they are computing — without routing any of this through a blockchain, a central broker, or an orchestration cloud.

### For a potential contributor
It is not another messenger. It is not another coin. It is an infrastructure project that takes privacy seriously enough to ship without the features that would have made it easier to adopt, and takes ambition seriously enough to solve the trustless-compute problem on top of the anonymous-transport problem in the same system. The boring part — Arti, libp2p, Firecracker, SEV-SNP — is well-understood engineering. The hard part is the discipline: no token, no directory, no clearnet bridge, ever. That discipline is the entire contribution.

### For a skeptic
You are right that each piece exists somewhere. You are right that the hardest attacks — global passive correlation, determined global active suppression — remain partially unsolved here and elsewhere. What we claim is narrower: that the *combination* of these pieces, built under one design system with enforced invariants, is useful to the five people in `03-personas.md` in ways that no existing system is. If that claim is wrong for all five of them, the project is wrong. If it is right for any of them, the project is worth building.

---

## What this document is not

This is not marketing copy. It is a statement of scope. When someone asks "isn't this just Tor?" or "isn't this just Phala?", this is the answer they get. When a contributor proposes a feature that would make Ultranet *more* like any one of its predecessors — add a token, add a clearnet bridge, add a central directory — this is the document they are arguing against.

---

*Revised whenever the prior-art survey turns up a project that eats part of the differentiator. Revised whenever a persona's needs force a new scope decision. Otherwise stable.*
