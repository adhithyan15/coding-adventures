"""Tests for ParseCache — cache hit/miss behavior.

The ParseCache avoids re-parsing unchanged documents. These tests verify
the (uri, version) keyed caching, eviction, and diagnostic propagation.
"""

from __future__ import annotations

from ls00 import ParseCache, Diagnostic, DiagnosticSeverity, Position, Range
from ls00.language_bridge import LanguageBridge
from ls00.types import Token
from typing import Any


class _MockBridge:
    """Minimal bridge for testing the parse cache."""

    def tokenize(self, source: str) -> list[Token]:
        return []

    def parse(self, source: str) -> tuple[Any, list[Diagnostic]]:
        diags: list[Diagnostic] = []
        if "ERROR" in source:
            diags.append(Diagnostic(
                range=Range(
                    start=Position(line=0, character=0),
                    end=Position(line=0, character=5),
                ),
                severity=DiagnosticSeverity.ERROR,
                message="syntax error",
            ))
        return source, diags


class TestParseCacheHitAndMiss:
    """Tests for cache hit and miss behavior."""

    def test_cache_miss_then_hit(self) -> None:
        """First call is a miss (parses), second call with same version is a hit."""
        bridge = _MockBridge()
        cache = ParseCache()

        r1 = cache.get_or_parse("file:///a.txt", 1, "hello", bridge)
        assert r1 is not None

        r2 = cache.get_or_parse("file:///a.txt", 1, "hello", bridge)
        assert r1 is r2  # same object = cache hit

    def test_different_version_is_miss(self) -> None:
        """Different version produces a new parse result."""
        bridge = _MockBridge()
        cache = ParseCache()

        r1 = cache.get_or_parse("file:///a.txt", 1, "hello", bridge)
        r3 = cache.get_or_parse("file:///a.txt", 2, "hello world", bridge)
        assert r3 is not r1

    def test_evict_forces_reparse(self) -> None:
        """After eviction, same (uri, version) produces a new parse."""
        bridge = _MockBridge()
        cache = ParseCache()

        r1 = cache.get_or_parse("file:///a.txt", 1, "hello", bridge)
        cache.evict("file:///a.txt")

        r2 = cache.get_or_parse("file:///a.txt", 1, "hello", bridge)
        assert r1 is not r2

    def test_diagnostics_populated_for_error_source(self) -> None:
        """Source containing 'ERROR' produces diagnostics."""
        bridge = _MockBridge()
        cache = ParseCache()

        result = cache.get_or_parse(
            "file:///a.txt", 1, "source with ERROR token", bridge
        )
        assert len(result.diagnostics) > 0

    def test_no_diagnostics_for_clean_source(self) -> None:
        """Clean source produces no diagnostics."""
        bridge = _MockBridge()
        cache = ParseCache()

        result = cache.get_or_parse("file:///clean.txt", 1, "hello world", bridge)
        assert len(result.diagnostics) == 0
