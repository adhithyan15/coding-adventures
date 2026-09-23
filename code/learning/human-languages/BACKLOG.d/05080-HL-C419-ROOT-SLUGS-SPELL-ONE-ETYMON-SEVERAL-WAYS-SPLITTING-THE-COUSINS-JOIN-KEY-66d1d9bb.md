## HL-C419-66d1d9bb — Root slugs spell one etymon several ways, splitting the cousins join key

**Status: PARTLY CLOSED (2026-09-22).** The 110 `shape` splits are normalised
and gone; the 82 `bare-vs-tagged` remain and are still OPEN. See SHAPE
NORMALISATION below.
Found while pinning etymology anchors for a Spanish A2
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

### UPDATE — the guard is in, the normalisation is not

`check:root-slug-splits` now exists, with `core/root-tags.json` as the declared
vocabulary this entry said it needed. **Nothing has been normalised.** The
~700-lesson rename still wants its own PR and its own review, for the reason
given above: a wrong merge asserts a shared etymology that is not there.

`core/root-slug-split-baseline.json` pins **192 baseline entries** — 110
*shape* (`stare-latin` against `latin-stare`) and 82 *bare-vs-tagged* (`bonus`
against `bonus-latin`).

**The unit is an ENTRY, not an etymon**, and this entry's own method note says
to say so. Sixteen bare-vs-tagged entries strictly contain a shape entry for
the same lemma — `adiutare|` holds
`[adiutare, adiutare-latin, latin-adiutare]` while `adiutare|latin` holds the
last two — so collapsing the overlaps gives **175 distinct etymon groups over
372 slugs**. Calling it "192 splits" would double-count those sixteen.

The file **may only shrink**: a new split fails the gate, a baseline entry that
no longer splits also fails it, and `--write` refuses to grow the file without
an explicit `--allow-new`. The last of those matters because `--write` is the
command a contributor is told to run, so without it that is also the command
that quietly launders a new split into the accepted set.

The vocabulary is 99 tags. `pie` is declared an **alias** of
`proto-indo-european`, which turns `pie-dwoh` / `proto-indo-european-dwoh` from
an invisible pair into a reported split; it does real work on five lemmas.

Matching is **case-insensitive**, which was not true of the first draft and
should have been. The corpus carries exactly one case-only duplicate —
`SANSKRIT-PA-DRINK` in two Marwadi lessons against `sanskrit-pa-drink` in four
Gujarati, Punjabi and Marathi ones. One etymon, two spellings, six lessons,
four tracks: precisely the defect this guard exists for, and the case-sensitive
draft left the upper-case form opaque and the split invisible. A review found
it; the fix was one `toLowerCase`.

#### What the guard does NOT catch, stated plainly

Two classes are out of scope, and neither is an oversight:

- **One lemma under two different declared tags** — `bursa-greek` against
  `bursa-latin`, `dravidian` against `proto-dravidian`. Folding these would
  *assert* a shared etymology rather than reveal one, which is the failure mode
  `cousins.ts` exists to prevent. Thirty-eight lemmas carry two or more
  canonical tags; `dravidian`/`proto-dravidian` account for eight and
  `frankish`/`germanic` for three, and in those the tags name genuinely
  different languages. **The `bursa` pair is a real defect this guard will not
  report**, because nothing distinguishes it from the legitimate cases without
  reading the two lessons.
- **Prefix families** — `permittere-latin` against `mittere-latin`. The lemmas
  genuinely differ, so no normalisation of shape can see the relationship.

That second one is not hypothetical. Writing `ES-C458-admitir` I claimed
*admitir* was the **third** corpus word off *mittere*, after *meter* and *el
permiso*. It is the fourth: `ES-C405-permitir` has taught *mittere* since
chapter 405, and I missed it because a census keyed on the slug **cannot see
it**. A review caught the wrong sentence before it merged.

So the honest scope: this guard stops the corpus drifting *further* apart in
the ways that are unambiguous, and stops an invented slug from silently
creating a second spelling. It does not find every split that exists, and 175
is a **floor**.

#### One notion of a slug

`liveRootSlugs` calls `cousins.ts`'s own exported `rootSlugs()` rather than
reading the frontmatter a second time. The first draft used
`lesson.realization.roots` — `parse.ts` `arrayify` — which wraps an unbracketed
`roots: a, b` as ONE string where `cousins.ts` splits it on the comma. A
genuine split written without brackets would then have produced two join keys
in the joiner and one opaque slug in the guard, and the gate would have
reported nothing. Every live `roots:` line is bracketed today, so this was
latent rather than live; it is fixed because a guard reading different bytes
from the thing it guards is not a guard.

