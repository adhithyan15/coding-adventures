# PREP01: a generic source preprocessor engine

**Status:** Draft — design spec, committed before implementation (sign-off = merge).

## 1. Why this exists

Several languages in this repo need a text- or token-level rewriting pass that
runs *before* parsing. Today none of them have one, and at least one frontend is
explicitly blocked on it:

- **C** — `code/specs/SIR27-c-to-semantic-ir.md` §"Preprocessor" scopes the v1
  frontend to "only `#include <stdint.h>` / `#include <stdio.h>` (ignored). No
  `#define`, no macros, no other directives," and its non-goals list names "the
  full preprocessor." The Rust `c-lexer` does not merely ignore directives, it
  *discards* them, and not even via a hook: `code/grammars/c/c.tokens` line 123
  carries `PREPROC = /#[^\n]*/` inside the grammar's `skip:` section, so every
  directive line is dropped before the parser or any `post_tokenize` hook could
  see it. There is no real C until this exists.
- **COBOL** — `COPY … REPLACING` is a text-substitution facility in the
  standard. `cobol-lexer` / `cobol-parser` implement none of it: `COPY` is not
  in the COBOL keyword list at all, and the only `REPLACING` present belongs to
  the runtime `INSPECT … REPLACING` statement, which is a different feature.
- Others in the same shape: PL/I's `%INCLUDE`, Fortran's `INCLUDE`, Erlang's
  `-define`/`-include`, and assembler macro facilities.

Writing one preprocessor per language duplicates the genuinely hard parts (the
source map, the non-recursive expansion algorithm, include-cycle and
resource bounds) four or five times, each with its own bugs. This spec defines a
shared **engine** with per-language **dialect** plug-ins.

## 2. What is shared and what is not

This is the central design question. A preprocessor that tries to be universal
at the *semantics* layer serves no language well: C macro expansion and COBOL
`COPY … REPLACING` pseudo-text matching are different algorithms with different
matching rules, and unifying them would produce a facility that is wrong for
both.

**Shared (this engine owns it):**

| Concern | Why it is shared |
|---|---|
| Source map | Keeping a token's true origin across splicing, inclusion and expansion is the same problem everywhere, and is brutal to retrofit. |
| Physical→logical line splicing | Backslash-newline (C), and the general shape of continuation. |
| Include resolution | Search-path policy, cycle detection, depth bounds, virtual filesystem. |
| Conditional-compilation stack | `if`/`elif`/`else`/`endif` nesting, skipped-group tracking, "skip but still track nesting" rules. |
| Macro table + expansion driver | The hide-set ("blue paint") non-recursive expansion algorithm is subtle, identical wherever token-level macros exist, and is the single most-commonly-miscoded part of a preprocessor. |
| Resource bounds | Expansion bombs, include bombs and path escapes are engine-level safety, not per-language policy. |

**Per-language (the dialect supplies it):**

| Concern | Example divergence |
|---|---|
| Directive syntax and names | `#define` vs `-define(…)` vs `COPY … REPLACING`. |
| Tokenization | Each frontend already owns its `.tokens` grammar. |
| `#if` expression semantics | C's integer constant expressions with `defined()` are not PL/I's. |
| Stringize / paste | C has `#` and `##`; COBOL pseudo-text has neither. |
| Replacement matching | C substitutes by parameter name; COBOL matches pseudo-text runs. |

**Explicitly not unified:** COBOL's `COPY … REPLACING` will use the include,
source-map, dispatch and bounds layers and bring *its own* replacement matcher.
That is the intended shape, not a shortfall. Slice 4 exists specifically to
prove the trait boundary is not C-shaped.

## 3. Layer position, and a correction to an existing spec

`code/specs/lexer-parser-hooks.md` already anticipates this work. It defines
`pre_tokenize` (`str → str`) and `post_tokenize` (`[Token] → [Token]`) hook
chains on the shared lexer, and names as use cases "C #include file insertion"
and "line splicing" at `pre_tokenize`, and "C #define macro expansion",
"C #ifdef conditional compilation" and "Token pasting (C ## operator)" at
`post_tokenize`.

