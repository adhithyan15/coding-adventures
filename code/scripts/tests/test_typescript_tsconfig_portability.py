from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(SCRIPTS_DIR))

import typescript_tsconfig_portability as portability  # noqa: E402


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
        dev_dependencies: dict[str, str] | None = None,
        locked_dev_dependencies: dict[str, str] | None = None,
        locked_node_version: str | None = None,
        typescript_version: str = "5.7.3",
    ) -> Path:
        project = root / "code" / "packages" / "typescript" / name
        manifest: dict[str, object] = {
            "name": f"@coding-adventures/{name}",
            "scripts": {"build": "tsc"},
        }
        if dev_dependencies is not None:
            manifest["devDependencies"] = dev_dependencies
        write_json(
            project / "package.json",
            manifest,
        )
        config: dict[str, object] = {}
        if extends_shared:
            config["extends"] = "../tsconfig.base.json"
        if compiler_options is not None:
            config["compilerOptions"] = compiler_options
        if config_fields is not None:
            config.update(config_fields)
        write_json(project / "tsconfig.json", config)
        lock_root_dependencies = {"typescript": "^5.5.0"}
        if locked_dev_dependencies is not None:
            lock_root_dependencies.update(locked_dev_dependencies)
        elif dev_dependencies is not None:
            lock_root_dependencies.update(dev_dependencies)
        lock_packages: dict[str, object] = {
            "": {"devDependencies": lock_root_dependencies},
            "node_modules/typescript": {"version": typescript_version},
        }
        if locked_node_version is not None:
            lock_packages["node_modules/@types/node"] = {
                "version": locked_node_version
            }
        write_json(
            project / "package-lock.json",
            {"lockfileVersion": 3, "packages": lock_packages},
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

    def test_rejects_node_apis_without_direct_dev_provider(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self.make_shared_base(root, "${configDir}/src", "${configDir}/dist")
            project = self.make_project(root, "node-user")
            (project / "src").mkdir()
            (project / "src" / "index.ts").write_text(
                """
export { readFile } from "node:fs";
const path = await import("path");
const url = require("url");
process.cwd();
Buffer.from("value");
const env: NodeJS.ProcessEnv = process.env;
console.log(__dirname, __filename, path, url, env);
""".lstrip(),
                encoding="utf-8",
            )

            summary = portability.audit_repository(root)

            self.assertEqual(
                [(issue.code, issue.path) for issue in summary.issues],
                [
                    (
                        "NODE_TYPES_NOT_OWNED",
                        "code/packages/typescript/node-user/package.json",
                    )
                ],
            )
            self.assertEqual(summary.node_api_projects, 1)
            self.assertEqual(summary.node_provider_projects, 0)
            self.assertEqual(summary.missing_node_provider_projects, 1)

    def test_accepts_direct_node_provider_with_synchronized_lock(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self.make_shared_base(root, "${configDir}/src", "${configDir}/dist")
            project = self.make_project(
                root,
                "owned-node-types",
                dev_dependencies={"@types/node": "^22.0.0"},
                locked_node_version="22.19.18",
            )
            (project / "src").mkdir()
            (project / "src" / "index.ts").write_text(
                'import { readFile } from "node:fs";\nexport { readFile };\n',
                encoding="utf-8",
            )

            summary = portability.validate_repository(root)

            self.assertEqual(summary.node_api_projects, 1)
            self.assertEqual(summary.node_provider_projects, 1)
            self.assertEqual(summary.missing_node_provider_projects, 0)
            self.assertEqual(summary.stale_node_provider_locks, 0)
            self.assertEqual(summary.node_lock_exemptions, 0)

    def test_accepts_the_reviewed_native_napi_lock_exception(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self.make_shared_base(root, "${configDir}/src", "${configDir}/dist")
            project = self.make_project(
                root,
                "matrix-rust-napi",
                dev_dependencies={"@types/node": "^20.0.0"},
            )
            (project / "package-lock.json").unlink()
            (project / ".gitignore").write_text(
                "package-lock.json\n", encoding="utf-8"
            )
            (project / "src").mkdir()
            (project / "src" / "index.ts").write_text(
                'import { createRequire } from "node:module";\n'
                "export { createRequire };\n",
                encoding="utf-8",
            )

            summary = portability.validate_repository(root)

            self.assertEqual(summary.node_api_projects, 1)
            self.assertEqual(summary.node_provider_projects, 1)
            self.assertEqual(summary.stale_node_provider_locks, 0)
            self.assertEqual(summary.node_lock_exemptions, 1)

    def test_ignores_node_words_in_comments_strings_and_properties(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self.make_shared_base(root, "${configDir}/src", "${configDir}/dist")
            project = self.make_project(root, "browser-only")
            (project / "src").mkdir()
            (project / "src" / "index.ts").write_text(
                """
// process.cwd(); import "node:fs";
const prose = 'require("path") and Buffer.from("value")';
const worker = { process() { return prose; }, Buffer: prose };
worker.process();
console.log(worker.Buffer);
""".lstrip(),
                encoding="utf-8",
            )

            summary = portability.validate_repository(root)

            self.assertEqual(summary.node_api_projects, 0)
            self.assertEqual(summary.missing_node_provider_projects, 0)

    def test_checks_template_expressions_but_ignores_template_prose(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self.make_shared_base(root, "${configDir}/src", "${configDir}/dist")
            project = self.make_project(root, "template-node-user")
            (project / "src").mkdir()
            (project / "src" / "index.ts").write_text(
                """
const prose = `process.cwd() and require(\"node:fs\")`;
const nested = `outer ${{ path: `${process.cwd()}` }.path}`;
console.log(prose, nested);
""".lstrip(),
                encoding="utf-8",
            )

            summary = portability.audit_repository(root)

            self.assertEqual(summary.node_api_projects, 1)
            self.assertEqual(summary.missing_node_provider_projects, 1)

    def test_rejects_node_provider_with_stale_lock_metadata(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self.make_shared_base(root, "${configDir}/src", "${configDir}/dist")
            project = self.make_project(
                root,
                "stale-node-lock",
                dev_dependencies={"@types/node": "^22.0.0"},
                locked_dev_dependencies={},
            )
            (project / "src").mkdir()
            (project / "src" / "index.ts").write_text(
                'import "node:fs";\n', encoding="utf-8"
            )

            summary = portability.audit_repository(root)

            self.assertEqual(
                [(issue.code, issue.path) for issue in summary.issues],
                [
                    (
                        "NODE_TYPES_LOCK_MISMATCH",
                        "code/packages/typescript/stale-node-lock/package-lock.json",
                    )
                ],
            )
            self.assertEqual(summary.node_api_projects, 1)
            self.assertEqual(summary.node_provider_projects, 1)
            self.assertEqual(summary.stale_node_provider_locks, 1)

    def test_repository_contract_is_portable(self) -> None:
        summary = portability.validate_repository(REPO_ROOT)

        # Project inventory: +1 for code/packages/typescript/script-ductus, the handwriting
        # modules extracted out of language-ladder so the book pipeline can
        # reach them. A new TypeScript project is a new row here by
        # construction; the count is the contract that says so out loud.
        # +1: path-raster, the scanline rasterizer P2D08 specifies.
        # +1: chief-of-staff-channel-crypto, the portable D18F message profile.
        # +1: chief-of-staff-channel-store, the durable D18P profile.
        # +1: chief-of-staff-channel-epoch-activation, the D18T coordinator.
        # +1: canonical-cbor, the native CBR01 encoder/decoder lane.
        # +1: forme-theme-classless, the reusable resolved Style IR theme.
        # +1: forme-resolve-asset-refs-fs, the source-safe asset reference lane.
        # +1: forme-load-assets-fs, the canonical-contained Asset IR loader.
        self.assertEqual(summary.total_projects, 469)
        self.assertEqual(summary.shared_projects, 291)
        self.assertEqual(summary.inherited_root_dir, 130)
        self.assertEqual(summary.inherited_out_dir, 133)
        self.assertEqual(summary.standalone_emit_projects, 148)
        self.assertEqual(summary.isolated_standalone_projects, 148)
        self.assertEqual(summary.unbounded_root_projects, 0)
        self.assertEqual(summary.outside_root_inputs, 0)
        # 94: +1 for script-ductus. Nothing the package SHIPS touches a Node
        # API -- it takes fonts as an ArrayBuffer and returns plain objects --
        # but its tests read the shipped .ttf files off disk to check authored
        # pen paths against the real glyph outlines, which is the whole point
        # of the package. Tests are compiler input, so the classification is
        # correct and `@types/node` is owned directly rather than inherited.
        # +1: chief-of-staff-channel-store reads the shared fixture in tests.
        # -31: the runtime-grammar-loading fix removed every readFileSync +
        # fileURLToPath/dirname/join(fs/path/url) disk-path lookup from 31
        # lexer/parser packages (algol, brainfuck, csharp, css-parser,
        # dartmouth-basic, dot-parser, haskell, java, javascript, json,
        # lisp, nib, python, ruby, starlark, toml-lexer, typescript, xml).
        # They now import a pre-compiled `_grammar.ts` module instead, so
        # they no longer touch any Node builtin API at all.
        # +1: forme-resolve-asset-refs-fs resolves paths and reads identity
        # sidecars through the Node filesystem API.
        # +1: forme-load-assets-fs resolves canonical paths and reads bytes.
        self.assertEqual(summary.node_api_projects, 67)
        # +1: script-ductus owns `@types/node` directly, because its tests
        # read the shipped fonts off disk to verify the pen paths.
        # +1: chief-of-staff-channel-store owns the test-only Node provider.
        # -31: see node_api_projects -- a project can only be a node
        # provider if it's a node API project, so the same 31 packages drop
        # out of both counts together.
        # +1: forme-resolve-asset-refs-fs owns its Node provider directly.
        # +1: forme-load-assets-fs owns its Node provider directly.
        self.assertEqual(summary.node_provider_projects, 67)
        self.assertEqual(summary.missing_node_provider_projects, 0)
        self.assertEqual(summary.stale_node_provider_locks, 0)
        self.assertEqual(summary.node_lock_exemptions, 1)
        # +1: script-ductus owns `@types/node` directly, because its tests
        # read the shipped fonts off disk to verify the pen paths.
        # +1: path-raster, the scanline rasterizer P2D08 specifies.
        # +1: chief-of-staff-channel-crypto locks its standalone compiler.
        # +1: chief-of-staff-channel-store locks its standalone compiler.
        # +1: chief-of-staff-channel-epoch-activation locks its compiler.
        # +3: json-serializer, json-value, and starlark-ast-to-bytecode-
        # compiler (downstream consumers exercised while verifying the
        # runtime-grammar-loading fix) had no package-lock.json committed
        # before; running `npm install` to test them produced one.
        # +1: canonical-cbor locks the shared TypeScript compiler toolchain.
        # +1: forme-theme-classless locks its standalone compiler toolchain.
        # +1: forme-resolve-asset-refs-fs locks its compiler toolchain.
        # +1: forme-load-assets-fs locks its compiler toolchain.
        self.assertEqual(summary.locked_compilers, 463)


if __name__ == "__main__":
    unittest.main()
