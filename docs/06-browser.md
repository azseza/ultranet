# L5-runtime — The Verified Hyperdocument Console

**Status:** Design note v0.1 — 2026-04-30.
**Layer:** L5-runtime, the user-facing surface beneath every L5-app.
**Purpose:** Define the console through which users encounter Ultranet. This is *not* a web browser. Modeling Ultranet's primary surface on the web would preserve the shape of the thing that broke (DNS, ambient cookies, third-party requests, stateful JS, navigation history). The web's affordances are wrong for our personas. This document defines what we build instead.

---

## What it is, in one sentence

A personal, sovereign reading-writing-computing surface in which the universal datatype is the **verified hyperdocument** — a signed, provenance-bearing, capability-linked artifact that flows between named peers, never through a central host, and that runs any embedded computation inside an attested sandbox.

Everything users do in Ultranet — message, share files, listen to a station, run an inference, archive a case — is an interaction with hyperdocuments. The console is the only surface that touches them.

## Why "browser" is the wrong word

The web is shaped for surveillance. Every successful web feature — DNS lookups, third-party requests, autocomplete from history, persistent cookies, fingerprintable canvases, "remember this device" — leaks. A privacy browser (Tor Browser, Brave) is a privacy patch on a hostile substrate.

The console preserves the user-recognizable form ("here is a thing I read; here is an address I follow; here is something I send") while replacing every privacy-hostile primitive with one that fails closed by design. We call it a browser because users will recognize the silhouette. Internally, every contributor should remember it is a *console*, and the document — not the page — is the unit of everything.

---

## The atom: the verified hyperdocument

A hyperdocument is a self-describing, signed artifact. Its structure:

```
┌───────────────────────────────────────────────────────────────┐
│  HEADER (cleartext, signed)                                   │
│   • author:        ed25519 verifying key (peer identity)      │
│   • persona:       persona handle within the author identity  │
│   • created_at:    monotonic counter + author-local time hint │
│   • content_type:  MIME-style identifier                      │
│   • derivation:    optional list of parent doc hashes + role  │
│   • attestation:   optional SEV-SNP report binding compute    │
│   • caps_in:       capabilities this doc was generated from   │
│   • caps_out:      capabilities this doc grants to readers    │
├───────────────────────────────────────────────────────────────┤
│  BODY (typed payload)                                         │
│   • plain text / markdown / structured form / image / audio   │
│   • optional active fragments (sandboxed, capability-typed)   │
├───────────────────────────────────────────────────────────────┤
│  SIGNATURE (over header || body, by persona key)              │
└───────────────────────────────────────────────────────────────┘
```

A hyperdocument is **verified** when:

1. The signature checks against the persona key in the header.
2. The persona key is bound to the author identity via a separate persona-binding document (also a hyperdocument; the user has accepted this binding or is being shown an unbound persona warning).
3. If `derivation` is present, every parent hash resolves to a hyperdocument the user has either retrieved or chosen to trust by hash alone.
4. If `attestation` is present, the SEV-SNP report verifies against AMD's cert chain *and* the measurement matches a binary the user has chosen to trust.

Verification is mandatory. A document that fails any check is rendered with explicit, irreducible warning chrome and cannot grant any capability, derive any further document, or appear in any feed.

---

## What replaces what

