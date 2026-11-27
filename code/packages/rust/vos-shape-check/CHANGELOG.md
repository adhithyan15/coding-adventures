# Changelog

All notable changes to this package are documented here.

## [Unreleased]

### Added (VOS-040c)

- **`vos.toml` key `sat`** (VOS31 10.3): a closed enum (`SatLane`, one member, `Model`; `VosToml::sat: Option<SatLane>`). Only `"model"` is valid, an absent key is `None` (off), a duplicate, another spelling or a non-string is a `vos.toml` error, and the unknown-key message names the key. Test: `sat_key_is_a_closed_enum_with_one_member`.

### Added (VOS-039d, VOS-D089)

- **The tick audit** (`tests/tick_audit.rs`, VOS03 section 7.1).  Every `while`, `loop` and `for` in `src/**/*.rs`
  outside `#[cfg(test)]` calls `steps::tick()` in its own body or has a `// bounded: <reason>` line directly above
  it; the test fails with every offending `file:line`.  Loops are found over RS01 tokens (`lex_documents`,
  `FileCtx`), with no second scanner: `for` is a loop only with an `in` before its body, `for<'a>` and
  `impl X for Y` are not, and an inner loop's tick does not excuse the outer loop.  Negative controls cover each
  case.
- **Ticks in 180 loops** that had none (the lexer adapter and comment scan, the shebang scan, `FileCtx`
  construction, the outer loop of `mask_cfg_test`, and the outer loops of the rule passes, the loader, the manifest
  reader and the TOML reader); two loops carry `// bounded:` (the lexer retry, at most four attempts; the fixed pair
  of editions).  Findings are unchanged; only the step count grows.
- **Linearity tests of the formerly invisible paths**: plain tokens, many block comments, comments after `#!`,
  kept `#[..]` attributes and alternating `#[cfg(test)]` items, each n against 4n against 16n.


### Added (VOS-D112)

- **SH19 (h), a closed attribute list for `kind = "contract"` crates** (`command::attributes`).  Every attribute is
  `derive` of the bare core traits `Copy Clone Debug PartialEq Eq PartialOrd Ord Hash Default`, `repr` of `u8`, `C`
  or `transparent`, `no_std`, `forbid`, `deny`, `warn`, `allow`, `doc`, `must_use`, `inline`, `default` (the
  helper of `derive(Default)`) or exactly `cfg(test)`; `non_exhaustive` passes this pass and is reported by the
  closed-set rule.  Any path attribute (`#[evil::rewrite]`), a derive of a path (`derive(evil::Gen)`), `cfg_attr`,
  `path`, `macro_use`, `proc_macro*`, `test` or an unknown name is an SH19 finding.  A whitelist over the spelling,
  so a foreign macro cannot rewrite a command enum after SH19 judged it.  The dependency half is
  `vos-link check-component` CC10.

### Fixed (security review of VOS-039b-2a)

