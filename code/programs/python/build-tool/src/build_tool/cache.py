"""
cache.py -- Build Cache Management
===================================

This module manages a JSON-based cache file (``.build-cache.json``) that
records the state of each package after its last build. By comparing current
hashes against cached hashes, we determine which packages need rebuilding.

Cache format
------------

The cache file is a JSON object mapping package names to cache entries::

    {
        "python/logic-gates": {
            "package_hash": "abc123...",
            "deps_hash": "def456...",
            "last_built": "2024-01-15T10:30:00",
            "status": "success"
        },
        ...
    }

Atomic writes
-------------

To prevent corruption if the process is interrupted mid-write, we write to
a temporary file (``.build-cache.json.tmp``) first, then atomically rename
it to the final path. On POSIX systems, ``os.replace`` is atomic within the
same filesystem.
"""

from __future__ import annotations

import json
import os
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path


@dataclass
class CacheEntry:
    """A single package's cached build state.

    Attributes:
        package_hash: SHA256 of the package's source files.
        deps_hash: SHA256 of transitive dependency hashes.
        last_built: ISO 8601 timestamp of the last build.
        status: "success" or "failed".
    """

    package_hash: str
    deps_hash: str
    last_built: str
    status: str


class BuildCache:
    """Read/write interface for the build cache file.

    Usage::

        cache = BuildCache()
        cache.load(Path(".build-cache.json"))

        if cache.needs_build("python/logic-gates", pkg_hash, deps_hash):
            # ... run build ...
            cache.record("python/logic-gates", pkg_hash, deps_hash, "success")

        cache.save(Path(".build-cache.json"))
    """

    def __init__(self) -> None:
        self._entries: dict[str, CacheEntry] = {}

    def load(self, path: Path) -> None:
        """Load cache entries from a JSON file.

        If the file doesn't exist or is malformed, we start with an empty
        cache (no error raised -- a missing cache just means everything
        gets rebuilt).
        """
        if not path.exists():
            self._entries = {}
            return

        try:
            text = path.read_text(encoding="utf-8")
            data = json.loads(text)
        except (json.JSONDecodeError, OSError):
            self._entries = {}
            return

        self._entries = {}
        for name, entry_data in data.items():
            try:
                self._entries[name] = CacheEntry(
                    package_hash=entry_data["package_hash"],
                    deps_hash=entry_data["deps_hash"],
                    last_built=entry_data["last_built"],
                    status=entry_data["status"],
                )
            except (KeyError, TypeError):
                # Skip malformed entries
                continue

    def save(self, path: Path) -> None:
        """Save cache entries to a JSON file with atomic write.

        Writes to a temporary file first, then renames. This prevents
        corruption if the process is interrupted.
        """
        data = {}
        for name, entry in sorted(self._entries.items()):
            data[name] = {
                "package_hash": entry.package_hash,
                "deps_hash": entry.deps_hash,
                "last_built": entry.last_built,
                "status": entry.status,
            }

        tmp_path = path.parent / f"{path.name}.tmp"
        tmp_path.write_text(
            json.dumps(data, indent=2) + "\n", encoding="utf-8"
        )
        os.replace(str(tmp_path), str(path))

    def needs_build(self, name: str, pkg_hash: str, deps_hash: str) -> bool:
        """Determine if a package needs rebuilding.

        A package needs rebuilding if:
        1. It's not in the cache at all (never built).
        2. Its source hash changed (files were modified).
        3. Its dependency hash changed (a dependency was modified).
        4. Its last build failed.

        Args:
            name: Package name (e.g., "python/logic-gates").
            pkg_hash: Current SHA256 of the package's source files.
            deps_hash: Current SHA256 of transitive dependency hashes.

        Returns:
            True if the package should be rebuilt.
        """
        if name not in self._entries:
            return True

        entry = self._entries[name]

        if entry.status == "failed":
            return True

        if entry.package_hash != pkg_hash:
            return True

        if entry.deps_hash != deps_hash:
            return True

        return False

    def record(
        self, name: str, pkg_hash: str, deps_hash: str, status: str
    ) -> None:
        """Record a build result in the cache.

        Args:
            name: Package name.
            pkg_hash: SHA256 of the package's source files at build time.
            deps_hash: SHA256 of transitive dependency hashes at build time.
            status: "success" or "failed".
        """
        self._entries[name] = CacheEntry(
            package_hash=pkg_hash,
            deps_hash=deps_hash,
            last_built=datetime.now(timezone.utc).isoformat(),
            status=status,
        )

    @property
    def entries(self) -> dict[str, CacheEntry]:
        """Read-only access to the cache entries."""
        return dict(self._entries)
