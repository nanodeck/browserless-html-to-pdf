use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use hmac::{Hmac, Mac};
use opendal::{Operator, Scheme};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub fn build_operator(scheme: &str, opts: &HashMap<String, String>) -> Result<Operator, String> {
    let scheme = Scheme::from_str(scheme).map_err(|e| format!("bad OPENDAL_SCHEME: {e}"))?;
    Operator::via_iter(scheme, opts.clone()).map_err(|e| format!("failed to build operator: {e}"))
}

pub async fn put(op: &Operator, path: &str, bytes: Vec<u8>) -> Result<(), String> {
    op.write(path, bytes)
        .await
        .map(|_| ())
        .map_err(|e| format!("storage write failed: {e}"))
}

#[allow(clippy::too_many_arguments)]
pub async fn signed_url(
    op: &Operator,
    scheme: &str,
    public_base_url: &str,
    signing_key: &[u8],
    path: &str,
    ttl_secs: u64,
    now_unix: u64,
) -> Result<String, String> {
    if scheme == "fs" {
        let expires = now_unix + ttl_secs;
        let sig = sign(signing_key, path, expires);
        Ok(format!(
            "{public_base_url}/downloads/{path}?expires={expires}&sig={sig}"
        ))
    } else {
        let req = op
            .presign_read(path, Duration::from_secs(ttl_secs))
            .await
            .map_err(|e| format!("presign failed: {e}"))?;
        Ok(req.uri().to_string())
    }
}

pub fn sign(key: &[u8], path: &str, expires: u64) -> String {
    let mut mac = HmacSha256::new_from_slice(key).expect("hmac key");
    mac.update(path.as_bytes());
    mac.update(b":");
    mac.update(expires.to_string().as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

pub fn verify(key: &[u8], path: &str, expires: u64, sig_hex: &str) -> bool {
    let Ok(sig) = hex::decode(sig_hex) else {
        return false;
    };
    let mut mac = HmacSha256::new_from_slice(key).expect("hmac key");
    mac.update(path.as_bytes());
    mac.update(b":");
    mac.update(expires.to_string().as_bytes());
    mac.verify_slice(&sig).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fs_signature_roundtrips() {
        let key = b"secret-key";
        let path = "pdfs/abc/report.pdf";
        let expires = 1_900_000_000u64;
        let sig = sign(key, path, expires);
        assert!(verify(key, path, expires, &sig));
        assert!(!verify(key, path, expires, "deadbeef"));
        assert!(!verify(b"other", path, expires, &sig));
    }

    #[tokio::test]
    async fn memory_backend_persists_and_reads() {
        let op = build_operator("memory", &std::collections::HashMap::new()).unwrap();
        op.write("k/x.bin", vec![1u8, 2, 3]).await.unwrap();
        let got = op.read("k/x.bin").await.unwrap().to_vec();
        assert_eq!(got, vec![1, 2, 3]);
    }
}
