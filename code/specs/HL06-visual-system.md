# HL06 — Visual System: Figures, Script Diagrams, and Illustrations

## Status and purpose

This spec gives the Human Languages books a visual layer: stroke-order diagrams that
teach the reader to *write* a script, data-derived diagrams that make etymology and
sound visible, and illustrations that make the books pleasant to read.

It extends [HL00](./HL00-human-language-curriculum-framework.md)'s "Book Format" and
[HL04](./HL04-shared-spine-and-content-pipeline.md)'s one-source pipeline. Figures are
a **fourth output view** of the canonical content, not a parallel asset library.

The design requirements are:

1. A reader learning a non-Latin script must see how the pen actually moves.
2. Book and app must teach handwriting from **one** source, not two.
3. Every figure that carries a factual claim must be derived from data and verifiable
   in CI.
4. Decorative art may never carry a factual claim.
5. The books stay reproducible: a clean checkout plus CI must rebuild every PDF.

## The gap this closes

| Observation | Value |
|---|---|
| Images in any of the 20 books | **0** |
| Graphics packages loaded in any preamble | **0** |
| Renderer support for `![alt](src)` | none — silently degrades to `!\href{src}{alt}` |
| Authored geometric stroke paths (`DUCTUS`) | **1 letter** (Tamil ம) |
| UI or book consumers of that stroke data | **1 app** (HL-C08), 0 books |
| Scripts with cited prose stroke order | 9 scripts, 190 letters, rendered as a plain `<ol>` |

The most striking of these is the fourth. `strokes.ts` contains a complete, carefully
reasoned pen-path model — strokes as pen-down runs, segments as labelled parts that
must meet head-to-tail, `penPathD()` emitting SVG path data, `penTip()` emitting an
animated pen position — validated against real glyph outlines extracted from the
vendored fonts by `truetype.ts` to a sub-2-font-unit join tolerance. It was imported by
nothing but its own test: the machinery for teaching handwriting properly was built,
tested, and dark.

**HL-C08 has since lit the app half of it** — `language-ladder`'s Browse detail panel
renders the build-up for any letter with an authored pen path, falling back to the
prose `<ol>` for the rest. The books remain dark, and the data remains one letter, so
the row above still reads 1. Widening `DUCTUS` is HL-C09; it is a per-letter
provenance problem, not a rendering one.

## Three classes of figure

Figures are separated by **what makes them trustworthy**, because that determines how
each is produced, verified, and permitted to be used.

### Class A — script figures (generated, font-derived)

Stroke-order build-ups showing how a letter is formed: the finished glyph outline, the
pen path, the segment labels, the lift points.

- **Source of truth:** `DUCTUS` in `strokes.ts` for the pen path; the vendored font
  outline via `truetype.ts` for the glyph shape.
- **Renderer:** `penPathD()` / `penTip()` composed into SVG by `ductusview.ts`, now
  in `@coding-adventures/script-ductus`, which emits an `SvgNode` tree plus a
  serialiser and takes no runtime dependency — the app is a browser bundle, and
  `paint-vm-svg` would be dead weight in it. The **book** pipeline consumes that
  same renderer's output rather than re-composing it; see *As built (HL-C300)*
  below for why it arrives as generated data and not as a package import. Both
  read the same `DUCTUS` and the same font outline, which is what makes them the
  same figure.
- **Verification:** the existing `strokes.test.ts` invariants continue to gate the
  data — every pen point on real ink, every intra-stroke join under tolerance, the
  path covering the whole letter — plus provenance (`citation`, `url`) on every entry.
- **Output:** committed SVG, hash-gated exactly like generated `.tex`; CI converts to
  PDF at build time.

> **Hard rule — the glyph monopoly.** Class A is the *only* pipeline permitted to
> depict a letter, glyph, ligature, conjunct, or handwriting stroke, in the book or
> the app. This generalises the argument `truetype.ts` already makes about hand-drawn
> shapes: a subtly wrong Tamil ண looks completely correct to precisely the audience
> that cannot yet read Tamil, so the error would not merely ship — it would ship *as
> the lesson*. No drawn, traced, or model-generated image may render script. Ever.

Authoring cost is real and should not be understated: `DUCTUS` holds 349 cited
letters across thirteen script ids as of HL-C300, and **Bengali has none at all** —
it is absent from `src/strokes/` even though its font is already vendored, and it
is the track with the most never-taught glyphs, so it is the highest-value gap. The Dravidian syllabaries (1,378 entries across Kannada, Telugu and Malayalam)
do **not** need per-syllable ductus — they compose, so only base consonants and vowel
signs are authored, and the syllable figure is assembled from those parts.

#### As built (HL-C300) — the printed filmstrip

The first Class-A figure kind is `script-filmstrip`: one letter, one frame per
labelled segment, the finished outline behind in pale grey, the strokes already
written muted, and the segment being added in ink with its own authored label as
the caption. Tamil **அ**, Devanagari **आ** and Perso-Arabic **چ** ship as the
proving set; all thirteen authored script ids render, and every one of the 349
cited glyphs resolves an outline in the font its canonical inventory names.

**Divergence from the original plan, and why.** The spec assumed the book would
re-compose `penPathD()`/`penTip()` through `paint-vm-svg`. It does not, because
that would be a second renderer for the same picture — the exact drift this class
exists to prevent — and because it is not reachable anyway:

- `script-ductus` cannot run under plain Node. `scriptdata.ts` reads the canonical
  Japanese/Perso-Arabic/Tamil/Urdu inventories through a Vite virtual module, and
  the plugin that serves it imports `human-language-data`.
- `human-language-data` is a Node CLI, so it cannot import `script-ductus`
  directly; and because `script-ductus` already depends on it, a dependency back
  would close a cycle the repository's build tool rejects outright.

So the two meet on generated data. `script-ductus` writes
`data/ductus/filmstrip-geometry.json` — the frames its own renderer produced, as
escaped SVG fragments in one shared viewBox, with the citation and the font each
was drawn from — and regenerates and byte-checks it inside its own test suite, so
a stroke edited and not regenerated fails there rather than in the book.
`human-language-data` reads that ledger and does the one job it cannot: the
printed layout (a wrapping grid of panels, a heading, the citation). There is
still exactly one renderer; the ledger is its output written down.

`paint-vm-svg` therefore remains the Class-B renderer only. Class A composes SVG
directly, which is what `ductusview.ts` has always done for the app.

**Consequence for the glyph monopoly.** Unchanged and reinforced: the book's
frames are the font's own outline plus a cited pen path, never a drawing. The
book generator re-checks every fragment it embeds — tags, attribute names,
attribute values, nesting balance and text — before it can reach a committed
file, and places each frame in a nested SVG viewport so a frame can only ever
paint inside its own panel. A tampered ledger therefore fails the build, and
could at worst spoil the one frame it belongs to.

**Tiny marks and one-frame strips.** Two layout rules keep the smallest strips
readable. Every frame's box is the letter's ink plus fixed padding, so the panel
zooms in on a dot-sized mark, and a pen line and pen dot sized for whole letters
(26 and 34 font units) covered the mark. When a letter's whole pen path spans
less than 150 font units (`TINY_STROKE_EXTENT`; today the nukta ़, Devanagari ं
and Gujarati ં, at 44 to 60), the ledger draws the pen and the dot at
`extent / 150` of their default size, never below 0.3 of it. Every other letter
keeps the defaults. Separately, a strip of ONE frame (a dab such as ़ or ्, or
Perso-Arabic ا) wraps its heading and citation at the width of a two-frame strip
(310) instead of at its one 150-unit panel, which had made a figure three heading
lines and ten citation lines tall. The panel keeps its size at the left margin.

#### As built (HL-C443) — sequence strips

A writing lesson that names several letters gets one `script-filmstrip` figure
with `letters: [...]`: each letter's own ledger frames, in writing order, as a
labelled group ("Letter 2 of 3 — …"), with short letters sharing a row. Two
headword shapes qualify:

- a **list** of single letters (`வ, க`, `ક — ણ — શ`, `в, р`, `ع ي`), in any
  script, because a letter written by itself is exactly what its isolated ductus
  draws; and
- a **word** in a script whose letters stand apart within a word (chinese,
  japanese, tamil, gujarati, kannada, telugu, malayalam), when every grapheme is
  one base letter with no mark.

The figure prints only when every letter is cited, credits every letter's source
in its footer, and claims nothing about relative size, spacing or joins: each
letter keeps its own panels at its own scale, and the `<desc>` says so.

Words are deliberately **not** composed where the parts would assemble into
something false. Devanagari (and Bengali and Gurmukhi) words share one
continuous headline, while every cited letter draws its own (a Devanagari
word is since drawn another way, as ONE composed entry: see "Devanagari words"
below); Arabic-family
letters join and change shape by position, while the ductus holds isolated
forms; the cited Cyrillic hand is connected cursive. A word with a vowel sign,
virama or length mark is refused in every script that has no written-order
table (below), because some marks are written before the consonant they follow
in Unicode, so drawing in code-point order would draw strokes in the wrong
order. These are the open work for combinations: mark ductus with a
written-order model for scripts other than Tamil and Gujarati, a cited
word-level headline, and positional or joined forms.

#### As built — Tamil vowel signs in written order

