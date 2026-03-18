// Package cache manages a JSON-based build cache file (.build-cache.json)
// that records the state of each package after its last build.
//
// # Why caching?
//
// Without caching, every "build" would rebuild every package — even those
// whose source files haven't changed. This is wasteful for large monorepos.
// The cache records the SHA256 hash of each package's source files and
// dependencies at build time. On the next build, we compare current hashes
// against cached hashes to determine which packages actually need rebuilding.
//
// # Cache format
//
// The cache file is a JSON object mapping package names to cache entries:
//
//	{
//	    "python/logic-gates": {
//	        "package_hash": "abc123...",
//	        "deps_hash": "def456...",
//	        "last_built": "2024-01-15T10:30:00Z",
//	        "status": "success"
//	    }
//	}
//
// # Atomic writes
//
// To prevent corruption if the process is interrupted mid-write, we write
// to a temporary file first, then atomically rename it. On POSIX systems,
// os.Rename is atomic within the same filesystem.
package cache

import (
	"encoding/json"
	"os"
	"sort"
	"sync"
	"time"
)

// Entry represents a single package's cached build state.
type Entry struct {
	PackageHash string `json:"package_hash"` // SHA256 of source files
	DepsHash    string `json:"deps_hash"`    // SHA256 of dependency hashes
	LastBuilt   string `json:"last_built"`   // ISO 8601 timestamp
	Status      string `json:"status"`       // "success" or "failed"
}

// BuildCache provides a read/write interface for the build cache file.
// It is safe for concurrent reads after loading, but writes (Record, Save)
// should be synchronized by the caller or use the built-in mutex.
type BuildCache struct {
	mu      sync.Mutex
	entries map[string]Entry
}

// New creates an empty BuildCache.
func New() *BuildCache {
	return &BuildCache{
		entries: make(map[string]Entry),
	}
}

// Load reads cache entries from a JSON file. If the file doesn't exist
// or is malformed, we start with an empty cache — no error is raised.
// A missing cache simply means everything gets rebuilt, which is the
// safe default.
func (c *BuildCache) Load(path string) {
	c.mu.Lock()
	defer c.mu.Unlock()

	data, err := os.ReadFile(path)
	if err != nil {
		c.entries = make(map[string]Entry)
		return
	}

	var raw map[string]Entry
	if err := json.Unmarshal(data, &raw); err != nil {
		c.entries = make(map[string]Entry)
		return
	}

	c.entries = raw
}

// Save writes cache entries to a JSON file with atomic write.
//
// The atomicity guarantee: we write to path + ".tmp" first, then rename.
// If the process crashes during the write, the original cache file is
// untouched. If it crashes during the rename, the temporary file may
// be left behind, but no data is lost.
func (c *BuildCache) Save(path string) error {
	c.mu.Lock()
	defer c.mu.Unlock()

	// Sort entries by key for deterministic output.
	keys := make([]string, 0, len(c.entries))
	for k := range c.entries {
		keys = append(keys, k)
	}
	sort.Strings(keys)

	ordered := make(map[string]Entry, len(c.entries))
	for _, k := range keys {
		ordered[k] = c.entries[k]
	}

	data, err := json.MarshalIndent(ordered, "", "  ")
	if err != nil {
		return err
	}
	data = append(data, '\n')

	tmpPath := path + ".tmp"
	if err := os.WriteFile(tmpPath, data, 0644); err != nil {
		return err
	}

	return os.Rename(tmpPath, path)
}

// NeedsBuild determines if a package needs rebuilding. A package needs
// rebuilding if any of these conditions hold:
//
//  1. It's not in the cache (never built before).
//  2. Its source hash changed (files were modified).
//  3. Its dependency hash changed (a dependency was modified).
//  4. Its last build failed.
//
// This is the decision function at the heart of incremental builds.
func (c *BuildCache) NeedsBuild(name, pkgHash, depsHash string) bool {
	c.mu.Lock()
	defer c.mu.Unlock()

	entry, ok := c.entries[name]
	if !ok {
		return true
	}
	if entry.Status == "failed" {
		return true
	}
	if entry.PackageHash != pkgHash {
		return true
	}
	if entry.DepsHash != depsHash {
		return true
	}
	return false
}

// Record stores a build result in the cache.
func (c *BuildCache) Record(name, pkgHash, depsHash, status string) {
	c.mu.Lock()
	defer c.mu.Unlock()

	c.entries[name] = Entry{
		PackageHash: pkgHash,
		DepsHash:    depsHash,
		LastBuilt:   time.Now().UTC().Format(time.RFC3339),
		Status:      status,
	}
}

// Entries returns a copy of all cache entries (for inspection/testing).
func (c *BuildCache) Entries() map[string]Entry {
	c.mu.Lock()
	defer c.mu.Unlock()

	result := make(map[string]Entry, len(c.entries))
	for k, v := range c.entries {
		result[k] = v
	}
	return result
}
