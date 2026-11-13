# RS01 — Versioned Rust Grammars (0.1 to today)

**Status:** Draft, 2026-11-12. Spec only (slice A1); no grammar, script or
crate exists yet. RS01 follows [LANG84](LANG84-grammar-driven-versioned-frontends.md)
and amends it (see LANG84 "Amendment RS01"). It also amends
[VER25a](VER25a-rust-subset-grammar.md) and adds a note to
[VER25b](VER25b-rust-to-wir-frontend.md). Repository state read:
`origin/main` at `0bf84a881e1`.

## Overview

This spec defines a `.tokens`/`.grammar` pair for **every tagged Rust
release**, from the first public 0.1 (January 2012) to the newest tag, and,
from 1.0 on, for **every edition** each release supports. The pairs run on
the repository's generic `GrammarLexer` and `GrammarParser`, like the Python
([PY01](PY01-python-grammars.md)), C# ([CS01](CS01-csharp-grammars.md)) and
Java ([JV01](JV01-java-grammars.md)) grammars do. It covers four things the
earlier grammar specs never needed:

1. **A second selection axis.** A Rust program is read by a *release* of
   `rustc` under an *edition* (2015, 2018, 2021, 2024). `async` is an
   identifier in `rustc 1.85.0 --edition 2015` and a keyword in the same
   compiler with `--edition 2018`. The release alone does not decide the
   syntax.
2. **Macros that are not a text preprocessor.** C's `#define` and C#'s `#if`
   work on text or lines. Rust macros take **token trees**, return token
   trees, are re-parsed by fragment kind, and are hygienic. RS01 defines the
   layers that handle them (section 5).
3. **Lexical cases a regular expression cannot see**, such as nested block
   comments and the current shebang rule (section 4).
4. **Real executable oracles.** Every 1.x `rustc` is still downloadable, so
   most pairs can be checked against the compiler they describe (section 6).

### Why versioned Rust grammars?

Rust's syntax changed a great deal before 1.0 and has kept changing since,
just more politely:

| Era | Example of what changed | Why a single grammar fails |
| --- | --- | --- |
| 0.1–0.3 (2012) | `ret`, `alt`, `#fmt[...]` syntax extensions with *expression* arguments | none of this parses as modern Rust |
| 0.4–0.12 (2012–2014) | `fail!`, `~T` owned pointers, `#[attr];` inner attributes, `do` blocks | removed piece by piece; each release differs |
| 1.0–1.30 (2015–2018) | `?` (1.13), `pub(crate)` (1.18), `..=` and `impl Trait` (1.26) | features arrive release by release |
| 2018 edition | `async`/`await`/`dyn` become keywords | same compiler, two lexers |
| 2021 edition | reserved prefixes: `k#x` is now an error; `c"…"` C strings (1.77) | same compiler, three lexers |
| 2024 edition | `gen` reserved; `let` chains (1.88); `unsafe extern` required | same compiler, four lexers |

The uses are the same as PY01's: parse a historical program with the grammar
of its time, teach the language's evolution by diffing pairs, test that a
feature is a syntax error before its release, and build release- and
edition-aware tools. RS01 adds a fifth use that drives its order (section 9):
the Warrant proof front end and the capability analyzer both need a faithful
modern Rust lexer and parser **now**, and a hand-written one is what they use
today.

## Layer Position

```text
Rust release tags (rust-lang/rust) + RELEASES.md + rustc binaries (oracle)
        |
code/grammars/rust/rust<release>[-e<edition>].tokens|.grammar   <── YOU ARE HERE
        |
grammar-tools (validate, cross-validate, compile, deduplicate)
        |
GrammarLexer [+ the two Rust C3 hooks until X1/X3] --> GrammarParser --> CST
        |
        +--> cfg evaluator (L1)  --> macro expansion (L2/L3)  --> expanded CST
        |
        +--> consumers: Warrant WIR front end (rs-confined), capability
             analyzer (lexer first), formatters/LSP (CST only)
```

**Input from:** each tag's own lexer, token and parser sources; `RELEASES.md`;
`rustc_feature`'s accepted-feature list; the Reference and the edition guide;
the `rustc` binaries themselves.

**Output to:** the generic lexer and parser, and through them the consumers in
section 7.

## Relationship to other specs

| Spec | Relationship |
| --- | --- |
| LANG84 | The contract RS01 follows: every tagged release, materialized pairs, `releases.csv`, stages G and P, C1–C5. RS01 amends it for editions, Rust hooks, the expander, `semantic-copy` evidence and value-first order. |
| PY01, CS01, JV01 | Earlier versioned-grammar specs. PY01 is the closest precedent for `pair-evidence.csv` and release-keyed corpus overrides. CS01's preprocessor approach (directives are skip tokens) does not fit Rust (section 5.1). |
| PREP01 | The generic text preprocessor behind LANG84's "preprocessor dialects". RS01's expander is classified under an amended form of that exception, but it is not a PREP01 dialect. |
| F04, F10, F11 | Token groups, declarative mode transitions, adjacency predicates. RS01 uses all three, with no engine change until phase D. |
| VER25a, VER25b | The hand-written confined-Rust parser and its WIR lowering. RS01 replaces the parser with a grammar subset (`rs-confined`) after a differential test, and moves the lowering onto CST views. |
| VER08a | The Rust capability analyzer. Its lexer moves onto the shared Rust lexer in phase C (C4). |
| VER-BACKLOG | Tracks the slices of section 9 and LP-001a. |

---

## 1. Scope: every Rust release

### 1.1 What is in scope (owner direction, 2026-11-12)

**RS01 covers every published, semantically versioned Rust release, and
nothing else.**

| In scope | Out of scope |
| --- | --- |
| 0.1 through 0.12.0, including 0.3.1 | nightly toolchains (`nightly-YYYY-MM-DD`) |
| the published 1.0 pre-releases 1.0.0-alpha, 1.0.0-alpha.2 and 1.0.0-beta (semver pre-release versions) | beta-channel snapshots of any 1.x release |
| every 1.x minor and patch release | untagged 1.0 beta refreshes (beta.2 and later) |

Two consequences run through the rest of this spec:

- **Feature-gated syntax is in no grammar.** Syntax that exists only behind a
  nightly feature gate (`#![feature(...)]`) is not part of any pair, in any
  release or edition. This is the same rule as the stable-acceptance
  decision (section 3), seen from the inventory's side.
- **"Daily" is a CI schedule, not a toolchain.** Where this spec runs oracle
  sweeps on a schedule, it says "scheduled (daily) CI job". It never means
  a nightly Rust toolchain; every oracle binary is a pinned published
  release.

### 1.2 The inventory

The release unit is the upstream tag in `rust-lang/rust` (LANG84 "Every
tagged release"). `git ls-remote --tags` on 2026-11-12 listed 165 tag names:

| Group | Releases | Count | Channel |
| --- | --- | ---: | --- |
| Pre-1.0 finals | 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 0.10, 0.11.0, 0.12.0 | 12 | `final` |
| Pre-1.0 patch | 0.3.1 (tag `release-0.3.1`) | 1 | `patch` |
| 1.0 pre-releases | 1.0.0-alpha, 1.0.0-alpha.2, 1.0.0-beta | 3 | `alpha`, `alpha`, `beta` |
| 1.x minors | 1.0.0 … 1.99.0 | 100 | `final` |
| 1.x patches | 1.12.1 … 1.99.1 | 42 | `patch` |
| **Total tagged releases** | | **158** | |

**Excluded tags** (recorded in a header comment of `releases.csv`, with the
reason): `release-0.1` … `release-0.7` point at exactly the same commits as
`0.1` … `0.7`. They are aliases, not releases. `release-0.3.1` is the only
`release-` tag kept, because it is a real release with no other spelling.

**1.100.0 waits for its tag.** `RELEASES.md` lists 1.100.0 as released on
2026-11-12, but no tag was visible at `ls-remote` time. LANG84 covers tags
only, so its row is added by the first inventory change after the tag
appears, not before (decision Q13).

**Release IDs** are the upstream tag spellings exactly (`0.10`, `0.11.0`,
`1.0.0-alpha.2`, `1.99.1`). They are never compared as semantic versions
(LANG84).

**Dates** come from `RELEASES.md`, with two exceptions that the directory
README records: 0.3.1 is not in `RELEASES.md` and is dated `2012-07?` from
its tag; 1.0.0-beta (2015-04-03) is not in `RELEASES.md` and is dated from
the release announcement's source.

**Order and lineage.** Rows are in LANG84 canonical order (by date).
`predecessor` is:

- for a 1.x minor, the previous `X.Y.0` (lineage `branch-point`);
- for a 1.x patch, `X.Y.(z-1)`, or `X.Y.0` for the first patch (`line`);
- for 1.0.0-alpha, 0.12.0;
- for 0.3.1, 0.3. Because a predecessor must be an *earlier row*, 0.3.1 sits
  **after** 0.3 (2012-07-12). Its reduced-precision date `2012-07?` allows
  any day in July, so LANG84's order check accepts that position.

### 1.3 Counting pairs

From 1.0 on, a release has one pair per edition it supports (section 2).
Pre-1.0 releases have no editions and one pair each.

| Releases | Editions | Releases | Pairs |
| --- | --- | ---: | ---: |
| 0.1 … 0.12.0, 0.3.1 | none | 13 | 13 |
| 1.0.0-alpha … 1.30.x | 2015 | 45 | 45 |
| 1.31.0 … 1.55.x | 2015, 2018 | 34 | 68 |
| 1.56.0 … 1.84.x | 2015, 2018, 2021 | 43 | 129 |
| 1.85.0 … 1.99.1 | 2015, 2018, 2021, 2024 | 23 | 92 |
| **Today** | | **158** | **347** |
| 1.100.0, once tagged | all four | +1 | +4 = **351** |

Most of these pairs are identical to a neighbour. Section 2.5 explains how
they are stored and compiled without 347 copies of work.

---

## 2. The edition axis

### 2.1 What an edition changes

An edition is opt-in per crate (`edition = "2021"` in `Cargo.toml`, or
`rustc --edition`). Bare `rustc` defaults to 2015. Editions change the lexer,
the parser and macro matching, so they cannot be a semantic profile alone:

| Edition | First release | Lexer | Parser | Macros |
| --- | --- | --- | --- | --- |
| 2015 | 1.0.0 | baseline; `dyn` is a weak keyword from 1.27; `async`, `await`, `try` are identifiers | anonymous trait parameters `fn f(u8);` allowed | baseline |
| 2018 | 1.31.0 | `async`, `await`, `dyn` strict keywords; `try` reserved | anonymous trait parameters removed; `async fn` and `.await` from 1.39 | `?` Kleene operator from 1.32 (reaches 2015 in 1.37) |
| 2021 | 1.56.0 | reserved prefixes `ident#`, `ident"…"`, `ident'…'`; C strings `c"…"`/`cr#"…"#` from 1.77; raw lifetimes `'r#a` from 1.83 | bare trait objects and `...` range patterns are hard errors | `$x:pat` matches a top-level `\|` |
| 2024 | 1.85.0 | `gen` reserved; guarded strings `#"…"#` and `##` reserved | `unsafe extern` required; `#[unsafe(no_mangle)]` etc. required; `let` chains from 1.88 | `$x:expr` matches `const {}` and `_` |

Two subtleties that a reader of the edition guide can trip on:

- C strings and raw lifetimes are listed on the 2021 pages of the edition
  guide although they were stabilized later. They are edition-gated *and*
  release-gated: `c"x"` needs both 2021+ and 1.77+.
- Not every late change is edition-gated. 1.89 made a missing fragment
  specifier (`$x` with no `:frag` in a matcher) a hard error in **all**
  editions.

### 2.2 Pair stems and files

Every pair lives in `code/grammars/rust/`:

| Release kind | Stem | Example |
| --- | --- | --- |
| pre-1.0 | `rust<release>` | `rust0.4.tokens`, `rust0.4.grammar` |
| 1.0 and later | `rust<release>-e<edition>` | `rust1.85.0-e2024.tokens`, `rust1.0.0-alpha.2-e2015.grammar` |

Releases 1.0–1.30 carry `-e2015` too. Those compilers implement the 2015
language although `--edition` only became stable in 1.31, and a uniform
stem means a reader never has to know that.

The stem is parsed from the right: a trailing `-e` followed by four digits is
the edition, the rest after the `rust` prefix is the release ID. No Rust
release ID ends in `-e<digits>`; `release_inventory.py` refuses one in an
edition-axis inventory so the split can never be ambiguous.

```text
code/grammars/rust/
  README.md              date sources, lineage rules, exclusions, gaps
  releases.csv           158 rows (LANG84 schema + `editions` + `lineage`)
  subsets.csv            rs-token-trees, rs-confined (+ `allowlist` column)
  pair-evidence.csv      one row per (release, edition) pair
  oracle-toolchains.csv  pinned rustc archives (section 6.4)
  rs-confined.allowlist  the proof subset's gate (section 7.1)
  rust0.1.tokens         rust0.1.grammar
  …
  rust1.97.0-e2015.tokens  rust1.97.0-e2015.grammar
  rust1.97.0-e2018.tokens  rust1.97.0-e2018.grammar
  rust1.97.0-e2021.tokens  rust1.97.0-e2021.grammar
  rust1.97.0-e2024.tokens  rust1.97.0-e2024.grammar
  corpus/
    rs-token-trees/      lexical-pair probes (stage P)
    rs-confined/         the proof subset (section 7.1)
    rs-syntax/           full-grammar conformance cases (section 6.2)
    historical.csv       manifest of pre-1.0 upstream tests (section 6.5)
```

Each file's header names its release and edition and cites its evidence, as
Python's pairs do:

```text
# Rust 1.85.0 (2025-02-20), edition 2024: tokens.
# @version 1
# source_ref: https://github.com/rust-lang/rust/tree/1.85.0
# source_sha: <peeled commit>
# evidence: compiler/rustc_lexer/src/lib.rs blob …; compiler/rustc_span/src/symbol.rs blob …
# basis: reviewed | copy:<release>[-e<edition>] | semantic-copy:<release>[-e<edition>]
```

### 2.3 The `editions` column

`releases.csv` gains a Rust-specific column (LANG84 allows extra columns
after the required ones):

| Column | Values |
| --- | --- |
| `editions` | `-` for pre-1.0 rows; otherwise the `;`-separated editions the release supports, ascending, such as `2015;2018;2021;2024` |

The checker does not trust the column; it derives it. A header comment
declares each edition's first release:

```text
# editions: 2015@1.0.0-alpha 2018@1.31.0 2021@1.56.0 2024@1.85.0
```

A row's expected editions are its `predecessor`'s editions plus any edition
whose first release is this row. Lineage, not date, carries the set, so a
patch to an old line released after a newer minor never gains an edition.
1.0.0-alpha's predecessor is 0.12.0 (`-`), and it gains 2015.

### 2.4 One status per release, uniform across its editions

LANG84 keeps one status (`grammar_status`, `grammar_stage`,
`grammar_subset`) per `releases.csv` row. RS01 keeps that and adds a rule:
**a release's claims hold for every edition it lists.** A non-pending row
needs both files for every one of its editions, and a pair file whose edition
is not in the row's `editions` is an error. Editions of one release
therefore advance together. Per-pair details (which blobs, which basis) live
in `pair-evidence.csv`, one row per (release, edition).

The cost is small: a subset such as `rs-confined` (section 7.1) uses no
edition-dependent construct, so its four edition grammars differ only where
the editions differ, and identical files are deduplicated when compiled.

### 2.5 Deduplicating identical pairs

There are two kinds of sameness, handled at two levels.

1. **On disk, every pair is materialized.** LANG84 forbids aliases: no pair
   points at another's files. A pair identical to its predecessor (or to a
   sibling edition) is a *copy* whose header cites the evidence that allowed
   the copy (section 6.3). Copies are produced by `rust_release_pairs.py`,
   never by hand, and its `check` mode fails if a copy's body drifts from its
   source.
