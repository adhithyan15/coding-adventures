"""Regression checks for the one-off lesson-authoring frontmatter readers."""

import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

try:
    import yaml
except ImportError:  # PyYAML is optional for these standalone scripts.
    yaml = None

import author_adjective_wave
import author_deixis_wave
import author_recognition_segments
import propose_letter_ledger
import reading_closure
import recover_romanization
from authoring_frontmatter import decode_scalar

LESSON = """---
id: test-lesson
sequence: 1
chapter: 1
headword: 'क''था\\पथ'
romanization: "katha"
gloss: '"you''re welcome" — a reply'
introduces:
  knowledge: [a, b]
---
Body with headword: not-the-frontmatter-value.
"""


class AuthoringFrontmatterTests(unittest.TestCase):
    def test_scalar_forms_match_production_reader(self):
        cases = [
            ("plain", "plain"),
            ('"double quoted"', "double quoted"),
            ("'single quoted'", "single quoted"),
            ("'l''amour'", "l'amour"),
            ("'क''था\\पथ'", "क'था\\पथ"),
            ("'  spaced  '", "  spaced  "),
        ]
        for source, expected in cases:
            with self.subTest(source=source):
                self.assertEqual(decode_scalar(source, "gloss"), expected)

    def test_literal_fields_reject_ambiguous_double_quoted_backslash(self):
        with self.assertRaisesRegex(ValueError, "single quotes"):
            decode_scalar('"a\\b"', "headword")

    @unittest.skipUnless(yaml is not None, "PyYAML is not installed")
    def test_valid_yaml_scalars_agree_with_yaml_parser(self):
        values = yaml.safe_load(LESSON.split("---")[1])
        parsed = author_adjective_wave.fm_of(LESSON)
        for key in ("headword", "romanization", "gloss"):
            self.assertEqual(parsed[key], values[key])

    def test_flat_readers_decode_quoted_values_without_changing_lists(self):
        readers = (
            author_adjective_wave.fm_of,
            author_deixis_wave.fm_of,
            author_recognition_segments.fm_of,
            propose_letter_ledger.read_frontmatter,
        )
        for reader in readers:
            with self.subTest(reader=reader.__module__):
                values = reader(LESSON)
                self.assertEqual(values["headword"], "क'था\\पथ")
                self.assertEqual(values["romanization"], "katha")
                self.assertEqual(values["gloss"], '"you\'re welcome" — a reply')
                self.assertEqual(values["introduces.knowledge"], "[a, b]")

    def test_recovery_reader_decodes_single_quotes(self):
        values = recover_romanization.fm(LESSON)
        self.assertEqual(values["headword"], "क'था\\पथ")
        self.assertEqual(values["gloss"], '"you\'re welcome" — a reply')

    def test_reading_closure_reads_decoded_headword(self):
        with tempfile.TemporaryDirectory() as root:
            lessons = Path(root) / "test" / "lessons"
            lessons.mkdir(parents=True)
            (lessons / "test-lesson.md").write_text(LESSON, encoding="utf-8")
            with patch.object(reading_closure, "HL", root):
                rows = reading_closure.load("test")
        self.assertEqual(rows[0]["headword"], "क'था\\पथ")

    def test_recognition_does_not_propose_an_already_taught_quoted_glyph(self):
        with tempfile.TemporaryDirectory() as root:
            lessons = Path(root) / "test" / "lessons"
            lessons.mkdir(parents=True)
            (lessons / "glyph.md").write_text(
                "---\ndelivery: script\nheadword: 'क'\n---\n", encoding="utf-8"
            )
            (lessons / "word.md").write_text(
                "---\ndelivery: word\nheadword: 'ख'\n---\n", encoding="utf-8"
            )
            with patch.object(author_recognition_segments, "HL", root):
                taught = author_recognition_segments.taught_alone_glyphs("test")
        self.assertEqual(taught, {"क"})


if __name__ == "__main__":
    unittest.main()
