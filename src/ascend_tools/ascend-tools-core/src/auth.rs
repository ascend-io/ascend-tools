use anyhow::{Context, Result, bail};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde_json::Value;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use ureq::Agent;

/// Manages authentication for the Ascend API.
///
/// Signs Ed25519 JWTs and exchanges them for instance tokens
/// via the Instance API's /api/v1/auth/token endpoint.
pub struct Auth {
    service_account_id: String,
    key_bytes: Vec<u8>,
    instance_api_url: String,
    agent: Agent,
    cloud_api_domain: Mutex<Option<String>>,
    cached_token: Mutex<Option<CachedToken>>,
}

struct CachedToken {
    token: String,
    expires_at: u64,
}

impl Auth {
    pub fn new(
        service_account_id: String,
        key_b64: &str,
        instance_api_url: String,
        agent: Agent,
    ) -> Result<Self> {
        let key_bytes = URL_SAFE_NO_PAD
            .decode(key_b64.trim())
            .or_else(|_| base64::engine::general_purpose::STANDARD.decode(key_b64.trim()))
            .context("failed to decode service account key from base64")?;

        Ok(Self {
            service_account_id,
            key_bytes,
            instance_api_url,
            agent,
            cloud_api_domain: Mutex::new(None),
            cached_token: Mutex::new(None),
        })
    }

    /// Get a valid instance token, refreshing if needed.
    pub fn get_token(&self) -> Result<String> {
        let mut guard = self
            .cached_token
            .lock()
            .expect("token cache mutex poisoned");

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Return cached token if still valid (with 5-minute buffer)
        if let Some(ref cached) = *guard {
            if cached.expires_at > now + 300 {
                return Ok(cached.token.clone());
            }
        }

        // Refresh while holding the lock to prevent thundering herd
        let domain = self.get_cloud_api_domain()?;
        let sa_jwt = self.sign_jwt(now, &domain)?;
        let (instance_token, expires_at) = self.exchange_token(&sa_jwt)?;

        *guard = Some(CachedToken {
            token: instance_token.clone(),
            expires_at,
        });

        Ok(instance_token)
    }

    /// Fetch the Cloud API domain from the Instance API's auth config endpoint.
    /// Cached after the first call.
    fn get_cloud_api_domain(&self) -> Result<String> {
        let mut guard = self
            .cloud_api_domain
            .lock()
            .expect("cloud_api_domain mutex poisoned");

        if let Some(ref domain) = *guard {
            return Ok(domain.clone());
        }

        let url = format!("{}/api/v1/auth/config", self.instance_api_url);
        let mut resp = self
            .agent
            .get(&url)
            .call()
            .map_err(|e| anyhow::anyhow!("failed to fetch auth config ({url}): {e}"))?;

        let status = resp.status().as_u16();
        let body: String = resp.body_mut().read_to_string()?;

        if !(200..300).contains(&status) {
            bail!("Failed to fetch auth config (HTTP {status}): {body}");
        }

        let json: Value = serde_json::from_str(&body).context("failed to parse auth config")?;
        let domain = json
            .get("cloud_api_domain")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("no cloud_api_domain in auth config: {body}"))?
            .to_string();

        *guard = Some(domain.clone());
        Ok(domain)
    }

    /// Sign a JWT with the Ed25519 private key.
    fn sign_jwt(&self, now: u64, cloud_api_domain: &str) -> Result<String> {
        let header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::EdDSA);
        let claims = serde_json::json!({
            "sub": self.service_account_id,
            "aud": format!("https://{cloud_api_domain}/auth/token"),
            "exp": now + 300,
            "iat": now,
            "name": self.service_account_id,
            "service_account": self.service_account_id,
        });
        let der_key = ed25519_seed_to_pkcs8_der(&self.key_bytes)?;
        let key = jsonwebtoken::EncodingKey::from_ed_der(&der_key);
        jsonwebtoken::encode(&header, &claims, &key).context("failed to sign JWT")
    }

    /// Exchange the service account JWT for an instance token
    /// via the Instance API's /api/v1/auth/token endpoint.
    fn exchange_token(&self, sa_jwt: &str) -> Result<(String, u64)> {
        let url = format!("{}/api/v1/auth/token", self.instance_api_url);
        let mut resp = self
            .agent
            .post(&url)
            .header("Authorization", &format!("Bearer {sa_jwt}"))
            .header("Content-Type", "application/json")
            .send_empty()
            .map_err(|e| anyhow::anyhow!("failed to exchange token ({url}): {e}"))?;

        let status = resp.status().as_u16();
        let resp_body: String = resp.body_mut().read_to_string()?;

        if !(200..300).contains(&status) {
            bail!("Token exchange failed (HTTP {status}): {resp_body}");
        }

        let json: Value =
            serde_json::from_str(&resp_body).context("failed to parse token response")?;

        let token = json
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("no access_token in response: {resp_body}"))?
            .to_string();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let expires_at = json
            .get("expiration")
            .and_then(|v| v.as_u64())
            .unwrap_or(now + 3600);

        Ok((token, expires_at))
    }
}

impl std::fmt::Debug for Auth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Auth")
            .field("service_account_id", &self.service_account_id)
            .field("instance_api_url", &self.instance_api_url)
            .field("service_account_key", &"[REDACTED]")
            .finish()
    }
}

/// Wrap a raw 32-byte Ed25519 seed into PKCS#8 v1 DER format (RFC 8410 §7).
///
/// We hand-roll the DER rather than using the `pkcs8` crate because:
/// - The encoding is fixed-size (48 bytes) with no conditional branches
/// - Ed25519 PKCS#8 has a single canonical form (no parameters, no public key)
/// - Adding `pkcs8` as a direct dep would require coordinating features (`alloc`)
///   across three separate Cargo.lock files (core, cli, py)
///
/// Structure:
///   SEQUENCE {
///     INTEGER 0                          -- version (v1)
///     SEQUENCE { OID 1.3.101.112 }       -- algorithm (Ed25519)
///     OCTET STRING { OCTET STRING seed } -- privateKey (CurvePrivateKey)
///   }
fn ed25519_seed_to_pkcs8_der(seed: &[u8]) -> Result<Vec<u8>> {
    if seed.len() != 32 {
        bail!("expected 32-byte Ed25519 seed, got {} bytes", seed.len());
    }
    let prefix: &[u8] = &[
        0x30, 0x2e, // SEQUENCE (46 bytes total)
        0x02, 0x01, 0x00, // INTEGER 0 (version)
        0x30, 0x05, // SEQUENCE (5 bytes)
        0x06, 0x03, 0x2b, 0x65, 0x70, // OID 1.3.101.112 (Ed25519)
        0x04, 0x22, // OCTET STRING (34 bytes)
        0x04, 0x20, // OCTET STRING (32 bytes) — the seed
    ];
    let mut der = Vec::with_capacity(prefix.len() + 32);
    der.extend_from_slice(prefix);
    der.extend_from_slice(seed);
    Ok(der)
}
