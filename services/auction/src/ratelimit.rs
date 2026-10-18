//! Per-client token bucket for the public API. Keyed by the connecting address, or by the first
//! `x-forwarded-for` entry only when `TRUST_PROXY` says a reverse proxy sets it. Anonymous write and
//! lookup endpoints (envelopes, receive addresses, proofs, standing) are the ones worth protecting:
//! each derives an address or does a query on behalf of an unauthenticated caller.

use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::Mutex,
    time::Instant,
};

use axum::{
    extract::{ConnectInfo, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};

pub struct Limiter {
    capacity: f64,
    refill_per_sec: f64,
    trust_proxy: bool,
    buckets: Mutex<HashMap<IpAddr, (f64, Instant)>>,
}

impl Limiter {
    pub fn new(capacity: u32, refill_per_sec: f64, trust_proxy: bool) -> Self {
        Self { capacity: capacity as f64, refill_per_sec, trust_proxy, buckets: Mutex::new(HashMap::new()) }
    }

    fn key(&self, headers: &axum::http::HeaderMap, peer: Option<SocketAddr>) -> IpAddr {
        if self.trust_proxy {
            if let Some(ip) = headers
                .get("x-forwarded-for")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.split(',').next())
                .and_then(|v| v.trim().parse::<IpAddr>().ok())
            {
                return ip;
            }
        }
        peer.map(|p| p.ip()).unwrap_or(IpAddr::from([127, 0, 0, 1]))
    }

    /// Take one token for `ip`; false when the bucket is empty.
    pub fn allow(&self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let mut b = self.buckets.lock().unwrap();
        if b.len() > 50_000 {
            b.retain(|_, (_, t)| now.duration_since(*t).as_secs() < 600);
        }
        let (tokens, updated) = b.entry(ip).or_insert((self.capacity, now));
        *tokens = (*tokens + now.duration_since(*updated).as_secs_f64() * self.refill_per_sec).min(self.capacity);
        *updated = now;
        if *tokens < 1.0 {
            return false;
        }
        *tokens -= 1.0;
        true
    }
}

/// axum middleware: 429 when the client's bucket is empty.
pub async fn limit(
    State(limiter): State<std::sync::Arc<Limiter>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next,
) -> Response {
    let ip = limiter.key(req.headers(), Some(peer));
    if !limiter.allow(ip) {
        return (StatusCode::TOO_MANY_REQUESTS, axum::Json(serde_json::json!({ "error": "too many requests; slow down" }))).into_response();
    }
    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bucket_empties_and_refills() {
        let l = Limiter::new(3, 1000.0, false);
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        assert!(l.allow(ip) && l.allow(ip) && l.allow(ip));
        assert!(!l.allow(ip), "fourth call in the same instant is refused");
        std::thread::sleep(std::time::Duration::from_millis(5));
        assert!(l.allow(ip), "refilled");
    }

    #[test]
    fn forwarded_header_only_counts_behind_a_trusted_proxy() {
        let mut h = axum::http::HeaderMap::new();
        h.insert("x-forwarded-for", "203.0.113.9, 10.0.0.2".parse().unwrap());
        let peer: SocketAddr = "10.0.0.2:4000".parse().unwrap();
        assert_eq!(Limiter::new(1, 1.0, false).key(&h, Some(peer)).to_string(), "10.0.0.2");
        assert_eq!(Limiter::new(1, 1.0, true).key(&h, Some(peer)).to_string(), "203.0.113.9");
    }
}
