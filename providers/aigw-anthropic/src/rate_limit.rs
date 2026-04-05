//! Rate limit information parsed from Anthropic API response headers.
//!
//! Every Anthropic API response includes `anthropic-ratelimit-*` headers that
//! describe the current rate limit state. This module provides typed access
//! to those values.
//!
//! See <https://docs.anthropic.com/en/api/rate-limits>

use reqwest::header::HeaderMap;

/// Rate limit information extracted from Anthropic API response headers.
///
/// All fields are `Option` because a proxy or non-standard deployment may
/// omit some or all of these headers.
///
/// # Example
///
/// ```
/// use aigw_anthropic::RateLimitInfo;
/// use reqwest::header::HeaderMap;
///
/// let mut headers = HeaderMap::new();
/// headers.insert("anthropic-ratelimit-requests-limit", "100".parse().unwrap());
/// headers.insert("anthropic-ratelimit-requests-remaining", "95".parse().unwrap());
/// headers.insert("anthropic-ratelimit-tokens-limit", "100000".parse().unwrap());
/// headers.insert("anthropic-ratelimit-tokens-remaining", "90000".parse().unwrap());
///
/// let info = RateLimitInfo::from_headers(&headers);
/// assert_eq!(info.requests_limit, Some(100));
/// assert_eq!(info.tokens_remaining, Some(90000));
/// ```
#[derive(Debug, Clone, Default)]
pub struct RateLimitInfo {
    /// Maximum requests allowed in the current window.
    pub requests_limit: Option<u64>,
    /// Remaining requests in the current window.
    pub requests_remaining: Option<u64>,
    /// When the request limit resets (RFC 3339 timestamp).
    pub requests_reset: Option<String>,

    /// Maximum tokens allowed in the current window.
    pub tokens_limit: Option<u64>,
    /// Remaining tokens in the current window.
    pub tokens_remaining: Option<u64>,
    /// When the token limit resets (RFC 3339 timestamp).
    pub tokens_reset: Option<String>,

    /// Maximum input tokens allowed in the current window.
    pub input_tokens_limit: Option<u64>,
    /// Remaining input tokens in the current window.
    pub input_tokens_remaining: Option<u64>,
    /// When the input token limit resets (RFC 3339 timestamp).
    pub input_tokens_reset: Option<String>,

    /// Maximum output tokens allowed in the current window.
    pub output_tokens_limit: Option<u64>,
    /// Remaining output tokens in the current window.
    pub output_tokens_remaining: Option<u64>,
    /// When the output token limit resets (RFC 3339 timestamp).
    pub output_tokens_reset: Option<String>,

    /// Seconds to wait before retrying (present on 429 responses).
    pub retry_after: Option<u64>,
}

impl RateLimitInfo {
    /// Parse rate limit information from response headers.
    ///
    /// Missing or unparseable headers are silently ignored (set to `None`).
    pub fn from_headers(headers: &HeaderMap) -> Self {
        Self {
            requests_limit: parse_u64(headers, "anthropic-ratelimit-requests-limit"),
            requests_remaining: parse_u64(headers, "anthropic-ratelimit-requests-remaining"),
            requests_reset: parse_string(headers, "anthropic-ratelimit-requests-reset"),

            tokens_limit: parse_u64(headers, "anthropic-ratelimit-tokens-limit"),
            tokens_remaining: parse_u64(headers, "anthropic-ratelimit-tokens-remaining"),
            tokens_reset: parse_string(headers, "anthropic-ratelimit-tokens-reset"),

            input_tokens_limit: parse_u64(headers, "anthropic-ratelimit-input-tokens-limit"),
            input_tokens_remaining: parse_u64(
                headers,
                "anthropic-ratelimit-input-tokens-remaining",
            ),
            input_tokens_reset: parse_string(headers, "anthropic-ratelimit-input-tokens-reset"),

            output_tokens_limit: parse_u64(headers, "anthropic-ratelimit-output-tokens-limit"),
            output_tokens_remaining: parse_u64(
                headers,
                "anthropic-ratelimit-output-tokens-remaining",
            ),
            output_tokens_reset: parse_string(headers, "anthropic-ratelimit-output-tokens-reset"),

            retry_after: parse_u64(headers, "retry-after"),
        }
    }

