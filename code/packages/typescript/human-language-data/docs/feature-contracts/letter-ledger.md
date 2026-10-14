# The letter ledger — what order to meet the letters (HL11)

The ramp budgets say how fast glyphs may arrive. They say nothing about *which*
glyphs, and for a reader who cannot yet speak the language that is the whole
question.

```ts
import { loadEverything, loadChapterPolicy, validateLetterLedger, summarizeLetterLedger }
  from "@coding-adventures/human-language-data";

const { lessons, letterLedgers } = loadEverything();
const ledger = letterLedgers.find((l) => l.script === "tamil")!;

summarizeLetterLedger(ledger);
// { script: "tamil", tracks: ["tamil"], positions: 24, openingWords: 30,
//   firstWritableWord: "…", firstWritablePosition: 2,
//   writableAfter: [{ position: 8, words: 2 }, { position: 16, words: 11 },
//                   { position: 24, words: 18 }] }

validateLetterLedger(ledger, lessons, { unspentWindow: 6 });  // []
```

Ordering letters by the words they complete reaches **நன்றி at Tamil's tenth
glyph and வணக்கம் at its eleventh**, and नमस्ते at Devanagari's twelfth. The same
walk in traditional recitation order completes **zero** words after twelve
glyphs — that gap is the reason the ledger exists.

`firstWritableWord` is deliberately a *word* and not a letter count. Twenty
taught letters is not an achievement a reader can feel; writing *thank you* is.
The measurement is of the payoff, not the effort — the same reason HL05 measures
a chapter by what the reader can do at the end of it.

The ledgers are **authored intent** (`data/scripts/<script>-ledger.json`), like
`chapters.json`: proposed by `data/scripts/propose_letter_ledger.py`, reviewed,
and committed. `validateLetterLedger` may check one and may never rewrite it. It
asks six questions, each with an answer that is invisible by eye — contiguous
positions, glyphs really belonging to the named script, no vowel sign before a
base letter, families sitting together, every claimed unlock naming a lesson
that exists, and no letter that unlocks nothing for a whole window (the Root
Ledger's rule applied to glyphs).

The fifth is the one that earns its keep: a ledger and a curriculum drift apart
silently, because the ledger keeps asserting a payoff long after the lesson
delivering it was renamed, and nothing else would notice.

Note that `loadScripts` deliberately skips `*-ledger.json`. Both files live in
`data/scripts` and both carry the same `script` key, so reading them into one
map would have had one silently overwrite the other, with the winner decided by
filename sort order.
