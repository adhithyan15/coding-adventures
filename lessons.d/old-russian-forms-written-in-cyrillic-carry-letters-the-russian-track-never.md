---
category: Testing & coverage
---

# Old Russian forms written in Cyrillic carry letters the Russian track never teaches

Filling 1,148 Russian etymologies from Vasmer, the lesson-level checks were
green and the full `vitest run` failed three tests:

- **`script-closure`** found 52 lessons using a glyph the track never teaches:
  the hard sign **ъ** in Old Russian and Old Church Slavonic forms quoted the
  way Vasmer prints them (*сапогъ*, *котъка*, *къде*), **ѣ** (*надобѣ*),
  Ukrainian **і** (*місто*), and **щ** in *Красная площадь* in a chapter
  before **щ** is taught. The Russian track teaches modern spelling only, and
  the ceiling is 0. Fix: romanize any old form that needs those letters with
  the scholarly transliteration (*sapogŭ*, *kotŭka*, *kŭde*, *nadobě*,
  *misto*), and name a later-taught word in English.
- **`continuity`** (forward references, pinned at 0) caught two hand-written
  lessons whose new origin sentence printed a later headword (*тот*,
  *медленный*). A word taught later must be described, not printed.
- **`info-dump`**: "to get used to" in a gloss is an `is-used-for` rule
  statement; the ceiling has no headroom.

Do: for a track in a non-Latin script, quote old forms in transliteration
unless every letter is one the track teaches before that chapter, and run
`script-closure`, `continuity` and `info-dump` alone (seconds) before the full
suite. A home-made forward-reference script is no substitute for `continuity`.
