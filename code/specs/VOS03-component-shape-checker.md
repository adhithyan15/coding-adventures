# VOS03 -- The Component Shape Checker (`vos-shape-check`)

**Status:** Draft, specified before the code (backlog VOS-D002, lexical half).
The image-level companions (no path to the panic handler, static stack bound)
are **not** in this document's scope; they arrive with VOS-026 and VOS-028.

**Layer:** Tooling, below VOS01 and VOS02. VOS01 section 1.4 defines the shape
rules SH1-SH15 and names "one new, VOS-owned lexical check"; VOS02 section 4.6
adds SH17 and SH18, section 7.2-7.3 the static file and manifest bans and the
central evidence-feature list, section 6.8 the parser caps, and section 10
places the tool at `code/packages/rust/vos-shape-check`. This document is the
exact contract of that tool: inputs, which rule binds which crate, the precise
lexical check of each rule, what it cannot see, the report and the exit codes.

**Parent:** [VOS01](VOS01-component-contract-model.md) sections 1.4 and 1.6,
[VOS02](VOS02-interface-contracts-and-profile-linking.md) sections 4.6, 6.8,
7.2, 7.3 and 10.

---

## 0. What a pass means

The check is **lexical**, in the sense of VER08a section 8: a pass means *no
occurrence the lexer can see*. It reads tokens and the text of `Cargo.toml`;
it never expands a macro, resolves a name, or runs anything. A lexical check
can be too strict (a false positive: a harmless token that looks like a
violation) and it can be too weak (a violation it cannot see). Every rule below
states which of the two it risks, and section 6 collects what it cannot see.
Those residues are exactly the **image-level companions** (VOS-026, VOS-028)
and the **spine audits** (VOS-D024): the shape check narrows the review, it
does not replace the proof on the image.

Bias: where a token is ambiguous the check **fails** (reports). A contributor
rewrites the line; nobody reviews a hidden violation.

**Own lexer.** VOS02 section 10 says the tool reuses the VER08a lexer as a
library. VER08a's `lex`, `Token` and the bracket tables are private to
`parser.rs`, and its public surface (`scan_source`, `analyze_cargo_text`)
answers VER08a's questions (capability use, `CAP002`, `CAP003`), not "which
tokens, on which line". Exposing a token API means editing
`ca-capability-analyzer`, which selects every link check in CI (VER29 section
11.2) and which this work must not touch. So `vos-shape-check` carries its own
small lexer (zero dependencies) and logs the gap (backlog VOS-D029). The two
checks therefore stay independent, which is a feature: VER08a `CAP002` remains a
second line for `unsafe`, asm and foreign linkage.

---

## 1. Inputs

A run checks **one crate directory**. It reads, nothing else:

| Input | Where | Cap |
|---|---|---|
| `Cargo.toml` | crate root; required | 64 KiB |
| `vos.toml` | crate root; optional (absent means class K, kind unspecified) | 4 KiB, at most 16 keys (VOS02 section 6.8) |
| every `*.rs` file under `src/` | recursive, depth at most 16 | 1 MiB each, 512 files and 8 MiB in total |
| the names of the entries of the crate root | to find `build.rs` | 4,096 entries |
| the central evidence-feature list | an argument; default `ghost`, `cov` (VOS02 section 7.3) | 64 names |

Files outside `src/` (`tests/`, `benches/`, `examples/`) are not read: they are
not part of the shipped library. `tests/generated/` evidence harnesses may use
`std`, `unwrap`, floats and the rest.

**`#[cfg(test)]` items are masked.** An item whose attribute list contains
exactly `cfg(test)` is removed from the token stream before any rule runs. It
is not compiled into an image or into the generated evidence (the evidence
harness links the library without `cfg(test)`). The mask is a trust decision
and is stated here so it is not discovered: a `cfg(test)` item is *invisible* to
every rule below.

### 1.1 `vos.toml` (VOS02 section 7.3, bounded)

```text
# comment lines start with '#'; blank lines are ignored
class = "K"                      # K, C, S, M, P or H; default K
kind  = "production"             # contract, production, reference, any, spine, hal or profile
evidence-features = ["ghost"]    # names that must be in the central list
```

