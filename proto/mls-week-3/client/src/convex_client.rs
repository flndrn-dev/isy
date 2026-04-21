//! Thin HTTP client for the Convex deployment used by the MLS Week-3 prototype.
//!
//! Convex exposes two relevant POST endpoints on the cloud deployment URL:
//!   - POST {base_url}/api/query    for read-only queries
//!   - POST {base_url}/api/mutation for mutations
//!
//! Both accept a JSON body of the form:
//!   {"path": "module:function", "args": { ... }, "format": "json"}
//!
//! And respond with a JSON envelope:
//!   success -> {"status":"success","value": <Value>}
//!   error   -> {"status":"error","errorMessage": "..."}
//!
//! This module provides a small wrapper plus base64 helpers for `v.bytes()`
//! fields, which Convex represents as `{"$bytes": "<base64>"}` envelopes in
//! the JSON transport.

use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde::Deserialize;
use serde_json::{json, Value};

pub struct ConvexClient {
    base_url: String,
    http: reqwest::Client,
}

#[derive(Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
enum ConvexResponse {
    Success {
        value: Value,
    },
    Error {
        #[serde(rename = "errorMessage")]
        error_message: String,
    },
}

impl ConvexClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            http: reqwest::Client::new(),
        }
    }

    /// Load `CONVEX_URL` from `proto/mls-week-3/.env.local`.
    ///
    /// Works whether the binary is invoked from the repo root or from
    /// `proto/mls-week-3/` directly.
    pub fn from_env() -> Result<Self> {
        for candidate in [".env.local", "proto/mls-week-3/.env.local"] {
            let path = std::path::Path::new(candidate);
            if path.exists() {
                let contents = std::fs::read_to_string(path)
                    .with_context(|| format!("reading {}", path.display()))?;
                for line in contents.lines() {
                    if let Some(rest) = line.trim().strip_prefix("CONVEX_URL=") {
                        let url = rest.trim().trim_matches('"').to_string();
                        if !url.is_empty() {
                            return Ok(Self::new(url));
                        }
                    }
                }
            }
        }
        Err(anyhow!(
            "CONVEX_URL not found in .env.local (searched ./.env.local and proto/mls-week-3/.env.local)"
        ))
    }

    /// Call a Convex mutation. `path` is of the form `module:function`.
    pub async fn mutation(&self, path: &str, args: Value) -> Result<Value> {
        self.call("mutation", path, args).await
    }

    /// Call a Convex query. `path` is of the form `module:function`.
    pub async fn query(&self, path: &str, args: Value) -> Result<Value> {
        self.call("query", path, args).await
    }

    async fn call(&self, kind: &str, path: &str, args: Value) -> Result<Value> {
        let endpoint = format!("{}/api/{}", self.base_url, kind);
        let body = json!({ "path": path, "args": args, "format": "json" });
        let resp = self
            .http
            .post(&endpoint)
            .json(&body)
            .send()
            .await
            .with_context(|| format!("POST {}", endpoint))?;
        let status = resp.status();
        let text = resp.text().await.context("reading convex response body")?;
        if !status.is_success() {
            return Err(anyhow!("convex HTTP {}: {}", status, text));
        }
        let parsed: ConvexResponse = serde_json::from_str(&text)
            .with_context(|| format!("parsing convex response: {}", text))?;
        match parsed {
            ConvexResponse::Success { value } => Ok(value),
            ConvexResponse::Error { error_message } => {
                Err(anyhow!("convex error: {}", error_message))
            }
        }
    }
}

/// Convert a byte slice into the JSON representation Convex accepts for
/// `v.bytes()` fields.
///
/// Convex's JSON transport wraps bytes in a `{"$bytes": "<base64>"}` envelope
/// rather than using a bare base64 string; see
/// `convex/dist/esm/values/value.js` lines 108-213 for the reference
/// JS implementation. A bare base64 string fails server-side validation with
/// `ArgumentValidationError: Value does not match validator. Validator: v.bytes()`.
pub fn bytes_to_json(b: &[u8]) -> Value {
    json!({ "$bytes": B64.encode(b) })
}

/// Extract bytes from a Convex `v.bytes()` JSON value.
///
/// Accepts both the wrapped form `{"$bytes": "<base64>"}` that Convex emits
/// and a bare base64 string, for robustness.
pub fn json_to_bytes(v: &Value) -> Result<Vec<u8>> {
    let s = if let Some(obj) = v.as_object() {
        obj.get("$bytes")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("expected {{$bytes: string}}, got {}", v))?
    } else {
        v.as_str()
            .ok_or_else(|| anyhow!("expected base64 string or $bytes object, got {}", v))?
    };
    B64.decode(s).map_err(|e| anyhow!("base64 decode failed: {}", e))
}
