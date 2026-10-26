"""Executable contract for replay-safe book publication (HL00)."""

from __future__ import annotations

import hashlib
import importlib
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS_DIR))
publication = importlib.import_module("check_human_language_book_publication")

OLDER = "1" * 40
NEWER = "2" * 40


class PublicationDecisionTests(unittest.TestCase):
    def test_older_run_cannot_replace_newer_pages(self) -> None:
        with self.assertRaisesRegex(publication.PublicationError, "older"):
            publication.pages_should_publish(OLDER, NEWER, "behind")

    def test_divergent_run_cannot_replace_pages(self) -> None:
        with self.assertRaisesRegex(publication.PublicationError, "diverged"):
            publication.pages_should_publish(OLDER, NEWER, "diverged")

    def test_same_source_replay_leaves_pages_alone(self) -> None:
        self.assertFalse(publication.pages_should_publish(NEWER, NEWER, None))
        self.assertTrue(publication.pages_should_publish(NEWER, OLDER, "ahead"))
        self.assertTrue(publication.pages_should_publish(NEWER, None, None))

    def test_pages_source_message_must_be_unambiguous(self) -> None:
        self.assertEqual(publication.parse_pages_source("deploy: " + NEWER), NEWER)
        with self.assertRaises(publication.PublicationError):
            publication.parse_pages_source("deploy: unknown")

    def test_release_tag_stays_stable_across_attempt_dates(self) -> None:
        tag = "human-languages-v2026.10.26.12125"
        releases = [{"tag_name": tag}]
        self.assertEqual(
            publication.select_release_tag(releases, 12125, "2026.11.01"), tag
        )
        self.assertEqual(
            publication.select_release_tag([], 12126, "2026.10.26"),
            "human-languages-v2026.10.26.12126",
        )

    def test_exact_immutable_release_replay_skips_upload(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            pdf = Path(directory) / "punjabi.pdf"
            payload = b"%PDF-known-bytes"
            pdf.write_bytes(payload)
            tag = "human-languages-v2026.10.26.12125"
            release = {
                "tag_name": tag,
                "target_commitish": NEWER,
                "immutable": True,
                "assets": [
                    {
                        "name": pdf.name,
                        "size": len(payload),
                        "state": "uploaded",
                        "digest": "sha256:" + hashlib.sha256(payload).hexdigest(),
                    }
                ],
            }
            self.assertFalse(
                publication.release_should_create(release, tag, NEWER, [pdf])
            )

            release["assets"][0]["digest"] = "sha256:" + "0" * 64
            with self.assertRaisesRegex(publication.PublicationError, "digest"):
                publication.release_should_create(release, tag, NEWER, [pdf])

    def test_release_target_or_asset_set_mismatch_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            pdf = Path(directory) / "punjabi.pdf"
            pdf.write_bytes(b"%PDF-test")
            tag = "human-languages-v2026.10.26.12125"
            release = {
                "tag_name": tag,
                "target_commitish": OLDER,
                "immutable": True,
                "assets": [],
            }
            with self.assertRaisesRegex(publication.PublicationError, "target"):
                publication.release_should_create(release, tag, NEWER, [pdf])
            release["target_commitish"] = NEWER
            with self.assertRaisesRegex(publication.PublicationError, "asset"):
                publication.release_should_create(release, tag, NEWER, [pdf])


if __name__ == "__main__":
    unittest.main()
