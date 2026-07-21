#!/usr/bin/env python3

"""Check all locally reproducible generated HTML fixture outputs.

This manifest intentionally covers the lexer and parser fixture generators that
can be checked from checked-in inputs. Generators that require upstream source
downloads are included when their source path is provided explicitly.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


FIXTURE_DIR = Path(__file__).resolve().parent
RUST_DIR = FIXTURE_DIR.parents[2]
WORKTREE_ROOT = RUST_DIR.parents[2]
PARSER_FIXTURE_DIR = RUST_DIR / "html-parser" / "tests" / "fixtures"


@dataclass(frozen=True)
class FixtureCheck:
    name: str
    command: tuple[str, ...]


def main() -> int:
    args = parse_args()
    checks = default_checks()

    if args.entities_json is not None:
        checks.append(
            FixtureCheck(
                "whatwg-entities",
                (
                    str(FIXTURE_DIR / "generate_whatwg_entities_fixture.py"),
                    str(Path(args.entities_json).expanduser().resolve()),
                    "--check",
                ),
            )
        )

    if args.html5lib_tests is not None:
        html5lib_tests = str(Path(args.html5lib_tests).expanduser().resolve())
        checks.extend(
            [
                FixtureCheck(
                    "html5lib-coverage-audit-report",
                    (
                        str(PARSER_FIXTURE_DIR / "audit_html5lib_coverage.py"),
                        html5lib_tests,
                        "--check-report",
                    ),
                ),
                FixtureCheck(
                    "html5lib-coverage-audit-counts",
                    (
                        str(PARSER_FIXTURE_DIR / "audit_html5lib_coverage.py"),
                        html5lib_tests,
                        "--expect-tree-upstream-cases",
                        "1778",
                        "--expect-tree-local-cases",
                        "2485",
                        "--expect-tokenizer-upstream-cases",
                        "6806",
                        "--expect-tokenizer-local-raw-cases",
                        "7015",
                        "--expect-normalized-cases",
                        "7242",
                        "--expect-normalized-skipped",
                        "0",
                    ),
                ),
            ]
        )

    for check in checks:
        run_check(check)
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run generated HTML lexer/parser fixture stale checks."
    )
    parser.add_argument(
        "--entities-json",
        help="Optional WHATWG entities.json source table for the entities fixture.",
    )
    parser.add_argument(
        "--html5lib-tests",
        help="Optional html5lib-tests checkout for coverage audit report checks.",
    )
    return parser.parse_args()


def default_checks() -> list[FixtureCheck]:
    return [
        FixtureCheck(
            "html-fixture-python-scripts-compile",
            (
                str(FIXTURE_DIR / "check_html_fixture_scripts_compile.py"),
                "--check",
            ),
        ),
        FixtureCheck(
            "html-fixture-case-identities",
            (
                str(FIXTURE_DIR / "check_html_fixture_case_ids.py"),
                "--check",
            ),
        ),
        FixtureCheck(
            "html-fixture-json-schemas",
            (
                str(FIXTURE_DIR / "check_html_fixture_schemas.py"),
                "--check",
            ),
        ),
        FixtureCheck(
            "html-fixture-format-registry",
            (
                str(FIXTURE_DIR / "check_html_fixture_format_registry.py"),
                "--check",
            ),
        ),
        FixtureCheck(
            "html-fixture-readme-inventory",
            (
                str(FIXTURE_DIR / "check_html_fixture_readme_inventory.py"),
                "--check",
            ),
        ),
        FixtureCheck(
            "html5lib-tokenizer-normalized",
            (
                str(FIXTURE_DIR / "normalize_html5lib_fixtures.py"),
                str(FIXTURE_DIR / "upstream-html5lib-smoke.test"),
                str(FIXTURE_DIR / "html5lib-smoke.json"),
                "--check",
            ),
        ),
        FixtureCheck(
            "html5lib-tokenizer-coverage",
            (
                str(FIXTURE_DIR / "check_html5lib_tokenizer_coverage.py"),
                "--check",
            ),
        ),
        FixtureCheck(
            "whatwg-lexer-fixture-metadata",
            (
                str(FIXTURE_DIR / "check_whatwg_lexer_fixture_metadata.py"),
                "--check",
            ),
        ),
        FixtureCheck(
            "whatwg-lexer-rust-tests",
            (
                str(FIXTURE_DIR / "check_whatwg_lexer_rust_tests.py"),
                "--check",
            ),
        ),
        *lexer_fixture_checks(),
        FixtureCheck(
            "whatwg-tree-insertion-audit",
            (
                str(PARSER_FIXTURE_DIR / "generate_whatwg_tree_insertion_audit_fixture.py"),
                "--check",
            ),
        ),
        FixtureCheck(
            "whatwg-frameset-audit",
            (
                str(PARSER_FIXTURE_DIR / "generate_whatwg_frameset_audit_fixture.py"),
                "--check",
            ),
        ),
        FixtureCheck(
            "whatwg-table-audit",
            (
                str(PARSER_FIXTURE_DIR / "generate_whatwg_table_audit_fixture.py"),
                "--check",
            ),
        ),
        FixtureCheck(
            "whatwg-form-interactive-audit",
            (
                str(PARSER_FIXTURE_DIR / "generate_whatwg_form_interactive_audit_fixture.py"),
                "--check",
            ),
        ),
        FixtureCheck(
            "whatwg-text-control-audit",
            (
                str(PARSER_FIXTURE_DIR / "generate_whatwg_text_control_audit_fixture.py"),
                "--check",
            ),
        ),
        FixtureCheck(
            "whatwg-foreign-audit",
            (
                str(PARSER_FIXTURE_DIR / "generate_whatwg_foreign_audit_fixture.py"),
                "--check",
            ),
        ),
        FixtureCheck(
            "whatwg-formatting-audit",
            (
                str(PARSER_FIXTURE_DIR / "generate_whatwg_formatting_audit_fixture.py"),
                "--check",
            ),
        ),
        FixtureCheck(
            "whatwg-ruby-audit",
            (
                str(PARSER_FIXTURE_DIR / "generate_whatwg_ruby_audit_fixture.py"),
                "--check",
            ),
        ),
        FixtureCheck(
            "whatwg-noscript-audit",
            (
                str(PARSER_FIXTURE_DIR / "generate_whatwg_noscript_audit_fixture.py"),
                "--check",
            ),
        ),
        FixtureCheck(
            "whatwg-head-body-audit",
            (
                str(PARSER_FIXTURE_DIR / "generate_whatwg_head_body_audit_fixture.py"),
                "--check",
            ),
        ),
        FixtureCheck(
            "whatwg-void-element-audit",
            (
                str(PARSER_FIXTURE_DIR / "generate_whatwg_void_element_audit_fixture.py"),
                "--check",
            ),
        ),
        FixtureCheck(
            "whatwg-list-item-audit",
            (
                str(PARSER_FIXTURE_DIR / "generate_whatwg_list_item_audit_fixture.py"),
                "--check",
            ),
        ),
        FixtureCheck(
            "whatwg-paragraph-audit",
            (
                str(PARSER_FIXTURE_DIR / "generate_whatwg_paragraph_audit_fixture.py"),
                "--check",
            ),
        ),
        FixtureCheck(
            "whatwg-block-boundary-audit",
            (
                str(PARSER_FIXTURE_DIR / "generate_whatwg_block_boundary_audit_fixture.py"),
                "--check",
            ),
        ),
        FixtureCheck(
            "whatwg-fragment-context-audit",
            (
                str(PARSER_FIXTURE_DIR / "generate_whatwg_fragment_context_audit_fixture.py"),
                "--check",
            ),
        ),
        FixtureCheck(
            "whatwg-character-reference-audit",
            (
                str(PARSER_FIXTURE_DIR / "generate_whatwg_character_reference_audit_fixture.py"),
                "--check",
            ),
        ),
        FixtureCheck(
            "whatwg-legacy-element-audit",
            (
                str(PARSER_FIXTURE_DIR / "generate_whatwg_legacy_element_audit_fixture.py"),
                "--check",
            ),
        ),
        FixtureCheck(
            "whatwg-document-shell-audit",
            (
                str(PARSER_FIXTURE_DIR / "generate_whatwg_document_shell_audit_fixture.py"),
                "--check",
            ),
        ),
        FixtureCheck(
            "whatwg-template-audit",
            (
                str(PARSER_FIXTURE_DIR / "generate_whatwg_template_audit_fixture.py"),
                "--check",
            ),
        ),
        FixtureCheck(
            "whatwg-select-list-audit",
            (
                str(PARSER_FIXTURE_DIR / "generate_whatwg_select_list_audit_fixture.py"),
                "--check",
            ),
        ),
        FixtureCheck(
            "whatwg-misc-recovery-audit",
            (
                str(PARSER_FIXTURE_DIR / "generate_whatwg_misc_recovery_audit_fixture.py"),
                "--check",
            ),
        ),
        FixtureCheck(
            "whatwg-parser-audit-coverage",
            (
                str(PARSER_FIXTURE_DIR / "check_whatwg_audit_coverage.py"),
                "--check",
            ),
        ),
        FixtureCheck(
            "html5lib-tree-construction-smoke",
            (
                str(PARSER_FIXTURE_DIR / "check_html5lib_tree_construction_smoke.py"),
                "--check",
            ),
        ),
        FixtureCheck(
            "whatwg-parser-audit-rust-tests",
            (
                str(PARSER_FIXTURE_DIR / "check_whatwg_audit_rust_tests.py"),
                "--check",
            ),
        ),
        FixtureCheck(
            "whatwg-parser-audit-manifest",
            (
                str(PARSER_FIXTURE_DIR / "check_whatwg_audit_manifest.py"),
                "--check",
            ),
        ),
    ]


def lexer_fixture_checks() -> list[FixtureCheck]:
    names = (
        "numeric_references",
        "character_reference_boundaries",
        "input_stream",
        "chunk_boundaries",
        "eof_recovery",
        "text_mode_delimiters",
        "script_escape_boundaries",
        "cdata_boundaries",
        "markup_declarations",
        "comment_boundaries",
        "attribute_edges",
        "attribute_boundaries",
        "tag_open_recovery",
        "doctype_boundaries",
        "text_mode_boundaries",
    )
    return [
        FixtureCheck(
            f"whatwg-{name.replace('_', '-')}",
            (str(FIXTURE_DIR / f"generate_whatwg_{name}_fixture.py"), "--check"),
        )
        for name in names
    ]


def run_check(check: FixtureCheck) -> None:
    command = (sys.executable, *check.command)
    print(f"checking {check.name}", flush=True)
    subprocess.run(command, cwd=WORKTREE_ROOT, check=True)


if __name__ == "__main__":
    raise SystemExit(main())
