//! Ultranet core types and layer definitions.
//!
//! This crate is the shared vocabulary across all Ultranet layers. It
//! deliberately contains no I/O, no networking, no cryptography
//! implementations — only the types, traits, and layer boundaries that
//! every other crate agrees on.
//!
//! The layer model is specified in `docs/05-design-system.md` §2.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Layer identifiers as defined in the design system.
///
/// Every component in Ultranet belongs to exactly one layer. A component
/// that cannot name its layer is mis-designed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Layer {
    /// L1 — Substrate. Whatever carries bits (TCP/IP today, FSO later).
    Substrate,
    /// L2 — Anonymous transport. Onion circuits, stream multiplexing.
    Transport,
    /// L3 — Rendezvous & naming. Hidden-service descriptors, DHT.
    Rendezvous,
    /// L4 — Service. Blind-compute execution, relay policies, key agreement.
    Service,
    /// L5 — Application. Messaging, file drop, compute marketplace UI.
    Application,
}

impl Layer {
    /// Human-readable label including layer number.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Substrate => "L1 Substrate",
            Self::Transport => "L2 Transport",
            Self::Rendezvous => "L3 Rendezvous",
            Self::Service => "L4 Service",
            Self::Application => "L5 Application",
        }
    }
}

/// The project-wide invariants enumerated in the design system.
///
/// A runtime check against an invariant must reference one of these by
/// name. This gives us a single source of truth between documentation
/// and code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Invariant {
    /// I1 — No ambient authority.
    NoAmbientAuthority,
    /// I2 — No plaintext identifiers on disk.
    NoPlaintextAtRest,
    /// I3 — No outbound connection that isn't an onion circuit.
    OnionOnlyEgress,
    /// I4 — No dependency on a central service.
    NoCentralService,
    /// I5 — Deterministic, reproducible builds.
    ReproducibleBuilds,
    /// I6 — Fail closed.
    FailClosed,
    /// I7 — Cover traffic is a feature, not a decoration.
    CoverTraffic,
}

impl Invariant {
    /// Short label for logs and error messages.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::NoAmbientAuthority => "I1 NoAmbientAuthority",
            Self::NoPlaintextAtRest => "I2 NoPlaintextAtRest",
            Self::OnionOnlyEgress => "I3 OnionOnlyEgress",
            Self::NoCentralService => "I4 NoCentralService",
            Self::ReproducibleBuilds => "I5 ReproducibleBuilds",
            Self::FailClosed => "I6 FailClosed",
            Self::CoverTraffic => "I7 CoverTraffic",
        }
    }
}

/// Project version string, sourced from the workspace.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// A peer's long-term identity.
///
/// In M2 this is backed by the `.onion` string form of a Tor hidden
/// service — human-readable, copy-pasteable, and the sole way a peer
/// is addressed on the network. The inner representation will become
/// a structured type when we replace the HSDir with our own directory.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PeerId(String);

impl PeerId {
    /// Wrap a `.onion` string as a peer id. The caller is responsible
    /// for the string being a valid Tor hidden-service address; parsing
    /// is done by the transport layer when the id is used to dial.
    #[must_use]
    pub fn from_onion(onion: String) -> Self {
        Self(onion)
    }

    /// The `.onion` string form of this peer id.
    #[must_use]
    pub fn as_onion(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PeerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_labels_are_stable() {
        assert_eq!(Layer::Substrate.label(), "L1 Substrate");
        assert_eq!(Layer::Application.label(), "L5 Application");
    }

    #[test]
    fn invariant_labels_start_with_identifier() {
        for inv in [
            Invariant::NoAmbientAuthority,
            Invariant::NoPlaintextAtRest,
            Invariant::OnionOnlyEgress,
            Invariant::NoCentralService,
            Invariant::ReproducibleBuilds,
            Invariant::FailClosed,
            Invariant::CoverTraffic,
        ] {
            assert!(inv.label().starts_with('I'));
        }
    }
}
