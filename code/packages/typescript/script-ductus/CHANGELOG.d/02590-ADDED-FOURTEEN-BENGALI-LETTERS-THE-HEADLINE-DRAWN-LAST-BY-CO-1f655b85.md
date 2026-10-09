### Added — fourteen Bengali letters, the headline drawn last by convention, from LipiTk native-writer traces

- **Fourteen new entries** at the end of `src/strokes/bengali.ts`, keyed
  `bengali:<letter>` and sourced through `bengaliLetterSource` to their rows
  in `data/scripts/bengali.json`: ই চ ছ জ ড ত দ ন ফ ভ ম য ল হ. Each body
  follows the majority of HP Labs India's LipiTk 4.0 Bangla recognizer
  prototypes (MIT model; counts and shares only, no trace copied): one body
  stroke for চ ছ ড ত দ ন ফ ভ ম য ল হ (72.6% to 94.0% of each letter's
  prototypes), two for জ (189/268, left part first in all 189) and ই
  (165/218, the lower part first in 162). Paths are fitted to the bundled Noto
  Sans Bengali outline at the default tolerances: every stroke 100% on ink,
  untraced ink 0% (ছ 0.3%); no override, no excused ink.
- **The headline convention.** In none of the fourteen does one headline
  placement win a majority of the traces (for example ন: 238 of 498 draw no
  headline run, 195 end with it, 44 open with it). So, by a convention each
  record names, the body is drawn in its majority order and the headline is
  drawn LAST, as its own stroke, left to right: the way the Bengali track's
  lessons already teach it (body first, bar last), since the bar runs across
  the whole word, as in Devanagari. Where a placement does win and covers the
  printed bar (ব, র) the path still follows it. The owner's header states
  both cases.
- **Left out, with reasons in the inventory:** ক (bar first, 296/524, but
  stopped at the stem in 281, so it does not cover Noto's bar over the loop);
  আ গ ট ধ প (no body stroke count wins a majority: 46%, 45%, 46%, 43%, 41%);
  স (two body strokes in 133/250, a near tie); শ (one stroke and a clear
  start and end, but the way between splits 66 to 77); ঝ (Noto prints its
  headline in two pieces around the rising right stem, so a headline drawn
  last cannot be one stroke on the ink).
- **Evidence.** `tests/strokes/bengali.test.ts` takes the twenty-four glyphs
  and pins `HEADLINE_LAST_BY_CONVENTION`: exactly those fourteen end with one
  straight left-to-right headline stroke and say "By the convention" in their
  variation, and ব and র keep the headline first. It also pins the turns (ত ভ
  ড জ clockwise, চ counterclockwise, ম's loop clockwise), the stem climbed
  from its foot in ন ম য ল, the ends of ছ দ হ ফ, and ই as হ's body plus the
  hook climbed to its tip. `tests/ductusview/bengali.test.ts` adds their
  frames, lifts and summaries.
- **Pins and ledger.** `tests/stroke-ownership/bengali.json`: 10 -> 24; key
  and data hashes move; no other script changes. `filmstrip-geometry.d/
  bengali.json` gains the fourteen entries; no existing entry changes.
- **README:** the Bengali section states the convention and what stays out.