Grammar: a line is blank, a comment, or `key = value`; a key is one of the
three above; a value is a double-quoted string without `\` or a one-line array
of such strings; an end-of-line comment is allowed after a value. Anything else,
an unknown or duplicate key, a value outside its closed set, more than 16 keys,
a file over 4 KiB, a line over 1,024 bytes or non-ASCII, is rule **VT01**. The
**class tag lives here, not in the README** (VOS01 section 1.6 and VOS02
section 10 say README; backlog VOS-D030): a tag relaxes rules, so it must sit
on the CODEOWNERS authority list of VOS02 section 7.4 (which already contains
each crate's `vos.toml`), and a prose file is not parsed.

`kind = "any"` requires the package name to end in `-any`, and a package named
`...-any` must declare `kind = "any"` (**VT02**). The other direction of the
name lint (a production crate that carries an `any` block) is evidence, not
lexical, and belongs to `vos-link` (VOS02 section 3.1).

---

## 2. Which rules bind which class

VOS01 section 1.6, as data. SH16 is a profile-level gate (VOS02 section 7) and is
not checked here. P and H crates have no shape rule (only VT01 to VT03).

| Class | Rules checked |
|---|---|
| **K** (default) | SH1-SH15, SH17, SH18 |
| **C**, **S** | SH2-SH11, SH13-SH15, SH17, SH18 |
| **M** | SH2-SH9, SH11, SH13-SH15, SH17, SH18 (exempt SH1, SH10, SH12) |
| **P**, **H** | none |

SH17 and SH18 follow SH13 (the ghost face), which binds K, C, S and M. The
table is one function in the code; a change of VOS01 section 1.6 is a one-line
edit. Two disagreements between documents are recorded, not resolved here:
EMU00 section 7.5 calls class M "exempt from the no-panic shape rules" where
VOS01 section 1.6 exempts only SH1, SH10 and SH12 (so SH8 binds M here), and
the literal M row binds SH6, SH7 and SH11 although the wrapped simulators loop,
divide and use `f64` (backlog VOS-D031).

---

## 3. Report and exit codes

```text
vos-shape-check: crate vos-ready-bitmap, class K (vos.toml), kind production
SH7 src/lib.rs:12 division operator `/`; RV32I has no divide (use shifts and masks)
SH8 src/lib.rs:20 `unwrap` is a path to a panic
result: FAIL, 2 findings
```

One finding per line, `RULE FILE:LINE MESSAGE`, sorted by file, line, rule id
(numeric order, then `VT`) and message, duplicates removed, so the report is
byte-identical across runs and machines. The last line is `result: PASS, 0
findings` or `result: FAIL, N findings`. At most 1,000 findings are printed; if
there are more, a `VT03` line says so (the result is FAIL regardless).

| Exit | Meaning |
|---|---|
| 0 | PASS: no findings |
| 1 | FAIL: one or more findings, including every content-level failure (a file that does not lex, a cap exceeded by content) |
| 2 | no verdict: usage error, a missing `Cargo.toml`, or the filesystem could not be read safely (a symlink or non-regular file under the crate, an I/O error, a file over its cap before reading, non-UTF-8 text) |

**Fail closed.** A source file that does not lex (unterminated string or
comment, unbalanced `() [] {}`, a character the lexer cannot classify, more
than 262,144 tokens) is **VT03** and the run FAILs; the file is never "skipped".
A `Cargo.toml` outside the supported TOML subset is VT03 too.

**Filesystem (binary only).** The library is pure (text in, report out). The
binary reads with the VOS02 section 6.8 discipline: it `lstat`s every path and
refuses a symlink at any entry it visits, reads only regular files, checks the
size before reading and bounds the read after it, never follows a link and
never writes.

---

## 4. The lexical rules

Tokens are identifiers (raw identifiers `r#x` are *not* the keyword `x`),
lifetimes, numbers (integer or float), strings and characters, and punctuation.
Comments and the contents of strings are never code. The contents of an
attribute (`#[...]`, `#![...]`) are examined only by the attribute rules.
`::`, `->`, `=>` and `..` are single tokens; every other punctuation mark is one
character and records whether the next character is punctuation (so `<<`, `>>`,
`/=` are recognised without mistaking the `>>` that closes `Vec<Vec<u8>>`; a
`>>` or `<<` is read as a shift by the rule of SH9). *Rule text cites VOS01 or
VOS02; the second column is the exact lexical check.*

