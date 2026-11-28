# Historical Web C# verification

The assurance project calls the public HWEB10 API and replays the shared
`historical-web-v1` vectors. It does not call the Python or other language
implementations from the normal test build. Fixture discovery walks upward
from `AppContext.BaseDirectory`; a missing fixture, required category, malformed
JSON, odd-length hex, or non-hex fixture byte fails the test run.

## Build and coverage

From `code/packages/csharp/historical-web`, `BUILD` runs the package xUnit
project with Coverlet's MSBuild integration. `BUILD_windows` selects the
PowerShell equivalent. The coverage include filter targets only the production
assembly `CodingAdventures.HistoricalWeb.CSharp`, and both fronts enforce at
least 95% line coverage. The project uses the existing C# test-bootstrap for
VER05 and VER06; no new runtime dependency is introduced.

Against runtime commit `720891192cedf6e996142a370e6dbc69dd56f53b` (the
verification was repeated against integration head `7111b4307fc0633e1f4a664749e5766f6ebb767b`),
the pinned .NET SDK 9.0.319 run passed 9 xUnit tests. Coverlet reported 98.58%
production line coverage, 97.72% branch coverage, and 96.72% method coverage.
The same single `dotnet test` command passed after clearing package, test, and
test-bootstrap `bin`/`obj` outputs; a separate prebuild wrapper was not needed.

The test suite covers:

- All shared HTTP categories and HTML trees, with pinned counts of 3 accepted
  requests, 6 invalid addresses, 8 invalid requests, 3 response bodies, 2
  invalid responses, 13 HTML parser vectors, and 1 full exchange.
- All 131,586 HTTP payload/framing pairs for payload lengths 0 through 2 and
  LF/CRLF endings. The oracle independently checks the exact prefix, one
  terminal framing sequence, and nonempty printable ASCII `0x21..0x7e`; accepted
  output bytes must match exactly and rejects must have the typed request error.
- VER05 properties: 500 arbitrary byte-address cases, 300 generated valid
  printable-address round trips, and 400 arbitrary byte sources mapped to
  valid Latin-1 strings. Successful HTML parses must satisfy independent flat
  arena reachability, index, parent, sibling, and attribute-range invariants.
- VER06 fuzzing: 1,200 iterations with seed `0x485745423130`, input cap 96,
  step budget 500,000, and 64 MiB memory budget. Every request is checked by
  the independent byte oracle; every HTML arena is checked structurally.
  Semantic edges distinguish accepted/rejected requests and recovered HTML
  trees with roots, recognized attributes, and children.
- An exhaustive one-code-unit UTF-16 sweep: all 65,536 code units are tested;
  the 2,048 isolated surrogates must raise `InvalidUnicode` at offset 0, while
  the other 63,488 units remain text. Additional tests cover valid surrogate
  pairs and non-BMP Unicode, unpaired high/low surrogates at other offsets,
  embedded NUL, raw-text Unicode boundaries, malformed anchor values, first-wins
  duplicate attributes, CRLF, empty input, and null input.
- Byte-array snapshot behavior, immutable HTML collection surfaces including
  array/list/non-generic collection and `SyncRoot` checks, accessor bounds,
  and a 12,000-level ADDRESS tree with 4,096 unmatched end-tag lookups.

The optional `tools/run-python-parity.sh` builds a small console adapter and
compares the C# parser with the Python reference on all 14 shared HTML/exchange
documents plus 5,000 deterministic grammar-bearing documents. With the pinned
Python 3.13.17 and .NET 9.0.319, all 5,014 trees matched (seed
`0x485745423130`). The optional audit is outside `BUILD` so the package build
does not depend on Python source being present.

A disposable negative control changed the fixture request count assertion
from 3 to 4. The selected xUnit test failed, and `dotnet test` exited 1; the
original source was restored before the passing package run.

## Bounded proof limits

[`verification/README.md`](verification/README.md) describes the optional
symbolic model for checked 32-bit request-length additions. It verifies the
successful checked-add domain and a negative control with the upper-bound guard
removed. It does not prove .NET arithmetic execution, span contents, allocator
success, maximum array-size policy, or the entire HTTP parser; those remain
runtime properties outside the model.
