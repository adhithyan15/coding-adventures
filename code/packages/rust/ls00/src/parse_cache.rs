//! `ParseCache` -- avoid re-parsing unchanged documents.
//!
//! # Why Cache Parse Results?
//!
//! Parsing is the most expensive operation in a language server. For a large
//! file, parsing on every keystroke would lag the editor noticeably.
//!
//! The LSP protocol helps by sending a version number with every change. If the
//! document hasn't changed (same URI, same version), the parse result from the
//! previous keystroke is still valid.
//!
//! # Cache Key Design
//!
//! The cache key is `(uri, version)`. Version is a monotonically increasing
//! integer that the editor increments with each change:
//!
//! - Same `(uri, version)` -> cache hit -> return cached result
//! - Different version -> cache miss -> re-parse and cache new result
//!
//! The old entry is evicted when a new version is cached for the same URI.
//! This keeps memory bounded at O(open_documents) entries.

use crate::language_bridge::LanguageBridge;
use crate::types::Diagnostic;
use std::any::Any;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// ParseResult
// ---------------------------------------------------------------------------

/// The outcome of parsing one version of a document.
///
/// Even on parse error, we store the partial AST and diagnostics so that other
/// features (hover, folding, symbols) can still work on the valid portions.
pub struct ParseResult {
    /// The parsed AST. May be `None` if the parser couldn't produce any AST.
    pub ast: Option<Box<dyn Any + Send + Sync>>,
    /// Parse errors and warnings.
    pub diagnostics: Vec<Diagnostic>,
    /// Non-`None` only for fatal parsing failures.
    pub err: Option<String>,
}

// ---------------------------------------------------------------------------
// CacheKey
// ---------------------------------------------------------------------------

/// Cache key combining document URI and version number.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    uri: String,
    version: i32,
}

// ---------------------------------------------------------------------------
// ParseCache
// ---------------------------------------------------------------------------

/// Stores the most recent parse result for each open document.
///
/// Create one with `ParseCache::new()`. Call `get_or_parse()` on every feature
/// request to get the current (possibly cached) parse result.
pub struct ParseCache {
    cache: HashMap<CacheKey, ParseResult>,
}

impl ParseCache {
    /// Create an empty `ParseCache`.
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Return the parse result for `(uri, version)`.
    ///
    /// If the result is already cached, it is returned immediately without
    /// calling the bridge again. Otherwise, `bridge.parse(source)` is called,
    /// the result is stored, and the previous cache entry for this URI (if any)
    /// is evicted to prevent unbounded growth.
    pub fn get_or_parse(
        &mut self,
        uri: &str,
        version: i32,
        source: &str,
        bridge: &dyn LanguageBridge,
    ) -> &ParseResult {
        let key = CacheKey {
            uri: uri.to_string(),
            version,
        };

        // Cache hit check -- must use contains_key to satisfy the borrow checker.
        if self.cache.contains_key(&key) {
            return self.cache.get(&key).unwrap();
        }

        // Cache miss: parse and store. Evict any stale entry for this URI first.
        self.evict_internal(uri);

        let result = match bridge.parse(source) {
            Ok((ast, mut diags)) => {
                if diags.is_empty() {
                    diags = Vec::new(); // normalize for JSON
                }
                ParseResult {
                    ast: Some(ast),
                    diagnostics: diags,
                    err: None,
                }
            }
            Err(e) => ParseResult {
                ast: None,
                diagnostics: Vec::new(),
                err: Some(e),
            },
        };

        self.cache.insert(key.clone(), result);
        self.cache.get(&key).unwrap()
    }

    /// Remove all cached entries for a given URI.
    ///
    /// Called when a document is closed (`didClose`) so the cache entry is
    /// cleaned up.
    pub fn evict(&mut self, uri: &str) {
        self.evict_internal(uri);
    }

    fn evict_internal(&mut self, uri: &str) {
        self.cache.retain(|k, _| k.uri != uri);
    }
}
