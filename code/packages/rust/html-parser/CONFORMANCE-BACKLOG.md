# HTML Parser Conformance Backlog

Last audited: 2026-08-09

## Completion Boundary

The parser conformance loop is complete when a fresh audit against current WPT
tree-construction resources and html5lib tokenizer tests reports:

- zero missing WPT tree-construction signatures
- zero missing html5lib tokenizer signatures
- zero normalized tokenizer skips
- every tree-construction case with declared parse errors emits diagnostics
- tree-construction cases without declared parse errors do not gain spurious
  diagnostics unless the checked fixture intentionally omits a current WHATWG
  error
- all checked parser, lexer, fixture, formatting, and lint gates pass

The checked `tests/fixtures/html5lib-coverage-audit.json` report records the
exact upstream commits used for the latest completed audit.

## Prioritized Queue

The 2026-08-09 upstream audit at WPT
`54f8f933629e7c010ae98a246729af01f8abcda5` and html5lib-tests
`224991ec10db04f056a89eed8b0bd8695fd2950e` covered all 1,934 WPT
tree-construction cases and all 6,806 html5lib tokenizer cases with zero missing
signatures and zero normalized skips. DOM output is complete, but diagnostic
coverage is not:
the checked 2,637-case tree corpus declares 6,243 errors across 2,183 cases.
After the seeded-SVG foreign-breakout diagnostic slice, all 2,183 malformed
cases emit at least one lexer or parser diagnostic and none remain uncovered.
Another 139 cases emit diagnostics despite having no legacy `#errors` rows.
These are reviewed rather than automatically removed: 89 are full-document
inputs for which the legacy fixtures omit the Standard-required missing-doctype
error, including the current processing-instruction cases.

The concrete document-shell insertion-mode inventory is complete: missing and
nonconforming doctypes, duplicate shell tags, body/html boundary errors,
post-head head-content starts, after-body and frameset modes, implied-shell
paragraph end tags, and rejected frameset starts all report their
current-Standard parse errors.

Fragment in-body EOF diagnostics are also complete. Authored disallowed open
elements now report the same error as full-document parsing, while synthetic
context and table-wrapper nodes are excluded. Remaining fragment EOF-labeled
rows depend on table foster-parenting/scope recovery or foreign-content table
boundaries and stay assigned to those algorithm audits.

Text insertion-mode EOF diagnostics are complete for authored `script`,
RCDATA, RAWTEXT, and scripting-enabled `noscript` elements. EOF now reports the
mode's parse error, pops the current text element, and reprocesses EOF in the
original mode; synthetic text fragment contexts remain diagnostic-free.

The in-body "any other end tag" parse error is now reported when implied end
tags leave the matching open element non-current, including special-element
scope stops and nested formatting recovery. Remaining in-body work covers
formatting-reconstruction and other stray-tag branches. Obsolete `menuitem`
end tags now report when no matching element is current. Heading end tags report
the specialized parse error when no heading is in scope or the current heading
does not match the token. A `li` start tag reports when its implied-end-tag
recovery closes a non-current list item, and a paragraph end tag reports when it
breaks out of MathML foreign content. A formatting end tag also reports when an
open table blocks adoption-agency recovery. Description-list item start tags
now report when implied-end-tag recovery closes a non-current `dt` or `dd`,
without flagging adjacent description-list items.

There are no residual malformed tree-construction cases without a lexer or
parser diagnostic. HTML `p` and `br` start tags recovered from seeded foreign
fragment contexts now report their foreign-content breakout parse error. A
second `body` end tag in a seeded `html` fragment reports its after-body
insertion-mode parse error, and non-whitespace text discarded by a seeded
`colgroup` context reports its table insertion-mode parse error.

Prioritized work items:

1. **Table, select, and template insertion modes.** Continue the algorithm
   audit beyond the currently exercised diagnostic corpus.
   Non-whitespace table text now reports when it is foster-parented. The
   specialized SVG and MathML start-tag path now reports when table insertion
   mode foster-parents foreign content, and table-context end tags now report
   when they force recovery from foreign content. Hidden `input` start tags in
   table structure now report their required parse error while retaining their
   special in-table insertion behavior. Foster-parented `title` and `meta`
   start tags now report the general in-table parse error. Nested `select` and
   `input` start tags now report when they force recovery from select mode.
   Repeated `option` and `optgroup` start tags now report the current-Standard
   parse error when implied-end-tag recovery finds an option or optgroup in
   select scope; the legacy html5lib error rows exercise the DOM recovery
   without declaring that branch error.
   Non-whitespace text after a template row now reports when it forces recovery
   from template table mode. Formatting start tags now report when table
   structure foster-parents them, covering the remaining silent fostered-anchor
   starts. Caption recovery now reports when a table-structure start tag or
   `table` end tag closes the caption while a non-implied node remains current;
   caption-scoped table endings now reprocess after closing the caption.
   Cell start tags now report when table-body recovery must synthesize a
   missing row, matching the current Standard and WPT's
   `unexpected-cell-in-table-body` evidence while leaving normal row and
   template-mode cells quiet.
   Nested `table` start tags now report before table-mode recovery closes the
   open table and reprocesses the token, matching the minimal WPT
   `tests6.dat` `<table><table>` evidence while leaving tables nested inside
   cells quiet.
   `form` start tags processed in table mode now report the required parse
   error both when the form pointer is initially null and when a repeated form
   is ignored, matching WPT `tests20.dat` `unexpected-form-in-table` evidence
   while leaving forms inside cells quiet.
   Non-hidden `input` start tags processed in table mode now report the general
   parse error before foster parenting, matching WPT `tests7.dat` and
   `webkit02.dat` evidence while preserving the specialized hidden-input path
   and leaving inputs inside cells quiet.
   Seeded table and foreign fragment-shell boundaries now report their required
   parse errors.
2. **Adoption agency and active formatting.** Cover malformed formatting cases
   without changing their now-conforming DOM output.
3. **Diagnostic positions and error taxonomy.** Carry source positions into
   tree construction and map diagnostics to current WHATWG concepts. Legacy
   WPT/html5lib error labels are evidence hints, not a normative public API.
4. **Input boundary review.** Document the Unicode-code-point parser boundary
   and either add or explicitly separate byte decoding and encoding sniffing.
5. **Algorithm and differential audit.** Map implemented states/modes to the
   current HTML Standard and add deterministic differential/fuzz coverage for
   branches not exercised by the upstream corpora.

## Intake Order

When a fresh upstream audit discovers work, add each bounded item here before
starting the next pull request and prioritize it in this order:

1. Missing executable WPT tree-construction cases.
2. Missing or skipped html5lib tokenizer cases.
3. Missing required tokenizer or tree-construction diagnostics.
4. Public DOM model gaps required by an upstream case.
5. Input-boundary or algorithm-coverage gaps.
6. Audit, fixture, or documentation drift that weakens reproducibility.

Keep one item per pull request. After each merge, rerun the upstream audit,
record newly discovered items, reprioritize this queue, and select the highest
remaining item.
