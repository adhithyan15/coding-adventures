## Unreleased — the curriculum plans leave the first-paint path

CI failed the eager-bundle gate at **502,251 bytes against a 500,000 limit**.
The offender was `curriculum-plans`, a single chunk holding all 22 tracks'
`curriculum.json` — and it was on the first-paint path only because
`src/curriculum.ts` globbed them with `{ eager: true }`.

Nothing on that path reads them. The first frame is the header plus "Loading
the lessons needed for this view…"; every plan consumer runs after the awaited
corpus refresh. So half a megabyte of authored path, extension and spine ledger
was being downloaded and parsed before a screen that shows none of it.

The glob is now lazy and **one chunk per track** (`curriculum-<track>`), fetched
by `loadCurriculumPlans()` inside the corpus refresh the app already awaited.
`main.ts` derives its plan-dependent state in one place, `installCurriculumPlans()`:
the mapped-lesson-id set, and the restored Learn progress (which is pruned
against the authored paths, so it could never have been restored earlier
anyway). Which tracks are *offered* still resolves synchronously, from
`MAPPED_LANGUAGE_IDS` — the glob's KEYS are file paths, not file contents, so
that question costs no plan bytes.

**Largest eager chunk: 502,251 → 287,353 bytes**, and the ceiling stops being a
countdown: no `curriculum.json` byte is eagerly imported any more, so the daily
tranche of lessons cannot walk it back up. Per-track chunking also means a
Telugu tranche re-downloads Telugu's plan alone instead of invalidating one
shared half-megabyte blob. This is the fix HL-C110 named as the next candidate,
done the way it insisted — the bytes left the preload set, rather than being
split across several eager chunks to satisfy a gate that measures the largest
one. It supersedes the `maxSize: 250_000` briefly placed on the old
`curriculum-plans` group, which turned one 502 kB eager chunk into four smaller
ones the browser still downloaded in full before first paint.

Two things the eager import got for free and the lazy one has to earn, both
handled here. Failure is **not** memoised — caching a rejected promise would
make one dropped chunk permanent, leaving the app in its error frame until a
reload, so a failed attempt is forgotten and a retry really retries. And the
picker now offers tracks by DIRECTORY name while every lookup still resolves
them by the plan's own `language` field, so a test pins that the two agree —
a track whose folder disagreed with its `language` would otherwise appear in the
picker and resolve to nothing.

Behaviour unchanged, verified in a browser and not only in tests: 22 lazy
`curriculum-*` chunks fetch on load with no console errors, Learn shows all 22
frontier steps with their per-track lesson counts, the language picker still
reports 3,034 mapped micro-lessons and 851 extensions, Lessons and Concepts
render the full 3,120-lesson corpus, and a seeded `learn-progress` blob still
restores and advances the Spanish frontier.

