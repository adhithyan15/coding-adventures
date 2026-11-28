# HTTP/1.0 request-line assurance receipt

This receipt covers only `http-1.0-request-line-1996`, the HWEB08 closed
request-line subset. The independent suite is
`tests/test_http10_request_line_assurance.py`; portable data lives in
`code/specs/fixtures/historical-web-http10-request-line-v1/vectors.json`.

## Checks

- Replay every accepted and rejected fixture, requiring all fixture categories
  and their exact object shapes. The test also reads the existing HWEB00
  `historical-web-v1/vectors.json`: accepted H00 request lines must be rejected
  by HWEB08, and accepted HWEB08 lines must be rejected by the H00 parser.
- Use a separate grammar oracle based on explicit byte sets and field
  boundaries. Exhaust all 65,536 two-byte path suffixes and 4,096 one-byte
  variants across the target's leading slash, method positions, both
  separators, version bytes, and terminal framing bytes: 69,632 wire cases.
- Check exact built-in input types, exact output bytes, record equality and
  immutability, and the distinction between `TypeError` for type mismatches
  and `ValueError` for unsupported methods, targets, or wire syntax.
- Run real repository VER05 property campaigns over arbitrary request bytes
  and generated valid method/path pairs, plus a fixed-seed VER06 campaign from
  accepted HTTP/1.0, malformed HTTP/1.0, and legacy HTTP/0.9 request lines.
- Reuse the bounded SAT bridge in
  [`verification/sat_http10_request_line.py`](verification/sat_http10_request_line.py).
  Its claim is limited to GET with `/` plus zero to two allowed suffix bytes:
  a canonical HWEB08 line is SAT, the same wire is UNSAT for HWEB00, and
  weakening only HWEB00's non-space lower bound to admit SP is SAT. This is a
  bounded byte-language model, not a proof that parser implementations are
  equivalent or a proof over longer targets and other methods.

## Scope limits

The exhaustive tests cover the named one- and two-byte positions and a finite
path domain. PBT and fuzz runs are seeded finite campaigns. HWEB08 is not a
general RFC 1945 parser: it does not parse headers, bodies, absolute URIs,
queries, percent escapes, or extension methods. HWEB00 implementation behavior
is checked for version separation only; this suite does not redefine its wire
grammar.

## Validation snapshot

Against the read-only runtime at `9fd5aec3bfae88bc002ef4a5d6f5bcf6cb0acce3`,
the pinned Python 3.13.17 run passed the complete historical-web test directory:
79 tests passed, with 99% aggregate statement coverage and 100% coverage for
`http10_request_line.py`. Ruff check and formatting passed. The shared SAT
bridge reported SAT/UNSAT/SAT at each target length 1–3, including its stated
negative control.
