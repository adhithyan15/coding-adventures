from __future__ import annotations

import hashlib
import os
import subprocess
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path


RECEIPT = Path(__file__).resolve().parents[1] / "mosaic_flutter_android_build_receipt.py"


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
            "import 'package:path_provider_android/path_provider_android.dart';\n"
            "PathProviderAndroid.registerWith();\n",
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
        analyzer.write_text("#!/bin/sh\n" + f"cat <<'DEX'\n{dex}DEX\n", encoding="utf-8")
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
        dex = """registerWith(FlutterEngine)
new-instance v0, Lcom/github/dart_lang/jni/JniPlugin;
invoke-direct {v0}, Lcom/github/dart_lang/jni/JniPlugin;-><init>()V
new-instance v1, Lcom/github/dart_lang/jni_flutter/JniFlutterPlugin;
invoke-direct {v1}, Lcom/github/dart_lang/jni_flutter/JniFlutterPlugin;-><init>()V
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
            # construction in GeneratedPluginRegistrant.
            dex_names_only = """registerWith(FlutterEngine)
new-instance v0, Lother/RegisteredPlugin;
invoke-direct {v0}, Lother/RegisteredPlugin;-><init>()V
const-string v1, "Lcom/github/dart_lang/jni/JniPlugin;"
const-string v2, "Lcom/github/dart_lang/jni_flutter/JniFlutterPlugin;"
"""
            apk, lock, registrant, aapt, tools = self._fixture(
                root, member=False, dex=dex_names_only
            )
            registrant.write_text("// no generated registration\n", encoding="utf-8")
            result = self._run(root, apk, lock, registrant, aapt, tools)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("x86_64_libdartjni=absent", result.stdout)
            self.assertIn("dart_registrant_source=absent", result.stdout)
            self.assertIn("dex_jni_plugin=absent", result.stdout)
            self.assertIn("dex_jni_flutter_plugin=absent", result.stdout)

            invalid_apk = root / "invalid.apk"
            invalid_apk.write_text("not a zip", encoding="utf-8")
            missing_registrant = root / "missing.dart"
            bad_aapt = tools / "bad-aapt"
            bad_aapt.write_text("#!/bin/sh\nexit 1\n", encoding="utf-8")
            bad_aapt.chmod(0o755)
            (tools / "apkanalyzer").unlink()
            empty_path = root / "empty-path"
            empty_path.mkdir()
            unavailable = self._run(
                root, invalid_apk, lock, missing_registrant, bad_aapt, empty_path
            )
            self.assertEqual(unavailable.returncode, 0, unavailable.stderr)
            self.assertIn("apk_sha256=", unavailable.stdout)
            self.assertIn("x86_64_libdartjni=unavailable", unavailable.stdout)
            self.assertIn("package_id=unavailable", unavailable.stdout)
            self.assertIn("dart_registrant_source=unavailable", unavailable.stdout)
            self.assertIn("dex_jni_plugin=unavailable", unavailable.stdout)
            self.assertIn("dex_jni_flutter_plugin=unavailable", unavailable.stdout)


if __name__ == "__main__":
    unittest.main()
