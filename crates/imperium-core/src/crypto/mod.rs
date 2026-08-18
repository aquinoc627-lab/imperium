//! Cryptographic Primitives
//!
//! All crypto operations for IMPERIUM.
//! Uses Ring for constant-time operations, BLAKE3 for hashing.

use crate::error::{CryptoError, CryptoResult};
use aes_gcm::{Aes256Gcm, Key, Nonce, KeyInit};
use aes_gcm::aead::{Aead, OsRng, Payload};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{SaltString, rand_core::OsRng as ArgonOsRng};
use blake3;
use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};
use rand_core::RngCore;
use secrecy::{SecretBox, SecretVec, ExposeSecret};
use serde::{Deserialize, Serialize};
use std::fmt;
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret as X25519StaticSecret};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Hash output (BLAKE3, 32 bytes)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct Hash(pub [u8; 32]);

impl Hash {
    /// Compute BLAKE3 hash of data
    pub fn blake3(data: &[u8]) -> Self {
        let hash = blake3::hash(data);
        Self(*hash.as_bytes())
    }

    /// Compute BLAKE3 hash of multiple parts
    pub fn blake3_multi(parts: &[&[u8]]) -> Self {
        let mut hasher = blake3::Hasher::new();
        for part in parts {
            hasher.update(part);
        }
        Self(*hasher.finalize().as_bytes())
    }

