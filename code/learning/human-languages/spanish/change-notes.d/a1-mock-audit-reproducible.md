# The A1 mock audit is now a maintained artifact

The original A1 sitting documented its scoring policy but left the mechanical
scripts in a scratchpad. That made later curriculum work rely on a careful
reconstruction and left a one-item ambiguity in the historical baseline.

The new `spanish-a1-mock-audit-cli` turns the policy into maintained code. It
loads the current Spanish A1 path, normalizes lesson headwords, applies the
documented citation-form and numeral credits, parses both committed answer
keys, and scores an objective item only when every required lexeme is present.

The generated `mocks/a1/book-bounded-audit.json` is deliberately detailed: it
contains every failed item and missing lexeme, not just headline totals. A
check command and focused tests make future vocabulary tranches update the
same evidence instead of inventing a new scratch harness.
