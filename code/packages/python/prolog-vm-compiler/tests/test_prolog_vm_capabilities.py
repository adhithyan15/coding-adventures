"""Tests for the Prolog VM capability manifest."""

from __future__ import annotations

from prolog_vm_compiler import (
    deferred_prolog_vm_capabilities,
    prolog_vm_capabilities,
    prolog_vm_capability_manifest,
)


def test_manifest_marks_stream_io_extension_complete() -> None:
    manifest = prolog_vm_capability_manifest()

    assert manifest.track == "Prolog-on-Logic-VM PR00-PR79"
    assert manifest.status == "core-plus-stream-io"
    assert manifest.dialects == ("iso", "swi")
    assert manifest.backends == ("structured", "bytecode")
    assert manifest.complete_count == 9
    assert manifest.deferred_count == 3


def test_completed_capabilities_cover_pr00_through_pr79_once() -> None:
    covered_specs = [
        spec for capability in prolog_vm_capabilities() for spec in capability.specs
    ]

    assert covered_specs == [f"PR{index:02d}" for index in range(80)]


def test_capability_manifest_is_json_serializable() -> None:
    payload = prolog_vm_capability_manifest().as_dict()

    assert payload["status"] == "core-plus-stream-io"
    assert payload["capabilities"][0]["id"] == "frontend-loader"
    assert payload["capabilities"][-1]["id"] == "host-file-stream-io"
    assert payload["deferred_capabilities"][0]["status"] == "deferred"


def test_deferred_capabilities_are_explicitly_advanced_dialect_work() -> None:
    deferred = deferred_prolog_vm_capabilities()

    assert {capability.id for capability in deferred} == {
        "full-dialect-emulation",
        "advanced-solver-services",
        "host-runtime-services",
    }
    assert all(capability.status == "deferred" for capability in deferred)
