## HL-C443-f07c9794 — No book prints a stroke-order filmstrip, though about 380 writing lessons have cited stroke data

**Status: OPEN — top of the queue.** Raised by the project owner: writing should
be taught one letter at a time, then combinations, and the ductus work should
show up as filmstrips.

### What exists

- **Stroke data:** 378 cited ductus records in
  `code/packages/typescript/script-ductus/src/strokes/`:
  - chinese 60, devanagari 44, gujarati 44, cyrillic 33, arabic 32,
    urdu-nastaliq 31, tamil 29, perso-arabic 24, japanese 23, hebrew 22,
    malayalam 14, kannada 13, telugu 9
  - bengali and gurmukhi have none
  - Each record has a `strokeOrderSource` and a pen path that has been
    checked against the font's ink.
- **Renderer:** HL-C300 proved that every script renders. Filmstrip SVGs come
  from `figure-generation.json` targets through `generate:filmstrip-ledger`
  (script-ductus) and `generate:figures`.

### Why no learner sees one

1. **Every filmstrip needs a hand-declared target.** Only three targets were
   ever declared, as proofs: `TA-S119-letter-a`, `HI-S04-letter-aa` and
   `FA-C03-chist`.
2. **No lesson references even those three.** A lesson has to carry a Markdown
   image (`![…](figures/<id>-filmstrip.svg)`) for `book.ts` to print it, and
   none does. `TA-S119-letter-a` still prints its five strokes as a text list.
   No gate notices a target that nothing references.

### How many lessons this affects

Writing lessons whose headword already has cited stroke data (the second
number in each pair) are the ones that could show a filmstrip today:

- chinese 72 of 74
- hindi 43 of 87
- marathi 42 of 102
- sanskrit 39 of 53
- gujarati 34 of 86
- marwadi 32 of 52
- japanese 23 of 57
- tamil 21 of 82
- russian 18 of 31
- kannada 13 of 75
- malayalam 13 of 75
- urdu 12 of 19
- persian 10 of 16
- telugu 9 of 68
- arabic 3 of 21

That is about 380 filmstrips that could print today.

### Plan

1. **Derive targets instead of declaring them.** Add
   `writingFilmstripTargets(root)` in human-language-data:
   - Map each track to its script: hindi, marathi, sanskrit and marwadi to
     devanagari; persian to perso-arabic; urdu to urdu-nastaliq; and so on.
   - Select every `type: writing` lesson whose headword glyph has a
     `strokeOrderSource`.
   - Merge the result with the explicit targets in both the filmstrip ledger
     and `figure-cli`.
2. **Print it without editing 380 lesson files.** `book.ts` places the
   lesson's filmstrip under its `## Writing:` block whenever a target exists
   for that lesson.
3. **Gate it.** Fail when a writing lesson whose glyph has a cited source has
   no filmstrip, and when a filmstrip target is never printed.
4. **Fill the stroke-data gaps, one letter at a time, each with a citation:**
   - Telugu, Kannada and Malayalam consonants (today only vowels and chillus
     are sourced).
   - Bengali inventory and sources (see HL-C212).
   - Gurmukhi (nothing yet).
5. **Combinations.** HL11 §5.3 says composed forms (consonant + vowel sign)
   derive from base letter + sign. Nothing implements that yet. Conjuncts need
   their own cited records.

### Progress

- **Mechanism shipped in #16015.** Filmstrip targets are now derived, the book
  places each figure, and a gate fails if a figure is never printed.
- **Tamil first,** then the four Devanagari tracks.
- **Then every other track with any cited ductus.** 207 lessons now print a
  filmstrip.
- **Next: headword formats the derivation does not read yet.**
  - chinese (72 cited letters), russian (18) and urdu (12) print none
  - arabic and japanese print one each

  In these tracks a writing lesson's headword is not one grapheme. It is a
  character with its reading, or an upper and lower pair. The fix is a
  per-track extractor that reads the letter out of such a headword, instead of
  requiring the whole headword to be one grapheme.

- **Filmstrips in the app.** The language-ladder app does not show filmstrips
  yet. Its eager figure map excludes `*-filmstrip.svg`, because 370 URLs pushed
  first paint over its 500 kB budget. Showing them there needs a lazily loaded
  figure map.

### The shape of the writing ramp (project owner, 2026-09-25)

Writing must be gentle in the same way vocabulary is:

- **Teach a letter from a word the reader already knows.** A letter lesson
  follows the first word lesson that uses the letter. The reader copies a
  letter out of a word they can already say, never a letter out of nowhere.
  This is HL11's drizzled ramp, and where it exists it stays the rule.
- **One letter at a time, then combinations.** Single letters come first.
  Consonant + vowel-sign forms and conjuncts come only after both of their
  parts have been taught, and a combination is drawn as its parts.
- **Eventually every letter.** A track's writing ramp is incomplete while any
  letter of its inventory has no letter lesson. Some letters appear in no word
  the track teaches yet: for example Telugu ఙ ఛ ఝ ఱ and the independent vowels
  ఈ ఊ ఓ ఔ, and Hindi's independent इ. For those, the fix is a vocabulary
  lesson whose word needs the letter, followed by the letter lesson. A letter
  taught in isolation does not count.

The work that follows from this:

- A per-track measure: inventory letters with a letter lesson; letters with no
  anchoring word; letter lessons that come BEFORE their anchoring word.
- Gates that let each of those numbers fall and never rise.

Progress (2026-09-25): the measure exists. `measureLetterAnchoring`
(`human-language-data/src/letter-anchoring.ts`) sorts every letter lesson on
every non-Latin track into one of four groups:

- **anchored:** an earlier word headword holds the letter;
- **builds-toward:** the word comes later in the same chapter;
- **cold:** no such word;
- **unmeasured:** a Han component. Telling whether 亻 belongs to 你 needs
  decomposition data.

It also lists the **unwritten** letters: read in a word, but written by no
letter lesson. `tests/letter-anchoring.test.ts` pins per-track ceilings that
may fall and must not rise.

Measured: 845 letter lessons, where a letter set like "வ, க" or "௧ ௨ ௩"
counts as one. 490 are anchored, 190 builds-toward, 156 cold and 9
unmeasured. There are 93 unwritten letters.

The fixes, biggest first:

- **Unwritten.** Malayalam 9, Bengali 5, Japanese 5.
  Each fix is one letter lesson that follows the first word using the letter.
  Two tracks are done, each with one lesson per letter that names its word:
  - Tamil 14 → 0, in chapters 109-112.
  - Arabic 18 → 0, in chapters 46-49. Four of those 18 were already written in
    "letters — word" lessons that the measure now reads.
  - Persian 18 → 0, in chapters 22-26.
  - Urdu 16 → 0, in chapters 34-37.
- **Cold.** Marathi 43, Gujarati 34, Kannada 19, Telugu 13, Malayalam 12,
  Hindi 9. These are mostly alphabet-first openings, where the letters come
  before any word. The fix moves the letter lesson after its first word, or
  adds a word first.
- **Builds-toward.** Chinese 51, Marwadi 49, Japanese 32. Here the word
  follows the letter within the same chapter. The fix reorders the letter
  lesson to come after the word.

Still to measure: inventory letters that appear in NO word. The unwritten list
only sees letters some word already shows, so letters like Telugu ఙ ఛ ఝ ఱ need
a script inventory to be counted.

The filmstrip goes on the letter lesson, so it always lands right after the
word that introduced the letter.

HL11 §5.2's rule stands throughout: no citation, no pen path, no figure.
