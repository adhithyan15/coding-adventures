## Drivable-prefix reordering audit — no lesson moved (2026-08-06)

- Audited HL-C30's proposal to raise Arabic's drivable prefix by moving the
  `AR-W*` writing lessons that open Chapters 3 and 4 later in their chapters.
  **No lesson was moved, and none should be.** Every Arabic number is unchanged:
  37 `voice` / 18 `sight` / 16 `pen`, 52% drivable, 31 lessons reachable in
  chapter-prefix order across 27 chapters. Per chapter the prefix is 4/14, 6/12,
  0/9, 0/8, 1/2, 1, 1, 0/2, 2, 2, 2, 0/1, 1, 0/1, 1, 1, 1, 1, 0/1, 0/1, then 1
  for each of Chapters 21–27 — before and after.
- **Chapters 3 and 4 are prefix-0 under every legal ordering**, not just the
  authored one. A prefix can only start with a lesson that has no in-chapter
  prerequisite. Chapter 3's only such roots are `AR-W07-hook-family-ha-kha`
  (`pen`) and `AR-C03-kayfa` (`sight` — it carries a two-column table);
  Chapter 4's only root is `AR-W10-ayn`, because `AR-C04-maa-with` declares it
  as a prerequisite — مع cannot be read without ʿayn. Removing all six writing
  lessons would leave both chapters at 0.
- **The obstacle is the table, not the script.** All 18 of the track's `sight`
  lessons are `sight` because of a Markdown table — 18 of 18, none because of a
  script block. Only `AR-C03-bi-khayr` is `voice` in Chapter 3 and it sits
  behind kayfa → hal → kayfa-ḥāluka; all five non-writing lessons in Chapter 4
  are `sight`. Recovering these chapters is HL-C17 (table linearisation) work.
- The move would also have broken the teaching. `AR-C03-kayfa` tells the reader
  ك has already been written "in the writing set" — it assumes
  `AR-W08-kaf-and-ra`, which requires `AR-W07` — and `AR-W09-khayr-bikhayr`
  assembles خير so the *bi-khayr* reply the chapter ends on can be written by
  hand. HL00's inline-letters rule puts them where they are.
- Chapters 8, 12, 14, 19 and 20 are also prefix 0 and also have nothing to
  reorder: 12, 14, 19 and 20 hold a single table-bearing lesson each, and
  Chapter 8's second lesson declares its table-bearing first lesson as a
  prerequisite.
- Recorded but not acted on: Chapters 1 and 2 are **undercounted**. All 26 of
  their lessons are legacy and carry no `sequence`, so the report sorts them
  alphabetically and reports prefixes of 4 and 6 where the authored
  `curriculum.json` path gives 7 and 7. Recovering those four lessons means
  giving legacy lessons sequences — a schema migration no track has made (0 of
  565 legacy lessons corpus-wide carry one) and one the validator cannot check
  for collisions outside schema v2. Slots 10–260 are reserved below Chapter 3's
  270 for whoever does it. Full analysis in
  the [`../BACKLOG.d/`](../BACKLOG.d/) history.