### SHAPE NORMALISATION — done, 192 entries down to 82

All 110 `shape` entries are resolved. 184 `roots:` uses rewritten across **169
lessons in 18 tracks** — Portuguese 41, French 40, Spanish 23, German 12,
Italian 9, Hindi 9, Persian 6, Russian 5, and ten more tracks in ones and twos.
Every lesson diff is exactly one `roots:` line: 169 removed, 169 added, nothing
else in any lesson file.

**The bare-vs-tagged 82 were deliberately not touched.** `findRootSlugSplits`
calls the shape kind "defects with no argument available" — same lemma, same
declared tag, different arrangement — so merging one asserts nothing the guard
had not already established. A bare slug is the opposite case: it could be a
different word that happens to share a spelling, so each of the 82 needs its
two lessons read. That is a separate pass.

#### The canonical form is PER TAG, and the entry above guessed wrong

This entry proposed normalising "to the majority `lemma-latin` form". Measured
by lesson-slug incidence, there is **no corpus-wide majority to normalise to** —
the global count is nearly even (prefix 1671, suffix 1507) because it sums two
opposite conventions:

| tag | prefix `tag-lemma` | suffix `lemma-tag` |
|---|---:|---:|
| latin | 257 | **1121** |
| greek | 12 | **56** |
| sanskrit | **405** | 54 |
| pie | **289** | 5 |
| dravidian | **144** | 102 |
| germanic | **131** | 54 |

So `lemma-latin` is right for Latin and Greek and **wrong for everything else**.
A single global shape would have rewritten either ~1100 Latin uses or ~800
Indic and Iranian ones, neither of which this work justifies. Each entry was
resolved to the shape its own tag already prefers, which moved 184 uses instead.

Two choices in that rule are worth stating because they are not forced:

- **`pie`, not `proto-indo-european`.** The vocabulary declares them aliases, so
  the guard sees one tag — but the join is exact string equality, so the slug
  still has to pick one spelling. `pie` has 289 uses against 39.
- **`turkish` had no independent evidence.** The tag's entire corpus presence is
  the one split, one slug on each side, so the choice rests only on incidence
  (3 uses against 2). Every other tag had slugs outside its splits to vote.

The case-only duplicate this entry predicted resolved as expected:
`SANSKRIT-PA-DRINK` folded into `sanskrit-pa-drink`. It needed a deliberate
tie-break — ranking by code unit alone picks the upper-case form, because
capitals sort first.

#### The guard refused the fix it asked for

`--check` reported the 17 `bare-vs-tagged` entries that lost a spelling and told
the author to run `generate:root-slug-splits`. That command then **refused**,
counting each shrunken entry among "new split(s)" and pointing at `--allow-new`
— the flag that exists to launder a genuine regression. The only route through
a correct normalisation was the escape hatch built for the incorrect one.

`diffRootSlugSplits` now separates `shrunk` from `added`. An entry whose live
slugs are a **strict subset** of its baseline slugs is a shrink; `--write`
accepts it without a flag and `--check` still fails until the baseline records
it, so the file cannot go stale in either direction. Subset membership is
checked rather than list length, because `[a,b] -> [a,c]` is the same length and
`[a,b,c] -> [a,x]` is shorter, and both smuggle in a spelling the baseline never
accepted. All 17 were strict subsets; none was new.

#### Measured payoff, and the honest limit on it

The joiner gains a lot:

| | before | after |
|---|---:|---:|
| lessons with a cousin panel | 365 | **492** |
| cousin pairs joined | 575 | **857** |

Spanish 154 -> 206, French 63 -> 93, Italian 43 -> 60, Portuguese 51 -> 60,
Latin 44 -> 54, German 7 -> 15. The three concrete panels this entry named as
silently empty now join: `ES-C280-queso` gains `portuguese:o queijo`, four
`dormire` lessons gain `french:dormir`, and every `scribere` lesson but one
gains a cousin it did not have.

Two read `(none)` in the reverse direction — `PT-C24-queijo` and
`FR-C26-dormir` — and that is a **different** limit, not this one falling short:
`cousinsFor` defaults to `ROMANCE_COUSINS` and excludes the lesson's own
language, so a Portuguese lesson whose only cousin is Spanish has nowhere to
show it.

**Nothing renders those panels yet.** `cousinsFor` is exported and tested but
has no production consumer: `book.ts` builds its `cousinweb` from an authored
`etymology` block, not from the joiner. So the 127 are a latent gain in the join
layer, realised whenever a surface consumes it — and not a line of book text
changed here. Worth knowing that the layer HL-C419 exists to protect is one
nothing reads today.
