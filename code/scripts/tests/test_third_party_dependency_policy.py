from __future__ import annotations

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


if __name__ == "__main__":
    unittest.main()
