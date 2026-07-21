# MA10 — Scilab: a numerical/array language that is genuinely not MATLAB

## Status

Design-only kickoff (**MA-10a**). Wave 6 of the historical math-languages
roadmap ([`HML00`](HML00-historical-math-languages-roadmap.md) §7) — the
**second** Wave-6 kickoff, after J ([`MA06`](MA06-j-language.md)). No
`code/grammars/` files and no crate land in this item — only the language
design, so that the answer to this spec's one real question is fixed *before*
any lexer/parser/runtime code exists, exactly as MA06 fixed J's trains and
MA09 fixed Maple's aggregate-type trap before their own implementation PRs.

**The one real question**, stated by the task that spawned this spec: Octave
is also "MATLAB-like with syntax differences," and this repo's real answer for
Octave was *don't build a frontend at all* — `octave-runtime::octavify`
rewrites Octave-only surface forms to plain MATLAB text and delegates
wholesale to `matlab-runtime` ([`MA01`](MA01-matlab-language.md) §5). Does
Scilab's real, documented syntax fit that same thin-wrapper pattern, or does
it need its own grammar/lexer/parser/runtime — the APL→J pattern
([`MA05`](MA05-apl-language.md)/[`MA06`](MA06-j-language.md))?

**Conclusion, stated plainly up front: Scilab needs its own frontend.** It is
close enough to MATLAB that the *grammar shape* (matrix literals, ranges, the
operator-precedence cascade, indexing) is worth forking from
`matlab.grammar` rather than designing from scratch — but it fails the one
test that makes the Octave wrapper *sound*: at least one shared piece of
surface syntax (the `+` operator, §1) has genuinely different *runtime
semantics* in the two languages, not just a different spelling of the same
meaning. A text-rewrite shim cannot fix that, because it would have to know
the operand's *type* to decide what `+` should even mean — which is no longer
"thin." §1 lays out this finding and five further corroborating differences,
each checked directly against current Scilab documentation at help.scilab.org,
not assumed from "MATLAB-like" family resemblance.

## §1 Why Scilab is not "MATLAB with different spelling"

Octave qualifies for the wrapper pattern precisely because every one of its
departures from MATLAB — `#` comments, `endif`/`endfor`/`endwhile`/
`endfunction`/`endswitch`/`end_try_catch`, `!=`/`!` — is a **surface
respelling of identical semantics** (`octave-runtime`'s own doc comment:
"Octave's departures from MATLAB are a small, local set of surface forms").
`octavify` can therefore rewrite Octave text into MATLAB text and hand it to
`matlab-runtime` with a *correctness guarantee*: whatever `matlab-runtime`
computes for the rewritten text **is** the correct Octave answer, because the
two languages agree on what every rewritten construct *means*. That guarantee
is the entire justification for skipping a real frontend.

Scilab breaks that guarantee, and not marginally:

1. **The `+` operator means different things on strings in the two
   languages — this is the decisive finding.** Scilab's own official
   "Matlab to Scilab Conversion Tips" documentation states the divergence
   explicitly, operator by operator: in MATLAB, `'str1'+'str2'` performs
   ASCII-code numeric addition (`[230,232,228,99]`); in Scilab, the *same
   syntax* concatenates the strings (`'str1str2'`) — "string addition is the
   same as string concatenation, what is done in Matlab by the `strcat`
   function" (help.scilab.org, `m2sci_addition` page). Scilab's own
   general "plus" reference page states it just as bluntly in its title:
   "plus (+) — Numerical addition. **Text concatenation (gluing)**"
   (`docs/2024.1.0/en_US/plus.html`). No text-rewrite shim can resolve this:
   deciding what `+` should compile to requires knowing whether its operands
   are numeric or string *at the point of translation*, which a syntactic
   shim (Octave's whole model) does not have. (This repo's own
   `matlab-runtime::MatValue::Char` currently just *errors* on any arithmetic
   over char arrays rather than computing MATLAB's numeric answer — so a
   naive wrapper wouldn't silently miscompute *today*, it would simply refuse
   valid Scilab code. But the underlying disagreement is real and
   documented at the specification level on both sides, so fixing that stub
   correctly for both languages at once is not possible without splitting
   the runtime — which is exactly the frontend fork this spec commits to.)
2. **Scilab ships its own official MATLAB→Scilab translator, M2SCI**
   (`help.scilab.org/About_M2SCI_tools`, `mfile2sci` page) — a bundled tool
   whose whole job is walking a MATLAB M-file and rewriting each call site to
   its Scilab equivalent, flagging anything it cannot translate safely with a
   `//!` comment. The Scilab project itself treats "compile MATLAB into
   Scilab" as a real, imperfect, semantics-aware translation problem
   important enough to ship dedicated tooling for — not a family resemblance
   a find-and-replace pass can close.
