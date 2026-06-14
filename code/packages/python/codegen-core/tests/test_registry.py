"""Tests for BackendRegistry — name-to-backend mapping."""

from __future__ import annotations

from typing import Any

import pytest

from codegen_core import BackendRegistry

# ---------------------------------------------------------------------------
# Minimal mock backends
# ---------------------------------------------------------------------------

class _BackendA:
    name = "alpha"
    def compile(self, ir: Any) -> bytes | None: return b"\x01"
    def run(self, binary: bytes, args: list) -> Any: return 1

class _BackendB:
    name = "beta"
    def compile(self, ir: Any) -> bytes | None: return b"\x02"
    def run(self, binary: bytes, args: list) -> Any: return 2


# ---------------------------------------------------------------------------
# Registry tests
# ---------------------------------------------------------------------------

class TestBackendRegistry:

    def test_empty_registry(self) -> None:
        reg = BackendRegistry()
        assert len(reg) == 0
        assert reg.names() == []

    def test_register_and_get(self) -> None:
        reg = BackendRegistry()
        reg.register(_BackendA())
        backend = reg.get("alpha")
        assert backend is not None
        assert backend.name == "alpha"

    def test_get_missing_returns_none(self) -> None:
        reg = BackendRegistry()
        assert reg.get("nonexistent") is None

    def test_get_or_raise_success(self) -> None:
        reg = BackendRegistry()
        reg.register(_BackendA())
        assert reg.get_or_raise("alpha").name == "alpha"

    def test_get_or_raise_raises_on_missing(self) -> None:
        reg = BackendRegistry()
        with pytest.raises(KeyError, match="nonexistent"):
            reg.get_or_raise("nonexistent")

    def test_get_or_raise_lists_available(self) -> None:
        reg = BackendRegistry()
        reg.register(_BackendA())
        with pytest.raises(KeyError, match="alpha"):
            reg.get_or_raise("nonexistent")

    def test_register_two_backends(self) -> None:
        reg = BackendRegistry()
        reg.register(_BackendA())
        reg.register(_BackendB())
        assert len(reg) == 2
        assert reg.names() == ["alpha", "beta"]

    def test_names_sorted(self) -> None:
        reg = BackendRegistry()
        reg.register(_BackendB())
        reg.register(_BackendA())
        assert reg.names() == ["alpha", "beta"]

    def test_register_replaces_existing(self) -> None:
        reg = BackendRegistry()

        class _AlphaV2:
            name = "alpha"
            def compile(self, ir: Any) -> bytes | None: return b"\xff"
            def run(self, binary: bytes, args: list) -> Any: return 99

        reg.register(_BackendA())
        reg.register(_AlphaV2())
        assert reg.get_or_raise("alpha").compile(None) == b"\xff"
        assert len(reg) == 1  # still one entry

    def test_contains_operator(self) -> None:
        reg = BackendRegistry()
        reg.register(_BackendA())
        assert "alpha" in reg
        assert "beta" not in reg

    def test_repr(self) -> None:
        reg = BackendRegistry()
        reg.register(_BackendA())
        r = repr(reg)
        assert "alpha" in r

    def test_repr_empty(self) -> None:
        reg = BackendRegistry()
        assert "<empty>" in repr(reg)
