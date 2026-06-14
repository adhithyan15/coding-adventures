"""Tests for CodeGenerator protocol and CodeGeneratorRegistry (LANG20).

Covers:
- ``CodeGenerator`` runtime-checkable protocol (isinstance checks)
- ``CodeGeneratorRegistry``: register, get, get_or_raise, names, all, len, contains
- Top-level exports from ``codegen_core``
"""

from __future__ import annotations

import pytest

from codegen_core import CodeGenerator, CodeGeneratorRegistry


# ---------------------------------------------------------------------------
# Mock code generators for testing
# ---------------------------------------------------------------------------


class _MockGen:
    """Minimal code generator satisfying CodeGenerator[str, str]."""

    name = "mock"

    def validate(self, ir: str) -> list[str]:
        return [] if ir else ["IR must not be empty"]

    def generate(self, ir: str) -> str:
        if not ir:
            raise ValueError("empty IR")
        return f"GENERATED({ir})"


class _AnotherGen:
    """Second generator for multi-registration tests."""

    name = "another"

    def validate(self, ir: str) -> list[str]:
        return []

    def generate(self, ir: str) -> bytes:
        return ir.encode()


class _NoGenerateGen:
    """Object that has name and validate but NOT generate — must fail isinstance."""

    name = "broken"

    def validate(self, ir: str) -> list[str]:
        return []


class _NoValidateGen:
    """Object that has name and generate but NOT validate — must fail isinstance."""

    name = "broken2"

    def generate(self, ir: str) -> str:
        return ir


class _NoNameGen:
    """Object that has validate and generate but NOT name — must fail isinstance."""

    def validate(self, ir: str) -> list[str]:
        return []

    def generate(self, ir: str) -> str:
        return ir


# ---------------------------------------------------------------------------
# CodeGenerator protocol — isinstance checks
# ---------------------------------------------------------------------------


class TestCodeGeneratorProtocol:

    def test_mock_gen_satisfies_protocol(self) -> None:
        assert isinstance(_MockGen(), CodeGenerator)

    def test_another_gen_satisfies_protocol(self) -> None:
        assert isinstance(_AnotherGen(), CodeGenerator)

    def test_missing_generate_fails_protocol(self) -> None:
        assert not isinstance(_NoGenerateGen(), CodeGenerator)

    def test_missing_validate_fails_protocol(self) -> None:
        assert not isinstance(_NoValidateGen(), CodeGenerator)

    def test_missing_name_fails_protocol(self) -> None:
        assert not isinstance(_NoNameGen(), CodeGenerator)

    def test_plain_dict_fails_protocol(self) -> None:
        assert not isinstance({"name": "x"}, CodeGenerator)

    def test_protocol_exported_from_codegen_core(self) -> None:
        """CodeGenerator must be importable from the top-level codegen_core module."""
        import codegen_core
        assert hasattr(codegen_core, "CodeGenerator")
        assert codegen_core.CodeGenerator is CodeGenerator


# ---------------------------------------------------------------------------
# CodeGeneratorRegistry
# ---------------------------------------------------------------------------


class TestCodeGeneratorRegistry:

    def test_empty_registry_names(self) -> None:
        registry = CodeGeneratorRegistry()
        assert registry.names() == []

    def test_empty_registry_len(self) -> None:
        registry = CodeGeneratorRegistry()
        assert len(registry) == 0

    def test_register_single_generator(self) -> None:
        registry = CodeGeneratorRegistry()
        registry.register(_MockGen())
        assert "mock" in registry.names()

    def test_get_registered_generator(self) -> None:
        registry = CodeGeneratorRegistry()
        gen = _MockGen()
        registry.register(gen)
        retrieved = registry.get("mock")
        assert retrieved is not None
        assert retrieved.name == "mock"

    def test_get_unknown_returns_none(self) -> None:
        registry = CodeGeneratorRegistry()
        assert registry.get("nonexistent") is None

    def test_register_multiple_generators(self) -> None:
        registry = CodeGeneratorRegistry()
        registry.register(_MockGen())
        registry.register(_AnotherGen())
        assert "mock" in registry.names()
        assert "another" in registry.names()

    def test_names_sorted(self) -> None:
        registry = CodeGeneratorRegistry()
        registry.register(_AnotherGen())   # "another"
        registry.register(_MockGen())      # "mock"
        assert registry.names() == ["another", "mock"]

    def test_all_returns_sorted_generators(self) -> None:
        registry = CodeGeneratorRegistry()
        registry.register(_AnotherGen())
        registry.register(_MockGen())
        all_gens = registry.all()
        assert [g.name for g in all_gens] == ["another", "mock"]

    def test_len_increases_with_registration(self) -> None:
        registry = CodeGeneratorRegistry()
        assert len(registry) == 0
        registry.register(_MockGen())
        assert len(registry) == 1
        registry.register(_AnotherGen())
        assert len(registry) == 2

    def test_contains_registered_name(self) -> None:
        registry = CodeGeneratorRegistry()
        registry.register(_MockGen())
        assert "mock" in registry

    def test_contains_unregistered_name(self) -> None:
        registry = CodeGeneratorRegistry()
        assert "phantom" not in registry

    def test_register_replaces_existing(self) -> None:
        """Second registration with the same name replaces the first."""
        registry = CodeGeneratorRegistry()
        gen1 = _MockGen()
        gen2 = _MockGen()  # same name "mock"
        registry.register(gen1)
        registry.register(gen2)
        assert registry.get("mock") is gen2
        assert len(registry) == 1

    def test_get_or_raise_returns_generator(self) -> None:
        registry = CodeGeneratorRegistry()
        registry.register(_MockGen())
        assert registry.get_or_raise("mock").name == "mock"

    def test_get_or_raise_on_unknown_raises_key_error(self) -> None:
        registry = CodeGeneratorRegistry()
        with pytest.raises(KeyError, match="nonexistent"):
            registry.get_or_raise("nonexistent")

    def test_get_or_raise_error_lists_available_names(self) -> None:
        registry = CodeGeneratorRegistry()
        registry.register(_MockGen())
        with pytest.raises(KeyError, match="mock"):
            registry.get_or_raise("missing")

    def test_repr_shows_names(self) -> None:
        registry = CodeGeneratorRegistry()
        registry.register(_MockGen())
        assert "mock" in repr(registry)

    def test_repr_empty(self) -> None:
        registry = CodeGeneratorRegistry()
        assert "<empty>" in repr(registry)

    def test_registry_exported_from_codegen_core(self) -> None:
        """CodeGeneratorRegistry must be importable from the top-level module."""
        import codegen_core
        assert hasattr(codegen_core, "CodeGeneratorRegistry")
        assert codegen_core.CodeGeneratorRegistry is CodeGeneratorRegistry


# ---------------------------------------------------------------------------
# Round-trip usage
# ---------------------------------------------------------------------------


class TestRoundTrip:

    def test_validate_then_generate(self) -> None:
        gen = _MockGen()
        errors = gen.validate("hello")
        assert errors == []
        result = gen.generate("hello")
        assert result == "GENERATED(hello)"

    def test_validate_reports_error(self) -> None:
        gen = _MockGen()
        errors = gen.validate("")
        assert len(errors) == 1
        assert "empty" in errors[0].lower()

    def test_registry_roundtrip(self) -> None:
        registry = CodeGeneratorRegistry()
        registry.register(_MockGen())
        gen = registry.get_or_raise("mock")
        assert gen.validate("x") == []
        assert gen.generate("x") == "GENERATED(x)"
