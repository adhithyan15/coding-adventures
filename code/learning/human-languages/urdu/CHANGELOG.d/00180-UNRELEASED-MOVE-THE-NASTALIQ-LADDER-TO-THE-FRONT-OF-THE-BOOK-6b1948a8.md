## Unreleased — move the Nastaliq ladder to the front of the book (HL-C241)

- Redistributed all fourteen letter lessons out of chapters 16–18 and into
  chapters 1–8, two content lessons apart, so the ladder now runs from sequence
  35 to 295 instead of 730 to 1000. `UR-W16-*`, `UR-W17-*` and `UR-W18-*` were
  renumbered to `UR-W01-*` … `UR-W08-*` to match the chapters they now live in.
- **Closure violations fell 46 → 41**, and the number the exposure rule was
  quietly carrying fell with them: glyphs excused by the headword-romanization
  exemption dropped 263 → 142 inside Urdu. Headwords without romanization
  stayed at 0; glyphs shown but never taught stayed at 22.
- Retired chapter 16's outright breach of `minLessonsBetweenScriptSegments: 2`
  — seven letter lessons in seven consecutive positions. No two script lessons
  in the track are now closer than two content lessons apart.
- Reordered the ladder so **sīn** comes first, before alif, and taught it as
  **ش** *shīn* with its three dots taken off. The reader's first letter drawn
  from scratch is one they have already been looking at since *shukriyā*, and
  it is what makes `UR-C02-mera-naam` decodable in chapter 2 rather than 16.
- Rewrote every letter lesson that had been anchored to chapter 17 and 18
  vocabulary so it stands on words already taught where it now sits: gol he
  demonstrates its three faces on **ہم**, **کہانی** and **کمرہ** and points at
  the **ہ** that has closed *shukriyā* since chapter 1; nūn ghunna keeps
  **میں**, **ہیں** and **نہیں**; baṛī ye trades **کالے** for **میرے**; alif
  madda shows its two spellings of long *ā* inside **آسان**; pe uses **آپ**,
  **پیر** and **اپنا**; vāʾo uses **وکیل**, **ہوں** and **کون**. Every example
  word in every letter lesson is now spelled entirely from letters already
  taught.
- Stopped two lessons showing the reader script they could not yet decode:
  `UR-C06-hona` now names *hūṅ*'s dotless nūn instead of printing **ہوں** a
  lesson before the letter arrives, and `UR-C06-ana` names *āp* instead of
  printing **آپ** two chapters before pe. Both now say when the missing shape
  is coming, which is a better lesson than the silent one it replaces.
- Gave the moved letters real spaced returns rather than a single appearance:
  mīm reviews lām, baṛī ye's warm-up rewrites nūn ghunna, gol he is assessed
  again inside nun ghunna's and vāʾo's word blocks, and chapters 16–18 keep
  their reading payoffs, which now cash in letters learned eleven chapters
  earlier. Corpus-wide missed R3 reinforcement windows fell 4342 → 4324.
- Rebuilt the curriculum path to match: the ladder is interleaved into path
  segments 003–008-B with nine new script extension nodes, and the three old
  script extension nodes keep only their reading payoffs. Chapters 16 and 18
  were retitled, and chapters 1–8 now state the letters they teach.
- Recorded in `BACKLOG.d` that redistribution has a floor of 41 and why:
  simulating the whole remaining alphabet taught at policy pace still leaves 40,
  because eight untaught glyphs are first demanded inside chapter 7. The entry
  also names eight glyphs whose pedagogy already exists in content lessons that
  earn no closure credit, the measured letter order whose payoff cliff is at
  **ظ**, four "untaught" glyphs that are really Arabic and Persian etymon
  citations, and the three metrics this move cost.

