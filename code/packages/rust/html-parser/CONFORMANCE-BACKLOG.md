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
`d6c801946a63a6c5fec07c3d476b8b021e5eca34` and html5lib-tests
`224991ec10db04f056a89eed8b0bd8695fd2950e` covered all 1,934 WPT
tree-construction cases and all 6,806 html5lib tokenizer cases with zero missing
signatures and zero normalized skips. DOM output is complete, but diagnostic
coverage is not:
the checked 2,637-case tree corpus declares 6,243 errors across 2,183 cases.
After the rejected-frameset diagnostic slice, 1,837 of those cases emit at
least one lexer or parser diagnostic and 346 remain uncovered. Another 139
cases emit diagnostics despite having no legacy `#errors` rows. These are
reviewed rather than automatically removed: 89 are full-document inputs for
which the legacy fixtures omit the Standard-required missing-doctype error,
including the current processing-instruction cases.

The concrete document-shell insertion-mode inventory is complete: missing and
nonconforming doctypes, duplicate shell tags, body/html boundary errors,
after-body and frameset modes, implied-shell paragraph end tags, and rejected
frameset starts all report their current-Standard parse errors.

Prioritized work items:

1. **Fragment EOF diagnostics.** Audit the current in-body EOF rule against
   fragment parsing. Legacy fragment fixtures omit many EOF error rows, so add
   explicit current-WHATWG evidence before extending the full-document
   diagnostic into fragments.
2. **In-body and text insertion modes.** Cover scope failures, implied-end-tag
   recovery, formatting reconstruction, stray start/end tags, and the large
   executable cluster of unclosed script/style/title/noframes text-mode EOF
   diagnostics.
3. **Table, select, and template insertion modes.** Cover foster parenting,
   table scopes, select recovery, and template mode-stack errors.
4. **Adoption agency and active formatting.** Cover malformed formatting cases
   without changing their now-conforming DOM output.
5. **Foreign content and fragment parsing.** Cover SVG/MathML integration
   boundaries and context-sensitive fragment errors.
6. **Diagnostic positions and error taxonomy.** Carry source positions into
   tree construction and map diagnostics to current WHATWG concepts. Legacy
   WPT/html5lib error labels are evidence hints, not a normative public API.
7. **Input boundary review.** Document the Unicode-code-point parser boundary
   and either add or explicitly separate byte decoding and encoding sniffing.
8. **Algorithm and differential audit.** Map implemented states/modes to the
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
