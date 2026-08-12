use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

macro_rules! id_newtype {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }

            pub fn from_uuid(id: Uuid) -> Self {
                Self(id)
            }

            pub fn parse(s: &str) -> std::result::Result<Self, uuid::Error> {
                Ok(Self(Uuid::parse_str(s.trim())?))
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

id_newtype!(TransactionId);

impl TransactionId {
    /// Deterministic id: first 16 bytes of SHA-256(`fingerprint`), UUID version 8.
    pub fn from_fingerprint(fingerprint: &str) -> Self {
        let digest = Sha256::digest(fingerprint.as_bytes());
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(&digest[..16]);
        bytes[6] = (bytes[6] & 0x0f) | 0x80;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        Self(Uuid::from_bytes(bytes))
    }
}

id_newtype!(MerchantId);
id_newtype!(CategoryId);
id_newtype!(ImportBatchId);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_id_is_stable_and_distinct() {
        let fp = "v1|generic_csv|assets:bank|2026-01-02|-42.15|USD|generic:row:2|amzn";
        let a = TransactionId::from_fingerprint(fp);
        let b = TransactionId::from_fingerprint(fp);
        let c = TransactionId::from_fingerprint("v1|other");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, TransactionId::new());
    }
}
