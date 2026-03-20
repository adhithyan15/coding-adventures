"""Tests for manifest loading and capability comparison.

These tests verify that the comparison engine correctly answers:
"Does this package use only the capabilities it declared?"

The comparison is asymmetric:
- Undeclared capabilities (detected but not in manifest) → ERRORS
- Unused declarations (in manifest but not detected) → WARNINGS
"""

import json

import pytest

from ca_capability_analyzer.analyzer import DetectedCapability
from ca_capability_analyzer.manifest import (
    ComparisonResult,
    Manifest,
    compare_capabilities,
    default_manifest,
    load_manifest,
)

# ── Helper factories ─────────────────────────────────────────────────


def _cap(
    category: str = "fs",
    action: str = "read",
    target: str = "*",
    file: str = "test.py",
    line: int = 1,
    evidence: str = "",
) -> DetectedCapability:
    """Create a DetectedCapability for testing."""
    return DetectedCapability(
        category=category,
        action=action,
        target=target,
        file=file,
        line=line,
        evidence=evidence,
    )


def _manifest(
    capabilities: list[dict[str, str]] | None = None,
    package: str = "python/test-pkg",
) -> Manifest:
    """Create a Manifest for testing."""
    return Manifest(
        package=package,
        capabilities=capabilities or [],
        justification="Test manifest.",
    )


# ── Manifest loading ─────────────────────────────────────────────────


class TestManifestLoading:
    """Tests for loading manifests from JSON files."""

    def test_load_manifest(self, tmp_path: object) -> None:
        from pathlib import Path

        tmp = Path(str(tmp_path))
        manifest_file = tmp / "required_capabilities.json"
        manifest_file.write_text(
            json.dumps(
                {
                    "version": 1,
                    "package": "python/test-pkg",
                    "capabilities": [
                        {"category": "fs", "action": "read", "target": "*"}
                    ],
                    "justification": "Needs file access.",
                }
            )
        )
        m = load_manifest(manifest_file)
        assert m.package == "python/test-pkg"
        assert len(m.capabilities) == 1
        assert m.capabilities[0]["category"] == "fs"

    def test_load_manifest_with_banned_exceptions(self, tmp_path: object) -> None:
        from pathlib import Path

        tmp = Path(str(tmp_path))
        manifest_file = tmp / "required_capabilities.json"
        manifest_file.write_text(
            json.dumps(
                {
                    "version": 1,
                    "package": "python/analyzer",
                    "capabilities": [],
                    "justification": "Analyzer package.",
                    "banned_construct_exceptions": [
                        {"construct": "compile", "justification": "Used by ast.parse"}
                    ],
                }
            )
        )
        m = load_manifest(manifest_file)
        assert len(m.banned_construct_exceptions) == 1

    def test_load_manifest_missing_file(self, tmp_path: object) -> None:
        from pathlib import Path

        tmp = Path(str(tmp_path))
        with pytest.raises(FileNotFoundError):
            load_manifest(tmp / "nonexistent.json")

    def test_load_manifest_invalid_json(self, tmp_path: object) -> None:
        from pathlib import Path

        tmp = Path(str(tmp_path))
        bad_file = tmp / "bad.json"
        bad_file.write_text("not json{{{")
        with pytest.raises(json.JSONDecodeError):
            load_manifest(bad_file)

    def test_load_manifest_missing_package_key(self, tmp_path: object) -> None:
        from pathlib import Path

        tmp = Path(str(tmp_path))
        manifest_file = tmp / "required_capabilities.json"
        manifest_file.write_text(json.dumps({"capabilities": []}))
        with pytest.raises(KeyError):
            load_manifest(manifest_file)


# ── Default manifest ─────────────────────────────────────────────────


class TestDefaultManifest:
    """Tests for the default (empty) manifest."""

    def test_default_manifest(self) -> None:
        m = default_manifest("python/test-pkg")
        assert m.package == "python/test-pkg"
        assert m.capabilities == []
        assert m.is_empty

    def test_default_manifest_denies_everything(self) -> None:
        """A default manifest should cause any capability to be an error."""
        m = default_manifest("python/test-pkg")
        detected = [_cap(category="fs", action="read")]
        result = compare_capabilities(detected, m)
        assert not result.passed
        assert len(result.errors) == 1


# ── Manifest properties ──────────────────────────────────────────────


class TestManifestProperties:
    """Tests for Manifest dataclass properties."""

    def test_is_empty_true(self) -> None:
        m = _manifest(capabilities=[])
        assert m.is_empty

    def test_is_empty_false(self) -> None:
        m = _manifest(
            capabilities=[{"category": "fs", "action": "read", "target": "*"}]
        )
        assert not m.is_empty


# ── Capability comparison ────────────────────────────────────────────


