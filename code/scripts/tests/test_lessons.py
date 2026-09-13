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


if __name__ == "__main__":
    unittest.main()
