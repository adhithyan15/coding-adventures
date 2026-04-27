"""Tests for JITCache and JITCacheEntry."""

from __future__ import annotations

import time

import pytest

from jit_core.cache import JITCache, JITCacheEntry
from jit_core.cir import CIRInstr

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _entry(
    fn_name: str = "f",
    binary: bytes = b"bin",
    backend_name: str = "mock",
    param_count: int = 0,
    ir: list | None = None,
    compilation_time_ns: int = 1000,
) -> JITCacheEntry:
    return JITCacheEntry(
        fn_name=fn_name,
        binary=binary,
        backend_name=backend_name,
        param_count=param_count,
        ir=ir or [],
        compilation_time_ns=compilation_time_ns,
    )


# ---------------------------------------------------------------------------
# JITCacheEntry
# ---------------------------------------------------------------------------

class TestJITCacheEntry:
    def test_deopt_rate_zero_when_never_executed(self):
        e = _entry()
        assert e.deopt_rate == 0.0

    def test_deopt_rate_zero_when_no_deopts(self):
        e = _entry()
        e.exec_count = 10
        assert e.deopt_rate == 0.0

    def test_deopt_rate_computed(self):
        e = _entry()
        e.exec_count = 10
        e.deopt_count = 2
        assert e.deopt_rate == pytest.approx(0.2)

    def test_deopt_rate_one_hundred_percent(self):
        e = _entry()
        e.exec_count = 5
        e.deopt_count = 5
        assert e.deopt_rate == pytest.approx(1.0)

    def test_as_stats_keys(self):
        e = _entry(fn_name="add", binary=b"x" * 8, param_count=2, compilation_time_ns=500)
        e.exec_count = 3
        e.deopt_count = 1
        stats = e.as_stats()
        assert stats["fn_name"] == "add"
        assert stats["backend"] == "mock"
        assert stats["param_count"] == 2
        assert stats["binary_size"] == 8
        assert stats["exec_count"] == 3
        assert stats["deopt_count"] == 1
        assert stats["deopt_rate"] == pytest.approx(1 / 3)
        assert stats["compilation_time_ns"] == 500

    def test_as_stats_ir_size(self):
        ir = [CIRInstr(op="const_u8", dest="v0", srcs=[1], type="u8")]
        e = _entry(ir=ir)
        stats = e.as_stats()
        assert stats["ir_size"] == 1

    def test_default_counts_are_zero(self):
        e = _entry()
        assert e.exec_count == 0
        assert e.deopt_count == 0


# ---------------------------------------------------------------------------
# JITCache — basic operations
# ---------------------------------------------------------------------------

class TestJITCacheBasic:
    def test_get_miss(self):
        cache = JITCache()
        assert cache.get("missing") is None

    def test_put_and_get(self):
        cache = JITCache()
        e = _entry("f")
        cache.put(e)
        assert cache.get("f") is e

    def test_put_overwrites(self):
        cache = JITCache()
        e1 = _entry("f", binary=b"v1")
        e2 = _entry("f", binary=b"v2")
        cache.put(e1)
        cache.put(e2)
        assert cache.get("f") is e2

    def test_len_empty(self):
        cache = JITCache()
        assert len(cache) == 0

    def test_len_after_put(self):
        cache = JITCache()
        cache.put(_entry("a"))
        cache.put(_entry("b"))
        assert len(cache) == 2

    def test_contains_true(self):
        cache = JITCache()
        cache.put(_entry("f"))
        assert "f" in cache

    def test_contains_false(self):
        cache = JITCache()
        assert "f" not in cache

    def test_stats_empty(self):
        cache = JITCache()
        assert cache.stats() == {}

    def test_stats_populated(self):
        cache = JITCache()
        cache.put(_entry("f"))
        stats = cache.stats()
        assert "f" in stats
        assert isinstance(stats["f"], dict)


# ---------------------------------------------------------------------------
# JITCache — invalidation
# ---------------------------------------------------------------------------

class TestJITCacheInvalidation:
    def test_invalidate_removes_entry(self):
        cache = JITCache()
        cache.put(_entry("f"))
        cache.invalidate("f")
        assert cache.get("f") is None
        assert "f" not in cache

    def test_is_invalidated_after_invalidate(self):
        cache = JITCache()
        cache.put(_entry("f"))
        cache.invalidate("f")
        assert cache.is_invalidated("f")

    def test_is_invalidated_false_when_present(self):
        cache = JITCache()
        cache.put(_entry("f"))
        assert not cache.is_invalidated("f")

    def test_is_invalidated_false_when_never_seen(self):
        cache = JITCache()
        assert not cache.is_invalidated("f")

    def test_invalidate_nonexistent_is_safe(self):
        cache = JITCache()
        cache.invalidate("ghost")  # should not raise
        assert cache.is_invalidated("ghost")

    def test_put_after_invalidate_clears_invalidated_flag(self):
        cache = JITCache()
        cache.put(_entry("f"))
        cache.invalidate("f")
        assert cache.is_invalidated("f")
        cache.put(_entry("f"))
        assert not cache.is_invalidated("f")
        assert cache.get("f") is not None

    def test_len_after_invalidate(self):
        cache = JITCache()
        cache.put(_entry("f"))
        cache.put(_entry("g"))
        cache.invalidate("f")
        assert len(cache) == 1

    def test_stats_excludes_invalidated(self):
        cache = JITCache()
        cache.put(_entry("f"))
        cache.invalidate("f")
        assert "f" not in cache.stats()


# ---------------------------------------------------------------------------
# JITCache — multiple entries
# ---------------------------------------------------------------------------

class TestJITCacheMultiple:
    def test_multiple_independent_entries(self):
        cache = JITCache()
        for name in ("f", "g", "h"):
            cache.put(_entry(name, binary=name.encode()))
        assert cache.get("f").binary == b"f"
        assert cache.get("g").binary == b"g"
        assert cache.get("h").binary == b"h"

    def test_invalidate_one_leaves_others(self):
        cache = JITCache()
        cache.put(_entry("f"))
        cache.put(_entry("g"))
        cache.invalidate("f")
        assert cache.get("g") is not None
        assert cache.get("f") is None


# ---------------------------------------------------------------------------
# JITCache.now_ns
# ---------------------------------------------------------------------------

class TestNowNs:
    def test_now_ns_returns_positive_int(self):
        t = JITCache.now_ns()
        assert isinstance(t, int)
        assert t > 0

    def test_now_ns_is_monotonic(self):
        t1 = JITCache.now_ns()
        time.sleep(0.001)
        t2 = JITCache.now_ns()
        assert t2 >= t1
