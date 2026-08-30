"""Tests for the pure extra-CI toolchain declaration boundary."""

from __future__ import annotations

import json
from copy import deepcopy
from pathlib import Path

import pytest

from build_tool.toolchain_detection import (
    CANONICAL_TOOLCHAINS,
    evaluate_snapshot,
    parse_extra_toolchains,
)

REPO_ROOT = Path(__file__).resolve().parents[5]
CONFORMANCE_CASES = (
    REPO_ROOT / "code" / "specs" / "fixtures" / "build-tool-v1" / "cases"
)
EXPECTED_FIXTURES = (
    "toolchain-detection-affected-only.json",
    "toolchain-detection-crlf-grammar.json",
    "toolchain-detection-declarations.json",
    "toolchain-detection-empty.json",
    "toolchain-detection-force-full.json",
    "toolchain-detection-null-all.json",
    "toolchain-detection-platform-darwin.json",
    "toolchain-detection-platform-linux.json",
    "toolchain-detection-platform-windows.json",
    "toolchain-detection-shared.json",
    "toolchain-detection-unsupported.json",
)


def _package(
    *,
    name: str = "rust/app",
    language: str = "rust",
    build_files: dict[str, str] | None = None,
) -> dict[str, object]:
    return {
        "name": name,
        "language": language,
        "build_files": {"BUILD": ""} if build_files is None else build_files,
    }


def test_independently_consumes_every_neutral_toolchain_fixture():
    fixture_paths = sorted(CONFORMANCE_CASES.glob("toolchain-detection-*.json"))
    fixture_names = tuple(path.name for path in fixture_paths)
    assert fixture_names == EXPECTED_FIXTURES

    for fixture_name in fixture_names:
        fixture = json.loads(
            (CONFORMANCE_CASES / fixture_name).read_text(encoding="utf-8")
        )
        options = fixture["input"]["options"]
        expected = fixture["expected"]

        actual = evaluate_snapshot(
            options["platform"],
            options["force_full"],
            options["packages"],
            options["scheduled_packages"],
            options["forced_toolchains"],
        )

        assert actual["outcome"] == expected["outcome"], fixture["id"]
        assert actual["toolchains"] == expected["result"].get("toolchains", {}), (
            fixture["id"]
        )
        assert actual["diagnostics"] == expected["diagnostics"], fixture["id"]


def test_enforces_exact_utf8_byte_ceiling():
    exact_ascii = _package(build_files={"BUILD": "x" * 65_536})
    exact_unicode = _package(build_files={"BUILD": "é" * 32_768})

    assert evaluate_snapshot("linux", False, [exact_ascii], None, [])["outcome"] == "ok"
    assert (
        evaluate_snapshot("linux", False, [exact_unicode], None, [])["outcome"] == "ok"
    )

    with pytest.raises(ValueError, match="per-file resource ceiling"):
        evaluate_snapshot(
            "linux",
            False,
            [_package(build_files={"BUILD": "x" * 65_537})],
            None,
            [],
        )
    with pytest.raises(ValueError, match="per-file resource ceiling"):
        evaluate_snapshot(
            "linux",
            False,
            [_package(build_files={"BUILD": "é" * 32_769})],
            None,
            [],
        )


def test_enforces_exact_logical_line_ceiling():
    exact = _package(build_files={"BUILD": "\n" * 4_095})
    assert evaluate_snapshot("linux", False, [exact], None, [])["outcome"] == "ok"

    with pytest.raises(ValueError, match="per-file resource ceiling"):
        evaluate_snapshot(
            "linux",
            False,
            [_package(build_files={"BUILD": "\n" * 4_096})],
            None,
            [],
        )


