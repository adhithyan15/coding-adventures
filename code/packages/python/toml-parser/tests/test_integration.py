"""Integration tests — full TOML documents parsed end-to-end.

These tests use realistic TOML documents (Cargo.toml style, pyproject.toml
style, configuration files) to verify the complete pipeline works correctly.
They also compare results against Python 3.11+'s built-in ``tomllib`` for
known inputs.
"""

from __future__ import annotations

import tomllib

from toml_parser import parse_toml

# =============================================================================
# Realistic Document Tests
# =============================================================================


class TestRealisticDocuments:
    """Test with realistic TOML documents."""

    def test_cargo_toml_style(self) -> None:
        """Parse a Cargo.toml-style document."""
        source = """\
[package]
name = "my-project"
version = "0.1.0"
edition = "2021"
authors = ["Alice <alice@example.com>"]

[dependencies]
serde = "1.0"

[dev-dependencies]
tokio = { version = "1.0", features = ["full"] }
"""
        doc = parse_toml(source)
        assert doc["package"]["name"] == "my-project"
        assert doc["package"]["version"] == "0.1.0"
        assert doc["package"]["authors"] == ["Alice <alice@example.com>"]
        assert doc["dependencies"]["serde"] == "1.0"
        assert doc["dev-dependencies"]["tokio"]["version"] == "1.0"
        assert doc["dev-dependencies"]["tokio"]["features"] == ["full"]

    def test_pyproject_toml_style(self) -> None:
        """Parse a pyproject.toml-style document."""
        source = """\
[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[project]
name = "my-package"
version = "0.1.0"
description = "A test package"
requires-python = ">=3.12"

[project.optional-dependencies]
dev = ["pytest>=8.0", "ruff>=0.4"]
"""
        doc = parse_toml(source)
        assert doc["build-system"]["requires"] == ["hatchling"]
        assert doc["project"]["name"] == "my-package"
        assert doc["project"]["requires-python"] == ">=3.12"
        assert doc["project"]["optional-dependencies"]["dev"] == [
            "pytest>=8.0",
            "ruff>=0.4",
        ]

    def test_server_config_style(self) -> None:
        """Parse a server configuration-style document."""
        source = """\
title = "Server Config"

[database]
server = "192.168.1.1"
ports = [8001, 8001, 8002]
connection_max = 5000
enabled = true

[servers.alpha]
ip = "10.0.0.1"
dc = "eqdc10"

[servers.beta]
ip = "10.0.0.2"
dc = "eqdc10"
"""
        doc = parse_toml(source)
        assert doc["title"] == "Server Config"
        assert doc["database"]["server"] == "192.168.1.1"
        assert doc["database"]["ports"] == [8001, 8001, 8002]
        assert doc["database"]["connection_max"] == 5000
        assert doc["database"]["enabled"] is True
        assert doc["servers"]["alpha"]["ip"] == "10.0.0.1"
        assert doc["servers"]["beta"]["dc"] == "eqdc10"

    def test_array_of_tables_document(self) -> None:
        """Parse a document with arrays of tables."""
        source = """\
[[fruit]]
name = "apple"

[fruit.physical]
color = "red"
shape = "round"

[[fruit.variety]]
name = "red delicious"

[[fruit.variety]]
name = "granny smith"

[[fruit]]
name = "banana"

[[fruit.variety]]
name = "plantain"
"""
        doc = parse_toml(source)
        assert len(doc["fruit"]) == 2
        assert doc["fruit"][0]["name"] == "apple"
        assert doc["fruit"][0]["physical"]["color"] == "red"
        assert len(doc["fruit"][0]["variety"]) == 2
        assert doc["fruit"][0]["variety"][0]["name"] == "red delicious"
        assert doc["fruit"][0]["variety"][1]["name"] == "granny smith"
        assert doc["fruit"][1]["name"] == "banana"
        assert doc["fruit"][1]["variety"][0]["name"] == "plantain"


# =============================================================================
# Comparison with tomllib
# =============================================================================


