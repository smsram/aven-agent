use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Safe metadata for a credential to expose to the frontend (never reveals secrets)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CredentialHealthMetadata {
    pub index: usize,
    pub masked: String,
    pub is_healthy: bool,
    pub failure_count: usize,
    pub cooldown_seconds_remaining: u64,
}

/// Provider credential pool status safe for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderPoolStatus {
    pub provider: String,
    pub total_keys: usize,
    pub active_index: usize,
    pub keys: Vec<CredentialHealthMetadata>,
}

/// Categorized failure type for intelligent failover and cooldown rules
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureCategory {
    AuthFailure,         // 401, 403: Invalid credentials or token expired
    RateLimit,           // 429: Too many requests
    QuotaExhausted,      // 402, 429 quota: Account ran out of credits / billing limit
    ServerOrNetwork,     // 5xx, timeouts, connection drop
    PermanentRequestErr, // 400 bad request, 404 model not found (do NOT rotate keys)
}

/// Mask secret string for safe display and logging (e.g. "sk-a1b2...e5f6")
pub fn mask_secret(secret: &str) -> String {
    let trimmed = secret.trim();
    if trimmed.is_empty() {
        return "****".to_string();
    }
    if trimmed.len() <= 8 {
        format!("{}****", &trimmed[..1.min(trimmed.len())])
    } else {
        format!("{}...{}", &trimmed[..4], &trimmed[trimmed.len() - 4..])
    }
}

/// Parse and validate comma-separated API keys.
/// Rules:
/// - Trims whitespace around every entry.
/// - Ignores empty entries caused by leading, trailing, or repeated commas.
/// - Rejects internal whitespace inside individual keys.
/// - Returns at least one key on success.
pub fn parse_and_validate_keys(raw: &str) -> Result<Vec<String>, String> {
    let keys: Vec<String> = raw
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if keys.is_empty() {
        return Err("No API keys provided. Please enter at least one valid API key.".to_string());
    }

    for (i, key) in keys.iter().enumerate() {
        if key.contains(' ') {
            return Err(format!(
                "API key #{} contains invalid internal whitespace.",
                i + 1
            ));
        }
    }

    Ok(keys)
}

/// A resilient credential pool for any AI provider supporting multiple API keys
#[derive(Debug, Clone)]
pub struct ProviderCredentialPool {
    pub provider_name: String,
    pub keys: Vec<String>,
    pub preferred_index: usize,
    pub cooldowns: HashMap<usize, Instant>,
    pub failure_counts: HashMap<usize, usize>,
    pub exhausted_indices: Vec<usize>,
}

impl ProviderCredentialPool {
    pub fn new(provider_name: impl Into<String>, keys: Vec<String>) -> Self {
        Self {
            provider_name: provider_name.into(),
            keys,
            preferred_index: 0,
            cooldowns: HashMap::new(),
            failure_counts: HashMap::new(),
            exhausted_indices: Vec::new(),
        }
    }

    pub fn set_keys(&mut self, keys: Vec<String>) {
        self.keys = keys;
        self.preferred_index = 0;
        self.cooldowns.clear();
        self.failure_counts.clear();
        self.exhausted_indices.clear();
    }

    /// Return candidate key indices in failover priority order.
    /// Healthy keys first, then cooldown keys sorted by earliest expiration.
    /// Permanently exhausted keys are placed last.
    pub fn get_candidate_indices(&self) -> Vec<usize> {
        let n = self.keys.len();
        if n == 0 {
            return Vec::new();
        }

        let now = Instant::now();
        let mut available = Vec::new();
        let mut in_cooldown = Vec::new();
        let mut exhausted = Vec::new();

        for step in 0..n {
            let idx = (self.preferred_index + step) % n;
            if self.exhausted_indices.contains(&idx) {
                exhausted.push(idx);
                continue;
            }

            if let Some(exp) = self.cooldowns.get(&idx) {
                if *exp > now {
                    in_cooldown.push((idx, *exp));
                    continue;
                }
            }

            available.push(idx);
        }

        if available.is_empty() && in_cooldown.is_empty() {
            return exhausted;
        }

        if available.is_empty() {
            in_cooldown.sort_by_key(|&(_, exp)| exp);
            let mut res: Vec<usize> = in_cooldown.into_iter().map(|(idx, _)| idx).collect();
            res.extend(exhausted);
            return res;
        }

        in_cooldown.sort_by_key(|&(_, exp)| exp);
        for (idx, _) in in_cooldown {
            available.push(idx);
        }
        available.extend(exhausted);

        available
    }

    pub fn mark_success(&mut self, index: usize) {
        if index < self.keys.len() {
            self.preferred_index = index;
            self.cooldowns.remove(&index);
            self.failure_counts.insert(index, 0);
            if let Some(pos) = self.exhausted_indices.iter().position(|&x| x == index) {
                self.exhausted_indices.remove(pos);
            }
        }
    }

    pub fn mark_failure(
        &mut self,
        index: usize,
        category: FailureCategory,
        retry_after_secs: Option<u64>,
    ) {
        let count = self.failure_counts.entry(index).or_insert(0);
        *count += 1;

        let duration = match category {
            FailureCategory::AuthFailure => Duration::from_secs(300), // 5 min cooldown for invalid/unauthorized
            FailureCategory::RateLimit => {
                Duration::from_secs(retry_after_secs.unwrap_or(30).clamp(5, 300))
            }
            FailureCategory::QuotaExhausted => {
                if !self.exhausted_indices.contains(&index) {
                    self.exhausted_indices.push(index);
                }
                Duration::from_secs(3600) // 1 hour cooldown for exhausted quota
            }
            FailureCategory::ServerOrNetwork => Duration::from_secs(15), // Short cooldown for transient network
            FailureCategory::PermanentRequestErr => Duration::ZERO,
        };

        if duration > Duration::ZERO {
            self.cooldowns.insert(index, Instant::now() + duration);
        }
    }

