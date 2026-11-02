"""Tests for `lessons.py`, the tool over the sharded `lessons.d/`.

The point of sharding was to stop concurrent PRs conflicting on one 640KB
`lessons.md`. That only holds while two properties do, so both are pinned here
rather than assumed:

  * a lesson's FILENAME is its identity -- no `id:` field, no ordinal prefix,
    both of which have already failed in this repo (an ordinal scheme renamed
    21 files on one insertion; a separate `id` let two branches write the same
    id into differently-named files, which git merged in silence);

  * the aggregate view is DERIVED. If `render` could quietly drop a lesson,
    the shards would stop being the source of truth and nobody would notice,
    because the only thing anyone reads in bulk is the render.

The last test runs the real `lessons.d/` through `validate`, so a malformed
shard fails CI rather than waiting to be noticed.
"""

from __future__ import annotations

import contextlib
import io
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO / "code" / "scripts"))

import lessons  # noqa: E402

META = """\
# Lessons

## Categories

- Rust
- Python
"""


def write_dir(tmp: Path, shards: dict[str, str], meta: str = META) -> Path:
    directory = tmp / "lessons.d"
    directory.mkdir()
    (directory / "_meta.md").write_text(meta, encoding="utf-8")
    for name, text in shards.items():
        (directory / name).write_text(text, encoding="utf-8")
    return directory


def run(command, *args) -> tuple[int, str, str]:
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        code = command(*args)
    return code, out.getvalue(), err.getvalue()


class SlugTests(unittest.TestCase):
    def test_markup_does_not_change_the_slug(self):
        """`foo`, **foo** and foo must land on ONE file.

        If they did not, the same lesson written with and without emphasis
        would produce two files, and the duplicate detection that makes
        filename-as-identity safe would never fire.
        """
        plain = lessons.slugify("A grep for a shape finds it")
        self.assertEqual(lessons.slugify("A `grep` for a shape finds it"), plain)
        self.assertEqual(lessons.slugify("A **grep** for a shape finds it"), plain)

    def test_long_titles_cut_on_a_word_boundary(self):
        slug = lessons.slugify(" ".join(f"word{i}" for i in range(40)))
        self.assertLessEqual(len(slug), 80)
        self.assertFalse(slug.endswith("-"))
        # A hard character slice can end mid-word and make two unrelated
        # lessons collide; cutting on words keeps the tail intact.
        self.assertTrue(slug.split("-")[-1].startswith("word"))

    def test_taken_slugs_get_a_suffix(self):
        taken = {"a-lesson"}
        self.assertEqual(lessons.slugify("A lesson", taken), "a-lesson-2")

    def test_a_title_of_pure_punctuation_still_yields_a_name(self):
        self.assertEqual(lessons.slugify("!!! ???"), "lesson")


