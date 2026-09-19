import importlib.util
from pathlib import Path
import tempfile
import unittest
import zipfile
import json
import hashlib

spec = importlib.util.spec_from_file_location("package_web", Path(__file__).with_name("package-web.py"))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class ReleasePackageTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.dist = self.root / "dist"
        self.dist.mkdir()
        (self.dist / "index.html").write_text('<script src="/assets/app.js"></script>')
        (self.dist / "assets").mkdir()
        (self.dist / "assets/app.js").write_text("console.log('VisiCalc')")
        (self.dist / "visicalc_mosaic_app.wasm").write_bytes(b"\0asm\x01\0\0\0")

    def build(self, output="out", version="0.1.0-preview.1"):
        return module.package(self.dist, self.root / output, version, "a" * 40)

    def test_reproducible_archive_and_complete_checksums(self):
        first = self.build()
        second = self.build("second")
        self.assertEqual(first.read_bytes(), second.read_bytes())
        checksum = (first.parent / "SHA256SUMS").read_text().split()[0]
        self.assertEqual(checksum, hashlib.sha256(first.read_bytes()).hexdigest())
        with zipfile.ZipFile(first) as bundle:
            manifest = json.loads(bundle.read("release.json"))
            self.assertEqual(set(bundle.namelist()), set(manifest["files"]) | {"release.json"})
            for name, digest in manifest["files"].items():
                self.assertEqual(hashlib.sha256(bundle.read(name)).hexdigest(), digest)

    def test_existing_archive_is_never_replaced(self):
        first = self.build()
        original = first.read_bytes()
        with self.assertRaises(FileExistsError):
            self.build()
        self.assertEqual(original, first.read_bytes())

    def test_rejects_missing_runtime_and_invalid_version(self):
        with self.assertRaises(ValueError):
            self.build(version="../../elsewhere")
        (self.dist / "visicalc_mosaic_app.wasm").unlink()
        with self.assertRaises(ValueError):
            self.build()

    def test_rejects_metadata_collision(self):
        (self.dist / "release.json").write_text("{}")
        with self.assertRaises(ValueError):
            self.build()

    def test_rejects_symlink_to_outside_distribution(self):
        outside = self.root / "private.txt"
        outside.write_text("not part of the release")
        try:
            (self.dist / "linked.txt").symlink_to(outside)
        except OSError:
            self.skipTest("this host requires privileges to create symlinks")
        with self.assertRaises(ValueError):
            self.build()


if __name__ == "__main__":
    unittest.main()