| Id | The check | Risk and what it cannot see |
|---|---|---|
| **SH1** | (a) `src/lib.rs` carries the inner attribute `#![no_std]`; (b) the identifier `extern` followed by `crate` anywhere; (c) the identifier `std` or `alloc` followed by `::`, or following `use`, unless it is the second segment of a path (so `core::alloc::Layout` passes) | (a) accepts the attribute text, not its effect (a `cfg_attr` spelling is rejected, so it only errs strict). (c) a path to `std` through a `use` alias is caught at the `use` |
| **SH2** | (a) the identifier `unsafe`; (b) `extern` not followed by `crate` (foreign linkage); (c) `asm`, `global_asm`, `naked_asm`, `llvm_asm` followed by `!`; (d) an attribute naming `link` or `link_name`; (e) `#![forbid(unsafe_code)]` present in `src/lib.rs`; (f) any `allow`, `warn`, `deny`, `expect` or `cfg_attr` attribute naming `unsafe_code` (a waiver, VOS01 SH2 "and no waiver") | Lexical; `unsafe` produced by a macro is excluded because SH15 bans non-core macros. `forbid` is required, not `deny`, because an inner `allow` overrides `deny` |
| **SH3** | (a) the identifier `static` (a lifetime `'static` is a lifetime token, not this); (b) `thread_local` followed by `!`; (c) a `const` item (`const NAME: T`, `const _: T`, associated or trait) whose type tokens contain an interior-mutability name: `Cell`, `RefCell`, `UnsafeCell`, `OnceCell`, `LazyCell`, `OnceLock`, `LazyLock`, `Mutex`, `RwLock`, `SyncUnsafeCell`, any `Atomic*`, **or** the name of a `struct`, `enum`, `union` or `type` of the same crate whose definition mentions such a name (computed to a fixed point, so wrapper types are caught) | (c) is by name: an interior-mutable type of another crate under a name outside the list passes. A `const` generic parameter `<const N: usize>` has no `=` and is not a const item |
| **SH4** | (a) the identifiers `dyn`, `Fn`, `FnMut`, `FnOnce`; (b) `fn` immediately followed by `(` (a function-pointer type); (c) a closure: a `|` or a `||` whose previous token is `(`, `,`, `=`, `{`, `;`, `[`, `:`, `=>`, `->`, `return`, `move` or `async`, or the keyword `move` followed by `|`; (d) `package.edition` is the literal `"2021"` or `"2024"` (older editions accept a bare trait object with no `dyn`) | (c) is a heuristic and a leading `\|` in a pattern after `(` or `{` is a false positive. A function item coerced to a pointer without a written type is invisible |
| **SH5** | the name-resolved lexical call graph is acyclic. Nodes are `fn` items keyed by (enclosing `impl` or `trait` type or none, name). An edge goes from the innermost enclosing `fn` to: a bare `f(` (the free `f`); `Self::f(` (the current type); `Q::f(` or `<Q as T>::f(` (the key (Q, f) when the crate defines it, the methods named `f` when `Q` is a trait of the crate, otherwise no edge); `x.f(` (every method named `f`, since the receiver type is unknown); a lowercase path `m::f(` (the free `f`). Every node on a cycle, direct recursion included, is reported once at its `fn` line with one cycle | by name, so two unrelated methods called `f` can look like recursion (reported; rename). Calls into **other crates**, through generic parameters (`<P::Ready as ReadySet>::insert` has qualifier `P`, no edge), through macros and through a path to an `fn` value are invisible: cross-crate recursion is the static stack bound (VOS-028) |
| **SH6** | (a) the identifiers `while` and `loop`; (b) every `for` that is a loop (the pattern is followed by `in` before the next `{`) has exactly the form `for IDENT in 0..K {` with `K` an integer literal or a path whose last segment is a constant name (upper-case letters, digits and `_`, e.g. `LEN`, `Self::N`); `0..=K`, a non-zero start and an expression bound are findings; (c) a method call (`.name(` or `.name::<..>(`) whose name is an iterator adaptor: `iter`, `iter_mut`, `into_iter`, `map`, `filter`, `filter_map`, `flat_map`, `flatten`, `zip`, `enumerate`, `rev`, `fold`, `try_fold`, `collect`, `step_by`, `skip`, `skip_while`, `take_while`, `chain`, `sum`, `product`, `any`, `all`, `for_each`, `windows`, `chunks`, `chunks_exact`, `position`, `find_map`, `peekable`, `scan`, `cycle`, `nth` | that `K` is the *intended* bound (VOS01 rule 7) is not lexical: a const that equals 1,000,000 passes. (c) is by name and can fire on `Option::map` |
| **SH7** | the punctuation `/` or `%` (so `/=` and `%=` too; comments are already gone); a method call or path whose name is `div`, `rem`, or an identifier with `div` or `rem` as an `_`-separated segment (`div_euclid`, `checked_div`, `wrapping_rem`, `div_ceil`), followed by `(` | the operator is seen only as written. A division inside a library the component calls is the image companion |
| **SH8** | (a) a method call named `unwrap`, `expect`, `unwrap_err`, `expect_err` or `unwrap_unchecked`; (b) the macros `panic`, `unreachable`, `todo`, `unimplemented`, `assert`, `assert_eq`, `assert_ne`, `debug_assert`, `debug_assert_eq`, `debug_assert_ne`, **except** `assert!` inside a `const _: () = ..;` item (compile time); (c) an index expression `e[ix]` (a `[` whose previous token is a non-keyword identifier, `)` or `]`) unless `ix` is exactly one identifier that is the variable of an enclosing `for v in 0..K` loop; everything else (a literal, `i + 1`, a range) is a finding ("else `get`") | the check that `K` equals the indexed array's own length, and every panic inside a called function (`copy_from_slice`, `split_at`, `swap`, `[]` on a slice in another crate), are not lexical: the image companion (no path to the panic handler) |
| **SH9** | a shift (`<<`, `>>`, `<<=`, `>>=`) whose right operand does not start with an integer literal is a finding; the fix is `wrapping_shl/shr` or `checked_shl/shr`. A `>>` is a shift only when the token after it is an integer, a float, an identifier other than `where`, `as`, `for`, `in`, or `(` (otherwise it closes generics); a `<<` is a shift unless it follows `::` or is followed by `ident as` (`<<T as X>::Y>`) | "provably below the bit width" by a guard (`c < N` plus a `const` assert) is **not lexical**: such a shift is reported and rewritten. **Bare `+ - *` are not checked** (a binary operator cannot be told from a trait-bound `+`, a unary `-` or a deref `*` without a parser): `clippy::arithmetic_side_effects` denied in CI is the second line, and wiring it is VOS-D032 |
| **SH10** | (a) the identifiers `Vec`, `String`, `Box`, `Rc`, `Arc`, `HashMap`, `HashSet`, `BTreeMap`, `BTreeSet`, `VecDeque`, `BinaryHeap`; (b) `isize`, and `usize` unless it is the cast target in `as usize` or the type of a `const NAME: usize` item or const generic parameter (the loop-bound and index uses of VOS01 section 1.3); (c) a reference type: any `&` inside a `fn` signature (from `fn` to the body or `;`, including `&self`), inside a `struct`, `enum` or `union` body, or a `&` followed by a lifetime | an alias (`type Idx = usize;`) is reported at the alias; "usize only as a loop or array index" is not checkable beyond the three admitted spellings; references local to a function body are not types of `State`, `In` or `Out` and are not reported unless they carry a lifetime |
| **SH11** | the identifiers `f32`, `f64`; a float literal (`1.0`, `1e3`, `2f32`) | exact |
| **SH12** | read from `Cargo.toml`: every `[dependencies]`, `[build-dependencies]` and `[target.*.dependencies]` entry (`[dev-dependencies]` are exempt, see below) must be `core` or a path dependency (no `version`, `git`, `registry`, `workspace`, `package`) whose name matches `vos-<iface>-contract` (`-` and `_` equivalent); anything else is a finding at its line | the name pattern stands in for "the interface's own contract crate"; whether the `path` really points at the contract crate is the closed-world manifest check of `vos-link` (VOS02 section 7.2). `[dev-dependencies]` do not count (they build only under `cargo test`, never into an image or the evidence library; the generated harness needs `coding_adventures_sha256` and `coding_adventures_test_bootstrap`, backlog VOS-D022) |
| **SH13** | (a) a `mod ghost` item is preceded by exactly `#[cfg(feature = "F")]` with `F` an evidence feature; (b) no `fn` named `alpha` or `alpha_*` or `vos_ghost_*` outside the ghost module; (c) no evidence feature is reachable from `default` in `[features]` | the *image build* not enabling the feature is VOS22's resolved-graph check (SH16); (c) reads the manifest only |
| **SH14** | **only its field-privacy half**, for kinds `production`, `reference`, `any` (a component implementation): a `struct` outside the ghost module has no field written bare `pub` (`pub(crate)` passes). The spine half (a field passes only to its own interface's operations) needs the UFCS reach pass: VOS-D024 and VER29 slice 4 | privacy is a compiler fact once `pub` is absent |
| **SH15** | (a) `macro_rules` followed by `!`; (b) `include`, `include_str`, `include_bytes` followed by `!`; (c) any other macro invocation (`ident ! (` or `[` or `{`, identifier not a keyword) other than the pure `core` list `matches`, `cfg`, `compile_error`, `concat`, `stringify`, `line`, `column`, `file`, `module_path` (the SH8 and SH3 macros are reported by their own rule, `env!` and `option_env!` are not on the list); (d) an attribute naming `path`, `link_section`, `no_mangle`, `export_name`, `link` or `link_name` (inside `cfg_attr` too); (e) a file named `build.rs` among the crate-root entries; (f) in `Cargo.toml` a `build` key under `[package]` of **any** value (`false` included, in any TOML spelling: dotted key, inline table, quoted key), a `links` key, `proc-macro` (or `proc_macro`) that is not the literal `false`, a `crate-type` array containing `proc-macro`; (g) a `[lib]` or `[[bin]]` `path` that is absolute, contains `..`, or does not start with `src/` | a macro or build script of another crate is invisible; all of the above are bans, not analyses, so they cannot be "mostly" satisfied |
| **SH17** | in the ghost scope (an inline `mod ghost { }` or the files `src/ghost.rs`, `src/ghost/**`): (a) every `fn` is named with the prefix `vos_ghost_`, and every leaf of a crate-root `pub use ghost::...` is named with it too (an alias counts; a glob is a finding); (b) a ghost `fn` whose body contains a UFCS call `<T as Trait>::name(` is a **wrapper**: it contains exactly one such call and no other call (`ident(` or `.m(`), no `if`, `else`, `match`, `while`, `loop`, `for`, `return`, `break`, `continue`, no `as` outside the call, and none of the operators `+ - * / % ^ | & ! ?` and no `<`, `>` outside the call; (c) every `let` pattern in a wrapper is an exhaustive destructure: a struct pattern with no `..` whose field names are exactly the fields of the crate's non-ghost `struct` of that name; (d) every ghost `struct` built with `Name { .. }` in a wrapper whose body destructures a production struct `S` (the *image* of `S`) has the same field names, the same field types as token text, in the same order as `S`; (e) a ghost `struct` named `Image` that no wrapper pairs with a production struct is a finding | the wrapper shape is the VOS02 section 4.6 text; **`alpha` and the configuration exports have no UFCS call and are therefore not wrappers**: they are prefixed (a) but their bodies are free (the "same four-line shape" of VOS02 section 4.2 cannot hold for them, backlog VOS-D033). The prefix rule is applied to `fn`s and root re-exports only: the types `Image` and `AbsW` of the VOS02 section 4.2 listing are not prefixed (VOS-D033) |
| **SH18** | (a) every `cfg`, `cfg_attr` or `cfg!` naming `feature = "F"`: `F` must be declared in `[features]`; if `F` is an evidence feature (the central list, plus any name `vos.toml` declares) the use must be exactly `#[cfg(feature = "F")]` on a `mod ghost` item or on a `pub use ghost::..` item, and nowhere else (not in `cfg_attr`, not `cfg!`, not `not(..)`, not in a combination); a feature that is not an evidence feature must not gate code in a class K crate; (b) outside the ghost scope, a path containing `ghost ::`, or a `use` mentioning `ghost`, or an identifier starting `vos_ghost_`, is a finding unless it is inside such a gated `pub use ghost::..` item; (c) every name in `vos.toml` `evidence-features` is in the central list (**VT01** otherwise) | Cargo's resolved feature set (unification through other dependents) is not computed: that is the resolved-graph check of VOS22 |

---

## 5. Worked examples

The toy crates of VOS01 section 5.3 and 5.4 and the ghost face of VOS02 section
4.2 are the **positive controls** (`tests/fixtures/pass-*`): they must PASS.
Each broken variant in `tests/fixtures/fail-*` must FAIL with a named rule.
Every rule also has a clean and a violating case in the unit tests; no rule is
shipped without a test that fails when the rule is removed.

---

## 6. What this check cannot see (the residue)

1. **Macros.** SH15 bans `macro_rules` and every non-core macro in the crate
   under check, so no macro of the crate can expand into a forbidden item. The
   core macros on the list (`matches!`, `cfg!`, ...) expand into `match` and
   constants the lexer does not see; a macro from another crate does not
   exist (SH12).
2. **Names.** No resolution: SH5 is by name, SH3 and SH10 by type names, SH8's
   panic methods by method name. A type alias to a forbidden type defined in
   another crate and a `use ... as` rename of a banned name outside the lexer's
   patterns are invisible.
3. **Values.** `for i in 0..K` bounds are checked as *spelled*, not by value;
   an index loop bound equal to the array's own length is not checked; the
   guard behind a shift or an index is not evaluated.
4. **Arithmetic.** Bare `+ - *` (SH9) are not detected here; `clippy::
   arithmetic_side_effects` in CI is the other line (VOS-D032).
5. **Callees in other crates.** A panic, a division, a loop or recursion in a
   function the component calls, even in `core`, is invisible: the two
   image-level companions (VOS-026, VOS-028) close this.
6. **Resolution of features and of the build.** What Cargo enables
   (unification, `--features`, `[patch]`) and what the build produces is the
   hermetic recorded build of VOS22 (SH16).
7. **Self-declaration.** A `vos.toml` class tag *relaxes* rules, and a crate
   can write `class = "M"` for itself. The tag is on the CODEOWNERS authority
   list, and `vos-link` must cross-check class and kind against the profile
   and a central crate-to-class registry (backlog VOS-D030). This tool checks
   the tag's syntax and values, not its truth.
8. **`cfg(test)` masking** (section 1) and the files outside `src/`.
9. **SH14 spine half, SH16, and the audits of VOS02 section 5** (the
   `impl Profile` block, `site_` claims): VOS-D024, VOS-034 and VOS22.

---

## 7. Conformance

- [ ] one clean and one violating unit test per rule in section 4, and the
      VT01 to VT03 rules;
- [ ] the VOS01 section 5 toy crates and the VOS02 section 4.2 ghost face pass;
      broken variants of each fail with the expected rule ids;
- [ ] the report is deterministic and the exit codes are 0, 1, 2 as in section 3;
- [ ] over-cap, unlexable, symlinked and non-UTF-8 inputs fail closed;
- [ ] line coverage of the crate is above 95%;
- [ ] the crate has zero third-party dependencies and does not depend on
      `ca-capability-analyzer` (section 0).

---

## 8. Findings and open questions

Logged in [VOS-BACKLOG](VOS-BACKLOG.md): VOS-D029 (VER08a has no public token
API; the lexer is duplicated here), VOS-D030 (class tag source: README in VOS01
and VOS02 against `vos.toml` here; self-declared relaxation), VOS-D031 (class M
binding conflicts: EMU00 section 7.5 against VOS01 section 1.6, and the literal
M row against the wrapped simulators), VOS-D032 (SH9 bare arithmetic and the
guarded-shift exemption are not lexical), VOS-D033 (SH17 text against the
VOS02 section 4.2 listing: `Image` and `AbsW` are not prefixed, `alpha` and
`cfg` exports are not four-line wrappers).
