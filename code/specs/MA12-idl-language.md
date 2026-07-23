# MA12 — IDL: the science/astronomy array language with a keyword-argument calling convention

## Status

Design-only kickoff (**MA-12a**). Wave 6 of the historical math-languages
roadmap ([`HML00`](HML00-historical-math-languages-roadmap.md) §7) — the
**fourth and final** Wave-6 language, after J ([`MA06`](MA06-j-language.md)),
Scilab ([`MA10`](MA10-scilab-language.md)), and Q ([`MA11`](MA11-q-language.md)).
No `code/grammars/` files and no crate land in this item — only the language
design, so that the answer to this spec's one real question is fixed *before*
any lexer/parser/runtime code exists, exactly as MA06 fixed J's trains, MA10
fixed Scilab's wrapper-vs-frontend decision, and MA11 fixed Q's function
literals before their own implementation PRs.

HML00 §7's Wave-6 bullet names this item with a single word — "IDL" — and its
§5 survey table gives it one honestly-thin line: *"Array language for
science/astronomy."* Following the same discipline every prior Wave-6 kickoff
applied to its own one-line label (MA06 checking J's ASCII spellings were not
one-for-one APL glyphs; MA10 checking Scilab's `+` was genuinely not MATLAB's;
MA11 discovering "K/Q" was two languages and having to pick one), this spec does
not accept "array language for science/astronomy" at face value. It checks — against
the current NV5 Geospatial / L3Harris IDL documentation and the official IDL
Online Help — **which** IDL to target (§1), whether `array-runtime` covers IDL's
value model (§2), and the **one genuinely new grammar/evaluator problem** IDL
introduces that no prior array-family frontend in this repo has ever had to
solve (§3): a **keyword-argument calling convention** layered over a
**two-kinds-of-callable** (procedure vs. function) split, with two entirely
different call syntaxes.

