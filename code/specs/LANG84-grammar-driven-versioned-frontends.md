# LANG84 — Grammar-driven, release-versioned language frontends

**Status:** Draft, 2026-10-20. Umbrella contract for every IIR frontend.
LANG84 sits under LANG78 and generalizes LANG82's per-release grammar rule to
every language. It depends on LANG83, which defines the IIR operations that
frontends emit and backends lower.

## Goal

In the owner's words:

> "we should be using tokens and grammar files. ... go from simply defining a
> tokens and grammar file and we should get everything from lexer, parser and
> then ... AOT, JVM, CLR, WASM, etc working".

> "I am happy to write a custom IIR compiler for each. But the input to that
> IIR compiler is the parsed input that is generated from a parser that takes
> in grammar and tokens file".

> "we need to support from the earliest version of every language. From 1.0
> all the way to the end." The release unit is "every tagged release", as in
> Perl's 776-row inventory.

The following rulings are binding. LANG84 turns them into a checkable contract:

1. **No declarative lowering DSL.** There are no `.lower` files. Each language
   has one hand-written IIR compiler. Its only input is the parse tree produced
   by the generic grammar-driven lexer and parser from `<lang><release>.tokens`
   and `<lang><release>.grammar`.
2. **Hand-written lexers are not allowed.** Ruby's lexer is ported to
   `ruby<release>.tokens`. Small Rust pre- and post-tokenize hooks around
   `GrammarLexer` are allowed (see [Allowed hooks](#c3-allowed-hooks)).
3. **The IIR compiler walks the parse tree.** No hand-written code may convert
   the CST into an owned typed AST before IIR compilation. Twig, McCarthy Lisp
   and JavaScript are rewritten to consume `GrammarASTNode` directly.
4. **Every tagged release of every language is covered.** Grammars are written
   earliest release first, the same practice as LANG82. IIR compilers pick up
   each release once its grammar lands.
5. **There is no portable runtime.** Frontends emit LANG83 registry operations,
   and every backend lowers those operations to its own instructions.

## Pipeline

```text
code/grammars/<lang>/<lang><release>.tokens ─┐
code/grammars/<lang>/<lang><release>.grammar ─┤  (compiled into _grammar.rs,
                                              │   deduplicated by content)
source ──► [allowed pre-tokenize hooks] ──► GrammarLexer ──► [allowed post-tokenize hooks]
       ──► GrammarParser ──► GrammarASTNode (CST)
       ──► <lang>-iir-compiler (hand-written; walks the CST; reads a ReleaseProfile)
       ──► IIRModule using LANG83 registry ops
       ──► lang_aot::check_module_for(backend)   (LANG83 compile-time refusal)
       ──► x86_64 | aarch64 | LLVM | WASM | JVM | CLR | BEAM | vm-core | JIT
           (each backend lowers every op to its own instructions)
```

Semantic IR is not a stage in this pipeline (LANG78). A host interpreter may
serve as an oracle. It never executes a compiled program.

## Relationship to other specs

| Spec | Relationship |
| --- | --- |
| LANG78 | Umbrella for the dynamic-language frontends. LANG84 is its frontend contract and applies to every language, not only the four in LANG78. |
| LANG79–LANG81 | Executable pilots. Their subsets become the first named subsets (`py-float-print`, `rb-int-puts`, `pl-int-print`) in the release status model. |
| LANG82 | Perl's per-release grammar line. LANG84 generalizes its rules (materialized pair per release, no aliases, partial pairs labelled, source evidence) to every language and extends its `releases.csv` schema. |
| LANG83 | The operation vocabulary. A frontend may emit only registry ops and builtins, plus pre-registry IIR the registry does not gate. A backend lowers each op or refuses it at compile time. |
| F04, F10 | Lexer pattern groups and declarative mode transitions. LANG84's [grammar-tools extensions](#grammar-tools-capability-extensions) extend the same table-driven engine. |
| lexer-parser-hooks.md | Defines pre/post hook signatures. LANG84 restricts which hooks an IIR frontend may register. |
| PREP01 | Generic source preprocessor (MacroOct, MacroNib). See [Preprocessor dialects](#preprocessor-dialects). |
| compiled-grammars.md | `_grammar.rs` generation. LANG84 drives the version list from `releases.csv` and adds content deduplication. |

## The Frontend Contract

The words MUST, MUST NOT, SHOULD and MAY are normative. "Frontend" means the
`<lang>-lexer`, `<lang>-parser` and `<lang>-iir-compiler` crates of one
language, plus that language's arm in `lang-aot`.

### C1. Allowed inputs

1. A frontend MUST obtain tokens only from `lexer::GrammarLexer` running over
   the compiled `TokenGrammar` of exactly one inventory release. Token
   transforms are permitted only through the hooks in C3.
2. A frontend MUST obtain its tree only from `parser::GrammarParser` running
   over the compiled `ParserGrammar` of that same release.
3. The IIR compiler's inputs are the `GrammarASTNode` root, the selected
   release ID, that release's `ReleaseProfile` (static data, see
   [Release profiles](#release-profiles)), the module name, and a
   `FrontendBudgets` value. Nothing else.
4. The IIR compiler MAY decode the value of a single token: digits to an
   integer, escape sequences in a string token, a heredoc dedent already
   delimited by the lexer. It MUST NOT split a token into further syntactic
   tokens. It MUST NOT re-scan source text, and it MUST NOT re-parse a token
   sequence.
5. The IIR compiler MUST dispatch on the **effective token type**
   (`type_name` or `type_`), never on spelling alone. A string token that spells
   `puts` is not a callee (LANG80 lesson).
6. The compiler MAY use **borrowed CST views**: thin accessor types tied to the
   lifetime of a `GrammarASTNode`, such as `struct IfView<'a>(&'a GrammarASTNode)`
   with `fn cond(&self) -> ExprView<'a>`. They MAY be hand-written or generated
   from the `.grammar`. A view MUST NOT allocate an owned tree, re-associate
   operators, fold repetitions into nested boxes, or desugar. Each accessor maps
   onto one grammar rule's children in O(1) or O(children).
7. Analysis passes the compiler needs, such as free variables, type
   annotations and arity, MUST run over the CST as well. They MAY produce
   **side tables** keyed by CST node identity (rule name plus start/end span)
   instead of a second tree.

### C2. Forbidden

A frontend on the IIR path MUST NOT contain any of the following:

- **Hand-written lexers.** No code may scan source characters to produce tokens
  beyond the C3 hooks. A TOML or other state machine interpreted by
  language-specific Rust action code counts as a hand-written lexer, which
  rules out the current `ruby-lexer` design.
- **Hand-written parsers.** No recursive descent, precedence climbing,
  Pratt parsing or retry loop over tokens outside `GrammarParser`. That
  includes a mini-parser inside the IIR compiler that walks a flat token list
  with its own precedence. Precedence and associativity belong in the
  `.grammar`.
- **Typed lifts.** No conversion of `GrammarASTNode` into an owned
  language-specific AST (`twig_parser::Program`, `Vec<LispExpr>`,
  `javascript_ast::Program`) that is then compiled. Such ASTs MAY remain for
  other consumers (twigc tooling, closurec, LSP), but not on the IIR path.
- **Runtime grammar mutation.** No editing of a compiled grammar before use,
  such as `grammar.definitions.retain(|r| r.name != "REGEX")` in
  `tokenize_javascript_division_only`. A different lexical behaviour needs a
  different grammar construct or a different release pair, never surgery.
- **Structural post-parse hooks.** `GrammarParser::add_post_parse` and other
  tree-rewriting hooks MUST NOT be registered on the IIR path.
- **Version comparisons in the compiler.** The IIR compiler MUST NOT parse,
  order or compare release IDs (`if version >= "2.7"`). It branches only on CST
  shape and `ReleaseProfile` fields. A CI grep check enforces this, with an
  allow-list limited to the release-resolution module.
- **Portable-runtime helpers** (LANG83). No IIR helper functions that implement
  language semantics and are compiled like user code. No language-named
  builtins that duplicate a registry op: `rb_int_div` becomes `i64_div_floor`
  and `rb_puts_int` becomes `print_i64`.

### C3. Allowed hooks

A frontend MAY register these hooks:

- `GrammarLexer::add_pre_tokenize` (`String → String`)
- `GrammarLexer::add_post_tokenize` (`Vec<Token> → Vec<Token>`)
- `GrammarParser::add_pre_parse` (`Vec<Token> → Vec<Token>`)

Every registered hook must meet all of the following criteria. Together they
define "small":

1. **Pure and deterministic**, as in lexer-parser-hooks.md principle 1. A hook
   cannot see parser state. The [feedback channel](#e7-goal-directed-lexing-parserlexer-feedback-and-binding-predicates)
   exists for parser-dependent decisions.
2. **Linear time and O(input) memory.** It has no input-dependent recursion
   depth and makes one pass, or a bounded constant number of passes.
3. **Bounded scope.** It handles one fixed, documented concern that the
   grammar formats cannot yet express, such as fixed-column layout, physical
   line-number relabelling, a single comment convention, or newline
   significance inside brackets. It MUST NOT recognize the language's general
   token set.
4. **Size.** At most 200 non-test, non-comment Rust lines per hook, and at
   most 400 such lines across all hooks of one language.
5. **Registered and justified.** Each hook is listed in the table below, with
   the grammar-tools extension that would replace it, if one exists. A new hook
   needs a LANG84 amendment to this table.
6. **Tested.** Positive and negative probes, including hostile-size input
   within the source budget.

| Language | Hook | Stage | Replaceable by |
| --- | --- | --- | --- |
| COBOL-60 | `strip_cobol_columns` | pre-tokenize | E1 column guards (optional) |
| Dartmouth BASIC | `relabel_line_numbers`, `suppress_rem_content` | post-tokenize | none planned; stays a hook |
| Derive, Wolfram | `drop_bracketed_newlines` | post-tokenize | E3 depth-conditional skip |

The hooks listed in the table above are compliant today. When an extension
that replaces a hook lands, the hook SHOULD be removed in a follow-up PR, but
replacing it is not mandatory.

### C4. Release selection and no silent fallback

1. Every compile resolves to **exactly one** release ID from the language's
   `releases.csv`. The resolved ID is recorded in the produced `IIRModule`
   (new `release: Option<String>` field, added by the PR that introduces
   `compile_source_to_iir_at`). It also appears in every diagnostic.
2. An unknown release ID is a typed error (`UnknownRelease`) that lists up to
   five inventory IDs sharing the longest prefix.
3. A known release whose grammar is `pending`, or whose release subsets do not
   include the program's constructs, is a typed error
   (`ReleaseNotAvailable { stage }` or `Unsupported { release, .. }`). The
   frontend MUST NOT substitute a neighbouring release, the unversioned pair,
   or a "closest" grammar.
4. When the caller names no release, the language's `DEFAULT_RELEASE` constant
   applies. It is an exact inventory ID declared in the frontend crate. CI
   asserts that it is a row with a non-empty `iir_status`. The API takes
   `Option<&str>`, and an empty string is an error, not a default. Today
   `python-parser` silently maps `""` to its default; that mapping MUST NOT be
   reachable from the IIR path.
5. Once a language has a versioned pair covering the IIR subset, the IIR path
   MUST NOT use the unversioned `<lang>.tokens`/`<lang>.grammar`. The
   unversioned pair MAY remain for non-IIR consumers until they migrate.

### C5. Budgets for untrusted input

Source programs are untrusted. Grammar files are trusted repository inputs,
validated at generation time. Every frontend MUST enforce a
`FrontendBudgets` value with these defaults, which trusted callers may raise:

| Budget | Default | Enforced at |
| --- | --- | --- |
| Source bytes | 64 KiB (current pilots), up to 16 MiB for trusted callers | before lexing; file runners bound while reading |
| Parser rule depth | measured per language (e.g. Python 190) | `GrammarParser::with_max_depth`, which is required because the default is `usize::MAX` |
| CST nodes and tokens | 16,384 for caller-supplied trees; source-derived trees are bounded by tokens | direct-AST entry points before lowering |
| Text in a caller-supplied CST | 1 MiB aggregate; 64 KiB per field | direct-AST entry points |
| Compiler traversal depth | ≤ parser depth cap; iterative where unbounded by the grammar | IIR compiler |
| Flat operator spine | 512 | before any allocation that folds a repetition (lesson: *check flat operator spines…*) |
| IIR instructions | 1,000,000 | IIR compiler |
| Lexer group/pair stack (E2) | 256 frames | lexer |
| Captured slot length (E4) | 256 bytes | lexer |
| Pending deferred bodies (E5) | 64 per line, 4,096 total | lexer |
| Goal-directed lex calls (E7) | 2 × source bytes + 1,024 | lexer/parser |
| Binding log (E7) | ≤ token count; scope depth ≤ parser depth cap | parser |

A budget breach is a typed `Budget { which, limit }` error. It MUST NOT be a
panic, an abort or a stack overflow. Every frontend MUST carry hostile-input
probes run in a child process, so that an abort is distinguishable from an
error. The probes cover deep nesting, long flat operator chains, unterminated
constructs, and every new lexer extension's stack or queue.

### C6. Error spans

Every frontend error MUST carry the release ID and a 1-based start line and
column. Lexer errors point at the offending character. Parse errors point at
the furthest failure position and list the expected terminals. `Unsupported`
errors name the CST rule and the node's start and end span. Backend refusals
carry function, op and backend (LANG83 `UnsupportedOps`). End positions SHOULD
be included wherever the CST provides them. `Token` currently records only its
start position. Adding a byte offset and an end position to `Token` is a
follow-up and not a prerequisite for this contract.

Error categories (one `FrontendError` enum per frontend, mapped into
`LangAotError::FrontendError`): `Lex`, `Parse`, `Unsupported`, `Budget`,
`UnknownRelease`, `ReleaseNotAvailable`.

### C7. LANG83 operation validation

1. Every emitted instruction MUST be either a LANG83 registry entry (Op or
   Builtin) whose status is not `Later`, or a pre-registry instruction that
   the registry does not gate.
2. A new operation a frontend needs is added to the LANG83 registry first, in
   its own PR, with every backend's `op_support` declaration. The frontend PR
   that emits it comes after.
3. The produced module MUST pass `interpreter_ir` module validation. The
   frontend's own tests MUST run `lang_aot::check_module_for` for all nine
   backends on the subset corpus. A `Refused` result is acceptable at stage
   B(S) only as a compile-time refusal (see [Status stages](#status-stages)).
4. Trap kinds come only from `interpreter_ir::exception_kind`. Language kinds
   are added there by the PR that first emits them (LANG83).

### C8. CST contract per release

The IIR compiler depends on rule names and child shapes, so each language
keeps those names stable across releases. A rename counts as a breaking change
to the compiler. Each IIR compiler crate MUST carry a `cst_contract` test. For
every release whose `iir_status` is non-empty, the test checks that every
rule name the compiler dispatches on exists in that release's compiled
grammar. Shape assertions run on the subset corpus.

## Versioning

### Every tagged release

The release unit is the **upstream tag**: a git tag in the language's
canonical source repository, or, for languages without one, a dated published
edition or version. A language's inventory MUST list every such release from
the earliest through the newest at the time of the inventory PR, including
maintenance, patch-level, preview, alpha, beta and release-candidate tags.
Tags that upstream itself says are not releases are excluded in a header
comment that gives the reason, as with Perl's `v5.17.7.0`.

### `code/grammars/<dir>/releases.csv` schema

LANG82's Perl inventory (`release,date,perlhist_line,grammar_status`) is the
precedent. LANG84 fixes a common header. Language-specific extra columns MAY
follow the required ones. Leading `#` lines are provenance comments covering
retrieval date, upstream URLs, and blob or commit IDs of the history sources.

| Column | Required | Values |
| --- | --- | --- |
| `release` | yes | Upstream public spelling (`2.7.8`, `1.8.7-p374`, `3.0.0-preview1`, `3.13.0rc2`, `es5.1`, `js1.8.5`, `5.004_04-m1`). Unique. Charset `[A-Za-z0-9._-]`; anything else is normalized by a mapping documented in the directory README. |
| `date` | yes | ISO `YYYY-MM-DD`, or ISO reduced precision `YYYY-MM` / `YYYY` when the source states only a month or year (it sorts as the start of that period). Uncertain dates take a `?` suffix, and the README states the source. |
| `channel` | yes | `final`, `patch`, `preview`, `alpha`, `beta`, `rc`, `dev`, `trial`, `edition` |
| `source_ref` | yes | Tag name plus resolvable URL, or archive URL, or document URL |
| `source_sha` | yes | Git commit or tag object ID, or SHA-256 of the archive or document; `-` only for a row still pending evidence |
| `predecessor` | yes | Release ID this release's syntax derives from: the previous release on the same branch, or the branch point. Empty only for the first release. |
| `grammar_status` | yes | `pending`, `partial`, `complete` (LANG82 meanings) |
| `grammar_stage` | yes | `none`, `G`, `P` (measured; see below) |
| `grammar_subset` | yes | The subset a `partial` pair covers, or `full`, or `-` |
| `iir_status` | yes | `;`-separated subset names at stage I, or `-` |
| `backend_status` | yes | `;`-separated subset names at stage B, or `-` |
| `profile` | yes | `ReleaseProfile` name, or `-` when `iir_status` is `-` |

Row order is **canonical inventory order**: chronological by `date`, with
ties broken by upstream tag-creation order and then lexically. Release IDs are
never ordered or compared as semantic versions. Interleaved maintenance
branches, for example Ruby 1.8.7-p374 (2013-06) released after 2.0.0-p0
(2013-02), make semver order wrong for scheduling and lineage. Syntax lineage
follows `predecessor`, a tree. Inventory order schedules work: earliest first.

A reduced-precision date sorts as the start of its period, but it is not
forced there. A row may sit anywhere its period allows. The order check passes
when some day inside each row's period makes the column non-decreasing. This
keeps perlhist's `5.000alpha4 1993-???-??` between alpha3 (1993-10-10) and
alpha6 (1994-03-18), where upstream lists it. *(Amended in 2b. The 2a check
put every reduced date at the start of its period, which would have moved
alpha4 ahead of alpha1.)*

Migration of the Perl inventory: the existing columns are kept. `perlhist_line`
becomes a language-specific extra column. The schema PR adds the required
columns and fills them mechanically: `partial` rows become `grammar_stage=P`
with `grammar_subset=pl-decimal-print`, and `pending` rows become `none`.
The rules used for the other columns (ISO dates, row order, channel, source,
and lineage) are in `code/grammars/perl/README.md`.

`code/scripts/release_inventory.py check` (new) validates the following:

- the header, uniqueness and charset
- the date order
- that the `predecessor` graph is acyclic and points to earlier rows
- the monotonic claims: B(S) ⇒ I(S) ⇒ `grammar_stage=P`, and `partial` or
  `complete` ⇒ P
- that every non-pending row has both pair files and every pair file has a row
- that every named subset exists in `subsets.csv`, and every named profile in
  the compiler crate

It reads columns by header name and fails on an absent column (lesson: *read
the inventory header before grouping release statuses*).

Pre-LANG84 pairs that are not release pairs, such as Python's minor-line
`python3.12.*`, are declared in a header comment,
`# legacy-minor-line-pairs: python2.7 python3.0 ...`. The pair-file check skips
the stems it lists and fails if a listed stem has no files.

**Status (2a).** The first `check` landed with the phase 2a inventories. It
covers the header, uniqueness and charset, date format and order, the
predecessor rule (an earlier row; empty only on the first row), the allowed
status values, the monotonic claims, pair files in both directions, declared
subsets, and named profiles. ~~`--matrix` is not implemented yet. Perl's
inventory keeps the LANG82 header and is reported as skipped until the
phase 2b migration.~~ *(Superseded by 2b, below.)*

**Status (2b).** Every inventory, Perl's included, now has the common header,
and the legacy-schema skip (`LEGACY_SCHEMA`) is gone. CI runs
`release_inventory.py check --matrix`.

- **Perl migration.** All 776 rows keep their IDs and `grammar_status`.
  `perlhist_line` stays as an extra column, and `lineage` is added
  (`first`/`line`/`branch-point`, as for Ruby and Python). The migration
  converted dates to ISO and reordered 164 rows into date order. It derived
  `channel` from the ID. It cited the upstream tag (493 rows) or the pinned
  `perlhist.pod` line, at commit `d97e2dc0ca5f…`, as `source_ref`. It derived
  `predecessor` from per-ID lines rather than git ancestry. 2c is expected to
  confirm that lineage. `iir_status`, `backend_status` and `profile` stay `-`:
  stage I for `pl-int-print` needs the corpus and the `release_subsets` test
  from 4c. The reordering changes only the order of the generated
  `INVENTORY_RELEASES`/`SUPPORTED_RELEASES`. It moves no release to another
  table, changes no `g_` module, and does not change the first release of any
  class. The grammar-tools reader now refuses a row whose field count differs
  from the header's. The perl-parser tests read the CSV by header name, and
  none of them matches `,partial` row endings any more.
- **`subsets.csv`** for `perl/` (`pl-decimal-print`, LANG82; `pl-int-print`,
  LANG81), `ruby/` (`rb-int-puts`, LANG80), `python/` (`py-float-print`,
  LANG79) and `ecmascript/` (`es-number-log`, LANG78). The checker reads the
  four columns by name. It requires a subset name matching
  `[a-z0-9]+(-[a-z0-9]+)*`, other than `full`, and a non-empty description.
  `corpus_dir` must be exactly `corpus/<subset>`, so a cell cannot point
  outside the grammar directory. `spec` must name a file in `code/specs/`, and
  `subsets.csv` must not exist without a `releases.csv`. A subset claimed at
  I or B needs at least one positive corpus case. Every case has exactly one
  source file and exactly one of `.expect` or `.reject`. Every `.expect` has a
  `.cite`, and an `.expect@<profile>` override has a plain `.expect`.
- **`--matrix`** (contract for 4c). For each release R claiming B(S), each
  positive case C of S needs one of three things. `release_classes.csv` can
  mark (C, R) `rejected`, meaning R does not accept C: R's grammar does not
  parse it, or a `.reject@<profile>` override puts it outside R's profile
  (the second reading added in 4c). A
  `lang_matrix` row can run C on R. Or `release_classes.csv` can put (C, R) in
  an IIR-identity class whose representative (its first release in inventory
  order) has such a row. `release_classes.csv` (`subset,case,release,class`,
  where `class` is 12–64 hex digits of the IIR hash, or `rejected`) is
  written by 4c's `release_sweep.rs`. A `lang_matrix` row counts when it names
  `release: Some("<id>")` and `case: Some("<subset>/<name>")`. The `case`
  field is new: it ties a row to a corpus case without comparing program
  text. The checker reads `Prog { .. }` literals with string and character
  literals masked, so program text cannot be mistaken for a field. It fails
  on a row naming an unknown release or case, and on a malformed or
  duplicate class entry. Matrix rows that name no release are counted in a
  note rather than failed until 4c adds the field to the 11 Perl rows.
  *(Since 4c such a row fails the check.)* Each
  inventory prints a coverage line (`N backend claim(s); case coverage …`),
  so a pass with zero claims is visible rather than silent.

### Materialized pair per release

As in LANG82, every non-pending release has its own
`<prefix><release>.tokens` and `<prefix><release>.grammar`. No release is an
alias to another's files. The prefix is the existing file stem: `ruby`,
`python` and `perl` for those languages. The `ecmascript/` directory takes no
prefix, because its IDs already start with `es` or `js`.

**Evidence automation.** `code/scripts/release_evidence.py` (new) records,
per release, the blob IDs of the upstream files that define syntax:

| Language | Upstream syntax files |
| --- | --- |
| Ruby | `parse.y`, `defs/keywords`, and `prism/config.yml` from 3.3 |
| Python | `Grammar/Grammar` or `Grammar/python.gram`, `Grammar/Tokens`, `Parser/tokenizer.c` |
| Perl | `perly.y`, `toke.c`, `keywords.h` |
| JavaScript | the edition document's digest |

A release whose syntax blobs are identical to its predecessor's MAY have its
pair produced mechanically: a copy of the predecessor pair with a header that
cites both releases' identical blob IDs. That counts as LANG82 "release
evidence". A release with changed blobs needs a reviewed delta that records
what changed and which subset is affected.

### Compiled tables deduplicate identical grammars

`grammar-tools generate-rust-compiled-grammars <pkg>` reads the release list
from `releases.csv` (non-pending rows) instead of the hard-coded
`*_VERSIONS` arrays in `programs/rust/grammar-tools/src/lib.rs`. For each pair
it:

1. Parses the pair and serializes it canonically, dropping comments and
   `line_number` fields.
2. Hashes the result with SHA-256.
3. Emits one module per **distinct** hash, named `g_<first 12 hex>`.
4. Emits `token_grammar(release)` and `parser_grammar(release)` match arms
   that map every release ID to its module, plus `SUPPORTED_RELEASES` in
   inventory order.

Grammar line numbers in diagnostics come from the first release in each
class. A regeneration check fails CI if `_grammar.rs` disagrees with the CSV
or the files. If one crate's compiled size or build time becomes a problem,
eras MAY be gated behind cargo features (see [Open questions](#open-questions)).

### Selection API

- CLI: `lang-aot --lang ruby@2.7.8 prog.rb`. A bare `--lang ruby` uses
  `DEFAULT_RELEASE`. `Language::parse` splits on the first `@`.
  `detect_language_from_path` returns a language with no release.
- Library: `lang_aot::compile_source_to_iir_at(language, release: Option<&str>,
  source, module_name) -> Result<IIRModule, LangAotError>`. The existing
  `compile_source_to_iir` becomes `..._at(language, None, ..)`.
- Frontend crates: `<lang>_iir_compiler::compile_source_at(source,
  release: Option<&str>, module_name, &FrontendBudgets)`. The existing
  `compile_source` delegates to it with `None`.
- New `LangAotError` variants: `UnknownRelease`, `ReleaseNotAvailable`.
- Release-named variants such as `Algol60` and `Cobol60` become
  `Algol`/`Cobol` with releases. Their old spellings (`algol60`, `cobol60`)
  remain as aliases that resolve to an **explicit** release (`algol@1960`,
  `cobol@60`), never to a default.

### Release profiles

Some semantics differ between releases while the CST stays the same: Python 2
versus 3 `/` on integers, `print` as a statement, Ruby 1.8 `?a` being an
Integer where 1.9 makes it a String. These differences are carried by
`ReleaseProfile`, a plain struct of semantic switches defined in the IIR
compiler crate (`profiles.rs`), such as `int_div: IntDiv::Floor` or
`char_literal: CharLit::Integer`. Each CSV row names its profile, and many
releases share one. Branching on profile fields is the **only** way the
compiler varies by release beyond the CST shape (C2).

### Status stages

Each stage is measured per (release, subset S) by a named test. The CSV may
claim only what the test proves.

| Stage | Meaning | Proven by |
| --- | --- | --- |
| **G** | Both files exist, `grammar-tools validate` passes for each, the tokens/grammar cross-validator passes, and the header cites `source_ref` and `source_sha` | `grammar-tools validate`/`cross-validate` over every non-pending row |
| **P** | Positive and negative parse probes for the release's declared `grammar_subset` pass through the compiled tables, and hostile probes return errors | `cargo test -p <lang>-parser release_probes` |
| **I(S)** | Every corpus case of subset S that applies to the release compiles to IIR that validates and passes C7. Every negative case is rejected with a typed `Unsupported` or `Parse` error naming the release | `cargo test -p <lang>-iir-compiler release_subsets` |
| **B(S)** | For subset S, every case runs on **all nine** LANG83 backends (x86_64, aarch64, LLVM, WASM, JVM, CLR, BEAM, VM, JIT) with output matching the oracle, **or** the backend refuses the module at compile time with `UnsupportedOps`. A miscompile, a silent skip of an in-process backend, or a runtime refusal fails the stage | `lang_matrix` rows plus the release sweep (below) |

Subsets are declared in `code/grammars/<dir>/subsets.csv`
(`subset,description,corpus_dir,spec`), for example
`rb-int-puts,LANG80 integer puts subset,corpus/rb-int-puts,LANG80`.

### Corpora and oracles

The corpus lives under `code/grammars/<dir>/corpus/<subset>/`. Each case is
`<name>.<ext>` plus `<name>.expect` (stdout, exit code, or
`trap:<kind>` after a stdout prefix, mirroring LANG83 `StdoutThenTrap`) and
`<name>.cite`. A case may carry `<name>.expect@<profile>` overrides. A case
applies to a release when the release lists the subset and its grammar parses
the case. Negative probes live in `<name>.reject`, with the expected error
category. A positive case may also carry `<name>.reject@<profile>`: under that
profile the case is outside the subset and must be refused with the named
category, because the release's own semantics for it are not established
(Perl's negative zero, below). *(Added in 4c.)*

Every `.expect` file MUST have an oracle record in its `.cite`. One of:

- **Host interpreter**, for current releases. The CI image's `ruby`,
  `python3`, `node` or `perl` produced the output. The record gives the exact
  interpreter version, and the case is checked only against that release row
  (and, through IIR identity, its class). Lexing parity MAY use `Ripper.lex`
  or `Prism.lex`, CPython `tokenize`, or Acorn as differential oracles.
- **Pinned historical source**, for older releases. The record names the tag
  plus SHA and the file and line (grammar, tokenizer, a test in the release's
  own suite, test262 with its `features` metadata, ruby/spec, CPython
  `Lib/test`), or a section of the edition document.
- **Optional nightly historical-interpreter containers.** Container images
  pinned by digest that build an old interpreter from its tagged source. They
  run in a network-sandboxed nightly job that is never blocking. Their
  outputs may be committed as oracle records with the image digest.

### CI bounding

There are about 1,200 Ruby, about 680 Python and 776 Perl releases, so every
release cannot run on every backend in every PR. The bound is computed, not
hand-picked:

1. **Compile everything.** For every release claiming I(S), compile every
   applicable case of S to IIR. This is in-process and cheap.
2. **IIR identity classes.** Group the releases by the hash of the produced
   IIR, ignoring the `release` field. Within a class, releases are
   indistinguishable to every backend. The CI asserts each case's class
   membership and fails if a release silently changes class without a CSV or
   profile change.
3. **Boundary releases.** The first release of each class (in inventory
   order), plus the `DEFAULT_RELEASE` and the newest release. Boundary
   releases run on all nine backends: per PR for classes whose grammar,
   profile, compiler or backend changed (per build-tool diff), and nightly for
   all classes.
4. **Every other release** is covered by compile-only plus IIR identity with
   its boundary representative.

`lang_matrix`'s `Prog` gains `release: Option<&'static str>`. It is required
(`Some`) for every language with an inventory, and `None` only for in-repo
languages without one. It also gains `case: Option<&'static str>`
(`"<subset>/<name>"`), which names the corpus case the row runs. *(Added in
2b for `--matrix`.)* A new `release_sweep.rs` test performs steps 1–2 and
generates the boundary set. `release_inventory.py check --matrix` fails if a
release claims B(S) while a case of S has neither a matrix row on its
boundary release nor membership in a class whose representative has one.

## grammar-tools capability extensions

Each extension lands in its own PR, Rust first, with an F-series spec (an
F10 amendment or a new F-number assigned at the time). It is backward
compatible: grammars that don't use it behave byte-identically. It has
validator checks and the budgets of C5.

New section names (`pairs`, `goals`, `goal_of`, `line_terminators`,
`deferred`, `max_pair_depth`) join the F04/F10 reserved set, and none of them
can be used as a group name.

The non-Rust grammar-tools ports (Python, Ruby, TypeScript, Go, Elixir) MUST
**reject** a grammar file that uses an extension they do not implement, with a
clear error. Silently ignoring the extension is not allowed. Porting is
optional and outside LANG84's critical path.

The summary below lists the extensions in dependency order:

| # | Extension | Ruby port | JS rewrite | Others | Replaces |
| --- | --- | --- | --- | --- | --- |
| E1 | Line-start / column guards | needed (R1, R5) | minor (hashbang, Annex B `-->`) | Perl POD/`__END__`, COBOL fixed form | optionally `strip_cobol_columns` |
| E2 | Bracket-kind stack, depth-guarded push/pop, nesting delimiters | needed (R2, R3) | needed (templates, `}` kind) | Perl `q{}` nesting | the hard-coded JS template path in `grammar_lexer.rs` (`template_entry_depths`, `template`/`template_div`) |
| E3 | Mode- and depth-conditional skip | needed (R4) | — | Wolfram, Derive | `drop_bracketed_newlines` hooks |
| E4 | Captured terminators (capture, then literal match) | needed (R3, R5) | — | Perl custom delimiters and heredocs | — |
| E5 | Deferred-body queue (heredocs, `<<~` dedent, FIFO) | needed (R5) | — | Perl heredocs | — |
| E6 | Token-adjacency predicates and ASI in `.grammar` | `@space_before` (R6) | needed (ASI, `[no LineTerminator here]`) | Go-like languages | `javascript-parser/src/asi.rs` on the IIR path |
| E7 | Goal-directed lexing (parser→lexer) and binding predicates | needed (R6) | needed (full regex/division) | Perl (later) | JS `tokenize_javascript_division_only`; `ruby-lexer` `ParserOracle` |

### E1. Line-start and column guards

**Motivation.** Ruby `=begin`/`=end` and `__END__`, and Perl POD (`=pod` …
`=cut`) and `__END__`/`__DATA__`, are recognized only at the start of a line.
Heredoc terminators (E5) are whole lines. ECMAScript 2023's hashbang is legal
only at input offset 0. COBOL and FORTRAN fixed form are column-based.

**Syntax** (`.tokens`, a suffix on a definition):

```
EMBDOC      = /=begin\b[\s\S]*?\n=end\b[^\n]*/  @line_start
DATA_TAIL   = /__END__(\r?\n[\s\S]*)?$/          @line_start
HASHBANG    = /#![^\n]*/                          @input_start
SEQ_AREA    = /[^\n]{1,6}/                        @column(1)
INDICATOR   = /[ *\/D-]/                          @column(7)
```

`line_terminators: /\r\n|[\n\r  ]/` is an optional top-level
directive. The default is `/\r\n|\n/`, which matches what the lexer counts as
a line today.

**Semantics.**

- A guarded definition is tried only when its guard holds at the current
  position.
- `@line_start` holds at offset 0 or immediately after a line terminator.
- `@input_start` holds only at offset 0.
- `@column(N)` and `@column(N..M)` compare the 1-based column, counted in
  Unicode scalar values with no tab expansion, against N or the range.
- Guards also apply to transitions (`on X if line_start -> …`) and to `skip:`
  entries.
- A guard never changes the order of definitions. An unguarded earlier
  definition still wins if it matches.

**Security and budget.** Each guard is a constant-time check. The validator
rejects `@column` values above 4,096 and ranges with N > M.

### E2. Bracket-kind stack, depth-guarded push/pop, nesting delimiters

**Motivation.** Two cases depend on knowing which opener a closer closes:

- The `}` that closes `#{…}` (Ruby) or `${…}` (JS) ends an interpolation only
  at the depth where the interpolation opened. Today JS hard-codes this in
  `GrammarLexer` (`template_entry_depths`, with `template`/`template_div`
  group names baked into the engine). F10 lists the `}` and `{` kinds as
  residual hard cases.
- Ruby `%w[a [b] c]`, `%q(a (b) c)` and Perl `q{a {b} c}` nest their own
  bracket delimiter.

**Syntax.**

```
pairs:
  LPAREN       -> RPAREN
  LBRACKET     -> RBRACKET
  LBRACE       -> RBRACE
  LBRACE_BLOCK -> RBRACE as RBRACE_BLOCK      # closer retyped by top-of-stack kind
  INTERP_BEGIN -> RBRACE as INTERP_END pop    # closing the frame also pops its group

transitions:
  on INTERP_BEGIN -> push code
  on TEMPLATE_HEAD -> push code
```

A nesting-delimiter token form for single-token literals without
interpolation:

```
PCT_W = nested(/%w/, open: bracket)     # body scanned to the matching closer
```

**Semantics.**

- The lexer keeps a **stack of open pair kinds**. It generalizes today's
  per-character `BracketDepths` counters and is keyed by declared token type,
  not by character.
- An opener token pushes its kind.
- A closer token pops the top kind if that kind's declared closer is this
  token type. The closer is then emitted with the retyped name, if one is
  given, and runs the pair's `pop` action, if one is given.
- A closer that does not match the top kind is a lexer error with a span.
  The lexer does not silently recover.
- Guards `@depth(KIND > n)` and `@top(KIND)` are available on definitions,
  skip entries and transitions.
- `nested(prefix, open: bracket)` matches the prefix and reads the next
  character as the opener. It must be one of the declared bracket characters
  `( [ { <`, mapped to its closer. The body runs to the matching closer, with
  same-kind nesting counted and backslash escapes honoured. The whole literal
  is one token.
- Non-bracket delimiters, and literals with interpolation, use E4.
- After E2 lands, the JS template special case in `grammar_lexer.rs` is
  deleted. `es*.tokens` declare the template pairs instead, and the existing
  template tests are the acceptance gate.

**Security and budget.** The stack is capped at 256 frames by default
(`max_pair_depth:` directive, hard maximum 4,096). An overflow is a `Budget`
error. The `nested` scan is linear with an O(1) counter.

### E3. Mode- and depth-conditional skip

**Motivation.** Some newlines are trivia only in certain contexts:

- Ruby: after a binary operator (EXPR_BEG), and inside `[ ]` and hash `{ }`
  but not inside block `{ }`.
- Wolfram and Derive: inside brackets.

F04 skip patterns are global, so today these need Rust hooks.

**Syntax.**

```
skip:
  WS              = /[ \t]+/
  NL_IN_BRACKETS  = /\r?\n/  @depth(LBRACKET > 0) @top(LBRACKET)
  NL_AFTER_OP     = /\r?\n/  @in(beg)
```

**Semantics.** A guarded skip entry is tried, in order with the other skip
entries, only when its guards hold. `@in(MODE)` tests the active F10 mode, and
E2 guards test the pair stack. If no skip entry matches, the newline falls
through to ordinary definitions, such as a `NEWLINE` token. The skip
`enable-skip`/`disable-skip` toggles are unchanged.

**Security and budget.** Constant-time guard checks. No new state.

### E4. Captured terminators (capture, then literal match)

**Motivation.** Some literals end at a delimiter or terminator chosen by the
program:

- Ruby `%q|…|`, `%r!…!`, and `%Q<…#{x}…>` with interpolation
- Perl `q#…#`, `s{…}{…}`, `tr/…/…/`
- heredoc terminators (`<<EOS`)

Rust `regex` has no backreferences (patterns compile as `^(?:pat)`), so
`/(.)[^\1]*\1/` is impossible. LANG84 specifies **capture-then-literal-match**
instead of backreferences.

**Syntax.**

```
PCT_Q_BEGIN = /%Q(?P<open>[^\w\s])/  -> push dstring { close: closer(open), open: open }

group dstring:
  STRING_END     = $close   @frame_nest(0)
  NEST_CLOSE     = $close   -> frame_nest -1 as STRING_CONTENT
  NEST_OPEN      = $open    -> frame_nest +1 as STRING_CONTENT   # only when open != close
  INTERP_BEGIN   = "#{"
  ESCAPE         = /\\[\s\S]/
  STRING_CONTENT = until($close, $open, "#{", "\\")
```

**Semantics.**

- A pattern may contain **named capture groups**. A `push G { slot: expr }`
  action creates a **frame** on the E2 stack carrying named slots, each at
  most 256 bytes.
- A slot expression is one of:
  - a capture name
  - `first_of(a, b, …)`, the first capture that participated in the match
  - `closer(name)`, mapped through `pairs:` for bracket characters and
    otherwise the same character
  - `not(name)`, a flag that is true when the capture did not participate
  - a literal
- `$slot` in a definition is a **literal** match against the top frame's slot
  value. No regex is built from captured text.
- `until(lit, …)` scans forward to the earliest occurrence of any listed
  literal or slot value. It is linear and uses `memchr`-style search, and it
  requires at least one byte.
- `@frame_nest(n)` and `frame_nest ±1` read and update a per-frame counter, so
  bracket delimiters nest.
- `STRING_END` pops the frame.
- A frame still open at end of input is a lexer error at its opening span.

**Security and budget.** Captured text is never compiled as a pattern, so
there is no regex injection and no pathological backtracking. Slot length is
capped at 256 bytes and frame depth by E2. `until` is O(n) per token.

### E5. Deferred-body queue (heredocs)

**Motivation.** A heredoc's body starts on the *next* line while the rest of
the current line keeps lexing. Several heredocs on one line are read FIFO.
`<<~` removes common indentation. Ruby and Perl (`<<~` since 5.26) both need
this, and no regex lexer can express it.

**Syntax.**

```
HEREDOC_BEGIN = /<<(?P<mode>[-~]?)(?:"(?P<dq>[^"\n]+)"|'(?P<sq>[^'\n]+)'|(?P<bare>[A-Za-z_]\w*))/
                -> defer heredoc { term: first_of(dq, sq, bare), mode: mode,
                                   interpolate: not(sq) }
deferred heredoc:
  body_group: hstring          # lexed with E4 slots; interpolation unless single-quoted
  terminator: $term @line_start @allow_indent(mode in "-~")
  dedent: mode == "~"
```

The quoted forms close with a fixed quote character inside the regex. Rust
`regex` disallows duplicate group names, so the alternatives use distinct
names and `first_of` picks the one that matched. No backreference is used.

**Semantics.** This is MRI's model, written down as data. The lexer keeps a
**body cursor**, initially unset.

1. When a `defer` action fires, the lexer sets the body cursor to the start
   of the next line, if it is not already set.
2. It then lexes the body from the cursor in `body_group`. The body ends at
   the first line that, after optional leading whitespace when
   `@allow_indent` holds, literally equals `$term`, followed by a line
   terminator or end of input.
3. It emits the body tokens followed by `HEREDOC_END`, **immediately after**
   `HEREDOC_BEGIN` in the token stream.
4. It advances the cursor past the terminator line and resumes lexing the
   current line where it left off.
5. When the current line's terminator is consumed, lexing jumps to the body
   cursor and clears it.
6. Several `defer`s on one line therefore read their bodies in FIFO order.
   Tokens keep their true source lines and columns.

`dedent` uses MRI's rule: the minimum indentation over the body's non-blank
literal lines, with a tab advancing to the next multiple of 8. That amount is
removed from `STRING_CONTENT` segments that begin a line. Interpolated
segments are not dedented. A token's value changes; its span does not.

A missing terminator is a lexer error at `HEREDOC_BEGIN`.

**Security and budget.** At most 64 pending heredocs per line and 4,096 per
input. Terminator length is limited by the E4 slot cap. Heredocs nested
inside an interpolation inside a heredoc are limited by the E2 frame cap.
Total work is linear: every byte is lexed once.

### E6. Token-adjacency predicates and ASI in `.grammar`

**Motivation.**

- ECMAScript's restricted productions (`return [no LineTerminator here]
  Expression`, postfix `++`/`--`, `throw`, `break`/`continue` labels, `yield`,
  `async [nlth] function`, arrows) and Automatic Semicolon Insertion are
  grammar facts that change by edition.
- Today ASI is a hand-written retry-on-parse-error pass (`asi.rs`, about 338
  non-test lines). It re-parses once per insertion, which is quadratic on
  hostile input, and it depends on parser error internals.
- The existing `!NEWLINE` lookahead works only when newlines are tokens, and
  that makes the whole grammar newline-significant
  (`grammar_references_newline`).
- Ruby needs whitespace-adjacency facts (`foo -1`, `foo [1]`).

**Syntax** (zero-width elements in `.grammar`):

```
return_statement = "return" [ @no_newline_before expression ] statement_end ;
postfix_expr     = lhs_expr [ @no_newline_before ( "++" | "--" ) ] ;
statement_end    = SEMICOLON | @asi(SEMICOLON, RBRACE) ;
for_header       = "for" LPAREN [ init ] SEMICOLON [ test ] SEMICOLON [ update ] RPAREN ;
command_arg      = @space_before !@space_after_next MINUS primary ;
```

**Semantics.** These elements read token flags (`TOKEN_PRECEDED_BY_NEWLINE`
already exists). A new `TOKEN_PRECEDED_BY_SPACE` flag is added. The lexer MUST
set the newline flag when skipped trivia contains a line terminator, including
inside a multi-line comment, as ECMAScript requires.

- `@no_newline_before` succeeds iff the next token lacks the newline flag.
- `@space_before` succeeds iff the next token has the space flag.
  `@space_after_next` looks at the token after it.
- `@asi(T, C…)` succeeds without consuming when any of the following holds:
  - the next token has the newline flag
  - the next token's type is one of the closers C
  - the parser is at end of input

  On success it adds a **synthetic leaf token** of type `T` with a new
  `TOKEN_SYNTHETIC` flag, zero width, positioned at the next token. The CST
  shape is then the same whether or not the semicolon was written.

PEG greediness supplies ECMAScript's "offending token" condition: `@asi` is
tried only after the statement's expression has stopped consuming, so
`a \n (b)` still parses as a call, as the specification requires. Places
where ASI must not apply (the `for` header, an empty statement) use a plain
`SEMICOLON`. All predicates read only the next one or two tokens, so they are
packrat-memo-safe.

**Decision: ASI lives in the grammar, not in a hook.** Reasons:

1. Restricted productions and the ES2015 `do-while` ASI change vary by
   edition and belong in the versioned `.grammar`. A hook would need
   per-release Rust code, which C2 forbids.
2. The predicate form is linear, while retry-and-reparse is not (C5).
3. The current hook exceeds the C3 size limit and inspects parser failures,
   which violates C3.1.

`asi.rs` remains for closurec, a non-IIR consumer, until closurec adopts the
grammar construct.

**Security and budget.** Constant-time predicates. Synthetic tokens are capped
at one per real token.

### E7. Goal-directed lexing (parser→lexer feedback) and binding predicates

**Motivation.**

- **JavaScript lexical goals.** ECMA-262 defines lexical goal symbols
  (`InputElementDiv`, `InputElementRegExp`, `InputElementTemplateTail`, …).
  The syntactic context chooses the goal. F10 flat modes approximate this from
  the previous token, and F10 records the residual failures: `)` after
  `if (…)`, and `{` as block or object at statement start.
- **Ruby's local-variable rule.** Whether `foo /x/`, `foo -1`, `foo [1]` and
  `foo ::C` are a command call with an argument or a binary operation depends
  on whether `foo` is a local variable *at that point*. MRI's lexer asks the
  parser's local table, and the current `ruby-lexer` imitates this with a
  `ParserOracle` trait.

`GrammarParser::new(tokens, grammar)` takes a fully pre-lexed vector, so
neither case can be expressed today.

**Design.** Instead of the lexer calling into the parser, the **parser tells
the lexer which goal to use** for the token it is about to try, and the
**parser decides locality itself** with binding predicates. This is
context-aware scanning, the same idea as tree-sitter's `valid_symbols`, made
declarative.

**Syntax** (`.tokens`):

```
goals:
  div:    default                       # InputElementDiv: the default patterns
  regexp: default + { REGEX } - { SLASH, SLASH_EQUALS }
  template_tail: default + { TEMPLATE_MIDDLE, TEMPLATE_TAIL } - { RBRACE }
goal_of:
  REGEX: regexp
  TEMPLATE_MIDDLE: template_tail
  TEMPLATE_TAIL: template_tail
```

**Syntax** (`.grammar`):

```
method_def   = "def" @scope(hard) fname [ params ] body "end" ;
block        = "do" @scope(soft) [ block_params ] body "end" ;
param        = NAME@declare ;
assignment   = NAME@declare "=" expression ;
command_call = NAME@unbound @space_before REGEX { call_tail } ;
local_ref    = NAME@bound ;
```

**Semantics.**

- *Goal-directed lexing.* `GrammarParser::new_streaming(lexer, grammar)`
  pulls tokens lazily.
  - When the parser tries terminal `T` at a token boundary, the lexer lexes
    there under `goal_of(T)`, or `default` if none is given.
  - Results are cached by (byte offset, goal, lexer-state hash). The token
    sequence becomes a lattice, and packrat memo keys use token-node IDs
    instead of vector indices.
  - Lexer state, meaning the F10 mode, the E2 frames and the E5 cursor, is
    snapshotted at each token boundary, so backtracking only selects another
    cached branch.
  - A grammar with no `goals:` section takes the old eager path, unchanged.
- *Binding predicates.*
  - `@scope(hard)` starts an empty local table and `@scope(soft)` starts a
    table that inherits the enclosing one. The table ends when the rule ends.
  - `NAME@declare` records the token's value in the innermost table,
    effective for tokens **after** it. This is Ruby's `x = x` rule: the name
    is bound before the right-hand side.
  - `NAME@bound` and `NAME@unbound` match a `NAME` only if its value is, or is
    not, bound at that point.
  - Bindings form an append-only log tagged with token position. On
    backtrack, the log is truncated to its length at the backtrack point.
  - Memo entries record the log version at entry and are reused only when it
    matches.
- *The Ruby case.* `foo /x/` with `foo` unbound: the `command_call`
  alternative tries `REGEX`, and the lexer lexes `/x/` under the `regexp`
  goal. With `foo` bound, `NAME@unbound` fails first. The binary-operator
  alternative then tries `SLASH`, and the lexer uses `div`.

**Security and budget.** At most 2 × source bytes + 1,024 lex calls in total
(C5), after which the result is a `Budget` error, so hostile inputs cannot
drive exponential re-lexing. The binding log is capped at the token count.
Scope depth is capped at the parser depth. `goals:` and `goal_of:` targets are
validated (no undefined goals, no terminal in two `goal_of` entries).

**Replaces.** `tokenize_javascript_division_only` and its `REGEX` deletion.
Before E7, the F10 table already shipped in `es2025.tokens` is enough for
LANG78's numeric subset (see phase 6a). Also replaces the `ParserOracle`
trait in `ruby-lexer`.

## Compliance and migration

Audit at `dac64374d1`, 2026-10-20. GL = `GrammarLexer` over compiled grammar;
GP = `GrammarParser` → `GrammarASTNode`. Line counts are non-test lines unless
noted.

| Frontend | Lexer | Parser | IIR input | Releases today | Verdict | Migration |
| --- | --- | --- | --- | --- | --- | --- |
| ALGOL 60 | GL | GP | CST | unversioned | Compliant | Inventory (1960 report, 1963 revised, 1976 modified); `Algol60` → `Algol` with releases |
| Axiom | GL | GP | CST | unversioned | Compliant, not in lang-aot | Inventory; `Language::Axiom` |
| Brainfuck | GL | GP | CST | unversioned | Compliant | Single-release inventory (1993) |
| FLOW-MATIC | GL | GP | CST | unversioned | Compliant | Inventory (B-0) |
| Macsyma | GL | GP | CST | unversioned | Compliant | Inventory |
| Maxima | re-exports Macsyma | — | CST | none of its own | No grammar of its own | Own `maxima<release>` pairs from Maxima tags before any `Language::Maxima`. Until then `--lang maxima` is an error, not a silent Macsyma alias |
| Maple, Reduce | GL | GP | CST | unversioned | Compliant, not in lang-aot | Inventory; `Language` variants |
| Nib, Oct | GL | GP | CST | unversioned | Compliant (in-repo languages) | Single-row inventory (see open questions) |
| Python | GL (indentation mode) | GP | CST | 2.7, 3.0, 3.6, 3.8, 3.10, 3.12 + unversioned; no CSV | Compliant; selection hard-coded to `"3.12"` | Phase 7 |
| Perl | GL | GP | CST | 118 partial versioned pairs, compiled into 5 distinct token and 2 distinct parser tables; `releases.csv` 776 rows | Compliant; `compile_source_at` selects any checked release; `DEFAULT_RELEASE` is the inventory release `5.38.0` and the unversioned pair is off the IIR path (4b); `IIRModule.release` recorded; `Language::Perl` on all nine backends with `--lang perl@<release>` (4a, 4b); I and B(`pl-int-print`) for all 118 checked releases, two `ReleaseProfile`s, three IIR-identity classes (4c) | Phase 4 done; later subsets and pairs are phase 9 backfill |
| COBOL-60 | GL + `strip_cobol_columns` | GP | CST | unversioned | Compliant (C3 hook) | Inventory (60, 61, 61-ext, 65, 68, 74, 85, 2002, 2014, 2023); `Cobol60` → `Cobol` |
| Dartmouth BASIC | GL + 2 hooks (`lib.rs`: 391 non-test lines incl. docs) | GP | CST | unversioned | Compliant (C3 hooks) | Inventory (editions 1–7) |
| Derive, Wolfram | GL + `drop_bracketed_newlines` | GP | CST | unversioned | Compliant (C3 hook) | Optional E3 replacement; inventories |
| MacroOct, MacroNib | GL + Oct/Nib GP | GP | CST | — | Base parse compliant; directive layer is a preprocessor | See [Preprocessor dialects](#preprocessor-dialects) |
| McCarthy Lisp | GL | GP | CST | unversioned | Compliant since `mccarthy-lisp-iir-compiler` 0.9.0 (was C2 typed lift via `extract_program` → `Vec<LispExpr>`) | Phase 8 CST rewrite done; inventory (LISP 1, 1.5) |
| Twig | GL | GP, then `ast_extract.rs` (1,401) + `ast_nodes.rs` (636) → `twig_parser::Program` | typed lift | unversioned | **Non-compliant (C2 typed lift)** | Phase 8 rewrite |
| JavaScript | GL with runtime `REGEX`/`BAD_REGEX` deletion | GP over `es*.grammar` + `asi.rs` (≈338) | CST (`parse_javascript_cst_division_only`) | 14 pairs (es1, es3, es5, es2015–es2025) + unversioned; no CSV; hard-coded `Es2020` | **Partial:** the IIR compiler walks the CST since `javascript-iir-compiler` 0.3.0 (decision 6 met; was a C2 typed lift via `bridge.rs` (≈4,214) → `javascript_ast::Program`). Still non-compliant under C2: the runtime `REGEX` deletion in the lexer and `asi.rs`, which is longer than a small hook, remain until 6a/6b replace them | Phase 6c CST rewrite done; 6a, 6b, 6d, 6e remain |
| Ruby | hand-written `ruby-lexer` (≈2,200 lines) driving `ruby-1.8.lexer.states.toml` (1,917 lines; 15 eras share it, the era list stops at 3.3) | GP over `ruby.grammar` (919 lines) | CST | unversioned; stale 79-line `ruby.tokens` | **Non-compliant (C2 hand-written lexer)** | Phases 3 and 5 |

### Ruby: port the lexer to `ruby<release>.tokens`

The current lexer is replaced, not wrapped. The port is staged by feature.
Each slice adds tokens to the per-release pairs, earliest release first, and
is checked by differential tests against the current `ruby-lexer` and against
`Ripper.lex`/`Prism.lex` for the host Ruby's release.

| Slice | Constructs | Needs |
| --- | --- | --- |
| R0 | Integers (decimal, `0x`, `0b`, `0o`/leading `0`, `_` separators), floats, identifiers, constants, `@iv`, `@@cv`, `$g`, `:sym`, `?c`, keywords, operators (`**`, `<=>`, `>>`, compound assignments, `&.` from 2.3, `..`/`...`, `->` from 1.9), `#` comments, `\` line continuation, `NEWLINE`/`;`. Newline-after-operator continuation is handled in `.grammar` with optional `NEWLINE` after binary operators | nothing new (F10 exists) |
| R1 | Non-interpolating strings `'…'`, simple `"…"`, `=begin`/`=end`, `__END__` | E1 |
| R2 | Interpolation `"…#{…}…"`, backticks, `:"…"`, char escapes | E2, E4 |
| R3 | `%w %W %i %I %q %Q %r %s %x %()` with nesting brackets and arbitrary delimiters | E2, E4 |
| R4 | Regex literal versus division, and modifier `if`/`unless`/`while`/`until`/`rescue`, via F10 flat modes (EXPR_BEG/END/ARG/FNAME/DOT); newline trivia after operators and inside `[`/hash `{` | F10, E3 |
| R5 | Heredocs `<<ID`, `<<-ID`, `<<~ID`, quoted IDs, several per line | E1, E4, E5 |
| R6 | Disambiguation that depends on locals: `foo /x/`, `foo -1`, `foo [1]`, `foo ::C`, `p *a`, `foo &b` | E6 (`@space_before`), E7 |
| R7 | Per-release lexical deltas along the `predecessor` tree: 1.9 hash labels and `->`; 2.0 `%i`; 2.1 `1r`/`1i`; 2.3 `<<~`, `&.`; 2.6 endless range; 2.7 `_1` and beginless range; 3.0 `=>` rightward assignment and endless `def`; 3.1 `{x:}` and anonymous `&`; 3.2 anonymous `*`/`**`; 3.4 `it` | per-release pairs |

R0 is the only slice phase 5 needs. At phase 5 the IIR path stops using
`ruby-lexer`: `ruby-parser` builds `GrammarLexer` over `ruby<release>.tokens`
for the IIR entry points. After R6 reaches parity on the corpus, the state
machine TOML, the action interpreter and `oracle.rs` are deleted, and
`ruby-lexer` becomes the usual thin `GrammarLexer` wrapper.
`ruby-lexer-state-machine.md` and the era list in
`ruby-version-evolution.md` are then marked superseded by LANG84.

### JavaScript: rewrite to walk the CST

1. **6a.** Copy the F10 regex/division table (`start_mode`, `group div`,
   `transitions`) from `es2025.tokens` into every `es*.tokens` that lacks it.
   Delete `tokenize_javascript_division_only` and
   `parse_javascript_program_division_only` from the IIR path. The numeric
   subset's `console.log(-0, 1 / 0, 0 / 0)` lexes correctly under the table,
   because `NUMBER` sets `div` mode. The existing division-chain, regex and
   hostile-spine probes are the gate.
2. **6b.** E6: add `@asi`/`@no_newline_before` to the `es*.grammar` files, and
   stop using `asi.rs` on the IIR path.
3. **6c.** Rewrite `javascript-iir-compiler` (636 lines) to walk the
   `es<edition>` CST. Gate: it must emit IIR identical to the current
   typed-AST pipeline across the whole existing test corpus, then delete the
   `bridge.rs` dependency from the IIR crate. `bridge.rs` stays for closurec.
   **Done** in `javascript-iir-compiler` 0.3.0, ahead of 6a/6b (the
   division-only token goal and `asi.rs` are kept as hooks for now).
   `javascript-parser` 0.63.0 adds `parse_javascript_cst_division_only`, the
   typed division-only entry minus the bridge; the compiler's public
   `compile_ast` takes that `GrammarASTNode`. Borrowed `Copy` handles read
   the precedence ladder, flat binary chains (left fold by index) and flat
   suffix chains in place, with no owned tree; a `check_cst` pass keeps the
   bridge's errors for the compiled subset verbatim and in the same
   whole-program precedence. The IIR crate now names neither `bridge.rs` nor
   `javascript_ast` (source-checked in its tests; `javascript-ast` is a
   dev-dependency, used as a number-decoding oracle). Gate result: old and
   new compilers diffed byte for byte (`Debug` dump of the `IIRModule`, or the
   error text) over every string literal in the JavaScript, closure and jsdoc
   crates' Rust sources, every repository `.js` file under 70 KB, and a
   generated and hand-written edge corpus (8,346 inputs). 8,304 matched
   exactly (2,181 compiled, 6,123 identical errors); no input changed between
   compiling and failing, and no module differed. The 42 others are error
   texts for programs holding a construct outside the subset (both sides
   reject, with no output), two of which made the bridge panic (a string
   token whose unquoted text is a lone quote) and now return an error. Other editions
   plug into the same views: they share the rule names, es5's flat
   `arguments` is read like `argument_list`, and an unknown rule classifies
   as unsupported.
4. **6d.** Add the `ecmascript/releases.csv` rows and make `EsVersion`
   selection real (`compile_source_at`). Remove the hard-coded `Es2020`.
5. **6e.** Add `Language::JavaScript` to lang-aot, with matrix rows. JS numbers
   are `f64`, so B(S) for a printing subset depends on LANG83's `f64`
   formatting ops (`f64_to_str_js` / `write_f64_js`, LANG83 PR6). Until then,
   subsets are limited to what LANG83 has registered.
6. **Later.** E7 goals replace the F10 approximation for full JavaScript.

### Twig and McCarthy Lisp: rewrite to walk `GrammarASTNode`

Both are rewritten incrementally, one construct family per PR. The gate is
**IIR identity**: for every program in the existing test corpus and
`lang_matrix`, the CST-walking compiler emits IIR byte-identical to the
current pipeline. The old path is deleted once identity holds across the whole
corpus.

- `twig_parser::Program` stays for its 13 dependent crates (twigc, the type
  checker, SIR11, and others), but not on the IIR path.
- The Twig type-check pre-pass (TW05-B) currently consumes the typed AST.
  See [Open questions](#open-questions).
- McCarthy Lisp's `extract_program` is replaced by borrowed views over the
  CST. **Done** in `mccarthy-lisp-iir-compiler` 0.9.0, as one PR: the
  compiler takes `parse_to_cst`'s `GrammarASTNode` (`compile_ast`), reads
  `'X` and `list_body` children as cons structure through borrowed cursors
  (no owned tree), runs capture analysis over the CST (C1.7), and keeps the
  extractor's error cases in a validation pass. IIR identity was checked by
  diffing the old and new compilers' output over every string literal in
  the McCarthy and `lang-aot` test sources plus a hand-written edge-case
  corpus. `extract_program` and `LispExpr` stay public for other consumers.

### Python and Perl: make release selection real

- **Perl.** `perl-lexer` and `perl-parser` compile every non-pending pair
  through the deduplicating generator; today the versioned pairs are only
  loaded from disk in tests. Add `compile_source_at`. Before `DEFAULT_RELEASE`
  can move off the unversioned pair, a `perl5.38.0` pair grounded in tag
  `v5.38.0` replaces the bounded LANG81 execution grammar. The 117 partial
  pairs all cover `pl-decimal-print`, so they can reach `I(pl-int-print)` as
  soon as the corpus exists.
- **Python.** Inventory the releases. Add per-release pairs for the subset,
  earliest first. The existing minor-line pairs (`python3.12.*` and the
  others) are not tagged releases. They stay, for the legacy
  `parse_python(source, "3.12")` API only, until the per-release pairs of
  their line exist, and are then deleted. They are never selectable through
  the release API.

**Phase 4a result (2026-10-20).** Perl's default release is declared on all
eight executable `lang_matrix` columns and passes the LANG83 check on all
nine backends, and release selection is real in the frontend. CLR and BEAM
execute in CI; the authoring host had no `dotnet` or `erl`.

- `grammar-tools generate-rust-compiled-grammars perl` is the first
  inventory-driven family. It reads the release list from `releases.csv` by
  header name, compiles every non-pending pair, canonicalizes each (comment
  lines dropped, `line_number` zeroed), and emits one `g_<12 hex of SHA-256>`
  module per distinct grammar. The 117 checked pairs collapse to 5 token and
  2 parser tables. Generated: `INVENTORY_RELEASES` (all 776 IDs),
  `SUPPORTED_RELEASES` (the 117), `token_grammar(release)` /
  `parser_grammar(release)` returning `None` for pending and unknown IDs, and
  `unversioned_token_grammar()` / `unversioned_parser_grammar()` for the
  LANG81 pair, which is never reachable as release `""`.
- `perl_iir_compiler::compile_source_at(source, release, module_name,
  &FrontendBudgets)` resolves `release` in `release.rs` (exact IDs only, no
  version ordering), then lexes and parses with that release's tables. Typed
  `FrontendError` variants: `UnknownRelease` (with up to five
  longest-prefix suggestions, caller text truncated to 64 characters with
  control characters escaped),
  `ReleaseNotAvailable { stage: "pending" }`, `Budget`, `Lex` and `Parse`
  (line and column), `Unsupported` (rule plus start and end span). All name
  the release. `compile_source` delegates with `None`.
- `print` lowers to `call_builtin write_i64`; the Perl-only `pl_print_int`
  builtin is gone, so the module holds only LANG83 registry builtins and
  ungated scalar ops (`const`, `neg`, `add`, `sub`, `mul`, `ret_void`).
- `Language::Perl` (`--lang perl`/`pl`, `.pl`) compiles with
  `compile_source_at(.., None, ..)`. Eleven `lang_matrix` rows declare all
  eight executable columns. `perl_rows_agree_with_the_host_perl_oracle`
  checks every expectation against the host `perl` (v5.38.2 locally), and
  `perl_rows_pass_the_lang83_check_on_all_nine_backends` runs
  `check_module_for` on all nine backends. The matrix WASM host and VM/JIT
  runners gained `write_i64` on the shared ordered sink.
- Per release: every one of the 117 checked releases lowers the frontend's
  corpus to IIR identical to the default's and runs it on vm-core. Tests also
  show the releases differ where their sources say they do: a CR is a lex
  error under 5.003_26 and whitespace under 5.004_72, and a 251-digit literal
  lexes under 1.000 but not under 5.004_72. The per-crate tests pin the
  compiled tables against the inventory and against each pair parsed fresh,
  which acts as the regeneration check for this family.

Divergences from this spec, deferred deliberately:

- `DEFAULT_RELEASE` was `Release::Unversioned`, not an inventory ID, so C4.1
  and C4.4 were not met for a compile that named no release. A
  `perl5.38.2` partial pair already existed whose token and parser grammars
  equal the unversioned pair apart from comments. *Resolved in 4b: the owner
  chose to build the `perl5.38.0` pair this section asks for.*
- `IIRModule` had no `release` field, and lang-aot had no
  `compile_source_to_iir_at`, `--lang perl@<release>` or `UnknownRelease` /
  `ReleaseNotAvailable` variants. *Resolved in 4b.*
- No `ReleaseProfile`: the integer `print` subset has no semantic difference
  between releases. The first profile comes with the first construct that
  needs one.
- `/`, `%` and `int()` are not in the subset, so 4a emits no
  `i64_div_floor`/`i64_mod_floor` and no matrix row traps. Perl's `/` is
  float division and needs the LANG83 float family.
- The CI `rust-grammar-regen-check` job does not list `perl`; the crate
  tests above stand in for it. The 117 pairs are not yet swept across the
  backends (4c), and `subsets.csv`/`iir_status` wait for phase 2b.

**Phase 4b result (2026-10-20).** Every Perl compile now resolves to an
inventory release, including one that names none, and the release is
selectable and recorded end to end.

- **Default release.** `DEFAULT_RELEASE` is `5.38.0` (`DEFAULT_RELEASE_ID`),
  a new partial pair built the LANG82 way from the `v5.38.0` tag's own
  blobs, retrieved 2026-10-20 through a blob-less fetch of the tag: tag
  object `f47267bec5921b10a8cc16545c9fdf11972c5f2d`, commit
  `76298ae68aa7796f0ffc05095b127d23f4b2de8f`, `perly.y`
  `c0c09092c5e9305af5638535effebf18ea62cfe3`, `toke.c`
  `1f81b3614c9a186468098cb15216c1dfe7ab3f2c`, `keywords.h`
  `feae6f6dce7ac861ac1ca207c097e51209a2191b`, `parser.h`
  `46d6a19c7b1e8638c26ceaa8b30dfa3e75b8f62c` (blob IDs re-hashed locally
  with `git hash-object`). The pair's rules and what grounds each are in its
  header and in LANG82. The decisive difference from the unversioned pair is
  `scan_num`'s 250-digit decimal bound (`tokenbuf[256]` minus six reserved
  bytes), which `perl.tokens` lacked. Its rules equal 5.004_72's, so the
  generator puts both in one table class (118 pairs, still 5 token and 2
  parser tables). The `releases.csv` row is `partial` (LANG82 header; the
  LANG84 columns wait for 2b). `v5.38.2` has the same three syntax blobs, so
  the host `perl` v5.38.2 oracle runs the same tokenizer and grammar.
- **C4.5.** `perl_iir_compiler::Release` is a newtype over one checked
  inventory ID, obtainable only from `resolve_release` or `DEFAULT_RELEASE`.
  `Release::Unversioned` is gone, so the IIR path cannot reach `perl.tokens`
  or `perl.grammar`. Those stay for `tokenize_perl`/`parse_perl`.
- **C4.1.** `interpreter_ir::IIRModule` has `release: Option<String>`.
  `compile_ast_at` sets it. All 161 struct literals elsewhere in the
  repository (14 crates, doc examples included) name `release: None`.
- **Selection API.** `lang_aot::compile_source_to_iir_at(language, release,
  source, module_name)`, with `compile_source_to_iir` delegating with `None`.
  `LanguageRequest { language, release }` and `LanguageRequest::parse`
  split `--lang perl@<release>` on the first `@`. Every `compile_file_to_*`
  entry point takes `impl Into<LanguageRequest>`, so the CLI's release
  reaches every emit mode. New `LangAotError::UnknownRelease { language,
  release, nearest }` and `ReleaseNotAvailable { language, release, stage }`
  are mapped from the frontend's variants. Release text is bounded to 64
  characters, and control characters are escaped before it is echoed.
- **C8.** `perl_iir_compiler::cst` names every rule, token type and spelling
  the compiler dispatches on (`CST_RULES`, `CST_TOKEN_TYPES`,
  `CST_TOKEN_SPELLINGS`), and the compiler uses those constants.
  `tests::cst_contract` checks every checked release's compiled grammar
  against them and parses the corpus into exactly the destructured shapes.
- **C5.** `tests::hostile_probes` re-runs the test binary as a child (77
  probes over the default, 1.000 and 5.004_72). The parent fails on a signal
  (verified by planting an unbounded-depth parse, which the child reported as
  SIGABRT), on a non-zero exit, or on a missing completion marker. It kills
  a child still running after 600 s, and it clears `RUST_MIN_STACK` so the
  probes run on the default test stack.
- **Security review.** A sibling session reviewed the change and found
  nothing exploitable: three low and three informational findings, all
  addressed. `LangAotError`'s `Display` neutralizes release text a caller
  built itself, without escaping twice. `Language::parse` bounds what it
  echoes. New tests show that lex errors never echo control or bidi
  characters, and that near-miss release spellings are `UnknownRelease`.

Divergences from this spec in 4b:

- **C4.4.** CI cannot assert a non-empty `iir_status` for the default yet.
  Perl's `releases.csv` kept the LANG82 header until the 2b migration, and
  since 2b its `iir_status` is `-` until 4c proves stage I. The
  tests assert instead that `DEFAULT_RELEASE_ID` is a checked
  (non-pending) row with compiled tables and resolves to itself.
- **C8.** "Every release whose `iir_status` is non-empty" is read as every
  checked release, for the same reason. The shape assertions run on the
  frontend's own corpus, because `code/grammars/perl/corpus/` waits for 2b
  and 4c.
- **Selection API.** `Language::parse` does not split on `@`.
  `LanguageRequest::parse` does, because `Language` is a `Copy` enum used as
  a value and as a key in tests. The source-level helpers
  (`compile_source_to_llvm`, `_wasm`, `_jvm`, …) still take a plain
  `Language` and use the default release. Release-aware code calls
  `compile_source_to_iir_at` and then the `compile_iir_to_*` entry point. A
  release for a language with no inventory is `UnknownRelease` (empty
  `nearest`), not a separate variant.
- **`IIRModule.release` is not serialised.** The binary format stays 1.1,
  and `deserialise` yields `None`, as for the transient profiling fields.
  Carrying it would need a format bump that the other IIR readers must
  accept first.
- **C5 budgets.** The 16,384-item CST budget, which the compiler applies to
  every tree including source-derived ones, also bounds flat operator
  chains and statement lists: about 2,000 terms compile and 20,000 are a
  typed `Unsupported`. The compiler folds `{ }` repetitions iteratively and
  allocates no nested tree, so no separate 512-term spine budget applies.
- **Known gap, not fixed here.** `perl5.38.2` has the same `toke.c` blob as
  5.38.0 but keeps an unbounded `INT`. `perl5.44.0` and `perl5.45.3` also
  have unbounded `INT` rules, and their evidence is pod files, not
  `toke.c`. Correcting them is a separate LANG82 follow-up.

**Phase 4c (release sweep).** B(`pl-int-print`) for the checked releases,
bounded per [CI bounding](#ci-bounding):

- **Corpus.** `code/grammars/perl/corpus/pl-int-print/` holds the eleven
  `lang_matrix` programs as positive cases, a CRLF layout case, and negative
  cases whose category is the same under every checked release. Every case
  has a `.cite`. `.expect` holds the exact stdout plus one final LF that is
  not part of it (Perl's `print` subset never writes a newline).
- **Profiles.** `perl-iir-compiler/src/profiles.rs` defines the
  `ReleaseProfile`s the `profile` column names, mapped from exact IDs only.
  The one semantic switch the subset needs is the sign of a computed zero.
  Before PERL_PRESERVE_IVUV, Perl evaluates `+`, `-`, `*` and unary minus on
  C doubles, so `-0` and `0 * -3` are IEEE negative zeros, and whether
  `print` shows `0` or `-0` depends on the release's constant folding and on
  the platform's `printf`/`Gconvert` (pinned sources in `profiles.rs`). Under
  that profile the compiler tracks each value's zero sign and refuses to
  print a negative zero (`Unsupported`), so a program whose output the
  release does not pin down never compiles. Cases that print one carry
  `.reject@<profile>`.
- **Sweep.** `perl-iir-compiler/tests/release_sweep.rs` compiles every case
  under every checked release, groups the releases into IIR-identity classes
  (releases whose outcome, IIR digest or rejection, is the same for every
  case; the `release` field is not serialised), and compares the result with
  the committed `code/grammars/perl/release_classes.csv`
  (`PERL_RELEASE_SWEEP_WRITE=1` rewrites it). A class's `class` cell is the
  first 16 hex digits of the SHA-256 of its whole outcome vector; a case the
  release does not accept is `rejected`. `tests/release_subsets.rs` proves
  stage I for every release whose `iir_status` claims the subset.
- **Matrix.** `lang_matrix`'s `Prog` gains `release` and `case`. The eleven
  Perl rows name `5.38.0` and their case; new rows run every case each class
  representative accepts, on all eight executable columns.
- **CSV.** `iir_status`, `backend_status` and `profile` are filled for every
  release the sweep, the stage-I test and `check --matrix` prove.

**Phase 4c result (2026-10-20).** All 118 checked Perl releases claim
`pl-int-print` at stages I and B in `releases.csv`, and every claim is
proven by a named test or by `check --matrix`.

- **Corpus.** 12 positive and 7 negative cases. The positive cases are the
  eleven 4a matrix programs plus `crlf_layout` (byte-exact in git through a
  `-text` attribute; under the repository's `eol=lf` it would otherwise be
  normalized on commit). The negative cases are refused with the same
  category under every release: `Parse` for a decrement, an octal literal,
  `print 1;` and a missing `;`, and `Unsupported` for three out-of-range
  values. Every `.cite` names host `perl` v5.38.2, and the stage-I test
  checks every `.expect` and every negative case's recorded Perl behaviour
  against it.
- **Profiles.** `perl_nv` (114 releases: 1.000 to 5.004_05, 1.0.15, 1.0_16)
  and `perl_iv` (5.38.0, 5.38.2, 5.44.0, 5.45.3), with the pinned sources
  (tag, commit, file and line, read 2026-10-20) in `profiles.rs`. Fact 1:
  every non-zero value and every positive zero prints as its exact decimal
  integer in every checked release (doubles hold these integers exactly,
  and `%.20g`, `Gconvert(.., DBL_DIG, ..)` and `%ld` print them exactly).
  Fact 2: the negative zero is where releases differ, so `perl_nv` refuses
  to print one. `zero_and_negation.reject@perl_nv` records that refusal.
- **Sweep.** Three classes, which differ only in the cases they accept.
  Every release that accepts a case lowers it to the same IIR digest.

  | Representative | Members | Not accepted |
  | --- | --- | --- |
  | `1.000` | 41 (`perl_nv`, CR is whitespace) | `zero_and_negation` |
  | `5.003_26` | 73 (`perl_nv`, CR is a lex error through 5.004_71) | `zero_and_negation`, `crlf_layout` |
  | `5.38.0` | 4 (`perl_iv`) | — |
- **Matrix.** 33 Perl rows (12 cases on 5.38.0, 11 on 1.000, 10 on
  5.003_26), 264 cells. `check --matrix` reports 118 claims: 33 cells
  direct, 1,196 by IIR class and 187 not applicable. Every runner compiles
  with the row's release, and the full non-ALGOL aggregate passes with the
  change. CLR and BEAM ran only in CI; the authoring host had no `dotnet`
  or `erl`.
- **The INT follow-up from 4b is fixed.** `perl5.38.2` is a mechanical copy
  of `perl5.38.0` (identical syntax blobs). `perl5.44.0` and `perl5.45.3`
  bound `INT` at 1018 digits, because `tokenbuf` grew to
  `256 * MAX_UNICODE_UTF8_BYTES` bytes after 5.38. See LANG82.
- **C4.4 and C8 now hold as written.** The default `5.38.0` claims
  `iir_status` (`the_default_release_claims_pl_int_print`). "Every release
  whose `iir_status` is non-empty" is now the same set as every checked
  release, which is what `cst_contract` walks.
- **Security review.** A sibling session reviewed the change: verdict PASS,
  no finding above low, all fixed. `release_inventory.py` now takes profile
  names only from `name: "<id>"` fields of a `profiles.rs` (matching any
  word let `pub` or a comment word pass as a profile), refuses two overrides
  for one profile, a corpus file over 64 KiB and a CR not followed by LF.
  The sweep's write switch fails the run that writes and refuses CI. The
  Rust corpus readers repeat the checker's path and symlink rules. Every
  claiming release must run a baseline case. The CRLF case and its
  `.gitattributes` line are pinned. The host-`perl` oracle runs each corpus
  file with a cleared environment, a scratch directory, no stdin and a
  timeout.
- **Merge fix.** The 4b hostile probes counted a one-instruction `ret_void`
  epilogue. Since 4a's `main` returns `i64` 0 (`const 0; ret`), the epilogue
  is two instructions.

Divergences from this spec in 4c:

- **Classes are per release, over the whole corpus.** "CI bounding" groups
  releases by "the hash of the produced IIR". 4c takes that hash over a
  release's outcome for every case, an IIR digest or `rejected`, so a class
  is the set of releases that are identical for every case. `class` holds
  that 16-hex digest for each case the release accepts. The checker's
  per-case representative is therefore the class's first release.
- **Boundary releases.** The default release is a representative, so it
  runs on every column. The newest release (5.45.3) does not get rows of
  its own. It is in the default's class, and the class's identical IIR
  stands for it. There is no nightly tier: all three representatives run on
  every PR, at 33 rows.
- **`perl_nv` is conservative for 5.001 to 5.004_05.** In those releases a
  folded integral `double` constant becomes an IV, so `print(-0)` most
  likely prints `0`. The profile still refuses every negative zero rather
  than depend on which side of the folding changes each untagged
  development release falls (5.000 has no fold to IV, and 5.005 stops
  folding `OP_NEGATE` results).
- **No `Lex` category among the negative cases.** I(S) names `Unsupported`
  or `Parse`. The 250/251-digit boundary is not a corpus case because its
  category depends on the release: `Parse` where `INT` is bounded,
  `Unsupported` in the unbounded early pairs. It stays in the unit tests
  and the hostile probes.
- **`cst_contract`'s shape assertions still use the frontend's own
  corpus**, not `corpus/pl-int-print/`. The two hold the same programs.
- **CI triggering.** A change confined to `code/grammars/perl/` (the corpus,
  `releases.csv`, `release_classes.csv`) maps to no package. On such a
  change the contracts job's `check --matrix` runs, but `release_sweep` and
  `release_subsets` run only when a Perl crate changes. A path gate for
  them is a follow-up.

### Wire every IIR frontend into lang-aot

Add `Language` variants with release support for Python, JavaScript, Ruby and
Perl, and for Maple, Reduce, Wolfram, Axiom, Derive and MacroNib. Maxima waits
for its own grammar. Each variant comes with at least one `lang_matrix` row on
its default release.

### Preprocessor dialects

MacroOct and MacroNib run a hand-written directive layer (`dialect.rs`, about
1,309 and 208 lines) on the PREP01 `source-preprocessor` engine before the
base language's GL and GP. LANG84 classifies a PREP01 dialect as a
**preprocessor**: a separate language layer, not a lexer or parser for the
target language. It is permitted only under all of the following conditions:

- its output feeds the base language's compliant GL and GP unchanged
- it does not emit IIR
- an IIR-identity test (already present in
  `macrooct-iir-compiler/tests/iir_identity.rs`) ties the dialect to its
  hand-expanded base program

A later PREP spec MAY move directive syntax into its own
`.tokens`/`.grammar` pair. LANG84 does not require it.

## Release inventories to create

| Language | Directory | Approximate rows | Source of truth | Cross-check |
| --- | --- | --- | --- | --- |
| Ruby | `ruby/` | ≈1,196 tags: ≈178 final teeny (`vX_Y_Z`/`vX.Y.Z`, latest `v4.0.7`), 853 patch-level (`vX_Y_Z_P` → `X.Y.Z-pP`), ≈96 preview/rc | `git ls-remote --tags https://github.com/ruby/ruby` | ruby-lang.org `/en/downloads/releases/`; tags that are not releases are excluded with reasons |
| Python | `python/` | ≈677 tags: ≈301 final, the rest a/b/c/rc | `git ls-remote --tags https://github.com/python/cpython` | python.org/downloads; the earliest tag (0.9.x line) is recorded as found |
| Perl | `perl/` | 776 rows exist | pinned `perlhist.pod` plus annotated tags (LANG82) | Reconcile against 272 `vX.Y.Z`, 120 RC and ≈90 legacy `perl-*` tags. Every tag maps to a row or an excluded-with-reason line |
| JavaScript | `ecmascript/` | 29: ECMA-262 editions 1, 2, 3, 5, 5.1, 2015–2026 (17; ES4 abandoned and excluded with reason) and Netscape/Mozilla JavaScript 1.0, 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.8.1, 1.8.2, 1.8.5 (12) | ECMA archive PDFs (editions 1–5.1), tc39/ecma262 tags `es2016`…`es2026`, MDN and SpiderMonkey release notes and source | The repo has no pairs for es2, es5.1, es2026 or any Netscape version |
| Dartmouth BASIC | `dartmouth_basic/` | editions 1–7 | Dartmouth manuals (archived scans) | — |
| ALGOL 60 | `algol/` | 1960, 1963, 1976 | the reports | ALGOL 58 and 68 are separate languages (open question) |
| COBOL | `cobol/` | 60, 61, 61-ext, 65, 68, 74, 85, 2002, 2014, 2023 | CODASYL and ANSI/ISO standards | — |
| Maxima | `maxima/` | Maxima git tags | SourceForge git | — |
| Maple, Wolfram (Mathematica), REDUCE, Derive, Axiom/FriCAS, Macsyma | each | vendor versions or tags | vendor release notes; REDUCE and FriCAS repositories | Evidence is weaker for proprietary systems; rows may stay `pending` until evidence exists |

Inventory PRs are CSV and README only. Every row starts at `grammar_status`
`pending` unless a pair already exists and has been checked.

**Phase 2a result (2026-10-20).** Every row in the three new inventories is
`pending`, stage `none`.

- `ruby/` has 1,189 rows. Seven of the 1,196 tags are excluded: branch, freeze
  and CVS-import markers, plus one conversion artifact. Most tag-only patch
  levels date from their tag commit, marked `?`. `v1_9_0_N` keeps the upstream
  tarball spelling `1.9.0-N`, not `-pN`.
- `python/` has 657 rows. The 20 excluded tags are retired-branch heads.
  Thirteen releases with NEWS entries but no tags are listed in the header.
- `ecmascript/` has 29 rows. The existing `es*` pairs do not cite their
  sources, so they have not reached stage G.

Each directory README gives the date sources, the lineage rules (`line`,
`branch-point`, `curated`) and the remaining gaps.

## Phased roadmap

Only one implementation PR is active at a time (LANG78 practice). The sizes
are rough estimates of reviewed diff size, not counting regenerated
`_grammar.rs` or CSV rows.

| Phase | PR(s) | Content | Size |
| --- | --- | --- | --- |
| 1 | 1 | This spec, plus cross-links from LANG78 and LANG82 | doc only |
| 2 | 2a | `releases.csv` and README for Ruby (1,189 rows), Python (657) and ecmascript (29), plus the first `release_inventory.py check` and its CI hook | ~1,900 CSV rows, ~200 lines of prose, ~400 lines of checker and tests |
| | 2b | **Done:** `subsets.csv` scaffolding, Perl CSV schema migration (including the perl-parser tests that match `,partial` row endings), `--matrix` | ~600 lines |
| | 2c | `release_evidence.py` (blob-ID capture per release) | ~400 lines |
| 3 | 3.1 | E1 line/column guards (+ F-spec) | ~800 lines Rust + tests |
| | 3.2 | Ruby R0: `ruby<earliest…>` partial pairs for `rb-int-puts`, mechanically materialized from evidence; differential tests | ~600 lines + pairs |
| | 3.3 | E2 pair-kind stack and depth-guarded push/pop; delete the JS template special case | ~1,200 lines |
| | 3.4 | E3 conditional skip (optional Wolfram/Derive hook removal) | ~400 lines |
| | 3.5 | E4 capture-then-literal-match | ~1,000 lines |
| | 3.6 | Ruby R1–R3 | ~800 lines per slice |
| | 3.7 | E5 deferred bodies; Ruby R5 | ~1,000 + ~600 lines |
| | 3.8 | E6 adjacency predicates and ASI | ~800 lines |
| | 3.9 | E7 goal-directed lexing and binding predicates (largest; may split into lattice and bindings) | ~2,500 lines over 2–3 PRs |
| | 3.10 | Ruby R4, R6; delete the `ruby-lexer` state machine | ~1,500 lines |
| 4 | 4a–4c | **Perl on all backends** (= LANG83 PR3, after LANG83 PR1b). **4a done:** compile versioned pairs (deduplicating generator); `compile_source_at`; `print` → `write_i64`; `Language::Perl`; matrix rows. **4b done:** default on the inventory release `5.38.0` (new pair grounded in tag `v5.38.0`); `IIRModule.release`; lang-aot release API; `cst_contract`; child-process hostile probes. **4c done:** `pl-int-print` corpus, `ReleaseProfile`s, release sweep (3 IIR-identity classes), `Prog.release`/`case`, I and B for all 118 checked releases. Fastest win, because Perl is already compliant | ~1,500 lines over 3 PRs |
| 5 | 5a–5b | **Ruby integer subset on all backends** (= LANG83 PR2), on R0 pairs: `rb_int_div` → `i64_div_floor`, `puts` → `print_i64`, `StdoutThenTrap` rows; `Language::Ruby`; the IIR path drops `ruby-lexer` | ~1,200 lines over 2 PRs |
| 6 | 6a–6e | JavaScript CST rewrite (see above; 6c, the compiler rewrite, done in `javascript-iir-compiler` 0.3.0) | ~3,000 lines over 5 PRs |
| 7 | 7a–7c | Python: inventory pairs for the subset, `compile_source_at`, `Language::Python`, matrix rows. The float subset depends on LANG83 float formatting; an int subset depends on the LANG83 `*_ovf` family | ~2,000 lines over 3 PRs |
| 8 | 8.x | Twig CST rewrite (≈5,700-line compiler, ~6–10 PRs by construct family, IIR-identity gate). McCarthy Lisp CST rewrite (~3 PRs; done in one, `mccarthy-lisp-iir-compiler` 0.9.0) | large |
| 9 | ongoing | Release backfill, earliest first: per-release pairs and subset status for every language; remaining inventories (BASIC, ALGOL, COBOL and others); remaining lang-aot variants | many small PRs |

Phases 4 and 5 may start once 2b (and, for Ruby, 3.2) has landed. They do not
wait for the rest of phase 3.

## Open questions

1. **Typed views.** C1.6 allows borrowed, non-allocating CST views (in the
   style of rust-analyzer's typed AST over a CST). Confirm that this meets
   "the IIR compiler must walk the parse tree", or forbid views and require
   raw `GrammarASTNode` matching.
2. **The Twig type checker.** TW05-B's pre-pass consumes
   `twig_parser::Program`. Port it to the CST, or run it on the typed AST only
   as a *checker* whose results do not feed IIR emission? The second option
   would keep a typed lift beside the IIR path.
3. **The preprocessor exception.** Is PREP01's hand-written directive layer
   acceptable under the conditions in
   [Preprocessor dialects](#preprocessor-dialects)?
4. **Release versus language identity.** Are ALGOL 58/60/68, LISP 1/1.5, and
   Macsyma versus Maxima one language with releases or separate languages?
   LANG84 assumes ALGOL 60 report revisions and LISP 1/1.5 are releases, and
   that ALGOL 58/68 and Maxima are separate languages.
5. **In-repo languages.** Twig, Nib, Oct, MacroOct and MacroNib have no
   upstream tags. Proposed: a single row named for the defining spec version,
   with `source_ref` = spec path + commit, until the language defines
   versions.
6. **Selection shorthands.** Should `ruby@2.7` resolve to the newest release
   on that line (printed in diagnostics), or should only exact IDs be accepted?
   LANG84 v1 accepts only exact IDs.
7. **Compiled-table size.** If ~100 distinct Ruby grammars make
   `ruby-parser` too large or slow to build, should eras be cargo features
   (`releases-1x`, `releases-2x`, …) with the default release always built?
8. **Mechanical materialization.** `release_evidence.py` copies pairs for
   blob-identical releases. Is that sufficient LANG82 evidence, or must each
   release still get an individual review note?
9. **Exact Ruby binding rules.** Named regex captures (`/(?<x>…)/ =~ s`
   declares `x`), `for x in`, `rescue => e`, pattern-matching captures, and
   numbered and `it` parameters need `@declare` placements. They are
   confirmed in the E7 PR against `parse.y` per release.
10. **Pre-1.0 releases.** Ruby 0.95 and the earliest Python 0.9.x releases may
    not exist as upstream tags. Are untagged but published releases in scope?
    LANG84 v1 covers tags only and records known untagged releases in the
    inventory header.