    pub fn get_status(&self) -> ProviderPoolStatus {
        let now = Instant::now();
        let metadata = self
            .keys
            .iter()
            .enumerate()
            .map(|(i, key)| {
                let remaining = self
                    .cooldowns
                    .get(&i)
                    .and_then(|exp| {
                        if *exp > now {
                            Some(exp.duration_since(now).as_secs())
                        } else {
                            None
                        }
                    })
                    .unwrap_or(0);

                let is_exhausted = self.exhausted_indices.contains(&i);
                let is_healthy = remaining == 0 && !is_exhausted;

                CredentialHealthMetadata {
                    index: i,
                    masked: mask_secret(key),
                    is_healthy,
                    failure_count: *self.failure_counts.get(&i).unwrap_or(&0),
                    cooldown_seconds_remaining: remaining,
                }
            })
            .collect();

        ProviderPoolStatus {
            provider: self.provider_name.clone(),
            total_keys: self.keys.len(),
            active_index: self.preferred_index,
            keys: metadata,
        }
    }
}

/// Global registry of credential pools across all supported providers
pub struct GlobalCredentialRegistry {
    pools: Arc<Mutex<HashMap<String, ProviderCredentialPool>>>,
}

impl GlobalCredentialRegistry {
    pub fn new() -> Self {
        Self {
            pools: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn register_or_update(&self, provider: &str, keys: Vec<String>) {
        let mut map = self.pools.lock().await;
        if let Some(pool) = map.get_mut(provider) {
            pool.set_keys(keys);
        } else {
            map.insert(
                provider.to_string(),
                ProviderCredentialPool::new(provider, keys),
            );
        }
    }

    pub async fn get_candidates(&self, provider: &str) -> Vec<(usize, String)> {
        let map = self.pools.lock().await;
        if let Some(pool) = map.get(provider) {
            let candidate_indices = pool.get_candidate_indices();
            candidate_indices
                .into_iter()
                .filter_map(|idx| pool.keys.get(idx).map(|k| (idx, k.clone())))
                .collect()
        } else {
            Vec::new()
        }
    }

    pub async fn mark_success(&self, provider: &str, index: usize) {
        let mut map = self.pools.lock().await;
        if let Some(pool) = map.get_mut(provider) {
            pool.mark_success(index);
        }
    }

    pub async fn mark_failure(
        &self,
        provider: &str,
        index: usize,
        category: FailureCategory,
        retry_after: Option<u64>,
    ) {
        let mut map = self.pools.lock().await;
        if let Some(pool) = map.get_mut(provider) {
            pool.mark_failure(index, category, retry_after);
        }
    }

    pub async fn get_status(&self, provider: &str) -> Option<ProviderPoolStatus> {
        let map = self.pools.lock().await;
        map.get(provider).map(|p| p.get_status())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_and_validate_keys() {
        let raw = " sk-key1 , sk-key2,  sk-key3 ,, ";
        let res = parse_and_validate_keys(raw).unwrap();
        assert_eq!(res, vec!["sk-key1", "sk-key2", "sk-key3"]);

        assert!(parse_and_validate_keys("").is_err());
        assert!(parse_and_validate_keys("  , , ").is_err());
        assert!(parse_and_validate_keys("key1, key 2 with space").is_err());
    }

    #[test]
    fn test_mask_secret() {
        assert_eq!(mask_secret("sk-1234567890abcdef"), "sk-1...cdef");
        assert_eq!(mask_secret("short"), "s****");
        assert_eq!(mask_secret(""), "****");
    }

    #[test]
    fn test_pool_rotation_and_cooldown() {
        let keys = vec!["k1".into(), "k2".into(), "k3".into()];
        let mut pool = ProviderCredentialPool::new("test_provider", keys);

        // Initial candidates start with 0
        assert_eq!(pool.get_candidate_indices(), vec![0, 1, 2]);

        // Key 0 fails with 429
        pool.mark_failure(0, FailureCategory::RateLimit, Some(60));
        assert_eq!(pool.get_candidate_indices(), vec![1, 2, 0]);

        // Key 1 succeeds -> becomes preferred
        pool.mark_success(1);
        assert_eq!(pool.preferred_index, 1);
        assert_eq!(pool.get_candidate_indices(), vec![1, 2, 0]);

        // Key 1 exhausts quota
        pool.mark_failure(1, FailureCategory::QuotaExhausted, None);
        // Key 2 is now the primary candidate
        assert_eq!(pool.get_candidate_indices(), vec![2, 0, 1]);
    }

    #[test]
    fn test_permanent_request_error_does_not_trigger_cooldown() {
        let keys = vec!["k1".into(), "k2".into()];
        let mut pool = ProviderCredentialPool::new("test_provider", keys);

        // Permanent 400 error
        pool.mark_failure(0, FailureCategory::PermanentRequestErr, None);
        // Cooldown shouldn't be set
        assert!(!pool.cooldowns.contains_key(&0));
        // Still available in normal order
        assert_eq!(pool.get_candidate_indices(), vec![0, 1]);
    }

    #[test]
    fn test_all_credentials_exhausted_fallback() {
        let keys = vec!["k1".into(), "k2".into()];
        let mut pool = ProviderCredentialPool::new("test_provider", keys);

        pool.mark_failure(0, FailureCategory::QuotaExhausted, None);
        pool.mark_failure(1, FailureCategory::QuotaExhausted, None);

        // Even if all are exhausted, get_candidate_indices returns all candidates rather than an empty list
        let candidates = pool.get_candidate_indices();
        assert_eq!(candidates.len(), 2);
    }
}