**That split is not faithful to C, and this spec corrects it.** Inclusion cannot
be a text-level pass that runs before conditional compilation, because the two
are mutually dependent:

```c
#if USE_FAST_PATH      /* whether the #include happens at all …   */
#  include "fast.h"    /* … and fast.h #defines symbols that …    */
#endif
#if FAST_VERSION > 2   /* … later #if directives test.            */
```

A text-level include pass would pull in `fast.h` unconditionally, and a
token-level conditional pass running afterwards could not have seen the macros
`fast.h` defined in time to affect the inclusion decision. Real C is defined as
a single ordered traversal in which inclusion, conditional selection and macro
definition all interleave.

Therefore:

- **`pre_tokenize` carries line splicing only** (and, for COBOL, the existing
  column strip). These are genuinely context-free text transforms.
- **`post_tokenize` carries the whole preprocessor as one pass.** The engine
  owns include resolution itself and re-lexes included files through the
  dialect's own lexer, rather than pasting their text in beforehand.

The hook *signature* still fits — the engine is constructed with its
configuration and hands out a closure of the required `[Token] → [Token]`
shape. But it would be an overclaim to call that closure **pure**, and
`lexer-parser-hooks.md`'s Design Principle 1 says "No side effects. No state."
The engine does filesystem I/O and carries a macro table, an include stack and
every §6 counter. Three consequences are therefore normative here:

- **The engine is the last `post_tokenize` hook.** §4's `Locus` vector is
  positional, and Principle 2 permits hooks registered after it. A later hook
  that reorders or synthesises tokens would silently misattribute every
  diagnostic's file and line. The map's length is checked against the token
  stream's length at consumption, so a mismatch is a hard error rather than
  silent misattribution.
- **The closure is single-use per translation unit.** At the start of each
  invocation the engine resets the macro table, the include stack and all §6
  counters, or refuses a second invocation. Otherwise a lexer reused across
  units leaks macro definitions between them, and the §6 counters either
  persist (unit 1 exhausts the budget and denies unit 2) or reset (N units each
  spend the full budget, defeating the cap in a batch compile).
- **The `SourceMap` is retrieved per invocation through an explicit accessor**,
  not through ambient shared state. The `[Token] → [Token]` signature has no
  channel to return it, and shared mutable state captured in a closure is
  precisely where a `Sync`/aliasing bug or a cross-unit leak would live.

`lexer-parser-hooks.md` gets an amendment noting the corrected ordering and
these constraints; its API is unchanged.

```text
source text
   │
   ├─ pre_tokenize:  line splicing (and COBOL column strip)
   ▼
dialect lexer  (existing per-language .tokens grammar)
   │
   ├─ post_tokenize: PREP01 engine — one pass:
   │                   include ⟷ conditionals ⟷ macro definition/expansion
   ▼
[Token] + SourceMap  →  existing parser (unchanged)
```

## 4. Source locations: a side table, not a wider `Token`

`lexer::token::Token` carries `line` and `column` but **no file identity**. Once
tokens can arrive from an included file or a macro body, `line:column` alone is
ambiguous.

The obvious fix — add a `file` field to `Token` — is rejected: **136 crates in
`code/packages/rust/` depend on the shared `lexer`**, so widening that struct is
a repo-wide blast radius for a feature two frontends need.

Instead the engine keeps a **side table**, the design GCC's line maps and LLVM's
`SourceManager` both use:

- Each emitted token keeps a **presumed** `line`/`column`: the location in the
  file whose text it came from. Consumers that do not care about inclusion see
  plausible, useful numbers, exactly as today.
