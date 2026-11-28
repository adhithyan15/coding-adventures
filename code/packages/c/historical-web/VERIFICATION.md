# C historical Web verification

This receipt describes the C port's independent checks. The unit suite is
`tests/historical_web_test.c`, compiled together with the production sources by
the package BUILD target. It is a consumer of the public header; it does not
reach into parser internals.

## Evidence

- Replay each category in `code/specs/fixtures/historical-web-v1/vectors.json`
  using the test-only bounded JSON reader. Missing or malformed required
  vectors fail the test rather than being skipped. Request encoding is
  compared to canonical CRLF; request parsing separately checks accepted LF.
- Independently check all 131,586 request/framing combinations over payload
  lengths zero, one, and two: `(1 + 256 + 256²)` payloads each with LF and
  CRLF framing. The oracle enforces the exact `GET ` prefix, one terminal line
  ending, and the printable ASCII address domain, with terminal CR interpreted
  as CRLF framing.
- Exercise output handles initialized to NULL, preservation of pre-owned
  outputs on rejected calls, stable canaries, returned byte ownership, and
  HTML document view lifetime after input mutation/free. Cover null output and
  input pairs, zero-length inputs, and exact protocol/UTF-8 diagnostic offsets.
- Check HTML fixtures and focused regressions for empty `<P>` markers,
  SGML-style omitted list ends, first recognized duplicate/valueless anchor
  attributes, literal XMP/LISTING/PLAINTEXT regions, UTF-8 near raw end-tag
  search boundaries, embedded NUL text, malformed source recovery, and the
  valueless recovery for `<A HREF=>` and `<A HREF==x>`.
- Walk deep nested and malformed documents iteratively through public node
  accessors. Check parent/child/sibling/root bounds, reachability, and UTF-8
  span validity; repeated unmatched `</A>` tags inside the nested ADDRESS
  chain check indexed miss handling. Release the result to exercise iterative
  cleanup.
- Run deterministic C VER05i properties and VER06j fuzz campaigns against the
  exported API. Fuzz outcomes include parser success and documented rejection;
  an unexpected failure status or invalid returned arena is a crash. Corpus
  seeds and explicit edge labels cover request framing and HTML mode paths.

## Limits

The short-wire enumeration covers payload lengths zero through two, not
arbitrary length strings. Generated properties and fuzzing are bounded
deterministic campaigns, not proofs over all source documents. The shared
byte-admission SAT model and
[`verification/verify_size_arithmetic.py`](verification/verify_size_arithmetic.py)
are finite evidence for HTTP admission and full-width size arithmetic. They do
not prove C source pointers, HTML behavior, allocation, ownership, or arbitrary
length inputs. No solver is duplicated in this package. These checks are
evidence for the specified profile, not a claim of complete protocol or browser
conformance.

An optional GNU ld allocation-failure sweep is documented in
[`verification/test_allocation_failures.c`](verification/test_allocation_failures.c).
It fails each real allocation in turn for representative public API calls and
checks that failures return `HWEB_ERR_NO_MEMORY` without exposing partial
outputs. It is deliberately not part of the portable package test target.

On the GCC 14.2 snapshot used for this receipt, gcov line coverage was 95.79%
for HTTP and 92.00% for HTML from the normal test target. Combining the normal
suite with the optional allocation-failure sweep raised those figures to
98.95% and 95.71%, respectively. These measurements exclude test-bootstrap
and bounded JSON lines.

## Local validation

The package BUILD recipe is the source of truth for compiler flags and test
translation units. Run its normal test target with GCC 14.2, then repeat with
AddressSanitizer and UndefinedBehaviorSanitizer enabled where supported. The
sanitizer run is kept separate from coverage-instrumented fuzzing.
