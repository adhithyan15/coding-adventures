## HL-C419-66d1d9bb — Root slugs spell one etymon several ways, splitting the cousins join key

**Status: OPEN.** Found while pinning etymology anchors for a Spanish A2
vocabulary chapter, and confirmed by census.

`roots:` is the cousins join key, and the join is **exact string equality** on
the slug. `src/cousins.ts` is explicit that this is the one layer whose whole
value is that its etymology can be trusted, and that the join deliberately
avoids `concept_tag` because that would emit false etymology at scale.

**The corpus spells the same etymon several ways.** Three shapes are in use —
bare `lemma`, `lemma-<language>`, and `<language>-lemma` — and nothing enforces
a choice. `facere-latin` and `latin-facere` are one Latin verb. So are
`spondere`, `spondere-latin` and `latin-spondere`.

### What is solid

These reproduce under every counting rule tried, and each is one grep:

| etymon | lessons | spellings in use |
|---|---|---|
| *stare* | 27 | `latin-stare`, `stare`, `stare-latin` |
| *dies* | 22 | `dies`, `dies-latin` |
| *esse* | 19 | `esse`, `esse-latin`, `latin-esse` |
| *bonus* | 19 | `bonus`, `bonus-latin` |

The corpus carries **2389 distinct root slugs**.

Concrete panels that are silently empty as a result:

- `latin-dormire` (French *dormir*) against `dormire-latin` (Spanish *dormir*)
  — the same verb, and the panel shows nothing.
- `latin-scribere` (French, German, Portuguese) against `scribere-latin`
  (Italian, Spanish) — one cousin set split in two.
- `latin-caseus` (Portuguese *queijo*) against `caseus-latin` (Spanish *queso*).

### What is NOT solid, and why that matters more

The corpus-wide total depends on an arbitrary decision, because **the corpus has
no declared vocabulary of language tags**. Whether `X-dravidian` or `X-semitic`
counts as a tagged slug, or whether the `-seek` in a gloss-style slug does,
changes the answer:

| how a language tag is recognised | etymons split | panels lost | lesson-slug pairs | distinct lessons |
|---|---|---|---|---|
| a hardcoded list of 10 common names | 129 | 89 | 617 | 534 |
| the 47-name vocabulary below | **168** | **88** | **815** | **709** |
| any hyphen-separated token | 58 | 26 | 339 | 289 |

**Counting unit matters and is stated on purpose.** *Lesson-slug pairs* counts a
lesson once per affected slug it carries; *distinct lessons* counts the files.
About ninety lessons carry more than one affected slug, which is the whole gap
between the two columns.

The middle row uses this vocabulary, written down here because a number computed
from an undeclared list is not reproducible:

> akkadian, arabic, aramaic, basque, catalan, celtic, chinese, dravidian, dutch,
> egyptian, english, etruscan, frankish, french, gaulish, german, germanic,
> gothic, greek, hebrew, hindi, hungarian, italian, japanese, kannada, korean,
> latin, malayalam, nahuatl, norse, occitan, persian, phoenician, pie,
> portuguese, prakrit, quechua, russian, sanskrit, semitic, spanish, sumerian,
> taino, tamil, telugu, turkish, urdu

That list is already a judgement call. **semitic** is a family, not a language,
and sixteen live slugs use it as a tag; dropping it alone moves the row from
168/815 to 166/799. **pie** is a proto-language. Reasonable people would draw
that line differently, and the number moves when they do.

That instability is not a flaw in the census. **It is the defect seen from the
other side**: you cannot count the splits reliably precisely because the thing
that would let you count them — a defined set of language tags — is what is
missing. Trailing tokens in live slugs include `dravidian` (63) and `sanskrit`
(36), which are languages, alongside `see` (9), `be` (9), `heart` (7),
`disputed` (6) and bare single letters, which are glosses. Nothing
distinguishes them.

### What to do

**A guard is worth more than the fix.** Normalising the ~700 affected lessons to the majority
`lemma-latin` form wants its own PR and its own review, because a wrong merge
would assert a shared etymology that is not there — the one failure mode
`cousins.ts` exists to prevent. But without a guard the corpus drifts back the
first time somebody types a slug from memory.

The guard has to come with a **declared tag vocabulary**, since the check is
undefined without one:

1. Declare the permitted language tags in a shard beside `sound-tags.d/`.
2. Add `check:root-slug-splits`, failing when two live slugs normalise to the
   same `(lemma, tag)` pair, and when a slug's tag is not in the vocabulary.
3. Then normalise, with the gate green as the proof.

**Nothing is wrong today.** No gate fails, no panel prints anything false, and
the etymology in every lesson body is accurate. The loss is entirely of
omission, which is why it survived: a missing cousin panel looks exactly like a
word that has no cousins yet.

### Method note

This entry was drafted four times and the first three were wrong, which is
worth recording because the failure is reusable.

The first census counted 83 pairs by matching only `X-latin` against
`latin-X` — it missed the bare-lemma form entirely. The second reached 129 by
adding the bare form but kept a **hardcoded list of ten language names**, so
every slug tagged `-dravidian`, `-sanskrit`, `-persian` or `-telugu` went
uncounted; a review that re-derived the figures independently caught it. Only
the third pass derived the tag vocabulary from the data and then had to admit
the total is rule-dependent.

A review then re-derived the corrected figures and found two more faults, both
of which this entry now fixes: the 46-name vocabulary was not written down, so
the middle row was not reproducible; and "lessons affected" was lesson-slug
incidence while the prose read as distinct lessons.

The lesson: an aggregate computed with a hand-written list of categories is only
as complete as the list, and the list is invisible in the result. Derive the
categories from the data, write the list down beside the number, and say what
one unit of the count is. Every fault in all four drafts was one of those three
omissions.