- The engine additionally returns a `SourceMap` with one `Locus` per emitted
  token, resolving to `(FileId, line, column, expansion_chain)`, where
  `expansion_chain` records the macro invocations a token passed through.
  Diagnostics that need full fidelity ("in expansion of `FOO`, from `bar.h:12`,
  included from `main.c:3`") consult it.

`Token` is not modified. No existing frontend changes.

**The expansion chain must be interned, not owned per token.** A `Locus` that
owns its chain makes the map `O(tokens × expansion_depth)`, so with the §6
macro-depth bound of 200 a token cap of N still admits 200N chain entries — any
operator who sets the token cap believing it bounds memory would under-count by
up to 200×. The chain is therefore a shared immutable structure in a side
arena: each entry is an expansion id naming its parent expansion id, exactly as
LLVM's `SourceManager` does. That makes the map `O(tokens + expansions)`. This
is normative, because §6's memory bounds depend on it.

**Known limitation, stated deliberately:** the per-token `Locus` vector is
positional, so it is valid only for a consumer that does not reorder or
synthesise tokens between the engine and the parser. That holds for the intended
pipeline. If a future consumer needs to reorder, the map must move into `Token`
or into a wrapper type, and that will be a separate, scoped decision — not
something to guess at now.

## 5. Engine interface (Rust, `source-preprocessor` crate)

```rust
/// What a language plugs in. The engine never lexes, and never decides
/// what a directive means.
pub trait Dialect {
    /// Recognise a logical line as a directive. `None` = ordinary source.
    fn directive_of(&self, line: &[Token]) -> Option<Directive>;

    /// Evaluate a conditional's controlling expression.
    fn eval_condition(&self, toks: &[Token], macros: &MacroTable)
        -> Result<bool, PpError>;

    /// Lex an included unit's text. The engine calls this, never a lexer
    /// directly, so each language keeps its own grammar.
    fn lex(&self, text: &str, file: FileId) -> Result<Vec<Token>, PpError>;

    /// Stringize (C `#`). `None` = the dialect has no such operator.
    fn stringize(&self, toks: &[Token]) -> Option<Token> { None }

    /// Token paste (C `##`). `None` = unsupported.
    fn paste(&self, left: &Token, right: &Token) -> Option<Token> { None }
}

