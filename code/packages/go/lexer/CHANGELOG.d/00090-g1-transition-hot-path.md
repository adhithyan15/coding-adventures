# G1 transition matching avoids rule copies

`GrammarLexer.applyTransitions` now walks the immutable ordered transition table
by index, avoiding a `ModeTransition` value copy for every token, and rejects a
nonempty mode guard before scanning that rule's token list. Rule priority,
wildcard modes, value guards, first-match behavior (including an empty-action
winner), and ordered action execution are unchanged. A frozen-oracle comparison
suite covers deterministic bounded cases and Go fuzz inputs.

The unchanged G1 gate passed all A1/B1/B2/A2 cells on Ubuntu 24.04 and 26.04
in run `44370322596`. The separate BestPass1 helper mean was 5.94% lower on
the Xeon 8573C host and 0.39% lower on the EPYC 9V45 host, within that host's
observed spread. These results do not establish a general gain or rescue the
historical EPYC 7763 slow-host miss; G1 host robustness remains open. The
production gate, 14 MB/s floor, input, and default CI selection are unchanged.
