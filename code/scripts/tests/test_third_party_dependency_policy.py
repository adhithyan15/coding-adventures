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
                        "peerDependencies": {
                            "scoped-alias": "npm:@scope/actual@2"
                        },
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
