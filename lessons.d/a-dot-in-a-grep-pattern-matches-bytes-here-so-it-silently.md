---
category: Repo policy / workflow reminders
---

# A dot in a grep pattern matches bytes here, so it silently fails on accented Spanish

The pre-check for every Spanish chapter greps candidate words across
`spanish/lessons/`. A word with **zero** hits is taken as genuinely untaught
and gets a lesson written for it. That makes a false zero expensive: it
produces a duplicate lesson for a word the corpus already teaches, which is
exactly what `HL-C418` forbids.

Chapter 473's pre-check produced one. Checking whether *acompañar* was taught:

```sh
grep -rln "^headword:.*acompa.ar" *.md    # 0 hits
```

Zero. But `ES-C446-acompanar.md` exists, its headword is `acompañar`, and it
introduces `ES-LEX-C446-MOVER-03`. The word is taught with a full lesson.

This container starts with **`LC_ALL` and `LANG` both unset**, so grep runs in
the C locale and `.` matches **one byte**, not one character. In UTF-8 `ñ` is
two bytes, `á` and `í` likewise. So `acompa.ar` needs one byte where the text
has two, and the match fails with no warning of any kind.

Worse, the failure is not consistent, which is what makes it dangerous. The
same session ran `grep -rl "compa.ero"` and got four files — because the
corpus filenames and lesson ids use **ASCII** spellings (`ES-C436-companero`),
so the pattern matched `companero` in an id while missing `compañero` in the
prose. A pattern can therefore return a plausible non-zero count and still be
measuring the wrong thing.

**Type the accented character.** `grep -rl "acompañar"` and `grep -rl
"compañero"` both work, and they are no harder to write:

```sh
grep -rl "compañero" lessons/    # 7 files, headword in ES-C436-companero.md
```

If a wildcard is genuinely needed, set the locale explicitly for that command
(`LC_ALL=en_US.UTF-8 grep …`) rather than trusting the default.

And a check that matters should never rest on a single pattern. A zero result
for a common word is itself suspicious: confirm it a second way — look for the
lesson file, the `headword:` line, or the atom in `introduces:` — before
concluding the corpus does not teach it.
