from __future__ import annotations

import importlib.util
import os
import re
import signal
import shutil
import subprocess
import sys
import tempfile
import time
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

    def test_engram_screens_scroll_test_runs_in_the_lane(self) -> None:
        """EN-6: the lane runs EngramScrollUiTest against the generated Engram
        project, on a fresh state file of its own, with the runtime the
        project carries, so a screen that stops scrolling fails CI."""

        workflow = WORKFLOW.read_text(encoding="utf-8")
        start = workflow.index("# The screens scroll (EN-6): EngramScrollUiTest")
        block = workflow[start : workflow.index("\n\n", start)]
        self.assertIn(
            "cp code/programs/mosaic/engram-app/conformance/compose/EngramScrollUiTest.kt",
            block,
        )
        self.assertIn('cmp "$engram_runtime_library" "$engram_app_resources/libmosaic_app.so"', block)
        self.assertIn('rm -f "$engram_scroll_state"', block)
        self.assertIn('MOSAIC_APP_STATE_PATH="$engram_scroll_state"', block)
        self.assertIn('-Dcompose.application.resources.dir=$engram_app_resources', block)
        self.assertIn("run-under-xvfb.sh", block)
        self.assertEqual(block.count("test --tests EngramScrollUiTest"), 1)
        # After the Engram project compiles, before Journal's block begins.
        self.assertLess(
            workflow.index('-p "$engram_output/compose" compileKotlin'), start
        )
        self.assertLess(start, workflow.index("# ---- Journal: emit and COMPILE on Compose"))

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
        # The lane runs in its own job, which sets the SDK up for it...
        lane_job = workflow.index("\n  mosaic-linux-runtime:\n")
        self.assertLess(lane_job, start)
        self.assertLess(start, workflow.index("\n  build:\n"))
        setup = workflow.index("- name: Set up Android SDK", lane_job)
        self.assertLess(setup, start)
        self.assertIn(
            "if: needs.detect.outputs.needs_mosaic_compose_runtime == 'true'",
            workflow[setup:workflow.index("\n", setup + 40)],
        )
        # ...and the build job, whose Compose packages build Android
        # projects too, keeps the SDK through its disk reclaim.
        self.assertIn('[ "$NEEDS_MOSAIC_COMPOSE" != "true" ]', workflow)
        build_setup = workflow.index("- name: Set up Android SDK", workflow.index("\n  build:\n"))
        self.assertIn(
            "(needs.detect.outputs.needs_mosaic_compose_runtime == 'true' && runner.os == 'Linux')",
            workflow[build_setup:workflow.index("\n", build_setup + 40)],
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
        """UI89 step 7 (§3.10): Engram is built like Journal. Its Android
        Anki handler is installed and reaches the dex (§3.12); the desktop
        one is kept out of it."""

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
        self.assertIn("'LEngramAndroidEffectsKt;' 'installEngramAndroidEffects'; do", block)
        self.assertIn(
            "installEngramAndroidEffects(it); platformRouter = installMosaicPlatformEffects("
            'it, documentPicker, setOf("importAnki", "exportAnki"))',
            block,
        )
        self.assertLess(
            block.index("installEngramAndroidEffects(it); platformRouter"),
            block.index('bash code/scripts/assemble-mosaic-android-debug.sh "$android_project"'),
        )
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
        self.assertIn('activity_class="${4-mosaic.android.MosaicActivity}"', gate)
        self.assertIn('activity="$package/$activity_class"', gate)
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

    def test_journal_runs_its_instrumented_ui_test_on_the_emulator(self) -> None:
        """UI89 §4.2: Journal's instrumented test is built beside the APK and
        run on the emulator after the Journal gate, across two cold
        launches."""

        workflow = WORKFLOW.read_text(encoding="utf-8")
        start = workflow.index("- name: Build Journal for Android with its Rust runtime (UI89 step 7)")
        build = workflow[start : workflow.index("\n      - name:", start)]
        copy = (
            "cp code/packages/rust/journal-mosaic-app/conformance/compose-android/"
            'JournalAndroidUiTest.kt "$android_project/src/androidTest/kotlin/"'
        )
        assemble = 'bash code/scripts/assemble-mosaic-android-debug.sh "$android_project" --with-android-test'
        self.assertIn(copy, build)
        self.assertIn(assemble, build)
        # The test is in place before Gradle compiles the androidTest sources.
        self.assertLess(build.index(copy), build.index(assemble))
        self.assertIn("build/outputs/apk/androidTest/debug", build)
        test_source = (
            Path(__file__).resolve().parents[2]
            / "packages/rust/journal-mosaic-app/conformance/compose-android/JournalAndroidUiTest.kt"
        )
        self.assertTrue(test_source.is_file(), test_source)
        source = test_source.read_text(encoding="utf-8")
        self.assertIn("package dev.codingadventures.journalapp.uitest", source)
        self.assertIn("createAndroidComposeRule<MosaicActivity>()", source)
        self.assertIn('getString("mosaicLaunch")', source)

        emulator = workflow.index("- name: Launch Trestle, Journal and Engram on an Android emulator")
        gate = workflow[emulator : workflow.index("\n      - name:", emulator)]
        run = (
            'bash code/scripts/mosaic-android-ui-test.sh "$journal_apk" "$journal_test_apk" '
            "dev.codingadventures.journalapp dev.codingadventures.journalapp.uitest.JournalAndroidUiTest"
        )
        self.assertIn(run, gate)
        # After the gate, whose launches leave state that the test clears.
        self.assertLess(gate.index("dev.codingadventures.journalapp journal-app"), gate.index(run))

    def test_engram_runs_its_picker_ui_test_on_the_emulator(self) -> None:
        """UI89 §4.4: Engram's instrumented test is built beside its APK and
        run after the Engram gate. It answers the document picker with
        Espresso-Intents and a file in the app's cache, so no provider ships."""

        workflow = WORKFLOW.read_text(encoding="utf-8")
        start = workflow.index("- name: Build Engram for Android with its Rust runtime (UI89 step 7)")
        build = workflow[start : workflow.index("\n      - name:", start)]
        copy = (
            "cp code/programs/mosaic/engram-app/conformance/compose-android/"
            'EngramAndroidUiTest.kt "$android_project/src/androidTest/kotlin/"'
        )
        assemble = 'bash code/scripts/assemble-mosaic-android-debug.sh "$android_project" --with-android-test'
        self.assertIn(copy, build)
        self.assertIn(assemble, build)
        self.assertLess(build.index(copy), build.index(assemble))
        source = (
            Path(__file__).resolve().parents[2]
            / "programs/mosaic/engram-app/conformance/compose-android/EngramAndroidUiTest.kt"
        ).read_text(encoding="utf-8")
        self.assertIn("package dev.codingadventures.engramapp.uitest", source)
        self.assertIn("intending(hasAction(Intent.ACTION_CREATE_DOCUMENT)).respondWith(chosen)", source)
        self.assertIn("intending(hasAction(Intent.ACTION_OPEN_DOCUMENT)).respondWith(chosen)", source)
        self.assertIn("Uri.fromFile(document)", source)
        self.assertIn("Intents.release()", source)

        emulator = workflow.index("- name: Launch Trestle, Journal and Engram on an Android emulator")
        gate = workflow[emulator : workflow.index("\n      - name:", emulator)]
        run = (
            'bash code/scripts/mosaic-android-ui-test.sh "$engram_apk" "$engram_test_apk" '
            "dev.codingadventures.engramapp dev.codingadventures.engramapp.uitest.EngramAndroidUiTest"
        )
        self.assertIn(run, gate)
        self.assertLess(gate.index("dev.codingadventures.engramapp engram-app"), gate.index(run))

    def test_trestle_runs_its_instrumented_ui_test_on_the_emulator(self) -> None:
        """UI89 §4.4: Trestle's instrumented test is compiled with its APK,
        through the same verified wrapper jar, and run on the emulator after
        the Trestle gate, across two cold launches."""

        workflow = WORKFLOW.read_text(encoding="utf-8")
        start = workflow.index("- name: Build Trestle for Android with its Rust runtime (UI89 step 5)")
        build = workflow[start : workflow.index("\n      - name:", start)]
        copy = (
            "cp code/packages/rust/task-mosaic-app/conformance/compose-android/"
            'TrestleAndroidUiTest.kt "$android_project/src/androidTest/kotlin/"'
        )
        assemble = "--no-daemon --stacktrace assembleDebug assembleDebugAndroidTest"
        self.assertIn(copy, build)
        self.assertIn(assemble, build)
        self.assertLess(build.index(copy), build.index(assemble))
        # Still the verified jar, never the unverifiable gradlew script.
        self.assertLess(build.index("verify-gradle-wrapper-jar.sh"), build.index(assemble))
        self.assertIn("build/outputs/apk/androidTest/debug", build)
        test_source = (
            Path(__file__).resolve().parents[2]
            / "packages/rust/task-mosaic-app/conformance/compose-android/TrestleAndroidUiTest.kt"
        )
        self.assertTrue(test_source.is_file(), test_source)
        source = test_source.read_text(encoding="utf-8")
        self.assertIn("package dev.codingadventures.trestle.uitest", source)
        self.assertIn("createAndroidComposeRule<MosaicActivity>()", source)
        self.assertIn('getString("mosaicLaunch")', source)

        emulator = workflow.index("- name: Launch Trestle, Journal and Engram on an Android emulator")
        gate = workflow[emulator : workflow.index("\n      - name:", emulator)]
        run = (
            'bash code/scripts/mosaic-android-ui-test.sh "$apk" "$trestle_test_apk" '
            "dev.codingadventures.trestle dev.codingadventures.trestle.uitest.TrestleAndroidUiTest"
        )
        self.assertIn(run, gate)
        self.assertLess(gate.index("dev.codingadventures.trestle task-app"), gate.index(run))
        # Before Journal's gate: the step stays in app order.
        self.assertLess(gate.index(run), gate.index("dev.codingadventures.journalapp journal-app"))

        script = (SCRIPT.parent / "mosaic-android-ui-test.sh").read_text(encoding="utf-8")
        self.assertIn('runner="$package.test/androidx.test.runner.AndroidJUnitRunner"', script)
        self.assertLess(script.index('adb shell pm clear "$package"'), script.index("run_launch 1\n"))
        self.assertLess(script.index("run_launch 1\n"), script.index("run_launch 2\n"))
        self.assertIn('"OK (1 test)"', script)
        self.assertIn('"FAILURES!!!"', script)
        self.assertIn('"INSTRUMENTATION_CODE: -1"', script)

    def test_the_ui_test_script_reads_am_instrument_output_not_its_status(self) -> None:
        """`am instrument` exits 0 when a test fails, so the script decides
        from its output: a fake adb plays each outcome."""

        script = SCRIPT.parent / "mosaic-android-ui-test.sh"
        ok = "INSTRUMENTATION_STATUS: test=t\nOK (1 test)\nINSTRUMENTATION_CODE: -1\n"
        cases = {
            "passes": (ok, ok, 0),
            "launch 2 fails": (ok, "FAILURES!!!\nTests run: 1,  Failures: 1\nINSTRUMENTATION_CODE: -1\n", 1),
            "no test ran": ("OK (0 tests)\nINSTRUMENTATION_CODE: -1\n", ok, 1),
            "app crashed": ("INSTRUMENTATION_RESULT: shortMsg=Process crashed.\nINSTRUMENTATION_CODE: 0\n", ok, 1),
            "empty output": ("", ok, 1),
        }
        for name, (first, second, expected) in cases.items():
            with self.subTest(name), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                (root / "one.txt").write_text(first, encoding="utf-8")
                (root / "two.txt").write_text(second, encoding="utf-8")
                log = root / "adb.log"
                adb = root / "adb"
                adb.write_text(
                    "#!/usr/bin/env bash\n"
                    f'echo "$*" >> "{log}"\n'
                    'case "$*" in\n'
                    f'  *"mosaicLaunch 1"*) cat "{root}/one.txt" ;;\n'
                    f'  *"mosaicLaunch 2"*) cat "{root}/two.txt" ;;\n'
                    "esac\n"
                    "exit 0\n",
                    encoding="utf-8",
                )
                adb.chmod(0o755)
                for apk in ("app.apk", "test.apk"):
                    (root / apk).write_bytes(b"")
                result = subprocess.run(
                    [
                        "bash",
                        str(script),
                        str(root / "app.apk"),
                        str(root / "test.apk"),
                        "dev.example.app",
                        "dev.example.app.uitest.AppUiTest",
                    ],
                    env={"PATH": f"{root}:/usr/bin:/bin"},
                    capture_output=True,
                    text=True,
                    check=False,
                )
                self.assertEqual(result.returncode, expected, result.stdout + result.stderr)
                calls = log.read_text(encoding="utf-8").splitlines()
                # Both APKs installed and the app cleared before launch 1.
                self.assertTrue(calls[0].startswith("install -r -t "), calls)
                self.assertEqual(calls[2], "shell pm clear dev.example.app", calls)
                instrument = [call for call in calls if "am instrument" in call]
                self.assertIn(
                    "-e class dev.example.app.uitest.AppUiTest -e mosaicLaunch 1 "
                    "dev.example.app.test/androidx.test.runner.AndroidJUnitRunner",
                    instrument[0],
                )
                # A failed launch 1 stops before launch 2.
                self.assertEqual(len(instrument), 2 if name in ("passes", "launch 2 fails") else 1, calls)

        # Names that would reach the device shell unchecked are refused.
        for package, test_class in (
            ("dev.example.app; reboot", "dev.example.app.T"),
            ("dev.example.app", "dev.example.app.Outer$Inner"),
            ("dev.example.app", "dev.example.app.T#method"),
            ("dev.example.app", "T"),
        ):
            with self.subTest(package=package, test_class=test_class), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                for apk in ("app.apk", "test.apk"):
                    (root / apk).write_bytes(b"")
                result = subprocess.run(
                    ["bash", str(script), str(root / "app.apk"), str(root / "test.apk"), package, test_class],
                    env={"PATH": "/usr/bin:/bin"},
                    capture_output=True,
                    text=True,
                    check=False,
                )
                self.assertEqual(result.returncode, 2, result.stderr)

    def test_every_flutter_app_runs_through_the_emulator_gate(self) -> None:
        """UI89 §7 (steps 2, 4): Trestle, Journal and Engram are each built
        through the Flutter backend from their Compose step's per-ABI engines
        by build-mosaic-flutter-phone-app.sh, and gated on the emulator
        through .MainActivity after their Compose app is uninstalled."""

        workflow = WORKFLOW.read_text(encoding="utf-8")
        start = workflow.index("- name: Build Trestle, Journal and Engram for Android with Flutter (UI89 §7)")
        build = workflow[start : workflow.index("\n      - name:", start)]
        self.assertIn("needs.detect.outputs.needs_mosaic_compose_runtime == 'true'", build)
        self.assertIn("needs.detect.outputs.needs_mosaic_flutter_runtime == 'true'", build)
        self.assertIn('cp -R "$jni_libs" "$phone_runtime/android"', build)
        self.assertIn(
            'bash code/scripts/build-mosaic-flutter-phone-app.sh android "$program" "$phone_runtime" '
            '"$RUNNER_TEMP/mosaic-flutter-$name-android" dev.codingadventures "$name"',
            build,
        )
        # The list is read on fd 3, so nothing the loop runs can eat it.
        self.assertIn("while read -r program jni_libs name <&3; do", build)
        self.assertIn("done 3<<EOF", build)
        for row in (
            "code/programs/mosaic/task-app $RUNNER_TEMP/mosaic-trestle-jnilibs trestle",
            "code/programs/mosaic/journal-app $RUNNER_TEMP/mosaic-journal-jnilibs journalapp",
            "code/programs/mosaic/engram-app $RUNNER_TEMP/mosaic-engram-jnilibs engramapp",
        ):
            self.assertIn(row, build)
        # After every Compose runtime step, whose jniLibs it reuses.
        self.assertLess(workflow.index("- name: Build Engram for Android with its Rust runtime (UI89 step 7)"), start)

        emulator = workflow.index("- name: Launch Trestle, Journal and Engram on an Android emulator")
        self.assertLess(start, emulator)
        gate = workflow[emulator : workflow.index("\n      - name:", emulator)]
        self.assertIn("MOSAIC_FLUTTER_LANE: ${{ needs.detect.outputs.needs_mosaic_flutter_runtime }}", gate)
        self.assertIn("while read -r name package application_id <&3; do", gate)
        run = 'bash code/scripts/mosaic-android-emulator-gate.sh "$flutter_apk" "$package" "$application_id" .MainActivity'
        uninstall = 'adb uninstall "$package"'
        self.assertIn(run, gate)
        self.assertLess(gate.index(uninstall), gate.index(run))
        for row in (
            "trestle dev.codingadventures.trestle task-app",
            "journalapp dev.codingadventures.journalapp journal-app",
            "engramapp dev.codingadventures.engramapp engram-app",
        ):
            self.assertIn(row, gate)
        # The Flutter gates come after every Compose gate and UI test.
        self.assertLess(gate.index("EngramAndroidUiTest"), gate.index(run))

        script = (SCRIPT.parent / "build-mosaic-flutter-phone-app.sh").read_text(encoding="utf-8")
        self.assertIn('create=(flutter create "--platforms=$platform" --org "$org" --project-name "$name" .)', script)
        self.assertIn('grep -qxF "    ${create[*]}" README.md', script)
        self.assertIn("allowBackup\\(0x01010280\\)=false", script)
        self.assertIn('diff "$checks/$abi.packaged.machine" "$checks/$abi.input.machine"', script)
        self.assertIn('diff "$checks/$abi.packaged.symbols" "$checks/$abi.input.symbols"', script)
        self.assertIn("for abi in x86_64 arm64-v8a; do", script)
        self.assertNotIn("cmp ", script)

    def test_the_flutter_phone_script_refuses_names_before_it_builds(self) -> None:
        """The org and project name reach `flutter create` and a bundle
        identifier: anything but the README's own shapes is refused first."""

        script = SCRIPT.parent / "build-mosaic-flutter-phone-app.sh"
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "runtime/android").mkdir(parents=True)
            (root / "program").mkdir()
            for platform, org, name in (
                ("windows", "dev.example", "app"),
                ("android", "dev.Example", "app"),
                ("android", "dev.example;id", "app"),
                ("android", "dev.example", "my-app"),
                ("android", "dev.example", "2app"),
                ("android", "", "app"),
            ):
                with self.subTest(platform=platform, org=org, name=name):
                    result = subprocess.run(
                        ["bash", str(script), platform, str(root / "program"), str(root / "runtime"), str(root / "out"), org, name],
                        env={"PATH": "/usr/bin:/bin"},
                        capture_output=True,
                        text=True,
                        check=False,
                    )
                    self.assertEqual(result.returncode, 2, result.stderr)
                    self.assertFalse((root / "out").exists())
            # A folder that is not an earlier build's output is refused, not
            # replaced. (The root itself is refused by a guard pinned below,
            # never by running the script at `/`.)
            occupied = root / "occupied"
            occupied.mkdir()
            (occupied / "keep.txt").write_text("mine", encoding="utf-8")
            for output in ("", str(occupied)):
                with self.subTest(output=output):
                    result = subprocess.run(
                        ["bash", str(script), "android", str(root / "program"), str(root / "runtime"), output, "dev.example", "app"],
                        env={"PATH": "/usr/bin:/bin"},
                        capture_output=True,
                        text=True,
                        check=False,
                    )
                    self.assertEqual(result.returncode, 2, result.stderr)
            self.assertEqual((occupied / "keep.txt").read_text(encoding="utf-8"), "mine")
        text = script.read_text(encoding="utf-8")
        self.assertIn('if [[ -z "$output" || "$output" == "/" ]]; then', text)
        self.assertIn('rm -rf -- "${output:?}"', text)
        self.assertLess(text.index('"$output" == "/"'), text.index("rm -rf --"))

    def test_the_emulator_gate_takes_an_activity_class_name_only(self) -> None:
        """UI89 §7.5: a Flutter app launches `.MainActivity`, so the gate takes
        the activity's class name and builds the component itself. Anything
        that is not a plain class name is refused before adb is reached,
        because `adb shell` joins arguments into one device command line."""

        gate = SCRIPT.parent / "mosaic-android-emulator-gate.sh"

        def run(*activity: str, adb_body: str = "exit 0\n") -> tuple[subprocess.CompletedProcess[str], list[str]]:
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                log = root / "adb.log"
                log.write_text("", encoding="utf-8")
                adb = root / "adb"
                adb.write_text(
                    "#!/usr/bin/env bash\n" f'echo "$*" >> "{log}"\n' + adb_body,
                    encoding="utf-8",
                )
                adb.chmod(0o755)
                (root / "app.apk").write_bytes(b"")
                result = subprocess.run(
                    ["bash", str(gate), str(root / "app.apk"), "dev.example.app", "example-app", *activity],
                    env={"PATH": f"{root}:/usr/bin:/bin"},
                    capture_output=True,
                    text=True,
                    check=False,
                )
                return result, log.read_text(encoding="utf-8").splitlines()

        # `am start` fails at once in the fake, so the gate stops at its first
        # launch: enough to see which component it asked for.
        stop_at_launch = 'case "$*" in *"am start"*) exit 1 ;; esac\nexit 0\n'
        for activity, component in (
            ((), "dev.example.app/mosaic.android.MosaicActivity"),
            ((".MainActivity",), "dev.example.app/.MainActivity"),
            (("dev.example.app.MainActivity",), "dev.example.app/dev.example.app.MainActivity"),
        ):
            with self.subTest(activity=activity):
                result, calls = run(*activity, adb_body=stop_at_launch)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(f"shell am start -W -n {component}", calls, calls)

        for activity in (
            "dev.example.app/.MainActivity",
            ".Main;reboot",
            ".Main Activity",
            "$(reboot)",
            "",
            "..Main",
            ".Main.",
        ):
            with self.subTest(activity=activity):
                result, calls = run(activity)
                self.assertEqual(result.returncode, 2, result.stderr)
                self.assertEqual(calls, [], "refused before adb is reached")

    def test_flutter_state_diagnostic_is_bounded_and_cannot_mask_gate_failure(self) -> None:
        gate = SCRIPT.parent / "mosaic-android-emulator-gate.sh"
        gate_text = gate.read_text(encoding="utf-8")
        self.assertIn("timeout --kill-after=1 5 bash -c", gate_text)
        self.assertIn("timeout --foreground 1 adb shell", gate_text)
        self.assertIn('adb logcat -d -v brief --pid="$pid" -t 4000', gate_text)
        self.assertIn("test -d files || ! test -x files", gate_text)
        self.assertIn("test -f $state && test -r $state", gate_text)
        self.assertIn("test -e $state || test -L $state", gate_text)
        self.assertIn("elapsed_us=[0-9]+$", gate_text)
        self.assertIn(
            "environment_answer result=(decoded_success|decoded_error|decode_rejected|native_status_rejected)",
            gate_text,
        )
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            project = root / "mosaic-flutter-trestle-android" / "flutter"
            apk = project / "build/app/outputs/flutter-apk/app-debug.apk"
            apk.parent.mkdir(parents=True)
            apk.write_bytes(b"diagnostic-test-apk")
            (project / "pubspec.lock").write_text("lock data\n", encoding="utf-8")

            android_home = root / "android-sdk"
            build_tools = android_home / "build-tools" / "35.0.0"
            build_tools.mkdir(parents=True)
            aapt2 = build_tools / "aapt2"
            aapt2.write_text(
                "#!/usr/bin/env bash\n"
                "if [[ \"${DIAG_MODE:-ok}\" == aapt_fail ]]; then exit 12; fi\n"
                "printf \"package: name='dev.codingadventures.trestle' versionCode='1' versionName='1.0'\\n\"\n",
                encoding="utf-8",
            )
            aapt2.chmod(0o755)

            bin_dir = root / "bin"
            bin_dir.mkdir()
            app_data_root = root / "app-data"
            state_parent = app_data_root / "files" / "task-app"
            state_parent.mkdir(parents=True)
            state_path = state_parent / "mosaic-state.v1.json"
            state_path.write_bytes(b"")
            run_as_helper = root / "run-as-probe.py"
            run_as_helper.write_text(
                "import shlex, subprocess, sys\n"
                "words = shlex.split(sys.argv[1])\n"
                "if len(words) != 5 or words[0] != 'run-as' or words[2:4] != ['sh', '-c']:\n"
                "    raise SystemExit(90)\n"
                "result = subprocess.run(['/bin/sh', '-c', words[4]], cwd=sys.argv[2])\n"
                "raise SystemExit(result.returncode)\n",
                encoding="utf-8",
            )
            adb = bin_dir / "adb"
            adb.write_text(
                "#!/usr/bin/env bash\n"
                "case \"$1\" in\n"
                "  install) exit 0 ;;\n"
                "  logcat)\n"
                "    if [[ \"$2\" == -c ]]; then exit 0; fi\n"
                "    if [[ \"$6\" == -t && \"${DIAG_MODE:-ok}\" == log_fail ]]; then exit 7; fi\n"
                "    if [[ \"$6\" == -t && \"${DIAG_MODE:-ok}\" == log_large ]]; then\n"
                "      for i in $(seq 1 1200); do printf 'I/flutter ( 4321): MOSAIC_NATIVE_DIAG v=1 stage=environment_answer result=decoded_success elapsed_us=%s\\n' \"$i\"; done\n"
                "      exit 0\n"
                "    fi\n"
                "    if [[ \"$6\" != -t ]]; then printf 'ordinary gate failure log\\n'; exit 0; fi\n"
                "    if [[ \"${DIAG_MODE:-ok}\" == log_wrong_pid ]]; then printf 'I/flutter ( 9999): MOSAIC_NATIVE_DIAG v=1 stage=environment_answer result=decoded_success elapsed_us=17\\n'; exit 0; fi\n"
                "    if [[ \"${DIAG_MODE:-ok}\" == log_wrong_body ]]; then printf 'I/Other ( 9999): note mentions ( 4321) MOSAIC_NATIVE_DIAG v=1 stage=environment_answer result=decoded_success elapsed_us=17\\n'; exit 0; fi\n"
                "    if [[ \"${DIAG_MODE:-ok}\" == log_unknown ]]; then printf 'I/flutter ( 4321): MOSAIC_NATIVE_DIAG v=1 stage=environment_answer result=unexpected elapsed_us=17\\n'; exit 0; fi\n"
                "    if [[ \"${DIAG_MODE:-ok}\" == log_suffix ]]; then printf 'I/flutter ( 4321): MOSAIC_NATIVE_DIAG v=1 stage=environment_answer result=decoded_success elapsed_us=17 trailing\\n'; exit 0; fi\n"
                "    if [[ \"${DIAG_MODE:-ok}\" == log_timeout || \"${DIAG_MODE:-ok}\" == real_timeout ]]; then\n"
                "      printf '%s\\n' \"$$\" > \"$DIAG_HANG_PID_PATH\"\n"
                "      cat /proc/$$/stat > \"$DIAG_HANG_WORKER_STAT_PATH\"\n"
                "      trap '' TERM\n"
                "      /bin/sleep 30 &\n"
                "      child=$!\n"
                "      printf '%s\\n' \"$child\" > \"$DIAG_HANG_CHILD_PID_PATH\"\n"
                "      cat /proc/$child/stat > \"$DIAG_HANG_CHILD_STAT_PATH\"\n"
                "      wait \"$child\"\n"
                "    fi\n"
                "    if [[ \"${DIAG_MODE:-ok}\" != outer_timeout ]]; then\n"
                "      printf 'I/flutter ( 4321): MOSAIC_NATIVE_DIAG v=1 stage=environment_answer result=decoded_success elapsed_us=17\\n'\n"
                "      exit 0\n"
                "    fi\n"
                "    exit 7\n"
                "    ;;\n"
                "  shell)\n"
                "    command=\"$2\"\n"
                "    case \"$command\" in\n"
                "      *'am start -W'*) exit 0 ;;\n"
                "      pidof)\n"
                "        if [[ \"${DIAG_MODE:-ok}\" == pid_fail ]]; then exit 8; fi\n"
                "        echo 4321; exit 0\n"
                "        ;;\n"
                "      *'test -d files'* )\n"
                "        if [[ \"${DIAG_MODE:-ok}\" == probe_fail ]]; then exit 9; fi\n"
                "        python3 \"$DIAG_RUN_AS_HELPER\" \"$command\" \"$DIAG_APP_DATA_ROOT\"\n"
                "        exit $?\n"
                "        ;;\n"
                "      *'test -s files/task-app/mosaic-state.v1.json'*) exit 1 ;;\n"
                "      *) exit 0 ;;\n"
                "    esac\n"
                "    ;;\n"
                "esac\n"
                "exit 0\n",
                encoding="utf-8",
            )
            adb.chmod(0o755)

            sleep = bin_dir / "sleep"
            sleep.write_text("#!/usr/bin/env bash\nexit 0\n", encoding="utf-8")
            sleep.chmod(0o755)
            timeout = bin_dir / "timeout"
            timeout.write_text(
                f"#!{sys.executable}\n"
                "import os, signal, subprocess, sys, time\n"
                "args = sys.argv[1:]\n"
                "if os.environ.get('DIAG_MODE') == 'real_timeout':\n"
                "    real = os.environ['REAL_TIMEOUT']\n"
                "    if args[:2] == ['--kill-after=1', '5']:\n"
                "        result = subprocess.run([real, *args], check=False)\n"
                "        with open(os.environ['DIAG_REAL_TIMEOUT_STATUS_PATH'], 'w', encoding='ascii') as status_file:\n"
                "            status_file.write(str(result.returncode) + '\\n')\n"
                "        raise SystemExit(result.returncode)\n"
                "    os.execv(real, [real, *args])\n"
                "if args[:2] == ['--kill-after=1', '5']:\n"
                "    if os.environ.get('DIAG_MODE') in ('outer_timeout', 'log_timeout', 'probe_timeout'):\n"
                "        child = subprocess.Popen(args[2:], start_new_session=True)\n"
                "        time.sleep(0.15)\n"
                "        try: os.killpg(child.pid, signal.SIGTERM)\n"
                "        except ProcessLookupError: pass\n"
                "        try: child.wait(timeout=0.15)\n"
                "        except subprocess.TimeoutExpired:\n"
                "            try: os.killpg(child.pid, signal.SIGKILL)\n"
                "            except ProcessLookupError: pass\n"
                "            child.wait()\n"
                "        sys.exit(124)\n"
                "    os.execvpe(args[2], args[2:], os.environ)\n"
                "if args and args[0] == '--foreground': args = args[1:]\n"
                "if len(args) > 1 and args[0] == '1': os.execvpe(args[1], args[1:], os.environ)\n"
                "sys.exit(125)\n",
                encoding="utf-8",
            )
            timeout.chmod(0o755)
            sha256sum = bin_dir / "sha256sum"
            sha256sum.write_text(
                "#!/usr/bin/env bash\n"
                "if [[ \"${DIAG_MODE:-ok}\" == hash_fail ]]; then exit 11; fi\n"
                "printf '%064d  %s\\n' 0 \"$1\"\n",
                encoding="utf-8",
            )
            sha256sum.chmod(0o755)
            temp_tracker = root / "temp-files.txt"
            mktemp = bin_dir / "mktemp"
            mktemp.write_text(
                "#!/usr/bin/env bash\n"
                f"path='{root}/diagnostic.XXXXXX'\n"
                "path=\"${path/XXXXXX/$$}\"\n"
                ": > \"$path\"\n"
                f"printf '%s\\n' \"$path\" >> '{temp_tracker}'\n"
                "printf '%s\\n' \"$path\"\n",
                encoding="utf-8",
            )
            mktemp.chmod(0o755)

            def run(mode: str) -> subprocess.CompletedProcess[str]:
                if state_parent.is_dir():
                    if mode == "probe_absent":
                        state_path.unlink(missing_ok=True)
                    elif mode == "probe_nonempty":
                        state_path.write_bytes(b"{state:true}")
                    else:
                        state_path.write_bytes(b"")
                env = {
                    "PATH": f"{bin_dir}:/usr/bin:/bin",
                    "ANDROID_HOME": str(android_home),
                    "MOSAIC_NATIVE_DIAGNOSTICS_CAPTURE": "1",
                    "DIAG_RUN_AS_HELPER": str(run_as_helper),
                    "DIAG_APP_DATA_ROOT": str(app_data_root),
                    "DIAG_MODE": mode,
                    "DIAG_HANG_PID_PATH": str(root / "hung-adb.pid"),
                    "DIAG_HANG_CHILD_PID_PATH": str(root / "hung-sleep.pid"),
                    "DIAG_HANG_WORKER_STAT_PATH": str(root / "hung-adb.initial-stat"),
                    "DIAG_HANG_CHILD_STAT_PATH": str(root / "hung-sleep.initial-stat"),
                    "DIAG_REAL_TIMEOUT_STATUS_PATH": str(root / "real-timeout.status"),
                }
                if shutil.which("timeout"):
                    env["REAL_TIMEOUT"] = shutil.which("timeout") or "timeout"
                return subprocess.run(
                    [
                        "bash",
                        str(gate),
                        str(apk),
                        "dev.codingadventures.trestle",
                        "task-app",
                        ".MainActivity",
                    ],
                    env=env,
                    capture_output=True,
                    text=True,
                    check=False,
                )

            successful_capture = run("ok")
            self.assertEqual(successful_capture.returncode, 1)
            self.assertIn("diagnostic_state_probe=present_empty", successful_capture.stdout)
            self.assertIn(
                "MOSAIC_NATIVE_DIAG v=1 stage=environment_answer result=decoded_success elapsed_us=17",
                successful_capture.stdout,
            )
            self.assertIn("diagnostic_apk_package_version=dev.codingadventures.trestle 1", successful_capture.stdout)

            probe_failure = run("probe_fail")
            self.assertEqual(probe_failure.returncode, 1)
            self.assertIn("diagnostic_state_probe=probe_failed", probe_failure.stdout)
            self.assertIn("diagnostic_log_capture=complete", probe_failure.stdout)

            probe_timeout = run("probe_timeout")
            self.assertEqual(probe_timeout.returncode, 1)
            self.assertIn("diagnostic_capture=timed_out", probe_timeout.stdout)

            absent_state = run("probe_absent")
            self.assertEqual(absent_state.returncode, 1)
            self.assertIn("diagnostic_state_probe=absent", absent_state.stdout)

            nonempty_state = run("probe_nonempty")
            self.assertEqual(nonempty_state.returncode, 1)
            self.assertIn("diagnostic_state_probe=present_nonempty", nonempty_state.stdout)

            state_path.unlink(missing_ok=True)
            state_parent.rmdir()
            state_parent.write_text("not a directory", encoding="utf-8")
            parent_unavailable = run("probe_parent_unavailable")
            self.assertEqual(parent_unavailable.returncode, 1)
            self.assertIn("diagnostic_state_probe=probe_failed", parent_unavailable.stdout)
            state_parent.unlink()
            state_parent.mkdir()

            log_failure = run("log_fail")
            self.assertEqual(log_failure.returncode, 1)
            self.assertIn("diagnostic_state_probe=present_empty", log_failure.stdout)
            self.assertIn("diagnostic_log_capture=failed", log_failure.stdout)

            wrong_pid = run("log_wrong_pid")
            self.assertEqual(wrong_pid.returncode, 1)
            self.assertIn("diagnostic_host_markers=none_observed_for_current_pid", wrong_pid.stdout)
            self.assertNotIn("elapsed_us=17", wrong_pid.stdout)

            unknown_marker = run("log_unknown")
            self.assertEqual(unknown_marker.returncode, 1)
            self.assertIn("diagnostic_host_markers=none_observed_for_current_pid", unknown_marker.stdout)
            self.assertNotIn("unexpected elapsed_us", unknown_marker.stdout)

            wrong_body = run("log_wrong_body")
            self.assertEqual(wrong_body.returncode, 1)
            self.assertIn("diagnostic_host_markers=none_observed_for_current_pid", wrong_body.stdout)
            self.assertNotIn("elapsed_us=17", wrong_body.stdout)

            trailing_marker = run("log_suffix")
            self.assertEqual(trailing_marker.returncode, 1)
            self.assertIn("diagnostic_host_markers=none_observed_for_current_pid", trailing_marker.stdout)
            self.assertNotIn("trailing", trailing_marker.stdout)

            pid_failure = run("pid_fail")
            self.assertEqual(pid_failure.returncode, 1)
            self.assertIn("diagnostic_pid_probe=unavailable", pid_failure.stdout)
            self.assertIn("diagnostic_host_markers=unavailable_pid_or_log", pid_failure.stdout)

            aapt_failure = run("aapt_fail")
            self.assertEqual(aapt_failure.returncode, 1)
            self.assertIn("diagnostic_apk_package_version=unavailable", aapt_failure.stdout)

            hash_failure = run("hash_fail")
            self.assertEqual(hash_failure.returncode, 1)
            self.assertIn("diagnostic_apk_sha256=unavailable", hash_failure.stdout)
            self.assertIn("diagnostic_pubspec_lock_sha256=unavailable", hash_failure.stdout)

            clipped_log = run("log_large")
            self.assertEqual(clipped_log.returncode, 1)
            self.assertIn("diagnostic_log_capture=clipped", clipped_log.stdout)
            self.assertLessEqual(
                sum("MOSAIC_NATIVE_DIAG v=1" in line for line in clipped_log.stdout.splitlines()),
                24,
            )
            summary_start = clipped_log.stdout.index("---- bounded Flutter Trestle diagnostics")
            summary_end = clipped_log.stdout.index("::error::", summary_start)
            summary = clipped_log.stdout[summary_start:summary_end]
            self.assertLess(len(summary.encode("utf-8")), 64 * 1024)
            self.assertNotIn("lock data", clipped_log.stdout)
            self.assertNotIn(str(apk), clipped_log.stdout)

            timed_out_capture = run("outer_timeout")
            self.assertEqual(timed_out_capture.returncode, 1)
            self.assertIn("diagnostic_capture=timed_out", timed_out_capture.stdout)
            timeout_start = time.monotonic()
            hanging_log = run("log_timeout")
            timeout_elapsed = time.monotonic() - timeout_start
            self.assertEqual(hanging_log.returncode, 1)
            self.assertIn("diagnostic_capture=timed_out", hanging_log.stdout)
            self.assertLess(timeout_elapsed, 2.0)
            recorded_temps = temp_tracker.read_text(encoding="utf-8").splitlines()
            self.assertTrue(recorded_temps)
            self.assertTrue(all(not Path(path).exists() for path in recorded_temps))

            real_timeout = shutil.which("timeout")
            if real_timeout:
                real_start = time.monotonic()
                real_timeout_gate = run("real_timeout")
                real_elapsed = time.monotonic() - real_start
                self.assertEqual(real_timeout_gate.returncode, 1)
                self.assertIn("diagnostic_capture=timed_out", real_timeout_gate.stdout)
                self.assertLess(real_elapsed, 7.5)
                real_timeout_status = (root / "real-timeout.status").read_text(
                    encoding="ascii"
                ).strip()
                hung_pid = int((root / "hung-adb.pid").read_text(encoding="utf-8").strip())
                hung_child_pid = int((root / "hung-sleep.pid").read_text(encoding="utf-8").strip())

                def proc_identity(raw_stat: str) -> dict[str, str | int]:
                    close_paren = raw_stat.rfind(")")
                    self.assertGreaterEqual(close_paren, 0, raw_stat)
                    fields = raw_stat[close_paren + 2 :].split()
                    self.assertGreaterEqual(len(fields), 20, raw_stat)
                    return {
                        "state": fields[0],
                        "ppid": int(fields[1]),
                        "pgrp": int(fields[2]),
                        "session": int(fields[3]),
                        "starttime": int(fields[19]),
                    }

                initial_worker = proc_identity(
                    (root / "hung-adb.initial-stat").read_text(encoding="ascii")
                )
                initial_child = proc_identity(
                    (root / "hung-sleep.initial-stat").read_text(encoding="ascii")
                )

                def snapshot_process(pid: int) -> dict[str, str | int] | None:
                    try:
                        raw_stat = Path(f"/proc/{pid}/stat").read_text(encoding="ascii")
                    except FileNotFoundError:
                        return None
                    current = proc_identity(raw_stat)
                    expected = initial_worker if pid == hung_pid else initial_child
                    if current["starttime"] != expected["starttime"]:
                        return {"state": "pid_reused", **current}
                    return current

                def group_snapshot(expected_pgrp: int) -> list[dict[str, str | int]]:
                    members: list[dict[str, str | int]] = []
                    for proc_entry in Path("/proc").iterdir():
                        if not proc_entry.name.isdigit():
                            continue
                        try:
                            raw_stat = (proc_entry / "stat").read_text(encoding="ascii")
                            current = proc_identity(raw_stat)
                        except (FileNotFoundError, PermissionError, ValueError):
                            continue
                        if current["pgrp"] == expected_pgrp:
                            members.append({"pid": int(proc_entry.name), **current})
                    return members[:32]

                process_history: list[dict[str, object]] = []
                observation_start = time.monotonic()
                for checkpoint in (0.0, 0.05, 0.20, 0.50):
                    remaining = checkpoint - (time.monotonic() - observation_start)
                    if remaining > 0:
                        time.sleep(remaining)
                    process_history.append(
                        {
                            "elapsed_s": round(time.monotonic() - observation_start, 3),
                            "worker": snapshot_process(hung_pid),
                            "sleep_child": snapshot_process(hung_child_pid),
                            "group": group_snapshot(int(initial_worker["pgrp"])),
                        }
                    )
                self.assertEqual(
                    real_timeout_status,
                    "124",
                    f"real outer timeout status={real_timeout_status!r}; process_history={process_history!r}",
                )
                try:
                    os.kill(hung_pid, 0)
                except ProcessLookupError:
                    pass
                else:
                    self.fail(
                        "term-resistant adb worker PID remained addressable after timeout; "
                        f"initial_worker={initial_worker!r}, initial_sleep_child={initial_child!r}, "
                        f"process_history={process_history!r}"
                    )
                recorded_temps = temp_tracker.read_text(encoding="utf-8").splitlines()
                self.assertTrue(all(not Path(path).exists() for path in recorded_temps))

    @unittest.skipUnless(shutil.which("timeout"), "GNU timeout is unavailable on this host")
    def test_gnu_timeout_kills_term_resistant_process_group(self) -> None:
        start = time.monotonic()
        with tempfile.TemporaryDirectory() as directory:
            pid_file = Path(directory) / "child.pid"
            child = (
                "trap '' TERM; printf '%s\\n' \"$$\" > \""
                + str(pid_file)
                + "\"; while :; do /bin/sleep 0.05; done"
            )
            result = subprocess.run(
                [
                    shutil.which("timeout") or "timeout",
                    "--kill-after=0.2",
                    "0.2",
                    "bash",
                    "-c",
                    child,
                ],
                capture_output=True,
                check=False,
            )
            elapsed = time.monotonic() - start
            self.assertIn(result.returncode, (124, 137, -signal.SIGKILL))
            self.assertLess(elapsed, 2.0)
            pid = int(pid_file.read_text(encoding="utf-8").strip())
            with self.assertRaises(ProcessLookupError):
                os.kill(pid, 0)

    def test_a_lane_script_change_alone_requires_acceptance(self) -> None:
        """The Android scripts belong to no package; changing one must still
        run the lane that executes it."""

        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)

            def git(*arguments: str) -> None:
                subprocess.run(
                    [
                        "git",
                        # A `git commit` may start a detached `gc --auto` /
                        # `maintenance run --auto` that is still writing into
                        # `.git` when TemporaryDirectory removes it, which fails
                        # cleanup with "Directory not empty: '.git'".  Throwaway
                        # repositories never need maintenance, so turn it off.
                        "-c", "gc.auto=0", "-c", "maintenance.auto=false",
                        "-c", "user.name=t", "-c", "user.email=t@example.com",
                        *arguments,
                    ],
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


# The Linux Mosaic lanes (Qt, Flutter, Compose, Android) run in a job of their
# own, `mosaic-linux-runtime`, not as steps of `build`. `build` is main's
# 15-shard matrix, and a step there runs on every shard: these lanes used to
# cost ~520 runner-minutes per merge and ~18 GB of disk on every shard. Moving
# one back into `build` would still pass every test above, so this pins where
# they live, and that CI gate still requires them.
LINUX_LANE_STEPS = (
    "Round-trip Rust engine through standard Qt binding",
    "Round-trip Rust engine through standard Flutter binding",
    "Round-trip Rust engine through standard Compose binding",
    "Build Trestle for Android (UI89 step 4)",
    "Build Trestle for Android with its Rust runtime (UI89 step 5)",
    "Build Journal for Android with its Rust runtime (UI89 step 7)",
    "Build Engram for Android with its Rust runtime (UI89 step 7)",
    "Build Trestle, Journal and Engram for Android with Flutter (UI89 §7)",
    "Launch Trestle, Journal and Engram on an Android emulator (UI89 steps 5, 7)",
)


def job_text(workflow: str, job: str) -> str:
    """The text of one top-level job: from its key to the next job's key."""

    start = workflow.index(f"\n  {job}:\n")
    following = re.search(r"\n  [a-z0-9-]+:\n", workflow[start + 1 :])
    end = start + 1 + following.start() if following else len(workflow)
    return workflow[start:end]


class MosaicLinuxRuntimeJobTests(unittest.TestCase):
    def setUp(self) -> None:
        self.workflow = WORKFLOW.read_text(encoding="utf-8")
        self.lane_job = job_text(self.workflow, "mosaic-linux-runtime")
        self.build_job = job_text(self.workflow, "build")

    def test_every_linux_lane_runs_in_its_own_job_not_in_the_shards(self) -> None:
        for step in LINUX_LANE_STEPS:
            with self.subTest(step=step):
                self.assertEqual(self.lane_job.count(f"- name: {step}\n"), 1)
                self.assertNotIn(f"- name: {step}\n", self.build_job)

    def test_the_job_runs_when_any_linux_lane_is_required(self) -> None:
        condition = re.search(r"\n    if: (.*)\n", self.lane_job)
        self.assertIsNotNone(condition)
        for lane in ("qt", "flutter", "compose"):
            with self.subTest(lane=lane):
                self.assertIn(
                    f"needs.detect.outputs.needs_mosaic_{lane}_runtime == 'true'",
                    condition.group(1),
                )
        self.assertNotIn("matrix.", self.lane_job)

    def test_build_still_installs_what_flutter_on_linux_needs(self) -> None:
        """Package builds run `flutter build linux` whenever Flutter is on
        PATH, and that needs GTK 3's headers. The Flutter lane used to
        install them inside `build`; when it moved out, venture-browser
        failed on a shard with `gtk+-3.0` not found. So wherever `build`
        installs Flutter on Linux, it must install GTK too, after Flutter
        and before the package build."""

        flutter = self.build_job.index("      - name: Set up Flutter\n")
        install = self.build_job.index("      - name: Install Flutter Linux desktop dependencies\n")
        step = self.build_job[install : self.build_job.index("\n      - name:", install)]
        self.assertIn(
            "if: needs.detect.outputs.needs_mosaic_flutter_runtime == 'true' && runner.os == 'Linux'\n",
            step,
        )
        self.assertIn("bash code/scripts/ci/apt-install.sh libgtk-3-dev xvfb", step)
        self.assertLess(flutter, install)
        self.assertLess(install, self.build_job.index("      - name: Full build on main merge\n"))
        self.assertLess(install, self.build_job.index("      - name: Build and test affected packages\n"))

    def test_ci_gate_requires_the_job(self) -> None:
        gate = job_text(self.workflow, "ci-gate")
        needs = re.search(r"\n    needs: \[(.*)\]\n", gate)
        self.assertIsNotNone(needs)
        self.assertIn("mosaic-linux-runtime", [n.strip() for n in needs.group(1).split(",")])
        self.assertIn(
            "MOSAIC_LINUX_RUNTIME_RESULT: ${{ needs['mosaic-linux-runtime'].result }}", gate
        )
        loop = re.search(r"\n          for r in (.*); do\n", gate)
        self.assertIsNotNone(loop)
        self.assertIn('"$MOSAIC_LINUX_RUNTIME_RESULT"', loop.group(1))


if __name__ == "__main__":
    unittest.main()
