# F# HWEB00 assurance receipt

The test assembly exercises the public `CodingAdventures.HistoricalWeb.FSharp`
API independently of the Python reference. `BUILD` and `BUILD_windows` use
the pinned .NET 9 test toolchain and fail below 95% line coverage for the
production assembly only.

## Bounded checks

- Shared `historical-web-v1/vectors.json`: replay every HTTP success/rejection,
  response and HTML tree case. Missing fields, malformed hex, or an absent
  fixture fail closed.
- HTTP parser: exhaustive all payload byte strings of lengths zero through two
  under LF and CRLF: `(1 + 256 + 256²) × 2 = 131,586` cases. An independent
  byte oracle checks accepted address contents and exact failure class.
- UTF-16: all 65,536 one-code-unit strings; all 2,048 isolated surrogate
  values must fail at offset zero, all other code units must round trip as
  text. Additional tests cover scalar pairs, NUL, attributes, malformed
  markup, raw-text modes, diagnostic offsets, and a deeply nested flat arena.
- VER05l and VER06m bootstrap bindings use deterministic seeds and semantic
  feedback. Coverage is caller-reported and remains separate from Coverlet's
  source line metric.
- The F# HTTP oracle is also applied to every arbitrary fuzz input, including
  wrong prefixes. It distinguishes LF from CRLF: a CR immediately before LF is
  framing; a CR before an explicit CRLF terminator remains address content.
- `tests/parity_audit.py` optionally compares all shared HTML source cases and
  5,000 fixed-seed grammar-bearing mixed documents against the Python reference.
  Its subprocesses have a 180-second bound and honor `PYTHON` and `DOTNET`.
- [`verification/README.md`](verification/README.md) records the symbolic
  UTF-16 high-surrogate two-code-unit advance proof. It checks that the guarded
  `remaining - 2` transition cannot advance beyond the source; removing the
  next-unit bounds condition produces a SAT counterexample. This is a one-step
  range model, not a parser-wide Unicode proof.

Python parity is an optional local audit; it is not part of either portable
build front because its reference runtime is not a declared package input on
all build hosts. The receipt must record the exact corpus seed/count and
Python/.NET versions if run. No F# runtime behavior is inferred from parity.

## Recorded run

On .NET SDK 9.0.319, `CI=true sh BUILD` passed 13 tests and measured 98.43% line,
94.79% branch, and 95.91% method coverage for only
`CodingAdventures.HistoricalWeb.FSharp`. A temporary assertion mutation from
131,586 to 131,585 caused its xUnit test to fail (1 failed, 12 passed). Setting
the threshold to 100% also exited nonzero at 98.43%, confirming coverage gate
propagation. The optional parity command matched 5,014 sources (all fixture
HTML sources plus 5,000 generated), seed `0x48454211`, using Python 3.13.17 and
.NET 9.0.319. Both checks ran against production commit
`fcb27850ed94b571cd0d7b7002873f15d5736ccb`.

The full-wire HTTP oracle also guards the framing distinction that stripping
both the explicit CRLF terminator and a preceding payload CR would be wrong.
