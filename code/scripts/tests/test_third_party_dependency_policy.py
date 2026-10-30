from __future__ import annotations

import contextlib
import io
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPTS_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS_DIR))

import third_party_dependency_policy as policy  # noqa: E402


class CargoManifestTests(unittest.TestCase):
    def test_collects_normal_dev_build_workspace_and_target_dependencies(self) -> None:
        manifest = """
[dependencies]
serde = "1"
renamed-json = { package = "serde_json", version = "1" }
local-core = { path = "../local-core" }
shared = { workspace = true }

[dev-dependencies]
tempfile = "3"

[build-dependencies]
cc = { git = "https://example.invalid/cc" }

[workspace.dependencies]
shared = "2"

[target.'cfg(windows)'.dependencies]
windows = "0.62"

[target.'cfg(unix)'.build-dependencies]
libc = "0.2"
"""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / "package" / "Cargo.toml"
            local_manifest = root / "local-core" / "Cargo.toml"
            path.parent.mkdir()
            local_manifest.parent.mkdir()
            local_manifest.write_text(
                "[package]\nname = 'local-core'\n", encoding="utf-8"
            )
            path.write_text(manifest, encoding="utf-8")

            self.assertEqual(
                policy.manifest_dependency_names(
                    path,
                    root,
                    {path.resolve(), local_manifest.resolve()},
                ),
                {"cc", "libc", "serde", "serde_json", "shared", "tempfile", "windows"},
            )

    def test_rejects_an_invalid_dependency_declaration(self) -> None:
        with self.assertRaisesRegex(policy.PolicyError, "unsupported declaration"):
            policy.third_party_name("broken", 42)

    def test_untracked_or_outside_path_dependencies_are_not_trusted(self) -> None:
        root = Path("/repo").resolve()
        manifest = root / "package" / "Cargo.toml"
        tracked = {manifest}

        self.assertEqual(
            policy.third_party_name(
                "outside",
                {"path": "../../../outside"},
                manifest_path=manifest,
                repository_root=root,
                tracked_manifests=tracked,
            ),
            "outside",
        )
        self.assertEqual(
            policy.third_party_name(
                "vendored",
                {"path": "../vendor"},
                manifest_path=manifest,
                repository_root=root,
                tracked_manifests=tracked,
            ),
            "vendored",
        )

    def test_reads_a_sparse_manifest_from_the_git_index(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            path = root / "Cargo.toml"
            path.write_text('[dependencies]\nserde = "1"\n', encoding="utf-8")
            subprocess.run(["git", "add", "Cargo.toml"], cwd=root, check=True)
            path.unlink()

            self.assertEqual(
                policy.manifest_dependency_names(path, root, {path.resolve()}),
                {"serde"},
            )

    def test_sparse_inventory_discovers_an_absent_nested_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            path = root / "nested" / "Cargo.toml"
            path.parent.mkdir()
            path.write_text('[dependencies]\nserde = "1"\n', encoding="utf-8")
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            path.unlink()

            manifests = policy.git_cargo_manifests(root)
            inventory = policy.build_inventory(root, manifests)

            self.assertEqual(manifests, [path])
            self.assertEqual(inventory, {"serde": ["nested/Cargo.toml"]})


class NpmManifestTests(unittest.TestCase):
    def test_collects_all_dependency_tables_and_normalizes_aliases(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            app = root / "app" / "package.json"
            local = root / "local" / "package.json"
            app.parent.mkdir()
            local.parent.mkdir()
            app.write_text(
                json.dumps(
                    {
                        "name": "app",
                        "dependencies": {
                            "local": "file:../local",
                            "renamed": "npm:actual-package@^1.0.0",
                        },
                        "devDependencies": {"typescript": "^5.0.0"},
                        "peerDependencies": {"scoped-alias": "npm:@scope/actual@2"},
                        "optionalDependencies": {"optional-tool": "3"},
                    }
                ),
                encoding="utf-8",
            )
            local.write_text(
                json.dumps({"name": "local", "dependencies": {}}),
                encoding="utf-8",
            )
            tracked = {app.resolve(), local.resolve()}

            self.assertEqual(
                policy.npm_manifest_dependency_names(
                    app,
                    root,
                    tracked,
                    {"app", "local"},
                ),
                {
                    "@scope/actual",
                    "actual-package",
                    "optional-tool",
                    "typescript",
                },
            )

    def test_only_tracked_in_repository_file_edges_are_local(self) -> None:
        root = Path("/repo").resolve()
        manifest = root / "app" / "package.json"

        self.assertEqual(
            policy.npm_third_party_name(
                "outside",
                "file:../../../outside",
                manifest_path=manifest,
                repository_root=root,
                tracked_manifests={manifest},
                local_package_names={"outside"},
            ),
            "outside",
        )
        self.assertEqual(
            policy.npm_third_party_name(
                "vendored",
                "file:../vendor",
                manifest_path=manifest,
                repository_root=root,
                tracked_manifests={manifest},
                local_package_names={"vendored"},
            ),
            "vendored",
        )

    def test_rejects_an_invalid_npm_specifier(self) -> None:
        with self.assertRaisesRegex(policy.PolicyError, "invalid npm specifier"):
            policy.npm_third_party_name(
                "broken",
                42,
                manifest_path=Path("/repo/app/package.json"),
                repository_root=Path("/repo"),
                tracked_manifests=set(),
                local_package_names=set(),
            )

    def test_rejects_duplicate_json_keys(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / "package.json"
            path.write_text(
                '{"dependencies":{"safe":"1","safe":"file:../safe"}}',
                encoding="utf-8",
            )

            with self.assertRaisesRegex(policy.PolicyError, "duplicate JSON key"):
                policy.npm_manifest_dependency_names(
                    path,
                    root,
                    {path.resolve()},
                    set(),
                )

    def test_sparse_inventory_discovers_an_absent_npm_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            path = root / "nested" / "package.json"
            path.parent.mkdir()
            path.write_text(
                json.dumps(
                    {
                        "name": "nested",
                        "devDependencies": {"typescript": "5"},
                    }
                ),
                encoding="utf-8",
            )
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            path.unlink()

            manifests = policy.git_npm_manifests(root)
            inventory = policy.build_npm_inventory(root, manifests)

            self.assertEqual(manifests, [path])
            self.assertEqual(inventory, {"typescript": ["nested/package.json"]})


class NpmToolTests(unittest.TestCase):
    def test_collects_npx_exec_and_named_install_tools(self) -> None:
        commands = """
npx --prefix tools tsx scripts/build.ts && npx --yes @vscode/vsce package
npm exec --package=typescript -- tsc -p .
npm install --global npm@11.13.0 jest@29
npm install --silent 2>/dev/null
if sudo env CI=1 npm add alias@npm:actual-tool@3; then (npx wrapped-tool); fi
echo "npm install imaginary-package"
# Error: npm install imaginary-package
# note; npx imaginary-package
cmd = "npx vitest run"
"""

        self.assertEqual(
            policy.npm_tool_names(commands),
            {
                "@vscode/vsce",
                "actual-tool",
                "jest",
                "npm",
                "tsx",
                "typescript",
                "vitest",
                "wrapped-tool",
            },
        )

    def test_normalizes_scoped_and_unscoped_versions(self) -> None:
        self.assertEqual(policy.npm_package_name("npm@11.13.0"), "npm")
        self.assertEqual(policy.npm_package_name("@scope/tool@2"), "@scope/tool")
        self.assertEqual(
            policy.npm_package_name("alias@npm:@scope/actual@2"),
            "@scope/actual",
        )
        self.assertIsNone(policy.npm_package_name("file:../local-tool"))

    def test_rejects_unnamed_remote_tool_acquisition(self) -> None:
        with self.assertRaisesRegex(policy.PolicyError, "must use a package name"):
            policy.npm_tool_names("npm install https://example.invalid/tool.tgz")

    def test_rejects_missing_option_values(self) -> None:
        with self.assertRaisesRegex(policy.PolicyError, "requires a value"):
            policy.npm_tool_names("npx --prefix")
        with self.assertRaisesRegex(policy.PolicyError, "requires an npm package"):
            policy.npm_tool_names("npx --package")

    def test_sparse_inventory_reads_sources_and_package_scripts_from_index(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            workflow = root / ".github" / "workflows" / "ci.yml"
            manifest = root / "app" / "package.json"
            workflow.parent.mkdir(parents=True)
            manifest.parent.mkdir()
            workflow.write_text(
                "steps:\n  - run: npm install --global npm@11\n",
                encoding="utf-8",
            )
            manifest.write_text(
                json.dumps({"scripts": {"build": "npx esbuild src.ts"}}),
                encoding="utf-8",
            )
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            workflow.unlink()
            manifest.unlink()

            sources = policy.git_npm_tool_sources(root)
            manifests = policy.git_npm_manifests(root)
            inventory = policy.build_npm_tool_inventory(root, sources, manifests)

            self.assertEqual(sources, [workflow])
            self.assertEqual(manifests, [manifest])
            self.assertEqual(
                inventory,
                {
                    "esbuild": ["app/package.json"],
                    "npm": [".github/workflows/ci.yml"],
                },
            )

    def test_rejects_non_string_package_scripts(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = root / "package.json"
            manifest.write_text(
                json.dumps({"scripts": {"test": 42}}),
                encoding="utf-8",
            )

            with self.assertRaisesRegex(policy.PolicyError, "map strings to strings"):
                policy.build_npm_tool_inventory(root, [], [manifest])


class PythonToolTests(unittest.TestCase):
    def test_collects_pip_pipx_and_uv_tool_packages(self) -> None:
        commands = """
python3 -m pip install --quiet jsonschema==4.26.0 fonttools
pip install --no-deps -e ../matrix -e . build
pip install -e ".[dev]" > /dev/null 2>&1
uv pip install --system pytest pytest-cov
uv tool install --with ruff mypy
uv tool run --from black black --check .
uvx --with typing-extensions pyright .
pipx run --spec hatch hatch build
echo "pip install imaginary-package"
# pip install imaginary-package
cmd = "python -m pip install requests"
"""

        self.assertEqual(
            policy.python_tool_names(commands),
            {
                "black",
                "build",
                "fonttools",
                "hatch",
                "jsonschema",
                "mypy",
                "pyright",
                "pytest",
                "pytest-cov",
                "requests",
                "ruff",
                "typing-extensions",
            },
        )

    def test_named_direct_urls_are_counted_and_unnamed_urls_are_rejected(
        self,
    ) -> None:
        self.assertEqual(
            policy.python_tool_names(
                "pip install 'named-tool @ https://example.invalid/tool.whl'"
            ),
            {"named-tool"},
        )
        with self.assertRaisesRegex(policy.PolicyError, "must use a package name"):
            policy.python_tool_names(
                "python -m pip install https://example.invalid/tool.whl"
            )

    def test_rejects_requirements_file_indirection_and_missing_values(self) -> None:
        with self.assertRaisesRegex(policy.PolicyError, "declare packages inline"):
            policy.python_tool_names("pip install -r requirements.txt")
        with self.assertRaisesRegex(policy.PolicyError, "requires a value"):
            policy.python_tool_names("uv pip install --index-url")
        with self.assertRaisesRegex(policy.PolicyError, "requires a Python package"):
            policy.python_tool_names("uvx --from")

    def test_sparse_inventory_reads_sources_and_package_scripts_from_index(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            workflow = root / ".github" / "workflows" / "ci.yml"
            manifest = root / "app" / "package.json"
            workflow.parent.mkdir(parents=True)
            manifest.parent.mkdir()
            workflow.write_text(
                "steps:\n  - run: python -m pip install pytest\n",
                encoding="utf-8",
            )
            manifest.write_text(
                json.dumps({"scripts": {"lint": "uvx ruff check ."}}),
                encoding="utf-8",
            )
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            workflow.unlink()
            manifest.unlink()

            sources = policy.git_python_tool_sources(root)
            manifests = policy.git_npm_manifests(root)
            inventory = policy.build_python_tool_inventory(root, sources, manifests)

            self.assertEqual(sources, [workflow])
            self.assertEqual(manifests, [manifest])
            self.assertEqual(
                inventory,
                {
                    "pytest": [".github/workflows/ci.yml"],
                    "ruff": ["app/package.json"],
                },
            )


class PythonManifestTests(unittest.TestCase):
    def test_collects_standard_groups_and_normalizes_distribution_names(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            app = root / "app" / "pyproject.toml"
            local = root / "local" / "pyproject.toml"
            app.parent.mkdir()
            local.parent.mkdir()
            app.write_text(
                """
[build-system]
requires = ["Hatchling>=1"]

[project]
name = "app"
dependencies = [
  "local_pkg>=1",
  "Requests[security] @ https://example.invalid/requests.whl",
]

[project.optional-dependencies]
images = ["Pillow>=10"]

[dependency-groups]
test = ["pytest>=9", { include-group = "lint" }]
lint = ["typing_extensions; python_version < '3.13'"]

[tool.uv.sources]
local-pkg = { path = "../local" }
""",
                encoding="utf-8",
            )
            local.write_text(
                '[project]\nname = "local-pkg"\nversion = "1"\n',
                encoding="utf-8",
            )

            self.assertEqual(
                policy.build_python_inventory(root, [app, local]),
                {
                    "hatchling": ["app/pyproject.toml"],
                    "pillow": ["app/pyproject.toml"],
                    "pytest": ["app/pyproject.toml"],
                    "requests": ["app/pyproject.toml"],
                    "typing-extensions": ["app/pyproject.toml"],
                },
            )

    def test_workspace_sources_apply_only_to_verified_workspace_members(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            workspace = root / "workspace" / "pyproject.toml"
            app = root / "workspace" / "app" / "pyproject.toml"
            local = root / "workspace" / "local" / "pyproject.toml"
            outsider = root / "outsider" / "pyproject.toml"
            app.parent.mkdir(parents=True)
            local.parent.mkdir()
            outsider.parent.mkdir()
            workspace.write_text(
                """
[project]
name = "workspace"
version = "1"
[tool.uv.workspace]
members = ["app", "local"]
[tool.uv.sources]
local-pkg = { workspace = true }
""",
                encoding="utf-8",
            )
            app.write_text(
                '[project]\nname = "app"\nversion = "1"\ndependencies = ["local-pkg"]\n',
                encoding="utf-8",
            )
            local.write_text(
                '[project]\nname = "local-pkg"\nversion = "1"\n',
                encoding="utf-8",
            )
            outsider.write_text(
                '[project]\nname = "outsider"\nversion = "1"\ndependencies = ["local-pkg"]\n',
                encoding="utf-8",
            )

            self.assertEqual(
                policy.build_python_inventory(root, [workspace, app, local, outsider]),
                {"local-pkg": ["outsider/pyproject.toml"]},
            )

    def test_untracked_or_outside_paths_are_not_trusted(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            app = root / "app" / "pyproject.toml"
            app.parent.mkdir()
            app.write_text(
                """
[project]
name = "app"
version = "1"
dependencies = ["vendored", "outside"]
[tool.uv.sources]
vendored = { path = "../untracked" }
outside = { path = "../../../outside" }
""",
                encoding="utf-8",
            )

            self.assertEqual(
                policy.build_python_inventory(root, [app]),
                {
                    "outside": ["app/pyproject.toml"],
                    "vendored": ["app/pyproject.toml"],
                },
            )

    def test_rejects_an_invalid_dependency_group_entry(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = root / "pyproject.toml"
            manifest.write_text(
                '[dependency-groups]\ntest = [{not-an-include = "lint"}]\n',
                encoding="utf-8",
            )

            with self.assertRaisesRegex(policy.PolicyError, "invalid include"):
                policy.build_python_inventory(root, [manifest])

    def test_rejects_requirement_text_after_the_distribution_name(self) -> None:
        with self.assertRaisesRegex(policy.PolicyError, "invalid requirement"):
            policy.python_requirement_name(
                "requests definitely-not-a-specifier", "test"
            )

    def test_conflicting_path_source_fields_do_not_earn_a_local_exemption(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            app = root / "app" / "pyproject.toml"
            local = root / "local" / "pyproject.toml"
            app.parent.mkdir()
            local.parent.mkdir()
            app.write_text(
                """
[project]
name = "app"
version = "1"
dependencies = ["local-pkg"]
[tool.uv.sources]
local-pkg = { path = "../local", git = "https://example.invalid/local" }
""",
                encoding="utf-8",
            )
            local.write_text(
                '[project]\nname = "local-pkg"\nversion = "1"\n',
                encoding="utf-8",
            )

            self.assertEqual(
                policy.build_python_inventory(root, [app, local]),
                {"local-pkg": ["app/pyproject.toml"]},
            )

    def test_sparse_inventory_discovers_an_absent_python_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            path = root / "nested" / "pyproject.toml"
            path.parent.mkdir()
            path.write_text(
                '[project]\nname = "nested"\nversion = "1"\ndependencies = ["Requests"]\n',
                encoding="utf-8",
            )
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            path.unlink()

            manifests = policy.git_python_manifests(root)
            inventory = policy.build_python_inventory(root, manifests)

            self.assertEqual(manifests, [path])
            self.assertEqual(inventory, {"requests": ["nested/pyproject.toml"]})


class DependencyPolicyTests(unittest.TestCase):
    def test_inventory_records_each_declaring_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            first = root / "one" / "Cargo.toml"
            second = root / "two" / "Cargo.toml"
            first.parent.mkdir()
            second.parent.mkdir()
            first.write_text('[dependencies]\nserde = "1"\n', encoding="utf-8")
            second.write_text(
                '[dev-dependencies]\nserde = "1"\ntempfile = "3"\n',
                encoding="utf-8",
            )

            inventory = policy.build_inventory(root, [second, first])

        self.assertEqual(
            inventory,
            {
                "serde": ["one/Cargo.toml", "two/Cargo.toml"],
                "tempfile": ["two/Cargo.toml"],
            },
        )

    def test_policy_rejects_both_new_and_stale_names(self) -> None:
        inventory = {"serde": ["code/a/Cargo.toml"], "toml": ["code/b/Cargo.toml"]}

        self.assertEqual(
            policy.policy_differences(inventory, ["old-crate", "serde"]),
            (["toml"], ["old-crate"]),
        )

    def test_base_allowlist_makes_growth_a_failure(self) -> None:
        baseline = ["serde"]
        current = ["serde", "toml"]

        self.assertEqual(sorted(set(current) - set(baseline)), ["toml"])

    def test_loads_the_trusted_allowlist_from_a_git_ref(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            relative_path = Path("policy/rust-allowlist.json")
            path = root / relative_path
            path.parent.mkdir()
            path.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "ecosystem": "cargo",
                        "allowed_dependency_names": ["serde"],
                    }
                ),
                encoding="utf-8",
            )
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(
                [
                    "git",
                    "-c",
                    "user.name=Dependency Policy Test",
                    "-c",
                    "user.email=dependency-policy@example.invalid",
                    "commit",
                    "-qm",
                    "baseline",
                ],
                cwd=root,
                check=True,
            )
            path.write_text("{}\n", encoding="utf-8")

            self.assertEqual(
                policy.load_base_allowlist(root, "HEAD", relative_path),
                ["serde"],
            )

    def test_command_rejects_inventory_and_allowlist_growth_together(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            manifest = root / "Cargo.toml"
            allowlist = root / "policy" / "rust-allowlist.json"
            allowlist.parent.mkdir()
            manifest.write_text('[dependencies]\nserde = "1"\n', encoding="utf-8")
            allowlist.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "ecosystem": "cargo",
                        "allowed_dependency_names": ["serde"],
                    }
                ),
                encoding="utf-8",
            )
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(
                [
                    "git",
                    "-c",
                    "user.name=Dependency Policy Test",
                    "-c",
                    "user.email=dependency-policy@example.invalid",
                    "commit",
                    "-qm",
                    "baseline",
                ],
                cwd=root,
                check=True,
            )
            manifest.write_text(
                '[dependencies]\nserde = "1"\ntoml = "1"\n', encoding="utf-8"
            )
            allowlist.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "ecosystem": "cargo",
                        "allowed_dependency_names": ["serde", "toml"],
                    }
                ),
                encoding="utf-8",
            )

            errors = io.StringIO()
            with contextlib.redirect_stderr(errors):
                result = policy.main(
                    [
                        "check",
                        "--root",
                        str(root),
                        "--allowlist",
                        str(allowlist),
                        "--base-ref",
                        "HEAD",
                    ]
                )

            self.assertEqual(result, 1)
            self.assertIn("approved dependency set may only shrink", errors.getvalue())
            self.assertIn("toml", errors.getvalue())

    def test_npm_command_rejects_allowlist_growth(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            manifest = root / "package.json"
            allowlist = root / "policy" / "npm-allowlist.json"
            allowlist.parent.mkdir()
            manifest.write_text(
                json.dumps(
                    {
                        "name": "app",
                        "devDependencies": {"typescript": "5"},
                    }
                ),
                encoding="utf-8",
            )
            allowlist.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "ecosystem": "npm",
                        "allowed_dependency_names": ["typescript"],
                    }
                ),
                encoding="utf-8",
            )
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(
                [
                    "git",
                    "-c",
                    "user.name=Dependency Policy Test",
                    "-c",
                    "user.email=dependency-policy@example.invalid",
                    "commit",
                    "-qm",
                    "baseline",
                ],
                cwd=root,
                check=True,
            )
            manifest.write_text(
                json.dumps(
                    {
                        "name": "app",
                        "devDependencies": {"typescript": "5", "vitest": "4"},
                    }
                ),
                encoding="utf-8",
            )
            allowlist.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "ecosystem": "npm",
                        "allowed_dependency_names": ["typescript", "vitest"],
                    }
                ),
                encoding="utf-8",
            )

            errors = io.StringIO()
            with contextlib.redirect_stderr(errors):
                result = policy.main(
                    [
                        "check",
                        "--ecosystem",
                        "npm",
                        "--root",
                        str(root),
                        "--allowlist",
                        str(allowlist),
                        "--base-ref",
                        "HEAD",
                    ]
                )

            self.assertEqual(result, 1)
            self.assertIn("approved dependency set may only shrink", errors.getvalue())
            self.assertIn("vitest", errors.getvalue())

    def test_allowlist_requires_sorted_unique_names(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "allowlist.json"
            path.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "ecosystem": "cargo",
                        "allowed_dependency_names": ["toml", "serde", "serde"],
                    }
                ),
                encoding="utf-8",
            )

            with self.assertRaisesRegex(policy.PolicyError, "sorted and unique"):
                policy.load_allowlist(path)

    def test_allowlist_requires_the_selected_ecosystem(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "allowlist.json"
            path.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "ecosystem": "cargo",
                        "allowed_dependency_names": ["serde"],
                    }
                ),
                encoding="utf-8",
            )

            with self.assertRaisesRegex(policy.PolicyError, "npm allowlist"):
                policy.load_allowlist(path, "npm")


if __name__ == "__main__":
    unittest.main()
