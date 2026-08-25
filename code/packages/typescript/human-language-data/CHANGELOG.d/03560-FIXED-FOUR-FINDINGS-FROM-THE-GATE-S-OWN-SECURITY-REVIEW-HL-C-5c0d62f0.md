### Fixed - four findings from the gate's own security review (HL-C221)

- **Polynomial ReDoS.** `<\s*\/?\s*` split N whitespace characters between two `\s*` in O(N²) ways: measured **2,634 ms at N=64,000**, and reachable from ordinary Markdown because a long inline code span is blanked *into spaces*. Regrouping the slash gives **0 ms** at the same input. Timing harness self-tested against `/(a+)+b/` first.
- **Exemptions no longer apply to the rendered layer.** In LaTeX `<!--` is not a comment and a backtick is an open quote; blanking on them hid 7,852 characters of the real corpus and let `\&nbsp;` be smuggled past inside a comment.
- **`<!--` and `-->` are now themselves rendered-layer findings**, since either one reaching a `.tex` means an authoring comment reached the reader.
- **`stripHtmlComments` replaces the comment regex with a linear scan**, fixing three CodeQL high alerts at once: `js/polynomial-redos` (`<!--[\s\S]*?-->` rescanned to EOF from every unterminated opener — 76 ms at 32 KB, now 1 ms), `js/bad-tag-filter` (HTML also closes a comment with `--!>`, so one closed that way survived the strip), and `js/incomplete-multi-character-sanitization` (a single pass over a malformed comment could leave fragments that re-form). Consuming the whole span from `<!--` to its terminator means nothing can re-form; an unterminated comment runs to end-of-text, as a browser treats one.
- **Control characters are stripped from rendered findings.** `[^>]*` admits ESC, CR and BEL and the finding goes straight to a terminal; a finding must not be able to repaint the report that reports it.



