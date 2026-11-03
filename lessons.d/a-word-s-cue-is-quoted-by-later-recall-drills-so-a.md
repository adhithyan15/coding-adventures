---
category: Testing & coverage
---

# A word's cue is quoted by later recall drills, so a look-at-the-page phrase inside it turns those lessons into sight lessons

**What went wrong.** Chinese B1 tranche j gave 守岁 *shǒusuì* the cue "to see the
new year in". Generation went through, but the openers check then failed: chapter
334's first lesson (正月) was now a *sight* lesson, with reason `sight-cue`.
`human-language-data`'s modality reader (`SIGHT_CUE_RULES` in `src/modality.ts`)
treats "see the", "look at", "shown below", "the table", "the chart", "column" and
"written above" as an instruction to look at the page. A word's cue is not only
printed in its own lesson. The next lessons quote it again in their recall drills
("say the Mandarin for to see the new year in"). So one idiomatic English gloss made
a lesson in a *different* chapter undrivable by ear. That lesson opened its chapter,
and openers fall under the by-content ceiling in `tests/drivable-prefix.test.ts`,
a ceiling that may only fall.

**The fix.** The cue became "to keep the New Year vigil". The first rewrite, "to
stay up to welcome the new year", broke the builder's six-word cue limit, which
cost another run. The tranche builder now refuses any cue or gloss that contains
one of those phrases. It does so before anything is generated, and it names the
phrase it found.

**Do differently.** When authoring cues and glosses, screen them against the
modality reader's phrase list, not just against the lesson that owns them. Text
is reused: cues, glosses and hooks reappear in later recall drills and reviews.
A check that only reads the owning lesson misses what the reuse does. Keep the
builder's copy of the list in step with `SIGHT_CUE_RULES`. When a reworded cue is
needed, check it against every builder rule (length, single sense, no clash with
a neighbouring cue such as 除夕's "lunar New Year's Eve") before re-running a
multi-hour generation.