**Conclusion, stated plainly up front.** IDL's *value model* fits
`array-runtime` unchanged — in one respect (§2) it fits *better* than MATLAB's
own does. There is **no `array-runtime` substrate gap** for the numeric core,
the same finding MA06/MA10/MA11 each reached. The one small value-model addition
is a runtime-level string scalar, reusing the exact `ScilabValue::{Num, Str}`
*pattern* MA10 §2 already established (its own enum, deliberately not MATLAB's).
The genuinely novel work is entirely in `idl-runtime`'s own parser and
evaluator: IDL is the first language in this repo's array family whose **call
sites are not purely positional** and whose **callables come in two kinds
invoked by two different syntaxes** — a real grammar-and-evaluator problem,
fixed here (§3) before any code lands.

## §1 Why IDL, and which IDL — the square-bracket-subscript language (5.0-and-later)

IDL (Interactive Data Language) originated in 1977 (David Stern, at what became
Research Systems Inc.) as an array-oriented interactive language for scientific
data analysis, and became the lingua franca of astronomy, remote sensing, and
medical/atmospheric imaging. It is proprietary and still actively developed and
sold; the ownership lineage is **RSI → ITT Visual Information Solutions →
Exelis VIS → L3Harris Geospatial → NV5 Geospatial** (NV5 completed its
acquisition of the L3Harris Visual Information Solutions commercial geospatial
software business in 2023; current releases are IDL 8.9 / 9.x). The canonical
reference is the **NV5 Geospatial IDL Reference / "Using IDL" documentation**
(`nv5geospatialsoftware.com/docs/`), which is the direct continuation of the
older L3Harris / Exelis / RSI *IDL Reference Guide* and *IDL Online Help*.

Unlike "K/Q" (MA11 §1), "IDL" is genuinely one language, not a shorthand for
two — so this spec does not have MA11's fork-in-the-road to resolve. It does,
however, have a **version-era decision** exactly like MA07's Derive-6.1 pin and
MA11's "Q since kdb+ 2.0" pin, because IDL's own syntax changed in one
load-bearing way that a grammar must commit to up front:

- **IDL 5.0 (1997) replaced parenthesis array-subscripting `a(i)` with
  square-bracket subscripting `a[i]`, to remove a real grammar ambiguity.**
  Before 5.0, IDL used parentheses for *both* array subscripts *and* function
  argument lists, so — in the official documentation's own worked example — a
  statement like `value = fish(5)` was ambiguous: it could mean "element 5 of an
  array named `fish`" or "call the function `fish` with argument 5," and the
  compiler could not tell which from the syntax alone. IDL 5.0 introduced `[ ]`
  for subscripting so that "an array subscripted in this way is unambiguously
  interpreted as an array under all circumstances"
  (NV5/L3Harris *Understanding Array Subscripts*). The old `( )` subscript form
  is still *accepted* for backward compatibility (subject to that ambiguity),
  and RSI shipped an `idlv4_to_v5` conversion utility (June 1997) to migrate
  code.

  This is directly parallel to the fact MA10 §1 turned up for Scilab and MA11
  §1 turned up for Q: a documented, version-specific syntax fact that decides
  the grammar. **This spec targets the modern, unambiguous `[ ]`-subscript
  language** — the language every IDL programmer writes today and the one the
  current NV5 Reference documents — and treats the pre-5.0 `( )` subscript form
  as explicitly out of scope (§4), for exactly the same reason MA10 §4 targeted
  Scilab's modern `~`/`^` over the deprecated `@`/`**`: a first cut should
  reconstruct the *canonical, unambiguous* form of the language, not carry a
  known ambiguity into a brand-new grammar. Pinning to the bracket form is what
  lets `idl-parser` distinguish `a[i]` (indexing) from `f(x)` (function call)
  structurally, instead of needing IDL 4's own symbol-table-driven,
  compile-time disambiguation that no grammar-tools grammar could express
  cleanly.

**Decision:** this spec, and every item under it, targets **IDL 5.0-and-later
as documented in the current NV5 Geospatial IDL Reference** — concretely, the
square-bracket-subscript language, verified against NV5/L3Harris documentation
and the official IDL Online Help rather than assumed from the survey table's
"array language for science/astronomy" line.

## §2 Substrate check: `array-runtime` unchanged; one small runtime-level string value

Checked directly against the current `array-runtime` public API (`value::Array`
— a dense **column-major** `f64` buffer with a `shape: Vec<usize>` where `[]` is
a rank-0 scalar, `[n]` a vector, `[r,c]` a matrix; `ops::{add, sub, mul, div,
matmul, transpose, reduce, scan, outer}` over the 12-variant `BinOp`; per
[`MA00`](MA00-array-runtime.md) §3) and against what the IDL documentation
actually says IDL's value model is:

- **IDL is 0-based.** Array indices start at 0 (`a[0]` is the first element).
  This is the *same* base `array-runtime` uses internally and the same base
  J (MA06 §1) and Q (MA11 §4) already lower to — so, unlike MATLAB/APL/Scilab
  (whose 1-based surface must be translated to the IR's 0-based convention at
  lowering time), **IDL needs no index-base translation at all**. IDL joins
  J and Q as a 0-based frontend.
- **IDL is column-major (Fortran order).** The official *Columns, Rows, and
  Array Majority* documentation states IDL stores arrays in column-major
  format "the same as Fortran," so that the first subscript varies fastest in
  memory — the *identical* storage convention `array-runtime` was built with
  (MA00 §3: element `(r,c)` at flat index `c*nrows + r`, leftmost dimension
  fastest). The raw buffer layout matches with no transpose at the boundary.
- **An IDL scalar is genuinely rank-0, not a 1×1 array.** Per the *SIZE*
  reference, a scalar has *zero* dimensions (`SIZE(x, /N_DIMENSIONS)` returns 0
  for a scalar; a single-element array still reports rank 1). This is a real
  divergence from MATLAB's "everything is a matrix, a scalar is 1×1" model
  (MA01 §2) — and `array-runtime` models a scalar as `shape == []` (rank-0),
  **which fits IDL's genuine rank-0 scalar more directly than it fits MATLAB's
  own 1×1 convention.** The one place this cut must be careful is the same one
  MA00's own `Display`/`from_rows` helpers already make a MATLAB-flavored
  choice about (`[nrows, ncols]`), addressed in the two frontend-lowering notes
  below.

Two things are **frontend-lowering concerns, not substrate gaps** — the same
"disambiguation is the frontend's job" discipline SIR22/SIR10 already apply,
and the same category as J's 0-vs-1 base (MA06 §1) and MATLAB's `end`-relative
indexing (SIR22, resolved before `IndexArg::Scalar` is emitted):

1. **IDL's subscript order is `[column, row]`, transposed from MATLAB's
   `[row, column]`.** The *Columns, Rows, and Array Majority* page states IDL
   treats a 2-D array as `a[column, row]` — the first index is the column — so
   `intarr(ncols, nrows)` declares columns first. `array-runtime`'s *storage*
   (column-major, leftmost-dimension-fastest) matches IDL's storage exactly,
   but MA00's `from_rows`/`matmul`/`transpose`/`Display` helpers were written
   to the MATLAB `[row, column]` reading of the same buffer. Whether IDL's
   `a[i,j]` (col `i`, row `j`) maps to `array-runtime`'s element `(i,j)` or
   `(j,i)` is therefore a concrete lowering decision `idl-runtime` /
   `idl-to-semantic-ir` must make deliberately — **this is exactly the kind of
   convention detail a later implementation item must confirm empirically**
   (against a real IDL session's `PRINT` output) before relying on it, not
   assume carries over from MATLAB's helpers. No `array-runtime` change is
   needed either way; the buffer is the same, only the labelling of which
   dimension is "rows" differs.
2. **IDL supports negative subscripts counting from the end** (`vec[-1]` is the
   last element, per *Array Subscript Ranges*) and a `*`-based "all/rest"
   subscript (`vec[*]`, `vec[s0:*]`, `vec[s0:*:n]`). These are surface
   conveniences the frontend resolves to concrete 0-based positions at lowering
   time — negative-from-end is the same shape as MATLAB's `end` (resolved to a
   `size`-minus-offset expression before `IndexArg::Scalar`, per SIR22), and
   `*`/`s0:*` map onto SIR22's existing `IndexArg::Whole` / `Range`. No new IR
   node and no substrate change; only frontend translation.

**The one genuine value-model addition is a runtime-level string scalar, and
it is not new to this repo.** IDL has a first-class string type, and a
faithful textbook session needs it for `PRINT, 'hello'` and for string-valued
keyword arguments (`TITLE = 'flux'`). `array-runtime` is pure `f64` with no
string concept, and MATLAB's `MatValue::Char` is a `matlab-runtime`-internal
wrapper carrying MATLAB's own char-array arithmetic semantics — so, exactly as
MA10 §2 concluded for Scilab, `idl-runtime` gets **its own** small value enum,
`IdlValue::{Num(Array), Str(String)}`, deliberately not `MatValue` (reusing
`MatValue` would silently import MATLAB's answer to "what does `+` mean on this
variant"). This cut scopes `Str` to assignment / display / `PRINT` / equality /
keyword-value only — no operator overloading over `Str` yet — so the addition
is real but small and lives entirely in `idl-runtime`, touching no shared
crate. This is the same "own enum, same pattern" move MA10 made, cited as
precedent, not reinvented.

**One numeric-semantics divergence is deferred and flagged honestly.** IDL is a
*typed* numeric language: `INDGEN` returns integers, `FINDGEN` returns floats,
and integer `/` truncates (`5/2` is `2`, whereas `5.0/2` is `2.5`), with
integer overflow wrapping. `array-runtime` is pure `f64`, so this cut computes
every numeric value in `f64` and therefore does **not** reproduce IDL's
integer-typed arithmetic. This is a real, documented divergence, not an
oversight — it is deferred exactly as J/APL/Scilab all deferred exact
integer/typed semantics, and it is called out here as **a behavior a later item
must confirm empirically and decide how to model (a type tag on `IdlValue::Num`,
or a typed-array substrate) before relying on it**, rather than papered over.

`array-runtime` is therefore reused **unchanged** for this cut's numeric core —
the same conclusion MA06 §2 and MA10 §2 reached — with the `[column,row]`
order and negative/`*` subscripts handled at the frontend, and a small
`idl-runtime`-local string value added on MA10's precedent.

## §3 The one genuinely new grammar/evaluator problem: a keyword-argument calling convention over two kinds of callable

Every array-family frontend in this repo so far invokes a callable in exactly
**one** way, and that way is **purely positional**:

- APL / J / Q apply a verb to its operands by juxtaposition (monadic/dyadic),
  resolved by which application production matched (MA05 §3, MA06 §3, MA11 §3) —
  positional, and at most two operands.
- MATLAB / Scilab call `f(x, y)` with a parenthesised positional argument list
  (MA01, MA10). MATLAB's so-called "name-value pairs" are *not* a language-level
  keyword mechanism — they are ordinary positional string+value arguments the
  *callee* parses at runtime; the grammar sees only positional args.
- Q (MA11 §2/§3) is the one prior frontend with genuine user-defined callables
  — named parameters, a multi-statement body, a scope frame — but Q's *call
  site* is still positional juxtaposition (`f[x;y]` binds by position).

IDL breaks all of this at once, and the break is the headline problem this
spec fixes before any grammar or runtime code lands. IDL introduces **three
intertwined novelties**, none of which any production in this repo's array
family expresses today:

1. **Two kinds of callable, invoked by two entirely different syntaxes.**
   - A **function** is called in *expression* position, parenthesised, and
     returns a value: `y = SIN(x)`, `a = INDGEN(5)`. Defined
     `FUNCTION name, p1, p2, ... & ... & RETURN, value & END`. This is close to
     what MATLAB/Scilab already have.
   - A **procedure** is called in *statement* position, **command-style, with
     no parentheses and no return value**: `PRINT, x`, `PLOT, x, y`. Defined
     `PRO name, p1, p2, ... & ... & END`. A bare identifier at the start of a
     statement, followed by a comma-separated argument list, is a **procedure
     call statement** — a statement form no prior frontend in this repo has.
     The documentation states the split directly: a procedure "is used by
     giving its name followed by any parameters that it needs," while a
     function's "arguments and keyword arguments … should be supplied within
     the parentheses that follow the function's name" (NV5/L3Harris *Functions
     and Procedures* / *IDL Syntax*).
2. **Keyword arguments at the call site**, mixed freely with positional
   arguments, in *both* procedure and function calls: `PLOT, x, y, TITLE='flux',
   COLOR=255` and `r = HISTOGRAM(a, BINSIZE=2, MIN=0)`. Inside an argument
   list, `IDENT = expr` is a **keyword binding**, not the statement-level
   assignment `IDENT = expr` means everywhere else. No prior array frontend
   parses a named binding inside an argument list — this is a genuinely new
   argument-list production, and the one place `idl-parser` must know it is
   inside a call-argument context to read `=` as keyword-bind rather than
   assign.
3. **The `/KEYWORD` boolean shorthand.** `/KEYWORD` is defined by IDL to be
   exactly equivalent to `KEYWORD = 1` (NV5/L3Harris *Keywords* / *Parameters*),
   so `PLOT, x, /YLOG` means `PLOT, x, YLOG=1`. A leading `/` immediately before
   an identifier *inside an argument list* introduces a set-boolean keyword —
   which the lexer/parser must distinguish from `/` as the division operator
   everywhere else (a parse-context signal, resolved the same "know which
   production you are inside" way MA11 §3 resolved Q's `/`-as-comment-vs-reduce,
   though here the signal is grammatical position, not whitespace).

**What genuinely transfers from Q, and what is new.** IDL's *definition* side —
a named callable with declared parameters, a multi-statement body, and a call
scope frame that binds arguments to parameter names and evaluates body
statements in order — is the **same shape Q's `QFn::Lambda` already
established** (MA11 §2): the environment/scope-frame concept is no longer novel
to this repo. So this spec does **not** re-litigate "how do you evaluate a
user-defined function body" — Q settled that. What is new, and what §3 fixes,
is the layer *on top* of it:

- the **procedure-vs-function** statement/expression split and its two call
  syntaxes (item 1) — a grammar novelty (a `procedure_call_stmt` production with
  no analogue in any prior frontend), and
- the **keyword-argument dispatch** (items 2–3) — a real new *evaluator*
  concern: binding a call's mixed positional-and-keyword argument list onto a
  callable's declared positional-and-keyword parameters, with `/KW` defaulting
  the keyword to `1` and an *omitted* keyword left undefined (IDL's idiomatic
  `N_ELEMENTS(kw) EQ 0` "was this keyword passed?" test relies on omitted
  keywords being genuinely absent, not defaulted to a sentinel — a detail the
  evaluator must get right, and one a later item should confirm empirically).

This is `idl-runtime`-internal work, parallel to how `QFn` lives in `q-runtime`
and `JFn` in `j-runtime` rather than in `array-runtime` — **no shared crate
changes.** `idl-runtime` needs an `IdlCallable` representation distinguishing a
`Procedure { params, keywords, body }` from a `Function { params, keywords,
body }`, plus a keyword-aware argument-binding step its scope-frame setup runs
before evaluating the body — the direct analogue of how MA06 §3 had to fix
hook/fork evaluation and MA11 §3 had to fix lambda evaluation before their
runtimes could be written.

**Secondary grammar work — real, but ordinary, following prior precedent.**
Beyond the headline call-convention problem, IDL's block syntax needs a few
productions that are new to *this* grammar but well within grammar-tools'
proven expressiveness, each with a direct precedent:

- **A family of matched block terminators.** IDL blocks may close with a
  generic `END` *or* with a specific matched terminator the compiler checks
  against the opener: `IF…ENDIF`, `ELSE…ENDELSE`, `FOR…ENDFOR`, `WHILE…ENDWHILE`,
  `REPEAT…ENDREP` (and, deferred to §4, `CASE…ENDCASE`, `SWITCH…ENDSWITCH`,
  `FOREACH…ENDFOREACH`) (NV5/L3Harris *BEGIN…END* / *Compound Statements*).
  This is the same "a distinct closing keyword gets its own production" shape
  MA10 §3 used for Scilab's `endfunction`, generalised to a small family; the
  optional compiler-side "does `ENDFOR` match a `FOR`?" check is a nicety the
  parser can carry, not a semantic requirement (plain `END` closes any block).
- **Two forms of every conditional/loop body** — a single-statement form
  (`IF expr THEN stmt`, `FOR v=a,b DO stmt`) and a `BEGIN … ENDxxx` block form —
  the same two-form shape MA10 handled for Scilab's control flow.
- **`THEN`/`DO` are mandatory linker keywords**, not optional-elidable ones:
  `IF expr THEN …`, `WHILE expr DO …`, `FOR v = init, limit[, step] DO …`,
  `REPEAT … UNTIL expr` (NV5/L3Harris *IF…THEN…ELSE*, *WHILE…DO*,
  *FOR*, *REPEAT…UNTIL*). This is *simpler* than Scilab's optional-elidable
  `then`/`do` (MA10 §3 finding 4) — IDL's are always present, so the grammar
  needs no elision rule.
- **Lexer trivia**: `;` opens a comment to end of line; `&` separates multiple
  statements on one line; `$` is the line-continuation character
  (NV5/L3Harris *IDL Statement Syntax* / *Compound Statements*). These are
  ordinary skip/continuation patterns, the same category as J's `NB.` (MA06 §5)
  and Scilab's `//` (MA10 §3).

None of these secondary items is the reason this spec exists; they are noted so
the grammar item (MA-12b/c) scopes them in from day one. The **call convention
(items 1–3) is the one genuinely new problem**, and it is fixed here.

## §4 Language scope (the historical core)

In scope for the first cut — a faithful "textbook IDL session" subset,
following the same honesty-about-subsets convention as every language here
([`MA01`](MA01-matlab-language.md), [`MA06`](MA06-j-language.md),
[`MA10`](MA10-scilab-language.md), [`MA11`](MA11-q-language.md)):

- **Dense numeric arrays**, the IDL value model of §2: `f64` via
  `array-runtime::Array`, 0-based, column-major, with a genuine rank-0 scalar
  distinct from a 1-element array.
- **Array construction intrinsics**: the `*INDGEN` index-filled family
  (`INDGEN`, `FINDGEN`, `DINDGEN`, `LINDGEN` — each element set to its
  subscript) and the `*ARR` zero-filled family (`INTARR`, `FLTARR`, `DBLARR`,
  `LONARR`), computed in `f64` (§2's deferred typed-arithmetic note).
- **Array literals**: `[1, 2, 3]` (comma-separated), and concatenation via the
  same `[a, b]` bracket form (NV5/L3Harris *Array Concatenation*).
- **Subscripting** (the modern `[ ]` form only, §1): `a[i]`, negative-from-end
  `a[-1]`, ranges `a[s0:s1]`, strided ranges `a[s0:s1:n]`, all/rest `a[*]` /
  `a[s0:*]` / `a[s0:*:n]`, and 2-D `a[i, j]` in IDL's `[column, row]` order
  (§2, resolved at the frontend). Subscripted assignment `a[i] = expr`.
- **Arithmetic & elementwise**: `+ - * /`, `^` (power), matrix multiply `#`
  and `##` (IDL's two matrix-product operators), transpose `TRANSPOSE(a)`,
  scalar↔array broadcasting — lowered onto `array-runtime`'s
  elementwise/`matmul`/`transpose` ops.
- **Comparisons and logic as named operators**: IDL spells these as words —
  `EQ NE LT LE GT GE` (comparison) and `AND OR NOT XOR` (logical/bitwise)
  (NV5/L3Harris *Relational Operators* / *Boolean Operators*), producing this
  repo's `0`/`1` convention (matching APL/J/Scilab). The word-operator spelling
  is itself a small lexer note — these are keywords, not glyphs — but not a new
  *grammar* problem (they slot into the ordinary precedence cascade).
- **Reductions / common intrinsics**: `TOTAL` (sum), `MIN`, `MAX`,
  `N_ELEMENTS`, `SIZE` — the small, idiomatic set, lowered onto
  `array-runtime::ops` reductions.
- **Control flow**: `IF…THEN…ELSE` (single-statement and `BEGIN…ENDIF/ENDELSE`
  forms), `FOR v = init, limit[, step] DO …`, `WHILE expr DO …`,
  `REPEAT … UNTIL expr`, `BREAK`, `CONTINUE` — with the mandatory `THEN`/`DO`
  linkers and the matched `ENDxxx`/generic-`END` terminators (§3).
- **Procedures and functions** (§3): `PRO name, pos…, KW=kw…` / `FUNCTION name,
  pos…, KW=kw… & RETURN, val`, both terminated by `END`; command-syntax
  procedure calls (`PRINT, x, /QUIET`), parenthesised function calls
  (`y = SIN(x)`), keyword arguments (`KW=value` and the `/KW` shorthand for
  `KW=1`), and positional arguments — the headline in-scope feature.
- **Assignment & statements**: `x = expr`; `&` statement separator; `$`
  continuation; `;` comment.
- **Strings**: single- and double-quoted string *scalars*
  (`IdlValue::Str`, §2) — assignment, `PRINT`, equality, and use as
  keyword/positional argument values only. No string operators or string
  arrays this cut.

**Deferred (post-MA-12), each a follow-on item exactly as J/Scilab/Q deferred
their own harder extras:**

- **Structures** — IDL's C-struct-like aggregate, both anonymous
  (`{TAG1: v1, TAG2: v2}`) and named (`{NAME, TAG1: v1, …}`), with `s.tag`
  field access (NV5/L3Harris *Structures*). A real new heterogeneous
  named-field *value* substrate nothing in this repo's array stack models —
  the direct analogue of Scilab's `list`/`tlist`/`mlist` (MA10 §1 finding 8),
  Q's tables (MA11 §4), and Maple's aggregate types (MA09 §1). This is IDL's
  single largest deferred surface and its own most distinctive value feature;
  deferring it is a corner *postponed*, not permanently cut.
- **Pointers and heap variables** (`PTR_NEW`, dereference `*p`,
  `PTR_FREE`) — reference/heap semantics, a second new value substrate.
  Deferred.
- **Objects / object-oriented IDL** (IDL 5+ `OBJ_NEW`, class definition
  structures, method calls `obj->Method()`, `obj.property`) — deferred.
- **`LIST` / `HASH`** (the IDL 8.0 dynamic containers) — deferred alongside
  structures.
- **`COMMON` blocks** — IDL's named-shared-variable global-scope mechanism —
  deferred, matching Scilab's `global` deferral (MA10 §4) and Q's nested-scope
  deferral (MA11 §4); this cut's callables have their own local scope frame and
  read/write only their parameters, keywords, and locals.
- **The keyword-inheritance mechanism** (`_EXTRA` / `_REF_EXTRA` keyword
  forwarding) — an advanced layer over the keyword convention; this cut
  implements explicit named keywords only, not keyword pass-through.
- **`CASE` / `SWITCH` / `FOREACH`** and their `ENDCASE`/`ENDSWITCH`/`ENDFOREACH`
  terminators — later control-flow forms (`FOREACH`/`SWITCH` are IDL 8-era);
  the first cut keeps `IF`/`FOR`/`WHILE`/`REPEAT`, the classic four.
- **IDL's typed numeric tower and integer-typed arithmetic** (byte/int/uint/
  long/ulong/float/double/complex/dcomplex, integer division truncation and
  overflow wrap) — deferred; this cut computes in `f64` (§2, flagged as a
  divergence to confirm empirically before a later item relies on it).
- **The wider intrinsic library and all graphics** (`PLOT`, `CONTOUR`,
  direct/object graphics, and the hundreds of library routines) — only the
  small `PRINT`/construction/reduction set above is in scope; the rest is the
  same broad-library deferral MA11 §4 made for Q's q-SQL surface and Wolfram
  made for its `cas-*` surface.
- **The legacy pre-5.0 `( )` array-subscript syntax** (§1) — excluded
  structurally by targeting the `[ ]` form only; the same "target the modern,
  unambiguous spelling" choice MA10 §4 made for Scilab's deprecated operators.
- **File I/O, `EXECUTE`/`CALL_FUNCTION` dynamic dispatch, `ON_ERROR`/`CATCH`
  error handling, `COMPILE_OPT`** — ordinary further-out surface, deferred.

## §5 Reuse strategy

- **`array-runtime`**: reused **unchanged** (§2) — `Array`, `ops`,
  `execute`/`execute_sum`, the GPU-dispatch-by-lowering pipeline. Zero
  substrate work, the same conclusion MA06 §2 / MA10 §2 / MA11 §2 reached for
  J/Scilab/Q. The `[column,row]` order and negative/`*` subscripts are handled
  at the frontend (§2), not in the substrate.
- **`grammar-tools`** (`GrammarLexer`/`GrammarParser`): reused exactly as every
  other frontend, per [`feedback_no_handwritten_lexers_parsers`].
  `code/grammars/idl/idl.tokens` + `idl.grammar` compile to committed
  `_grammar.rs` in `idl-lexer`/`idl-parser` via the grammar-tools CLI.
- **Grammar shape**: unlike Scilab (which forked `matlab.grammar` as a template,
  MA10 §3) or J/Q (which reused APL's verb/noun split, MA06/MA11 §3), IDL's
  surface is an **Algol/Fortran-family imperative grammar** (statements,
  `PRO`/`FUNCTION` definitions, `IF/FOR/WHILE/REPEAT` blocks, an infix
  operator-precedence expression cascade with word operators `EQ`/`AND`/…),
  closer in shape to this repo's `algol`/`dartmouth_basic` grammars than to any
  array-family grammar. So `idl.grammar` is written to IDL's own shape rather
  than forked from an array sibling — the array *values* come from
  `array-runtime`, but the *syntax* is imperative, not tacit/array-notational.
  The one genuinely new production is the **procedure-call statement plus the
  keyword-argument argument-list** (§3); everything else (expression cascade,
  block forms, matched terminators) is ordinary imperative-grammar work with
  the precedents named in §3.
- **Runtime**: `idl-runtime` walks the `GrammarASTNode` CST over `IdlValue`
  (`{Num(Array), Str(String)}`, §2), lowering arithmetic/reductions through the
  same `array-runtime` ops APL/J/Scilab/Q already call. Its own `IdlCallable`
  representation (§3) distinguishes procedures from functions and carries a
  keyword-aware argument-binding step — the one piece of genuinely new
  evaluator design this spec fixes, built on Q's already-established
  scope-frame precedent (MA11 §2), not from scratch.
- **REPL & binary**: `idl-repl` + an `idl` binary, mirroring
  `scilab-repl`/`q-repl`. The continuation scanner tracks paren/bracket
  balance and the `$` line-continuation character (and `BEGIN…ENDxxx` block
  depth), the same continuation-scanning shape the prior REPLs already have;
  `;` comments are stripped at the lexer skip-pattern level, so the REPL scanner
  needs no special comment handling (mirroring `scilab-repl`'s `//` and
  `q-repl`'s `/`).
- **`symbolic-ir`/`symbolic-vm`/`cas-*`**: **irrelevant.** IDL is Stream A —
  numerical/array family (HML00 §2) — not Stream B (symbolic CAS); this work
  touches no `cas-*` crate, exactly as MATLAB/Octave/APL/J/Scilab/Q never do.
- **`HML01`'s `-to-semantic-ir` convention**: per [`HML01`](HML01-math-to-semantic-ir.md)
  §2's amended per-language pattern and every prior Wave-6 precedent,
  `idl-to-semantic-ir` is built **alongside** the runtime in this same wave,
  not bolted on afterward. It lowers the numeric-array core onto
  [`SIR22`](SIR22-array-matrix-semantic-ir.md)'s array/matrix domain, reusing
  the `Expr` variants `matlab`/`apl`/`j`/`scilab-to-semantic-ir` already
  established (elementwise ops, `matmul`, `transpose`, reductions, ranges,
  `IndexGet`/`IndexSet`) — IDL's `[column,row]` order, negative-from-end, and
  `*`/`s0:*` subscripts all resolve to the *existing* `IndexArg::Scalar`/
  `Whole`/`Range` shapes at lowering time (§2), needing no new array node. The
  one open lowering question is the **keyword-argument call**: SIR10/SIR16's
  general-purpose closure/`Call` vocabulary is positional, so
  `idl-to-semantic-ir` lowers a procedure/function onto that closure/call
  vocabulary and **desugars keyword arguments to positional bindings against
  the callee's declared parameter order at lowering time** for in-module
  callees. Whether a first-class keyword-argument SIR node is warranted (for
  separately-compiled/library callees, where the callee's parameter order is
  not statically known) is a decision for that implementation item, not this
  kickoff — the same "depends on what the shared IR already has by the time the
  frontend starts" deferral MA11 §5 made for its own `Closure`-node question.

## §6 Crate layout and rollout (one item = one PR)

```
idl-lexer/           src/{lib.rs, _grammar.rs}   ← MA-12b (+ code/grammars/idl/idl.tokens)
idl-parser/          src/{lib.rs, _grammar.rs}   ← MA-12c (+ code/grammars/idl/idl.grammar)
idl-runtime/         src/{lib.rs, eval.rs, value.rs, builtins.rs}   ← MA-12d
idl-repl/            src/{lib.rs, main.rs}       ← MA-12d (the `idl` binary)
idl-to-semantic-ir/  src/{lib.rs, lower.rs}      ← MA-12e
```

- **MA-12a — this spec.** The version-era decision (§1, the `[ ]`-subscript
  language), the substrate check finding no `array-runtime` change is needed
  (§2), and the one genuinely new grammar/evaluator problem — a
  keyword-argument calling convention over the procedure/function split (§3) —
  fixed before any lexer/parser/runtime code lands.
- **MA-12b — `idl-lexer`.** `idl.tokens`: `;` line comments, `$` continuation,
  `&` separator, single/double-quoted strings, the word operators
  (`EQ`/`NE`/`LT`/`LE`/`GT`/`GE`/`AND`/`OR`/`NOT`/`XOR`), `#`/`##` matrix-product
  operators, the `[ ]` subscript brackets, and the `/`-before-identifier
  boolean-keyword lexing wrinkle (§3 item 3), with longest-match discipline
  throughout, following `j.tokens`/`scilab.tokens`'s digraph precedent.
- **MA-12c — `idl-parser`.** `idl.grammar` written to IDL's imperative shape
  (§5): the expression precedence cascade with word operators; `IF`/`FOR`/
  `WHILE`/`REPEAT` in both single-statement and `BEGIN…ENDxxx` forms with the
  matched-terminator family (§3); `PRO`/`FUNCTION` definitions; and the one
  genuinely new production — the **procedure-call statement and the
  keyword-argument argument-list** (§3). Should ship with a recursion-depth cap
  from day one, measured against **this** grammar's own native-stack floor for
  every distinct deep-recursion shape — parenthesised expression nesting,
  deeply nested `IF`/`FOR` blocks, and long argument lists — following the
  "measure, don't assume one shape's floor bounds the others" methodology
  `apl-parser`/`j-parser`/`scilab-parser`'s `CHANGELOG.md`s document.
- **MA-12d — `idl-runtime` + `idl-repl` + the `idl` binary.** A working REPL:
  the §4 in-scope surface, `IdlValue::{Num, Str}` (§2), 0-based/column-major
  subscripting with negative-from-end and `*` ranges, the array-construction
  and reduction intrinsics, and — the headline — the `IdlCallable`
  procedure/function representation with keyword-aware argument binding (§3),
  built on Q's scope-frame precedent (MA11 §2).
- **MA-12e — `idl-to-semantic-ir`**, built alongside per `HML01` §2 / every
  prior Wave-6 precedent (§5) — `compile`/`compile_source` lowering
  `idl-parser`'s CST into a `semantic_ir::Module` over the shared SIR22
  array/matrix domain, desugaring keyword arguments to positional bindings for
  in-module callees (§5), with the first-class-keyword-node question left to
  that item.
- **Next**: Wave 6 is complete after IDL. Possible later follow-ons, each its
  own fresh design pass rather than a rubber stamp (the lesson MA06/MA10/MA11
  all closed on): a pinned raw-K item reusing `q-runtime`'s engine (MA11 §1),
  and IDL's own deferred surfaces above (structures first — its most
  distinctive value feature — then pointers/objects), promoted to their own
  items as warranted, per HML00 §7's wave discipline.

## §7 References

Internal: [`HML00`](HML00-historical-math-languages-roadmap.md) (§5 survey — the
"array language for science/astronomy" line this spec resolves; §7 Wave 6, whose
fourth and final language this is), [`HML01`](HML01-math-to-semantic-ir.md) (the
`-to-semantic-ir` built-alongside convention adopted at MA-12e),
[`MA00`](MA00-array-runtime.md) (the substrate — unchanged, §2; column-major
storage and the rank-0-scalar model IDL fits directly),
[`SIR22`](SIR22-array-matrix-semantic-ir.md) (the array/matrix IR domain MA-12e
lowers onto, and the "disambiguation is the frontend's job" indexing discipline
§2 reuses), [`MA01`](MA01-matlab-language.md) (MATLAB — the `[row,column]` /
1×1-scalar conventions §2 contrasts IDL against, and the `MatValue::Char`
wrapper §2 deliberately does *not* reuse), [`MA06`](MA06-j-language.md) (J — the
0-based-frontend and zero-substrate-gap precedents §2 follows),
[`MA10`](MA10-scilab-language.md) (Scilab — the `ScilabValue::{Num, Str}`
own-enum string-value pattern §2 reuses, the `endfunction`-distinct-production
and two-form control-flow precedents §3 follows, and the fellow-Wave-6
"is it really just family resemblance" discipline this spec applies to IDL),
[`MA11`](MA11-q-language.md) (Q — the immediately-prior Wave-6 language, whose
`QFn::Lambda` scope-frame/user-defined-callable precedent §3 builds the
keyword-convention layer on top of, and whose §1 version-pin discipline §1
mirrors), [`MA09`](MA09-maple-language.md) (Maple — the aggregate-type-trap
precedent §4's structures deferral echoes).

External, checked against the current NV5 Geospatial IDL documentation
(`nv5geospatialsoftware.com/docs/`) and the official IDL Online Help. The
current NV5 docs center is a search-driven single-page application whose deep
pages do not all serve directly to a plain fetcher; where a canonical page did
not resolve directly, it is cited via a well-known **verbatim mirror of the
official RSI/ITT/Exelis IDL Online Help** (e.g. the IRyA/UNAM `manuales/IDL`
mirror and the Dartmouth `northstar-www` IDL 6.2 help mirror) and/or the
immediately-prior official L3Harris `l3harrisgeospatial.com/docs` pages —
stated honestly rather than presented as freshly fetched from the current NV5
site:

- *Understanding Array Subscripts* / *Array Subscript Ranges*
  (`nv5geospatialsoftware.com/docs/Array_Subscript_Ranges.html`, resolved
  directly) — the `[ ]`-vs-`( )` subscript history and the `fish(5)` ambiguity
  (§1); the subscript-range forms `[s0:s1]`, `[s0:s1:n]`, `[s0:*]`, `[*]`,
  negative-from-end `[-1]` (§2, §4).
- *Columns, Rows, and Array Majority* — IDL's column-major (Fortran-order)
  storage and the `[column, row]` subscript order (§2).
- *SIZE* / *N_ELEMENTS* — a scalar has zero dimensions, distinct from a
  1-element array (§2).
- *Functions and Procedures* / *IDL Syntax* / *Defining a Procedure* — the
  procedure (command-syntax, no parens) vs. function (parenthesised) split and
  the `PRO`/`FUNCTION`/`RETURN`/`END` definition forms (§3, §4).
- *Parameters: Arguments and Keywords* / *Keywords* — positional vs. keyword
  parameters; the `KEYWORD = value` call syntax and the `/KEYWORD` shorthand
  defined as equivalent to `KEYWORD = 1` (§3).
- *IF…THEN…ELSE*, *WHILE…DO*, *FOR*, *REPEAT…UNTIL*, *BEGIN…END*, *Compound
  Statements* — the mandatory `THEN`/`DO` linkers, the single-statement vs.
  `BEGIN…END` block forms, the matched terminator family
  `ENDIF`/`ENDELSE`/`ENDFOR`/`ENDWHILE`/`ENDREP`(/`ENDCASE`/`ENDSWITCH`/
  `ENDFOREACH`), and the `&` statement separator / `;` comment / `$`
  continuation trivia (§3, §4).
- *Array Creation* / *Creating Arrays* — the `*INDGEN` (index-filled:
  `INDGEN`/`FINDGEN`/`DINDGEN`/`LINDGEN`) and `*ARR` (zero-filled:
  `INTARR`/`FLTARR`/`DBLARR`/`LONARR`) families (§4).
- *Relational Operators* / *Boolean Operators* — the word operators
  `EQ`/`NE`/`LT`/`LE`/`GT`/`GE` and `AND`/`OR`/`NOT`/`XOR` (§4).
- *Structures* — anonymous `{TAG:v}` and named `{NAME, TAG:v}` aggregates and
  `s.tag` access (§4, deferred).
- Ownership/version history: NV5 Geospatial's completed acquisition of the
  L3Harris Visual Information Solutions geospatial software business (2023) and
  the RSI → ITTVIS → Exelis VIS → L3Harris → NV5 lineage; current IDL 8.9 / 9.x
  releases; IDL 5.0 (1997) as the square-bracket-subscript inflection with the
  `idlv4_to_v5` migration utility (§1).

**Flagged as unverified / to confirm empirically at implementation time** (per
this repo's honesty discipline, called out rather than assumed): (1) the exact
mapping of IDL's `a[i, j]` (`[column, row]`) onto `array-runtime`'s internal
element position, since MA00's helpers use MATLAB's `[row, column]` reading of
the same column-major buffer (§2 note 1); (2) IDL's integer-typed arithmetic
(integer division truncation, overflow wrap), which this `f64`-only cut does not
reproduce and a later item must decide how to model (§2, §4); (3) the precise
"omitted keyword is genuinely undefined" semantics IDL's `N_ELEMENTS(kw) EQ 0`
idiom relies on, to be confirmed against a real IDL session before the evaluator
commits to it (§3).
