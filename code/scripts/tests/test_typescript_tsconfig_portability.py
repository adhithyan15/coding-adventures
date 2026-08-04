from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(SCRIPTS_DIR))

import typescript_tsconfig_portability as portability


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


class TypeScriptTsconfigPortabilityTests(unittest.TestCase):
    def make_project(
        self,
        root: Path,
        name: str,
        *,
        extends_shared: bool = True,
        compiler_options: dict[str, object] | None = None,
        config_fields: dict[str, object] | None = None,
        typescript_version: str = "5.7.3",
    ) -> Path:
        project = root / "code" / "packages" / "typescript" / name
        write_json(
            project / "package.json",
            {
                "name": f"@coding-adventures/{name}",
                "scripts": {"build": "tsc"},
            },
        )
        config: dict[str, object] = {}
        if extends_shared:
            config["extends"] = "../tsconfig.base.json"
        if compiler_options is not None:
            config["compilerOptions"] = compiler_options
        if config_fields is not None:
            config.update(config_fields)
        write_json(project / "tsconfig.json", config)
        write_json(
            project / "package-lock.json",
            {
                "lockfileVersion": 3,
                "packages": {
                    "": {"devDependencies": {"typescript": "^5.5.0"}},
                    "node_modules/typescript": {"version": typescript_version},
                },
            },
        )
        return project

    def make_shared_base(self, root: Path, root_dir: str, out_dir: str) -> None:
        write_json(
            root / "code" / "packages" / "typescript" / "tsconfig.base.json",
            {"compilerOptions": {"rootDir": root_dir, "outDir": out_dir}},
        )

    def test_rejects_relative_paths_in_extendable_base(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self.make_shared_base(root, "src", "dist")
            self.make_project(root, "inherits-paths")

            summary = portability.audit_repository(root)

            self.assertEqual(
                [issue.code for issue in summary.issues],
                ["SHARED_PATH_NOT_PORTABLE", "SHARED_PATH_NOT_PORTABLE"],
            )
            with self.assertRaises(portability.PortabilityError):
                portability.validate_repository(root)

    def test_accepts_config_dir_templates_and_counts_inheritors(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self.make_shared_base(root, "${configDir}/src", "${configDir}/dist")
            self.make_project(root, "inherits-paths")

            summary = portability.validate_repository(root)

            self.assertEqual(summary.total_projects, 1)
            self.assertEqual(summary.shared_projects, 1)
            self.assertEqual(summary.inherited_root_dir, 1)
            self.assertEqual(summary.inherited_out_dir, 1)
            self.assertEqual(summary.locked_compilers, 1)

    def test_explicit_child_paths_remain_authoritative(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self.make_shared_base(root, "${configDir}/src", "${configDir}/dist")
            self.make_project(
                root,
                "local-layout",
                compiler_options={"rootDir": ".", "outDir": "artifacts"},
            )

            summary = portability.validate_repository(root)

            self.assertEqual(summary.shared_projects, 1)
            self.assertEqual(summary.inherited_root_dir, 0)
            self.assertEqual(summary.inherited_out_dir, 0)

    def test_rejects_checked_in_compiler_older_than_config_dir(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self.make_shared_base(root, "${configDir}/src", "${configDir}/dist")
            self.make_project(root, "old-compiler", typescript_version="5.4.5")

            summary = portability.audit_repository(root)

            self.assertEqual(
                [issue.code for issue in summary.issues], ["TYPESCRIPT_TOO_OLD"]
            )

    def test_rejects_emit_capable_standalone_config_without_out_dir(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self.make_shared_base(root, "${configDir}/src", "${configDir}/dist")
            self.make_project(root, "standalone", extends_shared=False)

            summary = portability.audit_repository(root)

            self.assertEqual(summary.total_projects, 1)
            self.assertEqual(summary.shared_projects, 0)
            self.assertEqual(summary.inherited_root_dir, 0)
            self.assertEqual(summary.inherited_out_dir, 0)
            self.assertEqual(summary.standalone_emit_projects, 1)
            self.assertEqual(summary.isolated_standalone_projects, 0)
            self.assertEqual(
                [(issue.code, issue.path) for issue in summary.issues],
                [
                    (
                        "STANDALONE_OUTPUT_NOT_ISOLATED",
                        "code/packages/typescript/standalone/tsconfig.json",
                    )
                ],
            )

    def test_accepts_type_check_only_standalone_config(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self.make_shared_base(root, "${configDir}/src", "${configDir}/dist")
            self.make_project(
                root,
                "type-check-only",
                extends_shared=False,
                compiler_options={"noEmit": True},
            )

            summary = portability.validate_repository(root)

            self.assertEqual(summary.standalone_emit_projects, 0)
            self.assertEqual(summary.isolated_standalone_projects, 0)

    def test_accepts_isolated_standalone_output(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self.make_shared_base(root, "${configDir}/src", "${configDir}/dist")
            self.make_project(
                root,
                "isolated-output",
                extends_shared=False,
                compiler_options={"outDir": "dist"},
            )

            summary = portability.validate_repository(root)

            self.assertEqual(summary.standalone_emit_projects, 1)
            self.assertEqual(summary.isolated_standalone_projects, 1)

    def test_rejects_default_inputs_outside_effective_root(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self.make_shared_base(root, "${configDir}/src", "${configDir}/dist")
            project = self.make_project(root, "unbounded-inputs")
            (project / "src").mkdir()
            (project / "src" / "index.ts").write_text("export {};\n", encoding="utf-8")
            (project / "tests").mkdir()
            (project / "tests" / "index.test.ts").write_text(
                "export {};\n", encoding="utf-8"
            )

            summary = portability.audit_repository(root)

            self.assertEqual(
                [(issue.code, issue.path) for issue in summary.issues],
                [
                    (
                        "INPUT_BOUNDARY_MISSING",
                        "code/packages/typescript/unbounded-inputs/tsconfig.json",
                    )
                ],
            )
            self.assertEqual(summary.rooted_projects, 1)
            self.assertEqual(summary.bounded_root_projects, 0)
            self.assertEqual(summary.unbounded_root_projects, 1)
            self.assertEqual(summary.outside_root_inputs, 1)

    def test_accepts_source_include_with_tests_outside_root(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self.make_shared_base(root, "${configDir}/src", "${configDir}/dist")
            project = self.make_project(
                root,
                "bounded-inputs",
                config_fields={"include": ["src"]},
            )
            (project / "src").mkdir()
            (project / "src" / "index.ts").write_text("export {};\n", encoding="utf-8")
            (project / "tests").mkdir()
            (project / "tests" / "index.test.ts").write_text(
                "export {};\n", encoding="utf-8"
            )

            summary = portability.validate_repository(root)

            self.assertEqual(summary.rooted_projects, 1)
            self.assertEqual(summary.bounded_root_projects, 1)
            self.assertEqual(summary.unbounded_root_projects, 0)
            self.assertEqual(summary.outside_root_inputs, 0)

    def test_repository_contract_is_portable(self) -> None:
        summary = portability.validate_repository(REPO_ROOT)

        self.assertEqual(summary.total_projects, 458)
        self.assertEqual(summary.shared_projects, 287)
        self.assertEqual(summary.inherited_root_dir, 129)
        self.assertEqual(summary.inherited_out_dir, 132)
        self.assertEqual(summary.standalone_emit_projects, 143)
        self.assertEqual(summary.isolated_standalone_projects, 143)
        self.assertEqual(summary.unbounded_root_projects, 0)
        self.assertEqual(summary.outside_root_inputs, 0)
        self.assertEqual(summary.locked_compilers, 444)


if __name__ == "__main__":
    unittest.main()
