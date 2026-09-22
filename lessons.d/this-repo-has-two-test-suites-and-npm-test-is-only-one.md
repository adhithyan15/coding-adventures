---
category: Testing & coverage
---

# This repo has two test suites and `npm test` is only one of them

Chapter 469 shipped with a full local verification: preflight 6/6, `npm run
validate` 21/21, and `npm test` green at 173 files and 2,225 tests. CI then
failed on the very first check, `Repo-wide metadata contracts`:

```
lessons: 634 shards, 0 problem(s)
FAIL: test_render_has_exactly_one_heading_per_lesson
AssertionError: 636 != 634
```

The cause was a `lessons.d/` shard filed in the same commit. It used `##`
subheadings to organise a long entry. `lessons.py render` demotes a shard's
own headings by one level, so `##` becomes `###` — the same level the
aggregate uses for a lesson title — and the outline claimed two more lessons
than exist. The failing test's own docstring says exactly this, and 633 of the
634 shards use bold lead-ins and flowing prose instead. Only `_meta.md`, which
is the entry point rather than a lesson, carries `##`.

The shard was not the real mistake; not running the gate that covers it was.
`npm test` runs the TypeScript package's vitest suite. It does not touch
`code/scripts/tests/`, which is Python `unittest` and is where the corpus
metadata contracts live — `test_lessons.py`, `test_ci_gate_registry.py`,
`test_forme_spec_map.py` and the rest of a long battery. A change to
`lessons.d/`, `BACKLOG.d/` or anything else `code/scripts/` validates is
invisible to `npm test` by construction.

Before pushing, run the gate that covers what you actually changed, not the
one you happen to know:

```sh
python3 code/scripts/lessons.py validate
python3 -m unittest discover -s code/scripts/tests -p 'test_lessons.py'
```

More generally: open `.github/workflows/ci.yml`, find the job whose name
matches the files in your diff, and run its commands. That is cheaper than a
CI round trip and it is the only way to know a gate exists at all. A green
`npm test` is evidence about the TypeScript package and nothing else.
