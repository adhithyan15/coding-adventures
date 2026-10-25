# NN47: Bounded JSON Foundation for Native C Training

Status: specified before implementation; implemented and locally validated;
publication and exact-head CI pending.

### Exact-head CI scheduling repair

Initial head e20ded7a22e passed applicable CI, but the planner omitted Windows
for C-only inputs. Linux and macOS each actually built bounded-json and the ISO
harness; that is not MSVC CI evidence. Before merge, expose a narrow plan-derived
Windows requirement when c/bounded-json is affected (fail closed on an unreadable
plan), retain the existing Windows runner and MSVC bootstrap, and run the shared
harness self-test and this package's existing Windows front there. Pin selection,
bootstrap, execution and required-build-rollup ownership with executable planner
tests. Do not broaden all C/C++ scheduling or introduce another compiler scheduler.

The repair passes six executable production-planner/ownership tests plus the
existing four neural-gate tests. GCC/Clang each repeat 1605 checks; actual local
MSVC repeats the same 1605 plus shared C/C++ positive/negative fixtures and six
cardinality guards through the new step's commands. Windows CI remains pending
until the repaired head actually runs. Initial Linux/macOS package BUILD success
and the neutral CodeQL workflow (no configured C/C++ analyzer) are not C scanning
or Windows execution evidence.

Repaired-head Windows job121319644429 then confirmed that clang-cl accepted
both negative GNU-extension fixtures under the shared MSVC-only flags, while
cl rejected them and both compilers passed positive fixtures. Extend the shared
flag helper with compiler-specific Clang pedantic diagnostics in positive and
negative paths; preserve the existing standard, warning and error flags, and
all installed compiler discovery. Test flag selection plus real cl/clang-cl
accept/reject and full parser execution before a new exact-head review/push.

## Reassessment after five-native delivery

NN46 merged PR #18252 at exact head `ff37a3e44e3`, squash `da1b408680f`,
after every applicable CI/CodeQL check passed. Selected conformance executed
all twenty directed pairs through 3002, five native lanes and no bindings;
actual package builds passed on Linux, macOS and Windows. The final roll-up
recorded required:true/result:success. Runner scheduling delayed the verdict,
not a source defect, so no source changes or cancellations were made.

Reprioritize reliability, prerequisites, portability and smallest complete
delivery. C already has independent double-precision matrix and float-math
foundations and the strict ISO multi-compiler harness. Its `neural-network`
package is a topology/property graph, explicitly not trainable dense layers.
There is no native C JSON package in the current package inventory. Safely
loading bounded, typed, owned checkpoints is therefore a prerequisite: deliver
one reusable bounded JSON parser before a separate C dense training tranche.
Do not wrap Rust, introduce an external JSON/ML dependency, or claim C training
or six-native interchange from this foundation.

## Public C17 ownership contract

Create `code/packages/c/bounded-json` in pure ISO C17 with an installable public
header and standalone source. Use explicit status codes and an opaque owned
document with const value views: null, boolean, number, UTF-8 string, ordered
array and ordered object members. Retain duplicate object names in source order
so a later closed checkpoint loader can reject duplicate/unknown fields rather
than silently accept a last-wins interpretation. Names/strings carry lengths,
including embedded decoded NUL; never truncate names at NUL during lookup.

Provide a default parser plus explicit bounded options and accessors for kind,
numbers, strings, arrays and object members. Numeric inspection distinguishes
exact int64 lexical integers from fractional/exponent forms and finite floating
values; `1.0`, `1e0`, true and null cannot become integer metadata. An integer
lexeme outside int64 may be represented as a finite float, never wrapped or
saturated into an integer. JSON number grammar is strict before conversion.
Numeric conversion must honor JSON's decimal point independently of the caller's
numeric locale without changing global locale. Reject nonfinite conversion;
document finite underflow/rounding behavior rather than invent exact bigints.

Caller byte buffers must actually contain the supplied readable length (ordinary
C pointer precondition). Successful documents own all storage and do not depend
on the original buffer. Invalid arguments, allocation failure, malformed text
or limit violations produce no partially owned output. A document release API
resets its handle and is safe on NULL/empty/already-released handles. Views are
valid only until their document is released; document that lifetime explicitly.
Do not promise safety for arbitrary forged C pointers.