/// All file access goes through this. There is no ambient filesystem
/// access in the engine, so tests run fully in memory.
///
/// SECURITY CONTRACT. `resolve` MUST reject any request resolving outside a
/// declared search root. Because the engine never sees a path (see below),
/// this is an obligation of the implementation, not something the engine can
/// check. The crate ships `RootedFs` as the only sanctioned production
/// implementation; dialects MUST NOT supply their own `SourceFs`.
pub trait SourceFs {
    /// Resolves, opens and validates in one step. The returned `FileId`
    /// names a RETAINED, already-verified open handle — see below.
    fn resolve(&self, request: &IncludeRequest) -> Result<FileId, PpError>;
    fn read(&self, file: FileId) -> Result<SourceText, PpError>;
}
```

**`FileId` names a retained open handle, and that is load-bearing.** Splitting
the operation across two calls would otherwise reopen the very TOCTOU the rules
below exist to close: if `resolve` opened a handle, validated it and dropped
it, `read` would have to reopen, and every swap the handle check defeats — a
symlink or junction substituted into a directory component, the file replaced
after its size was checked — is live again in the window between the two calls.
So `resolve` opens the file, performs every check below against that handle,
and retains it; `FileId` indexes it; `read` reads *that* handle and never
re-resolves a path. An implementation that reopens in `read` must re-perform
the full identity and metadata verification on its own handle.

*Implementation note, not a security property:* retaining a handle per `FileId`
can hold open descriptors up to the §6 inclusion cap of 10 000, which on some
hosts exceeds the default per-process descriptor limit. Nothing requires
retention past `read`, so slice 1 should release the handle once the file's
text has been read and verified. If a descriptor limit is hit first it surfaces
as an ordinary `PpError` diagnostic — it fails closed — but it should not
surprise the implementation.

**`FileId` is opaque and constructible only by `SourceFs`.** It is handed to
dialects through `Dialect::lex(&self, text, file: FileId)`; a transparent
newtype over an integer would let a dialect mint a `FileId` naming a different
file and misattribute a token's provenance throughout the source map.

**The engine never sees a path, so §6's containment row is enforced here.**
That is a deliberate split — it keeps path policy in one auditable place — but
it means the guarantee is only as good as `RootedFs`, which must:

- **Verify containment on the opened handle, not on the path.** Canonicalise-
  then-open is TOCTOU by construction: a symlink swapped into a directory
  component between the check and the open wins. Open with symlink-following
  disabled (`O_NOFOLLOW` / `FILE_FLAG_OPEN_REPARSE_POINT`), or open
  directory-relative and verify the opened file's identity (device+inode /
  `FILE_ID_INFO`) lies under the root.
- **Reject, each with a located diagnostic:** absolute paths (`/etc/passwd`,
  `C:\…`); any symlink or reparse-point component; and — this repo is
  Windows-primary — UNC paths (`\\host\share\x.h`, which triggers an outbound
  SMB authentication and leaks an NTLM hash, a credential-disclosure primitive
  from nothing but a source file), NTFS alternate data streams (`x.h::$DATA`),
  reserved device names (`CON`, `NUL`, `COM1`…), 8.3 short names, and directory
  junctions.
- **Read only regular files.** A FIFO or character device is the most durable
  DoS available: `#include "/dev/stdin"` or an included FIFO blocks forever and
  no §6 bound fires, because the engine is stuck inside `read`. `/dev/zero`
  exhausts memory the same way. Directories, sockets, and block devices are
  rejected too. File size is checked from the opened handle's metadata *before*
  reading, and `read` must not block indefinitely.
- **Reject non-UTF-8 input with a diagnostic rather than converting lossily.**
  Silent replacement-character substitution changes the token stream. Embedded
  NUL bytes and a leading BOM are likewise the engine's business to decide
  explicitly, not the implementor's to guess.

`read` returns an owned `SourceText` handle rather than a `&str` borrowed from
`&self`. A borrow would retain every file for the engine's lifetime with no
eviction, and would push implementors toward arena allocation, `Box::leak`, or
interior mutability with `unsafe` — a poor shape for the one component that
touches attacker-controlled input.

## 6. Resource and safety bounds

These are engine-level and on by default. **Every bound has a finite default,
and the configuration surface is tighten-only — for the embedding host as much
as for a dialect.** Neither can raise a bound to infinity or disable one, so no
deployment can reach an unbounded token or fuel budget through configuration
alone. Each bound has a test in the slice that introduces it.

The organising principle: **bound work and bytes, not only shape.** Counting
nesting depth and emitted tokens leaves the dimensions an attacker actually
reaches for — expansion whose result is discarded, byte growth that does not
grow the token count, and fan-out that is never a cycle.

