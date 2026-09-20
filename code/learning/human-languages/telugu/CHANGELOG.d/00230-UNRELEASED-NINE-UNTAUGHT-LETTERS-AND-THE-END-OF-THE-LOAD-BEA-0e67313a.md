## Unreleased — nine untaught letters, and the end of the load-bearing headword

Telugu's script-closure violations fall **45 → 22** and its load-bearing
headwords **17 → 0**. Both numbers come from `report-cli`'s `script closure` and
`exposure` lines; corpus-wide they move 643 → 620 and 283 → 266, and every glyph
of that movement is Telugu's.

**Nine new script lessons, `TE-S125` … `TE-S133`**, one letter each, continuing
the `TE-S01` … `TE-S124` chain:

| lesson | letter | placed at | first met in |
|---|---|---|---|
| `TE-S125-letter-ha` | హ | ch. 22 | సహాయం, హైదరాబాద్ |
| `TE-S126-letter-nna` | ణ | ch. 23 | వాతావరణం, శ్రావణం |
| `TE-S127-letter-ja` | జ | ch. 24 | రోజు, జ్యేష్ఠం |
| `TE-S128-letter-bha` | భ | ch. 25 | శుభ రాత్రి, భాద్రపదం |
| `TE-S129-letter-vocalic-r` | ఋ | ch. 26 | ఋతువు |
| `TE-S130-letter-tha` | థ | ch. 34 | అర్థం |
| `TE-S131-vowel-sign-vocalic-r` | ృ | ch. 39 | హృదయం |
| `TE-S132-letter-nya` | ఞ | ch. 51 | కృతజ్ఞత |
| `TE-S133-vowel-sign-au` | ౌ | ch. 51 | గౌరవం |

Placement is the whole point and it is not decorative. Each letter is taught in
the chapter **after** the first word that carries it has been glossed, so no
lesson asks a reader to decode a shape before they can already say the word it
lives in. That ordering is why ృ waits until chapter 39 and ఞ until chapter 51
rather than being batched with the rest: the words that need them do not arrive
until then. Four of the nine also pair off against letters already taught — ణ
against న, భ against బ, థ against త, ృ against ఋ — so each is a contrast rather
than an isolated shape.

The letters were chosen by measurement, not by inventory order. All nine were
**shown and never taught** anywhere in the track, which is exactly the failure
the gloss-first rule forbids. Telugu's taught-glyph count goes 40 → 51 and its
never-taught count 24 → 13; ఠ and ఫ picked up credit incidentally from the
example words two of those lessons carry (జ్యేష్ఠం and ఫాల్గుణం).

**Seventeen headwords gained a `romanization`.** `TE-C02-naa-peru`,
`TE-C02-mii-peru-emiti`, `TE-C02-santosham`, `TE-C03-elaa`,
`TE-C03-miiru-elaa-unnaaru`, `TE-C05-nenu-telugu-maatlaadataanu`,
`TE-C07-numbers-6-10`, `TE-C09-kshaminchandi`, `TE-C10-vaaram`,
`TE-C11-rangulu`, `TE-C12-kutumbam`, `TE-C14-rutuvulu`, `TE-C15-neellu-biyyam`,
`TE-C16-nelalu`, `TE-C17-madhyaahnam-ardharaatri`, `TE-C21-kukka-pilli` and
`TE-C31-subha-madhyahnam-register` all printed their headword in Telugu script
with no spoken form recorded in frontmatter, which made those headwords
load-bearing rather than exposure. Every romanization added here is copied from
the lesson's own prose — each of these lessons already told the reader how to
say the word, in the title line or in a bold gloss; the frontmatter simply did
not record it. Telugu is now the first of the four Dravidian tracks
at **zero** headwords carrying no romanization — Tamil still has 21, Malayalam
31, Kannada 14.

Honest accounting of what got worse: the nine new atoms push Telugu's
reinforcement-window misses 823 → 849, because a `TE-SCRIPT-RECOG-*` atom is
revisited exactly once, by the next letter in the chain, and then not again.
That is the same shape the first twenty-four script lessons already have; it is
not a regression this tranche introduced, but it is nine more instances of it.
Forward references are unchanged at 10 — an earlier draft of `TE-S125` cited
మధ్యాహ్నం, whose owning lesson is ten lessons later, and the example was
replaced rather than the pin moved.

`HL-C217` in the shared backlog records why the remaining 22 violations will
not yield to more lessons of this kind: all of them sit at or before chapter 19,
upstream of where the ladder starts.

