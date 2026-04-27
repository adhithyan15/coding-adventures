"""JITCache and JITCacheEntry — compiled function storage for jit-core.

The cache maps function names to ``JITCacheEntry`` objects.  Each entry holds
the native binary, the post-optimization CIR, and runtime statistics.

Deoptimization tracking
-----------------------
Each entry tracks how many times the compiled function deopted at runtime
(``deopt_count``) and how many times it successfully ran (``exec_count``).

When ``deopt_count / exec_count > 0.1`` the JIT marks the function as
``UNSPECIALIZABLE`` and invalidates the compiled version.

Invalidation
------------
``JITCache.invalidate(fn_name)`` removes the entry from the cache.  A separate
set of invalidated names (``_invalidated``) is maintained so the JIT knows
not to attempt re-compilation.
"""

from __future__ import annotations

import time
from dataclasses import dataclass, field

from jit_core.cir import CIRInstr


@dataclass
class JITCacheEntry:
    """A compiled function's cached state.

    Parameters
    ----------
    fn_name:
        Name of the function as it appears in the ``IIRModule``.
    binary:
        Native binary bytes produced by the backend.  Opaque to jit-core.
    backend_name:
        Name of the backend that produced this binary.
    param_count:
        Number of parameters the function accepts.
    ir:
        Post-optimization ``CIRInstr`` list — the IR that was handed to the
        backend.  Preserved for ``dump_ir()`` and debugging.
    compilation_time_ns:
        Wall-clock nanoseconds spent in specialise + optimize + compile.
    deopt_count:
        Number of times execution fell back to the interpreter.  Incremented
        externally by the deopt stub or ``JITCore._record_deopt()``.
    exec_count:
        Number of times the compiled binary was executed successfully.
        Incremented by the JIT handler wrapper.
    """

    fn_name: str
    binary: bytes
    backend_name: str
    param_count: int
    ir: list[CIRInstr]
    compilation_time_ns: int
    deopt_count: int = field(default=0)
    exec_count: int = field(default=0)

    @property
    def deopt_rate(self) -> float:
        """Fraction of executions that deopted.  0.0 if never executed."""
        if self.exec_count == 0:
            return 0.0
        return self.deopt_count / self.exec_count

    def as_stats(self) -> dict:
        """Return a statistics snapshot as a plain dict."""
        return {
            "fn_name": self.fn_name,
            "backend": self.backend_name,
            "param_count": self.param_count,
            "ir_size": len(self.ir),
            "binary_size": len(self.binary),
            "compilation_time_ns": self.compilation_time_ns,
            "exec_count": self.exec_count,
            "deopt_count": self.deopt_count,
            "deopt_rate": self.deopt_rate,
        }


class JITCache:
    """Dictionary-backed cache mapping function names to compiled binaries.

    Thread safety: NOT thread-safe.  One ``JITCache`` per ``JITCore`` instance.
    """

    def __init__(self) -> None:
        self._entries: dict[str, JITCacheEntry] = {}
        self._invalidated: set[str] = set()

    def get(self, fn_name: str) -> JITCacheEntry | None:
        """Return the cached entry for ``fn_name``, or ``None``."""
        return self._entries.get(fn_name)

    def put(self, entry: JITCacheEntry) -> None:
        """Store ``entry`` in the cache, overwriting any previous entry."""
        self._entries[entry.fn_name] = entry
        self._invalidated.discard(entry.fn_name)

    def invalidate(self, fn_name: str) -> None:
        """Remove ``fn_name`` from the cache and mark it as unspecializable."""
        self._entries.pop(fn_name, None)
        self._invalidated.add(fn_name)

    def is_invalidated(self, fn_name: str) -> bool:
        """Return True if this function has been permanently invalidated."""
        return fn_name in self._invalidated

    def stats(self) -> dict[str, dict]:
        """Return a snapshot of per-function statistics."""
        return {name: entry.as_stats() for name, entry in self._entries.items()}

    def __len__(self) -> int:
        return len(self._entries)

    def __contains__(self, fn_name: str) -> bool:
        return fn_name in self._entries

    @staticmethod
    def now_ns() -> int:
        """Return a monotonic nanosecond timestamp."""
        return time.monotonic_ns()