All size arithmetic, node/member/string capacities and allocations are checked
before multiplication/addition. Bound input to 4 MiB and container depth to 128
by default, with explicit node/allocation budgets preventing tiny-value input
from expanding without limit. Options may select tighter limits; reject unsafe
or unrepresentable limits before parsing. Provide allocator injection or a
similarly deterministic private test seam to exercise every allocation-failure
path and verify complete partial-tree cleanup. No hidden file/network/process
I/O or global mutable parse state belongs in the library.

## Text and boundary semantics

Parse exactly one complete JSON value with only RFC JSON whitespace afterward.
Reject trailing documents, malformed literals/numbers, leading zeros, unfinished
fractions/exponents, invalid escapes, unescaped controls and truncated tokens.
Decode escapes, Unicode and surrogate pairs to valid UTF-8; reject invalid raw
UTF-8, unpaired surrogates, overlong encodings and out-of-range code points.
Byte, depth, node and memory envelopes must apply on every success/error path,
including long strings and wide arrays/objects. Error reports expose a closed
kind and bounded offset, not raw potentially sensitive input fragments.

Test objects with duplicate and embedded-NUL names explicitly. This is text
parsing, not NN40 checkpoint acceptance: the existing 55 fixture documents may
be parsed/inspected as JSON, but schema-level rejection still belongs to the
future public native dense loader. No parser test or package registration is
native checkpoint-training evidence.

## Portable validation and delivery

Use the existing ISO harness and dependency metadata with both BUILD fronts.
Linux CI requires GCC and Clang; Windows exercises MSVC; macOS Apple Clang.
Warnings are errors and non-ISO extensions fail. Tests cover strict grammar,
all value kinds, nested/wide limits, Unicode, exact integer boundaries, finite
numeric conversion, ownership, accessor bounds, allocator faults and cleanup.
Target at least 95% library line coverage; run AddressSanitizer/UBSan where
supported, including allocation-failure tests. Measure actual coverage, not just
test counts. Any standalone fixture/command helper targets at least 80%.

Provide explicit install/payload metadata appropriate to the C lane, required
capabilities, README/API examples, changelog, teaching lesson/trace, curriculum
and framework roadmap. Use the existing shared harness, not a bespoke competing
compiler scheduler. Do not change matrix/math semantics or five-native gate
selection merely to claim this parser is training. Later native C and actual
six-native exchange remain separate tranches.

Preserve dirty primary/unrelated work and initialize sparse paths before adding
files. Disk fell near 106 MiB after NN46; only the validated regenerable NN38
target was cleaned (92.8 MiB, sources retained), while Windows' unregistered
NN46 residual directory was preserved. Recheck disk before builds. Independent
read-only security review is mandatory before every push; publish one ready PR,
inspect real package BUILD/applicable CI/CodeQL and confirm exact-head merge
before selecting the next tranche.

## Confirmed prerequisite repairs and local evidence

The existing Windows ISO harness exposed three blockers during actual native
validation: scalar compiler discovery under StrictMode, a BOM-free UTF-8
diagnostic misdecoded by PowerShell 5, and a constant runtime self-test assertion
rejected by MSVC /W4 /WX. Capture discovery with @(...), keep the diagnostic ASCII,
and use _Static_assert for the compile-time invariant; six production-guard
cardinality regressions cover zero/one/two compiler results. The scheduler and
strict flags remain unchanged. The new wrapper explicitly decodes the trusted
shared source as UTF-8. These narrowly confirmed dependency repairs are included
because native package BUILD cannot succeed without them; no unrelated harness
refactor is part of NN47.

The parser passes 1605 checks under GCC, Clang and actual MSVC, plus Clang
ASan/UBSan. GCC gcov reports 100% library lines, 100% branches executed and
96.15% branches taken at least once; GCC's static analyzer reports no findings.
Coverage is measured from successful complete fixture-owning package
runs, not a failed output-directory run. The shared C/C++ conforming and negative
fixtures pass under GCC/Clang/MSVC, as do all six compiler-cardinality guards.
An actual CMake static-library build/install and separate find_package consumer
compile/link/run prove the exported header/archive/config payload.

Default resource budgets are 65536 value nodes and 16 MiB arena allocations;
all options may only tighten hard ceilings. Numeric conversion translates the
JSON decimal point to the host locale's spelling before strtod without changing
locale; caller-controlled concurrent global locale changes need external
serialization. Every allocator failure in valid and malformed partial trees
returns no document and zero live custom allocations. Native C training and
six-native exchange remain entirely pending. No checkpoint-schema acceptance
claim is derived from parsing the 55-case fixture inventory.
