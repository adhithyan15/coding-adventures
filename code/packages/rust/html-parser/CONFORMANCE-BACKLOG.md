# HTML Parser Conformance Backlog

Last audited: 2026-08-02

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

The 2026-08-02 upstream audit covered all 1,934 WPT tree-construction cases and
all 6,806 html5lib tokenizer cases with zero missing signatures and zero
normalized skips. DOM output is complete, but diagnostic coverage is not:
the checked 2,637-case tree corpus declares 6,243 errors across 2,183 cases.
After the after-after-frameset diagnostic slice, 1,794 of those cases emit at
least one lexer or parser diagnostic and 389 remain uncovered. Another 139
cases emit diagnostics despite having no legacy `#errors` rows. These are
reviewed rather than automatically removed: 89 are full-document inputs for
which the legacy fixtures omit the Standard-required missing-doctype error,
including the current processing-instruction cases.

Prioritized work items:

1. **Document shell insertion modes.** Emit the remaining parse errors around
   explicit and implied `html`, `head`, and `body` creation. Missing-doctype
   handling, nonconforming initial-doctype diagnostics, duplicate shell
   start-tag diagnostics, body/html end tags with disallowed open elements,
   full-document in-body EOF diagnostics, unexpected-token recovery in the
   after-body and after-after-body modes, and the implied-shell `</body>`
   transition, and after-after-frameset unexpected-token diagnostics are
   complete. The fresh 389-case inventory identifies stray end tags before an
   implied root or body and rejected frameset starts as the remaining concrete
   shell slices, in that order.
2. **Fragment EOF diagnostics.** Audit the current in-body EOF rule against
   fragment parsing. Legacy fragment fixtures omit many EOF error rows, so add
   explicit current-WHATWG evidence before extending the full-document
   diagnostic into fragments.
3. **In-body and text insertion modes.** Cover scope failures, implied-end-tag
   recovery, formatting reconstruction, stray start/end tags, and the large
   executable cluster of unclosed script/style/title/noframes text-mode EOF
   diagnostics.
4. **Table, select, and template insertion modes.** Cover foster parenting,
   table scopes, select recovery, and template mode-stack errors.
5. **Adoption agency and active formatting.** Cover malformed formatting cases
   without changing their now-conforming DOM output.
6. **Foreign content and fragment parsing.** Cover SVG/MathML integration
   boundaries and context-sensitive fragment errors.
7. **Diagnostic positions and error taxonomy.** Carry source positions into
   tree construction and map diagnostics to current WHATWG concepts. Legacy
   WPT/html5lib error labels are evidence hints, not a normative public API.
8. **Input boundary review.** Document the Unicode-code-point parser boundary
   and either add or explicitly separate byte decoding and encoding sniffing.
9. **Algorithm and differential audit.** Map implemented states/modes to the
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