| Bound | Default | Failure mode it prevents |
|---|---|---|
| Include nesting depth | 200 | Deep include chains. (C requires ≥15 to work.) |
| Include cycle detection | always on | `a.h` → `b.h` → `a.h`. **Stack-based**: a file may not appear twice on the *active* include stack. It is not global dedup, and cannot be — C permits a header to be included many times. |
| **Total file inclusions** | 10 000 | The DAG include bomb, which defeats both rows above: `a.h` includes ten `b*.h`, each ten `c*.h`… At depth 8 — far inside the depth limit, and acyclic, so cycle detection never fires — that is 10⁸ file processings. §7 defers `#pragma once`, removing the one mechanism that would deduplicate it, so this counter is what holds v1 up. |
| **Total source bytes processed** | 256 MiB | Many small files rather than deep ones. |
| **Maximum bytes per included file** | 16 MiB | Checked from the opened handle's metadata before reading. |
| Path containment, regular-files-only, encoding | always on | See §5. Enforced in `RootedFs`. |
| Macro expansion depth | 200 | Mutually recursive function-like macros. |
| **Total tokens produced** | 64 M | Expansion bombs. Counts every token the expander *creates* — emitted, consumed by `eval_condition`, or discarded. Counting only *emitted* tokens leaves a hole: `#define A0 1` / `A1 A0 A0` / … / `A40 A39 A39` inside `#if A40` produces 2⁴⁰ tokens that are consumed by the condition and never emitted, against a depth of only 40. The counter is shared across directive evaluation and body expansion and is never reset mid-translation-unit. |
| **Maximum token spelling length** | 64 KiB | `stringize` and `paste` grow *bytes* while holding the token count flat, so a token counter is structurally blind to them. Nested pasting via an indirection layer yields identifier text exponential in source length from ~1 token. |
| **Total bytes of synthesised token text** | 64 MiB | Same class, aggregate. Both byte bounds are checked at the engine's `stringize`/`paste` call sites, not delegated to the dialect — a dialect's `paste` that allocates before returning is already past the bound. |
| **Macro-argument grouping nesting depth** | 200 | `F(((((…10⁶ parens…)))))` during argument collection. |
| **Controlling-expression nesting depth** | 200 | The same, inside `#if`. |
| Conditional nesting depth | 200 | Pathological `#if` nesting. |
| **Total expansion steps ("fuel")** | 2³⁰ | The catch-all. One monotonically decreasing budget decremented by every token copied, hide-set union, rescan and file read, checked in the engine's main loop. This is the most valuable row in the table, because preprocessor DoS historically arrives through whichever dimension nobody enumerated — and no list, including this one, is complete. |
| **Maximum diagnostic count / quoted-text length** | 100 / 1 KiB | A stringized megabyte-long token renders a megabyte-long message, and a cascade emits one per token. Truncating *diagnostic text* is distinct from the no-silent-truncation rule on token streams below. |

Three further requirements, normative because they constrain the engine's core
loop rather than adding a check to it:

1. **No native recursion.** The include, expansion, argument-collection and
   conditional traversals use explicit stacks. A recursive implementation turns
   a depth bound into a stack overflow, which in Rust is an abort — not a
   catchable panic, and not containable by any `catch_unwind` in an embedding
   host. This constrains `Dialect::eval_condition` implementations too — and
   because the dialect does that parsing, **the engine pre-scans the
   controlling-expression token slice for grouping depth before dispatching to
   `eval_condition`**, rather than trusting each dialect to self-police.
   Otherwise a dialect authored later quietly reintroduces the abort.
2. **Hide-sets are shared/persistent** — interned set ids or an immutable
   linked structure, never cloned per token. A naive owned hide-set makes
   expansion quadratic in tokens × active macros at no extra token count, i.e.
   invisible to every counter above.
3. **A skipped conditional group is not expanded.** Inside a group being
   skipped the engine does directive recognition and nesting tracking only: no
   macro expansion, no evaluation of nested controlling expressions, no include
   resolution. Otherwise every bomb above becomes reachable from inside
   `#if 0` — which is exactly where hostile input would put it, and where a
   human reviewer stops reading.

No input, malformed or otherwise, may cause a panic, an abort, an
out-of-bounds index, a non-terminating loop, or a silent truncation of the
token stream. Every bound and every malformed construct — unterminated `#if`
at EOF, `#endif` without `#if`, `#else` after `#else`, an unterminated macro
argument list, `#define` with no name, a bare `#` at EOF — produces a
diagnostic naming the construct and its source location.

## 7. Implementation sequencing

One slice per PR, following the discipline `AOT00-T2-exceptions.md` §12 uses.
Each slice states an executable acceptance criterion.

