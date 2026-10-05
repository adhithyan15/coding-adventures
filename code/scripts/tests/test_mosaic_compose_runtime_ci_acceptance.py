from __future__ import annotations

import importlib.util
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "mosaic_compose_runtime_ci_acceptance.py"
WORKFLOW = Path(__file__).resolve().parents[3] / ".github" / "workflows" / "ci.yml"
COMPOSE_CONFORMANCE = (
    Path(__file__).resolve().parents[2]
    / "packages"
    / "rust"
    / "mosaic-app-bindings"
    / "conformance"
    / "compose"
)
COMPOSE_PACKAGE = (
    Path(__file__).resolve().parents[2]
    / "packages"
    / "rust"
    / "mosaic-app-conformance"
    / "package"
)
SPEC = importlib.util.spec_from_file_location(
    "mosaic_compose_runtime_ci_acceptance", SCRIPT
)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class MosaicComposeRuntimeCIAcceptanceTests(unittest.TestCase):
    def test_force_plan_requires_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_mosaic_compose_runtime({"affected_packages": None})
        )

    def test_compose_emitter_requires_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_mosaic_compose_runtime(
                {"affected_packages": ["rust/mosaic-emit-compose"]}
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
                    MODULE.requires_mosaic_compose_runtime(
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
                    MODULE.requires_mosaic_compose_runtime(
                        {"affected_packages": [package]}
                    )
                )

    def test_journal_block_emits_strictly_and_builds(self) -> None:
        """Journal's block (J5b): the strict binding, pinned-empty reports,
        and the build gate, all inside the block itself."""

        workflow = WORKFLOW.read_text(encoding="utf-8")
        self.assertIn("# ---- Journal: emit and COMPILE on Compose", workflow)
        start = workflow.index("# ---- Journal: emit and COMPILE on Compose")
        journal_block = workflow[start:workflow.index("\n\n", start)]
        self.assertIn(
            "cargo build --manifest-path code/packages/rust/Cargo.toml -p journal-mosaic-app",
            journal_block,
        )
        self.assertIn("libjournal_mosaic_app.so", journal_block)
        self.assertIn("--backend compose", journal_block)
        self.assertIn("--profile native-complete", journal_block)
        self.assertIn('--runtime-library "$journal_runtime_library"', journal_block)
        self.assertIn("'.replacedGeneratedFiles == []'", journal_block)
        self.assertIn('.degradations | type == "array" and length == 0', journal_block)
        # compileKotlin plus the distributable the UI launches load from.
        self.assertIn(
            '-p "$journal_output/compose" compileKotlin createDistributable',
            journal_block,
        )
        self.assertIn('cmp "$journal_runtime_library" "$installed_journal_runtime"', journal_block)

    def test_journal_is_driven_twice_on_one_state_file(self) -> None:
        """JournalUiTest runs as a first launch and a restored launch against
        the SAME state file, with the packaged runtime, so a regression in
        write, save or restore fails CI rather than only compiling."""

        workflow = WORKFLOW.read_text(encoding="utf-8")
        start = workflow.index("# Drive the generated app, not just compile it (J5, #14416)")
        block = workflow[start:workflow.index("\n\n", start)]
        self.assertIn(
            "cp code/packages/rust/journal-mosaic-app/conformance/compose/JournalUiTest.kt",
            block,
        )
        self.assertEqual(block.count("test --tests JournalUiTest"), 2)
        self.assertEqual(block.count('MOSAIC_APP_STATE_PATH="$journal_ui_state"'), 2)
        self.assertEqual(block.count("MOSAIC_EXPECT_RESTORED=1"), 1)
        self.assertIn("--rerun-tasks", block)
        self.assertEqual(
            block.count('-Dcompose.application.resources.dir=$(dirname "$installed_journal_runtime")'),
            2,
        )
        self.assertIn('rm -f "$journal_ui_state"', block)

    def test_engram_layout_variant_is_compiled_beside_the_default(self) -> None:
        """UI48 §7.5: the Compose lane checks Engram's touch variant and its
        selector are in the source set before compiling it."""

        workflow = WORKFLOW.read_text(encoding="utf-8")
        start = workflow.index("# UI48 §7.5 (ENV2/ENV3): Engram is the one package with a layout")
        block = workflow[start:workflow.index("\n\n", start)]
        self.assertIn("^fun EngramAppTouch($", block)
        self.assertIn("then exit 1; fi", block)
        self.assertIn("when (mosaicLayoutVariant(environmentReport))", block)
        self.assertIn("rust/mosaic-package-manifest", MODULE.ACCEPTANCE_PACKAGES)

    def test_platform_effects_library_is_driven_in_the_journal_lane(self) -> None:
        """UI87 §7: the Compose platform library every app gets is exercised
        with fake dialogs in the Journal project, not only compiled."""

        workflow = WORKFLOW.read_text(encoding="utf-8")
        start = workflow.index("# The platform library's own behaviour (UI87 §7)")
        block = workflow[start:workflow.index("\n\n", start)]
        self.assertIn(
            "cp code/packages/rust/mosaic-app-bindings/conformance/compose/MosaicPlatformEffectsTest.kt",
            block,
        )
        self.assertIn("test --tests MosaicPlatformEffectsTest", block)

    def test_trestle_builds_an_android_apk(self) -> None:
        """UI89 step 4: the Compose lane keeps the Android SDK, builds
        TaskApp's generated Android project, and checks what the APK is."""

        workflow = WORKFLOW.read_text(encoding="utf-8")
        start = workflow.index("- name: Build Trestle for Android (UI89 step 4)")
        block = workflow[start:workflow.index("\n      - name:", start)]
        self.assertIn("needs.detect.outputs.needs_mosaic_compose_runtime == 'true'", block)
        self.assertIn('android_project="$RUNNER_TEMP/mosaic-compose-taskapp/compose/android"', block)
        self.assertIn('test ! -e "$android_project/src/main/kotlin/Main.kt"', block)
        self.assertIn(
            "-cp gradle/wrapper/gradle-wrapper.jar org.gradle.wrapper.GradleWrapperMain",
            block,
        )
        self.assertIn("launchable-activity: name='mosaic\\.android\\.MosaicActivity'", block)
        self.assertIn("package: name='dev\\.codingadventures\\.trestle'", block)
        self.assertIn("^(min)?[sS]dkVersion:'26'", block)
        self.assertIn('test -f "$dex/lib/x86_64/libjnidispatch.so"', block)
        # A failed check says which one, instead of a silent `grep -q`.
        self.assertIn("::error::the APK's badging has no line matching", block)
        # The SDK survives the disk reclaim and is set up for this lane.
        self.assertIn('[ "$NEEDS_MOSAIC_COMPOSE" != "true" ]', workflow)
        setup = workflow.index("- name: Set up Android SDK")
        self.assertIn(
            "(needs.detect.outputs.needs_mosaic_compose_runtime == 'true' && runner.os == 'Linux')",
            workflow[setup:workflow.index("\n", setup + 40)],
        )

    def test_trestle_android_apk_carries_the_rust_runtime(self) -> None:
        """UI89 step 5: every ABI's engine is built with cargo-ndk, installed
        through --runtime-library, and proven in the APK by its symbol."""

        workflow = WORKFLOW.read_text(encoding="utf-8")
        start = workflow.index("- name: Build Trestle for Android with its Rust runtime (UI89 step 5)")
        block = workflow[start:workflow.index("\n      - name:", start)]
        self.assertIn("needs.detect.outputs.needs_mosaic_compose_runtime == 'true'", block)
        self.assertIn('bash code/scripts/build-mosaic-android-libs.sh task-mosaic-app "$jni_libs"', block)
        self.assertIn('--runtime-library "$jni_libs"', block)
        self.assertIn("for abi in arm64-v8a armeabi-v7a x86_64 x86; do", block)
        self.assertIn("' T mosaic_app_create$'", block)
        self.assertIn("cargo install --locked cargo-ndk --version", block)

    def test_journal_android_apk_carries_its_rust_runtime(self) -> None:
        """UI89 step 7 (§3.9): Journal's engine is built for every ABI and
        packaged like Trestle's, through the verified wrapper jar."""

        workflow = WORKFLOW.read_text(encoding="utf-8")
        start = workflow.index("- name: Build Journal for Android with its Rust runtime (UI89 step 7)")
        block = workflow[start:workflow.index("\n      - name:", start)]
        self.assertLess(
            workflow.index("- name: Build Trestle for Android with its Rust runtime (UI89 step 5)"),
            start,
        )
        self.assertIn("needs.detect.outputs.needs_mosaic_compose_runtime == 'true'", block)
        self.assertIn('bash code/scripts/build-mosaic-android-libs.sh journal-mosaic-app "$jni_libs"', block)
        self.assertIn("pkg code/programs/mosaic/journal-app --backend compose", block)
        self.assertIn('--runtime-library "$jni_libs"', block)
        self.assertIn('bash code/scripts/assemble-mosaic-android-debug.sh "$android_project"', block)
        self.assertIn("package: name='dev\\.codingadventures\\.journalapp'", block)
        self.assertIn("launchable-activity: name='mosaic\\.android\\.MosaicActivity'", block)
        self.assertIn("for abi in arm64-v8a armeabi-v7a x86_64 x86; do", block)
        self.assertIn("' T mosaic_app_create$'", block)

        assemble = (SCRIPT.parent / "assemble-mosaic-android-debug.sh").read_text(encoding="utf-8")
        # The jar is verified before anything runs it.
        self.assertLess(
            assemble.index("verify-gradle-wrapper-jar.sh"),
            assemble.index("org.gradle.wrapper.GradleWrapperMain"),
        )

    def test_engram_android_apk_carries_its_rust_runtime(self) -> None:
        """UI89 step 7 (§3.10): Engram is built like Journal, and its
        desktop-only Anki handler is kept out of the Android dex."""

        workflow = WORKFLOW.read_text(encoding="utf-8")
        start = workflow.index("- name: Build Engram for Android with its Rust runtime (UI89 step 7)")
        block = workflow[start : workflow.index("\n      - name:", start)]
        self.assertLess(
            workflow.index("- name: Build Journal for Android with its Rust runtime (UI89 step 7)"),
            start,
        )
        self.assertIn('bash code/scripts/build-mosaic-android-libs.sh engram-mosaic-app "$jni_libs"', block)
        self.assertIn("pkg code/programs/mosaic/engram-app --backend compose", block)
        self.assertIn('bash code/scripts/assemble-mosaic-android-debug.sh "$android_project"', block)
        self.assertIn("package: name='dev\\.codingadventures\\.engramapp'", block)
        self.assertIn("'LEngramAppKt;'", block)
        self.assertIn("for needle in 'LEngramEffectsKt;' 'installEngramEffects'; do", block)
        self.assertIn('if [ "$found" -ne 1 ]; then', block)
        self.assertIn("' T mosaic_app_create$'", block)

        emulator = workflow.index("- name: Launch Trestle, Journal and Engram on an Android emulator")
        gate = workflow[emulator : workflow.index("\n      - name:", emulator)]
        self.assertLess(
            gate.index("dev.codingadventures.journalapp journal-app"),
            gate.index(
                'bash code/scripts/mosaic-android-emulator-gate.sh "$engram_apk" '
                "dev.codingadventures.engramapp engram-app"
            ),
        )

    def test_trestle_launches_and_restores_on_an_android_emulator(self) -> None:
        """UI89 step 5, second half: the APK with the runtime boots on an
        x86_64 emulator, keeps its state and quarantines refused state."""

        workflow = WORKFLOW.read_text(encoding="utf-8")
        start = workflow.index("- name: Launch Trestle, Journal and Engram on an Android emulator (UI89 steps 5, 7)")
        block = workflow[start:workflow.index("\n      - name:", start)]
        self.assertLess(
            workflow.index("- name: Build Trestle for Android with its Rust runtime (UI89 step 5)"),
            start,
        )
        self.assertIn("needs.detect.outputs.needs_mosaic_compose_runtime == 'true'", block)
        self.assertIn("mosaic-compose-taskapp-android-runtime/compose/android/build/outputs/apk/debug", block)
        self.assertIn('bash code/scripts/start-mosaic-android-emulator.sh "$emulator_log"', block)
        self.assertIn(
            'bash code/scripts/mosaic-android-emulator-gate.sh "$apk" dev.codingadventures.trestle task-app',
            block,
        )
        self.assertIn("adb emu kill", block)
        # Journal runs the same gate after Trestle, on the same emulator.
        self.assertIn("mosaic-compose-journal-android-runtime/compose/android/build/outputs/apk/debug", block)
        self.assertLess(
            block.index("dev.codingadventures.trestle task-app"),
            block.index(
                'bash code/scripts/mosaic-android-emulator-gate.sh "$journal_apk" '
                "dev.codingadventures.journalapp journal-app"
            ),
        )

        scripts = SCRIPT.parent
        gate = (scripts / "mosaic-android-emulator-gate.sh").read_text(encoding="utf-8")
        # Where MosaicActivity points the host, the three launches, and the
        # checks that make each one mean something.
        self.assertIn('state="files/$application_id/mosaic-state.v1.json"', gate)
        self.assertIn('activity="$package/mosaic.android.MosaicActivity"', gate)
        self.assertIn('printf {} > $state', gate)
        self.assertIn("FATAL EXCEPTION", gate)
        self.assertIn("rejected persisted state", gate)
        self.assertLess(gate.index('eventually "test -e $corrupt"'), gate.rindex("expect_state_written\n"))
        emulator = (scripts / "start-mosaic-android-emulator.sh").read_text(encoding="utf-8")
        self.assertIn('image="system-images;android-34;default;x86_64"', emulator)
        self.assertIn("sys.boot_completed", emulator)
        # The device lives where the emulator looks, and is listed before the
        # wait: the first CI run created it somewhere else and booted nothing.
        self.assertIn('export ANDROID_AVD_HOME="${ANDROID_AVD_HOME:-$HOME/.android/avd}"', emulator)
        self.assertIn('avds="$("$emulator" -list-avds 2>/dev/null || true)"', emulator)
        self.assertNotIn('-list-avds | grep', emulator)
        self.assertIn("the emulator exited before it appeared to adb", emulator)

    def test_a_lane_script_change_alone_requires_acceptance(self) -> None:
        """The Android scripts belong to no package; changing one must still
        run the lane that executes it."""

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

    def test_every_script_the_android_steps_call_is_a_lane_script(self) -> None:
        """A script only the Trestle Android steps call belongs to no package,
        so the lane reruns on a change to it only if CI_SCRIPT_PATHS lists it.
        Read the scripts the steps actually call, so a new one cannot be
        forgotten the way verify-gradle-wrapper-jar.sh first was."""

        workflow = WORKFLOW.read_text(encoding="utf-8")
        called: set[str] = set()
        for name in (
            "Build Trestle for Android (UI89 step 4)",
            "Build Trestle for Android with its Rust runtime (UI89 step 5)",
            "Build Journal for Android with its Rust runtime (UI89 step 7)",
            "Build Engram for Android with its Rust runtime (UI89 step 7)",
            "Launch Trestle, Journal and Engram on an Android emulator (UI89 steps 5, 7)",
        ):
            start = workflow.index(f"- name: {name}")
            block = workflow[start : workflow.index("\n      - name:", start)]
            called.update(re.findall(r"code/scripts/[\w./-]+\.(?:sh|py|ps1)\b", block))
        self.assertTrue(called)
        self.assertEqual(sorted(called - set(MODULE.CI_SCRIPT_PATHS)), [])

    def test_task_app_requires_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_mosaic_compose_runtime(
                {"affected_packages": ["mosaic/programs/task-app"]}
            )
        )

    def test_engram_app_requires_acceptance(self) -> None:
        """The lane compiles Engram's `[host_effects]` handler.

        Without this, editing `host/compose/EngramEffects.kt` -- which affects
        only this package -- would skip the one check that compiles it, so the
        edits most likely to break the handler are the ones that go unchecked.
        """

        self.assertTrue(
            MODULE.requires_mosaic_compose_runtime(
                {"affected_packages": ["mosaic/programs/engram-app"]}
            )
        )

    def test_standard_mosaic_package_requires_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_mosaic_compose_runtime(
                {"affected_packages": ["mosaic/mosaic-pkg-grid"]}
            )
        )

    def test_unrelated_plan_skips_acceptance(self) -> None:
        self.assertFalse(
            MODULE.requires_mosaic_compose_runtime(
                {"affected_packages": ["rust/html-parser"]}
            )
        )

    def test_workflow_change_self_tests_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_mosaic_compose_runtime(
                {"affected_packages": []}, workflow_changed=True
            )
        )

    def test_invalid_affected_packages_is_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "array or null"):
            MODULE.requires_mosaic_compose_runtime({"affected_packages": "all"})

    def test_cli_emits_github_output_value(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            plan = Path(directory) / "build-plan.json"
            plan.write_text(
                '{"affected_packages":["rust/mosaic-emit-compose"]}',
                encoding="utf-8",
            )
            result = subprocess.run(
                [sys.executable, str(SCRIPT), str(plan)],
                check=True,
                capture_output=True,
                text=True,
            )
        self.assertEqual(result.stdout, "required=true\n")

    def test_workflow_routes_rust_jvm_and_acceptance(self) -> None:
        workflow = WORKFLOW.read_text(encoding="utf-8")
        self.assertIn(
            "needs_mosaic_compose_runtime: "
            "${{ steps.mosaic-compose-runtime.outputs.required }}",
            workflow,
        )
        self.assertIn(
            "python3 code/scripts/mosaic_compose_runtime_ci_acceptance.py",
            workflow,
        )
        self.assertIn("Round-trip Rust engine through standard Compose binding", workflow)
        self.assertIn("needs_mosaic_compose_runtime == 'true'", workflow)
        self.assertIn("timeout-minutes: 20", workflow)
        self.assertIn(
            "--backend compose --output \"$taskapp_output\" --emit-project",
            workflow,
        )
        self.assertIn(
            "cargo build --manifest-path code/packages/rust/Cargo.toml -p mosaic-app-conformance",
            workflow,
        )
        self.assertIn(
            "$bundled_output/compose/src/main/kotlin/MosaicRuntimeHost.kt", workflow
        )
        self.assertIn(
            "cargo build --manifest-path code/packages/rust/Cargo.toml -p task-mosaic-app",
            workflow,
        )
        self.assertIn("libtask_mosaic_app.so", workflow)
        self.assertIn(
            'cmp "$task_runtime_library" "$installed_taskapp_runtime"', workflow
        )
        self.assertIn("*/bin/task_app", workflow)
        self.assertIn('run-under-xvfb.sh" timeout 8s "$installed_taskapp"', workflow)
        self.assertIn('test "$taskapp_status" -eq 124', workflow)
        self.assertIn("Mosaic Rust runtime unavailable", workflow)
        self.assertIn("--runtime-library \"$runtime_library\"", workflow)
        self.assertIn("compileKotlin createDistributable", workflow)
        self.assertIn("*/resources/libmosaic_app.so", workflow)
        self.assertIn("cmp \"$runtime_library\" \"$installed_runtime\"", workflow)
        self.assertIn("unset MOSAIC_APP_LIBRARY", workflow)
        self.assertIn("-Dcompose.application.resources.dir", workflow)
        self.assertIn("mosaic-pkg-rating-controls", workflow)
        self.assertIn("--profile native-complete", workflow)
        self.assertIn(
            "gradle --no-daemon --stacktrace -p \"$strict_output/compose\" compileKotlin",
            workflow,
        )

    def test_workflow_builds_engram_and_wires_its_effect_handler(self) -> None:
        """The Engram block, mirroring the Qt and SwiftUI ones.

        Scoped to the text AFTER the Engram marker rather than searched over the
        whole workflow: `--profile native-complete` and `compileKotlin` both
        appear several times in this lane already, so a whole-file `assertIn`
        would pass with no Engram block at all.
        """

        workflow = WORKFLOW.read_text(encoding="utf-8")
        self.assertIn("mosaic-compose-engram", workflow)
        # Anchored on the block's opening comment, not on the `$RUNNER_TEMP`
        # path: the build of the runtime library precedes that path, so slicing
        # there would put half the block out of scope.
        self.assertIn("# Engram on Compose", workflow)
        engram_step = workflow[workflow.index("# Engram on Compose"):]
        self.assertIn(
            "cargo build --manifest-path code/packages/rust/Cargo.toml -p engram-mosaic-app",
            engram_step,
        )
        self.assertIn("libengram_mosaic_app.so", engram_step)
        self.assertIn('test -f "$engram_runtime_library"', engram_step)
        self.assertIn("--backend compose", engram_step)
        self.assertIn("--profile native-complete", engram_step)
        self.assertIn('--runtime-library "$engram_runtime_library"', engram_step)
        # The override is pinned absent, which is what keeps the migration from
        # quietly coming undone.
        self.assertIn(
            "jq -e '.replacedGeneratedFiles == []' "
            '"$engram_output/compose/mosaic-degradations.json"',
            engram_step,
        )
        # And the install is checked in the generated app, which no assertion
        # over the handler's own source can reach.
        self.assertIn("installEngramEffects(it)", engram_step)
        self.assertIn('gradle --no-daemon --stacktrace -p "$engram_output/compose" compileKotlin', engram_step)

    def test_harness_does_not_duplicate_the_generated_binding(self) -> None:
        source = COMPOSE_CONFORMANCE / "src" / "main" / "kotlin"
        self.assertTrue((source / "Conformance.kt").is_file())
        self.assertFalse((source / "MosaicRuntimeHost.kt").exists())
        gradle = (COMPOSE_CONFORMANCE / "build.gradle.kts").read_text(encoding="utf-8")
        self.assertIn("net.java.dev.jna:jna:5.19.1", gradle)
        self.assertIn("kotlinx-serialization-json:1.11.0", gradle)

    def test_conformance_engine_has_a_real_mosaic_package(self) -> None:
        self.assertTrue((COMPOSE_PACKAGE / "mosaic-package.toml").is_file())
        for suffix in ("mil", "mll", "msl"):
            self.assertTrue((COMPOSE_PACKAGE / "src" / f"Counter.{suffix}").is_file())


if __name__ == "__main__":
    unittest.main()
