//! Ultranet cryptographic primitives.
//!
//! A thin wrapper around `ed25519-dalek`. The point of this crate is
//! not to reimplement crypto; it's to keep the choice of algorithm
//! behind a seam so that when (not if) we swap to post-quantum
//! signatures, the change is local.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub use ed25519_dalek::{SIGNATURE_LENGTH, SignatureError};

/// Length in bytes of a verifying (public) key.
pub const VERIFYING_KEY_LENGTH: usize = 32;

/// An ed25519 signing (private) key.
pub struct SigningKey(ed25519_dalek::SigningKey);

/// An ed25519 verifying (public) key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VerifyingKey(ed25519_dalek::VerifyingKey);

/// An ed25519 signature.
#[derive(Debug, Clone, Copy)]
pub struct Signature(ed25519_dalek::Signature);

impl SigningKey {
    /// Generate a fresh signing key from the OS cryptographic RNG.
    #[must_use]
    pub fn generate() -> Self {
        let mut csprng = rand_core::OsRng;
        Self(ed25519_dalek::SigningKey::generate(&mut csprng))
    }

    /// The verifying (public) key corresponding to this signing key.
    #[must_use]
    pub fn verifying_key(&self) -> VerifyingKey {
        VerifyingKey(self.0.verifying_key())
    }

    /// Sign the given message.
    #[must_use]
    pub fn sign(&self, msg: &[u8]) -> Signature {
        use ed25519_dalek::Signer;
        Signature(self.0.sign(msg))
    }
}

impl VerifyingKey {
    /// Parse a verifying key from its 32-byte serialized form.
    ///
    /// # Errors
    /// Returns an error if the bytes do not represent a valid
    /// point on the curve.
    pub fn from_bytes(bytes: &[u8; VERIFYING_KEY_LENGTH]) -> Result<Self, SignatureError> {
        ed25519_dalek::VerifyingKey::from_bytes(bytes).map(Self)
    }

    /// Serialized form of the key.
    #[must_use]
    pub fn to_bytes(&self) -> [u8; VERIFYING_KEY_LENGTH] {
        self.0.to_bytes()
    }

    /// Verify a signature against this key and the given message.
    ///
    /// # Errors
    /// Returns an error if the signature is invalid for this key and
    /// message.
    pub fn verify(&self, msg: &[u8], sig: &Signature) -> Result<(), SignatureError> {
        use ed25519_dalek::Verifier;
        self.0.verify(msg, &sig.0)
    }
}

impl Signature {
    /// Parse a signature from its 64-byte serialized form.
    #[must_use]
    pub fn from_bytes(bytes: &[u8; SIGNATURE_LENGTH]) -> Self {
        Self(ed25519_dalek::Signature::from_bytes(bytes))
    }

    /// Serialized form of the signature.
    #[must_use]
    pub fn to_bytes(&self) -> [u8; SIGNATURE_LENGTH] {
        self.0.to_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_and_verify_roundtrip() {
        let sk = SigningKey::generate();
        let vk = sk.verifying_key();
        let msg = b"ultranet test message";
        let sig = sk.sign(msg);
        vk.verify(msg, &sig).expect("signature should verify");
    }

    #[test]
    fn wrong_message_fails() {
        let sk = SigningKey::generate();
        let vk = sk.verifying_key();
        let sig = sk.sign(b"one");
        vk.verify(b"two", &sig).expect_err("should reject different message");
    }

    #[test]
    fn wrong_key_fails() {
        let sk1 = SigningKey::generate();
        let sk2 = SigningKey::generate();
        let sig = sk1.sign(b"msg");
        sk2.verifying_key().verify(b"msg", &sig).expect_err("should reject wrong key");
    }
}
