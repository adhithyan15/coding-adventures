# HWEB01 — bounded assurance contract for the historical-web reference

This contract applies VER05c (Python PBT) and VER06d (Python fuzzing) to the
public exports of the `historical-web` Python reference. It is an assurance
slice, not a universal proof and not a SAT certificate. The independent model
is deliberately smaller than the implementation: it checks wire framing,
ASCII admission, response decoding, and structural tree invariants without
reusing the parser's explicit byte/token scanning or stack.

## Threat classes and controls

| Threat class | Control and evidence |
| --- | --- |
| malformed wire accepted or valid wire rejected | exhaustive address alphabet/domain checks, negative framing cases, and PBT round trips through `encode_http09_request` and `parse_http09_request` |
| response bytes silently changed or decoded outside ASCII | exhaustive bytes over a bounded alphabet, invalid-byte negatives, and PBT identity checks through `decode_http09_response` |
| parser crashes, loses text, or violates tree shape on hostile markup | VER06d real-export fuzz campaign over `parse_html_tags_1992`, plus independent recursive tree invariants |
| omission/raw-text/attribute scope regression | explicit boundary vectors and negative controls for unmatched ends, comments, raw text, plaintext, implied closes, and first-anchor-attribute wins |
| harness false confidence | test-bootstrap `check_property` and `fuzz_function` are called directly; the model does not call production tokenization helpers |

## Explicit bounds

The exhaustive slice enumerates all HTTP addresses of length 0–3 over the
ASCII alphabet `a`, `/`, `?`, and `0` (85 cases including invalid empties),
and all response byte strings of length 0–2 over `{0x00, 0x41, 0x7f}` (13
cases, with non-ASCII negatives). PBT runs 256 deterministic cases for each
of three properties: wire acceptance/output, request round trips, and omitted
`LI` semantics (768 property cases total). The wire property uses an
independent acceptance model for arbitrary 0–32-byte inputs and checks exact
outputs and refusals. Fuzz runs 512 iterations, with initial seeds no longer than 64
bytes and `max_input_len=128`; these bounds are campaign receipts, not claims
about unbounded inputs. Tree-depth checks cap generated nesting at 12.

The profile intentionally ignores unmatched recognized end tags, preserves
non-matching `<` text, decodes only the three archived entities once, and
accepts CRLF or LF request termination. `plaintext` consumes the remainder;
`xmp` and `listing` consume through their case-insensitive matching end tag.
These are contract assumptions tested by boundary cases, not general HTML5
error-recovery claims.

## Evidence boundary

The tests establish bounded examples, PBT verdicts, fuzz outcomes, and model
invariants for the named exports. They do not establish SAT/DRAT evidence,
Warrant/source-level proof, native coverage, or behavior beyond the stated
finite domains and campaign budgets. SAT scope remains pending an independent
model and must not be inferred from these tests. CWE-400 is addressed only by
the explicit input/depth/iteration bounds; no claim is made about process-wide
memory or wall-clock exhaustion outside those bounds. CWE-20 is a tested
negative-control class for this profile, not a universal parser-hardening
claim.
