### Fixed — a cue wrapped across source lines renders as a cue, not raw brackets

Sixty generated chapters in twelve books printed a lesson's delivery cue as
literal LaTeX-escaped brackets — `{[}YOU SAY: …{]}`, `{[}PAUSE 4s{]}`,
`{[}REPEAT x2{]}` — on the page: chinese 8, japanese 8, punjabi 8, marathi 7,
hindi 6, spanish 13, tamil 4, sanskrit 2, french 1, gujarati 1, malayalam 1,
marwadi 1. By kind: 113 `PAUSE`, 30 `REPEAT`, 44 `YOU SAY`, 6 `YOU HEAR`, and
one each of `YOU ANSWER`, `YOU READ ALOUD`, `YOU RUN`, `YOU SWAP`.

**Root cause.** `bookVoice` found cues with anchored regular expressions, one
source line at a time, and only at the head of a line or filling a whole
bullet. Every other position leaked:

- a cue **wrapped** across source lines (`[YOU SAY: *… shũ` / `chhe*]`) —
  sanskrit ch05, gujarati ch21, tamil ch29 (whose last line is flush left);
- a pause **mid-sentence** (`Cover the model. [PAUSE 5s] Which …`) and a
  repeat **after prose** (`… let it creak. [REPEAT x2]`);
- a cue **after prose in a bullet** (`- "I finish at one." [YOU SAY: …]`);
- **two cues in one bullet**, which the end-anchored greedy pattern read as one
  cue whose content was `a] [YOU ANSWER: b` (marwadi ch01);
- a **qualified** prompt — `[YOU SAY (m.): …]`, `[YOU RUN the pattern: …]`,
  `[YOU READ ALOUD, gathering …: …]` — which no pattern accepted at all.

The narration export had its own, better grammar (a depth-tracking bracket
scan over whole paragraphs), so it handled wrapped cues — but it too rejected
qualified prompts, and read all 18 of them aloud as prose:
`[YOU SAY (m.): boltā hūṁ]`.

**Changes.**

- New `delivery-cue.ts`: the ONE cue grammar. `closingBracket` (depth-tracking,
  escape-aware, bounded lookahead), `opensDeliveryCue` (a constant-time keyword
  filter run before any bracket scan), `parseDeliveryCue` (pause / repeat /
  prompt, with the prompt's upper-case verb split from an optional qualifier),
  `joinWrappedLines` and `joinQualifier`. Index scans throughout; no pattern is
  run across a paragraph.
- `book.ts` `bookVoice`: each paragraph and each list item is scanned as the
  single unit renderMarkdown typesets it as, so a cue may cross source lines
  (never a blank line, a numbered item, a table row, or a quote break). A
  wrapped cue's content is re-joined exactly as Markdown joins lines, so it
  typesets byte-for-byte like the same cue on one line. Pauses are deleted
  wherever they sit, with the blank they leave; a repeat after prose prints
  *Twice through.*; a qualified prompt prints *Say it (m.):*. A bullet's cue
  may finish on a flush-left line while its bracket is still open; a bullet
  that was only a pause is dropped; a run of `> ` lines is one unit. Uniform
  lead-ins ("Say these aloud:") are not used for qualified cues, which would
  lose the qualifier.
- `narration.ts`: `parseNarrationCue` and `splitNarrationCues` now call the
  shared grammar. A qualified prompt becomes a `prompt` segment with an optional
  `qualifier` field (absent on every unqualified prompt, so their JSON is
  unchanged) and is spoken as `[your turn — say (m.): …]`.
- Any blank after a cue keyword — space, tab, or a wrap's newline — counts
  the same in the parser, the keyword filter and the gate, so `[YOU\tSAY: hi]`
  is a cue (and would be caught if it ever were not).
- A bullet's lines are gathered into an array with a running open-cue
  tracker, read once each; re-scanning the growing bullet per flush-left line
  was quadratic (28 s for 80,000 `[YOU a` lines).
- New gate: `findPrintedDeliveryCues` (exported) lists every `{[}YOU`,
  `{[}PAUSE`, `{[}REPEAT` (or bare `[YOU` …) followed by a space, a tab or the
  line's end, and `book-cli` fails `--check` **and** `--write` when any
  generated `.tex` contains one, naming the file and line. It scans every
  `.tex` the book compiles — the 5,834 chapters and the never-committed
  compile inputs (`book.tex`, `chapter-modalities.tex`, glossary, answer-key,
  index and pronunciation appendices) — all 5,972 clean today.
- Regenerated: the 60 chapters above (no other chapter changes), and 8
  narration chapters (french 29; hindi 4, 5, 34, 35, 43; punjabi 8, 9) with
  their narration-hash owners.

**Tests.** `tests/delivery-cue.test.ts` (new) pins the grammar.
`tests/book.test.ts` covers two- and three-line wrapped cues for WRITE, SAY,
TRACE, HEAR, READ and ANSWER, a cue followed by prose, inside a numbered item,
inside a blockquote, after prose in a bullet, finishing flush left, with bold
and Devanagari script spans through to the LaTeX, pauses and repeats
mid-line, two cues in one bullet, qualified cues, a tab after the keyword,
untouched glosses, and five hostile inputs that were quadratic in drafts of
the scanner (asserted on output, not only on the clock).
`tests/narration.test.ts` pins qualified and wrapped cues in the script.
`tests/book-cli.test.ts` asserts no generated and no committed chapter prints
a cue's brackets, and that the CLI fails on a lesson whose bracket cannot be
voiced.
