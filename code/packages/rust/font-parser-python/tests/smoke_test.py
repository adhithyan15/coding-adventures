"""Smoke test: import the freshly built extension into a real CPython and call it.

font-parser-python has no Rust unit tests.  It is a thin shim over the
font-parser crate, and everything that can go wrong in the shim lives at the
boundary: a Python C API symbol that does not link, a reference count that is
wrong, an exception that is not raised.  Those only show up when an interpreter
actually loads the compiled library, so that is what this does::

    cargo build --release -p font-parser-python --target-dir target
    python3 tests/smoke_test.py

On Windows it also proves that build.rs found ``pythonXY.lib`` and that the
resulting .pyd really loads.

The library is copied to a temporary directory under the name Python expects
(``font_parser_native.pyd`` on Windows, ``font_parser_native.so`` elsewhere)
because cargo's file name differs per platform (lib*.so, lib*.dylib, *.dll).
"""

import importlib
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

PACKAGE_DIR = Path(__file__).resolve().parent.parent
FIXTURE = PACKAGE_DIR.parent.parent.parent / "fixtures" / "fonts" / "Inter-Regular.ttf"


def _built_library() -> Path:
    # The recipe passes `--target-dir target`, deliberately NOT the shared
    # workspace target: font-parser-node, -python and -ruby all emit a cdylib
    # named font_parser_native and would overwrite each other's file.
    target_dir = PACKAGE_DIR / "target"
    name = {
        "win32": "font_parser_native.dll",
        "darwin": "libfont_parser_native.dylib",
    }.get(sys.platform, "libfont_parser_native.so")
    built = target_dir / "release" / name
    if not built.exists():
        raise FileNotFoundError(f"built extension not found at {built}; run the cargo build first")
    return built


def _import_extension():
    staging = Path(tempfile.mkdtemp(prefix="font-parser-python-"))
    suffix = ".pyd" if sys.platform == "win32" else ".so"
    shutil.copyfile(_built_library(), staging / f"font_parser_native{suffix}")
    sys.path.insert(0, str(staging))
    return importlib.import_module("font_parser_native")


fp = _import_extension()
FONT_BYTES = FIXTURE.read_bytes()


class FontParserPythonSmokeTest(unittest.TestCase):
    def test_exports_the_documented_surface(self):
        for name in ("load", "font_metrics", "glyph_id", "glyph_metrics", "kerning"):
            self.assertTrue(callable(getattr(fp, name)), name)

    def test_font_metrics_reads_inter(self):
        font = fp.load(FONT_BYTES)
        self.assertEqual(
            fp.font_metrics(font),
            {
                "units_per_em": 2048,
                "ascender": 1984,
                "descender": -494,
                "line_gap": 0,
                "x_height": 1118,
                "cap_height": 1490,
                "num_glyphs": 2926,
                "family_name": "Inter",
                "subfamily_name": "Regular",
            },
        )

    def test_glyph_id_and_metrics_follow_cmap_and_hmtx(self):
        font = fp.load(FONT_BYTES)
        a = fp.glyph_id(font, 0x41)
        self.assertEqual(a, 2)
        self.assertEqual(fp.glyph_metrics(font, a), {"advance_width": 1413, "left_side_bearing": 52})
        # U+10FFFF has no glyph: None, not an exception.
        self.assertIsNone(fp.glyph_id(font, 0x10FFFF))

    def test_kerning_is_zero_for_inter(self):
        # Inter keeps its pair adjustments in GPOS, not the legacy kern table.
        font = fp.load(FONT_BYTES)
        self.assertEqual(fp.kerning(font, fp.glyph_id(font, 0x41), fp.glyph_id(font, 0x56)), 0)

    def test_bad_input_raises_instead_of_crashing(self):
        with self.assertRaises(ValueError):
            fp.load(b"junk")
        with self.assertRaises(Exception):
            fp.load("not bytes")


if __name__ == "__main__":
    unittest.main(verbosity=2)
