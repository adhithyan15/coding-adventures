---
category: Repo policy / workflow reminders
---

# German vocabulary needs a glyph check for Proto-Germanic forms and a compile check for long compounds

**What went wrong.** The German B1 vocabulary (chapters 262-533) passed the
forward-reference census and `cli validate` on its first generation, but two
book gates failed:

1. **Glyph coverage.** Seven etymology lines wrote Proto-Germanic forms with
   *ǭ* (U+01ED, o with ogonek and macron): *gatwǭ*, *sīdǭ*, *kwellǭ*. EtymDB
   prints them that way, but the book font has no *ǭ*, so
   `glyph-coverage.test.ts` failed.
2. **Underfull boxes.** In the strict compile, three lines set underfull
   `\hbox`es over the baseline. Each was a quoted example or note whose long
   compounds (*Schwiegerväter*, *Schwiegermutter*, *Karriere gemacht*) left a
   short line that TeX could not fill.

**Fix.** Write the nasal *ō* in the older notation, *-ōn* (*gatwōn*). Reword
the three lines, then recompile until `scan_latex_log_warnings.py` reports
`underfull=0` for the track.

**Next time.**
- Before generating, grep every spec for characters outside the font. The
  usual offenders in etymologies are ǭ (Proto-Germanic) and ȳ (Old English).
- For a German, Dutch or Scandinavian track, run the strict compile on the
  first generation, not only at the end. Long compounds in narrow quote boxes
  are the usual cause of underfull lines.