    /// Create from bytes
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Get bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Hex encode
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Parse from hex
    pub fn from_hex(hex: &str) -> CryptoResult<Self> {
        let bytes = hex::decode(hex)?;
        if bytes.len() != 32 {
            return Err(CryptoError::InvalidHashLength);
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl Default for Hash {
    fn default() -> Self {
        Self([0u8; 32])
    }
}

/// Ed25519 signing key
#[derive(Debug, Clone, Zeroize, ZeroizeOnDrop)]
pub struct SigningKeyPair {
    secret: SecretBox<SigningKey>,
    public: VerifyingKey,
}

impl SigningKeyPair {
    /// Generate new key pair
    pub fn generate() -> CryptoResult<Self> {
        let mut csprng = OsRng;
        let secret = SigningKey::generate(&mut csprng);
        let public = secret.verifying_key();
        Ok(Self {
            secret: SecretBox::new(Box::new(secret)),
            public,
        })
    }

    /// Load from seed (32 bytes)
    pub fn from_seed(seed: &[u8; 32]) -> CryptoResult<Self> {
        let secret = SigningKey::from_bytes(seed);
        let public = secret.verifying_key();
        Ok(Self {
            secret: SecretBox::new(Box::new(secret)),
            public,
        })
    }

    /// Get public key
    pub fn public_key(&self) -> &VerifyingKey {
        &self.public
    }

    /// Sign data
    pub fn sign(&self, data: &[u8]) -> Signature {
        self.secret.sign(data)
    }

    /// Verify signature
    pub fn verify(&self, data: &[u8], signature: &Signature) -> CryptoResult<()> {
        self.public.verify(data, signature)?;
        Ok(())
    }

    /// Export secret key (DANGEROUS - use with caution)
    pub fn export_secret(&self) -> SecretVec<u8> {
        SecretVec::new(self.secret.to_bytes().to_vec())
    }
}

/// Ed25519 verifying key (public key)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct VerifyingKeyWrapper(pub VerifyingKey);

impl VerifyingKeyWrapper {
    pub fn from_bytes(bytes: &[u8; 32]) -> CryptoResult<Self> {
        Ok(Self(VerifyingKey::from_bytes(bytes)?))
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        self.0.to_bytes()
    }

    pub fn verify(&self, data: &[u8], signature: &Signature) -> CryptoResult<()> {
        self.0.verify(data, signature)?;
        Ok(())
    }

    pub fn verify_strict(&self, data: &[u8], signature: &[u8]) -> CryptoResult<()> {
        let sig = Signature::from_slice(signature)?;
        self.verify(data, &sig)
    }
}

/// X25519 key pair for key exchange
#[derive(Debug, Clone, Zeroize, ZeroizeOnDrop)]
pub struct X25519KeyPair {
    secret: SecretBox<X25519StaticSecret>,
    public: X25519PublicKey,
}

impl X25519KeyPair {
    /// Generate new key pair
    pub fn generate() -> Self {
        let secret = X25519StaticSecret::random_from_rng(OsRng);
        let public = X25519PublicKey::from(&secret);
        Self {
            secret: SecretBox::new(Box::new(secret)),
            public,
        }
    }

    /// Get public key
    pub fn public_key(&self) -> &X25519PublicKey {
        &self.public
    }

    /// Perform key exchange
    pub fn diffie_hellman(&self, peer_public: &X25519PublicKey) -> SharedSecret {
        let shared = self.secret.diffie_hellman(peer_public);
        SharedSecret(SecretBox::new(Box::new(shared.to_bytes())))
    }
}

/// Shared secret from key exchange
#[derive(Debug, Clone, Zeroize, ZeroizeOnDrop)]
pub struct SharedSecret(SecretBox<[u8; 32]>);

impl SharedSecret {
    /// Derive encryption key using HKDF
    pub fn derive_key(&self, salt: &[u8], info: &[u8]) -> EncryptionKey {
        use hkdf::Hkdf;
        use sha2::Sha256;

        let hk = Hkdf::<Sha256>::new(Some(salt), self.0.expose_secret());
        let mut key = [0u8; 32];
        hk.expand(info, &mut key).expect("HKDF expand");
        EncryptionKey(SecretBox::new(Box::new(key)))
    }

    /// Get raw bytes (for compatibility)
    pub fn as_bytes(&self) -> &[u8; 32] {
        self.0.expose_secret()
    }
}

/// AES-256-GCM encryption key
#[derive(Debug, Clone, Zeroize, ZeroizeOnDrop)]
pub struct EncryptionKey(SecretBox<[u8; 32]>);

impl EncryptionKey {
    /// Generate random key
    pub fn generate() -> Self {
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        Self(SecretBox::new(Box::new(key)))
    }

    /// Create from bytes
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(SecretBox::new(Box::new(bytes)))
    }

    /// Encrypt data
    pub fn encrypt(&self, plaintext: &[u8], aad: &[u8]) -> CryptoResult<EncryptedData> {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(self.0.expose_secret()));
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let payload = Payload {
            msg: plaintext,
            aad,
        };

        let ciphertext = cipher.encrypt(nonce, payload)?;
        Ok(EncryptedData {
            nonce: nonce_bytes,
            ciphertext,
        })
    }

    /// Decrypt data
    pub fn decrypt(&self, data: &EncryptedData, aad: &[u8]) -> CryptoResult<Vec<u8>> {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(self.0.expose_secret()));
        let nonce = Nonce::from_slice(&data.nonce);
        let plaintext = cipher.decrypt(nonce, Payload {
            msg: data.ciphertext.as_slice(),
            aad,
        })?;
        Ok(plaintext)
    }
}

/// Encrypted data container
#[derive(Debug, Clone, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct EncryptedData {
    pub nonce: [u8; 12],
    pub ciphertext: Vec<u8>,
}

/// Argon2 password hash
#[derive(Debug, Clone, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct PasswordHash {
    pub hash: String, // PHC string format
}

impl PasswordHash {
    /// Hash a password
    pub fn hash(password: &[u8]) -> CryptoResult<Self> {
        let salt = SaltString::generate(&mut ArgonOsRng);
        let argon2 = Argon2::default();
        let hash = argon2.hash_password(password, &salt)?
            .to_string();
        Ok(Self { hash })
    }

    /// Verify a password
    pub fn verify(&self, password: &[u8]) -> CryptoResult<()> {
        let parsed_hash = PasswordHash::new(&self.hash)?;
        let argon2 = Argon2::default();
        argon2.verify_password(password, &parsed_hash)?;
        Ok(())
    }
}

/// Secure random bytes
pub fn random_bytes(len: usize) -> Vec<u8> {
    let mut bytes = vec![0u8; len];
    OsRng.fill_bytes(&mut bytes);
    bytes
}

/// Constant-time comparison
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    use subtle::ConstantTimeEq;
    a.ct_eq(b).into()
}