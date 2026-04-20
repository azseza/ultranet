# M2 — Rendezvous by key

**Status:** Design note, 2026-04-20. Read and agreed before any M2 code is written.

---

## What M2 proves

One sentence: **two Ultranet nodes find each other using only a cryptographic key, with no IP address ever exposed to either side.**

This is the first milestone where the code does something the manifesto is actually about. M1 proved we can join the Tor network; M2 proves we can use it for rendezvous without a central address book.

---

## The smallest thing that proves it

Two terminal windows. One machine is fine. Two machines is also fine; the code doesn't care.

- **Terminal 1 — the listener:**
  ```
  $ ultranet serve
  ultranet 0.0.1
  bootstrapping L2...  READY
  starting L3 listener...
  my peer id: ult1qn5k...7xez4.onion   ← the "address" others use to reach us
  waiting for connections...
  ```

- **Terminal 2 — the dialer:** (run after copy-pasting the id above)
  ```
  $ ultranet dial ult1qn5k...7xez4.onion
  ultranet 0.0.1
  bootstrapping L2...  READY
  dialing peer...  CONNECTED
  sent HELLO, got HELLO-ACK from ult1qn5k...7xez4
  ```

- **What the listener prints when dialed:**
  ```
  incoming connection from ult1abc...xyz.onion
  received HELLO, sent HELLO-ACK
  ```

That's it. Two processes, they find each other by key, they exchange a short handshake, both can exit cleanly.

---

## What we reuse from Tor (and what we'll replace later)

Tor already has hidden services. A hidden service:
- is identified by an ed25519 public key (the `.onion` address is a short encoding of that key),
- publishes a signed "here is how to reach me" descriptor to Tor's Hidden Service Directory (HSDir),
- is reachable by anyone who knows the key, over an onion circuit, with no IP exposed on either side.

This is exactly the L3 primitive we need. For M2 we use it directly. Concretely: we use the `arti` crate `tor-hsservice` to run a hidden service, and `arti-client`'s existing ability to dial `.onion` addresses to reach it.

**What we're borrowing:** the HSDir as our "find a peer by key" mechanism.

**What will eventually be replaced:** the HSDir is operated by Tor's directory authorities. Our threat model (`02-threat-model.md` §4.3) commits us to replacing it with BFT gossip among bootstrap peers, because directory authorities are a coercion target (T4). That replacement is not M2. It's a later milestone. M2 is allowed to use the HSDir because it's the smallest step that proves the claim, and the replacement is an isolated change at the L3 crate boundary.

This is the kind of tradeoff the design system's §4 tension table is for: **invariant stability wins long-term, but not at the cost of never shipping anything**. We ship on HSDir, then replace it.

---

## Data structures

Two types, added to `ultranet-core`:

```rust
/// A peer's long-term identity. The bytes are an ed25519 public key.
/// Displaying it produces the .onion form that humans copy-paste.
pub struct PeerId([u8; 32]);

/// A live connection to a peer: a bidirectional byte stream over an
/// onion circuit. Nothing application-level yet.
pub struct PeerConnection { /* wraps an arti DataStream */ }
```

`PeerId` is the only name a peer has. No DNS, no usernames, no accounts.

---

## Protocol — one handshake message, one reply

At the byte level, once a stream is open:

```
dialer -> listener:  "HELLO\n" + dialer's PeerId bytes (32 bytes) + "\n"
listener -> dialer:  "HELLO-ACK\n" + listener's PeerId bytes (32 bytes) + "\n"
```

That's the entire M2 protocol. No framing scheme, no length prefixes, no versioning — we'll add those when we have something to put inside them (M3/M4). M2's only job is to prove that a named peer can reach a named peer.

A real protocol (versioning, framing, signed handshake, key rotation) comes with M3 or M4, when there's an actual application workload to justify the complexity.

---

## Crate layout

New crate: `crates/ultranet-rendezvous` (L3 in the design system).

```
ultranet-rendezvous
├── publish(...) -> Listener     // starts a hidden service, returns PeerId + accept loop
└── dial(peer_id) -> PeerConnection
```

It depends on `ultranet-transport` for the bootstrapped Tor client, and on `tor-hsservice` for the hidden-service logic.

`ultranet-node` gains two new subcommands — `serve` and `dial PEER_ID` — that compose transport + rendezvous into the demo above.

---

## Out of scope for M2 — on purpose

None of these are wrong ideas; they're just not M2.

- **Persistent peer identity.** M2 generates a fresh ed25519 key on each `serve`. Real identity persistence needs the I2 (no plaintext at rest) design, which we haven't written.
- **Peer discovery beyond copy-paste.** M2 assumes the dialer already knows the listener's `PeerId`. How peers learn about each other in the first place — friend-of-friend, QR code, out-of-band — is a later milestone.
- **Multiple simultaneous peers.** Listener handles one connection at a time. Concurrency comes when it's needed.
- **Authenticated handshake.** The HELLO exchange is not signed. Tor's circuit crypto guarantees the listener is the holder of the `.onion` key; the dialer's identity in HELLO is asserted, not proven. We'll add a proof-of-key challenge in M3.
- **Replacement of HSDir with our own directory.** Later milestone. Isolated to this crate.
- **Any attempt at traffic-analysis resistance beyond what Tor provides.** Cover traffic (I7) is a much later milestone.

---

## What M2 feels like when it works

You open two terminals. In the first, you type `ultranet serve` and a long string appears. You copy it. In the second, you type `ultranet dial <paste>`. Both terminals print a handshake line. You close them.

No IP addresses were exchanged. Your router logs show only connections to random Tor relays, same as any other Tor user on the network. Anyone watching either machine sees Tor traffic; nobody sees that the two machines just spoke to each other. That's what we're building.

---

## Effort and risk estimate

- **Code:** ~200–300 lines across the new crate and the node binary.
- **First-compile time:** probably not much additional — `tor-hsservice` is already in the arti dependency graph.
- **Risk:** `tor-hsservice` API may have moved since last I looked. Bootstrapping a hidden service takes longer than a client circuit (can be up to a minute). No major technical risk; the primitive exists and works in arti.

---

## Ready?

If this design is right, the next step is:
1. Create `crates/ultranet-rendezvous` with `publish` and `dial`.
2. Add `serve` and `dial` subcommands to `ultranet-node`.
3. Run the two-terminal demo above end-to-end.
4. Report.

If anything in this design is wrong, say so now — any of the decisions above (using HSDir, the minimal HELLO protocol, skipping persistent identity) is a negotiable choice. The invariants in `05-design-system.md` are not; the shape of this particular milestone is.
