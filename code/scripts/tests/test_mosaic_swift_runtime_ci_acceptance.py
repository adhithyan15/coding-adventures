from __future__ import annotations

import importlib.util
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "mosaic_swift_runtime_ci_acceptance.py"
WORKFLOW = Path(__file__).resolve().parents[3] / ".github" / "workflows" / "ci.yml"
SWIFT_CONFORMANCE = (
    Path(__file__).resolve().parents[2]
    / "packages"
    / "rust"
    / "mosaic-app-bindings"
    / "conformance"
    / "swiftui"
)
SPEC = importlib.util.spec_from_file_location("mosaic_swift_runtime_ci_acceptance", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class MosaicSwiftRuntimeCIAcceptanceTests(unittest.TestCase):
    def test_force_plan_requires_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_mosaic_swift_runtime({"affected_packages": None})
        )

    def test_swift_emitter_requires_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_mosaic_swift_runtime(
                {"affected_packages": ["rust/mosaic-emit-swiftui"]}
            )
        )

    def test_standard_runtime_binding_requires_acceptance(self) -> None:
        for package in (
            "rust/mosaic-app-bindings",
            "rust/mosaic-app-capi",
            "rust/mosaic-app-conformance",
            "rust/mosaic-app-runtime",
            "rust/task-mosaic-app",
            "rust/engram-mosaic-app",
        ):
            with self.subTest(package=package):
                self.assertTrue(
                    MODULE.requires_mosaic_swift_runtime(
                        {"affected_packages": [package]}
                    )
                )

    def test_journal_requires_acceptance(self) -> None:
        # J5b: this lane emits and compiles Journal, so its engine, its app
        # and its package all have to trigger it.
        for package in (
            "rust/journal-core",
            "rust/journal-mosaic-app",
            "mosaic/programs/journal-app",
        ):
            with self.subTest(package=package):
                self.assertTrue(
                    MODULE.requires_mosaic_swift_runtime(
                        {"affected_packages": [package]}
                    )
                )

    def test_journal_block_emits_strictly_and_builds(self) -> None:
        """Journal's block (J5b): the strict binding, pinned-empty reports,
        and the build gate, all inside the block itself."""

        workflow = WORKFLOW.read_text(encoding="utf-8")
        self.assertIn("# ---- Journal: emit and BUILD on SwiftUI", workflow)
        start = workflow.index("# ---- Journal: emit and BUILD on SwiftUI")
        journal_block = workflow[start:workflow.index("\n\n", start)]
        self.assertIn(
            "cargo build --manifest-path code/packages/rust/Cargo.toml -p journal-mosaic-app",
            journal_block,
        )
        self.assertIn("libjournal_mosaic_app.dylib", journal_block)
        self.assertIn("--backend swiftui", journal_block)
        self.assertIn("--profile native-complete", journal_block)
        self.assertIn('--runtime-library "$journal_swift_runtime"', journal_block)
        self.assertIn("'.replacedGeneratedFiles == []'", journal_block)
        self.assertIn('.degradations | type == "array" and length == 0', journal_block)
        self.assertIn('swift build --package-path "$journal_swift_output/swiftui"', journal_block)

    def test_ios_links_the_real_runtime_statically(self) -> None:
        """UI89 §2.1: Trestle is built for the iOS Simulator and devices with
        its Rust engine linked from an .xcframework, and nm proves the engine
        is in both binaries."""

        workflow = WORKFLOW.read_text(encoding="utf-8")
        start = workflow.index("# iOS and iPadOS with the REAL runtime linked in (UI89")
        block = workflow[start:workflow.index("\n\n", start)]
        self.assertIn("rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios", block)
        self.assertIn("bash code/scripts/build-mosaic-xcframework.sh task-mosaic-app", block)
        self.assertIn('--runtime-library "$ios_runtime"', block)
        self.assertIn("MOSAIC_RUNTIME_STATIC", block)
        self.assertIn("-destination 'generic/platform=iOS Simulator'", block)
        self.assertIn("-destination 'generic/platform=iOS'", block)
        self.assertIn("nm -gU", block)
        # `nm | grep -q` under pipefail fails on SIGPIPE whenever grep exits
        # at its first match while nm is still writing.
        self.assertNotRegex(block, r"(?m)^\s*nm [^\n]*\| *grep -q")

    def test_ios_app_target_builds_installs_and_launches(self) -> None:
        """UI89 §2.2: the generated Xcode project builds an .app with the
        package's identity and the engine, and it keeps running on a
        simulator."""

        workflow = WORKFLOW.read_text(encoding="utf-8")
        start = workflow.index("# An installable iOS / iPadOS APP (UI89")
        block = workflow[start:workflow.index("\n\n", start)]
        self.assertIn("xcodebuild -project App.xcodeproj -target App -sdk iphonesimulator", block)
        self.assertIn("nm -gU \"$ios_app/App\"", block)
        self.assertNotRegex(block, r"(?m)^\s*nm [^\n]*\| *grep -q")
        self.assertIn("= dev.codingadventures.trestle", block)
        self.assertIn("xcrun simctl install", block)
        self.assertIn("xcrun simctl launch", block)
        self.assertIn("launchctl list | grep 'dev.codingadventures.trestle' > /dev/null", block)

    def test_platform_effects_library_is_driven_in_the_runtime_lane(self) -> None:
        """UI87 §7: the SwiftUI platform library every app gets is exercised
        with a fake host and fake panels in the conformance harness, not only
        compiled."""

        workflow = WORKFLOW.read_text(encoding="utf-8")
        start = workflow.index("# The SwiftUI platform library's own behaviour (UI87 §7): every SwiftUI app")
        block = workflow[start:workflow.index("\n\n", start)]
        self.assertIn(
            'cp "$bundled_output/swiftui/Sources/App/MosaicPlatformEffects.swift"',
            block,
        )
        self.assertIn("-Xswiftc -DMOSAIC_PLATFORM_EFFECTS Conformance --platform-effects", block)

    def test_harness_does_not_duplicate_the_platform_library(self) -> None:
        """The checks are compiled against the generated library, never a copy,
        and only behind the flag, so the release workflow's plain harness
        build still compiles without it."""

        sources = SWIFT_CONFORMANCE / "Sources" / "Conformance"
        self.assertFalse((sources / "MosaicPlatformEffects.swift").exists())
        checks = (sources / "PlatformEffectsChecks.swift").read_text(encoding="utf-8")
        self.assertIn("#if MOSAIC_PLATFORM_EFFECTS", checks)
        self.assertIn("#else", checks)
    def test_ios_state_lives_in_the_sandbox_and_is_restored(self) -> None:
        """UI89 §2.3 (step 3): seeded state is quarantined inside the app's
        container when rejected, and the macOS run's real snapshot is
        restored without quarantine; the app also runs on an iPad."""

        workflow = WORKFLOW.read_text(encoding="utf-8")
        start = workflow.index("# State lives in the app's sandbox and is read back (UI89 §2.3,")
        block = workflow[start:workflow.index("\n\n", start)]
        self.assertIn("xcrun simctl get_app_container", block)
        self.assertIn('Library/Application Support/task-app"', block)
        self.assertIn("printf '{}' > \"$ios_state\"", block)
        self.assertIn('test -f "$ios_state.corrupt"', block)
        self.assertIn('cp "$task_state_path" "$ios_state"', block)
        self.assertIn('test ! -e "$ios_state.corrupt"', block)
        self.assertEqual(block.count("launchctl list | grep 'dev.codingadventures.trestle'"), 2)
        ipad_start = workflow.index("# The same app on iPadOS (UI89 §2.2")
        ipad = workflow[ipad_start:workflow.index("\n\n", ipad_start)]
        self.assertIn('select(.name | startswith("iPad"))', ipad)
        self.assertIn('xcrun simctl install "$ipad" "$ios_app"', ipad)

    def test_journal_runs_and_keeps_its_state_on_ios(self) -> None:
        """UI89 §2.4 (step 7): Journal's engine is linked statically, its
        app carries the default identity, and the simulator gate's three
        launches run before the iPad launch."""

        workflow = WORKFLOW.read_text(encoding="utf-8")
        start = workflow.index("# ---- Journal on iOS and iPadOS (UI89 §2.4, step 7).")
        block = workflow[start : workflow.index("\n\n", start)]
        self.assertIn("bash code/scripts/build-mosaic-xcframework.sh journal-mosaic-app", block)
        self.assertIn('--runtime-library "$journal_ios_runtime"', block)
        self.assertIn("xcodebuild -project App.xcodeproj -target App -sdk iphonesimulator", block)
        self.assertIn("= dev.codingadventures.journalapp", block)
        self.assertNotRegex(block, r"(?m)^\s*nm [^\n]*\| *grep -q")
        gate = (
            'bash code/scripts/mosaic-ios-simulator-gate.sh "$journal_ios_app" '
            'dev.codingadventures.journalapp journal-app "$simulator"'
        )
        self.assertIn(gate, block)
        self.assertLess(block.index(gate), block.index('xcrun simctl install "$ipad" "$journal_ios_app"'))

        script = (SCRIPT.parent / "mosaic-ios-simulator-gate.sh").read_text(encoding="utf-8")
        # Where the host keeps state, and the order that makes launch 3 mean
        # something: the quarantine is seen before fresh state is trusted.
        self.assertIn('state_dir="$container/Library/Application Support/$application_id"', script)
        self.assertIn("printf '{}' > \"$state\"", script)
        self.assertLess(script.index('test -e "$corrupt"'), script.rindex('test -s "$state"'))

    def test_a_lane_script_change_alone_requires_acceptance(self) -> None:
        """The iOS scripts belong to no package; changing one must still run
        the lane that executes it."""

        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)

            def git(*arguments: str) -> None:
                subprocess.run(
                    ["git", "-c", "user.name=t", "-c", "user.email=t@example.com", *arguments],
                    cwd=repo,
                    check=True,
                    capture_output=True,
                )

            git("init", "-q", "-b", "main")
            for path in (MODULE.CI_WORKFLOW_PATH, *MODULE.CI_SCRIPT_PATHS, "README.md"):
                (repo / path).parent.mkdir(parents=True, exist_ok=True)
                (repo / path).write_text("v1\n", encoding="utf-8")
            git("add", ".")
            git("commit", "-q", "-m", "base")
            git("checkout", "-q", "-b", "change")
            (repo / "README.md").write_text("v2\n", encoding="utf-8")
            git("commit", "-q", "-am", "unrelated")
            self.assertFalse(MODULE.workflow_changed(repo, "main"))
            for path in MODULE.CI_SCRIPT_PATHS:
                (repo / path).write_text("v2\n", encoding="utf-8")
                git("commit", "-q", "-am", f"change {path}")
                self.assertTrue(MODULE.workflow_changed(repo, "main"), path)
                git("reset", "-q", "--hard", "HEAD~1")

    def test_every_script_the_swift_lane_calls_is_a_lane_script(self) -> None:
        """Read the scripts the SwiftUI runtime step actually calls, so a new
        one cannot be left out of CI_SCRIPT_PATHS."""

        workflow = WORKFLOW.read_text(encoding="utf-8")
        start = workflow.index("- name: Round-trip Rust engine through standard SwiftUI binding")
        block = workflow[start : workflow.index("\n      - name:", start)]
        called = set(re.findall(r"code/scripts/[\w./-]+\.sh\b", block))
        self.assertTrue(called)
        self.assertEqual(sorted(called - set(MODULE.CI_SCRIPT_PATHS)), [])

    def test_ios_project_generator_requires_acceptance(self) -> None:
        self.assertIn("rust/mosaic-ios-project", MODULE.ACCEPTANCE_PACKAGES)

    def test_task_app_requires_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_mosaic_swift_runtime(
                {"affected_packages": ["mosaic/programs/task-app"]}
            )
        )

    def test_engram_app_requires_acceptance(self) -> None:
        """The lane has built Engram since #13728 -- but not on Engram's account.

        This package was missing from the acceptance set, so a change to
        `host/swiftui/EngramEffects.swift` skipped the only step that compiles
        it. The gap was found while adding the matching Compose lane.
        """

        self.assertTrue(
            MODULE.requires_mosaic_swift_runtime(
                {"affected_packages": ["mosaic/programs/engram-app"]}
            )
        )

    def test_standard_mosaic_package_requires_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_mosaic_swift_runtime(
                {"affected_packages": ["mosaic/mosaic-pkg-grid"]}
            )
        )

    def test_unrelated_plan_skips_acceptance(self) -> None:
        self.assertFalse(
            MODULE.requires_mosaic_swift_runtime(
                {"affected_packages": ["rust/html-parser"]}
            )
        )

    def test_workflow_change_self_tests_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_mosaic_swift_runtime(
                {"affected_packages": []}, workflow_changed=True
            )
        )

    def test_invalid_affected_packages_is_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "array or null"):
            MODULE.requires_mosaic_swift_runtime({"affected_packages": "all"})

    def test_cli_emits_github_output_value(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            plan = Path(directory) / "build-plan.json"
            plan.write_text(
                '{"affected_packages":["rust/mosaic-emit-swiftui"]}',
                encoding="utf-8",
            )
            result = subprocess.run(
                [sys.executable, str(SCRIPT), str(plan)],
                check=True,
                capture_output=True,
                text=True,
            )
        self.assertEqual(result.stdout, "required=true\n")

    def test_workflow_routes_rust_and_swift_acceptance(self) -> None:
        workflow = WORKFLOW.read_text(encoding="utf-8")
        swift_runtime_step = workflow.split(
            "- name: Round-trip Rust engine through standard SwiftUI binding", 1
        )[1].split(
            "- name: Round-trip Rust engine through standard Qt binding", 1
        )[0]
        self.assertIn(
            "needs_mosaic_swift_runtime: "
            "${{ steps.mosaic-swift-runtime.outputs.required }}",
            workflow,
        )
        self.assertIn(
            "python3 code/scripts/mosaic_swift_runtime_ci_acceptance.py",
            workflow,
        )
        self.assertIn(
            "needs_mosaic_swift_runtime == 'true'", workflow
        )
        self.assertIn(
            "Round-trip Rust engine through standard SwiftUI binding", workflow
        )
        # 30, not 15: since UI89 §2.2 the step also boots an iOS simulator
        # and launches the generated app.
        self.assertIn("timeout-minutes: 45", swift_runtime_step)
        self.assertIn(
            "mosaic-compile/Cargo.toml -- pkg code/programs/mosaic/task-app",
            workflow,
        )
        self.assertIn(
            "--backend swiftui --output \"$taskapp_output\" --emit-project --profile native-complete --runtime-library \"$task_runtime_library\"",
            swift_runtime_step,
        )
        self.assertIn(
            "cargo build --manifest-path code/packages/rust/Cargo.toml -p mosaic-app-conformance",
            workflow,
        )
        self.assertIn("mosaic-app-conformance/package", workflow)
        self.assertIn('--runtime-library "$runtime_library"', workflow)
        self.assertIn(
            "find \"$swift_bin\" -type f -path '*/Runtime/libmosaic_app.dylib'",
            workflow,
        )
        self.assertIn('cmp "$runtime_library" "$installed_runtime"', workflow)
        self.assertIn("Sources/App/MosaicRuntimeHost.swift", workflow)
        self.assertIn("Sources/CMosaicRuntime/CMosaicRuntime.c", workflow)
        self.assertIn(
            'env -u MOSAIC_APP_LIBRARY MOSAIC_APP_STATE_PATH="$state_path"',
            swift_runtime_step,
        )
        self.assertIn(
            'MOSAIC_EXPECT_RESTORED=1 \\\n'
            '            swift run --package-path "$harness" Conformance '
            '--library "$installed_runtime"',
            swift_runtime_step,
        )
        self.assertIn(
            'MOSAIC_EXPECT_PERSISTENCE_WARNING=1 \\\n'
            '            swift run --package-path "$harness" Conformance '
            '--library "$installed_runtime"',
            swift_runtime_step,
        )
        self.assertIn("mosaic-swift-bundled-conformance", workflow)
        self.assertIn(
            '--backend swiftui --output "$bundled_output" '
            '--emit-project --profile native-complete --runtime-library "$runtime_library"',
            workflow,
        )
        self.assertIn(
            ".nativeComplete == true and (.degradations | length == 0)",
            workflow,
        )
        self.assertIn(
            'swift build --package-path "$bundled_output/swiftui"', workflow
        )
        self.assertIn(
            "cargo build --manifest-path code/packages/rust/Cargo.toml -p task-mosaic-app",
            swift_runtime_step,
        )
        self.assertIn("libtask_mosaic_app.dylib", swift_runtime_step)
        self.assertIn(
            'cmp "$task_runtime_library" "$installed_taskapp_runtime"',
            swift_runtime_step,
        )
        self.assertIn('installed_taskapp="$taskapp_bin/App"', swift_runtime_step)
        self.assertIn('env -u MOSAIC_APP_LIBRARY \\', swift_runtime_step)
        self.assertIn('"$installed_taskapp" >"$taskapp_log"', swift_runtime_step)
        self.assertIn(
            'if ! kill -0 "$taskapp_pid" 2>/dev/null; then', swift_runtime_step
        )
        self.assertIn(
            'SWIFT_BACKTRACE="enable=yes,interactive=no,output-to=stderr"',
            swift_runtime_step,
        )
        self.assertIn('cat "$taskapp_log"', swift_runtime_step)
        self.assertIn('"$HOME/Library/Logs/DiagnosticReports"', swift_runtime_step)
        self.assertIn("-name 'App*.ips'", swift_runtime_step)
        self.assertIn("com.apple.security.get-task-allow", swift_runtime_step)
        self.assertIn(
            'lldb --batch -o run -o "thread backtrace all"', swift_runtime_step
        )
        self.assertIn("diagnose_taskapp_crash", swift_runtime_step)
        self.assertIn("Mosaic Rust runtime unavailable", swift_runtime_step)
        self.assertIn("missing required MIL prop", swift_runtime_step)
        self.assertIn(
            '--backend swiftui --output "$ios_output" --emit-project',
            swift_runtime_step,
        )
        self.assertIn("MOSAIC_APP_LIBRARY", swift_runtime_step)

    def test_harness_does_not_duplicate_the_generated_binding(self) -> None:
        program = SWIFT_CONFORMANCE / "Sources" / "Conformance" / "Program.swift"
        package = (SWIFT_CONFORMANCE / "Package.swift").read_text(encoding="utf-8")
        self.assertTrue(program.is_file())
        self.assertFalse(
            (
                SWIFT_CONFORMANCE
                / "Sources"
                / "Conformance"
                / "MosaicRuntimeHost.swift"
            ).exists()
        )
        self.assertIn('name: "CMosaicRuntime"', package)
        self.assertIn('name: "Conformance"', package)


if __name__ == "__main__":
    unittest.main()
