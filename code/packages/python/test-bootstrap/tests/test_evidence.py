"""Tests for evidence summary generation and emission."""

import json

import pytest

from coding_adventures_test_bootstrap.evidence import EvidenceSummary
from coding_adventures_test_bootstrap.tfv import parse_tfv


def test_evidence_summary_valid() -> None:
    ev = EvidenceSummary(
        package="python/test-pkg",
        version="0.1.0",
        unit_test_count=10,
        property_test_count=5,
        fuzz_iterations=1000,
        claims_proven=4,
        capabilities_declared=[],
        verdict="pass",
        seed=42,
    )

    d = ev.to_dict()
    assert d["package"] == "python/test-pkg"
    assert d["unit_test_count"] == 10
    assert d["verdict"] == "pass"

    # JSON emission
    json_str = ev.emit_json()
    parsed_json = json.loads(json_str)
    assert parsed_json["package"] == "python/test-pkg"
    assert parsed_json["seed"] == 42

    # TFV emission
    tfv_str = ev.emit_tfv()
    records = parse_tfv(tfv_str)
    assert len(records) == 1
    assert records[0].record_id == "assurance/python/test-pkg"
    assert records[0].fields["unit_tests"] == 10
    assert records[0].fields["verdict"] == "pass"
    assert records[0].fields["seed"] == 42


def test_evidence_summary_no_seed() -> None:
    ev = EvidenceSummary(
        package="python/no-seed",
        version="0.1.0",
        unit_test_count=1,
        property_test_count=1,
        fuzz_iterations=10,
        claims_proven=1,
        capabilities_declared=["fs:read"],
        verdict="report-only",
        seed=None,
    )
    d = ev.to_dict()
    assert "seed" not in d
    tfv_str = ev.emit_tfv()
    records = parse_tfv(tfv_str)
    assert "seed" not in records[0].fields


def test_evidence_summary_validation_errors() -> None:
    with pytest.raises(ValueError, match="Invalid verdict"):
        EvidenceSummary(
            package="p",
            version="1",
            unit_test_count=1,
            property_test_count=1,
            fuzz_iterations=1,
            claims_proven=1,
            capabilities_declared=[],
            verdict="unknown",
        )

    with pytest.raises(ValueError, match="cannot be negative"):
        EvidenceSummary(
            package="p",
            version="1",
            unit_test_count=-1,
            property_test_count=1,
            fuzz_iterations=1,
            claims_proven=1,
            capabilities_declared=[],
            verdict="pass",
        )

    with pytest.raises(ValueError, match="cannot be negative"):
        EvidenceSummary(
            package="p",
            version="1",
            unit_test_count=1,
            property_test_count=1,
            fuzz_iterations=-1,
            claims_proven=1,
            capabilities_declared=[],
            verdict="pass",
        )