class TestTomllibComparison:
    """Compare parse_toml output against Python's built-in tomllib.

    Python 3.11+ includes ``tomllib`` in the standard library. We use it
    as a reference implementation to verify our parser produces identical
    results for valid TOML inputs.
    """

    def _compare(self, source: str) -> None:
        """Parse with both parsers and compare results."""
        ours = parse_toml(source)
        theirs = tomllib.loads(source)
        assert dict(ours) == theirs, f"Mismatch:\n  ours: {ours}\n  theirs: {theirs}"

    def test_simple_kv(self) -> None:
        """Simple key-value pairs."""
        self._compare('name = "TOML"\nversion = "1.0.0"')

    def test_integers(self) -> None:
        """Various integer formats."""
        self._compare("a = 42\nb = -17\nc = 0\nd = 1_000")

    def test_floats(self) -> None:
        """Various float formats."""
        self._compare("a = 3.14\nb = -0.5\nc = 1e10")

    def test_booleans(self) -> None:
        """Boolean values."""
        self._compare("a = true\nb = false")

    def test_strings(self) -> None:
        """Various string types."""
        self._compare(
            'a = "basic"\n'
            "b = 'literal'\n"
            'c = """multi\nline"""\n'
            "d = '''ml literal'''"
        )

    def test_dates(self) -> None:
        """Date/time values."""
        self._compare(
            "a = 1979-05-27\n"
            "b = 07:32:00\n"
            "c = 1979-05-27T07:32:00\n"
            "d = 1979-05-27T07:32:00Z"
        )

    def test_arrays(self) -> None:
        """Array values."""
        self._compare('a = [1, 2, 3]\nb = ["x", "y"]')

    def test_tables(self) -> None:
        """Table structures."""
        self._compare('[a]\nx = 1\n[b]\ny = 2')

    def test_nested_tables(self) -> None:
        """Nested table structures."""
        self._compare('[a.b]\nc = 1\n[a.d]\ne = 2')

    def test_inline_tables(self) -> None:
        """Inline table values."""
        self._compare("point = { x = 1, y = 2 }")

    def test_array_of_tables(self) -> None:
        """Array of tables."""
        self._compare('[[item]]\nname = "a"\n[[item]]\nname = "b"')

    def test_mixed_document(self) -> None:
        """A document mixing many features."""
        source = """\
title = "Example"

[owner]
name = "Tom"
dob = 1979-05-27T07:32:00Z

[database]
ports = [8001, 8001, 8002]
enabled = true
"""
        self._compare(source)

    def _deep_compare(self, ours: object, theirs: object, path: str = "") -> None:
        """Deep comparison that handles nested TOMLDocument vs dict."""
        if isinstance(ours, dict) and isinstance(theirs, dict):
            assert set(ours.keys()) == set(theirs.keys()), f"Key mismatch at {path}"
            for key in ours:
                self._deep_compare(ours[key], theirs[key], f"{path}.{key}")
        elif isinstance(ours, list) and isinstance(theirs, list):
            assert len(ours) == len(theirs), f"Length mismatch at {path}"
            for i, (a, b) in enumerate(zip(ours, theirs, strict=True)):
                self._deep_compare(a, b, f"{path}[{i}]")
        else:
            assert ours == theirs, f"Value mismatch at {path}: {ours!r} != {theirs!r}"

    def test_full_featured_document(self) -> None:
        """A comprehensive document using all TOML features."""
        source = """\
# This is a full-featured TOML document

title = "TOML Example"

[owner]
name = "Tom Preston-Werner"

[database]
server = "192.168.1.1"
ports = [8001, 8001, 8002]
connection_max = 5000
enabled = true

[servers]

[servers.alpha]
ip = "10.0.0.1"
dc = "eqdc10"

[servers.beta]
ip = "10.0.0.2"
dc = "eqdc10"

[[products]]
name = "Hammer"
sku = 738594937

[[products]]
name = "Nail"
sku = 284758393
color = "gray"
"""
        ours = parse_toml(source)
        theirs = tomllib.loads(source)
        self._deep_compare(ours, theirs)