    /// Returns `true` if no rate limit headers were found at all.
    pub fn is_empty(&self) -> bool {
        self.requests_limit.is_none()
            && self.requests_remaining.is_none()
            && self.tokens_limit.is_none()
            && self.tokens_remaining.is_none()
            && self.input_tokens_limit.is_none()
            && self.input_tokens_remaining.is_none()
            && self.output_tokens_limit.is_none()
            && self.output_tokens_remaining.is_none()
    }
}

/// An API response body paired with rate limit information from the response headers.
///
/// ```ignore
/// let resp = client.messages(&req).await?;
/// println!("{}", resp.body.id);
/// if let Some(remaining) = resp.rate_limit.tokens_remaining {
///     println!("tokens remaining: {remaining}");
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ApiResponse<T> {
    /// The parsed response body.
    pub body: T,
    /// Rate limit state from the response headers.
    pub rate_limit: RateLimitInfo,
}

fn parse_u64(headers: &HeaderMap, name: &str) -> Option<u64> {
    headers.get(name)?.to_str().ok()?.trim().parse::<u64>().ok()
}

fn parse_string(headers: &HeaderMap, name: &str) -> Option<String> {
    Some(headers.get(name)?.to_str().ok()?.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use reqwest::header::HeaderMap;

    use super::RateLimitInfo;

    #[test]
    fn parse_all_headers() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "anthropic-ratelimit-requests-limit",
            "1000".parse().unwrap(),
        );
        headers.insert(
            "anthropic-ratelimit-requests-remaining",
            "999".parse().unwrap(),
        );
        headers.insert(
            "anthropic-ratelimit-requests-reset",
            "2026-04-05T12:00:00Z".parse().unwrap(),
        );
        headers.insert(
            "anthropic-ratelimit-tokens-limit",
            "100000".parse().unwrap(),
        );
        headers.insert(
            "anthropic-ratelimit-tokens-remaining",
            "90000".parse().unwrap(),
        );
        headers.insert(
            "anthropic-ratelimit-tokens-reset",
            "2026-04-05T12:00:00Z".parse().unwrap(),
        );
        headers.insert(
            "anthropic-ratelimit-input-tokens-limit",
            "80000".parse().unwrap(),
        );
        headers.insert(
            "anthropic-ratelimit-input-tokens-remaining",
            "70000".parse().unwrap(),
        );
        headers.insert(
            "anthropic-ratelimit-input-tokens-reset",
            "2026-04-05T12:00:00Z".parse().unwrap(),
        );
        headers.insert(
            "anthropic-ratelimit-output-tokens-limit",
            "20000".parse().unwrap(),
        );
        headers.insert(
            "anthropic-ratelimit-output-tokens-remaining",
            "18000".parse().unwrap(),
        );
        headers.insert(
            "anthropic-ratelimit-output-tokens-reset",
            "2026-04-05T12:00:00Z".parse().unwrap(),
        );
        headers.insert("retry-after", "30".parse().unwrap());

        let info = RateLimitInfo::from_headers(&headers);

        assert_eq!(info.requests_limit, Some(1000));
        assert_eq!(info.requests_remaining, Some(999));
        assert_eq!(info.requests_reset.as_deref(), Some("2026-04-05T12:00:00Z"));
        assert_eq!(info.tokens_limit, Some(100_000));
        assert_eq!(info.tokens_remaining, Some(90_000));
        assert_eq!(info.input_tokens_limit, Some(80_000));
        assert_eq!(info.input_tokens_remaining, Some(70_000));
        assert_eq!(info.output_tokens_limit, Some(20_000));
        assert_eq!(info.output_tokens_remaining, Some(18_000));
        assert_eq!(info.retry_after, Some(30));
        assert!(!info.is_empty());
    }

    #[test]
    fn empty_headers() {
        let info = RateLimitInfo::from_headers(&HeaderMap::new());
        assert!(info.is_empty());
        assert_eq!(info.requests_limit, None);
        assert_eq!(info.retry_after, None);
    }

    #[test]
    fn malformed_values_ignored() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "anthropic-ratelimit-requests-limit",
            "not-a-number".parse().unwrap(),
        );
        headers.insert(
            "anthropic-ratelimit-tokens-remaining",
            "50000".parse().unwrap(),
        );

        let info = RateLimitInfo::from_headers(&headers);
        assert_eq!(info.requests_limit, None);
        assert_eq!(info.tokens_remaining, Some(50_000));
    }
}
