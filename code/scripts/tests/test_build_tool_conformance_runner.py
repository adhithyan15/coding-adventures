from __future__ import annotations

import copy
import hashlib
import io
import json
import os
import re
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest import mock

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS_DIR))

import build_tool_conformance as runner

FIXTURE_ROOT = runner.DEFAULT_FIXTURE_ROOT
CASES_ROOT = FIXTURE_ROOT / "cases"


def load_case(name: str) -> dict[str, object]:
    return runner.load_document(CASES_ROOT / name)


class StrictJsonTests(unittest.TestCase):
    def assert_parse_error(self, raw: bytes, code: str) -> None:
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.strict_load_bytes(raw)
        self.assertEqual(raised.exception.code, code)

    def test_rejects_ambiguous_and_nonportable_json(self) -> None:
        cases = {
            b'{"domain":"discovery","domain":"execution"}': "JSON_DUPLICATE_KEY",
            b'{"value":NaN}': "JSON_NON_FINITE",
            b'{"value":1.5}': "JSON_FLOAT_FORBIDDEN",
            b'{"value":9007199254740992}': "JSON_INTEGER_RANGE",
            b'{"value":"\\ud800"}': "JSON_UNICODE_SURROGATE",
            b"\xef\xbb\xbf{}": "JSON_BOM_FORBIDDEN",
        }
        for raw, code in cases.items():
            with self.subTest(code=code):
                self.assert_parse_error(raw, code)

    def test_rejects_invalid_utf8_syntax_delimiters_and_top_level_values(
        self,
    ) -> None:
        cases = {
            b'{"value":"\xff"}': "JSON_UTF8_INVALID",
            b"}": "JSON_SYNTAX_INVALID",
            b'{"value":': "JSON_SYNTAX_INVALID",
            b"[]": "JSON_TOP_LEVEL_INVALID",
        }
        for raw, code in cases.items():
            with self.subTest(code=code):
                self.assert_parse_error(raw, code)

    def test_rejects_moderate_and_extreme_depth_with_the_same_code(self) -> None:
        for depth in (65, 1100):
            raw = b'{"value":' + (b"[" * depth) + b"0" + (b"]" * depth) + b"}"
            with self.subTest(depth=depth):
                self.assert_parse_error(raw, "JSON_DEPTH_EXCEEDED")

    def test_rejects_oversized_input_before_decoding(self) -> None:
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.strict_load_bytes(b'{"value":"oversized"}', max_bytes=8)
        self.assertEqual(raised.exception.code, "JSON_INPUT_TOO_LARGE")

    def test_load_document_reports_missing_files(self) -> None:
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.load_document(Path("definitely-not-present.json"))
        self.assertEqual(raised.exception.code, "DOCUMENT_READ_FAILED")

    def test_load_document_bounds_the_file_read_and_rejects_links(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            oversized = root / "oversized.json"
            oversized.write_bytes(b'{"value":"' + (b"x" * 100) + b'"}')
            with self.assertRaises(runner.ConformanceError) as raised:
                runner.load_document(oversized, max_bytes=16)
            self.assertEqual(raised.exception.code, "JSON_INPUT_TOO_LARGE")

            target = root / "target.json"
            target.write_text("{}", encoding="utf-8")
            link = root / "link.json"
            try:
                link.symlink_to(target)
            except OSError:
                return
            with self.assertRaises(runner.ConformanceError) as raised:
                runner.load_document(link)
            self.assertIn(
                raised.exception.code,
                {"DOCUMENT_READ_FAILED", "DOCUMENT_TYPE_INVALID"},
            )

    def test_portable_path_validation_covers_canonical_edge_cases(self) -> None:
        self.assertIsNotNone(runner.portable_path_error(None))
        self.assertIsNotNone(runner.portable_path_error("a" * 513))
        self.assertIsNotNone(runner.portable_path_error("fixtures/e\u0301.txt"))
        self.assertIsNotNone(runner.portable_path_error("fixtures/name."))
        self.assertIsNotNone(runner.portable_path_error("fixtures/COM¹.txt"))
        self.assertIsNotNone(runner.portable_path_error("fixtures/LPT².txt"))
        self.assertIsNone(runner.portable_path_error("fixtures/.hidden"))
        self.assertIsNone(runner.portable_glob_error("src/foo.*"))
        self.assertIsNone(runner.portable_glob_error("src/*.*"))
        self.assertIsNotNone(runner.portable_glob_error("src/foo."))

    def test_portable_glob_character_classes_match_neutral_semantics(self) -> None:
        cases = (
            ("src/[!a].cs", "src/b.cs", True),
            ("src/[!a].cs", "src/a.cs", False),
            ("src/[]].cs", "src/].cs", True),
            ("src/[-a].cs", "src/-.cs", True),
            ("src/[a-].cs", "src/-.cs", True),
            ("src/[a-c].cs", "src/b.cs", True),
            ("src/[.cs", "src/[.cs", True),
            ("src/[^].cs", "src/^.cs", True),
        )
        for pattern, path, expected in cases:
            with self.subTest(pattern=pattern, path=path):
                self.assertEqual(
                    expected,
                    runner._portable_glob_matches(pattern, path),
                )

    def test_schema_validation_never_retrieves_external_references(self) -> None:
        for keyword in ("$ref", "$dynamicRef"):
            schema = {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                keyword: "http://127.0.0.1:9/schema.json",
            }
            with (
                self.subTest(keyword=keyword),
                mock.patch("urllib.request.urlopen") as retrieve,
                self.assertRaises(runner.ConformanceError) as raised,
            ):
                runner._schema_errors({}, schema)
            self.assertEqual(raised.exception.code, "SCHEMA_REFERENCE_FORBIDDEN")
            retrieve.assert_not_called()


class CorpusTests(unittest.TestCase):
    def test_checked_in_corpus_and_manifest_validate(self) -> None:
        summary = runner.validate_corpus(FIXTURE_ROOT)

        self.assertEqual(summary["schema_version"], 1)
        self.assertEqual(summary["case_count"], 141)
        self.assertEqual(summary["implementation_count"], 16)
        self.assertEqual(summary["established_languages"], 15)
        self.assertEqual(summary["execution_case_count"], 0)
        self.assertEqual(summary["front_door_count"], 12)
        self.assertEqual(summary["adapter_ready_count"], 0)
        self.assertEqual(summary["conformance_run_count"], 0)
        self.assertEqual(summary["conformance_status"], "not-run")
        self.assertEqual(summary["source_input_language_count"], 23)
        self.assertEqual(len(summary["source_input_registry_sha256"]), 64)
        self.assertEqual(summary["repository_source_boundary_count"], 18)
        self.assertEqual(summary["repository_source_input_count"], 21)
        self.assertEqual(len(summary["repository_source_boundary_sha256"]), 64)
        self.assertEqual(
            summary["domains"],
            [
                "ci_gate_selection",
                "cli",
                "diff_selection",
                "discovery",
                "graph",
                "hashing_cache",
                "plan",
                "resolution",
                "sharding",
                "source_collection",
                "starlark",
                "toolchain_detection",
                "validation",
            ],
        )

    def test_manifest_classifies_every_established_front_door(self) -> None:
        manifest = runner.load_document(FIXTURE_ROOT / "implementations.json")
        implementations = {
            item["language"]: item for item in manifest["implementations"]
        }
        established = {
            language
            for language, item in implementations.items()
            if item["lane_status"] == "established"
        }
        present = {
            language
            for language, item in implementations.items()
            if item["implementation_status"] in {"present", "shared_engine"}
        }
        missing = {
            language
            for language, item in implementations.items()
            if item["implementation_status"] == "missing"
        }

        self.assertEqual(established, set(runner.ESTABLISHED_LANGUAGES))
        self.assertEqual(
            present,
            {
                "csharp",
                "elixir",
                "fsharp",
                "go",
                "haskell",
                "lua",
                "perl",
                "python",
                "ruby",
                "rust",
                "swift",
                "typescript",
            },
        )
        self.assertEqual(missing, {"dart", "java", "kotlin", "ocaml"})
        self.assertEqual(implementations["fsharp"]["shared_engine"], "csharp")
        self.assertEqual(implementations["ocaml"]["lane_status"], "emerging")

    def test_language_source_input_registry_is_closed_and_canonical(self) -> None:
        schema = runner.load_document(
            FIXTURE_ROOT / "language-source-input-registry.schema.json"
        )
        registry = runner.load_document(
            FIXTURE_ROOT / "language-source-input-registry.json"
        )
        summary = runner._validate_source_input_registry(registry, schema)

        self.assertEqual(summary["language_count"], 23)
        self.assertEqual(
            {entry["language"] for entry in registry["languages"]},
            set(runner.CLI_LANGUAGES) - {"all"},
        )
        self.assertEqual(
            registry["universal_inputs"]["root_exact_basenames"],
            ["required_capabilities.json"],
        )
        self.assertEqual(
            registry["universal_inputs"]["build_filenames"],
            [
                "BUILD",
                "BUILD_linux",
                "BUILD_mac",
                "BUILD_mac_and_linux",
                "BUILD_windows",
            ],
        )
        self.assertEqual(
            registry["universal_inputs"]["generated_directory_components"],
            sorted(
                runner.SOURCE_COLLECTION_SKIP_COMPONENTS,
                key=lambda value: value.encode("utf-8"),
            ),
        )
        by_language = {entry["language"]: entry for entry in registry["languages"]}

        def scoped(language: str, rule_id: str) -> dict[str, object]:
            return next(
                rule
                for rule in by_language[language]["scoped_inputs"]
                if rule["id"] == rule_id
            )

        self.assertIn(".cs", by_language["csharp"]["recursive_suffixes"])
        self.assertIn(".fs", by_language["fsharp"]["recursive_suffixes"])
        self.assertIn(".java", by_language["java"]["recursive_suffixes"])
        self.assertIn(".kt", by_language["kotlin"]["recursive_suffixes"])
        self.assertIn(".dart", by_language["dart"]["recursive_suffixes"])
        self.assertIn(".ml", by_language["ocaml"]["recursive_suffixes"])
        self.assertIn("spec.json", by_language["go"]["root_exact_basenames"])
        self.assertIn("basename.json", by_language["go"]["root_exact_basenames"])
        self.assertIn("tools/run.sh", by_language["c"]["root_exact_relative_paths"])
        self.assertIn(
            "regen-embedded-grammars.sh",
            by_language["swift"]["root_exact_relative_paths"],
        )
        self.assertTrue(by_language["dart"]["scoped_inputs"])
        self.assertTrue(by_language["rust"]["scoped_inputs"])
        self.assertTrue(by_language["swift"]["scoped_inputs"])
        self.assertTrue(by_language["typescript"]["scoped_inputs"])
        self.assertIn(
            "android/gradle/wrapper/gradle-wrapper.properties",
            by_language["dart"]["root_exact_relative_paths"],
        )
        dart_android = next(
            rule
            for rule in by_language["dart"]["scoped_inputs"]
            if rule["id"] == "dart-flutter-android-host-inputs"
        )
        self.assertNotIn(".properties", dart_android["suffixes"])
        self.assertIn(
            ".csproj",
            next(
                rule
                for rule in by_language["csharp"]["scoped_inputs"]
                if rule["id"] == "csharp-tests-project-inputs"
            )["suffixes"],
        )
        self.assertIn(
            ".csproj",
            next(
                rule
                for rule in by_language["csharp"]["scoped_inputs"]
                if rule["id"] == "csharp-winui-project-inputs"
            )["suffixes"],
        )
        self.assertIn(
            "gradle/wrapper/gradle-wrapper.properties",
            by_language["kotlin"]["root_exact_relative_paths"],
        )
        self.assertIn(
            ".ts",
            next(
                rule
                for rule in by_language["mosaic"]["scoped_inputs"]
                if rule["id"] == "mosaic-host-source-inputs"
            )["suffixes"],
        )
        self.assertIn(
            ".rs",
            next(
                rule
                for rule in by_language["mosaic"]["scoped_inputs"]
                if rule["id"] == "mosaic-tests-source-inputs"
            )["suffixes"],
        )
        self.assertIn("py.typed", by_language["python"]["recursive_exact_basenames"])
        self.assertIn(
            ".tw",
            next(
                rule
                for rule in by_language["python"]["scoped_inputs"]
                if rule["id"] == "python-package-resource-inputs"
            )["suffixes"],
        )
        self.assertIn(
            ".alg",
            next(
                rule
                for rule in by_language["python"]["scoped_inputs"]
                if rule["id"] == "python-tests-resource-inputs"
            )["suffixes"],
        )
        self.assertIn(
            ".swift",
            next(
                rule
                for rule in by_language["rust"]["scoped_inputs"]
                if rule["id"] == "rust-template-source-inputs"
            )["suffixes"],
        )
        self.assertNotIn(
            "js/smoke.mjs",
            by_language["rust"]["root_exact_relative_paths"],
        )
        engram_inputs = next(
            item
            for item in by_language["rust"]["package_exact_inputs"]
            if item["id"] == "rust-engram-wasm-build-inputs"
        )
        self.assertEqual(
            engram_inputs["package_root"],
            "code/packages/rust/engram-wasm",
        )
        self.assertEqual(
            engram_inputs["paths"],
            [
                "js/engram-mosaic-host-wasm.mjs",
                "js/smoke.mjs",
                "pkg/engram_engine.wasm",
            ],
        )
        self.assertIn(
            ".csproj",
            next(
                rule
                for rule in by_language["rust"]["scoped_inputs"]
                if rule["id"] == "rust-test-host-inputs"
            )["suffixes"],
        )
        self.assertIn(
            ".mil",
            next(
                rule
                for rule in by_language["rust"]["scoped_inputs"]
                if rule["id"] == "rust-mosaic-package-inputs"
            )["suffixes"],
        )
        self.assertIn(
            ".json",
            next(
                rule
                for rule in by_language["typescript"]["scoped_inputs"]
                if rule["id"] == "typescript-test-resource-inputs"
            )["suffixes"],
        )
        self.assertIn(".csv", scoped("go", "go-testdata-resource-inputs")["suffixes"])
        self.assertIn(".wasm", scoped("dart", "dart-test-resource-inputs")["suffixes"])
        self.assertIn(
            ".json", scoped("elixir", "elixir-test-resource-inputs")["suffixes"]
        )
        self.assertIn(
            ".csv", scoped("fsharp", "fsharp-tests-project-inputs")["suffixes"]
        )
        self.assertIn(".json", scoped("lua", "lua-tests-resource-inputs")["suffixes"])
        self.assertIn(".csv", scoped("perl", "perl-test-resource-inputs")["suffixes"])
        self.assertIn(".json", scoped("ruby", "ruby-test-resource-inputs")["suffixes"])
        self.assertIn(".py", scoped("ruby", "ruby-test-resource-inputs")["suffixes"])
        self.assertIn(".toml", scoped("ruby", "ruby-test-resource-inputs")["suffixes"])
        self.assertIn(".wast", scoped("rust", "rust-tests-resource-inputs")["suffixes"])
        self.assertIn(
            ".lattice",
            scoped("typescript", "typescript-source-resource-inputs")["suffixes"],
        )
        self.assertIn(
            "grammar-tools.cli.json",
            scoped("typescript", "typescript-program-config-inputs")["exact_basenames"],
        )

    def test_repository_source_input_boundary_is_closed_and_canonical(self) -> None:
        source_registry = runner.load_document(
            FIXTURE_ROOT / "language-source-input-registry.json"
        )
        schema = runner.load_document(
            FIXTURE_ROOT / "repository-source-input-boundary.schema.json"
        )
        boundary = runner.load_document(
            FIXTURE_ROOT / "repository-source-input-boundary.json"
        )
        summary = runner._validate_repository_source_input_boundary(
            boundary,
            schema,
            source_registry,
        )

        self.assertEqual(
            summary,
            {
                "boundary_count": 18,
                "input_count": 21,
                "scope_count": 483,
                "authorization_count": 486,
            },
        )
        self.assertEqual(
            runner.repository_source_input_boundary_digest(boundary),
            "963cc4090e165752fd3a62921b699dfff8f0677b49d7236812398a8abed0a25f",
        )
        by_id = {entry["id"]: entry for entry in boundary["boundaries"]}
        self.assertEqual(
            by_id["rust-root-workspace-manifest"]["applies_to"]["excluded_roots"],
            [
                "code/packages/rust/erl-nif-bridge",
                "code/packages/rust/lua-bridge",
                "code/packages/rust/node-bridge",
                "code/packages/rust/os-kernel",
                "code/packages/rust/perl-bridge",
                "code/packages/rust/python-bridge",
                "code/packages/rust/ruby-bridge",
            ],
        )
        self.assertEqual(
            by_id["typescript-program-workspace-configuration"]["inputs"],
            [
                {
                    "path": "code/packages/typescript/tsconfig.base.json",
                    "role": "cross_package_exact",
                }
            ],
        )
        self.assertEqual(
            [
                item["path"]
                for item in by_id["typescript-visicalc-deno-cross-package-inputs"][
                    "inputs"
                ]
            ],
            [
                "code/programs/typescript/visicalc-html/index.html",
                "code/programs/typescript/visicalc-html/vendor/spreadsheet-engine-wasm.js",
            ],
        )
        tracked_paths = {
            item["path"] for entry in boundary["boundaries"] for item in entry["inputs"]
        }
        staged = (
            subprocess.run(
                ["git", "ls-files", "--stage", "-z", "--", *sorted(tracked_paths)],
                cwd=runner.REPO_ROOT,
                check=True,
                capture_output=True,
            )
            .stdout.rstrip(b"\0")
            .split(b"\0")
        )
        listed: set[str] = set()
        for record in staged:
            header, raw_path = record.split(b"\t", 1)
            mode = header.split(b" ", 1)[0]
            self.assertIn(mode, {b"100644", b"100755"})
            listed.add(raw_path.decode("utf-8"))
        self.assertEqual(listed, tracked_paths)
        candidate_hex_limit = runner.load_document(
            FIXTURE_ROOT / "pure-domains.schema.json"
        )["$defs"]["repository_source_candidate_file"]["properties"]["content_hex"][
            "maxLength"
        ]
        largest_input_bytes = max(
            (runner.REPO_ROOT / path).stat().st_size for path in tracked_paths
        )
        self.assertLessEqual(largest_input_bytes * 2, candidate_hex_limit)
        self.assertLess(
            largest_input_bytes * 2,
            runner.MAX_REPOSITORY_SOURCE_DOCUMENT_BYTES,
        )
        for program in ("ircd", "macsyma-browser-repl"):
            tsconfig = (
                runner.REPO_ROOT
                / "code"
                / "programs"
                / "typescript"
                / program
                / "tsconfig.json"
            ).read_text(encoding="utf-8")
            self.assertIn("../../../packages/typescript/tsconfig.base.json", tsconfig)
        visicalc_deno = (
            runner.REPO_ROOT
            / "code"
            / "programs"
            / "typescript"
            / "visicalc-deno"
            / "main.ts"
        ).read_text(encoding="utf-8")
        self.assertIn('from "../visicalc-html/index.html"', visicalc_deno)
        self.assertIn(
            'from "../visicalc-html/vendor/spreadsheet-engine-wasm.js"',
            visicalc_deno,
        )

        def build_roots(root: Path, needle: str | None = None) -> list[str]:
            roots: set[str] = set()
            for build_file in root.rglob("BUILD*"):
                if not build_file.is_file() or build_file.name not in {
                    "BUILD",
                    "BUILD_windows",
                }:
                    continue
                if needle is not None and needle not in build_file.read_text(
                    encoding="utf-8"
                ):
                    continue
                roots.add(build_file.parent.relative_to(runner.REPO_ROOT).as_posix())
            return sorted(roots, key=lambda path: path.encode("utf-8"))

        haskell_root = runner.REPO_ROOT / "code/packages/haskell"
        expected_haskell = [
            root
            for root in build_roots(haskell_root)
            if not (runner.REPO_ROOT / root / "cabal.project").exists()
        ]
        self.assertEqual(
            by_id["haskell-workspace-project"]["applies_to"]["exact_roots"],
            expected_haskell,
        )
        self.assertEqual(
            by_id["lua-workspace-lint-configuration"]["applies_to"]["exact_roots"],
            build_roots(runner.REPO_ROOT / "code/packages/lua", "luacheck"),
        )
        python_workspace = (
            runner.REPO_ROOT / "code/packages/python/pyproject.toml"
        ).read_text(encoding="utf-8")
        member_block = python_workspace.split("[tool.uv.workspace]", 1)[1].split(
            "[tool.uv.sources]", 1
        )[0]
        project_aware_uv = re.compile(
            r"\buv\s+(?:add|build|export|lock|remove|run|sync|tree|venv)\b",
            re.IGNORECASE,
        )
        expected_python = []
        for member in re.findall(r'"([^"]+)"', member_block):
            root = runner.REPO_ROOT / "code/packages/python" / member
            lines = [
                line
                for build_file in root.glob("BUILD*")
                for line in build_file.read_text(encoding="utf-8").splitlines()
                if not line.lstrip().startswith("#")
            ]
            if any(
                project_aware_uv.search(line) and "--no-project" not in line.lower()
                for line in lines
            ):
                expected_python.append(root.relative_to(runner.REPO_ROOT).as_posix())
        expected_python.sort(key=lambda path: path.encode("utf-8"))
        self.assertEqual(
            by_id["python-uv-workspace-manifest"]["applies_to"]["exact_roots"],
            expected_python,
        )
        self.assertEqual(
            by_id["rust-windows-cargo-launcher"]["applies_to"]["exact_roots"],
            build_roots(runner.REPO_ROOT / "code/packages/rust", "_windows_cargo.sh"),
        )
        external_workspace_cargo_roots: set[str] = set()
        for search_root in (
            runner.REPO_ROOT / "code/packages",
            runner.REPO_ROOT / "code/programs",
        ):
            for build_file in search_root.rglob("BUILD*"):
                if not build_file.is_file() or build_file.name not in {
                    "BUILD",
                    "BUILD_windows",
                }:
                    continue
                root = build_file.parent.relative_to(runner.REPO_ROOT).as_posix()
                if root.startswith("code/packages/rust/"):
                    continue
                for line in build_file.read_text(encoding="utf-8").splitlines():
                    if line.lstrip().startswith("#") or not re.search(
                        r"\bcargo\b", line, re.IGNORECASE
                    ):
                        continue
                    if re.search(r"(?:^|[/\\])rust(?:[/\\]|$)", line, re.IGNORECASE):
                        external_workspace_cargo_roots.add(root)
                        break
        self.assertEqual(
            by_id["rust-root-workspace-manifest-cross-consumers"]["applies_to"][
                "exact_roots"
            ],
            sorted(
                external_workspace_cargo_roots, key=lambda path: path.encode("utf-8")
            ),
        )
        expected_typescript = []
        for root in build_roots(runner.REPO_ROOT / "code/packages/typescript"):
            tsconfig = runner.REPO_ROOT / root / "tsconfig.json"
            if tsconfig.is_file() and "../tsconfig.base.json" in tsconfig.read_text(
                encoding="utf-8"
            ):
                expected_typescript.append(root)
        self.assertEqual(
            by_id["typescript-workspace-configuration"]["applies_to"]["exact_roots"],
            expected_typescript,
        )
        for language in ("elixir", "go", "rust", "typescript"):
            boundary_id = f"starlark-{language}-library-rule-consumers"
            input_path = f"code/packages/starlark/library-rules/{language}_library.star"
            self.assertEqual(
                by_id[boundary_id]["applies_to"]["exact_roots"],
                build_roots(runner.REPO_ROOT / "code/packages", input_path),
            )
        self.assertEqual(
            by_id["starlark-ruby-library-rule-consumer"]["applies_to"]["exact_roots"],
            build_roots(
                runner.REPO_ROOT / "code/packages",
                "code/packages/starlark/library-rules/ruby_library.star",
            ),
        )
        grammar_build = (
            runner.REPO_ROOT / "code/packages/typescript/human-language-data/BUILD"
        ).read_text(encoding="utf-8")
        self.assertIn("generate_grammar_cells.py", grammar_build)
        grammar_checker = (
            runner.REPO_ROOT
            / "code/learning/human-languages/data/generate_grammar_cells.py"
        ).read_text(encoding="utf-8")
        self.assertIn("core/grammar-slots.json", grammar_checker)
        self.assertIn("spanish/grammar-cells.json", grammar_checker)
        for language in ("go", "ruby", "rust"):
            fixture_build = (
                runner.REPO_ROOT
                / f"code/programs/{language}/neural-fixture-consumer/BUILD"
            ).read_text(encoding="utf-8")
            self.assertIn("00-weighted-neuron.json", fixture_build)
        required_capabilities_build = (
            runner.REPO_ROOT / "code/packages/rust/required-capabilities-compiler/BUILD"
        ).read_text(encoding="utf-8")
        self.assertIn("required_capabilities.json", required_capabilities_build)
        exact_roots_without_build = set()
        for entry in boundary["boundaries"]:
            for exact_root in entry["applies_to"]["exact_roots"]:
                root = runner.REPO_ROOT / exact_root
                if (
                    not (root / "BUILD").is_file()
                    and not (root / "BUILD_windows").is_file()
                ):
                    exact_roots_without_build.add(exact_root)
            for excluded_root in entry["applies_to"]["excluded_roots"]:
                root = runner.REPO_ROOT / excluded_root
                self.assertTrue(
                    (root / "BUILD").is_file() or (root / "BUILD_windows").is_file(),
                    excluded_root,
                )
        self.assertEqual(
            exact_roots_without_build,
            set(),
        )

    def test_repository_source_input_boundary_rejects_drift_and_collisions(
        self,
    ) -> None:
        source_registry = runner.load_document(
            FIXTURE_ROOT / "language-source-input-registry.json"
        )
        schema = runner.load_document(
            FIXTURE_ROOT / "repository-source-input-boundary.schema.json"
        )
        canonical = runner.load_document(
            FIXTURE_ROOT / "repository-source-input-boundary.json"
        )

        def boundary_by_id(document: dict[str, object], boundary_id: str):
            return next(
                entry for entry in document["boundaries"] if entry["id"] == boundary_id
            )

        mutations: list[tuple[str, dict[str, object], str]] = []
        wrong_digest = copy.deepcopy(canonical)
        wrong_digest["language_source_input_registry_sha256"] = "0" * 64
        mutations.append(
            (
                "wrong-language-registry",
                wrong_digest,
                "REPOSITORY_SOURCE_REGISTRY_DIGEST_MISMATCH",
            )
        )
        unordered = copy.deepcopy(canonical)
        unordered["boundaries"][0], unordered["boundaries"][1] = (
            unordered["boundaries"][1],
            unordered["boundaries"][0],
        )
        mutations.append(
            ("unordered-boundaries", unordered, "REPOSITORY_SOURCE_NOT_CANONICAL")
        )
        duplicate_path = copy.deepcopy(canonical)
        boundary_by_id(duplicate_path, "rust-root-workspace-manifest")["inputs"].append(
            {"path": "code/packages/rust/cargo.toml", "role": "shared_ancestor"}
        )
        mutations.append(
            ("casefold-path-alias", duplicate_path, "REPOSITORY_SOURCE_INPUT_COLLISION")
        )
        generated_as_shared = copy.deepcopy(canonical)
        boundary_by_id(generated_as_shared, "rust-cargo-target-configuration")[
            "inputs"
        ][0] = {
            "path": "code/packages/rust/.cargo/config.toml",
            "role": "shared_ancestor",
        }
        mutations.append(
            (
                "generated-as-shared",
                generated_as_shared,
                "REPOSITORY_SOURCE_ROLE_INVALID",
            )
        )
        package_nested_shared = copy.deepcopy(canonical)
        boundary_by_id(package_nested_shared, "typescript-workspace-configuration")[
            "inputs"
        ][0]["path"] = "code/packages/typescript/hash-functions/private.config"
        mutations.append(
            (
                "package-nested-shared",
                package_nested_shared,
                "REPOSITORY_SOURCE_ROLE_INVALID",
            )
        )
        cross_inside_consumer = copy.deepcopy(canonical)
        boundary_by_id(
            cross_inside_consumer, "typescript-program-workspace-configuration"
        )["inputs"][0]["path"] = "code/programs/typescript/ircd/tsconfig.json"
        mutations.append(
            (
                "cross-inside-consumer",
                cross_inside_consumer,
                "REPOSITORY_SOURCE_ROLE_INVALID",
            )
        )
        unsorted_roots = copy.deepcopy(canonical)
        roots = boundary_by_id(unsorted_roots, "haskell-workspace-project")[
            "applies_to"
        ]["exact_roots"]
        roots[0], roots[1] = roots[1], roots[0]
        mutations.append(
            ("unsorted-roots", unsorted_roots, "REPOSITORY_SOURCE_NOT_CANONICAL")
        )
        casefold_root = copy.deepcopy(canonical)
        roots = boundary_by_id(casefold_root, "lua-workspace-lint-configuration")[
            "applies_to"
        ]["exact_roots"]
        roots.append("code/packages/lua/ZIP")
        mutations.append(
            (
                "casefold-root-alias",
                casefold_root,
                "REPOSITORY_SOURCE_NOT_CANONICAL",
            )
        )
        cross_boundary_root_alias = copy.deepcopy(canonical)
        boundary_by_id(
            cross_boundary_root_alias,
            "repository-human-language-grammar-cell-inputs",
        )["applies_to"]["exact_roots"] = [
            "code/packages/lua/ZIP",
            "code/packages/typescript/human-language-data",
        ]
        mutations.append(
            (
                "cross-boundary-root-casefold-alias",
                cross_boundary_root_alias,
                "REPOSITORY_SOURCE_SCOPE_COLLISION",
            )
        )
        unsorted_descendants = copy.deepcopy(canonical)
        boundary_by_id(unsorted_descendants, "rust-root-workspace-manifest")[
            "applies_to"
        ]["descendant_roots"] = ["code/packages/rust", "code/packages/haskell"]
        mutations.append(
            (
                "unsorted-descendant-roots",
                unsorted_descendants,
                "REPOSITORY_SOURCE_NOT_CANONICAL",
            )
        )
        unsorted_exclusions = copy.deepcopy(canonical)
        exclusions = boundary_by_id(
            unsorted_exclusions, "rust-root-workspace-manifest"
        )["applies_to"]["excluded_roots"]
        exclusions[0], exclusions[1] = exclusions[1], exclusions[0]
        mutations.append(
            (
                "unsorted-excluded-roots",
                unsorted_exclusions,
                "REPOSITORY_SOURCE_NOT_CANONICAL",
            )
        )
        excluded_outside = copy.deepcopy(canonical)
        boundary_by_id(excluded_outside, "rust-root-workspace-manifest")["applies_to"][
            "excluded_roots"
        ] = ["code/packages/typescript/os-kernel"]
        mutations.append(
            (
                "excluded-outside-descendant",
                excluded_outside,
                "REPOSITORY_SOURCE_SCOPE_INVALID",
            )
        )
        redundant_exact = copy.deepcopy(canonical)
        boundary_by_id(redundant_exact, "rust-root-workspace-manifest")["applies_to"][
            "exact_roots"
        ] = ["code/packages/rust/commonmark"]
        mutations.append(
            (
                "redundant-exact-root",
                redundant_exact,
                "REPOSITORY_SOURCE_SCOPE_COLLISION",
            )
        )
        cross_descendants = copy.deepcopy(canonical)
        applies_to = boundary_by_id(
            cross_descendants, "rust-root-workspace-manifest-cross-consumers"
        )["applies_to"]
        applies_to["exact_roots"] = []
        applies_to["descendant_roots"] = ["code/packages/go"]
        mutations.append(
            (
                "cross-package-descendants",
                cross_descendants,
                "REPOSITORY_SOURCE_ROLE_INVALID",
            )
        )
        wrong_input_origin = copy.deepcopy(canonical)
        boundary_by_id(wrong_input_origin, "rust-required-capabilities-input")[
            "input_origin"
        ] = "typescript"
        mutations.append(
            (
                "registered-input-origin-mismatch",
                wrong_input_origin,
                "REPOSITORY_SOURCE_ROLE_INVALID",
            )
        )
        repository_origin_in_lane = copy.deepcopy(canonical)
        boundary_by_id(repository_origin_in_lane, "rust-required-capabilities-input")[
            "input_origin"
        ] = "repository"
        mutations.append(
            (
                "repository-origin-inside-lane",
                repository_origin_in_lane,
                "REPOSITORY_SOURCE_ROLE_INVALID",
            )
        )
        repeated_generated_component = copy.deepcopy(canonical)
        boundary_by_id(repeated_generated_component, "rust-cargo-target-configuration")[
            "inputs"
        ][0]["path"] = "code/packages/rust/.cargo/nested/.cargo/config.toml"
        mutations.append(
            (
                "repeated-generated-component",
                repeated_generated_component,
                "REPOSITORY_SOURCE_ROLE_INVALID",
            )
        )
        case_variant_generated_component = copy.deepcopy(canonical)
        boundary_by_id(
            case_variant_generated_component, "rust-cargo-target-configuration"
        )["inputs"][0]["path"] = "code/packages/rust/.Cargo/config.toml"
        mutations.append(
            (
                "case-variant-generated-component",
                case_variant_generated_component,
                "REPOSITORY_SOURCE_ROLE_INVALID",
            )
        )
        unknown_consumer_lane = copy.deepcopy(canonical)
        boundary_by_id(
            unknown_consumer_lane, "typescript-program-workspace-configuration"
        )["applies_to"]["exact_roots"] = ["code/programs/brain/ircd"]
        mutations.append(
            (
                "unknown-consumer-lane",
                unknown_consumer_lane,
                "REPOSITORY_SOURCE_SCOPE_INVALID",
            )
        )
        overlapping_authority = copy.deepcopy(canonical)
        exact_roots = boundary_by_id(
            overlapping_authority, "rust-root-workspace-manifest-cross-consumers"
        )["applies_to"]["exact_roots"]
        exact_roots.append("code/packages/rust/commonmark")
        exact_roots.sort(key=lambda path: path.encode("utf-8"))
        mutations.append(
            (
                "overlapping-authority",
                overlapping_authority,
                "REPOSITORY_SOURCE_SCOPE_COLLISION",
            )
        )
        sensitive_path = copy.deepcopy(canonical)
        boundary_by_id(sensitive_path, "haskell-workspace-project")["inputs"][0][
            "path"
        ] = "code/packages/haskell/.env"
        mutations.append(
            (
                "sensitive-workspace-input",
                sensitive_path,
                "REPOSITORY_SOURCE_SENSITIVE_PATH",
            )
        )

        for name, boundary, expected_code in mutations:
            with (
                self.subTest(name=name),
                self.assertRaises(runner.ConformanceError) as raised,
            ):
                runner._validate_repository_source_input_boundary(
                    boundary,
                    schema,
                    source_registry,
                )
            self.assertEqual(raised.exception.code, expected_code)

        with (
            mock.patch.object(runner, "MAX_REPOSITORY_SOURCE_SCOPES", 480),
            self.assertRaises(runner.ConformanceError) as raised,
        ):
            runner._validate_repository_source_input_boundary(
                canonical,
                schema,
                source_registry,
            )
        self.assertEqual(raised.exception.code, "REPOSITORY_SOURCE_SCOPE_LIMIT")

        with (
            mock.patch.object(runner, "MAX_REPOSITORY_SOURCE_AUTHORIZATIONS", 483),
            self.assertRaises(runner.ConformanceError) as raised,
        ):
            runner._validate_repository_source_input_boundary(
                canonical,
                schema,
                source_registry,
            )
        self.assertEqual(
            raised.exception.code,
            "REPOSITORY_SOURCE_AUTHORIZATION_LIMIT",
        )

        for path in (
            "code/packages/typescript/.env.local",
            "code/packages/typescript/tsconfig.local.json",
            "code/packages/rust/.cargo/config.local.toml",
        ):
            with self.subTest(sensitive_path=path):
                self.assertTrue(runner._repository_source_sensitive_path(path))
        self.assertFalse(
            runner._repository_source_sensitive_path(
                "code/packages/rust/.cargo/config.toml"
            )
        )

    def test_repository_source_boundary_digest_frames_every_authoritative_field(
        self,
    ) -> None:
        canonical = runner.load_document(
            FIXTURE_ROOT / "repository-source-input-boundary.json"
        )
        canonical_digest = runner.repository_source_input_boundary_digest(canonical)
        reordered_keys = {
            key: canonical[key] for key in reversed(tuple(canonical.keys()))
        }
        self.assertEqual(
            runner.repository_source_input_boundary_digest(reordered_keys),
            canonical_digest,
        )

        mutations = []
        for name, mutate in (
            ("schema-version", lambda value: value.__setitem__("schema_version", 2)),
            (
                "registry-digest",
                lambda value: value.__setitem__(
                    "language_source_input_registry_sha256", "0" * 64
                ),
            ),
            (
                "boundary-id",
                lambda value: value["boundaries"][0].__setitem__(
                    "id", "haskell-workspace-project-v2"
                ),
            ),
            (
                "input-origin",
                lambda value: value["boundaries"][0].__setitem__(
                    "input_origin", "python"
                ),
            ),
            (
                "exact-root",
                lambda value: value["boundaries"][0]["applies_to"][
                    "exact_roots"
                ].__setitem__(0, "code/packages/haskell/arithmetic-v2"),
            ),
            (
                "input-path",
                lambda value: value["boundaries"][0]["inputs"][0].__setitem__(
                    "path", "code/packages/haskell/cabal-v2.project"
                ),
            ),
            (
                "input-role",
                lambda value: value["boundaries"][0]["inputs"][0].__setitem__(
                    "role", "cross_package_exact"
                ),
            ),
            (
                "generated-component",
                lambda value: next(
                    entry
                    for entry in value["boundaries"]
                    if entry["id"] == "rust-cargo-target-configuration"
                )["inputs"][0].__setitem__("generated_component", "vendor"),
            ),
            (
                "owner",
                lambda value: value["boundaries"][0].__setitem__("owner", "other"),
            ),
            (
                "reason",
                lambda value: value["boundaries"][0].__setitem__(
                    "reason", "Different reviewed reason."
                ),
            ),
        ):
            changed = copy.deepcopy(canonical)
            mutate(changed)
            mutations.append((name, changed))

        for name, changed in mutations:
            with self.subTest(name=name):
                self.assertNotEqual(
                    runner.repository_source_input_boundary_digest(changed),
                    canonical_digest,
                )

    def test_repository_source_boundary_maximum_input_shape_is_linear(self) -> None:
        source_registry = runner.load_document(
            FIXTURE_ROOT / "language-source-input-registry.json"
        )
        schema = runner.load_document(
            FIXTURE_ROOT / "repository-source-input-boundary.schema.json"
        )
        boundary = {
            "schema_version": 1,
            "language_source_input_registry_sha256": runner.source_input_registry_digest(
                source_registry
            ),
            "boundaries": [
                {
                    "id": f"bulk-{boundary_index:03d}",
                    "input_origin": "rust",
                    "applies_to": {
                        "exact_roots": [
                            f"code/packages/go/bulk-consumer-{boundary_index:03d}"
                        ],
                        "descendant_roots": [],
                        "excluded_roots": [],
                    },
                    "inputs": [
                        {
                            "path": (
                                "code/packages/rust/bulk/"
                                f"{boundary_index:03d}/input-{input_index:03d}.toml"
                            ),
                            "role": "cross_package_exact",
                        }
                        for input_index in range(64)
                    ],
                    "reason": "Maximum-shape linear prefix-collision regression.",
                    "owner": "build-tool-shared-and-generated-boundary-source-input-contract",
                }
                for boundary_index in range(256)
            ],
        }
        self.assertEqual(
            runner._validate_repository_source_input_boundary(
                boundary,
                schema,
                source_registry,
            ),
            {
                "boundary_count": 256,
                "input_count": 16384,
                "scope_count": 256,
                "authorization_count": 16384,
            },
        )

    def test_language_source_input_registry_covers_reviewed_repository_inputs(
        self,
    ) -> None:
        registry = runner.load_document(
            FIXTURE_ROOT / "language-source-input-registry.json"
        )
        registry_sha256 = runner.source_input_registry_digest(registry)
        samples = [
            ("c", "code/packages/c/aes/tools/run.sh", "tools/run.sh"),
            ("cpp", "code/packages/cpp/aes/tools/run.ps1", "tools/run.ps1"),
            (
                "csharp",
                "code/packages/csharp/sql-csv-source/tests/CodingAdventures.SqlCsvSource.Tests/fixtures/departments.csv",
                "tests/CodingAdventures.SqlCsvSource.Tests/fixtures/departments.csv",
            ),
            (
                "dart",
                "code/packages/dart/wasm-runtime/test/fixtures/square_nostd.wasm",
                "test/fixtures/square_nostd.wasm",
            ),
            (
                "elixir",
                "code/packages/elixir/commonmark_parser/test/fixtures/spec.json",
                "test/fixtures/spec.json",
            ),
            (
                "fsharp",
                "code/packages/fsharp/sql-csv-source/tests/CodingAdventures.SqlCsvSource.Tests/fixtures/departments.csv",
                "tests/CodingAdventures.SqlCsvSource.Tests/fixtures/departments.csv",
            ),
            (
                "go",
                "code/packages/go/sql-csv-source/testdata/departments.csv",
                "testdata/departments.csv",
            ),
            (
                "go",
                "code/programs/go/mosaicbook-server/static/index.html",
                "static/index.html",
            ),
            (
                "haskell",
                "code/programs/haskell/conduit-hello/tools/run-tests.sh",
                "tools/run-tests.sh",
            ),
            (
                "lua",
                "code/packages/lua/commonmark/tests/commonmark_spec.json",
                "tests/commonmark_spec.json",
            ),
            (
                "mosaic",
                "code/programs/mosaic/venture-browser/scripts/build-all.sh",
                "scripts/build-all.sh",
            ),
            (
                "perl",
                "code/packages/perl/sql-csv-source/t/fixtures/departments.csv",
                "t/fixtures/departments.csv",
            ),
            (
                "python",
                "code/programs/python/unix-tools/basename.json",
                "basename.json",
            ),
            (
                "ruby",
                "code/packages/ruby/commonmark_parser/test/spec.json",
                "test/spec.json",
            ),
            (
                "ruby",
                "code/programs/ruby/build-tool/test/fixtures/simple/code/packages/ruby/pkg-a/src/main.py",
                "test/fixtures/simple/code/packages/ruby/pkg-a/src/main.py",
            ),
            (
                "ruby",
                "code/programs/ruby/build-tool/test/fixtures/simple/code/packages/ruby/pkg-a/pyproject.toml",
                "test/fixtures/simple/code/packages/ruby/pkg-a/pyproject.toml",
            ),
            (
                "rust",
                "code/packages/rust/wasm-conformance/tests/fixtures/testsuite/address.wast",
                "tests/fixtures/testsuite/address.wast",
            ),
            (
                "rust",
                "code/packages/rust/chief-of-staff-agent-stdio-host/tests/fixtures/echo_agent.py",
                "tests/fixtures/echo_agent.py",
            ),
            (
                "rust",
                "code/packages/rust/engram-wasm/js/engram-mosaic-host-wasm.mjs",
                "js/engram-mosaic-host-wasm.mjs",
            ),
            (
                "rust",
                "code/packages/rust/engram-wasm/js/smoke.mjs",
                "js/smoke.mjs",
            ),
            (
                "rust",
                "code/packages/rust/engram-wasm/pkg/engram_engine.wasm",
                "pkg/engram_engine.wasm",
            ),
            (
                "swift",
                "code/packages/swift/grammar-tools/regen-embedded-grammars.sh",
                "regen-embedded-grammars.sh",
            ),
            (
                "typescript",
                "code/packages/typescript/conduit/tsconfig.test.json",
                "tsconfig.test.json",
            ),
            (
                "typescript",
                "code/packages/typescript/grammar-tools/program/grammar-tools.cli.json",
                "program/grammar-tools.cli.json",
            ),
            (
                "typescript",
                "code/programs/typescript/checklist-app/electron/tsconfig.json",
                "electron/tsconfig.json",
            ),
        ]
        expected_digest = hashlib.sha256(b"reviewed-input").hexdigest()
        for language, repository_path, package_path in samples:
            with self.subTest(language=language, path=repository_path):
                self.assertTrue((runner.REPO_ROOT / repository_path).is_file())
                package_root = repository_path[: -(len(package_path) + 1)]
                actual = runner._expected_source_collection(
                    {
                        "language": language,
                        "package_root": package_root,
                        "mode": "extension",
                        "registry_sha256": registry_sha256,
                        "declared_srcs": [],
                        "candidates": [
                            {
                                "path": package_path,
                                "kind": "file",
                                "content_hex": b"reviewed-input".hex(),
                            }
                        ],
                    },
                    registry,
                )
                self.assertEqual(
                    actual,
                    [{"path": package_path, "digest": expected_digest}],
                )

    def test_engram_wasm_registry_projects_exact_tracked_bytes(self) -> None:
        registry = runner.load_document(
            FIXTURE_ROOT / "language-source-input-registry.json"
        )
        registry_sha256 = runner.source_input_registry_digest(registry)
        package_root = "code/packages/rust/engram-wasm"
        package_paths = [
            "js/engram-mosaic-host-wasm.mjs",
            "js/smoke.mjs",
            "pkg/engram_engine.wasm",
        ]

        def tracked_blob(repository_path: str) -> bytes:
            stage_record = subprocess.check_output(
                ["git", "ls-files", "--stage", "-z", "--", repository_path],
                cwd=runner.REPO_ROOT,
            )
            entries = [entry for entry in stage_record.split(b"\0") if entry]
            self.assertEqual(len(entries), 1)
            metadata, staged_path = entries[0].split(b"\t", 1)
            mode, object_id, stage = metadata.decode("ascii").split()
            self.assertEqual(mode, "100644")
            self.assertEqual(stage, "0")
            self.assertEqual(staged_path.decode("utf-8"), repository_path)
            return subprocess.check_output(
                ["git", "cat-file", "blob", object_id],
                cwd=runner.REPO_ROOT,
            )

        candidates = []
        expected = []
        tracked_bodies: dict[str, bytes] = {}
        for package_path in package_paths:
            repository_path = f"{package_root}/{package_path}"
            body = tracked_blob(repository_path)
            tracked_bodies[package_path] = body
            candidates.append(
                {"path": package_path, "kind": "file", "content_hex": body.hex()}
            )
            expected.append(
                {"path": package_path, "digest": hashlib.sha256(body).hexdigest()}
            )

        self.assertEqual(
            runner._expected_source_collection(
                {
                    "language": "rust",
                    "package_root": package_root,
                    "mode": "extension",
                    "registry_sha256": registry_sha256,
                    "declared_srcs": [],
                    "candidates": candidates,
                },
                registry,
            ),
            expected,
        )
        build_text = tracked_blob(f"{package_root}/BUILD").decode("utf-8")
        smoke_text = tracked_bodies["js/smoke.mjs"].decode("utf-8")
        self.assertIn("node js/smoke.mjs", build_text.splitlines())
        self.assertIn(
            'from "./engram-mosaic-host-wasm.mjs"',
            smoke_text,
        )
        self.assertIn(
            'readFileSync(join(here, "..", "pkg", "engram_engine.wasm"))',
            smoke_text,
        )

    def test_language_source_input_registry_rejects_drift_and_collisions(self) -> None:
        schema = runner.load_document(
            FIXTURE_ROOT / "language-source-input-registry.schema.json"
        )
        canonical = runner.load_document(
            FIXTURE_ROOT / "language-source-input-registry.json"
        )

        mutations: list[tuple[str, object, str]] = []

        missing = copy.deepcopy(canonical)
        missing["languages"].pop()
        mutations.append(
            ("missing-language", missing, "SOURCE_INPUT_REGISTRY_SCHEMA_INVALID")
        )

        duplicate = copy.deepcopy(canonical)
        duplicate["languages"].insert(1, copy.deepcopy(duplicate["languages"][0]))
        mutations.append(
            ("duplicate-language", duplicate, "SOURCE_INPUT_LANGUAGE_DUPLICATE")
        )

        unsorted = copy.deepcopy(canonical)
        unsorted["languages"][0], unsorted["languages"][1] = (
            unsorted["languages"][1],
            unsorted["languages"][0],
        )
        mutations.append(("unsorted-language", unsorted, "SOURCE_INPUT_NOT_CANONICAL"))

        collision = copy.deepcopy(canonical)
        collision["languages"][0]["root_exact_basenames"].append(
            collision["languages"][0]["recursive_exact_basenames"][0]
        )
        collision["languages"][0]["root_exact_basenames"].sort(
            key=lambda value: value.encode("utf-8")
        )
        mutations.append(("cross-role", collision, "SOURCE_INPUT_SELECTOR_COLLISION"))

        undeclared_alias = copy.deepcopy(canonical)
        undeclared_alias["languages"][2]["root_exact_basenames"].append("NUGET.CONFIG")
        undeclared_alias["languages"][2]["root_exact_basenames"].sort(
            key=lambda value: value.encode("utf-8")
        )
        mutations.append(
            (
                "undeclared-case-alias",
                undeclared_alias,
                "SOURCE_INPUT_SELECTOR_COLLISION",
            )
        )

        unsafe = copy.deepcopy(canonical)
        unsafe["languages"][0]["root_exact_relative_paths"].append("../escape")
        mutations.append(
            ("unsafe-path", unsafe, "SOURCE_INPUT_REGISTRY_SCHEMA_INVALID")
        )

        bidi = copy.deepcopy(canonical)
        bidi["languages"][0]["root_exact_basenames"].append("safe\u202efile")
        mutations.append(("bidi-control", bidi, "SOURCE_INPUT_PATH_UNSAFE"))

        generated_scope = copy.deepcopy(canonical)
        generated_scope["languages"][3]["scoped_inputs"][0]["path_prefix"] = "build"
        mutations.append(
            ("generated-scope", generated_scope, "SOURCE_INPUT_PATH_UNSAFE")
        )

        package_language = copy.deepcopy(canonical)
        rust_entry = next(
            entry for entry in package_language["languages"]
            if entry["language"] == "rust"
        )
        rust_entry["package_exact_inputs"][0]["package_root"] = (
            "code/packages/typescript/engram-wasm"
        )
        mutations.append(
            (
                "package-root-language-mismatch",
                package_language,
                "SOURCE_INPUT_PACKAGE_ROOT_LANGUAGE_MISMATCH",
            )
        )

        duplicate_package_id = copy.deepcopy(canonical)
        rust_entry = next(
            entry for entry in duplicate_package_id["languages"]
            if entry["language"] == "rust"
        )
        rust_entry["package_exact_inputs"].append(
            copy.deepcopy(rust_entry["package_exact_inputs"][0])
        )
        mutations.append(
            (
                "duplicate-package-id",
                duplicate_package_id,
                "SOURCE_INPUT_NOT_CANONICAL",
            )
        )

        unsafe_package_root = copy.deepcopy(canonical)
        rust_entry = next(
            entry for entry in unsafe_package_root["languages"]
            if entry["language"] == "rust"
        )
        rust_entry["package_exact_inputs"][0]["package_root"] = (
            "code/packages/rust/engram-wasm."
        )
        mutations.append(
            (
                "unsafe-package-root",
                unsafe_package_root,
                "SOURCE_INPUT_PATH_UNSAFE",
            )
        )

        duplicate_package_root = copy.deepcopy(canonical)
        rust_entry = next(
            entry for entry in duplicate_package_root["languages"]
            if entry["language"] == "rust"
        )
        second_rule = copy.deepcopy(rust_entry["package_exact_inputs"][0])
        second_rule["id"] = "rust-engram-wasm-second-inputs"
        rust_entry["package_exact_inputs"].append(second_rule)
        rust_entry["package_exact_inputs"].sort(
            key=lambda item: item["id"].encode("utf-8")
        )
        mutations.append(
            (
                "duplicate-package-root",
                duplicate_package_root,
                "SOURCE_INPUT_SELECTOR_COLLISION",
            )
        )

        unsorted_package_paths = copy.deepcopy(canonical)
        rust_entry = next(
            entry for entry in unsorted_package_paths["languages"]
            if entry["language"] == "rust"
        )
        rust_entry["package_exact_inputs"][0]["paths"].reverse()
        mutations.append(
            (
                "unsorted-package-paths",
                unsorted_package_paths,
                "SOURCE_INPUT_NOT_CANONICAL",
            )
        )

        unsafe_package_path = copy.deepcopy(canonical)
        rust_entry = next(
            entry for entry in unsafe_package_path["languages"]
            if entry["language"] == "rust"
        )
        rust_entry["package_exact_inputs"][0]["paths"].append("js/smoke.mjs.")
        rust_entry["package_exact_inputs"][0]["paths"].sort(
            key=lambda value: value.encode("utf-8")
        )
        mutations.append(
            (
                "unsafe-package-path",
                unsafe_package_path,
                "SOURCE_INPUT_PATH_UNSAFE",
            )
        )

        generated_package_path = copy.deepcopy(canonical)
        rust_entry = next(
            entry for entry in generated_package_path["languages"]
            if entry["language"] == "rust"
        )
        rust_entry["package_exact_inputs"][0]["paths"].append(
            ".build/generated.wasm"
        )
        rust_entry["package_exact_inputs"][0]["paths"].sort(
            key=lambda value: value.encode("utf-8")
        )
        mutations.append(
            (
                "generated-package-path",
                generated_package_path,
                "SOURCE_INPUT_PATH_UNSAFE",
            )
        )

        package_prefix_collision = copy.deepcopy(canonical)
        rust_entry = next(
            entry for entry in package_prefix_collision["languages"]
            if entry["language"] == "rust"
        )
        rust_entry["package_exact_inputs"][0]["paths"] = ["a", "a/b"]
        mutations.append(
            (
                "package-prefix-collision",
                package_prefix_collision,
                "SOURCE_INPUT_SELECTOR_COLLISION",
            )
        )

        package_global_prefix_collision = copy.deepcopy(canonical)
        rust_entry = next(
            entry for entry in package_global_prefix_collision["languages"]
            if entry["language"] == "rust"
        )
        rust_entry["root_exact_relative_paths"].append("js")
        rust_entry["root_exact_relative_paths"].sort(
            key=lambda value: value.encode("utf-8")
        )
        mutations.append(
            (
                "package-global-prefix-collision",
                package_global_prefix_collision,
                "SOURCE_INPUT_SELECTOR_COLLISION",
            )
        )

        for index, sensitive_package_path in enumerate(
            (
                ".env",
                "credentials.json",
                "local.properties",
                "secrets/data.json",
                "signing.key",
                "token.txt",
            )
        ):
            sensitive_package_input = copy.deepcopy(canonical)
            rust_entry = next(
                entry for entry in sensitive_package_input["languages"]
                if entry["language"] == "rust"
            )
            rust_entry["package_exact_inputs"][0]["paths"] = [
                sensitive_package_path
            ]
            mutations.append(
                (
                    f"sensitive-package-path-{index}",
                    sensitive_package_input,
                    "SOURCE_INPUT_SENSITIVE_PATH",
                )
            )

        for index, sensitive_root_component in enumerate(
            (".env", "credentials", "local", "secrets", "signing", "token")
        ):
            sensitive_package_root = copy.deepcopy(canonical)
            rust_entry = next(
                entry for entry in sensitive_package_root["languages"]
                if entry["language"] == "rust"
            )
            rust_entry["package_exact_inputs"][0]["package_root"] = (
                f"code/packages/rust/{sensitive_root_component}"
            )
            rust_entry["package_exact_inputs"][0]["paths"] = ["README.md"]
            mutations.append(
                (
                    f"sensitive-package-root-{index}",
                    sensitive_package_root,
                    "SOURCE_INPUT_SENSITIVE_PATH",
                )
            )

        package_global_collision = copy.deepcopy(canonical)
        rust_entry = next(
            entry for entry in package_global_collision["languages"]
            if entry["language"] == "rust"
        )
        rust_entry["root_exact_relative_paths"].append("js/smoke.mjs")
        rust_entry["root_exact_relative_paths"].sort(
            key=lambda value: value.encode("utf-8")
        )
        mutations.append(
            (
                "package-global-collision",
                package_global_collision,
                "SOURCE_INPUT_SELECTOR_COLLISION",
            )
        )

        package_global_casefold_collision = copy.deepcopy(canonical)
        rust_entry = next(
            entry for entry in package_global_casefold_collision["languages"]
            if entry["language"] == "rust"
        )
        rust_entry["root_exact_relative_paths"].append("js/Smoke.mjs")
        rust_entry["root_exact_relative_paths"].sort(
            key=lambda value: value.encode("utf-8")
        )
        mutations.append(
            (
                "package-global-casefold-collision",
                package_global_casefold_collision,
                "SOURCE_INPUT_SELECTOR_COLLISION",
            )
        )

        package_casefold_collision = copy.deepcopy(canonical)
        rust_entry = next(
            entry for entry in package_casefold_collision["languages"]
            if entry["language"] == "rust"
        )
        rust_entry["package_exact_inputs"][0]["paths"].append("js/Smoke.mjs")
        rust_entry["package_exact_inputs"][0]["paths"].sort(
            key=lambda value: value.encode("utf-8")
        )
        mutations.append(
            (
                "package-casefold-collision",
                package_casefold_collision,
                "SOURCE_INPUT_SELECTOR_COLLISION",
            )
        )

        scoped_collision = copy.deepcopy(canonical)
        overlapping = copy.deepcopy(
            scoped_collision["languages"][3]["scoped_inputs"][0]
        )
        overlapping["id"] = "dart-flutter-android-overlap"
        overlapping["suffixes"] = [".kt"]
        scoped_collision["languages"][3]["scoped_inputs"].append(overlapping)
        scoped_collision["languages"][3]["scoped_inputs"].sort(
            key=lambda item: item["id"].encode("utf-8")
        )
        mutations.append(
            (
                "scoped-collision",
                scoped_collision,
                "SOURCE_INPUT_SELECTOR_COLLISION",
            )
        )

        scoped_global_suffix = copy.deepcopy(canonical)
        scoped_global_suffix["languages"][0]["scoped_inputs"].append(
            {
                "id": "c-global-suffix-overlap",
                "role": "native_companion",
                "decision": "include",
                "scope": "subtree",
                "path_prefix": "native",
                "suffixes": [".c"],
                "exact_basenames": [],
                "reason": "negative collision fixture",
                "owner": "build-tool-language-source-input-registry-corpus-and-engine-audit",
            }
        )
        scoped_global_suffix["languages"][0]["scoped_inputs"].sort(
            key=lambda item: item["id"].encode("utf-8")
        )
        mutations.append(
            (
                "scoped-global-suffix",
                scoped_global_suffix,
                "SOURCE_INPUT_SELECTOR_COLLISION",
            )
        )

        scoped_global_basename = copy.deepcopy(canonical)
        scoped_global_basename["languages"][0]["scoped_inputs"].append(
            {
                "id": "c-global-basename-overlap",
                "role": "native_companion",
                "decision": "include",
                "scope": "subtree",
                "path_prefix": "native",
                "suffixes": [],
                "exact_basenames": ["CMakeLists.txt"],
                "reason": "negative collision fixture",
                "owner": "build-tool-language-source-input-registry-corpus-and-engine-audit",
            }
        )
        scoped_global_basename["languages"][0]["scoped_inputs"].sort(
            key=lambda item: item["id"].encode("utf-8")
        )
        mutations.append(
            (
                "scoped-global-basename",
                scoped_global_basename,
                "SOURCE_INPUT_SELECTOR_COLLISION",
            )
        )

        exact_path_suffix = copy.deepcopy(canonical)
        exact_path_suffix["languages"][3]["root_exact_relative_paths"].append(
            "main.dart"
        )
        exact_path_suffix["languages"][3]["root_exact_relative_paths"].sort(
            key=lambda value: value.encode("utf-8")
        )
        mutations.append(
            (
                "exact-path-suffix-overlap",
                exact_path_suffix,
                "SOURCE_INPUT_SELECTOR_COLLISION",
            )
        )

        recursive_basename_suffix = copy.deepcopy(canonical)
        recursive_basename_suffix["languages"][3]["recursive_exact_basenames"] = [
            "main.dart"
        ]
        mutations.append(
            (
                "recursive-basename-suffix-overlap",
                recursive_basename_suffix,
                "SOURCE_INPUT_SELECTOR_COLLISION",
            )
        )

        selector_budget = copy.deepcopy(canonical)
        selector_budget["languages"][0]["scoped_inputs"] = [
            {
                "id": f"c-budget-{group:02d}",
                "role": "native_companion",
                "decision": "include",
                "scope": "subtree",
                "path_prefix": f"budget-{group:02d}",
                "suffixes": [
                    f".budget-{group:02d}-{index:03d}" for index in range(256)
                ],
                "exact_basenames": [],
                "reason": "negative aggregate selector-budget fixture",
                "owner": "build-tool-language-source-input-registry-corpus-and-engine-audit",
            }
            for group in range(17)
        ]
        mutations.append(
            (
                "aggregate-selector-budget",
                selector_budget,
                "SOURCE_INPUT_SELECTOR_LIMIT",
            )
        )

        excluded_scope = copy.deepcopy(canonical)
        excluded_scope["languages"][3]["scoped_inputs"][0]["decision"] = "exclude"
        mutations.append(
            (
                "excluded-scope",
                excluded_scope,
                "SOURCE_INPUT_REGISTRY_SCHEMA_INVALID",
            )
        )

        malformed_selector_type = copy.deepcopy(canonical)
        malformed_selector_type["languages"][0]["recursive_suffixes"] = 7
        mutations.append(
            (
                "malformed-selector-type",
                malformed_selector_type,
                "SOURCE_INPUT_REGISTRY_SCHEMA_INVALID",
            )
        )

        missing_owner = copy.deepcopy(canonical)
        missing_owner["languages"][3]["scoped_inputs"][0]["owner"] = ""
        mutations.append(
            (
                "missing-owner",
                missing_owner,
                "SOURCE_INPUT_REGISTRY_SCHEMA_INVALID",
            )
        )

        for name, registry, expected_code in mutations:
            with (
                self.subTest(name=name),
                self.assertRaises(runner.ConformanceError) as raised,
            ):
                runner._validate_source_input_registry(registry, schema)
            self.assertEqual(raised.exception.code, expected_code)

        allowed_near_names = copy.deepcopy(canonical)
        rust_entry = next(
            entry for entry in allowed_near_names["languages"]
            if entry["language"] == "rust"
        )
        rust_entry["package_exact_inputs"][0]["paths"] = sorted(
            [
                ".env-example",
                "credentials-guide.json",
                "localization.properties",
                "secretary-notes.txt",
                "signature-guide.md",
                "tokenizer.json",
            ],
            key=lambda value: value.encode("utf-8"),
        )
        self.assertEqual(
            runner._validate_source_input_registry(allowed_near_names, schema)[
                "language_count"
            ],
            23,
        )
        for allowed_root_component in (
            "credential-custody",
            "environment",
            "localization",
            "signature",
            "tokenizer",
        ):
            with self.subTest(allowed_root_component=allowed_root_component):
                allowed_root = copy.deepcopy(canonical)
                rust_entry = next(
                    entry for entry in allowed_root["languages"]
                    if entry["language"] == "rust"
                )
                rust_entry["package_exact_inputs"][0]["package_root"] = (
                    f"code/packages/rust/{allowed_root_component}"
                )
                rust_entry["package_exact_inputs"][0]["paths"] = ["README.md"]
                self.assertEqual(
                    runner._validate_source_input_registry(allowed_root, schema)[
                        "language_count"
                    ],
                    23,
                )

    def test_expected_results_are_checked_in_canonical_order(self) -> None:
        for case_path in sorted(CASES_ROOT.glob("*.json")):
            case = runner.load_document(case_path)
            self.assertEqual(
                case["expected"],
                runner.canonicalize_result(case["expected"]),
                case_path.name,
            )

    def test_every_process_free_domain_is_bootstrap_modeled(self) -> None:
        self.assertEqual(
            runner.BOOTSTRAP_DOMAINS,
            set(runner.DOMAIN_CAPABILITIES) - {"execution"},
        )

    def test_ci_gate_selection_oracle_closes_fail_open_and_negative_cases(self) -> None:
        expectations = {
            "ci-gate-selection-force.json": [True, True],
            "ci-gate-selection-null-affected.json": [True, True],
            "ci-gate-selection-null-changed-files.json": [True, True],
            "ci-gate-selection-machinery.json": [True, True],
            "ci-gate-selection-package-and-path.json": [True, True],
            "ci-gate-selection-recursive-glob.json": [False, True],
            "ci-gate-selection-unrelated.json": [False, False],
        }
        for filename, required in expectations.items():
            with self.subTest(filename=filename):
                case = load_case(filename)
                self.assertEqual(
                    [gate["required"] for gate in case["expected"]["result"]["gates"]],
                    required,
                )
                self.assertEqual(
                    runner.assert_result_matches(case, copy.deepcopy(case["expected"])),
                    case["expected"],
                )

    def test_ci_gate_selection_rejects_duplicate_ids_and_output_names(self) -> None:
        schema_args = {
            "case_schema": runner.load_document(FIXTURE_ROOT / "schema.json"),
            "result_schema": runner.load_document(FIXTURE_ROOT / "result.schema.json"),
            "plan_schema": runner.load_document(
                runner.REPO_ROOT / "code/specs/schemas/build-plan-v1.schema.json"
            ),
        }
        mutations = (
            ("alpha-job", "CASE_CI_GATE_DUPLICATE"),
            ("alpha_job", "CASE_CI_GATE_OUTPUT_COLLISION"),
        )
        for gate_id, code in mutations:
            with self.subTest(code=code):
                case = load_case("ci-gate-selection-unrelated.json")
                case["input"]["options"]["registry"]["gates"][1]["id"] = gate_id
                with self.assertRaises(runner.ConformanceError) as raised:
                    runner.validate_case_document(case, **schema_args)
                self.assertEqual(raised.exception.code, code)

    def test_malformed_capabilities_fail_schema_validation_not_routing(self) -> None:
        case = load_case("discovery-simple.json")
        case["capabilities"] = [{"execution": False}]
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.validate_case_document(
                case,
                case_schema=runner.load_document(FIXTURE_ROOT / "schema.json"),
                result_schema=runner.load_document(FIXTURE_ROOT / "result.schema.json"),
                plan_schema=runner.load_document(
                    runner.REPO_ROOT / "code/specs/schemas/build-plan-v1.schema.json"
                ),
            )
        self.assertEqual(raised.exception.code, "CASE_SCHEMA_INVALID")

    def test_replace_existing_requires_write_capability(self) -> None:
        case = load_case("plan-replace-existing.json")
        case["capabilities"] = ["plan_v1_read"]
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.validate_case_document(
                case,
                case_schema=runner.load_document(FIXTURE_ROOT / "schema.json"),
                result_schema=runner.load_document(FIXTURE_ROOT / "result.schema.json"),
                plan_schema=runner.load_document(
                    runner.REPO_ROOT / "code/specs/schemas/build-plan-v1.schema.json"
                ),
            )
        self.assertEqual(raised.exception.code, "CASE_CAPABILITY_MISSING")

    def test_replace_existing_validates_both_input_plans(self) -> None:
        schema_args = {
            "case_schema": runner.load_document(FIXTURE_ROOT / "schema.json"),
            "result_schema": runner.load_document(FIXTURE_ROOT / "result.schema.json"),
            "plan_schema": runner.load_document(
                runner.REPO_ROOT / "code/specs/schemas/build-plan-v1.schema.json"
            ),
        }
        for key, code in (
            ("existing_plan", "CASE_EXISTING_PLAN_SCHEMA_INVALID"),
            ("plan", "CASE_PLAN_SCHEMA_INVALID"),
        ):
            with self.subTest(key=key):
                case = load_case("plan-replace-existing.json")
                case["input"]["options"][key] = {}
                with self.assertRaises(runner.ConformanceError) as raised:
                    runner.validate_case_document(case, **schema_args)
                self.assertEqual(raised.exception.code, code)


class ExecutionDenialTests(unittest.TestCase):
    def assert_denied_before_side_effects(self, case: dict[str, object]) -> None:
        with (
            mock.patch.object(tempfile, "TemporaryDirectory") as temporary,
            mock.patch.object(runner.base64, "b64decode") as decode,
            mock.patch.object(os, "chmod") as chmod,
            mock.patch.object(subprocess, "run") as process,
            self.assertRaises(runner.ConformanceError) as raised,
        ):
            runner.preflight_workspace(case)

        self.assertEqual(raised.exception.code, "EXECUTION_DISABLED")
        temporary.assert_not_called()
        decode.assert_not_called()
        chmod.assert_not_called()
        process.assert_not_called()

    def test_execution_intent_is_denied_in_every_routing_field(self) -> None:
        base = load_case("discovery-windows-override.json")

        domain = copy.deepcopy(base)
        domain["domain"] = "execution"
        self.assert_denied_before_side_effects(domain)

        operation = copy.deepcopy(base)
        operation["input"]["operation"] = "execution"
        self.assert_denied_before_side_effects(operation)

        execution_capability = copy.deepcopy(base)
        execution_capability["capabilities"].append("execution")
        self.assert_denied_before_side_effects(execution_capability)

        trusted_capability = copy.deepcopy(base)
        trusted_capability["capabilities"].append("trusted_execution")
        self.assert_denied_before_side_effects(trusted_capability)

    def test_validate_result_rejects_execution_before_reading_the_result(
        self,
    ) -> None:
        case = load_case("discovery-windows-override.json")
        case["domain"] = "execution"
        with tempfile.TemporaryDirectory() as directory:
            case_path = Path(directory) / "case.json"
            case_path.write_text(json.dumps(case), encoding="utf-8")
            missing_result = Path(directory) / "missing-result.json"
            with self.assertRaises(runner.ConformanceError) as raised:
                runner.validate_result_files(case_path, missing_result)
        self.assertEqual(raised.exception.code, "EXECUTION_DISABLED")

    def test_cli_execution_intent_is_denied_before_workspace_decoding(
        self,
    ) -> None:
        case = load_case("cli-dry-run-success.json")
        case["input"]["options"]["requires_execution"] = True
        case["workspace"]["files"] = [
            {
                "path": "fixtures/invalid.bin",
                "content_base64": "not base64!",
            }
        ]
        with (
            mock.patch.object(tempfile, "TemporaryDirectory") as temporary,
            mock.patch.object(runner.base64, "b64decode") as decode,
            mock.patch.object(subprocess, "run") as process,
            self.assertRaises(runner.ConformanceError) as raised,
        ):
            runner.preflight_workspace(case)
        self.assertEqual(raised.exception.code, "EXECUTION_DISABLED")
        temporary.assert_not_called()
        decode.assert_not_called()
        process.assert_not_called()


class WorkspacePreflightTests(unittest.TestCase):
    def test_decodes_exact_files_without_creating_a_workspace(self) -> None:
        case = load_case("discovery-simple.json")
        case["workspace"]["files"].append(
            {
                "path": "fixtures/space and & metacharacters.bin",
                "content_base64": "AAEC/w==",
            }
        )

        with mock.patch.object(
            tempfile,
            "TemporaryDirectory",
        ) as temporary:
            staged = {
                entry.path: entry.content for entry in runner.preflight_workspace(case)
            }

        temporary.assert_not_called()
        self.assertEqual(
            staged["code/packages/python/demo/BUILD"],
            b"python -m unittest discover tests\n",
        )
        self.assertEqual(
            staged["fixtures/space and & metacharacters.bin"],
            b"\x00\x01\x02\xff",
        )

    def test_invalid_base64_and_workspace_limit_fail_before_root_creation(self) -> None:
        invalid = load_case("discovery-simple.json")
        invalid["workspace"]["files"][0] = {
            "path": "code/packages/python/demo/BUILD",
            "content_base64": "AB==",
        }
        with (
            mock.patch.object(tempfile, "TemporaryDirectory") as temporary,
            self.assertRaises(runner.ConformanceError) as raised,
        ):
            runner.preflight_workspace(invalid)
        self.assertEqual(raised.exception.code, "WORKSPACE_BASE64_NONCANONICAL")
        temporary.assert_not_called()

        oversized = load_case("discovery-simple.json")
        oversized["limits"]["workspace_bytes"] = 1
        with (
            mock.patch.object(tempfile, "TemporaryDirectory") as temporary,
            self.assertRaises(runner.ConformanceError) as raised,
        ):
            runner.preflight_workspace(oversized)
        self.assertEqual(raised.exception.code, "WORKSPACE_BYTE_LIMIT")
        temporary.assert_not_called()

        malformed = load_case("discovery-simple.json")
        malformed["workspace"]["files"][0] = {
            "path": "code/packages/python/demo/BUILD",
            "content_base64": "not base64!",
        }
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.preflight_workspace(malformed)
        self.assertEqual(raised.exception.code, "WORKSPACE_BASE64_INVALID")

    def test_malformed_workspace_shapes_are_rejected(self) -> None:
        base = load_case("discovery-simple.json")
        for workspace, code in (
            ({}, "WORKSPACE_FILES_INVALID"),
            ({"files": ["not-an-object"]}, "WORKSPACE_FILE_INVALID"),
            (
                {"files": [{"path": "fixtures/no-content"}]},
                "WORKSPACE_CONTENT_MISSING",
            ),
        ):
            case = copy.deepcopy(base)
            case["workspace"] = workspace
            with self.subTest(code=code):
                with self.assertRaises(runner.ConformanceError) as raised:
                    runner.preflight_workspace(case)
                self.assertEqual(raised.exception.code, code)

    def test_path_aliases_collisions_and_prefix_conflicts_fail_preflight(self) -> None:
        base = load_case("discovery-simple.json")
        unsafe_paths = (
            "/absolute",
            "C:/drive",
            "//server/share",
            "../escape",
            "fixtures/CONIN$.txt",
            "fixtures/CONOUT$.txt",
            "fixtures/CLOCK$.txt",
        )
        for unsafe_path in unsafe_paths:
            case = copy.deepcopy(base)
            case["workspace"]["files"][0]["path"] = unsafe_path
            with self.subTest(path=unsafe_path):
                with (
                    mock.patch.object(
                        tempfile,
                        "TemporaryDirectory",
                    ) as temporary,
                    self.assertRaises(runner.ConformanceError) as raised,
                ):
                    runner.preflight_workspace(case)
                self.assertEqual(raised.exception.code, "WORKSPACE_PATH_UNSAFE")
                temporary.assert_not_called()

        collision = copy.deepcopy(base)
        collision["workspace"]["files"].append(
            {
                "path": "CODE/PACKAGES/PYTHON/DEMO/build",
                "content_utf8": "collision\n",
            }
        )
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.preflight_workspace(collision)
        self.assertEqual(raised.exception.code, "WORKSPACE_PATH_COLLISION")

        prefix = copy.deepcopy(base)
        prefix["workspace"]["files"] = [
            {"path": "fixtures/data", "content_utf8": "file\n"},
            {"path": "fixtures/data/child", "content_utf8": "child\n"},
        ]
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.preflight_workspace(prefix)
        self.assertEqual(raised.exception.code, "WORKSPACE_PATH_PREFIX_CONFLICT")


class ResultValidationTests(unittest.TestCase):
    def test_domain_result_schema_rejects_field_name_drift(self) -> None:
        discovery = load_case("discovery-simple.json")
        typo = copy.deepcopy(discovery["expected"])
        package = typo["result"]["packages"][0]
        package["buildfile"] = package.pop("build_file")
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.assert_result_matches(discovery, typo)
        self.assertEqual(raised.exception.code, "RESULT_SCHEMA_INVALID")

        toolchains = load_case("toolchain-detection-shared.json")
        incomplete = copy.deepcopy(toolchains["expected"])
        del incomplete["result"]["toolchains"]["ocaml"]
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.assert_result_matches(toolchains, incomplete)
        self.assertEqual(
            raised.exception.code,
            "RESULT_PURE_SCHEMA_INVALID",
        )

        graph = load_case("graph-diamond.json")
        extra = copy.deepcopy(graph["expected"])
        extra["result"]["unexpected"] = []
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.assert_result_matches(graph, extra)
        self.assertEqual(raised.exception.code, "RESULT_SCHEMA_INVALID")

    def test_domain_aware_canonicalization_accepts_set_order_variation(self) -> None:
        case = load_case("graph-diamond.json")
        actual = copy.deepcopy(case["expected"])
        actual["result"]["edges"].reverse()
        actual["result"]["levels"][1].reverse()

        canonical = runner.assert_result_matches(case, actual)

        self.assertEqual(canonical, case["expected"])

    def test_result_mismatch_and_identity_mismatch_are_distinct(self) -> None:
        case = load_case("graph-diamond.json")
        mismatch = copy.deepcopy(case["expected"])
        mismatch["result"]["edges"][0] = [
            "python/pkg-a",
            "python/pkg-b",
        ]
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.assert_result_matches(case, mismatch)
        self.assertEqual(raised.exception.code, "RESULT_MISMATCH")

        wrong_identity = copy.deepcopy(case["expected"])
        wrong_identity["case_id"] = "graph/not-this-case"
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.assert_result_matches(case, wrong_identity)
        self.assertEqual(raised.exception.code, "RESULT_CASE_ID_MISMATCH")

    def test_plan_semantics_reject_unknown_references_and_duplicate_names(
        self,
    ) -> None:
        case = load_case("plan-affected-empty.json")

        duplicate = copy.deepcopy(case["expected"])
        duplicate_package = copy.deepcopy(duplicate["result"]["plan"]["packages"][0])
        duplicate_package["build_commands"] = ["different"]
        duplicate["result"]["plan"]["packages"].append(duplicate_package)
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.assert_result_matches(case, duplicate)
        self.assertEqual(raised.exception.code, "RESULT_PLAN_PACKAGE_DUPLICATE")

        unknown_edge = copy.deepcopy(case["expected"])
        unknown_edge["result"]["plan"]["dependency_edges"] = [
            ["python/missing", "python/pkg-a"]
        ]
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.assert_result_matches(case, unknown_edge)
        self.assertEqual(raised.exception.code, "RESULT_PLAN_EDGE_UNKNOWN")

        unknown_affected = copy.deepcopy(case["expected"])
        unknown_affected["result"]["plan"]["affected_packages"] = ["python/missing"]
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.assert_result_matches(case, unknown_affected)
        self.assertEqual(raised.exception.code, "RESULT_PLAN_AFFECTED_UNKNOWN")

    def test_validate_result_uses_the_bounded_parser_for_result_bytes(self) -> None:
        case_path = CASES_ROOT / "graph-diamond.json"
        with tempfile.TemporaryDirectory() as directory:
            result_path = Path(directory) / "result.json"
            result_path.write_bytes(
                b'{"schema_version":1,"case_id":"graph/diamond",'
                b'"domain":"graph","outcome":"ok","result":'
                + (b"[" * 1100)
                + b"0"
                + (b"]" * 1100)
                + b',"diagnostics":[]}'
            )
            with self.assertRaises(runner.ConformanceError) as raised:
                runner.validate_result_files(case_path, result_path)
        self.assertEqual(raised.exception.code, "JSON_DEPTH_EXCEEDED")

    def test_validate_result_enforces_the_case_output_limit(self) -> None:
        case_path = CASES_ROOT / "graph-diamond.json"
        case = load_case(case_path.name)
        payload = json.dumps(case["expected"]).encode("utf-8")
        output_limit = case["limits"]["output_bytes"]
        oversized = (b" " * (output_limit + 1 - len(payload))) + payload
        with tempfile.TemporaryDirectory() as directory:
            result_path = Path(directory) / "result.json"
            result_path.write_bytes(oversized)
            with self.assertRaises(runner.ConformanceError) as raised:
                runner.validate_result_files(case_path, result_path)
        self.assertEqual(raised.exception.code, "JSON_INPUT_TOO_LARGE")


class PureDomainValidationTests(unittest.TestCase):
    def _schema_args(self) -> dict[str, object]:
        return {
            "case_schema": runner.load_document(FIXTURE_ROOT / "schema.json"),
            "result_schema": runner.load_document(FIXTURE_ROOT / "result.schema.json"),
            "plan_schema": runner.load_document(
                runner.REPO_ROOT / "code/specs/schemas/build-plan-v1.schema.json"
            ),
            "pure_domain_schema": runner.load_document(
                FIXTURE_ROOT / "pure-domains.schema.json"
            ),
        }

    def test_cli_parser_normalizes_defaults_and_typed_values(self) -> None:
        parsed, diagnostic = runner._parse_cli_argv(
            [
                "--root=code",
                "--language",
                "ocaml",
                "--jobs=256",
                "--force",
                "--no-validate-build-files",
                "--clippy",
                "--diff-base",
                "HEAD~1",
                "--cache-file=artifacts/cache.json",
                "--emit-plan",
                "artifacts/plan.json",
                "--shard-count=256",
                "--emit-shard-matrix",
            ]
        )
        self.assertIsNone(diagnostic)
        self.assertEqual(
            parsed,
            {
                "cache_file": "artifacts/cache.json",
                "clippy": True,
                "detect_languages": False,
                "diff_base": "HEAD~1",
                "dry_run": False,
                "emit_plan": "artifacts/plan.json",
                "emit_shard_matrix": True,
                "force": True,
                "jobs": 256,
                "language": "ocaml",
                "plan_file": None,
                "root": "code",
                "shard_count": 256,
                "shard_index": None,
                "validate_build_files": False,
            },
        )

        plan_parse, plan_diagnostic = runner._parse_cli_argv(
            [
                "--root",
                ".",
                "--plan-file=artifacts/plan.json",
                "--shard-index",
                "0",
                "--detect-languages",
                "--validate-build-files",
            ]
        )
        self.assertIsNone(plan_diagnostic)
        if plan_parse is None:
            self.fail("valid plan-consumption arguments did not produce a parse")
        self.assertEqual(plan_parse["root"], ".")
        self.assertEqual(plan_parse["plan_file"], "artifacts/plan.json")
        self.assertEqual(plan_parse["shard_index"], 0)
        self.assertTrue(plan_parse["detect_languages"])
        self.assertTrue(plan_parse["validate_build_files"])

    def test_cli_parser_rejects_reserved_and_inert_host_syntax(self) -> None:
        cases = {
            ("--workspace-root=repo",): "CLI_ARGUMENT_RESERVED",
            ("@args.txt",): "CLI_ARGUMENT_UNSAFE",
            ("TOKEN=value",): "CLI_ARGUMENT_UNSAFE",
            ("--root=$HOME",): "CLI_ARGUMENT_UNSAFE",
            ("--diff-base=$(whoami)",): "CLI_ARGUMENT_UNSAFE",
            ("--root=repo>outside",): "CLI_ARGUMENT_UNSAFE",
            ("--language=python;whoami",): "CLI_ARGUMENT_UNSAFE",
            ("--language=e\u0301",): "CLI_USAGE_INVALID",
            ("--root=repo\nx",): "CLI_ARGUMENT_UNSAFE",
            ("\ud800",): "CLI_ARGUMENT_UNSAFE",
            ("--emit-plan", "../outside.json"): "CLI_PATH_UNSAFE",
            ("--jobs=0",): "CLI_USAGE_INVALID",
            ("--jobs=01",): "CLI_USAGE_INVALID",
            ("--language=",): "CLI_USAGE_INVALID",
            ("--language=zig",): "CLI_USAGE_INVALID",
            ("--diff-base=../main",): "CLI_USAGE_INVALID",
            ("--diff-base=refs/foo.lock/bar",): "CLI_USAGE_INVALID",
            ("--diff-base=refs/.hidden/main",): "CLI_USAGE_INVALID",
            ("--diff-base=refs/main.",): "CLI_USAGE_INVALID",
            ("--dry-run", "--dry-run"): "CLI_USAGE_INVALID",
            ("--plan-file=plan.json", "--emit-plan=other.json"): "CLI_USAGE_INVALID",
            ("--shard-count=2",): "CLI_USAGE_INVALID",
            ("--shard-index=0",): "CLI_USAGE_INVALID",
            ("--emit-shard-matrix",): "CLI_USAGE_INVALID",
            tuple("--force" for _ in range(65)): "CLI_ARGUMENT_LIMIT",
            ("x" * 257,): "CLI_ARGUMENT_LIMIT",
            tuple("é" * 64 for _ in range(64)): "CLI_ARGUMENT_LIMIT",
        }
        for argv, expected in cases.items():
            with self.subTest(argv=argv):
                parsed, diagnostic = runner._parse_cli_argv(list(argv))
                self.assertIsNone(parsed)
                self.assertEqual(diagnostic, expected)

    def test_cli_parse_error_precedes_a_modeled_dispatch_failure(self) -> None:
        case = load_case("cli-package-failure.json")
        invalid_usage = load_case("cli-invalid-usage.json")
        case["input"]["options"]["argv"] = ["--unknown"]
        case["expected"] = copy.deepcopy(invalid_usage["expected"])
        case["expected"]["case_id"] = case["id"]

        runner.validate_case_document(case, **self._schema_args())
        runner.assert_result_matches(
            case,
            copy.deepcopy(case["expected"]),
            pure_domain_schema=self._schema_args()["pure_domain_schema"],
        )

    def test_validation_allows_repeated_envelope_diagnostic_codes(self) -> None:
        case = load_case("validation-missing-build.json")
        case["input"]["options"]["packages"].append(
            {
                "name": "python/other",
                "rel_path": "code/packages/python/other",
                "language": "python",
                "build_file_state": "missing",
                "build_references": [],
                "is_starlark": False,
                "declared_srcs": [],
                "declared_deps": [],
            }
        )
        case["expected"]["diagnostics"].append(
            {
                "code": "BUILD_FILE_MISSING",
                "severity": "error",
                "path": "code/packages/python/other",
            }
        )

        runner.validate_case_document(case, **self._schema_args())
        runner.assert_result_matches(
            case,
            copy.deepcopy(case["expected"]),
            pure_domain_schema=self._schema_args()["pure_domain_schema"],
        )

    def test_lua_windows_sibling_parity_is_derived_from_snapshot(self) -> None:
        case = load_case("validation-lua-windows-sibling-parity-absent.json")
        runner.validate_case_document(case, **self._schema_args())
        runner.assert_result_matches(
            case,
            copy.deepcopy(case["expected"]),
            pure_domain_schema=self._schema_args()["pure_domain_schema"],
        )

        repaired = copy.deepcopy(case)
        demo = repaired["input"]["options"]["packages"][1]
        demo["windows_build_file_state"] = "present"
        demo["windows_lua_sibling_installs"] = list(
            demo["canonical_lua_sibling_installs"]
        )
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.validate_case_document(repaired, **self._schema_args())
        self.assertEqual(raised.exception.code, "EXPECTED_VALIDATION_INCONSISTENT")

    def test_lua_windows_sibling_snapshot_rejects_impossible_state(self) -> None:
        case = load_case("validation-lua-windows-sibling-parity-absent.json")
        demo = case["input"]["options"]["packages"][1]
        demo["windows_lua_sibling_installs"] = ["lua/arithmetic"]
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.validate_case_document(case, **self._schema_args())
        self.assertEqual(
            raised.exception.code,
            "CASE_VALIDATION_SNAPSHOT_INCONSISTENT",
        )

    def test_orphan_crate_coverage_is_derived_from_closed_snapshots(self) -> None:
        schema_args = self._schema_args()
        for filename in (
            "validation-orphan-crates-clean.json",
            "validation-orphan-crates-unlisted.json",
            "validation-orphan-exemptions-invalid.json",
            "validation-orphan-exemptions-stale.json",
        ):
            with self.subTest(filename=filename):
                case = load_case(filename)
                runner.validate_case_document(case, **schema_args)
                runner.assert_result_matches(
                    case,
                    copy.deepcopy(case["expected"]),
                    pure_domain_schema=schema_args["pure_domain_schema"],
                )

        dishonest = load_case("validation-orphan-crates-clean.json")
        dishonest["input"]["options"]["orphan_snapshot"]["build_files"][0]["state"] = (
            "empty"
        )
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.validate_case_document(dishonest, **schema_args)
        self.assertEqual(raised.exception.code, "EXPECTED_VALIDATION_INCONSISTENT")

        wrong_count = load_case("validation-orphan-crates-clean.json")
        wrong_result = copy.deepcopy(wrong_count["expected"])
        wrong_result["result"]["pending_exemption_count"] = 2
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.assert_result_matches(
                wrong_count,
                wrong_result,
                pure_domain_schema=schema_args["pure_domain_schema"],
            )
        self.assertEqual(raised.exception.code, "RESULT_VALIDATION_INCONSISTENT")

    def test_orphan_snapshot_joins_use_exact_portable_paths(self) -> None:
        schema_args = self._schema_args()
        case_variant = load_case("validation-orphan-crates-clean.json")
        case_variant["input"]["options"]["orphan_snapshot"]["exemptions"][0]["path"] = (
            "code/packages/rust/Compile-only"
        )
        diagnostics, pending_count = runner._expected_orphan_validation(
            case_variant["input"]["options"]
        )
        self.assertEqual(pending_count, 1)
        self.assertIn(
            (
                "ORPHAN_CRATE_UNLISTED",
                "code/packages/rust/compile-only",
            ),
            {(item["code"], item["path"]) for item in diagnostics},
        )
        self.assertTrue(
            any(
                item["code"] == "ORPHAN_EXEMPTION_STALE"
                and item["details"]["problem"] == "MISSING_DIRECTORY"
                for item in diagnostics
            )
        )

        inconsistent = load_case("validation-orphan-crates-clean.json")
        manifests = inconsistent["input"]["options"]["orphan_snapshot"]["manifests"]
        manifests[2]["path"] = "code/packages/rust/Direct"
        manifests.sort(key=lambda item: item["path"])
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.validate_case_document(inconsistent, **schema_args)
        self.assertEqual(
            raised.exception.code,
            "CASE_VALIDATION_SNAPSHOT_INCONSISTENT",
        )

    def test_orphan_snapshot_rejects_noncanonical_or_impossible_state(self) -> None:
        schema_args = self._schema_args()
        mutations = (
            lambda snapshot: snapshot["directories"].reverse(),
            lambda snapshot: snapshot["manifests"].append(
                copy.deepcopy(snapshot["manifests"][0])
            ),
            lambda snapshot: snapshot["build_files"][0].__setitem__(
                "path", "code/packages/rust/ancestor/NOT_BUILD"
            ),
            lambda snapshot: snapshot["exemptions"][1].__setitem__("line", 10),
        )
        for mutate in mutations:
            case = load_case("validation-orphan-crates-clean.json")
            mutate(case["input"]["options"]["orphan_snapshot"])
            with self.assertRaises(runner.ConformanceError) as raised:
                runner.validate_case_document(case, **schema_args)
            self.assertEqual(
                raised.exception.code,
                "CASE_VALIDATION_SNAPSHOT_INCONSISTENT",
            )

    def test_tracked_artifact_absence_is_derived_from_closed_snapshots(self) -> None:
        schema_args = self._schema_args()
        for filename in (
            "validation-tracked-artifacts-clean.json",
            "validation-tracked-artifacts-forbidden.json",
            "validation-tracked-artifacts-aliases.json",
            "validation-tracked-artifacts-invalid.json",
            "validation-tracked-artifacts-unicode-boundaries.json",
        ):
            with self.subTest(filename=filename):
                case = load_case(filename)
                runner.validate_case_document(case, **schema_args)
                runner.assert_result_matches(
                    case,
                    copy.deepcopy(case["expected"]),
                    pure_domain_schema=schema_args["pure_domain_schema"],
                )

        dishonest = load_case("validation-tracked-artifacts-clean.json")
        dishonest["input"]["options"]["tracked_artifact_snapshot"]["entries"][0][
            "path"
        ] = "code/node_modules/index.js"
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.validate_case_document(dishonest, **schema_args)
        self.assertEqual(raised.exception.code, "EXPECTED_VALIDATION_INCONSISTENT")

    def test_tracked_artifact_snapshot_normalizes_and_redacts_paths(self) -> None:
        forbidden = load_case("validation-tracked-artifacts-forbidden.json")
        diagnostics = runner._expected_tracked_artifact_validation(
            forbidden["input"]["options"]
        )
        self.assertEqual(diagnostics, forbidden["expected"]["diagnostics"])
        self.assertTrue(all("\\" not in item["path"] for item in diagnostics))

        invalid = load_case("validation-tracked-artifacts-invalid.json")
        diagnostics = runner._expected_tracked_artifact_validation(
            invalid["input"]["options"]
        )
        self.assertEqual(diagnostics, invalid["expected"]["diagnostics"])
        serialized = json.dumps(diagnostics, ensure_ascii=False)
        for entry in invalid["input"]["options"]["tracked_artifact_snapshot"][
            "entries"
        ]:
            self.assertNotIn(entry["path"], serialized)

        boundaries = load_case("validation-tracked-artifacts-unicode-boundaries.json")
        diagnostics = runner._expected_tracked_artifact_validation(
            boundaries["input"]["options"]
        )
        self.assertEqual(diagnostics, boundaries["expected"]["diagnostics"])
        forbidden = [
            item["path"]
            for item in diagnostics
            if item["code"] == "TRACKED_ARTIFACT_FORBIDDEN"
        ]
        self.assertEqual(
            forbidden,
            [
                "\ue000/node_modules/b",
                "\U00010000/node_modules/a",
                (
                    "\U0001cce3\U0001cce4\U0001ccd9\U0001ccda_"
                    "\U0001cce2\U0001cce4\U0001ccd9\U0001ccea"
                    "\U0001cce1\U0001ccda\U0001cce8/version.txt"
                ),
            ],
        )

    def test_tracked_artifact_snapshot_requires_increasing_ordinals(self) -> None:
        schema_args = self._schema_args()
        for ordinals in ((1, 1, 3), (2, 1, 3)):
            with self.subTest(ordinals=ordinals):
                case = load_case("validation-tracked-artifacts-forbidden.json")
                entries = case["input"]["options"]["tracked_artifact_snapshot"][
                    "entries"
                ]
                for entry, ordinal in zip(entries, ordinals, strict=True):
                    entry["ordinal"] = ordinal
                with self.assertRaises(runner.ConformanceError) as raised:
                    runner.validate_case_document(case, **schema_args)
                self.assertEqual(
                    raised.exception.code,
                    "CASE_VALIDATION_SNAPSHOT_INCONSISTENT",
                )

    def test_tracked_artifact_path_problem_registry_is_stable(self) -> None:
        cases = {
            "": "EMPTY",
            "a" * 513: "TOO_LONG",
            "code/e\u0301/file": "NON_NFC",
            "/code/file": "ABSOLUTE",
            "C:\\code\\file": "DRIVE_QUALIFIED",
            "code//file": "EMPTY_SEGMENT",
            "code/trailing/": "EMPTY_SEGMENT",
            "code\\trailing\\": "EMPTY_SEGMENT",
            "code/../file": "DOT_SEGMENT",
            "code/file.": "TRAILING_DOT_OR_SPACE",
            "code/file?.txt": "UNSAFE_CHARACTER",
            "code/CON.txt": "RESERVED_BASENAME",
        }
        for path, problem in cases.items():
            with self.subTest(path=path):
                normalized, actual_problem = runner._normalize_tracked_artifact_path(
                    path
                )
                self.assertIsNone(normalized)
                self.assertEqual(actual_problem, problem)
        self.assertEqual(
            runner._normalize_tracked_artifact_path("code\\src\\file.ts"),
            ("code/src/file.ts", None),
        )

    def test_remaining_validation_oracles_are_derived_from_snapshots(self) -> None:
        filenames = (
            "validation-clean-full.json",
            "validation-dependency-oracles.json",
            "validation-starlark-declarations-invalid.json",
            "validation-identity-manifest-ambiguous.json",
            "validation-toolchain-unsupported.json",
            "validation-path-unsafe.json",
        )
        schema_args = self._schema_args()
        for filename in filenames:
            with self.subTest(filename=filename):
                case = load_case(filename)
                runner.validate_case_document(case, **schema_args)
                runner.assert_result_matches(
                    case,
                    copy.deepcopy(case["expected"]),
                    pure_domain_schema=schema_args["pure_domain_schema"],
                )

    def test_validation_rejects_unknown_duplicate_and_cyclic_graph_inputs(
        self,
    ) -> None:
        schema_args = self._schema_args()

        unknown = load_case("validation-dependency-oracles.json")
        unknown["input"]["options"]["dependency_edges"][0][0] = "python/missing"
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.validate_case_document(unknown, **schema_args)
        self.assertEqual(raised.exception.code, "CASE_EDGE_UNKNOWN")

        duplicate = load_case("validation-dependency-oracles.json")
        duplicate["input"]["options"]["packages"].append(
            copy.deepcopy(duplicate["input"]["options"]["packages"][0])
        )
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.validate_case_document(duplicate, **schema_args)
        self.assertEqual(raised.exception.code, "CASE_PACKAGE_DUPLICATE")

        cyclic = load_case("validation-dependency-oracles.json")
        cyclic["input"]["options"]["dependency_edges"].append(
            ["python/app", "python/core"]
        )
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.validate_case_document(cyclic, **schema_args)
        self.assertEqual(raised.exception.code, "CASE_EDGE_CYCLE")

    def test_validation_result_cannot_self_assert_a_dishonest_outcome(self) -> None:
        case = load_case("validation-path-unsafe.json")
        actual = copy.deepcopy(case["expected"])
        actual["outcome"] = "ok"
        actual["result"] = {"valid": True, "diagnostic_codes": []}
        actual["diagnostics"] = []
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.assert_result_matches(
                case,
                actual,
                pure_domain_schema=self._schema_args()["pure_domain_schema"],
            )
        self.assertEqual(raised.exception.code, "RESULT_VALIDATION_INCONSISTENT")

    def test_starlark_load_scanner_handles_lexical_literal_forms(self) -> None:
        commented = load_case("starlark-structured-context.json")
        commented["workspace"]["files"][0]["content_utf8"] = (
            '# load("../../../../../outside.star", "x")\n'
            + commented["workspace"]["files"][0]["content_utf8"]
        )
        runner.validate_case_document(commented, **self._schema_args())

        escaping_literals = (
            '"../../../../../outside.star"',
            'r"../../../../../outside.star"',
            'b"../../../../../outside.star"',
            '"""../../../../../outside.star"""',
        )
        for literal in escaping_literals:
            with self.subTest(literal=literal):
                escaping = load_case("starlark-structured-context.json")
                escaping["workspace"]["files"][0]["content_utf8"] = escaping[
                    "workspace"
                ]["files"][0]["content_utf8"].replace(
                    '"code/build/defs.star"',
                    literal,
                )
                with self.assertRaises(runner.ConformanceError) as raised:
                    runner.validate_case_document(escaping, **self._schema_args())
                self.assertEqual(
                    raised.exception.code,
                    "EXPECTED_STARLARK_MODULE_ERROR_INVALID",
                )

        spaced = load_case("starlark-structured-context.json")
        spaced["workspace"]["files"][0]["content_utf8"] = spaced["workspace"]["files"][
            0
        ]["content_utf8"].replace("load(", "load (", 1)
        runner.validate_case_document(spaced, **self._schema_args())

        member = load_case("starlark-structured-context.json")
        member["workspace"]["files"][0]["content_utf8"] = (
            'rules.load("../../../../../outside.star")\n'
            + member["workspace"]["files"][0]["content_utf8"]
        )
        runner.validate_case_document(member, **self._schema_args())

        split = load_case("starlark-structured-context.json")
        split["workspace"]["files"][0]["content_utf8"] = split["workspace"]["files"][0][
            "content_utf8"
        ].replace("load(", "load\n(", 1)
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.validate_case_document(split, **self._schema_args())
        self.assertEqual(
            raised.exception.code,
            "EXPECTED_STARLARK_MODULE_ERROR_INVALID",
        )

    def test_starlark_metering_corpus_closes_every_budget(self) -> None:
        expected_errors = {
            "starlark-meter-step-limit.json": "STARLARK_STEP_LIMIT",
            "starlark-meter-recursion-limit.json": "STARLARK_RECURSION_LIMIT",
            "starlark-meter-aggregate-limit.json": "STARLARK_AGGREGATE_LIMIT",
            "starlark-meter-range-limit.json": "STARLARK_RANGE_LIMIT",
            "starlark-meter-value-limit.json": "STARLARK_VALUE_LIMIT",
            "starlark-meter-load-depth-limit.json": "STARLARK_LOAD_DEPTH_LIMIT",
            "starlark-meter-module-limit.json": "STARLARK_MODULE_LIMIT",
            "starlark-meter-load-cycle.json": "STARLARK_LOAD_CYCLE",
            "starlark-meter-output-limit.json": "STARLARK_OUTPUT_LIMIT",
        }

        boundary = load_case("starlark-meter-boundary.json")
        limits = boundary["input"]["options"]["evaluation_limits"]
        self.assertIn("range_items", limits)
        self.assertIn("value_bytes", limits)
        self.assertEqual(boundary["expected"]["outcome"], "ok")
        runner.validate_case_document(boundary, **self._schema_args())

        for filename, code in expected_errors.items():
            with self.subTest(filename=filename):
                case = load_case(filename)
                diagnostics = case["expected"]["diagnostics"]
                self.assertEqual(case["expected"]["outcome"], "error")
                self.assertEqual(case["expected"]["result"], {})
                self.assertEqual([item["code"] for item in diagnostics], [code])
                self.assertEqual(diagnostics[0]["severity"], "error")
                self.assertIn("path", diagnostics[0])
                runner.validate_case_document(case, **self._schema_args())

        cycle = load_case("starlark-meter-load-cycle.json")
        staged = runner.preflight_workspace(cycle)
        self.assertEqual(
            runner._starlark_module_error(cycle["input"]["options"], staged),
            ("STARLARK_LOAD_CYCLE", "code/build/a.star"),
        )

    def test_toolchain_detection_ignores_unscheduled_unsupported_packages(
        self,
    ) -> None:
        case = load_case("toolchain-detection-empty.json")
        case["input"]["options"]["packages"].append(
            {
                "name": "zig/unused",
                "language": "zig",
                "build_files": {"BUILD": ""},
            }
        )

        runner.validate_case_document(case, **self._schema_args())

    def test_toolchain_declaration_corpus_closes_selection_and_platforms(
        self,
    ) -> None:
        filenames = (
            "toolchain-detection-declarations.json",
            "toolchain-detection-crlf-grammar.json",
            "toolchain-detection-affected-only.json",
            "toolchain-detection-force-full.json",
            "toolchain-detection-platform-windows.json",
            "toolchain-detection-platform-linux.json",
            "toolchain-detection-platform-darwin.json",
        )
        for filename in filenames:
            with self.subTest(filename=filename):
                case = load_case(filename)
                runner.validate_case_document(case, **self._schema_args())
                self.assertEqual(
                    runner._expected_toolchains(case["input"]["options"]),
                    case["expected"]["result"]["toolchains"],
                )

    def test_toolchain_declarations_strip_only_crlf_carriage_return(self) -> None:
        self.assertEqual(
            runner._extra_toolchain_declarations("# needs-toolchain: python\r\n"),
            ["python"],
        )
        self.assertEqual(
            runner._extra_toolchain_declarations("# needs-toolchain: ruby\r"),
            [],
        )
        self.assertEqual(
            runner._extra_toolchain_declarations("# needs-toolchain: lua\r  "),
            [],
        )

    def test_toolchain_force_full_requires_null_selection(self) -> None:
        case = load_case("toolchain-detection-force-full.json")
        case["input"]["options"]["scheduled_packages"] = ["rust/app"]
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.validate_case_document(case, **self._schema_args())
        self.assertEqual(
            raised.exception.code,
            "CASE_TOOLCHAIN_FORCE_SELECTION_INVALID",
        )

    def test_toolchain_build_snapshots_are_byte_and_line_bounded(self) -> None:
        for content in (
            "é" * (runner.MAX_TOOLCHAIN_BUILD_BYTES // 2 + 1),
            "\n" * runner.MAX_TOOLCHAIN_BUILD_LINES,
        ):
            with self.subTest(encoded_bytes=len(content.encode("utf-8"))):
                case = load_case("toolchain-detection-declarations.json")
                package = case["input"]["options"]["packages"][0]
                package["build_files"]["BUILD"] = content
                with self.assertRaises(runner.ConformanceError) as raised:
                    runner.validate_case_document(case, **self._schema_args())
                self.assertEqual(
                    raised.exception.code,
                    "CASE_TOOLCHAIN_SNAPSHOT_LIMIT_EXCEEDED",
                )

    def test_sharding_ignores_unscheduled_unreferenced_toolchains(self) -> None:
        case = load_case("sharding-prerequisite-closed.json")
        case["input"]["options"]["packages"].append(
            {
                "name": "zig/unused",
                "language": "zig",
                "build_command_count": 0,
            }
        )

        runner.validate_case_document(case, **self._schema_args())

    def test_source_collection_corpus_closes_registry_modes_and_links(self) -> None:
        expected_components = {
            ".git",
            ".hg",
            ".svn",
            ".venv",
            ".tox",
            ".mypy_cache",
            ".pytest_cache",
            ".ruff_cache",
            ".stack-work",
            "__pycache__",
            "node_modules",
            "vendor",
            "dist",
            "dist-newstyle",
            "_build",
            "build",
            "target",
            ".claude",
            "Pods",
            ".gradle",
            ".dart_tool",
            "gradle-build",
            "deps",
            ".build",
            ".cargo",
            "cover",
        }
        cases = [
            load_case("source-collection-extension.json"),
            load_case("source-collection-declared.json"),
        ]
        self.assertEqual(
            {case["input"]["options"]["mode"] for case in cases},
            {"extension", "declared_sources"},
        )

        registry = runner.load_document(
            FIXTURE_ROOT / "language-source-input-registry.json"
        )
        registry_digest = runner.source_input_registry_digest(registry)

        for case in cases:
            options = case["input"]["options"]
            self.assertEqual(options["language"], "ocaml")
            self.assertEqual(options["registry_sha256"], registry_digest)
            self.assertNotIn("include_extensions", options)
            self.assertNotIn("special_filenames", options)
            candidates = case["input"]["options"]["candidates"]
            excluded_components = {
                candidate["path"].split("/")[1]
                for candidate in candidates
                if candidate["path"].startswith("excluded-")
            }
            self.assertEqual(excluded_components, expected_components)
            self.assertEqual(
                {candidate["kind"] for candidate in candidates},
                {"file", "symlink", "reparse_point"},
            )
            included = {entry["path"] for entry in case["expected"]["result"]["files"]}
            self.assertTrue(
                {
                    "case/_Build/generated.ml",
                    "near/Build/generated.ml",
                    "near/Dist-newstyle/generated.ml",
                    "near/_build-example/generated.ml",
                    "near/dist-newstyle-example/generated.ml",
                }.issubset(included)
            )
            self.assertTrue(
                all(
                    not path.startswith(("excluded-", "linked/", "reparse/"))
                    for path in included
                )
            )

        role_case = load_case("source-collection-registry-roles.json")
        role_options = role_case["input"]["options"]
        included = {
            entry["path"]
            for entry in runner._expected_source_collection(role_options, registry)
        }
        self.assertIn("required_capabilities.json", included)
        self.assertNotIn("nested/required_capabilities.json", included)
        self.assertNotIn("nested/pubspec.yaml", included)
        self.assertNotIn("other/lib.rs", included)
        self.assertIn("android/gradle.properties", included)
        self.assertIn(
            "android/gradle/wrapper/gradle-wrapper.properties",
            included,
        )
        self.assertNotIn("android/key.properties", included)
        self.assertNotIn("android/local.properties", included)

        engram_case = load_case("source-collection-engram-wasm-exact-inputs.json")
        engram_options = engram_case["input"]["options"]
        self.assertEqual(engram_options["registry_sha256"], registry_digest)
        self.assertEqual(
            runner._expected_source_collection(engram_options, registry),
            engram_case["expected"]["result"]["files"],
        )
        engram_included = {
            entry["path"]
            for entry in engram_case["expected"]["result"]["files"]
        }
        self.assertNotIn("js/sibling.mjs", engram_included)
        self.assertNotIn("pkg/engram_engine_copy.wasm", engram_included)
        declared_engram = copy.deepcopy(engram_options)
        declared_engram["mode"] = "declared_sources"
        declared_engram["declared_srcs"] = ["unrelated/**"]
        self.assertEqual(
            runner._expected_source_collection(declared_engram, registry),
            engram_case["expected"]["result"]["files"],
        )
        case_variants = copy.deepcopy(engram_options)
        case_variants["candidates"] = [
            {
                "path": "js/Smoke.mjs",
                "kind": "file",
                "content_hex": "736f757263650a",
            },
            {
                "path": "pkg/Engram_engine.wasm",
                "kind": "file",
                "content_hex": "736f757263650a",
            },
        ]
        self.assertEqual(
            runner._expected_source_collection(case_variants, registry),
            [],
        )
        other_rust_package = copy.deepcopy(engram_options)
        other_rust_package["package_root"] = "code/packages/rust/task-wasm"
        other_rust_package["candidates"] = [
            {
                "path": "js/smoke.mjs",
                "kind": "file",
                "content_hex": "736f757263650a",
            }
        ]
        self.assertEqual(
            runner._expected_source_collection(other_rust_package, registry),
            [],
        )

        wrong_digest = copy.deepcopy(role_options)
        wrong_digest["registry_sha256"] = "0" * 64
        with self.assertRaises(runner.ConformanceError) as raised:
            runner._expected_source_collection(wrong_digest, registry)
        self.assertEqual(
            raised.exception.code,
            "CASE_SOURCE_REGISTRY_DIGEST_MISMATCH",
        )

        unknown_language = copy.deepcopy(role_options)
        unknown_language["language"] = "zig"
        with self.assertRaises(runner.ConformanceError) as raised:
            runner._expected_source_collection(unknown_language, registry)
        self.assertEqual(raised.exception.code, "CASE_SOURCE_LANGUAGE_UNKNOWN")

        changed = copy.deepcopy(role_options)
        capability = next(
            candidate
            for candidate in changed["candidates"]
            if candidate["path"] == "required_capabilities.json"
        )
        capability["content_hex"] = "6368616e6765640a"
        before = {
            entry["path"]: entry["digest"]
            for entry in runner._expected_source_collection(role_options, registry)
        }
        after = {
            entry["path"]: entry["digest"]
            for entry in runner._expected_source_collection(changed, registry)
        }
        self.assertNotEqual(
            before["required_capabilities.json"],
            after["required_capabilities.json"],
        )

    def test_declared_source_glob_work_is_bounded_across_candidates(self) -> None:
        self.assertIsNotNone(runner.portable_glob_error("src/[a--!].cs"))
        registry = runner.load_document(
            FIXTURE_ROOT / "language-source-input-registry.json"
        )
        options = {
            "language": "csharp",
            "package_root": "code/packages/csharp/demo",
            "mode": "declared_sources",
            "registry_sha256": runner.source_input_registry_digest(registry),
            "declared_srcs": [
                f"unmatched/{'a' * 220}{index:03d}*.cs" for index in range(256)
            ],
            "candidates": [
                {
                    "path": f"src/file{index:03d}.cs",
                    "kind": "file",
                    "content_hex": "61",
                }
                for index in range(100)
            ],
        }

        with self.assertRaises(runner.ConformanceError) as raised:
            runner._expected_source_collection(options, registry)
        self.assertEqual(raised.exception.code, "SOURCE_HASH_LIMIT_EXCEEDED")

    def test_repository_source_collection_closes_shared_and_pruned_boundaries(
        self,
    ) -> None:
        boundary = runner.load_document(
            FIXTURE_ROOT / "repository-source-input-boundary.json"
        )
        boundary_digest = runner.repository_source_input_boundary_digest(boundary)
        case_names = [
            "source-collection-repository-cross-language-workspace.json",
            "source-collection-repository-direct-build-inputs.json",
            "source-collection-repository-link-boundaries.json",
            "source-collection-repository-python-workspace.json",
            "source-collection-repository-rust-boundary.json",
            "source-collection-repository-starlark-load.json",
            "source-collection-repository-typescript-program-shared.json",
            "source-collection-repository-typescript-shared.json",
            "source-collection-repository-visicalc-deno-cross-package.json",
        ]
        cases = [load_case(name) for name in case_names]

        for case in cases:
            with self.subTest(case=case["id"]):
                options = case["input"]["options"]
                self.assertEqual(case["input"]["operation"], "source_collection")
                self.assertEqual(options["mode"], "repository_boundary")
                self.assertEqual(options["boundary_sha256"], boundary_digest)
                self.assertEqual(
                    runner._expected_repository_source_collection(options, boundary),
                    case["expected"]["result"]["files"],
                )
                runner.validate_case_document(case, **self._schema_args())

        rust_options = copy.deepcopy(cases[0]["input"]["options"])
        rust_options["boundary_sha256"] = "0" * 64
        with self.assertRaises(runner.ConformanceError) as raised:
            runner._expected_repository_source_collection(rust_options, boundary)
        self.assertEqual(
            raised.exception.code,
            "CASE_REPOSITORY_SOURCE_BOUNDARY_DIGEST_MISMATCH",
        )

        unmatched = copy.deepcopy(cases[1]["input"]["options"])
        unmatched["package_root"] = "code/programs/typescript/unregistered"
        self.assertEqual(
            runner._expected_repository_source_collection(unmatched, boundary),
            [],
        )

        excluded = copy.deepcopy(
            load_case("source-collection-repository-rust-boundary.json")["input"][
                "options"
            ]
        )
        excluded["package_root"] = "code/packages/rust/os-kernel"
        self.assertEqual(
            runner._expected_repository_source_collection(excluded, boundary),
            [],
        )

        mismatched_language = copy.deepcopy(cases[1])
        mismatched_language["input"]["options"]["language"] = "rust"
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.validate_case_document(
                mismatched_language,
                **self._schema_args(),
            )
        self.assertEqual(
            raised.exception.code,
            "CASE_REPOSITORY_SOURCE_ROOT_LANGUAGE_MISMATCH",
        )

    def test_repository_boundary_reverse_diff_is_exact_and_digest_pinned(
        self,
    ) -> None:
        boundary = runner.load_document(
            FIXTURE_ROOT / "repository-source-input-boundary.json"
        )
        case = load_case("diff-selection-repository-boundary.json")
        options = case["input"]["options"]

        self.assertEqual(
            runner._expected_diff_selection(
                options,
                case["input"]["changed_paths"],
                boundary,
            ),
            (
                {"swift/conduit"},
                {"swift/conduit", "swift/app"},
                {"swift/base"},
            ),
        )
        runner.validate_case_document(
            case,
            **self._schema_args(),
            repository_source_input_boundary=boundary,
        )

        mismatched = copy.deepcopy(case)
        mismatched["input"]["options"]["boundary_sha256"] = "0" * 64
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.validate_case_document(
                mismatched,
                **self._schema_args(),
                repository_source_input_boundary=boundary,
            )
        self.assertEqual(
            raised.exception.code,
            "CASE_REPOSITORY_SOURCE_BOUNDARY_DIGEST_MISMATCH",
        )

        near_path = copy.deepcopy(case)
        near_path["input"]["changed_paths"] = [
            "code/packages/rust/Cargo.toml.backup"
        ]
        self.assertIsNone(
            runner._expected_diff_selection(
                near_path["input"]["options"],
                near_path["input"]["changed_paths"],
                boundary,
            )
        )

    def test_hashing_cache_sorts_local_and_boundary_union_by_raw_utf8(self) -> None:
        case = load_case("hashing-cache-local-boundary-union.json")
        expected = case["expected"]["result"]
        package_digest, dependencies_digest, combined_digest = (
            runner._expected_hashes(
                case["input"]["options"],
                runner.preflight_workspace(case),
            )
        )

        self.assertEqual(package_digest, expected["package_digest"])
        self.assertEqual(dependencies_digest, expected["dependencies_digest"])
        self.assertEqual(combined_digest, expected["combined_digest"])
        runner.validate_case_document(case, **self._schema_args())

    def test_dependency_cycles_are_rejected_without_recursion(self) -> None:
        cyclic = load_case("diff-selection-transitive.json")
        cyclic["input"]["options"]["edges"].append(["python/app", "python/base"])
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.validate_case_document(cyclic, **self._schema_args())
        self.assertEqual(raised.exception.code, "CASE_EDGE_CYCLE")

        names = {f"python/package-{index}" for index in range(2000)}
        edges = [
            [f"python/package-{index}", f"python/package-{index + 1}"]
            for index in range(1999)
        ]
        edges.append(["python/package-1999", "python/package-0"])
        with self.assertRaises(runner.ConformanceError) as raised:
            runner._validate_known_edges(edges, names)
        self.assertEqual(raised.exception.code, "CASE_EDGE_CYCLE")

    def test_semantics_reject_unknown_references_and_bad_oracles(self) -> None:
        pure_schema = runner.load_document(FIXTURE_ROOT / "pure-domains.schema.json")
        schema_args = {
            "case_schema": runner.load_document(FIXTURE_ROOT / "schema.json"),
            "result_schema": runner.load_document(FIXTURE_ROOT / "result.schema.json"),
            "plan_schema": runner.load_document(
                runner.REPO_ROOT / "code/specs/schemas/build-plan-v1.schema.json"
            ),
            "pure_domain_schema": pure_schema,
            "source_input_registry": runner.load_document(
                FIXTURE_ROOT / "language-source-input-registry.json"
            ),
            "repository_source_input_boundary": runner.load_document(
                FIXTURE_ROOT / "repository-source-input-boundary.json"
            ),
        }

        unknown_edge = load_case("diff-selection-transitive.json")
        unknown_edge["input"]["options"]["edges"][0][0] = "python/missing"
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.validate_case_document(unknown_edge, **schema_args)
        self.assertEqual(raised.exception.code, "CASE_EDGE_UNKNOWN")

        hashing = load_case("hashing-cache-corrupt.json")
        hashing["expected"]["result"]["package_digest"] = "0" * 64
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.validate_case_document(hashing, **schema_args)
        self.assertEqual(raised.exception.code, "EXPECTED_HASH_MISMATCH")

        source_collection = load_case("source-collection-extension.json")
        source_collection["expected"]["result"]["files"][0]["digest"] = "0" * 64
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.validate_case_document(source_collection, **schema_args)
        self.assertEqual(
            raised.exception.code,
            "EXPECTED_SOURCE_COLLECTION_MISMATCH",
        )

        mutations = (
            (
                "starlark-structured-context.json",
                lambda result: result["result"]["targets"][0][
                    "rendered_commands"
                ].__setitem__(0, "wrong"),
                "RESULT_STARLARK_RENDER_MISMATCH",
            ),
            (
                "sharding-prerequisite-closed.json",
                lambda result: result["result"]["shards"][0].__setitem__(
                    "estimated_cost", 10
                ),
                "RESULT_SHARD_MISMATCH",
            ),
            (
                "validation-missing-build.json",
                lambda result: result["result"].__setitem__("diagnostic_codes", []),
                "RESULT_VALIDATION_INCONSISTENT",
            ),
            (
                "toolchain-detection-shared.json",
                lambda result: result["result"]["toolchains"].__setitem__(
                    "ocaml", True
                ),
                "RESULT_TOOLCHAIN_MISMATCH",
            ),
            (
                "cli-dry-run-success.json",
                lambda result: result["result"].__setitem__("exit_code", 1),
                "RESULT_CLI_EXIT_MISMATCH",
            ),
            (
                "cli-explicit-options-success.json",
                lambda result: result["result"]["parsed"].__setitem__("jobs", 7),
                "RESULT_CLI_PARSE_MISMATCH",
            ),
            (
                "cli-dry-run-success.json",
                lambda result: result["diagnostics"].append(
                    {"code": "CLI_USAGE_INVALID", "severity": "error"}
                ),
                "RESULT_CLI_DIAGNOSTIC_MISMATCH",
            ),
        )
        for filename, mutate, code in mutations:
            with self.subTest(filename=filename):
                case = load_case(filename)
                actual = copy.deepcopy(case["expected"])
                mutate(actual)
                with self.assertRaises(runner.ConformanceError) as raised:
                    runner.assert_result_matches(
                        case,
                        actual,
                        pure_domain_schema=pure_schema,
                    )
                self.assertEqual(raised.exception.code, code)

    def test_nested_paths_and_pure_authority_are_rejected(self) -> None:
        schema_args = {
            "case_schema": runner.load_document(FIXTURE_ROOT / "schema.json"),
            "result_schema": runner.load_document(FIXTURE_ROOT / "result.schema.json"),
            "plan_schema": runner.load_document(
                runner.REPO_ROOT / "code/specs/schemas/build-plan-v1.schema.json"
            ),
            "pure_domain_schema": runner.load_document(
                FIXTURE_ROOT / "pure-domains.schema.json"
            ),
        }
        mutations = (
            (
                "diff-selection-transitive.json",
                lambda case: case["input"]["options"]["packages"][0].__setitem__(
                    "rel_path", "code/packages/python/NUL"
                ),
                "CASE_NESTED_PATH_UNSAFE",
            ),
            (
                "hashing-cache-hit.json",
                lambda case: case["input"]["options"]["include_paths"].append(
                    "code/packages/python/demo/host-only"
                ),
                "CASE_HASH_PATH_UNKNOWN",
            ),
            (
                "starlark-structured-context.json",
                lambda case: case["input"]["options"].__setitem__(
                    "entrypoint", "code/packages/python/demo/missing.star"
                ),
                "CASE_STARLARK_ENTRYPOINT_MISSING",
            ),
            (
                "hashing-cache-hit.json",
                lambda case: case["workspace"]["files"][0].__setitem__(
                    "executable", True
                ),
                "CASE_PURE_AUTHORITY",
            ),
            (
                "source-collection-extension.json",
                lambda case: case["input"]["options"]["candidates"].append(
                    copy.deepcopy(case["input"]["options"]["candidates"][0])
                ),
                "CASE_SOURCE_CANDIDATE_DUPLICATE",
            ),
            (
                "source-collection-extension.json",
                lambda case: case["input"]["options"]["candidates"].append(
                    {
                        "path": "src/MAIN.ml",
                        "kind": "file",
                        "content_hex": "00",
                    }
                ),
                "CASE_SOURCE_CANDIDATE_DUPLICATE",
            ),
            (
                "source-collection-extension.json",
                lambda case: case["input"]["options"]["candidates"].append(
                    {
                        "path": "src/safe\u202efile.ml",
                        "kind": "file",
                        "content_hex": "00",
                    }
                ),
                "CASE_SOURCE_PATH_UNSAFE",
            ),
            (
                "source-collection-extension.json",
                lambda case: case["input"]["options"]["candidates"].append(
                    {"path": "src", "kind": "file", "content_hex": "00"}
                ),
                "CASE_SOURCE_CANDIDATE_COLLISION",
            ),
        )
        for filename, mutate, code in mutations:
            with self.subTest(filename=filename, code=code):
                case = load_case(filename)
                mutate(case)
                with self.assertRaises(runner.ConformanceError) as raised:
                    runner.validate_case_document(case, **schema_args)
                self.assertEqual(raised.exception.code, code)

    def test_pure_domain_set_order_is_canonicalized(self) -> None:
        diff = load_case("diff-selection-transitive.json")
        actual = copy.deepcopy(diff["expected"])
        actual["result"]["affected_packages"].reverse()
        self.assertEqual(runner.assert_result_matches(diff, actual), diff["expected"])

        shard = load_case("sharding-prerequisite-closed.json")
        actual = copy.deepcopy(shard["expected"])
        actual["result"]["shards"].reverse()
        actual["result"]["shards"][0]["package_names"].reverse()
        actual["result"]["shards"][0]["toolchains"].reverse()
        self.assertEqual(
            runner.assert_result_matches(shard, actual),
            shard["expected"],
        )

        source_collection = load_case("source-collection-extension.json")
        actual = copy.deepcopy(source_collection["expected"])
        actual["result"]["files"].reverse()
        self.assertEqual(
            runner.assert_result_matches(source_collection, actual),
            source_collection["expected"],
        )

    def test_pure_domain_validation_has_no_host_side_effects(self) -> None:
        pure_schema = runner.load_document(FIXTURE_ROOT / "pure-domains.schema.json")
        pure_domains = set(pure_schema["$defs"]["pure_domain"]["enum"])
        schema_args = {
            "case_schema": runner.load_document(FIXTURE_ROOT / "schema.json"),
            "result_schema": runner.load_document(FIXTURE_ROOT / "result.schema.json"),
            "plan_schema": runner.load_document(
                runner.REPO_ROOT / "code/specs/schemas/build-plan-v1.schema.json"
            ),
            "pure_domain_schema": pure_schema,
            "source_input_registry": runner.load_document(
                FIXTURE_ROOT / "language-source-input-registry.json"
            ),
            "repository_source_input_boundary": runner.load_document(
                FIXTURE_ROOT / "repository-source-input-boundary.json"
            ),
        }
        cases = [
            runner.load_document(path)
            for path in sorted(CASES_ROOT.glob("*.json"))
            if runner.load_document(path)["domain"] in pure_domains
        ]
        guarded_os = mock.Mock(wraps=os)

        with (
            mock.patch.object(tempfile, "TemporaryDirectory") as temporary,
            mock.patch.object(subprocess, "run") as process,
            mock.patch.object(subprocess, "Popen") as popen,
            mock.patch.object(runner, "os", guarded_os),
            mock.patch.object(os, "system") as system,
            mock.patch.object(os, "chmod") as chmod,
            mock.patch("urllib.request.urlopen") as retrieve,
            mock.patch("socket.socket") as network_socket,
            mock.patch("builtins.open") as file_open,
        ):
            for case in cases:
                runner.validate_case_document(case, **schema_args)

        temporary.assert_not_called()
        process.assert_not_called()
        popen.assert_not_called()
        guarded_os.getcwd.assert_not_called()
        guarded_os.cpu_count.assert_not_called()
        guarded_os.getenv.assert_not_called()
        self.assertEqual(guarded_os.environ.mock_calls, [])
        system.assert_not_called()
        chmod.assert_not_called()
        retrieve.assert_not_called()
        network_socket.assert_not_called()
        file_open.assert_not_called()


class CommandLineTests(unittest.TestCase):
    def test_validate_corpus_prints_a_machine_readable_summary(self) -> None:
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            exit_code = runner.main(
                ["validate-corpus", "--fixture-root", str(FIXTURE_ROOT)]
            )

        self.assertEqual(exit_code, 0)
        summary = json.loads(stdout.getvalue())
        self.assertEqual(summary["case_count"], 141)

    def test_validate_result_reports_match_and_rejects_execution_override(self) -> None:
        case_path = CASES_ROOT / "graph-diamond.json"
        case = load_case(case_path.name)
        with tempfile.TemporaryDirectory() as directory:
            result_path = Path(directory) / "result.json"
            result_path.write_text(
                json.dumps(case["expected"]),
                encoding="utf-8",
            )
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                exit_code = runner.main(
                    [
                        "validate-result",
                        "--case",
                        str(case_path),
                        "--result",
                        str(result_path),
                    ]
                )
            self.assertEqual(exit_code, 0)
            result = json.loads(stdout.getvalue())
            self.assertEqual(result["status"], "matched")
            self.assertEqual(result["conformance_status"], "pass")

        stderr = io.StringIO()
        with redirect_stderr(stderr):
            exit_code = runner.main(["validate-corpus", "--allow-execution"])
        self.assertEqual(exit_code, 2)

    def test_result_mismatch_is_a_conformance_exit_not_an_input_exit(self) -> None:
        case_path = CASES_ROOT / "graph-diamond.json"
        case = load_case(case_path.name)
        mismatch = copy.deepcopy(case["expected"])
        mismatch["result"]["edges"][0] = [
            "python/pkg-a",
            "python/pkg-b",
        ]
        with tempfile.TemporaryDirectory() as directory:
            result_path = Path(directory) / "result.json"
            result_path.write_text(json.dumps(mismatch), encoding="utf-8")
            stderr = io.StringIO()
            with redirect_stderr(stderr):
                exit_code = runner.main(
                    [
                        "validate-result",
                        "--case",
                        str(case_path),
                        "--result",
                        str(result_path),
                    ]
                )
        self.assertEqual(exit_code, 1)
        self.assertEqual(json.loads(stderr.getvalue())["code"], "RESULT_MISMATCH")

    def test_pure_semantic_mismatch_is_a_conformance_exit(self) -> None:
        case_path = CASES_ROOT / "hashing-cache-hit.json"
        case = load_case(case_path.name)
        mismatch = copy.deepcopy(case["expected"])
        mismatch["result"]["package_digest"] = "0" * 64
        with tempfile.TemporaryDirectory() as directory:
            result_path = Path(directory) / "result.json"
            result_path.write_text(json.dumps(mismatch), encoding="utf-8")
            stderr = io.StringIO()
            with redirect_stderr(stderr):
                exit_code = runner.main(
                    [
                        "validate-result",
                        "--case",
                        str(case_path),
                        "--result",
                        str(result_path),
                    ]
                )
        self.assertEqual(exit_code, 1)
        self.assertEqual(
            json.loads(stderr.getvalue())["code"],
            "RESULT_HASH_MISMATCH",
        )

    def test_matching_unsupported_result_is_never_reported_as_passing(self) -> None:
        case = load_case("discovery-simple.json")
        case["id"] = "discovery/unsupported"
        case["expected"] = {
            "schema_version": 1,
            "case_id": "discovery/unsupported",
            "domain": "discovery",
            "outcome": "unsupported",
            "result": {},
            "diagnostics": [
                {
                    "code": "DISCOVERY_UNSUPPORTED",
                    "severity": "error",
                }
            ],
        }
        with tempfile.TemporaryDirectory() as directory:
            case_path = Path(directory) / "case.json"
            result_path = Path(directory) / "result.json"
            case_path.write_text(json.dumps(case), encoding="utf-8")
            result_path.write_text(
                json.dumps(case["expected"]),
                encoding="utf-8",
            )
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                exit_code = runner.main(
                    [
                        "validate-result",
                        "--case",
                        str(case_path),
                        "--result",
                        str(result_path),
                    ]
                )
        result = json.loads(stdout.getvalue())
        self.assertEqual(exit_code, 1)
        self.assertEqual(result["status"], "matched")
        self.assertEqual(result["conformance_status"], "non-passing")
        self.assertEqual(result["outcome"], "unsupported")


if __name__ == "__main__":
    unittest.main()
