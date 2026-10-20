# Changelog

## Unreleased

- LANG84 phase 2b: the `cst_contract` note now says that `iir_status`
  exists in `releases.csv` but is `-` until phase 4c. No code change; the
  tests pass against the reordered inventory.

- LANG84 phase 4b: `DEFAULT_RELEASE` is the inventory release `5.38.0`
  (new `DEFAULT_RELEASE_ID`), whose pair is grounded in the `v5.38.0` tag.
  `Release` is now a newtype over one checked inventory ID (`id()` returns
  `&'static str`); `Release::Unversioned` and `Release::Inventory` are gone,
  because the unversioned LANG81 pair no longer reaches the IIR path (C4.5).
  Visible change: under the default, a 251-digit literal is now rejected
  before lowering, as `toke.c` does, instead of lexing and being refused as
  out of range.
- Every produced module records its release in `IIRModule::release`
  (`compile_ast_at` sets it; `compile_ast` records the default).
- C8 CST contract: `cst::*`, `CST_RULES`, `CST_TOKEN_TYPES` and
  `CST_TOKEN_SPELLINGS` name everything the compiler dispatches on, and the
  compiler uses those constants. `tests::cst_contract` checks every checked
  release's compiled grammar defines each rule and token type, lexes each
  spelling as one token, and parses the corpus into exactly the shapes the
  compiler destructures.
- C5 hostile-input probes run in a child copy of the test binary
  (`tests::hostile_probes`): deep nesting, deep unary minus, long flat
  chains, decrement runs, unterminated constructs, statement floods,
  foreign characters, the source and IIR budgets, the 250/251-digit
  boundary and deep/wide caller-supplied trees, across three releases. The
  parent reports a signal (stack overflow, abort) as an abort, kills a child
  that runs past 600 s, drains both pipes on threads, and clears
  `RUST_MIN_STACK` so the probes run on the default test stack.
- Security-review follow-ups: `lex_errors_never_echo_control_or_bidi_characters`
  pins that untrusted source reaches diagnostics only escaped, and
  `resolve_release` is tested against near-miss spellings (`v5.38.0`,
  `5.038.0`, padded, NUL-suffixed, `perl5.38.0`), all `UnknownRelease`.

- `main` returns `i64` 0 (`const 0; ret`) instead of `ret_void`. Every backend
  reads the entry function's result as the process status. CoreCLR rejected a
  void `int32 MccarthyEntry()` (CI's macOS and Windows CLR column), and on BEAM
  a void `return` hands back whatever x0 held, the atom `ok` from the last print,
  which the matrix launcher cannot read as an integer.
- LANG84 phase 4a: release selection. `compile_source_at(source, release,
  module_name, &FrontendBudgets)` compiles against exactly one checked
  inventory release, or `DEFAULT_RELEASE` (still the unversioned LANG81 pair)
  for `None`. `resolve_release` (in `release.rs`, the only code that looks at
  release IDs) returns typed `UnknownRelease` (up to five longest-prefix
  suggestions; caller text truncated to 64 characters and control
  characters escaped) or
  `ReleaseNotAvailable` errors and never falls back. `FrontendError` adds
  `Budget`, `Lex` and `Parse` with line and column, and `Unsupported` with the
  CST rule and span; every variant after resolution names the release.
  `FrontendBudgets` bounds source bytes, parser depth and IIR instructions.
  `compile_source`, `compile_ast` and `run_source` delegate to the new entry
  points; `compile_ast_at` and `run_source_at` are new.
- `print` lowers to the LANG83 `write_i64` builtin instead of the Perl-only
  `pl_print_int`, so every lang-aot backend runs the output.
- `plvm` accepts `--release <inventory-id>`.
- Tests: every checked release lowers the corpus to IIR identical to the
  default's and runs it on vm-core; release-specific lexing (CR, 250-digit
  limit); C7 op validation; typed errors and budgets.

- Reject legacy leading-zero octal tokens instead of lowering them as decimal.
- Bound every directly supplied AST text field and the module name before
  traversing or lowering the tree.

## 0.1.0

- Initial native Perl source-to-IIR-to-vm-core arithmetic and print pilot.
