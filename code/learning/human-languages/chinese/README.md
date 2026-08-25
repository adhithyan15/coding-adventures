# Mandarin Chinese

This track teaches standard Mandarin (Putonghua) in simplified characters through
the shared human-language spine. Every lesson is written for a fresh learner,
takes under five minutes, and introduces only the characters the expression on the
page actually needs.

The authored book has grown well beyond its original two-chapter prototype. Its
canonical inventory is deliberately derived from
[`chapters.json`](./chapters.json) and [`curriculum.json`](./curriculum.json),
rather than pinned here to a count that becomes false whenever a bounded tranche
lands. The current progression has four gentle phases:

- **Greeting and script foundations:** hear and use **你好** *nǐ hǎo*, then take
  its characters apart into reusable components and ordered strokes.
- **Numbers and first choices:** read and write one through five, reuse box
  shapes, negate a word, and answer with **是** or **不是**.
- **Leaving and time of day:** close a conversation with **再见**, then build a
  morning greeting without pretending its neutral-tone rule always fires.
- **Identity, repair, and courtesy:** say a name, ask **什么**, recover when a
  word is missed, thank and answer thanks, and use **请** with Mandarin rather
  than English pragmatics.

The opening split remains deliberate. Teaching the greeting's sounds, characters,
components, and strokes in one lesson would create a cliff with a gentle label on
it. Later chapters keep the same rule: oral meaning first when a new written form
would otherwise overload the learner, then staged handwriting and active recall.

## Why this track exists

Mandarin was added as a **scale test** after the registry had already crossed
several language families and script systems. It shares no ancestry with English,
is written logographically rather than alphabetically, and is tonal. It is here
to find out which parts of the method describe language in general and which
parts quietly describe alphabetic or Indo-European languages.

Three findings, recorded plainly because a track that hides them is worth less
than one that reports them:

### 1. The cousin web does not transfer. Nothing fully replaces it.

[`HL00`](../../../specs/HL00-human-language-curriculum-framework.md) calls
etymology "the engine of the whole curriculum": every Spanish word is taught as a
cousin of English words the reader already owns, because the brain attaches the
new to the already-known. **Chinese and English share no ancestor, so that engine
has no fuel here.** There is no honest chain from *nǐ* or *hǎo* to any English
word, and HL00 forbids inventing one.

What this track uses instead is **character composition**: 你 is 亻 (person) plus
尔, 好 is 女 (woman) beside 子 (child). It is a real structural fact, it is
visible on the page, and it does help. But it is a **weaker** hook than the cousin
web, for a reason worth naming: a cousin web attaches a new word to knowledge the
reader *already has*, while a component gloss attaches it to knowledge the reader
is acquiring *in the same breath*. 女 means nothing to a beginner until this
course says so. The anchor is internal to the language rather than borrowed from
the reader's existing memory.

It is also less reliable. The woman-and-child reading of 好 is a traditional
gloss, not settled palaeography, and the lesson says so. Roughly 80% of Chinese
characters are phono-semantic — one component for meaning, one for sound — and
their sound components are frequently opaque after three thousand years of drift.
A component-hook method has to keep flagging which glosses are mnemonic and which
are historical, or it becomes exactly the folk etymology HL00 bans.

**Verdict: the method's signature device does not generalise. The substitute is
usable, honest, and measurably weaker.**

### 2. Characters are not letters, and "the letters in this word" does not map cleanly.

HL00's inline-script rule is *"if a word needs four letters, introduce those four
letters in that word's lesson."* An alphabet has two levels — word and letter —
and the rule walks between them.

Chinese has **three**: word → character → component. 你好 is one word, two
characters, four components. The rule survives the translation only if you decide
which level "letter" means, and neither answer is right on its own: teaching the
characters alone leaves the shapes unmemorable, teaching the components alone
leaves the reader unable to say anything.

This track teaches all three levels in the same lesson, which works, but it makes
the character lesson denser than a letter lesson in any other track. It also
merges two sections HL00 keeps separate: for Chinese, *"the letters in this word"*
and *"the word, taken apart"* are the **same analysis**. 女 + 子 is at once the
orthographic decomposition and the memory hook. Every other track keeps script and
etymology in different boxes because letters carry no meaning.