3. **Comments use an entirely different character set, and the character
   MATLAB reserves exclusively for comments is a live token in Scilab.**
   Scilab comments are `//` to end of line and `/* ... */` for a block
   (`help.scilab.org/docs/6.0.1/en_US/comments.html`; the page states block
   comments were "introduced in version 6.0" of Scilab) — not MATLAB's `%`
   and `%{ %}`. Meanwhile `%` in Scilab is the sigil that opens a
   special-constant identifier: `%pi`, `%e`, `%i`, `%inf`, `%nan`, `%eps`,
   `%t`/`%f` (true/false) are all real, protected, undeletable predefined
   variables (help.scilab.org, several `.../section_136f188482853e8d4aa36ba1d67ad284.html`
   doc-version pages). A byte that MATLAB's lexer *always* treats as
   "swallow to end of line" is, in Scilab, the first byte of an ordinary
   value token — the exact opposite lexer contract for the same character.
4. **`if`/`elseif`/`select`/`case`/`while`/`for` all take an optional
   *linker* keyword with no MATLAB equivalent at all.** `if`/`elseif`/`case`
   in a `select` block take `then` (help.scilab.org `if`, `then`, `select`,
   `case` pages); `while`/`for` take `do` (help.scilab.org `while`, `do`,
   `for` pages). Every one of these pages states the identical rule in
   near-identical wording: the linker keyword "must be on the same line as"
   its header, and "can be replaced by a carriage return or a comma." MATLAB
   has no keyword in any of these six positions, ever — this is a genuine new
   grammar production (an optional token, elidable by punctuation), not a
   respelling of something MATLAB already has a slot for.
5. **The "last index" token is `$`, a different token from the `end`
   block-terminator keyword.** `help.scilab.org/dollar`: "`$` (preferred
   notation) can be replaced by `end`" for indexing — meaning **`end`
   gaining an indexing meaning is the newer, alternate spelling**, and
   `end`-as-last-index only exists "as of version 2026.0.0" per
   `help.scilab.org/end`. Classic and current-preferred Scilab keeps the two
   jobs on two different tokens, so a Scilab lexer never needs the
   context-sensitive "is this `end` a terminator or an index" resolution
   MATLAB's own lexer must do.
6. **Extra operator spellings with no MATLAB syntax at all.** `<>` is a fully
   valid not-equal alongside `~=` (help.scilab.org `comparison` page); the
   Kronecker-product family `.*.` / `./.` / `.\.` are real infix operators
   (help.scilab.org `symbols` page — `.*.` "kron", `./.`/`.\.` "krondivide")
   that MATLAB exposes only as the `kron()` *function*, never as syntax. The
   same `symbols` page also documents Scilab's own deprecated legacy
   spellings `@` for `~` (not) and `**` for `^` (power) — pre-modern Scilab
   syntax with no MATLAB counterpart, deprecated in Scilab itself but real
   history, not invented.
7. **`endfunction` is the historically-mandated function terminator, not an
   Octave-style added synonym.** "The final line of a function must be
   `endfunction`" (`help.scilab.org/doc/5.5.2/en_US/function.html`); generic
   `end` only became *usable* to close a function "as of version 6.0.0," and
   even then, per `help.scilab.org/end`'s own words, `endfunction` "still
   should be preferred." This is the opposite direction from Octave, which
   *added* `endfor`/`endif`/… synonyms on top of MATLAB's existing single
   `end` shape; Scilab's historical/preferred shape asymmetrically singles
   out functions, and this spec's "historical core" framing (matching every
   other MA spec's own choice to target a canonical/classic subset) keeps
   `endfunction` mandatory rather than reaching for the newest convergence.
8. **`list`/`tlist`/`mlist` — a three-tier typed-aggregate system**
   materially richer than MATLAB's cell arrays/structs
   (`help.scilab.org/tlist`, `/mlist`, `/list`): a `list` is an ordered
   heterogeneous sequence; a `tlist` ("typed list") tags a list with a type
   name so Scilab functions can be overloaded per-type on it; an `mlist`
   ("matrix-oriented typed list") is a `tlist` variant whose *indexing*
   syntax itself changes meaning (`M(i)` stops meaning "the `i`-th field" the
   moment `M` is an `mlist`, unlike an ordinary `tlist`). This is the same
   shape of trap [`MA09`](MA09-maple-language.md) §1 flagged for Maple's
   three aggregate types (expression sequence / list / set) — a different
   family, same lesson: real elaborateness under an innocuous-looking name,
   not assumable from "MATLAB has cell arrays too."

