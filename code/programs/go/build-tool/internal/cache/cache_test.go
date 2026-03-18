package cache

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

// ---------------------------------------------------------------------------
// Tests for BuildCache
// ---------------------------------------------------------------------------

func TestNewCacheIsEmpty(t *testing.T) {
	c := New()
	entries := c.Entries()
	if len(entries) != 0 {
		t.Fatalf("expected 0 entries, got %d", len(entries))
	}
}

func TestLoadMissingFile(t *testing.T) {
	c := New()
	c.Load("/nonexistent/path/.build-cache.json")
	entries := c.Entries()
	if len(entries) != 0 {
		t.Fatalf("expected 0 entries for missing file, got %d", len(entries))
	}
}

func TestLoadMalformedJSON(t *testing.T) {
	tmpDir := t.TempDir()
	path := filepath.Join(tmpDir, "cache.json")
	os.WriteFile(path, []byte("not valid json{{{"), 0644)

	c := New()
	c.Load(path)
	entries := c.Entries()
	if len(entries) != 0 {
		t.Fatalf("expected 0 entries for malformed json, got %d", len(entries))
	}
}

func TestSaveAndLoad(t *testing.T) {
	tmpDir := t.TempDir()
	path := filepath.Join(tmpDir, "cache.json")

	// Record some entries.
	c := New()
	c.Record("python/pkg-a", "hash-a", "deps-a", "success")
	c.Record("python/pkg-b", "hash-b", "deps-b", "failed")

	if err := c.Save(path); err != nil {
		t.Fatal(err)
	}

	// Load into a fresh cache.
	c2 := New()
	c2.Load(path)
	entries := c2.Entries()

	if len(entries) != 2 {
		t.Fatalf("expected 2 entries, got %d", len(entries))
	}
	if entries["python/pkg-a"].PackageHash != "hash-a" {
		t.Errorf("expected hash-a, got %s", entries["python/pkg-a"].PackageHash)
	}
	if entries["python/pkg-b"].Status != "failed" {
		t.Errorf("expected failed, got %s", entries["python/pkg-b"].Status)
	}
}

func TestSaveAtomicity(t *testing.T) {
	tmpDir := t.TempDir()
	path := filepath.Join(tmpDir, "cache.json")

	c := New()
	c.Record("python/pkg-a", "h1", "d1", "success")
	if err := c.Save(path); err != nil {
		t.Fatal(err)
	}

	// The tmp file should not exist after save.
	tmpFile := path + ".tmp"
	if _, err := os.Stat(tmpFile); !os.IsNotExist(err) {
		t.Fatal("temporary file should be removed after atomic rename")
	}

	// The main file should exist with valid JSON.
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}

	var parsed map[string]Entry
	if err := json.Unmarshal(data, &parsed); err != nil {
		t.Fatalf("saved file is not valid JSON: %v", err)
	}
}

// ---------------------------------------------------------------------------
// Tests for NeedsBuild
// ---------------------------------------------------------------------------

func TestNeedsBuildNewPackage(t *testing.T) {
	c := New()
	if !c.NeedsBuild("python/pkg-a", "hash", "deps") {
		t.Fatal("new package should need build")
	}
}

func TestNeedsBuildCachedSuccess(t *testing.T) {
	c := New()
	c.Record("python/pkg-a", "hash-a", "deps-a", "success")

	// Same hashes — should not need build.
	if c.NeedsBuild("python/pkg-a", "hash-a", "deps-a") {
		t.Fatal("cached success with same hashes should not need build")
	}
}

func TestNeedsBuildHashChanged(t *testing.T) {
	c := New()
	c.Record("python/pkg-a", "hash-a", "deps-a", "success")

	if !c.NeedsBuild("python/pkg-a", "hash-a-CHANGED", "deps-a") {
		t.Fatal("changed package hash should need build")
	}
}

func TestNeedsBuildDepsHashChanged(t *testing.T) {
	c := New()
	c.Record("python/pkg-a", "hash-a", "deps-a", "success")

	if !c.NeedsBuild("python/pkg-a", "hash-a", "deps-a-CHANGED") {
		t.Fatal("changed deps hash should need build")
	}
}

func TestNeedsBuildPreviousFailed(t *testing.T) {
	c := New()
	c.Record("python/pkg-a", "hash-a", "deps-a", "failed")

	// Even with same hashes, failed builds should retry.
	if !c.NeedsBuild("python/pkg-a", "hash-a", "deps-a") {
		t.Fatal("previously failed build should need rebuild")
	}
}

func TestRecordOverwritesPrevious(t *testing.T) {
	c := New()
	c.Record("python/pkg-a", "hash-1", "deps-1", "failed")
	c.Record("python/pkg-a", "hash-2", "deps-2", "success")

	entries := c.Entries()
	if entries["python/pkg-a"].PackageHash != "hash-2" {
		t.Fatal("Record should overwrite previous entry")
	}
	if entries["python/pkg-a"].Status != "success" {
		t.Fatal("status should be updated")
	}
}

func TestRecordSetsTimestamp(t *testing.T) {
	c := New()
	c.Record("python/pkg-a", "h", "d", "success")

	entries := c.Entries()
	if entries["python/pkg-a"].LastBuilt == "" {
		t.Fatal("last_built should be set")
	}
}
