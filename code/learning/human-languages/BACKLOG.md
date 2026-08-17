# Human Languages Backlog

This is the ordered delivery backlog for the shared-spine curriculum, books,
and Language Ladder. Reprioritize it after every merged work item. Add newly
discovered work here before starting it so the repository, rather than an agent
session, remains the source of truth.

**The order no longer lives in this file.** As of HL-C208 the queue is computed
from the measured deficit — run `npm run plan` in
`code/packages/typescript/human-language-data`, or see
[`HL15`](../../specs/HL15-the-completion-plan.md) for why. This file remains the
record of what was *learned*; it is no longer the record of what is *next*, because
a hand-ordered list goes stale silently and in the flattering direction. The
prioritization sections below are kept as history and are dated accordingly.

## HL-C224 — the loop's own push cadence is now failing CI, and the failures look like code failures

Three checks on PR 11870 failed with:

```
##[error]Response status code does not indicate success: 429 (Too Many Requests).
##[error]Failed to download archive '.../actions/setup-go/...' after 3 attempts.
```

**Every one failed at `Set up job`, before a single line of the build ran.** GitHub
is rate-limiting *action downloads* for this repository, and the cause is this
loop: eleven PRs in a session, each triggering CI + CodeQL + Books, several of them
twice, plus a re-push per PR for review fixes.

**Why this is worth a row rather than a shrug.** A 429 at setup is reported in
exactly the same place, and with exactly the same red X, as a genuine test failure.
The first instinct on seeing `Build and publish all human-language books: fail` is
to go looking at the books — and on this PR that was doubly misleading, because
there HAD been a real books failure (HL-C223) minutes earlier. **The second failure
looked like the first one not being fixed.** It was not; the fix was never
compiled.

**Rule: read the failing STEP name before the failing job name.** `Set up job`,
`Post job cleanup` and `Checkout` failures are infrastructure. Only a failure
inside a named build or test step is evidence about the change.

### What the loop should do about it

1. **One PR in flight at a time**, which was already the intent but not the
   practice — the Hindi branch was authored while Bengali was still in CI, and
   only held back from pushing because of a pin conflict (HL-C213's sibling).
2. **Do not re-push to fix review findings while CI is mid-run.** Each push
   cancels nothing and starts a second full set of workflows; this PR accumulated
   four CI runs.
3. **Space pushes.** The limit recovers on its own; the correct response to a 429
   is to wait and re-run the failed jobs, never to change code.

Recorded because an agent loop is exactly the thing that generates this pattern,
and the failure mode — infrastructure noise that reads as a code defect — costs
more than the delay does.

## HL-C223 — the glyph probe was mis-scoped, and HL-C214 was one tranche old when it happened

CI: `bengali: missing_character rose to 29 against a baseline of 0`. The cause was
**U+0254 OPEN O** in the romanizations (`nɔ`, `kɔ`, `mɔ`), and the Bengali book's
main font is Latin Modern, which does not have it. Fixed by using `ô`, which is
present and is the ISO 15919 romanization for that vowel anyway.

**HL-C214 was written ONE TRANCHE EARLIER and says to check the generated `.tex`
against the actual font. I did check.** The probe was scoped to *Bengali
codepoints* against the *Bengali font* — and both halves are the wrong question:

- the Bengali characters were never at risk. They sit inside `\bn{...}`, which
  selects a face chosen *because* it covers them.
- **the prose, the romanization and the IPA render in the MAIN font**, and nothing
  looked at that at all.

**A book is not one font.** The probe has to cover every non-ASCII character *not*
inside a script wrapper, against the main font — and separately the wrapped runs
against their own face.

### This is the second instance, which changes HL-C214 from a row to a job

HL-C214 specified a `check:glyphs` gate and deliberately did not build it, because
font resolution differs per track. Two CI round-trips later — `ǣ` in Latin, `ɔ` in
Bengali, both **avoidable and both caught only by the compiler** — that reasoning
no longer holds. The remaining tranches will keep reaching for phonetic characters,
because that is what an etymology-and-pronunciation curriculum does.

**Latin Modern is the recurring hazard specifically**, and it is worth stating on
its own: it is a *typesetter's* font with good Western European coverage and thin
coverage past it, and **every Latin-script book in this corpus uses it as the main
font** — including, for the main-font text, all sixteen non-Latin tracks. It lacks
`ǣ`, it lacks `ɔ`, and it will lack the next one. Characters known present and
useful: `ā ī ō ū ô ə`.

### And the diagnosis itself ran against the wrong tree

The first scan of `bengali/book/` was run **while checked out on the Hindi
branch**, which has no Bengali chapter, and confidently reported five pre-existing
preamble characters as the finding. The tool worked perfectly; the tree was wrong.
Same family as `git stash` not stashing untracked files (HL-C213).

**Print `git branch --show-current` in the same command as any cross-branch
diagnosis.** Cheap, and it would have saved the whole detour.

## HL-C223b — the glyph gate caught its first defect BEFORE a push, and it was a third instance

The Hindi tranche used **ɑ** (U+0251 LATIN SMALL LETTER ALPHA) in four
romanizations — `bhɑ`, `yɑ`, `bɑ`, `ḍɑ`. Latin Modern does not have it.

That is the **third** instance of this exact class:

```
ǣ  U+01E3  Latin      HL-C214   found by CI, 11 minutes after push
ɔ  U+0254  Bengali    HL-C223   found by CI, 11 minutes after push
ɑ  U+0251  Hindi      here      found locally, BEFORE push
```

**The pattern is now unmistakable and worth stating as a rule.** Every instance is
a **phonetic character in a romanization**, reached for because the prose is
describing a sound English has no letter for. That is not an occasional slip; it
is what an etymology-and-pronunciation curriculum does several times per tranche.

Latin Modern is a typesetter's font: excellent Western European coverage, and it
holds almost none of the IPA block. Known present and safe: `ā ī ō ū ô ə`. Known
absent: `ǣ ɔ ɑ`, and by extension most of U+0250–U+02AF.

**The cheap habit that removes the whole class:** describe the sound in words
("the vowel of English *awe*") and romanise with a character the font has. The IPA
symbol buys precision the reader mostly cannot use and costs a CI round trip.

## HL-C222 — Bengali: same four ideas, different DEFAULT, and a letter that lies about its sound

Seventh script tranche. `neverTaughtGlyphs` 48 → 39; `tracksTeachingNothing` down
to **2**, from 8 where these tranches started.

HL-C218 established that the fourth idea is chosen per track. Bengali confirms the
prediction made there — its greeting carries a conjunct, so it takes Marathi's
order unchanged — and adds a dimension neither Gujarati nor Marathi needed.

**The inherent vowel is not the same in every abugida.** Devanagari's bare
consonant says *ka*; Bengali's says *kɔ*, the vowel of English *awe*. Same script
family, same four ideas, **different default** — and it is most of why Bengali does
not sound like Hindi read aloud. It has to be taught on the FIRST shape, because
every letter after it inherits the difference.

**And one letter is written *s* but said *sh*.** স descends from the Sanskrit *s*,
every transliteration writes it that way, and Bengali says *sh*. Worth teaching
early and plainly rather than letting a reader discover that the transliteration
they were given does not predict the sound.

### What to check before the next Indic tranche

The template now has three per-track questions, not one:

1. **Which fourth idea?** — read the track's first three words (HL-C218).
2. **What is the inherent vowel?** — do not assume *a* (this row).
3. **Which letters lie about their sound in transliteration?** — teach those early.

Hindi and Sanskrit remain, and both are Devanagari, so (1) and (2) are already
answered for them by HL-C216. (3) is not: Hindi drops the inherent vowel at the end
of a word — *nām* not *nāma* — which is the same class of defect between spelling
and sound and belongs in Hindi's first script chapter.

## HL-C221 — the literal-markup gate is BUILT, and what it cost to justify

HL-C217 specified it and logged it. HL-C219 then reintroduced the defect **one PR
after fixing it** — two lessons in the very next tranche carried `&nbsp;` again,
from a template that still had it, caught by hand only because somebody went
looking. That is the whole argument: **a mistake that survives its own fix by one
iteration will not be fixed by remembering harder.**

Built as `literal-markup.ts`, blocking, 11 tests. `npm test` now fails on any
HTML entity or bare HTML tag that would reach a reader.

**Three design choices worth keeping:**

1. **Two layers, two patterns.** The source scan finds `&nbsp;` in a `.md` and
   names the file an author edits. The rendered scan finds `\&nbsp;` in the
   generated `.tex` — a *different pattern*, because the escaper has been over it,
   which is precisely why grepping the books for `&nbsp;` found nothing for three
   releases.
2. **Exemptions are load-bearing.** `<!-- hl-knowledge -->` IS the corpus's
   directive syntax; code fences and inline spans are how a lesson (or this
   backlog entry) *quotes* markup rather than emitting it. Without those three
   exemptions the gate flags every lesson in the corpus and is switched off within
   a day. Exempt spans are blanked, not deleted, so line numbers survive.
3. **Blocking from commit one.** The corpus is clean at both layers today — 0
   findings across 2,885 lessons and every generated book — so there is no
   inherited debt for authors to route around. HL05's "report-only first" rule
   exists for gates that start red; this one starts green.

**Self-tested against a real planted defect, not fixtures.** `&nbsp;` was added to
a committed Spanish lesson; the gate failed naming `ES-C01-hola:68 &nbsp;`; the
file was restored and the gate went green. A fixture-only proof would not have
shown that the corpus assertion is wired to the corpus.

**The corpus assertion names its findings rather than counting them**, and a
companion test plants one line into the real lesson set to pin that the same
measurement still fires — so a clean run cannot be vacuous.

### The class this belongs to, which is bigger than one gate

Every other check in this package asks whether the output is **safe** or
**reproducible**. `check:books` confirmed the byte-identical `\&nbsp;` on every
run for three releases, faithfully reproducing the mistake. **Nothing asked
whether the output was MEANINGFUL.**

Other members of the same class, not yet covered and worth their own rows: a
Markdown link whose text and target disagree, a `[YOU SAY: …]` stage direction
with an empty payload, a table with a header and no rows, a cross-reference to a
lesson that exists but teaches something else. All render perfectly and all say
the wrong thing.

## HL-C220 — three Russian words were written with Latin letters, and the ъ ones are NOT the same class

Surfaced by the security review of HL-C219 as an informational note, then swept
across the whole track. Thirty mixed-script tokens; **they are two completely
different things and only one is a defect.**

**Genuine defects — three words, fixed here.** A Latin letter standing in for the
identical-looking Cyrillic one:

```
привet   -> привет     Latin e, t   (RU-C01-privet)
спасибo  -> спасибо    Latin o      (RU-C01-spasibo)
Kофе     -> Кофе       Latin K      (RU-C06-kofe)
```

These render across **two fonts** at exit 0, which is the HL-C202/HL-C203 defect
class exactly. Nothing catches them: the glyphs are all covered, the LaTeX is
valid, and the words look right on screen.

**NOT defects — the ъ transliterations, 20 occurrences, deliberately left.**
`azъ`, `govorъ`, `glazъ`, `rъtъ` are Latin romanisations of Old East Slavic that
keep the Cyrillic **hard sign** — the standard scholarly convention, and the whole
point of the etymology sections that use them. A homoglyph gate will flag all
twenty, and **it must not "fix" them.**

**And one that is neither:** `оs` in a narration file is English pluralising a
Cyrillic letter name — *"the two оs behave the way спасибо taught you"*. Unavoidable
if you pluralise a letter in English prose.

### What this means for the gate

A corpus-wide mixed-script detector is worth building — this is the third track to
turn one up — but **a bare "mixed script = defect" rule would be wrong on 21 of
these 30**. It needs at least:

- an allowlist for **transliteration conventions** (Latin + ъ/ь is the Slavic one;
  the Indic tracks will have their own), and
- an exemption for a **script name or letter name inflected in the surrounding
  language**.

A gate that cries wolf on legitimate scholarship gets suppressed, and then it
catches nothing. Recorded with the real numbers so whoever builds it starts from a
classified sample rather than from the raw count.

## HL-C219 — Cyrillic needs a different organising idea, and it is about the READER not the script

Sixth script tranche, and the first non-abugida. `neverTaughtGlyphs` 37 → 25.

Five tracks were taught as *inherent vowel → mātrā → position → (fourth idea)*.
**None of that exists in Cyrillic.** It is an alphabet: one letter, one sound, no
inherent vowel, no mātrās, no conjuncts.

What replaces it is not a fact about the script at all — it is a fact about **the
reader's Latin habits**:

- **true friends** — look Latin, sound Latin;
- **false friends** — look Latin, sound different;
- **new shapes** — no Latin relative at all.

Every Cyrillic letter sorts into one of those three, and sorting them is most of
what learning the alphabet is. Each lesson closes by asking which kind it was.

**Generalisation for the remaining non-Latin tracks:** the organising idea is
chosen from the **distance between the script and what the reader already reads**,
not from the script's own typology. Greek would want the same three kinds. Arabic
and Perso-Arabic want something else again — position-dependent letterforms, where
the same letter has four shapes — and Hebrew wants the abjad idea that vowels are
mostly absent. Do not reach for the abugida template outside the Indic tracks.

**Letters were chosen by how many lessons each unblocks**, computed from the
closure report rather than by alphabet order: у was blocking 41 lessons, з 27,
ы 25. That is the HL-C211 rule ("order by what it unlocks") made mechanical, and
it is worth doing for every remaining tranche — the report already has the data.

### The `delivery: script` adoption rule, which the manifest enforces and nothing documents

Adding marked lessons to Russian broke a test that had been green: **once a track
marks ANY writing lesson `delivery: script`, every `type: writing` lesson in that
track must be marked.** `RU-W01`–`RU-W05` predate the marker and were silently
exempt while the track had no marked lesson at all.

That is a good rule — a half-marked track would report script coverage it does not
have — but it means **the first marked lesson in a track is a bigger change than it
looks**, and it is worth knowing before the next track with legacy writing lessons.
Marking the five is also why `taughtGlyphs` jumped 18 → 30 rather than 18 → 29.

### I reintroduced the `&nbsp;` defect one PR after fixing it

HL-C217 removed literal `&nbsp;` from eight lessons across four tracks. **Two of
the eleven lessons in this tranche had it again**, because the generator template
I was working from still contained it and nothing checks.

Caught by hand, before push, only because I went looking. **This is the argument
for the literal-markup gate in HL-C217 being worth building rather than logged:**
a defect that survives its own fix by one PR is not going to be fixed by
remembering harder.

## HL-C218 — the four ideas are not a fixed list; the fourth one is whatever the track's first words need

Fifth script tranche. `neverTaughtGlyphs` 45 → 36; `tracksTeachingNothing` reaches
**3**, from 8 where these tranches started.

HL-C216 concluded the abugida shape "transferred without modification". Punjabi
shows the limit of that, and it is worth recording before Bengali, Hindi and
Sanskrit are done from the same template.

**Gujarati and Marathi taught inherent vowel → mātrā → virama → conjunct.** That
order worked because both greetings *contain* a conjunct — નમસ્તે and नमस्कार
each have one, so the virama lesson had somewhere to land.

**Punjabi's greeting has none.** ਨਮਸਤੇ is four plain consonants in a row. Teaching
the virama there would have introduced machinery with nothing to spend it on,
which is precisely the info-dump the whole curriculum is built to avoid.

What Gurmukhi's first words need instead is the **bindi**, the nasalisation dot —
both *yes* (ਹਾਂ) and *no* (ਨਹੀਂ) are unreadable without it. So the fourth slot took
that, and the chapter closes by **naming the conjunct as still to come** rather
than claiming the system is complete.

**The rule, restated:** three ideas are universal to the abugida — inherent vowel,
mātrā, and the fact that mātrā position is part of a character's identity. The
**fourth is chosen per track, from whatever its first three words actually
require.** Pick it by reading the target words, not by copying the previous track.

**Bengali, Hindi and Sanskrit:** check the greeting before writing the chapter.
Hindi's नमस्ते and Sanskrit's नमस्ते both carry a conjunct, so those two follow
Marathi. Bengali's নমস্কার does too. Punjabi is the exception, and knowing that
in advance is worth more than the tranche itself.

## HL-C217 — `&nbsp;` reaches the reader as literal text, and it shipped in three merged tranches

Found by the security review of HL-C216, filed by it as a **non-security note** —
which is the only reason it was caught at all.

Eight lessons across **four** tracks used `&nbsp;` to space out a display line of
separate glyphs. `&nbsp;` is HTML, not Markdown. The LaTeX escaper does exactly
its job and neutralises the `&`, so the generated book prints:

```
\gu{હા} \&nbsp;\&nbsp; \gu{ના}
```

A reader of the PDF sees the literal characters `&nbsp;`. Three of the four tracks
— chinese, japanese, gujarati — **were already merged**, so this was live in three
published books.

**Why every gate missed it.** It is not a schema error, not an untaught glyph, not
a LaTeX injection, not a font gap, and not a ramp violation. It is *correctly
escaped text that should never have been text*. The whole gate suite checks that
generated output is SAFE and REPRODUCIBLE; nothing checks that it is
**meaningful**. `check:books` even confirms the byte-identical `\&nbsp;` on every
run, because the generator is faithfully reproducing the mistake.

**Fixed** in all eight lessons by using plain spaces — deliberately not a Unicode
space or a middle dot, because a new codepoint is new font-coverage exposure and
HL-C214 is one PR old.

### The gap worth closing

A **literal-markup gate**: scan generated `.tex` for escaped sequences that are
almost certainly authoring mistakes rather than content — `\&nbsp;`, `\&amp;`,
`\&lt;`, `\&#\d+;`, a bare `<br>`, `\textbackslash{}n`. Every one of them means
the author typed markup for a renderer that is not the one running.

Cheap, report-only, and it generalises past this instance: the class is
"**author-facing markup that survived escaping into reader-facing text**", and the
corpus will keep producing it as long as lessons are written in Markdown and
rendered to LaTeX.

## HL-C216 — Marathi: the abugida shape replicates, and the cost of script is now visible

Fourth script tranche, and the first that was a **replication rather than a
design**. `neverTaughtGlyphs` 46 → 36; `tracksTeachingNothing` reaches **4**, from
8 where these tranches started.

**HL-C215's four ideas transferred without modification** — inherent vowel, mātrā,
virama, conjunct — because they are facts about the abugida, not about Gujarati.
Only the shapes and one signature changed: Devanagari hangs from the
**shirorekhā** and Gujarati erases it, and naming that contrast on the first shape
makes both scripts easier to place. **Punjabi, Bengali, Hindi and Sanskrit should
be replications too**, not fresh designs; the per-track work is choosing which ten
pieces unlock that track's first words.

**Authoring cost, measured across four tranches:** 10 lessons for 8 glyphs
(Japanese), 10 for 9 (Gujarati), 11 for 10 (Marathi). Roughly **1.1 lessons per
glyph**, which puts the corpus's remaining ~410 untaught glyphs at about **450
lessons**. That is the whole cost of gentle script introduction across sixteen
non-Latin tracks, and it is small beside ~10,000 vocabulary tranches.

### The cost that is NOT free, and it finally showed up in a number

`drivablePercent` fell **72 → 71**. Three script tranches in a row have added
pen-only lessons and nothing a commuter can do, and the corpus share has now
rounded down for the first time.

**This is the designed trade, not a regression** — HL08's whole point is that the
complete book keeps the handwriting and the driving edition filters it out — but it
is worth stating plainly rather than letting it drift: **every script tranche makes
the corpus less drivable**, and the remaining ~450 script lessons will move this
number several more points.

Two things follow. The driving edition stops being a nice-to-have and becomes the
thing that keeps the course usable on a commute while the script work lands. And
`unstartableChapters` is now the number to watch instead: a script chapter has no
drivable prefix at all, so it is 186 and climbing by one per tranche, by design.

## HL-C215 — Gujarati: nine pieces, and an abugida taught as four ideas

Third script tranche. `neverTaughtGlyphs` 41 → 32, `scriptLessons` 0 → 10, and
`tracksTeachingNothing` reaches **5** — down from 8 where these tranches started.

**The method that generalises to the other five Indic tracks.** An abugida is not
forty shapes to memorise, it is four ideas plus shapes, and each idea gets its own
lesson before any shape depends on it:

1. the **inherent vowel** — a bare consonant already says *a*;
2. the **mātrā** — a mark that REPLACES that vowel rather than adding one;
3. the **independent vowel** — the same sound, a different character, used only at
   the start of a word. This is the commonest spelling error in every Indic script
   and it is cheaper to teach as a contrast than to correct later;
4. the **virama** — deletes the inherent vowel, and the stripped consonant fuses
   with the next into a conjunct.

After those four, the chapter can say something that is both true and encouraging:
*the script has around forty more letters, and no more systems.* Marathi, Punjabi,
Bengali, Hindi and Sanskrit should reuse this shape rather than reinvent an order.

**Two readable words inside the chapter.** One consonant plus one mātrā makes
"yes" readable at lesson two; one more consonant makes "no" readable at lesson
four. Payoff before the halfway point is what keeps a script chapter from reading
as a chore, and it fell out of choosing the ORDER by what it unlocks rather than
by frequency — the same lesson HL-C211 recorded for Japanese.

### A debt assertion was pointing the wrong way, and this tranche exposed it

`script-closure.test.ts` asserted `tracksTeachingNothing` **> 5** — written when
the number was 8 and the point was "the debt is large". Three tranches later the
number is 5 and the assertion FAILED ON PROGRESS.

Converted to a **ceiling** (`<= 5`), on the same footing as the forward-reference
ratchet: it may fall, never grow, and whoever raises it writes down why. The
`violations > 500` assertion in the same test still carries the test's stated
point on its own.

**Worth a sweep.** Any assertion of the form "we have at least N problems" will do
this. It reads as a strong test and is actually a floor under the debt, failing
exactly when the work succeeds.

### Still open on this track, and not touched here

`headwordsWithoutRomanization: 26` — 26 Gujarati lessons show a headword in
Gujarati script with no romanization declared, so those headwords are LOAD-BEARING
rather than exposure. And every Gujarati lesson has `sequence: 0`, so the track has
no declared reading order at all and the continuity walk is measuring it in
filename order. Both are pre-existing, both are cheap, and both should land before
the next Gujarati script tranche rather than after.

## HL-C214 — nothing local opens a font, so glyph coverage is only ever found in CI

HL-C212 passed 744 local tests and a clean security review, then failed CI eleven
minutes later on one line: `latin missing_character rose to 2 against a baseline
of 0`. The cause was **ǣ** (U+01E3) and **Ǣ** (U+01E2) in an Old English citation.
Latin Modern does not have them.

**Every local gate reads the corpus. None of them opens a font.** That is the gap,
and it will catch every future tranche that reaches for an unusual character —
which an etymology-driven curriculum does constantly, by design.

**Two instruments were wrong on the way to the answer, both caught by testing
them against a known case:**

1. **The guess.** `ǵ` in *\*ǵʰórtos* looks far more exotic than `ǣ` and was the
   obvious suspect. Querying the actual `.otf` cmap shows **`ǵ` is present**.
   Rewriting that lesson would have left the failure in place.
2. **The wrong artifact.** Scanning the lesson SOURCES flagged four more
   characters — `ʰ`, `₁`, `₂`, `ḗ` — every one a false positive, because the
   generator turns them into `\textsuperscript{}` / `\textsubscript{}` and they
   never reach the `.tex`. **Check the generated `.tex`; it is what XeLaTeX
   reads.**

### The work item

A `check:glyphs` gate that, per track, resolves the fonts that track's book
actually loads and asserts every non-ASCII character in its generated `.tex` is in
one of them. Report-only first, per the HL05 precedent.

**It is not free, and that is why this is a row rather than a commit.** Font
resolution differs per track: the Indic, Arabic and CJK books load vendored Noto
faces from `_fonts/`, which the repo owns and can read directly, while the
Latin-script books load **Latin Modern from the TeX Live installation** — a path
that exists on CI and on a developer machine with TeX Live, and nowhere else. The
gate has to degrade honestly when a font cannot be resolved (report "unmeasured",
never "clean"), which is the same distinction `script-closure.ts` already draws
for an unknown script.

**Until it exists:** any tranche introducing a character outside plain ASCII plus
macrons should run the fontTools cmap check by hand, self-tested in both
directions. Recorded in `lessons.md` with the snippet.

## HL-C213 — a late vocabulary lesson RETROACTIVELY creates forward references, and the fix is placement

Found by the Latin tranche, and it will recur on every track that adds basic
vocabulary at the end of a long book. Recorded because the number it moves is a
CEILING that may only be raised with a written cause.

Twenty new pre-A1 words in chapters 44-47 pushed `forwardReferences` 500 → 508.
**None of the eight is a new use.** Every one is a sentence that has been sitting
in an untouched lesson for months, and became measurable only because the word
finally got an owner:

```
LA-C08-manus              domus    71 lessons early   lists domus among feminine -us nouns
LA-C06-ruber-caeruleus    mare     84                 glosses caeruleum mare
LA-C27-bene               mel, terra  61, 50          cites mel->miel, terra->tierra
LA-C40-dormio             somnus   28                 names somnus while teaching dormire
LA-C41-ambulo             somnus   25                 somnambulist walks in sleep
LA-C28-dies               Lunae    57                 prints dies Lunae
LA-C24-propediem-te-videbo Lunae   61                 prints dies Lunae
```

This is the `LA-C08-manus` / `LA-C37-habeo` class the ceiling's own annotation
already names. **It was verified rather than assumed**: each of the eight was read
in its own file and confirmed unmodified by `git diff HEAD`, because "the
measurement got sharper" is exactly the excuse that would cover real decay if it
were taken on trust.

**A ninth WAS real and was fixed in content.** `LA-C45-terra` named *mare* four
lessons before it is taught, to build *mare mediterrāneum*. It now describes the
name in English and lets the *mare* lesson supply the Latin. That is the correct
disposition — the ceiling absorbs measurement artefacts, never new debt.

### The finding, which is bigger than this tranche

**Words used 25 to 84 lessons before they are taught are not in the right place.**
*Domus*, *mare*, *somnus* and *lūna* are pre-A1 basics that the Latin book has
been leaning on since chapter 6. Appending them at chapter 44 makes them
measurable and leaves them misplaced.

The real fix is a **placement pass**: these chapters belong early in the book, and
so do the equivalents on every other track. That is a renumbering of 47 chapters
and every cross-reference into them — its own change, with its own verification,
and explicitly NOT something to bury in a content PR. HL-C156 and HL-C137 reached
the same conclusion from different directions; this is the third instance, which
is enough to say it is structural.

**Consequence for the queue.** `completion-plan.ts` ranks a vocabulary item by
headword deficit alone. It cannot see that adding words at the end of a long track
is worth less than adding them in the right place, and it will keep recommending
the cheap version. Recorded so the placement work gets scheduled deliberately
rather than waiting for the plan to ask for it.

## HL-C212 — Latin: twenty pre-A1 nouns, chosen to prove cousinhood

Latin pre-A1 vocabulary **23 → 43** of 300. Four themed chapters, five words
each, one word per lesson per HL14.

**Selected for what they demonstrate, not for frequency.** This track's whole
purpose is showing an English reader where their own words come from, so every
chapter carries at least one genuine *cousin* beside its loans — *hortus* against
**yard**/**garden**, *ventus* against **wind**, *stēlla* against **star**, *ōvum*
against **egg**, and *piscis* against **fish**, which is where Grimm's Law is
finally named after three chapters of quietly showing its results.

**Two claims are hedged, and stay hedged.** The *salārium* salt story is Pliny's
account and no document records soldiers paid in salt; *caelum* → **ceiling** is
probable, not settled. A track whose selling point is etymology cannot afford to
launder a good story into a fact.

**Two gate catches worth carrying forward:**

1. *"the rule that holds this whole curriculum together"* failed
   `standalone-book`. A reader holding only the Latin PDF has no other volume, so
   nothing may point outside it. **Write for the book in the reader's hands.**
2. *"as it almost always is"* tripped the info-dump `always is` rule-statement
   pattern. A false positive, rephrased anyway.

## HL-C211 — Japanese: eight hiragana, and the sign taught late on purpose

Second script tranche off the computed queue. Ten lessons, eight signs, two
assemblies that introduce nothing. `neverTaughtGlyphs` 43 → 35;
`tracksTeachingNoScript` 8 → **6** across the two tranches so far.

**The ordering decision worth copying.** The sign for *wa* is taught LAST in the
tranche, after the greeting is already known. The daytime greeting sounds like it
ends in *wa* and is written with the sign read *ha*, because that sign doubles as
the topic marker. Teaching the *wa* sign at its alphabetical or frequency position
would have handed the reader the commonest beginner spelling error in the language
and then corrected it later. **Glyph order is a pedagogical choice, not a
frequency sort** — and the letter-ledger work for the Indic tracks should be read
with that in mind.

**What the Chinese tranche predicted, and what happened instead.** HL-C210 warned
that closing `neverTaughtGlyphs` leaves `violations` untouched when the teaching
chapter sits after the using chapter. That held: Japanese `violations` went 8 → 8.
The ordering half of the problem is unchanged and is still HL-C210's.

**Japanese chapter 1 is at 18 atoms against a budget of 12, and was already.** Not
introduced here — the ramp report carried it before this tranche and the pin did
not move. Recorded because the Chinese tranche hit the same number for a different
reason (there it WAS newly introduced, and the fix was a second chapter), and the
two should not be confused when someone comes to burn this debt down.

**Scale check for the remaining script work.** This tranche cost ten lessons for
eight glyphs. At that density the corpus's remaining 452 untaught glyphs are
roughly 560 lessons — which is the honest figure for "introduce the script gently
across sixteen non-Latin tracks", and it is small beside the ~10,000 vocabulary
tranches. Script really is the finite half.

## HL-C210 — teaching a glyph and teaching it FIRST are two different fixes

Found while closing Chinese's script debt, and it generalises to all sixteen
non-Latin tracks, so it is recorded before the next one is attempted.

`script-closure.ts` reports two numbers and they move independently:

| number | what it means | Chinese, after HL-C209 |
|---|---|---:|
| `neverTaughtGlyphs` | shown somewhere, taught nowhere | 7 -> **0** |
| `violations` | a lesson asks for a glyph not yet taught **at that point** | 4 -> **4** |

Authoring seven component lessons drove the first to zero and left the second
untouched, because the lessons that USE those glyphs sit in Chapter 1 and the
chapter that TEACHES them is Chapter 2. In reading order nothing changed.

**This is not a defect in the new lessons.** It is a second, pre-existing defect
that the first fix makes visible: Chapter 1's decomposition prose displays 亻, 尔,
女 and 子 as load-bearing text while teaching by ear. Its own headwords are exempt
(they carry romanization); the components in the body are not.

Two ways to close it, and the choice is a real one:

1. **Trim Chapter 1's prose** so it names the components without displaying them
   until Chapter 2 has taught them. Keeps HL11's ordering (useful by ear from
   page 1, script drizzles in behind) and costs some of the track's best writing.
2. **Put the script chapter first.** Satisfies closure outright and inverts HL11's
   stated ordering for this track — defensible only if Chinese is genuinely the
   case where decoding IS the ramp, which HL12 leaves open.

**Do not pick one from the armchair.** The measurement that decides it is
drivability: option 2 makes the track's opening chapter entirely pen-and-eye,
which for a commute-first course is a real cost and is already visible — this
tranche alone moved corpus `drivablePercent` 88 -> 87.

**The generalisation, which is the reason this row exists:** every one of the
eight tracks that teaches no letters at all will hit this the moment it stops.
Closing `neverTaughtGlyphs` is the cheap half. Ordering is the expensive half, and
the plan's `script-closure` family currently measures only the cheap one — see
`completion-plan.ts`, where `outstanding` is deliberately `neverTaughtGlyphs`
rather than `violations` because deleting a lesson would otherwise count as
progress. That choice is still right, and it means the queue will under-report
this family until ordering is measured separately.

## HL-C209 — Chinese teaches its script; one glyph per lesson, two of them assemblies

Seven writing lessons, Chapter 2. 人 → 亻 → 尔 → 你, then 女 → 子 → 好. Five teach a
component; **two introduce no new stroke at all** and instead show what two known
pieces do when they share a square — the composition lesson HL14 §1.1 records as
missing from every track.

`tracksTeachingNothing` 8 → 7. Chinese was one of eight tracks showing a reader a
writing system while teaching no letter of it.

**A new chapter, not a longer one.** Seven atoms into Chapter 1 took it to 18
against a budget of 12. HL-C167 says change the content rather than re-seat the
number, so the content changed shape: Chapter 1 stays the spoken greeting and is
drivable end to end, Chapter 2 is the pen.

### Three authoring traps, all caught by gates rather than by reading

1. **A citation carried its Chinese title.** *Xiandai Hanyu Tongyongzi Bishun
   Guifan* was written with its 现代汉语通用字笔顺规范 form alongside — **eleven
   untaught glyphs, in a lesson whose entire job is to teach one.** The script
   ramp caught it at 12 new glyphs against a budget of 3. HL-C191 already required
   romanised cross-script citations; this is the first time the cost was visible.
   **Check the SOURCE LINE, not only the prose.**
2. **"if you are used to an alphabet"** matched the info-dump `is/are used to`
   rule-statement pattern. A false positive — and it was still rephrased, because
   arguing with a gate inside the prose is how the prose gets worse.
3. **A chapter title carrying target script** drifts against the generated
   `\zh{...}` wrapper and reads as `titleDrift: 1`.

### And one that was NOT a trap

The chapter-2 payoff first assessed 3 of the chapter's 7 atoms — below the 0.5
representativeness floor. The fix was **not** to widen the declaration to hit the
number: the lesson genuinely retrieves the earlier assembly ("the first one you
built had a meaning half and a sound half"), so the atom was declared because it
is exercised. Widening a declaration to clear a floor is how a floor stops
measuring anything.

## HL-C208 — the backlog is now a function; and the head of it is SCRIPT, not vocabulary

The prioritization header on this file was three days stale on the day it was read:
it ordered work against the frame HL-C183/HL-C184 had already replaced. Nobody was
careless. That is what happens when the ordering lives in prose that nothing
recomputes, while every number needed to derive it is computed on every run.

So it is derived now (`completion-plan.ts`, `npm run plan`, spec HL15). Two things
the first build of it got wrong are worth recording, because both looked right:

**1. A flat sort by family produced a useless queue.** Level rank, then family
priority, then cost — each key defensible, and the head came out as twenty-two
consecutive `exam-inventory` items. Every track's research task ahead of every
track's content, because all 22 tracks sit on the same rung and that family is
priority 1. No language would have moved until all of them had moved on one axis.
**Fixed by rotating across tracks**: each language contributes its single most
important next action before any language contributes a second.

**2. Lookahead outranked the floor.** `pre-A1` is not a CEFR level and no awarding
body publishes an inventory for it, so a track at pre-A1 generates an inventory item
for **A1** — the rung above. Family priority then put that ahead of the pre-A1
content, which is exactly backwards: it is preparation for a climb that has not
started. An inventory for a level the track has not reached now sorts last within
that track, and jumps back to first the moment the track stands on that rung.

### What the corrected queue actually says

The head is **not** vocabulary. It is **script**, in fourteen of the first
twenty-two slots:

```
459 glyphs shown but never taught, corpus-wide, across 16 non-Latin tracks
756 lessons ask a reader to decode a glyph nobody taught them
8 tracks teach NO letters at all
```

This was already in the report and had never been ordered against anything. It
outranks the vocabulary grind for a reason that survives argument: it is the only
family with a terminal state. Tamil has 247 glyphs and then it is done forever;
vocabulary runs to 16,000 per track. A vocabulary tranche authored into an unclosed
script is authored onto sand, because the reader cannot decode the word it teaches.
The owner asked for gentle script introduction on 2026-08-17 independently, which
agrees with the measurement rather than overriding it.

### The size of the thing, stated plainly

**~10,172 items to C2 across 22 tracks**, of which 89 are enumerable today. 9,995
are vocabulary tranches, 131 are exam inventories, 46 are script tranches. The other
four families are not projectable and are reported as `null` rather than as zero,
because "cannot be projected" and "nothing left to do" are opposite facts.

**1 of 132 exam inventories exists** (Spanish A1). Until a track's inventory is
written, every number this repository reports for that track at that level is a
proxy for something nobody is graded on — which is HL-C184's Phase 0 restated, now
with the item queued per track instead of listed once and forgotten.

## HL-C207 — Hindi round two landed; Tamil and Malayalam are now the only tracks a round behind

Hindi chapters 52-58, 35 headwords on pre-A1 nodes, **85/300 -> 120/300**. Hindi now
leads the corpus, ahead of Spanish's 118. R1 0.2818 -> 0.2793 with the numerator held
at 1106; hindi `forwardReferences` holds at 11; all 35 lessons are `voice`, so the
whole wave is drivable.

**The queue after this tranche**, cheapest first:

```
tamil 84/300   malayalam 84/300      <- a round behind, and the next two tranches
sanskrit 104   telugu 116   kannada 117   spanish 118   hindi 120
```

Tamil and Malayalam are the only two still carrying a round-one number. Bringing
them up is the obvious next work, after which all seven sit within ~16 of each
other and round three can start from a level field.

### Two things this tranche learned that the next one should not relearn

**1. Do not write "chapter N" in lesson prose — the gate forbids the reference, not
the staleness.** Seven of the new lessons pointed back by number, and every number
was verified against the corpus before it was written. All seven still failed
`chapter-references.test.ts`, which pins cross-chapter references per track (hindi
at 20) because a number correct today goes stale at the next chapter split, silently.
Verifying them made them accurate and left the debt exactly as toxic. Name the thing
instead — "when the ear was named", "at the end of the welcome chapter". Recorded in
`lessons.md` too, because the careful-looking version of the habit is the trap.

**2. The Devanagari screen discards far less than the Spanish one, and that is a
property of the script, not of the screening.** 64 candidates screened, **1**
discarded, against Spanish round two's 35 of 146. A Latin-script headword hides
inside longer words constantly (*aam* inside *naam*, *anda* inside
`standard-colloquial` in frontmatter); a Devanagari one does not, because an
independent vowel (आ) and a vowel sign (ा) are different code points. **Screen on the
Devanagari headword against lesson BODIES, which is what `taughtWords` actually
indexes — not on the romanization, and not on raw substrings.** A romanization
screen over whole files reported 15 of 64 dirty; 14 of those were phantoms.

**A control that is itself wrong looks exactly like a broken instrument.** The
whole-word screen reported छत clean while a raw-substring pass flagged it in
`HI-C35-puchna.md`. The screen was right: छत occurs there only inside पूछते and
पृच्छति. Per HL-C203's rule, the dirty control was checked before the instrument was
blamed — and this time the control was the thing at fault.

## HL-C205 — the gap report's cost model, and why its test timeout is not a dial

Recorded because three separate sessions raised the same timeout and none of them
wrote down the growth rate, so each one rediscovered the problem from scratch.

`tests/cli.test.ts` builds the whole curriculum gap report twice. Its budget went
5s -> 20s (at 1,249 lessons) -> 35s (at 1,878), and at 2,771 it failed CI at 35s.
The cause was not corpus size as such: `measureContinuity`'s forward-reference walk
was **quadratic in track length**, asking each of Spanish's 549 lessons about each of
its ~550 taught words, and building a fresh `(?<![\p{L}\p{M}-])...` regex per pair.
Measured on this corpus, one such regex costs ~162µs to construct and ~168µs on its
first `.test()`, because `\p{L}` and `\p{M}` expand into large code-point tables.

Fixed by indexing candidates on their leading word-run, so a lesson is only asked
about words its own text can reach. `measureContinuity` 2,065ms -> 218ms; the whole
`--format json` run 3.94s -> 1.79s; both report formats byte-identical.

**What to expect now, and the number to check it against.** The remaining cost is
linear: ~0.65ms per lesson per build (1.79s / 2,771 lessons, two-thirds of it
reading, parsing and hashing the files). At that rate 35s covers roughly three times
today's corpus. So a future slow run on this test is evidence that something has gone
superlinear AGAIN, not evidence that the budget is stale — profile it before touching
the number, and never thin the report to fit the clock.

## HL-C196 — `repin_tests.py` oscillates on a field name shared with a synthetic fixture

Found by the Spanish tranche. `drivablePercent` appears twice in
`modality-manifest.test.ts`: line 359 is a **hand-built 3-lesson unit fixture**
(2 of 3 drivable = 67) and line ~998 is the **corpus** pin. The patcher matches on
field NAME, so it rewrote the synthetic fixture to the corpus value, which broke
the unit test, then flipped the corpus pin back — for 60 rounds, appending its
annotation tag **57 times** to one line before bailing.

The script's `MAX_ROUNDS` backstop worked; the damage was cosmetic and was undone
by hand. But it will recur on any tranche that moves `drivablePercent`.

**Fix:** the object-pin branch must confine its search to the failing assertion's
own block, not the whole file — take the line number vitest reports and only
rewrite fields within that `expect(...)` call. A `key: value` search across the
file is wrong wherever a test holds both a fixture and a corpus pin for the same
field, and `drivablePercent` will not be the only one.

Until it is fixed: when the script reports it did not converge, check whether it
edited a fixture, and look for a repeated annotation tag on a single line.

## HL-C204 — Spanish chapters 8 and 9 share a LaTeX label

`spanish/book/build.sh` emits `Label 'ch:how-are-you' multiply defined`. Chapters 8
and 9 both claim it. **Pre-existing at HEAD**, unrelated to any tranche, and it has
been surviving every build because it is a WARNING, not an error -- the build exits
0 and the PDF is produced.

What it costs: `\ref`/`\hyperref` to that label resolves to whichever chapter LaTeX
saw last, so a cross-reference silently points at the wrong chapter. Same family as
the rest of this session's findings -- **renders fine, says the wrong thing.**

Left out of the Spanish round-2 tranche deliberately: relabelling ripples into every
cross-reference that targets it, which is its own change with its own verification,
not something to bury in a vocabulary PR.

## HL-C203 — corpus-wide sweep: 31 mixed-script findings, and a THIRD blind tool

The Kannada round-2 audit ran HL-C202's per-word check over the WHOLE corpus, not
just its own track. **31 pre-existing findings.** One was in kannada and is fixed
(`kannada/roadmap.md:88` -- a Tamil word wearing U+0CB0 KANNADA LETTER RA
mid-word, rendering `\ta{}\kn{}\ta{}` across two fonts at exit 0). The rest
belong to other tracks and were deliberately left for their own change:

```
malayalam/roadmap.md:87            Tamil  wearing U+0D41 MALAYALAM VOWEL SIGN U
malayalam/ML-C37-mookku.md + ch37  Kannada  wearing U+0D41
malayalam/ML-C11-nirangal.md+ch11  mixing Malayalam and Tamil letters
telugu/TE-C37-noru.md + ch37       Latin v + TAMIL VOWEL SIGN AA + y  (sub-class b)
telugu/chapters.json:770           Latin v + TELUGU VOWEL SIGN AA + y
arabic/AR-C16-al-saa.md + ch16     Aramaic in ARABIC letters wearing HEBREW points
ZWNJ U+200C                        telugu ch05 (x3), persian CHANGELOG + roadmap
```

**The Arabic/Hebrew one widens the class beyond Indic.** Two right-to-left scripts
mixing inside one word is the same defect and the same silent render. Any detector
must be script-general, not a hardcoded Indic list.

**Note telugu/chapters.json:770.** Telugu's own round 2 fixed two TE-C33 lessons
and MISSED this one, because it audited its new files and the two it had touched
rather than the whole track. **Sweep the track, not the diff.**

### The third blind tool this session

The audit's first version used `\w` to find words. `\w` is isalnum()-based, so it
**stopped at every combining vowel sign -- exactly the character being inspected.**
It reported clean. Fixed by walking Unicode categories L/Mn/Mc/Me instead.

That is three in one session, all the same shape -- **the instrument silently
altered or excluded what it was measuring**:

1. `grep $'\x00'` is an EMPTY pattern in the shell; it flagged all 71 files as
   NUL-corrupted, including ones where only integers had changed.
2. A shell pipeline NORMALISED the very codepoints being decomposed and reported
   defective strings clean.
3. `\w` EXCLUDED combining marks, so the vowel-sign audit could not see vowel signs.

**Rule: before trusting a check that reports clean, run it against a case known to
be dirty.** All three were caught that way and none would have been caught by
reading the code.

**A fourth instance proved the rule within the hour.** The independent security
review of this same PR built its own detector, ran it against ten known-dirty
controls, and found it reported ALL TEN clean -- a shell heredoc had mangled the
literals in transit. It rebuilt the harness and only then trusted the result. That
is the procedure working exactly as intended, and it is why the self-test is not
optional: the detector looked correct in every case, and was wrong in all four.

## HL-C206 — the HL11 ductus/filmstrip work is UNSTARTED, and was only in a session task list

Four items sat in an in-session task list rather than here, and were closed out
when that list was cleared. The design is specified in
`code/specs/HL11-drizzled-script-ramp.md` and the corpus mentions ductus/stroke
order in ~95 places, but **nothing recorded that these four remain undone**, so
this row exists to say so plainly:

1. **`letter-ductus` figure kind** -- one letter to one filmstrip: n panels, panel k
   showing strokes 1..k, finished outline behind in grey, travelled path in ink, a
   dot at the pen, one caption per panel. Goes in
   `human-language-data/src/figure.ts` beside `etymology-route`, rasterised through
   `image-codec-png`, byte-gated by `core/generated-figure-hashes.json`.
2. **Wire those figures into the book pipeline** -- declare them in
   `core/figure-generation.json` and prove it on Tamil's 11 already-verified letters
   BEFORE any new research lands.
3. **Cited ductus research for five scripts**, in letter-ledger order. Tamil extends
   from 11; Telugu, Kannada, Malayalam and Devanagari start at zero. Per letter: a
   cited `strokeOrderSource`, a font-verified pen path, and `penLifts` (which the
   schema already refuses without a source). **No citation means no pen path and no
   figure** -- the letter ships prose-only and the gap is recorded as debt.
4. **Rebuild the six tracks in lockstep** -- redistribute the clustered writing
   lessons into one-letter segments across the first ~50 sequences.

**Sourcing risk, stated plainly and unchanged:** Tamil's source is solid and Telugu
has a plausible academic candidate. Kannada, Malayalam and Devanagari are unproven,
and recorded project memory says Indic stroke GEOMETRY is not transferable between
scripts -- only order and count are reliably citable. Expect some letters to land
prose-only. That is the designed outcome, not a failure.

**Do not let a missing citation become an invented pen path.** The whole point of
the `strokeOrderSource` gate is that a hand-drawn guess looks exactly like a
researched one on the page.

## HL-C202 — a COVERED script can still split a word across two fonts

Found while auditing the Telugu round-2 tranche. Four committed strings carried a
**foreign vowel sign on a cousin-script base**:

```
TE-C33-caduvu.md   Tamil படி  wearing U+0C41 TELUGU VOWEL SIGN I
TE-C33-raayu.md    Tamil எழுது wearing U+0C41 TELUGU VOWEL SIGN U
telugu/CHANGELOG.md x2   Kannada  and  wearing the Telugu sign
```

The rendered output proves these were live, not cosmetic. The renderer emitted

    \ta{}\te{}

-- a Tamil word wearing a Telugu vowel mark from a different font, mid-word.

**Why every existing guard missed it.** HL-C200 is about a script being
UNCOVERED, so its glyphs fall to the Latin font and vanish. This is the opposite:
both scripts are covered, both fonts load, nothing is missing, and the build exits
0 with 0 missing characters. The page renders -- as the wrong glyph. It is the
same family as the Malayalam chillu and the Kannada TTA-for-DA slip: **plausible
wrong text, which no build can detect.**

**The check that finds it** is per-WORD script purity: decompose each run into
Unicode letter names and flag any run whose characters name two different scripts.
Per-file checks pass; per-word checks do not.

**Read the bytes, not a pasted string.** An earlier attempt to decompose these same
strings through a shell pipeline reported them CLEAN -- the shell normalised the
characters in transit. Only opening the file and reading its bytes found the
defect. Same failure shape as the empty-pattern NUL grep: the tool being used to
look was quietly altering what it looked at.

**A second sub-class: a LATIN base carrying an Indic combining mark.** The security
review of the same PR found five more, all the mirror image of the first four --
not an Indic word wearing a foreign sign, but a ROMANIZATION wearing one:

```
telugu/chapters.json ch37 summary, narration/ch37.{txt,json}, lessons/TE-C37-noru.md
    U+0076 U+0BBE U+0079   =  v + TAMIL VOWEL SIGN AA + y
    almost certainly meant vay with U+0101 (a macron)
```

So the per-word purity check must also flag **any Latin run containing a combining
mark from an Indic block**, not only Indic runs mixing two Indic scripts. Both
sub-classes render as plausible wrong text and both build at exit 0.

**Still outstanding, deliberately left for their own change:** the same class exists
in `malayalam/lessons/ML-C11-nirangal.md` and `ML-C37-mookku.md` plus two roadmap
files. They belong to another track's generated output. The ~30 Devanagari-danda
hits in punjabi and bengali are conventional usage, NOT defects -- do not "fix"
them.

## HL-C201 — appending a chapter can make the PREVIOUS last lesson untrue

Found in the Sanskrit round-2 tranche. `SA-C30-anjali` was the final lesson of the
book and said so, twice: *"before the last word of this book"* and *"the book ends
where it began."* Both were true when written and both became **false the moment
chapter 31 existed.** Nothing failed. No test checks whether a lesson's prose still
describes the book it is now in.

This is not a Sanskrit problem. **Every tranche in this rotation appends after the
current last lesson**, so every tranche can silently falsify it. Seven tranches
have already landed this way and only this one was checked.

**The class is wider than "last".** Any prose asserting a position -- first, last,
only, "so far", "the final chapter", "nothing after this" -- is a claim about the
corpus, not about the language, and the corpus moves underneath it.

**Immediate action:** before appending, READ the current last lesson of the track
and reword any finality claim. Costs one file read per tranche.

**Real fix:** a detector for position-asserting prose, checked against the lesson's
actual position -- the same shape as the existing chapter-number-in-prose gate,
which catches "chapter 16" but not "the last chapter". Worth building because the
failure is invisible: the page still renders, the build still exits 0, and the
sentence is simply a lie to the reader.

**Audit DONE, and it came back clean.** The previously-final lessons of the six
other tracks -- ES-C281-sucio, TA-C43-family, TE-C45-wife, KA-C45-wife,
ML-C45-wife, HI-C44-people -- were each appended after without this check, so all
six were re-read. **None makes a finality claim.** Sanskrit's was the only one.

Worth recording for whoever builds the detector: a first-pass regex flagged three
of the six, and **all three were false positives.** Spanish, Telugu and Malayalam
each say *"the last word"* meaning THE PREVIOUS LESSON'S HEADWORD, which stays
true forever. So the detector cannot key on the phrase alone -- it has to
distinguish a claim about the BOOK's last word from a reference to the PRECEDING
word, and those read almost identically. A naive rule here would fire on the
common case and be turned off.

## HL-C200 — nine chapters diverge from their own track's script convention

Found while verifying the Telugu tranche. A book target declares its script one of
two ways: `scriptSet` (a named set that also loads the COUSIN scripts a comparison
table cites) or a bare `unicodeScript` + `scriptCommand` pair (target script only).

Census of `core/book-generation.json`:

```
track        scriptSet  unicodeScript
kannada             47              0   <-- FIXED; ch43-45 switched, ch46-52 born correct
malayalam           47              0   <-- FIXED; ch43-45 switched, ch46-52 born correct
telugu              44              3   <-- chapters 43, 44, 45
```

Every other track is internally consistent. Sanskrit is all-`unicodeScript`, which
is correct -- no sanskrit scriptSet exists.

**These nine are mine**, from the HL-C190 see/say and HL-C192 family-word tranches:
I copied a sibling track's entry shape instead of the entry directly above in the
same track, which is precisely what HL-C188 was written to prevent.

**Why it is worth fixing even though the books build clean today.** A bare
`unicodeScript` leaves cousin-script characters in the Latin font, where they
render as NOTHING and the build still exits 0. The Telugu tranche hit exactly this
-- 89 missing characters at exit 0 -- and it was caught only because someone read
the missing-character count rather than the exit status. Chapters 43-45 pass today
because they happen to cite no cousin script (HL-C191 romanised those citations).
They are one comparison table away from dropping glyphs silently.

**Fix:** switch the nine targets to their track's scriptSet
(`kannada-comparisons`, `malayalam-comparisons`, `telugu-comparisons`), regenerate
those nine chapters and their book hashes, and confirm missing-characters stays 0.
Kept out of the Telugu tranche PR deliberately: it touches three other tracks'
generated output and belongs in its own change.

**Kannada's three are done**, folded into the chapters 46-52 vocabulary tranche
because that change already owned the kannada book. The rendered .tex for 43-45
is byte-identical afterwards and their book hashes did not move, which is the
predicted result: those chapters cite no cousin script today, so the bare
`unicodeScript` was a latent trap rather than a live defect.

**Malayalam's three are done too**, folded into that track's own chapters 46-52
vocabulary tranche for the same reason. The result reproduced exactly: the
rendered .tex for 43-45 is byte-identical, `generated-book-hashes.json` gains 91
lines and loses none, and the book still builds at 0 missing characters, 0
overfull and 0 underfull over 305 pages.

**Telugu's three are done, and that closes HL-C200.** Folded into the telugu
chapters 53-59 vocabulary tranche, for the third time with the same result: the
rendered .tex for 43-45 is byte-identical, `generated-book-hashes.json` gains
lines and loses none, and the telugu book still builds at 0 missing characters,
0 overfull and 0 underfull. All nine chapters now declare their track's
`*-comparisons` set, and no track that has such a set carries a bare
`unicodeScript` target any more. The bare targets that remain in the corpus
(bengali, chinese, gujarati, marathi, persian, portuguese, punjabi, russian,
sanskrit, spanish ch139, urdu) are in tracks with no `*-comparisons` set to move
to, which is the internally-consistent case this entry never counted.

One thing the kannada pass added to the diagnosis: the trap is not only cousin
SCRIPTS. `KA-C47-finger` cited Malayalam using the old chillu spelling
`ല` + virama + ZWJ, and U+200D is in no script block at all, so no scriptSet can
catch it — it went to the Latin font and printed one missing character at exit 0.
The fix is the atomic chillu (U+0D7D). Any Malayalam citation written the old way
will do the same thing in any track.

The malayalam tranche was the highest-risk place for that trap, since every
chillu in it is a headword letter rather than a citation, and it was written
against the hazard from the start: all four chillu characters used across the
thirty-five lessons (U+0D7B, U+0D7C, U+0D7D, U+0D7E) are atomic, and a scan of
the new lessons and the changelog finds zero U+200D and zero U+200C.

## HL-C199 — only ONE track has a book/build.sh; the other 21 have none

The per-track content gate this project keeps citing --

    code/learning/human-languages/<track>/book/build.sh ; echo "exit=$?"

-- exists for **spanish only**. `ls */book/build.sh` returns exactly one path.
Every instruction that told an author to run it for tamil, sanskrit or any other
track named a file that is not there.

Why it matters: a missing script fails with a shell error, which is easy to read
as "the build is broken" or, worse, to skip. The Sanskrit tranche was verified by
running what spanish/book/build.sh runs, by hand, from the track's book dir:

    latexmk -xelatex -interaction=nonstopmode -halt-on-error book.tex

That is correct but it is not discoverable, and it means the exit-code discipline
(a failed build prints ZERO warnings and reads exactly like a clean one) rests on
each author reconstructing the command.

**Fix:** give every track the same build.sh, or replace all 22 with one
`build.sh <track>` at the corpus root. Prefer the latter -- one file to keep
correct rather than 22 copies that will drift.

## HL-C198 — ANSWERED: a lesson's spine NODE and its POSITION are independent

HL-C197 left the next tranche blocked on a design question: pre-A1 spine nodes
live early in the book, but HL-C192's chaining rule appends new chapters at the
end. Could a lesson appended last still declare a pre-A1 node?

**Yes — and the corpus already proves it, so no new mechanism is needed:**

```
spanish : 81 pre-A1 lessons, earliest ch1, LATEST ch144  (book runs to ch281)
hindi   : 62 pre-A1 lessons, earliest ch1, LATEST  ch39  (book runs to  ch44)
```

`ES-C55-la-bebo` sits past the halfway point of the Spanish book on a pre-A1
node, and validates clean. **Position in the book and level on the spine are
orthogonal.** The gate counts the node; the reader meets it wherever it is placed.

**So the next tranche is unblocked, and its shape is fixed:**

1. Append the chapter at the end, chained to the previous chapter's last lesson
   (HL-C192 — this is what keeps R1 falling rather than rising).
2. Declare a **pre-A1 spine node** on every lesson in it.
3. Choose words whose prerequisites are already met, so the late position costs
   nothing — which everyday concrete vocabulary satisfies by construction.

That combination satisfies the chaining rule and the pre-A1 criterion at once. It
is what the last two tranches should have done: they were good words on A1 nodes
and moved the blocker by zero.

**Target for the next tranches**, cheapest first, on the corrected numbers:
sanskrit 34/300 · spanish 48/300 · tamil 49 · malayalam 49 · hindi 50 ·
kannada and telugu to be measured. Sanskrit leads the queue.

## HL-C197 — the corrected number changes the plan: every track is ~1/6 of pre-A1, and Hindi leads

HL-C195's fix landed, and the picture it reveals is not the one the old line
implied:

```
spanish 48/300 at-or-below pre-A1 (227 total)   hindi 50/300 (107 total)
malayalam 49/300 (107)   tamil 49/300 (103)     sanskrit 34/300 (100)
```

**Hindi is AHEAD of Spanish on the criterion that counts** — 50 against 48 —
despite having under half the total vocabulary. Spanish's 227 is real teaching,
but most of it sits on A1-and-above nodes and does nothing for the pre-A1 gate.

Three consequences for the plan:

1. **The tracks are far closer together than the totals suggested.** "Spanish is
   ahead, the Indic six are behind" was an artefact of the wrong number. On the
   criterion, all six sit between 34 and 50 out of 300.
2. **Sanskrit is the real laggard** at 34, not because it has fewest words but
   because fewest of them are on pre-A1 nodes.
3. **Authoring must target pre-A1 SPINE NODES**, not just add words. That is in
   tension with HL-C192's chaining rule (append to the end, chain to the previous
   chapter's last lesson), because pre-A1 nodes live early in the book. Resolving
   that tension is the next design question, and it blocks efficient authoring:
   until it is settled, every tranche risks being another sixteen good words that
   move the blocker by zero.

The likely resolution is that a lesson's SPINE NODE and its POSITION are
independent — a lesson appended at the end may still declare a pre-A1 node if its
prerequisites are all met. Worth verifying against the gate before authoring on
it, because the whole tranche depends on it.

## HL-C195 — "spanish 211/300 (pre-A1)" was the WRONG NUMBER, and I quoted it all night

The report line I added in HL-C184a prints a track's **track-wide** vocabulary
against the **next level's** target. It reads as progress toward pre-A1. It is
not. The level gate's actual pre-A1 criterion counts only headwords taught on
**pre-A1 spine nodes**:

```
blocker: vocabulary - teaches 48 distinct headwords at or below pre-A1, against 300
```

**Spanish is at 48/300 on the criterion that matters, not 227/300.** The gap is
252 words, not 73.

Consequence, and it is the important part: **adding vocabulary on an A1-or-higher
node does not advance pre-A1 at all.** HL-C194 added sixteen good words on
`SPINE-DEFINITE-REFERENCE` (A1) and moved the pre-A1 blocker by **zero**. The
words are worth having and the tranche should land — but it did not do the job it
was aimed at.

**Fix the report line first** (it is actively misleading, and it is mine):
print the blocker's own number — headwords at or below the in-progress level —
beside the track-wide count, not instead of it. Both are real; only one is the
criterion.

**Then re-plan the authoring.** Clearing pre-A1 means authoring on **pre-A1 spine
nodes**, which are early in the book, which is a forward-reference risk and is in
tension with "chain to the previous chapter's last lesson" (HL-C192). That
tension needs resolving before the next vocabulary tranche, or the next one will
also miss.

## HL-C193 — `git checkout --theirs` DISCARDS your side on additive config files

Resolving a merge conflict in the HL config files with `git checkout --theirs`
silently threw away every addition this branch had made, in **four** files, and
each one surfaced as a different failure several steps later:

| file | what was lost | how it showed up |
|---|---|---|
| `<track>/curriculum.json` | path + extension nodes | `validate`: "declares SPINE-X but is absent from the local path" |
| `core/book-generation.json` | the chapter target | `book-cli`: "is in neither targets[] nor handwritten[]" |
| `<track>/chapters.json` | the chapter entry | `book-cli`: "declaration has no chapters.json capability" |
| `<track>/book/book.tex` | the `\input` line | `book-cli`: "is never \input into book.tex" |

`--theirs` is right for a **generated** file (regenerate afterwards and the
content is restored) and wrong for an **authored, additive** one, where both
sides appended different entries and neither is a superset.

**Do this instead:** resolve additive config by hand or with a script that
re-derives the entries from the lessons on disk, which is what recovered it here.
The four failures also arrive in sequence rather than together, so fixing one and
re-running looks like progress while three more wait — run `validate`, the suite,
AND `book-cli` before believing a merge is resolved.

## HL-C192 — family and people words, six Indic tracks (LANDED)

Twenty-four lessons, four per track, one new word each. Two structural findings
came out of it and are the reason the row is kept:

* **A chapter-number reference is not a repinnable pin.** The gate reports STUCK
  rather than offering a number, which is correct — HL-C102 says the fix is prose.
* **Appending a chapter un-exempts the previous last lesson from R1.**
  `continuity.ts` skips a window the track is too short to contain, so each
  track's final atom was exempt; adding four lessons would have created six
  silent new R1 misses. Chaining each new chapter's first lesson to the previous
  chapter's last closed them, and R1 FELL instead of rising.

## HL-C191 — cross-script citations must be ROMANISED; each book loads only its own font

A comparative note in a Telugu lesson cited Kannada's ನೋಡು in Kannada script.
The Telugu book has no Kannada font, so it built at **exit=0 with 37 missing
characters** — and Malayalam the same, with 31. Zero overfull, zero underfull,
exit 0: indistinguishable from clean unless the missing-character count is read.

Comparative content across the six Indic tracks is *valuable* — Kannada
ಮಾತಾಡು and Telugu మాట్లాడు are both *word* + *play*, and saying so teaches
something neither lesson could alone. **Cite the cousin in romanisation**
(*mātāḍu*), never in its own script, unless that book is known to load the font.

Applies to every track pair. The check is mechanical: a lesson's body should
contain no codepoints from a script other than its own track's.

## HL-C188 — copy the SIBLING target's script mechanism; HL-C164 is not a corpus-wide law

HL-C164 says a new generated chapter must declare `unicodeScript` and
`scriptCommand`. Applied to a new **Hindi** chapter it produced `\dv{आना}` —
**undefined control sequence, `latexmk exit=12`, and ZERO warnings in the log**,
which reads identically to a clean build unless the exit code is checked.

Hindi does not use that mechanism. Its sibling entries carry

```json
{ "language": "hindi", "chapter": 41, "scriptSet": "hindi-main" }
```

and its macro is `\hi`, not `\dv`. Kannada, Telugu, Malayalam and Sanskrit *do*
use `unicodeScript`/`scriptCommand`, which is why four of five books built and
only Hindi broke.

**Rule:** before adding a book target, read the **immediately preceding entry for
that same language** and match its shape. HL-C164 remains true for the tracks it
was measured on. A remembered rule is not a substitute for looking at the
neighbour — and this is the second time in two days that a rule generalised one
track too far (see also HL-C185, where a remembered "23% classifier" turned out
to be a field that did not exist).
## HL-C186 — THE RAMP RULE: one new word per lesson, unlimited reuse

**Owner, 2026-08-15:** *"One new word per lesson but n number of existing words
can be re-emphasized and used?"* Adopted as the governing rule for authoring.

This is the precise form of "gentle ramp" the project has been circling:
**the cap is on what is NEW; reuse is free and encouraged.**

### What it settles

* **The ±35% counting ambiguity dissolves.** After splitting, headword count *is*
  word count. `teaches_items` (proposed in HL-C185) becomes unnecessary — the rule
  makes it 1 by construction. **Do not build that field.**
* **List lessons split.** The months become twelve lessons; colours, numbers and
  seasons likewise.
* **R1 gets EASIER, not harder.** The reinforcement ratio that blocked Sanskrit
  (HL-C167) measures whether an atom is revisited within three lessons. If every
  lesson reuses prior words by design, those windows fill by construction. The
  drizzle-versus-R1 tension may simply evaporate under this rule — **re-measure
  HL-C167 after the first tranche before doing any work on it.**

### Migration cost, measured

| | |
|---|---:|
| `type: word` lessons, all tracks | 1,407 |
| teaching more than one item | **400 (28%)** |
| extra lessons if every one splits | **+718** |

718 is an **upper bound**: the splitter must not fire on multi-token *single*
items — `la cabeza` is one noun with its article, `السلام عليكم` is one greeting.
A comma is a reliable list separator in the Latin-script tracks; the Indic and
Arabic month lists are space-separated. **No single rule covers both, so the
split is a reviewed pass per track, not one regex.**

### The scale this implies, stated plainly

At one new word per lesson, a track reaches C2's 16,000-word target in **~16,000
lessons** — fewer than the earlier ~27,600 estimate, because 1.0/lesson is denser
than the corpus's current 0.58.

**Across 22 tracks that is ~352,000 lessons, not 50,000.** Per track the budget
holds comfortably; the full programme is roughly seven times the stated figure.
That is not an argument against the rule — it is the number the rule implies, and
it should be seen before it is adopted rather than discovered at track four.

### Order of work under the rule

1. **Write the rule into the spec** (HL08/HL11) so it governs authoring, not just
   this row.
2. **Split the 400 multi-item lessons**, per track, reviewed — Spanish first as
   the reference track.
3. **Re-measure everything**: vocabulary per track, R1, and the atom budget. The
   211/300 figure will move, and HL-C167 may resolve itself.
4. **Then** author against the deficit, one new word at a time.

## HL-C185 — `word_class` does not exist. It is a schema addition, not a classifier fix.

**Measured 2026-08-15.** HL-C184c assumed the classifier reached ~23% and needed
improving. Wrong on both counts:

* **No lesson in any track carries a `word_class` field.** Coverage is **0 of
  1,692** lexical lessons, across all 22 tracks.
* **The schema has no part-of-speech field at all.** The frontmatter keys in use
  are `chapter, concept_tag, delivery, duration, est_minutes, etymology_hook,
  gloss, headword, id, introduces, modes, practises, prerequisites, register,
  requires, reviews_of, romanization, roots, schema_version, sequence, skills,
  slots, sounds, spine_node, strands, teaches_cells, type, variety` — and that is
  the whole list.

The remembered "23%" was a **classifier experiment**, not stored data. So this is
*add a field and populate 1,692 lessons*, not *improve an inference*. Materially
different work, and it must be sized as such.

### The good news: 36% is already mechanical

Many `concept_tag`s encode part of speech in their name. Deriving from the tag:

| class | lessons |
|---|---:|
| verb | **400** |
| greeting | 75 |
| pronoun | 53 |
| number | 34 |
| adjective | 32 |
| noun | 13 |
| connective | 10 |
| adverb | 3 |
| **derivable** | **620 of 1,692 (36%)** |

**Note the verb number against the owner's 500-verb target: 400 is across ALL 22
tracks.** Spanish alone has ~46. Per track, the target is far off, and the
census will say so per track once the field exists.

### Why the other 64% is not derivable

The unmatched tags are **topical, not grammatical** — `AR-COLOUR-BLACK-WHITE`,
`AR-MONTHS`, `AR-SEASONS`, `AR-FOOD-GENERAL`. They say what a word is *about*,
not what it *is*. Several are also multi-word list headwords (twelve months in
one lesson), so "the word class of this lesson" is not even well defined for
them.

### Plan, and what NOT to do

1. **Add `word_class` to the schema** as optional, with a closed vocabulary
   (`noun, verb, adjective, adverb, pronoun, number, connective, interjection,
   phrase, other`).
2. **Populate the 620 mechanically** from the tag prefixes above, in one scripted
   pass, with the derivation table committed alongside so it is auditable.
3. **Leave the other 1,072 UNSET and report the coverage.** Do not guess. An
   inferred class that is wrong is worse than an absent one, because the census
   would then report a confident wrong number — the exact failure mode of
   [[feedback_a_number_that_never_moves_is_not_a_measurement]].
4. **Then** the per-track verb/adjective/adverb census the owner asked for
   becomes computable, and HL-C184d's "measure what the tranche cost" means
   something.

### Correction, same day: a month IS a noun, and the real gap is COUNT not CLASS

The paragraph this replaces claimed multi-word list lessons "cannot be classed at
all". **Owner: *"A month is a noun isn't it?"*** Yes. Every word in `AR-MONTHS`
is a noun; the class was never ambiguous. Two different questions were conflated:

* **What class are these words?** Noun. Unambiguous. `word_class` handles it.
* **How many lexical items does this lesson teach?** Twelve, not one. **This is
  the unsolved one, and it is not a classification problem.**

**And it makes the headline vocabulary number ambiguous by ±35%.** For Spanish:

| counting method | result |
|---|---:|
| headwords (what the gate does today) | **211** |
| one lexical item per lesson, lists expanded | **~285** |
| naive whitespace tokens | **361** |

No mechanical rule picks between them. `type` does not: `la cabeza` is
`type: word` with two tokens and teaches **one** noun, while `negro, blanco` is
`type: word` with two tokens and teaches **two** adjectives. Splitting on commas
fixes Spanish and breaks the space-separated Arabic months; splitting on spaces
fixes the months and breaks `la cabeza` and `السلام عليكم`.

**So the corpus must STATE it.** Add a second field alongside `word_class`:

    teaches_items: 12      # lexical items this lesson introduces, default 1

and make `vocabularyOf` sum it instead of counting headwords. Until then the
211/300 pre-A1 figure is a **lower bound**, not a measurement — and the true
number could clear pre-A1 already on some tracks. **That has to be settled before
HL-C184d authors a single word**, or the tranche will be sized against a number
that is wrong by a third.

Splitting list lessons one-word-per-lesson remains an option the 50,000-lesson
budget permits, and it would make `teaches_items` almost always 1 — but that is a
pedagogical change (twelve month-lessons instead of one), and it is the owner's
call, not a side effect of a measurement fix.

## HL-C184 — THE COMPLETION PLAN: 22 tracks to C2, driven by vocabulary

Measured 2026-08-14, once the report finally printed the right number:

```
VOCABULARY vs HL09 §3.1 targets: spanish 211/300 (pre-A1), hindi 97/300,
tamil 97/300, malayalam 95/300, french 94/300;
22 of 22 tracks short of the level they are working on
```

**Every track fails pre-A1.** The strongest is at 70% of the *first* rung. This
row is the ordered plan from here to C2 across all 22, and it is deliberately
long-horizon: the owner's budget is **50,000 lessons**, and at the corpus's
current density (~0.58 new words per lesson) reaching C2 on one track costs
~27,600 lessons. Nothing here is a sprint.

### Phase 0 — make the target visible (in progress)

* **HL-C184a. Print vocabulary in the report.** DONE — this row exists because of it.
* **HL-C184b. Make the vocabulary floor a REPORT-ONLY gate first**, per the HL05
  precedent: a gate that fails on inherited debt teaches authors to route around
  it. It becomes blocking per track as that track clears pre-A1.
* **HL-C184c. Close the `word_class` classifier** (~23% of Spanish lexical
  lessons). Without it the verb/adjective/adverb census the owner asked for
  cannot be computed at all, and "500 verbs" has no measurement.

### Phase 1 — clear pre-A1 on one track, and learn the real cost

* **HL-C184d. Spanish pre-A1: 211 → 300.** Eighty-nine words. Do it as one
  vocabulary tranche and **measure what it actually costs** — lessons written,
  wall-clock, how many ceilings move. Every later estimate depends on this
  number, and it is currently a guess.
* **HL-C184e. Write down the density that results.** Words per lesson, and
  whether the ramp gates (R1, atom budget) bind before the target does.

### Phase 2 — the ladder, cheapest rung first

Per track, in order: pre-A1 300 → A1 600 → A2 1,200 → B1 2,500 → B2 4,000 →
C1 8,000 → C2 16,000. **Author against the vocabulary deficit, not against spine
nodes** — the spine is 33 functional rungs and is already closed on Spanish while
the corpus fails pre-A1, which is exactly how coarse it is.

Sequencing across tracks is the owner's alternation rule: Spanish and the Indic
six move together, and no track is left to rot.

### Phase 3 — what vocabulary alone will not buy

Recorded now so it is not discovered late: HL-C182 lists what none of this
measures — retention, production under time pressure, listening at natural speed,
exam task formats. **A track at 16,000 words is a corpus claim, not a learner
claim.** The honest test needs a human sitting a past paper.

### Standing rules for every item above

* **A gentle ramp caps new atoms PER LESSON, never lesson count.** Chapters are
  sized to carry vocabulary at that density; a spine node closes when it closes.
  See [[feedback_gentle_ramp_means_few_atoms_per_lesson_not_few_lessons]].
* **Lead every report with `vocabulary / target`**, never spine coverage.
* A ratio going over means the ramp got worse — revert the content, do not
  re-seat (HL-C167).

## HL-C183 — VOCABULARY MASS is the dominant remaining cost, and the gate already said so

**Owner, 2026-08-14:** *"a native speaker knows something like 10K words … I want
at least 500 verbs covered. Lot more adjectives, adverbs and others. This feels
woefully inadequate."* Measured, and correct.

`LEVEL_VOCABULARY` (`level-gate.ts:42`) already sets the targets:

| level | target | Spanish |
|---|---:|---:|
| pre-A1 | 300 | **267 — FAILS** |
| A1 | 600 | |
| A2 | 1,200 | |
| B1 | 2,500 | |
| B2 | 4,000 | |
| C1 | 8,000 | |
| **C2** | **16,000** | **1.7% of it** |

Spanish teaches **267 distinct word tokens** across **211** word/phrase lessons,
and ~46 look like infinitives against an owner target of **500 verbs** (9%). It
does not clear pre-A1 — which is exactly why `attained` is null.

**The reframing this forces.** Closing the spine at 33/33 measured **functional
coverage**: a rung for every can-do statement. It says nothing about **lexical
mass**, the words needed to stand on those rungs. All 33 nodes were closed with
211 word lessons, which should have read as a warning rather than a milestone —
and every progress report in that session led with the spine number.

It also explains something noticed but not understood at the time: three of four
C1 "gaps" were satisfiable by alias. **A 33-node functional spine is too coarse
to drive authoring.** Vocabulary is the real cost, and it has a real target.

**Scale.** ~15,700 more words for Spanish alone to reach C2, and the same again
across 21 other tracks. HL09 §3's estimate of ~8,000 lessons for a complete track
is consistent; Spanish has 463.

**Order of work:**

1. **Lead with the vocabulary number in every report** — `vocabulary /
   LEVEL_VOCABULARY[next level]` per track, beside `attained`. Free, and it stops
   "33/33" reading as completeness. Do this *before* authoring anything.
2. **Close the `word_class` classifier** (~23% of Spanish lexical lessons today,
   per [[feedback_measure_before_building_it_redefines_the_task]]) so the
   verb/adjective/adverb census the owner asked for can exist at all. Prerequisite,
   not a side quest.
3. **Plan authoring against the deficit**, cheapest rung first: pre-A1 needs only
   **33 more words**, A1 another 300. The corpus is nearer a *usable* A1 than the
   C2 headline suggests.
4. **The lesson-to-word ratio — and the answer to "is 16,000 even reachable".**
   463 lessons yield 267 words: **~0.58 new words per lesson.** Holding that
   density, 16,000 words needs **~27,600 lessons** for Spanish.

   **Owner, 2026-08-14, on being shown the deficit:** *"This is why I kept telling
   you to ignore the page count and lesson count and focus on teaching with a very
   gentle ramp. If we end up 50K lessons, that is fine with me. We can split them
   up later."*

   So the answer is **yes, and without touching the ramp.** 27,600 sits inside the
   stated budget with room to spare. **The gentle ramp and the vocabulary target
   were never in tension** — and the sessions that treated them as if they were
   made a category error worth naming, because it will recur:

   > A gentle ramp means **few new atoms per lesson**. It does NOT mean few
   > lessons, small chapters, or a compact book. Closing a spine node with a tidy
   > four-lesson chapter optimises the wrong quantity. The budget that must stay
   > small is per-lesson load; the quantity that must grow without limit is the
   > number of lessons.

   Practical consequence: **stop sizing chapters to close nodes.** Size them to
   carry vocabulary at the density the ramp allows, and let the node close when it
   closes. See [[feedback_page_count_is_never_a_constraint]] — the instruction was
   already recorded; what was missing was the arithmetic showing it is the *only*
   way the target is reachable.

Related: HL-C182 (nothing measures whether a reader could pass anything) — this
row is the concrete, already-instrumented half of that gap.

## HL-C166 — CLOSED 2026-08-14. `data/scripts/repin_tests.py`

Committed, documented, and verified by breaking a pin on purpose: the dry run
reports the fix and writes nothing, the real run writes it, and the annotation
lands AFTER the trailing comma. Both bugs that motivated it are named in the
file's header, along with the property they share — invisible to a test *count*,
visible only to an exit *code*.

It re-pins COUNTS only. On a ceiling or a ratio it prints STUCK and stops, which
is correct: those failing means something regressed, and the answer is to fix the
content (HL-C167).

## HL-C166-ORIGINAL — superseded, kept for the record

Two self-inflicted delays in one PR, both from retyping the pin-patching script
inline instead of reusing it:

1. A copy modified the test lines in memory and **never wrote the file**, so it
   re-ran the whole suite unchanged until it was killed.
2. A copy appended its explanatory comment **before** the trailing comma —
   `totalLessons: 2266 // HL-C166,` — which comments the comma out and makes the
   file unparseable. A passing *test count* hides this completely; only the exit
   code catches it. Same bug, second time this week.

The working version is `/tmp/claude-501/repin.py` in-session, and it belongs in
the repo: `code/learning/human-languages/data/scripts/repin_tests.py`, taking the
annotation tag as `argv[1]`. It handles the three pin shapes the corpus actually
uses — a bare `toBe(N)` / `toHaveLength(N)`, an object field, and a
`{ term: "verb", lessons: N }` row — and it writes the file in **every** branch,
with the comment after the comma.

Until it is committed, every session re-derives it and re-introduces one of these
two bugs.

## HL-C164 — a new generated chapter must declare its script in book-generation.json

Discovered while authoring Sanskrit chapter 16 (2026-08-14). A target entry
written as

```json
{ "language": "sanskrit", "chapter": 16, "output": "…/ch16-places.tex" }
```

builds at `exit=0` and emits **888 missing-character warnings**: every
Devanagari run falls through to Latin Modern, because the renderer only wraps
script runs in `\sk{…}` when the target says so. The sibling entries carry two
more keys:

```json
"unicodeScript": "Devanagari", "scriptCommand": "sk"
```

The lesson markdown and frontmatter are **identical either way** — nothing in the
lesson tells you it is missing, and `exit=0` plus clean gates do not either. Only
the `Missing character` count in the log catches it, which is why the book build
is checked on warning counts and not just on its exit code.

**Applies to every non-Latin track**, not just Sanskrit. Add both keys whenever a
new chapter target is registered.

## HL-C182 — nothing in this repo measures whether a READER could pass anything

Raised by the owner, 2026-08-14, on seeing Spanish close its spine at 33/33.
There are **two** gaps between a track's headline number and a person sitting an
exam, and only the first is currently measured.

**Gap 1 — `touches` to `attained`. Measured.** `touches` moves on one lesson
pointed at a node; `attained` needs all four HL09 §3.1 criteria at the level and
every level below. Spanish reads `touches: C2`, `attained: null`. The level gate
exists precisely to keep these apart and it is doing its job.

**Gap 2 — `attained` to passing. NOT MEASURED, AND NOT PREVIOUSLY WRITTEN DOWN.**
All four criteria are properties of the **corpus**: nodes realized, cumulative
vocabulary, atom budget, atoms revisited twice. Every one asks *is the material
complete and well-paced to spec*. None of them touches:

* whether a learner did the lessons, or retained them a month later
* **production under time pressure** — speaking and writing to a prompt
* **listening to unfamiliar voices at natural speed**
* exam task formats, which are a separate skill from the language
* an examiner's judgement, which is not a checklist

So `attained: C1` would mean *this book covers C1 to spec*. It would **not** mean
a reader who finished it can sit a DELE C1. And the corpus claim is itself
proxied: "cumulative vocabulary ≥ N" and "every atom revisited twice" are
structural stand-ins for good teaching, not evidence of it.

**Why this belongs in the backlog rather than a footnote.** Every progress report
in this repo quotes spine coverage. A reader can reasonably infer that 33/33
says something about learner capability. It does not, and the distance is not
currently quantified anywhere.

**What would close it, cheapest first:**

1. **Name the gap in the reports.** `report.ts` should print `attained` beside
   `touches` with a one-line statement that both are corpus measures. Free, and
   it stops the inference at the source.
2. **A retrieval measure.** The corpus already tracks atoms and revisits; it
   could report, per level, what fraction of atoms a spaced-repetition schedule
   would still hold at 30 days. Still a corpus property, but a much closer proxy.
3. **Exam-task coverage.** Map each level's real exam task types (DELE, SIELE)
   against what the corpus asks the learner to DO. HL-C130 already models task
   shapes; this is a gap analysis on top of it, and it is the first measure that
   would be about the *reader's* activity rather than the book's contents.
4. **The honest one, and it needs a human:** a person who has done the track sits
   a past paper. Nothing computable substitutes, and every number above is a
   proxy for this.

Related: the MYCIN north star is "pass any board exam" — the same distinction
applies there and is worth stating in both places.

## HL-C176 — the forward-reference detector cannot tell SHOWN from USED

Surfaced by HL-C175. `ES-C272-si-claro` is the first lesson to own *claro*, and
the detector immediately reported an early use 356 lessons back in
`ES-C56-mente`. Looking at it:

```
> *claro* → *clara* → **claramente**
```

That is a **citation form inside a derivation table**. The word is being *shown*
as an example of how `-mente` adverbs are built, not *used* to mean "clear" in a
sentence a reader has to decode. The reader is never asked to know what *claro*
means there.

The detector counts it identically to a real early use, and it is the same family
as the HL-C150 substring cases (Tamil `-அது`, Malayalam `അതെ`) — a heuristic that
is right often enough to be worth having and wrong often enough to inflate a
ceiling that is supposed to mean something.

**Sharpening, in rough order of value:**

1. Exclude words appearing inside a **derivation or paradigm row** — the
   `a → b → c` arrow form is mechanical to detect and is always exemplification.
2. Exclude **block-quoted example lines** generally: a `>` line in these lessons
   is display material, not running prose the reader must parse.
3. Only then consider word-boundary matching, which HL-C150 already logged.

Each of these lowers the count *without* lowering the standard, which is the
opposite of re-seating the ceiling. Until it is done, every new lesson that
happens to own a common word will push the ceiling up by one for a reason that is
not real debt.

## HL-C174 — C1 opens at 2 of 4 on two aliases; what remains needs authoring

`HONORIFIC-SYSTEM` → `ES-REGISTER-TU-USTED` and `DIALECT-FEATURE` →
`ES-PRONOUN-VOS`. Both are the canonical instance of the concept rather than a
near-miss: *tú/usted* **is** a T–V honorific system, and *voseo* **is** the
textbook dialect feature of Spanish. **C1 goes 0/4 → 2/4, spine 25/33 → 27/33,
no lesson written.**

**Not aliased, and here is what each would need:**

* `POLITENESS-STRATEGY` — `ES-SYNTHESIS-COMMANDS-POLITENESS` covers it, but its
  type is `practice-mix`, which is not in `REALIZING_TYPES`. **Open question
  worth answering before the C1 sweep continues:** should a synthesis lesson be
  able to realize a concept? It is arguably the *best* evidence a concept is
  held, since a synthesis is where the learner uses it under load.
* `REGISTER-FORMAL` / `REGISTER-COLLOQUIAL` — the corpus teaches the *forms*
  (`hable usted`, the vosotros preterites) but never names register as a thing
  you choose. Needs authoring.
* `ACCENT-VARIATION`, `DIGLOSSIA` — genuinely absent.
* `LEXICAL-VARIANT-REGIONAL` — nearly there: `billete`/`boleto` is noted inside
  the travel lesson, and `ES-C09-falsos-amigos` is adjacent. Neither *owns* the
  concept. One small lesson on the regional word pairs would close it.

The two remaining C1 nodes (`SPINE-INFER-IMPLICIT-MEANING`,
`SPINE-STRUCTURE-EXTENDED-TEXT`) have **no** alias candidates at all — implicature,
irony, anaphora and paragraph planning are simply untaught. Those are authoring
work, and they are the first genuinely new *teaching* the ladder has needed since
B1.

## HL-C171 — cross-node aliases work; the relocation worry was unfounded

HL-C170 left cross-node aliasing as blocked, on the reasoning that pointing a
concept at a lesson filed under a *different* spine node would trip the
relocation ledger. **Tested, and it does not.** The relocation check fires when a
lesson's own `concept_tag` appears in `node.concepts` while its `spine_node`
differs — and an aliased lesson's tag is a track-local name that is never in
`node.concepts`, so the two mechanisms do not meet.

Five aliases added on that basis:

| concept | satisfied by | note |
|---|---|---|
| `REPORTED-SPEECH` | `ES-REPORT-QUE-OBLIGATORY` | *dice que* — he says THAT |
| `SPEECH-ACT-REPORT` | `ES-REPORT-DECIR-PRETERITE` | *dijo* |
| `MODAL-WOULD` | `ES-CONDITIONAL` | *hablaría*, cross-node |
| `VOICE-PASSIVE` | `ES-GRAMMAR-SE-PASSIVE-AGREEMENT` | cross-node |
| `RELATIVE-CLAUSE` | `ES-GRAMMAR-RELATIVE-QUE` | cross-node |

**`SPINE-EXPRESS-CONDITION` is now complete — omissions empty.**
**`SPINE-READ-EXTENDED-PROSE` opens. B2 goes 1/4 → 2/4, the spine 22/33 → 23/33,
and B1 is complete at the CONCEPT level, not just the node level.** No lesson
written.

**Deliberately NOT aliased**, because the match would have been wishful rather
than true — each of these needs authoring:

* `CONNECTIVE-HOWEVER` / `CONNECTIVE-ALTHOUGH` (`SPINE-ARGUE-A-VIEW`). The track
  has *pero*, *también*, *tampoco*. **`pero` is "but", not "however"** —
  *sin embargo* and *aunque* are genuinely absent. Aliasing `pero` here would
  have made the ledger lie.
* `GREETING-EVENING` (`SPINE-TIME-OF-DAY`) against `GREETING-GOODNIGHT`: evening
  is not night.
* `QUOTATION-DIRECT`: the track's `ES-REPORT-WH-ACCENT` and `ES-REPORT-YESNO-SI`
  are INDIRECT questions. Direct quotation is untaught.

The rule this establishes: an alias asserts *this lesson teaches that concept*.
Where it does not, the concept stays in `omits` and gets authored. The remaining
~65 unclaimed concepts should be swept with that test applied one at a time, not
pattern-matched in bulk.

## HL-C170 — grammar lessons can realize a spine concept now; HL-C169 was half a diagnosis

HL-C169 called the 73 unclaimed Spanish concepts a **tagging** problem. Measuring
the mechanism rather than the symptom found the larger half:
`CONTENT_TYPES = {word, phrase}` gated realization, so **a `grammar` lesson could
never realize a concept at all** — and half the spine's concepts *are* grammar
(`TENSE-BACKSHIFT`, `RELATIVE-CLAUSE`, `VOICE-PASSIVE`, `CONNECTIVE-IF`). No
`word` lesson will ever teach those. A node declaring them read as unrealized no
matter how completely the corpus taught it.

Fixed with a **separate `REALIZING_TYPES` set**, not by widening `CONTENT_TYPES`
— that set is read by eight call sites meaning the narrower thing, and widening
it raised 33 validation errors across other tracks on lessons that legitimately
carry no `concept_tag`.

The tagging half is real too, and is handled by `conceptAliases` in a track's
`curriculum.json`: the spine names concepts language-neutrally, a track names
lessons in its own terms, and an alias lets the second answer the first without
retagging and discarding the specific name.

**Result: B2 opens at 1/4 and the spine goes 21/33 → 22/33, with no lesson
written.**

**Still to do, and now cheap:**

* Three declared aliases so far (`CONNECTIVE-IF`, `CONDITIONAL-REAL`,
  `TENSE-BACKSHIFT`). Sweep the remaining ~70 the same way.
* **Cross-node aliases are not yet supported.** `VOICE-PASSIVE` and
  `RELATIVE-CLAUSE` are taught by `ES-C52-se-venden` and `ES-C53-que-relativo`,
  which sit under `SPINE-GIVE-REASONS` rather than the node declaring the
  concept, and aliasing across nodes would trip the relocation ledger. Either the
  lessons move to the declaring node or the relocation rule learns about aliases.
  Those two alone open `SPINE-READ-EXTENDED-PROSE`.
* `MODAL-WOULD` is the same case: `ES-C17-condicional` (*hablaría*) sits under
  `SPINE-TALK-ABOUT-FUTURE`. It is the last omission on `SPINE-EXPRESS-CONDITION`.

## HL-C169 — the spine's concept names and the corpus's tags were never reconciled

**Measured 2026-08-14. Spanish claims 88 of 161 spine concept names; 73 are
unclaimed.** That number is not a content gap, and reading it as one has now
misdirected work four times in a single session:

| node | read as | actually |
|---|---|---|
| `SPINE-TALK-ABOUT-PAST` (A2) | unrealized | 29 path segments teaching preterite + imperfect |
| `SPINE-TALK-ABOUT-FUTURE` (A2) | unrealized | near future taught; only the synthetic future was missing |
| `SPINE-EXPRESS-CONDITION` (B1) | unrealized | **11 lessons**, chapters 123 and 196-210 |
| `SPINE-REPORT-WHAT-OTHERS-SAID` / `SPINE-READ-EXTENDED-PROSE` (B2) | unrealized | `ES-REPORT-BACKSHIFT`, `ES-GRAMMAR-SE-PASSIVE-AGREEMENT`, `ES-GRAMMAR-RELATIVE-QUE` all exist |

Each time the obvious move was to author the missing concepts, and each time
measuring first showed that would have **duplicated real teaching**. The lessons
exist; they simply carry corpus-local tags (`ES-CONDITION-SI-REAL`,
`ES-REPORT-BACKSHIFT`) rather than the bare spine names the ledger looks for.

**This is one reconciliation pass, not sixteen content PRs**, and it should be
done before any more B2/C1/C2 authoring, because every future stage will hit it:

1. For each unclaimed spine concept, search the corpus for a lesson that already
   teaches it. Most will have one.
2. Decide the convention **once** — either the realizing lesson carries the bare
   spine name as its `concept_tag`, or `spine.json` gains an `aliases` field
   mapping a concept to the corpus tags that satisfy it. The second is less
   invasive and keeps the specific tags, which carry real information.
3. Author only what genuinely has no teacher. On the evidence so far that is a
   minority — `ayer`, the synthetic future and `depende` were three such, and
   each was one lesson rather than four.

Until this is done, "N of 33 spine nodes" understates the corpus and the
omission ledger records tagging debt as if it were missing content.

## HL-C167 — R1 and the script drizzle are in STRUCTURAL tension, and more chapters will not fix it

**Measured 2026-08-14, after HL-C163/165/166 took Sanskrit from 15 chapters to
20.** HL-C161 predicted the ledger would advance once the track had runway. The
runway worked exactly as predicted for ordinary content — R1 fell 0.3232 →
0.3175 as chapters 16–20 landed — and then the segments were re-run:

| Sanskrit segments added | R1 ratio | ceiling 0.32 |
|---:|---:|---|
| 0 | 0.3175 | pass |
| 4 | 0.3206 | **over** |
| 9 | 0.3222 | **over** |

Roughly **+0.0008 per segment**, against 0.0025 of headroom. Three would fit.

**This is not a tuning problem, and the next session should not spend a PR
tuning it.** R1 asks whether an atom is revisited within *three lessons*. The
drizzle deliberately spaces script segments about five lessons apart, one per
chapter — that spacing is the entire point of a gentle ramp (HL11). So a
drizzled letter is, *by construction*, alone inside R1's window. Making segments
revisit the previous letter does not help: the previous segment is ~5 lessons
back, still outside the window.

More chapters raise the denominator and buy a few more segments, but the tension
returns at every tranche. Shipping three of nine to slip under the ratchet would
be gaming the measurement, not satisfying it.

**The real question is a scoping one, and it needs deciding before more segments
are authored:** should a drizzled script atom be *in* the R1 population at all?
Three candidate answers, in the order I would try them:

1. **Score script atoms on their own window.** A letter's natural reinforcement
   is the next lesson that *uses* it in a word, which is chapters away by design.
   A `SCRIPT-R1` window measured in chapters rather than lessons would say
   something true; the current one says something true about the wrong thing.
2. **Exclude script atoms from R1 and gate them on letter-ledger closure
   instead** — which already exists and already measures the right property.
3. **Re-base R1 per track** with a stated drizzle allowance. Weakest option: it
   hides the tension behind a number rather than resolving it.

Until that is decided, Sanskrit's ledger stays at 8 of 24 and this row is the
reason. Everything else about the track is unblocked — 20 chapters, 114 lessons,
and R1 improving on every content PR.

## HL-C161 — Sanskrit's ledger is blocked on CHAPTERS, and the ratio proves it

**Measured 2026-08-14, and this row exists because the attempt was reverted
rather than shipped.**

`author_recognition_segments.py sanskrit` happily places ledger positions 15-23
— eight new segments, taking Sanskrit from 8 to 16. Everything passes except one
number, and that number is the whole point:

```
R1 missed / atomsTaught   1087 / 3363 = 0.3232     ceiling 0.32
```

R1 is the tightest reinforcement window: an atom must be revisited within three
lessons. Sanskrit has **15 chapters** against 41-42 in every sibling track, so a
letter taught in chapter 13 has nothing after it to come back to. The eight
segments are individually fine and collectively unreinforced.

This is a RATIO, not a count — it is scale-invariant on purpose, so exceeding it
is not "the corpus got bigger," it is "the ramp got worse." Re-seating it to
0.33 would have bought eight ledger positions by making the ramp measurably less
gentle, in the one track least able to afford it.

**So the order of work is fixed, and it is the reverse of what the ledger gap
suggests:**

1. Author Sanskrit chapters 16-40, bringing it level with its siblings.
2. THEN re-run the segment generator, which will place positions 15-24 into a
   book with runway behind them.

Doing (2) first is what this row is here to prevent. Related: HL12 §3.1 — size
is not a constraint, and no rule may be relaxed to save pages.

## HL-C162 — CLOSED 2026-08-14. Kannada's ledger is 24 of 24

The original diagnosis was wrong in an instructive way. It said no taught Kannada
word contains ಓ. **ಓದು (*ōdu*, "to read") has been taught since chapter 33.**

The generator writes 0 segments because its drizzle slots all sit in **chapters
6–20**, and ಓದು arrives at 33 — the word exists, but it exists *after* every slot
the generator will consider. So the position was not missing a word; it was
missing a **slot late enough to use the word it already had**.

Closed by authoring the segment by hand at chapter 34, immediately after ಓದು.
R1 moved 0.3175 → 0.3177 against its 0.32 ceiling: one segment fits comfortably,
which is consistent with the ~+0.0008-per-segment figure in HL-C167.

**Follow-on, still open:** the generator's drizzle window should extend past
chapter 20 so a late-taught word can host a late ledger position without hand
authoring. That is the same window question HL-C167 raises from the other side.

## HL-C162-ORIGINAL — superseded, kept for the record

`author_recognition_segments.py kannada` writes **0 segments**: no chapter slot
qualifies, because the drizzle rule requires the letter to appear in a word the
reader already knows, and no taught Kannada word contains ಓ. The ledger sits at
23 of 24 for want of one vocabulary lesson.

Fix is a Kannada word lesson containing ಓ (e.g. ಓದು, "to read") placed before
the drizzle slot; the generator then closes the position with no further work.


Note on ids: the HL11 script-ramp block was opened as HL-C113…HL-C120, but
HL-C113 had already been taken by the CEFR B1→C2 climb, which eight merged PRs
(#11078, #11081, #11086, #11090, #11094, #11096, #11099 and this one) cite in
their commit messages. The climb keeps **HL-C113**; the script block is
therefore **HL-C114…HL-C120 plus HL-C122** (the PNG encoder, renumbered).

## Prioritization, 2026-08-12 (third pass)

The owner reset the target: *"The goal is not whether something touches some
level. The goal is can someone pass that level of exam with just reading the
book and slowly following its gentle ramp."* That is a different question from
the one this backlog had been answering, and measuring against it moved the top
of the list again.

**The honest answer today is that a reader could not pass DELE at any level,
and grammar is not why.** Three measurements, taken 2026-08-12 over all 220
Spanish chapters:

| what the exam asks for | what the book has |
|---|---|
| four papers, and **≥30/50 in each of two groups** independently | 704 of 704 activities are one shape: `kind: "text"`, a prompt and a short answer |
| a reading paper over connected texts (A1: 25 questions, 4 tasks, 45 min) | longest continuous Spanish anywhere in the book: **10 words**; passages of 20+ words: **zero** |
| a written-expression paper at every level | `writing` claimed by 44 of 366 lessons (**12%**); 7 lessons of `type: writing` |
| ~80-100 enumerated A1 grammar points (Plan Curricular) | no mapping exists, so coverage is unknown rather than low |

**P0 — make the target measurable, then measure against it.**

1. **HL-C128** — replace `touches`/`attained` with Plan Curricular coverage.
   Until this exists, every other number in this file is a proxy for something
   nobody is graded on. It is also the cheapest row here: the inventory is
   published, finite and already split by level.
2. **HL-C130** — the task shapes. A candidate must clear both groups
   independently, so the book's 12% writing coverage is not a weakness to
   improve later, it is a guaranteed fail of Group 1 on its own.
3. **HL-C129** — connected prose. The 10-word ceiling is the single most
   damning number in this file, and it also happens to be the cheapest way to
   fix HL-C123's reinforcement blocker: a passage revisits fifty headwords at
   once, which fifty flashcards do not.

**P1 — breadth, now with a target that comes from the exam rather than from us.**

4. **HL-C123** / **HL-C121** — headwords and verbs (48 at-or-below-pre-A1 of a
   300 target; 43 verb lemmas of the owner's ~500). Still real, still large,
   but re-scope the targets against the Plan Curricular inventories rather than
   against numbers this repository invented.
5. **HL-C125** — the four zero-occurrence connectives, which are on the A1/A2
   inventory and block an already-open node.

**P2 — reachability and hygiene.**

6. **HL-C126** — `SPINE-DESCRIBE-EXPERIENCE` has 0 segments and gates two
   further nodes. 7. **HL-C127** — the promised `vosotros` and strong
   `nosotros` preterite forms. 8. **HL-C124** — `spineNodes` drift, ships with
   a test or not at all. 9. **HL-C43** — the voice-capable review format, now
   with three worked examples.

**Demoted: further CEFR climbing (HL-C113 steps 9+).** C1 and C2 stay at zero.
The climb was moving a number the exam does not award marks for. It resumes
when a level has been *passed* on HL-C128's criteria rather than *touched*.


Previously prioritized: 2026-08-12, when HL11 opened the drizzled script ramp for the
six Indic tracks. It sits at P0 alongside HL10 because it is the
same failure at a different layer: HL09 found a curriculum that measured gentle and
read brutal, and HL11 finds six tracks that pass the glyph budget while asking the
reader to decode letters no lesson ever taught.

Previously prioritized: 2026-08-09, after merging #10219 for HL-C06. Spanish
`ES-C17-comer-futuro` is now the first canonical
productive frame, with ordered app-visible slots limited to already-known *comer*,
*beber*, and *café*. Its three gates now exercise real content rather than passing over
an empty set. HL-C06 closes the next bounded structural slice: one canonical-data
etymology figure now travels through the shared SVG, book PDF, and app path.
With all 22 downloadable books now carrying pronunciation, glossary,
review-question, answer-key, and English-first index back matter, HL-C50 is
complete. The HL-C63 audit confirmed that all 98 missing handwritten chapters
already have canonical lesson sources; 47 lessons across 11 of those chapters
were additionally absent from their shared-spine paths. Validation then exposed
two more Spanish prerequisite lessons and showed that each added segment needed
an explicit language-local extension classification. HL-C63 is therefore one
coherent tranche: author every missing capability, place the 49 lessons in
prerequisite-safe paths, classify local support, keep legacy schema-v1 `assesses`
lists honestly empty until atom migration, and leave handwritten/generation
status unchanged. HL-C64 was discovered and fixed inside that tranche because
the nested-brace parser made title agreement impossible for four Spanish chapters.
HL-C10's stated structural signal is already complete across #10010, #10013,
and #10067: all seven stages carry nodes, and all 22 track ledgers name every
node without drift. HL-C65 migrates Spanish Chapter 7 across the schema-v2
boundary in #10132. HL-C66 carries Spanish Chapter 8 across that boundary in
#10135. HL-C67 carries Spanish Chapter 9 across the same boundary in #10142,
with the `ser`/`estar` contrast limited to already-known identity, state, and
location language. HL-C68 migrates Spanish Chapter 10 in #10146: `ir`, the
near-future frame, and possessives advance only from the singular-person,
known-infinitive, and known-noun frontier. HL-C69 migrates Spanish Chapter 11
in #10150: `querer`, `poder`, their singular vowel-change comparison, and
`nuestro`/`nuestra` stay inside that same closed frontier. HL-C70 migrates
Spanish Chapter 12 in #10153: `hacer`, `decir`, and the first `yo`-go comparison
stay away from later club members and untaught weather or homework vocabulary.
HL-C71 completes the next bounded step in #10159: Chapter 13 introduces `poner`, `salir`,
and `venir` one singular set at a time, and only the terminal checkpoint widens
the learned yo-go comparison. HL-C72 continues that boundary through Chapter 14:
`fui`/`fuiste`/`fue` and `hablé`/`hablaste`/`habló` arrive in two three-atom
steps, followed by a known-word checkpoint. HL-C73 continues through Chapter 15
in #10170: five teaching lessons introduce one bounded singular pattern apiece
for `comer`, `vivir`, `tener`, `hacer`, and `estar`; the terminal checkpoint
retrieves all twelve new atoms without importing later vocabulary or any plural
form. The migration also corrects the false `c`→`z` sound-preservation rule for
`hizo`, replaces direct letter-wearing claims with sound change and analogy, and
keeps all five teaching steps voice-drivable. HL-C74 continues through Chapter
16 in #10177: seven teaching lessons introduce twelve atoms, one
singular imperfect row at a time, and a terminal checkpoint retrieves them with
only known-word contrasts. A dedicated step teaches `ver` before its present or
imperfect forms are required; every plural form and every undeclared time clue,
person, place, and verb waits. The migration also replaces absolute irregular-
inventory and tidy three-source claims with bounded descriptions supported by
the learned evidence. HL-C75 continues through Chapter 17 in #10183:
seven teaching lessons introduce twelve atoms across separate singular future,
conditional, and three-stem steps; a mapped terminal checkpoint retrieves the
whole chapter. Every plural person, extra irregular stem, modern `haber`
paradigm, object-pronoun frame, Portuguese mesoclisis tangent, and clock-time
conjecture waits. HL-C76 continues through Chapter 18: eight teaching lessons
introduce twelve atoms, beginning with the asserted `hablas` versus wanted
`quiero que hables` contrast, then adding one singular regular or already-known
irregular row at a time before a mapped checkpoint. Plurals, extra irregulars,
object pronouns, person nouns, and additional triggers wait; `ojalá` keeps its
Hispanic-Arabic history without making the source phrase learner vocabulary.
HL-C77 closes that migration boundary: all 41 Spanish book chapters now generate
from the same canonical lesson ASTs used by Language Ladder, narration, objective
activities, and back matter. The twelve former handwritten entries are protected
generated targets with checked hashes, titles, labels, order, and output. HL-C19
is already complete; HL-C09A through HL-C09G verified Tamil அ, ஆ, இ, க, வ, ல,
and ற in #10222, #10223, #10226, #10228, #10230, #10234, and #10240. HL-C09H
verified Tamil ன from Frame 13's first row in #10244, and HL-C09I verified the
adjacent three-loop ண row in #10249. HL-C09J verified Tamil ந from Frame 12 in
#10252, completing the Tamil starter inventory. HL-C09K opens the smallest
remaining script inventory with Persian ا: UT Austin Persian Online's opening
freehand demonstration shows its isolated Naskh stem descending in one
unbroken movement, and the path is fitted to the vendored Noto Naskh outline.
HL-C09L verifies the adjacent Persian ب as a right-to-left Naskh bowl followed
by one sourced lift and its separate dot. HL-C09M's source audit records that
the intervening Persian-added پ is demonstrated at 00:16–00:21 but absent from
the starter inventory and therefore outside HL-C09's fixed 228-entry count. It
continues to the next actual prose entry, ت at 00:22–00:27: the same bowl, one
lift to the left dot, then another to the right. HL-C09N verifies the later س
row at 01:29–01:35: its three teeth flow right-to-left into the final bowl in
one unbroken movement. HL-C09O verifies Persian ل at 02:29–02:32: its tall
upright descends and turns directly into the leftward base curve without a
lift. HL-C09P verifies the source-adjacent Persian م row at 02:33–02:36: its
round head flows directly into the descending tail in one unbroken movement.
HL-C09Q verifies Persian ن at 02:37–02:43: one right-to-left bowl is followed
by a single lift for the dot above. Its production build also exposed the next
hard prerequisite: the largest eager chunk reached 499,525 of 500,000 allowed
bytes. HL-C09R moves the handwriting model, renderer, and font parser into one
required, independently cacheable chunk while keeping the same ceiling and
synchronous relative-path startup. The largest eager chunk is now 471,927 bytes,
restoring 28,073 bytes of measured headroom. HL-C09S now seeks the
source-adjacent row. The audit corrects the old queue: و, not ه, is demonstrated
after ن. Its 02:43–02:45 small head loop flows into the leftward curving tail in
one unbroken movement. HL-C09T verifies the later ه window at 02:47–02:50: the
source closes one simple handwritten loop without lifting, while the learner
path preserves that run across Noto Naskh's wider two-counter isolated form and
leftward baseline finish. This completes the Persian starter inventory. The
smallest remaining inventory is Urdu, but its first ا collides with Persian ا
under `DUCTUS`'s old glyph-only identity. HL-C09U closes that prerequisite with
script-aware lookup and a scoped Urdu key, so Persian and Urdu ا retain distinct
sources even though both canonical script files use the vendored Noto Naskh
fallback for path checking. Northwestern's *Zer o Zabar* independent-form
animation then verifies Urdu ا as one top-to-bottom continuous stroke; the
lesson explicitly contrasts final ـا, which travels bottom-to-top. The next
smallest tranche remains Urdu. HL-C09V verifies independent ج from the
textbook's dedicated jīm chapter: its animation places the below-dot first,
lifts once, then joins the pointed hooked head, descent, and bowl. The adjacent
insight preserves the flat-head alternative as aesthetic rather than a new
reading or lift pattern. HL-C09W then verifies independent ر as one unbroken
downward-line-then-leftward-curve run from the dedicated *Dāl, re, and wāw*
chapter, while keeping its final-form and Naskh/Nastaliq distinctions explicit.
HL-C09X then verifies independent س from both calligraphic and handwriting
animations: three close teeth flow right-to-left directly into the final bowl
in one zero-lift run. The standard toothed learner path keeps the chapter's
optional long gentle curve explicit as an especially common handwriting
alternative. HL-C09Y verifies adjacent ش: the complete س body comes first,
followed by separately lifted lower-left, lower-right, and centered upper dots.
The chapter's two-below, one-above arrangement and optional toothless body stay
explicit. HL-C09Z then verifies ک from Chapter 1: the main-line stem, flatter
bowl, and pronounced hook stay in one run before a single lift and the long
upper-right slash. HL-C09AA verifies ل from Chapter 2: its tall upright descends
and continues below the baseline through the leftward bowl without a lift.
HL-C09AB verifies م from Chapter 3: its round head and below-baseline tail stay
in one zero-lift run, while the prose distinguishes calligraphy from the
counterclockwise handwritten loop. HL-C09AC verifies ن from Chapter 6: its
below-baseline bowl comes first, one lift precedes the dot near the baseline,
and its initial/medial tooth remains a distinct form. HL-C09AD verifies ہ from
Chapter 4: one counterclockwise, zero-lift loop closes its independent
oval-or-teardrop body, while the other positional forms remain distinct.
HL-C09AE verifies independent ی from the same chapter: both demonstrations keep
its dotless S-shaped body and below-baseline bowl in one upper-right-to-left,
zero-lift run, while the two dots remain exclusive to the initial and medial
forms. HL-C09AF verifies independent ں from Chapter 6: both demonstrations keep
the same right-to-left, below-baseline bowl as ن in one zero-lift run, while the
finished form omits ن's dot and the initial/medial forms remain ordinary nūn.
HL-C09AG completes the Urdu starter inventory with independent ے from Chapter
4: both demonstrations start at the upper right, descend and sweep left across
the broad bowl, curl back underneath at the far left, then continue right along
the lower fold without lifting. The chapter's initial/medial be-series tooth
and independent/final sound distinction remain explicit. That made Arabic the
smallest remaining starter inventory at 21 entries and queued a
source-and-identity audit for its independent ا before another shared Unicode
glyph entered `DUCTUS`. HL-C09AH verifies that Arabic identity independently:
the University of Oregon's *Introduction to Arabic* video draws independent ا
top-to-bottom in one continuous stroke at 00:05–00:07, and the adjacent lesson
identifies alif as a one-way connector. HL-C09AI follows from the adjacent ب
video: the independent bowl starts at its upper-right tip, sweeps continuously
right-to-left, turns up at the left tip, then lifts once for the dot below.
HL-C09AJ combines that separately demonstrated Arabic bowl with the dedicated ت
clip, which opens on the completed body before drawing the left and right upper
dots as two separate strokes. HL-C09AK then audits the page's linked ث asset
instead of trusting its label: the 57-second clip is another Taa lesson whose
first independent form draws a bowl and exactly two upper dots, never the third
dot required by ث. That mismatch is now explicit deferred source debt rather
than an invented path. Arabic remains the smallest inventory; HL-C09AL moves to
the next viable source, ج, whose dedicated video draws the short upper head
left-to-right, continues down and around the bowl, then lifts once for the dot.
HL-C09AM resolves the page's unlinked Haa attachment through its WordPress media
ledger instead of borrowing Jeem's order: the clip finishes a short left stem,
lifts once, then restarts near its top and sweeps continuously around the dotless
ح bowl. The same ledger exposes Khaa media for the next audit.
HL-C09AN confirms that `kha.mov` returns to a body-first order: the short upper
head flows directly around the bowl, then one lift precedes the dot above.
HL-C09AO verifies the next page's independent Daal media: one pen-down run
descends down-right from the upper tip through the curved shoulder, then turns
left along the baseline. HL-C09AP follows the same page's independent Raa
media: one pen-down run descends from the upper tip through the short stroke,
then sweeps left through the lower curve. HL-C09AQ verifies the next page's
independent Seen media: one unbroken run shapes three close teeth right-to-left
and flows directly into the final bowl. HL-C09AR follows the page's Shiin media:
the same uninterrupted body precedes three separately lifted dots in lower-left,
lower-right, then upper-center order. HL-C09AS verifies the page's Saad media:
one run closes the oval clockwise and rises into its short left shoulder, then
one lift precedes a restarted sweep through the trailing bowl. HL-C09AT verifies
the page's embedded Daad lesson: it repeats those two body runs, lifts a second
time, then places the upper dot last. The directly linked `FullSizeRender-5.mov`
returned HTTP 403 during the audit, so the recorded timestamps come from the
accessible embedded primary lesson and are cross-checked against the embedded
Saad sequence. The source-backed **ع غ** page directly links `ayn.mov`, making
independent **ع** the next measured Arabic entry. HL-C09AU verifies its single
unbroken run from the open head into the broad lower bowl. The source-backed
**ي ك ل** page directly links `kaf.mov` and `lam.mov`; HL-C09AV verifies
independent **ك**, and HL-C09AW verifies independent **ل** as one unbroken
upright-and-bowl run. HL-C09AX verifies the same page's directly linked
`yaa.mov`: independent **ي** completes its body without lifting, then places
the lower-left and lower-right dots in separate runs. Arabic U+064A remains
independent of Urdu U+06CC **ی**. The next textbook page for **م ن** contains no
linked handwriting media, while the later **ه و ي** page directly links
`letter-haa.mov` and `waw.mov`. HL-C09AY verifies Heh's intertwined loop and
baseline run; HL-C09AZ verifies that independent **و** closes its head loop and
continues through the leftward tail without lifting. Dhaal is source-available
but is not one of this Arabic ledger's 21 measured entries. The index audit
found **2,075** canonical
candidates across the corpus: **1,426**
word and phrase lessons for English-first lookup, **136** dedicated grammar,
writing, etymology, culture, and pronunciation lessons, **427** chapter
capabilities, and **86** additional handwritten chapter declarations. Practice
drills stay out of the index; the checked title/label manifest provides durable
navigation even where the authored capability ledger is still missing.
Current generated baseline: **22** registered tracks, **1,680** canonical lessons,
**1,594** mapped lessons, and **22** downloadable LaTeX books spanning **513**
chapters, **420** of them generated from the canonical lesson AST. Fifty-eight
of **135** mapped non-lexical lessons across 18 tracks now carry compiled
objective activities, leaving **77**
mapped non-lexical lessons as explicit activity-coverage debt. Chapter 18's
last **10** legacy blockers are replaced by **9** schema-v2 lessons with typed
activity contracts. HL-V01 keeps the remaining migration debt reproducible in both
JSON and human-readable reports; the canonical schema-v2 tranches prove one
typed source across Language Ladder and generated book chapters without
discarding deep content.

## P-1 — The A2 claim was wrong, and the curriculum is being redesigned (HL09)

**2026-08-07.** The project owner, who has sat A2 examinations, did not believe the
gap report's claim that Spanish "reaches A2". The audit agreed, by a wide margin:

| | Spanish today | A2 requires | short by |
|---|---|---|---|
| distinct headwords | **178** | ~1,000–1,500 | **≈6.7×** |
| lessons realizing an A2 node | **14** | — | — |
| A2 spine nodes realized | **1 of 5** | 5 | 4 |
| tenses taught | present only | preterite, imperfect, perfect, near future, imperative | 5+ |

All 14 realize one node, `SPINE-SAY-WHAT-I-DO`, which declares **42 concepts**;
three of the other four A2 nodes declare **one concept each**, and
`SPINE-TALK-ABOUT-PAST` — one concept — stands for the entire past tense.

Three further findings from the same audit, each a defect in its own right:

- **38% of Spanish has no reading order.** 56 of 146 lessons carry no `sequence:`,
  so their order lives only in hand-typed LaTeX. French is worse: 64 of 73 (88%).
  A ramp whose order is unknown cannot be verified at all.
- **51% of taught atoms are never revisited.** 93 of 182; median revisits **zero**.
  `reviews_of` is set on 144 of 146 lessons and cannot close the gap — it names
  lesson ids, while atoms live in another namespace, so it has never reinforced
  anything.
- **Regional variation is absent.** `vos` appears **0 times in 146 lessons**, for a
  language whose everyday second person is voseo for ~100 million speakers.

A close read of Spanish chapters 1–8 (62 lesson files) found the mechanism behind
all of it, and it is not bad writing — chapters 1, 2, 5 and 6 are as well built as
anything commercial, and the etymology is genuinely distinctive:

- **The schema migration stopped at Chapter 6.** Chapters 1–6 carry `sequence`,
  `spine_node`, `duration` and knowledge atoms; chapters 7–8 carry **none** of it,
  so they are invisible to every tool that reads the knowledge graph. The ramp
  collapses exactly at that boundary.
- **`type: review` lessons: zero in the whole corpus**, though HL00 defines the
  type and the N+1/N+3/N+7/N+15 interval. `session-map.md`, the artifact HL00 says
  verifies the schedule, **covers chapters 1–3 of 33.**
- **Chapters 7–8 drill vocabulary from Chapters 26 and 31** — *pan*, *agua*,
  *un/una*, *veintiuno*, *¿Cuánto es?* — because chapters starved of reviewable
  material reach sideways for whatever they need.
- **The learner cannot say "no"** (`sí`/`no` are Chapter 19; they are questioned
  from Chapter 7) **or "I am"** (`estoy` used as a given in Chapter 4, taught
  nowhere; `yo` omitted from the entire path by ledger design).
- **Every lesson is `variety: general`.** The Spanish taught is unmarked, and the
  `tú`/`usted` lesson presents a two-way system as universal.
- **Chapter 7's order is genuinely ambiguous** — `curriculum.json` says
  comer→beber→qué→vivir→dónde; the lesson prose and `reviews_of` say
  comer→vivir→beber→qué→dónde. **This one needs a human decision.**

> A gentle ramp is not made of small steps; it's made of steps you can still
> stand on.

**Measured 2026-08-07, before authoring any review lesson:** R1 is n+1…n+3, so a
chapter-END review cannot close it for 11 of Spanish's 35 chapters (median 4
lessons, max 15) — which is why the existing `practice-mix` lessons never closed a
window. And **99 of Spanish's 114 R1 misses are never revisited at any distance**,
so this is an absence, not a scheduling error. HL09 §7.2 records the consequence:
R1/R2 must be closed by wiring `practises.knowledge` on the *teaching* lessons
(≈0 new lessons), and dedicated `review` lessons are for R3/R4 only. The naive
reading — one review lesson every three lessons — would have added ~50 lessons to
Spanish and ~2,600 at the 8,000-lesson target to say what one frontmatter line says.

[HL09](../../specs/HL09-gentle-ramp-curriculum-design.md) is the redesign. It sizes
the work honestly (**~8,000 lessons to C2**, against 146 today), and names **four**
ramps where the repo measured one: vocabulary, script, **sentence complexity**, and
**synthesis**. Length is not a cost; the owner has confirmed thousands of pages are
acceptable. Nothing may claim a level until HL09 §3.1 is satisfied.

## Priority rules

1. Close a learner-visible broken promise before adding breadth.
2. Prefer work that makes later corpus growth measurable or generated.
3. Finish a small vertical slice before starting the same migration everywhere.
4. Keep the application, book, and canonical lesson content aligned.

## The owner's direction, 2026-08-07

Recorded here because the repository, not an agent session, is the source of truth.
These restate and sharpen the program; where they conflict with an older item, these win.

1. **One core JSON is the source.** Book *and* app derive from it. It already records
   voice-vs-read modality (`core/lesson-modality.json`) so a voice assistant can teach
   while the learner drives — that stays first-class.
2. **The spine is shared across languages.** The same gentle ramp, in the same order, in
   every track, so a reader can pattern-match across languages.
3. **pre-A1 → C2 for every language.** Where a language has no CEFR exam, derive an
   equivalent ladder rather than leaving it unmapped. `core/spine.json` today has 16
   nodes and **zero above A2**, which is the binding ceiling (HL-C10).
4. **The ramp includes the script**, not just vocabulary. Sometimes you cannot introduce
   more than one script at a time. → HL-C18C, HL-C18D.
5. **Length is free.** Books may run to thousands of pages. Never trade gentleness for
   brevity; split rather than compress. The five-minute contract is per *lesson*, not per
   book, and no gate may penalise page, lesson, or chapter count.
6. **Re-emphasise constantly.** Keep resurfacing earlier material to build confidence.
7. **English is the only requirement for each book.** Same-family cousin material is an
   *additional* layer for readers who know a relative — a bonus, never a prerequisite, and
   never something that gates comprehension or inflates the script ramp. → HL-C48.
8. **Every chapter opens with a short, well-written intro.** Not "you learnt this in
   Hindi" cross-track name-drops, which dangle in a standalone single-language PDF.
   288 of 393 chapters currently have no intro at all. → HL-C49.
9. **The book is a standalone artifact.** No repo paths (98 chapters print
   `lessons/XX-CNN-*.md` at the reader today), no app assumptions, no dangling cross-track
   references, and the front/back matter a book is expected to have. → HL-C50.

## P0 — The Spanish pre-A1 → C2 course (HL10)

**2026-08-10.** [HL10](../../specs/HL10-spanish-pre-a1-to-c2-course-architecture.md)
is the course architecture HL09 implied but never wrote. HL09 fixed how gentle a
step must be; HL10 says what the steps are — eight parallel strands, a spine
rebuilt to ~400 nodes, a grammar ramp expressed in ~630 individually-taught verb
cells at one cell per lesson, etymology as a productive system with a payoff
ledger, and a stage-by-stage map of ~4,950 lessons across ~851 chapters
(≈10,500 pages, seven volumes).

Three measurements from the current corpus that set the starting point:
Spanish's 188 lessons contain **one** distinct culture atom, **zero** mentions of
*vos* (a form used by ~100 million speakers), and **188 of 188** lessons declare
`variety: general` — the unmarked default HL09 §8.1 forbids.

| ID | Status | Work item | Completion signal |
|---|---|---|---|
| HL-C79 | Partly complete — **measurement already shipped**; Spanish order is clean | Land the remainder of HL09 steps 1–3. **Re-measured 2026-08-10:** `measureContinuity` already reports order integrity, the R1–R4 windows and forward references, and Spanish measures **0** lessons without `sequence`, **0** forward prerequisites and **0** forward reviews — steps 1 and 2 are discharged for the pilot track. What remains is step 3 and the corpus beyond Spanish. | Spanish: close its 65 never-revisited atoms and 55 forward references by wiring `practises.knowledge` on teaching lessons (HL09 §7.2 option (b), which costs no new lessons). Corpus-wide: 477 lessons across 17 tracks still declare no `sequence`, and the R1/R2 miss counts (891/1816) are untouched. |
| HL-C80 | Complete (#10497) | Add the strand dimension to `spine.json` and the HL10 §2.2 budgets to `chapter-policy.json`, report-only. | All 33 nodes carry one of eight declared strands; `strands.ts` measures the distribution and `report-cli` prints it. First snapshot: FUNCTION 14, GRAMMAR 7, LEXICON 2, SOUND 0, ETYMOLOGY 0, CULTURE 3, IDIOM 0, TEXT 7 — **three declared ladders have no nodes on them**, ETYMOLOGY most sharply, which HL00 calls the signature of this curriculum and 708 lessons carry as prose no node promises. The seven budgets ship optional and report-only. |
| HL-C94 | Complete — verified against the corpus, not inferred | Split the four over-budget opening chapters. | **Measured after HL-C85:** ch3 27 atoms/15 lessons, ch4 31/15, ch5 17/7, ch6 19/9, against a 12-atom budget. This is the ramp the owner has always complained about, and re-sequencing cannot fix it — the chapters must become more chapters. **Seams, chosen from the data:** ch3 → *saying your name* (6) / *the three words for you* (9) / *asking someone's name* (12). ch4 → *thank you* (4) / *how are you* (13) / and its six W-lessons are a whole separate writing strand crammed in, which becomes *accents* (5) / *ñ and the upside-down marks* (9). ch5 → *goodbye* (8) / *until when* (9). ch6 → *a coffee, please* (6) / *your first verb* (5) / *more -ar verbs* (8). Four chapters become twelve; every resulting chapter lands inside budget. **The blocker is authoring, not renumbering.** HL05 requires every chapter to carry a payoff lesson, and all 41 do today; the splits produce **8 chapters with no payoff**, so 8 new payoff lessons must be written before the split can pass its own gate. Renumbering ch7-41 → ch15-49 is mechanical by comparison. Do the payoffs first, then the renumber, then regenerate and re-pin — and run `language-ladder` before pushing, per the standing rule. **Verified 2026-08-12, because the status said *planned* while three test files cited it in the past tense.** Every seam this row specified now exists as a chapter: *Saying Your Name* (4), *The Three Words for You* (5), *Asking Someone's Name* (6), *Thank You, Yes, and No* (7), *How Are You* (8), *The Written Accent* (10), *Enye and the Upside-Down Marks* (11), *Goodbye* (12), *Until When* (13), *A Coffee, Please* (14), *Your First Verb* (16) and the *-ar* run from 20. The work landed; only the status never moved. |
| HL-C81 | Queued — **strategy revised after measuring** | Split every oversized spine node, starting with `SPINE-SAY-WHAT-I-DO` (42 concepts → nine nodes, HL10 §3.3). | **Measured 2026-08-10: the blast radius is 328 lessons across 20 tracks and all 22 `curriculum.json` files** (Spanish 37, Latin 31, Portuguese 22, Italian 18, German/French 16 each, then 8–14 apiece). Re-pointing each lesson is not mechanical — it needs an editorial judgement about which person and conjugation that lesson actually teaches, so a bulk rewrite would silently invent facts. Revised plan, per BACKLOG priority rule 3 (finish a vertical slice first): (a) author the nine new nodes alongside the old one, (b) migrate **Spanish only** — 37 lessons, each judged against `grammar-cells.json` — and prove the level gate still computes, (c) migrate the remaining tracks one PR each, (d) delete `SPINE-SAY-WHAT-I-DO` once no lesson names it. The node stays over-ceiling and report-only until (d). |
| HL-C82 | Complete (#10501) — regular cells; overlays in HL-C91 | Author the Spanish grammar cell inventory (HL10 §5.1) as data. | 231 language-neutral slots in `core/grammar-slots.json` (144 finite, 30 imperative, 48 compound, 9 non-finite) and Spanish's filling in `spanish/grammar-cells.json`, with a validated prerequisite DAG — 4 roots (the three infinitives and *hablo*), max depth 15, no cycles, no dangling edges. Generated by `data/generate_grammar_cells.py` and drift-gated in `BUILD`. Corpus coverage measures **0 of 231**, deliberately not inferred from atom names. The ~400 irregular and stem-changing overlays are **not** included — see HL-C91. |
| HL-C83 | Complete (#10503) | Build the Root Ledger over the existing etymology references (HL10 §6.2). | `root-ledger.ts` counts **payoffs, not mentions** — an introduction scores zero — across both namespaces (1,966 `roots:` slugs + 751 `*-ETYMON-*` atoms). First measurement: **2,717 roots, 2,624 spent fewer than three times (97%), 1,807 never spent at all.** Spanish: 303 / 290 / 190. This is the burn-down list HL-C88's friends layer needs, since a root with recorded payoffs already knows which later words it predicts. |
| HL-C84 | Complete (#10507) | Enforce the info-dump gate (HL10 §7.3), report-only. | `info-dump.ts` measures rule statements and paradigm tables. **The prose is fine — 17 rule statements across 1,694 lessons.** The dumps are in tables: **70 lessons carry a paradigm-shaped table, 18 of them a full grid** (`FR-C05-parler`, `GE-C05-wohnen`, `ES-C17-practice` each present a complete six-person conjugation at once). Flags shape, not size — 470 tables have ≥3 rows and most are fine. Burn-down is HL-C92. |
| HL-C85 | Queued — spot-check suggests it landed; needs the same verification HL-C94 got | Absorb the existing 188 Spanish lessons into the rebuilt spine (HL10 §13). | Zero forward references and zero dead-end atoms in Spanish; the learner can say *no* and *I am*; nothing is deleted. |
| HL-C86 | Queued — **Chapter 1 drafted as a sample in HL10 Appendix A; opens on `hola` per the usefulness rule** | Author pre-A1 to its full 30 chapters / ~180 lessons. | HL09 §3.1 satisfied at pre-A1; ≥95% of lessons `voice`; every chapter drivable from its first lesson, with an intro, an etymological thread, and a culture note. |
| HL-C87 | In progress -- §10.1 and §10.3 COMPLETE; §10.2 playback done, recognition remains | Per-atom mastery and voice mode in Language Ladder (HL10 §10.1-10.2). | The app schedules from atom strength rather than lesson completion; a full chapter is completable hands-free. **Slice 1 landed: the learner's per-atom record now exists and is being written.** `atommastery.ts` is the pure engine -- strength 0..1 moving asymptotically on a hit and multiplicatively on a miss, decaying with a **10-day half-life**, with a cubic interval so the gap between *just met it* and *know it cold* is days rather than a constant factor. Every function takes `now` as an argument and touches no storage, which is what lets a test watch a month pass without waiting for one. `masterystore.ts` persists it under its own key in the `reviewstore.ts` style: untrusted blob, every field clamped, a bad row dropped rather than fatal, a wrong schema version dropped rather than guessed at. **The distinction the spec cares about is now real in the code:** the corpus's R1-R4 windows guarantee the *material* to practise every atom and are the same for everybody; this record is the *learner* schedule. Conflating them is why `reviews_of` never reinforced anything **for anybody in particular**. **Wired at both answer paths.** An authored activity credits exactly its `assesses` list -- the atoms the learner was actually tested on, not the whole lesson. A meaning check has no `assesses`, so `Lesson` now carries `introducesAtoms` from `introduces.knowledge` and credits what the lesson exists to teach. **Verified in a browser, not only in tests:** answering chapter 1's check wrote `ES-LEX-HOLA` and `ES-SOUND-H-SILENT` at strength 0.58 with a real due date ~35 days out; the record survived a reload; a wrong answer dropped it to 0.20 and counted a lapse. **Remaining:** (a) the scheduler still runs on lessons -- `dueAtoms()` and `heldAtoms()` exist and nothing consumes them yet, which is deliberate sequencing (the record has to be trustworthy before anything depends on it) but is the actual §10.1 ask; (b) §10.2 voice mode, which needs TTS/ASR and is a much larger slice; (c) §10.3 synthesis-drill generation, which `heldAtoms()` was written for. **Slice 2 landed: §10.1 is complete -- the app now schedules from the record.** `atomschedule.ts` picks review by **greedy set cover**: take the atoms actually due, repeatedly choose the completed lesson refreshing the most of them, remove those from the pool. Not optimal -- no cheap algorithm is -- but within a known factor, trivially explainable (*these three lessons cover most of what you owe*), and never pathological. **Predictability is worth more than optimality here**, so ties break on lesson id and the same book + clock always give the same queue; a review list that reshuffles between renders is unusable. Only *completed* lessons are candidates: offering an unstudied one would leak the course's forward order and hand the learner a lesson whose prerequisites they do not hold. The Learn view now opens with a **Due for review** section that says *what each pick refreshes*, because 'review this' with no reason is what makes review feel arbitrary. **A bug worth recording, because I wrote the warning and then violated it.** `atomschedule.ts`'s own comment says crediting and scheduling must agree about what counts, *or an atom scheduled by a lesson that will never credit it stays due forever*. The first implementation scheduled on activity `assesses` only, while the meaning-check path credits `introducesAtoms` -- so **ES-C01-hola, the very first lesson of the course, could come due and never be clearable.** It only surfaced because I drove the built app in a browser and the section did not appear; the unit tests were all green. `refreshesOf()` now unions both sets and a test pins the invariant. **Remaining:** §10.2 voice mode (TTS/ASR, a much larger slice) and §10.3 synthesis-drill generation, which `heldAtoms()` was written for. **Slice 3 landed: §10.3 synthesis drills.** HL09 §6 puts one synthesis activity at the end of every chapter -- a prompt whose answer is an utterance the course has never shown -- so there is exactly **one per chapter** and a learner who wants more has nowhere to get them. `synthesisdrill.ts` generates more from the learner's own record, and the constraint is **the forward-reference rule (§8.2) run backwards**: where the corpus checks no lesson uses a word it has not taught, this checks no drill uses an atom the learner does not currently **hold**. A drill is 2-4 held pieces from **different domains** -- three food words is a vocabulary quiz; a food word, a time word and a verb is a sentence you have to build. The corpus already carries what is needed: `concept_tag` is domain-prefixed (`ES-FOOD-WATER`, `VERB-EAT`), and grammar-only tags are excluded because *use a grammar rule in a sentence* is not an instruction anybody can follow. **Two things it deliberately does not do, both recorded in the module rather than faked.** (1) The spec's example prompt is *situational* -- "you are hungry, it is morning, speaking to your boss" -- and that needs per-atom situational metadata the corpus does not carry. A food word is tagged as food, not as *something you eat when hungry*; inventing the situation would mean inventing facts, so the prompt names the pieces plainly. **That is a corpus change, not a generator change.** (2) An open utterance cannot be graded without a parser this app does not have, so the check tests the one thing that is mechanically decidable and is also exactly what the drill claims: **did the answer contain each piece?** The verdict says so out loud rather than implying it graded the Spanish. **A design constraint found only by driving the built app.** Learn mode keeps just the frontier and completed lessons in memory -- for a beginner, **two Spanish lessons** -- so a drill built from what was loaded could never find two domains and silently never appeared. Every unit test was green and the generator was proven correct against the real corpus in Node; the UI was simply looking at a 2-lesson slice. It now pulls the full corpus in the background once the learner holds ~6 atoms, and offers no drill until it lands rather than a wrong one. **Slice 4 landed: §10.2 voice PLAYBACK.** Almost none of this was new work, and that is the finding: **the narration generator has been emitting typed segments all along** -- `pause` with seconds, `speech` with text, `prompt` with an instruction and a response budget, `activity` with its accepted answers, `table` pre-flattened into sayable rows, `repeat` with a count. So `voicescript.ts` parses nothing; it walks a structure the corpus already guarantees and flattens it to a list of instructions. **Flat, not a tree**, because a player that walks a tree makes every feature (skip, resume, repeat) tree surgery, while a flat list with an index makes them arithmetic. `voiceplayer.ts` drives it through `speechSynthesis`, with a BCP-47 tag per track (Latin gets Italian -- no TTS voice exists and it is the closest living phonology). **The subtle bug it is built against:** `speechSynthesis.cancel()` **fires pending `onend` handlers**, so a naive player advances one more step after being stopped. Each run holds a cancel token and a stale handler does nothing; there is a test that reproduces the browser's exact behaviour. **Recognition is deliberately absent, and that is stated in the module rather than hidden.** `SpeechRecognition` needs a microphone permission, is prefixed in most engines and missing in some, and **I have no microphone to verify it with** -- wiring it blind is how you ship a feature that works only on the machine it was written on. A `respond` step waits its authored budget and moves on, which is exactly what a cassette course did and is genuinely useful to somebody driving. **Verified in the browser** by intercepting `speechSynthesis.speak`: the button flips to Stop, utterances are queued in `es-ES` beginning with the lesson title then the block title, the running commentary line tracks the current step, Stop restores the button and **no further utterance is queued after it**, and leaving Learn mode stops the audio. **Remaining: recognition and scoring** -- the learner's half of the loop, which needs a device the person testing it can speak into. |
| HL-C90 | Complete (#PENDING) | Neutralize control characters in every corpus-derived string the gap report writes to stdout. | A node id or finding message containing an ESC sequence currently rewrites its own line in a terminal, so a crafted id can hide the very defect line a reviewer is reading to judge whether the data is sound. Package-wide, not local: `report.ts` interpolates `finding.message` exactly as `strands.ts` interpolates `nodeId`. Strip `/[\x00-\x08\x0b-\x1f\x7f]/g` in the shared render helpers, with a fixture proving a crafted id cannot erase a line. |
| HL-C88 | In progress -- all eight endings, their review and synthesis, and false friends landed; generated cousin panels remain | Build the **friends** layer (HL10 §6.7): English cognates, the eight systematic suffix correspondences, hidden friends via the sound laws, and generated cousin panels from `concept_tag`. | Every lesson may carry at most one friend; friends never count toward the ≤3 atom budget; the cousin panel is generated from the 1,131 `concept_tag` lessons rather than hand-typed (closes the authoring half of HL-C48); every asserted friend has a defensible etymology or is taught as a false friend. **First slice: the three highest-reach systematic correspondences, at chapters 23-27** -- placed immediately after `español`, which is early enough to pay off across the remaining 150 chapters and gives the `es-`/`s-` rule its example for free later. HL10 calls these *the highest-value lessons in the whole ETYMOLOGY strand*, and the arithmetic is why: **`-ción`/`-tion` (~2,000 words), `-dad`/`-ty` (~1,200), `-mente`/`-ly` (unbounded)**. Three chapters reach further than every vocabulary lesson in the book combined. Each is grounded rather than asserted: `-ción` and `-tion` are both Latin ***-tiōnem***, **inherited** by Spanish and **borrowed** by English -- the same word arriving from two directions, which is why it is a rule and not a resemblance. `-dad` looks *less* like `-ty` for a reason worth giving: English took *-tātem* through French where it had eroded further, so it is the same relationship with more wear on one side. And `-mente` **was a feminine noun** meaning *mind*, which is the entire explanation for the otherwise-arbitrary feminine adjective: *claramente* is *with a clear mind*, and English `-ly` hides *līc*, 'body', the same way. The review separates **decoders** (fixed reach, let you *read*) from **machines** (unbounded, let you *build*). The synthesis has the reader read *La universidad tiene una reputación internacional, naturalmente* -- **four words the book has never taught** -- and warns at no cost that the endings are reliable rather than perfect, so the false friends land later as memorable instead of as a betrayal. **A corpus rule learned here:** `type: pattern` is not 'a lesson about a rule'. The gate requires **exactly one introduced atom, named `-PATTERN-`**, plus slots and a production -- it is reserved for slot-filling productions. An ending correspondence is not one, so these are `type: grammar`, and the pattern-lesson test now says why rather than just listing one id. **Second slice: hidden friends via the sound laws**, at chapters 28-30. A systematic ending is a *suffix* rule; a sound law reaches words that carry no shared suffix at all, which is why it is the other half of the friends layer. The audit that preceded the writing changed the design twice. **`-CT- -> -ch-` was already taught at chapter 2** (`ES-C02-noche`, with *hecho*, *leche*, *ocho*), so it is reviewed here rather than introduced. And **`f- -> h-` was already introduced at chapter 6**, where `ES-C06-hablar` explicitly defers the general case in its own prose -- *the single path you need today is fabulari -> hablar; later words will let you test how broadly that pattern reaches.* So chapter 28 introduces `ES-SOUND-F-TO-H-DECODER`, the *general* rule, and practises the atom chapter 6 planted: the promise the book made 22 chapters earlier is now paid, in the same words. *Hablar* -> *fabulari* opens *fable*, *fabulous*, *affable*; and the lesson names its own limits, because *hola* and *hasta* are not dead *f*s and *facil*, *fuerte* and *forma* kept theirs. Chapter 29 takes `cl-`/`pl-`/`fl-` -> `ll-` on *llamar* <- *clamare* (English *claim*, *clamour*). Chapter 30 is a synthesis over all six decoding rules the book now holds, and it is deliberately a **decoder**, not a machine: it lets the reader *read* words the book will never teach. **Third slice: `es-` + consonant, at chapter 30 -- and a census picked it out of the five rather than the plan doing so.** The standing rule is to measure the lexical inventory at the insertion point before authoring a rung, and the measurement was decisive: **`es-` + consonant has 11 taught words across the book and 5 by chapter 31** (`español`, `estar`, `estudiar`, `está`, `estás`), while **`-ncia`, `-oso`, `-ario` and `-ismo`/`-ista` have ZERO taught words in all 181 chapters -- not one, at any point in the book.** Authoring those four now would ship a decoder with nothing in the book to decode, which is the noun-famine failure this project has already paid for once. So one rung was written and four are **blocked on vocabulary, not on authoring effort**; each needs words scheduled before it, and that scheduling is the actual next task. The rung that did ship is the strongest of the five anyway: it explains two verbs the reader has been using since chapters 4 and 6. Latin could begin a word with *st-*, *sp-*, *sc-*; Spanish could not, so it put an *e-* in front -- *stāre* → *estar*, *studēre* → *estudiar* -- while English borrowed the same words bare, which is why **state**, **stay**, **study** and **student** are sitting inside words the reader already says. The rule is stated backwards because backwards is the useful direction: **take the *e-* off and an English word is often standing there.** It also **stacks** with the endings: *estación* gives up both ends at once, *e-* off the front and *-ción* → *-tion* at the back, which is the first time the arc's rules compose. And it names its own limit: ***español* is not an example** -- its *Es-* is what is left of Latin ***Hispānia*** whose *H* was already silent, not an added vowel, so there is nothing to take off. The chapter-31 synthesis grew from six rules to seven and now says the rules stack. **Fourth slice: professions, at chapters 51-54 -- and it started as vocabulary scheduling for a blocked ending and found a hole in the curriculum instead.** The measurement: across **322 taught word forms there was not one profession noun**, and person-nouns were family-only (*madre*, *padre*, *hermano*, *hermana*). Meanwhile *ser* is taught at chapter 48 as *the verb used to identify someone or say what they are*. **The verb was taught and the sentence it exists to enable could not be said.** *Soy profesor* was unreachable. So this slice is not decoration around an ending -- it closes a core pre-A1 function that had been open for forty chapters. Four chapters: **profesor** (with the bare-noun rule -- after *ser* a job takes no *un*, which is the one place Spanish drops the article English insists on, and *profitērī* 'to declare publicly' explains **profess** and **profession**); **estudiante** (**built, not memorised** -- cut *-ar* from *estudiar*, add *-ante*, and the same ending gives English *particip-ant*, *assist-ant*; it is also *es-* + consonant, so two of the reader's rules land on one word); **-ista** (the blocked ending, now grounded -- *artista*, *dentista*, *turista* need no teaching at all, and *periodista* is the interesting one because the ending is regular while the root, *periódico*, is the surprise); and a **synthesis** where the reader says what they are and turns the question back with *¿Y tú?*. Both new endings are **invariant** -- one form for a man or a woman, only the article moves -- which is a real simplification and worth saying out loud. **Two gates earned their keep:** the corpus forbids naming a literal chapter number in prose and caught *since chapter 6* (which any later insert would falsify), and the validator requires a required atom to be reachable through the **prerequisite chain**, not merely taught earlier in reading order -- so the *es-* and *-ción* tie-ins had to become real prerequisites rather than assumed ones. **Fifth slice: the first describing words, at chapters 56-58 -- and the measurement found a hole even larger than the professions one.** Scheduling vocabulary for `-oso` meant counting the adjectives, and there were effectively none: **the book's only describing words are the colours, and they do not arrive until chapter 130.** Everything before that is *bien* and the fossilised *buenos días* / *buenas tardes* greetings, which are frozen phrases rather than productive adjectives. So a reader crossed **129 chapters** -- the whole present tense, *ser*/*estar*, the preterite, the future, the subjunctive, commands -- **without being able to say that anything was big, small, good, bad, new, old or tired.** And the sharpest part: ***ser* vs *estar* is taught at chapter 48**, and the entire point of that contrast is a quality versus a current state -- a distinction that **cannot be demonstrated without adjectives**. Exactly the shape of the professions gap: the grammar was taught and the words it operates on did not exist. Three chapters now sit immediately after *ser*/*estar* completes: **grande** (the first word for what a thing is *like*, plus the rule that a describing word follows the noun, and *grandis* 'full-grown' giving **grand**, **grandiose**, and the *grand*mother who is the full-grown one); **cansado** (a *state*, so *estoy cansado* and never *soy* -- the first word that makes the *ser*/*estar* contrast concrete, and deliberately a word whose **decoder finds nothing**: *campsāre*, 'to turn aside off the road', yields no English cousin, which is taught on purpose so the reader meets the rules' limits as information rather than as failure); and a **synthesis** on choosing the verb and making the ending agree, where the last letter tells you -- *-o* shifts, *-e* and *-ista*/*-ante* sit still. **Sixth slice: `-oso`, at chapter 59 -- and it landed by correcting a verdict this same backlog row had recorded.** `-oso` was written down as *blocked on vocabulary* because the corpus taught no `-oso` words. **That criterion was wrong for a decoder ending.** The value of `-oso` is reading words the book will never teach -- *famoso*, *delicioso*, *curioso*, *precioso*, *nervioso* are all transparent to an English speaker -- exactly as `-ista` was. What actually blocked it was that the book had **no adjectives at all**: no position rule, no agreement, no *ser*/*estar* choice to put one into. The adjective arc unblocked it, and the lesson is worth keeping straight: *does the corpus teach words ending in X* is the wrong question for a reading rule; *can the reader do anything with an adjective once they have one* is the right one. The chapter is deliberately built on the arc beneath it -- an `-oso` word goes after the noun, ends in `-o` so it shifts, and takes *ser* or *estar* by meaning (*es famoso* against *estoy nervioso*), none of which is restated because all of it was just taught. Latin **-ōsus** meant *full of*, so *famōsus* was full of fame; English `-ous` is the same suffix arriving through French, the same two-roads argument as `-ción`/`-tion`. And it names its limit: *hermoso* is *beautiful*, not *hermous*, because the ending is reliable while the **root still has to be shared**. **A defect of my own, found by the forward-reference checker and worth recording as the immediate next task.** The counter moved 446 -> 449 on this slice and I nearly pinned it up. It was right and I was wrong: the `-oso` lesson demonstrated agreement with *una comida deliciosa* and *una casa grande*, and **`comida` and `casa` are not taught for another 23-25 lessons**. Rewriting the examples to use *un profesor famoso* / *una profesora famosa* -- words the professions arc already taught -- took the lesson to **zero** forward references and the corpus total back to 446, so the pin never needed to move. **Nine remain, and they are mine too:** `ES-C09-grande` (4: *libro*, *casa*), `ES-C09-cansado` (2: *casa*), `ES-C09-sintesis-describir` (2: *casa*), all from the previous slice, plus `ES-C57-es-inicial` (*estudiante*, 44 early -- arguably legitimate, since a decoder lesson exists to read words the reader has not been taught). **The root cause is the same failure this row keeps documenting, and this time I committed it:** the adjective arc teaches *a describing word goes after the noun* at chapters 56-58, and the book's concrete nouns (*casa*, *libro*, *comida*) do not arrive until the 80s. I taught grammar ahead of the vocabulary it operates on, having just written two commits about not doing that. **The fix is scheduling, not prose:** move two or three concrete nouns before chapter 56, then rewrite the four lessons' examples onto them. Doing it as prose-only edits would leave the arc demonstrating noun-adjective order with almost no nouns. **Done (HL-C112), and the scheduling fix was the right one.** `la casa` and `el libro` moved from chapters 70-71 to **56-57**, directly ahead of the adjective arc, which now opens with two concrete nouns to describe. The four lessons' examples needed **no rewriting at all** -- *una casa grande*, *el libro grande* and *la casa es grande* simply became legal, which is the tell that the defect was ordering and not prose. `la comida` stayed at 72 because it requires `ES-GRAMMAR-HACER-PRESENT-SINGULAR`, taught at 65. **Forward references: 446 -> 438**, and the arc itself is at zero; the extra four beyond my own were other lessons whose early use of *casa* and *libro* is now taught rather than assumed. **The move cost more than it looked like it would, and the reason is worth recording.** Pulling two segments out of the middle of `SPINE-DEFINITE-REFERENCE` **severed the prerequisite chain for 17 downstream lessons**, which had been reaching *hacer*, *querer* and *decir* transitively *through* those segments rather than declaring them. 32 atom dependencies broke at once. The repair was to add the introducing lesson to each affected `prerequisites` list, which is an improvement on its own terms: those lessons genuinely require those verbs, and depending on an incidental path is exactly the fragility this demonstrated. **A path is a chain, so moving anything out of its middle is not a local edit** -- check the downstream chain before moving a segment, not after. **Seventh slice: `-ncia`, at chapter 62**, and it was unblocked by the same correction that freed `-oso`: a reading rule does not need the corpus to pre-teach words ending in it. Latin **-entia** turned a quality into a noun -- *differentia* was the state of differing -- and English took the same ending through French, worn down to `-nce`. *diferencia*, *importancia*, *experiencia*, *distancia* and *paciencia* are five words the reader can now read without being taught any of them. **It is placed to compose rather than to accumulate:** every `-ncia` word is a noun and every one is feminine, so it lands directly on the articles and the noun-gender rule; and because the adjective arc now sits just before it, *una diferencia grande* is sayable using three things taught separately -- a noun built by an ending, an article chosen by gender, and a describing word in Spanish position. Its limit is a **near miss** rather than a failure: *ciencia* is **science**, not *cience*, because the ending behaved exactly as promised and it is the root English spells differently. The rule the lesson draws from that is *trust the ending and check the root -- almost right is usually right*, which is a more useful habit than the flat warnings the earlier ending lessons gave. **Eighth slice closes the set: `-ario` at chapter 63, plus the review and synthesis the whole arc had been missing.** Only the first three endings ever got a review and a synthesis (chapters 26-27); the five added since had none, which left the reader holding eight rules and no place where they were ever assembled. `-ario` itself is the odd one out and the lesson says so: Latin **-ārius** meant *belonging to*, then *the place where a thing is kept* -- a *diccionario* is the place the words are kept, which is exactly what an English **dictionary** is and neither language says out loud -- and unlike the other seven it makes **both** nouns and adjectives, with the sentence deciding which. **The review sorts the eight by what they make rather than by when they were met** (thing / person / describing word / either), which is the useful axis, and it names the two things that save a decision every time: **gender is decided by the ending, not by the reader** (`-ncia` always feminine, `-ista` and `-ante` never changing), and **reach differs enormously** (`-mente` unbounded, `-ción` about two thousand words, `-ario` far fewer -- all worth having, not all worth the same). **The synthesis is the payoff the arc was built for:** a paragraph of Spanish the book has never taught -- *La universidad tiene una reputación extraordinaria. Es necesario estudiar con paciencia, naturalmente, pero la diferencia es enorme.* -- read with seven of its eight content words decoded by rule, and the eighth (*enorme*) guessed correctly, which is the point: the rules make the guessing safe. **Ninth slice: false friends as a formal block, at chapter 66** -- placed immediately after the synthesis, where the rules are freshest and the reader most trusts them, which is exactly when the counterweight is worth most. **The framing is the content:** a false friend is not a word that *breaks* the rules, it is a word that **obeys them and still means something else**. A word that resists every rule merely looks foreign and gets looked up; a false friend hands you an answer with a straight face. *éxito* (success, not exit), *actualmente* (currently, not actually), *librería* (bookshop, not library), *embarazada* (pregnant, not embarrassed), *ropa* (clothes, not rope) -- **and none of them is a coincidence.** Each is a shared root that split: *exitus* was *a going out*, and English kept the door while Spanish kept the outcome; *actual* meant *of the present moment*, which Spanish still uses and English drifted away from. So the block teaches history, not a warning list. **What it adds operationally is a second question, asked after the ending answers: does the meaning fit what is being said?** *Estoy embarazada* from someone apologising is not *I am embarrassed* because the sentence would be strange -- the ending offered a candidate and the context rejected it, which is what a reader already does in English with a two-sense word. Five false friends is a **habit**, not a list. **Every word in the lesson was grepped against the whole track before it was written** (all twelve candidates returned zero hits), which is the discipline the previous slice's false claim earned. **The last item is now specified correctly, and the spec was wrong.** HL10 §6.7 said cousin panels could be generated from `concept_tag`. Measuring the join before building it showed the two keys make **different claims**: a cousin panel asserts *reflexes of the same etymon*, while `concept_tag` joins lessons teaching the **same idea**. `VERB-GO` returns *ir · andare · aller* -- Spanish *ir* from *īre*, Italian *andare* from *ambitāre*, French *aller* from a third source -- three unrelated verbs that a generated panel would present as relatives. The `roots:` join returns *la hora · heure · ora · hora*, all four reflexes of *hōra*. **Building on `concept_tag` would emit false etymology at scale, in the one layer whose entire value is that its etymology can be trusted** -- the same class of error as the two etymological mistakes caught in slices 8 and 9, but automated and repeated across the corpus. §6.7 and the §12 checklist now say `roots:`. **Reach: 64 Spanish lessons** carry a `roots:` slug shared with another Romance track (`concept_tag` would have offered 63, so the correct key costs nothing). That is the honest ceiling to design the generator against, and the implementation is the next task. **The join now exists as `src/cousins.ts`, and building it surfaced the real open question.** `buildCousinIndex` / `cousinsFor` answer *which lessons in other Romance tracks teach a reflex of the same etymon*, keyed on `roots:`, excluding the lesson's own language, excluding **Latin** (an ancestor is not a sibling, and the etymology block above already names it), taking one word per language (earliest by reading order, so panels stay stable as lessons are added) and emitting a fixed language order (corpus order would churn every book hash for nothing). **Measured reach: 76 Spanish lessons**, higher than the 64 estimated from the spec measurement because a lesson can carry several roots. **But 76 is the join's reach, not the number of panels worth printing.** A headword is often a phrase that merely *contains* the relative: `bien → italian buongiorno · portuguese bom dia` reads as a claim that *bien* and *buongiorno* are the same word. Restricting both sides to single-token headwords cuts the reach to **25** and makes those 25 genuinely good -- *día/giorno/dia*, *estar/stare*, *trabajar/travailler/trabalhar* -- but it does **not** fix *bien*, because *buongiorno* is one token and the shared root slug is simply coarse. **So the display rule is left as an explicit decision rather than baked in**, and both numbers are pinned in the test so the quality-versus-coverage trade-off stays visible instead of being silently chosen. The rendering change is the next task and needs that decision made first. A first draft of `rootSlugs` also assumed `frontmatter.roots` was a string when the parser returns an **array**, and silently matched nothing corpus-wide; the tests that caught it were the positive ones, since every *refuses-to-pair* assertion passed vacuously against an empty result. **Review then found four more, three of which matter.** (1) **The earliest-by-reading-order rule was inoperative.** Only 41 of 105 French lessons carry a `sequence:`, so candidates routinely tied at the sentinel and the winner fell to whichever the corpus yielded first -- i.e. to `readdirSync` order. **Reversing the corpus array changed the printed cousin for 35 lessons**, which is precisely the churn the fixed language order existed to prevent, and the module's own comment claimed it could not happen. Fixed with a **total** order of (chapter, sequence, id); reversal now changes **0** lessons. (2) A missing `sequence` sorted as *latest*, so an unsequenced chapter-1 lesson lost to a sequenced chapter-33 one -- the inverse of the rule. Chapter-first ordering fixes it. (3) **Five of nine tests passed vacuously**, proved by stubbing the function to return `[]` and watching them stay green; and the *join really is etymological* assertion was **tautological**, since `cousin.root` is assigned from the querying lesson's own roots and could never fail. Both fixed, and the suite is now verified by stub: **0 of 10 survive**. **A latent issue worth noting rather than acting on here:** `loader.ts` iterates `readdirSync` unsorted, so other consumers may carry the same filesystem-order dependence. Sorting it would churn unrelated pins and belongs in its own change. **Auditing the 25 strict pairs by hand then found the blocker, and it is one level deeper than the `concept_tag` correction.** Most pairs are exactly right -- *estar/stare*, *hablar/falar*, *trabajar/travailler/trabalhar*, *escribir/scrivere*, *leer/leggere*, *ayudar/aiutare*, *pensar/pensare*. Three were not, and all three fail the same way: **`roots:` records every etymon a lesson DISCUSSES, not the etymon of its headword**, while a cousin panel needs the latter. `IT-C20-incontrare` declares `[contra-latin, cognoscere-latin]` because a second meeting-verb hides in its past tense -- correct for the lesson, wrong as a claim that *incontrare* descends from *cognoscere*, which paired it with Spanish *conocer*. `ES-C01-bien` declares `[bene, bonus]`, so it matched Italian *buongiorno* through `bonus`. And *tener* paired with *ottenere*, a real descendant of *tenēre* but a compound, where the direct cousin is *tenere*. **The single-token filter made one of these worse rather than better:** `IT-C01-buono` (headword *buono / buon*) is the right cousin for *bien* and the filter excluded it for having two tokens, leaving the compound behind. **A `roots:`-length-of-one heuristic** (single root, single token) gives **21 Spanish pairs, 20 of them clean** -- it removes both bad pairs and keeps *tener/ottenere* -- but it is a **proxy for the missing signal, not a fix.** **So the rendering is blocked on a schema question, which is a design decision rather than a cleanup:** does a lesson need to distinguish *the etymon of its headword* from *the etymons it discusses* -- a `headword_root:` field, or a convention that the first entry in `roots:` is the headword's? Until that is decided, generating panels would publish the same class of false etymology the `concept_tag` correction avoided, just at a lower rate. The join module is landed and correct; what it needs is a corpus that can answer the question it asks. **And a bigger question this raises:** a 129-chapter adjective famine is unlikely to be the only gap of its kind, and the corpus should be swept for other grammar taught without the words it needs, rather than found one arc at a time. **That sweep was attempted, and it failed in an instructive way -- do not repeat it as designed.** The approach was a probe list: name a grammar feature, hand-curate the words it operates on, and compare the chapter each first appears. It produced four candidate gaps and **two of the four were false**, both because the probe list encoded my expectations rather than the corpus. (a) *Comparatives taught at ch76 with `más` never taught* -- **wrong**: ch76 is `ES-C44-sintesis-mas-de-uno`, *talking about more than one*, i.e. **plurals**; the regex matched *more than* inside *more than one*, and the book does not teach comparatives at all. (b) *No noun until chapter 69* -- **wrong**: chapter 1 teaches *día* and chapter 2 *la noche*. The probe list held *libro*, *casa*, *gato* and simply missed the nouns this book actually opens with. The narrow true statement is that the first **household-object** nouns arrive at 69-71 (*la casa*, *el libro*, *la comida*), which is worth a look but is not a famine. **The lesson: a hand-curated probe list measures the distance between the corpus and the auditor's expectations, not a real gap, and it reports that distance with total confidence.** A detector built on it would file false findings at scale, which is worse than finding gaps one arc at a time. **What a real detector needs is a signal the corpus does not currently carry:** a word-class tag on lexical lessons (noun / verb / adjective / adverb / function word). `concept_tag` cannot stand in for it -- Spanish has **120 tag families and most have exactly one member**, so it is an identifier, not a classification. The honest options are to add `word_class:` to the schema and backfill it, or to keep finding these by hand and accept the rate. **Verified and still standing:** the two gaps already fixed (professions, adjectives) were both confirmed against the corpus before being acted on, and both were real. |
| HL-C89 | Complete (#10511) — ramp shipped, lint deliberately minimal | Build the metalanguage ramp and the dummy-friendly prose lint (HL10 §7.4–7.5). | `core/metalanguage.json` declares **54 terms**, each with the thing the learner must already be able to DO first and the `plainAlternative` a lesson must use until the term is earned. First measurement: **2,289 technical uses across 1,161 lessons** — `verb` 795, `noun` 398, `regular` 109, `tense` 109 — with nothing introducing any of them. Two numbers reported so `word` (1,555 lessons) cannot bury `dative` (53). The prose lint was **measured and found near-empty** (535 naive → 23 narrowed, most still innocent) and is not built; the corpus's prose is already kind. Burn-down of the 2,289 is HL-C93. |
| HL-C91 | Complete (#PENDING) | Add the irregular and stem-changing overlays to the Spanish cell inventory. | HL10 §5.1 sizes the Spanish verb system at ~630 cells; HL-C82 shipped the **231 regular** ones. The remaining ~400 are overlays — the four stem-change patterns (e→ie, o→ue, e→i, u→ue), the `-go` club, the strong preterites (*fui*, *hice*, *dije*, *tuve*, *estuve*, *pude*, *puse*, *supe*, *quise*, *vine*, *traje*), the three imperfect irregulars, and the irregular participles. Each attaches to the regular cell it deviates from, so the DAG gains depth rather than breadth, and each needs a frequency-ordered position. **HL-C90 is deliberately skipped here**: it is claimed by the in-flight HL-C80 branch (control-character neutralization), and reusing it would recreate the duplicate-id tangle HL-I06 exists to prevent. |
| HL-C92 | Queued — **Spanish portion closed as already-satisfied**; French/German still blocked | Split the 18 full paradigm grids, and the 52 partial ones behind them. | Each presents N grammar cells where `maxNewGrammarCellsPerLesson` allows one, so each becomes N lessons plus a recap table that is now legitimately a recap. **Re-measured 2026-08-11: `ES-C17-practice` is a false target and must not be split.** It is the terminal checkpoint of a chapter whose seven teaching lessons introduce one row apiece — `hablaré`, then `comeré`, then `viviré`, then the conditional row by row — and the checkpoint itself introduces **zero** atoms. Its table is precisely the recap HL10 §5.3 permits: every cell in it was individually taught first. The same holds for `ES-C18-practice` across the subjunctive chapter. HL-C84's flag is shape-based by design and its own note says most of the 470 multi-row tables are fine; these two are among them. A sweep of Spanish found **23 tables with ≥7 rows and no genuine info dump among them** — the rest are lists (months, numbers, days of the week), not paradigms. **Spanish needs no work here.** The real targets are `FR-C05-parler` and `GE-C05-wohnen`, and both remain blocked on extending `grammar-cells.json` past Spanish, or the split has no cell ids to name. |
| HL-C93 | Queued — found by HL-C89 | Wire `introduces_metalanguage` onto the lessons that first earn each of the 54 terms. | 2,289 technical uses across 1,161 lessons currently precede any introduction. Most are discharged by declaring the term on the lesson that first teaches the thing — `verb` on the lesson after *soy*/*estoy*/one present form, `mood` at subjunctive block D — rather than by rewriting prose. Where a term genuinely arrives too early, the fix is the recorded `plainAlternative`, not a gloss in place. |
| HL-C98 | Complete (#PENDING) | Give the first paradigm one grammar cell per chapter, and add the book's first review and synthesis chapters. | The book's first paradigm was one lesson — `hablo, hablas, habla` — presenting a three-row table on first exposure plus pro-drop, i.e. three cells where `maxNewGrammarCellsPerLesson` allows one. It is now five chapters: *hablo* · pro-drop → *hablas* → *habla* → **review** (the table, finally earned) → **synthesis** (the same conversation held warmly and respectfully, where only one letter changes). First lessons in the corpus to declare `teaches_cells:`, moving cell coverage **0 → 3 of 231**. Spanish 50 → 54 chapters; old 16–50 shifted to 20–54; lesson ids unchanged. Forward references **424 → 423**. |
| HL-C99 | Partly complete — 6 of 12 chapters split; **no 4-verb chapter remains** | One verb per chapter (HL10 §5.7), and an etymology for every verb. | **12 of 19** Spanish verb-teaching chapters introduce more than one verb; chapters **47, 48 and 52** introduce **four** apiece (numbering after HL-C98). Each verb owes the reader an origin, so a four-verb chapter either dumps four etymologies or gives three of them a sentence each. Split largest-first. **Corrected 2026-08-11:** this row originally also claimed `beber`, `preguntar` and `tomar` had no etymology. They do — all **42** verbs do. Those three carry it via `roots:`/`etymology_hook` rather than an `ES-ETYMON-*` atom, and the census had counted only the atom namespace, which contradicts the Root Ledger's own two-namespace definition. There is no etymology gap; the defect is only the crowding. **First slice landed:** chapter 47 (`pensar`, `entender`, `leer`, `escribir`) becomes six chapters — one verb each, then a review chapter and a synthesis chapter. Neither the old chapter nor its neighbour had *any* practice or payoff lesson, so four verbs arrived with no consolidation at all. The split also closes a duplicate: `entender` re-taught `no entiendo` 34 chapters after the repair kit introduced it, without linking to or practising `ES-LEX-NO-ENTIENDO-01`; it now does both, and the synthesis chapter turns the frozen formula into a sentence with moving parts. **Second slice landed:** chapter 53 (`tomar`, `preguntar`, `ayudar`, `gustar`) becomes six. `gustar` is deliberately placed **after** the review chapter rather than beside the other three, because it is not a fourth verb — it is the reverse-subject system, and the contrast only lands once the ordinary pattern has been consolidated. The review chapter states the shared shape out loud ("the sentence is about the one doing it") precisely so the next chapter can break it. The synthesis chapter closes a joke 53 chapters in the making: *mucho gusto*, taught in chapter 5, is `gustar`'s own noun. **Third slice landed:** chapter 62 (`traer`, `conseguir`, `jugar`, `conocer`) becomes six. The review chapter had an unusually strong thesis available: this chapter **completes the stem-change inventory** — e→ie and o→ue were already held, `conseguir` adds e→i and `jugar` adds u→ue, and u→ue has **exactly one member in the language**. Telling the learner a pattern is a single fact is easier than letting them brace for more. The synthesis chapter collects the three verb pairs where English offers one word and Spanish forces a choice — *preguntar*/*pedir*, *traer*/*llevar*, *conocer*/*saber* — taught chapters apart and never before placed side by side; the decision, not the word, is the work. **Fourth slice landed:** chapter 21 — the book's SECOND and THIRD paradigms, which were still bundled three-cells-to-a-lesson while `-ar` had just been given five chapters. `comer` now owns a chapter with one `-er` cell per lesson (`como` · `comes` · `come`) plus a review that earns the set. **`-ir` deliberately did NOT get the same treatment**: in the singular its endings *are* the `-er` endings, so three per-cell lessons would have been padding. That exposes a real gap between the letter and the purpose of `maxNewGrammarCellsPerLesson` — the rule should count **new forms, not new slots** — and `vivir` declares all three CONJ3 cells in one lesson on that basis. Worth settling in HL10 §5.2 properly. **Fifth slice landed:** chapter 30 (`poner`, `salir`, `venir`) becomes five, closing the `-go` club. The review chapter's thesis is historical rather than formal: the six `-go` verbs **are not a family**. They came from different Latin verbs by different routes and converged on the same ending by accident. That matters practically — a pattern with a cause keeps recruiting, an accident stops, so the learner can be told the list is *closed*. The synthesis chapter observes that the oddity lives only in the `yo` slot, i.e. in the form a beginner uses most, and pairs each `-go` form with its perfectly regular `tú`/`él` counterpart. **Sixth slice landed:** chapter 20 (`trabajar`, `estudiar`). Mild crowding — both verbs reuse the `-ar` pattern and introduce no grammar — but each owes the reader an origin, and these two are worth the room: `trabajar` ← *tripaliāre*, from *tripalium*, a three-stake torture device, which is why English **travail** is the same word; `estudiar` ← *studēre*, "to be eager". No new lessons were needed, only chapter boundaries. **6 chapters remain**: ch68, ch77, ch78 (3 verbs each), then ch34, ch35, ch69 (2 each). Spanish 54 → 59 → 64 → 69 → 72 → 76 → **78 chapters**. |
| HL-C100 | Substantially complete — cliff gone, vocabulary run consolidated | Give every stage its review and synthesis chapters (HL10 §5.8). | Before HL-C98, **zero of Spanish's 50 chapters introduced zero atoms** — the course never stopped to consolidate, and never once asked the learner to communicate an idea rather than acquire a word. HL-C98 adds the first pair, at chapters 18–19. The remaining stages need the same: a review chapter wherever a paradigm table wants to appear, and a synthesis chapter wherever a communicative choice becomes possible. Synthesis chapters must stay voice-drivable; review chapters carrying a table will honestly be `sight`. **First late-book slice landed.** Re-measuring grammar load found the ramp's remaining spike is not in the opening at all: chapters 39–42 carried **7, 8, 10 and 9** grammar atoms against a book mean of **1.4** — four consecutive chapters, each a whole tense or mood system, with nothing between them. The worst, chapter 41, held **two** systems: the future *and* the conditional, plus the irregular stems they share. Split into four — the future, the conditional, the shared stems, and a synthesis. The synthesis is the payoff for an etymology the course already taught: both endings are *haber* glued onto the infinitive, once in the present and once in the past, so future-vs-conditional is **one system seen from two moments**, and English's *will*/*would* is the same present/past pair of one old verb. **Second late-book slice landed:** the subjunctive, which was the worst remaining chapter at 9 grammar atoms. Split by *idea* rather than by count: the non-assertion concept alone (the part learners actually struggle with), then the regular forms, then the yo-stem irregulars, then `ojalá`, then a synthesis. The synthesis makes the mood a **stance** rather than a tense: *hablas español* puts a fact on the table, *quiero que hables español* claims nothing at all — and every later use of the mood (doubt, denial, hope) is the same move. `ojalá` closes it, because it explains itself: Arabic *wa-šā' allāh*, "and may God will it", kept through eight centuries of al-Andalus and long after the religion behind it, and a thing God has yet to will is by definition not a fact you can assert. **Third late-book slice:** the imperfect. Split into the regular forms, `ver` (a verb that had been inserted mid-chapter because `veía` needed it), the three irregulars, and a synthesis. The synthesis carries the fact no single lesson could: **the imperfect has exactly three irregular verbs in the whole language** — *ser*, *ir*, *ver* — and they are irregular *because* they are the most-used verbs, worn straight through from Latin *erat* and *ibat* without ever being tidied. Rare verbs get regularised; heavy use preserves an odd shape. **The spike is flattened.** **Fourth slice: the preterite, the last outlier.** Split into the regular forms, the strong preterites, and a synthesis. The synthesis is a recognition rule rather than a paradigm: the two kinds differ by **where the stress falls**, and you can hear it — *com**í*** stresses the ending and therefore carries a written accent, *tu**ve*** stresses the stem and therefore carries none. The accent is not extra spelling; it is the stress, written down. A preterite with no accent is a strong one. **The cliff is gone.** Four consecutive chapters once carried 7, 8, 10 and 9 grammar atoms against a book mean of 1.4. The worst chapter in the entire book is now **4**, and nothing exceeds it. Spanish **91 chapters**, 223 lessons. **The vocabulary run now has a synthesis.** Chapters 56–69 are fourteen consecutive lexical chapters — colours, family, body, food, seasons, months, time, weather, numbers, animals — and nothing between them ever asked the reader to combine what they held. A learner arriving at the end had **27 concrete nouns** and had never been asked to say one thing about their own life. Chapter 70 is that chapter, and it is the one that surfaced HL-C104: writing it was impossible until the indefinite article existed, because describing your own life means introducing things your listener does not know about yet, which is precisely the job `un` does. The lesson makes that the grammar point — *el perro* is the dog we were discussing, *un perro* is one you are mentioning for the first time. Remaining under HL-C100: earlier stages that still have no review or synthesis chapter, which is now a coverage question rather than a ramp one. |
| HL-C101 | Complete (#PENDING) | Move `español` earlier, or move `hablar` later. | `hablar` is taught at chapter 15 but `español` — the first thing the learner can actually say they speak — is at sequence 550, five chapters later. The synthesis chapter at 19 therefore cannot use *hablo español*, the single most useful sentence the verb affords, without a forward reference. **Fixed by moving the noun up.** `español` and the first built sentence now sit at chapters 21 and 22, ahead of the `-ar` synthesis chapter rather than three chapters behind it, and the synthesis says *hablo español* instead of a bare `¿Hablas?` — which was a workaround for the missing noun, not a sentence anyone says. The first attempt moved `español` alone and broke validation: `ES-C06-hablo-espanol` uses `trabajar` and `estudiar` as its worked examples, so the whole run had to move together. Final order: `-ar` review → `trabajar` → `estudiar` → `español` → *hablo español* → **synthesis**, which is also a better ramp than the original, since the synthesis now exercises everything before it rather than only the three cells. Spanish 78 → **79 chapters**. |
| HL-C102 | Complete (#PENDING) — Spanish at zero, rest pinned | Gate prose cross-references to chapter numbers, or forbid them. | 31 lesson-body references name a chapter number ("you learned this in chapter 14"). Nothing checks them, and every split shifts the numbering — `ES-C19-no` carried a stale "chapter 14" pointer through **three** renumbers after the lesson it named had moved to chapter 20. It was an author comment, so no reader saw it, but the same rot in learner-facing prose is invisible until someone reads that page. **Done, and the rot was worse than filed.** Three Spanish references were already wrong: `ES-C09-esta-en` sent the reader to "Chapter 7" for a question taught in 24; `ES-C41-explicar` placed `contar` in "chapter 38" when it had reached 71; and the `ES-C19-no` comment I *fixed* two PRs earlier had gone stale again, because HL-C101 moved the lesson it named. All 32 Spanish references are rewritten to name the thing rather than a number — "since the repair kit", "when you first met them", "the next chapter" — and Spanish is now at **zero**. `tests/chapter-references.test.ts` counts **cross-chapter** references only (a lesson naming its own chapter, as in an `# Chapter 2` heading, points nowhere else and cannot rot), holds Spanish at zero, and pins the other 19 tracks at their current 710 so the debt cannot grow while they are stable. Clear a track to zero before it starts splitting chapters. |
| HL-C103 | Complete (#PENDING) — census done, list grew by three | Add the English/Spanish homographs to `continuity.ts`'s `ENGLISH_COLLISIONS`. | The forward-reference detector already guards against ordinary English words colliding with Spanish headwords (`no` was reporting from seven lessons). **`comes` is not in the set**, so the sentence "*comer* **comes** from Latin *comedere*" reported the Spanish `tú` form as a forward reference from its own verb's lesson. **Census done.** Of 423 forward references, **368 matched via emphasis and 55 in plain prose only**; of those 55, 18 were pure-ASCII candidates and exactly **three** were actually English: `comes` ("*comer* comes from Latin"), `hand` ("the old German Fraktur hand") and `regular` ("Regular stress: TAR-de"). The other 15 — `luego`, `lait`, `branco`, `emere`, `capere`, `katze`, `nacht`, `morgen`, `hasta`, `tres`, `nuit`, `essere` — are genuine forward references and must keep reporting, so a list built from a plausible wordlist (my original guess included `pan`, `son`, `van`, `dice`, `mar`, `ten`) would have **suppressed 15 real defects**. The tempting structural fix — guard only the plain path and trust emphasis to mean target language — was already tried and documented as wrong: authors emphasise English for stress ("**no** glide, no drift"), which reported `no` from seven lessons. Both paths keep the guard. Forward references **423 → 418**. The census method is now recorded in `continuity.ts` so the next extension is grounded the same way. |
| HL-C104 | Complete (#PENDING) — found while looking for something else | Teach the indefinite article. It was **never taught anywhere in the corpus**. | Looking for a place to add a synthesis chapter after the 14-chapter vocabulary run, I could not write "I have a dog" — because `un`/`una` is not in the book. HL10 §5.4 rung 3 says "definite article, **then indefinite**" and §12.2 block 11 says "four lessons, **then indefinite**"; the definite articles shipped and the indefinite ones never followed. A learner reaching chapter 68 held 27 concrete nouns — colours, family, body parts, food, animals — and could say *the* dog but not *a* dog. Now chapter 3, immediately after the definite articles and the agreement payoff, where the learner already has gender and three nouns: `un`, `una`, and a review. The etymology is a gift — `un` **is** the number one (*ūnus*), and English *a*/*an* is *one* worn down, which is why *an hour* and *one hour* begin alike. Spanish left the seam visible; English hid it. |
| HL-C105 | Complete — 11/11 rungs closed | Close the 11 HL10 §5.4 rungs that are absent from the Spanish corpus. | HL-C104 (`un`/`una` missing across 91 chapters) suggested the ladder should be audited rather than assumed. It should have been: **11 of HL10's 28 grammar rungs are entirely absent.** The largest is rung 10. **The course is singular-only.** 44 verb-paradigm atoms are marked SINGULAR; exactly 2 are marked PLURAL, and both of those are *adjective* agreement (`buenos`/`buenas`), not verbs. No plural verb form — `hablamos`, `hablan`, `somos`, `son`, `tenemos`, `van` — appears as a headword anywhere. After 93 chapters and 227 lessons the reader holds five tenses and a mood, all singular, and cannot say *we speak*. Every lesson has been honest about this (each gloss says "singular"), which is exactly why it stayed invisible. **Absent:** 1 `hay`; **10 present plural**; 13 direct object pronouns; 14 indirect object pronouns; 15 double object/*se lo*; 20 preterite/imperfect contrast (HL10 calls it "the wall"); 23 perfect tenses; 24 commands; 26 *por*/*para*; 27 *se* passive/impersonal; 28 relative clauses. **Present and healthy:** 2–9, 11, 12, 16–19, 21, 22, 25. **Priority is rung 10 before everything else.** Rungs 13–15, 20, 23, 24 and 27 all assume plural forms exist, so they cannot be authored around it; and the ramp work of HL-C94..C104 makes the shape obvious — one cell per lesson, `-er`/`-ir` sharing where they genuinely share, a review chapter when the paradigm closes, a synthesis where the choice becomes communicative. **Method note:** this audit was two probes, and they disagreed on rungs 21 and 26 because the first regex was loose and the second too narrow. Rung 21 is present (`ES-GRAMMAR-NEAR-FUTURE-IR-A-INFINITIVE`); rung 26's only match was `ES-GRAMMAR-PORQUE-THREE-05`, which is *because*, not *por*/*para*. Resolve disagreements by reading the matched atom, never by trusting a count. **First slice landed:** the `-ar` present plural, in the shape HL-C94..C104 established — `hablamos`, then `hablan`, then `habláis`, one cell per lesson, then a review and a synthesis. Order is deliberate: the universally useful forms first, Spain-only last. The review chapter carries **the first complete paradigm in the book** — six forms, withheld for thirty-two chapters until every box in it was earned, which is HL10 §5.3 paying off exactly as designed. The synthesis makes `vosotros`/`ustedes` a genuine split rather than a footnote (rung 10's stated requirement): it is the first choice in the book decided by **geography rather than meaning**, and neither form corrects the other — ~40 million speakers use `vosotros` daily and several hundred million never will. **Rung 10's present tense is complete.** The `-er`/`-ir` plurals landed in four more chapters, and they carry a fact §5.2a exists for: the two families are **identical in four of the six slots and differ in exactly two** — *nosotros* and *vosotros*. So `comen`/`viven` share a lesson (same form, one fact) while `comemos`/`vivimos` do not (different forms, two facts). The `vivimos` lesson explains *why* the singular hid the difference: there the ending is unstressed and its vowel wears toward the other family's, while in the plural the stress lands on the ending and each family's own vowel survives. The families never diverged — the singular simply muffled them. The closing review prints **eighteen forms**: the present tense of every regular Spanish verb, each one built separately before it appeared in the grid. **`ser` now has its plural** — `somos`, `sois`, `son` plus a review, in the one-cell-per-lesson shape. It was the highest-value irregular to take first: the commonest verb in the language, and the learner held only its singular. The review earns a fact memorisation never gives: **five of `ser`'s six forms begin with `s-`, and `eres` does not, because it comes from somewhere else.** `ser` is two Latin verbs fused — *esse* (giving the `s-` forms) and *sedēre* (giving the infinitive, future and conditional) — so the irregularity is not chaos but a visible seam, and English builds *am/is/are/was/be* out of three Old English verbs for the same reason. **`estar` now has its plural too**, placed beside `ser`'s so the contrast holds in both numbers rather than only the singular. Its review sets the two paradigms side by side and makes the point that they are hard for **opposite** reasons: `ser` is two Latin verbs fused (*esse* + *sedēre*), so its forms share no shape; `estar` is one verb (*stāre*), so its plural is a plain `-ar` paradigm and the only oddity is where the stress falls — which the written accent records honestly. That generalises: where a Spanish verb looks chaotic, the usual explanation is not strange decay but **more than one word wearing a single name**. **`tener` and `ir` complete the four commonest verbs.** `tener` states the stem-change rule outright — **the stem breaks exactly where it is stressed**, which is why *nosotros* and *vosotros* never break in any stem-changing verb, and why the "boot" mnemonic is unnecessary. `ir` is a third suppletive: its whole present descends from *vādere*, not *īre*, so the present is not irregular at all — it is **regular for a verb whose infinitive Spanish stopped using**, exactly like English *go*/*went* (where *went* belongs to *wend*). Three of the four now teach the same underlying tool: **a verb that looks chaotic is usually more than one word wearing a single name**, and the regular-looking parts are where one word survived intact. **The remaining irregular plurals are now derived rather than taught.** A consolidation chapter closes rung 10 by introducing almost nothing: the stem-change rule (the stem breaks exactly where it is stressed) already predicts *queremos*/*quieren* and *podemos*/*pueden*, and the `-go` club has **no plural forms of its own at all**, because the `-go` was only ever in *yo* — so *decimos*/*dicen* and *venimos*/*vienen* break only as far as the stress rule says. Each lesson asks the reader to produce the form **before** reading on; getting it right is the evidence that the rule was learned rather than the forms memorised. The synthesis shows that three rules — the family endings, the stress rule, and *yo* being where irregularity hides — generate the entire present tense, and has the reader conjugate two verbs they were never taught. **Rung 10 is closed.** **Rung 13 (direct object pronouns) is now closed too**, in nine chapters at 61-70, and it needed two prerequisites that did not exist: ordinary nouns (HL-C106) and the plural articles (HL-C108). The singular half teaches `lo`, `la`, `me` in its plain non-reflexive job, and `te`; the plural half `nos`, `los`/`las` and `os`. Two of the eight cells were **never taught** -- the reader derives `los` and `las` in a warm-up before being given them -- and the etymology pays off a puzzle set two arcs earlier: the article `el` had to become `los` because it was a worn-down *elo*, while the pronoun `lo` kept its vowel and so needed no repair at all. `nos` also closes a quiet gap: the `-mos` ending has been in the reader's mouth for thirty chapters and **`nosotros` had never been taught as a word**. The closing synthesis states the cost as well as the benefit -- a pronoun buys speed and is paid for in attention, because it promises the listener that the thing it points at is still in the room. **Rung 14 (indirect object) is closed too**, in five chapters at 71-75. The headline is arithmetic: **a second complete pronoun table costs exactly two new words.** `me`, `te`, `nos` and `os` are letter-for-letter identical in both systems, so only `le` and `les` are new -- and `les` the reader derives with the same `-s`. Twelve cells are carried by nine shapes. `le` also brings relief rather than difficulty: **it does not mark gender**, so after four chapters of choosing `lo` against `la` by the gender of a noun, one form covers him, her and *usted*. The etymology is the deepest in the arc: `lo` is Latin *illum* and `le` is Latin *illī* -- the accusative and the dative, two **cases** of one word. Nouns lost their case endings entirely on the way to Spanish, but in these small constantly-used pronouns two cases survived intact, and English lost the same distinction more completely (it survives only as word order: *I told **him** the story*). A dedicated chapter replaces the verb-by-verb list with a **test** -- does the verb act *on* it, or aim *at* it? -- and names **leísmo** honestly in the same breath, so the reader meets *le veo* in Madrid as a known variety rather than a contradiction. The synthesis carries something genuinely new: this is the **first choice in the book that nothing on the page can settle.** Same slot, two letters each, one Latin ancestor; only meaning separates them. **Rung 15 (double object) is closed**, in five chapters at 76-80, and it introduced **no new pronoun at all** -- one order and one substitution. The order is fixed (person, then thing) and the lesson points out that a fixed order is *smaller* to learn than English's free one, where *give it to me* and *give me it* differ by a shade of emphasis you have to feel your way into. The substitution is `le`/`les` becoming `se` before `lo`/`la`/`los`/`las`; the reader is asked to say **le lo digo** aloud first, so the mouth objects before the grammar does. **The etymology chapter removes a confusion most learners carry for years.** The `se` of *se lo digo* is **not** the reflexive `se` of *se llama*, which the reader has held since chapter three. Old Spanish said **gelo**, from *illī* + *illum* squeezed together -- perfectly regular, no rule to learn. That re-split into *ge* + *lo*, and *ge* then drifted until it sounded exactly like *se*. Two unrelated words, Latin *sē* ("himself") and *illī* ("to him"), collided by sound in the sixteenth century. Learners who assume they are one word spend a long time trying to make "he tells it to himself" fit sentences where it plainly does not. The synthesis shows the payoff: **se la hago** is three words carrying a person and a thing, neither of them named anywhere in the sentence -- and states the doubled cost, that two pronouns are two promises both things are still in the room. **Rung 20 -- the wall -- is closed**, in five chapters at 89-93. Neither tense was new; all three teaching chapters are about the **choice**, which is the whole difficulty. The first states it plainly: the two pasts do not differ in **when**, they differ in where you are standing. *Ayer hablé con Ana* and *ayer hablaba con Ana* are the same afternoon. English asks the same question and lets you dodge it with extra words (*I spoke / was speaking / used to speak*); Spanish put the choice inside the verb, so every past sentence commits to a viewpoint before it commits to anything else. **That is why this stays hard longest -- not complexity, but a question your first language let you leave unanswered.** **`cuando` was never taught** and the contrast cannot be shown in one sentence without it, so it gets its own chapter: *estudiaba cuando habló Ana*, the imperfect sets the room and the preterite is what walks into it -- and reversing them changes the story rather than the style. The third chapter is the payoff: **`tenía` is *had* and `tuve` is *got*.** Not a lexical exception to memorise -- it follows from the aspect rule. *Tener* names a **state**, states have no edges, so forcing one into the preterite gives you the only edge it has: **the moment it began.** The reader can then predict every state verb's preterite without a list. The review deliberately hands over **a question, not a table** (*am I saying what happened, or what things were like?*), because the trigger-word lists most courses give fail exactly where the choice is interesting. The synthesis is **the first narration in the book**: four sentences whose imperfects are the background and whose preterites are the foreground, with *tener* appearing in both layers doing different jobs. **Rung 23 (perfect tenses) is closed**, in six chapters at 94-99. The arc's shape is different from every tense before it: this one is **built, not conjugated.** Six little words learned once, plus one participle per verb, instead of a new ending set for every family -- a fixed cost rather than a per-verb one, and the review names that shape because Spanish reuses the same two pieces for more tenses later. The participle chapter gets the `-er`/`-ir` merger for free: the reader has already seen those two families collapse in the preterite, so `-ido` is one new ending and one predicted. **`haber` gets its own chapter for the question learners actually have: it means nothing.** Latin *habēre* was an ordinary 'hold'; Spanish handed the meaning to `tener` and kept the machinery -- and **English did the same to *have***, which is why *I have eaten* holds nothing. Two languages wore their word for *hold* down into a tense marker independently. So when `haber` feels meaningless, that is an accurate perception, not a gap. The irregular participles are framed as **older than the rule** rather than broken by it: `hecho` ← *factum* (English **fact** -- a thing that has been done), `dicho` ← *dictum*, `visto` ← *vīsum*, `puesto` ← *positum*, inherited whole from Latin while the `-ado`/`-ido` machinery replaced them everywhere else. And they are the four commonest verbs, which is the point: **only frequent words are used enough to survive being irregular.** The synthesis carries the **third geography split** after *vosotros* and *os* -- for something that happened today, Madrid says *he hablado* and Mexico City says *hablé*, both correct -- and gives a default so the reader is not left choosing in the dark. **Rung 24 (commands) is closed**, in six chapters at 109-114, placed **after** the subjunctive on purpose: three of the four command boxes *are* the subjunctive, so putting commands first would have meant teaching a form twice. The arithmetic is the striking part -- **a whole grammatical mood whose only genuinely new material is eight one-syllable words.** The affirmative `tú` command IS the he/she present the reader has held since chapter 7 (*habla* said about somebody is a statement; said to somebody it is an order, and nobody has ever confused them). The negative is the subjunctive, and **not as a borrowing**: *no hables* asserts no speaking at all, so the non-asserting form is doing precisely its usual job. The `usted` command is the subjunctive too, and the chapter explains **why that is the polite one** rather than asserting it: `usted` is a worn-down *vuestra merced*, a **title**, and titles are third person -- so the order is aimed past the listener rather than at them, and that indirectness *is* the politeness. The eight irregulars (`di, haz, ve, pon, ten, sal, sé, ven`) are framed by a fact the reader can check by eye: **every one is a single syllable**, because a command is the sentence you shout, hurry and interrupt with, and these are the commonest verbs in the language. The review makes the same point structurally -- **the messiest box is the most-used box**, which has been true of every irregularity in this book, and there are no irregular *negative* commands because negatives are said less and had time to be regularised. The synthesis is a culture chapter: an English speaker's mistake with Spanish commands is **not tone but person**. *Come* to a friend is not blunt, it is close; `usted` with an old friend is not extra-polite, it opens a distance they will hear. **The politeness lives in the pronoun, chosen before the command left your mouth.** **Rung 26 (*por*/*para*) is closed**, in five chapters at 153-157, and it replaces the usual list of twelve rules with **one arrow**. *Para* points forward -- at a destination, a deadline, a recipient, a purpose. *Por* points back at a cause or through a middle. **The directions are inside the words**, which is why the test is not a mnemonic: *para* is Latin *per* + **ad**, and *ad* is the *ad* of *advance*, *admit*, *adhere*; *por* is *pro* + **per**, and *per* is still alive unchanged in English *per hour*, *per cent* (literally *through each hundred*), *percolate*, *perforate*. So the reader is reading a direction built into each word before Spanish existed, and it keeps working on uses the book never showed. **The `por` chapter explains three phrases the reader has owned since the opening chapters and was never told the meaning of:** `por favor` (literally *by way of a favour* -- asked **through** one, not aimed at one), `¿por qué?` (*through what?* = caused by what) and `porque`. The review says so plainly: much of what looks like late grammar is **a name for something already done blindly**. The synthesis is the first place in the book where **the smallest word carries the sentence's emotional content** -- *hago la comida para ti* is somebody cooking you dinner, *por ti* is somebody telling you they cooked **because of** you, which is love or complaint depending on the day. It also removes the anxiety honestly: a wrong choice is **not bad grammar**, it is a true sentence about a different situation, and somebody will look puzzled -- which is how it stops being difficult. **Rung 27 (*se* passive/impersonal) is closed**, in five chapters at 158-162, and it pays off rung 15 rather than repeating it. The impersonal `se` is **the same word as the reflexive**, grown from it -- and the proof is the agreement: *se compran libros* has a plural verb because, underneath, **books buy themselves**. Read that way, the strangeness disappears: *libros* is the subject of *se compran* exactly as `él` is the subject of *se llama*. So the review can finally say the thing that unifies material learned a hundred chapters apart -- **Spanish has two `se`s and a coincidence**, not three. The reflexive and the impersonal are one word stretched; the `se` of *se lo digo* is Latin *illī* wearing the same two letters by accident, which the reader met as *gelo* in rung 15. A one-line test comes with it: **a `lo`/`la`/`los`/`las` immediately after means the impostor**, and it is sitting right there in the open. **`¿cómo se dice?` gets its own chapter** and is called what it is -- the most valuable sentence in the book, because it is the only one that lets a conversation **repair itself**: it fetches a word the reader does not have, from the person in front of them, mid-sentence. It is also this chapter's grammar, asked out loud. The synthesis is a reading chapter rather than a producing one, because this construction is **written at you** rather than spoken to you -- four shop windows, three fully readable and one deliberately containing unknown words so the reader practises reading the *shape* and then asking. It closes on what the missing person buys a sign: **nobody is claiming anything, so there is nobody to argue with.** **Rung 28 (relative clauses) is closed**, in five chapters at 163-167, and it is the only rung in the whole campaign that introduced **no new word at all**. `que` has been held since *quiero que hables*; `lo` since the object pronouns; `lo que` is the two of them side by side. The first chapter names what actually changes: this is the point where **a sentence may contain another sentence**, which is what allows description, explanation and storytelling of any length. The second chapter is the one that earns its place -- **English deletes its joint and Spanish never does.** *The book I bought* has no *that* in it, so the learner thinks the English, translates what they thought, and produces *el libro compré*, which feels complete because their English was complete. The gap is invisible **because their own language put it there**. The habit given is mechanical: a noun, then a new subject and verb, with nothing between -- that needs a `que`. And the reason is not arbitrary strictness: English can delete the joint because its word order is rigid, while Spanish moves subjects and objects around, so a deletable joint would be genuinely ambiguous. **It is the price of the freedom Spanish has been giving the reader for 160 chapters.** `lo que` is framed as a **noun-shaped hole with a sentence attached** -- which is exactly what English *what* is, *the thing that* with *thing* removed. The synthesis makes the closing argument of the beginner's course: **length is not difficulty.** A relative clause adds a *slot*, not grammar; from here the growth is more of what the reader already has, arranged more deeply, and what remains genuinely hard is vocabulary and hearing speed -- both of which are time. **Rung 1 (`hay`) is closed, and with it the whole audit: all eleven absent rungs are done.** Five chapters at 168-172. `hay` is placed *late* rather than at its A1 difficulty, and deliberately: only after `haber` (rung 23) can it be **explained** instead of listed. It is **the one common Spanish verb form that agrees with nothing** -- one book or a hundred, `hay` does not move -- and that is only visible as a gift after 167 chapters of agreement, including the impersonal `se`, which *does* still agree with the thing. **The etymology chapter is the best in the arc.** `hay` is two words fused: `ha` (from `haber`, which the reader now holds) plus **`y`, from Latin *ibi*, 'there' -- a word Spanish otherwise lost completely.** It survives in exactly one place, welded to a verb, because glued words stop being available to be replaced. So the literal reading is *it has there*, and the neighbours confirm it: **French *il y a* is the identical three pieces kept separate**, and English *there is* also opens with a place-word doing no place-work. Three languages, three verbs, one instinct arriving three times. `hay que` then pairs with the impersonal `se` as **Spanish's second way of leaving a person out** -- one says something happens without a doer, the other says something must happen without one. The review names a pattern worth carrying: **the smallest words need the most explaining**, because constant use wears them into fossils that no longer explain themselves -- three letters took four chapters where *comida* took one. The final synthesis stacks four chapters in one line (*lo que hay, hay que comerlo*) and lets the reader parse `comerlo` **without ever having been taught it**, then states the honest end of a beginner's arc: not *you are finished*, but **the machinery is assembled** -- what remains is words and practice, which is not a gap. **Campaign complete: 11/11 rungs.** |
| HL-C106 | In progress — first noun slice landed at chapters 53-55 | Fix the noun famine: teach ordinary nouns early enough that the structures which operate on nouns have something to operate on. | Found while authoring rung 13. Object pronouns replace nouns, so writing `lo` and `la` requires a noun to replace — and **at chapter 53 there was not one to use.** Measured: the reader holds 75 lexical atoms there, of which the concrete nouns are `café`, `día`, `noche`, `tarde` and `mañana` — one drink and four times of day. Every other noun in the course (*pan*, *agua*, *vino*, *hermano*, *padre*, *mano*, *cabeza*, *gato*, *perro*) is introduced at **chapter 78 or later**. Meanwhile `NOUN-GENDER`, `NOUN-NUMBER`, `DEFINITE-ARTICLES` and both indefinite articles are all held by chapter 53: **the course teaches the whole apparatus for handling nouns and withholds the nouns.** Same failure as HL-C104's missing `un`/`una`, same cause — grammar was laddered, vocabulary was left to arrive as whatever an example happened to need. Recorded as HL10 §5.4b with two rules: a structure that operates on a category needs members of that category already taught, and vocabulary is a strand with its own ordering obligation rather than a byproduct of grammar examples. **First slice landed:** three nouns at chapters 53-55, chosen to be general rather than thematic — `la casa` (feminine), `el libro` (masculine), and `la comida`, which is **built from `comer`** with the `-ida` ending. The third matters most: it is the first noun the reader could have produced without being told, and the ending carries its gender, so it is a tool rather than a word. From chapter 55 every gender-dependent rule has two plain test cases to run against. **Remaining:** the noun inventory is still thin — no family, body, place, food or number-countable nouns before chapter 78. **Partly answered, and the other half is blocked on a signal the corpus does not carry.** The relocate-versus-add question was settled by doing it: HL-C112 moved `la casa` and `el libro` from chapters 70-71 to **56-57**, ahead of the adjective arc, and the four adjective lessons' examples needed **no rewriting** -- *una casa grande* simply became legal. Relocation is therefore the cheaper instrument where a noun already exists, and it fixed eight forward references rather than the four it was aimed at. **But the cross-track half cannot be measured yet.** Asking *when does each track teach its first ordinary noun* needs a way to tell a noun from a verb, and `concept_tag` cannot do it -- Spanish alone has 120 tag families of which most have a single member, so it is an identifier rather than a classification. This is **the same blocker as the vocabulary-famine sweep** recorded under HL-C111, which failed for exactly this reason and produced two false findings out of four when a hand-curated probe list was substituted for the missing signal. **The two items should be resolved together:** adding `word_class:` to the lexical schema unblocks both the famine detector and this cross-track census, and nothing else does. **The `word_class:` decision is now priced, and the answer is that it cannot be done mechanically.** A classifier built only on **corpus-internal** signals -- a headword carrying an article (*la casa*, *el libro*) is a noun; an infinitive in *-ar/-er/-ir* whose gloss opens *to …* is a verb -- reaches **60 of 259 Spanish lexical lessons, 23%**. That is enough for an **upper bound** on when the first noun appears and not enough for an answer, because any of the 199 unclassified lessons could hold an earlier one. Raising it means **authoring** the remaining 77% by hand, in Spanish and then in 21 further tracks including scripts the author cannot read -- which is the failure mode that produced two false findings in HL-C111 and two wrong etymologies in HL-C88 slices 8-9. **So this is a genuine authoring commitment rather than a cleanup, and it gates two items** (the famine detector and the cross-track noun census). Worth deciding deliberately: the cheap mechanical slice buys 23% and an upper bound; the full backfill buys the actual measurement. Recorded rather than started, because starting it halfway is the one option that costs the effort without buying the answer. |
| HL-C107 | Complete (#PENDING) | Add the `el agua` / `la bebo` payoff lesson, placed after the `agua` chapter. | Written and then pulled from HL-C105's branch when measurement showed `el agua` is taught at chapter 81, long after the object pronouns at 56-60. The lesson is worth keeping: `el agua` takes `el` for a purely phonological reason while being feminine underneath, and the existing chapter proves this with the adjective and the plural. **The object pronoun is a third proof, and a better one** — nothing sits in front of a pronoun, so no vowels collide and it reports the gender straight. That turns a memorised exception into a **test the reader can run**: when an article looks suspicious, listen for `lo` or `la`. Place it immediately after the `agua` chapter, requiring `ES-GRAMMAR-DIRECT-OBJECT-LA` and `ES-GRAMMAR-AGUA-ARTICLE-03`. **Landed at chapter 122**, immediately after the water/wine/bread chapter, where every prerequisite is finally held: the object pronoun `la` was taught at 57 and `el agua` at 121. That ordering is the whole reason this waited -- pulled from the HL-C105 branch precisely because the pronoun arc reached the payoff 64 chapters before the noun did. The lesson gives the reader **a third proof**, after the adjective and the plural, and the cheapest of the three: nothing sits in front of a pronoun, so no vowels collide and it reports the gender straight. The point it closes on is bigger than one word -- **when an article looks suspicious, the object pronoun is a witness that cannot be leaned on** -- which turns a memorised exception into a test the reader can run for the rest of their life. |
| HL-C108 | Complete (#PENDING) | Teach the plural definite articles `los` and `las`, and the consonant plural chapter one promised. | Measured during HL-C105: **no lesson introduces them.** They appear incidentally in the `agua` chapter's prose (*las aguas*) and are referred to in `ES-C29-la-hora` ("the feminine plural article"), but are never taught, so no atom exists for them. This blocks the plural object pronouns `los`/`las`/`nos` (rung 13's second half) the same way the noun famine blocked the singular ones — and it is a third instance of the HL-C104 pattern, which is now enough instances to justify a systematic sweep rather than another one-off fix. **Landed as five chapters at 56-60.** A second measurement while authoring found more than the missing articles: **the consonant plural rule was promised in chapter 1 and never delivered.** `ES-C01-dia` teaches "a noun ending in a vowel adds -s" and says "(consonant endings take -es -- later)". It is the only lesson that introduces `ES-GRAMMAR-NOUN-NUMBER`, and *later* never came, across 115 chapters. So the reader could say *días* but not *the days*, and could not pluralise any consonant-final word. The arc teaches `las` first (regular: the -s lands on article and noun alike, which is agreement doing what it always does), then `los` (the only article that changes more than its ending -- and the explanation is **the third appearance of one sound change**: `el` is a worn-down *elo*, the plural *illos* kept the vowel and became `los`, exactly as `el`/`lo` split three chapters earlier). Then the `-es` rule, anchored on **`ustedes`** -- a word the reader has used for fifty chapters without being told it *is* the consonant plural -- and framed as one rule with a repair rather than two rules: *add -s; if you cannot say it, make room*. The synthesis carries the payoff: the definite article pluralises and the **indefinite article leaves** (*tengo un libro* -> *tengo libros*), so bare `libros` and `los libros` are two different questions. That is the first place Spanish asks for **less** than English rather than more. **Unblocks** the plural object pronouns `los`/`las`/`nos`, which is rung 13's second half. |
| HL-C109 | In progress | Build every book locally before pushing, and fix the typesetting warnings that only a real XeLaTeX run can see. | Reported from a rendered page: **the table of contents overflows once chapter numbers reach three digits.** Confirmed by building locally. `book.cls` reserves `1.5em` for a chapter number in the TOC; at 10.95pt bold, `100` is **2.46pt wider than the box**, so from chapter 100 onward the number runs into its own title. Spanish crossed 100 chapters and the defect arrived in **21 lines at once**. **Nothing in the repo could have caught this.** The lesson suites, the drift gates, the bundle ceiling and the book-hash pins all passed green throughout, because none of them renders a page; only XeLaTeX can see a box overflow, and CI's log scanner reports it after the push rather than before. **Fixed** by redefining `\l@chapter` with the number box widened to `2.8em` (four bold digits, so a course that reaches 100 chapters can reach 1000 without revisiting this) and moving the section indents to match. The same build surfaced more: **three chapter titles overflowed the text block** (*Entender --- Understanding Is Stretch-* by 27.9pt) and five more were badly stretched, because `titlesec` was justifying `\huge` headings; chapter titles and section titles are now ragged-right, which is what headings should be. The capability blurb under each chapter opening is ragged-right for the same reason -- it sits in a `quote`, and a narrow measure plus a lesson id like `ES-C14-practice` cannot be justified. Spanish went from **24 overfull and 9 underfull boxes to 0 and 3**. **`\hbadness` and `\hfuzz` were deliberately left alone**: raising them would mute the warnings instead of fixing the pages, and CI's scanner would then be reading a muted log. **Standing practice added:** `code/scripts/build-books-locally.sh` builds any or all books, converts the SVG figures the way CI does, and fails on any overfull box, underfull box or missing glyph. Run it before pushing anything that touches lessons, book targets or a preamble. The script also carries the one-time macOS setup, which is real: Homebrew's TeX Live does not register its OpenType fonts with the system font database, so XeLaTeX silently falls back to `nullfont` and emits half a million missing-character lines instead of failing. **Measured across all 22 books, before and after:** 12 overfull and 24 underfull boxes over 2,875 pages became **0 overfull** and 24 underfull. Eleven of the twelve were **running heads**, not the TOC: a descriptive chapter title (*Answering, Meeting, and Three Ways to Play*) forced into one unbreakable line at the top of every verso, repeating once per page for the length of the chapter, in Italian, Portuguese, Punjabi and Tamil. Spanish had already solved this with `\chaptermark` and **the fix had never been propagated**; it is now in all 22. **Two attempted fixes were reverted because measurement said they did nothing:** setting `\lccode` on the Latin-diacritic range, and ragged-right `quote` blocks. Neither moved the underfull count, so neither earned a permanent change to 22 books' typography. The 24 that remain are transliterations no loaded hyphenation pattern will break (`likhṇā`, `bhrātṛ`, `thiṅkaḷ`, `vā̃chvũ`); one of them in a narrow measure stretches the line before it. **The CI warning baseline is now seeded** from these measurements rather than left `null`, so the zero-overfull result is gated rather than asserted. Also fixed in passing: `chinese` and `japanese` had no `book/.gitignore`, so their build artifacts were untracked-but-visible in every `git status`. **Local and CI disagree by two boxes, and CI wins.** The first CI run reported one more underfull than macOS TeX Live measured, in `russian` (0 -> 1) and `tamil` (1 -> 2) -- a hyphenation-pattern difference between the two distributions, not new damage: **overfull is 0 in every track on both**. The baseline carries CI's numbers and the provenance field records why, so the next disagreement is resolved the same way instead of re-litigated. That delta is the reason the local script is a pre-push filter and not a replacement for the CI gate. **A related audit, 2026-08-12: how far can the backlog's own status column be trusted?** Three rows were found stale in two days -- HL-C105 (status said rungs remained, body said 11/11 complete), HL-C48 (still named the join key the spec had corrected), and HL-C94 (*Queued -- planned* while all twelve of its chapters existed). **All three surfaced by accident, while working nearby.** So four more were checked deliberately, against the corpus rather than against citations: **HL-C81** -- `SPINE-SAY-WHAT-I-DO` is still one node of 46 segments, not nine, so Queued is **right**; **HL-C43** -- no driving or dictation edition target exists in `book-generation.json`, Queued is **right**; **HL-C46** -- zero lessons carry a writing block inside a non-writing lesson, so the *interspersed* segment is genuinely unwritten and Queued is **right**. **Three of three deliberate checks confirmed the status; three of three accidental findings contradicted it.** The column is not unreliable as a class -- it goes stale specifically where work landed under a different row's banner, which is exactly what HL-C88's slices did to HL-C48 and HL-C94. **The cheap proxy is not sufficient**: grepping a tag in shipped tests flags 10 of 15 rows, but an *In progress* row is legitimately cited for a partial landing, so the proxy over-reports by roughly threefold. Verify against the corpus, one row at a time, before starting anything the backlog calls Queued. **One near-miss worth recording:** the first pass at HL-C46 counted `type: writing` lessons and would have reported it landed; *interspersed* means a writing block inside an ordinary lesson, and refining the query turned 7 tracks into zero. **The last underfull boxes were found in HL-C88 and the cause was not the one the preamble comment guessed.** All three that survived in Spanish sat inside a **tcolorbox callout** -- `grammarlens`, `cousinweb` -- and neither `\tolerance` nor `\emergencystretch` moved them: rebuilding with `\tolerance=1000` and then with `\emergencystretch=1em` reported the *same three boxes at the same badness*, which is what proves the knobs were not responsible. A callout runs at a narrower measure than the body and holds the densest material in the book -- bold Spanish forms, Latin etymons with macrons, arrows between them -- none of which hyphenates under English patterns, so a justified line there has almost nowhere to break and must stretch the few spaces it has. `\tcbset{before upper=\raggedright}` removes the stretch instead of hiding it. **Measured across all 22 books: underfull 24 -> 5, overfull still 0, page counts unchanged.** The five that remain are diagnosed rather than tolerated: hindi 2 (a `quote` holding the unbreakable run *qitt/cattus/gato/chat/Katze*), latin 1 (a body paragraph dense in macronised forms), and kannada 1 + telugu 1 (weekday names in an Indic script inside a table cell). **The baseline was lowered to the new measurements**, with russian and tamil carried at 1 rather than the 0 measured locally because CI has consistently found one more box than macOS in exactly those two. **HL-C109b closed two more, and the fix had been named in the preamble for a month without being acted on.** That comment has listed slash-separated form lists -- `fui/fuiste/fue`, or hindi's `qiṭṭ/cattus/gato/chat/Katze` -- as an unbreakable token class since HL-C109, while doing nothing about them. A run like that is several words wide and TeX treats it as one word, so a line holding it must stretch the few interword spaces it has. **A slash is a natural break point** -- a reader already parses `fui/fuiste/fue` as three items -- so XeTeX's inter-character class mechanism now injects `\allowbreak` after every slash outside math. The slash's catcode is untouched, so `\url`, `\includegraphics` and file names are unaffected, and the glyph, its kerning and its spacing are identical; only permission to break is added. **hindi 2 -> 0, corpus-wide underfull 5 -> 3, overfull still 0 in every track.** **tamil paginates one page shorter** (249 -> 248), which is the change working: a run that had been forcing a loose line now breaks, the paragraph sets in fewer lines, and the saving reaches the end of the book. I had written *'which is why no page count moved'* into the preamble comment before measuring all 22; the build disproved it and the comment now records what actually happened. **`\emergencystretch` at 3em and 4em was tried on latin and moved nothing**, so it was not shipped -- the last-resort-line theory behind it was simply wrong. **The remaining three are diagnosed, and two of them are the same defect.** kannada and telugu each hold one table-of-contents entry built from a seven-word headword (`ಸೋಮವಾರ ಮಂಗಳವಾರ ...`, the weekdays); the entry wraps, and `\@dottedtocline` cancels its ragged-right fil on the last line, so a script with no hyphenation patterns has to justify it. Proved by shortening that one `\section[...]` short title in an isolated copy: the box goes and the page count holds. The fix belongs in `sectionShortTitle` in `book.ts`, which uses the whole headword -- a TOC line is a pointer, not the content -- and it is a separate change because it regenerates every `.tex` and re-pins the book hashes. latin's one (`ch25-see-you-tomorrow`, a body paragraph dense in macronised forms, badness 3088) is not yet explained and should not be guessed at. **HL-C109c explained and fixed latin's.** TeX only hyphenates a word that follows glue, so a word preceded by an opening delimiter -- `(`, `[`, or an opening quote -- is never hyphenated at all and becomes one unbreakable unit as wide as the whole phrase. The phrase was `(“tomorrow”)` on a paragraph's first line: nowhere to break, so the line stretched. **Proved rather than inferred:** deleting the delimiters cleared the box but also shortened the text, which cannot distinguish blocked hyphenation from a different fit; keeping them and writing `to\-mor\-row` by hand cleared it at unchanged length, which isolates hyphenation as the cause. `\nobreak\hskip0pt\relax` -- zero-width glue behind an infinite penalty -- restarts TeX's word scan without allowing a break after the delimiter. **Two mechanisms are needed and the reason matters:** inter-character classes act on characters read from the source, so `(` and `[` are reachable that way, but the quotes are emitted by the book generator as `\textquotedblleft{}`, a *command* -- a class keyed on U+201C compiles cleanly and does nothing. Those are wrapped at the command instead. **The build gate earned its keep twice here.** A first attempt used `\XeTeXcharclass\"201C`, where `\"` is a diaeresis rather than TeX's hex prefix, and died with exit 12. A second omitted `\relax`, and **russian failed to build**: a glue specification absorbs a following `plus` keyword, and `(plus a few invented for Slavic sounds)` is ordinary English, so `\hskip0pt` read on into `plus a` and stopped at *Missing number, treated as zero*. `\fi` does not terminate the scan -- it is expandable, so it is expanded away while TeX looks for the number. Both failures reported **zero warnings**, because a build that dies has no warnings to report; only the exit status distinguishes them from a clean run. **All 22 books: 0 overfull, 2 underfull, 0 missing, every build ok, no page count moved.** The last two are kannada and telugu's shared table-of-contents defect, and a census taken before writing that fix shows it is systemic rather than local: **177 non-practice lessons (10.6%) across 21 of 22 tracks carry headwords of four or more words**, 39 of them six or more. Only two warn today, but each of those is an unhelpful TOC line. A word cap is the wrong unit -- `di · haz · ve · pon · ten · sal · sé · ven` counts as fifteen words and reads as one list -- so the rule should be chosen against rendered width. **HL-C109d did that, and the corpus is now clean: 0 overfull, 0 underfull, 0 missing characters in all 22 books.** `sectionShortTitle` cuts to a budget of **40 display columns** -- the corpus's 99th percentile, against a median of 7 and a 95th percentile of 23, so it touches only the tail: **17 section lines in 17 chapters**, every one of them a month list, a weekday list or a season list. Combining marks count as zero columns and East Asian wide forms as two. The cut falls at a word boundary, never inside a word, and a single word wider than the whole budget is kept intact, because a truncated word is unreadable while one long word is a narrower defect than a wrapped entry. **Two things the first draft got wrong, both caught by reading the output rather than the tests.** A list-versus-phrase heuristic truncated `તમને મળીને આનંદ થયો` -- *nice to meet you*, a four-word **sentence**, not a list -- which is exactly the failure a word count invites and which I cannot detect by eye in a script I do not read. Width is the criterion because width is the requirement. And the first width cut left a **dangling separator**: `sal · sé · …`, which reads as though something were missing from the middle rather than trimmed from the end; trailing separators and commas are now dropped with the item they joined. Note also that the width metric is a **proxy, not a measurement** -- malayalam's twelve-month entry scores 64 columns and never warned, while kannada's 51 did -- so the budget was calibrated on the distribution and then **confirmed by building**, which is the only thing that knows real widths. telugu paginates two pages shorter; every other track is unchanged. |
| HL-C113 | In progress — `si` at 196-198, the preterite at 199-205, the imperfect subjunctive at 206-207, the unreal condition at 208-210 (closes `SPINE-EXPRESS-CONDITION`; B1 at 31 lessons), **B2 opened at 211-217** with reported speech (`dice que`, `dijo`, `dijo que`, `preguntó si`, `preguntó dónde`, review, synthesis --- `SPINE-REPORT-WHAT-OTHERS-SAID` closed at 7 segments); `SPINE-ARGUE-A-VIEW` opened at 218-220 with the connectives it consumes (`pero`, `también`, `tampoco` --- measured first: **`pero` had no lesson at all**, and `aunque`, `sin embargo`, `mejor`, `por eso` appear nowhere in the corpus); C1/C2 still 0 | Climb the CEFR ladder from B1 to C2, gently, one spine node at a time (HL10 §3.3). | **The starting measurement, taken before authoring anything:** the corpus had **361 A1 lessons, 480 A2, 23 B1, and zero at B2, C1 or C2** -- Spanish's 195 chapters were almost entirely pre-A1 to A2, and *B2-C2 remain authored-but-unrealized, in every track*. Of the five declared B1 spine nodes, `SPINE-NARRATE-EVENTS` (4 lessons) and `SPINE-GIVE-REASONS` (19) were realized; `SPINE-HANDLE-TRAVEL`, `SPINE-DESCRIBE-EXPERIENCE` and `SPINE-EXPRESS-CONDITION` were empty. **`SPINE-EXPRESS-CONDITION` was chosen first** because it is the GRAMMAR node -- the other two are FUNCTION nodes needing travel and feelings vocabulary the corpus does not have, which would repeat the noun famine. **And measuring the machinery reshaped the rung before a word was written.** A full conditional needs *si* + imperfect subjunctive + conditional. The conditional exists (ch123) and the present subjunctive exists (ch127-128), but **`si` was taught nowhere at all**, and **every plural preterite form is absent from the corpus** -- *hablaron*, *comieron*, *tuvieron*, *fueron*, *hicieron*, none of them anywhere, because chapters 103-105 teach the preterite in the SINGULAR ONLY. The imperfect subjunctive is derived from the third-person plural preterite (*hablaron* -> *hablara*), so it cannot be taught until that form exists. **That forces the ladder's order:** (1) `si` + present, which needs no new morphology at all; (2) the preterite plural, completing a half-taught tense; (3) the imperfect subjunctive derived from it; (4) the unreal condition. **First slice landed: chapters 196-198.** `si` pays off two things the reader already holds -- *si* (yes, ch7, from *sic*) and the diacritic accent that separates *el* from *el* (ch10) -- because Latin **sic** is **si** plus the pointing particle *-ce*, so *yes* is literally *if*, pointed at, and one accent is all that keeps them apart. Then `si` + future, whose rule is that the future may stand only in the result half (*si tendre* is not said) -- and the lesson gives the reason rather than the rule: a condition is not a prediction, so only the result half claims anything. The synthesis names what is genuinely new -- **the first sentence in the book that needs two clauses** -- and points at what is still missing, a way to suppose something untrue. **Four defects caught in review, and one is a lesson about atom names.** (1) The `si + futuro` chapter used ***iré***, **which the corpus has never taught** -- and the atom cited to license it, `ES-GRAMMAR-IR-FUTURE-SINGULAR`, is a **name collision**: it is introduced by `ES-C17-vivir-futuro` with headword *viviré* and means *the future of the -ir verb CLASS*, not the future of the verb *ir*. The reader has met *ir* only as *voy*, *vamos*, *voy a* and *iba*, and has every reason to expect an irregular future stem. Replaced with *comeré en casa*, all taught. **An atom name that reads like the thing you want is not evidence that it covers it.** (2) *Spanish will not put a future tense after si* was stated as an absolute and is only true of the conditional *si*; the *si* meaning **whether** takes the future happily (*No sé si tendré tiempo*), so the claim is now scoped and the other *si* is flagged as a different word. (3) *The order can flip and only the comma moves* -- when the *si* clause follows, the comma is **dropped**, not relocated. (4) The `byLevel.B1` pin carried a comment reading *the only B1 nodes any track has touched*, which **this very change falsified** by realizing a third node; the comment now says so. Also annotated: the `unmeasurableLessons` ratchet (which only moves down) got its own justification rather than a cross-reference, and `reinforcement.shortfall` improving 30 -> 29 is now accounted for. **Step 2 landed: the preterite plural, chapters 199-201.** This is a gap in its own right -- the reader could say *I spoke* but not *they spoke* -- and the hard prerequisite for everything above it. Three chapters, one new form each, and the middle one costs **nothing at all**: **hablaron** (the *-ar* plural, whose *-n* ending means the default stress already falls right, so unlike *habló* it takes **no written accent** -- the reading rule the reader already has does the work); **hablamos**, which is *we speak* **and** *we spoke*, identical in both tenses, because Latin kept *-āmus* and *-āvimus* apart and the second wore down onto the first -- Spanish noticed and did nothing, because context settles it exactly as English does with *I put it there*; and **comieron / vivieron**, where the *-er* and *-ir* families keep the bargain they struck in the singular and share one ending, *-ieron*. The third chapter also names what did **not** happen: *comemos* / *comimos* stay distinct, so **only the *-ar* family loses the present-past distinction**. Next: the strong plurals (*tuvieron*, *hicieron*, *fueron*), then a review and synthesis close the tense, and only then is the imperfect subjunctive derivable. **Two factual errors caught in review, and the second was self-refuting.** (1) The pronunciation respelling read *ha-bla-RON*, and the corpus convention capitalises the **stressed** syllable -- so it marked final stress in the very sentence explaining that the stress is **penultimate**, and the narration would have taught it aloud that way. Corrected to *ha-BLA-ron*. (2) The lesson claimed *only the -ar family loses the present-past distinction in the we form*. **That is wrong, and the lesson printed the counter-evidence twelve lines above its own claim:** *vivimos* is the present (taught at ch43) **and** the preterite, exactly as *hablamos* is both. **Two families out of three collapse; only *-er* keeps them apart** -- and only because its past borrowed *-imos* from the *-ir* family and so moved away from its own present *-emos*. The corrected version is a better fact than the one I invented, and it pays off ch43's *the first place -er and -ir stop agreeing* by showing the second place, where they agree again. Also scoped: *for every -ar verb* became *every **regular** -ar verb*, since *estar/estuvimos*, *dar/dimos* and *andar/anduvimos* do not collapse. **Step 3 closes the tense, chapters 202-205.** **tuvieron** -- the strong stems take the ordinary *-ieron*, and the interesting part is the stress: *TU-ve* holds it on the stem in the singular and *tu-VIE-ron* hands it back, because *-ieron* is two syllables and takes it. **fueron** -- the one plural that refuses both endings, adding only *-ron*, and serving *ser* and *ir* together exactly as *fui* did; the etymology pays off ch103's `ES-ETYMON-FUI` by noting that **Spanish borrowed the whole past of *to be* from a different verb** (Latin *fuī*, from a root meaning *to grow*) -- and that **English patched the same hole the same way**, since *was* comes from a root meaning *to dwell*, not from *be*. Then a **review** (the full six-by-three paradigm, and the honest summary that the tense only ever asked for **four memorised stems**) and a **synthesis** that names what changed: until now the past tense could only talk about *you*, and a story needs other people. The synthesis also points forward without teaching yet -- take *-ron* off any plural and *hablara*, *tuviera*, *fuera* are sitting there, which is **why the plural had to come first**. **One measurement worth carrying to HL-C43:** the review chapter is **sight-bound**, because consolidation is a side-by-side paradigm table -- it lands in the `sight` column the moment it is written, which is precisely the finding that blocks the dictation edition, **Five defects caught in review, and three of them were overclaims.** (1) The review said the tense was **finished**, with *six people* -- it has **five**. *Vosotros* (*hablasteis*) is taught nowhere in the corpus, and the book's own present-tense review defines a complete paradigm as **six forms** including *habláis*, so the standard was already set and I claimed completion below it. The strong verbs are shorter still: *tuvimos*, *hicimos* and *estuvimos* are also absent. The review now says five of six and names what is owed. (2) *Take the -ron off tuvieron* leaves ***tuvie-***, not *tuviera*; the rule is to **swap** *-ron* for *-ra*, and the wrong version had already been printed into the generated **answer key**. (3) *The stem held the stress in the singular* is false for *tuviste*, which is *tu-VIS-te* -- taught that way at ch105 -- so the claim is now scoped to *tuve* and *tuvo*, which is a sharper point anyway: **the stem only ever held the stress in two forms**. (4) *Until this chapter your past could only talk about you* -- *habló* has existed since ch103. (5) The *estuve* hook credited it with inherited stem stress, which ch105 explicitly refuses -- *estuve* is analogical. **And the best outcome came from a bookkeeping finding.** The review's 5x3 paradigm grid tripped `fullParadigmGrids`, which `info-dump.ts` calls *the exact artifact HL10 section 5.3 forbids*, and the narrator refused it (*there is a table here I cannot read to you*). Rewriting it as **three per-family chants** cleared the info-dump violation, cleared the narration refusal, and flipped the chapter to **`modality: voice`, `drivable: true`** -- so a review chapter written as chants **is** drivable. That is a worked example of exactly what HL-C43 said the dictation edition needs, produced by accident while fixing a lint counter. **Step 4: the imperfect subjunctive, chapters 206-207 — the payoff the last two rungs were built for.** The rule is one line: **take the *they* past and swap *-ron* for *-ra***. *hablaron → hablara*, *comieron → comiera*, *tuvieron → tuviera*, *fueron → fuera*. **It has no exceptions at all**, and the lesson gives the reason rather than celebrating the tidiness: every irregularity these verbs had was already spent in the preterite, so a form carved out of that one **inherits the irregularity instead of inventing new irregularity**. A tense built on a broken form comes out perfectly regular. The etymology is a genuine surprise: ***hablara* did not start as supposing** -- Latin *fabulāveram* meant **I had spoken**, an ordinary past, and it slid from *what had happened* to *what did not*, which is a short journey since both sit outside the present and only one is real. Spanish's other form for the same job, *hablase*, **did** come from a Latin subjunctive -- two different Latin tenses arriving at one modern meaning. Chapter 207 collects the four hard stems **for free**: *tuv-*, *hic-*, *estuv-*, *fue-* pay for themselves a second time, and *fuera* keeps the double duty *fui* and *fueron* had, serving *ser* and *ir* alike. **B1 lessons: 26 → 28.** **Three defects caught in review.** (1) *That is the whole rule, and it has no exceptions at all* was an overclaim: the **stem** never changes, but the *nosotros* form takes a written accent the source form has not got -- *habláramos*, *tuviéramos* -- so a reader handed *hablaron* and told that was the whole rule would produce the 1pl wrong. Now scoped to the stem, with the accent named. (2) ***fabulāveram* is not attested Latin.** *Fābulārī* is **deponent** -- it has no active perfect stem, so its classical pluperfect is *fābulātus eram* -- and I printed a Vulgar-Latin reconstruction unstarred as though it were classical, in a book whose etymology is its selling point, and whose ch28 already teaches *fabulari* as deponent. Replaced with *amāveram*, the standard non-deponent illustration, plus a note that Spanish built *hablara* on the pattern rather than inheriting it. (3) The root slug was `pluperfect-latin` -- **a grammatical category, where every other slug in the corpus is a lexeme** -- and minting it pushed `roots` and `neverSpent` up by two, where re-spending `fabulari-latin` moves `underspent` **down**. Changed, and the ledger improved rather than grew. Next: *si tuviera…, tendría* -- the unreal condition, which closes `SPINE-EXPRESS-CONDITION`. |
| HL-C110 | Complete (#PENDING) | Keep the language-ladder eager bundle under its 500 kB ceiling, and stop the check from reporting on a stale build. | CI failed the double-object PR with **500,459 bytes against a 500,000 limit**, and the local check had reported **487,797** -- the same number it had reported on the four previous PRs while forty lessons were added. **The number could not move**, because `check-bundle.mjs` reads `dist/` and nothing had rebuilt it. A measurement that cannot change is worse than no measurement, because it reads as evidence; it was quoted as passing verification five times. The script now **refuses to run against a stale build**: if anything under `src/` or the corpus is newer than `dist/index.html`, it exits non-zero and says so. **Second bug, same shape.** The set of eager chunks was a hardcoded name pattern, so when `book-ledgers` was moved behind a dynamic import the gate went on counting 500 kB the browser no longer downloads. The eager set is now **read from `dist/index.html`** -- the entry script plus every `modulepreload` -- which is the browser's own definition and cannot go stale. **The real fix** was making the diagnostic data lazy: `generated-book-hashes.json` (136 kB) and all 22 `chapters.json` (580 kB) were statically imported to compute **one word** in a metadata line, *book synced* or *book stale*. Both now load after first paint, the status reads `not-generated` until they land, and `whenBookHashesReady()` triggers a re-render. Eager chunk: **497,216 bytes**. **Still only ~2.8 kB of headroom**, and the shell grows with the corpus, so the next arc will hit this again. The `index` chunk was already 494 kB before any of this. Decide what else leaves the shell -- the lesson index and `curriculum-plans` (338 kB, still preloaded) are the candidates -- rather than raising a ceiling that exists to keep first paint honest. **Measured properly on the next arc, and my own estimate was wrong.** Adding five lessons took the eager chunk from **497,216 to 498,326** -- **222 bytes per lesson**, not the ~15 I had inferred from lesson-id length alone. Headroom is now **1,674 bytes, about seven more lessons**, so the next arc breaks CI unless this is done first. **This is now blocking, and it is the next task.** Do not estimate the per-lesson cost again: build, measure, subtract. And note that splitting one eager chunk into two would satisfy the gate (it measures the *largest* eager chunk) while downloading exactly the same bytes -- that is gaming the metric, not fixing first paint. The fix has to remove something from the preload set. **Fixed, and the cause was not what the name suggested.** `import.meta.glob` compiles to an object literal with **one entry per matching file** -- the full path as a key, plus an arrow function importing the batch it lives in. The lesson *bodies* were already lazy (that is what the 313 `lessons-*` chunks are), but **the map itself is code**, and it landed wherever it was imported. `main.ts` built its id set at module load (`const BUNDLED_LESSON_IDS = new Set(bundledLessonIds())`), so all 1,793 entries were eager: **363,818 bytes**, or ~200 per lesson, which is the 222-bytes-per-lesson growth measured on the last arc. My earlier guess of ~15 bytes counted only the id substring and missed the path and the preload wrapper around it. **The map now lives alone in `src/lesson-sources.ts`** and `lessons.ts` reaches it through `import()`; `bundledLessonIds()` and `loadBundledLessons()` became async, and the one synchronous consumer was already inside an async function. **Eager chunk: 498,326 -> 381,819 bytes**, a 23% cut, and the largest eager chunk is now `script-data`, which is per-script and **does not grow with lessons at all**. Headroom is 118 kB and structural rather than a countdown. Verified in a browser, not only in tests: no console errors, lessons render, and the lazy chunks fetch on demand. |

**Owner decisions, settled 2026-08-10** and recorded in HL10 §16:

1. The productive variety is **neutral Latin American**, with Peninsular and
   Rioplatense fully receptive from pre-A1, held as a track config key rather than
   baked into lessons.
2. **One curriculum; for now exactly one book.** The curriculum is canonical and
   books are derived views over it. No splitting yet — the driving edition
   (HL-C43), per-part editions, a writing companion and a reference edition are
   deferred generation targets, listed in HL10 §11.1 so they are visibly
   postponed rather than forgotten. No lesson may assume which edition it appears
   in. This puts a ~50× scale requirement on the book pipeline (214 pages today).

**A fourth directive, 2026-08-10, and it governs more than one lesson:** *the
book has to be useful from page 1; when deciding between etymology and
usefulness, always choose usefulness.* Etymology never decides WHICH word is
taught or when — it decides how the word already chosen is explained, and
`rootLedgerMinReuse` culls roots, never headwords. This **reverses** the earlier
decision to open the course on `gracias`: it opens on `hola`, per HL10 §6.3. The
reversal was already required by HL10 §9's own selection order (function first,
frequency second, cognate leverage third), which the `gracias` choice broke.

A third directive, same day, added the **friends** system and the
**dummy-friendly** requirement: introduce relatives from other languages so the
reader's brain can connect, and write for someone who knows no grammar
vocabulary at all. Both are specified in HL10 §6.7, §7.4 and §7.5 and carried by
HL-C88 and HL-C89 below.

## The owner's architecture directive, 2026-08-14: two reference tracks, then replicate

> *"I know we have even more work to do in other languages. But I want to layout
> the spine with Spanish and then layout the script work addendum in Tamil and
> then replicate it across all the languages."*

This supersedes "all six in lockstep" as the **method**, not as the goal. The goal
is still every language pre-A1 → C2. The method is now:

| | reference track | what it establishes | replicated to |
|---|---|---|---|
| **SPINE** | **Spanish** | the meaning ladder pre-A1 → C2: what each rung contains, in what order, at what pace | all 22 tracks |
| **SCRIPT ADDENDUM** | **Tamil** | the drizzle: letter order by word payoff, one character per segment, cited ductus, the filmstrip figure, and where the decoding ladder closes | every non-Latin track |

**Why this is better than what I was doing.** Six tracks in lockstep means every
design mistake is made six times and fixed six times. Wave I proved it: the
placement error, the forward references, the duplicated block type — each landed
in six tracks at once. One reference track makes a mistake cost one track, and
replication carries a pattern that has already survived contact.

It also matches where the two tracks actually are. Spanish is furthest up the
meaning ladder and has the exam-coverage gate to prove a rung is passed. Tamil is
the only track with a cited stroke order, so it is the only one where the script
addendum can be built end to end rather than stubbed.

**What this changes in practice.** A new pattern is authored ONCE, in its
reference track, and proved there — through the gates, into the book, read back
from the PDF. Only then is it replicated, and replication is a generator run plus
per-track review rather than six hand-authorings.

**What it does not change.** The gentle ramp, the drizzle riding inside the
meaning tranches rather than in front of them, and page count never being a
constraint. Those are properties of the pattern; establishing the pattern once
makes them easier to hold, not optional.

### The work this method implies

| ID | Status | Work item | Completion signal |
|---|---|---|---|
| HL-C152 | **NEXT (Spanish side)** | **Realize the thirteen unrealized spine nodes in Spanish**, lowest stage first: A2's one (`SPINE-NEGATE-AND-ASK`), B1's two, B2's two, then **C1's four and C2's four — which have ZERO realizations in the corpus's most advanced track.** Each needs the six parts HL13 §3 lists: entry lessons, vocabulary floor, grammar points, text shape, task shapes, payoff. | All 33 spine nodes realized in Spanish; the gates read clean per rung. |
| HL-C153 | Not started | **Deepen each Spanish rung to the density the lower ones have** — roughly twenty lessons per node, against 33 nodes. This is where the page count goes, and where *"do not worry about the number of pages"* is doing real work. | Every rung carries comparable weight; no rung is a single lesson standing in for a level. |
| HL-C154 | **Ledger complete (24/24); conjuncts, running text and the closing lesson remain** | **Complete Tamil's script addendum end to end** — the letter ledger's 24 positions, conjuncts, running text, and the named lesson where decoding closes. Tamil is the reference because it is the only track with a cited stroke order. | The addendum is whole in one track and ready to replicate. |
| HL-C155 | Not started | **Replicate the spine layout to the other 21 tracks**, one stage at a time, as a generator plus per-track review. | Every track has the same rung anatomy at every stage it has reached. |
| HL-C156 | Not started | **Replicate the script addendum to every non-Latin track**, recognition-first where the ductus is uncited. **No citation → no pen path → no figure** travels with it. | Every non-Latin track teaches its script; uncited pen paths are reported as debt, never invented. |


## The owner's standing goal, restated 2026-08-14

> *"For every language I want to start with pre-A1 level (absolute beginner) to C2."*

**Every language**, not only the six Indic tracks and Spanish. The registry holds
22, and the ladder in HL12 §3 is the shape all of them climb: pre-A1 → A1 → A2 →
B1 → B2 → C1 → C2, with the script drizzled in one character at a time underneath
for those whose reader cannot already read the alphabet.

Three constraints, all standing, all repeated by the owner more than once:

1. **The ramp has to be gentle.** Every rule in HL08/HL11/HL12 costs lessons, and
   paying that cost is the point.
2. **The script drizzles in slowly**, alongside the meaning ramp rather than in
   front of it — the book stays useful from page 1.
3. **Page count is never a constraint.** *"We can always split the book in the
   future."* Where a lesson is too big it splits; no rule is relaxed and no
   lessons merged to keep a book short.

The six Indic tracks lead because they are furthest behind. The other fifteen
follow the same order once these are moving under their own loop.

## The owner's direction, 2026-08-14: alternate Spanish and the Indic six

> *"It also looks like the Spanish loop stopped for some reason. So, alternate
> between Spanish and the Indian languages if you can. Spanish is obviously
> ahead. But I want all the Indian languages to eventually catch up."*

**Why it stopped, measured rather than assumed:** it did not stall. HL-C128
finished — A1 grammar coverage reached **85/85**, every point the Plan Curricular
enumerates — and no successor item was claimed. Spanish committed lessons on
every day from 2026-08-07 to 08-13 and then stopped the day the item closed.

So the loop alternates from here: one Spanish item, one Indic item, repeating.
Spanish stays ahead by design; the point of alternating is that the six close the
gap without Spanish going cold, which is what happened this week.

| next Spanish | next Indic |
|---|---|
| **HL-C129** connected prose — a reader can finish this book having never read a Spanish paragraph | **HL-C148** classify the owner's headings in `parse.ts`, then migrate 188 lessons to schema v2 |

**HL-C129's premise was wrong, and it took three measurements to find out.**

The row claimed the longest continuous Spanish in the book was 10 words. My first
re-measurement agreed with it -- and reproduced its exact error, counting single
`*...*` italic spans, which chops a multi-line dialogue into one run per line.
My second over-corrected: counting whole blockquotes found "139 passages of 12+
words", but most of those blocks are ENGLISH explanation, which is how a 61-word
"Spanish passage" in chapter 1 turned out to be a note about *hola* not being
related to *hello*.

The third, filtering blockquotes by language: **29 genuinely Spanish passages of
10+ words, 26 of 12+, five of 20+, one of 30+.**

Two things follow. The gap is real -- roughly one connected passage every
fourteen lessons, and a single one that reaches thirty words, against a DELE A1
reading paper made of connected texts. And a lesson I had already written and
validated for this item was **deleted before shipping**: it added a 34-word
greeting dialogue at chapter 40, which `ES-C05-practice` already does at chapter
13, more compactly. Teaching at chapter 40 what chapter 13 teaches is worse than
not writing it.

Measure by the unit the reader actually reads. A dialogue is one passage, not
eight runs; and a blockquote is not Spanish just because it sits in a Spanish
book.

## P0 — THE INDIC DRIVE ORDER: pre-A1 to C2, all six in lockstep

**2026-08-13.** This is the standing work order for Tamil, Telugu, Kannada,
Malayalam, Hindi and Sanskrit. It exists because the gap is not a difficulty
problem, it is an allocation problem, and the allocation is visible in the log.

### Where the six actually are, against Spanish

| track | lessons | atoms | vs Spanish |
|---|---:|---:|---:|
| **spanish** | **366** | **562** | — |
| tamil | 141 | 206 | 39% |
| hindi | 117 | 160 | 32% |
| malayalam | 101 | 135 | 28% |
| kannada | 96 | 136 | 26% |
| telugu | 95 | 132 | 26% |
| sanskrit | 65 | 73 | 18% |

### Why they are there, stated as an accounting fact

**Spanish has a drive loop. The six have a queue slot.** In the last seven days:
85 content commits to Spanish, 29 across all six Indic tracks combined. Over the
whole history: 85 lesson-adding commits for Spanish, 8-23 each for the others.

The six are fed by rotating vocabulary waves, and the rotation is the problem:

| wave | tracks served |
|---|---|
| 2 | French, German, Portuguese, Italian |
| 3 | Russian, Bengali, Gujarati, **Kannada** |
| 4 | Marathi, Punjabi, **Sanskrit**, Urdu |
| 5 | Persian, **Telugu**, **Malayalam** |
| 6 | Russian, Persian, Urdu, Bengali |

Four tracks per wave out of ~16, ~14 lessons each. **Kannada's last wave was #3.
Tamil and Hindi have never had one.** At 14 lessons a wave, closing a 250-lesson
gap takes ~18 waves for one track, and a track gets a wave about one time in
four. That is not slow progress, it is an arithmetic that never arrives.

The fix is this order: **the six get their own loop, and every tranche serves all
six.** Not a slot in a rotation.

### The shape of the climb

Two ramps, per HL12 §2, and only one of them ends:

```
DECODING  letters -> vowel signs -> vowel-killer -> conjuncts -> running text
          -> speed -> "you can read anything now"          <- IT CLOSES, at A1

MEANING   pre-A1 -> A1 -> A2 -> B1 -> B2 -> C1 -> C2       <- the whole climb
```

Never steepen both in one lesson (HL12 §2.1). Script segments ride inside the
meaning tranches rather than forming their own front, which is what "drizzled"
means and what keeps the book useful from page 1.

### Sizing, stated honestly and not used as a constraint

HL09 §3 puts a complete track at ~8,000 lessons; six is ~48,000. The pre-A1 floor
alone is ~300 words against today's 53-86, so roughly **230 words x 6 tracks =
1,400 lessons** before a single track can honestly claim its first rung. At ~12
words per track per tranche that is ~19 tranches. Those are the real numbers.
Per the owner's standing rule, page count is never a reason to compress: the
answer to 19 tranches is 19 tranches, not 8 fatter ones.

### The order

Priority is by what unblocks the most, then by what a reader feels soonest.
LEXICON leads because it is what the six are actually short of, and because
HL12 §4.2 names it as what carries the weight once the script retires.

| ID | Status | Work item | Completion signal |
|---|---|---|---|
| HL-C136 | **Done** — 42 lessons, 6 chapters, all six tracks; the whole wave is `voice` | **pre-A1 LEXICON drive, wave I of ~19.** ~12 headwords per track, all six, chosen by the HL10 §9 selection order (function first, frequency second, cognate leverage third). Every headword carries a `romanization`, so it is usable before its letters exist. | +72 lessons; `runLevelGate` pre-A1 vocabulary rises for all six in one tranche; every book still builds; no chapter crosses its atom budget. |
| HL-C137 | Queued behind HL-C149 — wave II | **Repeat HL-C136 to the floor** — waves II..~XIX, same shape, until every track holds ~300 pre-A1 words. Each wave is one PR. | pre-A1 **attained** for all six on `runLevelGate` (this is HL-C135's completion signal, reached by this route). |
| HL-C138 | Not started | **Finish the letter ledgers.** Each of the five scripts has 24 authored positions; 6-8 are taught. Author the remainder as one-character segments, drizzled into the meaning tranches at one per chapter. | 24 of 24 positions taught per script; `unspent letters` 0; closure violations fall per track. |
| HL-C139 | Not started | **Conjuncts and ligatures** — the decoding rung after the vowel-killer, and the one where these scripts stop being decomposable letter by letter. | Each track teaches its script's conjunct formation; a reader can decode a conjunct they have not seen. |
| HL-C140 | Not started | **Running text, speed, and the closing lesson.** Names the moment the decoding ladder ends (HL-C132) and prints the reframing: from here on the difficulty is meaning. | One named closing lesson per track; the book says so on the page; SCRIPT is silent after it. |
| HL-C141 | Not started | **A1 rung, all six.** ~1,000 words cumulative per HL09 §3, plus the A1 can-do descriptors. | A1 attained for all six; the script strand has closed underneath it. |
| HL-C142 | Not started | **A2 rung**, all six. Romanization now absent (HL-C133), closure strict and gating. | A2 attained; no A2+ lesson carries a headword romanization; closure violations 0. |
| HL-C143 | Not started | **B1 rung**, all six. | B1 attained. |
| HL-C144 | Not started | **B2 rung**, all six. | B2 attained. |
| HL-C145 | Not started | **C1 rung**, all six. | C1 attained. |
| HL-C146 | Not started | **C2 rung**, all six. The top of the ladder. | C2 attained for all six. |
| HL-C147 | Not started | **Split each track into pre-A1 … C2 editions** — one curriculum, N derived books, filtered by level. Deferred on purpose: the split is a filter over the source and must not shape it. | Each track emits per-level books from the same lessons; the driving edition is unaffected. |
| HL-C149 | **NEXT, before wave II** | **Derive the corpus-count pins instead of hard-coding them.** Fourteen snapshot assertions across seven test files hard-code counts (`totalLessons`, `chapterCount`, `atomsTaught`, `missedByWindow`, the whole modality summary). Every content tranche moves all of them, so every tranche conflicts with every other PR that moves any of them — and this repo lands a PR every few minutes. Wave I hit exactly this and had to be regenerated from a fresh base. Keep the running ANNOTATIONS, which carry the reasoning and are the valuable part; derive the NUMBERS from the corpus at test time, and assert the *relationships* that actually encode intent (`voice + sight + pen == total`, `drivablePrefixTotal` does not fall, `pen` moves only when a script lesson lands). | A content tranche can be authored, verified and merged without touching a test file; the reasoning notes survive; a regression that a raw count would have caught is still caught by a relationship. |

Riding alongside, not blocking the rungs:

| ID | Status | Work item | Completion signal |
|---|---|---|---|
| HL-C115 | Not started | `letter-ductus` filmstrip figures. **Both blockers merged**: `script-ductus` exposes the filmstrip to the book generator and `path-raster` renders it to PNG. | Declared in `core/figure-generation.json`, byte-gated, proved on Tamil's cited letters and Devanagari's अ and आ. |
| HL-C118 | Not started | Cited ductus for Telugu, Kannada, Malayalam and the rest of Devanagari. **No citation → no pen path → no figure**; the gap is reported, never filled by invention. | Every authored pen path font-verified and carrying a `strokeOrderSource`. |
| HL-C134 | **Half done.** Prose carried (91 blocks, parity 0 for all six); the FLIP is blocked on HL-C148 | Rewrite the handwritten chapters 1-5 into the generated pipeline for all six. The carry is complete and verified. The flip is not: `generate:books` refuses a schema-v1 lesson, and chapters 1-5 hold **188** of them. | Parity 0 per chapter *(done)*; then chapters move out of `handwritten` and every book still builds *(blocked)*. |
| HL-C151 | Not started | **Split the two lessons the carried prose pushed over five minutes** — `KA-C01-namaskara` at 333s and `TE-C01-namaskaram` at 314s. Nothing about these lessons changed; a reader of the book always read every one of those words. What changed is that the markdown now knows, so the gate can see them. Newly visible debt, not new debt — and the rule is that a lesson too big splits, never compresses. | `durationViolations` back to 0 without any threshold being moved. |
| HL-C148 | **NEXT — the critical path**, and bigger than a frontmatter rewrite | **Migrate the six tracks' 188 schema-v1 lessons.** `migrate_schema_v2.py` does the frontmatter: `spine_node` from `curriculum.json` (163 of 188 derivable), `duration` from `est_minutes`, coverage fields by lesson type, and one knowledge atom per lesson with its `hl-knowledge` directives. What it CANNOT do is the part that blocks it: v2 rejects any `##` heading `parse.ts` does not classify, and these lessons use the owner's own headings — *The word*, *The phrase, assembled*, *Across the family*, *Using them*, *The engine*. Teach the parser those headings, per the precedent it already sets for *letters in this word*; do not rewrite the author's prose to fit a parser. | All 188 are schema v2; `generate:books` accepts chapters 1-5; HL-C134's flip proceeds. |
| HL-C149 | **Done** | **Derive the corpus-count pins instead of hard-coding them.** Twenty snapshot assertions across six test files hard-coded counts that every content tranche moves — so every tranche conflicted with every other PR that moved any of them, and this repo lands a PR every few minutes. Converted to the shape each number actually has: **floors** for content volume, **ceilings** for inherited debt (stricter than the pin was — a ratchet that cannot slip back), **ratios** for debt that grows with honest content. The running annotations stay; only the digits churned. | Mutation-tested both ways: deleting one lesson still fails the floors, and adding 42 lessons passes without touching a test file. |
| HL-C150 | **NEXT** | **The six tracks teach core words about thirty chapters after they first use them.** Measured on merged main, before any new content: **46 forward references** across the six. Tamil's chapter 1 practice uses வா, taught in chapter 32. Chapter 3 uses இரு, taught in chapter 32. Malayalam's chapter 1 uses ഉണ്ട്, taught in chapter 32. Kannada's chapter 4 uses ಬಾ, taught in chapter 32. This is the corpus's shape, not one wave's mistake — and wave I was about to add six more of it by appending its chapter at the end. Move the words that the opening chapters already lean on to where they are used. | `forwardReferences` in the six falls toward zero; no core word is taught more than a few chapters after its first use; every book still builds. |


### Discovered while running the loop (2026-08-13)

**HL-C149 was found by wave I, not predicted.** The tranche was authored,
verified and pushed green, then went `DIRTY` three times before it could merge:
main kept landing chapters, and both sides had moved the same twenty
corpus-snapshot pins. Neither side's numbers described the merged corpus, so the
conflict could not be resolved by choosing — the wave had to be reset onto the
new base and every measurement re-derived. Nineteen waves would have meant
nineteen of those, so HL-C149 was pulled ahead of wave II.

The rule it protects is worth stating, because the tempting fix is the wrong
one: **never resolve a conflicted snapshot by taking either side.** A pin is a
measurement, and a measurement of neither corpus is not a compromise, it is a
wrong number with a confident annotation attached. Regenerate from the merged
base.

**HL-C150 was then found by HL-C149** — one gate catching what another had
hidden. Converting `forwardReferences` from an exact pin into a ceiling turned it
from a number nobody read into a ratchet, and the first thing it caught was wave
I itself. Not a defect in the new lessons: the wave *revealed* forward references
that were already there, because it is the first thing in the corpus that teaches
words chapter 1 has been using untaught since it was written. The lesson
generalises past this wave — a word the opening chapters already lean on belongs
near the opening chapters, and "append a chapter at the end" is the wrong shape
for function words however good the chapter is.

**Correction to the HL-C150 finding as first written.** It originally said that
`ML-C01-athe` uses അത് untaught in chapter 1. That specific claim was wrong, and
checking it is what found the real one. Two of the examples were homographs
rather than uses: Malayalam അതെ ("yes") merely contains the letters of അത്
("that"), and Tamil `TA-C33-puri` uses **-அது** as a THIRD-PERSON VERB ENDING
(புரி + கிற் + அது) and not as the demonstrative pronoun. Same letters, different
morpheme, and the detector cannot tell them apart.

Thirteen of the forty-six referenced words are three characters or fewer, so some
share of the rest is the same artifact. The finding survives the correction and
gets bigger: the genuine cases -- வா, இரு, ഉണ്ട്, ಬಾ, all used in chapters 1-5 and
taught in chapter 32 -- are core verbs, not edge cases, and they are pre-existing.
Wave I did not create this. It was about to join it.

Worth carrying forward as its own small item: the forward-reference detector
matches substrings, so its raw count overstates. Sharpening it to word boundaries
would make the ceiling in `continuity.test.ts` mean what it says.

**HL-C134 is the unblocker, and it is bigger than it looks.** The owner
authorised rewriting the handwritten chapters on 2026-08-13, which removes the
consent question but not the engineering one. Moving a chapter from `handwritten`
to `targets` regenerates it from the lesson markdown — and the markdown is a
subset of what is in the `.tex`. Pronunciation guidance especially was written
straight into LaTeX and never went back: of the 88 blocks at risk, the
overwhelming majority are `sounds`.

Every gate would have stayed green through that deletion, because nothing
compared the two representations. `handwritten_parity.py` now does, report-only
per the HL05/HL08 precedent, and it is the precondition for the migration rather
than part of it: a chapter may be flipped when its parity is zero, and not
before.

**HL-C134 split in half, and the second half moved behind HL-C148.** The prose
carry is done: 91 blocks out of the hand-written LaTeX and into the lessons, with
`handwritten_parity.py` reading 0 for every chapter of all six tracks. Generating
those chapters would now delete nothing.

The flip itself does not work yet, and the reason was not in the plan:
`generate:books` refuses a schema-v1 lesson outright, and chapters 1-5 hold
**188** of them — 33 in Tamil, 34 in Hindi, ~30 in each of the rest. So HL-C148,
which had been sitting in the "riding alongside" list as bookkeeping, is actually
the critical path for the entire opening of all six books. It is promoted.

Four conversion bugs were caught during the carry, each of which would have
quietly corrupted the owner's writing rather than failing: `\section[short]{long}`
merged two lessons and gave one's prose to the other; nested braces in
`\emph{va-\d{n}ak-kam}` left raw LaTeX on the page; a script-font pattern that
assumed a leading `t` passed `\ml{}` and `\kn{}` through untouched; and — the
worst — the carry matched a full heading while the parity check matched a
fragment, so a lesson that already had "The *phrase*, taken apart" was given a
second etymology block while parity reported the chapter safe.

**HL-C148 is not a frontmatter rewrite.** The frontmatter half is done and
committed as `migrate_schema_v2.py`: 29 of Tamil's 33 migrate cleanly, the other
4 are chapter practice lessons with no path node and are reported rather than
guessed. Running it surfaced the real blocker.

Schema v2 rejects any level-two heading that `parse.ts` cannot classify, and
these lessons are full of the owner's own headings — *The word* (5 lessons),
*The phrase, assembled*, *Across the family*, *Where the word fits*, *Using
them*, *The reply*, *The engine*, *Build the sentences*. In Tamil alone that is
17 headings across 11 lessons; across six tracks it will be several times that.

The fix is to teach the parser, not to rewrite the prose, and the file already
sets that precedent for itself. Its comment on *letters in this word* says the
heading appears in "240 lessons across 12 tracks", that classifying it honestly
"costs drivability and buys a migration path", and that "the driving edition is a
filter over the modality flag, not a quality bar, so the honest label is the
right one." Same reasoning applies here: *The word* is an `input` block, *Across
the family* is `etymology`, *Build the sentences* is `guided-production`.

Order for the next session: classify the headings in `parse.ts` first, with a
test per heading; then run the migrator per track; then HL-C134's flip; then the
chapters 1-5 placement that everything else has been waiting on.

**Tamil's letter ledger is complete: 24 of 24 positions taught as their own
one-character lesson.** Fifteen segments were added to the nine the drizzle
already had. Three of the fifteen (ல, அ, ஆ) carry a **cited** stroke order and
print a real numbered pen path — ல is four joined movements with no pen lift,
cited to Radhakrishnan Frame 9. The other twelve teach recognition and ask for
tracing, and say on the page why the order is not given.

That is the reference addendum's first half, in the reference track, exactly as
HL13 §5 orders it. Conjuncts, running text and the named lesson where decoding
closes are still ahead.

**Three generator defects were found and fixed getting there**, all of them
caught by gates and reverted before anything reached the corpus:

- numbering segments by counting files on disk is not idempotent — a second run
  counted the first run's output and numbered from S17, orphaning 38 files.
  Numbering is now by **ledger position**, which is stable across re-runs;
- merging registration too hard put the nine drizzle lessons into **two**
  extension nodes at once. The PATH node holds every segment a track has; an
  EXTENSION holds only the ones it is responsible for;
- chaining each segment's prerequisite to `n-1` assumed consecutive numbering,
  which ledger positions are not once some letters are already taught. Each
  segment now chains to the previous segment **in its own run**.

### The loop this order is executed by

One work item per PR. Push, watch CI and mergeability, auto-merge when green,
then take the next item. New work discovered mid-item is logged here and the
order re-prioritized before the next item is picked up — not deferred to memory.

## P0 — The Indic ladder, pre-A1 to C2 (HL12)

**2026-08-13.** [HL12](../../specs/HL12-indic-pre-a1-to-c2.md) is HL10's
counterpart for the six Indic tracks. HL11 fixed how the *script* arrives; HL12
fixes where the whole book is going, and the measurement that opens it is worse
than the tracks' own claims:

| track | touches | **attained** | vocabulary at or below pre-A1 | script lessons |
|---|---|---|---:|---:|
| tamil | A2 | **none** | 86 | 24 |
| hindi | A2 | **none** | 86 | 11 |
| telugu | A2 | **none** | 79 | **0** |
| kannada | A2 | **none** | 80 | **0** |
| malayalam | A2 | **none** | 84 | **0** |
| sanskrit | A2 | **none** | 53 | **0** |
| *spanish, for scale* | *B2* | *none* | *153* | *n/a* |

Every one of the six **points at** A2 and has **attained nothing** — not even
pre-A1, which HL09 §3.1 sets at roughly 300 words. The ladder has not been
climbed at all, and the first rung is four times taller than what exists.

**The owner's direction, 2026-08-13**, and it is the most useful sentence written
about this curriculum: *"I assume after a sizable number of lessons, reading the
script would become second nature. But what the script means will become a
problem."* That is two ramps, and they behave differently — decoding is finite
and **ends**; meaning is unbounded and is the whole climb to C2. HL12 §2.1 turns
it into a rule: **a lesson may sit at the frontier of decoding or of meaning, not
both**, because a reader who fails a lesson that is new in both cannot tell which
one they failed, and the two need opposite remedies. Measured over the six
tracks' 577 lessons: **59 ask for both at once**, and that number is small today
only because four of the six teach no script at all.

Two further directives, same day. Every handwritten chapter in these tracks is an
**unpublished draft and may be rewritten** — which removes the constraint that
forced HL11's Tamil placement. And **page count is never a constraint**: *"Write
as many chapters as you need. Even if the book is 10000 pages long, it is fine.
We can chop it up into pre-A1, A1...C2 in the future."* No rule may be relaxed
and no lessons merged to keep a book short; `HI-W01-shirorekha-na-ma`, twelve
Devanagari glyphs in one lesson, is exactly what economising looks like.

*(Renumbered on 2026-08-13: this section first claimed HL-C123–127, which
HL-C126 and HL-C127 were already carrying for Spanish — `SPINE-DESCRIBE-EXPERIENCE`
and the `vosotros` rung, both cited in merged commits. Those keep the ids; these
rows moved. A work-item id is a name two commits can agree on, so a duplicate is
worse than a gap.)*

| ID | Status | Work item | Completion signal |
|---|---|---|---|
| HL-C131 | Not started | Measure the both-ramps-steep set in `ramp.ts`, report-only beside the closure numbers: per track, the lessons whose new glyphs and new atoms arrive together, and which of the two frontiers each lesson sits at. | The 59 appear in the gap report, attributed per track and per lesson id; the list is a burn-down, and nothing throws. |
| HL-C132 | Not started | Name the lesson in each track where the decoding ladder **closes** — after which the script is never a topic again — and say so on the page, reframing what comes next as meaning. | Each track declares one closing lesson; the book prints the reframing; a track with no closing lesson is reported, not silently accepted. |
| HL-C133 | Not started | Schedule romanization's removal: present on every headword at pre-A1, first use only at A1, absent at A2 and above, with closure becoming a **gate** exactly as the exemption is withdrawn. | The schedule is measured per track and per level; no A2+ lesson carries a headword romanization; closure violations at A2+ are zero, and gating is on. |
| HL-C134 | Not started | Rewrite the handwritten draft chapters into the generated pipeline for all six tracks, carrying the prose across intact. This is what unblocks placement, ordering and every gate at once. | No track has a protected handwritten chapter; every lesson has a `sequence`; Hindi's 11 writing lessons reach the page rather than only the answer key. |
| HL-C135 | Not started | Author the pre-A1 rung honestly for all six — the ~300-word floor — in lockstep, splitting rather than compressing. | `runLevelGate` reports pre-A1 **attained** for all six; the A2 *touches* claim is withdrawn until it is earned. |

## P0 — The drizzled script ramp for the six Indic tracks (HL11)

**2026-08-12.** [HL11](../../specs/HL11-drizzled-script-ramp.md) covers the ramp
none of HL08–HL10 can express: the one a reader climbs who does not already know
the alphabet. HL08 caps how fast glyphs arrive; nothing checks whether the reader
was ever *taught* them, and the six Indic tracks — Tamil, Telugu, Kannada,
Malayalam, Hindi, Sanskrit — fail on that axis in both directions at once.

Measured over the committed corpus, walking each track in `sequence` order:

| track | on page 1 | by lesson 10 | by lesson 50 | writing lessons | first one at |
|---|---|---|---|---|---|
| tamil | 4 letters + 1 mark | 15 + 8 | 38 + 11 | 20 | `sequence: 270` |
| telugu | 4 + 4 | 38 + 11 | 49 + 12 | **0** | — |
| kannada | 7 + 5 | 39 + 11 | 52 + 12 | **0** | — |
| malayalam | 9 + 5 | 44 + 12 | 52 + 13 | **0** | — |
| hindi | 7 + 5 | 21 + 7 | 41 + 14 | 11 | lesson 1 |
| sanskrit | **12 + 8** | 24 + 11 | 35 + 13 | **0** | — |

`TA-C01-vanakkam` prints வணக்கம் at `sequence: 10` with not one of its letters
taught, and Tamil's writing strand does not begin for another 260 sequences.
`SA-C06-numbers-1-5` opens twenty distinct Devanagari glyphs in one lesson. Four
of the six tracks have no writing strand at all.

**The owner's direction, 2026-08-12**, which rules out the obvious fix: *"The books
have to be useful from page 1. So, do not start with script first. Instead, you can
slowly ramp up on that. For example, teach greetings first and then slowly drizzle
in one letter at a time. By the time we get to Lesson 50 or something like that we
could have the readers be able to write a few words."* Also settled the same day:
all six tracks advance in lockstep; figures are PNG raster; existing lessons are
kept and re-placed where they work and rewritten where they do not; and a letter
with no citable stroke order ships prose-only with no pen path and no figure.

The measurement that makes the instruction easy rather than ambitious: choosing
letters greedily by how many whole headwords each completes, **seven or eight
glyphs unlock five real words in every one of the six tracks** (first writable
word at 5, 2, 3, 3, 1, 1 glyphs respectively). The same walk in traditional
recitation order completes zero words after twelve glyphs — which is why HL11 §4
orders letters by payoff and records the order as a per-script letter ledger.

| ID | Status | Work item | Completion signal |
|---|---|---|---|
| HL-C122 | Not started | Add `image-codec-png` under `code/packages/typescript/`: a zero-dep PNG encoder over the existing `deflate` and `pixel-container` packages, mirroring `image-codec-bmp`'s `ImageCodec` shape. PNG rather than BMP because the books are XeLaTeX and XeLaTeX embeds PNG. | `encodePng`/`decodePng`/`PngCodec` round-trip; the package carries BUILD, README, CHANGELOG and >95% coverage; an encoded strip opens in XeLaTeX. | *(Renumbered from HL-C113 on 2026-08-12: that id was already carrying the CEFR B1-to-C2 climb, which eight merged PRs cite in their commit messages. The climb keeps the id; this row moves.)*
| HL-C114 | **Done** — `code/packages/typescript/script-ductus`, 845 tests; `data.ts` moved with them as `scriptdata.ts` so a pen path and the script file that cites the same letter live together | Extract `script-ductus` from the app. `language-ladder/src/{strokes,ductusview,truetype}.ts` hold `DUCTUS`, the font-outline reader, and the filmstrip builder; nothing under `code/packages/` may depend on something under `code/programs/`, so the book generator cannot reach them today. `ductusview.ts`'s own header already anticipates the move. | The three modules live in a package, `language-ladder` imports it with no behaviour change, and the font-verification tests (`fractionOnInk`, segment-meeting, whole-ink coverage) move across intact and still pass. |
| HL-C115 | Not started | Add the `letter-ductus` figure kind beside `etymology-route` — the only kind that exists today. One letter renders *n* panels, panel *k* showing strokes 1…*k*, the font's own outline behind in grey, the travelled path in ink, a dot at the pen, one caption per panel from the segment labels. | Declared in `core/figure-generation.json`, rasterised through HL-C113, byte-gated in `core/generated-figure-hashes.json`; proved on Tamil's eleven already-verified letters before any new research lands. |
| HL-C116 | **Done** — first measurement published; the number is 931 | Measure the script ramp's missing half in `ramp.ts`, report-only: load-bearing versus exposure target-script text, closure violations, `firstWritableWord`, the writable-word curve, letters per script segment, and unspent letters. | Every number appears in the gap report; the corpus's real closure debt is published per track; nothing throws, per the HL05/HL08 precedent. |
| HL-C117 | **Done** (letter ledgers, drizzle budget); spine nodes deferred to HL-C120 so they land with lessons that realize them | Add the `SCRIPT` strand and its pre-A1 nodes to `core/spine.json`, a drizzle budget to `core/chapter-policy.json` beside `maxNewGlyphsPerLesson`, and the per-script **letter ledger** — ordered by word payoff, authored intent, never rewritten by a validator. | Each of the five scripts has a ledger; every ledger entry names the words its letter completes; a letter that completes nothing for a long stretch is reported as unspent (the Root Ledger rule, applied to glyphs). |
| HL-C118 | Not started | Research and author cited ductus for the five scripts in letter-ledger order — Tamil outward from its eleven, and Telugu, Kannada, Malayalam and Devanagari from zero. Base letters and vowel signs only; composed syllables derive from theirs. | Every authored pen path is font-verified and carries a `strokeOrderSource` with `citation`, `url` and `variation`. **No citation → no pen path → no figure**, and the gap is reported rather than filled. Telugu has a plausible academic candidate (Vemuri, *The Shapes of Telugu*, UC Davis); Kannada, Malayalam and Devanagari are unproven and may land partly prose-only. |
| HL-C119 | **In progress** — nine Tamil one-letter segments authored and rendering (#11188); Hindi not started | Redistribute the script strands that already exist. Tamil's twenty `TA-W*` lessons are good material clustered at `sequence: 270`; Hindi's eleven include `HI-W01-shirorekha-na-ma`, the steepest lesson in the corpus at twelve glyphs. Both are re-cut into one-letter segments across the early sequences. | The prose survives the re-cut; each segment teaches exactly one letter; `drivablePercent` **does not fall** when the detachable writing segments land, which is the design's own falsification test. |
| HL-C120 | Not started | Author the missing script strand for Telugu, Kannada, Malayalam and Sanskrit, which have none, and migrate the six tracks' 233 schema-v1 lessons so they enter the gates at all. Fix the 30–34 lessons per track that carry no `sequence` — until that is done, no claim in HL11 is verifiable, because every one of them is a claim about order. | Closure violations reach zero per track; `firstWritableWord` lands near sequence 50; every book still builds and every `check:*` still passes. |

| HL-C119 | Not started | Redistribute the script strands that already exist. Tamil's twenty `TA-W*` lessons are good material clustered at `sequence: 270`; Hindi's eleven include `HI-W01-shirorekha-na-ma`, the steepest lesson in the corpus at twelve glyphs. Both are re-cut into one-letter segments across the early sequences. | The prose survives the re-cut; each segment teaches exactly one letter; `drivablePercent` **does not fall** when the detachable writing segments land, which is the design's own falsification test. |
| HL-C120 | **In progress** — 38 script segments landed across all five non-Tamil Indic tracks for Telugu, Kannada, Malayalam and Sanskrit (the four that taught nothing); schema-v1 migration and the missing `sequence` values not started | Author the missing script strand for Telugu, Kannada, Malayalam and Sanskrit, which have none, and migrate the six tracks' 233 schema-v1 lessons so they enter the gates at all. Fix the 30–34 lessons per track that carry no `sequence` — until that is done, no claim in HL11 is verifiable, because every one of them is a claim about order. | Closure violations reach zero per track; `firstWritableWord` lands near sequence 50; every book still builds and every `check:*` still passes. |

## Findings from HL-C120, payment one — the four silent tracks

- **What was actually wrong was worse than "no writing lessons."** Telugu,
  Kannada and Malayalam do not merely lack a writing strand; their script files
  carry no stroke order at all (0 of 455, 455 and 468 letters) and no component
  decomposition — every `components` entry is the syllable restating itself. So
  there was nothing to build a writing lesson out of, which is why four tracks
  had none rather than a few.
- **Recognition is a rung, not a consolation prize.** HL12 §2.2's decoding ladder
  is recognition before production at every step, so the 30 segments teach the
  eye and ask the pen only to trace a printed shape. Tracing needs no citation;
  "start here, go this way" does. When HL-C118 sources the stroke orders, writing
  segments slot in behind these without moving them.
- **The corpus already had the placement answer and nobody had asked it.**
  Measured both ways rather than argued: second-in-chapter costs 11 lessons from
  the drivable prefixes, last-in-chapter costs zero. The drivable prefix ends at
  the first lesson needing eyes, so a segment at the front truncates its whole
  chapter and one at the back truncates nothing.
- **A gentleness budget caught a real regression before it shipped.** Sanskrit's
  chapter 7 sat at exactly 12 atoms of 12, and a segment tipped it to 13. The
  generator now refuses any chapter that cannot afford one more atom, so Sanskrit
  takes six segments and its chapters 6 and 7 take none. The first version of
  that check silently passed everything because it read `introduces.knowledge` as
  an unindented key — the frontmatter nests one level in the FILE and only the
  parser flattens it to a dotted key.
- **A missing glyph is invisible in a passing build.** All four books built with
  exit 0, zero overfull and zero underfull — and 184 `Missing character`
  warnings, every one of them U+25CC, printing nothing where the character being
  taught should have been. Caught by grepping the log for the one warning class
  the usual three do not cover.
- **Still open on this row**: the six tracks' schema-v1 lessons, the 30-34
  per track carrying no `sequence`, and the rest of each letter ledger — 24
  positions authored, 6-8 taught.

## Findings from HL-C120, payment two — Hindi, and the citations nobody used

- **Hindi's writing strand existed and was unreachable.** All eleven `HI-W*`
  lessons sit in chapters 1-5, which are handwritten and protected from
  generation, so they rendered only in the answer key — while chapter 1's own
  prose promised the reader that each lesson introduces the letters its word
  needs. Eight new segments in chapters 6-13 are the first Hindi script lessons
  the book actually shows.
- **Two cited stroke orders were sitting unused.** `devanagari.json` cites nine
  letters, and अ and आ fall in the letter ledger's first positions — so the
  Sanskrit segments shipped in payment one asked the reader to trace letters
  whose pen path was already in the corpus. Both tracks now print the numbered
  path, the pen-lift count and the source. The lesson generalises: check what the
  data HAS before deciding what a lesson can claim.
- **Print what the script records, not the least common denominator.** Devanagari
  carries component breakdowns and a worked base+sign example for every mark
  (न + ◌ा = ना, *nā*); the three Dravidian files carry a sound and nothing else.
  Segments now show whatever their own script file has, so the Devanagari ones are
  longer and the Dravidian ones are not padded.
- **An example word the reader cannot say is a poor example.** Selection now
  prefers headwords carrying a `romanization`; Malayalam's first segment page had
  two of four bullets in script the reader had no way to pronounce.
- **A headword is not always a word.** Hindi's inherent-vowel lesson has the
  headword अ, so अ was offered as a word containing अ; `HI-W03-matras-naam` has the
  headword `ा, े`, a list of marks. Both are now rejected by requiring two Unicode
  letters that are not combining marks.
- **Adopting a marker turns on the check that guards it.** Marking the new
  segments `delivery: script` made the *"script strand is declared, not
  inferred"* corpus check apply to Hindi, which immediately found all eleven
  existing writing lessons undeclared.
- **Hindi chapters 10, 11 and 12 sat at exactly 2/4 atoms in their payoff** — on
  the 0.5 floor — and one segment atom took each to 2/5. Not repaired by widening
  the payoffs: a chapter promises what the reader can DO with the language, and
  recognising a character is the other ramp. Recorded in `hindi/chapters.json`.

## Findings from HL-C117

- The ledgers exist for all five scripts and the validator runs clean against the
  real corpus: 24 positions each, **0 issues**. Measured payoff, ordering letters
  by the words they complete:

| script | words in opening | after 8 | after 16 | after 24 | first writable |
|---|---:|---:|---:|---:|---|
| tamil | 30 | 2 | 11 | 18 | position 2 |
| telugu | 39 | 3 | 10 | 18 | position 3 |
| kannada | 39 | 3 | 10 | 17 | position 5 |
| malayalam | 38 | 3 | 11 | 17 | position 3 |
| devanagari (hindi+sanskrit) | 73 | 5 | 18 | 29 | position 1 |

  Tamil reaches *thank you* at the tenth glyph and its greeting at the eleventh;
  Devanagari reaches नमस्ते at the twelfth. The same walk in traditional
  recitation order completes **zero** words after twelve glyphs, which is the
  whole justification for HL11 section 4.

- **A vowel sign cannot come first**, and the payoff walk wanted to put one there
  in four of the five scripts. These are abugidas: a mark modifies a base letter,
  so a ledger opening on one describes a lesson that cannot be written down. The
  constraint costs one or two positions and is now enforced in both the proposal
  generator and the validator.

- **Families and payoff wanted the same thing**, which was not guaranteed. Tamil's
  ண/ன/ந/ற share a flat top bar and `tamil.json` already said they are best learned
  together; teaching them as a block at positions 7-10 is exactly what unlocks
  நன்றி. Devanagari's families are extractable mechanically, because its
  `components` already say things like "ध: like द with an extra inner loop" —
  eight derivational pairs, none of them asserted by hand.

- **`loadScripts` would have eaten the ledgers.** It globs every `*.json` in
  `data/scripts`, and a ledger carries the same `script` key as the inventory it
  orders, so one would have silently overwritten the other with the winner decided
  by filename sort order. Found before landing; the loader now skips the suffix.

- **The four Dravidian and Sanskrit tracks are too thin to drizzle into.** Tamil
  has 50 lessons in its first 50 sequence slots and Hindi 41, but Telugu, Kannada
  and Malayalam have ~20 each and Sanskrit 17. At one letter per two or three
  lessons, those four need roughly 30 more opening lessons authored before 16
  letters can land inside the first 50. That is HL-C120's real size.

- Telugu's and Malayalam's earliest "writable words" are grammatical suffixes
  (`-కు`, `-ിക്ക്`) rather than free words, because those tracks teach case endings
  as headwords in the opening. Reported rather than filtered: a suffix genuinely
  is something the reader writes, but a ledger milestone that is a bound morpheme
  is weaker payoff than one that is a greeting, and the authoring in HL-C120
  should improve it rather than the measurement hiding it.

## Open from HL-C116 — the OTHER way a track disappears

`measureScriptClosure` now names a track whose declared script it does not
recognise, instead of silently dropping it. That closes the mistyped-`track.json`
path. It does **not** close the path the historical Gujarati bug actually took.

`parse.ts` resolves an unregistered language to `"latin"` when `LANGUAGE_SCRIPT`
has no entry and the track ships no `track.json`. A `thai/` directory added
tomorrow would therefore resolve to Latin, be skipped as "reader already knows
this alphabet", and appear in neither `tracks` nor `unknownScriptTracks` — zero
violations, zero trace. `constants.ts` already carries a comment recording that
exact failure shipping once.

Not fixed here deliberately: the fallback lives upstream of this module and every
consumer of `ParsedLesson.script` inherits it, so changing it is its own change
with its own blast radius — not a line to slip into a measurement PR. The shape
of the fix is to resolve to a sentinel (`"unknown"`) rather than to `"latin"`
when a language is unregistered AND has no `track.json`, or to carry a
`scriptWasDefaulted` flag that closure routes into `unknownScriptTracks`.

Harmless today: all 15 non-Latin tracks are registered, `unknownScriptTracks` is
empty, and the module is report-only. It is written down because "no track can
vanish from this report" is currently true by luck rather than by construction.

## HL-C121 — paying down closure, payment one: 184 words a reader can say

**Done.** 184 of the 489 headwords that had no romanization now have one, across
all six Indic tracks. Corpus closure violations 932 -> 873; headwords without
romanization 489 -> 305.

| track | violations | headwords with no romanization |
|---|---|---|
| tamil | 68 -> **50** | 68 -> **22** |
| telugu | 83 -> **71** | 53 -> **17** |
| kannada | 85 -> **74** | 52 -> **14** |
| malayalam | 90 -> **85** | 57 -> **33** |
| hindi | 86 -> **83** | 88 -> **70** |
| sanskrit | 53 -> **43** | 26 -> **4** |

**The finding that shaped the work: mechanical transliteration is unsafe here.**
The obvious implementation was to derive romanizations from the script -- every
one of these maps to ISO-15919 by rule and `generate_syllabary.py` already has
the tables. Measured against the 195 romanizations these tracks' authors had
already written by hand, that derivation agreed with **71%**:

| track | agreement | why it differs |
|---|---:|---|
| tamil | 61% | one letter each for k/g/h, c/s, t/d, p/b — *cāppiṭu* vs *sāppiḍu* |
| hindi | 62% | schwa deletion — *kitane* vs *kitne* |
| malayalam | 51% | the half-u at a word's end — *uṇṭ* vs *uṇṭŭ* |
| telugu | 86% | anusvara assimilation — *uṁḍu* vs *uṇḍu* |
| kannada | 87% | same |
| sanskrit | 89% | same, plus vocalic-r notation |

Every disagreement was the machine being faithful to the SPELLING and wrong
about the SOUND. A romanization exists so the reader can say the word, and this
field is read aloud by the narration export — so 344 derivations would have been
344 confident mispronunciations. Two of the three worst tracks are the two with
the largest gaps.

So the tool recovers rather than derives: it takes the pronunciation each lesson
already gives its reader in prose and moves it into the field, checking the grab
against the headword's script through a per-script skeleton fold. Where nothing
matches it recovers nothing. **160 headwords still need a human** — 70 of them
Hindi, whose lessons state pronunciation in prose least often — and that is the
correct output rather than a gap to paper over.

## Findings from HL-C116

- **932 lessons across 16 non-Latin tracks ask the reader to decode a glyph
  nobody taught them.** HL08's glyph budget flags 61. That gap is the whole
  argument for the measurement: the budget caps how FAST glyphs arrive, and a
  track satisfies it perfectly while teaching no letters at all.

- **12 of the 16 non-Latin tracks teach zero letters.** Not late, not few: none.

| track | lessons | script lessons | glyphs shown | never taught | closure violations | headwords with no romanization |
|---|---:|---:|---:|---:|---:|---:|
| malayalam | 93 | **0** | 66 | 66 | 90 | 57 |
| hindi | 109 | 11 | 56 | 28 | 86 | 88 |
| kannada | 88 | **0** | 66 | 66 | 85 | 52 |
| telugu | 87 | **0** | 62 | 62 | 83 | 53 |
| arabic | 100 | 16 | 45 | 16 | 71 | 40 |
| tamil | 132 | 24 | 51 | 11 | 67 | 68 |
| bengali | 70 | **0** | 48 | 48 | 65 | 25 |
| marathi | 62 | **0** | 46 | 46 | 62 | 28 |
| russian | 64 | 5 | 55 | 37 | 59 | 0 |
| sanskrit | 59 | **0** | 48 | 48 | 53 | 26 |

- **The defect is not confined to the six Indic tracks HL11 was written for.**
  Arabic, Bengali, Marathi, Russian, Punjabi, Gujarati, Persian, Urdu, Japanese
  and Chinese all show it. HL11's rule generalises to the whole non-Latin corpus,
  which makes the six a pilot rather than a special case.

- **489 native-script headwords carry no romanization**, which is what makes them
  load-bearing rather than exposure. This is the cheapest remediation in the
  program: each one becomes exempt the moment somebody writes down how to say the
  word, and the reader genuinely gains from it. Hindi alone has 88, Tamil 68.

- **The exposure rule is doing far more work than a lesson count shows.** Only 49
  lessons corpus-wide are clean *because of* it — but it removed **1,997 glyphs**
  from load-bearing sets, most of them from lessons that violate anyway. A lesson
  reporting five untaught glyphs while fifteen more were exempted is not a lesson
  with five problems, and the per-lesson counter cannot see that. Both numbers are
  now published; the glyph count is the one that would move if an author started
  laundering script through the headword once 932 becomes a burn-down target.
  Malayalam (168), Bengali (179), Telugu (158) and Kannada (144) lean on it
  hardest; Hindi (7) and Tamil (10) barely at all, because those two mostly lack
  the romanizations that would trigger it.

- The steepest single lessons are Telugu `TE-C16-nelalu` (30 untaught glyphs),
  Kannada `KA-C16-tingalugalu` (29) and Malayalam `ML-C16-kollavarsham-maasangal`
  (24) — month-name lessons, which put a whole calendar's worth of unseen letters
  on one page.

- Tamil is measurably the least indebted of the six (11 glyphs never taught,
  against 62-66 for the three with no writing lessons), which is the difference
  its 24 script lessons buy. The measurement reflects that rather than flattening
  it, which is the check that it measures what it claims to.

## P0 — Step-by-Step capability program (HL05–HL08)

Specified in [HL05](../../specs/HL05-chapter-capability-and-step-by-step-shape.md),
[HL06](../../specs/HL06-visual-system.md),
[HL07](../../specs/HL07-spine-expansion-to-b1.md), and
[HL08](../../specs/HL08-modality-gentle-ramp-and-the-drivable-course.md). This program
adds a chapter-level capability layer above the existing lessons, gives the books a
visual system including inline script-writing instruction, grows the spine far enough
to carry a complete book, and makes the corpus teachable aloud by a voice assistant
while the learner drives. It rewrites no authored lesson content.

The measured starting point: 379 chapters, **zero** of which declare a goal or a
payoff; 11 spine nodes with **zero** at A2 or B1; **zero** images in any of the 20
books; and a complete, font-validated stroke-path model in `strokes.ts` that holds one
letter and is rendered nowhere.

On modality and ramp, measured across all 1,096 lessons: 51 need a pen and 7 carry a
script block, but of the remaining 1,038, some 322 contain a Markdown table and 56 a
sight cue — so **695 lessons, about 63% of the corpus, are drivable exactly as
authored**, and the table, not the script, is the main obstacle to the rest. The ramp
is already gentle in aggregate (mean 2.31 new atoms per lesson, median 2, p90 3) but
undefended: 52 lessons exceed a budget of 3 and the steepest teaches ten numbers at
once. Length is explicitly not a cost — splitting for gentleness is the intended
direction, and no gate may penalise page, lesson, or chapter count.

| ID | Status | Work item | Completion signal |
|---|---|---|---|
| HL-C01 | Complete (#9942) | Specify the chapter capability layer, the visual system, and spine expansion to B1. | HL05, HL06 and HL07 are committed before any implementation, per repo policy. |
| HL-C02 | Complete (#9957) | Add the `chapters.json` schema, loader, and `core/chapter-policy.json`. | `ChapterCapability`, `TrackChapters`, and `ChapterPolicy`, `loadTrackChapters` beside `loadLanguageCurricula`, all 22 track ledgers, and the policy loader and round-trip tests are shipped. |
| HL-C03 | Complete (#9994) | Land the nine HL05 gates as report-only output and publish the first chapter snapshot. | All nine stable `CHAPTER_GATE_CODES` run through the gap report; the live snapshot now measures all 513 chapters and keeps recorded debt report-only. |
| HL-C04 | Complete (#10207) | Derive book chapter titles and labels from `chapters.json`. | All 513 generated and handwritten declarations now resolve their title and label from the capability ledgers; duplicate metadata is rejected, missing capabilities fail closed, the title-drift gate remains at zero, and the shared book/app hash covers the canonical title and label. |
| HL-C05 | Complete (#10215) | Add the `pattern` lesson type and its first canonical realization. | Spanish `ES-C17-comer-futuro` introduces only `ES-PATTERN-ER-FUTURE-SINGULAR`, exposes ordered infinitive/object slots over required knowledge, and instantiates the frame three times; focused controls prove all three gates. |
| HL-C06 | Complete (#10219) | Add the figure pipeline: SVG generation, `graphicx`, SVG→PDF in CI, and a `--check` hash gate. | A generated figure round-trips from canonical data into a compiled PDF and fails CI on drift, reusing `paint-vm-svg`'s `renderToSvgString`. |
| HL-C07 | Complete (#9963) | Add the log-scanning warning gate with recorded per-track baselines. | Overfull/underfull boxes, missing glyphs, hyperref warnings, duplicate destinations, and font substitutions are machine-checked by `scan_latex_log_warnings.py` after the `latexmk` loop, against `core/latex-warning-baseline.json`. Baselines ship unseeded — `null` means unmeasured, never zero — so the gate reports today and fails the moment a seeded track regresses. The first CI run on main emits the real counts into the job summary for a human to paste back. |
| HL-C08 | Complete (#9974) | Render the ductus in Language Ladder. | `penPathD`/`penTip` drive the tested SVG stroke build-up in the app; the currently authored ductus is shared with validation and script practice. |
| HL-C09 | Queued — 157 of 228 verified | Expand `DUCTUS` to cover the ten scripts with prose stroke-order entries. | Add cited, font-checked ductus and verified pen-lift metadata for the remaining 71 entries measured by HL-C19; each passes the on-ink, join-tolerance, coverage, citation, and source-agreement invariants. Hebrew, Chinese, and the Devanagari starter inventory are complete. Arabic's three remaining entries are deferred for mismatched or unavailable sources; Cyrillic is now the smallest actionable inventory with 1 entry. |
| HL-C09A | Complete (#10222) | Verify Tamil அ as the first post-HL-C19 expansion tranche, using the primer already cited for Tamil handwriting. | அ carries a source-aligned two-stroke path with exactly one lift; its five movements, learner prose, source metadata, font-outline geometry, and rendered filmstrip agree. |
| HL-C09B | Complete (#10223) | Verify Tamil ஆ as the next source-backed expansion tranche from Frame 4 of the same primer. | ஆ carries a font-checked path for the அ body plus its long-vowel right-hand loop; its learner prose states every verified lift, and source, geometry, and filmstrip tests agree. |
| HL-C09C | Complete (#10226) | Verify Tamil இ as Frame 4's third source-backed vowel tranche. | இ carries a seven-movement, font-checked path whose learner prose states each evidenced lift; the cited order, Noto outline geometry, source metadata, and real filmstrip agree. |
| HL-C09D | Complete (#10228) | Verify Tamil க from Frame 3's final source-backed consonant row. | க carries a six-movement, font-checked three-stroke path whose learner prose states its two evidenced lifts; the cited order, Noto outline geometry, source metadata, and real filmstrip agree. |
| HL-C09E | Complete (#10230) | Verify Tamil வ from Frame 9's first source-backed consonant row. | வ carries a five-movement, font-checked unbroken path whose learner prose states the evidenced zero lifts; the cited order, Noto outline geometry, source metadata, and real filmstrip agree. |
| HL-C09F | Complete (#10234) | Verify Tamil ல from Frame 9's second source-backed consonant row. | ல carries a four-movement, font-checked unbroken path whose learner prose states the evidenced zero lifts; the cited order, Noto outline geometry, source metadata, and real filmstrip agree. |
| HL-C09G | Complete (#10240) | Verify Tamil ற from the source-adjacent Frame 10 row. | ற carries a five-movement, font-checked three-stroke path whose learner prose states the two evidenced lifts; the cited order, Noto outline geometry, source metadata, and real filmstrip agree. |
| HL-C09H | Complete (#10244) | Verify Tamil ன from Frame 13's first source-backed nasal row. | ன carries a six-movement, font-checked two-stroke path whose first five movements stay joined before the separate right upright; the cited order, Noto outline geometry, source metadata, and real filmstrip agree. |
| HL-C09I | Complete (#10249) | Verify Tamil ண from Frame 13's adjacent source-backed three-loop nasal row. | ண carries a seven-movement, font-checked two-stroke path whose first six movements stay joined through both inner arches and the top bar before the separate right upright; the cited order, Noto outline geometry, source metadata, and real filmstrip agree. |
| HL-C09J | Complete (#10252) | Verify Tamil ந from Frame 12's source-backed dental-nasal row. | ந carries a six-movement, font-checked three-stroke path adapted to the vendored Noto form: its first three movements stay joined, one lift precedes the rising middle stem and top bar, and a second precedes the right-hand descent; source, geometry, prose, and filmstrip agree. |
| HL-C09K | Complete (#10256) | Source and verify Persian ا as the first path in the smallest remaining script inventory. | UT Austin Persian Online's opening freehand demonstration supplies one top-to-bottom movement with zero lifts; the isolated Noto Naskh outline, learner prose, source timestamp and variation, downward path, and real one-frame filmstrip agree. |
| HL-C09L | Complete (#10265) | Verify Persian ب from the source-adjacent freehand row. | The cited 00:11–00:15 demonstration supplies a right-to-left Naskh bowl followed by one lift and its separate dot; the isolated Noto outline, two learner movements, prose, source metadata, and real filmstrip agree. |
| HL-C09M | Complete (#10271) | Correct the queue after checking the intervening Persian پ row, then verify the next starter entry, ت, from the same freehand source. | The audit records پ's 00:16–00:21 demonstration as deferred inventory work without changing HL-C09's denominator; ت uses a right-to-left Naskh bowl plus separately lifted left and right dots from 00:22–00:27, with the isolated Noto outline, three learner movements, prose, metadata, and filmstrip in agreement. |
| HL-C09N | Complete (#10277) | Locate and verify Persian س in the later row of the same full-alphabet demonstration. | The cited 01:29–01:35 demonstration joins three right-to-left teeth directly to the final bowl with zero lifts; the two learner movements, isolated Noto outline, metadata, and real two-frame filmstrip agree. |
| HL-C09O | Complete (#10281) | Locate and verify Persian ل in the later row of the same full-alphabet demonstration. | The cited 02:29–02:32 demonstration descends the tall upright directly into the leftward base curve with zero lifts; the two learner movements, isolated Noto outline, metadata, and real two-frame filmstrip agree. |
| HL-C09P | Complete (#10287) | Locate and verify Persian م in the source-adjacent later row of the same full-alphabet demonstration. | The cited 02:33–02:36 demonstration shapes the round head and flows directly into the descending tail with zero lifts; the two learner movements, isolated Noto outline, metadata, and real two-frame filmstrip agree. |
| HL-C09Q | Complete (#10292) | Locate and verify Persian ن in the next source-backed row. | The cited 02:37–02:43 demonstration sweeps the isolated bowl right-to-left, then lifts once to place the dot above; the two learner movements, isolated Noto outline, metadata, and real two-frame filmstrip agree. |
| HL-C09R | Complete (#10296) | Restore Language Ladder's enforced eager-bundle headroom before adding another ductus path. | The unchanged gate requires the named handwriting chunk and measures every eager chunk; the largest falls from 499,525 to 471,927 bytes while synchronous relative-path startup, direct-open, and GitHub Pages behavior remain intact. |
| HL-C09S | Complete (#10303) | Correct the source-adjacent queue, then locate and verify Persian و. | The audit confirms that و, not ه, follows ن. Its cited 02:43–02:45 demonstration loops the small head and flows into the leftward curving tail with zero lifts; the two learner movements, isolated Noto outline, metadata, and real filmstrip agree. A production-browser check also repairs the Tamil-only runtime font loader so Persian filmstrips actually render with Noto Naskh instead of silently falling back to prose. |
| HL-C09T | Complete (#10306) | Locate and verify Persian ه in the later source-backed row. | The cited 02:47–02:50 demonstration closes one simple isolated handwritten loop with zero lifts; the one-movement learner path preserves that run across the wider two-counter Noto Naskh form and leftward baseline finish, with prose, metadata, geometry, and a real one-frame filmstrip in agreement. This completes the Persian starter inventory. |
| HL-C09U | Complete (#10313) | Audit the smallest remaining Urdu inventory's sources and reconcile shared-glyph ductus identity before authoring its first tranche. | Script-aware lookup plus a scoped Urdu key make Persian ا and Urdu ا independently addressable; Northwestern's *Zer o Zabar* independent-form animation verifies Urdu ا as one top-to-bottom continuous stroke, distinct from bottom-to-top final ـا, with zero lifts and full source/data/Noto Naskh/filmstrip agreement. |
| HL-C09V | Complete (#10320) | Locate and verify independent Urdu ج in *Zer o Zabar*'s source-backed jīm chapter. | The independent animation places the dot below first, lifts once, then keeps the pointed hooked head, descent, and bowl in one continuous run. The chapter's flat-head alternative is recorded as purely aesthetic; source, prose, Noto Naskh fallback geometry, and a three-frame filmstrip agree. |
| HL-C09W | Complete (#10326) | Locate and verify independent Urdu ر in *Zer o Zabar*'s source-backed *Dāl, re, and wāw* chapter. | The independent animation and prose agree on one unbroken downward line that curves left with zero lifts. The distinct final-form motion and the chapter's Naskh/Nastaliq contrast remain explicit; source, prose, Noto Naskh fallback geometry, and a two-frame filmstrip agree. |
| HL-C09X | Complete (#10331) | Locate and verify independent Urdu س in *Zer o Zabar*'s source-backed *Sīn, shīn, baṛī he, nūn, and nūn-e ġhunna* chapter. | The independent calligraphic and handwriting animations agree on one right-to-left, zero-lift run through three close teeth and the final bowl. The chapter's optional long gentle curve remains explicit as an especially common handwriting alternative; script-scoped source, prose, Noto Naskh fallback geometry, and a two-frame filmstrip agree. |
| HL-C09Y | Complete (#10337) | Locate and verify independent Urdu ش in the same source-backed chapter. | Both independent animations finish the complete س body before three separate dot strokes: lower left, lower right, then centered above. The three sourced lifts, two-below/one-above arrangement, optional toothless body, script-scoped prose, Noto Naskh fallback geometry, and five-frame filmstrip agree. |
| HL-C09Z | Complete (#10342) | Locate and verify independent Urdu ک in *Zer o Zabar*'s source-backed Chapter 1, *Be, kāf, and short vowels*. | Both independent animations and the prose agree on two strokes: first the main-line stem, flatter bowl, and pronounced final hook; then, after one lift, the long slash descends from the upper right toward the stem. The explicit one-penstroke warning, script-scoped prose, Noto Naskh fallback geometry, and three-frame filmstrip agree. |
| HL-C09AA | Complete (#10348) | Locate and verify independent Urdu ل in *Zer o Zabar*'s source-backed Chapter 2, *Pe, gāf, alif, and lām*. | Both independent animations agree on one unbroken top-down stroke: descend the tall upright, continue below the baseline through the leftward bowl, and turn back up its outer side. The zero-lift motion, connector and final-bowl prose, script-scoped source, Noto Naskh fallback geometry, and two-frame filmstrip agree. |
| HL-C09AB | Complete (#10353) | Locate and verify independent Urdu م in *Zer o Zabar*'s source-backed Chapter 3, *Te, mīm, jīm, che, and more diacritics*. | Both independent animations keep the round head and below-baseline tail in one zero-lift run. The handwritten counterclockwise-loop guidance, calligraphic contrast, script-scoped source, Noto Naskh fallback geometry, and two-frame filmstrip agree. |
| HL-C09AC | Complete (#10356) | Locate and verify independent Urdu ن in *Zer o Zabar*'s source-backed chapter *Sīn, shīn, baṛī he, nūn, and nūn-e ġhunna*. | Both independent animations draw the below-baseline bowl first, then lift once for the dot near the baseline. The distinct initial/medial tooth form, script-scoped source, Noto Naskh fallback geometry, and two-frame filmstrip agree. |
| HL-C09AD | Complete (#10360) | Locate and verify independent Urdu ہ in *Zer o Zabar*'s source-backed chapter *Chhoṭī he, do-chashmī he, chhoṭī ye, baṛī ye, and punctuation*. | Both independent animations start at the upper right and draw one counterclockwise oval-or-teardrop loop around the base and back up to cross at the top without lifting. The distinct initial/medial divot-and-mark forms, final squiggle, script-scoped source, Noto Naskh fallback geometry, and one-frame filmstrip agree. |
| HL-C09AE | Complete (#10363) | Locate and verify independent Urdu ی in the same *Zer o Zabar* chapter. | Both independent chhoṭī ye animations start at the upper right and keep the dotless S-shaped body and below-baseline bowl in one continuous sweep to the rising left tip. The two dots belong only to initial/medial ye; that positional distinction, zero-lift motion, script-scoped source, Noto Naskh fallback geometry, and two-frame filmstrip agree. |
| HL-C09AF | Complete (#10516) | Locate and verify independent Urdu ں in *Zer o Zabar*'s source-backed chapter *Sīn, shīn, baṛī he, nūn, and nūn-e ġhunna*. | Both independent animations keep one right-to-left bowl below the baseline without lifting; the prose says the final/independent form is ن without its dot, and the vendored Noto Naskh U+06BA outline exactly matches U+0646's body contour with that dot removed. The positional distinction, zero-lift motion, script-scoped source, geometry, and one-frame filmstrip agree. |
| HL-C09AG | Complete (#10523) | Locate and verify independent Urdu ے in *Zer o Zabar*'s source-backed chapter *Chhoṭī he, do-chashmī he, chhoṭī ye, baṛī ye, and punctuation*. | Both independent animations start at the upper right, descend and sweep left across the broad bowl, curl back underneath at the far left, then continue right along the lower fold without lifting. The positional distinction, zero-lift motion, script-scoped source, Noto Naskh fallback geometry, and three-frame filmstrip agree, completing the Urdu starter inventory. |
| HL-C09AH | Complete (#10528) | Audit a source for independent Arabic ا and preserve its script identity before authoring the first path in the smallest remaining inventory. | The University of Oregon's instructional video verifies independent ا as one continuous top-to-bottom stroke with zero lifts. Its Arabic-scoped source, one-way-connector context, learner prose, Noto Naskh geometry, and one-frame filmstrip agree without inheriting Persian or Urdu provenance. |
| HL-C09AI | Complete (#10532) | Verify independent Arabic ب from the adjacent source-backed video. | The University of Oregon video verifies independent ب as one continuous right-to-left bowl followed by one lift and the dot below. Its Arabic-scoped source, two-way-connector context, learner prose, Noto Naskh geometry, and two-frame filmstrip agree without inheriting Persian provenance. |
| HL-C09AJ | Complete (#10539) | Locate and verify independent Arabic ت as the next entry in the smallest remaining inventory. | The University of Oregon page's Baa video verifies the shared right-to-left bowl; its dedicated Taa clip then places the left and right upper dots as two separately lifted strokes. The evidence split is explicit because Taa opens on the completed bowl. Arabic-scoped source, two-way-connector context, learner prose, Noto Naskh geometry, and three-frame filmstrip agree without inheriting Persian provenance. |
| HL-C09AK | Deferred — source mismatch | Verify independent Arabic ث from the source-adjacent demonstration. | The page labels `taa.mp4` as Thaa, but frame-by-frame inspection shows another Taa lesson: its first independent form draws the bowl at 00:01.1–00:02.1 and exactly two upper dots at 00:02.3–00:03.0. No third dot appears, and source search found no correctly linked Thaa asset, so ث remains conventional rather than inheriting an unsupported lift count. |
| HL-C09AL | Complete (#10545) | Reprioritize to independent Arabic ج from the next source-backed alphabet page. | The University of Oregon's Jeem clip draws the short upper head left-to-right at 00:05.1–00:05.4, continues down and around the independent bowl through 00:05.8 without lifting, then lifts once and places the dot below at 00:06.3–00:06.5. Arabic-scoped prose, source metadata, Noto Naskh geometry, and a three-frame filmstrip agree independently of Urdu's dot-first ج. |
| HL-C09AM | Complete (#10550) | Audit independent Arabic ح from the same ج/ح/خ lesson after the Jeem tranche lands. | The page's attachment ledger exposes `Haa.mov` even though its chapter body links only Jeem. The clip opens mid-mark, finishes the short left stem at 00:00.15, then visibly restarts near the stem's top at 00:00.32 and sweeps down-right and around the dotless bowl through 00:00.82. Arabic-scoped prose, source metadata, Noto Naskh geometry, and a three-frame filmstrip preserve that one-lift order rather than inheriting Jeem's body-first motion. |
| HL-C09AN | Complete (#10556) | Audit independent Arabic خ from the same attachment ledger. | The exposed `kha.mov` draws the short upper head left-to-right at 00:02.8–00:03.1, continues down and around the independent bowl through 00:03.9 without lifting, then lifts once and places the dot above at 00:04.2–00:04.4. Arabic-scoped prose, source metadata, Noto Naskh geometry, and a three-frame filmstrip preserve Khaa's own body-first order rather than assuming Jeem's lower dot or Haa's stem-first restart. |
| HL-C09AO | Complete (#10565) | Audit independent Arabic د from the next source-backed alphabet page. | The directly linked `letter-daal-2.mp4` draws independent د at 00:07.0–00:07.6 in one pen-down run: it begins at the upper tip, descends down-right through the curved shoulder, then turns left along the baseline. Arabic-scoped prose, one-way-connector context, source metadata, Noto Naskh geometry, and a two-frame filmstrip agree with zero lifts. |
| HL-C09AP | Complete (#10573) | Audit independent Arabic ر from the same source-backed alphabet page. | The directly linked `raa.mp4` draws independent ر at 00:08.8–00:09.3 in one pen-down run: it begins at the upper tip, descends through the short stroke, then sweeps left through the lower curve. Arabic-scoped prose, one-way-connector context, source metadata, Noto Naskh geometry, and a two-frame filmstrip agree with zero lifts independently of Urdu ر provenance. |
| HL-C09AQ | Complete (#10582) | Audit independent Arabic س from its source-backed alphabet page. | The directly linked `FullSizeRender-8.mov` draws independent س at 00:01.6–00:02.8 in one pen-down run: it shapes three close teeth right-to-left, then flows directly into the final bowl. Arabic-scoped prose, two-way-connector context, source metadata, Noto Naskh geometry, and a two-frame filmstrip agree with zero lifts independently of Persian and Urdu س provenance. |
| HL-C09AR | Complete ([#10592](https://github.com/adhithyan15/coding-adventures/pull/10592)) | Audit independent Arabic ش from the same source-backed alphabet page. | The directly linked `FullSizeRender-7.mov` draws the independent ش body continuously at 00:00.7–00:02.2, then lifts for the lower-left dot at 00:02.4–00:02.5, the lower-right dot at 00:02.7–00:02.8, and the centered upper dot at 00:02.9–00:03.0. Arabic-scoped prose, two-way-connector context, source metadata, Noto Naskh geometry, and a five-frame filmstrip agree with three lifts independently of Urdu ش provenance. |
| HL-C09AS | Complete ([#10597](https://github.com/adhithyan15/coding-adventures/pull/10597)) | Audit independent Arabic ص from the same source-backed alphabet page. | The directly linked `FullSizeRender-6.mov` draws the oval and short shoulder continuously at 00:01.1–00:02.4, then lifts once before restarting at the baseline junction and sweeping through the trailing bowl at 00:02.6–00:03.3. Arabic-scoped prose, two-way-connector context, source metadata, Noto Naskh geometry, and a three-frame filmstrip agree with two strokes and one lift independently of the adjacent Seen and Shiin demonstrations. |
| HL-C09AT | Complete ([#10614](https://github.com/adhithyan15/coding-adventures/pull/10614)) | Audit independent Arabic ض from the same source-backed alphabet page. | The page's embedded Daad lesson draws the oval and short shoulder at 00:43.1–00:45.0, lifts once to restart the trailing bowl at 00:45.2–00:45.4, then lifts again and places the upper dot last at 00:46.0–00:46.3. Arabic-scoped prose, two-way-connector context, source metadata, Noto Naskh geometry, and a four-frame filmstrip agree with three strokes and two lifts. The directly linked `FullSizeRender-5.mov` returned HTTP 403 during the audit, so the accessible embedded primary lesson supplies the timestamps and its Saad sequence is cross-checked against the separately audited direct clip. |
| HL-C09AU | Complete ([#10624](https://github.com/adhithyan15/coding-adventures/pull/10624)) | Audit independent Arabic ع from the next measured source-backed alphabet page. | The directly linked `ayn.mov` draws independent ع at 00:03.1–00:04.0 in one pen-down run: it begins at the upper-right tip, shapes the open head through 00:03.5, then continues without lifting down the left side and around the broad lower bowl to a rightward finish. Arabic-scoped prose, two-way-connector context, source metadata, Noto Naskh geometry, and a two-frame filmstrip agree with zero lifts independently of adjacent Ghayn. |
| HL-C09AV | Complete ([#10635](https://github.com/adhithyan15/coding-adventures/pull/10635)) | Audit independent Arabic ك from the next measured source-backed alphabet page. | The directly linked `kaf.mov` draws independent ك in two pen-down runs: at 00:11.8–00:12.9 it descends the main upright and turns left along the baseline without lifting, then after one lift it draws the inner arm from upper right down-left at 00:13.2–00:13.4. Arabic-scoped prose, two-way-connector context, source metadata, Noto Naskh geometry, and a three-frame filmstrip agree with two strokes and one lift independently of Urdu ک's different Unicode glyph and provenance. |
| HL-C09AW | Complete ([#10644](https://github.com/adhithyan15/coding-adventures/pull/10644)) | Audit independent Arabic ل from the same measured source-backed alphabet page. | The directly linked `lam.mov` draws independent ل in one pen-down run at 00:01.9–00:02.4: it descends the tall upright, turns left through the base bowl, and rises at its outer edge without lifting. Arabic-scoped prose, two-way-connector context, source metadata, Noto Naskh geometry, and a two-frame filmstrip agree with zero lifts independently of the Persian and Urdu records for the same Unicode glyph. |
| HL-C09AX | Complete ([#10652](https://github.com/adhithyan15/coding-adventures/pull/10652)) | Audit independent Arabic ي from the same measured source-backed alphabet page. | Frame-by-frame inspection of the directly linked `yaa.mov` shows one continuous body run at 00:33.2–00:34.4, followed by the lower-left dot at 00:34.5–00:34.7 and the lower-right dot at 00:34.8–00:35.0. Arabic-scoped prose, two-way-connector context, source metadata, Noto Naskh geometry, and a four-frame filmstrip agree with three strokes and two lifts independently of Urdu U+06CC **ی**, whose isolated body has no lower dots. |
| HL-C09AY | Complete ([#10661](https://github.com/adhithyan15/coding-adventures/pull/10661)) | Audit independent Arabic ه from the source-backed **ه و ي** alphabet page. | Frame-by-frame inspection of the directly linked `letter-haa.mov` shows one continuous run at 00:04.9–00:06.0: it closes the lower counter, threads through the centre into the upper-right counter, then sweeps left along the baseline without lifting. Arabic-scoped prose, two-way-connector context, source metadata, Noto Naskh geometry, and a three-frame filmstrip agree with zero lifts independently of the existing Persian **ه** provenance for the same Unicode glyph. |
| HL-C09AZ | Complete ([PR #10669](https://github.com/adhithyan15/coding-adventures/pull/10669)) | Audit independent Arabic و from the same source-backed **ه و ي** alphabet page. | Frame-by-frame inspection of the directly linked `waw.mov` shows one continuous run at 00:45.7–00:46.9: it sweeps left from the lower-right junction to close the small head loop, then continues down and left through the tail without lifting. Arabic-scoped prose, one-way-connector and w/long-ū context, source metadata, Noto Naskh geometry, and a two-frame filmstrip agree with zero lifts independently of the existing Persian **و** provenance for the same Unicode glyph. |
| HL-C09BA | Deferred — source unavailable | Recover an auditable independent Arabic م demonstration from the source-backed **م ن** page or its media ledger. | Chapter 769 has no embedded video or chapter attachment. The global WordPress ledger identifies media 1496, titled **م**, as `FullSizeRender-14.mov` (51 seconds, 1440×1920), but its exact upload URL returns HTTP 403; the WXR export exposes no alternate binary or Panopto id, and the Wayback index has no capture. Arabic Mim therefore remains conventional instead of inheriting Persian or Urdu provenance. |
| HL-C09BB | Deferred — source unavailable | Recover independent Arabic ن from the same **م ن** media ledger. | The global ledger identifies media 1494, titled **ن**, as `FullSizeRender-12.mov` (51 seconds, 1440×1920), but the exact upload also returns HTTP 403 and the chapter, WXR export, and archive search expose no accessible alternate. Arabic Nun remains conventional rather than borrowing the independently sourced Persian or Urdu order. |
| HL-C09BC | Deferred — inventory expansion | Preserve the newly recovered independent Arabic ف source without changing HL-C09's fixed denominator. | The **ف ق** page and its public Panopto session recover Faa's dedicated demonstration: draw the head loop, descend into the leftward bowl, then lift once for the dot at 00:01.7–00:03.3. Faa is outside the 22-entry Arabic JSON and HL-C09's fixed 228 prose entries, so this source is recorded for a future inventory-expansion tranche instead of silently moving the denominator. |
| HL-C09BD | Complete ([PR #10684](https://github.com/adhithyan15/coding-adventures/pull/10684)) | Reprioritize to Hebrew א, the first source-backed entry in the next-smallest remaining inventory. | HebrewPod101's second handwritten Alef demonstration draws the main descending diagonal at 01:33.0–01:34.25, lifts once, then draws the opposing diagonal through 01:34.5–01:35.8. The source variation note, two-stroke learner prose, vendored Noto Sans Hebrew geometry, and three-frame filmstrip agree while explicitly recording the compact handwritten-to-block-font adaptation. |
| HL-C09BE | Complete ([PR #10691](https://github.com/adhithyan15/coding-adventures/pull/10691)) | Verify Hebrew ב from the dedicated Bet portion of the same source lesson. | HebrewPod101's second, block-style Bet demonstration draws the top bar left-to-right and continues down the right side at 02:25.8–02:26.7, lifts once, then draws the baseline left-to-right at 02:26.9–02:27.7. The separately placed optional dagesh is excluded from base U+05D1's lift count; learner prose, vendored Noto Sans Hebrew geometry, and the three-frame filmstrip agree. The series' dedicated Gimel/Dalet lesson is queued next. |
| HL-C09BF | Complete ([PR #10698](https://github.com/adhithyan15/coding-adventures/pull/10698)) | Verify Hebrew ג from the series' dedicated Gimel/Dalet lesson. | HebrewPod101 explicitly contrasts rounded cursive and angular printed Gimel. The printed demonstration joins its short left-to-right top bar to the right stem and short lower-right leg at 00:54.2–00:55.4, lifts once, then restarts at the lower junction and draws the longer diagonal leg down-left at 00:55.4–00:55.9. Learner prose, vendored Noto Sans Hebrew geometry, and the four-frame filmstrip agree while preserving the cursive form as a documented variation. The same lesson's one-curve Dalet demonstration is queued next. |
| HL-C09BG | Complete ([PR #10703](https://github.com/adhithyan15/coding-adventures/pull/10703)) | Verify Hebrew ד from the source-adjacent Dalet demonstration. | HebrewPod101's cursive Dalet sweeps a broad arch left-to-right, curls through its small lower loop, and continues into the descending tail at 03:43.8–03:45.0 without lifting; the instructor explicitly summarizes it as "just one curve." The two-frame learner path preserves that zero-lift order while fitting the vendored Noto Sans Hebrew angular top bar and right downstroke. The series' dedicated Hei lesson (`FtCuWlS6V7g`) is queued next. |
| HL-C09BH | Complete ([PR #10712](https://github.com/adhithyan15/coding-adventures/pull/10712)) | Verify Hebrew ה from the series' dedicated Hei lesson. | HebrewPod101's printed Hei demonstration draws the top bar left-to-right and continues down the right side at 00:59.6–01:00.8, lifts once, then draws the detached left leg top-to-bottom at 01:01.2–01:01.9. Learner prose, Noto Sans Hebrew geometry, and the three-frame filmstrip agree while preserving the lesson's explicitly contrasted curved handwritten form. The series' dedicated Vav/Hirik/Shuruk lesson (`kJUMyHR0zN4`) is queued next. |
| HL-C09BI | Complete ([PR #10717](https://github.com/adhithyan15/coding-adventures/pull/10717)) | Verify Hebrew ו from the series' dedicated Vav/Hirik/Shuruk lesson. | HebrewPod101 explicitly says Vav has one stroke from top to bottom at 01:00.0–01:02.5. Its printed demonstration draws the short head left-to-right and turns directly into the descending stem at 01:08.6–01:09.8 without lifting. Learner prose, Noto Sans Hebrew geometry, and the two-frame filmstrip agree with zero lifts while excluding the lesson's later vowel marks from base U+05D5. The series' Zayin/Heit lesson (`XTqG_1dsFSU`) is queued next. |
| HL-C09BJ | Complete ([PR #10724](https://github.com/adhithyan15/coding-adventures/pull/10724)) | Verify Hebrew ז from the series' dedicated Zayin/Heit lesson. | HebrewPod101's rounded handwritten Zayin rises briefly to the right, then curves down the right side and around the base at 00:44.0–00:45.4 without lifting. The source identifies it as handwritten Gimel's mirror image and warns learners not to collapse Zayin into Vav. Learner prose, Noto Sans Hebrew geometry, and the two-frame filmstrip agree with zero lifts while documenting the handwritten-to-block adaptation. The same lesson's Heit demonstration is queued next. |
| HL-C09BK | Complete ([PR #10730](https://github.com/adhithyan15/coding-adventures/pull/10730)) | Verify Hebrew ח from the same Zayin/Heit lesson. | HebrewPod101's printed Heit draws the top bar left-to-right and continues down the right side at 02:44.6–02:45.3, lifts once, then draws the joined left leg top-to-bottom at 02:45.6–02:46.3. Learner prose, Noto Sans Hebrew geometry, and the three-frame filmstrip agree while preserving the lesson's rounded handwritten form. The series' Tet/Yod lesson (`NBUtBPVKchk`) is queued next. |
| HL-C09BL | Complete ([PR #10734](https://github.com/adhithyan15/coding-adventures/pull/10734)) | Verify Hebrew ט from the series' dedicated Tet/Yod lesson. | HebrewPod101's printed Tet draws the left side top-to-bottom and continues right along the base at 00:54.2–00:55.4, lifts once, then restarts at the lower-right, climbs the right side, and turns down-left into the inward hook at 00:55.7–00:56.3. Learner prose, Noto Sans Hebrew geometry, and the four-frame filmstrip agree while preserving the source's unusual bottom-up rounded handwriting. The same lesson's Yod demonstration is queued next. |
| HL-C09BM | Complete ([PR #10738](https://github.com/adhithyan15/coding-adventures/pull/10738)) | Verify Hebrew י from the source-adjacent Yod demonstration. | HebrewPod101's printed Yod draws its tiny head left-to-right and turns down through the short stem at 02:00.7–02:01.2 without lifting. Learner prose, compact Noto Sans Hebrew geometry, and the two-frame filmstrip agree while preserving the source's comma-like handwritten form. The series' dedicated Kaf lesson (`EcQ0gL-NM-k`) is queued next. |
| HL-C09BN | Complete ([PR #10741](https://github.com/adhithyan15/coding-adventures/pull/10741)) | Verify Hebrew כ from the series' dedicated Kaf lesson. | HebrewPod101's printed Kaf draws the top bar left-to-right, turns down the rounded right side, and turns left along the base at 00:51.3–00:53.2 without lifting. Learner prose, Noto Sans Hebrew geometry, and the three-frame filmstrip agree while preserving the lesson's rounded handwritten half-circle. The series' Lamed/Mem lesson (`CBU6aSCcPrE`) is queued next. |
| HL-C09BO | Complete ([PR #10746](https://github.com/adhithyan15/coding-adventures/pull/10746)) | Verify Hebrew ל from the series' Lamed/Mem lesson. | HebrewPod101's printed Lamed starts at the top of the tall left stroke, descends to the middle junction, continues right along the bar, and turns diagonally down-left at 01:22.4–01:23.9 without lifting. Learner prose, Noto Sans Hebrew geometry, and the three-frame filmstrip agree while preserving the lesson's rounded looping handwriting. The same lesson's Mem demonstration is queued next. |
| HL-C09BP | Complete ([PR #10755](https://github.com/adhithyan15/coding-adventures/pull/10755)) | Verify Hebrew מ from the source-adjacent Mem demonstration. | HebrewPod101's printed Mem draws the detached left part up to its corner and down-right through its short inner leg at 03:07.7–03:09.1, lifts once, then climbs through the upper shoulder, turns down the right side, and turns left along the base through 03:10.6. Learner prose, Noto Sans Hebrew geometry, and the five-frame filmstrip agree while preserving the lesson's narrow N-like cursive form. The independently published Nun lesson (`3gYCaDgB-Nk`) is queued next. |
| HL-C09BQ | [Complete (PR #10764)](https://github.com/adhithyan15/coding-adventures/pull/10764) | Correct the queued source, then verify Hebrew נ from Aural Writing's full-alphabet demonstration. | `3gYCaDgB-Nk` displays Nun's regular and final forms but supplies religious exposition rather than an auditable pen sequence. Aural Writing's printed Nun instead joins its left-to-right head, right descent, and leftward base at 02:04.1–02:04.6 without lifting. Learner prose, Noto Sans Hebrew geometry, and the three-frame filmstrip agree while preserving its rounder purple cursive form at 02:05.2–02:05.8. The same source's Samekh demonstration is queued next. |
| HL-C09BR | [Complete (PR #10769)](https://github.com/adhithyan15/coding-adventures/pull/10769) | Verify Hebrew ס from Aural Writing's next full-alphabet demonstration and repair the app README's stale Hebrew inventory. | Printed Samekh closes one clockwise loop at 02:19.5–02:20.8: flat top left-to-right, rounded right descent, leftward base, then the left side back to the start without lifting. The four-frame learner path, prose metadata, and Noto Sans Hebrew geometry agree while preserving the source's rounder purple cursive oval at 02:23.8–02:24.7. The adjacent Ayin demonstration is queued next. |
| HL-C09BS | [Complete (PR #10778)](https://github.com/adhithyan15/coding-adventures/pull/10778) | Verify Hebrew ע from Aural Writing's adjacent full-alphabet demonstration. | Printed Ayin forms one uninterrupted run at 02:27.4–02:28.9: descend the right branch into the base, sweep left, then turn back and climb the left branch. The three-frame learner path, prose metadata, and Noto Sans Hebrew geometry agree while preserving the source's compact purple cursive loop at 02:31.6–02:32.7. The adjacent Pe demonstration is queued next. |
| HL-C09BT | [Complete (PR #10787)](https://github.com/adhithyan15/coding-adventures/pull/10787) | Verify Hebrew פ from Aural Writing's adjacent full-alphabet demonstration and correct the next counted inventory target. | Printed Pe draws its top, right side, and returning base in one run at 02:36.3–02:38.4, lifts once, then adds the short inner curl through 02:38.9. The four-frame learner path, prose metadata, and Noto Sans Hebrew geometry agree while preserving the source's one-run purple cursive spiral at 02:41.5–02:43.0. The source demonstrates final Pe **ף** next, but that is already `פ.forms.final`, not another HL-C09 entry; the later Tsadi demonstration is the next counted inventory target. |
| HL-C09BU | [Complete (PR #10799)](https://github.com/adhithyan15/coding-adventures/pull/10799) | Verify Hebrew צ from Aural Writing's later full-alphabet demonstration and audit the next counted target. | Printed Tsadi descends its long upper-left diagonal and turns left along the base in one run at 02:59.8–03:00.6, lifts once, then curves the short upper-right arm down-left through 03:01.2. The three-frame learner path, prose metadata, and Noto Sans Hebrew geometry agree while preserving the source's one-run purple cursive form at 03:03.2–03:04.0. The source demonstrates final Tsadi **ץ** next, but that is already `צ.forms.final`; the later Qof demonstration is the next counted inventory target. |
| HL-C09BV | [Complete (PR #10808)](https://github.com/adhithyan15/coding-adventures/pull/10808) | Verify Hebrew ק from Aural Writing's later full-alphabet demonstration. | Printed Qof draws the top bar left-to-right and turns down-left through the right body in one run at 03:18.3–03:19.6, lifts once, then descends the separate inner-left stem below the writing line through 03:20.0. The three-frame learner path, prose metadata, and Noto Sans Hebrew geometry agree while preserving the source's one-run purple cursive hook at 03:22.0–03:23.3. The adjacent Resh demonstration is the next counted inventory target. |
| HL-C09BW | [Complete (PR #10817)](https://github.com/adhithyan15/coding-adventures/pull/10817) | Verify Hebrew ר from Aural Writing's adjacent full-alphabet demonstration. | Printed Resh draws its top bar left-to-right, rounds the top-right corner, and continues down the right side in one uninterrupted run at 03:26.2–03:27.1. The two-frame learner path, prose metadata, and Noto Sans Hebrew geometry agree while preserving the source's rounder one-run purple cursive hook at 03:29.3–03:30.0. The adjacent Shin demonstration is the next counted inventory target. |
| HL-C09BX | [Complete (PR #10827)](https://github.com/adhithyan15/coding-adventures/pull/10827) | Verify Hebrew ש from Aural Writing's adjacent full-alphabet demonstration. | Printed Shin draws the outer right branch, rounded base, and left branch in one run at 03:34.0–03:35.5, lifts once, then descends the middle branch through 03:36.3. The three-frame learner path, prose metadata, and Noto Sans Hebrew geometry agree while preserving the source's compact one-run purple cursive loop at 03:39.2–03:40.2. The adjacent Tav demonstration is the final outstanding Hebrew target. |
| HL-C09BY | [Complete (PR #10836)](https://github.com/adhithyan15/coding-adventures/pull/10836) | Verify Hebrew ת from Aural Writing's adjacent full-alphabet demonstration and close the Hebrew inventory. | Printed Tav draws the top bar left-to-right and continues down the right side in one run at 03:45.2–03:46.2, lifts once, then descends the separate left leg and curves left into its foot through 03:47.3. The four-frame learner path, prose metadata, and Noto Sans Hebrew geometry agree while preserving the source's one-run purple cursive form at 03:49.0–03:50.9. Hebrew is now complete; because Arabic ث, م, and ن remain source-blocked, Chinese is the next smallest actionable inventory. |
| HL-C09BZ | [Complete (PR #10845)](https://github.com/adhithyan15/coding-adventures/pull/10845) | Open Chinese with 人 and establish a reusable PRC-order source convention for the next actionable inventory. | Hanzi Writer Data's immutable 人 record supplies two properly ordered stroke paths and matching medians: the first falls from the upper centre down-left, then after one lift the second restarts at the junction and falls down-right. The two-frame learner path adapts those Arphic-derived medians to Noto Sans SC while preserving order and direction. The pinned per-character record plus Make Me a Hanzi's documented PRC ordering can now support the remaining 23 Chinese entries without treating component lists as complete pen paths. |
| HL-C09CA | [Complete (PR #10853)](https://github.com/adhithyan15/coding-adventures/pull/10853) | Verify Chinese 亻 as the compressed person radical using the same pinned PRC-order source convention. | Hanzi Writer Data's immutable 亻 record supplies two distinct medians: a long left-falling piě from the upper right, then after one lift a vertical shù from the central junction to the baseline. The two-frame learner path fits those runs independently to the narrow Noto Sans SC radical rather than mechanically squeezing 人, preserving source order and direction while reducing Chinese to 22 outstanding entries. |
| HL-C09CB | [Complete (PR #10862)](https://github.com/adhithyan15/coding-adventures/pull/10862) | Verify Chinese 口 and the first joined corner in the Chinese inventory. | Hanzi Writer Data's immutable 口 record supplies three ordered runs: descend the left side, lift and draw the top bar into the right side without breaking the héngzhé corner, then lift and close the bottom left-to-right. The four-frame learner path fits those medians to Noto Sans SC, proves the corner join, preserves the close-last rule, and reduces Chinese to 21 outstanding entries. |
| HL-C09CC | [Complete (PR #10868)](https://github.com/adhithyan15/coding-adventures/pull/10868) | Verify Chinese 女 and its bent first run using the pinned PRC-order source convention. | Hanzi Writer Data's immutable 女 record supplies three ordered runs: a piědiǎn stroke descends left before turning and sweeping down-right without lifting, the second stroke restarts at the upper right and falls down-left, and the third crosses the middle left-to-right. The four-frame learner path fits those medians to Noto Sans SC, proves the first turn remains joined, preserves the two source lifts, and reduces Chinese to 20 outstanding entries. 子 is the next counted Chinese target. |
| HL-C09CD | [Complete (PR #10875)](https://github.com/adhithyan15/coding-adventures/pull/10875) | Verify Chinese 子 and its two joined turns using the pinned PRC-order source convention. | Hanzi Writer Data's immutable 子 record supplies three ordered runs: the top horizontal turns down-left without lifting, the separately started central vertical hooks left at its base, and the final horizontal crosses the middle left-to-right. The five-frame learner path fits those medians to Noto Sans SC, proves both turns remain joined, preserves the two source lifts, and reduces Chinese to 19 outstanding entries. 日 is the next counted Chinese target. |
| HL-C09CE | [Complete (PR #10882)](https://github.com/adhithyan15/coding-adventures/pull/10882) | Verify Chinese 日 and its inside-before-close order using the pinned PRC-order source convention. | Hanzi Writer Data's immutable 日 record supplies four ordered runs: the left side, the top bar joined to the right side, the middle bar, and the separately closing bottom. The five-frame learner path fits those medians to Noto Sans SC, proves the top-right turn remains joined, preserves the three source lifts, and reduces Chinese to 18 outstanding entries. 讠 is the next counted Chinese target. |
| HL-C09CF | [Complete (PR #10890)](https://github.com/adhithyan15/coding-adventures/pull/10890) | Verify Chinese 讠 and its double-turning second stroke using the pinned PRC-order source convention. | Hanzi Writer Data's immutable 讠 record supplies two ordered runs: a down-right dot, then a separately started short horizontal that turns down and finishes up-right without either internal lift. The four-frame learner path fits those medians to Noto Sans SC, proves both later turns remain joined, preserves the one source lift, and reduces Chinese to 17 outstanding entries. 氵 is the next counted Chinese target. |
| HL-C09CG | [Complete (PR #10898)](https://github.com/adhithyan15/coding-adventures/pull/10898) | Verify Chinese 氵 and its three separately ordered water-radical strokes using the pinned PRC-order source convention. | Hanzi Writer Data's immutable 氵 record supplies three ordered runs: an upper down-right dot, a separately started middle down-right dot, then a bottom stroke that begins with a slight up-left turn before sweeping up-right. The four-frame learner path fits those medians to Noto Sans SC, proves the bottom turn remains joined to its rise, preserves the two source lifts, and reduces Chinese to 16 outstanding entries. 宀 is the next counted Chinese target. |
| HL-C09CH | [Complete (PR #10905)](https://github.com/adhithyan15/coding-adventures/pull/10905) | Verify Chinese 宀 and its joined horizontal hook using the pinned PRC-order source convention. | Hanzi Writer Data's immutable 宀 record supplies three ordered runs: a down-right top dot, a separately started left-side down-left stroke, then a horizontal roof that hooks down-left without lifting. The four-frame learner path fits those medians to Noto Sans SC, proves the roof and hook remain joined, preserves the two source lifts, and reduces Chinese to 15 outstanding entries. 你 is the next counted Chinese target. |
| HL-C09CI | [Complete (PR #10910)](https://github.com/adhithyan15/coding-adventures/pull/10910) | Verify Chinese 你 and its full component order using the pinned PRC-order source convention. | Hanzi Writer Data's immutable 你 record supplies seven ordered runs: 亻 first, then a left-falling stroke, a horizontal with a joined down-left hook, a vertical with a joined left hook, and two separately placed lower dots. The nine-frame learner path fits those medians to Noto Sans SC, proves both hooks remain joined, preserves the six source lifts, and reduces Chinese to 14 outstanding entries. 好 is the next counted Chinese target. |
| HL-C09CJ | [Complete (PR #10918)](https://github.com/adhithyan15/coding-adventures/pull/10918) | Verify Chinese 好 and its full 女-before-子 component order using the pinned PRC-order source convention. | Hanzi Writer Data's immutable 好 record supplies six ordered runs: all three strokes of 女 first, then 子's joined top turn, joined vertical hook, and middle horizontal. The nine-frame learner path fits those medians to Noto Sans SC, proves all three internal turns remain joined, preserves the five source lifts, and reduces Chinese to 13 outstanding entries. 我 is the next counted Chinese target. |
| HL-C09CK | [Complete (PR #10924)](https://github.com/adhithyan15/coding-adventures/pull/10924) | Verify Chinese 我 and its seven-stroke order using the pinned PRC-order source convention. | Hanzi Writer Data's immutable 我 record supplies seven ordered runs, including the hooked vertical and upward-hooking long curved slash. The nine-frame learner path fits those medians to Noto Sans SC, preserves both joined hooks and the six source lifts, and reduces Chinese to 12 outstanding entries. 是 is next. |
| HL-C09CL | [Complete (PR #10935)](https://github.com/adhithyan15/coding-adventures/pull/10935) | Verify Chinese 是 and its 日-before-lower-body order using the pinned PRC-order source convention. | Hanzi Writer Data's immutable 是 record supplies nine ordered runs: four strokes close 日 first, then five lower strokes finish the character. The ten-frame learner path fits those medians to Noto Sans SC, preserves the joined top-right corner, records eight source lifts, and reduces Chinese to 11 outstanding entries. Audit 叫 before choosing the next counted target. |
| HL-C09CM | [Complete (PR #10940)](https://github.com/adhithyan15/coding-adventures/pull/10940) | Audit the planned 叫 dependency, then verify Chinese 不 as the next counted inventory target using the pinned PRC-order source convention. | 叫 belongs to the future Chapter 4 plan but is absent from both `chinese.json` and the vendored font subset, so it cannot reduce the 228-entry HL-C19 debt without a separate inventory-expansion and font-resubsetting change. Hanzi Writer Data's immutable 不 record supplies four separate runs: top horizontal, long falling stroke, central vertical, and right-falling dot. The four-frame learner path fits those medians to Noto Sans SC, preserves three source lifts, reduces Chinese to 10 outstanding entries, and queues 名 next. |
| HL-C09CN | [Complete (PR #10947)](https://github.com/adhithyan15/coding-adventures/pull/10947) | Verify Chinese 名 and its 夕-before-口 component order using the pinned PRC-order source convention. | Hanzi Writer Data's immutable 名 record supplies six ordered runs: 夕's upper falling stroke, joined horizontal-to-down-left sweep, and inner dot, then 口's left side, joined top-right corner, and closing bottom. The eight-frame learner path fits those medians to Noto Sans SC, preserves both joined turns and five source lifts, reduces Chinese to 9 outstanding entries, and queues 字 next. |
| HL-C09CO | [Complete (PR #10952)](https://github.com/adhithyan15/coding-adventures/pull/10952) | Verify Chinese 字 and its 宀-before-子 component order using the pinned PRC-order source convention. | Hanzi Writer Data's immutable 字 record supplies six ordered runs: 宀's dot, left-side stroke, and joined horizontal hook, then 子's joined top turn, joined vertical hook, and middle horizontal. The nine-frame learner path fits those medians to Noto Sans SC, preserves all three joined turns and five source lifts, reduces Chinese to 8 outstanding entries, and queues 谢 next. |
| HL-C09CP | [Complete (PR #10959)](https://github.com/adhithyan15/coding-adventures/pull/10959) | Verify Chinese 谢 and its 讠-before-身-before-寸 component order using the pinned PRC-order source convention. | Hanzi Writer Data's immutable 谢 record supplies twelve ordered runs: 讠's dot and double-turning second stroke, seven strokes for 身 including its joined top-right enclosure and base hook, then 寸's horizontal, joined vertical hook, and final dot. The seventeen-frame learner path fits those medians to Noto Sans SC, preserves all five joined turns and eleven source lifts, reduces Chinese to 7 outstanding entries, and queues 请 next. |
| HL-C09CQ | [Complete (PR #10967)](https://github.com/adhithyan15/coding-adventures/pull/10967) | Verify Chinese 请 and its 讠-before-青 component order using the pinned PRC-order source convention. | Hanzi Writer Data's immutable 请 record supplies ten ordered runs: 讠's dot and double-turning second stroke, then 青's two upper horizontals, central vertical, wide middle horizontal, lower left side, joined lower enclosure, and two inner horizontals. The fourteen-frame learner path fits those medians to Noto Sans SC, preserves all four joined turns and nine source lifts, reduces Chinese to 6 outstanding entries, and queues 再 next. |
| HL-C09CR | [Complete (PR #11100)](https://github.com/adhithyan15/coding-adventures/pull/11100) | Verify Chinese 再 and its frame-before-close order using the pinned PRC-order source convention. | Hanzi Writer Data's immutable 再 record supplies six ordered runs: top horizontal, left side, joined top-right-base frame, central vertical, inner horizontal, and the long closing bottom horizontal. The eight-frame learner path fits those medians to Noto Sans SC, preserves both turns and five source lifts, reduces Chinese to 5 outstanding entries, and queues 见 next. |
| HL-C09CS | [Complete (PR #11109)](https://github.com/adhithyan15/coding-adventures/pull/11109) | Verify Chinese 见 and its frame-before-legs order using the pinned PRC-order source convention. | Hanzi Writer Data's immutable 见 record supplies four ordered runs: left side, joined top-and-right frame, inner left-falling leg, and joined vertical-bend hook. The seven-frame learner path fits those medians to Noto Sans SC, preserves all three joined turns and three source lifts, reduces Chinese to 4 outstanding entries, and queues 什 next. |
| HL-C09CT | [Complete (PR #11120)](https://github.com/adhithyan15/coding-adventures/pull/11120) | Verify Chinese 什 and its complete 亻-before-十 component order using the pinned PRC-order source convention. | Hanzi Writer Data's immutable 什 record supplies four separate ordered runs: 亻's left-falling stroke and vertical, then 十's horizontal and vertical. The four-frame learner path fits those medians to Noto Sans SC, preserves all three source lifts, reduces Chinese to 3 outstanding entries, and queues 么 next. |
| HL-C09CU | [Complete (PR #11126)](https://github.com/adhithyan15/coding-adventures/pull/11126) | Verify Chinese 么 and its joined falling-to-rightward-sweep stroke using the pinned PRC-order source convention. | Hanzi Writer Data's immutable 么 record supplies three ordered runs: upper left-falling stroke, joined second fall and rightward base sweep, then a final down-right dot. The four-frame learner path fits those medians to Noto Sans SC, preserves the joined turn and two source lifts, reduces Chinese to 2 outstanding entries, and queues 早 next. |
| HL-C09CV | [Complete (PR #11136)](https://github.com/adhithyan15/coding-adventures/pull/11136) | Verify Chinese 早 and its complete 日-before-十 component order using the pinned PRC-order source convention. | Hanzi Writer Data's immutable 早 record supplies six ordered runs: 日's left side, joined top-right enclosure, middle horizontal, and closing bottom, followed by 十's horizontal and vertical. The seven-frame learner path fits those medians to Noto Sans SC, preserves the joined turn and five source lifts, reduces Chinese to 1 outstanding entry, and queues 上 next. |
| HL-C09CW | [Complete (PR #11141)](https://github.com/adhithyan15/coding-adventures/pull/11141) | Verify Chinese 上 and close the Chinese starter inventory using the pinned PRC-order source convention. | Hanzi Writer Data's immutable 上 record supplies three ordered runs: the central vertical, short middle horizontal, and long base horizontal. The three-frame learner path fits those medians to Noto Sans SC, preserves two source lifts and the short-before-long contrast, completes Chinese, and queues a source audit for Devanagari अ next. |
| HL-C09CX | [Complete (PR #11147)](https://github.com/adhithyan15/coding-adventures/pull/11147) | Audit Devanagari's source variability, then verify अ as the first actionable entry after Chinese completes. | Saurmandal's four-frame modern printed sequence supplies a joined upper-and-lower left body, separately swept middle shoulder, top-to-bottom right stem, and left-to-right shirorekhā. The five-frame learner path fits those four runs to Noto Sans Devanagari, preserves three lifts, and records Thomas Egenes's six-stroke traditional Sanskrit form rather than presenting one order as universal. Devanagari आ is next. |
| HL-C09CY | [Complete (PR #11151)](https://github.com/adhithyan15/coding-adventures/pull/11151) | Verify Devanagari आ as the next independent vowel while preserving the source variation established for अ. | Saurmandal's five-frame modern printed sequence preserves अ's joined left body, then separately adds the middle shoulder, inner stem, trailing stem, and left-to-right shirorekhā. The six-frame learner path fits those five runs to Noto Sans Devanagari, preserves four lifts, and records that Egenes's traditional six-stroke base अ makes the joined body a modern-form choice rather than a universal rule. Devanagari इ is next. |
| HL-C09CZ | [Complete (PR #11158)](https://github.com/adhithyan15/coding-adventures/pull/11158) | Verify Devanagari इ and its continuous double-bowl body before the final headline. | Saurmandal's two-panel modern printed diagram starts at the upright, keeps both bowls and the down-right tail in one continuous run, then lifts once for the left-to-right shirorekhā. The five-frame learner path fits those two runs to Noto Sans Devanagari, preserves the single lift, and labels the diagram as one sourced teaching form rather than a universal handwriting standard. Devanagari ई is next. |
| HL-C09DA | [Complete (PR #11165)](https://github.com/adhithyan15/coding-adventures/pull/11165) | Verify Devanagari ई by preserving इ's continuous body before adding its separate upper curl and final headline. | Saurmandal's three-panel modern printed diagram reuses the upright, both bowls, and down-right tail as one run, then separately sweeps the upper curl upward and around before finishing with the left-to-right shirorekhā. The six-frame learner path fits those three runs to Noto Sans Devanagari, preserves two lifts, and identifies the diagram as one sourced teaching form rather than a universal handwriting standard. Devanagari उ is next. |
| HL-C09DB | [Complete (PR #11168)](https://github.com/adhithyan15/coding-adventures/pull/11168) | Verify Devanagari उ by preserving its upper bowl and lower loop as one continuous body before the final headline. | Saurmandal's two-panel modern printed diagram starts at the headline junction, curves down and left around the upper bowl, then carries the same run back through the waist and around the lower loop before the separate left-to-right shirorekhā. The three-frame learner path fits those two runs to Noto Sans Devanagari, preserves one lift, and identifies the diagram as one sourced teaching form rather than a universal handwriting standard. Devanagari ऊ is next. |
| HL-C09DC | [Complete (PR #11175)](https://github.com/adhithyan15/coding-adventures/pull/11175) | Verify Devanagari ऊ by preserving उ's continuous body before adding its separate right-hand loop and final headline. | Saurmandal's three-panel modern printed diagram reuses the upper bowl and lower loop as one run, then separately sweeps the right-hand loop upward, around, and down-left before finishing with the left-to-right shirorekhā. The four-frame learner path fits those three runs to Noto Sans Devanagari, preserves two lifts, and identifies the diagram as one sourced teaching form rather than a universal handwriting standard. Devanagari ए is next. |
| HL-C09DD | [Complete (PR #11180)](https://github.com/adhithyan15/coding-adventures/pull/11180) | Verify Devanagari ए by preserving its long left stem and descending tail as one run before the short hooked stem and final headline. | Saurmandal's three-panel modern printed diagram descends the long left stem, curves through the lower shoulder, and sweeps down the tail without lifting; a second run descends the shorter right stem into its inward hook before the left-to-right shirorekhā. The four-frame learner path fits those three runs to Noto Sans Devanagari, preserves two lifts, and identifies the diagram as one sourced teaching form rather than a universal handwriting standard. Devanagari ऐ is next. |
| HL-C09DE | [Complete (PR #11185)](https://github.com/adhithyan15/coding-adventures/pull/11185) | Verify Devanagari ऐ by preserving ए's two base strokes before adding its separate upper arc and final headline. | Saurmandal's four-panel modern printed diagram reuses the joined long stem and descending tail, then the shorter inward-hooked stem, before separately sweeping the upper arc upward and left and finishing with the left-to-right shirorekhā. The five-frame learner path fits those four runs to Noto Sans Devanagari, preserves three lifts, and identifies the diagram as one sourced teaching form rather than a universal handwriting standard. Devanagari ओ is next. |
| HL-C09DF | [Complete (PR #11193)](https://github.com/adhithyan15/coding-adventures/pull/11193) | Verify Devanagari ओ by preserving आ's four base strokes before adding its separate upper arc and final headline. | Saurmandal's six-panel modern printed diagram reuses the joined upper-and-lower अ body, separate middle shoulder, inner stem, and trailing stem of आ, before separately sweeping the upper arc upward and left and finishing with the left-to-right shirorekhā. The seven-frame learner path fits those six runs to Noto Sans Devanagari, preserves five lifts, and identifies the diagram as one sourced teaching form rather than a universal handwriting standard. Devanagari औ is next. |
| HL-C09DG | [Complete (PR #11199)](https://github.com/adhithyan15/coding-adventures/pull/11199) | Verify Devanagari औ by preserving आ's four base strokes before adding its two separate upper arcs and final headline. | Saurmandal's seven-panel modern printed diagram reuses the joined upper-and-lower अ body, separate middle shoulder, inner stem, and trailing stem of आ, before separately sweeping the lower and taller upper arcs upward and left and finishing with the left-to-right shirorekhā. The eight-frame learner path fits those seven runs to Noto Sans Devanagari, preserves six lifts, and identifies the diagram as one sourced teaching form rather than a universal handwriting standard. Devanagari क is next. |
| HL-C09DH | [Complete (PR #11208)](https://github.com/adhithyan15/coding-adventures/pull/11208) | Audit beyond the vowel-only SVG source boundary, then verify Devanagari क from the older animated stroke-order collection. | Wikimedia Commons' older GIF category covers every remaining Devanagari starter consonant even though its newer SVG category covers none of them. Opiaterein's 27-frame क animation writes the counterclockwise left bowl, top-to-bottom central stem, clockwise right-hand arch, and left-to-right shirorekhā as four separate runs; the Central Hindi Directorate's 2019 deskbook independently shows the same four-part buildup. The four-frame learner path fits those runs to Noto Sans Devanagari and preserves three lifts. Devanagari ग is next. |
| HL-C09DI | [Complete (PR #11217)](https://github.com/adhithyan15/coding-adventures/pull/11217) | Verify Devanagari ग by preserving its loop-to-ascending-stem join before the separate right stem and headline. | Opiaterein's 18-frame animation carries the counterclockwise loop directly up its joined stem, lifts once for the top-to-bottom right stem, then lifts again for the left-to-right shirorekhā. The Central Hindi Directorate's 2019 deskbook independently shows the same three-part buildup. The three-frame learner path fits those runs to Noto Sans Devanagari and preserves two lifts. Devanagari च is next. |
| HL-C09DJ | [Complete (PR #11226)](https://github.com/adhithyan15/coding-adventures/pull/11226) | Verify Devanagari च while distinguishing source-backed joins from component-order corroboration. | Opiaterein's 22-frame animation draws the short upper bar left-to-right and turns directly through the shoulder into the rounded open body, then separately descends the right stem and draws the shirorekhā. The Central Hindi Directorate's 2019 deskbook confirms the same component order but stages the upper bar and rounded body separately, so the two-lift claim remains explicitly animation-backed. The three-frame learner path fits those runs to Noto Sans Devanagari. Devanagari त is next. |
| HL-C09DK | [Complete (PR #11233)](https://github.com/adhithyan15/coding-adventures/pull/11233) | Verify Devanagari त by preserving its right-to-left shoulder and continuous downward body curve before the separate right stem and headline. | Opiaterein's 17-frame animation sweeps from the body's upper-right junction left across the shoulder and down around the open body, then separately descends the right stem and draws the shirorekhā. The Central Hindi Directorate's 2019 deskbook independently shows the same three-part buildup. The three-frame learner path fits those runs to Noto Sans Devanagari and preserves two lifts. Devanagari द is next. |
| HL-C09DL | [Complete (PR #11240)](https://github.com/adhithyan15/coding-adventures/pull/11240) | Verify Devanagari द while distinguishing its animation-backed body-to-curl join from component-order corroboration. | Opiaterein's 18-frame animation descends the short stem, then joins the outer body directly through the inward curl and down-right tail before the separate shirorekhā. The Central Hindi Directorate's 2019 deskbook confirms the same component order but stages the outer body and curl-tail separately, so the two-lift claim remains explicitly animation-backed. The three-frame learner path fits those runs to Noto Sans Devanagari. Devanagari ध is next. |
| HL-C09DM | [Complete (PR #11249)](https://github.com/adhithyan15/coding-adventures/pull/11249) | Verify Devanagari ध as four source-separated runs across its upper spiral, lower bowl, right stem, and headline. | Opiaterein's 27-frame animation uses long holds to delimit the upper spiral-and-shoulder, lower bowl, top-to-bottom right stem, and left-to-right shirorekhā. The Central Hindi Directorate's 2019 deskbook independently shows the same four-part buildup. The four-frame learner path fits those runs to Noto Sans Devanagari and preserves three lifts. Devanagari न is next. |
| HL-C09DN | [Complete (PR #11254)](https://github.com/adhithyan15/coding-adventures/pull/11254) | Verify Devanagari न as three source-separated runs across its clockwise left loop and shoulder, right stem, and headline. | Opiaterein's 20-frame animation uses long holds to delimit the clockwise loop-and-shoulder, top-to-bottom right stem, and left-to-right shirorekhā. The Central Hindi Directorate's 2019 deskbook independently shows the same three-part buildup and directions. The three-frame learner path fits those runs to Noto Sans Devanagari and preserves two lifts. Devanagari प is next. |
| HL-C09DO | [Complete (PR #11258)](https://github.com/adhithyan15/coding-adventures/pull/11258) | Verify Devanagari प as three source-separated runs across its descending left stem and lower bowl, right stem, and headline. | Opiaterein's 19-frame animation uses long holds to delimit the left stem curving right around the bowl, top-to-bottom right stem, and left-to-right shirorekhā. The Central Hindi Directorate's 2019 deskbook independently shows the same three-part buildup and directions. The three-frame learner path fits those runs to Noto Sans Devanagari and preserves two lifts. Devanagari ब is next. |
| HL-C09DP | [Complete (PR #11267)](https://github.com/adhithyan15/coding-adventures/pull/11267) | Verify Devanagari ब as four source-separated runs across its counterclockwise oval body, right stem, inner diagonal, and headline. | JackPotte's 13-frame animation spatially restarts between the counterclockwise oval, top-to-bottom right stem, down-right inner diagonal, and left-to-right shirorekhā. The Central Hindi Directorate's 2019 deskbook independently shows the same four-part buildup and directions. The four-frame learner path fits those runs to Noto Sans Devanagari and preserves three lifts. Devanagari भ is next. |
| HL-C09DQ | [Complete (PR #11276)](https://github.com/adhithyan15/coding-adventures/pull/11276) | Verify Devanagari भ while distinguishing its animation-backed continuous double-loop body from component-order corroboration. | JackPotte's 15-frame animation spatially restarts only after the clockwise upper loop, descending trunk, clockwise lower bowl, and rightward crossbar have been drawn continuously, then separately descends the right stem and draws the shirorekhā. The Central Hindi Directorate's 2019 deskbook confirms the same component order but stages the upper and lower body parts separately, so the two-lift claim remains explicitly animation-backed. The three-frame learner path fits those runs to Noto Sans Devanagari. Devanagari म is next. |
| HL-C09DR | [Complete (PR #11281)](https://github.com/adhithyan15/coding-adventures/pull/11281) | Verify Devanagari म while distinguishing its animation-backed left-stem-to-loop join from component-order corroboration. | JackPotte's 12-frame animation spatially restarts only after the descending left stem, clockwise lower loop, and rightward crossbar have been drawn continuously, then separately descends the right stem and draws the shirorekhā. The Central Hindi Directorate's 2019 deskbook confirms the same component order but stages the left stem and loop-crossbar separately, so the two-lift claim remains explicitly animation-backed. The three-frame learner path fits those runs to Noto Sans Devanagari. Devanagari य is next. |
| HL-C09DS | [Complete (PR #11288)](https://github.com/adhithyan15/coding-adventures/pull/11288) | Verify Devanagari य while preserving the sourced four-stroke form and its joined-body variation. | Opiaterein's 22-frame animation uses explicit holds to delimit the clockwise inner curl, restarted lower bowl, top-to-bottom right stem, and left-to-right shirorekhā as four runs. The Central Hindi Directorate's 2019 deskbook independently shows the same four-part buildup and directions. JackPotte's 11-frame animation instead joins the inner curl and lower bowl before the same stem and headline, so that documented two-lift variation remains explicit. The four-frame learner path follows the mutually corroborated order and fits those runs to Noto Sans Devanagari. Devanagari र is next. |
| HL-C09DT | [Complete (PR #11295)](https://github.com/adhithyan15/coding-adventures/pull/11295) | Verify Devanagari र while preserving the sourced three-stroke form and its joined-tail variation. | Opiaterein's 17-frame animation uses explicit holds to delimit the top-to-bottom stem curling clockwise around the lower loop, restarted down-right diagonal tail, and left-to-right shirorekhā as three runs. The Central Hindi Directorate's 2019 deskbook independently shows the same three-part buildup and directions. JackPotte's seven-frame animation instead joins the descending stem, loop, and tail before the separate headline, so that documented one-lift variation remains explicit. The three-frame learner path follows the mutually corroborated order and fits those runs to Noto Sans Devanagari. Devanagari ल is next. |
| HL-C09DU | Complete ([PR #11301](https://github.com/adhithyan15/coding-adventures/pull/11301)) | Verify Devanagari ल while preserving the sourced loop-first form and its stem-first variation. | Opiaterein's 23-frame animation uses explicit holds to delimit the clockwise open left loop, restarted up-right diagonal arm, top-to-bottom right stem, and left-to-right shirorekhā as four runs. The Central Hindi Directorate's 2019 deskbook independently shows the same four-part buildup and directions. JackPotte's 12-frame animation instead orders the right stem, diagonal arm, left loop, and headline, so that documented stem-first alternative remains explicit. The four-frame learner path follows the mutually corroborated loop-first order and fits those runs to Noto Sans Devanagari. Devanagari व is next. |
| HL-C09DV | Complete ([PR #11306](https://github.com/adhithyan15/coding-adventures/pull/11306)) | Verify Devanagari व from the surviving animated source while keeping corroboration and lift evidence distinct. | JackPotte's 11-frame animation draws the counterclockwise left loop, top-to-bottom right stem, and left-to-right shirorekhā as three spatially restarted runs. The Central Hindi Directorate's 2019 deskbook independently confirms the same three-part buildup, while the animation supplies the within-run directions and two-lift evidence. The three-frame learner path fits those runs to Noto Sans Devanagari. Devanagari श is next. |
| HL-C09DW | Complete ([PR #11313](https://github.com/adhithyan15/coding-adventures/pull/11313)) | Verify Devanagari श by preserving its joined double-loop body and diagonal tail before the separate right stem and headline. | Opiaterein's 25-frame animation carries the upper loop, descending outer curve, lower loop, and down-right diagonal tail in one continuous run, then uses 250 ms holds to delimit the restarted right stem and headline. JackPotte's separate 26-frame animation and the Central Hindi Directorate's 2019 deskbook independently confirm the same three-part buildup. The three-frame learner path fits those runs to Noto Sans Devanagari and preserves two lifts. Devanagari स is next. |
| HL-C09DX | Complete ([PR #11318](https://github.com/adhithyan15/coding-adventures/pull/11318)) | Verify Devanagari स while distinguishing its animation-backed hook-to-tail join from component-order corroboration. | JackPotte's 13-frame animation spatially restarts only after the descending left stem, central hook, and down-right diagonal tail have been drawn continuously, then separately draws the middle crossbar, descends the right stem, and finishes the shirorekhā. The Central Hindi Directorate's 2019 deskbook confirms the same component order but stages the left curve and diagonal tail separately, so the three-lift claim remains explicitly animation-backed. The four-frame learner path fits those runs to Noto Sans Devanagari. Devanagari ह is next. |
| HL-C09DY | Complete ([PR #11325](https://github.com/adhithyan15/coding-adventures/pull/11325)) | Verify Devanagari ह while distinguishing its animation-backed joined first body from component-order corroboration. | Opiaterein's 22-frame animation descends the right stem, sweeps left through the shoulder, and curves clockwise around the hooked body continuously, then uses holds to delimit the restarted down-left outer curve and down-right tail before the final shirorekhā. The Central Hindi Directorate's 2019 deskbook confirms the same component order but stages the joined first body across more buildup steps, so the two-lift claim remains explicitly animation-backed. The three-frame learner path fits those runs to Noto Sans Devanagari and completes the source-verified Devanagari starter inventory. Cyrillic and Gujarati are tied as the next smallest actionable inventories. |
| HL-C09DZ | Complete ([PR #11337](https://github.com/adhithyan15/coding-adventures/pull/11337)) | Break the tied Cyrillic/Gujarati queue by verifying lowercase Cyrillic а from a native teacher's all-letter school-hand demonstration. | RussianIrina's 00:50–00:55 lowercase а demonstration keeps its rounded body and right-hand finishing stem in one pen-down run. The two-frame learner path preserves zero lifts while fitting the handwritten single-storey motion through Noto Sans Cyrillic's double-storey printed shoulder. The same source covers all 33 Russian letters, so Cyrillic б is next. |
| HL-C09EA | Complete ([PR #11356](https://github.com/adhithyan15/coding-adventures/pull/11356)) | Verify lowercase Cyrillic б from the same native teacher's all-letter school-hand demonstration. | RussianIrina's 01:13–01:18 lowercase б demonstration circles the lower body counterclockwise and continues into the rising shoulder and rightward top flag without lifting. The two-frame learner path preserves that zero-lift body-to-flag order while routing the handwritten diagonal transition through Noto Sans Cyrillic's printed upper-left shoulder. Cyrillic в is next. |
| HL-C09EB | Complete ([PR #11370](https://github.com/adhithyan15/coding-adventures/pull/11370)) | Verify lowercase Cyrillic в from the source-adjacent school-hand demonstration. | RussianIrina's 01:33–01:38 lowercase в demonstration starts at the baseline, climbs through a tall upper loop, descends to the baseline, and continues counterclockwise around the lower bowl without lifting. The two-frame learner path preserves that zero-lift order while routing the cursive ascender through Noto Sans Cyrillic's compact printed upper bowl and straight left stem. Cyrillic г is next. |
| HL-C09EC | Complete ([PR #11385](https://github.com/adhithyan15/coding-adventures/pull/11385)) | Verify lowercase Cyrillic г from the source-adjacent school-hand demonstration. | RussianIrina's 01:54–01:57 lowercase г demonstration rises from the baseline into a main shoulder, descends and turns at the baseline, then continues through a smaller exit arch without lifting. The two-frame learner path preserves that zero-lift evidence while fitting Noto Sans Cyrillic's block glyph through its upright and retraced top bar; the variation note records that the printed form omits the cursive exit arch. Cyrillic д is next. |
| HL-C09ED | Complete ([PR #11395](https://github.com/adhithyan15/coding-adventures/pull/11395)) | Verify lowercase Cyrillic д from the source-adjacent school-hand demonstration. | RussianIrina's 02:14–02:19 lowercase д demonstration circles the rounded body counterclockwise, closes it, descends below the baseline, loops left, and rises into the rightward exit without lifting. The two-frame learner path preserves that zero-lift body-before-descender order while fitting Noto Sans Cyrillic's block glyph through its trapezoidal body, joined base shelf, and two retraced feet; the variation note records that the printed form replaces the cursive descender loop with those feet. Cyrillic е is next. |
| HL-C09EE | Complete ([PR #11411](https://github.com/adhithyan15/coding-adventures/pull/11411)) | Verify lowercase Cyrillic е from the source-adjacent school-hand demonstration. | RussianIrina's 02:26–02:30 lowercase е demonstration begins at the upper right, curves left around the upper loop, crosses through the middle, and continues counterclockwise around the rounded lower bowl without lifting. The two-frame learner path preserves that zero-lift upper-loop-to-middle-to-lower-bowl order while fitting Noto Sans Cyrillic's compact printed e through its upper bowl and long middle bar. Cyrillic ё is next. |
| HL-C09EF | Complete ([PR #11418](https://github.com/adhithyan15/coding-adventures/pull/11418)) | Verify lowercase Cyrillic ё from the source-adjacent school-hand demonstration. | RussianIrina's 02:51–02:56 lowercase ё demonstration completes the same looped body as е, then lifts for the left dot and lifts again for the right dot. The four-frame learner path preserves that body-before-left-dot-before-right-dot order and two-lift evidence while fitting Noto Sans Cyrillic's compact printed e body and two circular dots. Cyrillic ж is next. |
| HL-C09EG | Complete ([PR #11427](https://github.com/adhithyan15/coding-adventures/pull/11427)) | Verify lowercase Cyrillic ж from the source-adjacent school-hand demonstration. | RussianIrina's 03:16–03:21 lowercase ж demonstration rises from the lower left through a rounded left arch and tall central loop, descends through the middle, continues into a rounded right arch, and finishes through a smaller rightward exit without lifting. The two-frame learner path preserves that zero-lift left-to-centre-to-right order while fitting Noto Sans Cyrillic's straight central upright and four diagonal arms. Cyrillic з is next. |
| HL-C09EH | Complete ([PR #11437](https://github.com/adhithyan15/coding-adventures/pull/11437)) | Verify lowercase Cyrillic з from the source-adjacent school-hand demonstration. | RussianIrina's 03:34–03:39 lowercase з demonstration begins at the upper left, circles the smaller upper lobe to the right, descends through the middle, and continues around the larger lower lobe into a rising rightward exit without lifting. The two-frame learner path preserves that zero-lift upper-lobe-to-lower-lobe order while fitting Noto Sans Cyrillic's compact printed double-lobe glyph; the variation note records that the printed form omits the school hand's exit join. Cyrillic и is next. |
| HL-C09EI | Complete ([PR #11446](https://github.com/adhithyan15/coding-adventures/pull/11446)) | Verify lowercase Cyrillic и from the source-adjacent school-hand demonstration. | RussianIrina's 03:56–04:02 lowercase и demonstration descends the left stem, turns directly into a rising diagonal, descends the right stem, and finishes through a small rising exit without lifting. The three-frame learner path preserves that zero-lift left-stem-to-diagonal-to-right-stem order while fitting Noto Sans Cyrillic's printed backwards-N glyph; the variation note records that the printed form omits the school hand's rounded entry and exit joins. Cyrillic й is next. |
| HL-C09EJ | Complete ([PR #11454](https://github.com/adhithyan15/coding-adventures/pull/11454)) | Verify lowercase Cyrillic й from the source-adjacent school-hand demonstration. | RussianIrina's 04:17–04:24 lowercase й demonstration completes the same joined body as и, then lifts once and draws the breve above from left to right as one dipped arc. The four-frame learner path preserves that body-before-breve order, left-to-right breve direction, and one-lift evidence while fitting Noto Sans Cyrillic's printed backwards-N body and separate curved mark. Cyrillic к is next. |
| HL-C09EK | Complete ([PR #11462](https://github.com/adhithyan15/coding-adventures/pull/11462)) | Verify lowercase Cyrillic к from the source-adjacent school-hand demonstration. | RussianIrina's 04:45–04:51 lowercase к demonstration descends the left stem, rises through a looped upper-right arm and returns to the middle, then continues down-right through the lower arm and a small rising exit without lifting. The three-frame learner path preserves that zero-lift stem-to-upper-arm-to-lower-arm order while fitting Noto Sans Cyrillic's printed vertical and two angular diagonals; the variation note records the source's rounded loop and entry and exit joins. Cyrillic л is next. |
| HL-C09EL | Complete ([PR #11469](https://github.com/adhithyan15/coding-adventures/pull/11469)) | Verify lowercase Cyrillic л from the source-adjacent school-hand demonstration. | RussianIrina's 05:06–05:10 lowercase л demonstration curves left around a small baseline hook, rises steeply to a high apex, descends through the right leg, and finishes through a small rising exit without lifting. The three-frame learner path preserves that zero-lift hooked-left-leg-to-apex-to-right-leg order while fitting Noto Sans Cyrillic's curved left leg, horizontal top shoulder, and straight right stem; the variation note records the source's pointed apex, slanted right leg, and entry and exit joins. Cyrillic м is next. |
| HL-C09EM | Complete ([PR #11477](https://github.com/adhithyan15/coding-adventures/pull/11477)) | Verify lowercase Cyrillic м from the source-adjacent school-hand demonstration. | RussianIrina's 05:26–05:31 lowercase м demonstration curves left around a small entry hook, rises to the first apex, descends through the central valley, rises to the second apex, descends through the right leg, and finishes through a small rising exit without lifting. The four-frame learner path preserves that zero-lift entry-to-first-apex-to-valley-to-second-apex-to-baseline order while fitting Noto Sans Cyrillic's straight upright stems and deep central V; the variation note records the source's rounded arches and entry and exit joins. Cyrillic н is next. |
| HL-C09EN | Complete ([PR #11483](https://github.com/adhithyan15/coding-adventures/pull/11483)) | Verify lowercase Cyrillic н from the source-adjacent school-hand demonstration. | RussianIrina's 05:47–05:52 lowercase н demonstration descends the left stem, turns upward through a rounded middle bridge, rises to the right shoulder, descends the right stem, and finishes through a small rising exit without lifting. The three-frame learner path preserves that zero-lift left-stem-to-middle-bridge-to-right-stem order while fitting Noto Sans Cyrillic's straight uprights and horizontal middle bar; the variation note records the source's rounded bridge and entry and exit joins. Cyrillic о is next. |
| HL-C09EO | Complete ([PR #11491](https://github.com/adhithyan15/coding-adventures/pull/11491)) | Verify lowercase Cyrillic о from the source-adjacent school-hand demonstration. | RussianIrina's 05:59–06:03 lowercase о demonstration begins at the upper right, curves left across the top, descends the left side, sweeps through the bottom, rises along the right side, and closes the oval without lifting. The two-frame learner path preserves that zero-lift counterclockwise closure order while fitting Noto Sans Cyrillic's wider upright oval; the variation note records the source's tall, slightly slanted school hand. Cyrillic п is next. |
| HL-C09EP | Complete ([PR #11498](https://github.com/adhithyan15/coding-adventures/pull/11498)) | Verify lowercase Cyrillic п from the source-adjacent school-hand demonstration. | RussianIrina's 06:26–06:31 lowercase п demonstration descends the left stem, turns upward through a rounded top shoulder, descends the right stem, and finishes through a small rising exit without lifting. The three-frame learner path preserves that zero-lift left-stem-to-top-shoulder-to-right-stem order while fitting Noto Sans Cyrillic's squared arch, straight uprights, and horizontal top bar; the variation note records the source's rounded Latin-n-like school hand and entry and exit joins. Cyrillic р is next. |
| HL-C09EQ | Complete ([PR #11508](https://github.com/adhithyan15/coding-adventures/pull/11508)) | Verify lowercase Cyrillic р from the source-adjacent school-hand demonstration. | RussianIrina's 06:46–06:52 lowercase р demonstration descends its stem below the baseline, retraces upward through the same stem, curves right through a rounded shoulder, descends to the baseline, and finishes through a small rising exit without lifting. The three-frame learner path preserves that zero-lift stem-before-bowl order while fitting Noto Sans Cyrillic's straight descender and closed rounded bowl; the variation note records the source's open long-descender Latin-p-like school hand and entry and exit joins. Cyrillic с is next. |
| HL-C09ER | Complete ([PR #11512](https://github.com/adhithyan15/coding-adventures/pull/11512)) | Verify lowercase Cyrillic с from the source-adjacent school-hand demonstration. | RussianIrina's 07:04–07:08 lowercase с demonstration begins at the upper right, curves left across the top, descends the left side, sweeps through the bottom, and rises into a small lower-right exit without lifting. The two-frame learner path preserves that zero-lift counterclockwise open-curve order while fitting Noto Sans Cyrillic's wider upright C-like outline; the variation note records the source's tall, slightly slanted school hand and rising exit. Cyrillic т is next. |
| HL-C09ES | Complete ([PR #11520](https://github.com/adhithyan15/coding-adventures/pull/11520)) | Verify lowercase Cyrillic т from the source-adjacent school-hand demonstration. | RussianIrina's 07:29–07:36 lowercase т demonstration descends the left stem, rises through a rounded first arch, descends the middle stem, rises through a rounded second arch, descends the right stem, and finishes through a small rising exit without lifting. The three-frame learner path preserves that zero-lift initial-descent-before-joined-top-movements order while fitting Noto Sans Cyrillic's printed central stem and horizontal top bar; the variation note records the source's two-arch Latin-m-like school hand and rising exit. Cyrillic у is next. |
| HL-C09ET | Complete ([PR #11529](https://github.com/adhithyan15/coding-adventures/pull/11529)) | Verify lowercase Cyrillic у from the source-adjacent school-hand demonstration. | RussianIrina's 07:50–07:55 lowercase у demonstration descends the left arm, rises through the right arm, retraces into a long descender, curls left through a lower loop, crosses the descender, and rises into a short exit without lifting. The four-frame learner path preserves that zero-lift left-arm-to-right-arm-to-descender order while fitting Noto Sans Cyrillic's printed upper arms and broad left-curving terminal; the variation note records the source's loop-descender Latin-y-like school hand and rising exit. Cyrillic ф is next. |
| HL-C09EU | Complete ([PR #11538](https://github.com/adhithyan15/coding-adventures/pull/11538)) | Verify lowercase Cyrillic ф from the source-adjacent school-hand demonstration. | RussianIrina's 08:16–08:26 lowercase ф demonstration descends the long central stem below the baseline, lifts once, restarts near the upper junction, circles the left loop, crosses the stem, circles the right loop, and finishes through a small rising exit. The five-frame learner path preserves that stem-before-left-loop-before-right-loop order and one-lift evidence while fitting Noto Sans Cyrillic's straight ascender-descender and two wider upright bowls; the variation note records the source's narrower linked loops and rising exit. Cyrillic х is next. |
| HL-C09EV | Complete ([PR #11547](https://github.com/adhithyan15/coding-adventures/pull/11547)) | Verify lowercase Cyrillic х from the source-adjacent school-hand demonstration. | RussianIrina's 08:42–08:49 lowercase х demonstration draws a right-bulging left curve top-to-bottom, lifts once, then draws a left-bulging right curve top-to-bottom through the same crossing and into a small rising exit. The four-frame learner path preserves that left-run-before-right-run order, crossing, and one-lift evidence while fitting the two curves to Noto Sans Cyrillic's four straight diagonal arms; the variation note records the source's facing curves and joins. Cyrillic ц is next. |
| HL-C09EW | Complete ([PR #11551](https://github.com/adhithyan15/coding-adventures/pull/11551)) | Verify lowercase Cyrillic ц from the source-adjacent school-hand demonstration. | RussianIrina's 09:05–09:10 lowercase ц demonstration descends the left stem, turns through a rounded join into a rising then descending right stem, and continues directly into a lower tail loop and rising exit without lifting. The four-frame learner path preserves that zero-lift left-stem-to-right-stem-to-tail order while fitting Noto Sans Cyrillic's squared U-like body, bottom bar, and short right descender; the variation note records the source's rounded diagonal join and looped exit. Cyrillic ч is next. |
| HL-C09EX | Complete ([PR #11558](https://github.com/adhithyan15/coding-adventures/pull/11558)) | Verify lowercase Cyrillic ч from the source-adjacent school-hand demonstration. | RussianIrina's 09:24–09:28 lowercase ч demonstration descends the short left stem, turns through a rounded join and rises to the top of the right stem, descends the full right stem, and curls into a rising exit without lifting. The three-frame learner path preserves that zero-lift short-stem-to-bowl-to-long-stem order while fitting Noto Sans Cyrillic's shorter left stem, shallow rounded bowl, and full-height right stem; the variation note records the source's narrower bridge, curled baseline, and rising exit. Cyrillic ш is next. |
| HL-C09EY | Complete ([PR #11564](https://github.com/adhithyan15/coding-adventures/pull/11564)) | Verify lowercase Cyrillic ш from the source-adjacent school-hand demonstration. | RussianIrina's 09:49–09:57 lowercase ш demonstration descends the left stem, rises through a rounded first join, descends the middle stem, rises through a rounded second join, descends the right stem, and curls into a rising exit without lifting. The five-frame learner path preserves that zero-lift left-to-middle-to-right order while fitting Noto Sans Cyrillic's three straight stems and two horizontal baseline joins; the variation note records the source's diagonal rounded joins, curled baseline, and rising exit. Cyrillic щ is next. |
| HL-C09EZ | Complete ([PR #11573](https://github.com/adhithyan15/coding-adventures/pull/11573)) | Verify lowercase Cyrillic щ from the source-adjacent school-hand demonstration. | RussianIrina's 10:17–10:25 lowercase щ demonstration descends the left stem, rises and descends through the joined middle and right stems, then continues into a lower tail loop and rising exit without lifting. The six-frame learner path preserves that zero-lift left-to-middle-to-right-to-tail order while fitting Noto Sans Cyrillic's three straight stems, two horizontal baseline joins, and short right descender; the variation note records the source's diagonal rounded joins and looped exit. Cyrillic ъ is next. |
| HL-C09FA | Complete ([PR #11587](https://github.com/adhithyan15/coding-adventures/pull/11587)) | Verify lowercase Cyrillic ъ from the source-adjacent school-hand demonstration. | RussianIrina's 10:34–10:38 lowercase ъ demonstration curls through a narrow entry loop and rounded top shoulder, descends the main stem, then circles the joined lower bowl counterclockwise and closes it without lifting. The five-frame learner path preserves that zero-lift flag-to-stem-to-bowl order while fitting Noto Sans Cyrillic's broad horizontal top flag, straight stem, and closed lower bowl; the variation note records the source's looped entry and rounded shoulder. Cyrillic ы is next. |
| HL-C09FB | Complete ([PR #11601](https://github.com/adhithyan15/coding-adventures/pull/11601)) | Verify lowercase Cyrillic ы from the source-adjacent school-hand demonstration. | RussianIrina's 10:45–10:56 lowercase ы demonstration draws the left stem and joined lower bowl continuously, lifts once, then descends the separate right stem into a rising exit. The five-frame learner path preserves that body-before-right-stem order and one-lift evidence while fitting Noto Sans Cyrillic's straight left upright, wide closed lower bowl, and separate straight right stem; the variation note records the source's narrow entry loop and curled exit. Cyrillic ь is next. |
| HL-C09FC | Complete ([PR #11607](https://github.com/adhithyan15/coding-adventures/pull/11607)) | Verify lowercase Cyrillic ь from the source-adjacent school-hand demonstration. | RussianIrina's 11:16–11:20 lowercase ь demonstration descends the stem, turns at the baseline, circles the joined lower bowl counterclockwise, and closes it against the stem without lifting. The four-frame learner path preserves that zero-lift stem-to-bowl order while fitting Noto Sans Cyrillic's straight upright and closed lower bowl; the variation note records the source's narrow entry stroke and rounded handwritten bowl. Cyrillic э is next. |
| HL-C09FD | Complete ([PR #11612](https://github.com/adhithyan15/coding-adventures/pull/11612)) | Verify lowercase Cyrillic э from the source-adjacent school-hand demonstration. | RussianIrina's 11:25–11:32 lowercase э demonstration draws the outer backwards-C curve from upper left around the right side to lower left, lifts once, then draws the middle tongue from right to left. The four-frame learner path preserves that outer-before-tongue order, right-to-left tongue direction, and one-lift evidence while fitting Noto Sans Cyrillic's broad open-left curve and straight middle bar; the variation note records the source's narrower school-hand curve and gently hooked tongue. Cyrillic ю is next. |
| HL-C09FE | Complete ([PR #11619](https://github.com/adhithyan15/coding-adventures/pull/11619)) | Verify lowercase Cyrillic ю from the source-adjacent school-hand demonstration. | RussianIrina's 11:44–11:58 lowercase ю demonstration descends the left stem, turns through a rising connector, and continues clockwise around the right oval to close without lifting. The five-frame learner path preserves that zero-lift stem-to-connector-to-oval order while fitting Noto Sans Cyrillic's straight left upright, horizontal middle bar, and wide closed oval; the variation note records the source's looped entry, diagonal connector, and cursive oval. Cyrillic я is next. |
| HL-C09FF | Complete ([PR #11627](https://github.com/adhithyan15/coding-adventures/pull/11627)) | Verify lowercase Cyrillic я from the source-adjacent school-hand demonstration. | RussianIrina's 12:13–12:21 lowercase я demonstration rises from a curved baseline entry, circles the upper loop counterclockwise, descends the long diagonal leg, and turns into a short exit without lifting. The four-frame learner path preserves that zero-lift rise-to-loop-to-leg order while fitting Noto Sans Cyrillic's straight right upright, broad upper bowl, and angular lower-left leg; the variation note records the source's curved entry, narrow loop, slanted leg, and exit join. The Cyrillic lowercase inventory is now source-verified; Gujarati is the next actionable inventory. |
| HL-C09FG | Complete ([PR #11633](https://github.com/adhithyan15/coding-adventures/pull/11633)) | Open the Gujarati inventory with independent vowel અ from a dedicated teaching animation. | t30apps.com's version-1.0 અ animation writes the joined left curve, lower body, middle shoulder, and small right arch first, lifts once, then descends the separate right stem into its foot. The four-frame learner path preserves that body-before-right-stem order and one-lift evidence while fitting Noto Sans Gujarati's broader printed proportions; both data and source notes retain the app's explicit warning that this is one variant rather than a universal standard. Gujarati આ is next. |
| HL-C09FH | Complete (PR pending) | Verify source-adjacent Gujarati independent vowel આ as the full અ sequence plus its added trailing ā stem. | t30apps.com's version-1.0 આ animation writes the joined અ body, lifts to descend અ's right stem, then lifts again to descend the added trailing ā stem. The five-frame learner path preserves that body-before-first-stem-before-trailing-stem order and two-lift evidence while fitting Noto Sans Gujarati's broader body and wider stem spacing; the source's explicit variation warning remains attached. Gujarati ઇ is next. |
| HL-C10 | Complete (#10010, #10013, #10067) | Complete A1 and add the A2-through-C2 spine tranches with all registered realization ledgers. | All seven declared stages carry nodes; every one of the 22 registered tracks has a non-drifting ledger entry for every node. |
| HL-C11 | Queued — capability and closure coverage complete | Finish representative chapter payoffs across all 22 tracks. | #10128 brought all 513 chapters to an authored `canDo`, spine mapping, known payoff lesson, and closed assessment. Remediate the remaining 27 payoffs below the 0.5 representativeness floor across ten tracks, then enforce the clean tracks instead of leaving their gates report-only. |
| HL-C12 | Queued — licensing decided, pipeline outstanding | Add the Class C illustration pipeline with provenance sidecars and a size budget. Licensing is settled and recorded in [`_assets/LICENSE.md`](./_assets/LICENSE.md); the remaining work is the pipeline itself. | Every asset carries `license`, `rightsAsserted`, `generator`, `model`, `prompt`, `date`, and `sha256`; CI fails any asset without a provenance sidecar or a recorded licence, and enforces the per-track size budget. |
| HL-C13 | Complete in #10102 | Deploy Language Ladder to GitHub Pages. | The validated relative-path build publishes to `/coding-adventures/language-ladder/` after every app, data-package, or human-language content change, preserving the repository's other Pages subdirectories. |
| HL-C14 | Complete (#9981) | Derive modality (`voice`/`sight`/`pen`) for every lesson and each chapter's drivable prefix. | The gap report publishes per-track modality counts and the corpus-wide drivable percentage; overrides without a recorded reason are reported. |
| HL-C15 | Complete (#10211) | Print modality signs and the drivable prefix at every chapter opening. | All 513 chapter openings across 22 books show font-independent car/eye/pen signs, full printed-lesson counts, and the core-derived hands-free prefix from the same canonical modality rollup used by the app and narration. |
| HL-C16 | Complete (#9981) | Build the narration export (`narration-cli`) with `--write`/`--check`. | Done. `src/speech.ts` + `src/narration.ts` + `src/narration-cli.ts` emit `<track>/narration/chNN.txt` and `.json` for all 375 chapters, hash-gated by `core/generated-narration-hashes.json` and checked byte-for-byte in CI. `[PAUSE Ns]`, `[REPEAT xN]` and `[YOU …: …]` survive as typed directives; a spoken answer is scored only against a compiled `hl-activity` contract, never against a cue. Tables linearise up to **3 columns** (371 of 442 tables, 272 of 340 table-bearing files); a refused table is *spoken* — size, headings, and reason — and marks its lesson `sight`. `maxLinearisableTableColumns` moved 0 → 3 in `core/chapter-policy.json`, taking the corpus from **63% to 84% drivable** (694 → 925 lessons). This is the audio-script output HL04 named and nothing had ever built. |
| HL-C17 | Queued — correctness complete, 61 lesson files remain | Linearise or reclassify the wide-table lessons. | HL-C16 discharged the correctness invariant: every table either reads aloud or is spoken as a refusal that marks its lesson `sight`. The current manifest measures **61 lessons** with a table wider than three columns; 45 need eyes for that reason alone. Reshaping those 45 tables would move the corpus from 91% to about 94% core-drivable without weakening the three-column narration policy. |
| HL-C18 | Queued — Spanish slice complete, 40 remain | Burn down the lessons that exceed the gentle-ramp budget. | No lesson introduces more than `maxNewAtomsPerLesson`; the current gap report measures 40 violations across 17 non-Spanish tracks, with six atoms in each of `SA-C06-number-cognates`, `PA-C06-panj-convergence`, and `BN-C06-numbers-1-5` as the current maximum. |
| HL-C18A | Complete (#9982) | Split the fifteen over-budget Spanish lessons, including the corpus-worst `ES-C31-numeros-11-20` at seven. | Spanish measures zero over-budget lessons; the fifteen become thirty-three prerequisite-ordered micro-lessons and the corpus figure drops from 52 to 37. |
| HL-C18B | Queued | Split the current 40 over-budget lessons across the 17 affected non-Spanish tracks. | Every track measures zero lessons above `maxNewAtomsPerLesson`; the corpus maximum drops from 6 to 3 without hiding the 529 legacy lessons that still declare no measurable atoms. |
| HL-C18C | Complete (#10036) | Measure the **script** ramp, which no gate had ever counted. The atom budget measures units of meaning and is blind to decoding load: `HI-W01-shirorekha-na-ma` declares **one** atom and shows **twelve** new Devanagari glyphs, and passed cleanly for a whole release. | `measureScriptRamp` counts new target-script glyphs per lesson in reading order, charged once; `maxNewGlyphsPerLesson: 3` (the corpus's own p90, not its max of 12) and `maxNewScriptSystemsPerLesson: 1` land in `core/chapter-policy.json` with measured provenance. First published figures: **61** lessons over the glyph budget (38 of which declare zero atoms), **5** opening more than one writing system at once — all Japanese Chapter 1. Cousin-script glyphs are counted separately and never charged: conflating them made a Kannada Chapter-1 lesson read as a 34-glyph cliff when its real Kannada load is 7. Report-only, per the HL05 precedent. Also fixes `measureRamp` being called by nothing but its own test — the atom budgets now reach the gap report — and three tracks (**gujarati**, **chinese**, **japanese**) that silently resolved to `latin`. |
| HL-C18D | Queued | Burn down the 61 lessons over the script budget, steepest first, and split the 5 Japanese lessons that open two writing systems at once. | No lesson introduces more than `maxNewGlyphsPerLesson` new glyphs or opens more than one writing system; the corpus maximum drops from 12 to 3. Starts with `HI-W01-shirorekha-na-ma`, `MR-C01-dhanyavad` and `RU-C01-da` at twelve each. Follows HL-C18C, which produced the list. |
| HL-C48 | Queued — **join landed; blocked on the same schema question as HL-C88** | Generate the cousin/cognate layer from `roots:` instead of hand-typing it, and make it visually skippable. **The key was `concept_tag` in this row until HL-C88 measured it and the spec was corrected; the two keys make different claims and `concept_tag` pairs *ir · andare · aller* as though they were relatives.** This row and HL-C88 track the same feature -- HL-C88 owns the work, this row is the older statement of it -- and the join now exists as `src/cousins.ts` with 76 Spanish lessons of reach. What remains for both is one schema decision: `roots:` records every etymon a lesson *discusses*, not the etymon of its *headword*, so a panel built on it today pairs *conocer* with *incontrare*. See HL-C88 for the audit. | The cross-language comparison table exists in **18 hand-typed instances across 4 Dravidian tracks** (4.6% of chapters) while four prefaces promise it per-lesson; `parse.ts` has no block type for it, so an author cannot write one into a canonical lesson. `concept_tag` (1,131 lessons) is the join key that would generate them, and the book renderer ignores it — as it ignores 708 `etymology_hook` fields (~25,400 words) that the app already displays cross-language. Per the owner's rule, the layer is a bonus for readers who know a relative: it must never gate comprehension, and its foreign glyphs must stay out of the target-script ramp. |
| HL-C49 | Complete (#10055) | Give every chapter a short, well-written intro, generated from `chapters.json`. | Generated chapters derive a standalone capability-led introduction from canonical metadata; the already-authored chapter openings remain authored. |
| HL-C50 | Complete (#10058, #10060, #10062, #10064, #10066, #10112, #10116, #10120, #10124) | Finish making every book a standalone artifact. | All 22 downloadable books carry checked cross-volume links plus generated pronunciation references, target-first glossaries, review questions and answers, and English-first subject indexes. |
| HL-C50A | Complete (#10112) | Generate the five missing pronunciation appendices from each track's canonical `pronunciation-reference.md`. | Chinese, Japanese, Persian, Russian, and Urdu join the other 17 books in carrying pronunciation back matter; `book-cli --check` byte-gates all five generated `.tex` files, and all five PDFs compile with no warning regressions. |
| HL-C50B | Complete (#10116) | Generate a compact glossary for every book from canonical word and phrase lessons. | All 22 books carry byte-gated glossary back matter with headword, non-redundant romanization, gloss, and introduction chapter; duplicate realizations merge without losing distinct senses, every script uses the book's configured font, and all PDFs compile without warning regressions. |
| HL-C50C | Complete (#10120) | Generate review questions and an answer key for every book from the same compiled `hl-activity` contracts Language Ladder scores. Backfill one schema-v2 activity in French and Bengali, the two tracks with zero contracts; never infer an answer from a legacy `[YOU ...]` cue. | Every track has nonempty, byte-gated review-question and answer-key back matter; each entry identifies its source chapter and lesson and resolves to the authored canonical answer plus accepted variants from the typed AST; all 22 PDFs compile without warning regressions. |
| HL-C50D | Complete (#10124) | Generate a compact subject index for every book from English meanings, dedicated topic lessons, and the checked generated/handwritten chapter manifest; exclude practice drills and never scrape prose for guessed keywords. | All 22 books carry nonempty, byte-gated index back matter; entries are alphabetized, target-script forms use each book's configured font, facets identify explicitly typed grammar/script/etymology/culture/pronunciation coverage, and every reference links to a checked chapter label and page without PDF warning regressions. |
| HL-C63 | Complete (#10128) | Author capability-ledger entries for the 98 handwritten chapters that the index audit found outside `chapters.json`; map the 47 canonical lessons across 11 of those chapters plus two required Spanish prerequisites, classify each new language-local segment, and keep chapter generation status independent from capability coverage. | All 513 declared chapters have an authored `canDo`, spine-node mapping, and representative payoff in their track's `chapters.json`; legacy schema-v1 payoffs name no invented atoms, all 49 newly placed lessons are reachable through prerequisite-safe `curriculum.json` paths and explicit extensions, the 98 handwritten `.tex` files remain protected from generation, and the capability and book manifests agree on chapter number, title, and label. |
| HL-C64 | Complete (#10128) | Parse nested LaTeX commands inside `\\chapter{...}` titles without truncating at the first formatting brace. | `loadBookCorpus` returns the complete title for chapters containing nested `\\emph{...}` commands, a focused regression test pins the behavior, and chapter-title drift remains zero after handwritten capabilities become checkable. |
| HL-C65 | Complete (#10132) | Migrate Spanish Chapter 7, the first chapter after the schema boundary, onto the strict schema-v2 lesson contract and resolve its ambiguous authored order from canonical prerequisites and prose. | All six Chapter-7 lessons declare unique sequence, duration, shared-spine placement, typed knowledge closure, and objective practice where appropriate; one documented prerequisite-safe order drives the app, narration, and chapter payoff; generated/drift-gated outputs are current; no lesson exceeds five minutes or the atom budget. |
| HL-C66 | Complete (#10135) | Continue the schema migration through Spanish Chapter 8, keeping numbers, age, and `tener` on the Chapter-7 prerequisite frontier instead of treating the next handwritten chapter as an opaque legacy island. | All five Chapter-8 lessons declare unique sequence, duration, shared-spine placement, typed knowledge closure, and objective practice where appropriate; the terminal practice lesson is mapped and represents the chapter payoff; no lesson exceeds five minutes or either atom budget; app, narration, and downloadable-book outputs remain derived from the same sources. |
| HL-C67 | Complete (#10142) | Continue the schema migration through Spanish Chapter 9, deriving the `ser`/`estar` contrast from the already-taught `estar`, `tener`, identity, state, and location frontier instead of presenting the contrast as a memorized rule list. | All five Chapter-9 lessons declare unique sequence, duration, shared-spine placement, typed knowledge closure, and objective practice where appropriate; minimal pairs introduce no vocabulary or forms ahead of their lesson; the terminal practice lesson is mapped and represents the chapter payoff; no lesson exceeds five minutes or either atom budget; app, narration, and downloadable-book outputs remain derived from the same sources. |
| HL-C68 | Complete (#10146) | Continue the schema migration through Spanish Chapter 10, introducing `ir`, `ir a` + infinitive, and `mi`/`tu`/`su` only from the Chapter-9 singular-person and identity/location frontier. | All four Chapter-10 lessons declare unique sequence, duration, shared-spine placement, typed knowledge closure, and objective practice where appropriate; `ir` does not front-load untaught plural forms, the future frame reuses only known infinitives, possessives introduce no undeclared nouns or agreement, the terminal practice lesson is mapped and representative, and all derived outputs remain current. |
| HL-C69 | Complete (#10150) | Continue the schema migration through Spanish Chapter 11, introducing singular `querer` and `poder`, then singular `nuestro`/`nuestra`, from the Chapter-10 frontier without restoring full boot tables or undeclared noun paradigms. | All five Chapter-11 lessons declare unique sequence, duration, shared-spine placement, typed knowledge closure, and objective practice where appropriate; `querer` extends the already-known singular `tener` stem change, `poder` adds only the singular `o`→`ue` pattern, the comparison lesson generalizes no form it has not taught, `nuestro`/`nuestra` reuse known masculine and feminine singular nouns while plural agreement waits, the terminal practice is mapped and representative, and all derived outputs remain current. |
| HL-C70 | Complete (#10153) | Continue the schema migration through Spanish Chapter 12, introducing singular `hacer` and `decir`, then comparing only the learned `tengo`/`hago`/`digo` yo-go forms from the Chapter-11 frontier. | All four Chapter-12 lessons declare unique sequence, duration, shared-spine placement, typed knowledge closure, and objective practice where appropriate; `hacer` stays on `hago`/`haces`/`hace` with known objects rather than untaught weather nouns, `decir` adds only `digo`/`dices`/`dice` and a known `cómo` frame, the yo-go comparison does not preview `poner`/`salir`/`venir`, the terminal practice is mapped and representative, and all derived outputs remain current. |
| HL-C71 | Complete (#10159) | Continue the schema migration through Spanish Chapter 13, introducing singular `poner`, `salir`, and `venir` one verb at a time from the Chapter-12 frontier. | All four Chapter-13 lessons declare unique sequence, duration, shared-spine placement, typed knowledge closure, and objective practice where appropriate; each verb stays inside its learned singular set and known words, the comparison grows only after all three verbs are taught, no full person table or undeclared place/time vocabulary appears, the terminal practice is mapped and representative, and all derived outputs remain current. |
| HL-C72 | Complete (#10164) | Continue the schema migration through Spanish Chapter 14, introducing the shared `ser`/`ir` preterite and regular `-ar` preterite inside the established singular-person and known-word frontier. | All three Chapter-14 lessons declare unique sequence, duration, shared-spine placement, typed knowledge closure, and objective practice where appropriate; `fui`/`fuiste`/`fue` precede any plural form, regular `hablé`/`hablaste`/`habló` reuse known `hablar`, context disambiguates `ser` from `ir` using only known words, no undeclared time/place/person vocabulary appears, the terminal practice is mapped and representative, and all derived outputs remain current. |
| HL-C73 | Complete (#10170) | Redesign and migrate Spanish Chapter 15 so regular `-er`/`-ir` preterites and strong preterites advance one bounded singular pattern at a time from the Chapter-14 frontier. | Six schema-v2 lessons first establish singular `comí`/`comiste`/`comió` and `viví`/`viviste`/`vivió`, then introduce the singular strong preterites of known `tener`, `hacer`, and `estar` one verb at a time; every teaching step stays within three new atoms and five minutes, every plural form and undeclared context is deferred, overly tidy Latin-history and `hizo` spelling claims are corrected, the terminal checkpoint maps all twelve chapter atoms, and app, narration, modality, progress, and book outputs are regenerated. |
| HL-C74 | Complete (#10177) | Redesign and migrate Spanish Chapter 16 so the imperfect and its contrast with the preterite grow from the Chapter-15 singular-person and known-word frontier instead of arriving as full paradigms and untaught stories. | Eight schema-v2 lessons now introduce singular `hablaba`/`hablabas`/`hablaba`, `comía`/`comías`/`comía`, and `vivía`/`vivías`/`vivía` in separate bounded steps before newly teaching `ver` and adding singular `era`/`eras`/`era`, `iba`/`ibas`/`iba`, and `veía`/`veías`/`veía`; all twelve atoms fit the chapter budget, all 28 objective activities compile, every plural and undeclared context waits, the history is carefully bounded, and the terminal checkpoint retrieves the full chapter. |
| HL-C75 | Complete (#10183) | Redesign and migrate Spanish Chapter 17 so future and conditional forms grow from Chapter 16's singular-person and known-word frontier instead of opening full paradigms, ten irregular stems, auxiliary paradigms, and late clock-time vocabulary at once. | Eight schema-v2 lessons introduce singular regular future forms for known `hablar`, `comer`, and `vivir`, then their singular regular conditional forms, before one bounded step adds only the learned `har-`, `tendr-`, and `podr-` stems; all twelve atoms fit the chapter budget, all 28 objective activities compile, every plural and additional irregular waits, examples remain natural combinations of known words such as `Hablaré español`, `Beberé café`, `Viviré en Madrid`, `Haría café`, and `Podría hablar español`, the Latin-to-Romance history is carefully bounded, and the terminal checkpoint retrieves the full chapter. |
| HL-C76 | Complete (#10190) | Redesign and migrate Spanish Chapter 18 so the present subjunctive grows from one singular trigger-and-form contrast instead of opening full regular paradigms, inherited-stem inventories, outlier inventories, and subordinate-clause traps together. | Nine schema-v2 micro-lessons now distinguish asserted from wanted meaning first, add regular `hablar`, `comer`, and `vivir` singular rows separately, carry only known `querer`, `poder`, and `hacer` irregularities into separate bounded steps, preserve the carefully scoped Arabic route into `ojalá`, and retrieve all twelve atoms in a mapped terminal checkpoint; all plural persons, additional irregulars, object pronouns, person nouns, and additional triggers wait. |
| HL-C77 | Complete (#10195) | Make the redesigned Spanish Chapters 7–18 genuinely book/app single-source instead of leaving the canonical schema-v2 lessons beside protected older handwritten LaTeX chapter bodies. | All twelve chapters now generate from 67 schema-v2 lessons with checked hashes, titles, labels, order, examples, review questions, and answers; four presentation-only title commands became plain canonical text, narrow person rows replace two wide terminal tables, a portable text callout replaces the unsupported warning emoji, and the Chapter-18 comparison now wraps cleanly. `check:books` is byte-current, the complete Spanish PDF has no overfull box or missing glyph, and Language Ladder plus all 41 book chapters consume the same lesson AST. |
| HL-C19 | Complete (#10199) | Verify every prose `strokeOrder` against an authored ductus, so no letter's step list implies a pen lift nothing has checked. | The live audit found 228 prose entries across ten scripts, including the previously omitted Japanese inventory and four more Gujarati entries than the stale estimate: one letter — Tamil ம — carries `penLifts` + `strokeOrderSource` and the same cited, font-checked `DUCTUS`; all other 227 render as **shape parts in usual order, pen lifts unverified**. Validation rejects partial or malformed verification, and an integration gate proves every claimed count and source matches an authored ductus. [`data/scripts/README.md`](data/scripts/README.md) records the exact current breakdown. |
| HL-C121 | Not started | **Teach ~500 Spanish verbs.** Owner directive, 2026-08-12: "I would assume you would need to learn at least 500 or so verbs for you to be relatively fluent in a language. So, make sure you are teaching all of those." **Measured the same day: the Spanish track teaches 43 distinct verb lemmas as headwords** --- estar(8), hablar(16), trabajar(20), estudiar(21), comer(34), vivir(35), beber(36), tener(47), ser(48), poder/querer(69), decir/hacer(70), poner(71), salir(72), venir(73), ver(108), pensar(153), entender(154), leer(155), escribir(156), tomar(159), preguntar(160), ayudar(161), gustar(163), caminar/correr/dormir(165), abrir/cerrar/levantarse/sentarse(166), contar(167), traer(168), conseguir(169), jugar(170), conocer(171), comprar/contestar/esperar(174), creer/deber/explicar(175). That is **9% of the target**, and the last 20 of them landed in a 25-chapter window (153-175), so the rate is not the problem --- the absence of a plan is. Needs: (1) an ordering principle that is defensible without an external frequency list --- propose morphological family x semantic field, so each verb reinforces a pattern the reader is mid-way through learning; (2) one lesson per verb (owner directive: never clobber several verbs into one lesson --- each has an origin to learn); (3) interleaving with the CEFR spine in runs of 8-10 so the grammar ladder is not swamped; (4) a gate that pins the verb count per track so the number cannot silently stall again. Note the corpus already teaches many FORMS of few verbs (the preterite/imperfect/subjunctive arcs all drill the same ~10 verbs) --- this row is about LEMMAS, which is a different measurement and the one the owner asked for. |
| HL-C123 | Not started | **Spanish has never attained pre-A1, and vocabulary is the only reason.** The level gate, run 2026-08-12: `touches: B2`, `attained: null`, `inProgressAt: pre-A1`. Two blockers, both measured. (1) **Vocabulary: 48 distinct headwords at or below pre-A1, against a target of 300** — a shortfall of 252. The track teaches 153 headwords in total, so even counting everything it has ever taught, at every level, it is half of one level's requirement. (2) **Reinforcement: 29 atoms at or below pre-A1 are revisited fewer than twice** (38 etymology hooks correctly waived). This is the same finding as HL-C121 seen from the other side: the owner asked for ~500 verbs, and the gate independently says the corpus is starved of headwords. The climb up the CEFR spine has been adding *structures*; the gate has been saying since it was written that structures are not what is missing. **This row should outrank further spine climbing.** Needs: a headword campaign scoped by level (300 at pre-A1 before anything claims A1), a decision on whether the 300 target is right, and a re-run of the gate after each wave so the number moves visibly. |
| HL-C124 | Not started | **`chapters.json` `spineNodes` has drifted from the curriculum path in 32 of 217 Spanish chapters.** Two failure modes, measured 2026-08-12: chapters 191-207 all carry `["SPINE-ASK-LOCATION"]` regardless of their real node, because a new chapter entry is seeded by copying the previous one and the field is never re-examined; and earlier chapters (1, 34, 36, 37, 48, 49, …) disagree on set membership or only on ordering. Chapters 208-220 had the same defect and were corrected by hand in HL-C113 steps 7-8. **Nothing reads this field today** — `src/levels.ts` and `src/level-gate.ts` both derive the node from `curriculum.json` — which is exactly why it was free to rot, and why the fix must ship with a gate rather than alone. Needs: recompute every chapter's `spineNodes` from the curriculum path, add a test asserting the two sources agree for every track that has both files, verify the test fails when a wrong value is reintroduced, and check the other 21 tracks for the same drift. |
| HL-C125 | Not started | **The rest of `SPINE-ARGUE-A-VIEW` is blocked on four words the corpus has never used.** Measured 2026-08-12 across all 217 chapters: `aunque` 0 occurrences, `sin embargo` 0, `por eso` 0, `mejor` 0. The node's own concepts are CONNECTIVE-HOWEVER and CONNECTIVE-ALTHOUGH, so it cannot be finished without them. HL-C113 step 8 opened the node with `pero`, `también` and `tampoco` (and minted `tan`, `poco`); still owed are `sin` (needed by `sin embargo`), `aunque` with the indicative (a fact conceded) and `aunque` with the subjunctive (a supposition conceded — which pays off the imperfect-subjunctive arc at 206-210), `sin embargo` as the formal register partner to `pero`, then the node's review and synthesis. `muy` is also untaught and is wanted for ARGUMENT-EVIDENCE. |
| HL-C126 | Not started | **One unbuilt B1 node gates the entire remaining ladder.** Measured 2026-08-12: `SPINE-DESCRIBE-EXPERIENCE` has **0 segments**, and so does `SPINE-HANDLE-TRAVEL`. `SPINE-READ-EXTENDED-PROSE` (B2) lists DESCRIBE-EXPERIENCE as its prerequisite, and `SPINE-DISCUSS-ABSTRACT` (B2) requires READ-EXTENDED-PROSE — so three nodes across two levels are unreachable until one B1 node is authored. B1 therefore is **not** finished, despite HL-C113 closing `SPINE-EXPRESS-CONDITION`: 31 lessons across three of five nodes. DESCRIBE-EXPERIENCE's concepts are ADJECTIVE-FEELING, AMBITION-EXPRESS, COMPARISON-BASIC and TIME-DURATION; **check the lexical inventory for each before authoring** — COMPARISON-BASIC needs `más`/`menos`/`mejor`, and `mejor` is not taught anywhere (see HL-C125). HANDLE-TRAVEL (DIRECTION-ASK, TRANSPORT-TICKET, LODGING-ROOM, PROBLEM-REPORT) is a pure vocabulary node and should be scheduled with HL-C123. |
| HL-C127 | **Complete (#PENDING)** — paid at chapters 241-245. | **Two preterite debts, named out loud in chapter 204 and still owed.** The `vosotros` forms (`hablasteis`, `comisteis`, `tuvisteis`) and the strong `nosotros` forms (`tuvimos`, `hicimos`, `estuvimos`). Chapter 204 tells the reader both are coming, which makes this a promise in the text rather than a nice-to-have. Small — two lessons, possibly three with a review — and it closes the preterite properly before anything else builds on it. The `vosotros` preterite (`hablasteis`, `comisteis`, `tuvisteis`, `fuisteis`) and the imperfect plural (`hablabamos`, `comiamos`, and the plurals of all three irregulars) are taught, which closes **both past tenses as complete paradigms** — the first two tenses in the book with no outstanding note. Also closed A1-V-04 and A1-V-06 on the exam inventory. |
| HL-C128 | **In progress — gate built; Spanish 53/85 (62%) → 56 → 60 → 64 → 66 → 68 → 71 → 77 → 81/85 (95%) at A1 after the demonstratives landed.** | **Replace `touches`/`attained` with a gate that asks whether a reader could PASS the exam.** Owner correction, 2026-08-12: *"The goal is not whether something touches some level. The goal is can someone pass that level of exam with just reading the book and slowly following its gentle ramp."* Both current numbers are corpus-internal — they measure whether lessons exist and whether atoms were revisited, neither of which is what an examiner tests. The replacement has a source: the **Plan Curricular del Instituto Cervantes** publishes a *finite, enumerable* grammar inventory per level (15 categories, two columns marked A1 and A2, **roughly 80-100 distinct grammar points at A1 alone**), plus inventories for nociones generales/específicas, ortografía and fonética. Needs: map every taught atom onto a Plan Curricular point, report per-level coverage as *points covered / points enumerated*, and let a track claim a level only when coverage is complete **and** HL-C130's task-shape criteria are met. That number, unlike `touches`, cannot be moved by adding a lesson on something the exam does not test. Source: <https://cvc.cervantes.es/ensenanza/biblioteca_ele/plan_curricular/indice.htm> **First result, 2026-08-12:** `core/exam-inventory-es-a1.json` enumerates 85 A1 points restated from the Plan Curricular structure, each carrying an *executable* probe rather than a hand-filled `coveredBy` annotation — coverage is recomputed from the corpus on every run, so it falls when an atom is retired and cannot go stale in the flattering direction. `src/exam-inventory.ts` resolves it; `tests/exam-inventory.test.ts` pins the number and was verified adversarially (an empty probe throws at load; deleting a point fails the pin). **Spanish covers 53 of 85 (62%)** after 220 chapters that had climbed to a B2 node. The gaps are not exotic: the demonstratives are absent ENTIRELY (este/ese/aquel, 3 of 3 points), `muy` is untaught, the `al`/`del` contractions are untaught, the gerund is untaught, `quien` is untaught, and the personal `a` is untaught. Still owed on this row: the same treatment for A2 and above, and the non-grammar inventories (nociones, ortografia, fonetica). **Step 2, 2026-08-13:** closed `Los demostrativos`, the only category reading **0 of 3** — chapters 221-225 teach `este`/`ese`/`aquel`, the neuters `esto`/`eso`/`aquello`, and a review. The gate behaved exactly as designed on the way through: coverage would not move until the three probes were wired, and the pinned 53 had to be re-pinned to 56 **deliberately**, with the category assertion going 3/0 → 3/3 in the same edit. Remaining at A1: 29 points, of which the cheapest are `muy`, the `al`/`del` contractions, `quien`, `ni`/`o`, the personal `a`, and the gerund. **Step 3, 2026-08-13:** chapters 226-229 teach the degree words `muy`, `bastante` and `mal`, closing **four** points across three categories with three lessons — the best points-per-lesson available on the list, because `muy` and `bastante` each appear on more than one inventory line. `El sintagma adjetival` is off the floor (0/1 → 1/1), and the worst-category-first report reordered itself to `Los cuantificadores` at 1/4, which is the next thing to do. Remaining at A1: 25 points; the cheapest are the `al`/`del` contractions, `quien`, `ni`/`o`, ordinals, the personal `a`, and the gerund. **Step 4, 2026-08-13:** chapters 230-235 close the `al`/`del` contractions, `quien`, and both missing coordinators (`o`, `ni`) — finishing **`Coordinacion` outright** at 5/5. Remaining at A1: **21 points**, and the cheap ones are now gone; what is left is the ordinals, the gerund, the personal `a`, the vocative, the stressed pronouns after a preposition, the exclamative `que`, and the imperfect/preterite paradigm gaps that HL-C127 owes. **Step 5, 2026-08-13:** chapters 236-240 teach the gerund (`-ando` / `-iendo`), the progressive `estar` + gerund, and the personal `a`. Two points closed. A **third was deliberately left open**: `A1-V-03` asks for the present indicative's own actual and durative readings, and chapter 238 teaches the progressive and contrasts it with the plain present — adjacent, but not the same thing, so probing it with progressive atoms would be the gaming this gate exists to catch. The note in the inventory records why. Remaining at A1: 19 points. **Step 6, 2026-08-13:** HL-C127 paid, closing A1-V-04 and A1-V-06. Remaining: 17 points. **Step 7, 2026-08-13:** chapters 246-250 close the stressed pronouns after a preposition (`para mi`, `conmigo`), the exclamative `que`, and the vocative. Remaining: **14 points** — the ordinals, subject-verb agreement as a stated rule, the `ver`/`dar` preterite, `A1-V-03`, and the handful of noun/article/quantifier gaps. **Step 8, 2026-08-13:** chapters 251-256 finish every set the book had taught only HALF of — `ahi`/`alli` beside `aqui`, `ahora`/`hoy` beside `manana`, `unos`/`unas` beside `un`/`una`, `vuestro` beside `nuestro`, and the `ver`/`dar` preterite. Six points. One of them, `A1-Q-04`, closed with **no new content at all**: `bastante` was taught at ch227 and its probe had simply never been wired — a point left marked uncovered while genuinely taught is its own kind of measurement error, and the only reason it surfaced is that the report enumerates every uncovered point by name rather than reporting a bare count. **The remaining 8 are a different problem and should not be attacked the same way.** Four of them — A1-SN-03 (subject-verb agreement), A1-Q-03 (poco/mucho agreement), A1-N-01 (proper nouns as a class), A1-V-03 (the present tense's own actual/durative readings) — are things this book demonstrates on nearly every page and never states. They need lessons that make explicit what the reader already does by reflex, without info-dumping, which is a genuinely different authoring problem from anything solved in steps 1-8. The other four are unbuilt structures: the ordinals, `uno...otro`, word-order flexibility, and the infinitive as subject. **Step 9, 2026-08-13:** chapters 257-261 close the four points the book had *demonstrated on every page and never stated* — the present tense's two readings, subject-verb agreement, quantifier agreement, and proper nouns. Each lesson has the reader DO the thing first and then names it, so nothing is info-dumped and the reader can now answer a direct question about what they were already doing correctly. **Four points remain**, all of them genuinely absent structures rather than implicit knowledge: the ordinals, `uno...otro`, word-order flexibility, and the infinitive as subject. **Step 10, 2026-08-13:** chapters 262-266 close the last four — the ordinals, `uno...otro`, the infinitive as subject, and word-order flexibility. **Every A1 grammar point the inventory enumerates is now taught.** What remains for this row is the levels above A1 (the same treatment for A2 and up) and the non-grammar inventories (nociones, ortografia, fonetica), neither of which has been started. Note also that A1 coverage is necessary and NOT sufficient for HL-C123: the exam still has four papers and the corpus still has one activity shape, which is HL-C130, and it still has no connected prose, which is HL-C129. A reader who has finished chapter 266 holds all the A1 grammar and has still never read a Spanish paragraph. |
| HL-C129 | **NEXT (Spanish side)** — premise corrected 2026-08-14 | **Connected Spanish exists, and there is far too little of it.** The row previously said the longest run in the book was 10 words and that a reader could finish it never having read a paragraph. **That was wrong**, and so were two of my re-measurements. Counting DIALOGUE BLOCKS rather than single italic spans, and filtering out English explanation: **29 genuinely Spanish passages of >=10 words, 26 of >=12, five of >=20, and exactly one of >=30** (`ES-C05-practice`, chapter 13, 30 words) across 412 lessons. So the gap is real but it is scarcity, not absence: about one connected passage every fourteen lessons, and only one that reaches thirty words. The DELE A1 reading paper is four tasks on connected texts. | Passages at spaced points under **reading closure** — only words taught earlier, or glossed in the lesson as `ES-C09-sintesis-ocho` does. A passage every few chapters, reaching 60+ words by A2. |
| HL-C130 | Not started | **Every one of the 704 activities in the corpus is the same task shape, and the exam has four.** Measured 2026-08-12: all 704 `hl-activity` blocks are `kind: "text"` — a one-line prompt with a short expected answer. The DELE structure is four papers in two groups, and **a candidate must score at least 30/50 in EACH group independently** (Group 1 = reading + written expression; Group 2 = listening + oral expression), so being strong at half the exam is a fail, not a partial pass. Against that: `writing` is claimed by **44 of 366 lessons (12%)**, and only 7 lessons have `type: writing`, while `speaking` and `listening` are claimed by 98% each. There is no reading-comprehension item over a passage, no listening item, no free written production of the length the exam asks for, and no oral prompt. Needs: new activity kinds (`reading-comprehension`, `listening`, `free-writing`, `oral-prompt`), a written-production strand that reaches the exam's word counts, and at least one full mock paper per level so the reader has rehearsed the shape before sitting it. Source for structure and the per-group pass rule: <https://londres.cervantes.es/en/courses_spanish/students_spanish/dele_diplomas_info/exam_format.htm> |
| HL-C131 | Not started | **`quien` is printed from chapter 6 and taught at chapter 232 — 351 lessons later.** Surfaced by HL-C128 step 4, which did not create the problem but made it measurable: until a lesson OWNED `quien`, the continuity checker had no teacher to measure the distance against and stayed silent. Four lessons print it early — `ES-C03-como-acento` and `ES-C03-familia-qu` (ch6), `ES-C07-que` (ch36) and `ES-C53-que-relativo` (ch187) — all of them using it as an example of an asking word while the reader has no idea what it means. `al` has the same shape at a smaller distance: `ES-C39-jugar` prints *jugar al* 65 lessons before ch230 teaches the contraction. **The fix is scheduling, not prose**: `quien` belongs beside `que` and `donde` in the chapter-36 asking-word cluster, and `al` belongs before the first lesson that needs it. Both mean renumbering, which is why they were not done inside a content PR. Note the continuity ceiling was re-seated 455 -> 463 to record this rather than hide it, and two of those eight are false positives — `ES-W02-enye` prints "ni" as the Latin letter sequence behind n-tilde, not as the word. |
| HL-C78 | Complete (#10202) | Reconcile the early foundational backlog rows against what later deliveries actually shipped. | Every pre-HL-C19 queued row is checked against direct code and test evidence; work that is already complete is closed with the PRs or concrete implementation that delivered it, partially complete rows state only their measured remainder, and genuinely absent work stays queued in priority order. |
| HL-C30 | Closed — no move is both legal and useful | Recover Arabic's drivable prefix by moving the writing lessons that open Chapters 3 and 4 later in their chapters. | Measured and answered: zero. Both chapters are prefix-0 under **every** legal ordering because neither has a `voice` lesson without an in-chapter prerequisite, and all 18 of Arabic's `sight` lessons are tables, not script. Corpus-wide only 2 chapters (`portuguese ch2`, `italian ch2`, +4 lessons) can be improved by reordering at all; 116 of the 123 zero-prefix chapters are table-blocked at the root and belong to HL-C17. See *Findings from HL-C30*. |
| HL-C24 | Complete (#9979) | Pilot real chapter payoff lessons on the weakest Latin chapters. | Latin chapters 19, 21, 33, and 36 each own a dedicated terminal consolidation lesson built only from already-taught material, and `chapters.json` points their `payoff.lesson` at it. |
| HL-C25 | Queued | Scale the HL-C24 payoff pattern across the remaining 32 Latin chapters and the other 19 tracks. | Every chapter's payoff is a lesson written to be a payoff, not the chapter's last teaching lesson pressed into service. |
| HL-C26 | Complete (#9977) | Give the hand-written early chapters a checkable title and label without making them generated. | The 105 chapters with a committed `.tex` but no `targets[]` entry are recorded in a new `handwritten[]` list in `core/book-generation.json`, transcribed from what each `\chapter{}`/`\label{}` actually declares. `generatedBookOutputs` never walks that list, so `check:books` still passes byte-for-byte and no authored chapter can be overwritten; `chapter-title-drift` no longer skips them. |
| HL-C44 | Complete (#9983) | Emit the derived modality as a generated, drift-gated manifest so different outputs can be filtered from one source. HL-C14 derived `voice`/`sight`/`pen` per lesson and a drivable prefix per chapter, but only at runtime and only into the human-readable gap report — no book builder, app, or driving-edition renderer had a file to filter on. | `core/lesson-modality.json` carries per-lesson `id`/`language`/`chapter`/`sequence`/`modality`/`derived`/`drivable`/`reasons`/`sourceHash`, per-chapter drivable prefix and ordered `drivableLessonIds`, per-track rollups, and a corpus summary (1,096 lessons; 708 `voice`, 337 `sight`, 51 `pen`; 65% drivable; 375 chapters; 551 lessons reachable in prefix order; 199 fully drivable chapters; 121 unstartable by ear). `modality-cli --write`/`--check` mirrors the `book-cli` contract, `check:modality` runs in CI beside `check:books`, and the schema reserves room for HL-C41's `coreModality` as a purely additive key. |
| HL-C32 | Complete (#9980) | Diagnose and repair the Russian track, worst in the corpus on two independent measurements: 9% drivable with **zero** lessons reachable by ear in either chapter, and payoff representativeness of 0.20. | Russian measures 73% drivable (16 `voice`, 1 `sight`, 5 `pen`) with 15 lessons reachable in chapter-prefix order, and Chapter 2's payoff representativeness is 0.67 against the 0.5 floor. Zero new validation errors, zero duration violations. |
| HL-C39 | Complete (#9984) | Add Mandarin Chinese as the 21st track — a **scale test** for whether the curriculum model generalises beyond Indo-European and Dravidian. Chapter 1 only, authored deep rather than swept wide. | A complete, CI-green track: 7 schema-v2 Chapter 1 lessons, an 11-node `curriculum.json` ledger, an HL05 capability with a payoff assessing 10 of 11 chapter atoms, and a generated book chapter. Three findings reported rather than hidden: **(1)** the cousin web does **not** transfer — Chinese shares no ancestor with English, so character composition replaces etymology and is honestly a weaker hook, since it anchors to knowledge being acquired in the same breath rather than knowledge the reader already had; **(2)** HL00's word→letter script rule becomes word→character→component, three levels not two, and for Chinese the "letters in this word" and "the word, taken apart" sections collapse into one analysis; **(3)** tone needed a data-layer extension (`ScriptData.tones` and `ScriptData.toneSandhi`) because `Letter.tone` can label a glyph but cannot express an inventory or a rule that changes pitch across a *sequence*. Also added the `pronunciation` lesson type, because no earlier track ever needed a lesson about sound. |
| HL-C40 | Complete (#9986) | Add **Japanese** as a track, Chapter 1 only, as the corpus's hardest scale test: three writing systems at once, kanji with multiple readings, grammatical politeness, and no shared ancestry with English. | 21st registered track; 8 schema-v2 lessons; a ledger entry for all 11 spine nodes; `data/scripts/japanese.json` covering hiragana, katakana, and kanji in one inventory; `_fonts/NotoSansJP-Subset.ttf` with `subset-jp.sh`; generated Chapter 1 compiling under XeLaTeX with zero overfull boxes and zero missing glyphs. Findings recorded below. |
| HL-C41 | Complete (#9985) | Derive modality per block as well as per lesson, so a voice lesson can carry a detachable writing segment. | `deriveBlockModality` and `coreModality` ship with the `writing` block type, and the drivable prefix counts the core; the separately blocked ductus work remains only in HL-C47. |
| HL-C42 | Queued | Let a track declare more than one script. HL01 gives a track exactly one `script` id and validates headword glyphs against exactly one inventory, so Japanese's hiragana, katakana, and kanji share one file with a per-sign `role` doing the separating. | A track declares `scripts: [...]`, `uncoveredGlyphs` resolves a character against any declared inventory, and a lesson can say which system a word is written in without a naming convention inside `role`. |
| HL-C43 | Queued — **measured; the filter is trivial and the finding is not** | Build the dictation-friendly driving edition from canonical block modality. | A generated edition omits sight and pen blocks while preserving every voice-capable lesson's prerequisite order, narration directives, and objective practice contracts. **Measured 2026-08-12 before building, and the measurement moves the work.** The manifest already carries per-lesson `drivable`, `coreDrivable`, `detachableSegments`, chapter and sequence, so the *selection* is nearly free -- take drivable lessons in reading order. **What the selection reveals is the actual problem.** Spanish keeps **301 of 341 lessons (88%)**, but the omissions are not spread evenly: **24 chapters would vanish entirely**, and **15 of those 24 are Review or Synthesis chapters** -- 19 *The Three Forms Side by Side*, 26 *Three Endings*, 41 and 46 *the Whole Present Tense*, 60 *Describing Things*, 81, 86, 96, 101, 102, 114... The reason codes say why: **21 `sight-cue` and 5 `wide-table`**, because consolidation is taught by putting forms side by side, and a table is the one thing a driver cannot look at. **So a naive dictation edition deletes precisely the chapters the curriculum's design depends on** -- the review and synthesis chapters the owner asked for by name. The edition is not a filtering job; the real work is giving those chapters a voice-capable form (a spoken contrast, a call-and-response) so consolidation survives the drive. **Other tracks are worse and the ratio matters for scope:** french 85%, tamil 57%, arabic 44%, **japanese 1 lesson of 8 (12%)** -- a dictation edition of Japanese would not be a book. Decide whether this ships Spanish-first or waits for a corpus-wide voice-capable review format. |
| HL-C45 | Queued | Structure the `register` field, which Japanese has outgrown. Today it is an open string, adequate for a *tú*/*usted* word choice and inadequate for a system where politeness is verb morphology on every predicate and keigo swaps the verb outright (言う → おっしゃる / 申す). | `register` becomes a small record — speech level, addressee honorification, referent honorification — that the 20 existing tracks map onto losslessly and that Japanese Chapter 3's honorific prefix **お-** can express without a free-text convention. |
| HL-C46 | Queued | Author the first interspersed `writing` segment, in a track whose ductus is already sourced. | One ordinary Tamil lesson carries a `## Writing: ம` segment citing the UT Austin primer; the book prints it, `coreModality` stays `voice`, and the corpus regression pin moves deliberately. |
| HL-C47 | Blocked on provenance | Author Telugu / Kannada / Malayalam base-consonant ductus. | One openable published primer with numbered stroke arrows per script; then ~36 base consonants and the vowel signs per script pass the three `strokes.ts` font invariants with citation and URL. Blocked, not queued: no source was reachable in HL-C41. |
| HL-C27 | Complete (#9975) | Run the book catalog builder's tests in CI. `test_build_human_language_book_catalog.py` existed but was executed by no workflow, so the script that writes the published `index.html` and `catalog.json` shipped with its tests never running. | `human-languages-books.yml` runs the suite in its own named step before the expensive XeLaTeX build, and both the workflow's `paths:` trigger and the `detect` job's `git diff` list include the test file so a change to it re-runs the job. |
| HL-C51 | Complete (#10068) | Complete the shared forty-core-verb inventory. | Every one of the forty canonical verb concepts is taught by at least one track, with the final tranche preserving one-verb-per-lesson pacing. |
| HL-C53 | Complete (#10077) | Prove the pre-A1 vocabulary expansion mechanism. | The pilot establishes that one canonical lesson buys one measurable headword and records the real cost of the 300-word pre-A1 target. |
| HL-C54 | Complete (#10083) | Run vocabulary wave 2 across French, German, Portuguese, and Italian. | Four aligned track tranches add one headword per sub-five-minute lesson with generated books, narration, and modality kept in sync. |
| HL-C55 | Complete (#10085) | Run vocabulary wave 3 across Russian, Bengali, Gujarati, and Kannada. | Four aligned track tranches add one headword per sub-five-minute lesson while reducing the absolute never-revisited atom count. |
| HL-C56 | Complete (#10087) | Run vocabulary wave 4 across Marathi, Punjabi, Sanskrit, and Urdu. | Four aligned track tranches add one headword per sub-five-minute lesson and leave all four tracks realizing every pre-A1 spine node. |
| HL-C57 | Complete (#10050; legacy commit alias HL-C44) | Expand the eight-verb tranche to seven tracks. | The aligned core-verb slice reaches seven tracks without raising lesson or chapter ramp budgets. |
| HL-C58 | Complete (#10054; legacy commit alias HL-C45) | Expand the eight-verb tranche to eleven tracks. | Four more tracks receive the aligned eight-verb slice with prerequisite closure and generated outputs. |
| HL-C59 | Complete (#10059; legacy commit alias HL-C46) | Expand the eight-verb tranche to fifteen tracks. | Four more tracks receive the aligned eight-verb slice with reinforcement and drift gates preserved. |
| HL-C60 | Complete (#10061; legacy commit alias HL-C47) | Expand the eight-verb tranche to all twenty then-registered tracks. | Every then-registered track teaches the aligned eight-verb slice. |
| HL-C61 | Complete (#10063; legacy commit alias HL-C48) | Publish `coreModality` for the driving-course filter. | Detachable sight and writing blocks no longer hide an otherwise drivable lesson, raising the measured drivable corpus to 1,230 lessons. |
| HL-C62 | Complete (#10065; legacy commit alias HL-C49) | Add hear, sleep, sit, and stand to the everyday-verb program. | Four additional canonical verb concepts enter the shared spine and are taught in prerequisite-safe micro-lessons. |

The project owner decided on 2026-08-06 that the curriculum ships **two editions from
one canonical source**: the complete book, which keeps everything including the writing
instruction, and a later dictation-friendly **driving edition**, which omits what a
driver cannot do. HL-C44 is the machinery that makes the filter possible and nothing
more — `core/lesson-modality.json` is the file both editions read. HL-C43 builds the
driving edition itself. The manifest's `modality` field is the conservative
whole-lesson answer, so the edition filter is correct today; HL-C41 adds block-level
modality as a purely additive `coreModality` key, at which point the driving edition
can skip a lesson's short optional writing segment instead of dropping the whole
lesson. Today, any pen content costs a commuter the entire lesson — 121 of the 375
chapters cannot be started by ear at all.

The illustration licensing question HL06 raised is **settled**. The project owner
decided on 2026-08-06 that the books stay CC BY-SA 4.0 and that generated
illustrations are marked `CC0-1.0` with `rightsAsserted: false`, each carrying a
provenance sidecar — because a CC licence grants copyright permissions that purely
AI-generated output likely cannot support, and CC0 is safe whichever way the law
settles. The decision, its reasoning, the required sidecar fields, and the two
operational constraints on prompting and generator terms are recorded in
[`_assets/LICENSE.md`](./_assets/LICENSE.md), following the `_fonts/OFL.txt`
precedent. HL-C12 is therefore unblocked: CI still gates every Class C asset on having
a sidecar and a recorded licence, but the licence to record is now known.

HL05 also reserves, and deliberately does not implement, a `presents.knowledge` tier
that would let a payoff use a glossed-but-never-assessed word. Strict closure is kept,
so early chapters ramp slightly more slowly than a trade step-by-step grammar does.
Reserving the key now makes enabling it later a flag flip rather than a corpus
migration.

## P0 — current publication and validation gaps

| ID | Status | Work item | Completion signal |
|---|---|---|---|
| HL-B01 | Complete (#9472) | Publish the five authored Persian lessons as a two-chapter LaTeX starter book. | XeLaTeX builds the book; CI discovers an 18th PDF; chapters map to lessons 1 and 2. |
| HL-B02 | Complete (#9474) | Publish the five authored Urdu lessons as a two-chapter starter book. | XeLaTeX builds with correct RTL shaping; Urdu appears in the public catalog. |
| HL-B03 | Complete (#9478) | Publish Russian's two authored chapters as a starter book. | The existing Cyrillic lessons and roadmap produce a downloadable PDF. |
| HL-V01 | Complete (#9483) | Add a machine-readable curriculum gap report and computed duration budget. | CI reports lessons at or above 300 seconds, missing prerequisites, book coverage, and track-schema status. |

The Russian publication audit found eight of its thirteen Chapter 1--2
curriculum lessons currently declare five minutes or more, including one at six
minutes. This is concrete input to HL-V01 and HL-D01, not silently treated as
fixed by the book: the starter edition presents shorter dependency-ordered
micro-sections while the canonical duration split remains measurable debt.

The first deterministic HL-V01 snapshot measures 485 lessons at or above 300
effective seconds, zero unknown prerequisite ids, 42 later-chapter lessons with
no declared prerequisite, 257 lesson chapters without a matching book chapter,
and all 20 tracks still on the legacy lesson schema. The report is evidence for
the next migrations; it deliberately does not fail CI on already-recorded debt.

## P1 — one-source migration

| ID | Status | Work item | Why it follows P0 |
|---|---|---|---|
| HL-S01 | Complete (#9497) | Migrate Spanish Chapters 1–3 to schema version 2 with typed body blocks and knowledge closure. | The 24-lesson slice has unique order, transitive knowledge closure, typed blocks, and no effective-duration violation. |
| HL-G01 | Complete (#9505) | Generate a Spanish LaTeX chapter from the canonical lesson AST and compare source hashes with the app. | Removes the first handwritten book copy now that the AST contract is executable. |
| HL-G02 | Complete (#9509) | Generate Spanish Chapters 2–3 from their canonical schema-v2 lesson AST. | Extends the proven one-source path across the rest of the migrated pilot before broad corpus work. |
| HL-D01A | Complete (#9516) | Remove all nine sub-five-minute violations from the complete Russian starter track. | The report measures zero Russian violations; every changed or added lesson is below 300 effective seconds. |
| HL-D01B | Complete (#9520) | Remove all eight sub-five-minute violations from the Marathi track. | The report measures zero Marathi violations; the one genuinely long lesson is now two prerequisite-ordered micro-lessons. |
| HL-D01C | Complete (#9523) | Remove all nine sub-five-minute violations from the Gujarati track. | The report measures zero Gujarati violations; the one genuinely long lesson is now two prerequisite-ordered micro-lessons. |
| HL-D01D | Complete (#9528) | Remove all ten sub-five-minute violations from the Punjabi track. | The report measures zero Punjabi violations; the one genuinely long lesson is now two prerequisite-ordered micro-lessons. |
| HL-D01E | Complete (#9531) | Remove all ten sub-five-minute violations from the Sanskrit track. | The report measures zero Sanskrit violations; the 513-second anchor lesson is now three prerequisite-ordered micro-lessons. |
| HL-D01F | Complete (#9535) | Remove all eleven sub-five-minute violations from the Bengali track. | The report measures zero Bengali violations; all eleven lesson bodies remain unchanged because their computed durations were already below 300 seconds. |
| HL-D01G | Complete (#9540) | Remove all twenty sub-five-minute violations from the Italian track. | The report measures zero Italian violations; four new prerequisite-ordered micro-lessons preserve the register, metaphor, suppletion, and agreement content that did not fit safely in the original lessons. |
| HL-D01H | Complete (#9545) | Remove all twenty-three sub-five-minute violations from the Portuguese track. | The report measures zero Portuguese violations; five new prerequisite-ordered micro-lessons preserve the register, suppletion, grammar-choice, and etymology content from the five computed violations. |
| HL-D01I | Complete (#9552) | Remove all twenty-five sub-five-minute violations from the French track. | The report measures zero French violations; three new prerequisite-ordered micro-lessons preserve register, suppletion, and pronominal-agreement depth. |
| HL-D01J | Complete (#9559) | Remove all twenty-seven sub-five-minute violations from the German track. | The report measures zero German violations; five new prerequisite-ordered micro-lessons preserve register, practice, areal-history, agreement, and etymology depth. |
| HL-D01K | Complete (#9565) | Remove all thirty-six sub-five-minute violations from the Telugu track. | The report measures zero Telugu violations; one new prerequisite-ordered micro-lesson separates phrase formation from register and source-evidence judgment. |
| HL-D01L | Complete (#9571) | Remove all thirty-seven sub-five-minute violations from the Kannada track. | The report measures zero Kannada violations; one new prerequisite-ordered micro-lesson separates suffix forms and sound history from the agglutinative-versus-fusional comparison. |
| HL-D01M | Complete (#9579) | Remove all thirty-seven sub-five-minute violations from the Malayalam track. | The report measures zero Malayalam violations; four new prerequisite-ordered micro-lessons separate vocabulary from etymology, register, and cross-language comparison. |
| HL-D01N | Complete (#9585) | Remove all thirty-nine sub-five-minute violations from the Arabic track. | The report measures zero Arabic violations; four new prerequisite-ordered writing steps preserve the abjad, joining, whole-word assembly, and hamza content. |
| HL-D01O | Complete (#9593) | Remove all forty sub-five-minute violations from the Hindi track. | The report measures zero Hindi violations; thirteen new prerequisite-ordered lessons preserve its script, etymology, grammar, and register depth. |
| HL-D01P | Complete (#9604) | Remove all forty-two sub-five-minute violations from the Tamil track. | The report measures zero Tamil violations; twenty new prerequisite-ordered lessons preserve its script, etymology, grammar, register, and source-evidence depth. |
| HL-D01Q | Complete (#9610) | Remove all forty-three sub-five-minute violations from the Latin track. | The report measures zero Latin violations; six new prerequisite-ordered lessons preserve its grammar, etymology, usage, and attestation depth. |
| HL-D01R | Complete (#9624) | Remove all fifty-five remaining sub-five-minute violations from the Spanish track. | The report measures zero Spanish violations; twelve new prerequisite-ordered lessons preserve the grammar, etymology, usage, writing, and practice depth from the genuinely long lessons. |
| HL-D01 | Complete (#9624) | Split or rewrite every lesson whose computed duration is at least 300 seconds. | The deterministic report now reaches zero effective-duration violations across all twenty tracks. |
| HL-S02 | Complete (#9634) | Migrate Spanish Chapters 4–6 to schema v2 before generating their book chapters. | All 27 lessons have typed blocks, unique sequence, transitive knowledge closure, and sub-five-minute duration guarantees. |
| HL-G03 | Complete (#9646) | Generate Spanish Chapters 4–6 from their canonical schema-v2 lesson ASTs after HL-S02. | All six generated chapters now share lesson hashes with Language Ladder; Markdown tables retain their structure in print. |
| HL-G04 | Complete (#9915) | Normalize paired straight quotation marks when canonical prose is rendered into LaTeX. | Generated prose uses true opening and closing marks under every book's language rules without changing the canonical app text, code spans, escaped literals, or link destinations. |
| HL-G05 | Complete (#9917) | Preserve canonical Markdown hyperlinks in generated LaTeX chapters. | Source notes and learner-facing links render as live `\href` targets in PDFs instead of retaining only their labels. |
| HL-G06 | Complete (#9915) | Preserve indented continuation lines inside generated Markdown blockquotes. | Multiline learner examples remain inside one LaTeX quote/callout, so typography and layout do not split halfway through a canonical example. |
| HL-V02 | Complete (#9653) | Validate learner-facing target-language prompts against block-level knowledge declarations and prerequisite closure. | Schema-v2 production and recall blocks cannot ask for an undeclared form or a form absent from the lesson's transitive knowledge frontier. |
| HL-V03 | Complete (#9900) | Compile individual prompt, answer, accepted-variant, feedback, and response-time contracts from typed activity blocks. | Compact JSON directives compile into validated runtime answer sets; each activity names a non-empty assessed-atom subset, carries feedback/time, and never scrapes prose. |
| HL-A01 | In progress (#9901 + Russian and Persian/Urdu slices) | Author objective activity coverage for every mapped non-lexical frontier. | The first tranche covers every ready schema-v2 track; later slices cover Russian's naming chain, Persian/Urdu Chapters 3–5 practice, and each migrated Spanish terminal checkpoint. Coverage is 58 of 135 across 18 tracks; 77 lessons remain, and the Chapter-18 legacy migration blocker is now zero. |
| HL-Q01 | Complete (#10089, after #9916) | Restore a clean standalone TypeScript typecheck for Language Ladder. | `npm run typecheck` passes after fixing the pre-existing DOM element type, review-log cast, ESM fixture paths, and unused test symbols; BUILD now keeps the gate enforced. |
| HL-Q02 | Complete (#10098) | Split Language Ladder's monolithic production JavaScript bundle. | Learn mode lazily fetches only completed and current-frontier lessons; corpus-wide views opt into the full set; the four eager chunks are each below 410 kB and Vite emits no size warning. |
| HL-Q03 | Complete (#10100) | Batch lazy full-corpus loading without regressing Learn's frontier-sized downloads. | Track-local 32 kB caps reduce the full-corpus fan-out from 1,669 lesson requests to 278 batches while preserving lazy frontier loading; BUILD enforces both request and byte ceilings. |
| HL-B04 | Complete (#9661) | Publish Marathi Chapter 6 from its two canonical lessons rather than hand-copying another book chapter. | Both schema-v2 lessons now generate the PDF chapter from the same source hashes independently verified by Language Ladder. |
| HL-B05 | Complete (#9663) | Remove Marathi's duplicate practice labels and Unicode bookmark warnings. | Stable recap labels, bookmark-safe Devanagari, natural page bottoms, and explicit static-font shapes make the forced six-chapter build warning-free. |
| HL-B06 | Complete (#9669) | Publish Gujarati Chapter 6 from its two canonical lessons rather than hand-copying another book chapter. | Both schema-v2 lessons now generate the PDF chapter from the same source hashes independently verified by Language Ladder. |
| HL-B07 | Complete (#9675) | Remove Gujarati's missing punctuation glyphs and LaTeX layout/bookmark warnings. | Canonical recap labels, main-font punctuation, bookmark-safe Gujarati, natural page bottoms, and explicit static-font shapes make the forced six-chapter build warning-free. |
| HL-B08 | Complete (#9680) | Publish Punjabi Chapter 6 from its two canonical lessons rather than hand-copying another book chapter. | Both schema-v2 lessons now generate the PDF chapter from the same source hashes independently verified by Language Ladder. |
| HL-B09 | Complete (#9683) | Remove Punjabi's LaTeX layout, duplicate-label, font-shape, and Unicode bookmark warnings. | Stable recap labels, bookmark-safe Gurmukhi, natural page bottoms, explicit static-font shapes, and a shorter running title make the forced six-chapter build warning-free. |
| HL-B10 | Complete (#9690) | Publish Sanskrit Chapter 6 from its three canonical lessons rather than hand-copying another book chapter. | All three schema-v2 lessons now generate the PDF chapter from the same source hashes independently verified by Language Ladder. |
| HL-B11 | Complete (#9698) | Remove Sanskrit's LaTeX layout, duplicate-label, font-shape, and Unicode bookmark warnings. | Stable recap labels, bookmark-safe Devanagari, natural page bottoms, explicit static-font shapes, and shorter running titles make the forced six-chapter build warning-free. |
| HL-B12 | Complete (#9705) | Publish Bengali Chapter 6 from its canonical lesson rather than hand-copying another book chapter. | The schema-v2 lesson now generates the PDF chapter from the same source hash independently verified by Language Ladder. |
| HL-B13 | Complete (#9711) | Remove Bengali's missing glyphs and LaTeX layout/bookmark warnings. | Main-font punctuation, stable recap labels, bookmark-safe Bengali, natural page bottoms, explicit static-font shapes, and a breakable long title make the forced six-chapter build warning-free. |
| HL-I01 | Complete (#9715) | Reduce unified all-books workflow setup time without splitting the single publication bundle. | A focused, preflighted XeLaTeX dependency closure replaces `texlive-full`; the unchanged job still builds all 20 books, verifies one bundle, and publishes that bundle from `main`. |
| HL-I02 | Complete (#10094) | Refresh the human-language data package's vulnerable transitive development locks. | Nano ID 3.3.18 and PostCSS 8.5.26 clear GHSA-2v37-7h3g-55p8 and GHSA-fxqj-rqcc-2cmp; a clean install and `npm audit` report zero known vulnerabilities. |
| HL-I03 | Complete (#10092) | Derive the top-level track progress table from canonical curriculum and book-generation data. | A registry-ordered generated table reports canonical and mapped lesson counts plus authored/generated book progress for every track; CI fails byte-for-byte drift. |
| HL-I04 | Complete (#9908) | Restore exact-main full CI after `perl/wasm-module-encoder` exposed undeclared local test dependencies. | Its `cpanfile` and `Makefile.PL` declare every local package injected by `BUILD`; clean full-build metadata validation and focused Perl tests pass. |
| HL-I05 | Complete (#9910) | Make the repository's Lua bootstrap resilient to a temporary lua.org connection outage. | All CI platforms install pinned Lua 5.4.7 through an OS-specific cache or checksum-verified byte-identical Debian/Ubuntu source fallback without weakening the Windows MSVC ordering or silently skipping Lua tests. |
| HL-I06 | Complete (#10107) | Reconcile stale and reused backlog IDs with merged implementation evidence. | Every completed item names its merge PR, duplicate rows are removed, reused IDs are split into unique stable IDs, and remaining completion signals describe only unfinished work. |
| HL-B14 | Complete (#9728) | Publish Italian Chapters 2–17 from their canonical lessons rather than hand-copying sixteen book chapters. | Forty-nine schema-v2 lessons now generate sixteen chapters whose source hashes are independently verified against the Language Ladder corpus. |
| HL-B15 | Complete (#9735) | Remove Italian's LaTeX layout and Unicode bookmark warnings. | The forced 104-page build now has zero missing glyphs, overfull or underfull boxes, duplicate destinations, Hyperref warnings, or LaTeX warnings. |
| HL-B16 | Complete (#9744) | Publish Portuguese Chapters 2–17 from their canonical lessons rather than hand-copying sixteen book chapters. | Fifty schema-v2 lessons now generate sixteen chapters whose source hashes are independently verified against the Language Ladder corpus. |
| HL-B17 | Complete (#9748) | Remove Portuguese's LaTeX layout warnings. | The forced 105-page build now has zero missing glyphs, overfull or underfull boxes, duplicate destinations, Hyperref warnings, or LaTeX warnings. |
| HL-B18 | Complete (#9752) | Publish French Chapters 17–23 from their canonical lessons rather than hand-copying seven book chapters. | Nine schema-v2 lessons now generate seven chapters whose source hashes are independently verified against the Language Ladder corpus. |
| HL-B19 | Complete (#9761) | Remove French's LaTeX layout and Unicode bookmark warnings. | The forced 98-page build now has zero missing glyphs, overfull or underfull boxes, duplicate destinations, Hyperref warnings, or LaTeX warnings. |
| HL-B20 | Complete (#9765) | Publish German Chapters 17–23 from their canonical lessons rather than hand-copying seven book chapters. | Ten schema-v2 lessons now generate seven chapters whose source hashes are independently verified against the Language Ladder corpus. |
| HL-B21 | Complete (#9779) | Remove German's LaTeX layout and Unicode bookmark warnings. | The forced 104-page build now has zero missing glyphs, overfull or underfull boxes, duplicate destinations, Hyperref warnings, or LaTeX warnings. |
| HL-B22 | Complete (#9803) | Publish Telugu Chapters 6–31 from their canonical lessons rather than hand-copying twenty-six book chapters. | Thirty schema-v2 lessons now generate twenty-six chapters whose source hashes are independently verified against the Language Ladder corpus. |
| HL-B23 | Complete (#9815) | Remove Telugu's LaTeX layout, duplicate-label, bookmark, and font warnings. | The forced 95-page build now has zero missing glyphs, overfull or underfull boxes, duplicate destinations, Hyperref warnings, LaTeX warnings, or font warnings. |
| HL-B24 | Complete (#9823) | Publish Kannada Chapters 6–31 from their canonical lessons rather than hand-copying twenty-six book chapters. | Thirty schema-v2 lessons now generate twenty-six chapters whose source hashes are independently verified against the Language Ladder corpus. |
| HL-B25 | Complete (#9828) | Remove Kannada's LaTeX layout, duplicate-label, bookmark, and font warnings. | The forced 96-page build now has zero missing glyphs, overfull or underfull boxes, duplicate destinations, Hyperref warnings, LaTeX warnings, or font warnings. |
| HL-B26 | Complete (#9838) | Publish Malayalam Chapters 6–31 from their canonical lessons rather than hand-copying twenty-six book chapters. | Thirty-three schema-v2 lessons now generate twenty-six chapters whose source hashes are independently verified against the Language Ladder corpus. |
| HL-B27 | Complete (#9844) | Remove Malayalam's LaTeX layout, duplicate-label, bookmark, font, and header-only-verso warnings. | The forced 107-page build now has zero missing glyphs, overfull or underfull boxes, duplicate destinations, Hyperref warnings, LaTeX warnings, or font warnings; intentionally empty versos are truly empty. |
| HL-B28 | Complete (#9854) | Publish Arabic Chapters 3–27 and their writing companions from canonical lessons rather than hand-copying another twenty-five book chapters. | Forty-five schema-v2 lessons now generate twenty-five chapters whose source hashes are independently verified against the Language Ladder corpus. |
| HL-B29 | Complete (#9861) | Remove Arabic's LaTeX layout, duplicate-label, bookmark, font, and header-only-verso warnings. | The forced 104-page build now has zero missing glyphs, overfull or underfull boxes, duplicate destinations, Hyperref warnings, LaTeX warnings, or font warnings; intentionally empty versos are truly empty. |
| HL-B30 | Complete (#9868) | Publish Hindi Chapters 6–33 and its writing companions from canonical lessons rather than hand-copying another twenty-eight book chapters. | Fifty-one lessons now use schema v2; forty later lessons generate twenty-eight chapters whose source hashes are independently verified against Language Ladder, while eleven prerequisite-ordered writing companions remain inside the gentle hand-authored opening chapters. |
| HL-B31 | Complete (#9871) | Remove Hindi's LaTeX layout, duplicate-label, bookmark, font, and header-only-verso warnings. | The forced 114-page build now has zero missing glyphs, overfull or underfull boxes, duplicate destinations, Hyperref warnings, LaTeX warnings, or font warnings; intentionally blank chapter versos are truly empty. |
| HL-B32 | Complete (#9875) | Publish Tamil Chapters 6–31 and its writing companions from canonical lessons rather than hand-copying another twenty-six book chapters. | Fifty-one lessons now use schema v2; forty-three later lessons generate twenty-six chapters whose source hashes are independently verified against Language Ladder, while eight prerequisite-ordered writing companions remain inside the gentle hand-authored opening chapter. |
| HL-B33 | Complete (#9880) | Remove Tamil's LaTeX layout, duplicate-label, bookmark, font, and header-only-verso warnings. | The forced 117-page build now has zero missing glyphs, overfull or underfull boxes, duplicate destinations, Hyperref warnings, LaTeX warnings, or font warnings; intentionally blank chapter versos are truly empty. |
| HL-B34 | Complete (#9883) | Publish Latin Chapters 2–36 from canonical lessons rather than hand-copying another thirty-five book chapters. | All 53 Latin lessons now use schema v2; Chapters 2–36 are generated with source hashes independently verified against Language Ladder. |
| HL-B35 | Complete (#9885, repaired by #9887) | Remove Latin's remaining LaTeX layout, font, and header-only-verso warnings. | The forced 115-page build now has zero missing glyphs, overfull or underfull boxes, duplicate destinations, Hyperref warnings, LaTeX warnings, or font warnings; intentionally blank chapter versos are truly empty. |
| HL-B36 | Complete (#9890) | Publish Spanish Chapters 19–33 from canonical lessons rather than hand-copying another fifteen book chapters. | The Spanish PDF now includes all 33 canonical chapters; all 21 later lessons use schema v2 and generate fifteen chapters from the same content consumed by Language Ladder. |
| HL-B37 | Complete (#9891) | Remove Spanish's remaining legacy LaTeX layout, bookmark, font, and header-only-verso warnings. | The forced 214-page build now has zero missing glyphs, overfull or underfull boxes, duplicate destinations, Hyperref warnings, LaTeX warnings, or font warnings; all 19 intentionally blank physical pages are truly empty. |
| HL-P01 | Complete (#9893) | Make the unified Human Languages Books result a protected merge gate, or add an equivalent gate that auto-merge must await. | Every pull request now receives a stable books gate; relevant changes run the one all-books build, unrelated changes receive a checked fast-path, and pull-request and push contexts remain distinct. |
| HL-M01 | Complete (#9894) | Add per-track spine realization maps and language-specific extension nodes. | All 20 tracks have validated repeated-segment local paths, explicit omissions/relocations, typed extensions, and a pure prerequisite-safe frontier planner. |
| HL-M10 | Complete (#9896) | Replace Learn's global concept cursor with per-language frontier progression and focused-before-mixed eligibility. | Stable per-language completed prefixes drive each next lesson; wrong focused answers cannot advance; only independently passed, visually distinguishable lessons enter mixed review. |
| HL-M02 | Queued | Extend Telugu's roadmap and authoritative session map through canonical Chapter 31. | The roadmap narrative stops at Chapter 6 and the session map at Chapter 5 even though prerequisite-ordered lessons continue through Chapter 31; every canonical lesson needs a scheduled place, and the map must explicitly split or justify Chapter 20's numbers-and-weather topic collision. |
| HL-M03 | Queued | Extend Kannada's roadmap and authoritative session map through canonical Chapter 31. | The roadmap narrative stops at Chapter 6 and the session map at Chapter 5; every canonical lesson needs a scheduled place, and Chapter 20's unrelated numbers/weather pairing must be split or explicitly justified. |
| HL-M04 | Queued | Extend Malayalam's roadmap and authoritative session map through canonical Chapter 31. | The roadmap narrative stops at Chapter 6 and the session map at Chapter 5 even though prerequisite-ordered lessons continue through Chapter 31; every canonical lesson, including the four new support steps, needs a scheduled place. |
| HL-M05 | Queued | Reconcile Arabic's roadmap and authoritative session map with canonical Chapters 1–27 and the sixteen-step writing sequence. | The roadmap details only Chapters 1–4 and still calls Chapter 5+ planned; the session map stops at Chapter 2 even though prerequisite-ordered canonical lessons continue through Chapter 27. |
| HL-M06 | Queued | Reconcile Hindi's roadmap and authoritative session map with canonical Chapters 1–33 and the eleven-step writing sequence. | The roadmap details only Chapters 1–6 and still calls Chapter 6 planned; the session map stops at Chapter 5 even though prerequisite-ordered canonical lessons continue through Chapter 33. |
| HL-M07 | Queued | Reconcile Tamil's roadmap and authoritative session map with canonical Chapters 1–31 and the eight-step writing sequence. | The roadmap details only Chapters 1–6 and still calls Chapter 7+ planned; the session map stops at Chapter 5 even though prerequisite-ordered canonical lessons continue through Chapter 31. |
| HL-M08 | Queued | Reconcile Latin's roadmap and authoritative session map with canonical Chapters 1–36. | Both files stop at Chapter 1 and describe Chapter 2+ as planned even though prerequisite-ordered canonical lessons continue through Chapter 36. |
| HL-M09 | Queued | Reconcile Spanish's roadmap and authoritative session map with canonical Chapters 1–33 and the new support steps. | The roadmap stops at Chapter 18 and calls Chapter 19 next, while the session map stops at Chapter 3; both lag the prerequisite-ordered canonical curriculum. |
| HL-T01 | Complete (#9904) | Complete session maps and pronunciation references for Persian and Urdu. | Both five-lesson prefixes now have authoritative N+1/N+3/N+7/N+15 ledgers and sound-id-keyed references; the Urdu guide explicitly preserves the Naskh fallback debt. |
| HL-U01 | Complete (#10104) | Vendor and verify an appropriately licensed static Nastaliq font for normal Urdu presentation. | Official Noto Nastaliq Urdu 4.000 Regular and Bold static files are hash-pinned, licensed under OFL-1.1, and used by the book and every target-form surface in Language Ladder; Naskh remains only the app's fallback. |

## P2 — corpus growth

| ID | Status | Work item | Completion signal |
|---|---|---|---|
| HL-E01 | Complete (#9906) | Author Persian and Urdu Chapter 3 through the rest of `SPINE-EXCHANGE-NAMES`. | Each track gains prerequisite-safe, schema-v2 micro-lessons for the name question, its formality distinction, and a meeting response; realization maps, objective activities, generated book chapters, and Language Ladder consume the same AST. |
| HL-E02 | Complete (#9913) | Author Persian and Urdu Chapter 4 through `SPINE-CHECK-WELLBEING` and reconcile the older identity-first roadmap. | Both tracks gain a gentle wellbeing exchange before identity grammar, with language-specific register/script extensions, exact review ledgers, objective practice, and generated book chapters from the canonical AST. |
| HL-E03 | Complete (#9914) | Author Persian and Urdu Chapter 5 through `SPINE-TAKE-LEAVE`. | Both tracks gain prerequisite-safe micro-lessons for ending a short respectful interaction, local script/register/grammar/etymology extensions, objective practice, exact review ledgers, and generated book chapters from the canonical AST. |

- Expand every track toward B1 using the gap report to choose the next missing
  can-do, skill, mode, register, or realization.
- Add controlled dialogues and micro-stories whose tokens are validated against
  prior knowledge.
- Add provenance-labelled listening and dictation activities from the same
  canonical lesson blocks.

## Findings from HL-S01

- Spanish Chapters 1–3 contain 24 schema-v2 lessons after three overlong
  explanations were split into prerequisite-ordered support lessons for noun
  gender, the Latin *qu-* question family, and the origin of *usted*.
- The resulting snapshot has 976 lessons, 481 duration violations, and 40
  later-chapter prerequisite roots: four and two fewer respectively than the
  HL-V01 baseline, with the remaining debt still explicit.
- Every migrated lesson computes below 300 seconds; the tightest current budget
  is *buenos días* at 296 seconds, which should be watched during copy edits.
- Schema v2 now validates canonical spine mapping, unique local sequence,
  typed body blocks, explicit coverage metadata, same-language prerequisites,
  and transitive knowledge closure. Block-boundary prompt/answer knowledge
  declarations remain a later refinement; this slice does not claim them.

## Findings from HL-S02

- Spanish Chapters 4–6 now contain 27 schema-v2 lessons: the 25 existing
  vocabulary, grammar, practice, and writing lessons plus two short repair
  lessons that teach **y** and **café** before later dialogue asks learners to
  use them. Spanish now has 51 schema-v2 lessons and 77 legacy lessons.
- Every migrated lesson has a unique sequence from 250 through 510, begins with
  a typed warm-up, ends with typed recall, closes all declared knowledge over
  its transitive prerequisites, and remains below 300 effective seconds.
- Forward references to later material such as *sí*, *un poco*, *ojalá*, and
  untaught future farewells were removed from production prompts. The surviving
  dialogues and script exercises require only what the learner already knows.
- The editorial audit also caught undeclared prompt tokens that lesson-level
  atom closure cannot see. HL-V02 records block-level prompt closure as the
  next validation enhancement before schema migration expands beyond this
  carefully audited slice.
- The typed-block parser now recognizes `Script` sections explicitly, so the
  accent, eñe, and inverted-question lessons remain first-class canonical app
  content rather than falling into an unknown presentation bucket.
- This tranche deliberately does not replace the handwritten LaTeX chapters.
  HL-G03 is next and will generate Chapters 4–6 only after this canonical
  content contract has merged.

## Findings from HL-G01

- Spanish Chapter 1 is generated deterministically from seven canonical
  schema-v2 lessons in authored `sequence` order; the 18-book source is now 122
  rendered pages with no generated-chapter overfull boxes.
- The generated chapter and Language Ladder independently combine the same
  per-lesson FNV-1a fingerprints. The app exposes `book synced` only when its
  loaded Chapter 1 lesson AST matches the committed manifest.
- The unified book job now fails when generated TeX or the hash manifest is
  missing or stale. The fingerprint is a deterministic drift signal, not a
  cryptographic integrity claim.
- At the end of HL-G01, Chapter 1 was the first one-source slice and Chapters
  2–18 remained handwritten. That finding deliberately scoped HL-G02 to the
  already-schema-v2 Chapters 2–3 rather than skipping validation to generate
  later chapters.

## Findings from HL-G02

- All 24 schema-v2 Spanish lessons in Chapters 1–3 now generate their three
  LaTeX chapters and independently match Language Ladder's loaded AST. Chapter
  2 combines five lesson hashes; Chapter 3 combines twelve.
- The expanded canonical content produces a 138-page book. Rendered checks of
  both chapter openers, grammar and etymology boxes, nested emphasis, practice
  lists, and wrap-up recall found no generated-chapter overfull box or Hyperref
  warning.
- The renderer now handles nested bold-within-italic Markdown, wraps practice
  lists ragged-right, and keeps math arrows out of bookmark/running-header
  strings. Those fixes apply to every later generated chapter.
- The next learner-visible promise is the sub-five-minute cap. Russian is the
  smallest complete existing track with measurable debt: nine violations, of
  which five are computed at 312–405 seconds and four only need honest declared
  budgets below the cap. HL-D01A is therefore the next bounded tranche.

## Findings from HL-G03

- All 51 schema-v2 Spanish lessons in Chapters 1–6 now generate the same six
  LaTeX chapters whose source hashes Language Ladder recomputes from its loaded
  AST. The per-chapter lesson counts are 7, 5, 12, 13, 7, and 7.
- The shared renderer now preserves valid Markdown tables as width-aware LaTeX
  tables and maps the approximation sign safely. This keeps register contrasts,
  question families, farewell choices, and verb forms structured in both app
  and book instead of flattening them into pipe-delimited prose.
- The forced XeLaTeX build produces 158 pages. Every rendered page in the
  Chapter 4–6 span was checked for clipping, collisions, broken diacritics,
  malformed tables, and accidental blank pages; those generated chapters have
  no missing glyph, overfull/underfull box, or Hyperref warning.
- Remaining Spanish PDF warnings come from legacy Chapters 7–18 and stay
  explicit in HL-B37. HL-V02 is next because the HL-S02 editorial audit showed
  that lesson-level atom closure alone cannot detect undeclared target-language
  tokens inside learner production and recall prompts.

## Findings from HL-V02

- All 51 schema-v2 Spanish lessons now declare introductions and assessments at
  every one of their typed body boundaries. Production and recall blocks require
  non-empty assessment declarations, and all other blocks retain explicit empty
  lists when they change no knowledge state.
- Validation follows rendered order: assessed atoms must already belong to the
  lesson's transitive prerequisite frontier or an earlier block, block
  introductions must account exactly for `introduces.knowledge`, and assessed
  atoms must be declared in `practises.knowledge`.
- The editorial migration removed premature *muy bien*, *¿y usted?*, *el gusto
  es mío*, *ojalá*, and next-chapter question-form production. It promoted *te
  llamas* and *gusto* to explicit atoms and completed grammar, script,
  etymology, and phrase practice declarations exposed by the boundary audit.
- Block metadata changes the shared canonical hashes but not learner copy. The
  six generated Spanish chapters omit the directives, and Language Ladder now
  explicitly filters them from its lightweight Markdown view.
- Individual prompt/answer/variant/feedback records remain prose rather than a
  compiled activity schema. HL-V03 records that next validation layer; HL-B04
  is the next bounded learner-visible publication gap.

## Findings from HL-D01A

- Russian now has zero duration violations. The repository snapshot contains
  980 lessons and 472 violations overall, down from 481 before this tranche;
  unknown prerequisites remain at zero.
- Four lessons already computed below five minutes and only needed their
  declared estimates corrected. The five genuinely long lessons were shortened
  through de-duplication or split into four prerequisite-ordered support and
  practice lessons.
- The cross-language formality comparison, naming-as-action comparison, person
  shapes, and precise zero-copula explanation remain in the canonical corpus.
  The tightest changed lesson is `RU-C01-privet` at 293 computed seconds; every
  other changed or new lesson has a larger buffer.
- Marathi's eight violations are the smallest remaining track-sized set, ahead
  of Gujarati's nine and Punjabi's and Sanskrit's ten each. HL-D01B is therefore
  the next bounded duration tranche after this PR merges.

## Findings from HL-D01B

- Marathi now has zero duration violations. The repository snapshot contains
  981 lessons and 464 violations overall, down from 472 before this tranche;
  unknown prerequisites remain at zero.
- Seven lessons already computed between 126 and 171 seconds and only needed
  honest four-minute declared budgets. The one genuinely long lesson computed
  at 321 seconds.
- That counting lesson is now a 163-second core followed by a 240-second
  etymology lesson. The analogy and retention explanations remain complete and
  prerequisite-ordered in the canonical corpus consumed by Language Ladder.
- The audit also made a publication boundary explicit: Marathi Chapter 6 has
  canonical lessons but is not in the current five-chapter PDF. HL-B04 records
  the one-source migration and generation work instead of adding another manual
  copy.
- A forced build of the unchanged five-chapter book still succeeds with zero
  overfull boxes, but exposes four duplicate practice labels, 32 Unicode
  bookmark warnings, and two underfull boxes. HL-B05 records that pre-existing
  publication hygiene debt separately from the lesson remediation.
- Gujarati's nine violations are now the smallest remaining track-sized set,
  ahead of Punjabi's and Sanskrit's ten each. HL-D01C is therefore next after
  this PR merges.

## Findings from HL-B04

- Marathi Chapter 6 now comes from its two canonical schema-v2 lessons. The
  generator manifest and Language Ladder independently combine the same ordered
  lesson hashes, so an app/book edit can no longer drift silently.
- The strict migration adds the first shared `SPINE-COUNT-ONE-TO-FIVE` can-do
  node. Marathi keeps its local Devanagari, pronunciation, and historical
  extensions while later number chapters can reuse the communicative spine.
- Generated non-Latin chapters exposed a reusable pipeline need: each target may
  name a Unicode Script property and its book's existing LaTeX font command.
  Devanagari runs are wrapped automatically; Latin prose and bookmark-safe
  romanization remain in the main font.
- The deterministic report still measures 1,065 lessons, 20 books, zero duration
  violations, and zero unknown prerequisites. Publishing this chapter reduces
  the lesson-to-book chapter gap from 257 to 256 and moves Marathi from legacy
  to mixed schema status.
- The forced 31-page XeLaTeX build has zero missing glyphs and zero overfull
  boxes. The generated pages preserve Devanagari shaping, width-aware tables,
  box titles, and recall prompts without clipping; one new underfull page joins
  the older warning debt recorded in HL-B05.
- HL-B05 records the bounded cleanup for the older handwritten chapters so its
  presentation warnings are not confused with generated-chapter drift.

## Findings from HL-B05

- Each handwritten recap now uses its canonical lesson id (`MR-C01-practice`
  through `MR-C05-practice`) instead of five copies of `lesson:practice`.
- Hyperref's PDF-string fallback strips only the `\mr` / `\marathifont`
  presentation wrapper. The inspected outline keeps readable Devanagari and
  romanization in all handwritten section bookmarks, while the generated
  chapter keeps its intentionally Latin short titles.
- `\raggedbottom` lets pages around unbreakable lesson callouts end naturally
  instead of stretching vertical glue. Visual inspection of the three formerly
  underfull pages plus the final recall page found no clipping, collision, or
  awkward box spacing.
- The vendored static Devanagari file is now explicitly selected for regular,
  bold, italic, and bold-italic requests. This matches the old fallback's glyph
  appearance while avoiding misleading unavailable-shape warnings.
- The forced 31-page XeLaTeX build now reports zero package or LaTeX warnings,
  missing glyphs, overfull boxes, underfull boxes, and duplicate destinations.
  HL-B06 is next: publish Gujarati Chapter 6 through the same canonical pipeline.

## Findings from HL-B06

- Gujarati Chapter 6 now comes from its two canonical schema-v2 lessons. The
  generator manifest and Language Ladder independently combine the same ordered
  lesson hashes, so the app and downloadable book cannot drift silently.
- Both lessons realize `SPINE-COUNT-ONE-TO-FIVE` while retaining Gujarati's
  local headless-script clue, the *dvé → be* assimilation path, and the learned
  restoration of *r* in *traṇ*. Their effective 174- and 253-second boundaries
  remain below the strict five-minute ceiling.
- The shared Unicode generator wraps Gujarati runs with the book's existing
  `\gu` command, while authored romanization supplies readable section
  bookmarks. The final outline contains `ek be traṇ chār pā̃ch` and `be · traṇ`.
- The deterministic report now measures 1,065 lessons, 20 books, zero duration
  violations, zero unknown prerequisites, and 255 lesson chapters without book
  chapters. Gujarati joins Spanish and Marathi as the third mixed-schema track.
- The forced letter-size XeLaTeX build is 27 pages. Visual inspection of every
  generated page found shaped Gujarati, width-aware tables, intact callouts, and
  no clipping; tightening the final four-question recall kept it on one page.
- The generated chapter adds no new missing-glyph, overfull, underfull,
  duplicate-label, bookmark, or font warnings. HL-B07 remains the bounded cleanup
  for warnings already present in the five handwritten chapters.

## Findings from HL-B07

- Each handwritten recap now uses its canonical lesson id (`GU-C01-practice`
  through `GU-C05-practice`) instead of five copies of `lesson:practice`.
- Latin commas and the ellipsis now sit outside `\gu`, so the Gujarati-only
  static font is asked to shape only Gujarati characters while the visible
  punctuation remains unchanged.
- Hyperref's PDF-string fallback strips only the `\gu` / `\gujaratifont`
  presentation wrapper. The inspected outline keeps readable Gujarati and
  romanization across all handwritten sections, including `મારું નામ…છે`, while
  generated Chapter 6 keeps its intentionally Latin short titles.
- `\raggedbottom` lets pages around unbreakable lesson callouts end naturally.
  Visual inspection of all four formerly underfull pages plus the repaired
  Chapter 5 recap found no clipping, collision, or awkward box spacing.
- The vendored static Gujarati file is now explicitly selected for regular,
  bold, italic, and bold-italic requests. This preserves the prior glyph
  appearance without reporting unavailable font shapes.
- Rephrasing the three copula forms gives TeX natural breakpoints without
  changing the recap's meaning. The forced 27-page XeLaTeX build now reports
  zero package or LaTeX warnings, missing glyphs, overfull boxes, underfull
  boxes, and duplicate destinations. HL-B08 is next: publish Punjabi Chapter 6
  through the same canonical pipeline.

## Findings from HL-B08

- Punjabi Chapter 6 now comes from its two canonical schema-v2 lessons. The
  generator manifest and Language Ladder independently combine the same ordered
  lesson hashes, so the app and downloadable book cannot drift silently.
- Both lessons realize `SPINE-COUNT-ONE-TO-FIVE` while retaining Punjabi's
  Gurmukhi top-line clue, addak/tippi distinction, Chapter 5 five-rivers
  callback, and the independent Punjabi/Persian paths to *panj*.
- The strict knowledge gate confirms every prompt against its block and
  prerequisite frontier. The corpus report now has 1,065 lessons, 20 books,
  zero duration violations, zero unknown prerequisites, and 254 lesson chapters
  without book chapters. Punjabi joins Spanish, Marathi, and Gujarati as the
  fourth mixed-schema track.
- The forced letter-size XeLaTeX build is 30 pages. Visual inspection of all
  four generated pages found shaped Gurmukhi, width-aware comparison tables,
  intact callouts, a complete spaced-recall close, and no clipping. The PDF
  outline retains the romanized `ikk do tinn chār panj` and `panj · panj`
  section bookmarks.
- The generated chapter adds no new missing-glyph, overfull, underfull,
  duplicate-label, bookmark, or font warning. The audit did expose three
  pre-existing font-shape warnings omitted from the earlier inventory; HL-B09
  now includes those alongside the handwritten chapters' other warning debt.

## Findings from HL-B09

- Each handwritten recap now uses its canonical lesson id (`PA-C01-practice`
  through `PA-C05-practice`) instead of five copies of `lesson:practice`.
- Hyperref's PDF-string fallback strips only the `\pa` / `\punjabifont`
  presentation wrapper. The inspected outline retains readable Gurmukhi and
  romanization across every handwritten section, while generated Chapter 6
  keeps its intentional romanized short titles.
- `\raggedbottom` lets pages around unbreakable lesson callouts end naturally.
  Visual inspection of all four formerly underfull pages found shaped text,
  intact boxes, and no clipping, collision, or awkward vertical stretching.
- The vendored static Gurmukhi file is now explicitly selected for regular,
  bold, italic, and bold-italic requests. This preserves the existing glyph
  appearance without reporting unavailable font shapes.
- The `ਤੂੰ` / `ਤੁਸੀਂ` section now has a natural-language separator and a shorter
  running title, removing the lone overfull header without changing the lesson.
  The forced 30-page XeLaTeX build now reports zero package or LaTeX warnings,
  missing glyphs, overfull boxes, underfull boxes, and duplicate destinations.
  HL-B10 is next: publish Sanskrit Chapter 6 through the canonical pipeline.

## Findings from HL-B10

- Sanskrit Chapter 6 now comes from its three canonical schema-v2 lessons. The
  generator manifest and Language Ladder independently combine the same ordered
  lesson hashes, so the app and downloadable book cannot drift silently.
- All three lessons realize `SPINE-COUNT-ONE-TO-FIVE`, then extend it with
  Sanskrit's dual and gendered numeral forms, the daughter languages' neuter
  inheritance, PIE sound-law outcomes, and qualified lexical histories.
- The canonical prose now says Sanskrit preserves the Old Indo-Aryan forms
  behind the daughter languages and labels *four*'s `f-` as the usual analogy
  explanation rather than presenting either relationship too absolutely.
- The strict knowledge gate confirms every prompt against its block and
  prerequisite frontier. The corpus report remains at 1,065 lessons and 20
  books, with zero duration violations, zero unknown prerequisites, and 253
  lesson chapters without book chapters. Sanskrit becomes the fifth
  mixed-schema track.
- The forced letter-size XeLaTeX build is 35 pages. Visual inspection of all
  five generated pages found shaped Devanagari, width-aware grammar and
  sound-law tables, intact callouts and recall prompts, and no clipping. The PDF
  outline retains all three romanized section bookmarks.
- A text mapping renders the PIE superscript `ʷ` without a missing glyph. The
  generated chapter adds one visually benign underfull-page warning but no new
  overfull, duplicate-label, bookmark, or font warning. The full build's three
  font-shape warnings and seven underfull warnings are now recorded accurately
  in HL-B11 alongside the older handwritten warning debt.

## Findings from HL-B11

- Sanskrit's five authored recap anchors now use stable chapter-qualified ids
  (`SA-C01-practice` through `SA-C05-practice`) instead of five copies of
  `lesson:practice`.
- Devanagari is retained in the PDF outline while the presentation-only font
  switch is suppressed there. The vendored static font is selected explicitly
  for regular, bold, italic, and bold-italic requests, preserving the existing
  glyph appearance without unavailable-shape substitutions.
- Short pages now end naturally around kept-together lesson callouts. Concise
  running titles for the long “you,” “what,” and *karomi* sections remove the
  three overfull headings without removing lesson content. The forced 35-page
  XeLaTeX build reports zero package or LaTeX warnings, missing glyphs,
  overfull boxes, underfull boxes, and duplicate destinations. HL-B12 is next:
  publish Bengali Chapter 6 through the canonical pipeline.

## Findings from HL-B12

- Bengali Chapter 6 now comes from its canonical schema-v2 lesson. The
  generator manifest and Language Ladder independently combine the same source
  hash, so the app and downloadable book cannot drift silently.
- The lesson realizes `SPINE-COUNT-ONE-TO-FIVE` in a 290-second boundary and
  extends it with Bengali script, chandrabindu nasalization, the conservative
  vowel in *dui*, the everyday numeral's simplified *dv-* cluster, and the
  qualified Assamese/Odia/Nepali comparison.
- HL-I01 records publication latency observed after HL-B11: content drift and
  Bengali's warning debt remain learner-visible priorities ahead of optimizing
  a successful but slow single-job TeX setup.
- The strict data suite passes all 80 tests; Language Ladder passes all 287
  tests and its production build. The report remains at 1,065 lessons and 20
  books with zero duration violations and zero unknown prerequisites, while
  book drift falls to 252 lesson chapters and Bengali becomes the sixth
  mixed-schema track.
- The forced letter-size XeLaTeX build is 29 pages. Visual inspection of all
  three generated pages found shaped Bengali, width-aware numeral and etymology
  tables, an intact chandrabindu and recall box, and no clipping. The outline
  retains the authored `ek dui tin chār pā̃ch` bookmark.
- The generated chapter adds one visually benign underfull-page warning but no
  new missing glyph, overfull, duplicate-label, bookmark, or font warning. The
  full build's six missing glyphs, one overfull box, five underfull boxes, four
  duplicate recap labels, 27 Hyperref warnings, and three font-shape warnings
  are recorded accurately in HL-B13.

## Findings from HL-B13

- Bengali's five authored recap anchors now use stable chapter-qualified ids
  (`BN-C01-practice` through `BN-C05-practice`) instead of five copies of
  `lesson:practice`.
- Ellipsis, comma, and morpheme-boundary hyphen punctuation now stays in the
  Latin main font instead of being asked of the Bengali-only static font.
  Bengali remains visible in the PDF outline while the presentation-only font
  command is suppressed there, and all requested font shapes resolve to the
  vendored static file.
- Short pages end naturally around kept-together callouts, and the long
  “we'll meet again” title has a safe line-break point. The forced 29-page
  XeLaTeX build reports zero package or LaTeX warnings, missing glyphs,
  overfull boxes, underfull boxes, and duplicate destinations. HL-I01 is next:
  reduce successful unified-publication setup latency without splitting the
  single all-books job.
- Visual regression inspection covered nine formerly affected pages: Bengali
  shaping and main-font punctuation remain intact, the bilingual farewell title
  breaks cleanly, split callouts are unclipped, and short pages have deliberate
  natural bottoms. All 39 PDF outline entries remain readable, ending with the
  generated romanized Chapter 6 title.

## Findings from HL-I01

- Three successful unified-publication baselines spent 7:00, 8:09, and 8:02
  installing `texlive-full`; the most recent then built all 20 books in 1:41.
  Setup, not the single-job book loop, was the dominant and variable cost.
- The complete TeX source inventory uses the standard `book` class and eleven
  packages. Ubuntu's `texlive-xetex` dependency closure provides the LaTeX base,
  recommended, and extra collections containing those packages; the focused
  install adds `texlive-lang-arabic` for `bidi.sty`, `lmodern` for the named
  Latin Modern faces, `texlive-fonts-recommended` for Hyperref's `pzdr.tfm`,
  and `latexmk` as the build driver.
- All non-Latin faces are repository-vendored static fonts, so the system-wide
  Noto font collections are unrelated to the current builds. A fail-closed
  preflight now resolves the engine, driver, all eleven packages, the class,
  RTL support, and Latin Modern before compiling any book.
- The workflow remains one job with one setup, one dynamically discovered
  20-book loop, one verified artifact, and one `main`-only Pages publication.
  The exact merged-main run installed the focused toolchain in 87 seconds,
  built all books in 93 seconds, verified the bundle, and published Pages.
  HL-B14 and HL-B15 have since closed the Italian app/book drift and its measured
  presentation debt. HL-B16 is next: apply the same canonical generation path
  to Portuguese Chapters 2–17.

## Findings from HL-B14

- Italian Chapters 2–17 now comprise 49 strict schema-v2 micro-lessons with
  explicit shared-spine anchors, prerequisite-closed knowledge atoms, typed
  teaching blocks, and authored skill, mode, strand, register, variety, and
  duration contracts. Chapter 1 remains readable legacy content, so the track
  is intentionally mixed while one-source migration proceeds incrementally.
- All 49 migrated lessons remain below five minutes. `IT-C17-mano` is the
  tightest at 298 computed seconds; curriculum validation reports zero duration
  violations and zero unknown or misordered prerequisites.
- Sixteen deterministic generation targets now publish Chapters 2–17 from the
  same lesson AST loaded by Language Ladder. Their manifest covers all 49
  lessons, and app tests independently reproduce every chapter hash and lesson
  count. Repository-wide missing book chapters fall from 252 to 236.
- The generic renderer recognizes scoped “taken apart” headings and emits
  portable TeX for the scholarly symbols `↔`, `ṓ`, `₁`, and `ʰ`, preserving the
  app's precise Unicode while avoiding font-dependent gaps in generated PDFs.
- A forced XeLaTeX build produces a 104-page book with zero missing glyphs,
  duplicate destinations, or leaked generator metadata. All 104 rendered pages
  were inspected; the cover, three-page contents, chapter openings, callouts,
  dense tables, and final recall are unclipped, and the PDF outline contains
  Preface, pronunciation, and all seventeen chapter destinations.
- Four overfull boxes, ten underfull boxes, and the three pre-existing Chapter
  1 Hyperref warnings remain. HL-B15 is next and now records the expanded,
  measured clean-build debt rather than the old 13-page baseline.

## Findings from HL-B15

- The inline renderer now honors backslash escapes for Markdown punctuation, so
  an etymological reconstruction such as `**\*parabolāvit**` becomes one bold
  literal form rather than malformed nested emphasis. A focused regression test
  keeps the canonical app text and generated TeX aligned.
- Generated tables now begin without paragraph indentation. This removes the
  otherwise invisible 17-point width excess while preserving full-width,
  ragged-right columns for every generated language chapter.
- Italian's legacy Chapter 1 heading now supplies a bookmark-safe short title,
  and `\raggedbottom` makes deliberately short lesson pages explicit. Targeted
  canonical copy and table-cell edits remove the remaining horizontal layout
  warnings without dropping any vocabulary, grammar, or etymology.
- A forced XeLaTeX build produces 104 pages with zero missing glyphs, overfull
  or underfull boxes, duplicate destinations, Hyperref warnings, or LaTeX
  warnings. All pages were rendered and inspected with no clipping or
  collisions; the outline retains Preface, pronunciation, and Chapters 1–17,
  and no schema or source-hash metadata leaks into extracted text.
- HL-B16 is next: migrate Portuguese Chapters 2–17 to the same strict schema-v2
  lesson AST and publish them through the shared book/app generation path.

## Findings from HL-B16

- Portuguese Chapters 2–17 now comprise 50 strict schema-v2 micro-lessons with
  explicit shared-spine anchors, prerequisite-closed knowledge atoms, typed
  teaching blocks, and authored skill, mode, strand, register, variety, and
  duration contracts. Chapter 1 remains readable legacy content while the
  one-source migration proceeds incrementally.
- All 50 migrated lessons remain below five minutes: computed durations span
  141–298 seconds, with `PT-C17-mao` the tightest. Curriculum validation reports
  zero duration violations and zero unknown or misordered prerequisites.
- Sixteen deterministic generation targets now publish Chapters 2–17 from the
  same lesson AST loaded by Language Ladder. Their manifest covers all 50
  lessons, and app tests independently reproduce every chapter hash and lesson
  count. Repository-wide missing book chapters fall from 236 to 220.
- Chapter 4 preserves Arabic `حتى` beside transliterated *ḥattā*. Its generated
  run now uses the repository-vendored Noto Naskh Arabic font, preventing the
  Latin-only PDF path from silently dropping source-script evidence.
- A forced XeLaTeX build produces a 105-page book with zero missing glyphs,
  duplicate destinations, Hyperref warnings, LaTeX warnings, or leaked
  generator metadata. All pages were rendered and inspected; the outline
  retains Preface, pronunciation, and Chapters 1–17.
- Six overfull boxes and thirteen underfull boxes remain in the expanded book.
  HL-B17 is next and records this measured 105-page presentation debt rather
  than the old 13-page baseline.

## Findings from HL-B17

- Portuguese now uses `\raggedbottom`, making intentionally short micro-lesson
  pages explicit and removing eleven underfull vertical boxes without padding
  or stretching learner content.
- Six canonical lessons received small copy-flow edits: shorter resumable
  headings, clearer sentence boundaries, and two deliberate warm-up paragraph
  breaks. The same meaning, vocabulary, grammar, and etymology remain in both
  Language Ladder and the generated book.
- Regeneration updates the six affected chapter fingerprints, which Language
  Ladder independently reproduces from the canonical AST. All fifty migrated
  lessons remain below five minutes and prerequisite-closed.
- A forced XeLaTeX build produces 105 pages with zero missing glyphs, overfull
  or underfull boxes, duplicate destinations, Hyperref warnings, or LaTeX
  warnings. Every page was rendered and inspected; the outline retains Preface,
  pronunciation, and Chapters 1–17, and no generator metadata leaks into text.
- HL-B18 is next: close the smaller seven-chapter French app/book gap before its
  measured presentation-cleanup follow-up.

## Findings from HL-B18

- French Chapters 17–23 now comprise nine strict schema-v2 micro-lessons with
  explicit shared-spine anchors, prerequisite-closed knowledge atoms, typed
  teaching blocks, and authored skill, mode, strand, register, variety, and
  duration contracts. Chapters 1–16 remain readable legacy content while the
  one-source migration proceeds incrementally.
- All nine migrated lessons remain below five minutes: computed durations span
  194–287 seconds. Curriculum validation reports zero duration violations and
  zero unknown or misordered prerequisites.
- Seven deterministic generation targets now publish Chapters 17–23 from the
  same lesson AST loaded by Language Ladder. Their manifest covers all nine
  lessons, and app tests independently reproduce every chapter hash and lesson
  count. Repository-wide missing book chapters fall from 220 to 213.
- A forced XeLaTeX build produces a 98-page book with zero missing glyphs,
  duplicate destinations, LaTeX warnings, or leaked generator metadata. All
  pages were rendered and inspected; the outline retains Preface,
  pronunciation, and Chapters 1–23.
- The expanded book retains the exact pre-existing warning baseline: sixteen
  overfull boxes, nine underfull boxes, and six Hyperref warnings. HL-B19 is
  next and records cleanup against the full 98-page artifact.

## Findings from HL-B19

- French now uses `\raggedbottom`, making intentionally short micro-lesson
  pages explicit and removing nine underfull vertical boxes without padding or
  stretching learner content.
- Six concise optional section titles keep legacy running headers inside the
  text block, while two prose-only Chapter 12 titles provide clean PDF
  bookmarks without changing the visible mathematical arrows.
- Three internal source paths can now break naturally. Five dense legacy tables
  use a flexible final column, preserving every comparison while removing
  horizontal overflow, and one pronominal-verb explanation has clearer sentence
  boundaries for the same grammatical rule.
- A forced XeLaTeX build produces 98 pages with zero missing glyphs, overfull or
  underfull boxes, duplicate destinations, Hyperref warnings, or LaTeX warnings.
- HL-B20 is next: close the seven-chapter German app/book gap before its measured
  presentation-cleanup follow-up.

## Findings from HL-B20

- German Chapters 17–23 now comprise ten strict schema-v2 micro-lessons with
  explicit shared-spine anchors, prerequisite-closed knowledge atoms, typed
  teaching blocks, and authored skill, mode, strand, register, variety, and
  duration contracts. Their computed durations range from 164 to 262 seconds.
- The audit found that `Entschuldigung` occupied Chapter 19 while no Chapter 20
  lesson existed. A new Chapter 19 lesson now reviews `bitte` as “please” in
  **Wasser, bitte** using only Chapters 3 and 11; the unchanged apology content
  follows as prerequisite-dependent Chapter 20.
- Seven generated chapters carry deterministic hashes and lesson ids that
  Language Ladder independently reproduces from the canonical AST. The corpus
  grows to 1,066 lessons, repository-wide missing book chapters fall from 213
  to 207, and unknown prerequisites and duration violations remain at zero.
- A forced XeLaTeX build produces 104 pages with zero missing glyphs, duplicate
  destinations, LaTeX warnings, or leaked generator metadata. All pages were
  rendered and inspected; the outline retains Preface, pronunciation, and
  Chapters 1–23.
- The expanded warning baseline is eighteen overfull boxes, one underfull
  horizontal box, eleven underfull vertical boxes, and three Hyperref warnings.
  HL-B21 is next and records cleanup against the full 104-page artifact.

## Findings from HL-B21

- German now uses `\raggedbottom`, making intentionally short micro-lesson
  pages explicit and removing eleven underfull vertical boxes without adding
  filler or stretching learner content.
- Concise running titles, one prose-only bookmark, a breakable practice path,
  and three reflowed passages remove header, path, and paragraph overflow while
  preserving the same grammar and etymology explanations.
- Ten dense legacy tables now use responsive or explicitly bounded paragraph
  columns. Every register, vocabulary, conjugation, weekday, and word-origin
  comparison remains present and readable inside the text block.
- The two canonical copy edits keep the generated German chapters and Language
  Ladder hashes aligned: the `Kopf` recall reflows cleanly, while the shorter
  visible `Entschuldigung` heading leaves its complete “un-guilting” etymology
  in the lesson body.
- Full-page inspection found that straight ASCII quotes elsewhere in generated
  prose can become right-only quotation marks under German language rules.
  HL-G04 records a cross-book generator fix; it follows the larger missing-book
  gaps rather than expanding this focused layout tranche.
- A forced XeLaTeX build produces 104 pages with zero missing glyphs, overfull
  or underfull boxes, duplicate destinations, Hyperref warnings, or LaTeX
  warnings. All 104 rendered pages were inspected; the outline retains the
  Preface, pronunciation reference, and Chapters 1–23. HL-B22 is next: publish
  Telugu Chapters 6–31 from canonical lessons.

## Findings from HL-B22

- All thirty canonical Telugu lessons after Chapter 5 now use schema v2 with
  explicit spine nodes, prerequisite-safe sequences, honest sub-five-minute
  duration budgets, typed knowledge boundaries, skills, modes, strands,
  register, and variety metadata. The first thirty lessons remain schema v1,
  so the track is intentionally mixed while migration proceeds incrementally.
- Twenty-six generated chapters carry deterministic hashes and lesson ids that
  Language Ladder independently reproduces from the canonical AST. Telugu book
  coverage is now 100%, and the app and downloadable book share the same source
  through Chapter 31.
- The shared generator now supports named multi-script font sets. Telugu's
  comparison passages can render Telugu, Tamil, Kannada, Malayalam, Devanagari,
  and Arabic-script examples without hand-authored LaTeX or missing glyphs.
- Chapter 20 currently combines the numbers 11–20 with an unrelated weather
  lesson. HL-M02 records the need for the authoritative roadmap and session map
  to split that progression or explain the grouping explicitly.
- A forced XeLaTeX build produces 95 pages with zero missing glyphs or leaked
  generator metadata. All pages were rendered and inspected; the outline keeps
  Preface, the script reference, and Chapters 1–31 in order.
- The expanded warning baseline is eleven overfull boxes, nine underfull
  vertical boxes, four duplicate practice labels, 104 Hyperref warnings, and
  nine font warnings. No visual clipping was found; HL-B23 is next and records
  cleanup against the complete 95-page artifact.
- The repository corpus contains 1,066 lessons with zero unknown prerequisites
  and zero duration violations.
- A clean data-package install surfaced the moderate, development-only
  GHSA-fxqj-rqcc-2cmp advisory through Vitest, Vite, and PostCSS 8.5.19. A
  non-breaking 8.5.25 resolution is available; HL-I02 records the lockfile
  maintenance behind the remaining reader-facing book and app gaps.

## Findings from HL-B23

- Explicit regular, bold, italic, and bold-italic faces for every vendored
  comparison font remove nine substitution warnings while keeping Telugu,
  Tamil, Kannada, Malayalam, Devanagari, and Arabic-script examples available.
- Bookmark-safe definitions preserve the visible script while removing font
  presentation commands from PDF strings. All 104 Hyperref warnings disappear,
  and the outline retains Preface, the script reference, and Chapters 1–31.
- Five legacy practice sections now have chapter-specific labels. `\raggedbottom`
  makes natural micro-lesson page endings explicit, removing four duplicate
  destinations and nine underfull vertical boxes without adding filler.
- Concise visible headings, one responsive table, a three-part month list, and
  a shorter Chapter 20 title remove eleven overfull lines while preserving every
  vocabulary item, grammar explanation, comparison, and etymology in the body.
- Full-page review caught a long Section 4.4 running header touching its page
  number even after the build log was clean. A prose-only running title fixes
  that collision while retaining the complete Telugu heading in the lesson.
- A forced XeLaTeX build produces 95 pages with zero missing glyphs, overfull or
  underfull boxes, duplicate destinations, Hyperref warnings, LaTeX warnings,
  or font warnings. All pages were rendered and inspected; metadata, 33
  top-level bookmarks, 93 total outline entries, and generator-leak checks pass.
- HL-B24 is next: publish Kannada Chapters 6–31 from canonical lessons.

## Findings from HL-B24

- All thirty canonical Kannada lessons after Chapter 5 now use schema v2 with
  explicit spine nodes, prerequisite-safe sequences, honest sub-five-minute
  duration budgets, typed knowledge boundaries, skills, modes, strands,
  register, and variety metadata. The first thirty lessons remain schema v1,
  so the track is intentionally mixed while migration proceeds incrementally.
- Twenty-six generated chapters carry deterministic hashes and lesson ids that
  Language Ladder independently reproduces from the canonical AST. Kannada
  book coverage is now 100%, and the app and downloadable book share the same
  source through Chapter 31.
- A reusable Kannada comparison-font set renders Kannada, Tamil, Telugu,
  Malayalam, Devanagari, and Arabic-script examples without hand-authored
  LaTeX. The expanded book has zero missing glyphs, including PIE subscript and
  accented transliteration characters used by the etymology lessons.
- Chapter 20 currently combines numbers 11–20 with an unrelated weather lesson.
  HL-M03 records the need for the authoritative roadmap and session map to split
  that progression or explain the grouping explicitly.
- A forced XeLaTeX build produces 96 pages with 33 top-level and 93 total
  outline entries, correct title and author metadata, and no leaked generator
  directives. Every rendered page was inspected; no clipping, collision, or
  accidental blank page was found.
- The expanded warning baseline is nine overfull boxes, three underfull
  horizontal boxes, seven underfull vertical boxes, four duplicate practice
  labels, 106 Hyperref warnings, and nine font warnings. HL-B25 is next and
  records cleanup against the complete 96-page artifact.
- The unified publication gate builds all twenty books successfully, while the
  data package passes 84 tests and Language Ladder passes 385 tests plus its
  production build.

## Findings from HL-B25

- Explicit regular, bold, italic, and bold-italic faces cover every script used
  by Kannada comparisons without changing the vendored glyph source. Bookmark
  fallbacks retain readable Unicode while omitting presentation-only font
  commands.
- The five handwritten recap labels are unique, and shorter visible or running
  titles preserve transliteration and etymology in the lesson body without
  overflowing page headers or PDF bookmarks.
- Narrow canonical copy edits keep the complete teaching content while giving
  long multilingual lines natural breakpoints. The generated chapter hashes
  continue to be reproduced independently by the data package and Language
  Ladder.
- Natural page bottoms and the final line-break fixes make the forced 96-page
  build completely clean: zero missing glyphs, overfull or underfull boxes,
  duplicate destinations, Hyperref warnings, LaTeX warnings, and font warnings.
- All 96 rendered pages were inspected again after cleanup. The 33 top-level
  chapter bookmarks, 93 total outline entries, metadata, and generator-leak
  checks remain intact, with no clipping, collision, or accidental blank page.
- HL-B26 is next: publish Malayalam Chapters 6–31 from canonical lessons before
  addressing that expanded book's bounded warning cleanup in HL-B27.

## Findings from HL-B26

- All thirty-three canonical Malayalam lessons after Chapter 5 now use schema
  v2 with explicit spine nodes, prerequisite-safe sequences, honest
  sub-five-minute duration budgets, typed knowledge boundaries, skills, modes,
  strands, register, and variety metadata. The first thirty-one lessons remain
  schema v1, so the track is intentionally mixed while migration proceeds
  incrementally.
- Twenty-six generated chapters carry deterministic hashes and lesson ids that
  Language Ladder independently reproduces from the canonical AST. Malayalam
  book coverage is now 100%, and the app and downloadable book share the same
  source through Chapter 31.
- A reusable Malayalam comparison-font set renders Malayalam, Tamil, Telugu,
  Kannada, Devanagari, and Arabic-script examples without hand-authored LaTeX.
  Source-normalized chillus and IAST plus an explicit labialization fallback
  leave the expanded book with zero missing glyphs.
- A forced XeLaTeX build produces 107 pages with 33 top-level and 97 total
  outline entries, correct title and author metadata, and no leaked schema or
  generator directives. All 107 rendered pages were inspected; no teaching
  content is clipped, colliding, or accidentally omitted.
- The expanded warning baseline is 17 overfull boxes, four underfull horizontal
  boxes, ten underfull vertical boxes, four duplicate practice labels, 108
  Hyperref warnings, and seven font warnings. Several expected open-right verso
  pages still carry running headers. HL-B27 records both cleanup targets against
  the complete artifact.
- The corpus report remains at zero duration violations and zero unknown
  prerequisites across 1,066 lessons. It now reports 129 lesson chapters without
  book chapters, 26 fewer than before this migration.
- The unified publication gate builds and catalogs all twenty books in one job
  (270.4 seconds locally), while the data package passes 84 tests and Language
  Ladder passes 411 tests plus its production build.
- HL-B27 follows by making the complete Malayalam artifact warning-free before
  Arabic's larger canonical-book migration in HL-B28.

## Findings from HL-B27

- Explicit static bold and italic faces now cover Malayalam and all five
  comparison scripts, while bookmark-safe Unicode commands preserve readable
  outlines without asking Hyperref to interpret font switches.
- The five handwritten recap labels are unique. Concise running titles and
  narrow copy-flow edits in Chapters 1–3, 12, 20, and 22 remove every remaining
  horizontal overflow without dropping or weakening teaching content.
- Intentionally short micro-lessons use natural page bottoms, and open-right
  chapter breaks now insert genuinely empty versos with no running header or
  page number.
- A forced XeLaTeX build produces 107 pages with zero missing glyphs, overfull
  or underfull boxes, duplicate destinations, Hyperref warnings, LaTeX
  warnings, or font warnings. All 107 rendered pages were inspected again.
- The correct title and author metadata, 33 top-level and 97 total outline
  entries, generated source hashes, and zero schema or generator leaks remain
  intact.
- HL-B28 is next: publish Arabic Chapters 3–27 and the dependency-ordered
  writing companions from the canonical app corpus.

## Findings from HL-B28

- All forty-five canonical Arabic lessons in Chapters 3–27, including six
  dependency-ordered writing companions, now use schema v2 with explicit spine
  nodes, prerequisite-safe sequences, honest sub-five-minute duration budgets,
  typed knowledge boundaries, skills, modes, strands, register, and variety
  metadata. Chapters 1–2 remain intentionally hand-authored so their existing
  inline script introduction stays intact while migration proceeds
  incrementally.
- Twenty-five generated chapters carry deterministic hashes and lesson ids that
  Language Ladder independently reproduces from the canonical AST. Arabic book
  coverage is now 100%, and the app and downloadable book share one source
  through Chapter 27.
- Reusable Arabic and Hebrew script mappings render the Semitic comparisons
  without hand-authored LaTeX. The vendored static fonts leave the expanded
  artifact with zero missing glyphs.
- A forced XeLaTeX build produces 104 pages with 29 top-level and 90 total
  outline entries, correct title and author metadata, and no leaked schema or
  generator directives. All 104 rendered pages were inspected; no teaching
  content is clipped, colliding, or accidentally omitted.
- The expanded warning baseline is five overfull boxes, ten underfull vertical
  boxes, one duplicate practice label, 77 Hyperref warnings, two LaTeX warnings,
  and six font warnings. Several expected open-right verso pages still carry
  running headers. HL-B29 records both cleanup targets against the complete
  artifact.
- The corpus report remains at zero duration violations and zero unknown
  prerequisites across 1,066 lessons. It now reports 104 lesson chapters without
  book chapters, 25 fewer than before this migration.
- HL-B29 follows by making the complete Arabic artifact warning-free before
  Hindi's larger canonical-book migration in HL-B30.

## Findings from HL-B29

- Explicit static bold and italic faces now cover Arabic and Hebrew, while
  bookmark-safe Unicode commands preserve readable outlines without asking
  Hyperref to interpret font switches.
- The two handwritten recap labels are unique. A small emergency line-break
  reserve removes all five horizontal overflows without dropping or weakening
  teaching content.
- Intentionally short micro-lessons use natural page bottoms, and open-right
  chapter breaks now insert genuinely empty versos with no running header or
  page number.
- A forced XeLaTeX build produces 104 pages with zero missing glyphs, overfull
  or underfull boxes, duplicate destinations, Hyperref warnings, LaTeX
  warnings, or font warnings. All 104 rendered pages were inspected again.
- The correct title and author metadata, 29 top-level and 90 total outline
  entries, generated source hashes, and zero schema or generator leaks remain
  intact.
- HL-B30 is next: publish Hindi Chapters 6–33 and the dependency-ordered
  writing companions from the canonical app corpus.

## Findings from HL-B30

- Fifty-one Hindi lessons now use schema v2 with explicit shared-spine nodes,
  unique topological sequence numbers, honest sub-five-minute budgets, typed
  teaching blocks, and prerequisite knowledge boundaries. The set comprises
  forty lessons across Chapters 6–33 plus eleven dependency-ordered writing
  companions already placed inside Chapters 1–2.
- Twenty-eight generated chapters now carry canonical source hashes into the
  book. Language Ladder independently rebuilds and verifies every Chapter
  6–33 hash, so the browser and downloadable book consume the same lesson AST
  instead of parallel copies.
- The existing hand-authored opening remains intact: its writing companions
  gently introduce the headline, inherent vowel, mātrās, preposed short *i*,
  spineless letters, virama, conjuncts, and whole-word assembly
  exactly where the learner first needs them.
- Reusable Devanagari, Arabic, and Cyrillic font mappings preserve Hindi's
  Sanskrit, Perso-Arabic, and cross-language etymology comparisons. The shared
  renderer now emits stable LaTeX for stacked accents, PIE subscripts and
  superscripts, and comparison symbols; the forced PDF build has zero missing
  glyphs.
- The corpus remains at 1,066 lessons with zero duration violations and zero
  unknown prerequisites. Missing lesson chapters in books fall from 104 to 76.
- The expanded PDF builds successfully at 114 pages. Its remaining measured
  warning baseline is nine overfull boxes, one underfull line, five underfull
  pages, three duplicate practice labels, 108 Hyperref warnings, and seven
  font-shape warnings. Physical PDF pages 20, 40, 48, 52, 60, 74, 78, 82, 86,
  90, 94, and 112 are open-right versos containing only running headers and
  page numbers; pages 2 and 4 are the same front-matter pattern. HL-B31 is next
  and owns that cleanup plus the complete rendered-page audit.

## Findings from HL-B31

- Explicit static bold and italic faces now cover Devanagari, Arabic, and
  Cyrillic without changing glyph sources between local and CI builds.
- Bookmark-safe script commands keep readable Hindi, Arabic, and Russian text
  in the outline while preventing Hyperref from interpreting presentation-only
  font switches.
- Four hand-authored practice labels are unique. Natural page bottoms and a
  small emergency line-break reserve remove layout warnings without deleting
  teaching content; the one long Chapter 5 running title has a concise short
  form.
- The twelve open-right chapter versos and two front-matter versos remain in
  the print-friendly layout but are now genuinely empty: no running header and
  no page number.
- A forced XeLaTeX build produces 114 pages with zero missing glyphs, overfull
  or underfull boxes, duplicate destinations, Hyperref warnings, LaTeX
  warnings, or font warnings. All 114 rendered pages were inspected again.
- Correct title/author metadata, 35 top-level and 107 total outline entries,
  generated source hashes, and zero schema or generator leaks remain intact.
- TeX Live places two additional chapter transitions on blank even pages that
  MiKTeX does not need. The production artifact therefore has sixteen blank
  pages versus fourteen locally; both layouts remain 114 pages and preserve
  all content, and the two platform-specific versos were visually confirmed
  empty.
- HL-B32 is next: publish Tamil Chapters 6–31 and its dependency-ordered
  writing companions from the canonical app corpus.

## Findings from HL-B32

- Fifty-one Tamil lessons now use the strict schema-v2 contract: eight inline
  writing steps followed by forty-three content micro-lessons through Chapter
  31. Sequences 100–600 are unique and prerequisite-safe, and every typed body
  block declares the knowledge it introduces or assesses.
- The complete Tamil slice remains below five minutes. Effective durations
  range up to 299 seconds; the retroflex writing step and dative-subject lesson
  remain the intentionally watched boundary cases rather than losing their
  script, grammar, or etymology depth.
- Twenty-six generated chapters replace twenty-six potential hand-maintained
  copies. The committed source manifest lets Language Ladder independently
  reproduce every lesson id and canonical hash used by the PDF; repository-wide
  lesson chapters without a book chapter fall from 76 to 50.
- A forced XeLaTeX build expands the book from 29 to 117 pages with zero
  missing glyphs, correct title/author metadata, 33 top-level and 106 total
  outline entries, and zero schema or generator leaks.
- The expanded book exposes 30 overfull lines, five underfull lines, eleven
  underfull pages, four duplicate practice labels, 146 Hyperref warnings, and
  19 font warnings. HL-B33 is next and owns the full rendered-page and blank-
  verso cleanup instead of mixing publication hygiene into canonical content.
- The roadmap and authoritative session map still stop before the complete
  Chapter 31 corpus. HL-M07 continues to own that progression-metadata work.

## Findings from HL-B33

- Explicit static-font shape mappings remove all nineteen Tamil and comparison-
  script substitutions without introducing a system-font dependency.
- PDF-safe definitions preserve Tamil text in the outline while eliminating
  all 146 Hyperref warnings. Five unique practice labels remove the four
  duplicate destinations.
- Flexible page bottoms and a true-empty `\cleardoublepage` keep the open-right
  print layout while removing all eleven underfull-page warnings and every
  header-only verso.
- Shorter canonical headings, a two-column weekday comparison, and a scannable
  recall checklist remove the remaining line warnings in both book and app
  content. Regeneration keeps their lesson ids and source hashes independently
  verifiable instead of creating book-only copies.
- The forced XeLaTeX build remains 117 pages with correct title/author metadata,
  106 outline entries, zero schema leaks, and zero missing glyphs, overfull or
  underfull boxes, duplicate labels, Hyperref warnings, LaTeX warnings, or font
  warnings. All rendered pages were inspected again.
- HL-B34 is next: publish Latin Chapters 2–36 from the canonical app corpus.

## Findings from HL-B34

- All 53 Latin lessons now use schema v2. Sequences are unique and
  prerequisite-safe; every lesson has a shared-spine placement, explicit
  knowledge boundaries, stable typed blocks, and an effective duration below
  five minutes. Latin becomes the first all-v2 track in the twenty-language
  corpus.
- Thirty-five generation targets turn canonical Chapters 2–36 into book-ready
  LaTeX while retaining each lesson id and a deterministic source hash. The
  generated-chapter check independently guards the content Language Ladder and
  the book share instead of maintaining a second copy.
- The curriculum report remains at 1,066 lessons, 20 tracks, 20 books, zero
  duration violations, and zero unknown prerequisites. Publishing these
  chapters reduces the lesson-to-book chapter gap from 50 to 15.
- A forced XeLaTeX build expands the Latin volume from 12 to 113 pages with
  correct title/author metadata, 38 top-level and 91 total outline entries,
  zero missing glyphs, zero duplicate destinations, zero Hyperref warnings,
  and no schema metadata leaks.
- The expanded book exposes six overfull lines, six underfull lines, eight
  underfull header-only versos, and one unavailable small-caps shape (reported
  in two font warning blocks). HL-B35 owns that focused layout, font, and verso
  cleanup.
- The roadmap and authoritative session map still stop at Chapter 1. HL-M08
  continues to own the progression-metadata reconciliation through Chapter 36.
- HL-B35 is next: remove Latin's remaining layout and font warnings.

## Findings from HL-B35

- Selecting Latin Modern Caps explicitly supplies the book's small-caps shape
  without adding a system-font or vendored-font dependency.
- Natural page bottoms, a two-em emergency stretch, and compact numeric section
  marks remove every overfull and underfull line while keeping generated prose
  readable.
- A true-empty `\cleardoublepage` preserves open-right chapter starts while
  leaving intentionally blank versos free of running heads and page numbers.
- Three dense canonical recall paragraphs now use scannable bullet lists. Their
  generated Chapters 16, 17, and 20 retain independently checked lesson ids and
  source hashes, so the readability improvement reaches both app and book from
  the same source.
- The forced 115-page XeLaTeX build has correct title and author metadata, 38
  top-level and 91 total outline entries, zero schema leaks, and zero missing
  glyphs, overfull or underfull boxes, duplicate destinations, Hyperref
  warnings, LaTeX warnings, or font warnings. Every rendered page was visually
  inspected for clipping, collisions, broken tables, and malformed callouts.
- The original cleanup in #9885 exposed a cross-platform font-name mismatch in
  the unified books job after auto-merge had already completed. #9887 replaced
  the MiKTeX-specific font filename with the portable family name and verified
  the exact `main` artifact in production. HL-P01 records the missing protected
  books gate so a publication failure cannot lose that race again.
- HL-B36 is next: publish Spanish Chapters 19–33 from the canonical app corpus.

## Findings from HL-B36

- All 21 Spanish lessons in Chapters 19–33 now use schema v2, with unique
  sequences, shared-spine placement, explicit prerequisite and knowledge
  boundaries, stable typed blocks, and effective durations below five minutes.
  The *mano* and *agua / vino* lessons now explicitly require the grammatical-
  gender concept they already assumed.
- Fifteen generation targets turn the canonical later lessons into book-ready
  LaTeX while retaining each lesson id and deterministic source hash. The
  generated-chapter check independently guards the same AST loaded by Language
  Ladder instead of maintaining a second content copy.
- The curriculum report remains at 1,066 lessons, 20 tracks, 20 books, zero
  duration violations, and zero unknown prerequisites. Publishing Chapters
  19–33 reduces the lesson-to-book chapter gap from 15 to zero.
- Spanish book generation now supports an inline Arabic script command backed
  by the repository's static Naskh font. Chapter 22 preserves **لازورد** with
  correct right-to-left shaping and no missing glyphs on a clean machine.
- A forced XeLaTeX build expands the Spanish volume to 210 pages with correct
  title and author metadata, 35 top-level and 155 total outline entries, zero
  schema leaks, zero missing glyphs, and zero duplicate destinations. Every
  rendered page was inspected for clipping, collisions, broken tables, and
  malformed callouts.
- The expanded warning baseline is 51 overfull hboxes, 3 underfull hboxes, 19
  underfull vboxes, 14 Hyperref warnings, 2 font-warning matches, and zero
  generic LaTeX warnings. These are concentrated in the legacy layout and are
  owned by HL-B37 rather than hidden by this publication tranche.
- HL-B37 is next: remove Spanish's remaining legacy print warnings and
  header-only chapter versos.

## Findings from HL-B37

- Portable Latin Modern small caps remove the final font fallback on both
  MiKTeX and TeX Live. Natural page bottoms, a two-em emergency stretch,
  compact numeric running heads, and a true-empty `\cleardoublepage` remove
  legacy line, page, and header-only-verso warnings without hiding them.
- Fixed-width legacy grammar tables now use width-aware, ragged-right columns.
  Dense conjugation, tense, mood, and etymology comparisons wrap within the
  printed measure instead of protruding into the margin.
- Plain-text bookmark alternatives preserve the visible vowel-change notation
  while removing all fourteen math-token warnings from the PDF outline.
- The dense Chapter 21 weekday recall is now a scannable canonical bullet list.
  Its generated chapter retains the independently checked lesson id and source
  hash, so the readability improvement reaches both Language Ladder and the
  book from the same source.
- The forced 214-page XeLaTeX build has correct title and author metadata, 35
  top-level and 155 total outline entries, zero schema leaks, and zero missing
  glyphs, overfull or underfull boxes, duplicate destinations, Hyperref
  warnings, LaTeX warnings, or font warnings. All 214 rendered pages were
  inspected for clipping, collisions, broken tables, malformed callouts, and
  Arabic shaping; all 19 intentionally blank physical pages are truly empty.
- That audit led to HL-P01, which made the unified books result a protected
  merge gate that auto-merge cannot outrun.

## Findings from HL-P01

- A path-filtered workflow cannot be required globally: pull requests outside
  those paths would never receive its status context. The workflow now starts
  on every pull request, performs a low-cost path decision, and runs the
  existing single all-books build only when book inputs changed.
- `Human Languages Books gate` is stable and always present on pull requests.
  It fails when detection fails, when a relevant all-books build does not pass,
  or when the build/result combination is inconsistent. A legitimately skipped
  irrelevant build is the only fast-path success.
- Main pushes and manual runs publish `Human Languages Books push gate`, so a
  push result cannot satisfy the protected pull-request context while its book
  build is still in progress.
- Pull request #9893 and the exact merged `main` revision both passed the full
  20-book job. Repository protection now requires both `CI gate` and the
  pull-request-only `Human Languages Books gate`; the exact-main bundle is live
  in the public catalog.
- HL-M01 follows by making the shared/local curriculum relationship executable.

## Findings from HL-M01

- All 20 registered tracks now have an explicit `curriculum.json`: 346 ordered
  path segments map 896 lessons and attach 247 required, supporting, reference,
  or not-applicable extension nodes. All 11 current spine nodes are present in
  every map, including planned/empty ledgers.
- A shared node may recur in a track's path. Contracting each node to one
  contiguous occurrence creates false cycles because real curricula revisit
  greetings, time, definiteness, and other abilities after intervening grammar
  or script work.
- Validation proves canonical and schema-v2 lesson coverage, recursive
  prerequisite closure and topological order, exact extension attachment, and
  explicit omissions. Persian and Urdu each place a required script-entry
  extension inline with the first greeting lesson.
- Spanish, Kannada, Latin, Malayalam, Tamil, and Telugu intentionally teach
  `GREETING-GOODNIGHT` under `SPINE-TIME-OF-DAY` even though the canonical
  concept belongs to `SPINE-TAKE-LEAVE`; those six relocations are now data,
  not exceptions hidden in consumer code.
- The pure planner returns one safe local frontier per selected language and
  groups only frontiers currently ready at the same shared node. HL-M10 follows
  by moving the visible Learn flow and review eligibility onto those frontiers.

## Findings from HL-M10

- Learn no longer has one global concept index or a jump control that can expose
  a late local realization. Every selected language contributes exactly its
  first incomplete mapped lesson, so Persian can advance while Urdu remains at
  its own greeting frontier.
- Progress persists by stable lesson id per language. On every load, saved data
  is reduced to the longest valid local prefix; unknown ids, gaps, and a newly
  inserted prerequisite cannot grant progress past the first missing lesson.
- Lexical lessons require an English-meaning retrieval with the lesson and all
  other language cards hidden. Wrong answers reveal feedback but do not advance.
  Script, grammar, and other support lessons use their authored final recall as
  a self-check until HL-V03 compiles objective typed activity contracts.
- Mixed review contains only independently focused-successful shared lessons.
  It waits for two visually distinct answers, so Persian and Urdu's identical
  `سلام` cannot produce a fake one-option quiz; once another form is unlocked,
  the existing adaptive SRS and confusion log operate on the safe grid.
- Rendered Persian/Urdu QA proved wrong-answer blocking, independent advancement,
  persistence across reload, explicit RTL/script treatment, and delayed mixed
  eligibility with zero browser errors. The app build passes 30 test files and
  478 tests after this tranche.
- HL-V03 is next because objective prompt/answer contracts are the remaining
  prerequisite for replacing non-lexical self-confirmation without scraping
  lesson prose or inventing accepted answers.

## Findings from the Russian HL-A01 slice

- Russian was the only track with non-lexical debt that the first 15-track
  tranche could not cover. Its two frontiers depended on legacy pronoun and
  naming lessons, so attaching activity comments directly would have left their
  knowledge prerequisites unowned.
- The minimal honest migration is six lessons in one closed chain: *я* →
  *ты/вы* → polite *вы* → *меня зовут* → *как вас зовут* → the
  cross-language *how/what* comparison. Stable sequence values, typed block boundaries,
  explicit skill/mode/strand metadata, and transitive knowledge atoms preserve
  the existing prerequisite order and learner prose.
- Objective final-recall contracts now ask for the safest adult form (*вы*) and
  the comparison language that asks *what* (English). Both add eight seconds;
  all six lessons remain below five minutes.
- Measured activity coverage rises from 17 to 19 of 113 mapped non-lexical
  lessons across 16 tracks. The remaining debt falls from 96 to 94, and legacy
  non-lexical debt falls from 18 to 16.

## Findings from the first HL-A01 tranche

- Fifteen schema-v2 tracks with outstanding non-lexical debt now contribute one
  objective final-recall activity apiece. The slice spans Arabic, German,
  Gujarati, Hindi, Italian, Kannada, Latin, Malayalam, Marathi, Portuguese,
  Punjabi, Sanskrit, Spanish, Tamil, and Telugu instead of deepening only the
  original Spanish pilot.
- Every activity asks one exact question already answered by its lesson, assesses
  a containing-block knowledge atom, provides explicit accepted variants and
  feedback, and adds only eight seconds of learner response time. Italian's
  297-second Chapter 2 practice frontier was deliberately left untouched in
  favor of its 237-second Chapter 3 practice lesson, preserving the strict
  sub-five-minute gate.
- Bengali, French, Persian, and Urdu currently have no mapped non-lexical
  self-check debt. Russian's two remaining candidates are both legacy lessons,
  so their activity coverage stays coupled to an honest schema-v2 migration.
  The measured backlog falls from 111 to 96 lessons; all 18 legacy candidates
  remain explicit.
- A clean package install still reports the already-recorded HL-I02 advisory:
  Vitest 4.1.10 reaches PostCSS 8.5.19 through Vite 8.1.5, and the dry-run fix
  resolves it by moving the transitive package to 8.5.25.

## Findings from HL-V03

- A typed block can now carry one or more compact JSON `hl-activity` directives
  immediately after `hl-knowledge`. The canonical AST retains their stable id,
  text-response kind, assessed atoms, prompt, answer, variants, feedback, and
  response budget while book and app learner copy omits the metadata.
- Validation rejects malformed or misplaced JSON, non-lesson-prefixed or
  duplicate ids, empty/out-of-block assessment sets, ambiguous normalized
  variants, missing feedback, and response budgets outside 1–299 seconds.
  Runtime compilation therefore resolves every accepted response once without
  recovering answers from Markdown prose.
- Duration model v2 adds each activity's authored response budget. The first
  grammar and script pilots remain at 180 and 240 effective seconds, the full
  curriculum keeps zero errors and zero duration violations, and regenerated
  Spanish Chapters 1 and 4 retain identical learner prose with refreshed hashes.
- Language Ladder prefers a final-recall activity when present, hides the
  answer-bearing lesson summary during retrieval, shows authored corrective or
  success feedback, and advances only after a correct response plus explicit
  continue. Existing lexical meaning checks remain available.
- The measured follow-up is HL-A01: 113 mapped non-lexical lessons exist, these
  two pilots leave 111 without objective activities, and 18 of those still need
  schema-v2 body contracts before an activity can be attached honestly.

## Findings from HL-D01C

- Gujarati now has zero duration violations. The repository snapshot contains
  982 lessons and 455 violations overall, down from 464 before this tranche;
  unknown prerequisites remain at zero.
- Eight lessons already computed between 110 and 184 seconds and only needed
  honest four-minute declared budgets. The one genuinely long lesson computed
  at 370 seconds.
- That counting lesson is now a 174-second core followed by a 253-second
  etymology lesson. The *dvé → be* inheritance, cross-Indic comparison, and
  restored *r* in *traṇ* remain complete and prerequisite-ordered in the
  canonical corpus consumed by Language Ladder.
- Gujarati Chapter 6 has canonical lessons but is not in the current
  five-chapter PDF. HL-B06 records its one-source migration and generation work
  instead of adding another manual copy.
- A forced build of the unchanged five-chapter book succeeds, but exposes four
  missing punctuation glyphs, one overfull box, four underfull boxes, four
  duplicate practice labels, and 28 Unicode bookmark warnings. HL-B07 records
  that pre-existing publication hygiene debt separately.
- Punjabi and Sanskrit tie for the smallest remaining set at ten violations,
  each with nine declaration-only lessons and one genuine split. Punjabi's long
  lesson computes at 405 seconds versus Sanskrit's 513, so HL-D01D takes Punjabi
  first as the safer bounded tranche.

## Findings from HL-D01D

- Punjabi now has zero duration violations. The repository snapshot contains
  983 lessons and 445 violations overall, down from 455 before this tranche;
  unknown prerequisites remain at zero.
- Nine lessons already computed between 106 and 172 seconds and only needed
  honest four-minute declared budgets. The one genuinely long lesson computed
  at 405 seconds.
- That lesson is now a 229-second counting-and-script core followed by a
  241-second etymology lesson. The Gurmukhi mark distinction, Chapter 5 callback,
  same-source *panjāh/pacās* evidence, and convergence explanation remain
  complete and prerequisite-ordered in the Language Ladder corpus.
- Punjabi Chapter 6 has canonical lessons but is not in the current
  five-chapter PDF. HL-B08 records its one-source migration and generation work
  rather than adding another manual copy.
- A forced build of the unchanged five-chapter book succeeds with no missing
  glyphs, but exposes one overfull box, four underfull boxes, four duplicate
  practice labels, and 28 Unicode bookmark warnings. HL-B09 records that
  pre-existing publication hygiene debt separately.
- Sanskrit's ten violations are now the smallest remaining track-sized set.
  Nine are declaration-only; its 513-second numbers lesson will require a more
  careful split than the three preceding tranches, so HL-D01E is next.

## Findings from HL-D01E

- Sanskrit now has zero duration violations. The repository snapshot contains
  985 lessons and 435 violations overall, down from 445 before this tranche;
  unknown prerequisites remain at zero.
- Nine lessons already computed between 107 and 186 seconds and only needed
  honest four-minute declared budgets. The anchor numbers lesson computed at
  513 seconds and required two new support lessons rather than one.
- Chapter 6 is now a 232-second forms/grammar core, a 240-second east-west
  cognate and sound-law lesson, and a 180-second *pañca* travel lesson. The dual,
  gendered daughter forms, PIE outcomes, Grimm's law, analogy, and qualified
  lexical histories remain complete and prerequisite-ordered in Language Ladder.
- Sanskrit Chapter 6 has canonical lessons but is not in the current
  five-chapter PDF. HL-B10 records its one-source migration and generation work
  rather than adding another manual copy.
- A forced build of the unchanged five-chapter book succeeds with no missing
  glyphs, but exposes three overfull boxes, six underfull boxes, four duplicate
  practice labels, and 28 Unicode bookmark warnings. HL-B11 records that
  pre-existing publication hygiene debt separately.
- Bengali's eleven violations are now the smallest remaining track-sized set.
  All eleven already compute below 300 seconds (maximum 290), so HL-D01F is a
  bounded honest-budget correction with no content split required.

## Findings from HL-D01F

- Bengali now has zero duration violations. The repository snapshot remains at
  985 lessons and drops to 424 violations overall, down from 435 before this
  tranche; unknown prerequisites remain at zero.
- All eleven lessons already computed between 121 and 290 seconds, so only their
  declared estimates changed. No canonical lesson body, prerequisite, book
  source, or app behavior needed rewriting.
- `BN-C06-numbers-1-5` is the tightest corrected lesson at 290 seconds and
  should be watched during later copy edits.
- Bengali Chapter 6 has canonical app-ready content but is not in the current
  five-chapter PDF. HL-B12 records its one-source migration and generation work.
- A forced build of the unchanged book succeeds, but exposes six missing glyphs,
  one overfull box, four underfull boxes, four duplicate practice labels, and 27
  Unicode bookmark warnings. HL-B13 records that pre-existing hygiene debt.
- Italian's twenty violations are now the smallest remaining track-sized set.
  Seventeen are declaration-only and three are genuinely computed, with a
  404-second maximum, so HL-D01G is next.

## Findings from HL-D01G

- Italian now has zero duration violations. The repository grows from 985 to
  989 lessons and drops from 424 to 404 violations overall; unknown
  prerequisites remain at zero.
- Seventeen lessons needed only honest declared-budget corrections. The three
  computed violations were replaced by prerequisite-ordered steps: informal
  `Come stai?` → formal `Come sta?` → register-neutral `Come va?`; `essere`
  forms → the borrowed `stato` story → `andare` → participle agreement.
- The first attempted combined `Come sta? / Come va?` extension still measured
  325 seconds. Splitting register from metaphor produced two independent
  micro-lessons without deleting the cross-language or etymological depth.
- `IT-C02-practice` at 297 computed seconds and `IT-C17-mano` at 296 are the
  tightest remaining Italian lessons and should be watched during copy edits.
- The Italian PDF builds successfully at 13 pages but contains only Chapter 1,
  while canonical app lessons run through Chapter 17. HL-B14 records the
  schema-v2 migration and generated publication work for Chapters 2–17.
- That forced build reports no missing glyphs, overfull boxes, or duplicate
  labels, but does expose one underfull box and three Unicode bookmark warnings.
  HL-B15 records the pre-existing clean-build debt.
- Portuguese's twenty-three violations are now the smallest remaining set.
  Eighteen are declaration-only and five genuinely compute above the limit,
  with a 565-second maximum, so HL-D01H is next.

## Findings from HL-D01H

- Portuguese now has zero duration violations. The corpus grows from 989 to 994
  lessons and drops from 404 to 381 violations overall; unknown prerequisites
  remain at zero.
- Eighteen lessons needed only honest declared-budget corrections. Five new
  prerequisite-ordered lessons preserve all of the longer content: verb-free
  `Tudo bem?` → `Como vai? / Como está?` → casual practice → formal practice;
  `ser` forms → its two-verb, three-stem history → the core `ser/estar` choice →
  adjective meaning shifts; `cabeça` pronunciation → the `caput` doublet map.
- The new and rewritten lessons compute between 143 and 236 seconds.
  `PT-C17-mao` is the tightest remaining Portuguese lesson at 293 seconds and
  should be watched during later copy edits.
- The Portuguese PDF builds successfully at 13 pages but contains only Chapter
  1 while canonical lessons run through Chapter 17. HL-B16 records the
  schema-v2 migration and generated publication work for Chapters 2–17.
- The build has no missing glyphs, overfull boxes, duplicate labels, or Hyperref
  warnings, but reports three underfull boxes. HL-B17 records that pre-existing
  clean-build debt.
- French's twenty-five violations are now the smallest remaining set.
  Twenty-two are declaration-only and three genuinely compute above the limit,
  with a 489-second maximum, so HL-D01I is next.

## Findings from HL-D01I

- French now has zero duration violations. The corpus grows from 994 to 997
  lessons and drops from 381 to 356 violations overall; unknown prerequisites
  remain at zero.
- Twenty-two lessons needed only honest declared-budget corrections. Three new
  prerequisite-ordered lessons preserve the longer content: neutral `Ça va?` →
  explicit `tu/vous` register and liaison; `être` forms → its `es-/fu-/ét-`
  roots; motion/change agreement → pronominal direct-object agreement.
- The new and rewritten lessons compute between 147 and 244 seconds.
  `FR-C03-practice` at 293 seconds and `FR-C15-passe-simple` at 291 are the
  tightest remaining French lessons and should be watched during copy edits.
- The French PDF builds successfully at 79 pages through Chapter 16 while
  canonical lessons continue through Chapter 23. HL-B18 records the schema-v2
  migration and generated publication work for Chapters 17–23.
- The build has no missing glyphs or duplicate labels, but reports sixteen
  overfull boxes, nine underfull boxes, and six Hyperref warnings. HL-B19 records
  that pre-existing clean-build debt.
- German's twenty-seven violations are now the smallest remaining set.
  Twenty-two are declaration-only and five genuinely compute above the limit,
  with a 360-second maximum, so HL-D01J is next.

## Findings from HL-D01J

- German now has zero duration violations. The corpus grows from 997 to 1,002
  lessons and drops from 356 to 329 violations overall; unknown prerequisites
  remain at zero.
- Twenty-two lessons needed only honest declared-budget corrections. Five new
  prerequisite-ordered lessons preserve the longer content: informal wellbeing
  language → formal *Ihnen* register → casual practice → formal practice;
  Präteritum forms → north/south areal history; *sein*-perfect auxiliaries → the
  French/German agreement contrast; *Kopf* as cup → inherited *Haupt* and the
  Grimm's-law/container comparison.
- The new and rewritten lessons compute between 147 and 244 seconds.
  `GE-C16-sein` at 287 seconds and `GE-W03-capitalization` at 285 are the
  tightest remaining German lessons and should be watched during copy edits.
- The German PDF builds successfully at 84 pages through Chapter 16 while
  canonical lessons continue through Chapter 23. HL-B20 records the schema-v2
  migration and generated publication work for Chapters 17–23.
- The build has no missing glyphs or duplicate labels, but reports seventeen
  overfull boxes, eleven underfull boxes, and three Hyperref warnings. HL-B21
  records that pre-existing clean-build debt.
- Telugu's thirty-six violations are now the smallest remaining set.
  Thirty-five are declaration-only and one genuinely computes above the limit,
  with a 360-second maximum, so HL-D01K is next.

## Findings from HL-D01K

- Telugu now has zero duration violations. The corpus grows from 1,002 to 1,003
  lessons and drops from 329 to 293 violations overall; unknown prerequisites
  remain at zero.
- Thirty-five lessons needed only honest declared-budget corrections. The one
  genuinely long lesson is now a prerequisite-ordered pair: build
  **శుభ మధ్యాహ్నం** from the widened “noon” word → distinguish the two-source
  formal-register claim from the one-source lower-frequency claim.
- The two Chapter 31 steps compute to 152 and 193 seconds.
  `TE-C06-dative-subject` at 285 seconds and `TE-C29-subhodayam` at 279 are the
  tightest remaining Telugu lessons and should be watched during copy edits.
- The Telugu PDF builds successfully at 29 pages through Chapter 5 while
  canonical lessons continue through Chapter 31. HL-B22 records the schema-v2
  migration and generated publication work for Chapters 6–31.
- The build has no missing glyphs, but reports four overfull boxes, three
  underfull boxes, four duplicate practice labels, 27 Hyperref warnings, and a
  font-shape substitution warning. HL-B23 records that pre-existing clean-build
  debt.
- The roadmap narrative stops at Chapter 6 and the authoritative session map at
  Chapter 5. HL-M02 records the progression-metadata work through Chapter 31.
- Kannada's thirty-seven violations are now the smallest remaining set.
  Thirty-six are declaration-only and one genuinely computes above the limit,
  with a 360-second maximum, so HL-D01L is next.

## Findings from HL-D01L

- Kannada now has zero duration violations. The corpus grows from 1,003 to
  1,004 lessons and drops from 293 to 256 violations overall; unknown
  prerequisites remain at zero.
- Thirty-six lessons needed only honest declared-budget corrections. The one
  genuinely long lesson is now a prerequisite-ordered sequence: **-ಗೆ/-ಿಗೆ/-ಕ್ಕೆ**
  forms and *k → g* family history → visible Dravidian stacking versus fused
  Latin endings → the existing dative-subject application.
- The rewritten suffix lesson computes to 205 seconds, the new stacking lesson
  to 196, and the following dative-subject application to 281.
  `KA-C01-namaskara` at 295 seconds and `KA-C22-hasiru-haladi` at 294 are the
  tightest remaining Kannada lessons and should be watched during copy edits.
- The Kannada PDF builds successfully at 29 pages through Chapter 5 while
  canonical lessons continue through Chapter 31. HL-B24 records the schema-v2
  migration and generated publication work for Chapters 6–31.
- The build has no missing glyphs, but reports four overfull boxes, five
  underfull boxes, four duplicate practice labels, 30 Hyperref warnings, and
  undefined bold/italic Kannada font shapes. HL-B25 records that pre-existing
  clean-build debt.
- The roadmap narrative stops at Chapter 6 and the authoritative session map at
  Chapter 5. HL-M03 records the progression-metadata work through Chapter 31.
- Malayalam's thirty-seven violations are now the smallest remaining set.
  Thirty-three are declaration-only and four genuinely compute above the limit,
  with a 360-second maximum, so HL-D01M is next.

## Findings from HL-D01M

- Malayalam now has zero duration violations. The corpus grows from 1,004 to
  1,008 lessons and drops from 256 to 219 violations overall; unknown
  prerequisites remain at zero.
- Thirty-three lessons needed only honest declared-budget corrections. Four
  genuinely long lessons become prerequisite-ordered pairs: **ഉച്ച** “peak” noon
  → **പാതിരാ** half-night; Sanskrit *divasam/dinam* → surviving native **നാൾ**;
  Sanskrit **രാത്രി** and its PIE history → native *iravŭ/iruḷ* register split;
  formal **ശുഭ മധ്യാഹ്നം** → the Malayalam/Kannada/Telugu convergence map.
- The eight new or rewritten steps compute between 141 and 235 seconds.
  `ML-C26-raavile` at 299 seconds and `ML-C06-dative-subject` at 294 are the
  tightest remaining Malayalam lessons and should be watched during copy edits.
- The Malayalam PDF builds successfully at 31 pages through Chapter 5 while
  canonical lessons continue through Chapter 31. HL-B26 records the schema-v2
  migration and generated publication work for Chapters 6–31.
- The build has no missing glyphs, but reports seven overfull boxes, eight
  underfull boxes, four duplicate practice labels, 28 Hyperref warnings, and
  undefined bold/italic Malayalam font shapes. HL-B27 records that pre-existing
  clean-build debt.
- The roadmap narrative stops at Chapter 6 and the authoritative session map at
  Chapter 5. HL-M04 records the progression-metadata work through Chapter 31.
- Arabic's thirty-nine violations are now the smallest remaining set.
  Thirty-five are declaration-only and four genuinely compute above the limit,
  with a 360-second maximum, so HL-D01N is next.

## Findings from HL-D01N

- Arabic now has zero duration violations. The corpus grows from 1,008 to 1,012
  lessons and drops from 219 to 180 violations overall; unknown prerequisites
  remain at zero.
- Thirty-five lessons already computed below five minutes and needed only
  honest four-minute declared budgets. Four longer writing lessons are now
  prerequisite-ordered pairs: direction and *alif* → hidden short vowels;
  positional shapes → **سل/لا** joining; the dot family → writing **سلام**; and
  short-vowel marks → hamza.
- The eight new or rewritten writing steps compute between 135 and 279 seconds.
  `AR-C16-al-saa` at 299 seconds and `AR-C14-ashhur` at 298 are the tightest
  remaining Arabic lessons and should be watched during later copy edits.
- The Arabic PDF builds successfully at 18 pages with no missing glyphs, but it
  contains only Chapters 1–2 while canonical lessons continue through Chapter
  27 alongside sixteen writing steps. HL-B28 records the schema-v2 migration
  and generated publication work for the missing content.
- The build reports one overfull box, four underfull boxes, one duplicate
  practice label, 14 Hyperref warnings, and undefined bold/italic Arabic font
  shapes. HL-B29 records that pre-existing clean-build debt.
- The roadmap details only Chapters 1–4 and still labels Chapter 5+ as planned;
  the authoritative session map stops at Chapter 2. HL-M05 records the
  progression-metadata reconciliation through Chapter 27 and the expanded
  writing sequence.
- Hindi's forty violations are now the smallest remaining set. Twenty-nine are
  declaration-only and eleven genuinely compute above the limit, with a
  501-second maximum, so HL-D01O is next.

## Findings from HL-D01O

- Hindi now has zero duration violations. The corpus grows from 1,012 to 1,025
  lessons and drops from 180 to 140 violations overall; unknown prerequisites
  remain at zero.
- Twenty-nine lessons already computed below five minutes and needed only
  honest four-minute declared budgets. Eleven genuinely long lessons become
  thirteen new prerequisite-ordered steps: six script companions, two history
  supports for one-to-five, and focused lessons for age grammar, later-number
  sound changes, cat history, yellow-word evidence, and evening register.
- The 24 new or rewritten steps compute between 114 and 293 seconds.
  `HI-W02-abugida-ka-ta` at 293 seconds and `HI-W04-ra-sa-mera-naam` at 278 are
  the tightest remaining Hindi lessons and should be watched during copy edits.
- The Hindi PDF builds successfully at 29 pages with no missing glyphs, but it
  contains only Chapters 1–5 while canonical lessons continue through Chapter
  33 alongside eleven writing steps. HL-B30 records the schema-v2 migration and
  generated publication work for the missing content.
- The build reports two overfull boxes, five underfull boxes, three duplicate
  practice labels, 29 Hyperref warnings, and undefined bold/italic Devanagari
  font shapes. Visual inspection also finds the final running header colliding
  with the page number. HL-B31 records that pre-existing clean-build debt.
- The roadmap describes only Chapters 1–6 and still labels Chapter 6 as planned;
  the authoritative session map stops at Chapter 5. HL-M06 records the
  progression-metadata reconciliation through Chapter 33 and the expanded
  writing sequence.
- Tamil's forty-two violations are now the smallest remaining set. Twenty-two
  are declaration-only and twenty genuinely compute above the limit, with a
  441-second maximum, so HL-D01P is next.

## Findings from HL-D01P

- Tamil now has zero duration violations. The corpus grows from 1,025 to 1,045
  lessons and drops from 140 to 98 violations overall; unknown prerequisites
  remain at zero.
- Twenty-two lessons already computed between 107 and 285 seconds and needed
  only honest four-minute declared budgets. Twenty genuinely long lessons are
  now prerequisite-ordered pairs, adding focused script, etymology, grammar,
  register, family-comparison, and source-evidence steps without discarding the
  original depth.
- The forty rewritten or new split steps compute between 127 and 296 seconds.
  `TA-W02-ma-retroflex-na` at 296 seconds and `TA-C06-dative-subject` at 294 are
  the tightest remaining Tamil lessons and should be watched during copy edits.
- The Tamil PDF builds successfully at 29 pages with no missing glyphs, but it
  contains only Chapters 1–5 while canonical lessons continue through Chapter
  31 alongside eight writing steps. HL-B32 records the schema-v2 migration and
  generated publication work for the missing content.
- The build reports six overfull boxes, six underfull boxes, four duplicate
  practice labels, 27 Hyperref warnings, and undefined bold/italic Tamil font
  shapes. Visual inspection of the cover, middle, and final pages finds no
  additional clipping or collision. HL-B33 records that pre-existing
  clean-build debt.
- The roadmap details only Chapters 1–6 and still labels Chapter 7+ as planned;
  the authoritative session map stops at Chapter 5. HL-M07 records the
  progression-metadata reconciliation through Chapter 31 and the expanded
  writing sequence.
- Latin's forty-three violations are now the smallest remaining set.
  Thirty-seven are declaration-only and six genuinely compute above the limit,
  with a 370-second maximum, so HL-D01Q is next.

## Findings from HL-D01Q

- Latin now has zero duration violations. The corpus grows from 1,045 to 1,051
  lessons and drops from 98 to 55 violations overall; unknown prerequisites
  remain at zero.
- Thirty-seven lessons already computed between 143 and 297 seconds and needed
  only honest four-minute declared budgets. Six genuinely long lessons become
  prerequisite-ordered pairs: weather-word history → impersonal weather verbs;
  wellbeing questions → the `valeō/valē` family; dative possession → authorial
  name-case variation; the Plautine meeting phrase → its usage limits;
  `vesper`/west → Greek and Romance afterlives; and the missing afternoon
  formula → time-independent `salvē`.
- The twelve rewritten or new split steps compute between 153 and 295 seconds.
  `LA-C19-quid-agis` at 295 seconds and `LA-C17-canis-feles-cattus` at 297 are
  the tightest remaining Latin lessons and should be watched during copy edits.
- The Latin PDF builds successfully at 12 pages with no missing glyphs,
  overfull boxes, duplicate labels, or Hyperref warnings, but it contains only
  Chapter 1 while canonical lessons continue through Chapter 36. HL-B34 records
  the schema-v2 migration and generated publication work for the missing
  content.
- The build reports one underfull box and a small-caps font-shape substitution.
  Visual inspection of the cover, reference page, and final page finds no
  clipping or collision. HL-B35 records that pre-existing clean-build debt.
- The roadmap and authoritative session map stop at Chapter 1 and still call
  Chapter 2+ planned. HL-M08 records the progression-metadata reconciliation
  through Chapter 36.
- Spanish's fifty-five violations are the only duration debt left in the
  corpus. Forty-one are declaration-only and fourteen genuinely compute above
  the limit, led by a 731-second subjunctive lesson; HL-D01R is the final
  duration tranche.

## Findings from HL-D01R

- Spanish now has zero duration violations. The corpus grows from 1,051 to
  1,063 lessons and drops from 55 to zero violations overall; unknown
  prerequisites remain at zero. The integration suite now enforces zero as an
  invariant instead of asserting that migration debt must exist.
- Forty-one declaration-only lessons already computed below the limit and keep
  their bodies unchanged with honest four-minute budgets. The fourteen
  genuinely long lessons become prerequisite-ordered micro-steps or lose only
  duplicated recap prose.
- Twelve new support lessons separate regular subjunctive formation, inherited
  stems, outliers, the mood's name, two-subject clause traps, Arabic
  *ojalá*, form/subject/mood practice, formal/informal register, Arabic
  *hasta* limits, future conjecture, diacritic accents, and punctuation span.
  The unchanged long-form book narrative still preserves the combined depth.
- All 26 rewritten, added, or borderline-trimmed lessons inspected directly
  compute between 122 and 299 seconds. `ES-C17-practice` at 299 seconds and
  four lessons at 294–295 seconds should be watched during future copy edits.
- The 138-page Spanish PDF builds with no missing glyphs or duplicate labels,
  and visual inspection of its cover, middle, and final pages finds no clipping
  or collision. It stops at Chapter 18 while canonical lessons continue through
  Chapter 33; HL-B36 records the fifteen missing generated chapters.
- Chapters 4–18 remain handwritten rather than source-hash-checked from the
  canonical lesson AST. HL-S02 and HL-G03 form the next migration/generation
  slice for Chapters 4–6 before that approach expands further.
- The build reports 52 overfull boxes, 19 underfull boxes, 14 Hyperref warnings,
  and two font warnings. HL-B37 records that pre-existing clean-build debt.
- The roadmap stops at Chapter 18 and still calls Chapter 19 next, while the
  authoritative session map stops at Chapter 3. HL-M09 records reconciliation
  through canonical Chapter 33 and the new micro-lesson chains.
- With duration debt closed, HL-S02 is next: migrate Spanish Chapters 4–6 to the
  strict one-source schema, then generate the same content for the book and app.

## Findings from HL-T01

- Persian and Urdu each had a valid five-lesson dependency chain and a roadmap,
  but neither had the standard session map or on-demand pronunciation reference.
- Both new maps preserve the exact authored prefix and place every N+1, N+3,
  N+7, and N+15 retrieval through session 20 without inventing future lessons.
- Both references are keyed to the sound ids already declared in lesson
  frontmatter, teach script inside known words, and keep transliteration as
  temporary scaffolding rather than a reading prerequisite.
- The Urdu reference distinguished Nastaliq as the intended presentation from
  the then-current vendored Noto Naskh Arabic fallback and deliberately left
  HL-U01 open; the later HL-U01 findings record its closure.
- The next smallest corpus-growth slice is now explicit as HL-E01: complete the
  shared name exchange in both tracks from one canonical schema-v2 source before
  advancing either language to the wellbeing cluster.

## Findings from HL-E01

- Persian and Urdu each add five prerequisite-safe Chapter 3 micro-lessons:
  address/register, the question word, the complete name question, a meeting
  response, and cumulative objective practice. Each track now has ten mapped
  lessons across three published chapters.
- The two earlier name-statement lessons now use schema v2, so every Chapter 3
  prompt closes over explicitly owned knowledge. All twelve touched lessons
  remain below five minutes, with effective budgets from 210 through 240 seconds.
- Both realization maps add register, script, grammar, culture, and consolidation
  extensions without changing the shared path-segment count. Their session maps
  schedule the new lessons and every N+1, N+3, N+7, and N+15 retrieval through
  session 25 while allowing Chapter 4 to begin at session 11.
- Objective activity coverage rises from 19 of 113 to 21 of 115 mapped
  non-lexical lessons across 18 tracks. The remaining count stays at 94 because
  the two newly mapped practice lessons arrive with activities; 16 legacy
  candidates still require schema-v2 migration first.
- Generated Persian and Urdu Chapter 3 files carry the same canonical lesson
  hashes consumed by Language Ladder. The audit also found that Markdown link
  labels survive generation while their URLs do not; HL-G05 records that
  traceability gap instead of widening this curriculum tranche.
- The shared spine next calls for `SPINE-CHECK-WELLBEING`, while both older
  roadmaps currently plan identity grammar for Chapter 4. HL-E02 therefore
  reconciles that order explicitly rather than silently letting the two tracks
  drift from the shared spine.

## Findings from HL-E02

- Persian and Urdu each add six prerequisite-safe Chapter 4 micro-lessons and
  now reach the same shared wellbeing can-do through sixteen canonical lessons.
  The spine stays shared while the local teaching order differs where grammar
  requires it.
- Persian reuses ezafe in **hâl-e shomâ**, teaches one reliable careful question,
  and introduces only attached first-person **-am** in **khubam**. Colloquial
  contraction is visible for recognition but remains outside assessed production.
- Urdu gives **kaise/kaisī** agreement its own step before honorific
  **āp ... haiṅ**, then separates **maiṅ ... hūṅ** from **ṭhīk**. The Hindi
  bridge appears only after the Urdu form is independently readable and
  retrievable.
- Every new lesson carries an objective activity, a declared sub-five-minute
  budget, and closed knowledge prerequisites. Exact review ledgers extend from
  S25 through S31 at N+1, N+3, N+7, and N+15.
- The exact-main verification after HL-I04 found a repeated external Lua setup
  outage: every matrix shard received `ECONNREFUSED` from the Lua download host
  on both the initial attempt and the failed-job rerun. HL-I05/#9910 added a
  pinned cache and checksum-verified source fallback; the consolidated 20-book
  workflow itself was green at the same revision.

## Findings from HL-E03

- Persian and Urdu each add four prerequisite-safe Chapter 5 micro-lessons:
  **khodâ/khudā**, **hâfez/hāfiz**, the complete farewell, and cumulative
  start-versus-end practice. Both tracks now have twenty mapped lessons across
  five generated-book chapters.
- The shared phrase keeps different local writing contracts: Persian normally
  joins **خداحافظ**, while Urdu keeps **خدا حافظ** spaced. Language Ladder may
  compare them only after each local form has passed its own objective check.
- The etymology ramp is deliberately layered: the Persian history of
  **khodâ/khudā** comes first, the Arabic **ḥ-f-ẓ** guard-and-preserve root comes
  second, and the protective formula is assembled only after both words are
  independently readable.
- Every new lesson carries one compiled activity and remains below five minutes.
  Objective non-lexical coverage rises from 23 of 117 to 25 of 119 while the
  explicit 94-item debt remains unchanged.
- Exact review ledgers preserve all older due items and add N+1, N+3, N+7, and
  N+15 retrieval through S35. Casual, later, soon, tomorrow, and good-night
  forms remain explicit omissions until their own prerequisites are taught.
- The corpus report reaches 1,096 lessons, 20 books, zero duration violations,
  zero unknown prerequisites, and zero lesson-to-book chapter gaps. Both new
  Chapter 5 files and hashes are generated from the same AST loaded by the app.

## Findings from HL-G04

- All 270 generated chapter targets now render paired authored ASCII double
  quotes with explicit opening and closing LaTeX text commands. A corpus audit
  finds 5,631 balanced pairs, zero imbalanced generated files, and zero raw
  ASCII double quotes left in generated chapter prose.
- The pairing pass understands emphasis boundaries and nested quotations while
  deliberately preserving code spans, escaped literal quotes, link
  destinations, existing curly quotes, and genuinely unmatched marks. The
  canonical Markdown consumed by Language Ladder remains unchanged.
- The audit exposed indented continuation lines escaping Markdown blockquotes.
  HL-G06 records and completes the supporting fix: continued learner examples
  now remain in one generated quote/callout and one typography pass.
- A single local pass rebuilt all 20 books with zero LaTeX, package, box,
  missing-glyph, or font-warning matches. Visual checks cover Spanish emphasis,
  nested Arabic/RTL glosses, and a continued Telugu example without clipping.
- HL-G05 remains the next queued one-source book gap: generated link labels are
  readable, but their canonical destinations are not yet live PDF links.

## Findings from HL-G05

- The generator now preserves all 55 canonical links in the nine configured
  chapters that contain them: Spanish Chapters 1–3 and Persian/Urdu Chapters
  3–5. Absolute research citations stay on their authored HTTP(S) targets.
- Relative prerequisite and pronunciation-reference links resolve against the
  lesson's stable GitHub source URL. This keeps links useful after a book is
  downloaded, instead of emitting paths relative to an arbitrary PDF folder.
- Link labels still pass through the same emphasis, script-font, and quotation
  renderer as surrounding prose, while destinations use their own LaTeX-safe
  escaping and remain outside typography transformations.
- Generation fails closed when a relative link has no canonical source base or
  when a destination uses a non-HTTP(S) protocol. All lesson filenames match
  their canonical ids, so no additional source-path metadata is needed.
- The audit found 117 authored links across the wider lesson corpus. The 62 not
  yet represented in generated targets remain canonical app content and will
  become live automatically when those chapters migrate to book generation.

## Findings from HL-C18A

- Spanish had **fifteen** over-budget lessons, not the two the HL-C18 row named.
  All fifteen are now split, into **thirty-three** prerequisite-ordered
  micro-lessons; the corpus grows 1,096 → 1,114 and the over-budget count falls
  52 → 37, with the maximum dropping from 7 to 6.
- Splitting was the fix in every case. No lesson was waived, and no atom list
  was trimmed while the body kept teaching the material — each atom the original
  introduced is still introduced exactly once, by whichever half now owns it.
- Every boundary landed on a seam the language already had, not on an atom
  count. The clearest is `ES-C31-numeros-11-20`: Spanish 11–15 are **fused**
  Latin compounds (*ūndecim* → *once*, with only a worn *-ce* left of the
  "ten"), while 16–19 are **transparent** *dieci-* + digit. That is the
  difference between vocabulary you remember and grammar you generate, so the
  split falls after *quince* — and Latin's own subtractive *duodēvīgintī* /
  *ūndēvīgintī*, which Spanish refused to inherit, earns a lesson of its own.
- Five paired lessons were renamed to single-word ids (`ES-C22-rojo`,
  `ES-C26-agua`, `ES-C31-once-quince`, `ES-C32-gato`, `ES-C33-verde`) so the
  filename does not promise content the lesson no longer holds.
- Seven chapter payoffs moved to the new terminal lesson (Chapters 20, 22, 23,
  30, 31, 32, 33). `assesses` stays a subset of the payoff lesson's own
  `practises.knowledge` in every case.
- No lesson genuinely resisted splitting. The nearest thing to a hard case was
  `ES-C23-hermano-hermana` at four, where three atoms form one etymological
  story (*germen* → *germānus* → *hermano*) and the fourth is a sound-history
  correction about the silent *h*; that becomes a 3 + 1 split, and a one-atom
  lesson is well inside the corpus norm (median 2).
- The eighteen new lessons compute between 157 and 275 effective seconds, so the
  five-minute rule holds with room. Two of them are `writing` lessons, which
  moves the pinned modality counts: `pen` 51 → 53, `sight` 351 → 360, `voice`
  694 → 701, drivable share unchanged at 63%.
- HL-C18B is the remainder: 37 lessons across sixteen tracks, led by German (8),
  French (6), Sanskrit (3), Urdu (3) and Italian (3). Bengali, Punjabi and
  Sanskrit each hold a six-atom lesson, all three of them number lessons with
  the same shape as the Spanish one just split.

## Findings from HL-C24

- Four Latin chapters now end on a lesson written to be a payoff:
  `LA-C19-practice`, `LA-C21-practice`, `LA-C33-practice`, and
  `LA-C36-practice`. Latin had exactly one `practice` lesson across 36
  chapters before this tranche; it now has five, and the corpus reaches 1,100
  lessons with zero duration violations and zero prerequisite errors.
- **The representativeness gate cannot see this gap.** All 36 Latin chapters
  already measured 100% before the change, and all 36 still measure 100% after
  it, because a chapter's last teaching lesson cumulatively practises every atom
  the chapter introduced. Representativeness answers "does the payoff touch the
  chapter's material" — it cannot answer "is the payoff something the reader can
  *do*." HL-C03's gate set needs a distinct signal for that: the honest one
  available today is whether the chapter's terminal lesson is of a consolidation
  type (`practice`, `practice-mix`, `pattern`) at all. On that measure Latin was
  1 of 36 and is now 5 of 36.
- Three of the four payoffs are genuine `dialogue`s built only from taught
  words. Chapter 33 is deliberately **not**: it teaches *vesper* and its
  afterlives, with no greeting or exchange anywhere in it, so its payoff is a
  `task` — sort any European evening word into the *vesper* family or the
  *sērus* family, then produce *vespere*. Forcing a conversation there would have
  misrepresented what a taproot track is for.
- The constraint that actually bites is **strict knowledge closure combined with
  a single-word-per-lesson corpus**. A payoff may only recombine what the
  transitive prerequisite chain introduced, and Latin's chain is a thin line
  (each lesson names one or two prerequisites), so useful material taught in a
  *sibling* branch is invisible unless the payoff names it as an extra
  prerequisite. Chapter 19 could reach *grātiās tibi agō* only because
  `LA-C19-quid-agis` happens to depend on `LA-C01-ita-non`; chapter 36 had to
  name `LA-C34-bonum-vesperum` and `LA-C19-practice` explicitly to see the
  *bonus*-phrase and wellbeing atoms at all. Any track-wide scale-up should
  expect to author prerequisite edges, not just lessons.
- Chapters whose material is purely etymological or purely metalinguistic resist
  a usable payoff on principle, not on effort. Latin chapter 33 is the clean
  example; chapters 2 (numbers), 5 (weekday names), and 11 (months) are the same
  shape. `task` and `production` payoffs are the right answer there, and the
  ledger should say so rather than labelling them `dialogue`.

## Findings from HL-C26

- The gap is larger than the ledger work suggested: **105** chapters have a committed
  `book/chapters/ch*.tex` but no `targets[]` entry, across 19 tracks. They are not a
  scattering of stragglers — they are a contiguous hand-written *prefix* of nearly every
  book, ending where generation was switched on. French and German chapters 1–16, Spanish
  7–18, and all of Russian were missing from the informal list this work started from.
- **A `targets[]` entry is not a description; it is an instruction to generate.**
  `generatedBookOutputs` renders every target and `runBookGeneration --write` writes the
  result over the file at `output`. Minting targets for these chapters — the obvious
  reading of the task — would have destroyed them. Confirmed empirically: adding a target
  for `latin` ch1 made `check:books` report the committed file stale immediately, and the
  output it wanted to write was a different 235-line document (banner, regenerated prose,
  `\label{lesson:LA-C01-salve}` in place of `\label{lesson:salve}`) replacing 168 lines of
  authored text.
- The fix is therefore a separate `handwritten[]` list rather than a `generated: false`
  flag on `targets[]`. The two fail in opposite directions: a flag leaves authored prose
  one forgotten `if` away from being overwritten, whereas a second array cannot be
  rendered at all, because `generatedBookOutputs` only ever walks `config.targets`. The
  worst a mistake in `handwritten[]` can do is leave a chapter unchecked — today's
  behaviour — instead of destroying it.
- Every generated chapter opens with `% GENERATED FILE.` and no hand-authored chapter
  does (270/270 and 0/105). That makes the banner a list-independent check on the
  generator's claim, and it catches the one mistake the lists cannot see themselves: a
  chapter *promoted* out of `handwritten[]` into `targets[]`, which leaves the
  hand-written list and so escapes every check keyed on membership.
- **Chapter labels follow three incompatible conventions, and they are left alone.** Most
  hand-written chapters use a bare slug (`ch:greetings`), generated chapters use an
  ISO-code prefix (`ch:fa-`, `ch:la-`, `ch:it-`, `ch:ar-`), and hand-written Persian,
  Urdu, and Russian chapters use a language-*name* prefix (`ch:persian-name`,
  `ch:urdu-name`, `ch:russian-greetings`). So Persian ch2 is `ch:persian-name` while its
  generated ch3 sibling is `ch:fa-ask-and-answer-names`, in one book. Renormalising would
  break every existing `\hyperref`, so `handwritten[]` records what each `.tex` declares.
  No label collides with another inside the same track today. Worth a deliberate decision
  before HL-C04 makes `chapters.json` canonical.
- The bare-slug convention means `ch:greetings` is reused across 16 tracks. That is safe
  only because each track compiles its own PDF; any future combined volume would collide.

## Findings from HL-C30

HL-C30 asked whether Arabic's low drivable share (52%, 31 lessons reachable in
chapter-prefix order) could be recovered cheaply by moving the `AR-W*` writing
lessons that open Chapters 3 and 4 later in their own chapters. **It cannot.**
The measured answer is that no legal reordering changes any Arabic number, and
the premise that "most of the lessons after those openings are `voice`" does not
survive contact with the corpus. No lesson was moved. The findings below are the
whole deliverable.

- **Arabic Chapters 3 and 4 are provably immovable, and the writing lessons are
  not the reason.** A chapter's drivable prefix can only begin with a lesson that
  has no in-chapter prerequisite. Chapter 3 has exactly two such roots —
  `AR-W07-hook-family-ha-kha` (`pen`) and `AR-C03-kayfa` (`sight`, two-column
  table) — so **every** legal ordering starts with a non-`voice` lesson and the
  prefix is 0 no matter what moves. Chapter 4 has a single root,
  `AR-W10-ayn`, because `AR-C04-maa-with` declares it as a prerequisite: مع
  cannot be read without ʿayn. Deleting all six writing lessons outright would
  still leave both chapters at prefix 0.
- **The blocker is the table, not the script.** Of Chapter 3's six non-writing
  lessons only `AR-C03-bi-khayr` is `voice`, and it sits behind
  kayfa → hal → kayfa-ḥāluka by prerequisite. All five of Chapter 4's
  non-writing lessons are `sight`. Every one of Arabic's 18 `sight` lessons is
  `sight` because of a Markdown table — 18 of 18. Arabic's drivable share is an
  HL-C17 problem end to end.
- **Pedagogy would have blocked the move independently.** `AR-C03-kayfa`'s
  "letters in this word" section states that ك has already been written "in the
  writing set", i.e. it assumes `AR-W08-kaf-and-ra`, which requires `AR-W07`;
  `AR-W09-khayr-bikhayr` assembles خير so the learner can hand-write the
  *bi-khayr* reply the chapter ends on. The inline-letters rule in HL00 puts
  those lessons exactly where they are.
- **Arabic's other five zero-prefix chapters have nothing to reorder.**
  Chapters 12, 14, 19 and 20 hold one table-bearing lesson each; Chapter 8's
  second lesson declares its table-bearing first lesson as a prerequisite.
- **Corpus-wide, reordering is nearly worthless.** 123 chapters have a drivable
  prefix of 0. Only **7** contain a `voice` lesson with no in-chapter
  prerequisite, so only 7 are candidates at all; the other **116 are blocked at
  the root by a table** and belong to HL-C17. Arabic contributes none of the 7.
- **Only two of those 7 are genuine**, and both are one-lesson lifts of a
  table-bearing opener that nothing else in the chapter depends on:
  `portuguese ch2` (0 → 3, move `PT-C02-de-nada` after `PT-C02-tudo-bem`) and
  `italian ch2` (0 → 1, move `IT-C02-prego` later). That is the entire
  reordering burn-down for the corpus: **+4 lessons.**
- **The other five are a measurement artifact, and so is much of the report.**
  `orderChapterLessons` sorts by `sequence` with null last, tie-broken by id.
  In mixed-schema tracks the `W*` writing lessons are schema v2 and carry a
  sequence while the word lessons are still legacy and carry none, so the pen
  block sorts to the front of a chapter it does not actually open. **85
  chapters — 25 of them among the 123 zero-prefix chapters — are ordered by the
  report in a way that contradicts their own in-chapter prerequisite graph.**
  `hindi ch1` is reported as opening with `HI-W01-shirorekha-na-ma`, which
  *declares `HI-C01-namaste` as a prerequisite*; `tamil ch1` reports
  `TA-W01-curves-va-ka`, which declares `TA-C01-vanakkam-family-register`. Those
  chapters do not open with a writing lesson at all, and their prefixes are
  measured against an order no author wrote.
- **The list of chapters reported as opening with a `writing` or script-block
  lesson**, i.e. the burn-down order HL-C30 asked for: `arabic ch3`, `arabic ch4`
  (both real and both immovable, above); `hindi ch1`, `hindi ch2`, `tamil ch1`
  (all three artifacts of the ordering bug — nothing to move); `telugu ch7`,
  `telugu ch8` (real, but the openers are table- and script-block-bearing word
  lessons with no `voice` lesson anywhere in the chapter).
- **Arabic Chapters 1 and 2 are undercounted by the same artifact.** All 26 of
  their lessons are legacy and unsequenced, so they sort alphabetically:
  Chapter 1 reports a prefix of 4 where the authored `curriculum.json` path
  gives 7, and Chapter 2 reports 6 where the path gives 7. Recovering those +4
  lessons means giving legacy lessons a `sequence`, which **0 of the corpus's
  565 legacy lessons currently have** and which `validateCurriculum` does not
  check for uniqueness outside schema v2 — a schema migration with a silent
  collision hazard, not a reorder. It is deliberately left undone here. Arabic's
  sparse numbering has room reserved for it: Chapters 1–2 hold exactly 26
  lessons and slots 10–260 are free below Chapter 3's first sequence of 270.
- **Recommended follow-up owners.** The ordering artifact belongs beside HL-C14
  (either the comparator falls back to a prerequisite-respecting order, or the
  legacy tracks get sequences); the 116 table-blocked chapters belong to
  HL-C17. Reordering itself is closed at +4 lessons corpus-wide.

## Findings from HL-C32

The Russian repair is worth reading as a diagnosis, because the diagnosis
generalises and the fix does not.

- **Russian's 9% was one rule firing fifteen times out of fifteen.** Every
  `sight` lesson in the track tripped `wide-table`. Not one carried a `script`
  block. Twelve tripped nothing else. It was never a script-heavy track; it was a
  table-heavy one.
- **Two of the three sight cues that did match were false positives.** The cue
  list is literal substring matching, so `"the course's first look at case"`
  matched `look at`, and a sentence describing a comparison as `"the most extreme
  change in the table"` matched `the table` only because the table existed. Only
  `RU-C02-practice-cases` — *"cover the right column"* — points at anything real.
- **The tables were carrying prose.** Almost every one was a cross-language
  word→gloss list: `| Language | "yes" | built from |`. `RU-C01-privet` and
  `RU-C01-zdravstvuyte` carry exactly that section as sentences, and they were
  the track's only two `voice` lessons. The same content, set two ways, produced
  two different modalities — which is the whole finding.
- **One table was genuinely visual and stayed.** `RU-C02-practice-cases` is a
  cover-the-column retrieval drill; the table *is* the exercise. It remains
  `sight`, and Chapter 2's drivable prefix correctly stops there at 8 of 10.
- **The pattern is corpus-wide, and Russian was only its most extreme case.**
  Of 337 `sight` lessons remaining, 271 trip `wide-table` and nothing else.
  Grouped by track, `onlyTable` counts run: spanish 57, german 33, portuguese 32,
  french 29, italian 29, arabic 14, tamil 14, and so on. Those five European
  tracks sit at 43–47% drivable for the same reason Russian sat at 9%. HL-C17 is
  therefore not a Russian problem that leaked; it is the corpus's single largest
  modality lever, and this pass is a worked example of how to pull it —
  distinguishing a two- or three-column word→gloss list, which linearises, from a
  real multi-column paradigm, which does not.
- **Representativeness was a migration symptom, not an authoring failure.**
  Chapter 2's payoff pointed at a cross-language etymology lesson because that was
  the last schema-v2 lesson by sequence; the chapter's actual consolidation
  lesson was schema v1 and declared no atoms. Migrating that one lesson took
  representativeness from 0.20 to 0.67 without inventing content. The same
  substitution is visible in the remaining sub-floor chapters (arabic ch3/ch4 at
  0.11/0.13, spanish ch3 at 0.25, hindi ch2 at 0.17), and the same one-lesson fix
  should work wherever the chapter's consolidation lesson is the schema-v1 one.
- **Two artefacts remain and only a full migration closes them.** Fifteen Russian
  lessons are still schema v1. Because `sequence` is a schema-v2 field, Chapter 1
  is still ordered alphabetically rather than pedagogically for modality
  purposes, and `RU-C02-practice` now sorts ahead of its own schema-v1
  prerequisite. Neither affects validation or the drivable prefix, and neither is
  worth a cosmetic patch.

## Findings from HL-C38

- The book read like an export because it printed the lesson files' **audio
  scaffolding**: 1,438 `[PAUSE Ns]`, 1,411 `[YOU SAY: …]`, 30 `[REPEAT x2]`, and
  the internal block-type names `Warm-up` / `Guided Practice` / `Wrap-up recall`
  as printed headings, across all 270 generated chapters. None of that is a
  lesson-authoring bug: HL00 is right that lessons are audio scripts. The bug was
  that the **book view** rendered the stage directions.
- Fixed entirely in `src/book.ts`, in one documented "book voice" section. No
  lesson Markdown, `chapters.json`, or hash-manifest entry was touched, and the
  270 chapters regenerate to identical source hashes.
- `[PAUSE Ns]` is deleted (a reader sets their own pace). `[REPEAT xN]` becomes
  *Twice through:*. `[YOU <VERB>: …]` becomes a printed prompt — a single lead-in
  above a uniform run of bullets (*Say these aloud:*), or a per-bullet italic
  label (*Say it:*, *Write it:*) where a list mixes cue kinds. Twenty-eight cue
  verbs are mapped in one table; writing and tracing prompts render as real
  printed exercises and are never suppressed.
- Printed headings now read `Your turn`, `Before you move on`, `What to know
  first`; the warm-up loses its label entirely and stands as the section's
  indented lead-in. `You'll want to know — <descriptive tail>` headings are left
  alone: they are authorial prose, and rewriting them mechanically read worse.
- The chapter blurb ("generated from the canonical micro-lessons used by Language
  Ladder") is gone. A book does not describe its own build system.
- **The book is now a standalone artefact.** The pronunciation/script section
  moved from directly after `\mainmatter` to `\backmatter` in all 17 books that
  front-loaded it — HL00 forbids a front-loaded sounds chapter, so the book had
  been contradicting its own framework. Nothing was deleted; it is reference now,
  not a gate.
- `sourceBaseUrl` no longer feeds the book view, reversing that half of HL-G05.
  A reader holding the PDF cannot follow a link into a Git repository. Relative
  destinations (`./ES-C01-bien.md`, `../pronunciation-reference.md`) now print
  their label unlinked; absolute scholarly citations (UT Austin, MSU, Wiktionary)
  stay live `\href`s. The config field stays, validated, for other consumers.
- All 20 prefaces rewritten, each keeping its own track's material: a welcome, a
  "How to use this book" section, and the removal of the sentence that
  rationalised the front-loaded pronunciation section. The Latin and Sanskrit
  prefaces now say plainly that they are not learned for conversation; Tamil
  addresses the heritage reader HL00 describes. The title-page pointer at
  `code/learning/human-languages/<track>/lessons/` is gone from all 20.
- The 20 track READMEs keep their engineering detail below a `## For
  contributors` line; above it the spec-ID citations, `schema-v2`, source-hash,
  and Language Ladder references are gone.
- **"payoff" was checked case by case and kept.** All 10 uses in book prose are
  ordinary English ("here is the payoff", "two payoffs land at once"), not the
  HL05 field name. Only the README uses referring to `chapters.json` were moved
  below the contributor line.

## Findings from HL-C41

HL-C41 set out to teach Telugu handwriting and to design the interspersed-writing
pattern the project owner asked for. **One half landed and one half is blocked**, and
the blocked half is worth recording precisely, because the block is not a scheduling
problem.

- **Telugu handwriting is blocked on provenance, not on effort.** `strokes.ts` admits
  a letter only with a `citation` and a `url` for its stroke ORDER — the shape is
  checked against the font, the order cannot be, so it must trace to a real source.
  No such source could be reached for a single Telugu letter. The owner's pointer
  (`youtube.com/watch?v=57LhnFmilLs`) returns HTTP 403 and was treated as unverified.
  The candidates a search surfaced — Vemuri's *The Shapes of Telugu* (UC Davis), the
  Peace Corps *Conversational Telugu*, Wikisource's 1857 Brown grammar, Omniglot,
  `teluguaksharalu.com` — were all unreachable from the working session, and none
  could be opened to confirm what it says about any individual letter. A GitHub-wide
  search for an Indic stroke-order dataset returns nothing.
  **Zero letters authored, ~36 base consonants skipped.** Fewer letters honestly beats
  more letters invented, and the same conclusion holds for Kannada and Malayalam.
  What is needed is one openable primer with numbered stroke arrows; with it, the
  base-consonant inventory is a day's work, because the font-validation half of the
  pipeline already exists and `_fonts/NotoSansTelugu-Static.ttf` is vendored.
- **The one substantive claim that *is* attested is a warning, not a shortcut.** The
  premise behind the request — that Telugu is written largely without lifting the pen
  — is a simplification. The recurring published statement about Telugu stroke
  direction is that *the order of the strokes is not uniform across the letters; for
  some it is clockwise and for others counter-clockwise.* Telugu's roundness makes
  many letters **loop-continuous**, which is a real and teachable property, but it is
  not the same claim as "one stroke, no lifts", and the `talakattu` tick that crowns
  most consonants is widely described as a separate mark. So `penLifts` for Telugu is
  exactly the field that must stay ABSENT — meaning NOT VERIFIED — until a path is
  authored and checked.
- **The parts-vs-strokes rule is now written down** in
  [`data/scripts/README.md`](./data/scripts/README.md) and in the syllabary
  generator's own header, where the next author will meet it: only base consonants
  and vowel signs are ever authored, a syllable's figure is assembled from its parts,
  `penLifts` absent means NOT VERIFIED, and it must never be inferred from
  `strokeOrder.length`. Authoring 455 Telugu syllables was never the work; authoring
  ~36 shapes is.
- **Block-level modality landed, with its purpose corrected mid-flight.** The first
  framing — protect the drivable percentage from interspersed writing — was rejected
  by the project owner: *"the book is a standalone artifact… include the writing
  lessons in the books."* The amendment is therefore metadata for a future
  dictation-friendly edition, not a lever on the book. It is a strict improvement for
  that edition: today a lesson with any pen content is lost to a commuter wholesale;
  with block marking they get the voice core and defer only the segment.
- **The amendment is a measured no-op today, on purpose.** No track has authored an
  interspersed `writing` segment yet, so every lesson's core equals its full modality
  and the corpus figure is unmoved at **708 drivable, 65%** — pinned as a regression
  test alongside `lessonsWithWritingSegments === 0`, so the first interspersed lesson
  must move the number deliberately rather than by accident.
- **No demonstration lesson shipped, and that is the finding.** The interspersed
  pattern is implemented and unit-tested against synthetic lessons, but no Telugu
  lesson demonstrates it, because a writing segment for Telugu would have to assert a
  stroke order this repository cannot cite. HL-C42 carries that forward: the first
  interspersed lesson should land in a track whose ductus is already sourced —
  Tamil's ம traces to the UT Austin primer — rather than waiting on Telugu.

## Findings from HL-Q01

- Strict `tsc --noEmit` now passes across both `src/` and `tests/`. The shared
  DOM factory preserves the concrete element type for literal tag names, so
  button-only properties no longer require scattered casts.
- Defensive review-state loading keeps its runtime validation while making the
  intentional untrusted-object boundary explicit to TypeScript.
- Vendored-font tests use the Node declarations owned in #9916 plus ESM-safe
  `import.meta.url` paths rather than relying on a CommonJS global.
- The standalone BUILD runs typecheck before Vite and Vitest. A clean pass
  typechecks, produces the app, and passes all 523 tests in 31 files from the app
  package.
- Directly executing `BUILD` exposed that its dependency-install `cd` persisted
  despite a comment promising a subshell. The command is now actually grouped
  and uses deterministic `npm ci`, so the app gates run from the correct package
  without rewriting the dependency lockfile.
- The clean install also exposed a high-severity Nano ID development advisory.
  Refreshing the transitive lock entry from 3.3.16 to 3.3.18 leaves the app's
  standalone `npm audit` at zero known vulnerabilities.
- The production build emits one 7,115.81 kB JavaScript chunk (1,800.06 kB
  gzip), above Vite's 500 kB warning threshold. HL-Q02 records that separate
  learner-loading concern without coupling a bundle redesign to this repair.

## Findings from HL-Q02

- The authored Markdown glob is now lazy. Learn mode asks for only the selected
  paths' current frontiers plus lessons already passed there; Lessons and
  Concepts explicitly load the complete corpus when first opened.
- The production build has four eager JavaScript chunks: the 409.48 kB app
  shell, 305.12 kB script data, 299.31 kB curriculum plans, and 374.16 kB book
  ledgers. Vite emits no 500 kB warning, replacing the former 7,115.81 kB
  monolith without raising the warning threshold.
- A headless-Chrome run against `vite preview` rendered all 22 initial frontier
  cards, including Persian and Urdu, from the relative-path production build.
- The complete corpus currently produces 1,669 small lazy chunks. That is ideal
  for frontier-sized Learn requests but creates excessive fan-out when a learner
  first opens Lessons or Concepts, so HL-Q03 records a follow-up batching pass.

## Findings from HL-Q03

- Rolldown's manual splitting can group lazy raw-text modules by language and
  then apply a size cap. A 32,000-byte cap keeps each Learn request small while
  reducing a complete Lessons or Concepts load from 1,669 requests to 278.
- The largest emitted lesson batch is 31,391 bytes. The app shell grows from
  409.48 to 475.10 kB because it maps lesson IDs to shared chunk exports, but it
  remains below Vite's 500 kB warning threshold and the build remains clean.
- `npm run check:bundle` makes those budgets executable: fewer than 400 lesson
  batches, no lesson batch above 33,000 bytes, and no eager chunk above 500,000
  bytes. The standalone BUILD now runs the check after every production build.
- A headless-Chrome production run again rendered all 22 initial frontiers,
  including Persian and Urdu, through the grouped relative-path imports.

## Findings from HL-I03

- The index table now follows the authored registry order and derives every row
  from the language registry, parsed lessons, realization maps, authored book
  files, and the generated-book manifest. A newly registered track therefore
  appears even when all of its progress counts are still zero.
- The first generated snapshot keeps all 22 tracks visible and measures 1,669
  canonical lessons, 1,521 uniquely mapped lessons, 513 authored book chapters,
  and 408 generated chapters. These are outputs, not new hand-maintained claims.
- Each row reports the canonical/mapped lesson gap instead of hiding it inside a
  prose status. That makes unfinished one-source migration visible without
  confusing it with missing book publication.
- `generate:progress` rewrites only the marked table. `check:progress` compares
  the complete README byte for byte in the unified publication job before TeX is
  installed, so curriculum or book growth cannot silently stale the index again.
- The exact package BUILD also refreshed HL-I02's evidence: the development tree
  now reports one high Nano ID advisory and one moderate PostCSS advisory, both
  with fixes available. HL-I02 moves ahead of non-security housekeeping next.

## Findings from HL-I02

- Both advisories were lockfile-only development dependency debt beneath
  Vitest/Vite. No direct dependency range or curriculum runtime code changed.
- Nano ID moves 3.3.16 → 3.3.18 and PostCSS moves 8.5.19 → 8.5.26. A clean
  install resolves those exact versions and `npm audit` reports zero known
  vulnerabilities.
- The full data-package suite remains the behavioral guard: curriculum parsing,
  generated books, narration, modality, and the derived progress table all run
  against the refreshed toolchain before this item can merge.

## Findings from HL-C13

- Language Ladder already emits relative production asset URLs, so the exact
  bundle validated by its standalone BUILD can run unchanged below the GitHub
  project site's `/coding-adventures/language-ladder/` path.
- The repository's established subdirectory publication model is the safe fit:
  `peaceiris/actions-gh-pages` writes only `language-ladder/` with `keep_files`,
  preserving the downloadable books and every other Pages artifact.
- The deployment watches the app, its canonical data package, and the complete
  human-language curriculum tree. A content-only lesson change therefore
  republishes practice without requiring a separate application edit.
- The public curriculum index and application README now expose the stable URL;
  no hand-maintained duplicate content was added to make the app public.

## Findings from HL-U01

- The official Noto distribution publishes static Noto Nastaliq Urdu 4.000
  Regular and Bold TTFs under OFL-1.1. Both binaries are pinned to distribution
  commit `46074e15f8956b502051eb4a7796ed8c7d4f3076`, with their SHA-256 digests
  checked byte for byte by the app test suite.
- Both fonts cover the complete Urdu course probe and carry `arab` GSUB/GPOS
  tables with the `URD` language system. The book selects that family through
  fontspec; Language Ladder selects it on Urdu Learn, focused-check, mixed-review,
  Concepts, Browse, and script-practice surfaces while keeping Naskh as fallback.
- The standalone application BUILD passes typecheck, bundle budgets, and all
  527 tests. Its production output contains both static font assets, and a
  headless-Chrome run confirms the Urdu frontier uses the loaded Nastaliq face
  with right-to-left direction and enough line height for the descending form.
- XeLaTeX builds the 77-page Urdu book with no missing-character or fontspec
  warnings. Rendered checks of the opening lesson and later grammar pages show
  contextual Nastaliq joining without collisions or clipped descenders.
- The backlog had accumulated stale completed rows and reused IDs while earlier
  work landed; HL-I06 records that integrity repair without displacing this
  learner-visible Urdu typography fix.

## Findings from HL-I06

- The audit began with 136 table rows but only 133 distinct IDs: HL-C16,
  HL-C17, and HL-C18 each appeared twice. The obsolete copies are removed, and
  the prose-only driving-edition promise is restored as HL-C43; the canonical
  backlog now has 145 rows and 145 distinct IDs.
- Thirty-one completed rows lacked a concrete merge reference. Their statuses
  now point to the commits that introduced the recorded result, including the
  fifteen duration slices and the later chapter, modality, narration, script,
  catalog, link, and dependency work.
- Merge commits #10050, #10054, #10059, #10061, #10063, and #10065 reused
  HL-C44 through HL-C49 for work unrelated to the definitions already in this
  table. Those results now have stable IDs HL-C57 through HL-C62; the legacy
  aliases remain beside their PR numbers so repository history stays searchable.
- HL-C49's generated chapter introductions are recorded complete in #10055.
  HL-C50A's generated pronunciation appendices are recorded complete in #10112.
  HL-C50B's generated glossaries are recorded complete in #10116, HL-C50C's
  generated review questions and answer keys are recorded complete in #10120,
  and HL-C50D's generated subject indexes are recorded complete in #10124.
  Together with the five merged link and cross-volume cleanup slices, those
  artifacts complete HL-C50. HL-C63 and its blocking parser repair, HL-C64, enter
  #10128; HL-C10 is next because the shared spine remains the structural ceiling.
- A completed row must name at least one merge PR, an ID may occur only once,
  and `this PR` is not a durable status. HL-I06 was kept `In progress` until its
  own PR number existed so the repair did not violate the rule while making it.

## Findings from HL-C63

- The 98 missing capability entries were not missing lessons: every handwritten
  chapter already had canonical lesson sources. Authoring their ledgers therefore
  closes chapter intent without changing the 98-file handwritten protection list.
- Schema-v1 chapters still declare no typed knowledge atoms. Their payoffs point to
  real canonical closing lessons and keep `assesses: []`; the representativeness
  gate remains intentionally unscored rather than being made green with invented ids.
- Forty-seven lessons across eleven chapters were absent from the shared path.
  Executable placement also required two earlier Spanish prerequisites and explicit
  local-extension classification, taking mapped coverage from 1,521 to 1,570 lessons.
- The narration export is capability-driven. Closing the ledger gap consequently
  brings all 98 handwritten chapters into the checked app/audio projection while
  leaving book generation ownership unchanged.

## Findings from HL-C64

- The original `\\chapter` regex stopped at the first closing brace, so four Spanish
  titles containing nested `\\emph{...}` commands became newly visible title-drift
  failures as soon as HL-C63 authored their capability entries.
- A balanced-brace reader now returns the complete outer argument, preserves escaped
  literal braces, and falls back to the filename slug when the command is malformed.
  The focused regression test and the 513-chapter gate both pin the repair.

## Findings from HL-C78

- Direct history closes three stale foundation rows: #9957 shipped the typed chapter
  data layer, policy loader, and track-ledger round trips; #9994 shipped all nine
  report-only HL05 gates; and #9974 shipped the tested Language Ladder ductus renderer.
- The live inventory has **22** `chapters.json` ledgers and **513** chapter entries.
  Every chapter has an authored capability and a known, closed payoff after #10128,
  but **27 payoffs across ten tracks** remain below the 0.5 representativeness floor.
  HL-C11 is therefore narrowed, not falsely closed.
- The title transition is safe but unfinished. `chapter-title-drift` reports zero,
  while all **420** generated-book targets still duplicate `title` and `label` in
  `core/book-generation.json`. Removing that second authority is the next bounded
  structural task, HL-C04.
- HL-C05's three pattern gates exist, and its corpus test explicitly records that no
  canonical lesson uses the `pattern` type yet. HL-C06 and HL-C12 remain genuinely
  absent: the 513 chapter ledgers reference zero figures, `_assets` contains no binary
  assets, and there is no SVG/PDF generation, provenance-sidecar, hash, or size gate.
- HL-C15 also remains genuinely absent. The modality engine exports `🚗`, `👁`, and
  `✍` plus every chapter's drivable prefix, but generated book openings print only the
  capability and payoff; no `.tex` chapter prints those signs.
- Fresh report output replaces three stale burn-down snapshots: **61 lessons** have a
  table wider than three columns and 45 are sight-only for that reason; **40 lessons
  across 17 tracks** exceed the three-atom budget; and the script ramp remains at
  **61 glyph violations plus five multi-system Japanese openings**. The current
  corpus is 91% core-drivable, not the historical 84% recorded before later work.
- HL-C19 supersedes HL-C09's old estimate. There are **228** prose stroke-order
  entries across ten scripts: twenty verified ductus paths and 208 entries still
  needing cited, font-checked pen-lift evidence.

## Findings from HL-C05

- The three report gates existed, but the slot-closure check inspected extra introduced
  atoms rather than the declared slots. Since a valid pattern must introduce only its
  one pattern atom, that made the filler check vacuous. It now reads the ordered
  `slots.<name>` lists directly and rejects missing, scalar, empty, or out-of-closure
  declarations.
- `ES-C17-comer-futuro` was already a genuine productive exercise: it reuses known
  *comer*, *beber*, and *café* in three spoken instantiations. Typing it as `pattern`
  and exposing those fillers gives the book and app a canonical first realization
  without adding vocabulary or increasing the lesson's five-minute budget.
- The old gate counted only `*-PATTERN-*` atoms, so one pattern atom plus unrelated
  introductions could pass. The rule now enforces one introduced atom total and proves
  the failure and control directions with focused fixtures.

## Findings from HL-C04

- `core/book-generation.json` carried a second title and label authority for all
  **513** declared chapters: 420 generated targets and 93 handwritten declarations.
  Removing those 1,026 keys leaves each declaration responsible only for its
  coordinates, output path, and rendering mode.
- The book loader now resolves both generated and handwritten metadata through the
  matching `chapters.json` capability. It rejects legacy `title`/`label` fields and
  fails closed when a declaration has no capability entry, while the existing
  title-drift gate still compares the resolved title against committed LaTeX.
- Title and label now participate in the canonical book fingerprint alongside the
  printed capability and payoff summary. The regenerated 420 chapter files therefore
  change only their hash comment, and Language Ladder reproduces the same fingerprint.
- Handwritten books remain protected from generation. Their declarations still
  locate the source file, and the drift tests re-read that file against the one
  canonical title and label from the capability ledger.

## Findings from HL-C15

- The projection covers all **513** numbered chapters in all **22** downloadable
  books: **420** generated chapter bodies and **93** protected handwritten bodies.
  Handwritten prose remains authored; a single stable macro call beside each chapter
  label selects its generated modality record.
- Full `voice`/`sight`/`pen` counts describe everything the printed chapter contains.
  The separate hands-free prefix continues to use core modality in authored lesson
  order, so a later detachable writing segment does not hide an honestly drivable start.
- The signs use tiny TikZ paths instead of Unicode emoji. The repository carries no
  emoji font, and the LaTeX warning gate correctly treats a missing sign glyph as a
  regression; path signs render identically across the 22 existing font stacks.
- Book generation fails closed when a declared chapter has no modality rollup and
  `check:books` byte-gates every per-track projection. Focused Spanish and handwritten
  French builds render the opening cleanly with no missing-glyph, overfull, LaTeX,
  package, or font warning regression.

## Completed foundations

- HL04 defines the 45-concept shared spine and migration contract.
- The multi-track registry, Persian/Urdu pilots, full Markdown bodies, registry-driven
  language selection, RTL app rendering, and fail-closed prerequisites are merged.
- One CI job now installs TeX once, compiles every book, uploads one publication
  bundle, and publishes the catalog after changes reach `main`.
