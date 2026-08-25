#!/usr/bin/env python3
"""Regression tests for the source-gated syllabary regeneration boundary."""

import copy
import json
import os
import unittest

import generate_syllabary as syllabary


class VerifiedRowMergeTests(unittest.TestCase):
    def fixture(self, row: dict) -> dict:
        return {
            "letters": [row],
            "independentVowels": [],
            "digits": [],
        }

    def test_preserves_source_gated_fields_but_rebuilds_unicode_identity(self) -> None:
        generated = self.fixture({"glyph": "x", "sound": "new", "role": "vowel"})
        existing = self.fixture({
            "glyph": "x",
            "sound": "old",
            "role": "vowel",
            "components": ["verified shape"],
            "strokeOrder": ["verified movement"],
            "strokeOrderSource": {"citation": "source"},
            "futureEvidenceField": {"survives": True},
        })

        merged = syllabary.merge_verified_rows(generated, existing, "test")

        self.assertEqual(merged["letters"][0]["sound"], "new")
        self.assertEqual(merged["letters"][0]["components"], ["verified shape"])
        self.assertEqual(
            merged["letters"][0]["futureEvidenceField"],
            {"survives": True},
        )

    def test_explicit_generator_source_wins_for_a_new_or_updated_exception(self) -> None:
        generated = self.fixture({
            "glyph": "x",
            "sound": "x",
            "role": "vowel",
            "strokeOrder": ["new movement"],
            "strokeOrderSource": {"citation": "new source"},
        })
        existing = self.fixture({
            "glyph": "x",
            "sound": "x",
            "role": "vowel",
            "strokeOrder": ["old movement"],
            "strokeOrderSource": {"citation": "old source"},
            "futureEvidenceField": {"survives": True},
        })

        merged = syllabary.merge_verified_rows(generated, existing, "test")

        self.assertEqual(merged["letters"][0]["strokeOrder"], ["new movement"])
        self.assertEqual(
            merged["letters"][0]["futureEvidenceField"],
            {"survives": True},
        )

    def test_preserves_a_committed_glyph_outside_the_generated_core(self) -> None:
        generated = self.fixture({"glyph": "y", "sound": "y", "role": "vowel"})
        existing = self.fixture({
            "glyph": "x",
            "sound": "x",
            "role": "vowel",
            "strokeOrderSource": {"citation": "source"},
        })

        merged = syllabary.merge_verified_rows(generated, existing, "test")

        self.assertEqual([row["glyph"] for row in merged["letters"]], ["y", "x"])

    def test_fails_closed_on_duplicate_committed_glyphs(self) -> None:
        generated = self.fixture({"glyph": "x", "sound": "x", "role": "vowel"})
        existing = self.fixture({"glyph": "x", "sound": "x", "role": "vowel"})
        existing["letters"].append(existing["letters"][0].copy())

        with self.assertRaisesRegex(RuntimeError, "duplicate committed"):
            syllabary.merge_verified_rows(generated, existing, "test")

    def test_preserves_downstream_collections_the_unicode_grid_does_not_own(self) -> None:
        generated = self.fixture({"glyph": "x", "sound": "x", "role": "vowel"})
        existing = self.fixture({"glyph": "x", "sound": "x", "role": "vowel"})
        existing["marks"] = [{"mark": "m", "role": "virama"}]

        merged = syllabary.merge_verified_rows(generated, existing, "test")

        self.assertEqual(merged["marks"], [{"mark": "m", "role": "virama"}])

    def test_committed_outputs_are_regeneration_stable(self) -> None:
        for script_id, name, base, font, signature in syllabary.SCRIPTS:
            path = os.path.join(syllabary.HERE, f"{script_id}.json")
            with open(path, encoding="utf-8") as fh:
                existing = json.load(fh)
            regenerated = syllabary.merge_verified_rows(
                syllabary.build_script(script_id, name, base, font, signature),
                copy.deepcopy(existing),
                script_id,
            )
            regenerated["notes"] = existing["notes"]
            self.assertEqual(regenerated, existing, script_id)


if __name__ == "__main__":
    unittest.main()
