## HL-C216 — the CI "appendix warnings" are multi-pass noise; the artifact is clean

Reported from reading CI logs: lots of warnings around the appendix. **Investigated;
the books are clean. Nothing to fix, and the reason is worth writing down so the
next person does not re-open it.**

### What the CI log actually contains

The books build step emits **8,541 warning lines**. Nearly all are one of four,
once per track:

```
22x  Package rerunfilecheck Warning: File `book.out' has changed
22x  Package hyperref Warning: Rerun to get /PageLabels entry
22x  LaTeX Warning: There were undefined references
22x  LaTeX Warning: Label(s) may have changed. Rerun to get cross-references right
 9x  Package polyglossia Warning: hyphenation patterns for modern Latin
```

**These are INTERMEDIATE-PASS artifacts.** `latexmk` runs xelatex repeatedly; on the
first pass the `.aux` has no labels yet, so every cross-reference is undefined and
LaTeX asks to be re-run. By the final pass they resolve. Checked directly: the
individually-alarming `Reference 'ch:ru-script-eleven' undefined` and
`ch:fa-script-nine` both **are** defined -- one definition each, referenced 13 and 11
times -- and **do not appear undefined in any final `book.log`.**

### Why they look appendix-related

They are not. `latexmk` prints each file as it opens it, so the log interleaves

```
(./chapters/appendix-glossary.tex [97] [98] [99])
```

with warning text from the same pass. The appendix files are simply the LAST and
LONGEST things processed -- spanish's index is 4,739 lines -- so they sit next to the
end-of-pass warning burst. **Proximity in the log, not causation.**

### The measurement that matters

The repo's own scanner reads the FINAL `book.log` and counts six classes. Both
locally and in CI it reports, for all 22 tracks:

```
overfull=0 underfull=0 missing_character=0 hyperref_warning=0
duplicate_destination=0 font_substitution=0
```

The only non-zero values anywhere are spanish `font_substitution=2` (a bold-mono
face CI lacks; cosmetic, already understood) and one russian `underfull`. The
`latin: ... [over baseline]` lines in CI are the scanner's OWN self-test fixtures,
not corpus results -- worth knowing, since they look like a failing track.

### What would be a real finding

A warning that survives into the FINAL log. `data/scripts/build_all_books.sh` reads
exactly that, which is why it reports clean while the raw CI log looks alarming.
**Judge the artifact, not the transcript.**

### The one real defect it surfaced -- FIXED in this change

`ch:how-are-you` was defined twice, in spanish chapters 8 AND 9, because
`chapters.json` carried the same `label` on both. That one DID survive to the final
log, and it was not cosmetic: the index referenced that single target **eight**
times, four of them printed as *"Chapter 8, p.N"* -- but `\pageref` resolves to
whichever definition LaTeX saw last, which is chapter 9. **Every index entry
pointing at chapter 8 printed chapter 9's page number.** A reader following it lands
on the wrong page.

Chapter 9 now carries `ch:answering`. The index splits 4/4 across the two targets,
and the `multiply defined` warning -- the last real warning in the whole corpus -- is
gone. All 22 books rebuild clean.

**Worth noting how it was found:** not by the warning, which had been visible and
ignored for a long time as "pre-existing and cosmetic", but by asking what the
warning would actually DO to a reader. The warning named a duplicate label; the
defect was a wrong page number in an index.
