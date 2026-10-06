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
continuous headline, while every cited letter draws its own; Arabic-family
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

#### Designed — Devanagari words: the letters' bodies, then one shared headline

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
strokes · N−1 pen lifts · M movements". The last caption reads "lift, then one
shirorekha over the word". Its citation names every letter's source by
position ("letters 1 and 2: …") and the HP Labs India counts for the headline;
its `<desc>` says the frames draw the word, letter bodies first and the shared
headline last. The target carries `composition: "shared-headline"`, and
`script-ductus` builds the entry for exactly those targets.

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
