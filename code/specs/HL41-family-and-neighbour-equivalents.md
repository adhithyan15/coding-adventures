# HL41 — Family and neighbour equivalents

## Why

A word sticks when it hooks onto something the reader already holds. For a
Tamil learner, the strongest hooks are:

- its sisters: Kannada **ಹೋಗು** *hōgu*, Malayalam **പോകുക** *pōkuka*;
- the big neighbour: Hindi **जाना** *jānā*;
- English: *go*.

Some of those share a root with the Tamil word, like **போ** *pō* and *hōgu*
(one old *p* softened to *h*). Others do not, and that contrast is itself
worth seeing.

Today such comparisons appear only where an author thought to write them into
prose. About one Tamil lesson in ten names a sister-language word. Nothing
guarantees that a learner sees the family for every word.

HL41 makes the comparison systematic. Every vocabulary lesson in a track can
carry a small, uniform **family and neighbours** panel:

- the headword's equivalents in each language of the track's comparison set;
- each one in its own script, with romanization;
- each one marked for whether it shares the headword's root;
- the English last.

HL00's audience rule still holds. A comparison is information the page
supplies. It never assumes the reader knows the other language.

## Comparison sets

Each track names its **family** languages, and at most **one neighbour**. A
second neighbour is added only where it plainly helps memory. English is
always the last column. The neighbour is the largest language the learner is
likely to meet alongside the target, or the one whose loanwords run through
it.

| Track | Family | Neighbour |
|---|---|---|
| tamil | kannada, telugu, malayalam | hindi |
| telugu | tamil, kannada, malayalam | hindi |
| kannada | tamil, telugu, malayalam | hindi |
| malayalam | tamil, kannada, telugu | hindi |
| hindi | urdu, marathi, gujarati, bengali, punjabi | persian |
| marathi | hindi, gujarati, bengali | kannada |
| gujarati | hindi, marathi, marwadi | persian |
| bengali | hindi, marathi | sanskrit |
| punjabi | hindi, urdu | persian |
| marwadi | hindi, gujarati | persian |
| urdu | hindi, punjabi | persian, arabic |
| sanskrit | hindi, marathi | latin |
| persian | — | urdu, arabic |
| arabic | — | persian |
| spanish, french, italian, portuguese | the other three | latin |
| latin | spanish, french, italian, portuguese | — |
| german | — | — (English is Germanic family) |
| russian | — | german |
| japanese | — | chinese |
| chinese | — | japanese |

The sets live in `core/comparison-sets.json`. The validator refuses an
equivalent in a language outside the track's set. This keeps the panels
uniform and stops them from growing into lists nobody reads.

## Data

Equivalents are data, not lesson prose, for three reasons:

- A panel must not change a lesson's measured duration. Many lessons sit near
  the 300s budget, and a panel is reference material the learner glances at,
  not narration.
- A panel must not add atoms or glyphs to the lesson's teaching closure.
- One place per lesson makes the data reviewable. A native speaker can correct
  a Kannada form without reading Tamil lesson prose.

Each track keeps one owner file per lesson at
`<track>/equivalents.d/<LESSON-ID>.json`:

```json
{
  "lesson": "TA-C32-po",
  "english": "go",
  "equivalents": [
    { "language": "kannada",   "form": "ಹೋಗು",  "romanization": "hōgu",   "sameRoot": true  },
    { "language": "telugu",    "form": "వెళ్ళు", "romanization": "veḷḷu",  "sameRoot": false },
    { "language": "malayalam", "form": "പോകുക", "romanization": "pōkuka", "sameRoot": true  },
    { "language": "hindi",     "form": "जाना",  "romanization": "jānā",   "sameRoot": false }
  ],
  "source": "cross-track match: KA-VERB-HOOGU, TE-VERB-VELLU, ML-VERB-POKUKA; hindi hand-filled"
}
```

Validation rules:

- `lesson` names an existing lesson in the track. The lesson is a vocabulary
  lesson (`type: word` or `type: phrase`).
- Every `language` is in the track's comparison set. Each appears at most
  once, in comparison-set order.
- `form` is written in that language's declared script: every letter belongs
  to that script's Unicode block, plus spaces and ordinary punctuation.
  `romanization` is required unless the language is written in Latin script.
- `sameRoot` is optional. `true` means the words are cognate or one borrowed
  the other. `false` means a reviewer judged them unrelated. An absent value
  means nobody has judged the pair yet. The panel marks only `true`, so an
  unjudged pair is never presented as related.
- A `form` is one word or a short phrase of at most four words, never a
  list. A twelve-month list in one table cell cannot break across a page, and
  a list's "equivalent" is rarely the same list: Hindi's Gregorian months are
  not the Tamil solar months.
- A `form` is never a paradigm. When the matched lesson's
  headword lists several forms (`आना / आता / आती`), the entry takes the first.
- `source` says where the data came from, so a reviewer knows what to trust.
  It is either a cross-track match naming the matched lesson ids, or
  `hand-filled`.

## Rendering

**Books.** The panel renders at the end of the lesson's first teaching
section, as a compact table:

| | word | same root |
|---|---|---|
| Kannada | ಹೋಗು (*hōgu*) | yes |
| … | | |
| English | go | |

The word and its romanization share one cell. A long word such as Malayalam
എഴുന്നേൽക്കുക cannot break, and a third of the line holds it where a quarter
did not. The "same root" column appears only when some row is marked.

Each form is set in its own script's font. The book's script set must list
every script its comparison set uses. The Tamil book's `tamil-comparisons`
set already carries Telugu, Kannada, Malayalam and Devanagari.

**The app** will render the same table under the lesson's first section
(rollout step 2).

**Narration** does not read the panel. **Duration** does not count it.

Because the forms are reference material in a panel, the script-closure and
letter-anchoring measures ignore them. This is the same rule that already
covers the sister-language words about one Tamil lesson in ten names in prose.

## Seeding the data

Hand-writing thousands of forms invites errors. The first pass is derived
from the corpus itself, where the other tracks already teach the words:

1. **Match by concept.** A shared `concept_tag` (for example `VERB-GO`), or
   parallel tags that differ only by track prefix (`TA-FOOD-BASIC` /
   `KA-FOOD-BASIC`), link lessons across tracks.
2. **Match by gloss.** Next, match on the first sense of the gloss ("to go",
   "water"), keeping only unambiguous one-to-one matches.
3. **Hand-fill the rest**, and mark each such entry `hand-filled`.
4. **Human review.** A native speaker of the target language reviews each
   pilot track before it widens.

HL01 warns that a universal vocabulary creates false equivalences. So a match
is only a candidate until review, and `sameRoot` is always hand-judged.

## Rollout

1. The data layer, validator, comparison sets and book panel, together with
   Tamil equivalents for the vocabulary lessons.
   **Pilot: Tamil.** The user, a native Tamil speaker, reviews it.
2. The app panel, reading the same owner files.
3. Telugu, Kannada and Malayalam.
4. Hindi and the Indo-Aryan tracks.
5. Romance, then the rest.

## Not in scope

- Etymology prose. Lessons keep their "word, taken apart" sections, and the
  panel is not a replacement for them.
- Audio for the equivalents.
