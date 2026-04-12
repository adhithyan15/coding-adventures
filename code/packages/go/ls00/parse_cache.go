package ls00

// parse_cache.go — ParseCache: avoid re-parsing unchanged documents
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
//   - Same (uri, version) → cache hit → return cached result
//   - Different version  → cache miss → re-parse and cache new result
//
// The old entry is evicted when a new version is cached for the same URI.
// This keeps memory bounded at O(open_documents) entries.
//
// # Thread Safety
//
// The ParseCache is NOT goroutine-safe. This is intentional: the LspServer
// processes one message at a time (single-threaded), so no locking is needed.
// If concurrent request processing is added later, a mutex would be needed here.

// ParseResult holds the outcome of parsing one version of a document.
//
// Even on parse error, we store the partial AST and diagnostics so that other
// features (hover, folding, symbols) can still work on the valid portions.
type ParseResult struct {
	AST         ASTNode    // may be nil if the parser couldn't produce any AST
	Diagnostics []Diagnostic
	Err         error // non-nil only for fatal parsing failures
}

// cacheKey is the key for the parse cache. It combines the document URI and
// the version number so that two versions of the same file have distinct keys.
type cacheKey struct {
	uri     string
	version int
}

// ParseCache stores the most recent parse result for each open document.
//
// Create one with NewParseCache(). Call GetOrParse() on every feature request
// to get the current (possibly cached) parse result.
type ParseCache struct {
	cache map[cacheKey]*ParseResult // key=(uri,version) → result
}

// NewParseCache creates an empty ParseCache.
func NewParseCache() *ParseCache {
	return &ParseCache{cache: make(map[cacheKey]*ParseResult)}
}

// GetOrParse returns the parse result for (uri, version).
//
// If the result is already cached, it is returned immediately without calling
// the bridge again. Otherwise, bridge.Parse(source) is called, the result is
// stored, and the previous cache entry for this URI (if any) is evicted to
// prevent unbounded growth.
//
// This function is the single point of truth for "what is the parsed state of
// this document right now?" All feature handlers call it before operating on the AST.
func (pc *ParseCache) GetOrParse(uri string, version int, source string, bridge LanguageBridge) *ParseResult {
	key := cacheKey{uri: uri, version: version}

	// Cache hit: the document hasn't changed since last parse.
	if result, ok := pc.cache[key]; ok {
		return result
	}

	// Cache miss: parse and store. Evict any stale entry for this URI first.
	pc.evict(uri)

	ast, diags, err := bridge.Parse(source)
	if diags == nil {
		diags = []Diagnostic{} // normalize nil slice to empty slice for JSON
	}

	result := &ParseResult{
		AST:         ast,
		Diagnostics: diags,
		Err:         err,
	}
	pc.cache[key] = result
	return result
}

// Evict removes all cached entries for a given URI.
//
// Called when a document is closed (didClose) so the cache entry is cleaned up.
// Also called internally before adding a new entry to prevent memory growth.
func (pc *ParseCache) Evict(uri string) {
	pc.evict(uri)
}

// evict is the internal eviction implementation.
func (pc *ParseCache) evict(uri string) {
	for k := range pc.cache {
		if k.uri == uri {
			delete(pc.cache, k)
		}
	}
}
