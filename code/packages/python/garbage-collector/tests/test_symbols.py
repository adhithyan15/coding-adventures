"""Tests for the symbol table (symbol interning).

These tests verify that SymbolTable correctly:
- Interns symbols so equal names share the same address
- Allocates different symbols at different addresses
- Handles re-interning after GC frees symbols
- Provides lookup without allocation
- Lists all live symbols
"""

from __future__ import annotations

from garbage_collector.gc import Symbol
from garbage_collector.mark_sweep import MarkAndSweepGC
from garbage_collector.symbols import SymbolTable


class TestIntern:
    """Tests for the intern() method."""

    def test_same_name_same_address(self) -> None:
        """Interning the same name twice should return the same address."""
        gc = MarkAndSweepGC()
        table = SymbolTable(gc)

        addr1 = table.intern("foo")
        addr2 = table.intern("foo")
        assert addr1 == addr2

    def test_different_names_different_addresses(self) -> None:
        """Interning different names should return different addresses."""
        gc = MarkAndSweepGC()
        table = SymbolTable(gc)

        addr_foo = table.intern("foo")
        addr_bar = table.intern("bar")
        assert addr_foo != addr_bar

    def test_interned_symbol_is_symbol_object(self) -> None:
        """Dereferencing an interned address should give a Symbol."""
        gc = MarkAndSweepGC()
        table = SymbolTable(gc)

        addr = table.intern("hello")
        obj = gc.deref(addr)
        assert isinstance(obj, Symbol)
        assert obj.name == "hello"

    def test_many_symbols(self) -> None:
        """Interning many symbols should all have unique addresses."""
        gc = MarkAndSweepGC()
        table = SymbolTable(gc)

        names = ["alpha", "beta", "gamma", "delta", "epsilon"]
        addrs = [table.intern(name) for name in names]
        assert len(set(addrs)) == len(names)

        # Re-intern all — should get same addresses
        addrs2 = [table.intern(name) for name in names]
        assert addrs == addrs2


class TestInternAfterGC:
    """Tests for re-interning after garbage collection."""

    def test_reintern_after_gc_frees_symbol(self) -> None:
        """If a symbol is freed by GC, interning it again re-allocates."""
        gc = MarkAndSweepGC()
        table = SymbolTable(gc)

        addr1 = table.intern("temp")
        # No roots → GC frees everything
        gc.collect(roots=[])
        assert not gc.is_valid_address(addr1)

        # Re-intern should allocate a new Symbol
        addr2 = table.intern("temp")
        assert gc.is_valid_address(addr2)
        assert gc.deref(addr2).name == "temp"

    def test_surviving_symbol_keeps_address(self) -> None:
        """A rooted symbol should keep its address after GC."""
        gc = MarkAndSweepGC()
        table = SymbolTable(gc)

        addr = table.intern("keeper")
        gc.collect(roots=[addr])

        # Re-intern should return the same address
        addr2 = table.intern("keeper")
        assert addr == addr2


class TestLookup:
    """Tests for the lookup() method."""

    def test_lookup_existing(self) -> None:
        """Looking up an interned symbol should return its address."""
        gc = MarkAndSweepGC()
        table = SymbolTable(gc)

        addr = table.intern("exists")
        assert table.lookup("exists") == addr

    def test_lookup_nonexistent(self) -> None:
        """Looking up a non-interned symbol should return None."""
        gc = MarkAndSweepGC()
        table = SymbolTable(gc)

        assert table.lookup("nope") is None

    def test_lookup_after_gc_freed(self) -> None:
        """Looking up a GC-freed symbol should return None."""
        gc = MarkAndSweepGC()
        table = SymbolTable(gc)

        table.intern("doomed")
        gc.collect(roots=[])

        assert table.lookup("doomed") is None


class TestAllSymbols:
    """Tests for the all_symbols() method."""

    def test_empty_table(self) -> None:
        """Empty table should return empty dict."""
        gc = MarkAndSweepGC()
        table = SymbolTable(gc)
        assert table.all_symbols() == {}

    def test_all_symbols_after_intern(self) -> None:
        """Should list all interned symbols."""
        gc = MarkAndSweepGC()
        table = SymbolTable(gc)

        a = table.intern("a")
        b = table.intern("b")

        syms = table.all_symbols()
        assert syms == {"a": a, "b": b}

    def test_all_symbols_excludes_freed(self) -> None:
        """Should exclude symbols freed by GC."""
        gc = MarkAndSweepGC()
        table = SymbolTable(gc)

        a = table.intern("alive")
        table.intern("dead")

        gc.collect(roots=[a])

        syms = table.all_symbols()
        assert "alive" in syms
        assert "dead" not in syms