- **SH19 gate bypass closed by a closed vocabulary** (`command::vocabulary`).  A contract crate may not contain a
  `use` or `extern crate`, a `mod`, a macro definition or invocation, or an item (`struct`, `enum`, `union`,
  `type`, `trait`, `fn`, `const`, `static`) named like a Rust primitive or a Warrant built-in type
  (`RESERVED_TYPE_NAMES`, checked by a test against `warrant-grammar`'s list).  This refuses, as whole
  constructs, `type u8 = u32;`, `struct u8`, a `use .. as` rename hiding a foreign type, a macro-written enum, and
  the `struct U8(u8)` collision with CK10.  `Names::judge` also consults the item table before accepting `u8` or
  `bool` (`Local::Shadowed`).  Attributes and doc comments are unaffected; the real `vos-sched-contract` and the
  `pass-ready-contract` fixture still pass.
- `items::collect_types`: an `enum` or `struct` keyword not followed by a name (and `union`/`type` before `$`) is a
  gap (VT03, fail closed) for every crate, instead of a silent skip (`enum $n` in a macro body).
- **Quadratic work removed** (`Names`, `command_rules`): the reverse-reference graph is over DISTINCT NAMES (one
  node per name, deduplicated edges, a name fails if any item of that name fails), so n items named `X` and n
  fields naming `X` cost linear work, not n times n.  Test: `sh19_same_named_items_do_not_make_the_judgement_quadratic`.
- **Output injection**: type, variant, field and item names in SH19 messages are escaped (any non-printable-ASCII
  character) and cut at 48 characters, so a string literal with a newline in a field type cannot forge a line.
- Tests: a negative control for every reproduced bypass in class K, P and H, the `$` gap, `pub use`, a macro
  invocation, the reserved-name list against the Warrant checker, and the control-character message.

### Added (VOS-039b-2a, VOS-D077)

- **SH19, the command-type check** (`src/command.rs`, VOS03 section 4), bound by **kind** (`kind = "contract"`),
  not by class.  The command set is derived: every `enum` of the crate, `pub` or not.  An enum fails on
  `#[non_exhaustive]` (enum or variant), a generic list, or a variant field whose type is not exactly `u8`,
  `bool`, `[u8; <decimal literal>]` or the bare name of a same-crate struct or enum that itself passes.
  `usize`, wider integers, references, pointers, tuples, `Option<..>`, paths, a `type` alias (rejected, never
  resolved) and a const-named length are findings.  Nested types are judged by a work list over a reverse
  reference graph (no recursion, linear, a cycle terminates).  `Rule::Sh19`, `command::ACCEPTED_TYPES`
  (a test reads VOS03 and asserts it states them).
- `items`: `TypeDef` gains `variants`, `generic`, `non_exhaustive`, `malformed`; new `Variant`; the enum body is read
  by the existing token-tree scan (tuple, named-field, unit and discriminant variants).  An unreadable variant
  is a `Gap` (VT03, fail closed).  `parse_fields` now reports a skipped entry instead of dropping it silently.
- `loader` is a library module (it was private to the binary) with `load_sources`, so `vos-link` reuses the
  hardened source walk for `check-contract`'s CK10 instead of writing a weaker one.
- `publish = false` (an evidence-only tool, VOS-D104).
- Tests: the `Poke(usize)` and `Write { addr: u32 }` controls (inline and as fixtures `fail-contract-poke`,
  `fail-contract-addr`), every rejected spelling, alias, nested and cyclic cases, `Pick::Run(u8)` and `Upd`
  passing, the same enum in a production crate silent, the real `vos-sched-contract` passing, linearity over
  variants and structs, and a 20,000-deep chain.

### Added (VOS-039a)

- `vos.toml` key `excused-controls`: an array of inline tables `{ id = "<manifest id>", reason = "VOS-Dnnn" }` naming the negative controls a crate may fail (VOS-D065, VOS-D098).  Never empty, each id 1 to 128 plain bytes and distinct, at most 16; the reason is a backlog row, which `vos-link` requires to exist and be open, so an excuse expires with its row.  New `Excuse` and `is_backlog_ref`.
- `vos.toml` key `coverage-floor`: an integer percent 0 to 100, read into `VosToml::coverage_floor`.  The reader only
  bounds it (plain decimal digits; a sign, an underscore, a prefix, a fraction or a string is refused); the
  floor is enforced by `vos-link check-component` and the BUILD coverage line (VOS04 section 5.6, VOS-D070).

### Fixed (VOS-D075, CI flake `item11_item_scanning_is_linear`)

- The linearity tests (`item11`, `item12`, `l3`) no longer compare wall-clock growth
  ratios.  A shared runner pays about 4 s of fixed lexer construction per run with
  about 1 s of jitter, more than the increments the ratio relies on, so a run at a
  smaller size could be slower than one at a larger size (CI measured a ratio of 1276
  against a limit of 10).  They now assert on a deterministic work count; a 120 s
  hang guard remains and nothing else about time is checked.

### Added (VOS-D075)

- `steps` module and `check_crate_counted`: a thread-local work counter ticked by
  every token predicate (`Token::is_ident`, `Token::is_punct`) and by each iteration of
  the scanning loops of `syntax`, `items` and `graph` (including the mask-fill loops
  and the call-graph traversal).  `check_crate_counted` returns the report and the
  steps taken.  Tests assert that steps grow by about k (not k*k) when the input grows
  k times.  Proven sensitive by temporarily adding a quadratic loop to the graph
  builder: the count test failed (6.0 against a limit of 3.75), then it was reverted.
- Tests: the `mod ghost` case is now balanced (an unbalanced file fails at bracket
  pairing and exercised no scanner); `struct S<` runs at k = 3 and a larger n, because
  its per-item scan is a window capped at 4096 tokens and only a size well past the
  cap shows the linear slope.

### Changed (VOS-012b0, VOS-D061)

- `obligations` may list up to five tiers (the new `representation` tier, VOS02 2.7).  No `mode`
  key: a production crate's mode is derived from the tier its document carries.

### Added (VOS-033)

- `vos.toml` accepts two more keys for an implementation crate, `contract = "<package>"`
  (the contract package it realises) and `obligations = [..]` (the tiers `vos-link gen-doc`
  embeds), bounded here (a name of at most 32 bytes, at most 4 tiers, duplicates refused)
  and judged by `vos-link` (VOS02 section 10.1, D-GD2).  `VosToml` gains `contract` and
  `obligations`.

### Fixed (CI)

- The performance tests (`item11`, `item12`, `l3`) no longer assert absolute
  wall-clock limits, which failed on slow shared runners.  They time the
  workload at n, 4n and 16n and compare the two increments (the fixed lexer
  build cost cancels): about 4 for linear code, about 16 for quadratic, bound
  10.  A 120 s hang guard remains.

### Fixed (round 3 of the security review)

- SH9: a `<` counts as a generic open only after `::`, `impl`, or a name that
  follows `dyn`, `for`, `as`, a declaration keyword, `->` or a type-introducing
  `:` (nested opens only inside such a list); `{` and `::` no longer follow a
  `>>` generic except after a return type / `impl` header and a turbofish.
  `g(a < b, c < d >> { n })` and `n << y as u32 > ::core::u32::MAX` are reported.
  Intended false positives are listed in VOS03 section 6.
- The block-comment adapter counts depth for doc comments too (it no longer
  relies on the grammar refusing a nested comment in a doc comment).
- `check_crates` re-lexes a crate alone when a batch failure (a forged boundary,
  exhausted attempts) marked it, so one hostile crate cannot fail the others.

### Changed (round 2 of the security review)

- **Rust and TOML are read with the repository's grammar-driven crates**, not a
  hand-written lexer or parser: `GrammarLexer` on the RS01 lexical pair for the
  edition (2021 or 2024) with the two RS01 C3 hooks as adapters, and
  `coding-adventures-toml-parser` for `Cargo.toml` and `vos.toml` (duplicate keys
  and escapes fail closed). One lexer per run: all files are joined with a
  checked boundary and lexed once.
- The structural layer **fails closed**: any header, generic list, where clause
  or type that cannot be read (over 4,096 tokens, runs into the next item, ends
  with the file) is VT03. Const generics, `use<>` and `union` as a type name no
  longer truncate an item; long impl headers are read.
- SH9 `>>`/`<<` generics need type-position opens with type-ish tokens between
  them and a following token that can follow a type; match-arm rebinding of a
  loop variable is a shadow; only array-position keywords may precede `[`
  (`.await[i]` is an index); call sites are deduplicated before resolving; ghost
  exports are exact names; the `O_NOFOLLOW` constants are an explicit per-target
  table (unknown Linux architectures are refused at runtime); total source cap is
  1 MiB.

### Fixed

- Security review round 1 (33 items, one regression test each in
  `tests/security.rs`): shebang lookalike `#! [attr]` no longer hides code;
  raw identifiers (`r#unwrap`) are normalised so every name rule fires; SH17
  wrappers are mandatory except for a closed list of exports; `#![no_std]` and
  `#![forbid(unsafe_code)]` must be crate-level and exact; `[lib] path`, binaries,
  other targets and `[project]` are pinned; every index form is an index unless
  an array-literal position; shift, panic-path, iterator and closure evasions
  closed; `for` loops are judged by `impl` header context; turbofish and
  unnamed-type impls in SH5; every `cfg` other than the ghost gate is SH18;
  dev-dependencies are an allowlist; SH14 applies to class K with no kind and to
  tuple structs and unions; case-insensitive `build.rs`, `.cargo`,
  `rust-toolchain*`; `1.` and `1_f64` are floats.
- Quadratic scans (type extents, header scans, attributes, `#[cfg(test)]` mask,
  interior-mutability fixpoint, call-graph BFS) are linear or bounded, and fail
  closed with VT03 where a bound is hit; call graph uses iterative Tarjan.
- Loader: `O_NOFOLLOW | O_NONBLOCK` open with an open-handle `is_file` and
  `dev`/`ino` check, total byte, per-directory and total entry caps enforced
  during the walk, case-fold traps refused.

## [0.1.0] - 2026-11-16

### Added

- Lexical component shape checker (VOS-D002, VOS03): SH1-SH15, SH17 and SH18,
  class tags and kinds from `vos.toml` (VOS01 section 1.6), the static bans
  (`build.rs`, `build` key of any value, `links`, proc-macro, relocated targets),
  the central evidence-feature list, and the `-any` name lint.
- Own zero-dependency lexer, bracket pairing, a fail-closed TOML-subset reader,
  a by-name call graph for SH5 and the ghost-face wrapper checks for SH17.
- Deterministic report and exit codes 0, 1, 2; bounded, symlink-refusing loader in
  the binary only; the library is pure.
- Tests: per-rule clean and violating cases, the VOS01 toy crates and broken
  variants, CLI and filesystem tests. Line coverage 99%.
