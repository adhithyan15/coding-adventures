"""Contract test for the generated Dart mirror of the trig-reference-v1 vectors.

The Dart package has no filesystem capability, so its tests decode the vectors
from generated source (PHY00a 6.5).  This test proves that source is an exact,
current mirror of the TSVs and that the embedded hashes and counts are the
manifest's.
"""

from __future__ import annotations

import base64
import hashlib
import importlib.util
import json
import re
import subprocess
import sys
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
ROOT = SCRIPTS.parents[1]
FIXTURE = ROOT / "code" / "specs" / "fixtures" / "trig-reference-v1"
GENERATED = (
    ROOT / "code" / "packages" / "dart" / "trig" / "test" / "generated"
    / "trig_reference_vectors.dart"
)
GENERATOR = SCRIPTS / "generate_trig_reference_dart_fixtures.py"
GROUPS = ("edge", "sweep-binade", "sweep-cw", "sweep-prng")


def _load_generator():
    spec = importlib.util.spec_from_file_location("gen_trig_dart", GENERATOR)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _constant(source: str, name: str) -> str:
    match = re.search(rf"const String {name} =\n((?:    '[A-Za-z0-9+/=]+'\n?)+);", source)
    assert match, name
    return "".join(re.findall(r"'([A-Za-z0-9+/=]+)'", match.group(1)))


class TrigReferenceDartFixtureTests(unittest.TestCase):
    def test_generator_check_passes(self) -> None:
        result = subprocess.run(
            [sys.executable, str(GENERATOR), "--check"], capture_output=True, text=True, check=False
        )
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_generated_file_is_the_current_render(self) -> None:
        expected = _load_generator().render().encode("utf-8")
        self.assertEqual(GENERATED.read_bytes(), expected)

    def test_each_constant_decodes_byte_for_byte_to_its_tsv(self) -> None:
        source = GENERATED.read_text("utf-8")
        for name in GROUPS:
            ident = "".join(p.capitalize() for p in name.split("-"))
            decoded = base64.b64decode(_constant(source, f"trigReference{ident}Base64"), validate=True)
            self.assertEqual(decoded, (FIXTURE / f"{name}.tsv").read_bytes(), name)

    def test_embedded_hashes_and_counts_are_the_manifests(self) -> None:
        manifest = json.loads((FIXTURE / "manifest.json").read_text("utf-8"))
        source = GENERATED.read_text("utf-8")
        for group in manifest["groups"]:
            flat = re.sub(r"\s+", " ", source)
            self.assertIn(f"'{group['name']}': '{group['sha256']}',", flat)
            self.assertIn(f"'{group['name']}': {group['rows']},", source)
            raw = (FIXTURE / group["file"]).read_bytes()
            self.assertEqual(hashlib.sha256(raw).hexdigest(), group["sha256"])

    def test_generator_refuses_a_tsv_that_does_not_match_the_manifest(self) -> None:
        gen = _load_generator()
        original = gen.FIXTURE_ROOT
        try:
            gen.FIXTURE_ROOT = FIXTURE  # unchanged tree must pass
            gen.load_groups()
        finally:
            gen.FIXTURE_ROOT = original
        # A manifest hash that is wrong must stop the generator.
        manifest = json.loads((FIXTURE / "manifest.json").read_text("utf-8"))
        manifest["groups"][0]["sha256"] = "0" * 64
        real_read_text = Path.read_text

        def fake(self, *a, **k):
            if self.name == "manifest.json" and self.parent == FIXTURE:
                return json.dumps(manifest)
            return real_read_text(self, *a, **k)

        Path.read_text = fake
        try:
            with self.assertRaises(SystemExit):
                gen.load_groups()
        finally:
            Path.read_text = real_read_text


if __name__ == "__main__":
    unittest.main()
