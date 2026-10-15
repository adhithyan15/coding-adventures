"""Portable controls; mocked tools do not establish macOS link acceptance."""
from __future__ import annotations

import json
import shlex
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import macos_haskell_toolchain as toolchain

CLANG = "/Applications/Xcode Test.app/usr/bin/clang"
LD = "/Applications/Xcode Test.app/usr/bin/ld"
SDK = "/Applications/Xcode Test.app/SDKs/MacOSX.sdk"


class MacosHaskellToolchainTests(unittest.TestCase):
    def test_config_precedence_and_legacy(self):
        with tempfile.TemporaryDirectory() as temp:
            home = Path(temp)
            xdg = home / ".config/cabal/config"
            legacy = home / ".cabal"
            legacy.mkdir()
            self.assertEqual(legacy / "config", toolchain.locate_config({}, home))
            xdg.parent.mkdir(parents=True)
            xdg.write_text("", encoding="utf-8")
            self.assertEqual(xdg, toolchain.locate_config({}, home))
            custom = home / "xdg/cabal/config"
            self.assertEqual(legacy / "config", toolchain.locate_config({"XDG_CONFIG_HOME": str(home / "xdg")}, home))
            custom.parent.mkdir(parents=True)
            custom.write_text("", encoding="utf-8")
            for env, expected in [({"CABAL_CONFIG": str(home / "explicit")}, home / "explicit"),
                                  ({"CABAL_DIR": str(home / "dir")}, home / "dir/config"),
                                  ({"XDG_CONFIG_HOME": str(home / "xdg")}, home / "xdg/cabal/config")]:
                self.assertEqual(expected, toolchain.locate_config(env, home))
            self.assertEqual(home / "explicit", toolchain.locate_config({"CABAL_CONFIG": str(home / "explicit"), "CABAL_DIR": str(home / "dir")}, home))

    def test_phase_options_and_quoted_paths(self):
        expected = ["-pgmc", CLANG, "-pgma", CLANG, "-pgml", CLANG,
                    "-optc-isysroot", "-optc" + SDK, "-opta-isysroot", "-opta" + SDK,
                    "-optl-isysroot", "-optl" + SDK, "-optl-fuse-ld=" + LD]
        self.assertEqual(expected, toolchain.phase_options(CLANG, SDK, LD))
        actual = toolchain.overlay_config("repository private\n  secure: True\nstore-dir: /store\n", CLANG, LD, SDK)
        self.assertTrue(actual.startswith("repository private\n  secure: True\nstore-dir: /store\n"))
        self.assertIn("ghc-options: " + " ".join(json.dumps(v) for v in expected), actual)
        for value in ["--cc=" + CLANG, "--ld=" + CLANG, "--cflag=-isysroot", "--cflag=" + SDK, "--lflag=-isysroot", "--lflag=" + SDK]:
            self.assertIn(json.dumps(value), actual)
        self.assertIn("gcc-location: " + json.dumps(CLANG), actual)
        self.assertIn("ld-location: " + json.dumps(LD), actual)

    def test_existing_sections_preserve_all_unmanaged_lines(self):
        source = "-- header\nprogram-default-options\n  happy-options: -a\n  -- ghc-options: ignored\nrepository public\n  secure: True\nprogram-locations\n  alex-location: /keep/alex\n"
        actual = toolchain.overlay_config(source, CLANG, LD, SDK)
        cursor = 0
        for line in source.splitlines(keepends=True):
            found = actual.find(line, cursor)
            self.assertGreaterEqual(found, cursor)
            cursor = found + len(line)
        self.assertEqual(1, actual.count("\nprogram-default-options\n"))
        self.assertEqual(1, actual.count("\nprogram-locations\n"))

    def test_collision_duplicate_and_path_refusals(self):
        for section, field in [("program-default-options", "ghc-options"), ("program-default-options", "hsc2hs-options"),
                               ("program-locations", "gcc-location"), ("program-locations", "ld-location")]:
            with self.subTest(section=section, field=field):
                with self.assertRaises(ValueError):
                    toolchain.overlay_config(f"{section}\n  {field}: original\n", CLANG, LD, SDK)
        for section in ["program-default-options", "program-locations"]:
            with self.assertRaises(ValueError):
                toolchain.overlay_config(section + "\n" + section + "\n", CLANG, LD, SDK)
        for path in ["relative/tool", "", "/bad\npath", "/bad\rpath", "/bad\0path", "/bad\tpath"]:
            with self.subTest(path=path):
                with self.assertRaises(ValueError):
                    toolchain.phase_options(path, SDK, LD)

    def test_normalized_managed_names_and_unsupported_section_syntax(self):
        for source in [
            "program-default-options\n  GHC-OPTIONS: -pgmc /custom/clang\n",
            "program-default-options -- retained header comment\n  GHC-OPTIONS: -pgmc /custom/clang\n",
            "PROGRAM-LOCATIONS -- retained header comment\n  GCC-LOCATION: /custom/clang\n",
            "program-locations -- first\nPROGRAM-LOCATIONS -- second\n",
            "PROGRAM-LOCATIONS\n  GCC-LOCATION: /custom/clang\n",
            "program-locations\nPROGRAM-LOCATIONS\n",
            "program-default-options { ghc-options: -v }\n",
            "  program-locations\n    gcc-location: /custom/clang\n",
            "program-default-options\n  { happy-options: -a }\n",
        ]:
            with self.subTest(source=source):
                with self.assertRaises(ValueError):
                    toolchain.overlay_config(source, CLANG, LD, SDK)

    def run_mock_bootstrap(self, failed_stage=None, wrong_output=False, wrong_stage=None, ignore_refusal=False, omit_hsc_trace=False, bad_hsc_trace=False, wrong_refusal_cause=False):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        config = root / "original.config"
        original = "repository hackage.haskell.org\n  url: https://hackage.haskell.org/\n  secure: True\n"
        config.write_text(original, encoding="utf-8")
        github_env = root / "github-env"
        github_env.write_text("EXISTING=retained\n", encoding="utf-8")
        commands = []
        def runner(command, **kwargs):
            commands.append((command, kwargs))
            stage = len(commands)
            if stage == failed_stage:
                return subprocess.CompletedProcess(command, 7, "", "intentional failure")
            child_config = Path(kwargs["env"].get("CABAL_CONFIG", ""))
            if "v2-build" in command and child_config.name.startswith("refuse-"):
                missing = child_config.parent / ("missing-" + child_config.stem.removeprefix("refuse-"))
                if ignore_refusal:
                    return subprocess.CompletedProcess(command, 0, "42\n", "")
                if wrong_refusal_cause:
                    failed = str(missing) + "-other" if wrong_refusal_cause == "prefix" else "/unexpected/tool"
                    return subprocess.CompletedProcess(command, 1, "flags contain " + str(missing), "ghc: could not execute: " + failed)
                return subprocess.CompletedProcess(command, 1, "", "ghc: could not execute: " + str(missing))
            if "v2-build" in command and not omit_hsc_trace:
                route = json.loads((root / "output/hsc2hs-driver.json").read_text(encoding="utf-8"))
                effective = [route["real_hsc2hs"], "Main.hsc", *route["forced"]]
                if bad_hsc_trace:
                    effective.append("--cc=/unexpected/clang")
                (root / "output/hsc2hs-invocations.jsonl").write_text(json.dumps({"argv": effective}) + "\n", encoding="utf-8")
            if command[:1] == ["/usr/bin/xcrun"]:
                result = SDK if "--show-sdk-path" in command else (CLANG if command[-1] == "clang" else LD)
            elif "--numeric-version" in command:
                result = "9.4.8" if "ghc" in command[0] else "3.10.3.0"
            elif "--version" in command:
                result = "Apple clang selected"
            elif command[-1:] == ["exe:ci-toolchain-probe"] and "v2-run" in command:
                result = "wrong" if wrong_output else "42"
            elif len(command) == 1:
                result = "42"
            else:
                result = ""
            if stage == wrong_stage:
                result = "unexpected"
            return subprocess.CompletedProcess(command, 0, result + "\n", "")
        env = {"CABAL_CONFIG": str(config), "PATH": "/tools", "UNCHANGED": "retained"}
        with mock.patch.object(toolchain.shutil, "which", side_effect=lambda name, **_: "/tools/" + name):
            try:
                receipt = toolchain.bootstrap(root / "output", github_env, env, runner=runner)
            except RuntimeError:
                receipt = None
        self.assertEqual(original, config.read_text(encoding="utf-8"))
        self.assertEqual({"CABAL_CONFIG": str(config), "PATH": "/tools", "UNCHANGED": "retained"}, env)
        return root, github_env, receipt, commands

    def test_competing_compiler_identity_refused_and_selected_path_pinned(self):
        for source in [
            "with-compiler: /other/ghc\n",
            "WITH-COMPILER: /other/ghc -- competing\n",
            "  with-compiler: /other/ghc\n",
            "{ with-compiler: /other/ghc }\n",
            "program-locations\n  ghc-location: /other/ghc\n",
            "PROGRAM-LOCATIONS -- retained comment\n  GHC-LOCATION: /other/ghc\n",
            "compiler: ghcjs\n",
            "COMPILER: ghc\n",
            "{ compiler: ghcjs }\n",
            "with-hc-pkg: /other/ghc-pkg\n",
            "WITH-HC-PKG: /other/ghc-pkg\n",
            "program-locations\n  ghc-pkg-location: /other/ghc-pkg\n",
            "PROGRAM-LOCATIONS\n  GHC-PKG-LOCATION: /other/ghc-pkg\n",
        ]:
            with self.subTest(source=source):
                with self.assertRaises(ValueError):
                    toolchain.overlay_config(source, CLANG, LD, SDK)
        root, _, receipt, commands = self.run_mock_bootstrap()
        self.assertTrue(receipt["success"])
        for _, options in commands:
            config = Path(options["env"].get("CABAL_CONFIG", ""))
            if config.parent == root / "output":
                self.assertIn('  ghc-location: "/tools/ghc"', config.read_text(encoding="utf-8").splitlines())
                self.assertIn('with-compiler: /tools/ghc', config.read_text(encoding="utf-8").splitlines())

    def test_success_publishes_only_cabal_config_after_all_probes(self):
        root, github_env, receipt, commands = self.run_mock_bootstrap()
        self.assertTrue(receipt["success"])
        self.assertEqual(16, len(commands))
        self.assertIn("  main-is: Main.hs", (root / "output/cabal-probe/ci-toolchain-probe.cabal").read_text(encoding="utf-8").splitlines())
        self.assertTrue((root / "output/cabal-probe/Main.hsc").is_file())
        self.assertFalse((root / "output/cabal-probe/Main.hs").exists())
        self.assertEqual("EXISTING=retained\nCABAL_CONFIG=" + str(root / "output/cabal.config") + "\n", github_env.read_text(encoding="utf-8"))
        for command, options in commands:
            self.assertIsInstance(command, list)
            self.assertNotIn("shell", options)
            self.assertNotIn("SDKROOT", options["env"])
            self.assertNotIn("DEVELOPER_DIR", options["env"])
        cabal = [v for v in commands if "v2-build" in v[0] or "v2-run" in v[0]]
        self.assertEqual(5, len(cabal))
        self.assertTrue(all(Path(v[1]["env"]["CABAL_CONFIG"]).parent == root / "output" for v in cabal))
        self.assertTrue(all("--offline" in v[0] for v in cabal))

    def test_each_tool_failure_and_wrong_runtime_output_publish_nothing(self):
        for stage in range(1, 17):
            with self.subTest(stage=stage):
                _, github_env, receipt, commands = self.run_mock_bootstrap(failed_stage=stage)
                self.assertIsNone(receipt)
                self.assertEqual(stage, len(commands))
                self.assertEqual("EXISTING=retained\n", github_env.read_text(encoding="utf-8"))
        _, github_env, receipt, _ = self.run_mock_bootstrap(wrong_output=True)
        self.assertIsNone(receipt)
        self.assertEqual("EXISTING=retained\n", github_env.read_text(encoding="utf-8"))
        for stage in [5, 6, 9, 11, 16]:
            with self.subTest(wrong_output_stage=stage):
                _, github_env, receipt, _ = self.run_mock_bootstrap(wrong_stage=stage)
                self.assertIsNone(receipt)
                self.assertEqual("EXISTING=retained\n", github_env.read_text(encoding="utf-8"))

    def test_generated_driver_executes_without_shell_argument_reinterpretation(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            handler = root / "capture.py"
            handler.write_text("import json, sys\nprint(json.dumps(sys.argv[1:]))\n", encoding="utf-8")
            wrapper, forced = toolchain.write_hsc_driver(root, sys.executable, CLANG, LD, SDK)
            literal = "a;'$() literal with spaces"
            result = subprocess.run([sys.executable, str(root / "hsc2hs-driver.py"), str(handler),
                                     "Main.hsc", literal, "--cc=/caller/clang"],
                                    capture_output=True, text=True, check=True)
            self.assertEqual(["Main.hsc", literal, "--cc=/caller/clang", *forced], json.loads(result.stdout))
            self.assertEqual(1, toolchain.verify_hsc_route(root, sys.executable, forced))
            self.assertEqual(["exec", sys.executable, str(root / "hsc2hs-driver.py"), "$@"],
                             shlex.split(wrapper.read_text(encoding="utf-8").splitlines()[1]))

    def test_generated_driver_preserves_child_failure_status(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            handler = root / "failure.py"
            handler.write_text("raise SystemExit(23)\n", encoding="utf-8")
            _, forced = toolchain.write_hsc_driver(root, sys.executable, CLANG, LD, SDK)
            result = subprocess.run([sys.executable, str(root / "hsc2hs-driver.py"), str(handler), "Main.hsc"],
                                    capture_output=True, text=True, check=False)
            self.assertEqual(23, result.returncode)
            self.assertEqual(1, toolchain.verify_hsc_route(root, sys.executable, forced))

    def test_ignored_phase_refusal_and_missing_or_wrong_hsc_route_publish_nothing(self):
        for options in [{"ignore_refusal": True}, {"omit_hsc_trace": True}, {"bad_hsc_trace": True}, {"wrong_refusal_cause": True}, {"wrong_refusal_cause": "prefix"}]:
            with self.subTest(options=options):
                _, github_env, receipt, _ = self.run_mock_bootstrap(**options)
                self.assertIsNone(receipt)
                self.assertEqual("EXISTING=retained\n", github_env.read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()
