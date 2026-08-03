# HTML Parser Conformance Backlog

Last audited: 2026-08-02

## Completion Boundary

The parser conformance loop is complete when a fresh audit against current WPT
tree-construction resources and html5lib tokenizer tests reports:

- zero missing WPT tree-construction signatures
- zero missing html5lib tokenizer signatures
- zero normalized tokenizer skips
- all checked parser, lexer, fixture, formatting, and lint gates pass

The checked `tests/fixtures/html5lib-coverage-audit.json` report records the
exact upstream commits used for the latest completed audit.

## Prioritized Queue

There are no open conformance items. The 2026-08-02 audit covered all 1,934 WPT
tree-construction cases and all 6,806 html5lib tokenizer cases with zero missing
signatures and zero normalized skips.

## Intake Order

When a fresh upstream audit discovers work, add each bounded item here before
starting the next pull request and prioritize it in this order:

1. Missing executable WPT tree-construction cases.
2. Missing or skipped html5lib tokenizer cases.
3. Public DOM model gaps required by an upstream case.
4. Audit, fixture, or documentation drift that weakens reproducibility.

Keep one item per pull request. After each merge, rerun the upstream audit,
record newly discovered items, reprioritize this queue, and select the highest
remaining item.
