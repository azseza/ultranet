# M3 — Signed handshake

**Status:** Design note, 2026-04-20.

---

## What M3 proves

The dialer proves, cryptographically, that it holds a specific private key — so the listener ends the handshake knowing *who* just connected, not merely *that* someone connected.

M2's handshake was symmetric in its trust: either side could have been anyone. After M3 the listener can log, allowlist, deny, or rate-limit on the basis of a proven identity.

The listener's identity is *already* proven to the dialer for free — Tor's hidden-service protocol guarantees that whoever answers at `foo.onion` is the holder of `foo`'s HS key. So M3 only adds the reverse direction.

---

## Two flavors of PeerId after M3

- **Service PeerId** — a `.onion` address. Used to dial. Same as M2.
- **Identity PeerId** — a raw ed25519 public key. Used to prove who a peer is *on* a stream, regardless of whether they run a hidden service.

The listener has both (the .onion + a separate identity key). A dial-only peer has only an Identity PeerId. Displayed as a short lowercase base32 string with an `ult1` prefix, for example: `ult1abc...xyz`.

They are deliberately different types in the code (`ServicePeerId` and `IdentityPeerId`) because confusing them is the kind of bug that silently collapses a security property.

---

## Handshake protocol

Both sides agree on byte-level framing now — M2's ad-hoc newline-terminated strings don't compose with binary keys and signatures.

```
Fixed-size messages. All fields big-endian. No length prefix needed
because every message in M3 is fixed length.

1. Listener -> Dialer   (on stream open, before any dialer write)
   ULT1                            4 bytes  magic
   00                              1 byte   protocol version
   <32-byte random challenge>     32 bytes

2. Dialer -> Listener
   ULT1                            4 bytes  magic
   00                              1 byte   protocol version
   <32-byte ed25519 public key>   32 bytes  dialer's identity pubkey
   <64-byte ed25519 signature>    64 bytes  sig over "ultranet-m3\0" || challenge

3. Listener -> Dialer
   ULT1                            4 bytes  magic
   00                              1 byte   protocol version
   01                              1 byte   result: 01 = ok, 00 = reject
   <32-byte listener identity>    32 bytes  (all zeros if reject)
```

**Verification:** listener checks the signature over the **same** challenge bytes it sent, using the dialer's provided public key. If the signature verifies, the dialer proved possession of the key. If not, the listener sends a reject and closes.

**Domain separator.** The signed bytes are prefixed with the string `"ultranet-m3\0"` (12 bytes) before the 32-byte challenge. This prevents a signature produced for one protocol from being replayed against another protocol that happens to use the same key. Adding a domain separator is cheap; forgetting one is the kind of design error that appears in a Usenix paper five years later.

---

## Identity lifecycle (still ephemeral)

Both the listener's identity keypair and a dial-only peer's identity keypair are **generated fresh on every run** (consistent with M2.5a — no plaintext at rest yet). The listener prints both its .onion and its identity pubkey at startup. The dialer prints its identity pubkey too.

Persistent identity lands with M2.5b (passphrase-encrypted state). For M3 the point is the *protocol*, not the *identity storage*.

---

## Code changes

New crate: `crates/ultranet-crypto`. Thin wrapper over `ed25519-dalek` exposing:

```rust
pub struct SigningKey(ed25519_dalek::SigningKey);
pub struct VerifyingKey(ed25519_dalek::VerifyingKey);
pub struct Signature(ed25519_dalek::Signature);

impl SigningKey {
    pub fn generate() -> Self;
    pub fn verifying_key(&self) -> VerifyingKey;
    pub fn sign(&self, msg: &[u8]) -> Signature;
}
```

Keeping the wrapper thin on purpose — we don't want to re-implement a crypto library, and we don't want every caller to import `ed25519_dalek` directly either (makes switching algorithms later easier).

Additions to `ultranet-core`:

```rust
pub struct IdentityPeerId([u8; 32]);  // ed25519 public-key bytes
// Display as "ult1" + base32(bytes)
```

Changes to `ultranet-rendezvous`:
- `Listener::accept` now performs the signed handshake inside and returns the verified `IdentityPeerId` along with the stream.
- `dial(tor_client, service_peer, my_signing_key)` — takes the dialer's key, performs the handshake, returns the stream.

Changes to `ultranet-node`:
- `serve` generates a fresh identity key, prints `my identity: ult1...`, on each connection prints `incoming peer: ult1...`.
- `dial` generates a fresh identity key, prints its own identity, sends signed handshake.

---

## Out of scope for M3

- **Persistent identity.** M2.5b. M3 keys are regenerated on every run.
- **Mutual authentication at this layer.** Not needed — Tor's HS protocol covers it for the listener direction.
- **Key rotation, revocation, trust-on-first-use.** Future milestones.
- **Post-quantum signatures.** Ed25519 for now; the signature algorithm is isolated behind `ultranet-crypto` so it's swappable.
- **Symmetric session key derivation / double-ratchet.** The stream is already encrypted by Tor's circuit crypto. We don't layer on application-layer encryption until M4 or later, and only if there's a specific reason to.
- **Handshake framing beyond M3's three fixed messages.** A real framing scheme (length prefix + type tag + payload) comes when we have more than one message type to carry.

---

## Demo flow

Listener:
```
$ ultranet serve
my service peer id:  ult1qn5k7xez4.onion
my identity peer id: ult1abc...xyz
waiting for one incoming connection...
incoming peer: ult1def...uvw  (signature verified)
sent accept
M3 serve complete.
```

Dialer:
```
$ ultranet dial ult1qn5k7xez4.onion
my identity peer id: ult1def...uvw
dialing peer ult1qn5k7xez4.onion
CONNECTED
received challenge, signed and sent
received accept; listener identity ult1abc...xyz
M3 dial complete.
```

That's the milestone.