class TestComparison:
    """Tests for comparing detected capabilities against manifests."""

    def test_all_declared_passes(self) -> None:
        """When all detected capabilities are declared, comparison passes."""
        manifest = _manifest(
            capabilities=[
                {"category": "fs", "action": "read", "target": "*"},
            ]
        )
        detected = [_cap(category="fs", action="read", target="data.txt")]
        result = compare_capabilities(detected, manifest)
        assert result.passed
        assert len(result.errors) == 0
        assert len(result.matched) == 1

    def test_undeclared_capability_fails(self) -> None:
        """An undeclared capability should cause comparison to fail."""
        manifest = _manifest(capabilities=[])
        detected = [_cap(category="net", action="connect")]
        result = compare_capabilities(detected, manifest)
        assert not result.passed
        assert len(result.errors) == 1

    def test_unused_declaration_warns(self) -> None:
        """A declared but unused capability should produce a warning."""
        manifest = _manifest(
            capabilities=[
                {"category": "fs", "action": "read", "target": "*"},
                {"category": "net", "action": "connect", "target": "*"},
            ]
        )
        detected = [_cap(category="fs", action="read")]
        result = compare_capabilities(detected, manifest)
        assert result.passed  # Unused declarations are not errors
        assert len(result.warnings) == 1
        assert result.warnings[0]["category"] == "net"

    def test_wildcard_action_matches(self) -> None:
        """A declared action of '*' should match any detected action."""
        manifest = _manifest(
            capabilities=[
                {"category": "fs", "action": "*", "target": "*"},
            ]
        )
        detected = [
            _cap(category="fs", action="read"),
            _cap(category="fs", action="write"),
            _cap(category="fs", action="delete"),
        ]
        result = compare_capabilities(detected, manifest)
        assert result.passed
        assert len(result.matched) == 3

    def test_wildcard_target_matches(self) -> None:
        """A declared target of '*' should match any detected target."""
        manifest = _manifest(
            capabilities=[
                {"category": "fs", "action": "read", "target": "*"},
            ]
        )
        detected = [_cap(category="fs", action="read", target="any_file.txt")]
        result = compare_capabilities(detected, manifest)
        assert result.passed

    def test_glob_target_matches(self) -> None:
        """Glob patterns in declared targets should match detected targets."""
        manifest = _manifest(
            capabilities=[
                {
                    "category": "fs",
                    "action": "read",
                    "target": "../../grammars/*.tokens",
                },
            ]
        )
        detected = [
            _cap(category="fs", action="read", target="../../grammars/python.tokens"),
        ]
        result = compare_capabilities(detected, manifest)
        assert result.passed

    def test_glob_target_no_match(self) -> None:
        """Glob patterns should not match unrelated paths."""
        manifest = _manifest(
            capabilities=[
                {
                    "category": "fs",
                    "action": "read",
                    "target": "../../grammars/*.tokens",
                },
            ]
        )
        detected = [
            _cap(category="fs", action="read", target="/etc/passwd"),
        ]
        result = compare_capabilities(detected, manifest)
        assert not result.passed

    def test_exact_target_matches(self) -> None:
        """Exact target strings should match exactly."""
        manifest = _manifest(
            capabilities=[
                {"category": "fs", "action": "read", "target": "config.json"},
            ]
        )
        detected = [_cap(category="fs", action="read", target="config.json")]
        result = compare_capabilities(detected, manifest)
        assert result.passed

    def test_exact_target_no_match(self) -> None:
        manifest = _manifest(
            capabilities=[
                {"category": "fs", "action": "read", "target": "config.json"},
            ]
        )
        detected = [_cap(category="fs", action="read", target="other.json")]
        result = compare_capabilities(detected, manifest)
        assert not result.passed

    def test_detected_wildcard_target_matches_any_pattern(self) -> None:
        """When detected target is '*' (variable path), it matches any declared pattern.

        This is conservative — we accept it rather than false-positive, since
        we can't statically determine what the variable resolves to.
        """
        manifest = _manifest(
            capabilities=[
                {"category": "fs", "action": "read", "target": "config.json"},
            ]
        )
        detected = [_cap(category="fs", action="read", target="*")]
        result = compare_capabilities(detected, manifest)
        assert result.passed

    def test_mixed_pass_and_fail(self) -> None:
        """Some capabilities match, some don't."""
        manifest = _manifest(
            capabilities=[
                {"category": "fs", "action": "read", "target": "*"},
            ]
        )
        detected = [
            _cap(category="fs", action="read"),
            _cap(category="net", action="connect"),
        ]
        result = compare_capabilities(detected, manifest)
        assert not result.passed
        assert len(result.matched) == 1
        assert len(result.errors) == 1

    def test_empty_detected_passes(self) -> None:
        """No detected capabilities should always pass."""
        manifest = _manifest(
            capabilities=[
                {"category": "fs", "action": "read", "target": "*"},
            ]
        )
        result = compare_capabilities([], manifest)
        assert result.passed
        assert len(result.warnings) == 1  # Unused declaration

    def test_empty_both_passes(self) -> None:
        """Empty manifest and empty detection should pass."""
        manifest = _manifest(capabilities=[])
        result = compare_capabilities([], manifest)
        assert result.passed


# ── ComparisonResult summary ─────────────────────────────────────────


class TestComparisonResultSummary:
    """Tests for the human-readable summary output."""

    def test_pass_summary(self) -> None:
        result = ComparisonResult(passed=True, errors=[], warnings=[], matched=[])
        assert "PASS" in result.summary()

    def test_fail_summary(self) -> None:
        result = ComparisonResult(
            passed=False,
            errors=[_cap(category="net", action="connect", evidence="import socket")],
            warnings=[],
            matched=[],
        )
        summary = result.summary()
        assert "FAIL" in summary
        assert "Undeclared" in summary

    def test_warning_in_summary(self) -> None:
        result = ComparisonResult(
            passed=True,
            errors=[],
            warnings=[{"category": "net", "action": "connect", "target": "*"}],
            matched=[],
        )
        summary = result.summary()
        assert "PASS" in summary
        assert "Unused" in summary

    def test_matched_count_in_summary(self) -> None:
        result = ComparisonResult(
            passed=True,
            errors=[],
            warnings=[],
            matched=[_cap(), _cap()],
        )
        assert "2 capability(ies)" in result.summary()
