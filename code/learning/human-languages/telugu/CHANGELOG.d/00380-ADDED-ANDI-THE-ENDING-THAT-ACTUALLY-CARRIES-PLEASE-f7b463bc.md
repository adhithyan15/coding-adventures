## ‑ండి, the ending that actually carries "please"

`TE-C08-dayachesi` introduced **four** atoms against a budget of three — the only
Telugu content lesson that did — because it was doing two jobs. One was the word
**దయచేసి**; the other was a paragraph headed *"Be honest about how it's used"*,
explaining that day-to-day Telugu politeness rides on the respectful command
ending **‑ండి**, not on a separate word.

That second job is a lesson. It is now one.

#### What the new lesson says

The standalone **దయచేసి** is formal or emphatic. **కూర్చోండి** is already
"please sit" — the politeness is in the ending, and no extra word is required.
The two **stack** (*దయచేసి కూర్చోండి* is warmer) but they do not substitute:
putting **దయచేసి** in front of the short form reads as oddly as "kindly sit
down, mate."

The point is structural rather than lexical, which is why it earns a lesson:
English keeps politeness in a word you add, Telugu keeps it in the shape of the
verb. A learner hunting for a word to translate *please* will reach for
**దయచేసి** far too often and still sound abrupt.

#### What it cost the gate — nothing, which was the design

```
before   telugu: vocabulary 79, reinforcement 50, atom-budget 1
after    telugu: vocabulary 79, reinforcement 50
```

The `atom-budget` blocker is cleared and **neither other blocker moved**:

- `type: grammar` is outside `CONTENT_TYPES`, so the lesson adds no headword and
  the vocabulary shortfall is untouched.
- The relocated atom keeps its existing three revisits from `TE-C09`, `TE-C39`
  and `TE-C74`, so no reinforcement debt is created.

That is the shape HL-C424 argued every Telugu tranche has to have: content that
pays for its own atoms.

#### Three things the tests caught that reading had not

- **The atom id is load-bearing in four more places.** Renaming
  `TE-PRAGMATICS-C08-DAYACHESI-03` to match its new owner broke
  `core/exam-inventory-telugu-a1.json`, `chapters.d/0008.json`,
  `chapters.d/0009.json` and an **append-only** `CHANGELOG.d` shard. The id
  stays as it is; a slightly misleading name is the cheaper of the two wrongs.
- **The first draft added three forward references.** A plain-vs-respectful
  table printed the bare forms **కూర్చో**, **రా** and **ఇవ్వు**, which are
  headwords taught 93, 52 and 95 lessons later. The table now shows only the
  ‑ండి forms and defers the short ones in words.
- **A banned word.** "You have *just* learned" — HL10 §7.4.

Attribution for the graph digest was by reconstruction and structural diff: a
clean worktree at `29907337c4` reproduced the previous digest and 7485 byte for
byte, and diffing the loaded graphs gives sixteen changed lines, all of them
this lesson.
