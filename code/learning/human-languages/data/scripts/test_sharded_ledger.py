import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from sharded_ledger import (
    _files,
    _remove,
    _strict_directory_entries,
    load_book_generation,
    load_curriculum,
    load_script,
    load_script_inventory,
    write_book_generation_language,
    write_chapters,
    write_curriculum,
)


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def link_directory(target, link):
    """Exercise a real symlink on Unix and a privilege-free junction on Windows."""
    if os.name == "nt":
        subprocess.run(
            ["cmd", "/c", "mklink", "/J", str(link), str(target)],
            check=True,
            capture_output=True,
        )
    else:
        os.symlink(target, link)


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

    def book_generation_tree(self):
        directory = self.root / "core" / "book-generation.d"
        write_json(
            directory / "_meta.json",
            {
                "version": 1,
                "sourceBaseUrl": "https://example.test/human-languages/",
            },
        )
        for section in (
            "reference-appendices.d",
            "glossaries.d",
            "answer-keys.d",
            "indexes.d",
            "targets.d",
            "handwritten.d",
            "script-sets.d",
        ):
            (directory / section).mkdir(parents=True, exist_ok=True)
        write_json(
            directory / "script-sets.d" / "0010-spanish-main.json",
            [{"unicodeScript": "Latin", "scriptCommand": "es"}],
        )
        write_json(
            directory / "glossaries.d" / "spanish-appendix-glossary.json",
            {
                "language": "spanish",
                "output": "spanish/book/chapters/appendix-glossary.tex",
                "scriptSet": "spanish-main",
            },
        )
        write_json(
            directory / "targets.d" / "french-0001.json",
            {
                "language": "french",
                "chapter": 1,
                "output": "french/book/chapters/ch01-greetings.tex",
            },
        )
        write_json(
            directory / "targets.d" / "spanish-0001.json",
            {
                "language": "spanish",
                "chapter": 1,
                "output": "spanish/book/chapters/ch01-first-words.tex",
                "scriptSet": "spanish-main",
            },
        )
        write_json(
            directory / "targets.d" / "spanish-0002.json",
            {
                "language": "spanish",
                "chapter": 2,
                "output": "spanish/book/chapters/ch02-greetings.tex",
                "scriptSet": "spanish-main",
            },
        )
        write_json(
            directory / "handwritten.d" / "spanish-0004.json",
            {
                "language": "spanish",
                "chapter": 4,
                "output": "spanish/book/chapters/ch04-handwritten.tex",
                "embeddedLessonIds": ["ES-LESSON-004"],
            },
        )
        return directory

    def test_reparse_or_dangling_symlink_shard_cannot_create_its_target(self):
        directory = self.root / "spanish" / "chapters.d"
        write_json(directory / "_meta.json", {"version": 1, "language": "spanish"})
        target = self.outside / "created.json"
        if os.name == "nt":
            # A directory junction needs no Developer Mode or symlink privilege.
            link_directory(self.outside, directory / "0001.json")
        else:
            os.symlink(target, directory / "0001.json")

        with self.assertRaisesRegex(ValueError, "non-file ledger shard"):
            write_chapters(
                self.root,
                "spanish",
                self.chapter_document({"chapter": 1, "title": "one"}),
            )
        self.assertFalse(target.exists())

    @unittest.skipUnless(os.name == "nt", "Windows hard-link fixture")
    def test_windows_hardlinked_shard_does_not_overwrite_outside_file(self):
        directory = self.root / "spanish" / "chapters.d"
        write_json(directory / "_meta.json", {"version": 1, "language": "spanish"})
        target = self.outside / "sentinel.json"
        write_json(target, {"chapter": 1, "title": "outside sentinel"})
        before = target.read_bytes()
        os.link(target, directory / "0001.json")

        write_chapters(
            self.root,
            "spanish",
            self.chapter_document({"chapter": 1, "title": "new inside"}),
        )
        self.assertEqual(target.read_bytes(), before)
        self.assertEqual(json.loads((directory / "0001.json").read_text())["title"], "new inside")

    @unittest.skipUnless(os.name == "nt", "Windows file ACL fixture")
    def test_windows_replacement_preserves_restricted_shard_dacl(self):
        directory = self.root / "spanish" / "chapters.d"
        write_json(directory / "_meta.json", {"version": 1, "language": "spanish"})
        shard = directory / "0001.json"
        write_json(shard, {"chapter": 1, "title": "old"})
        user = subprocess.check_output(["whoami"], text=True).strip()
        subprocess.run(["icacls", str(shard), "/grant:r", f"{user}:(F)"], check=True, capture_output=True)
        subprocess.run(["icacls", str(shard), "/inheritance:r"], check=True, capture_output=True)

        def acl():
            listing = subprocess.check_output(["icacls", str(shard)], text=True)
            return listing.split(str(shard), 1)[1].split("Successfully processed", 1)[0].strip()

        before = acl()
        write_chapters(
            self.root, "spanish", self.chapter_document({"chapter": 1, "title": "replacement"}),
        )
        self.assertEqual(acl(), before)

    @unittest.skipUnless(os.name == "nt", "Windows handle cleanup fixture")
    def test_windows_failed_temp_cleanup_surfaces_both_errors(self):
        directory = self.root / "spanish" / "chapters.d"
        write_json(directory / "_meta.json", {"version": 1, "language": "spanish"})
        with mock.patch("sharded_ledger._windows_write_handle", side_effect=OSError("write failed")):
            with mock.patch("sharded_ledger._windows_dispose", side_effect=OSError("cleanup failed")):
                with self.assertRaisesRegex(OSError, "cleanup failed") as raised:
                    write_chapters(
                        self.root, "spanish", self.chapter_document({"chapter": 1}),
                    )
        self.assertEqual(str(raised.exception.__cause__), "write failed")

    def test_symlinked_track_ancestor_cannot_escape_root(self):
        directory = self.outside / "chapters.d"
        write_json(directory / "_meta.json", {"version": 1, "language": "spanish"})
        link_directory(self.outside, self.root / "spanish")

        with self.assertRaisesRegex(ValueError, "non-directory ledger ancestor"):
            write_chapters(self.root, "spanish", self.chapter_document())

    @unittest.skipUnless(os.name == "nt", "Windows directory junction fixture")
    def test_junctioned_track_ancestor_cannot_escape_root(self):
        write_json(
            self.outside / "chapters.d" / "_meta.json",
            {"version": 1, "language": "spanish"},
        )
        subprocess.run(
            ["cmd", "/c", "mklink", "/J", str(self.root / "spanish"), str(self.outside)],
            check=True,
            capture_output=True,
        )
        with self.assertRaisesRegex(ValueError, "non-directory ledger ancestor"):
            write_chapters(self.root, "spanish", self.chapter_document())
        self.assertFalse((self.outside / "chapters.d" / "0001.json").exists())

    @unittest.skipUnless(os.name == "nt", "Windows directory pin fixture")
    def test_windows_writer_stays_on_original_directory_after_junction_swap(self):
        directory = self.root / "spanish" / "chapters.d"
        write_json(directory / "_meta.json", {"version": 1, "language": "spanish"})
        outside_shard = self.outside / "0001.json"
        write_json(outside_shard, {"chapter": 1, "title": "outside sentinel"})
        before = outside_shard.read_bytes()
        from sharded_ledger import _windows_open_relative, _write_if_changed
        swapped = []
        moved = directory.with_name("chapters-moved")

        def attempt_swap(parent, name, **options):
            if options.get("create") and not swapped:
                os.rename(directory, moved)
                subprocess.run(
                    ["cmd", "/c", "mklink", "/J", str(directory), str(self.outside)],
                    check=True,
                    capture_output=True,
                )
                swapped.append(True)
            return _windows_open_relative(parent, name, **options)

        try:
            with mock.patch("sharded_ledger._windows_open_relative", side_effect=attempt_swap):
                _write_if_changed(
                    self.root, "spanish", "chapters.d", "0001.json",
                    value={"chapter": 1, "title": "one"},
                )
            self.assertEqual(outside_shard.read_bytes(), before)
            self.assertTrue((moved / "0001.json").is_file())
        finally:
            if swapped and directory.exists():
                os.rmdir(directory)
            if moved.exists():
                os.rename(moved, directory)
        self.assertTrue(swapped)
        self.assertTrue((directory / "0001.json").is_file())

    @unittest.skipUnless(os.name == "nt", "Windows directory junction fixture")
    def test_windows_remove_stays_on_original_directory_after_junction_swap(self):
        directory = self.root / "spanish" / "chapters.d"
        write_json(directory / "0001.json", {"chapter": 1, "title": "inside"})
        outside_shard = self.outside / "0001.json"
        write_json(outside_shard, {"chapter": 1, "title": "outside sentinel"})
        before = outside_shard.read_bytes()
        from sharded_ledger import _windows_open_relative
        moved = directory.with_name("chapters-moved")
        swapped = []

        def attempt_swap(parent, name, **options):
            if name == "0001.json" and not swapped:
                os.rename(directory, moved)
                subprocess.run(
                    ["cmd", "/c", "mklink", "/J", str(directory), str(self.outside)],
                    check=True,
                    capture_output=True,
                )
                swapped.append(True)
            return _windows_open_relative(parent, name, **options)

        try:
            with mock.patch("sharded_ledger._windows_open_relative", side_effect=attempt_swap):
                _remove(self.root, "spanish", "chapters.d", "0001.json")
            self.assertEqual(outside_shard.read_bytes(), before)
            self.assertFalse((moved / "0001.json").exists())
        finally:
            if swapped and directory.exists():
                os.rmdir(directory)
            if moved.exists():
                os.rename(moved, directory)
        self.assertTrue(swapped)

    @unittest.skipUnless(os.name == "nt", "Windows directory junction fixture")
    def test_windows_file_enumeration_rejects_swap_after_path_prevalidation(self):
        directory = self.root / "spanish" / "chapters.d"
        write_json(directory / "0001.json", {"chapter": 1, "title": "inside"})
        write_json(self.outside / "0002.json", {"chapter": 2, "title": "outside"})
        from sharded_ledger import _directory
        moved = directory.with_name("chapters-moved")
        swapped = []

        def attempt_swap(root, *parts):
            path = _directory(root, *parts)
            if parts == ("spanish", "chapters.d") and not swapped:
                os.rename(directory, moved)
                link_directory(self.outside, directory)
                swapped.append(True)
            return path

        try:
            with (
                mock.patch("sharded_ledger._directory", side_effect=attempt_swap),
                self.assertRaisesRegex(ValueError, "reparse"),
            ):
                _files(self.root, "spanish", "chapters.d")
        finally:
            if swapped and directory.exists():
                os.rmdir(directory)
            if moved.exists():
                os.rename(moved, directory)
        self.assertTrue(swapped)

    @unittest.skipUnless(os.name == "nt", "Windows directory junction fixture")
    def test_windows_owner_enumeration_rejects_swap_after_path_prevalidation(self):
        directory = self.book_generation_tree()
        write_json(self.outside / "outside.json", {"owner": "outside"})
        from sharded_ledger import _directory
        moved = directory.with_name("book-generation-moved")
        swapped = []

        def attempt_swap(root, *parts):
            path = _directory(root, *parts)
            if parts == ("core", "book-generation.d") and not swapped:
                os.rename(directory, moved)
                link_directory(self.outside, directory)
                swapped.append(True)
            return path

        try:
            with (
                mock.patch("sharded_ledger._directory", side_effect=attempt_swap),
                self.assertRaisesRegex(ValueError, "reparse"),
            ):
                _strict_directory_entries(self.root, "core", "book-generation.d")
        finally:
            if swapped and directory.exists():
                os.rmdir(directory)
            if moved.exists():
                os.rename(moved, directory)
        self.assertTrue(swapped)

    @unittest.skipUnless(os.name == "nt", "Windows native directory batches")
    def test_windows_file_enumeration_drains_multiple_native_batches(self):
        directory = self.root / "spanish" / "chapters.d"
        for index in range(300):
            write_json(directory / f"{index:04d}-{'x' * 110}.json", {"index": index})
        self.assertEqual(len(_files(self.root, "spanish", "chapters.d")), 300)

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

    def test_script_inventory_digits_section_is_optional(self):
        directory = self.script_tree()
        # No digits/ directory: no digits key, as before the section existed.
        self.assertNotIn("digits", load_script_inventory(self.root, "japanese"))
        write_json(directory / "digits" / "0010-U-6F0.json", {"glyph": "۰"})
        write_json(directory / "digits" / "0020-U-6F1.json", {"glyph": "۱"})
        inventory = load_script_inventory(self.root, "japanese")
        self.assertEqual(inventory["digits"], [{"glyph": "۰"}, {"glyph": "۱"}])

    def test_script_inventory_digit_glyph_cannot_also_be_a_letter(self):
        directory = self.script_tree()
        write_json(directory / "digits" / "0010-U-3042.json", {"glyph": "あ"})
        with self.assertRaisesRegex(ValueError, "owned by both"):
            load_script_inventory(self.root, "japanese")

    def test_script_inventory_refuses_a_symlinked_digits_directory(self):
        directory = self.script_tree()
        link_directory(self.outside, directory / "digits")
        with self.assertRaisesRegex(ValueError, "non-directory ledger ancestor"):
            load_script_inventory(self.root, "japanese")

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
        link_directory(self.outside, scripts / "tamil.d")
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

    def test_book_generation_reconstructs_direct_owners_in_filename_order(self):
        self.book_generation_tree()
        document = load_book_generation(
            self.root,
            expected_target_identities={"french/0001", "spanish/0001", "spanish/0002"},
            expected_handwritten_identities={"spanish/0004"},
        )

        self.assertEqual(
            list(document),
            [
                "version",
                "sourceBaseUrl",
                "scriptSets",
                "referenceAppendices",
                "glossaries",
                "answerKeys",
                "indexes",
                "targets",
                "handwritten",
            ],
        )
        self.assertEqual(list(document["scriptSets"]), ["spanish-main"])
        self.assertEqual(
            [(entry["language"], entry["chapter"]) for entry in document["targets"]],
            [("french", 1), ("spanish", 1), ("spanish", 2)],
        )

    def test_book_generation_expected_set_detects_clean_owner_deletion(self):
        directory = self.book_generation_tree()
        (directory / "targets.d" / "spanish-0002.json").unlink()

        with self.assertRaisesRegex(
            ValueError, "target owner set mismatch.*spanish/0002"
        ):
            load_book_generation(
                self.root,
                expected_target_identities={
                    "french/0001",
                    "spanish/0001",
                    "spanish/0002",
                },
            )

    def test_book_generation_expected_handwritten_set_is_independent(self):
        directory = self.book_generation_tree()
        (directory / "handwritten.d" / "spanish-0004.json").unlink()

        with self.assertRaisesRegex(
            ValueError, "handwritten owner set mismatch.*spanish/0004"
        ):
            load_book_generation(
                self.root,
                expected_handwritten_identities={"spanish/0004"},
            )

    # A section directory MUST exist and MAY be empty. The pair below pins both
    # halves, and they are pinned HERE as well as in grouped-shards.test.ts
    # because these are two independent readers of one contract -- the
    # TypeScript side was fixed first, and this reader then rejected a tree the
    # other had just accepted.
    def test_book_generation_accepts_an_empty_section_with_a_placeholder(self):
        directory = self.book_generation_tree()
        section = directory / "handwritten.d"
        for entry in section.iterdir():
            entry.unlink()
        (section / ".gitkeep").write_text("", encoding="utf-8")

        document = load_book_generation(self.root)
        self.assertEqual(document.get("handwritten", []), [])

    def test_book_generation_still_rejects_an_absent_section(self):
        directory = self.book_generation_tree()
        section = directory / "handwritten.d"
        for entry in section.iterdir():
            entry.unlink()
        section.rmdir()

        with self.assertRaisesRegex(
            ValueError, "missing book-generation owner directories.*handwritten.d"
        ):
            load_book_generation(self.root)

    def test_book_generation_rejects_unexpected_nested_owner(self):
        directory = self.book_generation_tree()
        nested = directory / "targets.d" / "spanish"
        nested.mkdir()
        write_json(nested / "0003.json", {"language": "spanish", "chapter": 3})

        with self.assertRaisesRegex(ValueError, "unexpected nested.*targets.d/spanish"):
            load_book_generation(self.root)

    def test_book_generation_rejects_symlinked_section_component(self):
        directory = self.book_generation_tree()
        (directory / "indexes.d").rmdir()
        link_directory(self.outside, directory / "indexes.d")

        with self.assertRaisesRegex(ValueError, "link or reparse book-generation owner"):
            load_book_generation(self.root)

    def test_book_generation_rejects_resurrected_monolith(self):
        self.book_generation_tree()
        write_json(self.root / "core" / "book-generation.json", {"version": 1})

        with self.assertRaisesRegex(ValueError, "resurrected book-generation monolith"):
            load_book_generation(self.root)

    def test_book_generation_rejects_noncanonical_script_set_ordinal(self):
        directory = self.book_generation_tree()
        owner = directory / "script-sets.d" / "0010-spanish-main.json"
        owner.rename(directory / "script-sets.d" / "0000-spanish-main.json")

        with self.assertRaisesRegex(ValueError, "ordinal is out of canonical order"):
            load_book_generation(self.root)

    def test_book_generation_rejects_filename_identity_mismatch(self):
        directory = self.book_generation_tree()
        path = directory / "targets.d" / "spanish-0002.json"
        value = json.loads(path.read_text())
        value["chapter"] = 3
        write_json(path, value)

        with self.assertRaisesRegex(ValueError, "identity does not match owner name"):
            load_book_generation(self.root)

    def test_book_generation_rejects_output_traversal_and_dangerous_keys(self):
        directory = self.book_generation_tree()
        path = directory / "glossaries.d" / "spanish-appendix-glossary.json"
        value = json.loads(path.read_text())
        value["output"] = "spanish/../appendix-glossary.tex"
        write_json(path, value)
        with self.assertRaisesRegex(ValueError, "unsafe output path"):
            load_book_generation(self.root)

        value["output"] = "spanish/book/chapters/appendix-glossary.tex"
        value["nested"] = {"__proto__": {"polluted": True}}
        write_json(path, value)
        with self.assertRaisesRegex(ValueError, "dangerous key"):
            load_book_generation(self.root)

    def test_book_generation_writer_only_changes_selected_language_owners(self):
        directory = self.book_generation_tree()
        document = load_book_generation(self.root)
        stable_paths = [
            directory / "targets.d" / "french-0001.json",
            directory / "targets.d" / "spanish-0001.json",
            directory / "glossaries.d" / "spanish-appendix-glossary.json",
            directory / "script-sets.d" / "0010-spanish-main.json",
            directory / "_meta.json",
        ]
        for index, path in enumerate(stable_paths):
            timestamp = 1_700_000_000_000_000_000 + index
            os.utime(path, ns=(timestamp, timestamp))
        before = {
            path: (path.read_bytes(), path.stat().st_mtime_ns) for path in stable_paths
        }

        document["targets"] = [
            entry
            for entry in document["targets"]
            if not (entry["language"] == "spanish" and entry["chapter"] == 2)
        ]
        document["targets"].append(
            {
                "language": "spanish",
                "chapter": 3,
                "output": "spanish/book/chapters/ch03-thanks.tex",
                "scriptSet": "spanish-main",
            }
        )
        document["targets"].sort(
            key=lambda entry: (entry["language"], entry["chapter"])
        )
        write_book_generation_language(
            self.root,
            "spanish",
            document,
            expected_target_identities={"french/0001", "spanish/0001", "spanish/0003"},
            expected_handwritten_identities={"spanish/0004"},
        )

        self.assertFalse((directory / "targets.d" / "spanish-0002.json").exists())
        self.assertTrue((directory / "targets.d" / "spanish-0003.json").is_file())
        for path in stable_paths:
            self.assertEqual((path.read_bytes(), path.stat().st_mtime_ns), before[path])

    def test_book_generation_writer_prevalidates_expected_set_before_writes(self):
        directory = self.book_generation_tree()
        document = load_book_generation(self.root)
        path = directory / "targets.d" / "spanish-0001.json"
        before = path.read_bytes()
        document["targets"][1]["output"] = "spanish/book/chapters/changed.tex"

        with self.assertRaisesRegex(ValueError, "target owner set mismatch"):
            write_book_generation_language(
                self.root,
                "spanish",
                document,
                expected_target_identities={"spanish/0001"},
            )
        self.assertEqual(path.read_bytes(), before)


if __name__ == "__main__":
    unittest.main()