### 3. Tone is phonemic, and the segmental "sounds you'll need" note only half-carries it.

HL00's inline pronunciation note was designed for facts attached to a letter —
*"the h is silent"*, *"the vowels are pure"*. It carries tone adequately at the
lesson level: `sounds: [tone-3]` in the frontmatter and one paragraph in the block
is enough for the reader.

Where the model actually had to grow was the **data layer**. See
[the CHANGELOG](./CHANGELOG.md) for the exact change; in short, `Letter.tone`
already existed and records which tone a *character* carries, but nothing could
express the tone *inventory* or a **sandhi rule** — a rule that changes a syllable's
pitch because of the syllable after it, without changing a single stroke of the
writing. `nǐ hǎo` is spoken `ní hǎo`. A learner who trusts the printed pinyin
mispronounces the commonest greeting in the language. That is a fact about a
sequence, not about a glyph, so it could not live on `Letter` at all.

The second thing that had to grow was the **lesson-type vocabulary**. Because every
earlier track's sound facts belong to letters, they always fit inside a word
lesson, and no track ever needed a lesson *about pronunciation*. Tone does not fit:
folding it into the first character lesson pushed that lesson to 352 effective
seconds, past the five-minute contract. HL08's rule is to split, not to waive, so
the material became its own lesson and `pronunciation` joined the exempt lesson
types beside `grammar` and `etymology`.

## Read and practise

- [`roadmap.md`](./roadmap.md) records the original authored-and-planned path
  toward B1, including the font-subset constraint that shapes chapter order;
  the machine-readable curriculum is authoritative as newer tranches land.
- [`session-map.md`](./session-map.md) is explicitly the opening chapter's exact
  review ledger, not an inventory of the whole track. It also records why that
  opening chapter's drivable prefix is one lesson.
- [`pronunciation-reference.md`](./pronunciation-reference.md) collects the tones,
  the sandhi rules, and the pinyin segments for lookup; it is never a prerequisite.
- [`chapters.json`](./chapters.json) is the HL05 capability ledger: one
  first-person "I can …" promise per chapter and the payoff lesson that proves it.
- [`lessons/`](./lessons/) contains the canonical short lessons referenced by the
  curriculum; the directory grows with each reviewed tranche.
- [`book/book.tex`](./book/book.tex) builds the generated book with XeLaTeX from
  those canonical lessons.

## A note on the empty `bridges` list

`core/languages.json` gives every track a directed `bridges` array — languages
whose history, vocabulary, or structure gives that track a leg up. Persian
bridges to Arabic and Urdu; German bridges to English. **Mandarin's is empty**,
and that is not an unfinished reciprocal edge. Japanese now has its own track and
correctly names Chinese as a bridge because Japanese borrowed a large layer of
Chinese vocabulary and characters. The Mandarin path does not assume Japanese,
Korean, or Vietnamese knowledge in the other direction. Listing English because
a handful of loanwords crossed (*tea*, *ketchup*, *typhoon*) would likewise claim
a teaching bridge no Mandarin lesson actually walks across.

## Script and font

Characters, components, stroke order, and the tone inventory live in
[`data/scripts/chinese.json`](../data/scripts/chinese.json). Unlike the Indic
scripts here, Chinese stroke order is a standardised taught system, so that file
marks its stroke orders `authoritative`.

The book sets Chinese in `_fonts/NotoSansSC-Subset.ttf`, a fonttools subset of
Noto Sans SC. It covers every CJK codepoint appearing **anywhere** in that JSON
file — not only the inventoried `letters`, but every character named inside
another entry's components, stroke notes or citations. The subset therefore
contains printable component and citation glyphs that are not taught as
standalone inventory items. Only a character mentioned nowhere in the file needs
an entry plus a re-run of
[`_fonts/subset-cjk.sh`](../_fonts/subset-cjk.sh). Check the font's cmap before
concluding a character is unavailable.
