### Fixed — the plan CLI and the script ramp stop paying for work they throw away

- **Why.** `tests/plan-cli.test.ts`'s first case sat near its 120s budget
  when the machine was loaded (about 60s on an idle machine, about 150s on
  a slower one), and the whole file took about 220s. `tests/ramp.test.ts`
  spent about 13s loading and measuring the corpus. Each assertion held.
  No timeout was raised, no case was skipped, and no assertion was
  weakened. The work got cheaper instead.
- **`src/ramp.ts`, script lookups.** `systemOf` and `belongsToAny` now
  answer from one memoised list per code point: every script in the map
  the character belongs to, in matcher order. Each call used to run the
  punctuation class and up to two dozen `\p{Script_Extensions=…}` regexes,
  about 2.2 million times per measurement across about 1,000 distinct
  glyphs. The memo holds the result of a pure function over a frozen
  matcher table, keeps single code points only, and freezes its lists.
  It cannot change an answer.
- **`src/ramp.ts`, `distinctNonAsciiCodePoints`.** This is new and
  exported. It yields exactly the non-ASCII entries of `new Set(text)`, in
  the same order, with surrogate pairs joined the way the string iterator
  joins them. The script ramp and the script closure
  (`src/script-closure.ts`) used `new Set(lesson.body)` and then asked
  about every entry, and every ASCII entry always answered "not script".
  Lesson bodies are mostly English and Markdown, about 41 million code
  units, so that was most of each measurement's cost. Whether ASCII can
  ever be script is computed from the matchers at module load. If a
  future map entry makes it possible, the walk falls back to `new Set`.
  Script ramp: 2.2s → 0.3s. Script closure: 1.4s → 0.2s.
- **`src/report.ts`, `buildLevelGateSections`.** This is new. It builds
  the level gate and the sections under it (levels, ramp, continuity,
  script closure, writing stages). `buildCurriculumGapReport` now calls it
  at the same point it used to build them, so there is one wiring for
  both callers.
- **`src/plan-cli.ts`.** The plan CLI calls `buildLevelGateSections`
  instead of building the whole gap report and reading three of its
  fields. The duration estimates, modality derivation, book coverage and
  chapter gates it no longer builds were about a third of every run. It
  also stops reading the chapter ledgers and the narration table-width
  policy, which only those sections used. One plan run: about 17s → about
  9.5s. Output is byte-identical for `--head 200`, `--head 25`, `--format
  json`, `--ceiling A1` and `--ceiling B2 --head 0`. The full gap report's
  JSON is also byte-identical before and after.
- **`tests/plan-cli.test.ts`.** One pruned corpus copy is made at import
  and deleted once in `afterAll` (120s, the budget the per-case hook
  already stated). Each case used to make its own copy, about 85,000
  files, 20s to copy and 9s to delete. The two cases that corrupt the
  French A1 inventory edit the shared copy through `editInventory`, which
  records the original bytes and has `afterEach` write them back. The
  clean plan run is memoised: the first case compares it with the real
  corpus, and the flag case reads the same run's exit code. The exit code
  does not depend on `--head`. The inventories-only fixture for the
  duplicate case is unchanged.
- **Tests.** `tests/ramp.test.ts` adds "the fast glyph walk". It pins that
  no ASCII character belongs to any script in the map. It checks the walk
  against filtered `new Set(text)` on 2,000 seeded inputs, including
  astral and lone-surrogate cases. It also checks that memoised lookups
  match fresh ones.
- **Timings, each file alone on an idle 4-core machine.** `plan-cli`:
  218s → 63s. Its first case went 58–63s → 22s, and the two corrupting
  cases went about 35–52s → 11–14s each. `ramp`: 13.1s → 6.4s.

`tests/continuity.test.ts` had the same shape: five real-corpus cases each
called `loadEverything()` and four ran `measureContinuity` over the whole
corpus inside a 30 s budget, and "keeps Spanish reading order explicit"
timed out in loaded full runs while its assertions held. The corpus is now
loaded and measured once at import; the five cases read the shared result
and take 0–3 ms each.
