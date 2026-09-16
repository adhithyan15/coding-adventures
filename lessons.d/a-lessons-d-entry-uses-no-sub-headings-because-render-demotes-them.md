---
category: Repo policy / workflow reminders
---

# A lessons.d entry uses no sub-headings, because render demotes them onto the lesson-title level and the outline overcounts

Two new `lessons.d` entries used `##` sub-headings to organise a long note.
`code/scripts/tests/test_lessons.py::test_render_has_exactly_one_heading_per_lesson`
failed in CI:

```
AssertionError: 539 != 536
```

`lessons.py render` **demotes every heading by one level** when it folds the
shards into the aggregate, so a shard's own `##` becomes `###` — the same level
the renderer uses for a lesson title. Six sub-headings across two files became
six phantom lessons in the outline.

**The convention is already unanimous and I did not check it.** Of 538 entries
in `lessons.d/`, exactly one file uses `##`, and that file is `_meta.md`, which
is the entry point rather than a lesson. Every real entry is an H1 title
followed by prose, tables and fenced blocks. One `grep -c '^## ' lessons.d/*.md`
before writing would have shown that in a second.

**The fix is free.** A `##` heading becomes a **bold lead-in** at the start of
its paragraph. It reads the same, it survives the render, and the outline stays
honest. Tables, block quotes and fenced code are all fine at any depth — the
demotion only touches ATX headings.

The general lesson is about where a format rule lives. This one is not written
in `CLAUDE.md` or in `_meta.md`; it is enforced by a test whose **docstring
explains it fully** and which nobody reads until it goes red. When adding a
file to a directory of 500 near-identical files, the cheapest possible check is
to ask what every other file in it does — and the answer is usually a one-line
grep, not a document.

**Run `python3 -m unittest discover -s code/scripts/tests -p 'test_lessons.py'`
after touching `lessons.d/`.** It takes 0.25 seconds. It is not part of the
human-language gate set, so a chapter's twelve `check:*` gates and its full
vitest suite all pass with this broken.
