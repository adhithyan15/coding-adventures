"""Structured test-evidence and TFV summary emission (VER00 §8, VER13).

Emits deterministic, machine-verifiable assurance evidence summaries.
Zero external dependencies; pure in-memory computation.
"""

from __future__ import annotations

import json
from dataclasses import asdict, dataclass
from typing import Any

from coding_adventures_test_bootstrap.tfv import TFVRecord, format_tfv


@dataclass(frozen=True)
class EvidenceSummary:
    """Deterministic test and verification evidence summary."""

    package: str
    version: str
    unit_test_count: int
    property_test_count: int
    fuzz_iterations: int
    claims_proven: int
    capabilities_declared: list[str]
    verdict: str  # "pass", "fail", "report-only"
    seed: int | None = None

    def __post_init__(self) -> None:
        if self.verdict not in ("pass", "fail", "report-only"):
            raise ValueError(f"Invalid verdict: {self.verdict}")
        if self.unit_test_count < 0 or self.property_test_count < 0:
            raise ValueError("Test counts cannot be negative")
        if self.fuzz_iterations < 0 or self.claims_proven < 0:
            raise ValueError("Iterations and claims cannot be negative")

    def to_dict(self) -> dict[str, Any]:
        """Convert to deterministic dictionary representation."""
        data = asdict(self)
        if self.seed is None:
            del data["seed"]
        return data

    def emit_json(self) -> str:
        """Emit canonical deterministic JSON string (sorted keys, 2-space indent)."""
        data = self.to_dict()
        return json.dumps(data, indent=2, sort_keys=True) + "\n"

    def emit_tfv(self) -> str:
        """Emit TFV v1 record of this evidence summary."""
        fields: dict[str, Any] = {
            "package": self.package,
            "version": self.version,
            "unit_tests": self.unit_test_count,
            "property_tests": self.property_test_count,
            "fuzz_iterations": self.fuzz_iterations,
            "claims_proven": self.claims_proven,
            "verdict": self.verdict,
        }
        if self.seed is not None:
            fields["seed"] = self.seed
        record = TFVRecord(
            record_id=f"assurance/{self.package}",
            fields=fields,
        )
        return format_tfv([record])