| Web concept | Console concept | Why the swap |
|---|---|---|
| URL | `caps://<peer>/<intent>?token=<cap>` | Locatability is the bug. A cap-URL resolves through L3, never DNS. |
| URL bar | Capability bar | Type a peer pseudonym or a feed name; L3 DHT resolves; descriptor verifies. No autocomplete from history. |
| Address autocomplete | Address book of named peers | History is a privacy leak. Suggestions from search engines are surveillance. The user's own pet-named peers are the only suggestion source. |
| Tabs | Workspaces | A workspace is a coherent task: peers in a panel, active capabilities pinned, an inbox of incoming docs. Tabs assume horizontal navigation; that's a web pattern, not a sovereign one. |
| Back / forward | Sessions | A session is a journal of documents read, compositions made, peers contacted. Encrypted, local, exportable as a signed bundle (P4's compliance requirement). |
| Bookmarks | Pinned capabilities | A pin is a typed reference (peer identity + intent + capability token), not a stored URL. |
| Cookies / localStorage | Per-persona, per-cap state | State is keyed by `(persona, capability)` and lives in encrypted local storage. No global cookie jar, no cross-cap state, no third-party state of any kind. |
| Browser history | Session journal (per-session) | History as a single global timeline you cannot escape is replaced by named sessions you start and close deliberately. |
| Search engine | Feed / inbox / address book | Discovery is from peers and feeds, not from a centralized index. Search-the-public-web is not a workflow. |
| JavaScript with full DOM | Capability-typed sandbox runtime | Active fragments run with no network, no clock, no filesystem outside an explicit per-fragment capability grant. Probably WebAssembly with restricted imports; possibly a smaller typed scripting layer. |
| TLS lock icon | Verification chain panel | Persistent, unmissable display: signer key (with pet-name), persona, attestation chain, derivation tree. Provenance is rendered, not hidden. |
| Incognito mode | The ordinary mode | There is no non-incognito mode. Every session is local-only by default; nothing is shared with anyone unless the user dispatches a doc to a named peer. |

---

## The verb model

The console exposes a small set of verbs. Every L5-app composes from them.

| Verb | What it does | Layer touched |
|---|---|---|
| **Read** `<peer>:<intent>` | Resolve a capability, retrieve a document, verify, render. | L3 (resolve), L2 (transport), L4-state (if persistent), L5-runtime (verify+render) |
| **Compose** | Author a new document; persona is selected, derivation is automatic if composing from open docs. | L5-runtime |
| **Send to** `<peer>` | Dispatch a document to a named peer's inbox. | L3 (resolve recipient), L2 (deliver), L4-state (if recipient offline) |
| **Compute on** `<peer>:<measurement>` | Submit a document to an attested L4-compute peer; verify attestation; receive derived doc. | L4-compute, L2, L3 |
| **Subscribe to** `<feed-cap>` | Follow a feed; new docs arrive in the inbox as derived items. | L3, L2 |
| **Pin** `<cap>` | Persist a capability in the workspace. | L5-runtime local state |
| **Switch persona** | Change the active persona; refuse to leak between personas. | L5-runtime |
| **Archive** | Replicate a document to one or more L4-state peers. | L4-state, L3 |
| **Open session** / **Close session** | Begin or end a journaled workflow. | L5-runtime |

Every L5-app is a curated arrangement of these verbs. A messenger is `compose` + `send to` over a peer-typed cap. A radio listener is `subscribe to` + automatic `read` on each new frame. A compute marketplace is `compute on` against a discovered set of peers. A clinical case-share (P2) is `send to` + `archive`. A journalistic intake (P3) is a published `feed-cap` plus an inbox view.

There is no verb for "go to a website."

---

## Visual structure

Three persistent regions, none of them removable:

1. **Persona band (top).** A colored strip identifying the active persona. Switching personas is explicit and produces a visible transition. The band is the user's primary cue for "which me am I right now?"

2. **Capability bar (under persona band).** A single text-entry surface. Accepts: peer pseudonyms, feed names, raw `caps://` references, and pet-names from the address book. Resolves through L3. Shows resolution status (pending → verified → rendered) inline. No autocomplete from session history; only from the user's pinned capabilities and address book.

3. **Verification panel (right side).** Always visible for the currently focused document. Renders:
   - Author identity and persona, with pet-name if known
   - Signature status (verified / unbound persona / failed)
   - Attestation chain (if any L4-compute touched the document)
   - Derivation tree (which docs this was composed from)
   - Capability grants (what this doc lets the reader do)

The center is the workspace itself — a typed-document surface. No address bar in the center; no chrome that obscures the document.

---

## Sandbox model

Active fragments inside a hyperdocument execute under strict isolation:

- **No network.** A fragment cannot dial. It cannot resolve a name. It cannot send a packet. If it needs a capability outside the document, it must request it explicitly through a structured prompt the runtime renders, and the user must grant per-action.
- **No clock by default.** A fragment receives a coarse, monotonic counter rather than wall-clock time. Wall-clock leaks identity-relevant timing.
- **No filesystem outside its document.** A fragment can read its own document and write into a per-fragment scratch space scoped to the open session.
- **No DOM in the web sense.** Rendering is structured: the fragment emits typed view updates the runtime applies to a constrained widget tree. There is no `document.cookie`, no `window`, no fetch, no iframe.
- **Resource bounds.** CPU time, memory, and I/O are bounded per fragment. Exceeding bounds is a render error, not a hang.

The implementation is most likely WebAssembly with a custom restricted import set — call it `ult-wasm`. It is **not** a browser engine. It is closer in spirit to a calculator that happens to be Turing-complete.

The runtime itself runs each open document in a separate Firecracker microVM where the substrate supports it (Track 2 appliances). On Track 1 hosts, each document runs in a process-isolated `wasmtime` (or equivalent) instance. Either way, the threat model treats the document as untrusted by default.

---

## Identity in the console

Identity is a structural concern, not a settings dialog. Three layers:

1. **Author identity.** The long-term ed25519 keypair. Backed by TPM (I2). Never used to sign hyperdocuments directly.
2. **Personas.** Per-author named subkeys, signed by the author identity. Each persona has its own colored band, its own pinned capabilities, its own session journal, its own L4 dispatch surface. Personas do not share state. The runtime refuses to dispatch a document signed by persona A to a peer the user has only ever contacted from persona B unless the user explicitly authorizes the cross.
3. **Pet-names.** Local, unilateral, per-persona names for other peers' identities. Pet-names never travel; remote peers see only the keypair. Two personas of the same user can use different pet-names for the same remote peer without leaking the link.

Persona-switching is a deliberate gesture (a keyboard shortcut and a visible transition). The runtime provides explicit support for *masking* — entering a persona-locked mode where the other personas are not visible at all in the UI, for situations where the device might be coercively examined.

Future doc `08-identity.md` will detail anonymous-credential primitives (BBS+ or successor) for selective disclosure.

---

## Sessions and the journal

A session is a first-class object: started, named, and closed by the user. A session contains:

- The list of documents read, with timestamps (per-session monotonic, not wall-clock)
- The compositions authored, with their derivation trees
- The dispatches sent (which doc, to which peer, when)
- The compute jobs run (with their attestation reports)
- The capabilities granted to active fragments

Sessions are stored encrypted with per-persona keys. They are never sent over the network unless the user explicitly exports them as a signed bundle (P4's compliance log). The bundle can be verified by any third party who has the user's persona key.

Sessions exist *because* the absence of browser history is a feature: the user gets a journal they chose to keep, not a global timeline they can never escape.

---

## Offline-first by physics

Tor latencies make every "spinner" feel broken on a clearnet-shaped UI. The console treats the network as a slow, unreliable resource and structures the UX around that:

- Every operation is async, with a clear three-state status: dispatched / acknowledged / settled.
- Documents stream and render progressively. The "page is loading" model is replaced by "the document is filling in."
- When the substrate is unreachable (T6 active suppression, or simply offline), the console keeps working: read what's local, author, queue sends. There is no error dialog that says "no internet." Ultranet's whole point is that "the public internet is down" should not be the same event as "your work is unavailable."
- Substrate transitions (A7) are surfaced through a small persistent status indicator. The user knows when they are operating on a degraded substrate and can choose to defer sensitive dispatches.

---

## Out of scope (deliberate)

These are not features. Listed here so contributors do not propose them.

- **A general web browser.** The console does not render arbitrary HTML from arbitrary sources. It renders hyperdocuments. If a user wants to read the public web, that is a different tool — not this one, and not on Ultranet.
- **A search engine.** Discovery is from peers and feeds. Centralized search is incompatible with the threat model.
- **Notifications.** Push notifications require an upstream notification service. The console pulls; it is not pushed to.
- **Account recovery.** "Forgot your key, click here" is impossible by design (A4, I4). Recovery is a tooling and education problem, not a runtime feature.
- **A visible URL bar showing onion addresses.** Onion addresses are an implementation detail of L2/L3. The user sees pet-names and capability handles. Showing the underlying onion address normalizes onion-address-as-identity, which we are explicitly *not* doing.

---

## What the console must let users do (acceptance)

The console is correct when each persona in `03-personas.md` can complete their canonical task without leaving it:

- **Leyla (P1).** Receives a `caps://` from a source via an out-of-band channel. Reads the source's submission. Composes a follow-up question. Dispatches it. Switches persona. Reviews the same submission under her outlet-facing persona to check whether the story is publishable. The source's persona, the questions exchanged, and the cross-persona review never become linkable to a network observer or a colluding peer.

- **Mathieu (P2).** Drops a case file on a named colleague's inbox. Receives confirmation. The file is a hyperdocument with `derivation` pointing to the patient's record (which lives only in his local store). Neither his hospital IT nor any cloud holds either document.

- **Aisha (P3).** Subscribes to her organization's intake feed. New submissions appear in her inbox as hyperdocuments with anonymous author identities. She archives the submissions to two peer-trusted L4-state nodes. The original submitter remains unobservable to her, to the L4-state operators, and to any observer of the substrate.

- **Jin (P4).** Sends privileged material to co-counsel. The console journals the session as a signed bundle. He exports the bundle to his firm's compliance system. The bundle proves *that* the communication happened, *to whom*, and *when*; it does not contain the substance, which remains scoped to the recipient.

- **Raza (P5).** Selects a document containing genomic data. Right-clicks → "Compute on" → picks a published L4-compute peer. The runtime fetches the attestation report, verifies AMD chain and measurement, displays the chain, asks for confirmation, dispatches the doc, receives the attested derived doc back. The partner lab sees cycles spent; nothing else.

If any of these workflows requires the user to leave the console for a CLI, a scripting language, or a config file, the console has failed.

---

## Implementation order

The console does not get built monolithically. Implementation order, mapped to roadmap milestones (`07-roadmap.md`):

1. **M6.0 — Capability resolver and verification chain.** A headless library that takes a `caps://` reference, resolves through L3, retrieves the document, verifies signature and attestation, and emits a structured verification trace. This is the kernel.

2. **M6.1 — Document store and session journal.** Local encrypted storage for received documents and active sessions. Persona-keyed.

3. **M6.2 — Composition and dispatch.** Authoring UI + send verb. Wires into the existing M3 handshake for delivery.

4. **M6.3 — Active-fragment sandbox.** `ult-wasm` runtime with restricted imports. No active fragments are accepted before this lands.

5. **M6.4 — Visual shell.** The persistent persona band, capability bar, verification panel. Keyboard-first, mouse-supplementary. Likely a Tauri-class shell or a native widget toolkit; explicitly *not* an Electron-with-web-content arrangement.

6. **M6.5 — Persona masking and plural-identity UX.** The deliberate persona-switching gesture, the masked mode, the cross-persona refusal logic.

The first four of those are headless and mechanically testable; the visual shell follows once the kernel is sound. We do *not* start with a UI and bolt the threat model on later.

---

## What this document commits to and does not

This is a vision document for the console. It commits to:

- The hyperdocument as universal type.
- Capability-typed addressing, never URLs.
- Provenance-rendered chrome.
- Plural identity as first-class, with refusal-to-link defaults.
- Sandboxed active fragments under resource bounds.
- Sessions instead of history.
- Offline-first behavior.

It does not commit to:

- A specific UI framework (Tauri vs. native widgets vs. egui — to be decided when M6.4 starts).
- A specific active-fragment language (Wasm-with-restricted-imports vs. a custom typed scripting layer).
- A specific cryptographic primitive for plural-identity unlinkability (BBS+ vs. successor schemes — `08-identity.md`).
- A specific persistence engine for the document store (sled vs. redb vs. raw files — to be decided when M6.1 starts).

Contributors who want to write code before these are decided should pick something reasonable, document why, and accept that the choice is provisional.

---

*This document is the bridge between the abstract design system and the concrete experience of using Ultranet. When a design decision in the console layer cannot be justified by the design system, one of the two is wrong.*
