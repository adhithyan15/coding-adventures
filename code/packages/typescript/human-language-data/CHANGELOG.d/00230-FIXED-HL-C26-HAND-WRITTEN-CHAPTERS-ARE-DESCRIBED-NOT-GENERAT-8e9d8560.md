### Fixed — HL-C26: hand-written chapters are described, not generated

- Add a `handwritten[]` list to `core/book-generation.json` recording the **105**
  chapters that have a committed `book/chapters/ch*.tex` but no `targets[]`
  entry, with `title` and `label` transcribed from what each `\chapter{}` and
  `\label{}` actually declares. These are the hand-authored prefixes of nearly
  every book, written before the generator existed and mostly still schema-v1.
- The obvious fix — giving them `targets[]` entries — would have **destroyed
  them**. A target is not a description but an instruction: `generatedBookOutputs`
  renders every target and `--write` writes the result over the file at `output`.
  A separate array is used instead of a `generated: false` flag precisely because
  the two fail in opposite directions; `generatedBookOutputs` only ever walks
  `config.targets`, so nothing in `handwritten[]` can be rendered by a missed
  branch. The worst a mistake there can do is leave a chapter unchecked.
- Add `handwrittenBookChapters()`, which reads the list without rendering
  anything. `check:books` output is unchanged, byte for byte.
- `chapter-title-drift` previously **skipped** any chapter with no target, which
  left those titles verified by nothing. It now checks them against
  `handwritten[]`, and a new test fails if any ledger chapter is covered by
  neither list — so the assertion cannot decay back into a silent `continue`.
- Add tests that re-read every hand-written `.tex` to prove its recorded title and
  label were transcribed rather than invented, that the two lists never claim the
  same chapter, that no hand-written path appears in `generatedBookOutputs()`, and
  that every committed chapter file is accounted for by one list or the other.
- Add a check that every generation target's committed file opens with
  `% GENERATED FILE.` (true of 270/270 generated and 0/105 hand-written chapters).
  This is the only guard that catches a chapter *promoted* into `targets[]`, which
  by leaving `handwritten[]` escapes every membership-based check.
- Labels are recorded as declared, not normalised. Three conventions coexist — a
  bare `ch:greetings` slug, an ISO-code `ch:fa-`/`ch:la-` prefix, and a
  language-name `ch:persian-`/`ch:urdu-`/`ch:russian-` prefix — so Persian ch2 is
  `ch:persian-name` beside a generated `ch:fa-ask-and-answer-names`. Rewriting a
  `\label` breaks existing `\hyperref` cross-references, so the inconsistency is
  recorded in the backlog for a deliberate decision rather than silently fixed.

