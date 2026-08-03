# HTML Parser Conformance Backlog

Last audited: 2026-08-03

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

The 2026-08-03 upstream audit at WPT
`a73cf1e91a6a95e4c5c39494d8fbfdab0b38cae1` and html5lib-tests
`224991ec10db04f056a89eed8b0bd8695fd2950e` covered all 1,934 WPT
tree-construction cases and all 6,806 html5lib tokenizer cases with zero missing
signatures and zero normalized skips. DOM output is complete, but diagnostic
coverage is not:
the checked 2,637-case tree corpus declares 6,243 errors across 2,183 cases.
After the specialized in-body list-item start-tag diagnostic slice, 2,007 of
those cases emit at least one lexer or parser diagnostic and 176 remain
uncovered.
Another 139 cases emit diagnostics despite having no legacy `#errors` rows.
These are reviewed rather than automatically removed: 89 are full-document
inputs for which the legacy fixtures omit the Standard-required missing-doctype
error, including the current processing-instruction cases.

The concrete document-shell insertion-mode inventory is complete: missing and
nonconforming doctypes, duplicate shell tags, body/html boundary errors,
after-body and frameset modes, implied-shell paragraph end tags, and rejected
frameset starts all report their current-Standard parse errors.

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
specialized paragraph and adoption-agency branches. Heading end tags now report
the specialized parse error when no heading is in scope or the current heading
does not match the token. A `li` start tag now also reports the specialized
parse error when its implied-end-tag recovery closes a non-current list item.

Prioritized work items:

1. **In-body insertion mode.** Cover specialized list-item, paragraph,
   adoption-agency, formatting-reconstruction, and stray-tag diagnostics.
2. **Table, select, and template insertion modes.** Cover foster parenting,
   table scopes, select recovery, and template mode-stack errors. The remaining
   fragment EOF inventory includes fostered-anchor `eof-in-table` rows and
   MathML/SVG table-boundary scope recovery.
3. **Adoption agency and active formatting.** Cover malformed formatting cases
   without changing their now-conforming DOM output.
4. **Foreign content and fragment parsing.** Cover SVG/MathML integration
   boundaries and context-sensitive fragment errors.
5. **Diagnostic positions and error taxonomy.** Carry source positions into
   tree construction and map diagnostics to current WHATWG concepts. Legacy
   WPT/html5lib error labels are evidence hints, not a normative public API.
6. **Input boundary review.** Document the Unicode-code-point parser boundary
   and either add or explicitly separate byte decoding and encoding sniffing.
7. **Algorithm and differential audit.** Map implemented states/modes to the
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