2. **In compiled tables, identical pairs collapse.** `grammar-tools
   generate-rust-compiled-grammars` already serializes each pair canonically
   (dropping comments and line numbers), hashes it with SHA-256, and emits one
   `g_<12 hex>` module per distinct hash. The Rust family adds edition stems:
   `token_grammar(release, edition)` and `parser_grammar(release, edition)`
   map each (release, edition) to its module.

Section 2.7 lists about 60 syntax boundaries, at most four editions each, so
the expected result is roughly **100–160 distinct modules for 351 pairs**.
If one crate's compiled size still becomes a problem, LANG84's open
question 7 (era cargo features) applies.

### 2.6 Selection

| Form | Meaning |
| --- | --- |
| `rust@0.4` | pre-1.0 release; no edition allowed |
| `rust@1.85.0/2021` | release 1.85.0, edition 2021 |
| `rust@1.85.0` | **error** `EditionRequired`: a 1.x release needs an edition |
| `rust@1.30.0/2018` | **error** `UnknownEdition`: 1.30.0 supports only 2015 |
| `rust@0.4/2015` | **error** `UnknownEdition`: pre-1.0 releases have no editions |

No edition is ever inferred, not even 2015, although bare `rustc` assumes it
(decision Q2). Consumers that read a crate know its edition from
`Cargo.toml`; tools that do not know it must say so (the capability
analyzer's dual-lex rule in section 7.3 is the model). A future Rust
frontend records the edition beside the release in `IIRModule` (LANG84
amendment). `DEFAULT_RELEASE` is an exact release ID, **1.97.0**, the first
modern release RS01 materializes; there is no default edition.

### 2.7 Provisional syntax classes

A *syntax class* is a run of releases whose pairs are identical for at least
one edition. The tables below list where a new class starts. Evidence tags:
**[R]** `RELEASES.md` "Language" (or the 0.x notes), **[A]** `accepted.rs`
`since` at 1.99.0, **[S]** source read at the tag, **[M]** the manual at the
tag, **[E]** edition guide or Reference, **[O]** local `rustc` 1.97 oracle.
Every row is provisional until the per-release oracle differential
(section 6) confirms it.

**Pre-1.0.** Every release is its own class, except that 0.3.1 copies 0.3.

| Release | Grammar-visible facts |
| --- | --- |
| 0.1 | [S] XID identifiers; block comments **nest**; tokens `#`, `#{`, `#<`, `<-` (move), `<->` (swap), `>>>`, `...`, `?`; keywords matched by the parser (`eat_word`), not reserved by the lexer; syntax extensions `#name(exprs)`/`#name[exprs]` with **expression** arguments, plus `#<ty>`, `#{block}` and `#macro([...])`; inner attribute `#[meta];`. [M] `.rc` crate files have their own grammar. |
| 0.2 | [R] `crust fn`, `loop`, `mutable`→`mut`, `let mut`, `export foo::*`, `alt check`, classes. |
| 0.3 (= 0.3.1) | [R] doc comments; `#!` comments; removed `be prove syntax note mutable bind crust native cont`; `do` expressions; `#line` … `#include_bin`. [S] `DOC_COMMENT` token; shebang only at line 1, column 0; nested comments. 0.3.1's lexer, token and parser blobs equal 0.3's. |
| 0.4 | [R] all keywords strict; `ret`→`return`, `alt`→`match` with `=>`; `import`→`use`, `use`→`extern mod`; `pub`/`priv`; structs. [M] `macro_rules!` with designators `item block stmt pat expr ty ident path tt matchers`. [S] block comments **no longer nest** (0.4–0.8). |
| 0.5 | [R] `<-` and `[T]/N` removed; `.rc` crate language removed; quasi-quoters; irrefutable patterns as arguments; tuple structs; `#fmt`→`fmt!` complete. |
| 0.6 | [R] `&'l T` lifetimes; `impl Trait for Type`; `Self`; explicit `self`; `+` between bounds; `const`→`static`; `fail!`/`assert!` macros; `(T,)`; `extern { }`; `[int, .. 3]`; `#[deriving(Eq)]`. [S] `LIFETIME` token first appears. |
| 0.7 | [R] no visibility on `impl`; `use mod` removed. |
| 0.8 | [R] `for x in iter`; `copy` removed; `yield`/`typeof` reserved; `extern mod foo = "url"`; `proto!` removed; `let`-hygienic `macro_rules!`. |
| 0.9 | [R]+[S] nested comments return; raw strings `r"…"`, `r#"…"#`; `continue`; `proc`; `\|a\| -> r` closure types; `@fn` and `fmt!` removed; `mut` in patterns; `0o` octal; non-ASCII identifiers feature-gated. |
| 0.10 | [R] `#![attr]` replaces `#[attr];`; `extern crate`; `do` removed; `{}` macro delimiters; trailing commas; `@` patterns removed. |
| 0.11.0 | [R] `~T`/`~[T]`/`~str`/`@T` removed; `b'…'`, `b"…"`, `br"…"`; `priv` removed; struct literals barred in `if`/`while`/`match`/`for` heads. |
| 0.12.0 | [R] `if let`; `where`; `[0..4]` slicing (gated); `[a, b, c..]`; `value.0` (gated); `extern crate foo as bar`; `use a as b`; `move` closures; `#[phase(plugin)]`. |
| 1.0.0-alpha | [R] `[T; N]`; `i..j`; unboxed closures; `isize`/`usize`. |
| 1.0.0-alpha.2 | [R] `\|&:\|` obsolete; `Self` a keyword; `<T as Trait>::X`. |
| 1.0.0-beta | [R] stabilization freeze, no separate "Language" entry; needs source review. |

**1.x.** The first release of each class:

| Release | Changes |
| --- | --- |
| 1.0.0 | [A] `associated_types`, `if_let`, `while_let`, `tuple_indexing`, `macro_rules`, `globs`; [R] binary and octal digits lexed eagerly (`0b1234` is one token) |
| 1.4 | [R] `use a::{b as c}` |
| 1.8 | [A] `braced_empty_structs` |
| 1.9 | [R] Pattern_White_Space is whitespace |
| 1.11 | [R] macros in trait items; braced statement macro ending a block |
| 1.13 | [A] `?`, `type_macros`; [R] attributes on statements |
| 1.14 | [A] `..` anywhere in tuple patterns |
| 1.15 | [R] empty tuple structs; braces for tuple structs |
| 1.17 | [A] `field_init_shorthand` |
| 1.18 | [A] `pub(restricted)`; [R] `0e+10` |
| 1.19 | [A] `loop_break_value`; [R] `union`; numeric fields in struct literals |
| 1.20 | [A] `associated_consts` |
| 1.21 | [R] optional `::` before `<` in types |
| 1.22 | [R] `_` inside `\u{…}` |
| 1.25 | [A] `match_beginning_vert`, `use_nested_groups` |
| 1.26 | [A] `..=`, `'_`, `impl Trait`, slice patterns |
| 1.27 | [A] `dyn_trait`, `generic_param_attrs`; [R] `proc` unreserved |
| 1.28 | [R] `pure sizeof alignof offsetof` unreserved; [A] `lifetime` fragment |
| 1.30 | [A] `raw_identifiers`, `crate_in_paths`, `attr_literals`, `tool_attributes`, `macro_vis_matcher`, `proc_macro_path_invoc` |
| 1.31 | **2018 edition**; [A] `min_const_fn`, `pattern_parentheses`, `tool_lints`, `impl_header_lifetime_elision` |
| 1.32 | [A] `macro_at_most_once_rep` (2018), `macro_literal_matcher` |
| 1.33 | [A] `if_while_or_patterns`, `cfg_attr_multi`, `underscore_imports` |
| 1.34 | [A] `unrestricted_attribute_tokens`, `extern_crate_self` |
| 1.37 | [A] `underscore_const_names`; `?` Kleene in 2015 |
| 1.39 | [A] `async_await` (2018), `param_attrs` |
| 1.40 | [A] `macros_in_extern`; [R] proc macros in type position |
| 1.41 | [R] visibility syntactically allowed on trait items and variants |
| 1.42 | [A] `slice_patterns`; [R] syntax-only acceptances (bodyless impl items, `type T: Ord;` in impls, …) |
| 1.43 | [R] `#[cfg]` on `if`; `item` fragments in traits, impls, extern blocks; `self` syntactically in every `fn` |
| 1.45 | [R] function-like proc macros in expression, pattern and statement position |
| 1.46 | [R] `x.0.0` without parentheses |
| 1.48 | [R] `unsafe mod` syntactically |
| 1.51 | [A] `min_const_generics` |
| 1.53 | [A] `non_ascii_idents`, `or_patterns`; [R] `pat_param` |
| 1.54 | [A] `extended_key_value_attributes` |
| 1.55 | [R] `X..` patterns |
| 1.56 | **2021 edition** |
| 1.57 | [A] `macro_attributes_in_derive_output`; [R] `m!{}.f()` |
| 1.59 | [A] `destructuring_assignment`, `const_generics_defaults` |
| 1.62 | [A] `derive_default_enum`; [R] `const extern fn` |
| 1.65 | [A] `let_else`, `generic_associated_types`, `label_break_value` |
| 1.66 | [A] `half_open_range_patterns`, `arbitrary_enum_discriminant`; [R] Unicode 15 |
| 1.67 | [R] invalid literals no longer an error under `cfg(FALSE)` |
| 1.77 | [A] `c_str_literals` (2021+); [R] braced macros denied in `let`-`else` |
| 1.79 | [A] `inline_const`, `associated_type_bounds` |
| 1.80 | [A] `exclusive_range_pattern` |
| 1.82 | [A] `raw_ref_op`, `unsafe_extern_blocks`, `unsafe_attributes`, `precise_capturing`; [R] bound-modifier order after `for<>` |
| 1.83 | [R] raw lifetimes (2021+); [A] `expr_fragment_specifier_2024` |
| 1.85 | **2024 edition**; [A] `async_closure` |
| 1.87 | [R] `..EXPR` after unary operators |
| 1.88 | [A] `let_chains` (2024), `cfg_boolean_literals` |
| 1.89 | [A] `generic_arg_infer`; [E] missing fragment specifier a hard error in all editions |
| 1.93 | [A] `asm_cfg`; [R] a keyword as a `cfg` predicate is an error |
| 1.94 | [R] Unicode 17 |
| 1.95 | [A] `if_let_guard`, `cfg_select`; [R] path-segment keywords imported with renaming |
| 1.96 | [R] an `expr` metavariable allowed in `cfg` |
| 1.97 | [R] trailing `self` in imports |
| 1.99 | [A] `c_variadic`; [R] `#[my_macro] mod foo;` |
| 1.100 | [R] never type stabilized (row waits for the tag) |

Every other minor release is a **candidate semantic copy** of its
predecessor (section 6.3). Some entries that look syntactic turned out to be
semantic only: 1.16's `Self` in impl headers, 1.36's NLL and 1.75's async
functions in traits.

---

## 3. Fidelity: what a pair accepts

**Rule (decision Q11, owner).** The pair for (R, E) accepts exactly what
**stable** `rustc` R accepts with `--edition E`, in configured-in code,
without any `#![feature]` attribute. For pre-1.0 releases, which have no
editions and no stable channel, the rule is: what the tagged compiler accepts
without a feature-gate attribute. For 1.0.0-alpha and alpha.2, unstable
features produced warnings rather than errors; RS01 still excludes them (see
"Not verified"). Syntax that only a nightly feature gate enables is never in
a grammar (section 1.1).

Why "stable", and why "configured-in"? `rustc`'s *parser* accepts more than
stable `rustc` lets through, in two ways that are visible only in code that is
configured out:

```rust
#[cfg(FALSE)]
fn f() {
    let _ = try { 1 };     // accepted on stable: gating runs after cfg-stripping
}
impl X {
    #[cfg(FALSE)]
    fn g();                // accepted: 1.42 "syntax-only" acceptance
}
```

Modelling the parser instead of stable would make every grammar accept
unstable syntax that no stable program can actually use, and would make the
grammars depend on feature-gate timing. So the grammars reject both, and
each such case is a **documented divergence**, tested against the oracle in
both directions: once wrapped in `cfg(FALSE)` (where `rustc` accepts and the
grammar rejects), and once configured in (where both reject).

| Id | Divergence | Grammar | `rustc` (stable) | Since |
| --- | --- | --- | --- | --- |
| RD1 | Feature-gated syntax inside `cfg(FALSE)` code (`try {}`, `box`, `yield`, …) | rejects | accepts | gate checks moved after expansion (per feature) |
| RD2 | 1.42 syntax-only acceptances (`fn g();` and `type T: Ord;` in impls, `default` in traits, `...` as any parameter type) inside `cfg(FALSE)` | rejects | accepts | 1.42 |
| RD3 | Invalid literals inside `cfg(FALSE)` code | rejects | accepts | 1.67 |
| RD4 | A nested `/*` inside a doc block comment (`/** a /* b */ c */`) | refuses (typed lexer error), until X1 | accepts | interim (section 4.2) |
| RD5 | `.rc` crate files, 0.1–0.4 | refuses, until X4 adds a second start rule | accepts | 0.1–0.4 |

A consumer that hits a divergence gets a typed refusal, never a different
parse. Inside macro token trees the question does not arise: a token tree
accepts any balanced tokens, so `quote! { try { 1 } }` lexes and parses
everywhere.

Identifier equality is **not** a grammar question. `rustc` compares
identifiers after NFC normalization. A lexer cannot normalize, so consumers
decode identifier values with the repository's `unicode-normalize` crate
(LANG84 C1.4 allows decoding a single token's value).

---

## 4. Lexical design

### 4.1 Engine facts that shape the design

These come from the `grammar-tools`, `lexer` and `parser` sources on
`origin/main`, and from probe experiments run against those crates:

- Token patterns compile as `^(?:pat)` with the `regex` crate: no
  backreferences, no lookaround, leftmost-first alternation.
- `skip:` patterns are tried before definitions, and they are global, even
  inside F04 groups.
- A token type whose name ends in `STRING` has its quotes stripped and its
  escapes decoded. Rust literals are therefore named `STR_LIT`, `RAW_STR_LIT`,
  `BYTE_STR_LIT` and so on, and stay raw for the consumer to decode.
- A grammar literal (`"fn"`) matches by token value and never matches a
  `RAW_IDENT` whose value is `r#fn`, which is exactly Rust's rule.
- The start symbol is the first rule. There is no parse-from-rule API yet
  (extension X4).
- The parser is packrat with F11 adjacency predicates (`@no_space_before`)
  and `@asi`; the lexer has E1 guards (`@input_start`, `@line_start`) and E8
  refusal transitions. LANG84's E2–E5 and E7 are specified but not
  implemented, so RS01 does not use them.

### 4.2 The hard cases

Every case below has a probe fixture in slice A4 and a positive and negative
corpus case in `rs-syntax`.

| # | Case | How it is expressed | Releases |
| --- | --- | --- | --- |
| 1 | **Nested block comments** `/* a /* b */ c */` | Not expressible by a regex (it cannot count). **Interim:** an F10 group tracks depth (`on BC_OPEN -> push bc`, `on BC_CLOSE -> pop`), and the C3 post-tokenize hook `rust_block_comments` removes the comment's tokens (section 4.3). **Later:** X1. | 0.1–0.3, 0.9 onward; 0.4–0.8 use a plain non-nesting skip regex |
| 2 | **Raw strings** `r#"…"#`, `br##"…"##`, `cr#"…"#` | One generated regex with an arm per hash count, 0–255 (about 68 KB). `r#"a"##` lexes as a raw string plus `#`, as `rustc_lexer` does; `r#ident` falls through to `RAW_IDENT`. Later, optionally, X2. | 0.9 onward; `cr` in 2021+ from 1.77 |
| 3 | **Lifetime or char** `'a`, `'a'`, `'\''` | `CHAR_LIT` ordered before `LIFETIME`. Raw lifetimes `'r#a` in 2021+ files from 1.83; the 2021 reserved `'ident#` is an `errors:` entry. | `LIFETIME` from 0.6 |
| 4 | **`>>`, `>=`, `>>=` inside generics** | Every punctuation character is its own token, and the grammar composes operators with F11: `shr = GT @no_space_before GT !(@no_space_before EQ)`. This is exactly `proc_macro`'s Joint/Alone spacing. | all |
| 5 | **Turbofish** `f::<T>()` | Expression paths require `COLON @no_space_before COLON LT`; type paths do not (`::` optional before `<` in types from 1.21). | all |
| 6 | **Struct literal or block** in `if`/`while`/`match`/`for` heads | A duplicated "no struct literal" expression ladder (`expr_ns`). Statement-level block-like expressions (`{ 1 } - 1` is two statements; `match x {..} + 1;` is an error) are PEG statement rules with lookaheads. Later, optionally, X6. | restriction from 0.11.0 |
| 7 | **Shebang** | 0.3–0.9: `/#![^\n]*/ @input_start`. 1.0: `#!` not immediately followed by `[` (an `@input_start` regex). **Current rule:** `#!` is a shebang unless the next token after whitespace and *non-doc* comments is `[`. A regex cannot skip nested comments, so the C3 pre-tokenize hook `rust_strip_shebang` handles it (section 4.3). **Later:** X3. | current rule from a release not yet pinned (see "Not verified") |
| 8 | **XID identifiers** | Explicit code-point classes per Unicode version, generated from that version's `DerivedCoreProperties.txt` (X7), never `\p{XID_*}`, whose tables drift with the `regex-syntax` crate. Non-ASCII identifiers are a refused token in stable pairs 0.9–1.52 (gated) and `NAME` from 1.53. | XID from 0.1 |
| 9 | **Raw identifiers** `r#async` | `RAW_IDENT` before `NAME`; `keywords:` reclassifies only `NAME`, so `r#async` stays an identifier; `r#crate`, `r#self`, `r#super`, `r#Self`, `r#_` are `errors:` entries. | 1.30 onward |
| 10 | **Suffixes and number forms** `1f32`, `0x1F_u8`, `1.0e-3_f64` | `INT_LIT`/`FLOAT_LIT` with an optional XID suffix. A trailing-dot float `1.` is **not** a token (no lookahead), so `1.foo()` and `0..1` lex correctly and the grammar composes `INT_LIT @no_space_before DOT`. `1e` is an `errors:` entry; `0b102` is one token whose digits the decoder validates (1.0). | all, per era |
| 10b | **Tuple index** `x.0.1` | An F10 mode after `DOT` lexes digits as `INT_LIT`, giving `INT 0 DOT INT 1`, which is `rustc`'s post-split view. | 1.46 onward; earlier pairs omit the mode |
| 11 | **Doc comments** `///`, `//!`, `/** */`, `/*! */` | Line forms are tokens (`OUTER_DOC`, `INNER_DOC`); `////` is a plain comment. Block forms ride on case 1. The grammar accepts a doc comment wherever an attribute of the same kind is allowed. | 0.3 onward |
| 12 | **Edition keywords, reserved prefixes, guarded strings** | Per-edition `.tokens` files: `keywords:` and `soft_keywords:` (`union`, `macro_rules`, `safe`, `raw`, `auto`, `default`, 2015 `dyn`). 2021 prefixes and 2024 `#"`/`##` are `errors:` regexes ordered after the raw, byte and C-string forms. **`reserved:` is never used**: it raises a lexer error, but every keyword is valid inside a token tree (`quote! { abstract }`). | per edition |
| 13 | **Pre-1.0 tokens** | `#`, `#{`, `#<`, `<-`, `<->`, `>>>`, `@` and `~` sigils are ordinary tokens; `#[a];` is grammar; integer suffixes `u`, `i`, `u8` are regex. | 0.1–0.12.0 |

Three more input rules, none of which needs a hook:

- **CRLF.** `rustc` turns `\r\n` into `\n`, and a bare CR inside a literal is
  an error. Literal regexes reject a lone CR (`(?:[^"\\\r]|\r\n|\\…)`), and
  decoders normalize CRLF.
- **BOM.** A leading U+FEFF is an `@input_start` skip entry. The shebang hook
  looks past it.
- **Frontmatter** (`---` blocks) is unstable at 1.99 and excluded, as the
  fidelity rule requires.

### 4.3 The two Rust C3 hooks (interim)

LANG84 C3 allows small, registered pre- and post-tokenize hooks. RS01
registers two (LANG84 amendment, C3 table). Both are deleted when phase D's
extensions land.

**`rust_block_comments` (post-tokenize, nesting releases only).** The lexer
already knows where comments start, because it knows where strings, raw
strings and char literals end: `"/*"` inside a string literal is consumed by
`STR_LIT` and never opens a comment. So the comment's *depth* is tracked
declaratively, and the hook only removes what the lexer emitted:

```text
INNER_BC_OPEN = "/*!"              # inner doc block, even `/*!*/`
OUTER_BC_OPEN = /\/\*\*[^*\/]/     # `/**x`: consumes one content char (there is
                                   # no lookahead), so `/**/` and `/***` stay plain
BC_OPEN       = "/*"               # after the doc openers; before SLASH and STAR
group bc:                          # inside a plain block comment
  BC_OPEN2 = "/*" -> BC_OPEN
  BC_CLOSE = "*/"
  BC_TEXT  = /[^*\/]+|\*|\//
group doc_bc:                      # inside a doc block comment
  BC_CLOSE2 = "*/" -> BC_CLOSE
  BC_TEXT2  = /[^*\/]+|\*|\// -> BC_TEXT
transitions:
  on BC_OPEN                         -> push bc
  on (INNER_BC_OPEN | OUTER_BC_OPEN) -> push doc_bc
  on BC_CLOSE                        -> pop
  # a "/*" inside doc_bc is refused by an E8 refusal (divergence RD4)
```

The hook makes one pass over the token list:

1. It deletes each run `BC_OPEN … BC_CLOSE` of a plain comment.
2. It keeps a doc block comment's tokens (`OUTER_BC_OPEN BC_TEXT… BC_CLOSE`,
   or the inner form) for the grammar, which accepts that run as an
   attribute of the same kind.
3. It deletes plain line comments. In nesting releases these are tokens
   (`LINE_COMMENT`), not `skip:` entries, because a global skip would also
   fire *inside* a block comment, where `/* // */` must still close at `*/`.
4. It sets the F11 "space before" flag on the token that follows a deleted
   run, so `>/**/>` is two separate `>` (Alone), as `rustc` reads it.

It never recognizes any other token and never reads source text. It is
linear and allocates one output list. Size target: under 80 lines.

**`rust_strip_shebang` (pre-tokenize, releases with the current rule).**
Input starts at offset 0, or after a BOM:

1. If the input does not start with `#!`, return it unchanged.
2. Scan forward over whitespace, non-doc line comments and non-doc block
   comments (counting nesting, with the depth budget of section 8). Strings
   cannot occur in this stretch: the first other character ends the scan.
3. If that character is `[`, it is an inner attribute (`#![allow(x)]`):
   return the input unchanged.
4. Otherwise replace the first line, up to but not including its newline,
   with spaces. Offsets, lines and columns of everything after it are
   unchanged.

Size target: under 100 lines. The two hooks together stay well inside
LANG84's 400-line limit.

These hooks also close two capability-analyzer findings by construction:
VER08a D25 (a shebang line `#!/x"` that opened a string and hid code) and,
through per-edition token files, D27 (`c"`/`cr#"` lexed as C strings in every
edition).

### 4.4 grammar-tools extensions (phase D)

Any change to `grammar-tools`, `lexer` or `parser` puts about 5,300 of 5,337
packages in the affected set and needs the `ci-full-build` label. So these
are batched, after milestone 1, in two PRs (decision Q14). Each follows
LANG84's extension rules: Rust first, an F-spec, byte-identical behaviour for
grammars that do not use it, port rejections in Python, Ruby, TypeScript, Go
and Elixir, and every new `GrammarElement` variant added to exhaustive
matches downstream.

| Id | Extension | Replaces or enables | PR |
| --- | --- | --- | --- |
| X1 | Nested delimited scanner with classification, usable in `skip:`: `nested("/*", "*/") classify { /^\/\*\*[^*\/]/ -> OUTER_BLOCK_DOC, /^\/\*!/ -> INNER_BLOCK_DOC, else -> skip }`, with a depth budget | deletes `rust_block_comments`; lifts RD4 | D1 (lexer side) |
| X2 | Counted raw delimiter `raw(prefix: /(?:b\|c)?r/, fence: "#", quote: "\"", max: 255)` | optional; replaces the 68 KB generated regex | D1 |
| X3 | Shebang guard: `@input_start` plus "the next significant token is not `LBRACKET`", built on X1's skipper | deletes `rust_strip_shebang` | D1 |
| X4 | `GrammarParser::parse_rule(name)` and `parse_rule_prefix(name) -> (node, end)` | macro fragments; re-parsing expansions; `.rc` files (lifts RD5) | D2 (parser side) |
| X5 | Declared synthetic token types (`synthetic:`) that the lexer can never produce | invisible delimiters for macro interpolation | D2 |
| X6 | Rule restriction flags (`expr<no_struct>`) | optional; removes the duplicated ladder of case 6 | D2 |
| X7 | Per-Unicode-version XID classes | not an engine change: a generator script and data | A4 |

X5 has a security reason. If the expander's invisible delimiters were
ordinary tokens with an "impossible" pattern, such as a private-use code
point, source text could forge an interpolated fragment. A declared synthetic
type has no pattern at all.

The Reference grammar uses a hard cut `^` that grammar-tools lacks. A
translated rule rewrites each cut as a `!`-lookahead plus ordering, and
records any acceptance difference as a divergence.

---

## 5. Macros: Rust's "preprocessor"

### 5.1 Why the C and C# approaches do not fit

| Approach | What it does | Why not for Rust |
| --- | --- | --- |
| CS01 | Treats `#if` and the other directives as skip tokens; conditional compilation is not modelled. | Rust `cfg` is an *attribute* on syntax the grammar parses anyway. Skipping it would change which items exist. |
| F05 / PREP01 | A text pass before tokenizing: collect definitions, evaluate conditionals, splice text. | Rust macros take token trees, produce token trees that are re-parsed by fragment kind, are hygienic and name-resolved, and run interleaved with `cfg` stripping. A text pre-pass is wrong by construction. |
| LANG84 "Preprocessor dialects" | A separate language layer whose output feeds the base grammar unchanged, which emits no IIR, and which an identity test ties to the hand-expanded program. | The closest fit, and the one RS01 uses, through an amendment (decision Q6): the `macro_rules!` matcher is a token-tree interpreter, which LANG84 C2 could otherwise read as a hand-written parser. |

### 5.2 Layers

```text
source ─► GrammarLexer(rust<rel>-e<ed>.tokens) [+ C3 hooks until X1/X3]
       ─► GrammarParser(rust<rel>-e<ed>.grammar)
            CST: every macro call, attribute and macro_rules! body is token trees;
                 cfg/cfg_attr predicates and macro_rules! matchers and
                 transcribers are parsed structurally
       ─► [L1] cfg evaluator: pure CST walk + CfgSet ─► configured view (side table)
       ─► [L2] invocation collector + macro resolution
       ─► [L3] expanders: macro_rules! (matcher + transcriber), builtins per
                release, builtin derives (optional), Unexpanded for the rest
       ─► re-parse each output with X4 parse_rule(fragment kind) ─► repeat L1–L3
          until no invocation is left or a budget is hit
       ─► ExpandedCrate { CST fragments, provenance, hygiene side table, Unexpanded[] }
```

Nothing below L1 changes a CST. Every layer's output is a side table keyed by
CST node identity, as LANG84 C1.7 allows, plus new CST fragments that came out
of the **same** release grammar.

### 5.3 Token trees in the grammar

One grammar shape serves every macro invocation, in every position:

```text
macro_call = simple_path BANG !(@no_space_before EQ) delim_tt ;
delim_tt   = LPAREN { tt } RPAREN | LBRACKET { tt } RBRACKET | LBRACE { tt } RBRACE ;
tt         = delim_tt | any_non_delimiter_token ;
```

grammar-tools has no wildcard terminal, so `any_non_delimiter_token` is an
alternative over every token type in the release's `.tokens` file except the
six delimiters. The generator writes it, so it can never fall out of date.

Positions, each with its own entry release: item and statement (a `()`/`[]`
call needs `;`), expression (with block-like rules for `{}`; `m!{}.f()`
continues from 1.57), pattern, type (1.13), trait, impl and extern items
(1.11, 1.40, 1.43). Releases 0.4–0.12 use the same shape. 0.1–0.3 use the
expression form of section 5.9.

**Attributes** follow the same idea:

```text
outer_attr = POUND LBRACKET attr RBRACKET | OUTER_DOC | doc_block_outer ;
inner_attr = POUND BANG LBRACKET attr RBRACKET | INNER_DOC | doc_block_inner ;
attr       = cfg_attr_form | cfg_form | [ "unsafe" LPAREN ] path attr_input [ RPAREN ] ;
attr_input = delim_tt | EQ expr | /* empty */ ;
```

`unsafe(…)` arrives in 1.82 (and is required for three attributes in 2024);
`EQ expr` arrives in 1.54 (literals only from 1.30, arbitrary token streams
from 1.34). Before 0.10, an inner attribute is `#[meta];`, decided by the
trailing `;`. `rustc` allows `# ! [`, so `inner_attr` has no adjacency
requirement (to be confirmed per era; see "Not verified").

### 5.4 `cfg` and `cfg_attr`, and the evaluator L1

The grammar parses `cfg` **structurally**, so a malformed predicate is a
parse error rather than a silent token tree:

```text
cfg_form      = "cfg" LPAREN cfg_pred RPAREN ;
cfg_attr_form = "cfg_attr" LPAREN cfg_pred COMMA attr { COMMA attr } [ COMMA ] RPAREN ;
cfg_pred      = "all" LPAREN [ cfg_pred { COMMA cfg_pred } [ COMMA ] ] RPAREN
              | "any" LPAREN [ cfg_pred { COMMA cfg_pred } [ COMMA ] ] RPAREN
              | "not" LPAREN cfg_pred RPAREN
              | "true" | "false"                       # 1.88 onward
              | NAME [ EQ STR_LIT ] ;
```

The generic `attr` alternative carries `!("cfg" | "cfg_attr")`, so the
structural forms cannot be bypassed. `cfg_attr` takes several attributes from
1.33 and nests from 1.11.

**L1, the evaluator,** is a pure function: CST plus a caller-supplied
`CfgSet` (`target_os`, `feature = "…"`, `test`, `debug_assertions`, `panic`
from 1.60, `target_has_atomic`, …) gives a side table of stripped nodes. Its
truth table is the obvious one:

| Predicate | True when |
| --- | --- |
| `all(p…)` | every `p` is true (`all()` is true) |
| `any(p…)` | some `p` is true (`any()` is false) |
| `not(p)` | `p` is false |
| `true` / `false` | always / never (1.88) |
| `key` | `key` is set in the `CfgSet` |
| `key = "v"` | the pair (`key`, `v`) is in the `CfgSet` |

It strips items, statements, expressions (`#[cfg]` on `if` from 1.43),
fields, variants, generic parameters, function parameters (1.39) and match
arms. `rustc` configures before **and during** expansion, so L1 runs again on
every expansion output. `cfg!(…)` is a builtin that becomes a boolean
literal; `cfg_select!` (1.95) is a builtin; a `$e:expr` metavariable in
`cfg` (1.96) matters only during expansion. Pre-1.0: `#[cfg(target_os =
"linux")]` from 0.1, `cfg!` from 0.8.

### 5.5 `macro_rules!` definitions

Definitions are parsed structurally, not as token trees:

```text
macro_rules_def = "macro_rules" BANG NAME rules_body ;
macro_rule      = matcher_group fat_arrow transcriber_group ;
fat_arrow       = EQ @no_space_before GT ;
matcher         = DOLLAR @no_space_before NAME COLON NAME          # $e:expr
                | DOLLAR @no_space_before LPAREN { matcher } RPAREN [ sep ] rep_op
                | matcher_group | plain_token ;
transcriber     = DOLLAR @no_space_before NAME                     # $e, $crate
                | DOLLAR @no_space_before LPAREN { transcriber } RPAREN [ sep ] rep_op
                | transcriber_group | plain_token ;
```

A probe parsed `macro_rules! m { ($e:expr, $($x:ident),* $(,)?) => { $e +
$($x)+* }; }` with this shape. It also exposed a precision trap worth
knowing: the separator can never be `*`, `+` or `?`, so in `$($x)+*` the `+`
is the operator and `*` is a literal token.

Fragment specifiers are per era: 0.4 includes `matchers`; `lifetime` from
1.28, `vis` from 1.30, `literal` from 1.32, `pat_param` from 1.53,
`expr_2021` from 1.83. The `?` operator is 1.32 (2018) and 1.37 (2015).

A separate pure **definition checker** applies what the grammar cannot: the
follow-set rules (1.11), fragment-name validity, and the missing-specifier
error (2024 edition; all editions from 1.89).

### 5.6 The expansion layer (L2, L3)

Expansion is a separate phase, but every fragment it parses goes through the
release grammar.

- **Matching.** `rustc`'s matcher is an NFA over token trees. It hands each
  `$x:frag` to the real parser and commits to what it consumed; it never
  backtracks into a fragment, and it reports "local ambiguity" when two
  positions both want a fragment. RS01's matcher does the same, calling
  `parse_rule_prefix(frag_rule)` (X4). `ident`, `lifetime` and `tt` match a
  single token or tree. The fragment-to-rule map is release-profile data
  (2021 `pat` is `pat_with_or`; 2024 `expr` includes `const {}` and `_`).
- **Transcription** substitutes captures. An `expr`, `ty`, `pat`, `stmt`,
  `item`, `block`, `path`, `meta`, `vis` or `literal` capture is wrapped in
  synthetic invisible delimiters (X5), so `$e * 2` keeps `$e`'s grouping when
  `$e` is `1 + 1`. `ident`, `lifetime` and `tt` are transparent.
- **Hygiene** is release-profile data: none in 0.1–0.7; `let`-hygienic in
  0.8; labels in 0.10; metavariables in 1.12; 1.x `macro_rules!` is
  mixed-site (locals, labels and `$crate` resolve at the definition, the rest
  at the call). Transcribed tokens carry a `SyntaxContext` in a side table. A
  consumer that cannot resolve hygienically must refuse any expansion that
  binds, or refers to, a local or label across the macro boundary.
- **Scoping.** Textual `macro_rules!` scope (order-dependent; `#[macro_use]`;
  `#[macro_export]`; `$crate`) and path-based resolution from 1.30. A local
  `macro_rules! assert` shadows `std`'s, so every allowlist checks the
  **resolved** macro, never its spelling.
- **Budgets** (section 8): expansion depth, total expanded tokens, invocation
  count and matcher work per step, because an NFA with commit can blow up on a
  hostile matcher.

**Classification.** The expander is LANG84's amended "preprocessor dialect"
(LANG84 amendment): its outputs are fed back to the unchanged release
`GrammarParser`; it emits no IIR; it parses no fragment itself (every
fragment goes through `parse_rule_prefix`); its interpreter walks token
trees, not characters; and an identity test ties expansions to the oracle
(`rustc -Zunpretty=expanded`, run only in oracle jobs, decision Q10).

### 5.7 Builtin macros per release

Builtins are compiler-defined, so each release profile lists its own:

| Era | Builtins |
| --- | --- |
| 0.1 | `#fmt #env #concat_idents #ident_to_str #log_syntax` |
| 0.3 | adds `#line #col #file #mod #stringify #include #include_str #include_bin` |
| 0.4–0.5 | `#fmt` → `fmt!`; 0.5 adds quasi-quoters |
| 0.6 | `fail! assert! debug! …` become macros |
| 0.8 | adds `format!`, `cfg!`, `unreachable!` |
| 0.9 | removes `fmt!`; adds `concat!` |
| 1.x | `format_args`, `concat`, `stringify`, `line`, `column`, `file`, `module_path`, `cfg!`, `cfg_select!` (1.95), `compile_error!`, `panic!` (2015 and 2021 forms), `include*`, `env!` |

Each builtin is either modelled from its source at the tag or is opaque.
`include*!` and `env!` are opaque unless the caller's `CfgSet`-like input
supplies their value. Library macros (`println!`, `vec!`, `assert_eq!`) are
`macro_rules!` in `std`; their definitions are extracted per release as data
files (decision Q9), so expanding them uses the same matcher as user macros.

### 5.8 What cannot be expanded: `Unexpanded`

Custom derives (1.15), attribute and function-like procedural macros (1.30,
1.40, 1.45), pre-1.0 compiler plugins (`#[phase(plugin)]`, 0.11–0.12),
`proto!` (0.4–0.7) and `include!`/`include_str!`/`include_bytes!` need code
execution or file reads. Each becomes an opaque node:

```text
Unexpanded { kind: Derive | Attribute | FunctionLike | Plugin | Include,
             path, span, input_tokens }
```

**Consumers fail closed on it:**

| `kind` | What stays usable |
| --- | --- |
| a derive | the item itself; any use that could resolve to a derived impl is refused |
| an attribute macro | nothing: the whole item is unknown and refused |
| a function-like macro | nothing at that position |
| `include*` | nothing (matches VER-BACKLOG WD-010: reject `include!`/`#[path]`/`build.rs`) |

Builtin derives (`Clone Copy Debug Default Eq Hash Ord PartialEq PartialOrd`;
pre-1.0 `#[deriving(…)]` from 0.6 and `#[deriving_eq]` in 0.5) add impls and
leave the item intact. Modelling their expansion is optional (slice E5);
until then they are opaque derives.

### 5.9 Pre-1.0 syntax extensions

0.1–0.3 syntax extensions are not token trees. The 0.1 parser
(`parse_syntax_ext_naked`) parses their arguments as **expressions**:

```text
syntax_ext = POUND NAME ( LPAREN [ expr { COMMA expr } ] RPAREN
                        | LBRACKET [ expr { COMMA expr } ] RBRACKET )
           | POUND_LT ty GT                       # #<ty>
           | POUND_LBRACE block RBRACE ;          # #{block}
```

So the CST holds `syntax_ext { name, args: [expr…] }`, and no token-tree
machinery is involved. `#macro([pattern, template], …)` is macro-by-example
over expression vectors with `...` repetition, unhygienic and expanded
outside-in; it gets a small AST-level expander ("ast-mbe"), separate from the
0.4+ token-tree matcher, as an optional sub-slice of G1. 0.4 probably accepts
both `#fmt` and `fmt!` (0.5's notes say it completed the move); G5 confirms
it from 0.4's `parser.rs`.

### 5.10 Who needs expansion

| Consumer | Needs | Policy |
| --- | --- | --- |
| Warrant WIR proof front end | expanded code for every reachable construct | fail closed; allow only listed **resolved** `std` macros with modelled meaning (`assert!`, `assert_eq!`, `debug_assert*!`, optional literal message); local `macro_rules!` only after phase E; `cfg` evaluated with a fixed, recorded `CfgSet` (`test = false`, …) |
| Capability analyzer | tokens today; already fails closed (CAP003) on `include!`, `#[path]`, proc macros | lexer first (C4); `macro_rules!` expansion later removes one CAP003 class (E6); proc macros stay CAP003 |
| A future Rust IIR front end | full expansion, including `println!` → `format_args!` | out of scope for RS01 |
| Formatter, LSP, highlighting | the CST with token trees | no expansion |

---

## 6. Conformance

### 6.1 Stages

RS01 uses LANG84's stages unchanged, per (release, edition) pair, with the
uniform-claim rule of section 2.4:

- **G:** `grammar-tools validate` and cross-validation pass, and the header
  cites `source_ref`, `source_sha` and the syntax blobs.
- **P:** `release_probes` in `rustlang-parser`: positive and negative probes
  per pair through the compiled tables, plus child-process hostile probes
  (deep nesting, long operator spines, unterminated comments, raw strings and
  token trees, a 255-hash raw string, maximum comment depth).
- `iir_status` and `backend_status` stay `-`: there is no Rust IIR compiler.
  The proof front end's progress is recorded as `grammar_subset=rs-confined`
  on a `partial` row, and proven by the VER25a differential (section 7.1).

### 6.2 Corpora

All under `code/grammars/rust/corpus/`:

| Corpus | Kind | Contents |
| --- | --- | --- |
| `rs-token-trees/` | `subsets.csv` subset | probes for the lexical pairs of A4, whose grammar is token trees plus delimiter balance only |
| `rs-confined/` | `subsets.csv` subset | the proof subset: `<name>.rs`, `<name>.expect` (stdout of a harness run under the oracle `rustc`), `<name>.reject` (`Lex`, `Parse` or `Unsupported`), `<name>.cite`. Every `.reject` must be a program `rustc` **accepts** (lesson `a-language-corpus-reject-case-must-be-a-program-the-real-language`); programs `rustc` refuses belong in parser tests |
| `rs-syntax/` | parse-only conformance corpus, **not** a `subsets.csv` subset (it has no `.expect`) | `<name>.rs` + `<name>.cite`, plus `<name>.reject` (`Lex` or `Parse`) for a case rejected everywhere. A case accepted only by some pairs carries `<name>.gate` instead: `first` and `last` release IDs (resolved through `predecessor` lineage, never by version comparison), `editions`, and `outside` (`Lex` or `Parse`), the category every other pair must reject it with. One positive and one negative case per class boundary of section 2.7, every hard case of section 4.2, every divergence of section 3, and edition pairs such as `async fn` (gate `first=1.39.0 editions=2018;2021;2024 outside=Parse`) and `k#x` (a 2021 lexer error). Checked by `release_probes` |
| repository self-corpus | positive only | every `.rs` under `code/` (about 1,480 crates; 1,476 declare 2021, 4 declare 2024) must lex and parse under the newest release's matching pair. It walks the whole tree, so it needs a `repository_inputs.go` consumer row |
| `historical.csv` | manifest | pre-1.0 upstream tests (section 6.5) |

In the two `subsets.csv` corpora, LANG84's per-case overrides apply, with one
addition: the override key may name a pair (`.reject@1.85.0-e2021`), not
only a release or profile. The inventory checker learns that form in slice
A2 (LANG84 amendment).

### 6.3 `pair-evidence.csv` and the three bases

One row per (release, edition) pair, following Python's layout:

| Column | Meaning |
| --- | --- |
| `release`, `edition`, `commit` | the pair and the tag's peeled commit (`edition` is `-` before 1.0) |
| syntax-file blob columns | pre-1.0: `src/comp/syntax/parse/{lexer,token,parser}.rs` at 0.1, `src/rustc/syntax/parse/*` at 0.2, `src/libsyntax/parse/*` from 0.3; 1.0: `src/libsyntax/parse/lexer/mod.rs`, `token.rs`; modern: `compiler/rustc_lexer/src/lib.rs`, `compiler/rustc_parse/src/{lexer,parser}/` (tree IDs), `compiler/rustc_span/src/symbol.rs`, `compiler/rustc_feature/src/accepted.rs`, `compiler/rustc_expand/src/mbe/`, and the XID crate version from `Cargo.lock` |
| `basis` | `reviewed`, `copy:<pair>` or `semantic-copy:<pair>` |
| `oracle` | the oracle record that backs the pair (section 6.4/6.5) |
| `note` | what changed, or why nothing did |

Slice A3 fixes the exact, era-neutral column layout (section 10, A3 decisions).

Blob and tree IDs come from a blobless, depth-1 fetch per tag.

**The bases:**

- **`reviewed`**: the pair was written or changed by hand from the tag's
  sources, as a reviewed delta from its predecessor.
- **`copy:<pair>`** (LANG84): every syntax blob equals the predecessor's. This
  covers all 1.x **patch** releases (1.99.0 and 1.99.1 have identical lexer,
  parser, symbol and feature blobs, checked) and 0.3.1.
- **`semantic-copy:<pair>`** (RS01's LANG84 amendment, decision Q3, owner).
  `rustc_parse` changes in **every** 1.x minor, even when no syntax changes:
  1.98.0 to 1.99.0 changed `expr.rs` and `item.rs` while `rustc_lexer` stayed
  identical. So the blob rule almost never applies to a 1.x minor's parser.
  A minor may instead copy its predecessor's pair when **all three** of these
  hold:
  1. `accepted.rs` at the release has **no** feature whose `since` is this
     release and which changes syntax;
  2. `RELEASES.md` has **no** grammar-visible "Language" entry for this
     release;
  3. the per-release **oracle sweep passes**: the predecessor's pair, applied
     to this release's `rustc`, agrees on the whole `rs-syntax` corpus for
     that edition.

  Each `semantic-copy` row records the evidence for all three conditions in
  `pair-evidence.csv`: the `accepted.rs` blob and the features checked, the
  `RELEASES.md` blob and section, and the sweep's oracle record (toolchain
  SHA-256 and result digest). If any one is missing the pair needs a
  `reviewed` delta instead.

### 6.4 The executable oracle: 1.x and 0.10–0.12

`static.rust-lang.org/dist` serves `rustc` for every 1.x release (probed:
1.0.0-alpha, 1.0.0, 1.30.0 and 1.85.0; about 70–100 MB each) and Linux
binaries for 0.10, 0.11.0 and 0.12.0.

**Pinning.** Each toolchain archive used by any oracle job is pinned by
SHA-256 in `code/grammars/rust/oracle-toolchains.csv` (release, target
triple, archive URL, SHA-256), written once by slice C2 and checked against
the upstream `.sha256` file at pin time. The job refuses an archive whose
digest differs. The jobs run network-sandboxed after download, in a
**scheduled (daily) CI job** that never blocks a PR, as LANG84's optional
historical-interpreter containers do. ("Daily" names the schedule; it has
nothing to do with nightly Rust toolchains, which RS01 never uses.)

**Technique 1: `cfg(FALSE)` acceptance.** Wrap the case and compile metadata
only:

```text
#[cfg(FALSE)] mod m { <case> }
rustc --edition <E> --emit=metadata --crate-type lib case.rs
```

`cfg(FALSE)` code is parsed but never type-checked, so the result measures
syntax only. Checked on local `rustc` 1.97:

| Case | Result |
| --- | --- |
| `let x = 1 + ;` | rejected in all editions |
| `undefined::<u8>(); let y: Nope = 3;` | accepted (no semantic checks) |
| `impl X { fn g(); type T: Ord; }` | accepted (1.42 syntax-only, divergence RD2) |
| `1else` | rejected ("expected at least one digit in exponent") |
| `let _ = try { 1 };` | **accepted** on stable 2021/2024 (divergence RD1) |

The last row is why every divergence case is checked twice (section 3), and
why corpus cases avoid gated constructs except as recorded divergences.

**Technique 2: the token dumper.** A 20-line function-like `proc_macro` prints
each token tree with its `Punct` spacing:

```rust
#[proc_macro]
pub fn dump(input: TokenStream) -> TokenStream {
    // Group, Punct (char + Joint/Alone), Ident, Literal, one per line, nested
    ...
}
```

`a >>= b` dumps as `PUNCT > Joint`, `PUNCT > Joint`, `PUNCT = Alone`, which
maps one to one onto single-character punctuation plus F11 flags. It works
from 1.30 (function-like proc macros); derive input works from 1.15; 1.0–1.29
use a `macro_rules!` `tt`-counting check instead.

**Technique 3: precedence and shape.** `rustc` has no stable CST dump, so
shape is checked by behaviour (`assert_eq!(1 + 2 * 3, 7)` compiled and run).
`-Zunpretty=expanded` with `RUSTC_BOOTSTRAP=1` is allowed **only in oracle
jobs**, never in a build or test that ships (decision Q10). It is used only
to get *diagnostic output* (expanded source, crate-root-only parses) from a
pinned **stable** binary. It never widens what counts as accepted: an
acceptance verdict always comes from the same binary without
`RUSTC_BOOTSTRAP`, and no `#![feature]` attribute ever appears in a corpus
case.

**Secondary sources** (useful for translation, never authoritative):
`src/grammar/RustLexer.g4`, `verify.rs` and `parser-lalr.y` (present at
1.0.0, 1.15.0, 1.25.0, 1.30.0), and the Reference's grammar blocks at each
tag's pinned `src/doc/reference` commit.

**In normal CI.** A host-oracle test runs only when `rustc --version` equals
a release that has a pair; otherwise it prints a visible skip, because CI's
`stable` moves every six weeks. Full sweeps run in the scheduled (daily)
CI job.

### 6.5 The source-and-test-suite oracle: 0.1–0.9

No binary exists for 0.1–0.9 (the 0.8 and 0.9 URLs return 404), and building
them needs era LLVM, GCC and stage-0 snapshots, which RS01 does not attempt.
The oracle for these releases is:

1. the pinned `lexer.rs`, `token.rs` and `parser.rs` at the tag (blob IDs in
   `pair-evidence.csv`), read as the primary source; the manuals are
   secondary and are wrong in places (the 0.1 manual lists `int`, `uint` and
   `vec` as keywords; the 0.1 parser does not reserve them);
2. that tag's own tests: `src/test/run-pass` as positive cases (0.1: 573
   files; 0.4: 945; 0.7: 1,236), and the `compile-fail` cases whose
   `error-pattern` is a parse error as negative cases (0.1 has 268
   compile-fail files before filtering), plus `pretty`.

These are not vendored. `corpus/historical.csv` lists (tag, path, blob ID,
expected outcome), and the scheduled (daily) CI job fetches and checks them
(decision Q8).
0.10–0.12 use both this oracle and, where their binaries run, technique 1.

### 6.6 Honest gaps

- No executable oracle exists for 0.1–0.9, and perhaps none for 0.10–0.12.
- No stable CST oracle exists for any release.
- The `cfg(FALSE)` technique over-accepts gated syntax (handled by the
  double check of section 3).

---

## 7. Consumers

### 7.1 `rs-confined`: the Warrant WIR proof front end (LP-001a)

**Today.** `warrant-ir/src/rust_subset/{lexer,parser}.rs` (about 18.6 KB and
38.9 KB) is a hand-written Pratt-style parser (VER25a). Its macro arguments
are comma- or semicolon-separated *expressions*, not token trees, and its
attributes allow only identifiers and strings.

**The subset.** `subsets.csv` gets:

```text
subset,description,corpus_dir,spec,allowlist
rs-confined,VER25a confined Rust subset (proof front end),corpus/rs-confined,VER25a,rs-confined.allowlist
```

The `allowlist` column is new (decision Q4). It names a file in
`code/grammars/rust/` itself, never inside `corpus/<subset>/`, where LANG84's
checker would read it as a corpus case; `-` means no gate. Membership is the **release
grammar plus a subset gate**: a checked-in allowlist of CST rule names, token
types and *resolved* macros. A generic CST walker enforces it and returns
`Unsupported { rule, span }` for anything else. The grammar is not cut down
per subset; the gate is. The pairs are those of `DEFAULT_RELEASE` (1.97.0) in
all four editions (section 2.4), and later the newest release; gf256 and
ct-compare are 2021 crates.

Budgets: VER25a caps nesting at 32 and tokens at 65,536. Rule depth is not
source depth, so the gate maps them onto `GrammarParser::with_max_depth` plus
its own source-nesting counter.

**The differential, before the hand-written parser goes.** For every corpus
case, every `warrant-ir` test input and the real `ct-compare/src/lib.rs`:

| Old parser | New path must |
| --- | --- |
| accepts | accept, and lower to **byte-identical** WIR |
| rejects | reject with the mapped error category, unless the case is in an explicit "newly in scope" list reviewed in the PR |

plus mutation fuzzing (token deletion, insertion and swaps) over the corpus
under the same parity check. Doc comments, which the old lexer discards as
comments, must lower to nothing. Non-ASCII identifiers, which the old lexer
rejects, stay refused by the gate until a reviewed "newly in scope" entry
says otherwise. Then: switch behind a flag, make it the default, and delete
the hand-written parser only after an agreed parity window (B5).

**The lowering walks CST views.** LANG84 C1 and C2 (no typed lifts, no
hand-written parsers) apply to the WIR front end (decision Q5), so VER25b's
lowering moves from `rust_subset::ast` to borrowed views over the CST
(LANG84 C1.6).

### 7.2 gf256: compile-time tables (owner decision)

gf256 today builds its log and antilog tables at run time:

```rust
static TABLES: OnceLock<Tables> = OnceLock::new();
fn tables() -> &'static Tables { TABLES.get_or_init(build_tables) }
```

Modelling `OnceLock` soundly in WIR would mean modelling lazy, racy-looking
initialization for one crate. The owner chose the simpler, more honest route:

- **gf256 is refactored to build its tables at compile time** with a
  `const fn` (`const fn build_tables() -> Tables`, `static TABLES: Tables =
  build_tables();`). This is a **separate gf256 PR**, before B4. gf256 has
  about 95 dependents. Its Warrant evidence and assurance report must be
  regenerated and stay passing, and its effect certificate regenerated
  (VER-BACKLOG OPS-005).
- **`OnceLock`, `LazyLock`, `lazy_static!`, `once_cell` and similar lazy
  statics stay unsupported.** The `rs-confined` gate refuses them
  (`Unsupported`), so a proof never silently assumes an initialization order.
- B4 then lowers a `static` whose initializer is a `const fn` call by
  lowering that `const fn` like any other function and running it, bounded,
  to initialize the WIR global, so the proofs reason about the same code that
  builds the table.

What gf256 still needs from the grammar and gate (B3): out-of-line `pub mod
capabilities;` (loaded or refused, per WD-010), `const` and `static` items,
`struct Tables { log: [u16; 256], … }`, struct-literal shorthand, `#[inline]`,
`assert!(b != 0, "msg")`, `return if … { 1 } else { 0 };`, `while`, `<<=`,
`>>=`, `^=`, `%`, `t.log[a as usize]`, `impl Field { fn new(…) -> Self { Self
{ … } } }`, `&self`, `&'static`, `for _ in 0..8`, and `capabilities.rs`'s
`#![deny(unsafe_code)]` and `#[derive(Debug, Clone, Copy, PartialEq, Eq)]`
(an opaque derive: the struct stays usable, the derived impls do not reach
proofs).

### 7.3 The capability analyzer

VER08a's analyzer lexes Rust by hand today. Several of its security-review
findings are lexer facts the shared lexer gets right by construction: D25
(the `#!/x"` shebang), D27 (edition-dependent `c"`/`cr#"`), the
Pattern_White_Space set, raw identifiers, and ASCII-only identifiers (CAP003
today, because the analyzer carries no Unicode tables).

**Step 1 (C4)** replaces the analyzer's `lex()` with the release and edition
`.tokens` file plus the C3 hooks, keeps its token walker, and adds a parity
test against the old lexer over every `.rs` in the repository and every
review proof of concept.

**An unknown edition** gets dual-lex fail-closed handling: lex under every
candidate edition, and if the token streams differ anywhere, report CAP003 at
the first divergence. This generalizes D27's rule and keeps its security
property.

Non-ASCII identifiers become analyzable through the generated XID classes and
`unicode-normalize`. That **changes a CAP003 class**, so C4 needs its own
security review. The analyzer gains `grammar-tools`, `lexer`,
`rustlang-lexer` and `regex` as dependencies; only three BUILD files invoke
it, so the rebuild set is small. The depth and raw-string length budgets of
section 8 must exist before the analyzer depends on the lexer.

---

## 8. Budgets for untrusted input

Rust source is untrusted (LANG84 C5). Every breach is a typed `Budget {
which, limit }` error, never a panic or stack overflow, and every budget has a
child-process hostile probe.

| Budget | Default | Enforced in |
| --- | --- | --- |
| Source bytes | LANG84 C5 defaults; 1 MiB for the proof front end (VER25a) | before lexing |
| Block-comment depth | 256 | `rust_block_comments`, `rust_strip_shebang`; later X1 |
| Raw-string hashes | 255 (`rustc`'s limit; the regex has exactly 256 arms) | lexer |
| Parser rule depth | measured per pair; required (`with_max_depth`) | `GrammarParser` |
| Source nesting (`rs-confined`) | 32 (VER25a) | subset gate |
| Tokens (`rs-confined`) | 65,536 (VER25a) | subset gate |
| Expansion depth | the release's `recursion_limit` default (raised from 64 to 1024 in 1.19 for macros) | expander |
| Expanded tokens, invocations | fixed caps, recorded in the release profile | expander |
| Matcher work | per matcher step | expander |

---

## 9. Slice plan

### 9.1 Order: value first (an explicit deviation from LANG84)

LANG84 rule 4 and phase 9 say "earliest release first". **RS01 deviates, by
owner decision (Q1):**

1. the modern proof subset (phases A, B);
2. the full modern grammar in four editions (C);
3. the engine batch (D);
4. expansion (E);
5. 1.x back-fill, newest to oldest (F): each step is a delta from a grammar
   already checked against the oracle, and every 1.x `rustc` is available;
6. pre-1.0, oldest to newest (G): each step is a delta from source, ending at
   the join with 1.0.0-alpha.

The reason is value: the proof front end and the capability analyzer need a
modern Rust grammar now, and a pre-1.0 grammar helps neither. LANG84's
amendment records this as a Rust-only exception.

**Milestone 1** is phases A and B. It needs **no** change to `grammar-tools`,
`lexer` or `parser`, which matters because such a change affects about 5,300
of 5,337 packages and needs `ci-full-build`.

**Would-build basis.** `code/specs`, `code/grammars` and `code/scripts` map to
zero packages (`sharedPrefixes` is empty), but path-filtered gates still fire:
`ruby-grammar-regen-check` on `code/grammars/**`, `rust-grammar-regen-check`
and `mosaic-grammar-regen-check` on the grammar-tools crates, the inventory
check in a CI script step, and **every** gate on any edit to `ci.yml` or
`code/specs/data/ci-gates.json`. Each slice runs `build-tool -root .
-diff-base origin/main -dry-run` **after** its first commit to confirm its
estimate.

### 9.2 Phases

**Phase A: foundations (no engine change).**

| Slice | Scope | Would-build |
| --- | --- | --- |
| A1 | This spec and its LANG84, VER25a, VER25b and VER-BACKLOG amendments | 0 packages |
| A2 | Inventory: 158 rows (1.100.0 when tagged), exclusions header, README with date sources, `editions` column and derivation, `subsets.csv` header with `allowlist`. Checker: edition-qualified stems, `editions` validation, uniform claims across editions, `.reject@<release>-e<edition>` overrides | 0 packages; CI script step; `ruby-grammar-regen-check` |
| A3 | Evidence tooling: `rust_release_pairs.py` (materialize `reviewed`, `copy:` and `semantic-copy:` pairs; `check` mode); first `pair-evidence.csv` rows | 0 packages |
| A4 | Modern lexical pairs: complete lexers for 1.97.0 in four editions, grammars of token trees plus delimiter balance (`rs-token-trees`); generated raw-string arms and XID classes (X7) | 0 packages + gates |
| A5 | Crates `rustlang-lexer`, `rustlang-parser` with compiled, deduplicated, edition-aware tables; the two C3 hooks; `release_probes`; hostile probes; host-gated token-oracle test; `code/grammars/rust/**` added to `rust-grammar-regen-check` | ~10 packages and **all gates** (the `ci-gates.json` edit) |

**Phase B: the proof subset (LP-001a).**

| Slice | Scope | Would-build |
| --- | --- | --- |
| B1 | `rs-confined` grammar (VER25a scope, which is ct-compare) in the four 1.97.0 pairs; subset gate and allowlist; corpus with `rustc`-run `.expect` | `rustlang-parser` + few dependents |
| B2 | `warrant-ir` grammar path; the differential against the hand-written parser; resolved-macro allowlist; fail-closed policy | `warrant-ir` + ~12 |
| B3 | gf256 grammar and gate coverage (section 7.2) | parser crate + `warrant-ir` |
| B4 | VER25b lowering over CST views: `const fn` table initialization, struct arrays, `if` as an expression; `gf256_proofs` (38 claims) goes live. Needs the gf256 const-table PR first | `warrant-ir` |
| B5 | Retire the hand-written parser after the parity window | `warrant-ir` |

**Phase C: full modern grammar, oracle, `cfg`, analyzer.**

| Slice | Scope | Would-build |
| --- | --- | --- |
| C1 | Full grammar for the newest release in four editions, translated from the Reference grammar blocks at the tag's pinned reference commit (cuts rewritten); `rs-syntax`; the repository self-corpus | parser crate + dependents |
| C2 | Oracle harness `rust_oracle.py`: pinned toolchains, `cfg(FALSE)` and dumper techniques, a scheduled (daily), non-blocking CI sweep writing oracle records | 0 packages; a workflow edit runs all gates |
| C3 | `rustlang-cfg`: the L1 evaluator with per-release `CfgSet` rules | a new crate |
| C4 | Capability analyzer step 1: shared lexer, dual-lex for unknown editions, parity suite, security review | the analyzer + prerequisites |

**Phase D: engine batch (`ci-full-build`, about 5,300 packages each).**

| Slice | Scope | Would-build |
| --- | --- | --- |
| D1 | Lexer side: X1, X3, optionally X2; delete both Rust C3 hooks; port rejections | ~5,300 |
| D2 | Parser side: X4, X5, optionally X6; port rejections | ~5,300 |

**Phase E: macro expansion** (crate `rustlang-macros`).

| Slice | Scope | Would-build |
| --- | --- | --- |
| E1 | `macro_rules!` definition checker (per-release fragments, follow sets, 1.89 rule) | new crate |
| E2 | Matcher, transcriber, invisible-delimiter interpolation, budgets | same |
| E3 | Hygiene side table; textual and path scoping; `$crate`; `#[macro_use]`/`#[macro_export]` | same |
| E4 | Builtins per release; `std` macro definitions extracted per release as data; `include*`/`env!` opaque or supplied | same |
| E5 | Optional builtin derives; the `Unexpanded` contract | same |
| E6 | Consumers adopt expansion (proof front end: local `macro_rules!`; analyzer: fewer CAP003) | consumers |

**Phase F: 1.x back-fill, newest to oldest.** Reviewed deltas at class
boundaries, `semantic-copy` elsewhere, and a scheduled (daily) oracle sweep before any
stage claim.

| Slice | Releases |
| --- | --- |
| F1 | 1.85–1.99 (2024 era) |
| F2 | 1.56–1.84 (2021 era) |
| F3 | 1.31–1.55 (2018 era) |
| F4 | 1.13–1.30 |
| F5 | 1.0.0–1.12 |
| F6 | 1.0.0-alpha, 1.0.0-alpha.2, 1.0.0-beta |

**Phase G: pre-1.0, oldest to newest, one PR per release.** Each derives its
pair from `lexer.rs`, `token.rs` and `parser.rs` at the tag, with pinned
blob IDs, and adds that tag's rows to `historical.csv`.

| Slice | Release | Notes |
| --- | --- | --- |
| G1 | 0.1 | `#name(exprs)` extensions; `.rc` files once X4 exists; ast-mbe `#macro` optional |
| G2 | 0.2 | |
| G3 | 0.3, 0.3.1 | 0.3.1 is a `copy:0.3` |
| G4 | 0.4 | block comments stop nesting; `macro_rules!` |
| G5 | 0.5 | confirm 0.4's dual `#fmt`/`fmt!` while here |
| G6 | 0.6 | `LIFETIME` token |
| G7 | 0.7 | |
| G8 | 0.8 | |
| G9 | 0.9 | nested comments and raw strings return |
| G10 | 0.10 | `#![attr]`; try the binary oracle |
| G11 | 0.11.0 | byte literals; struct-literal restriction |
| G12 | 0.12.0 | joins 1.0.0-alpha |

---

## 10. Decisions

RS01 settles the plan's open questions as follows. Q1, Q3, Q11 and the gf256
choice were decided by the owner on 2026-11-12. The rest follow the plan's
recommendations: they are **coordinator-decided, owner override welcome**.

| # | Question | Decision | Status |
| --- | --- | --- | --- |
| Q1 | Order of work | Value first: modern proof subset, full modern grammar, engine batch, expansion, 1.x newest to oldest, pre-1.0 oldest to newest; recorded as a deviation from LANG84 | owner-decided |
| Q2 | Default release and edition | No implicit edition, ever; `DEFAULT_RELEASE` is the exact ID 1.97.0 | coordinator-decided, owner override welcome |
| Q3 | Evidence for copying 1.x minors | `semantic-copy`, only when all three conditions of section 6.3 hold, with the evidence in `pair-evidence.csv` | owner-decided |
| Q4 | Status for non-IIR consumers | `subsets.csv` gains an `allowlist` column; the proof subset is recorded as `grammar_subset=rs-confined` on a `partial` row; no new status column | coordinator-decided, owner override welcome |
| Q5 | Do LANG84 C1/C2 bind the WIR front end? | Yes: VER25b's lowering walks CST views | coordinator-decided, owner override welcome |
| Q6 | Expander classification | An amended LANG84 "preprocessor dialect" exception | coordinator-decided, owner override welcome |
| Q7 | Names | Crates `rustlang-lexer`, `rustlang-parser` (and `rustlang-cfg`, `rustlang-macros`); stems `rust<release>-e<edition>` | coordinator-decided, owner override welcome |
| Q8 | Pre-1.0 test corpora | A manifest (tag, path, blob) fetched by the scheduled (daily) CI job; nothing vendored | coordinator-decided, owner override welcome |
| Q9 | `std` macros | Extracted per release as data files | coordinator-decided, owner override welcome |
| Q10 | `RUSTC_BOOTSTRAP=1` | Allowed only in oracle jobs, for diagnostic output from pinned stable binaries; it never widens accepted syntax | coordinator-decided, owner override welcome |
| Q11 | Fidelity target | Stable acceptance per (release, edition); divergences documented and oracle-tested | owner-decided |
| Q12 | Unicode tables | Generated XID classes per Unicode version, per release | coordinator-decided, owner override welcome |
| Q13 | 1.100.0 | Waits for its tag | coordinator-decided, owner override welcome |
| Q14 | Engine batch | Two PRs: lexer side (D1) and parser side (D2) | coordinator-decided, owner override welcome |
| gf256 | `OnceLock` tables | Refactor gf256 to compile-time `const fn` tables in a separate PR; `OnceLock` and lazy statics stay unsupported and fail closed | owner-decided |

### A2 decisions (inventory slice)

Slice A2 wrote `code/grammars/rust/{releases.csv,subsets.csv,README.md}` and
the checker support. Where this spec was silent or ambiguous, A2 took the
conservative reading:

1. **Dates.** Section 1.2 names `RELEASES.md` as the source but tabulates no
   dates. A2 read the `Version X (YYYY-MM-DD)` headings of `RELEASES.md` on
   2026-11-12 for 156 rows and copied the two stated exceptions exactly:
   0.3.1 is `2012-07?` and 1.0.0-beta is `2015-04-03`. The 1.0.0-beta date is
   taken from this spec, not re-verified against the announcement.
2. **Columns.** After the twelve required columns, `releases.csv` has
   `date_source` (`releases-md`, `tag-commit` for 0.3.1, `announcement` for
   1.0.0-beta), `lineage` and `editions`, following Python's layout plus
   section 2.3. `source_sha` is the commit each tag names (peeled for
   annotated tags); `source_ref` is the tag URL (`release-0.3.1` for 0.3.1).
3. **Lineage the spec leaves open.** Section 1.2 does not say what follows
   1.0.0-alpha. A2 chains 1.0.0-alpha.2 to 1.0.0-alpha, 1.0.0-beta to
   1.0.0-alpha.2 and 1.0.0 to 1.0.0-beta (all `line`), so 2015 reaches 1.0.0 by
   lineage. Each pre-1.0 final follows the previous final, so 0.4 follows 0.3,
   not 0.3.1. 1.0.0-alpha and every other final's first row of a line is a
   `branch-point`; the first row is `first`.
4. **The allowlist file.** A2 declares `rs-confined.allowlist` in
   `subsets.csv` but does not create it: the gate is slice B1's work (see the
   backlog). The checker therefore requires an allowlist file to exist only
   once a row claims the subset (`partial` with that `grammar_subset`, or an
   `iir_status`/`backend_status` claim). A declared name must be a plain
   `*.allowlist` file name in the grammar directory; a symlink is refused.
5. **No `release_classes.csv`.** Nothing claims a backend, so there are no
   classes to record.
6. **Checker scope.** The edition mechanics (header, column, derivation,
   stems, uniform claims, pair overrides) apply to any inventory that has an
   `editions` column or a `# editions:` comment, and to nothing else. The
   Rust-only rules (published IDs only, prescribed channel, predecessor and
   lineage, no two rows with the same predecessor and lineage) apply to the
   directory named `rust`, which must also declare the axis. The only
   published pre-releases are 1.0.0-alpha, 1.0.0-alpha.2 and 1.0.0-beta, so
   any other pre-release, nightly or `release-` looking ID is refused.
7. **Pair count.** The checker reports the derived count (347) as a note
   instead of hard-coding it; a unit test pins 347 and the per-era split
   (13, 45, 34, 43, 23 releases).
8. **`rs-token-trees`.** Section 6.2 names the subset but no spec for
   `subsets.csv`; A2 gives it spec `RS01` and `allowlist` `-`.

### A3 decisions (evidence tooling slice)

Slice A3 wrote `code/scripts/rust_release_pairs.py` (modes `check` and
`write`), its tests, and the header-only `code/grammars/rust/pair-evidence.csv`.
No pair file exists yet, so the evidence file has no rows and `check` passes on
the empty state; slice A4 writes the first pairs and rows. Where this spec was
silent or ambiguous, A3 took the conservative reading:

1. **Columns.** Section 6.3 lists "syntax-file blob columns" that differ by
   era. A3 fixes one era-neutral layout:
   `release,edition,commit,syntax_blobs,basis,accepted_blob,accepted_features,releases_md_blob,releases_md_section,sweep_toolchain_sha256,sweep_digest,oracle,note`.
   `syntax_blobs` is `;`-separated `<path>=<blob or tree ID>` entries (`-` for a
   path absent at the tag); the XID crate version is recorded as the
   `Cargo.lock` blob. The six `accepted_*`, `releases_md_*` and `sweep_*`
   columns are the three `semantic-copy` conditions and must be `-` on every
   other basis, so stale evidence cannot linger on a row that no longer relies
   on it.
2. **What "copy" copies.** Section 2.2 says each header names its own release
   and edition, and section 2.5 says `check` fails when a copy's *body* drifts.
   A3 therefore copies the body (everything after the header block's blank
   line) byte for byte, and generates the header from `releases.csv` and the
   evidence row. A copy's file is never byte-identical to its source's: only
   its body is, and `check` compares the whole file with the bytes `write`
   would produce.
3. **Which sources are allowed.** `copy:<pair>` may name (a) the same release
   at an earlier edition (a sibling) or (b) a release on the target's
   `predecessor` chain at the **same** edition. An edition-less pre-1.0 pair
   never seeds an edition pair, and an edition pair never seeds a different
   edition of another release. `copy:` also needs the recorded `syntax_blobs`
   to equal the source row's exactly.
4. **`semantic-copy` eligibility.** Only a 1.x minor (`1.Y.0`) may use it, only
   from its direct `predecessor` release, and only for the same edition.
   Patches use `copy:`. All six condition columns must be present and
   well formed (blobs 40 or 64 hex, digests 64 hex, `accepted_features` either
   `none`, meaning no feature had `since` equal to this release, or a
   `;`-list of feature identifiers, and a non-empty RELEASES.md section).
   `check` verifies that the evidence is recorded, not that it is true: it
   uses no network and runs no toolchain, so the blobs, the toolchain digest
   and the sweep result are re-established by slice C2's oracle harness.
5. **What a `reviewed` row names.** Its inventory commit, at least one
   `syntax_blobs` entry, a note saying what changed (section 6.3's "reviewed
   delta from its predecessor") and an `oracle` cell, which may be `-` until
   slice C2 writes records. The script never writes a reviewed file; `check`
   requires its header to cite `# source_sha:` equal to the row's commit and
   to carry a `# basis: reviewed` line.
6. **Coverage and order.** Evidence rows cover exactly the pair files on disk:
   a file with no row, a row with no files, and a pair with one of its two
   files are errors. A3 does **not** require evidence for every inventory pair
   yet (none exist); the inventory's own check still requires files for any
   non-pending release. Rows are listed in `releases.csv` order, then by
   edition, so the file has one canonical form.
7. **Hostile input.** The tool opens every file with `O_NOFOLLOW` and requires
   a regular file, refuses a symlinked grammar directory, builds paths only
   from stems that matched a strict pattern (and re-checks that they are direct
   children of the directory), caps file, row and cell sizes, refuses control
   and bidi characters in any cell, and prints messages through
   `release_inventory.printable`. It reuses `release_inventory`'s stem parser,
   editions header and edition derivation rather than duplicating them.
8. **CI.** `rust_release_pairs.py check` and its unit tests run in the
   repository-wide metadata contract step beside `release_inventory.py check`.
   The `ci.yml` build-tool classifier reports that edit as shared, which keeps
   package-scoped selection on a pull request: would-build is 0 packages.

---

## 11. Not verified

These are stated plainly so that no later slice treats them as facts.

- The release date of 0.3.1.
- Whether 0.4 accepts both `#fmt` and `fmt!`; whether `#macro` exists in 0.4;
  1.0.0-beta's syntax relative to 1.0.0-alpha.2.
- Whether 1.0 alpha releases treated unstable syntax as warnings or errors,
  and from which pre-1.0 release `#[feature]` gates existed.
- Whether the 0.10–0.12 Linux binaries run on today's CI image, and whether
  the oldest 1.x binaries do.
- Every "candidate semantic copy" in section 2.7. They come from two upstream
  documents, not from running each release's compiler.
- The exact XID crate and Unicode version per release (to be read from
  `Cargo.lock` at each tag).
- The 2015-era `?` separator ambiguity and per-era follow-set rules; only the
  current Reference was read.
- Whether `# ! [attr]` with spaces is accepted in every era.
- Whether any untagged 1.0 beta refresh (beta.2 and later) was ever
  published as a release. They are out of scope either way (section 1.1).
- The release in which `rustc` moved from 1.0's shebang rule to the current
  token-skipping rule.
- The would-build numbers: estimates from the build-tool source, the gate
  registry and lessons; no dry run was possible before a branch existed.
- For the interim comment design of section 4.3, which was designed after the
  probes rather than probed itself: that F10's push stack has, or can be
  given, the depth budget of section 8; that an E8 refusal can fire on a
  token inside an F10 group; that a post-tokenize hook can set the F11 flag
  of a token; and that keeping line comments as tokens in nesting releases
  avoids the global-skip trap. The probe showed only that an F10 group tracks
  nesting correctly. Slice A4 verifies the rest before A5 registers the hook.
- Whether pre-1.0 `rustc` archives come with upstream `.sha256` files to pin
  against.
