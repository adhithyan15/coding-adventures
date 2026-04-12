// ============================================================================
// ParseCache.swift — avoid re-parsing unchanged documents
// ============================================================================
//
// # Why Cache Parse Results?
//
// Parsing is the most expensive operation in a language server. For a large
// file, parsing on every keystroke would lag the editor noticeably.
//
// The LSP protocol helps by sending a version number with every change. If the
// document hasn't changed (same URI, same version), the parse result from the
// previous keystroke is still valid.
//
// # Cache Key Design
//
// The cache key is (uri, version). Version is a monotonically increasing integer
// that the editor increments with each change. Using version in the key means:
//
//   - Same (uri, version) -> cache hit -> return cached result
//   - Different version  -> cache miss -> re-parse and cache new result
//
// The old entry is evicted when a new version is cached for the same URI.
// This keeps memory bounded at O(open_documents) entries.
//
// # Thread Safety
//
// The ParseCache is NOT thread-safe. This is intentional: the LspServer
// processes one message at a time (single-threaded), so no locking is needed.
//
// ============================================================================

import Foundation

// ============================================================================
// ParseResult — the outcome of parsing one version of a document
// ============================================================================

/// Holds the outcome of parsing one version of a document.
///
/// Even on parse error, we store the partial AST and diagnostics so that other
/// features (hover, folding, symbols) can still work on the valid portions.
public class ParseResult {
    /// The parsed AST. May be nil if the parser couldn't produce any AST.
    public let ast: ASTNode?

    /// Diagnostics (errors, warnings) from the parse.
    public let diagnostics: [Diagnostic]

    /// Non-nil only for fatal parsing failures.
    public let error: Error?

    public init(ast: ASTNode?, diagnostics: [Diagnostic], error: Error? = nil) {
        self.ast = ast
        self.diagnostics = diagnostics
        self.error = error
    }
}

// ============================================================================
// CacheKey — internal key for the cache
// ============================================================================

/// Internal cache key combining URI and version.
private struct CacheKey: Hashable {
    let uri: String
    let version: Int
}

// ============================================================================
// ParseCache
// ============================================================================

/// Stores the most recent parse result for each open document.
///
/// Create one with `ParseCache()`. Call `getOrParse()` on every feature
/// request to get the current (possibly cached) parse result.
public class ParseCache {
    private var cache: [CacheKey: ParseResult] = [:]

    public init() {}

    /// Return the parse result for (uri, version), re-parsing if needed.
    ///
    /// If the result is already cached, it returns immediately without
    /// calling the bridge. Otherwise, `bridge.parse(source)` is called,
    /// the result is stored, and any previous entry for this URI is evicted.
    ///
    /// This is the single point of truth for "what is the parsed state of
    /// this document right now?"
    public func getOrParse(uri: String, version: Int, source: String, bridge: LanguageBridge) -> ParseResult {
        let key = CacheKey(uri: uri, version: version)

        // Cache hit: document hasn't changed since last parse.
        if let result = cache[key] {
            return result
        }

        // Cache miss: parse and store. Evict stale entries first.
        evictInternal(uri)

        let (ast, diags, err) = bridge.parse(source: source)
        let diagnostics = diags.isEmpty ? [] : diags

        let result = ParseResult(ast: ast, diagnostics: diagnostics, error: err)
        cache[key] = result
        return result
    }

    /// Remove all cached entries for a given URI.
    ///
    /// Called when a document is closed (didClose).
    public func evict(uri: String) {
        evictInternal(uri)
    }

    private func evictInternal(_ uri: String) {
        for key in cache.keys where key.uri == uri {
            cache.removeValue(forKey: key)
        }
    }
}
