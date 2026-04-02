use blake2::{Blake2b512, Digest};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;
use std::path::Path;

// ─── Errors ───────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("hex decode error: {0}")]
    Hex(#[from] hex::FromHexError),
    #[error("invalid signature bytes (expected 64)")]
    InvalidSignatureBytes,
    #[error("signature verification failed")]
    VerifyFailed,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid signing key length (expected 32 bytes)")]
    InvalidKeyLength,
}

// ─── Node Identity ────────────────────────────────────────────────────────────

/// Asymmetric ed25519 identity for a mesh node.
pub struct NodeIdentity {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
}

impl NodeIdentity {
    pub fn new() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        Self {
            signing_key,
            verifying_key,
        }
    }

    /// Hex-encoded 32-byte public key — used as the stable peer identifier.
    pub fn peer_id_hex(&self) -> String {
        hex::encode(self.verifying_key.to_bytes())
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        self.verifying_key
    }

    /// Sign arbitrary bytes; returns an `ed25519_dalek::Signature`.
    pub fn sign_bytes(&self, bytes: &[u8]) -> Signature {
        self.signing_key.sign(bytes)
    }

    /// Persist the raw 32-byte signing key to `path`.
    pub fn save(&self, path: &Path) -> Result<(), CoreError> {
        std::fs::write(path, self.signing_key.to_bytes())?;
        Ok(())
    }

    /// Load a previously saved identity from `path`.
    pub fn load(path: &Path) -> Result<Self, CoreError> {
        let bytes = std::fs::read(path)?;
        let arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| CoreError::InvalidKeyLength)?;
        let signing_key = SigningKey::from_bytes(&arr);
        let verifying_key = signing_key.verifying_key();
        Ok(Self {
            signing_key,
            verifying_key,
        })
    }
}

impl Default for NodeIdentity {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapability {
    pub models: Vec<String>,
    pub max_context: u32,
    pub quant: String,
    pub estimated_speed: f32,
    pub supported_tasks: Vec<String>,
}

impl Default for NodeCapability {
    fn default() -> Self {
        Self {
            models: vec!["local-model".to_string()],
            max_context: 8192,
            quant: "Q4_K_M".to_string(),
            estimated_speed: 25.0,
            supported_tasks: vec!["chat".to_string()],
        }
    }
}

impl NodeCapability {
    /// Simple score for how good this node is for a given job.
    pub fn score_for_job(&self, job_model: &str) -> f32 {
        let model_match = if self
            .models
            .iter()
            .any(|m| m == job_model || m.contains(job_model))
        {
            10.0
        } else {
            0.0
        };
        model_match + self.estimated_speed
    }
}

// ─── Job Envelope ─────────────────────────────────────────────────────────────

/// An unsigned or signed job envelope sent from requester to worker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobEnvelope {
    pub job_id: Uuid,
    /// Human-readable task description / routing hint.
    pub task: String,
    pub model: String,
    pub payload: String,
    /// Hex-encoded public key of the requester.
    pub sender: String,
    /// Hex-encoded ed25519 signature over canonical bytes (empty when unsigned).
    pub signature: String,
}

impl JobEnvelope {
    pub fn new(
        task: impl Into<String>,
        model: impl Into<String>,
        payload: impl Into<String>,
        sender: impl Into<String>,
    ) -> Self {
        Self {
            job_id: Uuid::new_v4(),
            task: task.into(),
            model: model.into(),
            payload: payload.into(),
            sender: sender.into(),
            signature: String::new(),
        }
    }

