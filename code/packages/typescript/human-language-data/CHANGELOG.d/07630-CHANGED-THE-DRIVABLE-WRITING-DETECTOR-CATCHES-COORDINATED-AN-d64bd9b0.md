### Changed — the drivable-writing detector catches coordinated and fronted writing instructions

- Issue #12070, fourth pass. `tests/drivable-writing-imperatives.ts` only
  fired when the writing verb opened a sentence or a "…, then write" clause.
  A hand review of the #16893, #16994 and #17014 fixes found the same writing
  tasks in two shapes it could not see, so the audio edition still told a
  driver to write:
  - **chained** onto an earlier step verb: "Say, read, and write each
    answer.", "Look, cover, wait five seconds, and write the word.", "Say and
    write **the Chinese language**", "4. **Writing:** hear all six and write
    them". The clause must open with a step verb from a closed list (say,
    read, hear, listen, cover, hide, look, wait, recall, repair, greet,
    identify, name, retrieve, check, perform, run and a few more), and the
    writing verb must follow ", and", " and" or "," and take an object.
  - **fronted**: "Without looking back, write …", "From spoken names only,
    write …", "With the page covered, write …", "Then, with the new word
    covered, write …", "Beside each, write …". The fronted phrase must open
    with one of a closed set of adverbs and prepositions and contain only
    plain words; at most two may stack.
- Both are judged one clause at a time by the new
  `opensChainedOrFrontedWriting` (clauses come from `clausesOf`, one forward
  pass over the span's stops). No regex has a lazy whole-sentence run or a
  nested quantifier. Each clause has its whitespace runs collapsed first, so
  the chain link names its gaps as single literal spaces (an unanchored
  `\s+and\s+` was quadratic on a long run of spaces); the quotation blanker
  uses `“[^“”]*”`, not `“[^”]*”` (which was quadratic on a run of unclosed
  `“`); and the recall-question guard is applied once per clause (a clause
  ending in "?" is skipped) instead of as a lookahead from every candidate.
  Unit tests run the helpers on 100,000-clause and 100,000-word inputs and on
  the two 40,000-character inputs a security review found quadratic,
  asserting the answers only.
- New exclusions, each from a corpus sentence that asks nobody to write: a
  chain whose last verb has no object ("look, listen, speak, write." — a list
  of skills); anything inside double quotation marks (`Say "I read and write
  Spanish".`); a chain after a subject or adverb ("wine is what you buy, ship,
  tax and write down", "only then will you read and write"); "go to the desk,
  and write your name" (a notice in a Japanese reading passage — "go" is not a
  step verb); "when it opens, write your name" (a subordinate clause is not a
  fronted phrase); "*bare*, write." (a gloss pair). The old exclusions — verb
  as subject, italic glosses, "Write:" vocabulary lines, "copy the sound",
  comma gloss lists, recall questions answered in brackets — apply to the new
  shapes too.
- Fifteen new positive fixtures, fifteen new controls (one is a chained
  recall question) and four `clausesOf` cases.
- Corpus: 129 newly flagged spans in 50 drivable lessons that were clean
  before, every one reviewed by hand and every one a real writing
  instruction (no false positives). Ten are fixed here as `[YOU WRITE: …]`
  cues — chinese 3 (ZH-R16-identity-2, ZH-R17-old-three-r2,
  ZH-R18-looking-three-r2), marathi 6 (MR-C01-practice, MR-R18-script-a-r4,
  -b-r4, -c-r4, -d-r4, MR-R18-script-warmup), russian 1 (RU-C85-date); see
  each track's changelog. The other forty join the existing ledgers:
  `drivable-writing-debt/japanese.json` +7 (95 → 102) and `marwadi.json` +33
  (187 → 220).
- The five lessons from #16994 the review also named (FA-C19-practice,
  UR-C30-practice, UR-C31-practice, RU-R26-close, TE-R152-jhari-recall) were
  already cued and are not flagged.
- Regenerated: the affected Chinese, Marathi and Russian book chapters,
  narration (`.json` and `.txt`), their generated book and narration hashes,
  and the ten `core/lesson-modality` owners (source hash only; all still
  `drivable: true`).
