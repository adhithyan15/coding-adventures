from __future__ import annotations

import hashlib
import importlib.util
import os
import subprocess
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path


RECEIPT = Path(__file__).resolve().parents[1] / "mosaic_flutter_android_build_receipt.py"
RECEIPT_SPEC = importlib.util.spec_from_file_location("mosaic_flutter_android_build_receipt", RECEIPT)
assert RECEIPT_SPEC is not None and RECEIPT_SPEC.loader is not None
RECEIPT_MODULE = importlib.util.module_from_spec(RECEIPT_SPEC)
RECEIPT_SPEC.loader.exec_module(RECEIPT_MODULE)


class MosaicFlutterAndroidBuildReceiptTests(unittest.TestCase):
    def _fixture(self, root: Path, *, member: bool, dex: str, aapt_ok: bool = True):
        apk = root / "app.apk"
        with zipfile.ZipFile(apk, "w") as archive:
            if member:
                archive.writestr("lib/x86_64/libdartjni.so", b"jni")
        lock = root / "pubspec.lock"
        lock.write_text("path_provider: 2.1.6\n", encoding="utf-8")
        registrant = root / "dart_plugin_registrant.dart"
        registrant.write_text(
            "import 'dart:io';\n"
            "import 'package:path_provider_android/path_provider_android.dart' as path_provider_android;\n"
            "class _PluginRegistrant {\n"
            "  static void register() {\n"
            "    if (Platform.isAndroid) {\n"
            "      try {\n"
            "        path_provider_android.PathProviderAndroid.registerWith();\n"
            "      } catch (_) {\n"
            "      }\n"
            "    } else if (Platform.isIOS) {\n"
            "    }\n"
            "  }\n"
            "}\n",
            encoding="utf-8",
        )
        tools = root / "tools"
        tools.mkdir()
        aapt = tools / "aapt2"
        if aapt_ok:
            aapt.write_text(
                "#!/bin/sh\nprintf \"package: name='dev.codingadventures.trestle' versionCode='1'\\n\"\n",
                encoding="utf-8",
            )
        else:
            aapt.write_text("#!/bin/sh\nexit 9\n", encoding="utf-8")
        aapt.chmod(0o755)
        analyzer = tools / "apkanalyzer"
        analyzer.write_text(
            "#!/bin/sh\n"
            "[ \"$5\" = --method ] || exit 42\n"
            "[ \"$6\" = 'registerWith(Lio/flutter/embedding/engine/FlutterEngine;)V' ] || exit 43\n"
            + f"cat <<'DEX'\n{dex}DEX\n",
            encoding="utf-8",
        )
        analyzer.chmod(0o755)
        return apk, lock, registrant, aapt, tools

    def _run(self, root: Path, apk: Path, lock: Path, registrant: Path, aapt: Path, tools: Path):
        env = {**os.environ, "PATH": f"{tools}:{os.environ.get('PATH', '')}"}
        return subprocess.run(
            [sys.executable, str(RECEIPT), str(apk), str(lock), str(registrant), str(aapt), ""],
            capture_output=True,
            text=True,
            check=False,
            env=env,
        )

    def test_reports_hashed_apk_lock_and_registration_evidence_without_dex_text(self) -> None:
        dex = """.method public static registerWith(Lio/flutter/embedding/engine/FlutterEngine;)V
new-instance v0, Lcom/github/dart_lang/jni/JniPlugin;
invoke-direct {v0}, Lcom/github/dart_lang/jni/JniPlugin;-><init>()V
new-instance v1, Lcom/github/dart_lang/jni_flutter/JniFlutterPlugin;
invoke-direct {v1}, Lcom/github/dart_lang/jni_flutter/JniFlutterPlugin;-><init>()V
.end method
"""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            apk, lock, registrant, aapt, tools = self._fixture(root, member=True, dex=dex)
            result = self._run(root, apk, lock, registrant, aapt, tools)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn(f"apk_sha256={hashlib.sha256(apk.read_bytes()).hexdigest()}", result.stdout)
            self.assertIn(f"lock_sha256={hashlib.sha256(lock.read_bytes()).hexdigest()}", result.stdout)
            self.assertIn("package_id=dev.codingadventures.trestle", result.stdout)
            self.assertIn("x86_64_libdartjni=present", result.stdout)
            self.assertIn("dart_registrant_source=present", result.stdout)
            self.assertIn("dex_jni_plugin=present", result.stdout)
            self.assertIn("dex_jni_flutter_plugin=present", result.stdout)
            self.assertNotIn("new-instance", result.stdout)

    def test_reports_absent_evidence_separately_from_unavailable_tools(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            # Merely mentioning the classes as DEX strings is not executable
            # construction, and the format is too ambiguous to call absent.
            dex_names_only = """.method public static registerWith(Lio/flutter/embedding/engine/FlutterEngine;)V
new-instance v0, Lother/RegisteredPlugin;
invoke-direct {v0}, Lother/RegisteredPlugin;-><init>()V
const-string v1, "Lcom/github/dart_lang/jni/JniPlugin;"
const-string v2, "Lcom/github/dart_lang/jni_flutter/JniFlutterPlugin;"
.end method
"""
            apk, lock, registrant, aapt, tools = self._fixture(
                root, member=False, dex=dex_names_only
            )
            registrant.write_text("// no generated registration\n", encoding="utf-8")
            result = self._run(root, apk, lock, registrant, aapt, tools)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("x86_64_libdartjni=absent", result.stdout)
            self.assertIn("dart_registrant_source=absent", result.stdout)
            self.assertIn("dex_jni_plugin=unavailable", result.stdout)
            self.assertIn("dex_jni_flutter_plugin=unavailable", result.stdout)

    def test_generated_registrant_requires_expected_alias_and_android_arm(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            registrant = root / "registrant.dart"
            registrant.write_text(
                "/* import 'package:path_provider_android/path_provider_android.dart' as path_provider_android;\n"
                "   if (Platform.isAndroid) {\n"
                "     path_provider_android.PathProviderAndroid.registerWith();\n"
                "   } */\n"
                "// path_provider_android.PathProviderAndroid.registerWith();\n",
                encoding="utf-8",
            )
            self.assertEqual(RECEIPT_MODULE.generated_registrant_status(registrant), "absent")

            registrant.write_text(
                "import 'package:path_provider_android/path_provider_android.dart' as path_provider_android;\n"
                "if (Platform.isAndroid) {\n"
                "} else if (Platform.isIOS) {\n"
                "  path_provider_android.PathProviderAndroid.registerWith();\n"
                "} else if (Platform.isLinux) {\n}\n",
                encoding="utf-8",
            )
            self.assertEqual(RECEIPT_MODULE.generated_registrant_status(registrant), "absent")

            registrant.write_text(
                "import 'package:path_provider_android/path_provider_android.dart' as pp;\n"
                "if (Platform.isAndroid) {\n"
                "  pp.PathProviderAndroid.registerWith();\n"
                "} else if (Platform.isIOS) {\n}\n",
                encoding="utf-8",
            )
            self.assertEqual(RECEIPT_MODULE.generated_registrant_status(registrant), "unavailable")

    def test_dex_receipt_uses_only_selected_register_method_and_known_smali(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            apk, _, _, _, tools = self._fixture(root, member=True, dex="")
            analyzer = tools / "apkanalyzer"
            analyzer.write_text(
                "#!/bin/sh\n"
                "[ \"$5\" = --method ] || exit 42\n"
                "[ \"$6\" = 'registerWith(Lio/flutter/embedding/engine/FlutterEngine;)V' ] || exit 43\n"
                "cat <<'DEX'\n"
                ".method public static otherMethod()V\n"
                "new-instance v0, Lcom/github/dart_lang/jni/JniPlugin;\n"
                "invoke-direct {v0}, Lcom/github/dart_lang/jni/JniPlugin;-><init>()V\n"
                ".end method\n"
                ".method public static registerWith(Lio/flutter/embedding/engine/FlutterEngine;)V\n"
                "new-instance v1, Lother/RegisteredPlugin;\n"
                "invoke-direct {v1}, Lother/RegisteredPlugin;-><init>()V\n"
                ".end method\n"
                "DEX\n",
                encoding="utf-8",
            )
            analyzer.chmod(0o755)
            result = self._run(root, apk, root / "missing.lock", root / "missing.dart", root / "missing-aapt", tools)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("dex_jni_plugin=absent", result.stdout)

            analyzer.write_text(
                "#!/bin/sh\n"
                "[ \"$5\" = --method ] || exit 42\n"
                "[ \"$6\" = 'registerWith(Lio/flutter/embedding/engine/FlutterEngine;)V' ] || exit 43\n"
                "cat <<'DEX'\n"
                ".method public static registerWith()V\n"
                "new-instance v0, Lcom/github/dart_lang/jni/JniPlugin;\n"
                "invoke-direct {v0}, Lcom/github/dart_lang/jni/JniPlugin;-><init>()V\n"
                ".end method\n"
                "DEX\n",
                encoding="utf-8",
            )
            analyzer.chmod(0o755)
            result = self._run(root, apk, root / "missing.lock", root / "missing.dart", root / "missing-aapt", tools)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("dex_jni_plugin=unavailable", result.stdout)

            analyzer.write_text(
                "#!/bin/sh\n"
                "[ \"$5\" = --method ] || exit 42\n"
                "[ \"$6\" = 'registerWith(Lio/flutter/embedding/engine/FlutterEngine;)V' ] || exit 43\n"
                "cat <<'DEX'\n"
                ".method public static registerWith(Lio/flutter/embedding/engine/FlutterEngine;)V\n"
                "new-instance v0, Lother/RegisteredPlugin;\n"
                "invoke-direct {v0}, Lother/RegisteredPlugin;-><init>()V\n"
                "const-string v1, 'new-instance v2, Lcom/github/dart_lang/jni/JniPlugin;'\n"
                ".end method\n"
                "DEX\n",
                encoding="utf-8",
            )
            analyzer.chmod(0o755)
            result = self._run(root, apk, root / "missing.lock", root / "missing.dart", root / "missing-aapt", tools)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("dex_jni_plugin=unavailable", result.stdout)

            analyzer.write_text(
                "#!/bin/sh\n"
                "[ \"$5\" = --method ] || exit 42\n"
                "[ \"$6\" = 'registerWith(Lio/flutter/embedding/engine/FlutterEngine;)V' ] || exit 43\n"
                "cat <<'DEX'\n"
                ".method public static registerWith(Lio/flutter/embedding/engine/FlutterEngine;)V\n"
                "new-instance v0, Lcom/github/dart_lang/jni/JniPlugin;\n"
                "invoke-direct/range {v0 .. v0}, Lcom/github/dart_lang/jni/JniPlugin;-><init>()V\n"
                ".end method\n"
                "DEX\n",
                encoding="utf-8",
            )
            analyzer.chmod(0o755)
            result = self._run(root, apk, root / "missing.lock", root / "missing.dart", root / "missing-aapt", tools)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("dex_jni_plugin=unavailable", result.stdout)

            analyzer.write_text(
                "#!/bin/sh\n"
                "[ \"$5\" = --method ] || exit 42\n"
                "[ \"$6\" = 'registerWith(Lio/flutter/embedding/engine/FlutterEngine;)V' ] || exit 43\n"
                "cat <<'DEX'\n"
                ".method public static registerWith(Lio/flutter/embedding/engine/FlutterEngine;)V\n"
                "new-instance v0, Lother/RegisteredPlugin;\n"
                "invoke-direct {v0}, Lother/RegisteredPlugin;-><init>()V\n"
                ".end method\n"
                "DEX\n",
                encoding="utf-8",
            )
            analyzer.chmod(0o755)
            result = self._run(root, apk, root / "missing.lock", root / "missing.dart", root / "missing-aapt", tools)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("dex_jni_plugin=absent", result.stdout)

    def test_receipt_tools_have_bounded_and_decoding_failure_results(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            apk, _, _, aapt, tools = self._fixture(root, member=True, dex="")
            old_stdout_limit = RECEIPT_MODULE.MAX_DEX_OUTPUT
            old_timeout = RECEIPT_MODULE.TOOL_TIMEOUT_SECONDS
            try:
                RECEIPT_MODULE.MAX_DEX_OUTPUT = 128
                analyzer = tools / "apkanalyzer"
                analyzer.write_text(
                    "#!/usr/bin/env python3\n"
                    "import sys\n"
                    "sys.stdout.buffer.write(b'x' * 4096)\n",
                    encoding="utf-8",
                )
                analyzer.chmod(0o755)
                self.assertEqual(
                    RECEIPT_MODULE.dex_registrant_statuses(apk, ""),
                    ("unavailable", "unavailable"),
                )

                aapt.write_text(
                    "#!/usr/bin/env python3\n"
                    "import sys\n"
                    "sys.stdout.buffer.write(b'\\xff')\n",
                    encoding="utf-8",
                )
                aapt.chmod(0o755)
                self.assertEqual(RECEIPT_MODULE.package_id_status(aapt, apk), "unavailable")

                oversized = root / "oversized-tool"
                oversized.write_text(
                    "#!/usr/bin/env python3\n"
                    "import sys\n"
                    "sys.stderr.buffer.write(b'e' * 70000)\n",
                    encoding="utf-8",
                )
                oversized.chmod(0o755)
                self.assertIsNone(RECEIPT_MODULE._run_bounded([str(oversized)], 1024))

                stdout_oversized = root / "stdout-oversized"
                stdout_oversized.write_text(
                    "#!/usr/bin/env python3\n"
                    "import sys\n"
                    "sys.stdout.buffer.write(b'o' * 1024)\n",
                    encoding="utf-8",
                )
                stdout_oversized.chmod(0o755)
                self.assertIsNone(RECEIPT_MODULE._run_bounded([str(stdout_oversized)], 128))

                sleeper = root / "sleeper"
                sleeper.write_text("#!/bin/sh\nsleep 10\n", encoding="utf-8")
                sleeper.chmod(0o755)
                RECEIPT_MODULE.TOOL_TIMEOUT_SECONDS = 0.1
                self.assertIsNone(RECEIPT_MODULE._run_bounded([str(sleeper)], 1024))
            finally:
                RECEIPT_MODULE.MAX_DEX_OUTPUT = old_stdout_limit
                RECEIPT_MODULE.TOOL_TIMEOUT_SECONDS = old_timeout

    def test_dex_unbalanced_or_unrecognized_method_body_is_unavailable(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            apk, _, _, _, tools = self._fixture(root, member=True, dex="")
            analyzer = tools / "apkanalyzer"
            selected_header = ".method public static registerWith(Lio/flutter/embedding/engine/FlutterEngine;)V"
            cases = (
                # The selected method cannot borrow another method's terminator.
                selected_header
                + "\n.method public static other()V\n"
                + "new-instance v0, Lcom/github/dart_lang/jni/JniPlugin;\n"
                + "invoke-direct {v0}, Lcom/github/dart_lang/jni/JniPlugin;-><init>()V\n"
                + ".end method\n",
                # Opcode-shaped text inside a string is not an instruction.
                selected_header
                + '\nconst-string v0, "new-instance v1, Lcom/github/dart_lang/jni/JniPlugin;"\n'
                + ".end method\n",
                # Duplicate selected bodies make the tool output ambiguous.
                selected_header
                + "\nnew-instance v0, Lother/Plugin;\n.end method\n"
                + selected_header
                + "\nnew-instance v1, Lother/Plugin;\n.end method\n",
            )
            for dex in cases:
                with self.subTest(dex=dex):
                    analyzer.write_text(
                        "#!/bin/sh\n"
                        "cat <<'DEX'\n"
                        + dex
                        + "DEX\n",
                        encoding="utf-8",
                    )
                    analyzer.chmod(0o755)
                    result = self._run(
                        root,
                        apk,
                        root / "missing.lock",
                        root / "missing.dart",
                        root / "missing-aapt",
                        tools,
                    )
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertIn("dex_jni_plugin=unavailable", result.stdout)
                    self.assertIn("dex_jni_flutter_plugin=unavailable", result.stdout)

    def test_bounded_tool_reader_rejects_output_caps_timeout_and_bad_text(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            noisy = root / "noisy"
            noisy.write_text(
                "#!/bin/sh\nprintf '%s' '" + ("x" * 256) + "'\n", encoding="utf-8"
            )
            noisy.chmod(0o755)
            old_stderr_limit = RECEIPT_MODULE.MAX_TOOL_STDERR
            old_timeout = RECEIPT_MODULE.TOOL_TIMEOUT_SECONDS
            try:
                RECEIPT_MODULE.MAX_TOOL_STDERR = 128
                self.assertIsNone(RECEIPT_MODULE._run_bounded([str(noisy)], 128))

                stderr_noisy = root / "stderr-noisy"
                stderr_noisy.write_text(
                    "#!/bin/sh\nprintf '%s' '" + ("e" * 256) + "' >&2\n", encoding="utf-8"
                )
                stderr_noisy.chmod(0o755)
                self.assertIsNone(RECEIPT_MODULE._run_bounded([str(stderr_noisy)], 1024))

                sleeper = root / "sleeper"
                sleeper.write_text("#!/bin/sh\nsleep 10\n", encoding="utf-8")
                sleeper.chmod(0o755)
                RECEIPT_MODULE.TOOL_TIMEOUT_SECONDS = 0.1
                self.assertIsNone(RECEIPT_MODULE._run_bounded([str(sleeper)], 1024))
            finally:
                RECEIPT_MODULE.MAX_TOOL_STDERR = old_stderr_limit
                RECEIPT_MODULE.TOOL_TIMEOUT_SECONDS = old_timeout


    def test_receipt_reports_unavailable_when_inputs_or_tools_cannot_be_read(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            tools = root / "tools"
            tools.mkdir()
            invalid_apk = root / "invalid.apk"
            invalid_apk.write_text("not a zip", encoding="utf-8")
            lock = root / "pubspec.lock"
            lock.write_text("path_provider: 2.1.6\n", encoding="utf-8")
            missing_registrant = root / "missing.dart"
            bad_aapt = tools / "aapt2"
            bad_aapt.write_text("#!/bin/sh\nexit 1\n", encoding="utf-8")
            bad_aapt.chmod(0o755)
            result = self._run(root, invalid_apk, lock, missing_registrant, bad_aapt, tools)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("apk_sha256=", result.stdout)
            self.assertIn("x86_64_libdartjni=unavailable", result.stdout)
            self.assertIn("package_id=unavailable", result.stdout)
            self.assertIn("dart_registrant_source=unavailable", result.stdout)
            self.assertIn("dex_jni_plugin=unavailable", result.stdout)
            self.assertIn("dex_jni_flutter_plugin=unavailable", result.stdout)


if __name__ == "__main__":
    unittest.main()
