### Changed — the inline-letters section is a `script` block, honestly

- `## The letters in this word` — HL00's inline-letters section, used by **240 lessons
  across 12 tracks** — parsed as `unknown`, which schema v2 rejects. That single gap
  blocked the v2 migration for every Indic track at once. It now parses as `script`,
  which is what it has always been: the place a word lesson teaches the glyphs that word
  needs.
- **This costs 20 points of drivable share (84% → 64%) and that is the point.** A glyph
  shape cannot be read aloud, so the previous number advertised a driving edition that
  would have narrated "ब plus the o-mātrā" at somebody on a motorway. Corpus moves
  `voice` 957 → 726, `sight` 124 → 355, `pen` unchanged at 53, unstartable chapters
  44 → 92.
- **The loss is recoverable and the route is known.** HL-C41 gave `writing` blocks a
  `coreModality` so a hands-free view can set them aside, and the inline-letters section
  is detachable in exactly that sense — HL00 calls it optional scaffolding a fluent reader
  skims. Adding `script` to `DETACHABLE_BLOCK_TYPES` was tried and reverted here: the
  model currently conflates "detachable" with "is a writing segment", so script blocks
  began claiming a lesson needs a **pen** to read letters (`pen` 53 → 309) and reported
  276 writing segments that are nothing of the kind. Separating those two ideas returns
  the core share to ~86% with the honest label intact, and is the natural next slice.

