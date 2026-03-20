"""Manifest loading and capability comparison.

This module loads a package's `required_capabilities.json` manifest and
compares it against the capabilities detected by the analyzer. The
comparison answers the question: "Does this package use only the
capabilities it declared?"

## Comparison Logic

The comparison is asymmetric:

- **Undeclared capabilities** (detected but not in manifest) are ERRORS.
  The code uses something it didn't declare.

- **Unused declarations** (in manifest but not detected) are WARNINGS.
  The manifest declares a capability the code doesn't use. This isn't
  a security issue — it's just a stale declaration.

## Default Deny

If no `required_capabilities.json` exists, the package is treated as
having zero declared capabilities. Any detected capability is an error.
This is the "no manifest = block everything" principle from the spec.

## Target Matching

When comparing detected targets against declared targets, we use glob-
style matching:

- `../../grammars/*.tokens` matches `../../grammars/python.tokens`
- `*` matches anything
- Exact strings match exactly

This mirrors OpenBSD's `unveil()` path matching.
"""

import json
from dataclasses import dataclass, field
from fnmatch import fnmatch
from pathlib import Path

from ca_capability_analyzer.analyzer import DetectedCapability


@dataclass
class Manifest:
    """A parsed required_capabilities.json manifest.

    Attributes:
        package:      Qualified package name (e.g., "python/logic-gates").
        capabilities: List of declared capability dicts.
        justification: Human-readable justification.
        banned_construct_exceptions: List of exempted banned constructs.
        path:         Path to the manifest file (if loaded from file).
    """

    package: str
    capabilities: list[dict[str, str]]
    justification: str
    banned_construct_exceptions: list[dict[str, str]] = field(default_factory=list)
    path: str | None = None

    @property
    def is_empty(self) -> bool:
        """True if the manifest declares zero capabilities."""
        return len(self.capabilities) == 0


def load_manifest(path: str | Path) -> Manifest:
    """Load a manifest from a JSON file.

    Args:
        path: Path to required_capabilities.json.

    Returns:
        Parsed Manifest object.

    Raises:
        FileNotFoundError: If the file does not exist.
        json.JSONDecodeError: If the file is not valid JSON.
        KeyError: If required fields are missing.
    """
    path = Path(path)
    with open(path, encoding="utf-8") as f:
        data = json.load(f)

    return Manifest(
        package=data["package"],
        capabilities=data.get("capabilities", []),
        justification=data.get("justification", ""),
        banned_construct_exceptions=data.get("banned_construct_exceptions", []),
        path=str(path),
    )


def default_manifest(package_name: str) -> Manifest:
    """Create a default (empty) manifest for a package without one.

    This represents the "no manifest = default deny" policy. A package
    without a required_capabilities.json is treated as declaring zero
    capabilities.
    """
    return Manifest(
        package=package_name,
        capabilities=[],
        justification="No manifest file — default deny (zero capabilities).",
    )


@dataclass
class ComparisonResult:
    """Result of comparing detected capabilities against a manifest.

    Attributes:
        passed:     True if all detected capabilities are declared.
        errors:     Detected capabilities not in the manifest (violations).
        warnings:   Declared capabilities not detected (stale declarations).
        matched:    Detected capabilities that matched a declaration.
    """

    passed: bool
    errors: list[DetectedCapability]
    warnings: list[dict[str, str]]
    matched: list[DetectedCapability]

    def summary(self) -> str:
        """Human-readable summary of the comparison result."""
        lines = []
        if self.passed:
            lines.append("PASS — all detected capabilities are declared.")
        else:
            lines.append(
                f"FAIL — {len(self.errors)} undeclared capability(ies) detected."
            )

        if self.errors:
            lines.append("\nUndeclared capabilities (ERRORS):")
            for cap in self.errors:
                lines.append(f"  {cap.file}:{cap.line}: {cap} ({cap.evidence})")

        if self.warnings:
            lines.append("\nUnused declarations (WARNINGS):")
            for decl in self.warnings:
                lines.append(f"  {decl['category']}:{decl['action']}:{decl['target']}")

        if self.matched:
            lines.append(f"\nMatched: {len(self.matched)} capability(ies).")

        return "\n".join(lines)


def _target_matches(pattern: str, actual: str) -> bool:
    """Check if a detected target matches a declared target pattern.

    Uses fnmatch-style glob matching:
    - "*" matches anything
    - "../../grammars/*.tokens" matches "../../grammars/python.tokens"
    - "file.txt" matches "file.txt" exactly

    Args:
        pattern: The declared target (from manifest).
        actual:  The detected target (from analyzer).

    Returns:
        True if the actual target matches the pattern.
    """
    if pattern == "*":
        return True
    if actual == "*":
        # Detected target is wildcard (non-literal) — it matches any
        # declared pattern, since we can't know what it will resolve to.
        # This is conservative: we accept it rather than false-positive.
        return True
    return fnmatch(actual, pattern)


def _capability_matches(
    declared: dict[str, str],
    detected: DetectedCapability,
) -> bool:
    """Check if a detected capability matches a declared one.

    A match requires:
    1. Same category (fs, net, proc, etc.)
    2. Compatible action (exact match, or declared is "*")
    3. Compatible target (glob match)
    """
    if declared["category"] != detected.category:
        return False
    if declared["action"] != "*" and declared["action"] != detected.action:
        return False
    return _target_matches(declared["target"], detected.target)


def compare_capabilities(
    detected: list[DetectedCapability],
    manifest: Manifest,
) -> ComparisonResult:
    """Compare detected capabilities against a manifest.

    This is the core comparison logic used by the CI gate. It determines
    whether a package's source code uses only the capabilities it declared.

    Args:
        detected: Capabilities found by the analyzer.
        manifest: The package's declared capabilities.

    Returns:
        ComparisonResult with pass/fail status, errors, and warnings.
    """
    errors: list[DetectedCapability] = []
    matched: list[DetectedCapability] = []

    for cap in detected:
        found_match = False
        for decl in manifest.capabilities:
            if _capability_matches(decl, cap):
                found_match = True
                break
        if found_match:
            matched.append(cap)
        else:
            errors.append(cap)

    # Find unused declarations (warnings)
    used_declarations: set[int] = set()
    for cap in detected:
        for i, decl in enumerate(manifest.capabilities):
            if _capability_matches(decl, cap):
                used_declarations.add(i)
                break

    warnings = [
        decl
        for i, decl in enumerate(manifest.capabilities)
        if i not in used_declarations
    ]

    return ComparisonResult(
        passed=len(errors) == 0,
        errors=errors,
        warnings=warnings,
        matched=matched,
    )
