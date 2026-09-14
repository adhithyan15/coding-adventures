---
category: Repo policy / workflow reminders
---

# A truncated view of a conflict list is not the conflict list

2026-09-13. Twice in one session, from the same habit.

**First**, reading what a PR added to `lessons.md` as `head -20` then `tail -22`
of the diff. The two windows did not meet. The paragraph in the gap was the
lesson's actual RULE — "before making two handlers behave alike, check they are
talking about the same object" — and it was nearly dropped while porting the
lesson to a new location. A mechanical line-by-line comparison caught it; eyes
had already signed off.

**Second**, resolving a merge from `git merge ... | tail -6`. That showed two
`CONFLICT` lines and cut off three more above them. Resolution started on the
two, and the first sign of the rest was a test failing with

    TOML parse error at line 29, column 1
    29 | <<<<<<< HEAD

— conflict markers committed into a manifest, found only because a test
happened to parse that file.

The list of conflicts is `git diff --name-only --diff-filter=U`. It is exact,
it is complete, and it stays correct as files get resolved. Scrollback is
neither, and `tail -N` of a command whose interesting output is at the TOP is a
generator of confident wrong answers.

**How to apply.** When a command's output is a LIST you are about to act on,
never sample it — ask for the list. `--diff-filter=U` for conflicts, and for a
diff, compare every line mechanically rather than reading two windows of it.
The cost of being wrong is not a wasted minute; it is a silent omission that
looks exactly like completion.

Related: a conflict resolver's assertions must be able to see what it
discarded — the same failure one level up.
