# G1 transition matching avoids rule copies

`GrammarLexer.applyTransitions` now walks the immutable ordered transition table
by index, avoiding a `ModeTransition` value copy for every token, and rejects a
nonempty mode guard before scanning that rule's token list. Rule priority,
wildcard modes, value guards, first-match behavior (including an empty-action
winner), and ordered action execution are unchanged. A frozen-oracle comparison
suite covers deterministic bounded cases and Go fuzz inputs.

This is a performance candidate only. No throughput gain is claimed until the
same-runner, two-host A1/B1/B2/A2 experiment and unchanged official G1 gate are
complete.