Six Tamil vowel signs have a cited ductus of their own (ா ி ீ ெ ே ை, from the
native-writer pen traces in HP Labs India's LipiTk Tamil recognizer), so a sign
lesson prints a one-glyph strip of the sign alone. To compose words with them,
`figure-targets.ts` holds a per-script table of which side of its consonant
each sign is **written** on (`WRITTEN_SIGN_SIDES`). Tamil was the first
script with one (Gujarati follows, below):

| typed | written | source of the side |
|---|---|---|
| C + ெ / ே / ை | sign, then C | Radhakrishnan Modules 6–7 (ெ ே); LipiTk recognizer manual (ை) |
| C + ொ / ோ (NFD: left half + ா) | left half, C, ா | the rows for each half |
| C + ா / ி / ீ | C, then sign | LipiTk recognizer manual |

Each row must match the cited `compositionSource` of its mark record, and a test
enforces it. A sequence strip that contains a sign calls its groups "parts"
("Part 2 of 4"), says in its `<desc>` that they are in written order, and draws
each sign without its consonant. A two-part sign taught by itself (ோ) prints
its two halves. Refused: any sign without a row (ு and ூ, which have no cited
ductus and fuse with their consonant, and ௌ, whose right half ௗ has none), the pairs Unicode fuses into ligatures (டி, டீ, லீ), and every sign
in every other script. The pen lift between parts is the same assumption the
separate-letter word strips already make: the recognizer stores these signs as
distinct symbols, written left to right.

#### As built — the Tamil pulli, after its consonant

The pulli ் has a cited ductus of its own: one short dab inside the bundled
Noto Sans Tamil disc. Its order comes from Abhinaya Rajarajan's *Varai*
(Swift Student Challenge 2026, commit `952294fa`), whose hand-recorded
reference drawings of the 18 consonants with pulli (க் to ன்) all draw the
body as one stroke and then the dot as a second stroke above it, centred at
0.47 to 0.65 of the body's width. An earlier reading of Info-farmer's
*Writing Tamil* animations on Wikimedia Commons agrees. *Varai* has no
licence, so the mark record cites facts only; it is one writer, so the
record says confidence is medium, and the dab's direction is not a claim.

`WRITTEN_SIGN_SIDES.tamil` gains a row for it:

| typed | written | source of the side |
|---|---|---|
| C + ் | C, then the dot | Varai: body first, dot second, in all 18 recordings |

So `வணக்கம்` is drawn வ, ண, க, ், க, ம, ், and a left-hand sign still comes
first (`இல்லை` is இ, ல, ், ை, ல). The font's cluster shows the same parts:
every consonant + pulli glyph in Noto Sans Tamil 2.004 (GSUB `haln`) is a
composite of the unchanged consonant outline and the unchanged pulli disc,
so the strip's dot is the printed dot. Two runs across the pulli are not:
the font prints க்ஷ (`akhn`) and ஸ்ரீ / ஶ்ரீ (`abvs`) as one glyph each. A
pulli ends a grapheme, so the per-grapheme pair check cannot see them;
`FUSED_LETTER_SEQUENCE_SOURCES` lists them with that citation, and a word
containing one is refused.

This unlocked 17 Tamil lessons (target count 48 to 65): the two that teach the
dot by itself and fifteen words. Words with ு or ூ, or with the fused டி,
are still refused.

#### As built — Gujarati vowel signs, written after the consonant

Eleven Gujarati signs have a cited ductus of their own: ા િ ી ુ ૂ ે ૈ ો ૌ, the
anusvara ં and the visarga ઃ. Their order, start, direction and pen lifts come
from KanoAI's hand-made barakhadi centre-line templates (one path per pen-down
run, pinned to commit `9d3e294`). KanoAI's licence is ambiguous (MIT in its
LICENSE file, GNU GPL in its README), so the mark records cite facts only and
no template path was copied; each path is fitted to the bundled Noto Sans
Gujarati outline of the sign by itself. The t30apps records already cited for
આ એ ઐ ઓ ઔ and HP Labs India's LipiTk Devanagari matra prototypes agree on the
shared shapes, and each `variation` says so.

Gujarati gets a `WRITTEN_SIGN_SIDES` table too, and every row is "after":

| typed | written | source of the place |
|---|---|---|
| C + ા / ી / ુ / ૂ / ે / ૈ / ો / ૌ | C, then sign | KanoAI: consonant group before sign group in every row whose consonant keeps its bare outline |
| C + િ | C, then િ (although it sits LEFT of C) | KanoAI: 33 of 34 rows (ઢિ lists the sign first) |
| C + ં / ઃ | C, then the mark | KanoAI: 33 of 33 and 34 of 34 rows |

So a Gujarati word is drawn in typed order, part by part, and its `<desc>` says
that its parts are in typed order (Tamil's says that a left-hand sign comes
first). ો and ૌ have no Unicode decomposition, so each is one sign of two or
three runs (bar first, then the flags). Refused: the virama ્ and the
vocalic-r sign ૃ (no Gujarati source; a Devanagari analogy is not used), two
signs on one consonant (ાં: no source orders them against each other; this
rule leaves Tamil unchanged, since its two-part signs put one half on each
side), and the consonant-sign pairs the bundled Noto Sans Gujarati reshapes,
read from its GSUB table: the ligatures ણુ, રુ, રૂ and the "stem" forms 22
consonants take before ુ and ૂ (`blws`), and જ and ૹ with ા, ી, ો and ૌ, whose
ā bar joins the consonant (`psts`). So જો and બજાર stay undrawn.