Finding 1 alone would be enough to disqualify the wrapper pattern (it is a
genuine semantic divergence in shared syntax, the one thing Octave's own
kickoff never had to argue around). Findings 3–5 additionally mean the
*lexer/grammar*, not just the runtime, would need real new productions even if
the semantics happened to line up — so this is not a close call decided on a
technicality; multiple independent lines of evidence point the same way.

**What still transfers.** The *shape* of the grammar is legitimately
MATLAB-family: matrix literals (`[1 2 3]`, `[1;2;3]`, `[1 2;3 4]`, with
space/comma separating columns and semicolon/newline separating rows — the
identical rule stated on `help.scilab.org/docs/2026.1.0/en_US/matrices.html`,
matching MA01 §2's MATLAB rule verbatim), ranges (`a:b`, `a:step:b`), and an
operator-precedence cascade that — per the individual documented facts in §3
— lands tier-for-tier on MATLAB's own well-known cascade. So §3 forks
`matlab.grammar` as a starting template (the same way MA06 §3 "reused APL's
verb/noun split nearly verbatim, renamed" while still shipping J as its own
crate) rather than designing a grammar from a blank page — but the fork
happens at the **grammar-source** level, compiled into `scilab-lexer`/
`scilab-parser` as fully independent crates, not at the Rust-crate dependency
level the way `octave-runtime` depends on `matlab-runtime` directly (§5).

## §2 Substrate gap: none for the numeric core; a value-model gap for strings

Checked against the current `array-runtime` public API (`Array`, `ops::{add,
sub, mul, div, matmul, transpose, reduce, scan, outer}` over the 12-variant
`BinOp` enum already including `Max`/`Min`/the six comparisons, `execute`/
`execute_sum` with a bit-exact `f64` GPU-ready path per `MA00` §5.1): this
cut's numeric scope (§4) — dense rectangular matrices, elementwise ops with
scalar broadcasting, matmul, transpose, ranges, reductions, comparisons — is
**exactly** what `array-runtime` already provides, unchanged, the same
"everything is a matrix" substrate MATLAB, Octave, APL, and J all already
share. **No `array-runtime` change is needed for this cut's numeric surface**,
the same conclusion MA06 reached for J's own kickoff (MA06 §2).

The one real substrate gap is **not** in `array-runtime` at all — it is that
neither `array-runtime` (pure `f64`, no string concept whatsoever) nor
`matlab-runtime` (whose `MatValue::Char(String)` wrapper is a
`matlab-runtime`-level addition, unusable outside that crate without also
importing MATLAB's own char-array semantics) has a **value representation
`scilab-runtime` can reuse for strings.** `scilab-runtime` needs its own
small value enum, e.g. `ScilabValue::{Num(Array), Str(String)}` — the same
*pattern* `matlab-runtime::MatValue` already established (numeric substrate
plus a thin string wrapper) but its **own** enum, deliberately not
`MatValue`, because reusing `MatValue` would silently reuse MATLAB's answer
to "what does `+` mean on this variant" (§1 finding 1) — precisely the trap
this spec's whole conclusion is built to avoid. This cut scopes that value
type to assignment/display/equality only (§4) — no operator overloading over
`Str` yet — so the gap is real but small, and does not block a first,
honest "historical textbook session" subset.

`list`/`tlist`/`mlist` (§1 finding 8) are a second, larger value-model gap:
nothing in this repo's array-family stack has any representation for a
heterogeneous ordered aggregate, typed or not. Deferred (§4), matching how
MATLAB's own cell arrays/structs are still deferred in `matlab-runtime`
(MA01 §2) and how Maple's own three-aggregate problem (MA09 §1) is still an
open design item for that language.

## §3 Grammar design

`scilab-lexer`/`scilab-parser` wrap the shared `GrammarLexer`/`GrammarParser`
(per [`feedback_no_handwritten_lexers_parsers`]), exactly like every other
frontend in this repo. `code/grammars/scilab/scilab.tokens` and
`scilab.grammar` are **forked from** `code/grammars/matlab/matlab.tokens`/
`matlab.grammar` (copied, then diverged) rather than written from a blank
page — §1's closing paragraph is the justification: the grammar *shape* is a
legitimate MATLAB-family inheritance even though the *language* is not.

### Lexer differences from `matlab.tokens`

- **Comments: `//` to end of line, `/* ... */` block — genuinely simpler
  than MATLAB's `%{ %}`.** Scilab's block-comment markers may appear inline
  on a line with code (the `comments.html` example is a single multi-line
  `/* ... */` run, with no "must be alone on its line" constraint the way
  MATLAB's `%{`/`%}` markers have — MA01 §3 calls that MATLAB constraint out
  explicitly). `scilab-lexer` therefore does *not* need MATLAB's
  alone-on-its-line block-comment restriction at all.
- **`'`/`.'` transpose-vs-string ambiguity recurs, needing the same fix MA01
  §3 designed for MATLAB** (a `prev_value`-tracking context hook that decides,
  from the previous emitted token, whether a bare `'` opens a string or closes
  a postfix transpose) — the *strategy* transfers, not the *code*: Scilab's
  hook lives in `scilab-lexer`, independent of `matlab-lexer`'s. Scilab's `"`
  string delimiter has **no such ambiguity** (there's no double-quote
  transpose), so it needs only an ordinary greedy regex rule, same shape as
  MATLAB's `DQ_STRING` — but note Scilab's `'...'` and `"..."` produce the
  **same** string type (`help.scilab.org/strings`'s own worked example,
  `"matrix"=="mat"+"rix"`, treats them interchangeably), unlike modern
  MATLAB's distinct char-array-vs-string-scalar split (MA01 §2) — so
  `scilab.tokens` needs only one STRING token kind, not two.
- **A new token class: `%`-prefixed special constants.** `%` immediately
  followed by an identifier-start character lexes as one `PERCENT_CONST`
  token (`%pi`, `%e`, `%i`, `%inf`, `%nan`, `%eps`, `%t`, `%f` — the closed,
  fixed vocabulary §4 scopes to); this is the mirror image of MATLAB's own
  lexer, which never needs to ask whether a `%` means anything but
  "comment starts here." `scilab-lexer` must check for this pattern
  *before* falling back to "no such thing as a `%`-comment in this
  language" — there is no comment use of `%` to conflict with, which is what
  makes the check safe, not merely convenient.
- **`$` — a single-character last-index token, unambiguous** (§1 finding 5);
  no context-sensitivity needed, unlike MATLAB's own `end`.
- **`<>` — a not-equal digraph**, tokenized with the same longest-match-
  before-bare-`<`/`>` discipline `apl.tokens`/`j.tokens` already use for
  their own digraphs (`∘.`, `<.`/`>.` respectively).
- **Kronecker trigraphs `.*.`/`./.`/`.\.` are not in `scilab.tokens` at all**
  (§4) — the simplest possible "deferred": omitting the trigraph pattern
  means `.`/`*`/`.` lexes as ordinary `.*` followed by a stray `.`, an
  honest parse-time rejection rather than a silent misparse, the same
  "absence, not special-cased exclusion" discipline MA06 §4 used for J's own
  deferred vocabulary.

### Parser differences from `matlab.grammar`

- **One new production, six use sites: an optional elidable linker token.**
  `if`/`elseif` and `select`/`case` take `THEN`; `while`/`for` take `DO`;
  every one of the six is, per §1 finding 4, individually replaceable by a
  comma or a newline. Modeled as a single reusable nonterminal —
  `stmt_sep : (THEN | DO)? (',' | NEWLINE)` used identically in all six
  header productions — the same "one new rule, reused at multiple sites"
  shape as J's `verb_train` production (MA06 §3) or APL's reduce/scan
  operator productions: one genuinely new grammar idea, not six separate
  ad hoc ones.
- **`endfunction` is its own production, distinct from the generic
  block-closer `end`** that `if`/`for`/`while`/`select` all reduce to (§1
  finding 7) — keeping the function-closing rule textually separate from
  ordinary block-closing, matching real Scilab's historical/preferred
  asymmetry rather than unifying them the way MATLAB's own single `end`
  already does.
- **Matrix literals, ranges, and indexing are inherited almost unchanged**
  from `matlab.grammar`'s own productions (§1's closing paragraph) — with
  one substitution: MATLAB's context-sensitive `end`-as-last-index production
  is replaced by a plain `$` terminal with no context sensitivity at all (§1
  finding 5).
- **Precedence cascade.** No single Scilab page publishes a complete,
  formal operator-precedence table the way MathWorks' own "Operator
  Precedence" reference page does for MATLAB (confirmed by an independent
  academic reference's own remark that "there is no listing of the
  precedence and associativity of Scilab's operators anywhere in the
  official documentation"). The table below is assembled from the
  individual facts help.scilab.org *does* state on each operator's own page
  — postfix transpose; `^`/`.^` right-associativity (`a^b^c` = `a^(b^c)`,
  stated directly); relational operators ranking "in between the numeric and
  the logical operators" (the `comparison` page's own words); `&`/`|` being
  equivalent to `&&`/`||` specifically when used directly inside an `if`/
  `while` condition (the `and_op`/`or_op` pages) — cross-checked against the
  fact that it lands tier-for-tier on MATLAB's own published cascade. That
  agreement is itself informative, not an assumption papered over: it is the
  concrete evidence for this spec's own claim that the *grammar shape* is a
  legitimate MATLAB-family inheritance, checked rather than presumed.

  | Tier (high → low) | Operators |
  |---|---|
  | 1 | `( )` grouping, `A(i,j)` / `A($)` indexing |
  | 2 | postfix `'` `.'` (transpose) |
  | 3 | `^` `.^` (power, **right**-associative) |
  | 4 | unary `+` `-` `~` |
  | 5 | `*` `.*` `/` `./` `\` `.\` (matrix/elementwise mul, right/left div) |
  | 6 | binary `+` `-` |
  | 7 | `:` (range) |
  | 8 | relational `<` `<=` `>` `>=` `==` `~=` `<>` |
  | 9 | `&` (elementwise and / short-circuit-in-condition) |
  | 10 | `\|` (elementwise or / short-circuit-in-condition) |
  | 11 | `&&` |
  | 12 | `\|\|` |

## §4 Honest scope — what is out (for now)

In scope for the first cut — a faithful, textbook-session subset, following
the same "honesty about subsets" convention as every other language here
([`MA01`](MA01-matlab-language.md), [`MA06`](MA06-j-language.md),
[`MA09`](MA09-maple-language.md)):

- **Everything is a matrix**, identical value model to MATLAB (§2): dense
  rectangular `f64` arrays via `array-runtime::Array`.
- **Matrix literals and ranges** — identical rule to MATLAB (§1, §3):
  `[1 2 3]`, `[1;2;3]`, `[1 2;3 4]`; `a:b`, `a:step:b`.
- **Arithmetic**: matrix `+ - * / \ ^`, elementwise `.* ./ .\ .^`, transpose
  `'`/`.'`.
- **Comparisons**: `== ~= <> < <= > >=` (both not-equal spellings, §1
  finding 6), boolean `0`/`1` result matching this repo's existing APL/J
  convention.
- **Logical**: `& | ~`, short-circuit `&& ||`.
- **Indexing**: `A(i)`, `A(i,j)`, `A(:,k)`, `A($)` last-index (§1 finding 5,
  §3) — **not** the newer `end`-as-index convergence (deferred below).
- **Assignment & statements**: `x = expr`, trailing `;` suppresses echo.
- **Control flow**: `if/elseif/else/end` and `select/case/else/end`, both
  with the optional `then` linker (§3); `while/end` and `for/end`, both with
  the optional `do` linker (§3); `break`, `continue`.
- **Functions**: `function [y1,...,yn]=f(x1,...,xm) ... endfunction` — the
  historical/preferred mandatory closer (§1 finding 7), multiple return
  values.
- **Comments**: `//` line, `/* ... */` block (§1 finding 3, §3).
- **Special constants**: `%pi`, `%e`, `%i`, `%inf`, `%nan`, `%eps`, `%t`,
  `%f` — a closed, fixed vocabulary of `PERCENT_CONST` tokens (§3), not a
  general sigil-dispatch mechanism (see deferred list).
- **Strings**: single- and double-quoted, the same underlying type (§3),
  doubled-quote escaping — assignment, display, and equality only. No
  operator (`+` or otherwise) is implemented over strings this cut (§1
  finding 1, §2) — a deliberate scope cut, not an oversight: implementing
  `+` at all here, without the typed-dispatch layer §2 defers, would risk
  landing on MATLAB's numeric-addition answer by accident, which would be
  *worse* than simply not having the operator yet.

**Deferred (post-MA-10):**

- **Any operator over strings**, especially `+`-as-concatenation (§1 finding
  1, §2) — needs a typed-dispatch runtime layer this cut does not build.
- **`list`/`tlist`/`mlist`** (§1 finding 8, §2) — a real, elaborate
  aggregate-type system; deferred exactly as MATLAB's own cell
  arrays/structs are still deferred in `matlab-runtime` (MA01 §2), and
  flagged with the same seriousness MA09 gave Maple's three aggregate types.
- **Kronecker operators `.*.`/`./.`/`.\.`** (§1 finding 6, §3) — real Scilab
  syntax, but MATLAB's `kron()` function already covers the same
  mathematics without new grammar; deferred by simply omitting the trigraph
  tokens (§3), not by adding exclusion logic.
- **The `end`-as-last-index convergence with MATLAB** ("as of version
  2026.0.0" per `help.scilab.org/end`) — this cut targets the classic and
  still-preferred `$` form only (§1 finding 5).
- **The deprecated legacy operator spellings** `@` (for `~`) and `**` (for
  `^`) (§1 finding 6) — real historical Scilab syntax, but Scilab's own
  documentation already marks them deprecated in favor of `~`/`^`, so this
  cut targets the modern, preferred spellings only, the same choice MA01
  made in targeting "classic" rather than every historical MATLAB spelling.
- **`global`/`clearglobal`, nested function definitions, `varargin`/
  `varargout`, complex numbers, sparse matrices, N-D arrays beyond rank 2,
  the wider built-in function library** — the same class of deferrals MA01
  §2 already made for MATLAB's own first cut, applied here for the same
  reasons (help.scilab.org/global confirms the construct exists and is
  real; it is simply not this cut's scope).
- **The general `%name` sigil-dispatch mechanism** beyond the fixed
  built-in-constant vocabulary above — this cut treats `%pi` et al. as a
  closed token set, not a general extensible-identifier-prefix mechanism.

## §5 Reuse strategy

- **`array-runtime`**: reused **unchanged** (§2) — `Array`, `ops`,
  `execute`/`execute_sum`, the GPU-dispatch-by-lowering pipeline. Zero
  substrate work, the same conclusion [`MA06`](MA06-j-language.md) §2
  reached for J.
- **`grammar-tools`** (`GrammarLexer`/`GrammarParser`): reused exactly as
  every other frontend, per [`feedback_no_handwritten_lexers_parsers`].
- **`matlab.grammar`/`matlab.tokens`**: reused as a **grammar-source fork**
  (§1, §3) — copied as a starting template, then diverged. This is *not* a
  Rust-crate dependency: `scilab-lexer`/`scilab-parser` do not depend on
  `matlab-lexer`/`matlab-parser` at build time, unlike `octave-runtime`,
  which depends on `matlab-runtime` directly. This is the structural
  difference from Octave's own reuse strategy — Octave reuses the *crate*;
  Scilab reuses the *grammar shape* as a text template but compiles into
  fully independent crates, because §1's findings mean the *evaluator*
  cannot be shared even though large parts of the *grammar* legitimately can.
- **`matlab-runtime`**: **not** reused (§1) — `scilab-runtime` is its own
  tree-walking evaluator over `array-runtime`, shaped like `matlab-runtime`/
  `j-runtime` (an interpreter walking a `GrammarASTNode` CST, computing over
  the shared `Array` value model) but with its own `ScilabValue` enum (§2)
  and its own builtin table under Scilab's own names.
- **`symbolic-ir`/`symbolic-vm`/`cas-*`**: **irrelevant.** Scilab is Stream A
  — numerical/array family (`HML00` §2) — not Stream B (symbolic CAS); this
  work touches no crate in the `cas-*` family, exactly as MATLAB, Octave,
  APL, and J never do either.
- **`HML01`'s `-to-semantic-ir` convention**: per `HML01` §2's amended
  per-language pattern and MA06's own precedent (§5), `scilab-to-semantic-ir`
  is built **alongside** the runtime in this same wave, not bolted on
  afterward. It lowers onto [`SIR22`](SIR22-array-matrix-semantic-ir.md)'s
  array/matrix domain, reusing whatever `Expr` variants `matlab-to-semantic-ir`
  /`apl-to-semantic-ir`/`j-to-semantic-ir` already established for the shared
  numeric core (elementwise ops, matmul, transpose, reduce/scan), adding new
  variants only where Scilab's in-scope surface genuinely has no analogue
  yet (e.g. the `PERCENT_CONST` special constants). The linker-keyword
  control-flow shape (§3) needs **no** new SIR node of its own — by lowering
  time, `then`/`do`/comma/newline have already collapsed to "which statements
  are in this branch/body," the identical shape an ordinary `if`/`while`
  lowering already produces, mirroring how J's own trains needed no
  train-specific SIR node either (MA06 §5).

## §6 Crate layout and rollout (one item = one PR)

```
scilab-lexer/           src/{lib.rs, _grammar.rs}   ← MA-10b (+ code/grammars/scilab/scilab.tokens)
scilab-parser/          src/{lib.rs, _grammar.rs}   ← MA-10c (+ code/grammars/scilab/scilab.grammar)
scilab-runtime/         src/{lib.rs, eval.rs, value.rs, builtins.rs}   ← MA-10d
scilab-repl/            src/{lib.rs, main.rs}       ← MA-10d (the `scilab` binary)
scilab-to-semantic-ir/  src/{lib.rs, lower.rs}       ← MA-10e
```

- **MA-10a — this spec.** The wrapper-vs-frontend decision (§1), the
  substrate gap (§2), the grammar design (§3), and honest scope (§4) fixed
  before any lexer/parser/runtime code lands.
- **MA-10b — `scilab-lexer`.** `scilab.tokens` forked from `matlab.tokens`
  (§3): `//`/`/* */` comments, the `'`/`"` string strategy (reusing MA01
  §3's context-hook *strategy*, not its code), the `%`-prefixed
  `PERCENT_CONST` token class, the `$` last-index token, the `<>` digraph.
  Longest-match-first discipline throughout, following `apl.tokens`/
  `j.tokens`'s own precedent for digraphs.
- **MA-10c — `scilab-parser`.** `scilab.grammar` forked from
  `matlab.grammar`: matrix literals/ranges/indexing inherited near-verbatim;
  the new `stmt_sep` linker production (§3) threaded through all six header
  sites; `endfunction` kept as its own distinct closing production;
  the §3 precedence cascade. Should ship with a recursion-depth cap from
  day one, measured against **this** grammar's own actual native-stack
  crash floor — parenthesised nesting, a flat right-recursive dyadic chain,
  and deeply nested `if`/`select` blocks — following the "measure, don't
  assume one shape's floor bounds the others" methodology
  `apl-parser`/`j-parser`'s own `CHANGELOG.md`s document.
- **MA-10d — `scilab-runtime` + `scilab-repl` + the `scilab` binary.** A
  working REPL: the §4 in-scope surface, `ScilabValue::{Num, Str}` (§2),
  built-ins for the special constants, `$`-based indexing, and the
  linker-keyword control flow.
- **MA-10e — `scilab-to-semantic-ir`**, built alongside per `HML01` §2 /
  MA06's own precedent (§5) — `compile`/`compile_source` lowering
  `scilab-parser`'s CST into a `semantic_ir::Module` over the shared
  SIR22 array/matrix domain. **Done** — this item's own three predictions
  (§5) all held exactly as stated: the `stmt_sep` linker keyword needed no
  SIR representation (it collapses to child-node position by lowering
  time), `select`/`case` needed no new node (desugared into a nested
  `if`-chain over a once-evaluated, hoisted selector temporary, mirroring
  `scilab-runtime::eval::eval_select`), and the eight `%`-prefixed
  constants were constant-folded directly into `IntLit`/`FloatLit` rather
  than needing a dedicated node. No new `semantic-ir` core `Expr`/
  `SirType`/`Feature` variant was added. Three implementation-level
  refinements below this spec's own level of detail, each traceable to a
  concrete finding rather than a change to this spec's architectural
  decisions (full rationale in `scilab-to-semantic-ir/CHANGELOG.md`): (1)
  `\`/`.\ ` lower *uniformly* as a broadcast reciprocal division,
  diverging from `matlab-to-semantic-ir`'s own asymmetric `\`-vs-`.\ `
  template, because `scilab-runtime::eval::apply_binop` — this repo's
  actual ground-truth Scilab interpreter — already makes exactly that
  simplification for both spellings; (2) every arithmetic/ordering
  operator rejects a directly-written string-literal operand, closing a
  gap the MATLAB template's own scalar/array heuristic leaves open, given
  §1 finding 1's own concern is precisely about a string reaching such an
  operator unnoticed; (3) `func_returns` parsing distinguishes single-
  output, explicit-bracket single-output (`[y] = f(...)`), and explicit
  zero-output (`[] = f(...)`) from a genuine multi-output name list,
  mirroring `scilab-runtime::eval::Interpreter::register_function`'s own
  more complete reading of this grammar shape rather than the MATLAB
  template's coarser one.
- **Next**: K/Q or IDL per
  [`HML00`](HML00-historical-math-languages-roadmap.md) Wave 6 — each gets
  its own fresh substrate/grammar-gap analysis, not a rubber stamp, the same
  lesson MA06's own closing section drew from APL→J.

## §7 References

Internal: [`HML00`](HML00-historical-math-languages-roadmap.md) (§5 survey —
the "MATLAB-like with syntax differences" line this spec resolves; §7 Wave
6), [`HML01`](HML01-math-to-semantic-ir.md) (the `-to-semantic-ir`
built-alongside convention adopted at MA-10e), [`MA00`](MA00-array-runtime.md)
(the substrate — unchanged, §2), [`MA01`](MA01-matlab-language.md)
(MATLAB — the frontend this spec forks at the grammar-source level but does
**not** reuse at the crate level, and whose §5 Octave section is the
thin-wrapper pattern this spec's §1 explicitly argues does not apply here),
[`MA06`](MA06-j-language.md) (J — the structural template: a fellow Wave-6
kickoff that also had to settle "is this just family resemblance" before any
code landed, and whose one-new-production/many-use-sites grammar shape this
spec's `stmt_sep` mirrors), [`MA09`](MA09-maple-language.md) (Maple — the
aggregate-type-trap precedent §1 finding 8 and §4 both echo for
`list`/`tlist`/`mlist`).

External, all checked directly against current Scilab documentation at
help.scilab.org (not assumed from MATLAB family resemblance):

- `docs/6.0.1/en_US/comments.html` — `//` line comments, `/* ... */` block
  comments (introduced in Scilab 6.0).
- `doc/5.5.2/en_US/function.html` and `docs/2026.0.1/en_US/functions.html` —
  the `function`/`endfunction` calling sequence and multiple-output
  `[y1,...,yn]=f(...)` syntax.
- `docs/5.3.3/en_US/if.html`, `/then`, `/do`, `docs/6.0.2/en_US/select.html`,
  `/case`, `/for`, `/while` — the `if`/`elseif`/`else`/`end`,
  `select`/`case`/`else`/`end`, `while`/`do`, `for`/`do` calling sequences
  and the shared optional-linker-keyword rule ("can be replaced by a
  carriage return or a comma").
- `/end` — `end` closes `for`/`while`/`if`/`select`; `endfunction` is the
  historical/preferred function closer, with generic `end` only an
  alternative "as of version 6.0.0"; `end` only gained a last-index meaning
  "as of version 2026.0.0."
- `/dollar` — `$` as the preferred, version-independent last-index token.
- `/quote` — `'`/`.'` as transpose vs. string delimiter.
- `/strings` and `docs/2024.1.0/en_US/strings.html` — single- and
  double-quoted strings as the same underlying type; doubled-quote escaping.
- `/comparison` — `~=` and `<>` both valid not-equal spellings; ordering
  comparisons restricted to real/integer types.
- `/and_op`, `/or_op` — `&`/`&&`, `|`/`||`, their short-circuit semantics, and
  their equivalence to `&&`/`||` specifically inside `if`/`while` conditions.
- `docs/2024.1.0/en_US/plus.html` ("plus (+) — Numerical addition. Text
  concatenation (gluing)") and `/m2sci_addition` ("+ (Matlab operator) —
  Plus") — the load-bearing citation for §1 finding 1: Scilab's own
  MATLAB-to-Scilab conversion-tips documentation states the `+` divergence
  explicitly, side by side, with the exact worked example
  (`'str1'+'str2'` → concatenation in Scilab vs. ASCII-code numeric addition
  in MATLAB).
- `/About_M2SCI_tools` and `docs/6.1.1/en_US/mfile2sci.html` — the M2SCI /
  `mfile2sci` MATLAB-to-Scilab conversion toolbox (§1 finding 2).
- `/tlist`, `/mlist`, `/list` — the typed-list / matrix-oriented-typed-list
  aggregate system (§1 finding 8).
- `docs/2026.1.0/en_US/symbols.html` — the full operator-symbol reference
  table: the Kronecker family `.*.`/`./.`/`.\.`, the `<>` not-equal digraph,
  and the deprecated legacy spellings `@` (for `~`) / `**` (for `^`).
- The `%pi`/`%e`/`%i`/`%inf`/`%nan`/`%eps` special-constant vocabulary, as
  documented across several doc-version "Constants"/"Predefined variables"
  pages (e.g. `docs/6.1.1/en_US/section_136f188482853e8d4aa36ba1d67ad284.html`
  and its sibling pages under other version prefixes).
- `/global` — the `global` keyword (confirmed real; out of scope this cut).
- On operator precedence specifically: no single official Scilab page
  publishes a complete formal precedence/associativity table (corroborated
  by an independent academic reference's own remark to that effect, found
  while searching for one); §3's table is assembled from the individual
  documented facts above, not from a single authoritative listing — stated
  honestly rather than presented as more authoritative than the sources
  support.
- Scilab's own official history (`scilab.org/about/company/history`):
  Scilab began as a 1990 INRIA/ENPC "Meta2" project open-sourcing an earlier
  in-house tool ("Basile," started 1982 by François Delebecque), itself
  inspired by Cleve Moler's original public-domain MATLAB — an independent
  reimplementation-of-an-inspiration, not a fork, which is consistent with
  §1's finding that the languages diverge in real, documented ways rather
  than being "the same language" the way Octave and MATLAB are.
