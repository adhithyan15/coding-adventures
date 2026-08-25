### Fixed - a report that its own subject could edit (HL-C90)

Every gate in this package interpolates author-written strings into lines
written to stdout: lesson ids, node ids, root slugs, finding messages. A lesson
id carrying an ANSI escape rewrites its own line in a terminal, so a crafted id
could erase the very defect line a reviewer is reading to decide whether the
corpus is sound.

These reports exist to make problems visible. A report that can be edited by its
subject does not.

`stripControlCharacters` now guards **nineteen interpolations** across
`report.ts`, `strands.ts`, `grammar-cells.ts`, `root-ledger.ts`, `info-dump.ts`
and `metalanguage.ts` -- every place a corpus-derived string reaches a report
line. Control characters are removed rather than escaped: the reports
are read by humans, not parsed, so a visible `\u001b` adds noise without adding
information. Tab and newline survive -- they are ordinary layout, and the render
helpers control their own line breaks.

Found by the security review of HL-C80 and filed whole rather than fixed halfway
inside an unrelated PR, because the pattern was package-wide from the start.

The tests build their control characters with `String.fromCharCode` rather than
writing literals, for two reasons learned in this session: a literal ESC in a
source file is invisible to a reviewer, and this repository has already had
non-ASCII source literals silently mangled on write.

