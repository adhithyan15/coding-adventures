## Unreleased — every Tamil headword now says how it is said (HL-C194), and Chapter 66

Two pieces of work on one branch, because the first one re-renders the book and
the second one wanted to land against a green tree.

### 1. The romanization sweep — 21 load-bearing headwords, closed

`measureScriptClosure` draws its exposure line at exactly one mechanical place:
**a lesson's headword is exposure when the lesson declares a `romanization`, and
something the reader has to decode when it does not.** Twenty-one Tamil lessons
printed a Tamil-script headword and declared none, so the track reported
`headwordsWithoutRomanization: 21` and carried closure violations for glyphs
that appeared nowhere but the undeclared headword.

```
headwordsWithoutRomanization   21 -> 0
script-closure violations      29 -> 21
corpus-wide exposure          188 -> 167   (Tamil is the whole difference)
```

**Every value was transcribed from that lesson's own prose. None was invented,
and none was normalised to a house style that would contradict the page.** That
is the load-bearing decision here, not the field itself. `TA-C14-kaalangal`
declares `vasantha kaalam kodai kaalam mazhai kaalam kulir kaalam` because that
is how its own table spells the four seasons — *kaalam*, not *kālam* — and a
"tidier" value would have made the frontmatter disagree with the lesson the
reader is looking at. The list headwords were checked word by word: twelve month
names for `TA-C16-thamizh-maadangal`, six kinship terms for `TA-C12-kudumbam`,
four Dravidian cognates for `TA-C06-dative-subject-family`.

The check that this is transcription rather than invention is mechanical and was
run over all twenty-one: **every whitespace-separated token of every new
`romanization` appears as a whole word in that lesson's own text**, with the
lesson's own frontmatter line excluded so it cannot verify against itself.

Two lessons could not pass that check as written, and were fixed in the page
rather than fudged in the field:

- `TA-W00-va-guided-copy` teaches the single letter **வ** and never romanised
  it, only the whole word it comes from. The lesson now names it — *va* — in its
  heading and at both points where it asks the reader to look at the shape.
- `TA-C19-vayathu` carries the full question **உனக்கு எத்தனை வயது ஆகிறது?** as
  its headword while its body only ever says *vayathu*. Its heading now carries
  the whole question romanised, in exactly the spelling the following lesson's
  own practice prompt already uses.

Sixteen book chapters, their narration and their hashes moved with the sweep,
which is why it is its own commit-sized change. The ToC and glossary improved as a side effect,
which was the reason to prefer this over any other way of moving the number: the
glossary prints the romanization beside the headword, and the ToC's short title
falls back to it, so `\section[\ta{சித்திரை} \ta{வைகாசி} …]` in the months
chapter is now `\section[Chithirai Vaikāsi Āṉi Āḍi Āvaṇi …]`.

`tests/corpus/tamil.test.ts` tightens 29 to 21 and adds
`expect(track?.headwordsWithoutRomanization).toBe(0)` — pinned at **zero**
rather than at a count, because it is the one number in the closure report an
author can only make worse, by shipping a lesson whose headword nobody can
pronounce.

### 2. Chapter 66 — Which Way

Eight lessons appended after chapter 65 and chained from `TA-C65-doing-recall`,
on `SPINE-MEET-GREET`.

```
vocabulary   160/300 -> 166/300   (shortfall 140 -> 134)
```

Three pairs, each word arriving beside the one that answers it:

```
மேலே     mēlē      up, above     கீழே      kīḻē      down, below
உள்ளே     uḷḷē      inside        வெளியே    veḷiyē    outside
வலது     valadu    right          இடது      iḍadu     left
```

**The chapter's subject is two endings the reader already owns.** Four of the
six are a bare place-word plus **-ஏ** — the same ending inside **இங்கே** and
**அங்கே**, taught long before — and the other two are a word plus **-து**, the
same ending inside **இது** and **அது**. That is why the four stand in front of a
**verb** and the two stand in front of a **noun**, and the chapter closes by
asking the reader to sort all six by which ending they take. Nothing about the
pattern is asserted; it is all visible in words the track already taught.

Every verb and noun the six attach to was taught earlier — **பார்**, **வா**,
**போ**, **கை** — so six new words buy a dozen usable instructions
(*mēlē pār*, *uḷḷē vā*, *veḷiyē pō*, *valadu kai*) without a single additional
new word.

`TA-W22-read-mele` is the script strand's **second consecutive no-new-letter
lesson**. It reads **மேலே**, whose entire content is the **ே** sign ridden
twice — once on **ம**, once on **ல** — so the reader watches the same
left-of-the-letter, said-after-the-letter move happen twice in a row. It sits
third, after *mēlē* was learned by ear and after *kīḻē* came between.

**The chapter costs nothing in script closure, and that is a property of the
inventory being closed rather than of care taken here.** Chapter 66 prints 27
distinct Tamil glyphs and the last of them to be taught, உ, is taught in chapter
36; Tamil's `neverTaughtGlyphs` stays **0** and its
violation count stays **21**. Both hazards from the chapter-65 tranche were
checked before any headword was chosen: no candidate is glossed by a later
lesson (forward references hold at the corpus-wide 670), and no candidate needs
a glyph outside the closed 51.

Concept tags are namespaced `TA-ADVERB-*` / `TA-ADJECTIVE-*` for the same reason
chapter 65's were `TA-VERB-*`: a canonical concept owned by an A1 spine node
would have relocated all six lessons off pre-A1 and moved the vocabulary count
by zero.

Two lessons were reworded after the modality gate classified them `sight` on a
"look at" cue, which would have made chapter 66 unstartable by ear. Both were
teaching an ending that is **audible**, so "listen to how it ends" is the more
honest instruction as well as the drivable one. Tamil's sight count returns to
its baseline 29, and its drivable chapter-prefix reach goes 206 → 208.

### Verification

`node dist/cli.js validate` 0 errors. `npx vitest run` 1689 passed / 3 skipped,
matching the pre-change baseline exactly, with only the two pre-existing
`figure*.test.ts` suite failures (missing `paint-vm` dependency). All nine
`check:*` gates exit 0. The Tamil book compiles under XeLaTeX with **zero**
missing characters, zero overfull and zero underfull boxes.

No regressions anywhere in the report: `neverTaughtGlyphs` 0 → 0, forward
references 670 → 670, reinforcement 585 atoms never revisited → 585, lessons
over the five-minute ceiling 0 → 0, atom-budget spikes 40 → 40.

