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
| every `*.rs` file under `src/` | recursive, depth at most 16 | 1 MiB each, 512 files and 8 MiB in total (the total is enforced while walking) |
| the names of the entries of the crate root | to find `build.rs`, `.cargo`, `rust-toolchain*` | 4,096 entries per directory, 16,384 in all |
| the central evidence-feature list | an argument; default `ghost`, `cov` (VOS02 section 7.3) | 64 names |

Files outside `src/` (`tests/`, `benches/`, `examples/`) are **not read**: they
are not part of the shipped library, and `tests/generated/` evidence harnesses
may use `std`, `unwrap`, floats and the rest. This is a trust statement, not a
proof: those files RUN in the evidence build, so a harness that lies about the
code under test is outside this check (the generated document and its digest,
VOS02 R1-R4, are what bind the harness). The dev-dependencies they use are an
allowlist (SH12), and whether the harness sources should themselves be scanned
is backlog VOS-D035.

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
| 2 | **no verdict (blocks, like 1):** usage error, a missing `Cargo.toml`, or the filesystem could not be read safely (a symlink or non-regular file under the crate, an I/O error, a file over its cap before reading, non-UTF-8 text) |

**Every non-zero exit blocks.** Exit 2 means the tool could not look, which is
never a pass: a CI wrapper must fail the job on any non-zero status and must
never treat 2 as "skipped" or "not applicable".

**Fail closed.** A source file that does not lex (unterminated string or
comment, unbalanced `() [] {}`, a character the lexer cannot classify, more
than 262,144 tokens) is **VT03** and the run FAILs; the file is never "skipped".
A `Cargo.toml` outside the supported TOML subset is VT03 too.

**Caps in the binary.** The binary refuses (exit 2) a `Cargo.toml` or `vos.toml`
over its cap before reading; the library, given the same text, reports VT01 or
VT03 (exit 1). Both are fail closed.

**Filesystem (binary only).** The library is pure (text in, report out). The
binary reads with the VOS02 section 6.8 discipline: it `lstat`s every path and
refuses a symlink at any entry it visits; it opens a file with `O_NOFOLLOW |
O_NONBLOCK` (inlined constants for macOS, FreeBSD and Linux, no dependency) and
then checks the **open handle**: a regular file and, on Unix, the same `dev` and
`ino` the `lstat` saw, so a swap between check and open is refused rather than
trusted and a FIFO cannot hang the open; it checks the size before reading and
bounds the read after it; it enforces depth, per-directory entries (4,096),
total entries (16,384) and total bytes **during** the walk; and it never writes.
On a case-insensitive filesystem `Lib.RS` is compiled and `Build.rs` is run by
Cargo, so an entry whose lower-cased name ends `.rs` but whose exact name does
not, and two entries that differ only in case, are refused (exit 2); `build.rs`,
`.cargo` and `rust-toolchain*` are matched case-insensitively (SH15). Non-Unix
platforms have no `O_NOFOLLOW` here and rely on the `lstat` and open-handle
`is_file` checks only.

---

## 4. The lexical rules

Tokens are identifiers (a raw identifier `r#name` is **normalised to `name`**
unless `name` is a keyword, so `o.r#unwrap()` is `unwrap`; `r#unsafe` stays a
non-keyword identifier),
lifetimes, numbers (integer or float), strings and characters, and punctuation.
A float may end in a dot (`1.` is a float; `1..2` and `1.max(2)` are not). A
first line `#!...` is a shebang and is skipped only when the next significant
token after `#!` (past whitespace and comments) is not `[`: `#! [attr]` is an
inner attribute. Comments and the contents of strings are never code. The contents of an
attribute (`#[...]`, `#![...]`) are examined only by the attribute rules.
`::`, `->`, `=>` and `..` are single tokens; every other punctuation mark is one
character and records whether the next character is punctuation (so `<<`, `>>`,
`/=` are recognised without mistaking the `>>` that closes `Vec<Vec<u8>>`; a
`>>` or `<<` is read as a shift by the rule of SH9). *Rule text cites VOS01 or
VOS02; the second column is the exact lexical check.*

