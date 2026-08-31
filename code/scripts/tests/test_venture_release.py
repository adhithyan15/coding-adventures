from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[3]
SCRIPT_PATH = REPO_ROOT / "code/scripts/venture_release.py"
SPEC = importlib.util.spec_from_file_location("venture_release", SCRIPT_PATH)
assert SPEC is not None and SPEC.loader is not None
venture_release = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(venture_release)


class VentureReleaseTests(unittest.TestCase):
    def test_repository_versions_form_the_0_9_1_release(self) -> None:
        self.assertEqual(
            venture_release.validate_release(REPO_ROOT, "venture-v0.9.1"),
            "0.9.1",
        )

    def test_rejects_a_mismatched_tag(self) -> None:
        with self.assertRaisesRegex(ValueError, "release tag must be venture-v0.9.1"):
            venture_release.validate_release(REPO_ROOT, "venture-v0.8.0")

    def test_semver_parser_rejects_numeric_prerelease_leading_zeroes(self) -> None:
        self.assertIsNone(venture_release.SEMVER.fullmatch("0.9.0-01"))
        self.assertIsNotNone(venture_release.SEMVER.fullmatch("0.9.0-rc.1"))

    def test_rejects_a_stable_version_before_readiness(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for relative_path in venture_release.MANIFESTS:
                path = root / relative_path
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text('[package]\nversion = "1.0.0"\n', encoding="utf-8")
            version_file = root / venture_release.VERSION_FILE
            version_file.parent.mkdir(parents=True, exist_ok=True)
            version_file.write_text("1.0.0\n", encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "pre-1.0"):
                venture_release.validate_release(root)


if __name__ == "__main__":
    unittest.main()
