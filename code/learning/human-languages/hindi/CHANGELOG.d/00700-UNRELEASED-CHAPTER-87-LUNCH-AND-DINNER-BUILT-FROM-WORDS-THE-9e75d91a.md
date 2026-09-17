## Unreleased — chapter 87: lunch and dinner, built from words the track already owned

Hindi A1 exam coverage **193/282 (68%) → 194/282 (69%)**, closing `HI-A1-LEX-18`
("the meals of the day") against the Hindi A1 inventory.

The inventory's note had already scoped the work exactly: *"`nashta` is now
taught in HI-C73-breakfast, and the doing-word sense of `khana` is taught in
HI-C73-drink. Lunch and dinner are still unnamed, so two thirds of the label's
demand remains."*

`HI-C79-dopahar-raat-ka-khana` names both, and **adds no new vocabulary to do
it**. दोपहर, रात and खाना are all already taught, so lunch and dinner arrive as
compounds the learner can read cold:

| meal | Hindi | built from |
|---|---|---|
| breakfast | नाश्ता | its own word, borrowed whole |
| lunch | दोपहर का खाना | a time + का + खाना |
| dinner | रात का खाना | a time + का + खाना |

The warm-up asks the learner to guess दोपहर का खाना before reading on, because
the guess is available to them — that is the argument for teaching these two as
a pattern rather than as two more entries in a word list. The lesson then notes
the asymmetry that keeps recurring: Hindi borrows a whole word where it has one
(नाश्ता) and builds a transparent phrase where it does not.

The etymology runs one layer deeper than the compound. दोपहर is दो ("two") plus
पहर, a traditional watch of roughly three hours — two watches into the day is
noon, a clock reading that became a time of day. रात is Sanskrit रात्रि worn
down by use, which is why शुभ रात्रि sounds more ceremonious than the everyday
noun.

### One generator fact worth recording

The first pass omitted `"scriptSet": "hindi-main"` from
`core/book-generation.d/targets.d/hindi-0087.json`, and the glyph gate caught
**fifteen characters that would not render** — क, ख and the rest of the
Devanagari in the chapter, emitted into main-font text that cannot draw them.
Every other Hindi target declares the script set; a new one that does not gets a
book full of missing glyphs and no warning until that gate runs. Same shape as
the Punjabi finding in #15260.

### Pins moved

| pin | before → after |
|---|---|
| Hindi lesson count | 359 → 360 |
| `coverage.covered` | 193 → 194 |
| `coverage.unmapped` | 89 → 88 |

Verified: `human-language-data` 145 files / 2083 tests; `language-ladder`
`bash BUILD` 39 files / 442 tests; all six `check:*` gates green.
