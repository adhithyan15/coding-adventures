from __future__ import annotations

import contextlib
import io
import json
import subprocess
import sys
import tempfile
import unittest
from collections.abc import Mapping
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
    LOCAL_NAME = "@coding-adventures/rank-statistics"

    def write_locked_sibling_fixture(
        self,
        root: Path,
        *,
        alias: str | None = None,
        specifier: str = "0.1.0",
        dependency_table: str = "dependencies",
        target_name: str | None = None,
        target_version: str = "0.1.0",
        target_relative: str = "packages/rank-statistics",
        lock_root_name: str = "@coding-adventures/microbenchmark",
        lock_root_version: str = "0.1.0",
        lock_dependency: str | None = None,
        lock_target_name: str | None = None,
        lock_target_version: str | None = None,
        lockfile_version: object = 3,
        include_lockfile_version: bool = True,
        resolved: str = "../rank-statistics",
        link: bool = True,
        lock_tracked: bool = True,
        target_tracked: bool = True,
        extra_link_fields: Mapping[str, object] | None = None,
        symlink_escape: Path | None = None,
    ) -> Path:
        """Create an indexed package pair and a versioned sibling lock link."""

        dependency_name = alias or self.LOCAL_NAME
        local_target_name = target_name or self.LOCAL_NAME
        local_lock_target_name = lock_target_name or local_target_name
        local_lock_target_version = lock_target_version or target_version
        app = root / "packages" / "microbenchmark"
        target = root / target_relative
        app.mkdir(parents=True)
        target.mkdir(parents=True)

        if symlink_escape is not None:
            (root / "escape").symlink_to(symlink_escape, target_is_directory=True)
            resolved = "../../escape"

        app_document: dict[str, object] = {
            "name": "@coding-adventures/microbenchmark",
            "version": "0.1.0",
            dependency_table: {dependency_name: specifier, "typescript": "5.0.0"},
        }
        (app / "package.json").write_text(json.dumps(app_document), encoding="utf-8")
        (target / "package.json").write_text(
            json.dumps({"name": local_target_name, "version": target_version}),
            encoding="utf-8",
        )
        root_dependencies = {
            dependency_name: specifier if lock_dependency is None else lock_dependency,
            "typescript": "5.0.0",
        }
        link_entry: dict[str, object] = {"resolved": resolved, "link": link}
        if extra_link_fields:
            link_entry.update(extra_link_fields)
        lock: dict[str, object] = {
            "name": "@coding-adventures/microbenchmark",
            "version": "0.1.0",
            "requires": True,
            "packages": {
                "": {
                    "name": lock_root_name,
                    "version": lock_root_version,
                    dependency_table: root_dependencies,
                },
                f"node_modules/{dependency_name}": link_entry,
                resolved: {
                    "name": local_lock_target_name,
                    "version": local_lock_target_version,
                },
            },
        }
        if include_lockfile_version:
            lock["lockfileVersion"] = lockfile_version
        (app / "package-lock.json").write_text(json.dumps(lock), encoding="utf-8")

        subprocess.run(["git", "init", "-q"], cwd=root, check=True)
        tracked = ["packages/microbenchmark/package.json"]
        if lock_tracked:
            tracked.append("packages/microbenchmark/package-lock.json")
        if target_tracked:
            tracked.append((target / "package.json").relative_to(root).as_posix())
        subprocess.run(["git", "add", *tracked], cwd=root, check=True)
        return app / "package.json"

    def inventory_for_root(self, root: Path) -> dict[str, list[str]]:
        return policy.build_npm_inventory(root, policy.git_npm_manifests(root))

    def index_as_symlink(self, root: Path, path: Path) -> None:
        target_blob = (
            subprocess.run(
                ["git", "hash-object", "-w", "--stdin"],
                cwd=root,
                input=b"../outside-target",
                check=True,
                capture_output=True,
            )
            .stdout.decode()
            .strip()
        )
        subprocess.run(
            [
                "git",
                "update-index",
                "--add",
                "--cacheinfo",
                "120000",
                target_blob,
                path.relative_to(root).as_posix(),
            ],
            cwd=root,
            check=True,
        )

    def index_as_conflict(self, root: Path, path: Path) -> None:
        blob = (
            subprocess.run(
                ["git", "hash-object", str(path)],
                cwd=root,
                check=True,
                capture_output=True,
            )
            .stdout.decode()
            .strip()
        )
        subprocess.run(
            [
                "git",
                "update-index",
                "--force-remove",
                path.relative_to(root).as_posix(),
            ],
            cwd=root,
            check=True,
        )
        records = b"".join(
            f"100644 {blob} {stage}\t{path.relative_to(root).as_posix()}\n".encode()
            for stage in (1, 2, 3)
        )
        subprocess.run(
            ["git", "update-index", "--index-info"],
            cwd=root,
            input=records,
            check=True,
        )

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

    def test_verified_exact_sibling_lock_link_is_local_only(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.write_locked_sibling_fixture(root)

            inventory = self.inventory_for_root(root)

            self.assertEqual(
                inventory,
                {"typescript": ["packages/microbenchmark/package.json"]},
            )

    def test_ranges_aliases_and_wrong_identity_or_version_stay_external(self) -> None:
        cases = [
            {"specifier": "^0.1.0"},
            {"specifier": "0.1.0-01"},
            {"specifier": "0.1.0+build"},
            {"specifier": "npm:@coding-adventures/rank-statistics@0.1.0"},
            {"dependency_table": "devDependencies"},
            {"dependency_table": "peerDependencies"},
            {"dependency_table": "optionalDependencies"},
            {"lockfile_version": 1},
            {"lockfile_version": 2},
            {"lockfile_version": 99},
            {"lockfile_version": "3"},
            {"lockfile_version": True},
            {"include_lockfile_version": False},
            {"target_name": "@coding-adventures/other"},
            {"target_version": "0.1.1"},
            {"lock_target_name": "@coding-adventures/other"},
            {"lock_target_version": "0.1.1"},
            {"lock_root_name": "@coding-adventures/other"},
            {"lock_root_version": "0.1.1"},
            {"lock_dependency": "^0.1.0"},
            {"alias": "rank-statistics-alias"},
            {
                "target_relative": "elsewhere/rank-statistics",
                "resolved": "../../elsewhere/rank-statistics",
            },
        ]
        for changes in cases:
            with (
                self.subTest(changes=changes),
                tempfile.TemporaryDirectory() as directory,
            ):
                root = Path(directory)
                self.write_locked_sibling_fixture(root, **changes)

                inventory = self.inventory_for_root(root)

                dependency_name = changes.get("alias", self.LOCAL_NAME)
                specifier = changes.get("specifier", "0.1.0")
                self.assertIn(
                    policy.npm_alias_target(specifier) or dependency_name,
                    inventory,
                )
                self.assertIn("typescript", inventory)

    def test_missing_untracked_remote_and_malformed_links_stay_external(self) -> None:
        cases = [
            {"lock_tracked": False},
            {"target_tracked": False},
            {"link": False},
            {
                "resolved": "https://registry.npmjs.org/rank-statistics/-/rank-statistics-0.1.0.tgz"
            },
            {"resolved": "../rank-statistics/.."},
            {"resolved": "../../../../outside-package"},
            {"extra_link_fields": {"integrity": "sha512-not-a-link-field"}},
        ]
        for changes in cases:
            with (
                self.subTest(changes=changes),
                tempfile.TemporaryDirectory() as directory,
            ):
                root = Path(directory)
                self.write_locked_sibling_fixture(root, **changes)

                inventory = self.inventory_for_root(root)

                self.assertIn(self.LOCAL_NAME, inventory)

    def test_shrinkwrap_disables_package_lock_evidence_when_tracked_or_present(
        self,
    ) -> None:
        for state in ("tracked", "sparse", "untracked"):
            with (
                self.subTest(state=state),
                tempfile.TemporaryDirectory() as directory,
            ):
                root = Path(directory)
                manifest = self.write_locked_sibling_fixture(root)
                shrinkwrap = manifest.with_name("npm-shrinkwrap.json")
                shrinkwrap.write_text("{}\n", encoding="utf-8")
                if state != "untracked":
                    subprocess.run(
                        ["git", "add", shrinkwrap.relative_to(root).as_posix()],
                        cwd=root,
                        check=True,
                    )
                if state == "sparse":
                    shrinkwrap.unlink()

                inventory = self.inventory_for_root(root)

                self.assertIn(self.LOCAL_NAME, inventory)

    def test_untracked_dangling_shrinkwrap_symlink_disables_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = self.write_locked_sibling_fixture(root)
            shrinkwrap = manifest.with_name("npm-shrinkwrap.json")
            try:
                shrinkwrap.symlink_to("missing-shrinkwrap.json")
            except OSError as error:
                self.skipTest(f"file symlink unavailable: {error}")

            inventory = self.inventory_for_root(root)

            self.assertIn(self.LOCAL_NAME, inventory)

    def test_symlink_package_lock_is_not_link_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = self.write_locked_sibling_fixture(root)
            lock = manifest.with_name("package-lock.json")
            lock_target = manifest.with_name("lock-target.json")
            lock.replace(lock_target)
            try:
                lock.symlink_to(lock_target.name)
            except OSError as error:
                self.skipTest(f"file symlink unavailable: {error}")
            subprocess.run(
                ["git", "add", "-A"], cwd=root, check=True, capture_output=True
            )

            inventory = self.inventory_for_root(root)

            self.assertIn(self.LOCAL_NAME, inventory)

    def test_lock_link_cannot_escape_repository_through_a_symlink(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            outer = Path(temporary)
            root = outer / "repo"
            outside = outer / "outside"
            root.mkdir()
            outside.mkdir()
            (outside / "package.json").write_text(
                json.dumps({"name": self.LOCAL_NAME, "version": "0.1.0"}),
                encoding="utf-8",
            )
            try:
                self.write_locked_sibling_fixture(
                    root, symlink_escape=outside, target_tracked=False
                )
            except OSError as error:
                self.skipTest(f"directory symlink unavailable: {error}")

            inventory = self.inventory_for_root(root)

            self.assertIn(self.LOCAL_NAME, inventory)

    def test_duplicate_keys_in_package_lock_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = self.write_locked_sibling_fixture(root)
            lock_path = manifest.with_name("package-lock.json")
            lock_path.write_text(
                lock_path.read_text(encoding="utf-8").replace(
                    '"link": true', '"link": true, "link": false'
                ),
                encoding="utf-8",
            )

            with self.assertRaisesRegex(policy.PolicyError, "duplicate JSON key"):
                self.inventory_for_root(root)

    def test_symlink_index_modes_do_not_earn_local_exemption(self) -> None:
        for symlink_path in ("lock", "target", "manifest"):
            with (
                self.subTest(symlink_path=symlink_path),
                tempfile.TemporaryDirectory() as directory,
            ):
                root = Path(directory)
                manifest = self.write_locked_sibling_fixture(root)
                indexed_path = (
                    manifest.with_name("package-lock.json")
                    if symlink_path == "lock"
                    else (
                        root / "packages" / "rank-statistics" / "package.json"
                        if symlink_path == "target"
                        else manifest
                    )
                )
                self.index_as_symlink(root, indexed_path)

                inventory = self.inventory_for_root(root)

                self.assertIn(self.LOCAL_NAME, inventory)

    def test_unmerged_index_stages_do_not_earn_local_exemption(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = self.write_locked_sibling_fixture(root)
            self.index_as_conflict(root, manifest.with_name("package-lock.json"))

            inventory = self.inventory_for_root(root)

            self.assertIn(self.LOCAL_NAME, inventory)


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
python3 -m pip install --quiet jsonschema==4.26.0 fonttools==4.68.0
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


class GoManifestTests(unittest.TestCase):
    def test_collects_external_and_verified_local_modules(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            app = root / "app" / "go.mod"
            local = root / "local" / "go.mod"
            app.parent.mkdir()
            local.parent.mkdir()
            app.write_text(
                """module example.com/app

go 1.25

require (
    example.com/local v0.0.0
    golang.org/x/sys v0.1.0
    example.com/old v1.0.0
)

replace example.com/local => ../local
replace example.com/old v1.0.0 => example.com/new v1.1.0
""",
                encoding="utf-8",
            )
            local.write_text("module example.com/local\n\ngo 1.25\n", encoding="utf-8")

            self.assertEqual(
                policy.build_go_inventory(root, [app, local]),
                {
                    "example.com/new": ["app/go.mod"],
                    "golang.org/x/sys": ["app/go.mod"],
                },
            )

    def test_untracked_outside_and_mismatched_replacements_are_not_trusted(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            app = root / "app" / "go.mod"
            local = root / "local" / "go.mod"
            app.parent.mkdir()
            local.parent.mkdir()
            app.write_text(
                """module example.com/app
require (
    example.com/untracked v1.0.0
    example.com/outside v1.0.0
    example.com/wrong v1.0.0
)
replace example.com/untracked => ../untracked
replace example.com/outside => ../../../outside
replace example.com/wrong => ../local
""",
                encoding="utf-8",
            )
            local.write_text("module example.com/local\n", encoding="utf-8")

            self.assertEqual(
                policy.build_go_inventory(root, [app, local]),
                {
                    "example.com/outside": ["app/go.mod"],
                    "example.com/untracked": ["app/go.mod"],
                    "example.com/wrong": ["app/go.mod"],
                },
            )

    def test_rejects_malformed_go_mod_dependency_syntax(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = root / "go.mod"
            manifest.write_text(
                "module example.com/app\nrequire example.com/broken\n",
                encoding="utf-8",
            )

            with self.assertRaisesRegex(policy.PolicyError, "invalid require"):
                policy.build_go_inventory(root, [manifest])

    def test_sparse_inventory_discovers_absent_go_mod(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            manifest = root / "nested" / "go.mod"
            manifest.parent.mkdir()
            manifest.write_text(
                "module example.com/app\nrequire golang.org/x/text v0.1.0\n",
                encoding="utf-8",
            )
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            manifest.unlink()

            manifests = policy.git_go_manifests(root)
            inventory = policy.build_go_inventory(root, manifests)

            self.assertEqual(manifests, [manifest])
            self.assertEqual(inventory, {"golang.org/x/text": ["nested/go.mod"]})


class GoToolTests(unittest.TestCase):
    def test_collects_remote_go_acquisition_and_ignores_local_targets(self) -> None:
        self.assertEqual(
            policy.go_tool_names(
                "go install golang.org/x/tools/cmd/stringer@v0.1.0 && "
                "go get example.com/tool@latest; "
                "go run github.com/example/generator@v2.0.0 --flag; "
                "go run ./cmd/local; go run main.go"
            ),
            {
                "example.com/tool",
                "github.com/example/generator",
                "golang.org/x/tools/cmd/stringer",
            },
        )

    def test_rejects_dynamic_go_tool_targets(self) -> None:
        with self.assertRaisesRegex(policy.PolicyError, "dynamic Go tool target"):
            policy.go_tool_names("go install $TOOL")

    def test_only_scans_go_generate_directives_in_go_sources(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "generate.go"
            source.write_text(
                'package demo\nconst example = "go install ignored.example/tool@v1"\n'
                "//go:generate go run example.com/generator@v1\n",
                encoding="utf-8",
            )

            self.assertEqual(
                policy.build_go_tool_inventory(root, [source], []),
                {"example.com/generator": ["generate.go"]},
            )


class ElixirManifestTests(unittest.TestCase):
    def test_collects_external_and_verified_local_mix_dependencies(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            app = root / "app" / "mix.exs"
            local = root / "local" / "mix.exs"
            app.parent.mkdir()
            local.parent.mkdir()
            app.write_text(
                """defmodule App.MixProject do
  use Mix.Project
  def project, do: [app: :app, deps: deps()]
  defp deps do
    [
      {:local_app,
       path: "../local"},
      {:jason, "~> 1.4"}
    ]
  end
end
""",
                encoding="utf-8",
            )
            local.write_text(
                "defmodule Local.MixProject do\n"
                "  use Mix.Project\n"
                "  def project, do: [app: :local_app]\n"
                "end\n",
                encoding="utf-8",
            )

            self.assertEqual(
                policy.build_elixir_inventory(root, [app, local]),
                {"jason": ["app/mix.exs"]},
            )

    def test_untracked_outside_and_mismatched_mix_paths_are_not_trusted(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            app = root / "app" / "mix.exs"
            local = root / "local" / "mix.exs"
            app.parent.mkdir()
            local.parent.mkdir()
            app.write_text(
                """defmodule App.MixProject do
  use Mix.Project
  def project, do: [app: :app]
  defp deps, do: [
    {:untracked, path: "../untracked"},
    {:outside, path: "../../../outside"},
    {:wrong_name, path: "../local"}
  ]
end
""",
                encoding="utf-8",
            )
            local.write_text(
                "defmodule Local.MixProject do\n"
                "  use Mix.Project\n"
                "  def project, do: [app: :local_app]\n"
                "end\n",
                encoding="utf-8",
            )

            self.assertEqual(
                policy.build_elixir_inventory(root, [app, local]),
                {
                    "outside": ["app/mix.exs"],
                    "untracked": ["app/mix.exs"],
                    "wrong_name": ["app/mix.exs"],
                },
            )

    def test_rejects_dynamic_mix_path_declarations(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = root / "mix.exs"
            manifest.write_text(
                "def project, do: [app: :app]\n"
                "defp deps, do: [{:local, path: local_path()}]\n",
                encoding="utf-8",
            )

            with self.assertRaisesRegex(policy.PolicyError, "unsupported Mix path"):
                policy.build_elixir_inventory(root, [manifest])

    def test_sparse_inventory_discovers_absent_mix_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            manifest = root / "nested" / "mix.exs"
            manifest.parent.mkdir()
            manifest.write_text(
                'def project, do: [app: :demo]\ndefp deps, do: [{:jason, "~> 1.4"}]\n',
                encoding="utf-8",
            )
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            manifest.unlink()

            manifests = policy.git_elixir_manifests(root)
            inventory = policy.build_elixir_inventory(root, manifests)

            self.assertEqual(manifests, [manifest])
            self.assertEqual(inventory, {"jason": ["nested/mix.exs"]})


class ElixirToolTests(unittest.TestCase):
    def test_collects_named_mix_installers(self) -> None:
        self.assertEqual(
            policy.mix_tool_names(
                "mix archive.install hex phx_new --force && "
                "mix escript.install dialyxir; "
                "mix local.hex --force; mix local.rebar --force"
            ),
            {"dialyxir", "hex", "phx_new", "rebar3"},
        )

    def test_rejects_unnamed_remote_mix_installers(self) -> None:
        with self.assertRaisesRegex(policy.PolicyError, "named Hex package"):
            policy.mix_tool_names("mix archive.install github owner/project")


class RubyManifestTests(unittest.TestCase):
    def test_collects_gemfile_and_gemspec_dependencies(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            app = root / "app"
            local = root / "local"
            app.mkdir()
            local.mkdir()
            gemfile = app / "Gemfile"
            app_spec = app / "app.gemspec"
            local_spec = local / "local.gemspec"
            gemfile.write_text(
                'gem "local-gem", path: "../local"\ngem "standard", require: false\n',
                encoding="utf-8",
            )
            app_spec.write_text(
                "Gem::Specification.new do |spec|\n"
                '  spec.name = "app"\n'
                '  spec.add_dependency "local-gem"\n'
                '  spec.add_development_dependency "minitest", "~> 5"\n'
                "end\n",
                encoding="utf-8",
            )
            local_spec.write_text(
                "Gem::Specification.new do |spec|\n"
                '  spec.name = "local-gem"\n'
                '  spec.add_runtime_dependency "json"\n'
                "end\n",
                encoding="utf-8",
            )

            self.assertEqual(
                policy.build_ruby_inventory(root, [gemfile, app_spec, local_spec]),
                {
                    "json": ["local/local.gemspec"],
                    "minitest": ["app/app.gemspec"],
                    "standard": ["app/Gemfile"],
                },
            )

    def test_untracked_outside_and_name_mismatched_paths_are_not_trusted(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            app = root / "app"
            local = root / "local"
            app.mkdir()
            local.mkdir()
            gemfile = app / "Gemfile"
            local_spec = local / "local.gemspec"
            gemfile.write_text(
                'gem "untracked", path: "../untracked"\n'
                'gem "outside", path: "../../../outside"\n'
                'gem "wrong-name", path: "../local"\n',
                encoding="utf-8",
            )
            local_spec.write_text('spec.name = "local-gem"\n', encoding="utf-8")

            self.assertEqual(
                policy.build_ruby_inventory(root, [gemfile, local_spec]),
                {
                    "outside": ["app/Gemfile"],
                    "untracked": ["app/Gemfile"],
                    "wrong-name": ["app/Gemfile"],
                },
            )

    def test_rejects_dynamic_dependency_declarations(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            gemfile = root / "Gemfile"
            gemfile.write_text("gem dependency_name\n", encoding="utf-8")

            with self.assertRaisesRegex(
                policy.PolicyError, "unsupported Ruby dependency declaration"
            ):
                policy.build_ruby_inventory(root, [gemfile])

    def test_rejects_dynamic_path_options(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            gemfile = root / "Gemfile"
            gemfile.write_text('gem "local", path: local_path\n', encoding="utf-8")

            with self.assertRaisesRegex(policy.PolicyError, "unsupported gem path"):
                policy.build_ruby_inventory(root, [gemfile])

    def test_sparse_inventory_discovers_absent_ruby_manifests(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            gemfile = root / "nested" / "Gemfile"
            gemspec = root / "nested" / "nested.gemspec"
            gemfile.parent.mkdir()
            gemfile.write_text('gem "rake"\n', encoding="utf-8")
            gemspec.write_text(
                'spec.name = "nested"\nspec.add_dependency "json"\n',
                encoding="utf-8",
            )
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            gemfile.unlink()
            gemspec.unlink()

            manifests = policy.git_ruby_manifests(root)
            inventory = policy.build_ruby_inventory(root, manifests)

            self.assertEqual(manifests, [gemfile, gemspec])
            self.assertEqual(
                inventory,
                {
                    "json": ["nested/nested.gemspec"],
                    "rake": ["nested/Gemfile"],
                },
            )


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
