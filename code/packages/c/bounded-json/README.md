# Bounded JSON (C17)

A zero-external-dependency, owned JSON parser for explicit byte buffers. It is
the NN47 checkpoint-loading prerequisite, not native neural training, a Rust
binding, or six-native checkpoint conformance.

```c
#include "bounded_json.h"
#include <string.h>

const char *input = "[42,1.0,null]";
BJsonDoc *doc = NULL;
BJsonError error;
if (bj_parse(input, strlen(input), &doc, &error) == BJ_OK) {
    int64_t count;
    /* Only the first coordinate is an exact lexical integer. */
    int accepted = bj_integer(bj_at(bj_root(doc), 0), &count);
    (void)accepted;
}
bj_doc_free(&doc);
```

## Ownership and inspection

`bj_parse` uses default limits; `bj_parse_with_options` accepts `bj_options()`
with tighter bounds and/or an allocator/release pair plus context. Output must
point to a NULL document handle. Errors leave it NULL; an already-owned output
is rejected without losing it. `bj_doc_free` resets the handle and tolerates
NULL/empty/repeated release. Success owns every value/name/string: callers may
change/free the input immediately afterward. Const value views expire when the
document is released. There is no library file/network/process access.

`bj_root`, `bj_kind`, `bj_boolean`, `bj_integer`, `bj_number`, `bj_string`,
`bj_size`, `bj_at` and `bj_member` inspect values. Wrong types, NULL views/output
arguments and absent indexes fail without writing outputs; noncontainer size
is zero. Arrays and object members preserve source order. Duplicate names are
retained, not silently replaced. Strings/names carry explicit byte lengths and
may contain decoded NUL; do not use strcmp/strlen as a schema-key boundary.

Allocator callbacks must return suitably aligned fresh memory and remain valid
until release. Caller buffers must really contain the indicated readable bytes
and remain unchanged during parsing. Arbitrary forged C pointers are outside
the API contract. No hidden global parse state is used.

## Strictness and resource envelope

Defaults are hard ceilings: 4 MiB input, 128 nested containers, 65536 values and
16 MiB total live arena allocation (including document/allocation headers).
Options may tighten these; depth zero permits scalar roots only. Every size
and cumulative allocation is checked before allocation. Long strings are
validated/counted before an exact owned allocation. Cleanup walks allocations
iteratively rather than recursively traversing a partially built tree.

Exactly one JSON value and RFC whitespace are accepted. Invalid/truncated
UTF-8, overlong encodings, control characters, escapes, unpaired surrogates,
trailing documents and malformed numbers fail. Error kind and bounded byte
offset never copy input fragments into diagnostics.

Numbers are finite doubles; int64 lexical integers also preserve their exact
integer value. `1.0` and `1e0` remain floating forms, not integer metadata.
Out-of-int64 integer tokens may become finite floats, never wrapped integers.
Nonfinite overflow fails; finite underflow follows strtod rounding, including
signed zero. Decimal conversion uses the host's locale spelling without calling
setlocale; the application must externally serialize any concurrent global
locale changes (the C runtime's process-wide locale constraint).

Ordered member access is linear in its index. The parser itself appends members
without quadratic copying. This is a text parser, not a checkpoint schema:
all 55 NN40/NN43 fixture documents can be valid JSON even when a future dense
loader must reject their shapes, types or metadata.

## Build, install and evidence

`sh BUILD` and `BUILD_windows` use the existing shared ISO harness, strict C17,
warnings-as-errors and every available compiler. Linux CI requires GCC and
Clang; Windows uses MSVC/clang-cl, macOS Apple Clang. The Windows wrapper decodes
shared UTF-8 explicitly for PowerShell 5. The shared positive/negative compiler
guards include zero/one/multiple-compiler regressions.

Optional conventional static-library installation (normal BUILD validation
continues to use the ISO harness):

```sh
cmake -S . -B _build/install-build -DCMAKE_INSTALL_PREFIX=/chosen/prefix
cmake --build _build/install-build --config Release
cmake --install _build/install-build --config Release
```

The explicit payload is the static archive, `include/bounded_json.h`, exported
`coding_adventures::bounded_json` CMake target/config/version and license/docs/
capability metadata. Consumers use `find_package(bounded_json 0.1 CONFIG REQUIRED)`
and `target_link_libraries(client PRIVATE coding_adventures::bounded_json)`.
No math library, external JSON library or runtime binding is required.

Native tests cover values/grammar/Unicode/int64 limits/locale/ownership/accessor
bounds, exact envelopes, every allocation failure, 500 deterministic byte probes,
and parsing the shared fixture inventory. GCC library line coverage is measured
with gcov; Clang ASan/UBSan and actual MSVC tests are separate evidence. Run test
executables from the package directory so fixture paths remain valid.
See [NN47](../../../specs/NN47-c-bounded-json-foundation.md).
