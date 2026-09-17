---
category: Repo policy / workflow reminders
---

# A warm-up block is not a staged block, and only staged blocks keep a zero-new writing contract measurable

A writing lesson that declares **no new atoms** is normally counted as
*measurement-blind* — an empty introduction list is ambiguous between "this
teaches nothing new" and "nobody has migrated its contract yet." There is one
escape hatch: `hasExplicitZeroNewWritingContract`, which treats the lesson as
deliberately zero-new when every block carrying an `hl-writing-stage` directive
assesses **exactly** the lesson's practised set.

Adding an atom to such a lesson's `practises` list therefore has a side effect.
The practised set grows; the staged blocks' assessed set does not; the two stop
matching; the escape hatch closes and the lesson silently becomes
measurement-blind.

**That is what happened here.** An atom was wired into a payoff lesson by
appending it to the first `hl-knowledge` comment in the file — which was the
**Warm-up**. The warm-up carries no stage directive, so it contributes nothing to
the check, while the `connected-composition` block that does carry one was left
untouched.

**Every one of the twelve gates passed.** `validate` reported zero errors. The
only visible symptom was two lines in the gentle-ramp snapshot diff:
`atomMeasurableLessons` down one, `atomMeasurementBlindLessons` up one.

**What to do.** When adding an atom to a writing lesson's practised set, add it
to the blocks with `hl-writing-stage` directives, not merely the first block in
the file. Check for the directive before editing:

```
grep -n "hl-knowledge\|hl-writing-stage" <lesson>.md
```

**The general shape:** a convenience regex that targets "the first match" will
find the warm-up, because warm-ups come first. Any rule that cares about a
*particular kind* of block needs the edit aimed at that kind, not at position.