| Id | The check | Risk and what it cannot see |
|---|---|---|
| **SH1** | (a) `src/lib.rs` carries `#![no_std]` as a **crate-level** inner attribute (the leading run of `#![..]` at the start of the file; one inside `mod m { }` applies to `m`); (b) the identifier `extern` followed by `crate` anywhere; (c) the identifier `std` or `alloc` followed by `::`, or following `use`, unless it is a later segment of a path whose previous segment is a non-keyword identifier (`core::alloc::Layout` passes; `use ::std::..` and `return ::std::..` do not) | (a) accepts the attribute text, not its effect (a `cfg_attr` spelling is rejected, so it only errs strict) |
| **SH2** | (a) the identifier `unsafe`; (b) `extern` not followed by `crate` (foreign linkage); (c) `asm`, `global_asm`, `naked_asm`, `llvm_asm` followed by `!`; (d) an attribute naming `link` or `link_name` (**SH2, not SH15**); (e) exactly `#![forbid(unsafe_code)]` as a crate-level inner attribute of `src/lib.rs` (`forbid(unsafe_code, x)` is not it); (f) any `allow`, `warn`, `deny`, `expect` or `cfg_attr` attribute naming `unsafe_code` (a waiver) | `unsafe` produced by a macro is excluded because SH15 bans non-core macros. `forbid` is required, not `deny`, because an inner `allow` overrides `deny` |
| **SH3** | (a) the identifier `static` (a lifetime `'static` is a lifetime token); (b) `thread_local` followed by `!`; (c) a `const` item (`const NAME: T`, `const _: T`, associated or trait) whose type tokens contain an interior-mutability name: `Cell`, `RefCell`, `UnsafeCell`, `OnceCell`, `LazyCell`, `OnceLock`, `LazyLock`, `Mutex`, `RwLock`, `SyncUnsafeCell`, any `Atomic*`, **or** the name of a `struct`, `enum`, `union` or `type` of the same crate whose definition mentions such a name (a worklist over a reverse map, linear) | (c) is by name: an interior-mutable type of another crate under a name outside the list passes. The type scan of a const is bounded (256 tokens) and a type definition is read to its end or the next item keyword (4,096 tokens at most) |
| **SH4** | (a) the identifiers `dyn`, `Fn`, `FnMut`, `FnOnce`; (b) `fn` immediately followed by `(`; (c) a closure: a `|` or `||` whose previous token is `(`, `,`, `=`, `{`, `;`, `[`, `:`, `=>`, `->`, `&`, `*`, `!`, `return`, `move`, `async`, `else`, `in`, `break`, `yield`, `await`, or the keyword `move` followed by `|`; (d) `package.edition` is the literal `"2021"` or `"2024"` | (c) is a heuristic and a leading `\|` in a pattern after `(` or `{` is a false positive. A function item coerced to a pointer without a written type is invisible |
| **SH5** | the name-resolved lexical call graph is acyclic: **every strongly connected component of two or more functions, and every self-loop, is reported** (iterative Tarjan, O(V+E)). Nodes are `fn` items keyed by (enclosing `impl` or `trait` type, or one synthetic key for an impl on an array, tuple, unit or pointer type, or none) and name. Edges go from the innermost enclosing `fn` to: a bare `f(`; `Self::f(`; `Q::f(`, `Q::<T>::f(` (qualifier BEFORE the turbofish), `Q<T>::f(` and `<Q as T>::f(` (the key (Q, f); every method named `f` if `Q` is a trait of the crate; otherwise no edge); `x.f(` (every method named `f`); a lowercase path `m::f(`. More than 200,000 edges, 4,096 functions, or a function header over 1,024 tokens is **VT03** (fail closed) | by name, so unrelated methods called `f` can look like recursion (rename). Calls into other crates, through generic parameters (`<P::Ready as ReadySet>::insert` has qualifier `P`), macros or fn values are invisible: cross-crate recursion is the static stack bound (VOS-028) |
| **SH6** | (a) the identifiers `while` and `loop`; (b) every `for` that is **not inside an `impl` header** (and not `for<'a>`) is a loop and must be exactly `for IDENT in 0..K {` with `K` an integer literal or a path whose last segment is a constant name (`LEN`, `Self::N`); a `for` with no `in`, `0..=K`, a non-zero start, an expression bound or a pattern other than the shape (`for S { a } in ..`) is a finding; (c) a method call or `::` path call whose name is an iterator adaptor: `iter`, `iter_mut`, `into_iter`, `map`, `filter`, `filter_map`, `flat_map`, `flatten`, `zip`, `enumerate`, `rev`, `fold`, `try_fold`, `collect`, `step_by`, `skip`, `skip_while`, `take_while`, `chain`, `sum`, `product`, `any`, `all`, `for_each`, `windows`, `chunks`, `chunks_exact`, `position`, `find_map`, `peekable`, `scan`, `cycle`, `nth`, `count`, `reduce`, `find` | that `K` is the *intended* bound (VOS01 rule 7) is not lexical. (c) is by name and can fire on `Option::map`. **Decided, not denied:** `last`, `max`, `min`, `take` (total, O(1) on integers, options and slices; they iterate only an iterator, which an adaptor above or a range expression produces, and `(0..N).max()` is the residual in section 6) |
| **SH7** | the punctuation `/` or `%` (so `/=`, `%=`); a method or path call whose name is `div`, `rem`, or has `div` or `rem` as an `_`-separated segment (`div_euclid`, `checked_div`, `wrapping_rem`, `div_ceil`), followed by `(` | the operator is seen only as written. A division inside a library the component calls is the image companion |
| **SH8** | (a) a method or `::` path call named `unwrap`, `expect`, `unwrap_err`, `expect_err` or `unwrap_unchecked` (`Option::unwrap(o)` counts); (b) the macros `panic`, `unreachable`, `todo`, `unimplemented`, `assert`, `assert_eq`, `assert_ne`, `debug_assert`, `debug_assert_eq`, `debug_assert_ne`, **except** `assert!` inside a `const _: () = ..;` item whose initializer contains no `fn` or `impl` token; (c) every `[` is an index unless the previous token is on an ALLOWLIST of array-literal, array-type and attribute positions (`=`, `(`, `,`, `;`, `{`, `:`, `#`, `!`, `&`, `<`, `->`, `=>`, `[`, `\|`, `..`, `+ - * / % ^`, a lifetime, or a keyword other than `self`/`Self`): `w.0[i]`, `f()?[i]`, `{..}[i]`, `self[i]`, `b"ab"[n]` are indexes. An index is allowed only if it is exactly one identifier that is the variable of an enclosing `for v in 0..K` loop AND no `let` (including `if let` and tuple patterns) between the loop start and the index re-binds that name | that `K` equals the indexed array's length, and every panic inside a called function, are not lexical (image companion). Shadowing by a `match` arm, a closure parameter or a nested `for` pattern is not seen (section 6). A loop body over 8,192 tokens exempts no index |
| **SH9** | (a) a shift (`<<`, `>>`, `<<=`, `>>=`) is a finding unless its right operand is an integer literal that is the **whole** operand: the next token must end it (`; ) ] } { , \| & ^ = < > ? : ! =>` or `as`), so `x << 1 + n`, `x << 1.max(n)`, `x << 8 * n` are findings. A `>>` is a shift unless two unmatched `<` precede it in the same statement (`Vec<Vec<u8>>`); a `<<` is generic only after `::` or when the group it opens closes right before `::` (`<<T as X>::Y>`); (b) a method or path call named `pow`, `shl` or `shr` | "provably below the bit width" by a guard is **not lexical**: the shift is reported and rewritten. **Bare `+ - *` are not checked** (VOS-D032): `clippy::arithmetic_side_effects` denied in CI is the second line |
| **SH10** | (a) the identifiers `Vec`, `String`, `Box`, `Rc`, `Arc`, `HashMap`, `HashSet`, `BTreeMap`, `BTreeSet`, `VecDeque`, `BinaryHeap`; (b) `isize`, and `usize` unless it is the cast target in `as usize` or the type of a `const NAME: usize` item or const generic parameter; (c) a reference type: any `&` inside a `fn` signature (from `fn` to the body or `;`, including `&self`), inside a `struct`, `enum` or `union` body, or a `&` followed by a lifetime | an alias (`type Idx = usize;`) is reported at the alias; references local to a function body are not types of `State`, `In` or `Out` and are not reported unless they carry a lifetime |
| **SH11** | the identifiers `f32`, `f64`; a float literal (`1.0`, `1.`, `1e3`, `2f32`, `1_f64`) | exact |
| **SH12** | read from `Cargo.toml`: every `[dependencies]`, `[build-dependencies]` and `[target.*.dependencies]` entry must be `core` (with no source key) or a path dependency (exactly `{ path = ".." }`) whose name matches `vos-<iface>-contract` (`-` and `_` equivalent). **Dev-dependencies are an allowlist, not "ignored":** an in-repo `{ path = ".." }` crate that is `coding_adventures_sha256`, `coding_adventures_test_bootstrap` (the generated harness's needs, VOS-D022) or a contract crate, in `[dev-dependencies]` and `[target.*.dev-dependencies]`; anything else is a finding | the name pattern stands in for "the interface's own contract crate"; that the `path` really points at it is **`vos-link`'s closed-world manifest check (VOS02 section 7.2), not this tool's** (VOS-D034). Evidence trust: a dev-dependency RUNS in the evidence harness, so the allowlist is part of the evidence's trust base |
| **SH13** | (a) a `mod ghost` item is preceded by exactly `#[cfg(feature = "F")]` with `F` an evidence feature; (b) no `fn` named `alpha`, `alpha_*` or `vos_ghost_*` outside the ghost module; (c) no evidence feature is reachable from `default` in `[features]` | the *image build* not enabling the feature is VOS22's resolved-graph check (SH16); (c) reads the manifest only |
| **SH14** | **only its field-privacy half**, for kinds `production`, `reference`, `any` **and for an unspecified kind in class K** (a missing `vos.toml` must not switch the rule off): a `struct` or `union` outside the ghost module has no field written bare `pub`, named or tuple (`struct S(pub u8)`); `pub(crate)` passes. Enum variants carry no visibility. The spine half (a field passes only to its own interface's operations) needs the UFCS reach pass: VOS-D024 and VER29 slice 4 | privacy is a compiler fact once `pub` is absent |
| **SH15** | (a) `macro_rules` followed by `!`; (b) `include`, `include_str`, `include_bytes` followed by `!`; (c) any other macro invocation other than the pure `core` list `matches`, `compile_error`, `concat`, `stringify`, `line`, `column`, `file`, `module_path` (**`cfg!` is not on it**: SH18); (d) an attribute naming `path`, `link_section`, `no_mangle` or `export_name` (inside `cfg_attr` too; `link`, `link_name` are SH2); (e) a crate-root entry named `build.rs` **in any case**, `.cargo`, or `rust-toolchain*`; (f) in `Cargo.toml` (`[package]` or its alias `[project]`) a `build` key of **any** value in any TOML spelling, a `links` key, `proc-macro` not the literal `false`, a `crate-type` containing `proc-macro`; (g) `[lib] path` other than exactly `src/lib.rs`, any `[[bin]]` target or `src/main.rs` or `src/bin/**` file (a component is a library; a binary is a second crate root the checks would not cover), and a `[[test]]`, `[[example]]` or `[[bench]]` path not under `tests/`, `examples/`, `benches/` | a macro or build script of another crate is invisible; all of the above are bans, not analyses |
| **SH17** | in the ghost scope (an inline `mod ghost { }` or the files `src/ghost.rs`, `src/ghost/**`): (a) every `fn` is named with the prefix `vos_ghost_`, and every leaf of a crate-root `pub use ghost::...` is too (an alias counts; a glob is a finding); (b) **only a CLOSED list of exports may have a free body: `vos_ghost_alpha`, `vos_ghost_alpha_*` and `vos_ghost_cfg_*`** (VOS02 section 4.4). Every other ghost function is a **wrapper** and must contain exactly one UFCS call `<T as Trait>::f(` and nothing else computing: no other call (`ident(`, `.m(`, `Type::f(`), no `if`, `else`, `match`, `while`, `loop`, `for`, `return`, `break`, `continue`, no `as` outside the call, none of `+ - * / % ^ \| & ! ?` and no `<`, `>` outside the call; a ghost function with no UFCS call that is not on the list is a finding, so a wrapper cannot be replaced by an inline reimplementation; (c) every `let` pattern in a wrapper is an exhaustive destructure with no `..` whose field names are exactly the fields of the crate's non-ghost `struct` of that name; (d) every ghost `struct` built with `Name { .. }` in a wrapper that destructures a production struct `S` has the same field names, types (as token text) and order as `S`; (e) a ghost `struct` named `Image` that no wrapper pairs is a finding; (f) a ghost function header over 1,024 tokens is a finding | the bodies of `alpha` and the `cfg_*` exports are not checked for faithfulness (the evidence's job). The prefix rule is applied to `fn`s and root re-exports only: the types `Image` and `AbsW` of VOS02 section 4.2 are not prefixed (VOS-D033) |
| **SH18** | (a) **every** `cfg`, `cfg_attr` and `cfg!` is a finding, in every class that binds SH18, **except** exactly `#[cfg(feature = "F")]` (`F` an evidence feature: the central list plus `vos.toml`'s) on a `mod ghost` item or a `pub use ghost::..` item, and the masked `cfg(test)` items. A `cfg(target_arch = ..)` gives the evidence host and the image two different crates, so it is not allowed whatever its predicate. Features named in a `cfg` must also be declared in `[features]`, and in class K a non-evidence feature must not gate code; (b) outside the ghost scope, `ghost` (not after `mod`), or an identifier starting `vos_ghost_`, is a finding unless inside such a gated `pub use ghost::..`; (c) every name in `vos.toml` `evidence-features` is in the central list (**VT01** otherwise) | Cargo's resolved feature set (unification through other dependents) is not computed: that is the resolved-graph check of VOS22 |

---

## 5. Worked examples

The toy crates of VOS01 section 5.3 and 5.4 and the ghost face of VOS02 section
4.2 are the **positive controls** (`tests/fixtures/pass-*`): they must PASS.
Three broken variants in `tests/fixtures/fail-*` fail with named rules (the
ghost leak, a build script, an image that drifted from its state), and every
sub-clause of section 4 that has a distinct token test has a clean and a
violating case in `tests/rules.rs`, `tests/edges.rs` or `tests/security.rs`
(the last holds one regression test per security-review item, each asserting a
specific rule id and each failing on the code it replaced). Two sub-clauses are
NOT asserted by a dedicated violating case: the `O_NOFOLLOW` open race (it cannot
be raced deterministically; the `dev`/`ino` comparison is reviewed, not tested)
and non-Unix behaviour. "A test that fails when the rule is removed" is the aim
for every rule, checked by hand for the review items, not by mutation testing.

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
8. **`cfg(test)` masking** (section 1) and the files outside `src/`: `tests/`,
   `examples/` and `benches/` are never scanned yet run in the evidence build
   (backlog VOS-D035).
9. **Shadowing and iterators.** An index variable shadowed by a `match` arm,
   a closure parameter or a nested `for` pattern is not seen (only `let`,
   `if let` and `while let`). `(0..N).max()` and `.last()` / `.take()` on a
   range are not denied (SH6), and a range expression used as an iterator
   without `.iter()` is invisible.
10. **Bounds of the scan.** A function or impl header over 1,024 tokens, a loop
    body over 8,192 tokens (no index exempt), a call graph over 4,096 functions
    or 200,000 edges, and a file over 262,144 tokens are refused or reported
    (VT03), never half-checked. Wide generic lists over 256 tokens are not
    recognised as generics.
11. **Filesystem.** Non-Unix platforms have no `O_NOFOLLOW`; the TOCTOU defence
    there is the `lstat` and open-handle `is_file` checks only. A case-folding
    collision cannot be created on a case-insensitive filesystem to be tested.
12. **SH14 spine half, SH16, and the audits of VOS02 section 5** (the
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
`cfg` exports are not four-line wrappers), VOS-D034 (the residue of section 6),
VOS-D035 (`tests/`, `examples/`, `benches/` are unscanned yet run in the evidence
build), VOS-D036 (round-1 security review: lexer, rule and loader bypasses, and
the lessons recorded), VOS-D037 (a closed list of ghost exports needs VOS02
section 4.4 to name them).
