"""Tests for the cache module."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from build_tool.cache import BuildCache, CacheEntry


class TestBuildCache:
    """Tests for BuildCache."""

    def test_empty_cache_needs_build(self):
        cache = BuildCache()
        assert cache.needs_build("python/test", "hash1", "dhash1") is True

    def test_after_record_no_build_needed(self):
        cache = BuildCache()
        cache.record("python/test", "hash1", "dhash1", "success")
        assert cache.needs_build("python/test", "hash1", "dhash1") is False

    def test_changed_hash_needs_build(self):
        cache = BuildCache()
        cache.record("python/test", "hash1", "dhash1", "success")
        assert cache.needs_build("python/test", "hash2", "dhash1") is True

    def test_changed_deps_hash_needs_build(self):
        cache = BuildCache()
        cache.record("python/test", "hash1", "dhash1", "success")
        assert cache.needs_build("python/test", "hash1", "dhash2") is True

    def test_failed_status_needs_build(self):
        cache = BuildCache()
        cache.record("python/test", "hash1", "dhash1", "failed")
        assert cache.needs_build("python/test", "hash1", "dhash1") is True

    def test_save_and_load(self, tmp_path):
        cache_path = tmp_path / ".build-cache.json"

        cache1 = BuildCache()
        cache1.record("python/test", "hash1", "dhash1", "success")
        cache1.save(cache_path)

        cache2 = BuildCache()
        cache2.load(cache_path)
        assert cache2.needs_build("python/test", "hash1", "dhash1") is False

    def test_load_nonexistent(self, tmp_path):
        cache = BuildCache()
        cache.load(tmp_path / "nonexistent.json")
        assert cache.entries == {}

    def test_load_malformed_json(self, tmp_path):
        cache_path = tmp_path / "bad.json"
        cache_path.write_text("not json at all!")
        cache = BuildCache()
        cache.load(cache_path)
        assert cache.entries == {}

    def test_load_malformed_entries(self, tmp_path):
        cache_path = tmp_path / "partial.json"
        cache_path.write_text(json.dumps({
            "good": {
                "package_hash": "h1",
                "deps_hash": "d1",
                "last_built": "2024-01-01T00:00:00",
                "status": "success",
            },
            "bad": {"missing_fields": True},
        }))
        cache = BuildCache()
        cache.load(cache_path)
        assert "good" in cache.entries
        assert "bad" not in cache.entries

    def test_atomic_write(self, tmp_path):
        cache_path = tmp_path / ".build-cache.json"
        cache = BuildCache()
        cache.record("python/test", "hash1", "dhash1", "success")
        cache.save(cache_path)

        # Verify the file exists and is valid JSON
        assert cache_path.exists()
        data = json.loads(cache_path.read_text())
        assert "python/test" in data

        # Verify no temp file left behind
        tmp_file = tmp_path / ".build-cache.json.tmp"
        assert not tmp_file.exists()

    def test_record_updates_existing(self):
        cache = BuildCache()
        cache.record("python/test", "hash1", "dhash1", "success")
        cache.record("python/test", "hash2", "dhash2", "failed")

        entry = cache.entries["python/test"]
        assert entry.package_hash == "hash2"
        assert entry.deps_hash == "dhash2"
        assert entry.status == "failed"

    def test_entries_property(self):
        cache = BuildCache()
        cache.record("python/a", "h1", "d1", "success")
        cache.record("python/b", "h2", "d2", "failed")
        entries = cache.entries
        assert len(entries) == 2
        assert "python/a" in entries
        assert "python/b" in entries

    def test_record_has_timestamp(self):
        cache = BuildCache()
        cache.record("python/test", "h1", "d1", "success")
        entry = cache.entries["python/test"]
        assert entry.last_built  # non-empty ISO timestamp
        assert "T" in entry.last_built  # basic ISO format check
