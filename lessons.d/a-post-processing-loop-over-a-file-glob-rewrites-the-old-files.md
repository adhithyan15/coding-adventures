---
category: Repo policy / workflow reminders
---

# A post-processing loop over a file glob rewrites the old files that match it as well as the new ones

**What went wrong.** LANG84 phase 7c slice 2 added eight `reject_*.py`
corpus cases with a shell helper that wrote each source as a `printf '%s'`
string holding a literal `\n`. A second pass then expanded the escapes:
`for f in reject_*.py; do printf '%b' "$(cat $f)" > $f.tmp && mv $f.tmp $f; done`.
The glob also matched the 17 `reject_*.py` cases that already existed. The
`$(...)` command substitution strips trailing newlines, so the loop
rewrote every old case without its final newline. That is a real change to
the corpus: the `lang_matrix` rows `include_str!` those bytes, and the
`.py` source is part of what the sweep hashes.

**The fix.** `git status` showed 17 modified files that the change never
meant to touch. They were restored from `HEAD` before anything ran against
them.

**Next time.**
- Write each new file once, in its final form (`printf 'print 08\n' >
  name.py`), instead of fixing it up afterwards.
- If a fix-up pass is needed, loop over the exact list of files just
  created, never over a glob that older files can match.
- After any bulk write, read `git status --short` for modified files the
  change never meant to touch.
