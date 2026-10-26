# NN50: Native C++ Bounded JSON Prerequisite

Status: specified before implementation. No native C++ checkpoint or training
claim is made by this parser tranche.

## Evidence and selection

NN49 merged in [PR #18301](https://github.com/adhithyan15/coding-adventures/pull/18301)
at exact reviewed head `8e93a791713148c2e9e0fb8fc6fc1b90cb9346c5`, squash
`d555c1091cd9aa134e6eb4b702938772f7487231`. All selected jobs and the final
required CI/Books gates passed. Actual conformance executed 78 children, all
thirty directed exchanges, six native languages and zero bindings through step
3002. The observed XOR loss was 0.00023307123378282047. Actual C package BUILDs
passed on Linux, macOS and Windows; no configured C CodeQL analyzer is claimed.

Reassess reliability, prerequisites, portability and smallest complete delivery.
The native C++ matrix and float-math packages already exist, but current main
has no C++ JSON parser. Their build fronts use the shared **C** ISO harness;
there is no separate C++ harness to create. A bounded parser is the smallest
independent prerequisite before accepting untrusted C++ training checkpoints.
Deliver it separately from native dense training and seven-native conformance.
Do not wrap the C parser or Rust core and call that native C++ implementation.

## Public ISO C++17 contract

Create `code/packages/cpp/bounded-json`, with namespace `ca::bounded_json`, an
owned RAII document and immutable value views. The document owns decoded data;
the caller may immediately destroy or overwrite the input after parsing. Views
expire with the document. State move/copy and view invalidation rules explicitly.
Use ordinary standard-library facilities, not an external JSON dependency.

Expose null, boolean, exact signed int64 integer, finite double, UTF-8 string,
ordered array and ordered object kinds. Retain every duplicate object key in
source order. String/key views carry lengths, including embedded decoded NUL;
never discard duplicates or silently truncate keys at NUL. Wrong-kind and
out-of-range access must fail explicitly, never become unchecked indexing.

Provide default parsing and caller-tightened limits with structured error kind
and bounded byte offset. Errors do not echo source fragments. Parsing consumes
exactly one complete JSON value followed only by JSON whitespace. No hidden
filesystem, network, subprocess or global mutable parser state is permitted.

Integer metadata must remain lexical: `1` is an integer, while `1.0`, `1e0`,
true and null are not. Int64 overflow cannot wrap or saturate; a finite floating
representation is allowed for a larger integer lexeme. Numeric conversion must
be locale-independent without changing process locale. Reject nonfinite or
out-of-range floating conversion; document and test the chosen finite underflow
boundary, including signed zero. Preserve JSON number grammar before conversion.

## Resource and text boundaries

Default hard ceilings are 4 MiB input, depth 128, 65536 value nodes and 16 MiB
parser-owned dynamic allocation. Caller options may only tighten these ceilings;
invalid or unrepresentable options fail before parsing. Count container storage,
member names and decoded strings, not just final node payloads. Check arithmetic
before allocation or growth. Do not label a logical payload estimate an actual
allocation ceiling. Use a bounded allocation owner/resource and a deterministic
allocation-failure seam; error paths must release all partially built state.

Strictly decode escapes and Unicode surrogate pairs to valid UTF-8. Reject raw
invalid UTF-8, overlong encodings, lone surrogates, out-of-range code points,
unescaped controls, invalid escapes, truncated tokens, trailing values, leading
zeros and unfinished fractions/exponents. Empty/wide/deep structures and long
escaped strings must honor the same budgets on success and failure paths.

This is JSON syntax/ownership, not NN40 schema acceptance. The shared 55-case
checkpoint corpus may be inspected as JSON; only a later dense loader can claim
schema rejection or resumable training. Keep null a distinct JSON kind.

## Validation and delivery plan

1. Commit this specification and NN49 completion record before implementation.
2. Implement native C++ parser/accessors with boundary and fault-injection tests.
3. Use existing shared ISO harness and both BUILD fronts; require GCC and Clang
   on Linux CI, actual MSVC/clang-cl on Windows and Apple Clang on macOS according
   to installed harness ownership. No competing compiler scheduler.
4. Measure at least 95% library line coverage; exercise partial-tree allocation
   failures and live-allocation cleanup. Run ASan/UBSan where supported. Exercise
   multiple translation units and an independent public-header consumer so
   header-only delivery does not hide ODR or include dependencies.
5. Supply package README, changelog, dependency/capability metadata, teaching
   trace and curriculum/roadmap updates. Validate actual package build selection;
   if Windows is omitted, repair only the confirmed scoped selection ownership
   with executable planner tests, not an unrelated planner rewrite.
6. Obtain independent read-only security review of the exact final head before
   pushing one ready PR. Inspect actual package logs, selected workflows and
   required gates; merge only their unchanged successful, conflict-free head.

Preserve unrelated work and the archived NN49 evidence. Disk space is variable:
recheck before builds. The old NN49 residual directory remains preserved after
an execution-policy rejection; do not bypass that rejection to remove it.
