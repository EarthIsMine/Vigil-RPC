use base64::Engine;
use solana_sdk::transaction::VersionedTransaction;

use crate::error::ApiError;

/// Decode a base64 or base58 encoded transaction into a VersionedTransaction.
///
/// Default encoding is "base64" (Solana JSON-RPC v2 default).
pub fn decode_transaction(
    encoded: &str,
    encoding: Option<&str>,
) -> Result<VersionedTransaction, ApiError> {
    let bytes = match encoding.unwrap_or("base64") {
        "base64" => base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|e| ApiError::TxDecodeFailed(format!("base64 decode: {e}")))?,
        "base58" => bs58::decode(encoded)
            .into_vec()
            .map_err(|e| ApiError::TxDecodeFailed(format!("base58 decode: {e}")))?,
        other => {
            return Err(ApiError::TxDecodeFailed(format!(
                "unsupported encoding: {other}"
            )));
        }
    };

    bincode::deserialize(&bytes)
        .map_err(|e| ApiError::TxDecodeFailed(format!("bincode deserialize: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use solana_sdk::{hash::Hash, signature::Keypair, signer::Signer};
    use solana_system_transaction as system_transaction;

    #[test]
    fn test_decode_base64_transaction() {
        // Create a simple transfer transaction for testing
        let from = Keypair::new();
        let to = Keypair::new().pubkey();
        let tx = system_transaction::transfer(&from, &to, 1000, Hash::default());

        // Serialize and encode as base64
        let bytes = bincode::serialize(&tx).unwrap();
        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);

        let decoded = decode_transaction(&encoded, Some("base64")).unwrap();
        assert_eq!(decoded.signatures.len(), 1);
    }

    #[test]
    fn test_decode_base58_transaction() {
        let from = Keypair::new();
        let to = Keypair::new().pubkey();
        let tx = system_transaction::transfer(&from, &to, 1000, Hash::default());

        let bytes = bincode::serialize(&tx).unwrap();
        let encoded = bs58::encode(&bytes).into_string();

        let decoded = decode_transaction(&encoded, Some("base58")).unwrap();
        assert_eq!(decoded.signatures.len(), 1);
    }

    #[test]
    fn test_decode_default_encoding_is_base64() {
        let from = Keypair::new();
        let to = Keypair::new().pubkey();
        let tx = system_transaction::transfer(&from, &to, 1000, Hash::default());

        let bytes = bincode::serialize(&tx).unwrap();
        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);

        let decoded = decode_transaction(&encoded, None).unwrap();
        assert_eq!(decoded.signatures.len(), 1);
    }

    #[test]
    fn test_decode_invalid_base64() {
        let result = decode_transaction("not-valid-base64!!!", Some("base64"));
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_unsupported_encoding() {
        let result = decode_transaction("data", Some("hex"));
        assert!(result.is_err());
    }
}
