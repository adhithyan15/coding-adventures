# bounded-json (native C++17)

An independent, header-only JSON parser in `ca::bounded_json`. This is NN50's
text/ownership prerequisite, **not** a checkpoint schema loader, training
framework, C wrapper or Rust binding. It uses only the C++17 standard library.

```cpp
#include "bounded_json.hpp"
auto result = ca::bounded_json::parse("{\"step\":3000}");
if (result) {
    const auto member = result.document->root().member(0);
    // member.first == "step"; member.second->integer() == 3000
}
```

## Ownership and errors

Successful parsing returns a unique document owning all nodes, strings and
object names. Input may immediately be changed or destroyed. Documents/values
cannot be copied or moved directly; moving the document's `unique_ptr` keeps
views valid. All views expire at document destruction. There is no mutable node
API. Arrays use `at(index)`; objects expose ordered `member(index)` pairs, with
duplicates and embedded NUL names preserved. Views always carry lengths.

Wrong-kind access throws `std::logic_error`; bad indices throw
`std::out_of_range`. Parsing failures instead return no document and a closed
`Status` plus byte offset at or before input end, with no source fragments.
The input `string_view` must designate readable storage for the call; a default
empty view is permitted and produces a syntax error.

`integer()` accepts only int64 lexical integers. `1.0` and `1e0` remain floating,
and booleans/null never become numeric metadata. Larger integer lexemes may
become finite doubles, not wrapped integers. `number()` also accepts int64 values
with the ordinary double rounding. Floating conversion uses locale-independent
`std::from_chars`: overflow and underflow reported as out-of-range are rejected;
representable subnormals are accepted. Floating `-0.0` preserves its sign;
lexical integer `-0` is the integer zero. NaN/infinity are not JSON values.

## Actual allocation envelope

`Options` defaults to hard ceilings of 4 MiB input, 128 nested containers,
65536 nodes and 16 MiB simultaneous allocation requests. Limits can only be
tightened; zero depth admits scalars but no containers. Each node and every
dynamic decoded-string/name buffer goes through one capped memory resource.
Temporary string growth counts both old and replacement buffers until release.
`peak_allocation_bytes()` reports measured requested bytes, not estimated
payload size. Linked container edges reside inside the counted nodes.

The cap excludes one fixed-size `Document` allocation, allocator bookkeeping,
stack frames, caller-owned input, and implementation-internal standard-library
conversion/runtime machinery; it is not a process-RSS promise. Stack recursion
is independently bounded. Accessor exception messages are not parse storage.
Optional `upstream` supports deterministic allocation-failure testing and must
obey `memory_resource`'s contract and outlive the document. All partial nodes are
released iteratively on every error. No global locale mutation, I/O or process
execution occurs in the library.

## Build and validation

Run `sh BUILD` on POSIX, or `powershell -File tools/run.ps1` on Windows from this
package. Both delegate compiler discovery/strict flags to `c/iso-harness`.
Linux CI requires both GCC and Clang. The tests link two translation units,
including an independent public-header consumer. No external JSON library or
extra compiler scheduler is installed.

Initial local Windows-hosted POSIX-front validation: GCC 474 checks, Clang 498
checks, zero failures. Counts differ because allocator growth strategies produce
different numbers of injected failure positions. GCC gcov measured **98.71% of
232 library lines**, 100% branches executed and 93.68% taken at least once.
Clang AddressSanitizer also passes 498 checks. These are not Linux/macOS/MSVC
or UBSan claims; those remain pending. Local MSVC is blocked before package
compilation by the installed SDK's missing `crtdbg.h`.

CMake can install the public-header interface target:

```sh
cmake -S . -B _build/install -DCMAKE_INSTALL_PREFIX=/chosen/prefix
cmake --install _build/install
```

Consumers use `find_package(ca_bounded_json CONFIG REQUIRED)` and link
`ca::bounded_json`. Actual GCC CMake installation and a separate `find_package`
consumer compile/link/run pass locally.

See [NN50](../../../specs/NN50-cpp-bounded-json-foundation.md) for the complete
delivery boundary. Schema rejection and native C++ training come later.
