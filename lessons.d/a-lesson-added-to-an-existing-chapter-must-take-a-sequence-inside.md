---
category: Repo policy / workflow reminders
---

# A lesson added to an existing chapter must take a sequence inside that chapter's span, not the track's next free number

PR #17791 added four Marathi timed-writing lessons to chapters 24 and 25 and
gave them the track's next free sequences (15921-15924), which come after
chapter 275. `validate` and the book generator still passed, because
`readingOrder` sorts by chapter first. But `tests/chapters.test.ts` ("keeps
chapter numbers non-decreasing when lessons are sorted by sequence") failed:

```
marathi: MR-R275-... (seq 15920, ch 275) is followed by MR-A1M19-partial-timer (seq 15921, ch 24)
```

**The fix is to pick a number between the chapter's last lesson and the next
chapter's first.** Start by listing the track's lessons by `sequence` and
`chapter`:

```bash
for f in code/learning/human-languages/<track>/lessons/*.md; do
  echo "$(grep -m1 '^sequence:' $f | cut -d' ' -f2) $(grep -m1 '^chapter:' $f | cut -d' ' -f2) $f"
done | sort -n
```

Sequences must be positive integers, so a densely numbered chapter may have no
free number left. Marathi chapter 25 runs 1060-1069 and chapter 26 starts at
1070. In that case, move the next chapter's *first* lesson into its own gap:
Marathi's `MR-C22-dene` went from 1070 to 1075, which is still before 1080.
That changes one existing lesson instead of renumbering the whole chapter.
Afterwards, rerun `generate:books`, `generate:narration` and
`generate:modality`, because the sequence is part of their source hashes.

The same PR also gave each repair activity a short id
(`MR-A1F14-separate-repair`). `validate` requires an activity id to start with
the **full** lesson id plus a hyphen (`MR-A1F14-partial-timer-separate-repair`).
That error shows up in `tests/integration.test.ts` and in the `cli validate`
exit code, not in the lesson's own corpus test.

The same PR also added one underfull line to the Marathi book (see
`new-lesson-prose-can-push-a-track-book-over-its-latex-underfull`). You can
test a rewording in seconds instead of rerunning the full strict compile.
Materialize with `node dist/book-cli.js --materialize-compile-inputs=<dir>`,
copy `<dir>/<track>/book/*.tex` into a scratch copy of the track's `book/`,
put that copy two levels below links to `_shared` and `_fonts`, delete every
`\input{chapters/...}` except the chapter you are testing, and run `xelatex`
once.
