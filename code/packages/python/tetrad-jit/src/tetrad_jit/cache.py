"""JIT code cache (TET05).

Stores compiled Intel 4004 binaries, post-optimization IR, and compilation
metadata keyed by function name.
"""

from __future__ import annotations

import time
from dataclasses import dataclass, field

from tetrad_jit.ir import IRInstr

__all__ = ["JITCache", "JITCacheEntry"]


@dataclass
class JITCacheEntry:
    """A single cached compiled function.

    Attributes
    ----------
    fn_name:
        The Tetrad function name.
    binary:
        Intel 4004 machine-code bytes, ready for ``Intel4004Simulator``.
    param_count:
        Number of u8 arguments the function expects (0, 1, or 2).
    ir:
        The post-optimization IR list (for ``dump_ir``).
    compilation_time_ns:
        Wall-clock nanoseconds spent compiling, for benchmarking.
    """

    fn_name: str
    binary: bytes
    param_count: int
    ir: list[IRInstr] = field(default_factory=list)
    compilation_time_ns: int = 0


class JITCache:
    """Thread-unsafe in-memory cache of compiled functions.

    Methods
    -------
    get(fn_name) → JITCacheEntry | None
    put(entry) → None
    invalidate(fn_name) → None
    stats() → dict
    """

    def __init__(self) -> None:
        self._store: dict[str, JITCacheEntry] = {}

    def get(self, fn_name: str) -> JITCacheEntry | None:
        """Return the cached entry for *fn_name*, or ``None`` if absent."""
        return self._store.get(fn_name)

    def put(self, entry: JITCacheEntry) -> None:
        """Insert or overwrite the cache entry for ``entry.fn_name``."""
        self._store[entry.fn_name] = entry

    def invalidate(self, fn_name: str) -> None:
        """Remove *fn_name* from the cache (no-op if absent)."""
        self._store.pop(fn_name, None)

    def __contains__(self, fn_name: str) -> bool:
        return fn_name in self._store

    def stats(self) -> dict[str, dict]:
        """Return per-function stats for benchmarking / introspection."""
        return {
            name: {
                "binary_bytes": len(entry.binary),
                "param_count": entry.param_count,
                "ir_instructions": len(entry.ir),
                "compilation_time_ns": entry.compilation_time_ns,
            }
            for name, entry in self._store.items()
        }

    @staticmethod
    def now_ns() -> int:
        """Return the current time in nanoseconds (for timing compilation)."""
        return time.perf_counter_ns()
