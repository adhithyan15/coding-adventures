### Added — Malayalam chapter 112, where to go

- `ML-A1-LEX-41` closes. Malayalam A1 coverage 219/243 -> **220/243 (91%)**, 23
  points unmapped.

#### The note was stale, and the census decided the scope

> vaidyan ('a doctor') is the only word in this whole area.

True when written; false when read. The health and money halves had already
arrived and nobody re-read the note when they did:

| already taught | where |
|---|---|
| **മരുന്ന്** medicine | `ML-C107-marunnu` |
| **ചികിത്സ** treatment | `ML-C107-chikitsa` |
| **പണം** money | `ML-C91-vila-panam` (`ML-LEX-C91-MONEY-02`; `-01` is **വില**, price) |

Only the **buildings** were missing. A census confirmed it rather than assuming
it: *police* and *pharmac* returned **zero** hits of any kind, and every hit for
*bank* and *hospital* was English prose — `ML-C59-ember` (*"banked at night"*),
`ML-C01-athe` (*"Bank athe as the simple word"*), `ML-C48-doctor` and
`ML-C107-chikitsa` on the English word for a hospital doctor, and `ML-C48-guest`
on hospitality. Control greps confirmed the search reached all 484 lessons.

This is the fourth of the last five Malayalam points whose defect was a sentence
asserting meaning rather than anything mechanical.

#### Two your ear can read, one it cannot

| | |
|---|---|
| **ബാങ്ക്** *bāṅkŭ* | English, audibly |
| **പോലീസ്** *pōlīsŭ* | English, audibly |
| **ആശുപത്രി** *āśupatri* | gives the English ear nothing |

**ആശുപത്രി** is placed last on purpose: the first two build the expectation the
third one breaks, and the recall asks the learner which one they could not
decode. The transferable habit is *sound a long word out before looking it up* —
and a word that does not answer has told you something too.

#### The origin of ആശുപത്രി is deliberately not claimed

Its etymology has more than one account in circulation. The lesson says so in
its own text, and `roots:` is empty on purpose rather than carrying a guess.

`roots:` is the **cousins join key** (`src/cousins.ts`), so a slug asserts a
shared etymology across languages. This track's convention is the **proximate**
source — `ML-C110-phon` uses `english-phone`, not a Greek root — so the two
loanwords take `english-bank` and `english-police`. Spanish's `banka-germanic`
(`ES-C353-banco`) and `hospitale-latin` (`ES-C353-hospital`) were **not** reused:
those are ultimate roots, a different granularity, and pairing on them would
overclaim.

#### No script cost, and the census re-proves it

All three spellings sit inside the 69 glyphs the track's 75 script lessons
already teach, and none introduces a new glyph in reading order. `shown` holds at
**69** in the inventory test — checked before drafting and re-proved by the gate.

Three script-owner-gap counts do move, and the three headwords are exactly why:
**ശ** 17/13 -> 18/14 from **ആശുപത്രി**, and **ങ** 14/12 -> 15/13 plus **ബ** 4/4
-> 5/5 from **ബാങ്ക്**. **പോലീസ്** moves nothing — its glyphs are all owned.

#### Caught in draft rather than in CI

- **ആശുപത്രി** was first tagged `malayalam-conjunct-pra`. The word contains
  **ത്ര** (*tra*), not **പ്ര**, and `core/sound-tags.d/malayalam.json` has no
  *tra* tag at all. Corrected to `[malayalam-sha, conjunct]`.
- The extension shard first said `"category": "lexis"`, which this corpus does
  not use. The values in use are vocabulary 47, language-specific 27, grammar 22,
  consolidation 5, script 4, pronunciation 1, etymology 1.

#### The migration guard moved, and attribution was checked

`tests/curriculum-membership-shards.test.ts` hashes the **live** curriculum
graph, so 7200 -> 7204 and a new digest are this chapter's four lessons. Verified
rather than assumed: `ML-PATH-112-PLACES` holds exactly the four and nothing
else, and the count moved by exactly four. `ML-S112-letter-pa` matches a naive
`/112/` search but is a **script** lesson on `ML-PATH-100` — the same false match
`ML-S111-letter-ca` produced last chapter.

#### Filed

`HL-C415` records that `ML-A1-F-33`'s note still denies a third-person pronoun
the track teaches. Its own citation disproves it: `ML-A1-PRON-03` is **closed**,
probed by `ML-LEX-C71-AVAN-AVAL-01`. The assembly half of that note is still
true and should be kept.

#### Verification

`npm run validate` 21/21, all twelve gates, the full suite (172 files, **2201
passed**, 1 skipped), `check-book-compile.sh --strict malayalam`, all six LaTeX
warning counters at zero for malayalam, and the append-only shard check empty.