class ParseTests(unittest.TestCase):
    def test_frontmatter_and_title(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "x.md"
            path.write_text(
                "---\ncategory: Rust\n---\n\n# A claim\n\nbody text\n",
                encoding="utf-8",
            )
            lesson = lessons.parse_lesson(path)
            self.assertEqual(lesson.category, "Rust")
            self.assertEqual(lesson.title, "A claim")
            self.assertEqual(lesson.body, "body text")

    def test_missing_frontmatter_is_uncategorised_not_an_error(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "x.md"
            path.write_text("# A claim\n\nbody\n", encoding="utf-8")
            self.assertEqual(
                lessons.parse_lesson(path).category, lessons.UNCATEGORISED
            )

    def test_unterminated_frontmatter_raises(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "x.md"
            path.write_text("---\ncategory: Rust\n\n# A claim\n", encoding="utf-8")
            with self.assertRaises(ValueError):
                lessons.parse_lesson(path)

    def test_no_heading_raises(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "x.md"
            path.write_text("just prose, no heading\n", encoding="utf-8")
            with self.assertRaises(ValueError):
                lessons.parse_lesson(path)


class ValidateTests(unittest.TestCase):
    def test_a_clean_directory_passes(self):
        with tempfile.TemporaryDirectory() as tmp:
            d = write_dir(Path(tmp), {"a-claim.md": "# A claim\n\nbody\n"})
            self.assertEqual(run(lessons.cmd_validate, d)[0], 0)

    def test_unknown_category_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            d = write_dir(
                Path(tmp),
                {"a-claim.md": "---\ncategory: Gopher\n---\n\n# A claim\n\nb\n"},
            )
            code, _, err = run(lessons.cmd_validate, d)
            self.assertEqual(code, 1)
            self.assertIn("Gopher", err)

    def test_duplicate_title_fails(self):
        """The real migration surfaced one of these, hidden in the big file."""
        with tempfile.TemporaryDirectory() as tmp:
            d = write_dir(
                Path(tmp),
                {
                    "a-claim.md": "# A claim\n\nbody\n",
                    "a-claim-2.md": "# A claim\n\nother body\n",
                },
            )
            code, _, err = run(lessons.cmd_validate, d)
            self.assertEqual(code, 1)
            self.assertIn("duplicate title", err)

    def test_a_title_that_does_not_match_its_filename_fails(self):
        """Otherwise the filename stops being the identity.

        A lesson filed under a stale name can be added a second time under its
        correct one, and neither the duplicate-title check nor git would see a
        collision.
        """
        with tempfile.TemporaryDirectory() as tmp:
            d = write_dir(Path(tmp), {"something-else.md": "# A claim\n\nb\n"})
            code, _, err = run(lessons.cmd_validate, d)
            self.assertEqual(code, 1)
            self.assertIn("slugs to", err)

    def test_unbalanced_code_fence_fails(self):
        """A fence running off the end means a split code block -- text that is
        present but unreadable. Zero of the 502 migrated shards had one."""
        with tempfile.TemporaryDirectory() as tmp:
            d = write_dir(
                Path(tmp),
                {"a-claim.md": "# A claim\n\n```\nrm -rf /\n"},
            )
            code, _, err = run(lessons.cmd_validate, d)
            self.assertEqual(code, 1)
            self.assertIn("code fence", err)

    def test_balanced_code_fence_passes(self):
        with tempfile.TemporaryDirectory() as tmp:
            d = write_dir(
                Path(tmp),
                {"a-claim.md": "# A claim\n\n```\nsafe\n```\n"},
            )
            self.assertEqual(run(lessons.cmd_validate, d)[0], 0)

    def test_title_with_no_body_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            d = write_dir(Path(tmp), {"a-claim.md": "# A claim\n\n"})
            code, _, err = run(lessons.cmd_validate, d)
            self.assertEqual(code, 1)
            self.assertIn("no body", err)


class RenderTests(unittest.TestCase):
    def test_every_lesson_reaches_the_render(self):
        with tempfile.TemporaryDirectory() as tmp:
            d = write_dir(
                Path(tmp),
                {
                    "first-claim.md": "---\ncategory: Rust\n---\n\n# First claim\n\nalpha\n",
                    "second-claim.md": "---\ncategory: Python\n---\n\n# Second claim\n\nbeta\n",
                    "third-claim.md": "# Third claim\n\ngamma\n",
                },
            )
            text = lessons.render(d)
            for needle in ("First claim", "alpha", "Second claim", "beta",
                           "Third claim", "gamma"):
                self.assertIn(needle, text)

    def test_categories_render_in_meta_order_not_alphabetically(self):
        with tempfile.TemporaryDirectory() as tmp:
            d = write_dir(
                Path(tmp),
                {
                    "a-rust-claim.md": "---\ncategory: Rust\n---\n\n# A rust claim\n\nx\n",
                    "a-python-claim.md": "---\ncategory: Python\n---\n\n# A python claim\n\ny\n",
                },
            )
            text = lessons.render(d)
            self.assertLess(text.index("## Rust"), text.index("## Python"))

    def test_uncategorised_lessons_are_not_silently_dropped(self):
        with tempfile.TemporaryDirectory() as tmp:
            d = write_dir(Path(tmp), {"a-claim.md": "# A claim\n\nbody\n"})
            self.assertIn(lessons.UNCATEGORISED, lessons.render(d))

    def test_render_has_exactly_one_heading_per_lesson(self):
        """The aggregate's outline must not overcount.

        A shard's own sub-heading is `##`, which becomes `###` in the render --
        the same level as a lesson title. Across the real directory that was 21
        extra `###` from 6 shards, so the outline claimed 529 lessons where
        there were 508.
        """
        text = lessons.render(lessons.LESSONS_DIR)
        headings = sum(
            1 for line in text.splitlines() if line.startswith("### ")
        )
        self.assertEqual(headings, len(lessons.load()))

    def test_demote_leaves_fenced_shell_comments_alone(self):
        """`# comment` at column 0 of a shell block matches the heading pattern
        exactly and is not a heading."""
        body = "para\n\n```bash\n# not a heading\n```\n\n## real subheading"
        out = lessons.demote_headings(body)
        self.assertIn("# not a heading", out)
        self.assertNotIn("## not a heading", out)
        self.assertIn("### real subheading", out)

    def test_the_render_says_it_is_generated(self):
        """A committed aggregate would reinstate the shared file we removed."""
        with tempfile.TemporaryDirectory() as tmp:
            d = write_dir(Path(tmp), {"a-claim.md": "# A claim\n\nbody\n"})
            self.assertIn("GENERATED", lessons.render(d))


class NewTests(unittest.TestCase):
    def test_new_creates_a_shard(self):
        with tempfile.TemporaryDirectory() as tmp:
            d = write_dir(Path(tmp), {})
            code, _, _ = run(lessons.cmd_new, d, "A brand new claim", "Rust")
            self.assertEqual(code, 0)
            path = d / "a-brand-new-claim.md"
            self.assertTrue(path.exists())
            path.write_text(
                path.read_text(encoding="utf-8").replace(
                    lessons.PLACEHOLDER, "The actual lesson."
                ),
                encoding="utf-8",
            )
            self.assertEqual(run(lessons.cmd_validate, d)[0], 0)

    def test_an_unedited_placeholder_fails_validation(self):
        """`new` must not emit something `validate` rejects -- the workflow
        would fail on its own first step -- but an UNFILLED template must not
        merge either, or the directory fills with empty lessons."""
        with tempfile.TemporaryDirectory() as tmp:
            d = write_dir(Path(tmp), {})
            run(lessons.cmd_new, d, "A brand new claim", "Rust")
            code, _, err = run(lessons.cmd_validate, d)
            self.assertEqual(code, 1)
            self.assertIn("placeholder", err)

    def test_new_refuses_an_existing_slug(self):
        """Refusing beats auto-suffixing: the right move is to sharpen the
        lesson that already exists, not to file a near-duplicate beside it."""
        with tempfile.TemporaryDirectory() as tmp:
            d = write_dir(Path(tmp), {"a-claim.md": "# A claim\n\nbody\n"})
            code, _, err = run(lessons.cmd_new, d, "A claim", None)
            self.assertEqual(code, 1)
            self.assertIn("already exists", err)

    def test_new_refuses_an_unlisted_category(self):
        with tempfile.TemporaryDirectory() as tmp:
            d = write_dir(Path(tmp), {})
            code, _, err = run(lessons.cmd_new, d, "A claim", "Gopher")
            self.assertEqual(code, 1)
            self.assertIn("unknown category", err.lower())


class EncodingTests(unittest.TestCase):
    """`index` must survive a stdout that cannot encode a lesson title.

    7 real shard titles carry U+2192 or U+2260 (five and two respectively,
    measured over the 526-shard corpus at 83e38b3a3b). On a Windows cp1252
    console `index` used to die partway with UnicodeEncodeError, printing 31
    lines of 526 and exiting 1.

    No gate saw it, and the reason is this file rather than CI topology: before
    these tests the suite called `cmd_index` zero times, so nothing exercised
    the command's output on ANY platform.

    These tests deliberately do NOT use the `run` helper above. It captures
    through `io.StringIO`, which accepts every codepoint and raises nothing, so
    a test built on it would pass with or without the fix: decoration rather
    than a check. A `TextIOWrapper` over `BytesIO` with `encoding="cp1252"`
    reproduces the real console, and was observed to raise on the unfixed code
    before this test was written.
    """

    ARROW_TITLE = "A claim → with an arrow"

    def test_index_emits_every_shard_on_a_codepage_that_cannot_encode_them(self):
        """Run the real CLI with a stdout that cannot encode the titles.

        A SUBPROCESS, not an in-process call, and the difference is the whole
        test. `lessons.py` reconfigures `sys.stdout` at import, on the real
        stream. `contextlib.redirect_stdout` replaces that object AFTERWARDS,
        so an in-process test can only pass by re-applying the fix itself --
        which it would then pass without the fix in place. Two earlier versions
        of this test did exactly that and were deleted.

        `PYTHONIOENCODING` is set explicitly because the default child encoding
        varies by machine: cp1252 on the Windows box this was written on, UTF-8
        on the Linux runner that gates. Pinning it makes the test reproduce the
        failing condition everywhere rather than only where the locale happens
        to supply it.

        No `cwd`: `LESSONS_DIR` comes from `parents[2]` of the script path, so
        the command is invoked identically from anywhere.
        """
        script = REPO / "code" / "scripts" / "lessons.py"
        process = subprocess.run(
            [sys.executable, os.fspath(script), "index"],
            capture_output=True,
            check=False,
            env={**os.environ, "PYTHONIOENCODING": "cp1252"},
        )
        self.assertEqual(process.returncode, 0, process.stderr.decode("utf-8", "replace"))
        self.assertEqual(process.stderr, b"")
        emitted = process.stdout.decode("utf-8").splitlines()
        self.assertEqual(len(emitted), len(lessons.load()))

    def test_the_cp1252_harness_really_cannot_encode_the_title(self):
        """The negative arm: without the reconfigure the harness MUST raise.

        An assertion never observed to fire is decoration, so the harness is
        checked against the character it exists to catch.
        """
        stream = io.TextIOWrapper(io.BytesIO(), encoding="cp1252")
        with self.assertRaises(UnicodeEncodeError):
            stream.write(self.ARROW_TITLE)
            stream.flush()

    def test_a_title_the_codepage_cannot_encode_is_actually_present(self):
        """The corpus arm above is only meaningful while such a title exists.

        If every shard title became plain ASCII the subprocess test would keep
        passing while checking nothing, so the precondition is asserted rather
        than assumed. Measured 2026-09-16: 7 titles qualify (five carrying
        U+2192, two U+2260), over the 526-shard corpus at 83e38b3a3b.
        """
        unencodable = []
        for lesson in lessons.load():
            try:
                lesson.title.encode("cp1252")
            except UnicodeEncodeError:
                unencodable.append(lesson.slug)
        self.assertGreater(
            len(unencodable),
            0,
            "no shard title exercises the codepage path any more",
        )


class RepositoryTests(unittest.TestCase):
    def test_the_real_lessons_directory_validates(self):
        self.assertTrue(lessons.LESSONS_DIR.is_dir(), "lessons.d/ is missing")
        code, out, err = run(lessons.cmd_validate, lessons.LESSONS_DIR)
        self.assertEqual(code, 0, f"{out}\n{err}")

    def test_the_real_directory_has_the_migrated_lessons(self):
        """A count floor, so an accident that empties the directory is loud.

        The migration wrote 500 shards. The floor is deliberately below that
        and never needs raising -- it exists to catch a wipe, not to pin a
        number that every new lesson would invalidate.
        """
        self.assertGreater(len(lessons.load()), 400)

    def test_no_shard_is_an_accidental_meta_copy(self):
        titles = [item.title for item in lessons.load()]
        self.assertEqual(len(titles), len(set(titles)))



GOOD_QUIRK = """\
---
package: rust/ct-compare
lane: rust
rung: E4
status: open
---

# A toolchain hides an early exit

The optimiser can reintroduce an early exit; the lane checks its IR.
"""


def write_quirks(tmp: Path, shards: dict[str, str]) -> Path:
    directory = tmp / "quirks.d"
    directory.mkdir()
    (directory / "_meta.md").write_text("# Quirks\n", encoding="utf-8")
    for name, text in shards.items():
        (directory / name).write_text(text, encoding="utf-8")
    return directory


class QuirkValidationTests(unittest.TestCase):
    """`quirks.d/` shares the lesson layout but has closed, required frontmatter
    and must never carry reproducing material (VER00 section 7.3)."""

    def validate(self, text: str, name: str = "a-toolchain-hides-an-early-exit.md"):
        with tempfile.TemporaryDirectory() as tmp:
            directory = write_quirks(Path(tmp), {name: text})
            return run(lessons.cmd_validate_quirks, directory)

    def test_a_well_formed_quirk_passes(self):
        code, out, err = self.validate(GOOD_QUIRK)
        self.assertEqual(code, 0, err)
        self.assertIn("1 shards, 0 problem(s)", out)

    def test_every_field_is_required(self):
        for field in ("package", "lane", "rung", "status"):
            with self.subTest(field=field):
                text = "\n".join(
                    line for line in GOOD_QUIRK.splitlines() if not line.startswith(field + ":")
                )
                code, _, err = self.validate(text + "\n")
                self.assertEqual(code, 1)
                self.assertIn(f"missing required field {field!r}", err)

    def test_unknown_fields_and_values_are_rejected(self):
        cases = {
            "category: Rust\n---": "unknown field",
            "rung: E9": "rung 'E9'",
            "status: fixed": "status 'fixed'",
        }
        for replacement, message in cases.items():
            with self.subTest(replacement=replacement):
                if replacement.startswith("category"):
                    text = GOOD_QUIRK.replace("status: open\n---", "status: open\n" + replacement, 1)
                elif replacement.startswith("rung"):
                    text = GOOD_QUIRK.replace("rung: E4", replacement)
                else:
                    text = GOOD_QUIRK.replace("status: open", replacement)
                code, _, err = self.validate(text)
                self.assertEqual(code, 1)
                self.assertIn(message, err)

    def test_reproducing_material_is_rejected(self):
        for leak in (
            "seed: 0x1234",
            "- tape = 1 2 3",
            "Counterexample: left=00 right=01",
            "reproducer: see attached",
            "The failing input was 00112233445566778899aabbccddeeff.",
            "seed 0x00112233445566778899aabbccddeeff00112233",
            "id 0011-2233-4455-6677-8899-aabb-ccdd",
            "The run used 1234567890123456 as its seed.",
            "`seed`: 1",
            "**Seed** - 7",
            "tape 3 4 5",
        ):
            with self.subTest(leak=leak):
                code, _, err = self.validate(GOOD_QUIRK + "\n" + leak + "\n")
                self.assertEqual(code, 1)
                self.assertIn("reproducing input", err)

    def test_a_duplicate_frontmatter_key_is_rejected(self):
        text = GOOD_QUIRK.replace("status: open", "status: bogus\nstatus: open")
        code, _, err = self.validate(text)
        self.assertEqual(code, 1)
        self.assertIn("duplicate frontmatter key 'status'", err)

    def test_the_keyword_pattern_is_linear_on_blank_lines(self):
        import time

        start = time.perf_counter()
        lessons.QUIRK_SECRET_LINE.search("\n" * 20000 + "x")
        self.assertLess(time.perf_counter() - start, 0.5)

    def test_the_new_quirk_placeholder_must_be_replaced(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = write_quirks(Path(tmp), {})
            code, _, _ = run(
                lessons.cmd_new_quirk, directory, "A toolchain hides an early exit",
                "rust/ct-compare", "rust", "E4", "open",
            )
            self.assertEqual(code, 0)
            code, _, err = run(lessons.cmd_validate_quirks, directory)
            self.assertEqual(code, 1)
            self.assertIn("placeholder", err)

    def test_new_quirk_rejects_bad_values_and_duplicates(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = write_quirks(Path(tmp), {})
            args = (directory, "A toolchain hides an early exit", "*", "*")
            self.assertEqual(run(lessons.cmd_new_quirk, *args, "E9", "open")[0], 1)
            self.assertEqual(run(lessons.cmd_new_quirk, *args, "E4", "done")[0], 1)
            self.assertEqual(run(lessons.cmd_new_quirk, *args, "E4", "open")[0], 0)
            self.assertEqual(run(lessons.cmd_new_quirk, *args, "E4", "open")[0], 1)

    def test_the_filename_must_derive_from_the_title(self):
        code, _, err = self.validate(GOOD_QUIRK, name="something-else.md")
        self.assertEqual(code, 1)
        self.assertIn("title slugs to", err)

    def test_the_repository_quirks_store_is_valid(self):
        code, _, err = run(lessons.cmd_validate_quirks, lessons.QUIRKS_DIR)
        self.assertEqual(code, 0, err)


if __name__ == "__main__":
    unittest.main()