    /// Deterministic bytes over which the signature is computed — excludes `signature`.
    pub fn canonical_bytes_without_signature(&self) -> Result<Vec<u8>, CoreError> {
        #[derive(Serialize)]
        struct Canonical<'a> {
            job_id: &'a Uuid,
            task: &'a str,
            model: &'a str,
            payload: &'a str,
            sender: &'a str,
        }
        serde_json::to_vec(&Canonical {
            job_id: &self.job_id,
            task: &self.task,
            model: &self.model,
            payload: &self.payload,
            sender: &self.sender,
        })
        .map_err(CoreError::Serde)
    }

    /// Sign this envelope in-place with the given identity.
    pub fn sign(mut self, identity: &NodeIdentity) -> Result<Self, CoreError> {
        let bytes = self.canonical_bytes_without_signature()?;
        let sig = identity.sign_bytes(&bytes);
        self.signature = hex::encode(sig.to_bytes());
        Ok(self)
    }

    /// Verify the envelope's signature against a known public key.
    pub fn verify(&self, verifying_key: &VerifyingKey) -> Result<(), CoreError> {
        let bytes = self.canonical_bytes_without_signature()?;
        let sig_bytes: Vec<u8> = hex::decode(&self.signature)?;
        let sig_arr: [u8; 64] = sig_bytes
            .try_into()
            .map_err(|_| CoreError::InvalidSignatureBytes)?;
        let sig = Signature::from_bytes(&sig_arr);
        verifying_key
            .verify(&bytes, &sig)
            .map_err(|_| CoreError::VerifyFailed)
    }
}

// ─── Job Result Envelope ──────────────────────────────────────────────────────

/// A signed result returned by the worker to the requester.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResultEnvelope {
    pub job_id: Uuid,
    /// Hex-encoded public key of the worker.
    pub worker: String,
    pub model_used: String,
    pub output: String,
    /// Blake2b-512 hex digest of `output`.
    pub receipt_hash: String,
    /// Hex-encoded ed25519 signature (empty when unsigned).
    pub signature: String,
}

impl JobResultEnvelope {
    pub fn new(
        job_id: Uuid,
        worker: impl Into<String>,
        model_used: impl Into<String>,
        output: impl Into<String>,
    ) -> Self {
        let output = output.into();
        let receipt_hash = Receipt::hash_str(&output);
        Self {
            job_id,
            worker: worker.into(),
            model_used: model_used.into(),
            output,
            receipt_hash,
            signature: String::new(),
        }
    }

    pub fn canonical_bytes_without_signature(&self) -> Result<Vec<u8>, CoreError> {
        #[derive(Serialize)]
        struct Canonical<'a> {
            job_id: &'a Uuid,
            worker: &'a str,
            model_used: &'a str,
            output: &'a str,
            receipt_hash: &'a str,
        }
        serde_json::to_vec(&Canonical {
            job_id: &self.job_id,
            worker: &self.worker,
            model_used: &self.model_used,
            output: &self.output,
            receipt_hash: &self.receipt_hash,
        })
        .map_err(CoreError::Serde)
    }

    pub fn sign(mut self, identity: &NodeIdentity) -> Result<Self, CoreError> {
        let bytes = self.canonical_bytes_without_signature()?;
        let sig = identity.sign_bytes(&bytes);
        self.signature = hex::encode(sig.to_bytes());
        Ok(self)
    }

    pub fn verify(&self, verifying_key: &VerifyingKey) -> Result<(), CoreError> {
        let bytes = self.canonical_bytes_without_signature()?;
        let sig_bytes: Vec<u8> = hex::decode(&self.signature)?;
        let sig_arr: [u8; 64] = sig_bytes
            .try_into()
            .map_err(|_| CoreError::InvalidSignatureBytes)?;
        let sig = Signature::from_bytes(&sig_arr);
        verifying_key
            .verify(&bytes, &sig)
            .map_err(|_| CoreError::VerifyFailed)
    }
}

// ─── Receipt ─────────────────────────────────────────────────────────────────

/// Receipt hashing utility using Blake2b-512.
pub struct Receipt;

impl Receipt {
    /// Returns the hex-encoded Blake2b-512 digest of `s`.
    pub fn hash_str(s: &str) -> String {
        let mut hasher = Blake2b512::new();
        hasher.update(s.as_bytes());
        hex::encode(hasher.finalize())
    }
}

