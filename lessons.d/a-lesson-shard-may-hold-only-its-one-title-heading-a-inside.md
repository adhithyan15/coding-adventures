---
category: Testing & coverage
---

# A lesson shard may hold only its one title heading; a ## inside it breaks the rendered aggregate's heading count

PR #19513 (WX-P1a) failed "Repo-wide metadata contracts": `test_lessons.py`
`test_render_has_exactly_one_heading_per_lesson` counted 2015 headings for 2014
lessons. The hash-functions lesson had grown a `## Resolution (...)` section;
`lessons.py render` demotes each shard's title into the aggregate, so any extra
heading inside a shard is counted as another lesson.

Fix: write follow-up sections as a bold lead-in line (`**Resolution (...).**`),
never a markdown heading. Before pushing a lessons.d edit, run
`python3 -m pytest -q code/scripts/tests/test_lessons.py`, which takes about a
second; `lessons.py validate` alone does not catch this.