**Slice 1 — engine core, no macros.**
`SourceFs`, `FileId`, `SourceMap`/`Locus`, line splicing, include resolution,
the conditional stack, directive dispatch, and every §6 bound except macro
depth. Proven against a deliberately non-C **synthetic dialect** defined in the
crate's own tests, so nothing about the boundary can be quietly shaped by C.
*Acceptance:* a synthetic-dialect program exercising nested includes,
conditionals and every §6 bound preprocesses to an exact expected token stream;
each bound has a test proving it yields a located diagnostic; `RootedFs` has a
test per rejected path form in §5, including the Windows set; and **the engine
is fuzzed against arbitrary byte input with a no-panic / no-hang oracle**. The
fuzz target is the highest-value item in this slice, because it covers the
dimensions §6 did not manage to enumerate.

**Slice 2 — macro expansion.**
`MacroTable`, object-like then function-like macros, argument pre-expansion, and
the hide-set algorithm that makes expansion non-recursive. `#`/`##` are routed
through the dialect hooks, not built in.
*Acceptance:* the classic self-referential and mutually-recursive cases
terminate with the standard-mandated output rather than looping; argument
pre-expansion ordering matches the C rules on a table of cases drawn from the
standard's own examples; and **slice 1's no-panic / no-hang fuzz oracle is
extended over macro definition and expansion**. That extension is required
here, not optional: slice 1 fuzzes an engine that has no macros, while the
exponential-blowup bounds in §6 (tokens produced, synthesised token bytes,
fuel, hide-set cost) all guard subsystems that only come into existence in this
slice.

**Slice 3 — the C dialect, and real C.**
`c.tokens` stops discarding `#…` lines; `c-lexer` surfaces directive tokens; a
`CDialect` implements §5; `c-to-semantic-ir` runs the engine as its
`post_tokenize` hook. `SIR27`'s preprocessor scope statement is updated.
*Acceptance:* a C program using `#define` (object- and function-like), `#if`/
`#ifdef`/`#else`/`#endif` and a real project-local `#include` compiles through
`c-to-semantic-ir` and executes with the expected result — the first C program
in this repo to depend on preprocessing.

**Slice 4 — COBOL `COPY … REPLACING`, the boundary proof.**
A `CobolDialect` that reuses the include, source-map, dispatch and bounds layers
while supplying its own pseudo-text replacement matcher.
*Acceptance:* a COBOL program using `COPY … REPLACING` compiles and runs; and
the engine core is unchanged by this slice, or any change it did require is
called out explicitly in the PR as a genuine genericity defect found by the
second customer.

**Later slices (not scoped here):** `__LINE__`/`__FILE__`, `#pragma once`,
variadic macros, `__has_include`, and a minimal in-VFS `stdint.h`/`stdio.h`.

## 8. Non-goals

- C++ templates or two-phase name lookup.
- Lisp reader macros and Rust-style procedural macros: these operate on an AST
  *after* parsing, and belong to `lexer-parser-hooks.md`'s `post_parse` hook.
- Replacing any existing `.tokens` grammar or parser.
- Preprocessing as a user-facing CLI. This is a library consumed by frontends.

## 9. Open questions for sign-off

1. **C conformance target.** This spec assumes C89/C99 macro-expansion
   semantics for the core algorithm, with variadic macros and `__has_include`
   deferred to later slices. Confirm that is the right line.
2. **Header provenance.** Slice 3 proves project-local `#include "…"`. Whether
   system `#include <…>` resolves against a small repo-vendored header set or
   against the host toolchain's real headers is a policy decision with
   reproducibility consequences, and is deliberately left open here.

   It has a security dimension that must be recorded with the question:
   resolving against the host toolchain means reading outside the declared
   search roots, which conflicts directly with §6's containment row and makes
   the trusted input set depend on host state that an attacker with local write
   access could influence. **Whichever way it is decided, system header
   directories must be declared search roots subject to the identical
   containment, regular-file and byte-cap rules, opened read-only, with no
   implicit fallback to an undeclared host path.** The containment property
   does not get weakened to accommodate the answer.
