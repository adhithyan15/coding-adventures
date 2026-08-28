import json
import os
import tempfile
import unittest
from pathlib import Path

from sharded_ledger import (
    load_curriculum,
    load_script,
    load_script_inventory,
    write_chapters,
    write_curriculum,
)


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


class ShardedLedgerSafetyTest(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name) / "root"
        self.outside = Path(self.temporary.name) / "outside"
        self.root.mkdir()
        self.outside.mkdir()

    def tearDown(self):
        self.temporary.cleanup()

    def chapter_document(self, *chapters):
        return {"version": 1, "language": "spanish", "chapters": list(chapters)}

    def curriculum_tree(self):
        directory = self.root / "spanish" / "curriculum.d"
        write_json(
            directory / "_meta.json",
            {
                "_keys": ["version", "language", "path", "spine", "extensions"],
                "version": 1,
                "language": "spanish",
            },
        )
        write_json(directory / "path" / "0010-ES-PATH-001.json", {"id": "ES-PATH-001"})
        write_json(directory / "spine" / "0010-SPINE-ONE.json", {"segments": []})
        (directory / "extensions").mkdir()
        return directory

    def script_tree(self):
        directory = self.root / "data" / "scripts" / "japanese.d"
        write_json(
            directory / "_meta.json",
            {
                "script": "japanese",
                "name": "Japanese",
                "font": "font.ttf",
                "direction": "ltr",
                "system": "mixed",
            },
        )
        write_json(directory / "letters" / "0010-U-3042.json", {"glyph": "あ"})
        write_json(directory / "marks" / "0010-U-309B.json", {"mark": "゛"})
        return directory

    def test_dangling_shard_symlink_cannot_create_its_target(self):
        directory = self.root / "spanish" / "chapters.d"
        write_json(directory / "_meta.json", {"version": 1, "language": "spanish"})
        target = self.outside / "created.json"
        os.symlink(target, directory / "0001.json")

        with self.assertRaisesRegex(ValueError, "non-file ledger shard"):
            write_chapters(
                self.root,
                "spanish",
                self.chapter_document({"chapter": 1, "title": "one"}),
            )
        self.assertFalse(target.exists())

    def test_symlinked_track_ancestor_cannot_escape_root(self):
        directory = self.outside / "chapters.d"
        write_json(directory / "_meta.json", {"version": 1, "language": "spanish"})
        os.symlink(self.outside, self.root / "spanish")

        with self.assertRaisesRegex(ValueError, "non-directory ledger ancestor"):
            write_chapters(self.root, "spanish", self.chapter_document())

    def test_duplicate_chapters_are_rejected_before_any_write(self):
        directory = self.root / "spanish" / "chapters.d"
        write_json(directory / "_meta.json", {"version": 1, "language": "spanish"})
        original = {"chapter": 1, "title": "original"}
        write_json(directory / "0001.json", original)

        with self.assertRaisesRegex(ValueError, "duplicate chapter identity"):
            write_chapters(
                self.root,
                "spanish",
                self.chapter_document(
                    {"chapter": 1, "title": "replacement"},
                    {"chapter": 1, "title": "duplicate"},
                ),
            )
        self.assertEqual(json.loads((directory / "0001.json").read_text()), original)

    def test_curriculum_filename_identity_mismatch_is_rejected(self):
        directory = self.curriculum_tree()
        write_json(
            directory / "extensions" / "0010-ES-EXT-001.json",
            {"id": "ES-EXT-WRONG"},
        )

        with self.assertRaisesRegex(ValueError, "identity does not match shard name"):
            load_curriculum(self.root, "spanish")

    def test_curriculum_document_is_fully_validated_before_writes(self):
        directory = self.curriculum_tree()
        document = load_curriculum(self.root, "spanish")
        document["path"][0] = {"id": "ES-PATH-001", "changed": True}
        document["extensions"] = [{"id": "bad/id"}]

        with self.assertRaisesRegex(ValueError, "unsafe curriculum id"):
            write_curriculum(self.root, "spanish", document)
        self.assertEqual(
            json.loads((directory / "path" / "0010-ES-PATH-001.json").read_text()),
            {"id": "ES-PATH-001"},
        )

    def test_script_inventory_reconstructs_sections_in_filename_order(self):
        self.script_tree()
        inventory = load_script_inventory(self.root, "japanese")
        self.assertEqual(inventory["script"], "japanese")
        self.assertEqual(inventory["letters"], [{"glyph": "あ"}])
        self.assertEqual(inventory["marks"], [{"mark": "゛"}])

    def test_mixed_script_loader_prefers_shards_and_falls_back_to_monolith(self):
        directory = self.script_tree()
        write_json(
            self.root / "data" / "scripts" / "tamil.json",
            {"script": "tamil", "letters": [{"glyph": "அ"}], "marks": []},
        )
        self.assertEqual(load_script(self.root, "japanese")["letters"], [{"glyph": "あ"}])
        self.assertEqual(load_script(self.root, "tamil")["letters"], [{"glyph": "அ"}])

        write_json(directory.parent / "japanese.json", {"script": "stale"})
        self.assertEqual(load_script(self.root, "japanese")["script"], "japanese")

    def test_mixed_script_loader_refuses_a_symlinked_shard_directory(self):
        scripts = self.root / "data" / "scripts"
        scripts.mkdir(parents=True)
        os.symlink(self.outside, scripts / "tamil.d")
        with self.assertRaisesRegex(ValueError, "non-directory ledger ancestor"):
            load_script(self.root, "tamil")

    def test_script_inventory_rejects_filename_identity_mismatch(self):
        directory = self.script_tree()
        write_json(directory / "letters" / "0020-U-3044.json", {"glyph": "う"})
        with self.assertRaisesRegex(ValueError, "identity does not match shard name"):
            load_script_inventory(self.root, "japanese")

    def test_script_inventory_rejects_duplicate_ordinals_across_one_section(self):
        directory = self.script_tree()
        write_json(directory / "letters" / "0010-U-3044.json", {"glyph": "い"})
        with self.assertRaisesRegex(ValueError, "duplicate letters ordinal"):
            load_script_inventory(self.root, "japanese")


if __name__ == "__main__":
    unittest.main()
