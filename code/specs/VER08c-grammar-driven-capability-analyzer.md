# VER08c: A Grammar-Driven Capability and Weakness Analyzer for the Remaining Lanes

**Parent:** [13 Capability Security](13-capability-security.md) (Layers 3 and 4), [VER08a](VER08a-rust-capability-analyzer.md) (the Rust analyzer whose contract this tool imitates), [OCAML06](OCAML06-capability-analyzer.md) (the OCaml analyzer), [VER15a](VER15a-weakness-registry-v2-and-lane-matrices.md) (names the rules this tool must implement, section 5.6; on PR #19954 until it merges), [logic-gates-everywhere](VER-BACKLOG.d/logic-gates-everywhere.md) (work item LGE-S4; builds LGE-A1 and LGE-A2).

**Status:** Draft. Docs only. Revised after design/security review: sections 16 and 17 are normative and override any earlier sentence it contradicts (fail-closed scope, version-gap caps, mandatory closed-world for ambient preludes, gate-only promotion). The letter `c` is the next free one: `VER08a` and `VER08b` exist on `origin/main`, and no `VER08c` does. (`VER08a` links a parent `VER08-capability-analyzers.md` at `VER08a-rust-capability-analyzer.md:3`; that file is not on main, so this spec does not depend on it.)

---

## 0. The idea in one page

Five languages in this repository have a capability analyzer: Rust, Go, Python, TypeScript and OCaml. The other thirteen lanes (c, cpp, csharp, fsharp, java, kotlin, ruby, perl, haskell, elixir, lua, swift, dart) have none. A package in those lanes that says `capabilities: []` ("I touch nothing") is believed, not checked.

The owner's decision (`VER-BACKLOG.d/logic-gates-everywhere.md`, "Owner decisions" item 2) is to fill the gap with **one tool, built on the repository's own grammars and parsers**, not thirteen hand-written scanners. This spec says how.

The shape of the answer:

```text
   source files of one package                     rule table for the lane (DATA, JSON)
            |                                                |
            v                                                v
   lane lexer (from <lang>.tokens)  ----->  +----------------------------------+
            |                                |   ca-grammar-analyzer (Rust)     |
            v                                |   - checks tokens or the tree    |
   lane parser (from <lang>.grammar) ----->  |   - deny by default              |
            |                                |   - bounded, no eval, no I/O     |
            v                                |     beyond reading the package   |
   GrammarASTNode tree  (only in AST mode)   +----------------+-----------------+
                                                              |
                                                              v
                      report: CAP001 / CAP002 / CAP003 lines, SUMMARY line, JSON
                                                              |
                                                              v
                      lane evidence "analyzer-rule:<lane>.<rule>"  (VER15a 5.2)
```

Two modes, because six lanes have no `.grammar` yet:

* **AST mode** (lanes that have a grammar): rules look at the parse tree.
* **Tokens-only mode** (lanes that will first get only a `.tokens` file): rules look at the token stream. This mode can only prove that certain *words* are absent. It cannot prove anything about *types* or about what a name refers to. Section 5 draws that line exactly.

A rule only counts as `live` evidence (VER15a section 5.2) when this tool runs in that lane's CI, and every rule carries a **negative control**: a tiny source file that must trigger it, so a rule that has quietly stopped matching anything cannot pass.

---

## 1. Goals and non-goals

### 1.1 Goals

1. **Same verdicts, same vocabulary** as the existing analyzers: `CAP001` (undeclared capability), `CAP002` (banned construct), `CAP003` (seen but cannot analyse), exit codes `0`/`1`/`2`, a machine-readable `SUMMARY` line (section 7).
2. **Deny by default.** A package that declares no capabilities may use only what a per-lane allowlist names. Everything the tool cannot classify is a finding, not a pass.
3. **Rules are data.** Adding a banned function, an allowed import or a whole language's first rule set is a JSON edit plus a negative-control fixture, not a code change (section 6).
4. **Built on the repository's grammars.** No hand-written lexer or parser in the tool. A lane whose grammar is missing or too old gets its grammar fixed, not a scanner bolted on.
5. **Honest soundness labels.** Every rule says what class of claim it supports (section 5.2), and the report never states a claim stronger than the weakest rule behind it.
6. **Produce evidence the lane matrices can cite** (`analyzer-rule:<lane>.<rule-id>`), including the normative C rules of VER15a section 5.6.

### 1.2 Non-goals

* Not a sandbox, not a runtime cage, not a linter for style. (`OCAML06-capability-analyzer.md` makes the same statement about itself.)
* Not a replacement for the Rust, Go, Python, TypeScript or OCaml analyzers. Those lanes keep theirs; this tool is for the thirteen that have none.
* Not a full type checker or a points-to analysis. Where a rule needs types (section 9) it is `planned` with a stated reason, not faked with a name list.
* Not a proof of absence of vulnerabilities. It checks capabilities and a short list of banned constructs.
* Not a way to analyse third-party dependencies. Like VER08a section 3.6 it lists them and says it did not look inside.
* No signing, publishing or approval flow (Spec 13 Layers 5 and 6 are separate).

---

## 2. What exists today (facts this design rests on)

Everything below was read on `origin/main` at the time of writing, with a `file:line` so it can be re-checked.

### 2.1 The existing analyzers and their contracts

| Lane | Tool | Detail |
|---|---|---|
| rust | `ca-capability-analyzer-rs` | Lexical (not a parse), zero third-party dependencies, `CAP001`/`CAP002`/`CAP003`, exit `0`/`1`/`2` (`VER08a-rust-capability-analyzer.md:221-229`). Present-but-excepted constructs stay visible (`VER08a...md:239-250`). `SUMMARY unsafe_code=... ffi=... unanalyzed_dependencies=N` (`code/packages/rust/ca-capability-analyzer/src/report.rs:218-232`). |
| ocaml | `coding-adventures-capability-analyzer` | Parses with `compiler-libs`; an unparseable file is exit `2`, "never a warning" (`OCAML06-capability-analyzer.md:69-71`); matching is syntactic and conservative (`:122`); an independent protected CI gate scans every OCaml package so a package's own `BUILD` cannot remove it (`:181-191`). |
| go | `go/ca-capability-analyzer` | `CAP001` and `CAP002` (`code/packages/go/ca-capability-analyzer/report.go:92-94`); exit `0`/`1`/`2` (`main.go:14-18`). |
| python | `python/ca-capability-analyzer` | Built on the `ast` module (`README.md:3`). |
| typescript | `typescript/ca-capability-analyzer` | TypeScript compiler API; reports **legacy category names** such as `net:socket` and `ffi:unsafe` (`code/packages/typescript/ca-capability-analyzer/src/rules.ts:27-28`), which are not Spec 13's closed pairs. |

Two consequences:

* The Spec 13 taxonomy is the only vocabulary a new analyzer should speak: 11 categories and 25 category/action pairs, closed (`13-capability-security.md:270-319`, `:319-352`), with the JSON schema at `code/specs/schemas/required_capabilities.schema.json`.
* The analyzers do **not** share one machine-readable output today. Rust prints text with a trailing `SUMMARY` line; the TypeScript analyzer has a `--json` flag with its own shape. Section 7 therefore defines the contract rather than pretending a common one exists.

### 2.2 How `ca-assurance-report` uses analyzers today (an important correction)

The coordinator's brief and VER15a both speak of an analyzer feeding the assurance report. The code says otherwise, and the difference matters for this design:

* `ca-assurance-report` does **not** run any `ca-capability-analyzer` binary and does not parse its output. Its `Cargo.toml` has no dependency on it, and apart from the report field named `analyzer` (next bullet), the occurrences of "analyzer" in its source are evidence-id strings (`code/packages/rust/ca-assurance-report/src/evidence.rs:130-150`).
* The report's `capabilities.analyzer` field is `"pass"` if the *effect certificate's* label is `proved`, else `"fail"` (`code/packages/rust/ca-assurance-report/src/lib.rs:119-126`).
* The attestation `analyzer::no-unsafe-or-ffi` is derived from the certificate's unsafe exceptions plus the report's **own** source scan (`scan_unsafe_sources`, `evidence.rs:676`, called at `evidence.rs:1182`; the attestation at `evidence.rs:1316-1318`).
* Lane evidence (rungs per language) is a separate path through `lane_loader` / `lane_report` and does not mention an analyzer (`lib.rs:17-19`, `lib.rs:132-134`).

So "the output contract the consumer can read" does not exist yet; it will be written by work item **LGE-W1** (VER15a section 10). This spec defines the producer side precisely enough for LGE-W1 to consume (section 7) and keeps the text lines identical to VER08a so a human or a CI grep sees the same thing in every lane.

### 2.3 The grammar engine

* `code/packages/rust/grammar-tools` reads `.tokens` and `.grammar` files and produces `TokenGrammar` and `ParserGrammar` values, plus a cross-validator and a compiler to Rust source (`code/packages/rust/grammar-tools/README.md:1-60`; `src/token_grammar.rs`, `src/parser_grammar.rs`, `src/compiler.rs`). The `.tokens` format has `keywords:`, `skip:` and `errors:` sections and lexer groups (see the C file below); the `.grammar` format is EBNF with ordered choice (PEG semantics; `code/grammars/c/c.grammar:7-14`).
* Each language that has grammars also has a lexer crate and a parser crate in which the grammar is **compiled in** (`_grammar.rs`), for example `c-lexer`, `c-parser`, `java-lexer`, `java-parser` under `code/packages/rust/`. The Java lexer is the generic `GrammarLexer` driven by the compiled token grammar (`java-lexer/src/lib.rs:20-33`).
* The parse result is a generic `GrammarASTNode { rule_name, children, start_line, start_column, end_line, end_column }` (`code/packages/rust/parser/src/grammar_parser.rs:77-88`), whose leaves are `Token { type_, value, line, column, type_name, flags }` (`code/packages/rust/lexer/src/token.rs:311-340`). So the tree knows **what rule matched and where**, and nothing else: no types, no scopes, no resolved names.
* **The recursion guard is opt-in.** `GrammarParser::new` sets `max_depth: usize::MAX` (`grammar_parser.rs:525`); `with_max_depth` must be called. Each lane parser crate calls it (for example `java-parser/src/lib.rs:64`, `c-parser/src/lib.rs:58-60`); this tool must never build a parser any other way (section 12).
* A generic `grammar-type-checker` crate exists (`code/packages/rust/grammar-type-checker/README.md:1-20`) but it is driven by *parser-emitted type declarations* for typed dialects of dynamic languages; it is not a C, Java or C# type checker.

### 2.4 Which lanes have grammars, parsers, and at what version

`code/grammars/` has `.tokens`/`.grammar` for c, csharp, fsharp, haskell, java, perl, ruby (and rust, python, typescript and others outside this scope). It has **nothing** for cpp, kotlin, elixir, lua, swift or dart, and there are no lexer or parser crates for them under `code/packages/rust/`.

The versions the logic-gates packages actually target, read from their build files:

| Lane | Package targets | Evidence | Grammar files that exist |
|---|---|---|---|
| java | Java 21 | `code/packages/java/logic-gates/build.gradle.kts:17-19` | `java1.0` ... `java21` (`code/grammars/java/`), parser takes a version string (`java-parser/src/lib.rs:59-65`) |
| csharp | `net9.0` (C# 13) | `code/packages/csharp/logic-gates/CodingAdventures.LogicGates.csproj:3` | `csharp1.0` ... `csharp12.0` only (`code/grammars/csharp/`) |
| fsharp | `net9.0` (F# 9) | `.../fsharp/logic-gates/CodingAdventures.LogicGates.fsproj:3` | up to `fsharp9`, `fsharp10` (`code/grammars/fsharp/`) |
| haskell | `Haskell2010` | `code/packages/haskell/logic-gates/logic-gates.cabal:17` | `haskell98`, `haskell2010`, and older (`code/grammars/haskell/`) |
| ruby | Ruby `>= 3.4.0` | `code/packages/ruby/logic_gates/coding_adventures_logic_gates.gemspec:13` | `ruby3.3.5` ... `ruby3.3.7`, `ruby0.49`, `ruby0.50`, unversioned `ruby` (`code/grammars/ruby/`); no 3.4 |
| perl | `>= 5.26` | `code/packages/perl/logic-gates/Makefile.PL:11` | per-release files such as `perl5.38.0`, `perl5.44.0`, plus unversioned `perl` (`code/grammars/perl/`); the 5.26.0 inventory row has status `pending` (`code/grammars/perl/releases.csv:587`); the parser entry point parses "the accepted Perl subset" (`code/packages/rust/perl-parser/src/lib.rs:46-48`) |
| c | the C package is on PR #19926, not on main; VER15a says it builds with `-std=c17` | VER15a section 5.3 | `c.grammar` / `c.tokens`: **integer-core subset only** (`code/grammars/c/c.grammar:1-3,28`) |
| cpp | package on PR #19928, not on main | backlog | none |
| kotlin | Kotlin 2.1.20 on JVM 21 | `code/packages/kotlin/logic-gates/build.gradle.kts:4,18` | none |
| elixir | `~> 1.14` | `code/packages/elixir/logic_gates/mix.exs:8` | none |
| lua | `>= 5.4` | `code/packages/lua/logic_gates/coding-adventures-logic-gates-0.4.0-1.rockspec:11` | none |
| swift | tools 5.9 | `code/packages/swift/logic_gates/Package.swift:1` | none |
| dart | SDK `^3.0.0` | `code/packages/dart/logic-gates/pubspec.yaml:8` | none |

Three things in this table are gaps, not details, and each becomes a rule in section 10:

* **C# 13 versus a C# 12 grammar**, **Ruby 3.4 versus a 3.3 grammar**, **Perl 5.26 versus no 5.26 grammar** and **C17 versus an integer-core grammar**. A grammar older than the language must make the tool *fail closed* (exit `2`, "this file does not parse under grammar X"), never pass. See section 8.3 for the residual danger (a PEG can accept newer syntax as something else).
* **The Ruby lexer is not driven by `ruby.tokens`.** `code/grammars/ruby/README.md:3-5` says `ruby.tokens` is stale because the hand-written `ruby-lexer` does not read it, and `ruby-lexer/src/lib.rs:3-4` says it is driven by a TOML state machine. The release API (`parse_ruby_at`) does use the generic `GrammarLexer` over per-release files (`ruby-parser/src/lib.rs:3-7`), but only up to 3.3.x. For Ruby 3.4 the tool therefore has two choices (section 10): extend the per-release grammar to 3.4, or accept the nearest older release and fail closed on anything it cannot parse.
* **The C pipeline is not a C pipeline.** `translation_unit = { function_def }` (`c.grammar:28`) means a header with prototypes does not parse, and the legacy `try_parse_c` entry point *deletes every line that begins with `#`* before parsing (`code/packages/rust/c-parser/src/lib.rs:84-100`), so `#include <stdio.h>` is invisible to that API. The token-input entry point (`try_parse_c_tokens`, `c-parser/src/lib.rs:109`) does not strip them, and the lexer produces `HASH` tokens (`c.tokens:116`). The tool must use the token-input path and read directives itself (section 9).

---

## 3. Architecture

### 3.1 One tool, one crate

A new Rust crate `code/packages/rust/ca-grammar-analyzer` (library plus binary `ca-grammar-analyzer`). It depends only on repository crates: `grammar-tools`, `lexer`, `parser`, and the per-lane lexer and parser crates. It has **no third-party dependency**, matching VER08a objective 4. Its own `required_capabilities.json` declares reading the analyzed package and the rule tables, and writing stdout (the same shape as the OCaml analyzer's manifest, `OCAML06-capability-analyzer.md:30-32`).

```text
ca-grammar-analyzer --lane java --dir code/packages/java/logic-gates [--json] [--verbose]
ca-grammar-analyzer --self-test --lane java        # runs the negative controls (section 11)
```

### 3.2 Where lane knowledge lives

| Thing | Lives in | Changes by |
|---|---|---|
| Lexical grammar | `code/grammars/<lang>/*.tokens` | a grammar PR |
| Syntactic grammar | `code/grammars/<lang>/*.grammar` | a grammar PR |
| Which grammar file and version the lane uses | the rule table's `grammar` block | a rule-table PR |
| What is allowed or banned | `code/specs/data/capability-rules/<lane>.json` | a rule-table PR |
| One fixture per rule that must trigger it | `code/specs/data/capability-rules/<lane>/controls/` | with the rule |

Rule tables are **data under `code/specs/data/`**, next to the weakness-registry files, and are embedded into the binary at build time (the way the OCaml analyzer embeds its rules; `OCAML06-capability-analyzer.md:87,176`). The binary never reads a rule table from the analyzed package, so a package cannot weaken its own rules.

### 3.3 Deny by default, concretely

For a package whose manifest declares `capabilities: []` (or has no manifest, which Spec 13 and VER08a treat as the same; `VER08a...md:40-42`):

0. **Scope of "deny by default".** It covers **imports only** unless `closed-world-identifiers` is on for the lane (section 16.2). Languages with ambient builtins (every lane except c and cpp; section 17.1 names each lane's implicit-import sources) give a program capabilities with no import at all, so an import allowlist alone says nothing about them.
1. **Imports.** Every import, include, require, use or `open` the tool can see must match an `allowed_imports` entry. An import that matches nothing is `CAP001` if it names a known capability module, and `CAP003` ("unclassified import") otherwise. This is the same "deny unknown std surface" rule as VER08a section 3.3 (`VER08a...md:146`).
2. **Banned constructs.** Anything in the lane's `banned` list is `CAP002`.
3. **Unanalysable constructs.** Anything the tool sees but cannot examine (section 5.4) is `CAP003`, never waivable.
4. **Parse failure** is exit `2`, never a pass and never a warning.

---

## 4. Inputs, exit codes and what "package" means

* Input is one package directory. The tool walks it deterministically (sorted relative paths), refuses symlinks, **skips nothing**: it enumerates `git ls-files` (the tracked set), not the working tree, so a build-output directory is simply untracked and a tracked one is analysed or refused (section 17.7), and it analyses every file whose extension the lane's rule table lists (`.java`, `.cs`, `.fs`, `.fsi`, `.hs`, `.rb`, `.pm`, `.pl`, `.c`, `.h`, ...). Test directories are **not** skipped: a path may be `nonsource` only if proven excluded from the published artifact, otherwise it is scanned under the same rules (section 17.7, which resolves the earlier tension with section 16.1; the lane's `source_roots` say which files are *expected*, and any shipped file outside them is a `CAP003`; compare gap G-5 at `VER08a...md:216,264`).
* Exit codes are exactly VER08a's: `0` clean, `1` violations (`CAP001`, `CAP002`, `CAP003`), `2` tool error (bad arguments, unreadable input, parse failure, quota exceeded, malformed manifest or rule table) (`VER08a...md:227-229`).
* The manifest is `required_capabilities.json` at the package root, validated against the Spec 13 schema; an unknown pair or a malformed file is exit `2` (`VER08a...md:38-55`). The schema's `banned_construct_exceptions[].language` enum currently lists only `python ruby go typescript rust elixir lua ocaml` (`code/specs/schemas/required_capabilities.schema.json:163`), so **for java, kotlin, csharp, fsharp, haskell, perl, c, cpp, swift and dart no exception can currently be written**. For the logic-gates packages that is the desired state (they declare no exceptions); extending the enum is a prerequisite item (LGE-A0, section 14) only when a package needs one.
* Resource ceilings follow VER08a section 5 (10,000 files, 4 MiB per file, 64 MiB total, 1 MiB manifest; `VER08a...md:211`), plus the parser budgets of section 12.

---

## 5. The two modes, and exactly what each can prove

### 5.1 The vocabulary of "sound"

A rule is **absence-sound** under stated assumptions if: when the tool reports no finding for it, the construct it describes does not occur. Rules are allowed to over-approximate (flag things that are harmless); they are not allowed to under-approximate. This is the same trade VER08a makes: "This over-approximates ... Accepted" (`VER08a...md:138`).

### 5.2 Rule kinds and their soundness level

Every rule has a `soundness` label. Higher numbers need more machinery and fewer rules reach them.

| Level | Name | What the rule looks at | Mode that can run it | What a pass means |
|---|---|---|---|---|
| S0 | `token` | one token: its type name (keyword, identifier, operator, string) and text | tokens-only or AST | "that exact word does not appear as code (outside comments and string literals as the lexer classifies them)" |
| S1 | `token-sequence` | a short run of adjacent tokens, for example `import` then a dotted name, or `NAME . NAME (` | tokens-only or AST | "that adjacent pattern does not appear". Sound only for the shape written; a reshaped spelling (a newline, a comment between the tokens) is already normalised away because comments are skipped by the lexer |
| S2 | `node` | a grammar rule match in the tree (`rule_name`) and its token leaves | AST only | "no subtree of that rule name with those leaves appears". Depends on the grammar being faithful to the language for that file |
| S3 | `name-resolved` | S2 plus resolving an identifier to its binding through imports, aliases, shadowing; **`closed-world-identifiers` is S3** and stays `planned` until a scope pass exists (section 17.2) | AST plus a resolver | **planned for every lane** (VER08a section 3.2 shows how much work this is for Rust; OCAML06 has a whole section on it) |
| S4 | `type-or-value` | types, value ranges, reachability | AST plus a semantic pass | **planned** (section 9 for C) |

A report lists the **lowest soundness level among the rules it relied on** for each claim. A claim built on S0 rules says "no occurrence of these names"; it never says "pure" or "no unsafe" unless S2 or higher rules back that wording.

### 5.3 What is sound in tokens-only mode, and what is not

Tokens-only mode exists so a lane with only a `.tokens` file can start producing evidence. It is useful and it is limited, and the limits are listed so no one stretches it.

**Sound (S0 and S1), under the lexer assumptions below:**

* **A banned keyword, annotation or modifier does not appear.** Examples: `native` (Java, Kotlin `external`), `unsafe` (C#, Swift's `Unsafe*` names are identifiers, F# `NativePtr`), `extern`, `asm`, `__asm__`.
* **A banned identifier does not appear.** Examples: `eval`, `load`, `loadstring`, `getClass`, `forName`, `system`, `exec`, `popen`.
* **No import of a module outside the allowlist.** For languages where imports are lexically regular (Java `import a.b.c;`, Kotlin, C# `using`, F# `open`, Haskell `import`, Dart `import '...'`, Swift `import X`, Elixir `alias/import/require/use`, Lua `require "x"`, C/C++ `#include`), an import statement is a short token sequence, so the allowlist check is S1.
* **A fully qualified use of a banned module without an import** (`java.io.File`, `System.IO.File`, `Foundation.FileManager`) when the rule bans the dotted *prefix* as a token sequence rather than the import line.
* **Presence counts**: the file contains no tokens of a given class (for example no string literal at all, if a lane's gate packages need none).

**Not provable in tokens-only mode (these rules are `planned` until AST mode exists for the lane):**

* Anything about **types**: "no pointer type", "no unchecked cast", "this integer cannot overflow". A token stream does not say what an expression's type is (VER15a section 5.6's C rules are of this kind; section 9).
* Anything about **what a name refers to**: whether `open` is the capability or a local helper, whether a local variable shadows `System`. Tokens-only mode therefore *over-approximates by name*: it flags every occurrence of the banned identifier, including a harmless local one. That can only cause false findings, never a false pass, **provided the banned thing cannot be reached without writing its name**. Where it can (reflection, dynamic dispatch, string-built names), the rule must instead ban the *reflective primitive* (`eval`, `Class.forName`, `_G`, `getattr`), and the report says that this closes the known routes, not all routes.
* Anything about **structure**: "this call is inside an `unsafe` block", "this function is reachable from a public one", "the argument is a literal".
* **Macro, preprocessor and template expansion** (C, C++, Rust-style macros, Elixir macros, Haskell Template Haskell, Swift macros, Lua `load`): the tokens the compiler sees can differ from the tokens in the file. See section 5.4.

**Lexer assumptions every S0/S1 rule silently depends on.** The rule table lists them per lane and each has its own negative control (section 11):

1. **Comments and strings never produce code tokens**, including nested comments (Haskell `{- {- -} -}`, Lua `--[==[ ... ]==]`), raw strings (C++ `R"x(...)x"`, Dart `r'...'`, Swift `#"..."#`), heredocs (Ruby, Perl, Elixir) and triple-quoted strings (Kotlin, Swift, Dart, Elixir).
2. **Identifiers are normalised the way the language normalises them.** Java translates `\uXXXX` escapes before lexing (so `System` is `System`); Kotlin and F# allow backtick identifiers; Haskell allows `qualified.Name` spelled across a layout; Perl allows `::` and `'` package separators. If the `.tokens` file does not model the normalisation, the lexer must report such constructs as an error token so the tool can say `CAP003`, instead of producing a harmless-looking token.
3. **String interpolation is lexed as code or refused.** `"${expr}"` (Kotlin, Dart), `"\(expr)"` (Swift), `"#{expr}"` (Ruby, Elixir), `` `... ${expr}` `` and `@{[ expr ]}` (Perl) all hide code inside a string. The lexer's group stack (`push_group` / `pop_group`, `code/packages/rust/lexer/src/grammar_lexer.rs:189-219`) can model nesting; a lane whose `.tokens` does not model interpolation must make any interpolated string a `CAP003`.
4. **Longest match and keyword priority are right** for the banned words: `.tokens` files in this repository put multi-character operators before single-character ones and use a `keywords:` section (`code/grammars/c/c.tokens:16-17,61-85`). A lane that lexes `native` as an identifier instead of a keyword still catches it as an S0 identifier rule, but the rule table records which kind it expects so the self-test notices a lexer change.

### 5.4 What the tool refuses to analyse (`CAP003`)

A `CAP003` is a construct the tool can see but cannot examine, never waivable by a manifest exception (VER08a section 3.5, `VER08a...md:164`). The per-lane list is in the rule table (`unanalysable`), and the common members are:

| Construct | Why |
|---|---|
| Any file the lane's grammar does not parse completely | not a `CAP003`: it is exit `2` (section 4), because the tool cannot even see the construct |
| A preprocessor or macro construct that can change tokens (`#define` anything but a recognised include guard, `##`, `#`-stringizing, `#if` over identifiers it does not model; Elixir `defmacro`/`quote`/`unquote`; Haskell `TemplateHaskell`, `{-# LANGUAGE CPP #-}`; Swift `@freestanding`/`@attached` macros; Lua `load`) | the compiler sees other tokens than the file contains |
| String interpolation the `.tokens` file does not model | code hidden inside a string |
| `eval`-like entry points as *banned* constructs (not `CAP003`): they are `CAP002` with no exception path in the logic-gates rule sets | |
| Conditional compilation that selects different source (`#if`, `cfg`, `{-# LANGUAGE CPP #-}`, Swift `#if`, Kotlin `expect`/`actual`) | one build sees different code from the one the tool read; VER08a ignores `cfg` and treats both branches as code (`VER08a...md:67`), and this tool does the same: it analyses *all* branches |
| Generated or included source (`#include "x.c"` of a non-allowlisted file, Ruby `require_relative` of a file outside the package, Dart `part`) | code outside the scanned set |

---

## 6. The rule-table format

One JSON file per lane, `code/specs/data/capability-rules/<lane>.json`, validated by `code/specs/data/capability-rules/rule-table.schema.json` (JSON Schema 2020-12, written with the implementation; the shape is fixed here). The tool refuses a rule table that fails the schema, has a duplicate rule id, or names a capability pair outside the 25 closed pairs: exit `2`.

### 6.1 Shape

```json
{
  "schema_version": 1,
  "lane": "java",
  "grammar": {
    "mode": "ast",
    "tokens": "code/grammars/java/java21.tokens",
    "grammar": "code/grammars/java/java21.grammar",
    "language_version": "21",
    "package_targets": "21 (build.gradle.kts:17-19)"
  },
  "extensions": [".java"],
  "source_roots": ["src/main"],
  "lexer_assumptions": ["comments-and-strings-hidden", "unicode-escapes-normalised"],
  "allowed_imports": [
    { "id": "java.allow.in-repo",
      "kind": "prefix", "value": "coding_adventures.",
      "note": "in-repo dependency; analysed by its own run (section 3.3)" }
  ],
  "rules": [
    { "id": "java.no-io-imports",
      "state": "active",
      "soundness": "S1",
      "kind": "import-prefix",
      "match": ["java.io", "java.nio", "java.net"],
      "capability": "fs:read",
      "emit": "CAP001",
      "weakness": ["CWE-22"],
      "control": "controls/java.no-io-imports.java",
      "message": "file and network classes are capabilities" },
    { "id": "java.no-native-method",
      "state": "active", "soundness": "S0", "kind": "token",
      "match": { "type": "KEYWORD", "text": ["native"] },
      "banned_construct": "foreign_linkage", "emit": "CAP002",
      "control": "controls/java.no-native-method.java" },
    { "id": "java.no-reflection",
      "state": "active", "soundness": "S2", "kind": "banned-call",
      "match": { "names": ["forName", "getDeclaredMethod", "getMethod", "invoke", "newInstance", "ClassLoader", "MethodHandles", "ServiceLoader", "ScriptEngine"] },
      "banned_construct": "dynamic_dispatch", "emit": "CAP002",
      "control": "controls/java.no-reflection.java" },
    { "id": "java.no-semantic-null-flow",
      "state": "planned", "soundness": "S4",
      "planned_item": "LGE-A1-java-sem",
      "reason": "needs types and flow; a token or node rule would be unsound" }
  ],
  "annotation_allowlist": ["Override", "FunctionalInterface", "SuppressWarnings"],
  "unanalysable": [ { "id": "java.cap003.unlisted-annotation", "kind": "annotation-not-in-allowlist" } ]
}
```

### 6.2 Fields, in plain words

* `grammar.mode`: `ast` or `tokens`. `tokens` rules may not use S2 or higher; the loader refuses a tokens-mode table that contains one.
* `grammar.language_version` versus `grammar.package_targets`: when the grammar's language version is older than what the package targets, the table must set `"version_gap": "<reason>"`, and the report prints it on every run. (C#, Ruby, Perl and C all need it; section 10.)
* `allowed_imports`: the allowlist of section 3.3. Kinds: `exact`, `prefix`, `in-repo-package`. An empty list means "no imports at all".
* `rules[].kind`:

| Kind | Level | Meaning |
|---|---|---|
| `token` | S0 | a banned keyword, identifier or operator token |
| `token-sequence` | S1 | a short banned adjacent-token pattern (patterns are token `type`/`text` pairs, no regular expressions on source text) |
| `import-prefix` / `import-exact` | S1 (tokens) or S2 (AST import node) | a banned import |
| `banned-call` | S2 | a call node whose callee is spelled with a banned name, with no resolution (over-approximates) |
| `banned-node` | S2 | a grammar rule match, for example every `unsafe_block` or every `array_declarator` |
| `grammar-is-allowlist` | S2 | the file must parse under a *restricted* grammar (section 9.2: C) |
| `token-whitelist` | S0 (tokens-only) | **tokens-mode alternative to closed-world** (section 17.2): the table field `identifier_allowlist: [..]` lists every identifier token the package may contain; any other identifier token is `CAP003`. Its soundness label is derived from `kind` as `S0` and the loader forces `max_evidence: claimed`: **it can never be `live` in tokens mode** (section 17.2). Example row: `{ "id": "lua.identifier-whitelist", "state": "active", "soundness": "S0", "kind": "token-whitelist", "identifier_allowlist": ["local", "function", "return", "a", "b", "and", "or", "not"], "emit": "CAP003", "control": "controls/lua.identifier-whitelist.lua" }` |
| `closed-world-identifiers` | S3 | mandatory for every lane except c and cpp (section 17.1); every identifier must be declared in the package or listed in `identifier_allowlist` (or a prelude-export field naming the prelude names allowed) | planned until a scope pass exists |
| `semantic` | S3/S4 | `planned` only, until a resolver or type pass exists |

* `capability`: one of the closed 25 Spec 13 pairs, so a finding means exactly what it means in the other analyzers. A rule that detects a banned construct instead carries `banned_construct`, a lane-scoped id (`unsafe_code`, `inline_assembly`, `foreign_linkage`, `dynamic_eval`, `dynamic_dispatch`, `unsafe_deserialization`, ...).
* `emit`: `CAP001` for a capability finding, `CAP002` for a banned construct, `CAP003` for an unanalysable one.
* `weakness`: the CWE ids or VER15a lane-threat ids the rule helps close; used only to build the evidence mapping of section 8. It is documentation, never a verdict.
* `control`: path to the negative-control fixture (section 11). **Required for every `active` rule**; the loader refuses an active rule without one.
* `covers` and `completeness`: the VER15a clauses the rule implements (for example `["5.6:part-A"]`) and `full` or `partial`. A `partial` rule can never be the sole closer of a threat row and can never be `live` (section 16.5).
* `build_files`: a top-level table section listing per lane the build-system files whose contents can change what is compiled, with CAP003 triggers (section 16.3).
* `max_evidence`: cap, `claimed` or `live` (default `live`). Wherever a forced condition applies (tokens mode, a lane with a non-null `version_gap`, any `token-whitelist` rule) the loader **forces the effective value to `claimed`**; an author-supplied `live` there is overridden to `claimed` or refused (exit `2`), never honoured. A rule with `max_evidence: claimed` can never be promoted to `live` whatever else holds. **Loader unit test:** a tokens-mode rule declaring `max_evidence: live` must be refused or forced to `claimed`; the same for a version-gap lane rule and a `token-whitelist` rule.
* `state`: `active` or `planned`. A `planned` rule is never evaluated and never counted; it exists so the table says honestly what is not covered (it must carry `planned_item` and `reason`).

### 6.3 Why JSON and not code

Spec 13's banned-construct tables are prose lists per language (`13-capability-security.md:652-698`). Putting the *same* lists into a schema-checked file means a reviewer can diff what changed in "what Java may do", and the negative-control requirement can be enforced mechanically. A rule table is reviewed like a spec change.

---

## 7. The output contract

### 7.1 Text lines (identical to VER08a)

```text
CAP001: Undeclared capability 'fs:read:*' detected at src/main/java/X.java:12:8 ('java.io.File')   [rule java.no-io-imports, S1]
CAP002: Banned construct 'foreign_linkage' detected at src/main/java/X.java:30:4 ('native')          [rule java.no-native-method, S0]
CAP003: Unanalysable construct detected at src/Y.c:3:0 ('#define'): macro can change tokens          [rule c.cap003.define]
PASS (deny-list, claimed): package 'java/logic-gates' capability check succeeded (4 files scanned, 8120 bytes) [lane java, grammar java21, mode ast]
```

The PASS line is qualified when the verdict cannot be `live`: `PASS (tokens-only, deny-list, claimed)` for a tokens-only lane; `PASS (claimed: version gap)` for a lane with a non-null `version_gap`; `PASS (deny-list, claimed)` for a lane whose closed-world rule is not live. A bare `PASS` is printed only when no cap applies, and **no lane is uncapped until a live S3 scope pass exists**, so in practice every PASS line today carries a qualifier.

```text
SUMMARY unsafe_code=absent inline_assembly=absent foreign_linkage=absent ffi=absent unanalyzed_dependencies=0   (any value above 0 is exit 1)
```

Line is 1-based, column 0-based, counted in characters, as VER08a section 6 states. The `SUMMARY` line keeps VER08a's three banned constructs and adds `dynamic_eval=` and `dynamic_dispatch=` for the lanes that have those constructs. A field prints `absent` only when an S2-or-higher rule backs it; when only S0/S1 name rules ran it prints `absent-by-name (S0)`, and a construct the lane cannot express prints `n/a`. `dynamic_dispatch` and `dynamic_eval` are never `absent` from name rules alone (section 16.7).

The `[rule ..., S1]` suffix is new and additive: it names the rule and its soundness level in every finding, so a reader can see how strong each verdict is.

### 7.2 JSON (`--json`), schema `ca-grammar-analyzer-report-v1`

The producer contract that LGE-W1 will consume:

```json
{
  "schema": "ca-grammar-analyzer-report-v1",
  "tool": { "name": "ca-grammar-analyzer", "version": "0.1.0",
            "rule_table_sha256": "<hex>", "tool_binary_sha256": "<hex>", "controls_sha256": "<hex>", "grammar_files_sha256": { "java21.tokens": "<hex>", "java21.grammar": "<hex>" } },
  "lane": "java", "package": "java/logic-gates",
  "commit": "<40 hex>", "package_tree_sha256": "<hex of the tracked-file tree>",
  "attestation": "<OIDC/Sigstore bundle reference>",
  "mode": "ast", "language_version": "21", "version_gap": null,
  "exit": 0,
  "files_seen": 5, "files_scanned": 4, "files_nonsource": 1, "bytes_scanned": 8120,
  "manifest_present": true, "declared": [],
  "violations": [ { "code": "CAP001", "rule": "java.no-io-imports", "soundness": "S1",
                    "capability": "fs:read:*", "file": "src/main/java/X.java", "line": 12, "col": 8, "evidence": "java.io.File" } ],
  "excepted": [],
  "rules": [ { "id": "java.no-io-imports", "state": "active", "soundness": "S1",
               "findings": 0, "control": "triggered" },
             { "id": "java.no-semantic-null-flow", "state": "planned", "planned_item": "LGE-A1-java-sem" } ],
  "summary": { "unsafe_code": "absent", "inline_assembly": "n/a", "foreign_linkage": "absent",
               "ffi": "absent", "unanalyzed_dependencies": 0 }
}
```

* `rules[].control` is `triggered` only if the same run executed the rule's negative-control fixture and the rule fired on it (section 11). `--json` without `--self-test` still runs the controls (they are small); a report whose controls did not all trigger is exit `2`.
* The hashes pin the exact grammar and rule table the verdict was computed with, so a later grammar change cannot silently reinterpret an old report.

### 7.3 How this relates to the existing consumer

Section 2.2 shows the assurance report reads an effect certificate and runs its own scan, not an analyzer's output. This spec does not change that. The **only** path from a rule to `live` evidence is section 8.2 (the central protected gate's attested report, consumed by LGE-W1). A grep of a lane's `BUILD` output, a locally produced report or a report from a pull request run is **never** evidence. LGE-W1 must verify `commit`, `package_tree_sha256`, `tool_binary_sha256`, `rule_table_sha256` and `controls_sha256` against the gate run it trusts, for example with an OIDC/Sigstore attestation of the gate workflow; a report that cannot be tied to such a run is ignored, which closes report forgery.

---

## 8. Mapping rules to VER15a threat ids and evidence states

### 8.1 The mapping

VER15a lane threat lists name evidence as `kind: analyzer-rule`, `id: <lane>.<rule>` and give it a `state` (`live`, `claimed`, `planned`, `unverified`) (VER15a sections 5.2 and 6.2). The rule ids in this spec's tables **are** those ids: `analyzer-rule:java.no-io-imports`. The `weakness` field of a rule lists the threats it helps close, and the lane files cite the rule, not the other way round.

### 8.2 When an analyzer rule is `live`

All of the following, evaluated by the reporter (LGE-W1), not asserted in prose:

1. The independent protected CI gate (LGE-A3) runs `ca-grammar-analyzer` over the package (the OCaml design: a package's own `BUILD` file must not be the only place the check lives; the lane's `BUILD` may also run it, but only the gate's report is consumed; `OCAML06-capability-analyzer.md:188-191`).
2. The run exits `0` and its JSON report says `control: triggered` for that rule in the same run.
3. The rule is `active` (a `planned` rule is `planned` evidence, whatever its text says).
4. Its soundness level is high enough for the claim the threat row makes (a row that says "no pointer types" cannot be closed by an S0 token rule; VER15a section 5.2: "Absence of a failure is not a proof of impossibility").

5. **The gate is the only source, and it runs the BASE-branch tool.** The report is the one produced by the central protected gate (LGE-A3). Lane `BUILD` runs may run the tool and fail the build, but their reports never promote a rule to `live`.
6. The lane has no non-null `version_gap` (section 16.2), every ambient-prelude lane has a live closed-world rule, or a qualifying prelude-export allowlist (section 17.1), and files_seen equals files_scanned plus explicitly-nonsource (section 16.1).

Until all hold the evidence is `claimed` or `planned`. Today **no lane has any `live` analyzer rule** (VER15a section 5.2, the `analyzer-rule` row), and this spec does not change that: it defines how they become live.

### 8.3 The version-gap rule

If `version_gap` is non-null (the grammar is older than the language the package targets): (a) any file that fails to parse is exit `2`; (b) the report carries the gap on every run; and (c) **no rule of that lane may be `live`**: the lane is capped at `claimed` until the grammar matches the target version, or a corpus-differential check against the target-version reference parser runs in CI (section 16.2). The residual risk is that a PEG with ordered choice may accept newer syntax *as something else* and so parse a file it does not understand. The mitigation is a per-lane list of "newer-than-grammar" constructs (C# 13 `params` collections and `lock` object, Ruby 3.4 `it` implicit block parameter, Perl 5.26-onward features not in the subset) each with a negative-control fixture asserting the tool rejects or flags it; the list is part of the lane's rollout row.

---

## 9. The C rules (VER15a section 5.6)

VER15a section 5.6 defines four rules that the C analyzer "must mean": `c.no-pointer-array-heap`, `c.no-ub-reachable`, `c.integer-value-range` and `c.no-shift-operators`. It defines them on the **preprocessed translation unit** and, for the first three, on **types after semantic analysis**. This section says what the repository can do today for each and what is missing. It deliberately does not claim more.

### 9.1 What the C grammar gives

`c.grammar` is the integer-core subset written for the SIR27 frontend (`c.grammar:1-3`). Reading it shows:

* Top level is only function definitions (`c.grammar:28`): no prototypes, no declarations, no `typedef`, `struct`, `enum`, `union`, `static`, `const`, `extern`, `inline`, `switch`, `do`, `goto`, `break`, `continue`. Of these, `struct`, `typedef`, `static`, `const`, `extern`, `sizeof` are not even keywords in `c.tokens` (the keyword list, `c.tokens:61-85`), so they lex as plain `NAME`s and cannot appear where the grammar expects a keyword; a use is a parse failure.
* **No pointer declarator and no unary `*` or `&`**: `unary = ( PLUS | MINUS | TILDE | BANG ) unary | postfix` (`c.grammar:105`), and `STAR` and `AMP` occur only as binary operators (`c.grammar:93,98`). A `char *p` or `*p` is a parse failure.
* **Arrays and strings are in the grammar**: `init_declarator = type_spec NAME [ LBRACKET [ INT_LIT ] RBRACKET ] ...` (`c.grammar:62`), `index_suffix` (`c.grammar:109`), and `STR_LIT` in `primary` (`c.grammar:115`). These must be banned by rule, not by parse failure.
* **Calls to any name are in the grammar** (`call_suffix`, `c.grammar:108`), including `malloc`, `sizeof(x)` (a `NAME` followed by a call suffix) and function-like macros the preprocessor would have expanded.
* **Shifts and casts are in the grammar** (`SHL`/`SHR`, `c.grammar:96`; `cast`, `c.grammar:103`).
* There is no semantic information in the tree (section 2.3).

So the subset grammar already *refuses by parse failure* most of what `c.no-pointer-array-heap` wants (pointer declarators, address-of, dereference, structs, function pointers, `typedef`-introduced types, prototypes). That is a genuine, cheap and sound mechanism, and the spec names it **`grammar-is-allowlist`**: *if the file parses under the restricted grammar, every construct the grammar cannot express is absent*. It is sound exactly as far as the grammar is correct about what it excludes, which is checked by the rule's negative controls (one fixture per excluded construct, each of which must fail to parse).

### 9.2 Rule by rule

| VER15a rule | Mechanism | State | What it needs / why not more |
|---|---|---|---|
| `c.no-shift-operators` ("no `<<` or `>>` at all") | `token` S0 rule on `SHL` and `SHR` (`c.tokens:30-31` shows the token names), plus a `banned-node` backstop on the `shift` rule when it has an operator child | **`active` in v1** | Fully token-level; the one VER15a rule that is. Also catches `<<=` only if the lexer has such a token; the subset has no compound assignment (`c.grammar:86`), so it parses as a failure. |
| `c.no-pointer-array-heap.a` (part A: declarations and expressions; `covers 5.6:part-A`, `completeness: partial` until the header grammar exists) | `grammar-is-allowlist` against the integer-core subset, plus S2 `banned-node` on `index_suffix`, on a declarator with `LBRACKET`, and on `STR_LIT` in `primary` | **`active` for what the grammar can express**; the C package's real headers do not parse yet (below) | Needs a **header grammar extension** (prototype declarations; section 9.3). Compound literals and `__func__` are `NAME`/syntax the subset cannot parse, so refused. |
| `c.no-pointer-array-heap.b` (part B; `covers 5.6:part-B`) | S2 `banned-call`: every call's callee must have a **body in the package** (not merely a prototype) **and** must not be on a libc/POSIX denylist (a package may define its own `malloc`; that is still refused) | **`active`**, S2 | This is a *name* check over `call_suffix` nodes. It is sound for "no allocator is called by name". It is not sound against a function pointer or a macro (the subset has no function pointer; macros are handled by the directive rule below). |
| include allowlist (`<limits.h>`, `<stdint.h>`, `<stdbool.h>`, the package header) | S1 over the `HASH` `include` directive tokens, read from the **token** stream by this tool itself | **`active`** | The tool must call `try_parse_c_tokens` after reading directives, never `try_parse_c`, because the latter deletes `#` lines (`c-parser/src/lib.rs:84-100`). Any directive other than the allowlisted `#include`, `#pragma once` and a recognised include-guard `#ifndef`/`#define`/`#endif` triple is `CAP003`. In particular any `#define` with a body is `CAP003`: PREP01 is not a finished C preprocessor (`PREP01-generic-source-preprocessor.md:1-26`, "completing C preprocessing is a long-term horizon"), so the tool cannot expand macros and refuses to guess. |
| variadics, `va_list`, inline assembly, `setjmp`/`longjmp`, `__attribute__` | S0 token rules; most are parse failures anyway (`...` has no token, `__attribute__`/`asm` are `NAME`s followed by syntax the grammar rejects) | **`active`** as controls proving the refusal | negative controls assert each is refused |
| `c.no-ub-reachable` | needs signed-overflow analysis, a shift rule (above), division/remainder checks, uninitialised reads, declaration/definition mismatch | **`planned`** | semantic; see 9.3 |
| `c.integer-value-range` | needs a value-range dataflow over typed expressions (VER15a: "`1 - a` with `a` in {0, 1}") | **`planned`** | semantic; see 9.3 |

### 9.3 The minimal type pass, and why the rest is `planned`

The two semantic rules need typed expressions and ranges. The tree has neither. What the repository does have is `c-to-semantic-ir` (SIR27), which "walks the CST, does C's static typing, and emits `semantic_ir::Module`" (`SIR27-c-to-semantic-ir.md:14-19`): it keeps a symbol table of `name -> IntSpec`, computes integer promotion and the usual arithmetic conversions (`code/packages/rust/c-to-semantic-ir/src/lower.rs:4-6,181-207`), and marks 32-bit signed `int` with `Overflow::Undefined` (`lower.rs:100-101`). So the minimal type pass is **not new code in this tool**; it is *reusing* that lowering:

* **Proposal (LGE-A1-c-sem).** Run `c-to-semantic-ir` on the token-input path and analyse the resulting `semantic_ir::Module`: every signed operation whose result spec has `Overflow::Undefined` must be covered by an interval pass over the validated operand ranges (`a in {0,1}` after the range check). That is `c.integer-value-range`, and with a "no unchecked operation" sweep it is `c.no-ub-reachable`. The SIR view makes the semantic content of C (width, signedness, wrap or undefined) explicit, which is exactly what those rules talk about.
* **What this still would not cover**, and what a reader must not infer: it covers only the integer-core subset (the same subset the lowering accepts); it says nothing about code the subset cannot parse (those files are exit `2`); and it inherits the correctness of the lowering, which is a compiler front end with its own bugs. The rule's report line must say `S4 over SIR27 lowering`.
* **Until then both rules are `planned` with these reasons**: no type information in the tree; no value-range pass; the lowering has not been audited as a source of analyzer facts. VER15a's rows that require these rules stay `open`, and the release text says so (VER15a section 7).

### 9.4 What blocks any C rule from being `live` on the real C package

The C logic-gates package (PR #19926, not on main) is written in real C with a public header (VER15a section 5.3 cites `include/logic_gates.h:46-58`, `src/logic_gates.c:22-93`). The grammar cannot parse a header with prototypes (`translation_unit = { function_def }`, `c.grammar:28`), `static inline` functions, or `#include` guards without a grammar extension. So the C slice is, in order:

1. **LGE-A1-c-g**: extend `c.grammar` with prototype declarations (`type_spec NAME ( param_list ) ;`) and nothing else, and decide, by *measuring the package*, whether the real sources parse. If they use anything else (for example `static`, `const`), either extend the grammar by exactly that keyword or require the package to drop it; each addition is a grammar PR with its own negative controls.
2. **LGE-A1-c-r**: the `active` rules of the table above.
3. **LGE-A1-c-sem**: the SIR27-based semantic rules.

---

**State rule (M5).** `c.no-pointer-array-heap.a`, `.b` and `c.no-shift-operators` are `claimed`, never `live`, until the C package parses end to end under the grammar in CI; `live` requires `completeness: full` and a full parse of every library file.

## 10. Per-lane rollout

"Grammar" says whether `.tokens` and `.grammar` exist. "Version gap" compares the grammar with the version the package targets (section 2.4). "First rules" are the rules a gate library needs first: **no I/O, no network, no process, no environment, no clock, no randomness, no threads, no FFI, no eval or reflection, no unsafe/asm** (the Spec 13 categories a pure bit-operation library must not touch), implemented as the import allowlist plus the banned lists, at the level each mode allows.

| Lane | Grammar | Parser crate | Version gap | Mode first | First rules |
|---|---|---|---|---|---|
| java | yes (`java21`) | `java-parser` | none (21 = 21) | AST | import allowlist (none beyond in-repo); ban `native`; ban `Runtime`, `ProcessBuilder`, `System.exit/getenv/getProperty/load*`, `Class.forName`, reflection names, `sun.misc.Unsafe`, `jdk.internal`, `java.io/nio/net`, `Thread`, `Random`/`SecureRandom`, `ObjectInputStream`. Unicode-escape normalisation negative control (`System`). |
| csharp | yes (to `csharp12.0`) | `csharp-parser` | **C# 13 vs 12**: set `version_gap`; list C# 13 constructs with controls | AST | `using` allowlist; ban `unsafe`, `fixed`, `stackalloc`, `DllImport`, `System.IO/Net/Diagnostics`, `Environment`, `Process`, `Reflection`, `Activator`, `Assembly.Load*`, `Marshal`, `Thread`/`Task` creation, `Random`/`DateTime.Now`. |
| fsharp | yes (to `fsharp9`/`fsharp10`) | `fsharp-parser` | none (F# 9) | AST | `open` allowlist; ban `NativePtr`, `Marshal`, `DllImport`, `System.IO/Net/Diagnostics`, `Reflection`, `inline IL`, `printf*` (stdout), `Environment`. |
| haskell | yes (`haskell2010`) | `haskell-parser` | none | AST | `import` allowlist (`Prelude`, `Data.Bits`, `Data.Word`, `Data.Int`); ban `System.IO*`, `System.Process`, `System.Environment`, `Network.*`, `Foreign.*`, `unsafePerformIO`, `unsafeCoerce`, `Debug.Trace`, `Control.Concurrent`; ban `foreign import`; `CPP` and `TemplateHaskell` pragmas are `CAP003`. |
| perl | yes (unversioned and per-release) | `perl-parser` | **5.26 has no grammar**; parser is a "subset" (`perl-parser/src/lib.rs:46-48`) | AST, with real risk it does not parse the package | `use`/`require` allowlist; ban `system`, `exec`, backticks, `qx`, `open`, `opendir`, `unlink`, `eval` (string form), `require` of a variable, `%ENV`, `time`/`localtime`, `rand`/`srand`, `fork`, `socket*`, `syscall`, `*{"..."}` symbolic references and `&{"..."}`/`$::{...}` stash access, `no strict 'refs'`. Measure whether the package parses before promising anything. |
| ruby | yes (to 3.3.7) | `ruby-parser` | **3.4 vs 3.3**; lexer for the legacy path is a TOML state machine, not the `.tokens` file (`ruby-lexer/src/lib.rs:3-4`) | AST via the **release API** (`parse_ruby_at`), nearest release | `require` allowlist; ban Spec 13's Ruby list (`eval`, `instance_eval`, `class_eval`, `send`/`public_send` with a non-literal, `method_missing`, `const_get`, backticks, `system`, `exec`, `%x{}`; `13-capability-security.md:674-687`), `File`, `IO`, `Dir`, `Kernel#open`, `ENV`, `Time.now`, `Random`, `Thread`, `Fiddle`, `Ractor`. |
| c | **integer-core subset only** | `c-parser` | **C17 vs integer-core**; real headers do not parse | AST with `grammar-is-allowlist` | section 9.2. |
| cpp | **none** | none | - | tokens-only first | `#include` allowlist (`<cstdint>`, `<climits>`, package header); every `#define` except an include guard, `##`, and `#` stringizing are `CAP003`; ban `new`, `delete`, `malloc` family, `reinterpret_cast`, `const_cast`, `asm`, `__asm__`, `goto`, `<iostream>/<fstream>/<cstdio>/<thread>/<chrono>/<random>/<cstdlib>`, `std::system`, `getenv`, `extern "C"`, `volatile`, `__attribute__`; raw-string lexing negative control. C++-specific memory rules (VER15a's `LGE-CPP-EXT`) need S3/S4 and are `planned`. |
| kotlin | **none** | none | - | tokens-only first | `import` allowlist (none); ban `external`, `@JvmStatic`-free FFI via JNI, `java.*`/`javax.*`/`jdk.*`/`sun.*`/`kotlin.io`/`kotlin.system`/`kotlin.concurrent`/`kotlinx` as dotted prefixes *and* imports (Java names are usable without an import); ban `Class.forName`, `::class.java`, `kotlin.reflect`, `System`, `Runtime`, `println`/`readLine`; backtick-identifier and string-template negative controls. |
| elixir | **none** | none | - | tokens-only first | ban `System.*`, `File.*`, `IO.*`, `Port`, `Node`, `Code.*`, `Module.concat`, `apply/2,3`, `spawn*`, `Task`, `Agent`, `:os`/`:erlang`/`:file`/`:inet` and every `:atom.` call, `defmacro`/`quote`/`unquote` (`CAP003`), `import`/`use`/`require` allowlist; sigil and heredoc and `#{}` interpolation lexing controls. |
| lua | **none** | none | - | tokens-only first | ban `os`, `io`, `debug`, `package`, `require`, `load`, `loadstring`, `loadfile`, `dofile`, `_G`, `_ENV`, `getfenv`, `setfenv`, `rawget`/`rawset` on globals, `collectgarbage`, `coroutine` (thread), `string.dump`; long-bracket string and comment lexing controls. A Lua global is a table lookup, so this closes the *known* routes to a capability, not every route (section 13.2). |
| swift | **none** | none | - | tokens-only first | `import` allowlist (none beyond the stdlib); ban `Foundation`, `Glibc`, `Darwin`, `Dispatch`, `FileManager`, `Process`, `ProcessInfo`, `CommandLine`, `exit`, `print`, `readLine`, `@_silgen_name`, `@_cdecl`, `unsafeBitCast`, `Unsafe*Pointer` names, `withUnsafe*`, `Mirror`, `NSClassFromString`, `@dynamicMemberLookup`/`@dynamicCallable`; raw string `#"..."#` and `\( )` interpolation lexing controls. |
| dart | **none** | none | - | tokens-only first | import URIs are string literals and so lexically exact: ban `dart:io`, `dart:ffi`, `dart:mirrors`, `dart:isolate`, `dart:html`, `dart:developer`, `package:` outside the in-repo allowlist, conditional imports (`if (dart.library.*)`), `part`/`export` unless allowlisted; ban `print`, `exit`, `Process`, `Platform`, `DateTime.now`, `Random`; `$x`/`${}` interpolation and raw string controls. |

### 10.1 The order inside each grammar-less lane

For each of cpp, kotlin, elixir, lua, swift and dart, in this order (VER-BACKLOG decision: "lanes without a grammar get `.tokens` first (a tokens-only mode) then `.grammar`"):

1. A `.tokens` file in `code/grammars/<lang>/`, validated by `grammar-tools` (`cross_validate` with no parser grammar is fine for tokens-only), with a lexer crate generated by the same compiler step the other lanes use (`code/packages/rust/c-lexer/src/_grammar.rs` is the compiled output for C), and a corpus of tricky lexical inputs (the lexer assumptions of section 5.3). The lexer is **not** hand-written.
2. The tokens-only rule table for the lane (S0 and S1 rules only).
3. Later, a `.grammar` file and a parser crate, after which the lane's table moves to AST mode and gains S2 rules.

---

## 11. Negative controls (a rule cannot pass vacuously)

A rule that never fires looks identical to a rule that is correct and has nothing to find. The only defence is to demand that every rule fires on something.

* **Every `active` rule has a control fixture**: a small source file, in the lane's language, that contains exactly the banned construct. The loader refuses an active rule with no `control` path or a path that does not exist.
* **The tool runs the controls itself**, in the same process and the same run as the real analysis, before reporting. A control that does not trigger its rule is exit `2` ("rule X did not fire on its own control"), and the JSON report records `control: triggered` per rule (section 7.2).
* **A control triggers its own rule only**, asserted by running every control against the whole rule set and checking the findings match exactly the rule the fixture names (a control that triggers three rules would hide a dead one).
* **Controls exist for the lexer assumptions too** (section 5.3): a banned word inside a comment, inside a string and inside a nested comment must *not* trigger (positive controls), and the same word as code must (negative controls). For each lane's interpolation, heredoc and raw-string forms there is a pair.
* **Controls exist for each version-gap construct** (section 8.3): the tool must reject or flag it.
* **A "no rules removed" check**: the rule table's content hash is part of the report. A pull request that deletes a rule changes the hash and the control manifest, so removal is visible in review (a control file nobody references is also an error).
* **A lane with zero active rules is exit `2`**, not a clean pass.

---

## 12. Testing the tool itself

| Layer | What is tested |
|---|---|
| Unit | rule-table loader (every refusal: duplicate id, unknown pair, missing control, S2 rule in tokens mode, `planned` without `planned_item`); token and node matchers; the report formatters, including the exact text lines of section 7.1 |
| Fixture corpus | per lane: clean package, one package per rule that must trigger it, one per lexer assumption, one per version-gap construct. A language-neutral JSON of cases like `code/specs/fixtures/ocaml-capability-analyzer-v1/cases.json` (`OCAML06-capability-analyzer.md:167-176`), so another implementation could be checked against it |
| Property | for any source, adding a banned token inside a comment or string never changes the verdict; shuffling the order of files never changes the verdict (deterministic output) |
| Fuzz | the lexer-plus-parser path with adversarial inputs (deep nesting, huge tokens, unterminated strings and comments, invalid UTF-8, NUL bytes) must return an error or a report, never panic, hang or abort |
| End to end | run the tool on every in-repo logic-gates package of its lane; the expected verdict is recorded; a change in verdict is a review item |
| Coverage | 95% or more on the library (CLAUDE.md standard 11) |

---

## 13. Security of the analyzer itself

The tool reads **untrusted source**: a malicious package author wants the tool to say "clean" or to crash or hang the build.

### 13.1 Rules

* **Bounded recursion.** Every parser is built with `with_max_depth(...)` (`grammar_parser.rs:536`); `GrammarParser::new` alone leaves the depth unbounded (`grammar_parser.rs:525`). Each lane crate passes its own cap (the C crate uses the engine's `DEFAULT_MAX_RULE_DEPTH`, `c-parser/src/lib.rs:51`; the Java crate uses 180, `java-parser/src/lib.rs:42`; the Perl crate uses 96, `perl-parser/src/lib.rs:33`). The tool calls only the lane crates' constructors, so a depth-exhaustion refusal is a parse error, exit `2`. The tool's own tree walks are **iterative with an explicit stack**, not recursive, so a tree the parser accepted cannot overflow the analyzer's own stack.
* **Bounded work.** Maximum tokens per file, bytes per token field and total token text, as `c-parser` already enforces (`MAX_INPUT_TOKENS = 2_000_000`, `MAX_TOKEN_FIELD_BYTES = 64 KiB`, `MAX_TOTAL_TOKEN_TEXT_BYTES = 64 MiB`; `c-parser/src/lib.rs:52-54`), plus VER08a's file-count and size ceilings (section 4). A packrat parser's memo table is bounded by the token budget; the tool refuses over-budget input before parsing, not after.
* **No `eval`, no shell, no dynamic loading, no network.** The tool never runs, compiles, imports or evaluates the analyzed source, never invokes a compiler or a package manager, and never executes anything named by the package (build scripts, `Cargo.toml` `build`, Gradle scripts, `Makefile.PL`). It does not follow symlinks.
* **No regular expressions over source text.** Token and node matchers compare token `type_name` and exact `value`. Rule tables have no regex field, which removes a ReDoS class (compare the TypeScript analyzer's CWE-1333 work, `typescript/ca-capability-analyzer/README.md`) and a whole family of escaping mistakes.
* **Fail closed.** Parse failure, over-budget input, a missing grammar for the declared version, a rule table that fails its schema, a control that does not fire: all exit `2`. There is no "skip the file with a warning" path (the OCaml analyzer's stated rule, `OCAML06-capability-analyzer.md:69-71`).
* **The rule table is trusted, the package is not.** The table is embedded in the binary and hashed into the report; the package cannot supply or override one. Build-system files that could change what the analyzer sees (Gradle `sourceSets`, `.csproj` `Compile Include`, `cabal` `hs-source-dirs`, `Makefile.PL`'s generated files, Ruby `$LOAD_PATH` edits) are listed per lane as `CAP003` triggers, the equivalent of VER08a's `Cargo.toml` rules (`VER08a...md:184-195`), so a package cannot steer the tool away from the code that ships.
* **Rule-table governance.** See section 17.4.
* **Own capabilities.** The tool's manifest declares reading the package and rule tables and writing stdout, and nothing else; the existing Rust analyzer runs over this crate like any other.

### 13.2 Honest limits

* **Names are not authority.** An S0/S1/S2 rule over names catches the *spelling* of a capability. A language that can build a name at run time (Lua `_G["o".."s"]`, Ruby `send(name)`, Java reflection, Elixir `apply/3`, Perl symbolic references) is closed by banning the *primitive that builds the name*. That removes the known routes; it is a deny-list of primitives, not a proof that no route exists, and every such rule says so in its `message`.
* **A third-party dependency is a hole in the analysis.** The tool does not look inside imported packages; it lists them. A pure gate package may import only in-repo packages that have their own passing report (section 3.3).
* **A grammar bug is a verdict bug.** The tool is as trustworthy as the grammar it reads. Grammars in this repository are written from language releases and tested against corpora (for example `code/grammars/perl/README.md`, `code/grammars/ruby/README.md`), but a grammar that parses something as the wrong construct would let the tool miss it. The negative controls and the per-lane "does the real package parse" measurement are the only guard.
* **Tokens-only mode is weaker, and the report says so**: `mode: tokens` is printed on every finding line and in the JSON, and a lane row built only on tokens-only rules says "no occurrence of these names", not "pure".
* **No claim about the caller or the compiler.** Compiler bugs, runtime bugs, build-system tampering outside the scanned set, and anything the manifest's `target` field was meant to scope (gap G-3, `VER08a...md:262`) are out of scope.
* **It checks capabilities and a short list of banned constructs.** It does not find logic bugs, and a `PASS` is not a security audit.

---

## 14. Work items

These replace the single LGE-A1 and LGE-A2 rows in the backlog (`VER-BACKLOG.d/logic-gates-everywhere.md`):

| ID | Work | Blocked on |
|---|---|---|
| LGE-A0 | **Prerequisite: engine fix LGE-PARSER-DEPTH** (section 16.4). Schema and shared fixtures: `rule-table.schema.json`, report schema `ca-grammar-analyzer-report-v1`, extend `banned_construct_exceptions[].language` only if a package needs an exception | this spec merged |
| LGE-A1-core | The `ca-grammar-analyzer` crate: rule-table loader, token and node matchers, report text and JSON, bounds, the negative-control runner, CLI, own manifest, tests | LGE-A0 |
| LGE-A1-java | Java rule table, controls, BUILD line, protected-gate row (the pilot lane: grammar matches the package version) | LGE-A1-core |
| LGE-A1-fsharp, LGE-A1-haskell | rule tables and controls | LGE-A1-core |
| LGE-A1-csharp | rule table and controls with the C# 13 version-gap list | LGE-A1-core |
| LGE-A1-perl | measure whether the package parses under the nearest grammar; rule table and controls; a grammar PR if it does not | LGE-A1-core |
| LGE-A1-ruby | decide 3.4 versus nearest-3.3 (grammar PR or version gap); rule table and controls | LGE-A1-core |
| LGE-A1-c-g | `c.grammar` header extension (prototypes only), decided by measuring the real package | C package merged (PR #19926) |
| LGE-A1-c-r | the active C rules of section 9.2 | LGE-A1-core, LGE-A1-c-g |
| LGE-A1-c-sem | SIR27-based `c.integer-value-range` and `c.no-ub-reachable` | LGE-A1-c-r |
| LGE-A2-tokens-cpp / -kotlin / -elixir / -lua / -swift / -dart | `.tokens` file, lexer crate, lexical corpus, per lane (six PRs) | none besides `grammar-tools` |
| LGE-A2-rules-cpp / -kotlin / -elixir / -lua / -swift / -dart | tokens-only rule table and controls, per lane | LGE-A1-core and that lane's tokens PR |
| LGE-A2-grammar-<lane> | `.grammar` and parser crate; lane table moves to AST mode | the lane's rules PR |
| LGE-A3-gate | the independent protected CI gate that runs the tool over every package of the 13 lanes (the OCaml pattern, `OCAML06-capability-analyzer.md:188-191`) | LGE-A1-core |
| LGE-W1 | `ca-assurance-report` reads the JSON of section 7.2 and promotes passing rules to `live` evidence (owned by VER15a section 10) | LGE-A1-core |

Order of attack: LGE-A0, LGE-A1-core, LGE-A1-java (the only lane where the grammar already matches the shipped version, so it validates the tool end to end), then the other grammar lanes in parallel with the six `.tokens` PRs.

---

## 15. Risks and open questions

1. **No consumer exists yet** (section 2.2). The brief expects an output `ca-assurance-report` can consume; it consumes none today. The JSON contract here is a proposal that LGE-W1 must adopt or amend. Until then the rule evidence is only as visible as a CI log.
2. **The C pipeline is thinner than the C rules assume** (section 9). The grammar is an integer-core subset, headers do not parse, the legacy parser API deletes `#` lines (so an include or a `#define system exit` would be invisible if the wrong entry point were used), the preprocessor is not a finished C preprocessor, and the type-dependent rules rest on a compiler front end that was written for lowering, not for analysis. VER15a's C rows will stay open for a while, and this spec says so rather than promising.
3. **Grammar age and fidelity.** C# 13 against a C# 12 grammar, Ruby 3.4 against 3.3, Perl 5.26 with no grammar, a Ruby lexer that is not `.tokens`-driven, and PEG ordered choice that can mis-parse newer syntax instead of failing. All are mitigated by fail-closed parsing plus per-construct controls, none is eliminated.
4. **Tokens-only lanes see strings and macros last.** Interpolation, heredocs, macros and the preprocessor are where a token scanner is most easily fooled; the design makes them `CAP003` unless the `.tokens` models them, which costs coverage in Kotlin, Swift, Dart, Elixir and C++ first.
5. **Decided (coordinator, 2026-11-24):** the central protected gate (LGE-A3) is the only source LGE-W1 consumes for `live`; lane `BUILD` lines may run the tool but never promote a rule.
6. **Decided:** `closed-world-identifiers` (or an enforced explicit prelude-export allowlist) is mandatory for every lane with ambient builtins; until it is live those lanes' rules are labelled "deny-list, claimed" (section 16.2).

---

## 16. Fail-closed amendments (normative)

These sections respond to the design and security review. Where they contradict earlier text, they win.

### 16.1 Scope: every regular file is classified (H1)

The tool classifies **every regular file under the package root** into exactly one of: `analysed` (extension and path in the lane table), `nonsource` (matched by an explicit path or glob **with a reason, supplied only by the per-lane table, never by the package**, and subject to section 17.7; for example `README.md: documentation`, `tests/**: test path, not shipped`), or `unknown`. Any `unknown` file is exit `2`. A shipped source-like file outside `source_roots` that is not on an allowlisted test path is `CAP003` (not a `NOTE`). The report carries `files_seen`, `files_scanned` and `files_nonsource`, and the run fails (exit `2`) if `files_scanned` is `0` or below the lane's `min_files` floor. Controls: a banned construct planted in a file with an unlisted extension, and one in a source file outside the roots; both must be refused.

### 16.2 Version gaps, ambient preludes and the closed world (H2, H4, decision b)

* **Version gaps (H2).** A lane with a non-null `version_gap` has no `live` rule. It may leave `claimed` only when (i) the grammar matches the target version, or (ii) a corpus-differential check runs in CI comparing this tool's parse with the target-version reference parser (javac, Roslyn, `ruby -c`, `perl -c`, ...) on the lane's corpus. Each such lane also has a per-lane **post-grammar-syntax corpus** (constructs newer than the grammar) proving parse failure yields exit `2`; a file the PEG accepts as something else is the residual risk, and the differential check is the control for it.
* **Ambient preludes (H4).** Every lane **except c and cpp** (section 17.1) **must** enable `closed-world-identifiers` or an enforced explicit prelude-export allowlist (the set of prelude names the package may use). Until that rule is `live`, the lane's other rules are labelled `deny-list, claimed`, shown as such in the report, and never `live`. Dart additionally uses a `dart:` library allowlist (only `dart:core` names on the allowlist, no other `dart:` import; `String.fromEnvironment` and `-D` defines are refused, section 17.6).

### 16.3 Build files and build-system steering (M3)

*Extended by section 17.3: build files are executable code and are scanned or schema-matched, and dependencies are gated. The triggers named here and in 17.3 are one list:* additionally, per lane, the executable build files are: Java and Kotlin Gradle `*.gradle.kts`/`settings.gradle.kts` (annotation processors, compiler plugins); C# `*.csproj`, `Directory.Build.props` and `Directory.Build.targets` (MSBuild `<Exec>`, targets, `ImplicitUsings`); F# `*.fsproj` and the same `Directory.Build.*`; Haskell `*.cabal` and `Setup.hs`; Ruby `*.gemspec`, `Rakefile` and `Gemfile`; Perl `Makefile.PL` and `cpanfile`; Elixir `mix.exs`; Lua `*.rockspec`; Swift `Package.swift`; Dart `pubspec.yaml` and `build.yaml`; C and C++ `Makefile`, `CMakeLists.txt`; and the repository `BUILD` file. Each is schema-matched or scanned (17.3); an unrecognised build file is exit 2.

The rule-table schema gains a `build_files` section: per lane, the build files, the keys that change compiled code or flags, and a CAP003 trigger for each. An unrecognised build file in the package is exit `2`. Minimum triggers: C/C++ `-D`, `-include`, `-I`, CMake and Makefile (any custom command or flag); Elixir `mix.exs` `elixirc_paths`, `compilers`; Lua rockspec `build.modules`; Swift `Package.swift` plugins and `unsafeFlags`; Dart `build.yaml`; Kotlin compiler plugins; Java annotation processors; Perl `-M` and `PERL5OPT`; plus the Gradle, `.csproj`, cabal and Ruby `$LOAD_PATH` items of section 13.1.

### 16.4 Parser depth flag is not trusted to surface (H3)

The engine reports a depth refusal only when the entry rule returned `None` (`code/packages/rust/parser/src/grammar_parser.rs:617-627`); the sticky `depth_exceeded` flag is not consulted on `Ok`, so a deep alternative that is cut off can be backtracked past and the parse can still return `Ok` over a tree missing what the deep branch held. The tool therefore treats `depth_exceeded` as a **hard error (exit `2`) whenever it is set, even when `parse()` returned `Ok`**. This needs access to the flag; the engine fix (item **LGE-PARSER-DEPTH**) is an LGE-A0 prerequisite. Fixture: an input nested past the lane's cap whose shallow alternative would succeed; the tool must exit `2`.

### 16.5 C rule split and completeness (M5)

Rules carry `covers` and `completeness: full|partial` mapping onto VER15a clauses (section 9.2). `live` needs `completeness: full` **and** end-to-end parse of the package. The C rule ids are split `.a` / `.b` as in section 9.2; both and `c.no-shift-operators` stay `claimed` until the C package parses.

### 16.6 Reflection and dynamic dispatch: ban outright (H5)

* **Ruby:** ban, with no literal-argument exception, `send`, `public_send`, `__send__`, `instance_exec`, `class_exec`, `module_eval`, `method`, `define_method`, `binding`, `ObjectSpace`, `Marshal`, `autoload`, `Open3`, `IO.popen`, `exit!`, `abort` (in addition to section 10's list), and any **symbol or string token naming a banned method** (`:system`, `"eval"`). This replaces Spec 13's "`send` with a non-literal argument" wording for these lanes.
* **Java:** ban reflection, `ClassLoader`, `MethodHandles`, `ServiceLoader`, `ScriptEngine`, `java.lang.invoke`, `java.lang.foreign`, and string tokens naming banned classes (`"java.lang.Runtime"`).
* The same "symbol or string token naming a banned name" control applies to every lane with reflection by string (Kotlin, C#, Perl, Elixir, Lua `_G`).

### 16.7 Summary honesty (M7)

See section 7.1: `absent` needs an S2+ rule; otherwise `absent-by-name (S0)` or `n/a`; dynamic constructs are never `absent` from name rules alone.

### 16.8 Lexer evasions every lane must refuse (M1, L1)

* **C/C++:** line splicing (backslash-newline) inside tokens and directives, digraphs and trigraphs, universal character names (UCNs), `_Pragma`, `#include MACRO`, `#include_next`, `__has_include`, `#embed`, `#line`, and any `#pragma` except `once`. The cpp row is exactly as strict as the C row.
* **Every lane:** non-ASCII or non-normalised identifiers are `CAP003` by default. Java unicode escapes are normalised or refused.
* **C#:** `@verbatim` identifiers (normalised so `@System` is `System`). **Perl:** source filters, `__DATA__`, `__END__`, POD. **Ruby:** `__END__` and `DATA`. **Elixir:** sigils, `?x` character literals, `@on_load`, dynamic `mod.fun()` calls.
* **Any ERROR or unknown token from the lexer is exit `2`** (explicit; the `errors:` section of a `.tokens` file, such as the `UNKNOWN` token of `c.tokens`, must not be skipped).
* **Nested interpolation (L1):** tokens inside string interpolation go through the same S0/S1 rules; control: a banned word inside `"${ ... }"`, `"\( ... )"` or `"#{ ... }"` triggers.

### 16.9 Import rules need token-sequence companions (M2)

Every `import-prefix` rule is paired with a `token-sequence` rule for the dotted prefix, covering fully-qualified use without an import, wildcard imports and Java `import static`, each with its own control.

### 16.10 In-repo dependencies (M4)

An allowlisted in-repo import is resolved to a package path; the run requires **that package's passing gate report hash** in the same run, otherwise `CAP003`. A prefix match alone never admits a dependency.

### 16.11 Controls are embedded and hashed (M6)

The control fixtures are embedded in the binary and hashed into the report (`controls_sha256`). Only the gate's report is consumed (section 8.2 item 5).

### 16.12 Input hygiene and engine helpers (L2-L5)

* **L2:** symlinks, hardlinks, FIFOs, devices and files that change while being read are exit `2`; reads are size-limited on the bytes actually read, not on a prior `stat`; invalid UTF-8 is exit `2`.
* **L3:** the tool must not call recursive engine helpers such as `expanded_tree_size` (`grammar_parser.rs:1654`) on untrusted trees, or must bound them; its own walks are iterative (section 13.1).
* **L4:** the Perl cap of 96 (`perl-parser/src/lib.rs:33`) is low; legitimate deeply nested Perl will be refused with exit `2`, an operational cost that is accepted.
* **L5:** `add_pre_parse` hooks (`grammar_parser.rs:550`, run at `:595-601`) can rewrite the token list before parsing. The tool forbids any pre-parse hook that is not part of the lane crate, and runs all S0/S1 rules on the **lexer's output before any hook**.

---

## 17. Security review round 2: fail-open holes closed (normative)

### 17.1 Ambient preludes: every lane but c and cpp (M1)

`closed-world-identifiers` (or an enforced explicit **prelude-export allowlist**) is mandatory for **every lane except c and cpp**. Each lane's rule table names that lane's implicit-import sources, each a `CAP003` unless allowlisted:

| Lane | Implicit sources the table must name |
|---|---|
| csharp | `ImplicitUsings` enabled in the project, and any `global using` (either is `CAP003` unless allowlisted) |
| fsharp | the FSharp.Core auto-open set (`stdin`, `stdout`, `printfn`, `exit`, `async`, ...) |
| kotlin | default imports (`kotlin.io.*`, `java.lang.*`, ...) |
| swift | implicit `_Concurrency` and `_StringProcessing`, plus the stdlib |
| haskell | the Prelude exports `readFile`, `writeFile`, `getLine`, `interact` (and `putStr`, `getContents`); Haskell is safe **only** with closed-world live |
| java, ruby, lua, perl, dart, elixir | `java.lang`, `Kernel`/`Object`, the base library, builtins, `dart:core`, Elixir `Kernel` |

Until the lane's closed-world rule is `live` its other rules are `deny-list, claimed`.

**Soundness of the prelude-export allowlist alternative:** an explicit prelude-export allowlist is **S2**, and it can be `live` only in AST mode with **resolved imports**, meaning every imported name is bound to a declared module export, with no wildcard import and no dynamic import. Otherwise only a live S3 closed-world rule qualifies.

### 17.2 Tokens-only mode is capped at `claimed` (M2)

`mode: tokens` means **no rule can be `live`** and the lane is capped at `claimed`; it can never, on its own, back a `required_capabilities: []` claim. The PASS line reads `PASS (tokens-only, deny-list, claimed)`. Closed-world is S3 (section 5.2) and stays `planned` until a scope pass exists. Because tokens mode cannot resolve scope, it **rejects any declaration whose name collides with an ambient or denylisted name** (`local print = print`, a parameter named `os`, shadowing): the collision is a `CAP003`. This rejection applies **in AST mode as well as tokens mode**: no scope pass exists in either until S3 lands, so a package may not declare a name that collides with an ambient or denylisted name. As the tokens-mode alternative to closed-world, a lane may use a **flat identifier-token whitelist**: every identifier token in the package must be in the lane's whitelist; anything else is `CAP003`. The whitelist is still `claimed`, not `live`.

### 17.3 Build files and dependencies are code (M3)

Executable build files must either **match a per-lane allowlisted declarative schema** (exact shape, no scripting) or be **scanned under the lane's rules with restricted imports**. This covers Gradle `.kts`, `Package.swift`, `Makefile.PL`, `mix.exs`, rockspec, gemspec, `Rakefile`, `Setup.hs`, `.csproj`, `Directory.Build.*` (MSBuild `<Exec>` and targets), `pubspec.yaml` and `build.yaml`. The repository `BUILD` file is in the classified set (section 16.1) and is held to the same rule. Any dependency that is **not in-repo with a pinned gate-report hash** is `CAP003`. `unanalyzed_dependencies > 0` is exit `1`, and `live` requires `0` (this tightens VER08a's warn-only treatment of third-party dependencies, `VER08a...md:199`).

### 17.4 Rule-table governance (M4)

* **CODEOWNERS** (security owners) are required on `code/specs/data/capability-rules/**`, the analyzer crate and the gate workflow.
* The gate runs the **base-branch** analyzer binary and tables against the pull request's package tree, never the head's tables.
* The loader **derives soundness from `kind`** and rejects a table whose declared `soundness` label does not match.
* Tables are **per-lane only**. Per-package tables, overrides, and package-supplied `nonsource` paths are forbidden.
* **L2:** manifest exceptions are CODEOWNERS-approved and are never allowed in the logic-gates gate lanes.

### 17.5 Attested reports (M5)

The report schema carries `commit`, `package_tree_sha256`, `tool_binary_sha256`, `rule_table_sha256` and `controls_sha256` (section 7.2). The `attestation` field must carry a **GitHub OIDC / Sigstore build-provenance attestation over the report's sha256**, whose claims bind: the repository; the workflow path and ref (the base-branch protected gate workflow); the run id; and the `commit` and `package_tree_sha256` of the report. LGE-W1-attest verifies those claims and **rejects** any mismatch, a missing attestation, or a workflow ref other than the protected gate. Section 7.3 defers to section 8.2 as the only promotion path.

### 17.6 Compiler-steering directives: allowlist, not denylist (M6)

Every lane gets an **allowlist of pragmas, attributes and annotations** (`annotation_allowlist` and equivalents in the table); anything else is `CAP003`. Named dangerous cases, each with a negative control:

* **Haskell:** `OPTIONS_GHC -fplugin`, `-F -pgmF`, `QuasiQuotes`, `ForeignFunctionInterface` and `foreign`.
* **Java:** annotation processors, triggered through the build (section 17.3). `@Generated` is the *output marker* of a processor, not its trigger, so it is not a rule.
* **Swift:** `@_extern`, `@_dynamicReplacement`, `@_implementationOnly`, `#externalMacro`.
* **Dart:** `external`, `@Native`, `String.fromEnvironment`, and `-D` defines.

### 17.7 Nothing is skipped; the tracked set is scanned (M7)

The tool scans the **tracked file list** (`git ls-files`), not the working tree; a build-output directory must be untracked. A `nonsource` test path must either be **proven excluded from the published artifact** (Ruby gemspec `files`, cabal `extra-source-files`, csproj globs, Gradle `sourceSets`) or be scanned under the same rules. This resolves the contradiction between section 4 ("skip a fixed list") and section 16.1 ("classify everything"): no directory is skipped, and `nonsource` is a proven, per-lane-table classification, not a skip.

The gate analyses a **clean checkout**: submodules and gitignored files are not scanned, and an untracked file referenced as a build input fails the run. **Submodules are forbidden in analysed package trees**; a submodule entry is `CAP003`, so `package_tree_sha256` covers only tracked blobs of the package and never submodule content.

### 17.8 Reference parsers and version-gap output (L1, L3)

* **L1:** the corpus-differential reference parsers (`perl -c` runs `BEGIN` blocks; javac and Roslyn run annotation processors and source generators) run **only on the lane corpus, never on package code**, sandboxed with no network.
* **L3:** a lane with a non-null `version_gap` prints `PASS (claimed: version gap)` (section 7.1).

### 17.9 New work items

| ID | Work |
|---|---|
| LGE-A0-codeowners | CODEOWNERS entries for the rule tables, the analyzer crate and the gate workflow |
| LGE-A3-base-gate | Gate runs the base-branch binary and tables on the PR tree; emits the attested report |
| LGE-A1-closed-world | `closed-world-identifiers` and the prelude-export allowlists per lane (S3, `planned` until the scope pass) |
| LGE-A1-buildfiles | Per-lane build-file schemas and scanning, repo `BUILD` in the classified set, dependency pin check |
| LGE-A1-directives | Per-lane pragma, attribute and annotation allowlists with controls |
| LGE-W1-attest | LGE-W1 verifies commit, tree, tool and control hashes against the gate attestation |
