# Historical Web Ruby verification receipt

`test/test_assurance.rb` replays every shared `historical-web-v1` HTTP, HTML,
and exchange vector. Fixture loading requires each category and rejects malformed
or odd-length hex. Request parsing receives the fixture bytes unchanged, while
the encoder is checked separately for canonical CRLF, including the shared
accepted LF vector. Response text and complete nested trees are compared.

An independent framing oracle exhausts all octet payloads of lengths zero
through two with LF and CRLF framing: **131,586 cases**. It checks acceptance
and the exact returned address. VER05h runs 256 deterministic printable
address round trips. VER06i runs 2,048 mutations with fixed seeds and a 256-byte
input cap; each mutation is checked as request bytes and response bytes, and
every valid UTF-8 mutation must parse as recoverable HTML. The independent HTML
arena check is iterative and validates indices, reachability, single ownership,
acyclicity, node kinds, and recognized attributes.

Boundary regressions cover frozen ASCII-8BIT address and request snapshots,
forged allocated addresses, raw octet parsing, US-ASCII response output without
transcoding input tagged UTF-16LE, accepted UTF-8/US-ASCII HTML encodings and
rejected invalid/unsupported encodings, 12,000-level nesting with repeated
unmatched known closes, 20,000 malformed tag-like prefixes, omitted SGML list
endings, empty `P` markers with following sibling text, first-wins valueless
anchor attributes, CRLF inside quoted attributes, and multibyte UTF-8 near
raw-text search endpoints.

The narrow HTTP SAT model remains the shared Python `sat_http09.py` harness,
delegating to the repository Go CDCL solver. This Ruby lane does not duplicate
the solver; its independent exhaustive replay checks the Ruby public parser.
The SAT scope remains bounded and is not a DRAT certificate or source proof.

The package library is in-memory and has no `eval` or shared-state key writes;
the verification suite reads only the committed fixture. The Ruby framework
does not yet provide a live per-package analyzer or source-level proof for
applicable CWE-94/CWE-1321; this receipt records review evidence, not formal
exclusion of those classes. These tests establish only the finite domains,
seeds, and budgets listed above.
