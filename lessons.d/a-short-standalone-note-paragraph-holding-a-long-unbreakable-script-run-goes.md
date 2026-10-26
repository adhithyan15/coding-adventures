---
category: Testing & coverage
---

# A short standalone note paragraph holding a long unbreakable script run goes underfull in the strict book scan

The Japanese B1 vocabulary builder wrote each verb's past as its own note,
`Past (polite): **せんせいが うながしました。**, *sensei ga unagashimashita*.`, and
each note becomes its own two-line paragraph in the book. A justified paragraph
whose first line holds a long kana run has few break points, so TeX stretched the
spaces: eight of the 442 such paragraphs came out underfull (badness 2269 to
10000), and `scan_latex_log_warnings.py` put Japanese over the warning baseline.
Every package check, the full vitest and the language-ladder suite passed; only
the strict compile plus the scan saw it.

The fix was at the source, not the baseline: the builder now appends the past to
the paragraph that already gives the polite form and the verb class (`Polite
**うながします**, *unagashimasu*; a godan verb. Past (polite): …`), which is how
the hand-written core chapters set it, and they compile clean. A longer paragraph
gives TeX enough break points, and its last line is free to stay short.

Do differently: run `check-book-compile.sh --strict <track>` and the warning scan
before calling generated output ready, and prefer one paragraph per headword's
forms over a stack of one-line notes, above all in tracks whose script has no
spaces inside words (Japanese, Chinese, Thai).
