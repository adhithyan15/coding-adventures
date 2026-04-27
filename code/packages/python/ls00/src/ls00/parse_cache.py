"""ParseCache — avoid re-parsing unchanged documents.

Why Cache Parse Results?
--------------------------

Parsing is the most expensive operation in a language server. For a large
file, parsing on every keystroke would lag the editor noticeably.

The LSP protocol helps by sending a version number with every change. If
the document hasn't changed (same URI, same version), the parse result
from the previous keystroke is still valid.

Cache Key Design
-----------------

The cache key is ``(uri, version)``. Version is a monotonically increasing
integer that the editor increments with each change. Using version in the
key means:

- Same ``(uri, version)`` -> cache hit -> return cached result
- Different version -> cache miss -> re-parse and cache new result

The old entry is evicted when a new version is cached for the same URI.
This keeps memory bounded at O(open_documents) entries.

Thread Safety
--------------

The ParseCache is NOT thread-safe. This is intentional: the LspServer
processes one message at a time (single-threaded), so no locking is needed.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

from ls00.language_bridge import LanguageBridge
from ls00.types import Diagnostic


@dataclass
class ParseResult:
    """Holds the outcome of parsing one version of a document.

    Even on parse error, we store the partial AST and diagnostics so that
    other features (hover, folding, symbols) can still work on the valid
    portions.
    """

    ast: Any  # may be None if the parser couldn't produce any AST
    diagnostics: list[Diagnostic] = field(default_factory=list)


class ParseCache:
    """Stores the most recent parse result for each open document.

    Create one with ``ParseCache()``. Call ``get_or_parse()`` on every
    feature request to get the current (possibly cached) parse result.
    """

    def __init__(self) -> None:
        self._cache: dict[tuple[str, int], ParseResult] = {}

    def get_or_parse(
        self,
        uri: str,
        version: int,
        source: str,
        bridge: LanguageBridge,
    ) -> ParseResult:
        """Return the parse result for ``(uri, version)``.

        If the result is already cached, it is returned immediately without
        calling the bridge again. Otherwise, ``bridge.parse(source)`` is
        called, the result is stored, and the previous cache entry for this
        URI (if any) is evicted to prevent unbounded growth.

        This function is the single point of truth for "what is the parsed
        state of this document right now?" All feature handlers call it
        before operating on the AST.
        """
        key = (uri, version)

        # Cache hit: the document hasn't changed since last parse.
        if key in self._cache:
            return self._cache[key]

        # Cache miss: parse and store. Evict any stale entry first.
        self._evict(uri)

        ast, diags = bridge.parse(source)
        if diags is None:
            diags = []  # normalize None to empty list for JSON

        result = ParseResult(ast=ast, diagnostics=diags)
        self._cache[key] = result
        return result

    def evict(self, uri: str) -> None:
        """Remove all cached entries for a given URI.

        Called when a document is closed (didClose) so the cache entry
        is cleaned up.
        """
        self._evict(uri)

    def _evict(self, uri: str) -> None:
        """Internal eviction implementation."""
        keys_to_remove = [k for k in self._cache if k[0] == uri]
        for k in keys_to_remove:
            del self._cache[k]
