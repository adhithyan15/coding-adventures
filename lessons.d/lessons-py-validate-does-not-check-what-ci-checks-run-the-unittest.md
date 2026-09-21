---
category: Repo policy / workflow reminders
---

# lessons.py validate does not check what CI checks -- run the unittest suite

After adding a lesson shard I ran `python3 code/scripts/lessons.py validate`,
got `lessons: 603 shards, 0 problem(s)`, and treated that as the shard being
well-formed. CI then failed the "Repo-wide metadata contracts" job on
`test_render_has_exactly_one_heading_per_lesson`: `AssertionError: 605 != 603`.

The two check different things. `validate` checks each shard in isolation —
front matter, title, filename agreement. The unittest suite additionally checks
properties of the *rendered aggregate*, and that is where the real constraint
lives: a shard's `##` sub-heading is demoted to `###` in the render, which is
the same level the render uses for lesson titles, so every sub-heading inflates
the outline by one. My shard had two, hence 605 against 603.

So a lesson shard gets exactly one heading: its `#` title. Structure the body
with bold lead-ins or plain paragraphs, never `##`. The test's own docstring
records that this already happened once at scale — 21 extra headings from 6
shards, an outline claiming 529 lessons where there were 508 — so the trap is
known and the guard exists; I simply ran the wrong command and believed it.

The general form: a tool named `validate` is not automatically the thing CI
runs, and a green subset is not a green gate. Before pushing, run what the
workflow runs. For lessons that is:

```bash
python3 -m unittest discover -s code/scripts/tests -p 'test_lessons.py'
```

Reading the workflow file for the job that gates your change is cheaper than a
CI round trip, and it is the only way to know which of several plausible
commands is the authoritative one.