Every fused pair, Tamil's and Gujarati's, stands on a cited source:
`FUSED_SIGN_PAIRS` is built only from `FUSED_SIGN_PAIR_SOURCES`, which groups
the pairs under a citation (Unicode 17.0 §12.6.3, Figure 12-21 for Tamil; Noto
Sans Gujarati 2.106's GSUB lookups by feature and number for Gujarati) and an
HTTPS URL. A test holds each citation to its pairs both ways (it names every
pair's letter and sign, and every letter it names has a pair), pins each
source's pair count, and checks that the bundled Gujarati font is still the
cited version.

#### As built — Devanagari signs drawn alone

Eight Devanagari signs have a cited ductus of their own: ु ू े, the anusvara ं,
the nukta ़, the virama ्, ृ and the candrabindu ँ. Their stroke count, start
and direction come from native writers' tablet pen traces in HP Labs India's
LipiTk Devanagari recognizer (classes 50 to 62; the model is MIT, the data
under it research-only, so the mark records cite counts and shares only and
no trace was copied). Each path is fitted to the bundled Noto Sans Devanagari
outline of the sign by itself, at the default tolerances.

| sign | strokes (share of stored prototypes) | start and direction (share) |
|---|---|---|
| ु | 1 (79/83) | top third (67/83), clockwise (65/83), ends left (65/83) |
| ू | 1 (83/83) | first move left (75/83), clockwise (72/83), ends lower right (77/83) |
| े | 1 (162/165) | upper left to lower right (116/165) |
| ं | 1 (156/157) | anticlockwise (112/157); start in the upper half (94/157) |
| ़ | 1 (82/83) | a dab down to the left (82/83) |
| ् | 1 (80/82) | downward (79/82); the slant is the printed one |
| ृ | 1 (81/83) | top third (77/83), anticlockwise (80/83), ends lower right (69/83) |
| ँ | 2 (79/82) | crescent first (76/82), left to right; then the dot above (78/82) |

The writers wrote each sign alone, with no consonant and no headline, so the
traces say nothing about when a sign is written against its consonant or the
shared headline. Devanagari therefore gets **no** `WRITTEN_SIGN_SIDES` row and
no mark record gains a `compositionOrder` (the nukta keeps its earlier
Unicode-cited carrier-first convention). Only a lesson whose headword is the
sign by itself prints a strip (32 lessons across Hindi, Marathi, Sanskrit and
Marwadi); a sign on a consonant (कि) and every word with a sign stay refused.
े's foot meets the headline in a word; drawn alone it floats, and no headline
is added.

Left out:

* ा, ि, ी, ो and ः. Noto Sans Devanagari prints each with a short piece of
  headline (x 0 to 273 font units) that the traces, written without one, never
  draw. A path that follows the traces leaves 9.7% (ा), 4.0% (ि), 3.5% (ी),
  4.5% (ो) and 37% (ः, two dots under a headline piece) of the printed ink
  untraced, over the 2% the honesty check allows, and no override is taken. ी's drawn form (stem, then the arch left
  to right) is also a minority (39 of 91, 43%), and ो's two-stroke form a weak
  one (42 of 83, 51%).
* ै and ौ. ै is two strokes in 76 of 83, but the flags' direction splits: 28
  of 83 (34%) draw both from upper left to lower right. ौ's commonest count
  is three strokes in 35 of 83 (42%).

(ा was later drawn with its headline piece as a last stroke; see "Design —
the ā sign in a Devanagari word". ी, ो and ः were later drawn without it,
under a headline-stub exception; see the next section. ि, ै and ौ are still
left out.)

#### As built — ी, ो and ः, and the headline-stub exception

**The decision.** The piece of headline Noto prints on ी, ो and ः (the
"stub") is not part of the sign as native writers draw it. It is there so
that a typeset word's shirorekhā runs on unbroken across the sign; in
handwriting that line is the word's ONE headline, which native writers draw
last, across the whole word (82% of the recognizer's 2,706 consonant
prototypes). So each sign is drawn by the majority order of the same LipiTk
traces, the stub is left undrawn and stays grey in the strip, and the
coverage check excuses exactly that stub, per glyph.

**The evidence**, re-measured from the stored prototypes of the cited
recognizer (classes 49, 55 and 60). The re-measurement reproduces every count
recorded above (ी stem-first 39 of 91, ो two strokes 42 of 83, ै two strokes
76 of 83, ौ three strokes 35 of 83).

| sign | drawn as | share |
|---|---|---|
| ी | one run: from the hook's lower tip, up, over the top, straight down the stem | arch before stem 49/91; of those, unbroken 44/49; the run itself 44/91, the commonest form. 29/91 add a top stroke |
| ो | the stem down; lift; the flag from its upper-left tip, right and down to the stem (as े) | stem before flag 76/83; stem down then a lift 58/83; of those, flag from its upper-left tip 41/58 (41/83 overall, the commonest form). 40/83 add a top stroke |
| ः | the upper dot, lift, the lower dot; each a loop from its top, anticlockwise (as ं) | two strokes 77/81; upper dot first 76/77; both loops anticlockwise 66/77 |

**The exception, and how narrow it is.** The coverage check ("the strokes
trace the WHOLE letter") counts ink samples more than 100 units from every
path. Without the stub those signs leave 4.6% (ी), 4.2% (ो) and 37% (ः) of
the printed ink untraced, over the 2% allowed. Raising those glyphs' ceilings
would excuse any untraced ink up to the new share. Instead the honesty check
takes, per glyph, an excused rectangle of printed ink (`ExcusedInk` in
script-ductus's `tests/support/stroke-honesty.ts`) and leaves only the
samples inside it out of the count; everything else must still be traced at
2%. `HEADLINE_STUBS` in `tests/strokes/devanagari.test.ts` declares it for
exactly ी, ो and ः, and its cases pin: those three and no other; each
rectangle's corners are on-curve points of the font's outline (x 0 to 273, or
217 for ः; y 551 to 622, across the headline height of 585); without it the
default check fails, with it nothing outside is untraced; and no path runs
along the stub. The mark records' `variation` and `strokeOrderNote` say the
stub is left undrawn and why, and the five lessons whose Writing block is
rewritten say so in a sentence.

ā is not under the exception: its stub is the piece a composed word's
headline is built from, so its second stroke draws it.

**Still left out.** ि: its 75 prototypes split three ways (31 draw the stem
down, lift, then the arch; 22 one run from the arch's right tip down the
stem; 20 one run up the stem and over), so no form wins a majority. ै and ौ,
as above.

**Unlocked:** 11 bare-sign lessons. Hindi HI-S116, HI-W12-ii-matra, HI-S150;
Marathi MR-W01-ii-matra, MR-W01-o-matra, MR-W02-visarga; Marwadi
MW-W04-ii-matra, MW-W05-o-matra; Sanskrit SA-S210, SA-S214, SA-S201. No word
changes: none of the three has a cited place against its consonant or the
headline, so the composer still takes ā only.

#### As built — Devanagari words: the letters' bodies, then one shared headline

A Devanagari word is not its letters' strips side by side. Every cited letter
ends with its own "lift, then draw the shirorekha rightward" stroke, while the
printed word hangs from ONE headline (shirorekhā) that runs across all of its
letters. So a word gets a strip of its own, drawn in this order:

1. each letter in reading order, its body strokes only, exactly as its own
   strip draws them (same paths, same labels), with its own headline stroke
   left out;
2. then one final movement: the shared headline, drawn once, left to right,
   from the start of the first letter's headline to the end of the last
   letter's, along the printed headline of the whole word.

**Why the headline comes last.** That is the native majority, not a rule. In
HP Labs India's LipiTk 4.0 Devanagari recognizer (native writers' tablet pen
traces; the model is MIT, the data under it research-only, so only counts and
shares are cited), the stored prototypes of the 33 consonants draw the
headline as the LAST stroke in 82% of 2,706 and as the first in about 5%.
Every cited Devanagari letter already ends with its headline for the same
reason. Some writers draw the headline first, so the strip prints the
"attested, not standardised" line and its `<desc>` says so. The traces are
single letters: no reachable source records native writers' headline timing
across a whole word (HP Labs India's `hpl-dvng-iso-word` set, which would, is
on a host the authoring environment cannot reach). Carrying the per-letter
majority across the word is this book's reading of that data, and the
figure's source note says so in those words.

**How the headline is found.** No new stroke data is authored. In the
Devanagari ductus every letter's last stroke is one segment labelled exactly
"lift, then draw the shirorekha rightward", a horizontal path drawn left to
right at the headline's height (585 font units; ऋ 586). A letter whose last
stroke is not that (every sign, and any future letter whose headline is part
of a body stroke) cannot join a word. Body strokes keep their authored labels
and are moved right by the advance widths the bundled font gives the letters
before them (`hmtx`), so each body sits where the printed word puts it.

**Fit to the word's ink.** The composed path is checked against the printed
word (each letter's outline at its advance) at the default tolerances the
per-letter ductus meet: at least 97% of every stroke on ink, and under 2% of
the word's ink farther than 100 units from every path. A word that fails is
not drawn. In particular a word whose printed headline is broken — a letter
whose own headline does not reach its left edge (अ आ ओ औ थ ध भ श) anywhere but
first — fails, because one straight headline would cross blank paper there.

**Which words.** A Devanagari writing lesson whose headword is ONE word of two
or more graphemes, each grapheme a single base letter (one code point that is
also one code point in NFD), with a Writing or Script block. Refused:

* any vowel sign, nukta, anusvara, candrabindu, visarga or virama. None has a
  cited written order against its consonant or the shared headline (the signs'
  traces were written alone; see above). ि's place is unresolved; ा has no
  cited ductus at all; the virama would make conjuncts and half forms the font
  fuses. A precomposed nukta letter (क़, U+0958) decomposes and is refused too.
* ई and ऐ: Noto Sans Devanagari 2.006 splits them while shaping (GSUB `abvs`
  multiple-substitution lookup 179: ई → इ + a reph-shaped mark, ऐ → ए + े),
  so the printed word is not those letters' outlines at their advances.
  Shaping every two- and three-letter string of the other cited letters with
  HarfBuzz gives exactly the `cmap` glyphs at their `hmtx` advances, with no
  offset, so for them the composed outline is the printed word.
* a letter without a cited ductus, a phrase of several words (each word has
  its own headline), a list (lists keep the letter-by-letter strip), digits
  and punctuation.

**What the figure says.** It is one ledger entry keyed by the word
(`devanagari:मम`), so it prints like a letter's strip: one panel per
movement, the whole word pale behind every panel, "How it is written — N
strokes · N−1 pen lifts · M movements". The last caption reads "lift, then the
word's shirorekha" (two printed lines; "lift, then one shirorekha over the
word" wrapped to three). Its citation names every letter's source by
position ("letters 1 and 2: …") and the HP Labs India counts for the headline;
its `<desc>` says the frames draw the word, letter bodies first and the shared
headline last. The target carries `composition: "shared-headline"`, and
`script-ductus` builds the entry for exactly those targets
(`composeHeadlineWord` in `src/headline-word.ts`; the font's advances come from
a new `Font.advanceFor`, and the ink measurements the honesty tests used moved
unchanged into `src/ink.ts` so the composer can refuse at build time).

**What it unlocked.** Three lessons: Sanskrit मम (SA-W03-mama-guided-copy,
-delayed-copy, -dictation), Sanskrit filmstrips 48 to 51. That is every
Devanagari writing headword that is one word of bare letters; Hindi, Marathi
and Marwadi have none. Every other Devanagari word lesson carries a sign
(नाम, सा, मम नाम: ा; नमः: ः; नमस्ते, धन्यवाद, अस्ति: the virama; हो: ो),
so the next unlock is a cited written order for ा (and a ductus for it: Noto
prints ा with a headline piece the shared headline would now cover), which
would add the six नाम, सा and मम नाम lessons. The guided-copy lesson's prose,
which told the learner to give each म its own headline stroke and let the two
meet, now points at the strip.

#### Design — the ā sign in a Devanagari word, and phrases word by word

The shared-headline strips drew only मम. Every other Devanagari writing
headword with a Writing or Script block is refused, and two causes cover
most of the near misses: the ā sign (नाम, सा, and the sign taught alone), and
phrases of words separated by a space (मम नाम). This design draws both, and
keeps refusing everything whose order is not cited.

**ā has no cited ductus today.** None of the eight cited signs is ā, so it
needs one before it can join a word. The evidence, all reachable and none of
it copied (the HP Labs data is research-only, so only counts and shares are
cited):

| claim | evidence | strength |
|---|---|---|
| the sign is a stem drawn from the top, then a piece of headline | HP Labs India LipiTk 4.0 Devanagari recognizer, class 47 (ा): 55 of 81 stored native prototypes are one stroke, 54 of those 55 downward; 23 of 81 are the stem and then a short top stroke, drawn left to right in 20 of the 23 | strokes: high. The 1-stroke majority is an artefact of collection: the signs were "collected without the shirorekha" (DvngChar.pdf) |
| on a consonant the stem comes after the consonant's body and before the headline | the cited आ (Saurmandal, *Devanagari आ stroke order.svg*, frames 1–5): the body, then frame 4 the trailing stem top-to-bottom, then frame 5 the shirorekhā. HP Labs' 83 stored prototypes of आ: 52 (63%) are body strokes, then the bar, then the headline; the bar follows the body in 69% and precedes the headline in 65% | **medium**: indirect (the bar of आ, not a consonant + ā syllable) |
| cross-check, another script | KanoAI's Gujarati barakhadi templates draw ા after its consonant in all 32 rows that keep the bare consonant outline | cross-script; Gujarati has no headline |

No reachable source records native writers' consonant + ā syllables: HP Labs
India's `hpl-dvng-iso-word` set, which would, is on a host the authoring
environment cannot reach. So carrying आ's order onto a consonant is this
book's reading, and the citation says "medium confidence" in words.

**The ductus.** `devanagari:ा` is two strokes, one lift: "draw the stem
straight down", then "lift, then draw the shirorekha rightward" over the
piece of headline Noto Sans Devanagari prints on the sign (x 0 to 273). Its
mark record cites class 47 for the strokes (`strokeOrderSource`) and the
cited आ, with the HP Labs and KanoAI counts, for the place
(`compositionOrder`, `compositionSource`). This revises "Left out: ा" in
"Devanagari signs drawn alone": that path followed the 1-stroke traces and
left 9.7% of the printed sign untraced; drawing the headline piece last, as
the 2-stroke writers and the cited आ do, traces it. So the four lessons that
teach ā by itself (HI-S06, SA-S06, MR-W01-aa-matra, MW-W01-aa-matra) print a
strip as well.

**ā in a word.** `composeHeadlineWord` accepts ā only straight after a
consonant (क to ह). Because the sign's last stroke is a headline labelled
like every letter's, it splits like a letter: its stem is drawn right after
its consonant's body, and its piece of headline becomes part of the word's
one headline. Unicode stores ā after its consonant, so reading order is the
written order. Shaping with HarfBuzz each of the 33 cited consonants + ā,
alone and between two of the other cited letters (ई and ऐ, which the font
splits, left out), gives exactly the `cmap` glyphs at their `hmtx` advances
in all 2,805 strings, so the composed outline is the printed word. A vowel
sign after an independent vowel is not a written syllable (after अ the font
even prints a dotted circle, in all 85 such strings), so ā after a vowel
letter, at the start of a word or after another sign is refused. The sign
table (`HEADLINE_WORD_SIGNS`) holds ā
only, and it reads the place citation from the mark record, so the strip's
footer and the data cannot drift apart. The footer names the sign by
position and adds its place:

    letter 1: <न's source>; sign 2: <class 47>; letter 3: <म's source>;
    the place of sign 2, after its consonant's body: <the cited आ>;
    the shared headline, drawn last: <HP Labs India counts>

A word of bare letters prints exactly what it printed before.

**Signs still refused.** Each needs a cited order against its consonant AND
the shared headline, and none has one:

* े and ै: written after the consonant only in Gujarati (KanoAI); in native
  ऐ the flag precedes the headline in 35 of 58 top sequences, a split. ै
  also has no ductus (its flags' directions split).
* ो and ौ: ो has a ductus drawn alone (since "ी, ो and ः, and the
  headline-stub exception") but no cited place; ौ has no majority.
* ी: a ductus drawn alone, but no headline timing at all.
* ि: its side is unresolved (Gujarati KanoAI: after, 33 of 34; Gurmukhi
  copyists: before, 10 of 17).
* ं, ः, ्, ़ and conjuncts: no cited place; the virama
  makes the half forms and conjuncts the font fuses.

**Phrases.** A headword of two or more words separated by single spaces
(U+0020) and nothing else becomes ONE strip in which every word is composed
exactly as a word is now: its letters' bodies (and ā stems), then that
word's own headline. Words come in reading order; the space breaks the
headline, as the printed phrase does. A one-letter word is that letter's
own strip, which already ends with its own headline. If any word fails a
check, the ink fit included, the whole phrase is refused, naming the word
("word 2 (नाम): …"); a phrase is never printed with a word missing.

* The words are separate ledger entries (`devanagari:मम`,
  `devanagari:नाम`), printed as the groups of a sequence strip: "How it is
  written — 2 words, one after another", then "Word 1 of 2 — …" and "Word 2
  of 2 — …". One entry for the whole phrase would scale a phrase 2,500 font
  units wide into a 150-unit panel, about half the size of मम's letters; as
  groups, each word keeps its own scale. The target is `letters: [words]`
  with `composition: "shared-headline"` (a combination that was refused).
* Punctuation is refused, not stripped: a label ("नाम: मीरा", "नाव: ___")
  has a colon no source draws, a sentence ends in । or ".", a list uses
  commas, dashes or middle dots. Stripping it would print a strip of
  something the lesson does not ask the learner to write.
* Length: like a sequence strip, a phrase is a candidate only up to
  `MAX_SEQUENCE_PIECES` (10) pieces, counting its letters and signs, and
  (as built, see below) up to `MAX_PHRASE_WORDS` (3) words. The figure must
  stay at or under the 1,801 units of the tallest printed strip.

**What it should unlock.** Ten lessons: Hindi नाम (HI-A1F01-name-label,
HI-W12-schwa-drop) and ā (HI-S06); Marathi ā (MR-W01-aa-matra); Marwadi सा
(MW-W01-saa) and ā (MW-W01-aa-matra); Sanskrit ā (SA-S06) and मम नाम
(SA-W03-mama-nama-guided-copy, -delayed-copy, -dictation). Every lesson that
gains a strip loses the "copy what you see" disclaimer, as before. Still
refused: हो (ो), नमः (ः), every conjunct word (नमस्ते, धन्यवाद, अस्ति, स्त),
every word with ि, े, ी, ं or a nukta, and every label, sentence and list
that carries punctuation.

**As built.** The design held, with one addition: the piece cap alone was
not enough. Each word of a phrase is a group of its own, so a phrase grows
one band per word, and two-letter words of the cited letters with the most
movements (औइ, औझ, धऋ, औब) wrap to three rows each. Measured with those,
three words print 1,571.14 units tall and four 2,048 (both within ten
pieces), so `MAX_PHRASE_WORDS` is 3; a case in
`script-ductus/tests/filmstrip-ledger.test.ts` renders the three-word worst
case and holds it under 1,801.14. The ten lessons above print strips and
nothing else changed: no earlier figure's bytes moved (a word strip alone
adds its unit to the figure's source hash). The book's alt text reads "How
मम नाम is written, word by word, each with its own headline". The
`<desc>` of a word strip says each word is drawn as its letters' bodies,
then one headline over that word. Prose fixed alongside: HI-S06 and SA-S06
lose the "copy what you see" disclaimer; SA-W03-mama-nama-guided-copy and
HI-A1F01-name-label stop telling the learner to join or draw each letter's
headline and follow the strip (one headline per word, last).

#### As built — Bengali, cited to native writers' pen traces

Bengali joins the derived filmstrips (`DERIVED_FILMSTRIP_SCRIPTS.bengali`) with
nine glyphs: এ ও খ থ ঞ ব র and the signs ঃ ঁ. Their order is not taken from a
teaching animation (Wikimedia Commons cannot be reached from the authoring
environment) but from the native writers' tablet pen traces stored in HP Labs
India's LipiTk 4.0 Bangla recognizer: the modal stroke count, the part order,
the start and the turn, each cited as counts and shares, never as copied
coordinates. The recognizer scales traces to a square, so proportions come from
the font: every path is fitted to Noto Sans Bengali at the default tolerances.

**The headline (mātrā).** The traces do not treat it one way. Across the
recognizer classes whose printed form has a full-width bar, 40% of one-stroke
prototypes draw no bar run, 27% draw it first and 16% last; multi-stroke writers
lift it out as the last stroke (20%) more often than the first (7%). The rule
that follows: a letter is drawn only where one placement wins a majority AND
covers the printed bar. ব and র qualify (bar first, left to right); খ and থ end
with a short move right into the flag beside the stem; এ ও ঞ ঃ ঁ have no bar.
ন (five lessons), ক, ম, ল, য, ত and others are left undrawn with the reason in
`data/scripts/bengali.json`.

**Divergence: the script inventory.** Bengali had no `data/scripts/` file. The
new `bengali.json` holds exactly what the track reads (30 letters, 11 signs, 10
digits) rather than the whole alphabet, because the letter-anchoring ceiling
pins unread inventory letters at zero; `complete` is false. The glyph-gap queue
stays empty only if digits count, so `validate.ts` now counts a script's
`digits` rows as covered (no other track has digit headwords). Bengali words
remain undrawn: there is no composer on this branch, and a Bengali word shares
one headline across its letters.

#### As built — Telugu signs, Bengali ং and Tamil ஸ, from LipiTk's recognizers

The same kind of evidence as Bengali and the Devanagari signs: the stored
prototypes of HP Labs India's LipiTk 4.0 Telugu, Bangla and Tamil recognizers
(`lipi-reco-indic-char` 4.0.0, MIT model), counted for stroke count, start,
order and turn; counts and shares only, no trace copied, every path fitted to
the bundled Noto outline of the glyph by itself at the default tolerances (no
override and no excused ink). A glyph is drawn only where its count, start,
order and turn each win a majority.

**Telugu signs, drawn alone.** The writers wrote each sign by itself, so, as
for Devanagari, Telugu gets no `WRITTEN_SIGN_SIDES` row: a sign is drawn only
in a lesson that teaches it alone, and no Telugu word is composed from it.

| sign | class | drawn as | share |
|---|---|---|---|
| ం | 14 | one ring from its top, anticlockwise | one stroke 103/104; anticlockwise 103/103; from the upper half 101/103 |
| ా | 52 | the bar from its left end, then the loop clockwise to the tip under the bar | one stroke 308/312; start in the left third 302/308; loop clockwise 190/308 (117 the other way) |
| ి | 53 | from the tail's lower-left tip, anticlockwise round the loop, curling in | one stroke 204/205; lower-left start 164/204; anticlockwise 201/204 |
| ీ | 54 | ి's loop, back along its top, then up over the hook to its tip | one stroke 180/213 (the 32 two-stroke writers all draw the loop first); lower-left start 157/180; highest point in the second half 150/180 |
| ు | 55 | from the lower-left tip, down round the bowl, up to the upper tip | one stroke 405/416; start on the left 403/405; end in the top third 370/405 |
| ూ | 56 | ు, then the bar and the loop on the right, clockwise | one stroke 482/517; start on the left 474/482; rightmost point in the last third 467/482; loop clockwise 273/482 (200 the other way) |
| ె | 57 | from the lower tip, round the right, back left along the top bar | one stroke 210/210; all four of start low, anticlockwise, end top and end left 175/210 |
| ే | 58 | ె; lift; the hook from its foot on the bar, clockwise to its tip | two strokes 163/206; ె part first 159/163; the whole form 153/163 |
| ొ | 60 | from the foot of the left bowl, up and over, the dip, the second arch, the loop clockwise | one stroke 294/303; start in the left third 286/294; loop clockwise 278/294 |
| ో | 61 | ొ, then up out of the loop into the hook and down to its tip | one stroke 301/320; start in the left third 299/301; loop clockwise 199/301 (27 the other way); end in the top third 222/301 |
| ్ | 62 | from the lower bar, clockwise round both bowls (out along the middle prong and back), out along the top bar | one stroke 101/104; start low, clockwise and end top right 96/101 |

The loops of ా and ూ are the weakest claims (62% and 57%); the font's tip
tucked under the bar on the loop's left is where the clockwise form ends, and
both records say the turn splits.

**Bengali ং** (class 46): the ring counterclockwise from its top, a lift, then
the tail from its upper-left end down to the right. Two strokes 183/189; ring
counterclockwise 138/183; tail down to the right 166/183; ring first 103/183,
the weakest claim; ring first, counterclockwise, then the tail is the
commonest form (71/183, ahead of the tail first and then the ring, 54).

**Tamil ஸ** (class 30): one stroke 150/153, from the tip inside the small left
loop, clockwise round it (147/150), over the big arch, down the stem and back
up it, over the second arch, round the bowl and up the tail to the top right
(142/150 end top right; 87/150 come down to the foot three times). Its
inventory row is new (`tamil.d/letters/0270-U-BB8.json`, the thirtieth
letter).

**Left out.** ై: the recognizer's ai class (59, 105 prototypes, all one
stroke) stores only the length mark below (ౖ), never the e hook above it, so
the order of the two parts is unattested; this is an identification by eye
from rendered prototypes. ృ and ౌ have no class in the recognizer, and the
Telugu digits none either.

**Unlocked:** 13 lessons. Telugu TE-S02, TE-S04, TE-S05, TE-S07, TE-S08,
TE-S115, TE-S119, TE-S120, TE-S134, TE-S153, TE-S154 (45 -> 56 strips);
Bengali BN-W41-anusvar (9 -> 10); Tamil TA-S129-letter-sa (65 -> 66).

#### As built — Punjabi (Gurmukhi), cited to a tracing lesson

Punjabi joins the derived filmstrips (`DERIVED_FILMSTRIP_SCRIPTS.punjabi =
"gurmukhi"`) with 27 letters: the vowel bearer ਅ and the consonants ਸ ਹ ਕ ਖ ਗ ਘ
ਚ ਛ ਜ ਟ ਠ ਡ ਣ ਤ ਥ ਦ ਨ ਪ ਫ ਬ ਭ ਮ ਰ ਲ ਵ ੜ. These are every letter a Punjabi
writing lesson prints alone or in a list of letters, so 29 lessons now print a
strip. The order comes from the Alphabet Tracing lesson of GNPS's Gurmukhi Sikho
app (`codemanxdev/gnps_learning_hub`, Apache-2.0, `lesson_tracing.dart` at a
pinned commit, cited line by line). It is cited the way the Telugu owner cites a
tracing app: one attested teaching order. Only facts are taken from it (order,
start, direction, lifts); no checkpoint coordinate is copied, and every path is
fitted to Noto Sans Gurmukhi at the default tolerances. Omniglot's copyist
counts and shares go into each `variation` as corroboration of the body strokes.

**The headline.** The source draws it first, left to right, and so do the paths.
That is the source's teaching order, and each record says so. Fluent writers are
often described as adding the headline last, as native Devanagari writers do in
HP Labs India's data. Omniglot is not cited for the headline, because its
copyists draw the Devanagari headline first too, where natives draw it last.
Noto prints a split headline in ਅ ਖ ਘ ਪ ਮ. The source draws no separate bar for
these letters, so the outline and the source agree: the bar's left part opens
the first stroke, and its right part, drawn right to left, opens the stem stroke.

**Divergence: lifts.** ਛ, ਨ and ਬ lift once less than the source. Omniglot's
copyists most often use one stroke fewer, and copyist counts are an upper bound
on native lifts (as for Kannada ಚ and ಯ). Each keeps the source's order and
joins one restart on the ink. **Divergence: printed loops.** Where Noto fills a
loop as a solid knob or tail (ਅ ਸ ਚ ਜ ਡ ਤ ਦ ਮ ੜ, and ਘ's middle upright), the
path loops inside it, turning the way the source's stroke turns.

**Left out.** The vowel signs (laga matra), bindi, tippi, addak, halant and the
dot below have no source, so they stay undrawn, and so does every list that
holds one. Gurmukhi words share one headline, so they are refused, exactly as
Devanagari words are. ਝ and ਧ, which the source covers, appear only in lists
with an uncited sign and are left for a later batch. The new
`data/scripts/gurmukhi.json` holds exactly what the track reads (33 letters, 14
signs, 4 digits), with `complete` false, the same way `bengali.json` does.

#### As built — Malayalam consonants, cited to a teaching tool's formation arrows

Seventeen Malayalam base consonants (ന മ സ ര ത ഷ പ വ ണ ട ദ ഹ ഗ റ ല ശ ബ) gain a
ductus, which brings the Malayalam filmstrips from 14 to 35: their letter
lessons, plus the four chapter-1 lessons whose headword is നമ (Malayalam is in
`SEPARATE_LETTER_SCRIPTS`, so a word of cited letters is drawn letter by
letter). The order comes from SPACE Kerala's Thooval, a Malayalam alphabet
teaching tool whose image for each letter marks the start, every turn where the
pen runs back along its own ink, and the end. Thooval is GPL-3.0, so it is a
source of facts only: no image, path or template point enters the repository.

**Pen lifts need a second kind of source.** Thooval has the learner keep the
pen down to the end of every letter, so it cannot show a lift even where a
writer would make one. The zero-lift claim therefore rests on two recordings
that could have shown one: Santhosh Thottingal's *hand* curves (MIT), which
store several strokes where a glyph has them, and the *grahyam* samples, cited
as counts only because the dataset has no licence. Every letter here is one
stroke in *hand*, and no grahyam sample jumps between points the way a lift
would (its files keep no pen-up marker, so a jump is the only sign). Letters whose sources disagree on the start (ക, യ) or whose source is
uncertain (ഏ) or missing (ം) are not drawn.

**Divergence: lesson prose (since resolved).** These lessons first kept their
"copy what you see" writing blocks, with the filmstrip added beside them. A
later prose fix pointed every strip lesson's writing block at its strip, and a
figure-targets guard now fails any strip lesson that still disclaims its stroke
order.

#### As built — Malayalam letters and signs, cited to a textbook's numbered movements

Twenty-two more Malayalam glyphs gain a ductus, which brings the Malayalam
filmstrips from 35 to 58: the consonants ക യ ഖ ങ ച ഛ ഞ ഥ ധ ഭ ഫ ള, the vowel ഏ,
the anusvara ം and the vowel signs ാ ി ീ ു ൂ ൃ െ േ, each drawn by itself. The
order comes from Rodney F. Moag's *Malayalam: A University Course and Reference
Grammar* (UT Austin South Asia Institute / COERLL, April 2018), whose Tables
II-IV number and arrow every movement of every letter and sign in the hand of a
native writer. The book is CC BY-NC-SA 4.0, so it is a source of facts only
(order, start, direction, end); each record links the page's scan in a digital
edition at a pinned commit, and nothing of the drawings is copied. Moag settles
ക and യ, held in the earlier batch while the other sources disagreed on their
start, and confirms that Thooval's `EE` image is ഏ.

**Movements are not lifts.** Moag numbers movements; the digital edition notes
that consecutive numbers can belong to one stroke. Each glyph is therefore one
run whose segments are Moag's numbers, and the zero-lift claim rests, as
before, on *hand* (MIT) and *grahyam* (counts only). Where the sources disagree
the record says so: യ's start, ു's end, and ം's direction (clockwise in Moag,
read from a small arrowhead, so medium-low confidence; anticlockwise in an
unlicensed tracing app and in the Gujarati and Devanagari analogues).

**One written-order row.** `WRITTEN_SIGN_SIDES.malayalam` has a single row, ം
"after", cited on the anusvara's mark record: in Moag's അം the ring is movement
9, after the eight movements of അ. Moag draws each vowel sign beside a dash
standing for the consonant but never numbers the consonant against the sign, so
no vowel sign gets a row: a sign is drawn only in a lesson that teaches it
alone, and a word with a vowel sign stays refused.

**Not drawn:** ജ (Moag's arrows do not place the short stem between its humps),
ഠ (Moag runs the ring clockwise, Thooval and grahyam anticlockwise), ൈ (two
pieces of ink need a lift no source records), ോ and ൊ (the consonant between
the sign's two parts is not numbered) and ് (Table V shows it without
movements).

**Divergence: lesson prose, now resolved.** The 21 newly stripped lessons that
still carried the "copy what you see / this book does not yet tell you where
to start" block now point at the numbered strip, in the wording the earlier
strip lessons use, so the figure-targets guard against a strip lesson
disclaiming its stroke order holds.

#### As built — Kannada digits and anusvara from a tracing app; Malayalam ജ and ൈ

Twelve more writing lessons print a strip: the Kannada digit lessons ೧ to ೯
and the anusvara ಂ (Kannada 47 -> 57), and the Malayalam letter ജ and vowel
sign ൈ (Malayalam 58 -> 60).

**Kannada: one tracing app, read as one source.** The Bangalore literacy NGO
Sutara Learning Foundation publishes its Chimple course content on GitHub in
two repositories: `chimple/chimple-zips` (lesson bundles whose trace pictures
hold hidden, ordered centre-line paths, which the lesson player walks in
order; no licence) and `chimple/bahama` (recorded traces; MPL-2.0). They come
from one organisation, so they count as ONE source, and every record says so.
Only facts are taken (stroke count, order, start, direction); no path or
artwork is copied, and every path is fitted to Noto Sans Kannada at the
default tolerances, with no override.

- **Digits ೧-೯** cite the digit lesson `LIDO_kn2_0318` (its sibling
  `LIDO_kn2_0319` repeats the same paths). Each digit is one path, and
  Chimple keeps one path per pen-down run elsewhere (the anusvara is a
  separate last path on every consonant), so each digit is one stroke. Each
  loop turns the way Chimple's does, and a test holds every caption that
  says "clockwise" or "anticlockwise" to the turning of its points. The
  source is designer-authored, single and unlicensed, and no second source
  for Kannada digits was found, so confidence is medium. **೦ is not drawn:**
  Chimple's ೧೦ re-uses the ೧ picture and never draws a zero.
- **ಂ** is one anticlockwise ring, drawn last, in all 34 of Chimple's
  consonant + anusvara pictures; it starts at the left in 20 of them, so the
  path does. The recorded ಅಂ agrees (it starts at the top). The ring gets no
  written-order row: no Kannada writing lesson has a word with ಂ in it.
- **ಃ was drawn in the wrong order and is fixed.** Its first ductus labelled
  the first loop "the upper dot" but drew it round the lower one (font units
  point up, and the path used the smaller y). Its cited animation and all 35
  of Chimple's consonant + visarga pictures draw the upper dot first, both
  loops anticlockwise; the paths now do, from near 8 o'clock, and a test
  holds the first loop above the second.
- `validate` now holds a digit row that names its strokes to the same rule as
  a letter (a lift count only with its source), and script-ductus resolves a
  cited digit's font as it does a letter's.

**Malayalam: two of the glyphs Moag's batch left out.**

- **ജ** cites Moag's six numbered movements (p. xxvi). Moag's arrow 2 stops
  above the short stem and arrow 3 starts at its foot; with no lift between
  them (*hand*: one stroke; *grahyam*: 45 of 45 unique samples in one run),
  the pen can only reach the foot down the stem, so movement 2 ends with that
  descent and movement 3 climbs it again. That join is read, not drawn, and
  the record says its confidence is medium.
- **ൈ** cites Moag's four movements (p. xxii): two coils of െ, each loop then
  arch. The two coils are separate pieces of ink, so the pen lifts once
  between them, and each coil is one run, as recorded writers write െ. No
  recording of ൈ itself was found, so the record says the lift is reasoned
  and confidence is medium. Noto composes the standalone sign from two copies
  of െ, so each run is the cited െ path. Like every Malayalam vowel sign it
  has no written-order row: it is drawn only alone.

**Still not drawn.** ഠ: this round re-read Moag's arrow as clockwise, but
Thooval and grahyam still run the ring anticlockwise, and no new source
breaks the tie. ൊ and ോ: Noto's standalone glyphs put a placeholder dot where
the consonant goes (`period.mlym` between the two parts), which no source
says to write; tracing the sign alone leaves about 5% of the printed ink
untraced, above the default limit, and drawing the dot would teach a mark
nobody writes. Moag also never places the consonant between the parts. ്:
Moag gives its position only, with no movements. Kannada ೦ and the Kannada
vowel signs: no source.

#### As built — Malayalam ്, ഠ, ൊ, ോ and digits from a handwriting recording; Kannada ಞ; Chinese 尔

Twelve more writing lessons print a strip: Malayalam 59 -> 69 (ML-S02 ്,
ML-S119 ോ, ML-S143 ൊ, ML-S147 ഠ, the സ ് ക list in ML-W01, and the ML-W07 digit lists ൧-൩, ൪-൫, ൬-൮ and
the two ൧-൫ copies), Kannada 56 -> 57 (KA-S133 ಞ) and Chinese 72 -> 73
(ZH-W01-er 尔).

**Malayalam: a recording.** Jayasree (`sachn1/jayasree` at commit `e0c9d57`,
Sachin Nandakumar) animates Malayalam handwriting from about 300 centre lines
that one recorder traced over the Manjari typeface, one gesture per pen-down
stroke. Unlike Moag's numbered arrows or Thooval's formation images, a
recording shows lifts directly, so the stroke count needs no second source.
The stroke data is CC BY 4.0 (`LICENSE-DATA`), which permits adaptation with
credit: each record names "Jayasree" by Sachin Nandakumar, links the data file
at the pinned commit and the licence, and says the path is an adaptation, and
that citation is printed under every strip. Only the facts are taken (count,
start, order, direction, end). Manjari is rounder and wider than Noto, and its
൪ ends in a straight rise where Noto curls, so every path is fitted to the
bundled Noto Sans Malayalam outline at the default tolerances, with no
override (on ink 1.0000 on every stroke, joins closed, nothing untraced), and
the captions are the package's own. One recorder, so confidence is medium.

- **്** is one stroke from the left tip, round the bottom of the cup, to the
  right tip. Moag gave its position only; the Unicode composition source
  still owns where it goes against its carrier, and no written-order row is
  added, so words with ് stay refused.
- **ഠ** is one anticlockwise ring from the top. Jayasree breaks the earlier
  tie: Thooval and grahyam (as read before) run anticlockwise too, and Moag's
  arrow alone runs clockwise. The record names the disagreement.
- **൧-൯** are one stroke each; stems the stroke goes down and back up (൩, ൬,
  ൮, ൯) are retraced, as the recording retraces them.

- **ൊ and ോ** are two recorded strokes: the left sign (െ or േ), a lift, then
  ാ clockwise. Noto builds each standalone glyph from exactly the cited
  left-sign outline, a placeholder dot, and the ാ outline shifted 923 (ൊ) or
  788 (ോ) units right, so each run is the cited path of its part.

**The consonant-placeholder exception.** The dot between the parts (Noto's
`period.mlym` component) marks where the consonant would sit. It is not ink a
writer draws, and skipping it leaves 4.9% (ൊ) and 5.2% (ോ) of the printed ink
untraced, over the default 2%. Drawing it would teach a mark nobody writes.
So the coverage check skips exactly that contour for exactly these two
glyphs: `NOTO_PLACEHOLDER_CONTOURS` (script-ductus test support), keyed per
glyph by ductus key, names the contour's index and its pinned bounds, and
refuses to skip a contour with other bounds. The 2% limit is not loosened,
and the on-ink and join checks still see the whole glyph. Tests pin the table
to exactly ൊ and ോ, prove the kept contours are exactly the cited parts and
the skipped one the dot standing alone between them (the same contour in both
signs), and show, as a control, that without the exception the dot alone
breaks the limit. The strip still prints the dot in grey, and both lessons
tell the learner it only marks where the consonant goes. Neither record
claims a written order against a consonant, so words with ോ stay refused.

**Still not drawn.** ൦: recorded, but no lesson draws it. ൰: Jayasree has no
൰, so the ൯-൰ and ൬-൰ lists stay undrawn.

**Kannada ಞ** cites Chimple's consonant lesson `LIDO_kn2_0304`, whose
`data.json` pairs the trace image with the question ಞ: two hidden paths, so
two strokes (the body, its loop clockwise, then the hook at the top right).
Chimple's recorded `bahama` trace agrees; the two count as one source, facts
only, medium confidence.

**Chinese 尔** cites Hanzi Writer Data's `尔.json` at the commit the other
characters use: the five strokes that already close 你, in the same order,
directions and lifts, fitted to the standalone Noto Sans SC 尔. ZH-W01-er's
cue "the middle with its hook last" contradicted that order (the two dots come
last) and now reads "the middle and its hook, then the two dots".

#### Design — the Latin script's first print letters

The six Latin-script tracks (Spanish, French, German, Italian, Portuguese,
Latin) have about 45 writing lessons, and none prints a strip: there is no
Latin inventory and no Latin ductus owner. This design adds both, for the
letters a source covers, and refuses the rest by name.

**Sources.** Letter order comes from the Grundschrift-App
(`Medien-Treibhaus/grundschrift-app-source`, pinned commit `f6dbd807`), built
in a research project of the Laborschule at Bielefeld University with the
Grundschulverband, on the Grundschulverband's Grundschrift model, with a
teacher advisory team. Each letter is an ordered list of paths, one per
pen-down stroke, and the app makes the child follow them in order. The
repository has no licence, so it is a source of facts only (stroke count,
order, start, direction); no point is copied, and every path is fitted to the
bundled outline. Native-writer corroboration comes from UJIpenchars2 (Prat et
al., UCI Machine Learning Repository dataset 177, CC BY 4.0): 60 adult Spanish
writers, two samples of each character, cited as counts and shares only. UJI
is also the only source for the marks: the tilde of ñ is written after the n
(108 of 120) and left to right (104 of those 108), the bar of ¡ comes before
its dot (114 of 119) and runs downward (109 of 119), and the hook of ¿ comes
before its dot (108 of 118), starting at the top, heading down and turning
anticlockwise to the lower right.

**Font.** No bundled font is a Latin font, but Noto Sans Devanagari 2.006
(`_fonts/NotoSansDevanagari-Static.ttf`) carries the Noto Sans Latin letters,
including ñ, ß, ¿ and ¡, which the Cyrillic subset lacks. The Latin inventory
names that file, and the paths are fitted to its Latin outlines at the default
tolerances, with no override.

**Precomposed letters.** ñ is one code point (NFC) and a base letter, so it is
its own ductus entry (`latin:ñ`): the n, then a lift, then the tilde, fitted
to the precomposed glyph. Latin gets no `WRITTEN_SIGN_SIDES` table, so a
headword typed with a combining tilde (n + U+0303) is refused rather than
drawn as n and a loose mark. Every Latin-track headword is NFC today.

**Words.** Print letters stand apart (no headline, no joins, no change of
shape), so `latin` joins `SEPARATE_LETTER_SCRIPTS` and a word of cited letters
is drawn letter by letter. The strip shows print letters, not the joined hand
that Grundschrift itself goes on to teach. Punctuation inside a word (¿cómo?)
is not a base letter, so such a word is refused; a list of single marks
("¿ ¡") is drawn.

**Refused, with reasons.**

* a and every word with it. Noto Sans prints a two-storey a, and every source
  draws the one-storey a (a bowl, then the stem). No bundled font has a
  one-storey a, so no path can both follow a source and lie on the printed
  letter. This one letter holds back most Latin word lessons (hola, salut,
  ciao, olá, Hallo, quia, buenos días, ayer, mañana, parce que) and the
  accent list "á é í ó ú".
* The grave (à è ù), circumflex, cedilla (ç), æ and œ: no source gives their
  order or direction. ë ï ÿ, ä and ö only by analogy with ü, so not drawn.
* Acute accents and ü are sourced (UJI) but unlock nothing without a, so they
  wait for a later batch. The acute's direction would be recorded as split
  (up-right in about 62% of samples, down-left in about 30%).

**Scope.** 18 glyphs: G b c e g h i l n o r s u w ß ñ ¿ ¡. Only Spanish and
German join `DERIVED_FILMSTRIP_SCRIPTS`, the tracks with a lesson these
letters complete. Four lessons gain a strip: ES-W02-enye (ñ), ES-W03-inverted
(¿ ¡), GE-W01-eszett (ß) and GE-W04-vier-zeilen (weil). GE-W03-capitalization
(Großschreibung) is cited letter for letter but stays undrawn: see the length
cap below. French, Italian, Portuguese and Latin stay
switched off: none of their writing headwords is fully cited.

**Inventory.** `data/scripts/latin.json` lists every Latin character a
Latin-track headword uses, NFD-decomposed: 26 small letters, the 24 capitals
in use (no X or Y), ß, œ, the cited ñ, ¿ and ¡, and seven combining marks
(grave, acute, circumflex, tilde, macron, diaeresis, cedilla), with `complete`
false. Only the 18 cited rows carry an order. Letter anchoring and script
closure already skip Latin tracks, so their ceilings do not move.

**As built.** As designed: owner `strokes/latin.ts` (keys `latin:<glyph>`,
appended last), 18 glyphs, every stroke 1.000 on ink at the default
tolerances and nothing untraced. Simple letters are one movement (c l o s w),
most others two (the stem, then "back up" and the rest), so a word strip stays
short; ñ is three movements over two strokes. Every record's `variation`
carries the UJI counts at the source's count (for example n: 119 of 120 one
stroke, 74 of those from the top left), and says where adults differ from the
school model: most start l at the bottom, as a joined hand does, and UJI has
no ß, so ß rests on the school model alone. The sequence renderer draws each
letter at its own scale, so narrow letters (i, l, ¡) print tall.

**Divergence: a length cap.** The fourteen-letter Großschreibung strip came
out about 2,380 units tall, against about 1,800 for the longest earlier strip
(a nine-piece Gujarati list), and the block-figure macro would shrink it to an
illegible size. `MAX_SEQUENCE_PIECES` (10) now keeps any sequence longer than
ten written pieces out of the candidates, so such a lesson prints no strip
rather than an unreadable one. It removes only Großschreibung; every earlier
strip has nine pieces or fewer.

**Divergence: the inventory.** It also lists the ordinal indicators ª and º
(Portuguese "1.º / 1.ª" is a headword): they are Latin-script letters the
design's census missed, because their Unicode names do not say "Latin". And
the validator's glyph closure turned out not to measure Latin at all (its
script matchers leave Latin out, so a Latin headword can never report a gap),
so the inventory's exactness is held by its own evidence module instead: every
Latin character of every Latin-track headword is listed, and every listed one
is read.

#### Design — a one-storey a: the Latin strips move to a literacy font

The first Latin batch refused a, and with it about twenty lessons, because
the only bundled Latin outline (the Noto Sans letters inside
NotoSansDevanagari-Static.ttf) prints a two-storey a and every source teaches
the one-storey a. A strip traces a school model's path over the printed
letter, so the printed letter must have the shape the model teaches. This
design changes the outline the Latin strips are drawn on, not the books.

**Font.** Andika 7.000 (SIL Global, `github.com/silnrsi/font-andika`, release
tag `v7.000`, SIL Open Font License 1.1) is a typeface made for literacy
work: its DEFAULT glyph for a is the one-storey a (`a.SngStory`) and for g
the single-storey g (`g.SngBowl`), so nothing has to be remapped. The
Grundschrift g is single-storey too, and Noto's g already was. A subset is
vendored as `_fonts/LatinPrint-Subset.ttf`:

* fetched by `_fonts/subset-latin.sh`, which checks the SHA-256 of the
  release zip and of `Andika-Regular.ttf` inside it, and asserts that the
  source's cmap maps a and g to those two glyphs;
* cut to printable Basic Latin, ß ñ Ñ ¿ ¡, the acute, diaeresis and tilde
  vowels, and every non-ASCII Latin character of every Latin-track headword
  and inventory row (about 140 characters, tens of kilobytes);
* composites flattened to plain contours (script-ductus's reader refuses
  scaled components) and the em scaled from 2048 to 1000 units, the em of
  every other vendored font, so the ductus tests' distances mean the same for
  Latin as for every other script;
* RENAMED "Latin Print Subset": the licence reserves the names Andika and SIL,
  and a subset is a Modified Version. The copyright and licence name records
  are kept verbatim, and Andika's own licence file is vendored beside it as
  `_fonts/OFL-Andika.txt`.

The `_fonts` README records the source URL, tag, both digests and the
command. The books' body text is unchanged (Latin Modern Roman, whose a is
two-storey): the strip shows the handwriting model's letter, not the book's
type, and every Latin record's source note says so.

**Refit.** The Latin inventory names the new file. All 18 existing paths are
refitted to its outlines (same order, starts, directions and lifts), and must
again be fully on ink at the default tolerances with nothing untraced and no
override. Andika's letters are heavier and its x-height lower than Noto's, so
the numbers pinned in the Latin ductus tests (where a stroke starts, where a
bar sits) move with the outline; what they assert does not.

**New glyphs (13).**

* a, d, q: one stroke (Grundschrift): the bowl anticlockwise from the top
  right, then back up to the top of the stem and down it (d to the top of its
  ascender; q down to the foot of its descender). UJI: a 118 of 120 one
  stroke; d 101; q mostly TWO (89), a short crossing stroke at the foot that
  print does not have, recorded as variation.
* p: one stroke: the stem down to the descender, back up it, the bowl
  clockwise. UJI 75 one stroke, 45 two.
* t: two strokes: the stem down and round to the right, then the crossbar
  left to right. UJI 113 two strokes, crossbar second in 111 of them.
* y: two strokes: the short line down to the right, then the long line from
  the top right down to the tail. UJI writes y mostly in ONE stroke (91 of
  120), recorded as variation; the school model's two strokes are drawn.
* H: three strokes: left stem, crossbar left to right, right stem. UJI splits
  (62 of 120 three strokes; crossbar second 37, third 25), recorded.
* á é í ó ú: precomposed, as ñ is: the base letter's cited path, a lift,
  then the acute. UJI puts the accent after the letter (á 120 of 120,
  é 120, ó 114, ú 115, í 66 of the 69 where it is found apart from the stem);
  its direction is split, up to the right in about 62% (á 74, é 75, ó 74,
  ú 70 of 120; í 44 of 69) and down to the left in about 30%. The majority
  is drawn and the split is recorded. On í the acute replaces the dot, as the
  printed glyph shows.
* ü: the u, a lift, the left dot, a lift, the right dot. UJI: dots last in
  119 of 120, left dot first in 116 of 118.

**Still refused.** The grave (à è ù), circumflex, cedilla, macron (Latin
salvē), æ and œ: no source. ä ö ë ï ÿ: analogy with ü only. So FR-W01-accents
(é è ê), FR-W03-trema (ï ë ü), GE-W02-umlauts (ä ö ü), FR-W02-cedille,
FR-C10-oe and the salvē lessons stay undrawn.

**Tracks.** French, Italian, Portuguese and Latin join
`DERIVED_FILMSTRIP_SCRIPTS` ("latin"), within `MAX_SEQUENCE_PIECES` (buenos
días is exactly ten pieces). Expected new strips (22): FR-W01-salut-observe,
-guided-copy, -delayed-copy, -dictation; FR-W04-quatre-lignes (parce que);
GE-W01-hallo-guided-copy, -delayed-copy, -dictation; IT-W01-ciao-guided-copy,
-delayed-copy, -dictation; LA-W04-quattuor-versus (quia);
PT-W01-ola-guided-copy, -delayed-copy, -dictation; ES-W00-hola-observe,
-guided-copy, -delayed-copy, -dictation; ES-W01-acento (á é í ó ú);
ES-W01-frase-propia (buenos días); ES-W02-cuatro-lineas-ayer. The four
earlier Latin strips are redrawn on the new outline. None of the 22 lessons
disclaims its stroke order, and ES-W01-acento's own prose already describes
the acute as one up-stroke rising left to right.

**As built.** As designed. `LatinPrint-Subset.ttf` is 26,292 bytes (141
characters, 152 glyphs, SHA-256 `88437a25…53cfab`), and `subset-latin.sh`
regenerates it byte for byte. All 31 Latin glyphs are 1.000 on ink with
nothing untraced at the default tolerances. Each precomposed letter's first
stroke is exactly its base letter's (í shares i's stem), so a word strip
draws the same a in hola and in á. The Latin ledger owner holds the 28 glyphs
a lesson draws (G, g and ü wait for a lesson). Exactly the 22 expected lessons
gain a strip: French 5, Italian 3, Portuguese 3, Latin 1, Spanish 2 -> 9,
German 2 -> 5. The tallest new strip is Hallo at about 1,510 units (its H
has three strokes), under the nine-piece Gujarati list's 1,800; buenos días,
at exactly ten pieces, is about 1,420. Narration, modality and lesson prose
are unchanged.

#### Design — a strip in modelled practice, when a lesson has no Writing or Script block

**The gap.** A derived strip lands in a lesson's first `## Writing` block,
else its first `## Script` block, and a writing lesson with neither was never
a candidate. 64 writing lessons on switched-on tracks have neither: their
blocks are Warm-up / Guided Practice / Wrap-up Recall (chinese 15, gujarati
18, hindi 12, marathi 3, punjabi 15, spanish 1). 30 of them have a headword
whose every piece is cited, among them the Chinese copy pair for each
character (ZH-W16-han-guided, ZH-W16-han-delayed: 汉 语 国 文 看 书 吗) and the
Gujarati place words (ઘર, મંદિર, હાથ …). Their -observe sibling prints a
strip; they print none.

**Not every such lesson should print one.** The same block shape carries
lessons whose design is that the learner sees no model: a dictation ("write
家, 汉, 语, 文, 国 without a model"), a "select, do not copy" form card, a timed
repair. A strip there gives away the answer the lesson is testing.

**The rule: follow the writing stage.** Every practice block that asks for
writing already declares its stage (`<!-- hl-writing-stage: … -->`, HL19),
from the seven defined in `core/assessment-policy.json`. Three of them show
the learner a model, and those blocks take the strip:

| stage | the learner … | strip |
|---|---|---|
| observe-trace | traces with the model visible | yes |
| guided-copy | copies beside the model | yes |
| delayed-copy | looks, hides the model, writes, then compares and repairs | yes |
| dictation-transcription | writes from sound, no model | no |
| controlled-composition | chooses and orders known language | no |
| connected-composition | writes connected sentences | no |
| timed-assessment-production | writes under exam timing | no |

Delayed copy is on the "yes" side because its model is shown before and
compared after ("Study 汉 for five seconds. Cover it. … Reveal the model and
repair"): a printed book has to give the learner a model to cover, and the
strip is that model with its route drawn in. A block with no stage directive
is never chosen, whatever its title: that keeps out the Punjabi selector
cards ("Guided Practice — decide before writing") and the Hindi concept
lessons whose Guided Practice is a list of cues (HI-W02-abugida-ka-ta's
headword is अ, a letter it never asks the learner to write).

So the strip's block is: the first Writing block, else the first Script
block, else the first block whose writing stage is observe-trace,
guided-copy or delayed-copy (`stripBlockIndex` in `figure-targets.ts`). The
fallback only adds lessons; no existing strip moves. Every other rule is
unchanged: the headword decides letter, list, word or phrase, and the ledger
decides whether every piece is cited.

**Letter anchoring is not widened.** `letter-anchoring.ts` counts a lesson as
a letter lesson only when its strip comes from a Writing or Script block
(`letterBlockIndex`). A copy lesson that gains a strip from its practice
block practises a letter an earlier lesson taught (ZH-W16-han-observe), so
counting it would only double-count that letter.

**The app follows the same rule.** language-ladder places a lesson's strip
itself (the book inserts it from generated targets, so the authored Markdown
has no image to find). Its `filmstripSectionIndex` applies the same three
steps, with the same three stages, held equal to the book's by a test. Its
section parser now reads the stage directive instead of printing it as a line
of lesson text, which it had been doing for every section with a stage.

**Expected.** Exactly 25 lessons gain a strip: chinese 14 (the seven
characters' -guided and -delayed lessons), gujarati 10 (GU-C20-ghar,
GU-C20-mandir, GU-C21-haath, GU-C21-paisa, GU-C22-shaalaa, GU-C22-shahar,
GU-C23-dukaan, GU-C23-gaam, GU-W20-gha, GU-W21-ai-matra) and hindi 1
(HI-W01-na-ma, the list न, म). Of the other five cited headwords,
ZH-R17-writing-five is a dictation and PA-W09-date-select a selector card,
and HI-W02-abugida-ka-ta, HI-W02-ka-ta-mouth-order and HI-W04-ra-sa-mera-naam
declare no stage (their letters are taught, with strips, by the HI-S letter
lessons).

**Prose follows the strip.** None of the 25 disclaims its stroke order. Two
kinds of prose are moved to agree with it. HI-W01-na-ma numbered म's pieces
"lower loop, upper loop, spine, bar", while the cited strip draws "descend,
loop, sweep right / climb the spine / descend it / the headline"; its list
now follows the strip. Five Chinese delayed-copy lessons (语 国 文 看 书) said
"look … and cover it" in the Warm-up, so the strip, printed at the top of
Guided Practice, appeared after the model was meant to be covered; the
look-and-cover sentence now opens the Guided Practice, under the strip.

#### Design — no strip in a Writing block that shows no model

**The gap.** The stage rule above was applied only to the fallback. A
`## Writing` or `## Script` block took the strip whatever stage it declared,
and 37 strip lessons declare a no-model stage on that block: 31 dictations
and 6 compositions (measured from `resolvedFigureTargets`, 829 strips, by the
declared stage of the block each strip lands in). The other 792 land in an
observe-trace, guided-copy or delayed-copy block, or in a Writing or Script
block that declares no stage (the twelve Kannada and Malayalam strips added
above are all of this last kind); a scan of those blocks for dictation cues
("YOU HEAR", "from sound", "without a model", "Cover the …") found only
presentations and copies ("Keep ਪ in front of you and do not cover it").

**What the reader saw.** The book prints a strip at the top of its block,
under the heading and above the task. In a dictation that is the answer,
drawn large, above the cue (Spanish chapter 1, before this change):

> **Writing — short dictation**
> [strip: *How hola is written, letter by letter, stroke by stroke*]
> *Hear:* *OH-la*. Write the Spanish greeting from that sound alone. Three
> sounds reached your ear. Four letters belong on your page. Your pen has to
> supply a letter that the sound never gave you — and it goes first.

The lesson tests exactly the silent *h*, and the strip draws it first. The
Warm-up had just said "Cover the answer line lower on this page". The
single-letter dictations say outright that no model is in view, a few lines
above the strip: SA-S02-dictation "Cover every **न** on the page. Nothing to
copy, nothing to uncover, and no stroke order in front of you"; KA-S01 and
FA-W00 the same. The Gujarati spaced-return lessons (GU-R03 … GU-R19,
"Writing — from sound") print "Cover every model. Write: ja, ka, ka" under a
strip of ક and જ. The compositions are no better: ES-W01-frase-propia is
titled "Your own line — the first writing with no model" and printed a strip
of *buenos días*, the very greeting its 4 p.m. task rules out.

**Is it a check instead?** No. In these lessons the comparison is a sentence
after the attempt ("Now uncover and compare: **hola**"), inside the same
block, or a Wrap-up line ("Uncover the model and compare") that points back
to the model the previous lesson printed. No lesson has a block that is the
answer key, so there is no later block a strip could honestly move to, and
the one it is in is read before the task. The Wrap-up blocks are themselves
recall ("From the heard cue hā alone, write …") or reflection.

**The rule.** A block whose declared stage shows no model (dictation, either
composition, timed production) is skipped wherever the strip is looked for:
the first Writing block that is not one, else the first Script block that is
not one, else the first modelled practice block (`stripBlockIndex`). A block
that declares no stage is unchanged.

**Expected.** 33 lessons lose their strip and 4 move:

- removed: arabic 1 (AR-W04-arbaa-sutur), french 2 (FR-W01-salut-dictation,
  FR-W04-quatre-lignes), german 2 (GE-W01-hallo-dictation,
  GE-W04-vier-zeilen), gujarati 14 (GU-W01-haa-dictation and the thirteen
  "Writing — from sound" returns GU-R03 … GU-R19), italian 1, kannada 1
  (KA-S01-dictation), latin 1 (LA-W04-quattuor-versus, Latin's only strip),
  malayalam 1, persian 1 (FA-W00-alef-dictation), portuguese 1, sanskrit 4
  (SA-S02-dictation, SA-W03-mama-dictation, SA-W03-mama-nama-dictation,
  SA-W05-vocalic-r-dictation), spanish 3 (ES-W00-hola-dictation,
  ES-W01-frase-propia, ES-W02-cuatro-lineas-ayer), telugu 1
  (TE-S01-dictation). Every one follows copy lessons of the same headword
  that keep their strips (ES-W00-hola-delayed-copy, SA-S02-letter-na …).
- moved: MR-W03-ba, -lla, -va, -ya. Their Script block prints the letter and
  describes its strokes, and the "Writing — heard cue" after it opens "Cover
  the model". The strip now sits with that model, and is covered with it.

None of the 37 mentions a strip in its prose, so no lesson text moves.
Letter anchoring is unchanged: `letter-anchoring.ts` asks whether a lesson
writes a letter (a single-letter dictation does), not whether it prints a
strip, and no longer goes through the strip placement to ask it. The app's
`filmstripSectionIndex` skips the same sections. A converse guard joins the
disclaimer guard: a lesson with no strip may not say "follow the numbered
strip" or "the strip shows".

**A mislabelled stage.** PA-W08-digit-recognition declared delayed-copy, but
its practice is "Hear: zero, one, two, two, five, one. Write one digit after
each word, then compare with A": a heard item turned into writing, which is
dictation-transcription. It is relabelled, and now says "Cover A" first and
"uncover A and compare" after, so value A is not in view while the digits are
written. It prints no strip either way (its digits are not cited). Punjabi
keeps delayed-copy evidence in eleven other lessons, the first in chapter 1
(PA-W01-haan-delayed-copy), so no stage gate moves.

#### Design — the app places its strip with the book's code

**The gap.** The two paragraphs above say the app "applies the same three
steps". It did not. language-ladder's `filmstripSectionIndex` took the first
section titled Writing OR Script, whichever came first; the book takes the
first Writing block and looks for a Script block only when there is none. A
letter lesson laid out Warm-up / "Script you'll notice: د" / "Writing: د"
showed its strip under the Script heading in the app and under the Writing
heading in print. The test that held the two "equal" compared only the list
of modelled stages, not the steps. Measured over the corpus (every writing
lesson with a committed `<id>-filmstrip.svg`, 795): 435 strips sat in a
different section, all of them a Script section above the book's Writing
block (malayalam 57, kannada 54, hindi 47, sanskrit 43, telugu 43, gujarati
41, marathi 39, tamil 37, urdu 18, persian 16, arabic 13, marwadi 10, punjabi
10, russian 5, bengali 1, japanese 1). No strip appeared or disappeared.

**The rule now has one home.** `strip-placement.ts` in human-language-data
holds `filmstripBlockIndex`, `stripBlockIndex`, `letterBlockIndex`,
`modelledPracticeBlockIndex` and `MODELLED_WRITING_STAGES`, moved unchanged
from `figure-targets.ts`, which re-exports them. The module has no imports,
because `figure-targets.ts` needs `node:path` and the app's eager chunk has
a size budget. The app runs `filmstripBlockIndex` on the book's own parse of
the body (`parseBodyBlocks`) and translates the block index into a section
through each section's `blockIndex`: the app shows the preamble as a section
and drops a heading with nothing under it, and the book does neither. A
corpus test checks, for all 795 lessons, that the section the app picks is
the block the book's `parseLesson` picks, by index and by heading.

**One deliberate gap.** FA-C03-chist is a word lesson with a declared strip
(glyph چ, placed in the block that introduces it). The app shows strips only
on writing lessons and captions them from the headword, which would read
"How چیست is written", so it still shows none there.

### Class B — data diagrams (generated)

Etymology and cousin-web trees built from lesson `roots`, sound-articulation diagrams
built from `sounds` ids and the pronunciation reference, gender maps, script-evolution
charts.

- **Source of truth:** the canonical lesson AST. A diagram may assert only what a
  lesson already asserts.
- **Renderer:** deterministic SVG via `paint-vm-svg`, same as Class A.
- **Verification:** hash-gated; every node in a rendered tree must trace to a `roots`
  entry or a declared knowledge atom. A diagram may not introduce a claim.

This matters because the cousin web is the project's signature method, and it is
currently prose-only. An etymology tree is the one diagram this curriculum most
obviously wants.

### Class C — illustrations (authored assets)

Model-generated raster art for scenes, objects, and cultural context — the "colour"
the books currently lack.

- **Storage:** `_assets/illustrations/<track>/`, beside the existing `_fonts/`.
- **Provenance:** each asset carries a sidecar JSON recording generator, model,
  prompt, date, and licence. CI verifies presence, required fields, and a content hash.
- **Determinism:** raster art cannot be regenerated byte-identically, so the *asset*
  is the committed artefact and the hash is the gate — the same posture the repo
  already takes toward vendored fonts.
- **Subject restriction:** non-linguistic subjects only. No script, no glyphs, no
  handwriting, no transliteration, no claim about a language's structure or history.
  Class A holds the glyph monopoly; Class B holds every factual diagram.
- **Budget:** a per-track asset-size cap enforced in CI, so the repository does not
  silently accumulate tens of megabytes of art.

### Licensing

**Decided by the project owner on 2026-08-06 and recorded in
[`_assets/LICENSE.md`](../learning/human-languages/_assets/LICENSE.md).** The books stay
**CC BY-SA 4.0** — no relicensing. Generated Class C illustrations are marked
**`CC0-1.0` with `rightsAsserted: false`**, each with a provenance sidecar.

The reasoning, in short: a Creative Commons licence grants copyright permissions, and
purely AI-generated output likely lacks the human authorship copyright requires (US
Copyright Office — *Zarya of the Dawn*, *Thaler v. Perlmutter*, and subsequent
guidance). Stamping CC BY-SA on such an image asserts a right that may not exist, and
its ShareAlike clause would bind readers to an obligation that may be unenforceable.
CC0 is safe whichever way the law settles. Jurisdictions differ — UK CDPA s9(3) grants
50 years for computer-generated works — which is why per-asset provenance matters more
than a single global claim. This is a recorded project decision, not legal advice.

Two operational constraints ride along with the decision: prompts must avoid living
artists, brands, and recognizable characters; and each generator's output terms must be
checked per asset, against the terms in force on the generation date.

CI still gates on the record, not on the outcome of the decision: every Class C asset
must carry a provenance sidecar with all required fields and a recorded licence, and
its `sha256` must match the committed file. An asset missing either fails the build —
an unlicensed image in a freely published book is a real problem, not a formality.

## Class D — the design system

Figures alone do not make a book colourful. The preambles already load `xcolor` and
`tcolorbox`, so the foundation exists:

- a named palette, defined once and shared across all 20 preambles;
- chapter openers that print the chapter's `canDo` from
  [HL05](./HL05-chapter-capability-and-step-by-step-shape.md);
- restyled `sounds`, `cousinweb`, `culture` and `grammarlens` boxes;
- a figure environment with consistent captioning and placement.

Per-track preambles are currently standalone copies. The palette and figure
environment should land in a shared `_shared/visual.tex` that each preamble inputs,
rather than being copied 20 times — the repo's own lessons warn specifically against
hand-written N-fold families.

## Pipeline changes

1. **Preambles** — add `graphicx` and `\graphicspath`; input the shared visual file.
2. **Renderer** — `book.ts` gains block-level and inline image branches in
   `renderMarkdown` and `renderInlineMarkdown`, plus a filename-safe path escaper
   distinct from `escapeLatexLinkDestination`, with a fail-closed allowlist
   (relative-only, no `..`, extension allowlist) mirroring `safeOutput` in
   `book-cli.ts`.
3. **Figure generation** — a `figure-cli` beside `book-cli`, with the same
   `--write` / `--check` contract and the same manifest-hash discipline.
4. **CI** — add `graphicx.sty` to the `kpsewhich` preflight and `librsvg2-bin` to the
   apt closure for SVG→PDF conversion.

### As built (HL-C06)

The first Class-B vertical slice implements all four pipeline steps. A checked
`figure-generation.json` manifest selects the canonical Spanish *café* lesson;
`figure-cli` reads its ordered `roots` and uses `paint-vm-svg` to commit a
deterministic SVG plus separate source/SVG hashes. The generated book chapter and
Language Ladder both consume that one SVG. The shared `_shared/visual.tex` owns
`graphicx`, placement, and captions; book rendering rewrites safe `.svg` references
to `.pdf`; local Spanish build helpers and the unified books workflow convert with
`rsvg-convert` before XeLaTeX. Unsafe or stale paths fail closed.

**TikZ is explicitly not adopted.** `texlive-pictures` is deliberately excluded from
the focused CI dependency closure, and pre-rendered vectors are both leaner and more
deterministic than compile-time drawing. Keep the toolchain lean.

## The warning gate

Every track's README and CHANGELOG asserts its book builds with zero missing glyphs,
overfull or underfull boxes, duplicate destinations, hyperref warnings, or font
substitutions. **Nothing enforces this.** CI fails only on hard TeX errors
(`-halt-on-error`) and generated-content drift; no step reads the `.log`.

Floats fight the existing `\raggedbottom` layout and are the classic source of exactly
those warnings, so this spec must not add figures without closing the hole it would
fall through. CI gains a log-scanning step after the `latexmk` loop, checking each
`.log` for `Overfull`, `Underfull`, `Missing character`, hyperref warnings, and
duplicate destinations, compared against a **recorded per-track baseline** so existing
debt is measured rather than newly broken.

### As built (HL-C07)

The gate is `code/scripts/scan_latex_log_warnings.py`, run by the books workflow
immediately after the `latexmk` loop, with its own unit tests run first in the step
before it — the gate is the thing being trusted, so a silently broken gate is worse
than no gate. It counts six classes per track: `overfull`, `underfull`,
`missing_character`, `hyperref_warning`, `duplicate_destination`, and
`font_substitution`. The sixth is not in the paragraph above but is claimed by every
track's README, so it is measured too.

Baselines live in `code/learning/human-languages/core/latex-warning-baseline.json`.
A track fails only when it exceeds its recorded counts. A track recorded as `null` has
never been measured and is reported but never failed; `null` means *unknown*, not
*zero*. Because real counts need a real XeLaTeX run over all 20 books, the file ships
fully unseeded and the scanner prints the counts it actually measured into
`$GITHUB_STEP_SUMMARY` as a copy-paste-ready `tracks` block — that is the bootstrap
path, and no number is ever guessed into the repository.

Two further rules keep the gate honest. A track that comes in *under* its baseline is
reported as `under baseline`, an invitation to tighten the number, never a failure. A
track that has a baseline but whose `book.log` has vanished *does* fail, because
otherwise deleting a file would quietly switch that track's gate off. The full scan is
also published beside the books as `latex-warnings.json`.

### Building locally, before the push (HL-C109)

The gate above is the right backstop and the wrong place to *discover* a broken page.
By the time CI reports an overfull box the change is already pushed, and the defect it
found may have been on `main` for weeks.

A typesetting defect is invisible to every other gate in this repo. When the table of
contents began overflowing its chapter-number box — `book.cls` reserves `1.5em`, and a
three-digit bold chapter number needs more — the lesson suites, the drift checks, the
book-hash pins and the bundle ceiling all stayed green, on 21 broken lines. None of
them renders a page. **Only a real XeLaTeX run can see this class of bug.**

So `code/scripts/build-books-locally.sh` builds any or all books exactly as CI does
(SVG figures converted with `rsvg-convert` first), and exits non-zero on any overfull
box, underfull box, or missing glyph. **Run it before pushing anything that touches a
lesson, a book target, or a preamble.**

The script also carries the one-time macOS setup, because the failure mode there is
silent rather than loud: a Homebrew TeX Live does not register its OpenType fonts with
the system font database, and XeLaTeX resolves `\setmainfont{Latin Modern Roman}` by
*name*. Unregistered, every font falls back to `nullfont` and the log fills with half a
million `Missing character` lines while still producing a PDF-shaped file.

One rule governs every fix made in response to this script: **change how the page is
typeset, never how it is reported.** `\tolerance` and `\emergencystretch` change how a
paragraph is broken and are fair game; `\hbadness` and `\hfuzz` change only what TeX
prints, and raising them would leave the CI scanner reading a muted log.

## Acceptance criteria

The visual system is complete when every non-Latin track prints a stroke-order figure
for every letter it teaches, generated from font-validated ductus data; the app renders
those same paths from the same source; every Class A and B figure is hash-gated and
regenerable from a clean checkout; every Class C asset carries provenance and a
recorded licence; the shared palette and figure environment live in one file rather
than 20; and the log-scanning gate reports zero new warnings against each track's
baseline.