def test_enforces_exact_aggregate_byte_ceiling_across_every_front():
    exact_files = {f"BUILD_{index}": "x" * 65_536 for index in range(16)}
    assert (
        evaluate_snapshot(
            "linux", False, [_package(build_files=exact_files)], None, []
        )["outcome"]
        == "ok"
    )

    oversized_files = {f"BUILD_{index}": "x" * 65_536 for index in range(17)}
    with pytest.raises(ValueError, match="aggregate resource ceiling"):
        evaluate_snapshot(
            "linux", False, [_package(build_files=oversized_files)], None, []
        )

    with pytest.raises(ValueError, match="per-file resource ceiling"):
        evaluate_snapshot(
            "linux",
            False,
            [
                _package(
                    build_files={
                        "BUILD": "",
                        "BUILD_windows": "x" * 65_537,
                    }
                )
            ],
            None,
            [],
        )


def test_keeps_declaration_grammar_byte_exact_across_crlf_and_lone_cr():
    assert parse_extra_toolchains(
        "  # needs-toolchain: python  \r\n\t# needs-toolchain:\tjava\t\r\n"
    ) == ["python", "java"]
    assert parse_extra_toolchains("# needs-toolchain: python\r") == []
    assert parse_extra_toolchains("# needs-toolchain: lua\r  ") == []
    assert parse_extra_toolchains("# needs-toolchain: swift\r\r\n") == []


def test_stably_deduplicates_only_exact_canonical_declarations():
    assert parse_extra_toolchains(
        "\n".join(
            (
                "# needs-toolchain: python",
                "# needs-toolchain:\tjava",
                "# needs-toolchain: python",
                "# needs-toolchain: Python",
                "# needs-toolchain:zig",
                "# needs-toolchain: java suffix",
            )
        )
    ) == ["python", "java"]


def test_preserves_empty_front_precedence_and_caller_owned_inputs():
    packages = [
        _package(
            build_files={
                "BUILD": "# needs-toolchain: java\n",
                "BUILD_windows": "",
            }
        )
    ]
    before = deepcopy(packages)

    actual = evaluate_snapshot("windows", False, packages, None, ["kotlin"])

    assert actual["toolchains"]["rust"] is True
    assert actual["toolchains"]["kotlin"] is True
    assert actual["toolchains"]["java"] is False
    assert packages == before


def test_null_and_empty_schedules_remain_distinct():
    packages = [_package()]

    all_packages = evaluate_snapshot("linux", False, packages, None, [])
    no_packages = evaluate_snapshot("linux", False, packages, [], [])

    assert all_packages["toolchains"]["rust"] is True
    assert not any(no_packages["toolchains"].values())


def test_returns_immutable_registry_and_fresh_complete_maps():
    assert tuple(sorted(CANONICAL_TOOLCHAINS)) == CANONICAL_TOOLCHAINS
    assert len(CANONICAL_TOOLCHAINS) == 16

    packages = [_package()]
    first = evaluate_snapshot("linux", False, packages, None, [])
    first["toolchains"]["cpp"] = True
    second = evaluate_snapshot("linux", False, packages, None, [])

    assert second["toolchains"] is not first["toolchains"]
    assert tuple(second["toolchains"]) == CANONICAL_TOOLCHAINS
    assert second["toolchains"]["cpp"] is False


def test_keeps_unsupported_package_and_forced_diagnostics_stable():
    unsupported_package = evaluate_snapshot(
        "linux",
        True,
        [_package(name="zig/app", language="zig")],
        None,
        [],
    )
    assert unsupported_package["diagnostics"] == [
        {
            "code": "TOOLCHAIN_UNSUPPORTED",
            "severity": "error",
            "package": "zig/app",
        }
    ]

    unsupported_forced = evaluate_snapshot("linux", False, [_package()], [], ["zig"])
    assert unsupported_forced["diagnostics"] == [
        {"code": "TOOLCHAIN_UNSUPPORTED", "severity": "error"}
    ]


def test_selected_unsupported_package_precedes_invalid_forced_toolchain():
    actual = evaluate_snapshot(
        "linux",
        False,
        [_package(name="zig/app", language="zig")],
        None,
        ["zig"],
    )

    assert actual["diagnostics"] == [
        {
            "code": "TOOLCHAIN_UNSUPPORTED",
            "severity": "error",
            "package": "zig/app",
        }
    ]
