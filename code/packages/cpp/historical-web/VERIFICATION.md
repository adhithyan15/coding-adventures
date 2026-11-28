# Historical Web C++ verification

This package-level assurance uses the public C++17 API and the shared
`historical-web-v1` fixture. It does not link to or call the C parser. The
fixture reader and hex decoder fail closed on a missing file, malformed JSON,
odd-length hex, or non-hex bytes. Category counts are pinned so an accidentally
empty fixture section cannot pass.

## Run

From `code/packages/cpp/historical-web`, `sh tools/run.sh` builds and runs the
suite under every available ISO C++ compiler. It compiles the VER05/VER06 C
engines as strict C17 translation units, then links them to the C++17 test and
runtime. Each compiler is also probed for a runnable ASan+UBSan pass; when the
sanitizer runtime is available, the full suite runs with both sanitizers and
recovery disabled. `BUILD_windows` selects `tools/run.ps1`, which compiles the C
engine as C, compiles the runtime and tests as C++17, then links them with
MSVC-family tools. The Windows entry is authored against the repository ISO
harness but was not run on a Windows host in this audit.

An optional Python differential audit is intentionally outside `BUILD`:

```sh
CXX=clang++ PYTHON=/path/to/python3.13 sh tools/run_python_parity.sh
```

It builds a small adapter over the public C++ parser, then compares its trees
with the Python reference. Python 3.12 or newer is required by the reference
package. Keeping this audit optional allows package builds that project only
the C++ sources and shared JSON fixture.

## Results

Against runtime commit `3f5dcc22d790524c84c709f1aeb5eaf45e5a070c`, the Linux
run passed with GCC 14.2.0 and Clang 19.1.7, including ASan+UBSan for both
compilers. The Python reference audit matched 5,014 documents: all 14 shared
HTML/exchange documents plus 5,000 deterministic grammar-bearing generated
documents (seed `0x485745423039`). No tree mismatches or sanitizer findings
were reported.

The suite checks:

- Every HTTP fixture category and all HTML/exchange trees, including request
  and response bytes. It verifies 3 accepted requests, 6 invalid addresses,
  8 invalid requests, 3 response bodies, 2 invalid responses, 13 HTML cases,
  and 1 complete exchange.
- All 131,586 HTTP short-wire pairs: every payload of length 0, 1, or 2 over
  all octet values, under LF and CRLF termination. The oracle independently
  peels one terminal CRLF before LF, then checks the exact `GET ` prefix and
  the nonempty printable-ASCII address range `0x21..0x7e`. Accepted return
  strings must equal the oracle address; rejected inputs must raise the typed
  invalid-request error.
- VER05 property campaigns with fixed seeds: 500 arbitrary byte addresses
  check encoder rejection or canonical encode/parse round trip; 300 generated
  nonempty printable addresses check exact round trips; 400 arbitrary HTML
  byte strings either recover to a structurally valid arena or fail only with
  the documented invalid-UTF-8 error.
- A VER06 campaign with fixed seed `0x485745423039`, 1,200 iterations,
  maximum input length 96, step budget 500,000, and memory budget 64 MiB.
  The target checks HTTP results against the independent byte oracle and
  checks HTML arena links. Named coverage edges distinguish accepted/rejected
  HTTP requests, recovered/invalid-UTF-8 HTML, roots, attributes, and children.
- HTML behavior around omitted list/glossary closes, nested list scopes, raw
  UTF-8 text at an unaligned end candidate, valueless and empty anchor values,
  malformed anchor values, comments/entities, embedded NUL, and valid
  two/three/four-byte UTF-8. It checks exact byte offsets for invalid or
  truncated UTF-8 and protocol errors.
- Flat-arena parent/child/sibling range and forward-index invariants, ownership
  after source destruction, copy/move assignment, out-of-range accessors, and
  a 12,000-level ADDRESS tree with 4,096 unmatched `</A>` misses before close.
  Construction and document destruction complete without recursive ownership.

## Bounded proof scope

The related C++ SAT model in the separate verification lane represents one
three-bit bounded append step: a root, first child, and sibling have in-range
indices and correct parent/backlink relationships. The intended constraints
are satisfiable; forcing a referenced link to the one-past-count value is
unsatisfiable. Removing the existing-parent precondition admits the negative
control where the new node becomes its own child. This checks the local append
relation and its negative control. It does not prove parser behavior for all
inputs, arbitrary vector growth, C++ allocation behavior, or traversal
termination; those claims are exercised by the independent runtime tests
above.
