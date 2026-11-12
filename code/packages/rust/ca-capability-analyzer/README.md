# ca-capability-analyzer-rs

A zero-third-party-dependency static capability analyzer and deny-by-default name allowlist for Rust packages in the monorepo ([VER08a](../../../specs/VER08a-rust-capability-analyzer.md), Spec 13 Layers 3 & 4).

## What It Does

Every Rust package in this repository declares its operating-system level capabilities in a `required_capabilities.json` manifest. Pure computation packages (such as `rust/ct-compare`, math libraries, codecs, and parsers) declare an empty capability list (`capabilities: []`), establishing a deny-by-default sandbox.

`ca-capability-analyzer-rs` verifies that:
1. Every ambient OS access in `.rs` files matches a declared capability (`CAP001` check);
2. All banned constructs (`unsafe` blocks/functions, inline assembly, foreign linkage) have matching manifest exceptions (`CAP002` check), and excepted constructs stay **visible** in the output (see below);
2b. Constructs the analyzer can see but cannot analyse (`include!` however it is reached, `#[path]`, proc-macros, macro-spliced paths and attributes, `use std::*`, non-ASCII identifiers, edition-dependent C-string prefixes when `package.edition` is unknown, root-level `mod` declarations into unscanned directories, unknown `std` paths, `Cargo.toml` paths into unscanned directories or target roots that are not `.rs` files, `macro_rules!` metavariables in a keyword position (`$k std::io as out;`) or as a method name (`$e.$m()`), `use $x`, `type $name = ..`, unparseable `type` items, alias chains past the resolver's caps) fail closed as `CAP003`, which no exception can waive. The lexer follows rustc's shebang, whitespace (`Pattern_White_Space`) and edition rules;
3. Manifests conform to `required_capabilities.schema.json` version 1;
4. Traversal adheres to strict resource quotas and refuses symlinked files;
5. Sealed capability modules (`src/capabilities.rs`) enforce "you cannot name what you did not declare" (VER23);
6. Canonical effect proof certificates (`effect_certificate.json`, schema `ver24-effect-cert-v1`) verify proof-carrying capability bounds via an independent small checker (VER24).

```text
  Package Rust Source Files
             │
             ▼
   ca-capability-analyzer-rs
             │
             ├── Traverses package source recursively (max 10,000 files, refuses symlinks)
             ├── Detects raw stdlib calls → std::fs, std::net, std::process, std::env, ...
             ├── Detects banned constructs → unsafe blocks, asm!, #[no_mangle], ...
             │
             └── Compares against required_capabilities.json
                    │
                    ├── Detected ⊆ Declared (and all banned constructs exempt) → EXIT 0 (pass)
                    └── Detected ⊄ Declared (or unexempt banned construct)   → EXIT 1 (violations)
```

## CLI Usage

```bash
# Analyze a specific package directory
cargo run -p coding_adventures_ca_capability_analyzer -- --dir code/packages/rust/ct-compare

# Verbose mode: display all detected capabilities and inspected files
cargo run -p coding_adventures_ca_capability_analyzer -- --dir code/packages/rust/ct-compare --verbose

# Generate sealed capabilities module (VER23)
cargo run -p coding_adventures_ca_capability_analyzer -- --dir code/packages/rust/ct-compare --generate-module

# Verify capabilities module is up-to-date with manifest
cargo run -p coding_adventures_ca_capability_analyzer -- --dir code/packages/rust/ct-compare --check-module

# Emit canonical effect proof certificate (VER24)
cargo run -p coding_adventures_ca_capability_analyzer -- --dir code/packages/rust/ct-compare --emit-cert code/packages/rust/ct-compare/effect_certificate.json

# Verify effect certificate using the small independent checker
cargo run -p coding_adventures_ca_capability_analyzer -- --dir code/packages/rust/ct-compare --verify-cert code/packages/rust/ct-compare/effect_certificate.json
```

## Exit Codes

| Code | Meaning |
|---|---|
| `0` | All detected capabilities are declared; no unexempt banned constructs |
| `1` | One or more violations found (`CAP001`, `CAP002` or `CAP003`) |
| `2` | Input error, malformed manifest JSON, resource ceiling exceeded, or parse error |

## Reading the output: "pass" does not mean "no unsafe"

```text
PASS: package 'rust/ct-compare' capability check succeeded (2 files scanned, 13598 bytes)
  NOTE: 2 banned construct occurrence(s) are PRESENT under manifest exception (not absent):
    - unsafe_code at src/lib.rs:105:8 ('unsafe')
    - unsafe_code at src/lib.rs:118:8 ('unsafe')
SUMMARY unsafe_code=present-excepted(2) inline_assembly=absent foreign_linkage=absent ffi=absent unanalyzed_dependencies=0
```

Per construct: `absent`, `present-excepted(N)` or `present-violation(N)`. Anything that claims
"no unsafe" must require `unsafe_code=absent`. Library API:
`AnalysisReport::construct_status("unsafe_code")`.

## What it can and cannot see

It is a *lexical* analyzer (a lexer, a `use`-tree/alias resolver that follows renames through every
path segment and resolves `Self` to the enclosing `impl`'s self type, and a classifier; no parser,
no macro expansion, no type information). A type it cannot name (a metavariable, a macro in type
position such as `<ty!()>::exists`, an associated-type projection such as `D::Target::exists`) is
an unknown type, and its item gets the verdict of a method call `.exists(` on an unknown receiver. Exact contracts, including the full list of detected
surfaces and the evasion routes handled, are in [VER08a](../../../specs/VER08a-rust-capability-analyzer.md)
and [VER23a](../../../specs/VER23a-rust-effect-modules.md). Out of scope, stated honestly (VER08a section 8):

- third-party dependencies are not analysed (they are listed as `WARN`/`unanalyzed_dependencies`);
- capability `target` is not enforced: `fs:read:/x` covers every detected `fs:read`;
- `tests/`, `benches/`, `examples/` and dev-dependencies are not analysed;
- macros are not expanded (authority produced only by another crate's macros is invisible);
- implicit authority that is never named in source (HashMap entropy seeding, allocation, panic output).

### Known limitations

Two false negatives found by the third security review are **open**: a type alias whose generic
parameter default names the type (`type F<P: ?Sized = std::path::Path> = P; <F>::exists(p)`,
CA-OG1), and bypasses of the local-receiver waiver for `Local::new().exists()` when `Local`
derefs to `Path` (a `cfg`-ed out or macro-body inherent impl, generic instantiations, same-named
types, `self: Rc<Self>`; CA-OG2). Each has an example, root cause and intended fix in
[VER08a section 8.1, "Known open gaps"](../../../specs/VER08a-rust-capability-analyzer.md#81-known-open-gaps-security-review-round-3),
and a row in [VER-BACKLOG](../../../specs/VER-BACKLOG.md). A `PASS` does not exclude them.

Bounded by design: every resolver stage has a work ceiling (VER08a section 5) and exceeding it is a
`CAP003`, never a hang or an allocation abort. `Cargo.toml` is read by a small tokenizing TOML reader
that fails closed on anything it cannot parse (VER08a section 3.5.1).

## Certificates

`--emit-cert` refuses to write a certificate unless the whole-package analysis is clean.
`--verify-cert` recomputes the expected certificate from disk and requires every field to match.
`source_digest` covers `src/**/*.rs` except `src/capabilities.rs` (CRLF-normalised, length-framed);
it does not cover `Cargo.toml`, `build.rs` or other files, which is why verification also re-runs
the full analysis. The label `proved` is a package-level lexical claim with the limits above.
`entry_points[].effects` is the sound over-approximation (all declared effects), not a per-function result.
A package whose `Cargo.toml` sets `[lib] path` to anything but `src/lib.rs` is refused: the entry
points and `source_digest` describe `src/lib.rs` only.

## Development

```bash
cargo test -p coding_adventures_ca_capability_analyzer
cargo clippy -p coding_adventures_ca_capability_analyzer --all-targets -- -D warnings
python3 tests/data/gen_sha256_vectors.py   # regenerate hashlib vectors (needs python3)
```

Test suites: `tests/test_evasion_corpus.rs` (350 exact-verdict cases), `test_secreview_round3.rs`,
`test_secreview_round4.rs` and `test_secreview_round5.rs` (review proofs of concept with decoys), `test_sha256_vectors.rs`,
`test_certificate_tamper.rs`, `test_manifest_hardening.rs`, `test_package_analysis.rs`,
`test_analyzer.rs` (original suite).
