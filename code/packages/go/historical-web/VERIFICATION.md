# Historical-web Go verification receipt

The independent test suite replays every shared HWEB00 HTTP, HTML, and
combined-exchange vector. It also exhaustively checks all 256-byte payloads of
length 0, 1, and 2 with both LF and CRLF framing: 131,586 wires. The oracle
checks both acceptance and the exact returned address; it does not call the
production encoder or parser to decide expected results.

The suite's deterministic property loop performs 1,000 printable-address
round trips and checks four malformed-wire negatives. Native Go fuzz targets
were run with the repository's pinned Go toolchain for three seconds each:

| Target | Executions | Result |
| --- | ---: | --- |
| `FuzzHTTP09Parser` | 194,354 | pass |
| `FuzzHTTP09Encoder` | 226,381 | pass |
| `FuzzHTTP09Response` | 181,011 | pass |
| `FuzzHTMLParser` | 132,313 | pass |

The HTML fuzz invariant is iterative: every emitted element has a lowercase,
non-empty tag and every child is a recognized text or element node. These are
bounded conformance and campaign receipts, not source-level proof or claims
about behavior outside the stated domains.

Focused parser regressions also cover CRLF normalization inside quoted
attributes, valueless attributes followed by another attribute, malformed
attribute prefixes, deeply nested trees, and adjacent text around malformed
tags. Run them with `go test -run '^TestHTMLParserRegression' .` from this
package directory.
