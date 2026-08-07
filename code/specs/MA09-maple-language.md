# MA09 — Maple (a subset)

## Status

Active spec / roadmap for a **Maple** frontend — Wave 5 of the historical-math
roadmap ([HML00 §7](HML00-historical-math-languages-roadmap.md)), named there
alongside Reduce and Derive as "more symbolic CAS on the shared engine," and
the last of the three still marked "unstarted" once
[`MA07`](MA07-derive-language.md) (Derive) and
[`MA08`](MA08-reduce-language.md) (Reduce) landed. Maple (Keith Geddes &
Gaston Gonnet, University of Waterloo, 1980–82; a C kernel plus a large
library written *in* the Maple language itself; still actively developed and
sold by Maplesoft) is, on the surface, deceptively close to Reduce and
Derive — the same `:=` assignment spelling, the same `and`/`or`/`not`
keywords — which makes it tempting to assume it is "Reduce again." It isn't:
Maple has **three** distinct aggregate literal types where Reduce/Derive each
have one, and its `f(x) := expr` spelling — which *is* Reduce's/Derive's own
general function-definition idiom — means something narrower and different
in real Maple (§1). This is a **design-only** kickoff: no `code/grammars/`
files, no crate, yet. Every claim in §3/§4 below was checked directly against
the current Maplesoft online Help system (the Maple Programming Guide and
its individual Help topic pages — see §6) rather than assumed from family
resemblance to Macsyma/Wolfram/Derive/Reduce, matching the verification
discipline [`MA08`](MA08-reduce-language.md)'s own header comment insists on.

## §1 Why Maple is "three aggregate types and a remember-table trap," not another `:=`-assignment CAS

Maple's assignment operator is spelled `:=`, identically to Reduce's and
Derive's own (Maple Programming Guide §5.5 "Assignments"; confirmed directly
against the Help page for `:=`) — so a family-resemblance guess would place
Maple as a close cousin of Reduce. Two real, well-documented facts complicate
that guess:

**First — Maple has three aggregate types, not one, and its own bracket
choices collide with the conventions Reduce and Derive already established
in this repo.** [Derive](MA07-derive-language.md) uses `[a, b, c]` for
*vectors*; [Reduce](MA08-reduce-language.md) uses `{a, b, c}` for *lists*.
Maple uses **both** brackets, for two genuinely different mathematical
objects, plus a third, bracket-less form:
an **expression sequence** `a, b, c` (built with the bare comma operator,
no brackets at all — Programming Guide §3.11 "Expressions for Data
Structures"; the `exprseq` Help page); a **list** `[a, b, c]` (ordered,
duplicates preserved — Programming Guide §4.3 "Immutable Data Structures");
and a **set** `{a, b, c}` (unordered, duplicates silently removed — same
§4.3, and the `set` Help page's own worked example: `{x, y, y}` and
`{y, x, y}` both produce `{x, y}`). So Maple's `[a, b, c]` means *list* the
way Derive's identical spelling does **not** (Derive's is a vector), and
Maple's `{a, b, c}` means *set* the way Reduce's identical spelling does
**not** (Reduce's is a list) — the same brackets, three different family
conventions, genuinely worth flagging before any lowering code gets written.

**Second — and this is the sharper trap — Maple's own `f(x) := expr`
spelling, which is *exactly* Reduce's manual's own general-definition idiom
("`h(l,m) := x-2*y`, where h is an operator," per
[MA08](MA08-reduce-language.md) §3) and Derive's `F(x) := e`, does **not**
define a general function in real Maple.** It is a narrower mechanism called
a **remember-table specific-value assignment**: `f(0) := 1` patches a single
cached value onto an *already-existing* procedure/operator `f`, and per
Maple's own Help ("remember" page; confirmed directly): "you will not be able
to substitute into it or do anything you normally do with functions" the way
a real function definition supports. Real Maple general function definition
is either the **arrow (functional) operator** — `f := (x, y) -> expr`, or
the unary form `f := x -> expr` (Help page "operators/functional"; "A
functional operator... written using arrow notation") — or the full
**`proc(params) ... end proc`** block form (Programming Guide Chapter 6
"Procedures," §6.2). This is a real, citable, and easy-to-get-wrong fact:
naively porting Reduce's/Derive's own assignment-based definition spelling
onto Maple source would silently produce the *wrong* Maple construct.

What doesn't change: the *engine* underneath is the same shared substrate
every symbolic-family language in this repo already drives. Maple's
arithmetic/comparison/logic surface (`+ - * / ^`, `= <> < > <= >=`,
`and or not`) is an ordinary infix expression grammar over the same
`IRNode::Apply { head, args }` shape Macsyma/Wolfram/Derive/Reduce already
lower to, evaluated by [`symbolic-vm`](../packages/rust/symbolic-vm) — so,
like Derive and Reduce, Maple needs a real new frontend (its own lexer +
parser; its surface syntax and precedence table are its own) but not a new
engine. One thing Maple genuinely lacks, verified rather than assumed: any
**surface pattern/rewrite-rule syntax** analogous to Wolfram's `_`/`x_`/`->`/
`/.` or Reduce's `let`/`for all ... let ... =>` rules. Maple does have a
pattern-matching facility (`patmatch`, `match`), but both are **ordinary
library function calls** (`patmatch(expr, pattern, 's')`,
`match(expr = pattern, v, 's')` — confirmed against the `patmatch`/`match`
Help pages) — not dedicated surface grammar the way Wolfram's `_` or
Reduce's `~x`/`let` are. A library function is just a function call; there
is no analogue of Wolfram's W-19/W-20 pattern items to port for Maple, and
`cas-pattern-matching`'s `Blank`/`Pattern`/`Rule`/`RuleDelayed` vocabulary has
nothing to bridge to at the surface level here.

## §2 The pieces (one item = one PR)

Following [HML00 §6](HML00-historical-math-languages-roadmap.md)'s
breakdown — mirroring [MA08](MA08-reduce-language.md)'s four-part split
(spec / tokens+lexer / grammar+parser / runtime+repl) rather than
[MA07](MA07-derive-language.md)'s five-part one. The reasoning is specific
to what §1 found, not a coin flip: Maple's **list** `[a, b, c]` and **set**
`{a, b, c}` literals are each just "bracket + comma-separated element list" —
structurally the *same* production shape every sibling CAS-family grammar in
this repo already implements for its own single aggregate type (Reduce's
`{...}`, Derive's `[...]`, Wolfram's `{...}`); having two bracket flavors
instead of one widens that production, it does not add a new one, so both
fold into the base runtime item exactly as Reduce's own single list type did.
The genuinely more exotic third aggregate — the **bare, unbracketed
expression sequence** `a, b, c` — is deferred *entirely* (§4), not even given
its own numbered item, because unlike Derive's vector/matrix literals (which
still needed their own D-5 to *hold* real vector/matrix *data* immediately,
ahead of any algebra), this subset's `f(a, b)` call-argument lists and
`[...]`/`{...}` literals already cover every place a comma-separated group
shows up in the in-scope surface — there is no committed use for a *bare*
top-level sequence *value* this narrow a subset needs yet, and introducing
one risks colliding with comma's two already-established roles (argument
separator, list/set element separator) for no immediate payoff. Item prefix
is **`MP-`**, not `M-` — `M-` is already Maxima's own item prefix in this
roadmap (HML00's "Item breakdown for the first three waves," MA03 §6), and
this repo's own precedent for a same-first-letter collision is to widen the
prefix (MATLAB's `ML-`, not `M-`), not overload it. This PR is a
**design-only** kickoff, with no grammar files yet:

- **MP-1 — this spec** *(this PR)*. Fixes language scope (§4) and the
  surface grammar shape (§3) the next items implement against; no
  `code/grammars/` files, no crate, yet.
- **MP-2 — `maple.tokens` + `maple-lexer`.** Authored in the grammar-tools
  format and validated with `grammar-tools validate`; the committed
  `_grammar.rs` compiled from `maple.tokens`, a sibling of
  `reduce-lexer`/`derive-lexer`/`wolfram-lexer`/`macsyma-lexer`. Needs the
  usual longest-match-first care (`<=`/`>=`/`<>` before `<`/`>`, `->` before
  a bare `-`) plus keyword-vs-identifier disambiguation for this subset's
  reserved words (`and`, `or`, `not`, `if`, `then`, `elif`, `else`, `end`,
  `fi`, `true`, `false`).
- **MP-3 — `maple.grammar` + `maple-parser`.** The committed `_grammar.rs`
  compiled from `maple.grammar`, over the generic `parser::GrammarParser`,
  with an explicit `MAX_RULE_DEPTH` measured the same way
  `apl-parser`/`j-parser`/`derive-parser`/`reduce-parser` measured theirs
  (per [MA06](MA06-j-language.md) §6's precedent), rather than assumed.
- **MP-4 — `maple-runtime` + `maple-repl`.** (✅ done) Lowers the parsed
  `GrammarASTNode` into `symbolic-ir`, evaluates with `symbolic-vm`'s shared
  `SymbolicBackend` — reused *unchanged*, with no custom `Backend` at all,
  the same reuse story `derive-runtime`/`reduce-runtime` already demonstrate
  (verified against the real handler table, §5 — not assumed from either
  crate's own spec prose). In scope: arithmetic, comparison, logic, the held
  `Assign`/`If` forms, `List`, a new `Set` head for set literals (no shared
  handler yet — see §5), the arrow-operator `Define` bridge, and `diff`→`D`/
  `int`→`Integrate` (thin calls into the same `cas-*`-backed handlers
  Derive's `DIF`/`INT` and Wolfram's `D`/`Integrate` already call under their
  own names). `if`/`elif`/`else`/`end if` desugars to nested `If`. Plus the
  interactive `maple-repl` (a plain, unnumbered read-eval-print loop — real
  Maple's own interactive session has no `#n:`/`In[n]:=` numbered-history
  convention either, matching [Reduce](MA08-reduce-language.md)'s own
  unnumbered `reduce-repl`) and the `maple` binary. `proc(...) ... end proc`
  block-structured procedures, `for`/`while` loops, and the rest of the
  `cas-*` surface under Maple names are **not** MP-4's scope — see §4.
  **Every claim in this bullet held up unchanged once `maple-runtime`
  actually landed** (unlike R-4's own two corrections, MA08 §2 — no
  surface-table row here turned out to be wrong): §3's own table, checked
  row by row against the shipped `crate::lower`/`crate::printer`, needed no
  correction. `proc`/`for`/`while` and the remember-table `f(x) := e`
  spelling are confirmed rejected at **parse** time by the already-merged
  `maple-parser` itself (none of those keywords exist in `maple.tokens`, so
  they lex as ordinary `NAME`s and the leftover tokens fail
  `statement_line`'s own terminator check) — `maple-runtime` needed no
  special-case rejection logic of its own, just to forward the parser's
  `Err`. One disclosed addition beyond this spec's own text: `maple-repl`
  tracks `if` / `end if`|`fi` block-keyword balance (in addition to bracket
  balance) for its line-continuation heuristic — Maple's `if_expr`, unlike
  Reduce's, requires an explicit closer, so an ordinary multi-line `if`
  needed this to be usable interactively at all; see `maple-repl`'s own
  README/CHANGELOG.

## §3 The supported surface (the grammar)

This spec's grammar (implemented by MP-2/MP-3) captures this subset of Maple
syntax, verified against the Maple Programming Guide and the individual Help
pages cited inline and in §6. Everything is desugared to a `head(args)`-shaped
`IRNode::Apply` (right column) in MP-4.

| Surface | Meaning | Lowers to |
|---------|---------|-----------|
| `123`, `1.5` | integer / real literal | `Integer` / `Float` |
| `1/3` | exact rational (division of two integer literals stays unreduced-to-float) | `Div[1, 3]` (MP-4 folds to `Rational`, matching Derive/Reduce) |
| `sin`, `x`, `foo` | symbol (built-ins are conventionally lowercase — `diff`, `int`, `sin`, unlike Derive's uppercase convention) | `Symbol` |
| `true`, `false` | boolean literal (lowercase — Help page `type/truefalseFAIL`) | bridged to the shared backend's pre-bound `True`/`False` symbols (§4 on `FAIL`) |
| `f(a, b)` | function/named-built-in call (ordinary parentheses) | `f[a, b]` |
| `[a, b, c]` | **list** (ordered, duplicates kept — Programming Guide §4.3) | `List[a, b, c]` |
| `{a, b, c}` | **set** (unordered, duplicates removed — Programming Guide §4.3, `set` Help page) | `Set[a, b, c]` (a head new to this repo — see §5) |
| `a + b`, `a - b` | additive | `Add` / `Sub` |
| `a * b` | multiply *(explicit `*` required — confirmed real Maple requirement, not a scope-narrowing choice; see §4)* | `Mul` |
| `a / b` | divide | `Div` |
| `a ^ b` | power (right-assoc; **`^` only** — real Maple documents no `**` synonym, unlike Reduce's `^`/`**` pair) | `Pow` |
| `-a` | negation | `Neg` |
| `a = b` | equation/relation (Programming Guide §3.10 "Boolean and Relational Expressions") | `Equal` |
| `a <> b` | not-equal (Maple's own spelling — not Reduce's `neq`, not Wolfram's `!=`) | `NotEqual` |
| `a < b`, `a > b`, `a <= b`, `a >= b` | relational | `Less`/`Greater`/`LessEqual`/`GreaterEqual` |
| `a and b`, `a or b`, `not a` | logical (§3.10; real Maple keywords, lowercase) | `And` / `Or` / `Not` |
| `x := e` | assignment (§5.5 "Assignments") | `Assign[x, e]` |
| `f := (x, y) -> e`, `f := x -> e` | function definition via the arrow/functional operator (Help page `operators/functional`) — the in-scope, general-purpose analogue of [Reduce](MA08-reduce-language.md)'s `h(l,m) := e` / [Derive](MA07-derive-language.md)'s `F(x) := e`, **not** `f(x) := e` itself (§1, §4) | `Define[f, [x, y], e]` |
| `if b then s1 elif b2 then s2 else s3 end if` (or `fi`) | conditional (Programming Guide §5.6 "Flow Control"; the `if` Help page confirms `fi` — "if" reversed — is short for `end if`) | `If[b, s1, If[b2, s2, s3]]` (each `elif` desugars to a nested `If`) |
| `( … )` | grouping | — |
| `;`, `:` | statement separator — `;` displays the result, `:` suppresses it (§5.3 "Statement Separators"; the same *shape* as Macsyma/Maxima's `;`/`$`, spelled with `:` instead of `$`) | *(display flag on the surrounding session, not an IR node)* |

**Precedence**, loosest → tightest — this is a *subset* of Maple's own real
operator-precedence chain (Help page `operators/precedence`, "the order of
precedence of all Maple... operators from highest to lowest binding
strength"), pruned to the operators this grammar supports and left in the
manual's own relative order (this subset drops, in relative-precedence order
from loosest to tightest: `assuming`, the general comma sequence operator,
`implies`, `xor`, the `$` sequence-repetition operator, the `..` range
operator, `mod`, the `.`/non-commutative-multiplication and `intersect`/
`union`/`minus` set-arithmetic operators, `!` factorial, prefix/postfix
`++`/`--`, custom `&`-operators, `::` type declaration, `||` concatenation,
and `:-` module-member selection — none of which this subset's surface
uses): assignment (`:=`) → arrow (`->`, used only as a `Define` right-hand
side in this subset) → `or` → `and` → `not` → relational
(`=`/`<>`/`<`/`<=`/`>`/`>=`) → additive (`+`/`-`) → multiplicative (`*`/`/`)
→ unary minus → `^` (right-assoc) → function application `f(…)` /
list-literal `[…]` / set-literal `{…}` → atoms.

Comments (`#` line comments, `(* ... *)` nestable block comments — both real,
confirmed against the `comment` Help page) are not part of this grammar; see
§4.

## §4 Honest scope — what is *out* (for now)

This is a clearly-scoped subset (per HML00 §9 and as every prior kickoff in
this family does). This spec's grammar deliberately omits, to be added later
if warranted, each cited against the real Maple documentation rather than
hand-waved:

- **Implicit multiplication by juxtaposition.** Confirmed genuinely absent
  in real Maple too, not a scope-narrowing decision here the way it is for
  Wolfram/Derive: the `arithop` Help page's own worked examples always show
  an explicit `*` (`3*x*y`, never `3xy`), and Maple's "invalid product or
  quotient" error page exists precisely because `3xy` is a parse error, not
  sugar. This subset's requirement of an explicit `*` simply matches real
  Maple, the same non-decision [MA08](MA08-reduce-language.md) §4 documents
  for Reduce.
- **`f(x) := expr` as a general function-definition spelling.** Per §1, this
  is real Maple's **remember-table** specific-value mechanism (`remember`
  Help page), not a general definition — it patches one cached value onto an
  *existing* procedure and, per Maple's own documentation, does not give you
  something you can substitute into like a real function. Deliberately
  **excluded** (not merely deferred) from this subset precisely because its
  surface spelling collides with Reduce's/Derive's own general-definition
  idiom; this subset requires the arrow-operator spelling (§3) for
  definitions instead, so no Maple program in this subset can accidentally
  mean the narrower remember-table thing.
- **Full `proc(params) ... end proc` block-structured procedures**
  (Programming Guide Chapter 6 "Procedures," §6.2 "Defining and Executing
  Procedures," §6.3 "Parameter Declarations" — required/optional/keyword
  parameters, the `$` end-of-parameters marker, parameter modifiers — and
  §6.5 "The Procedure Body" — `local`/`global`/`description`/`option`
  clauses, a multi-statement body, non-local `return`/`error`/`try`/`catch`).
  Confirmed real and substantially bigger than the single-expression
  arrow-operator form this subset covers, with no precedent anywhere else in
  this repo (no other CAS frontend here has local-scope-with-explicit-return
  procedures) — deferred to its own follow-on item, the identical reasoning
  [MA08](MA08-reduce-language.md) §4 used to defer Reduce's own
  block-structured `procedure ... begin ... end`.
- **Bare (unbracketed) expression sequences as first-class values**
  (`a, b, c` outside an argument list or a list/set literal — Programming
  Guide §3.11 "Expressions for Data Structures"; the `exprseq` Help page) and
  **multiple assignment** (`x, y := y, x`; §5.5 "Assignments" explicitly
  covers "multiple assignments"). Per §2, deferred entirely — not even a
  numbered item — since this subset's `f(a, b)` argument lists and
  `[...]`/`{...}` literals already cover every comma-grouped construct the
  in-scope surface needs, and there is no committed use yet for a *bare*
  top-level sequence value.
- **`for`/`while` loops** (Programming Guide §5.6 "Flow Control" — counted
  (`for i from a to b by c do ... end do`), `while`/`until`-guarded, and
  data-structure-iteration (`for x in list do ...`) forms, plus the
  "Looping Commands" `map`/`select`/`remove`/`zip` family). Confirmed real
  syntax (the `do` Help page's own general form), but `symbolic-vm`'s shared
  handler table has no existing `While`/`For` handler for the CAS-family
  languages the way it does for the array-family MATLAB/APL/J side (SIR16's
  `Loops` feature) — the identical gap [MA08](MA08-reduce-language.md) §4
  documents for Reduce's own `for`/`while` — so wiring these would be new
  engine code, not reuse. Deferred to its own follow-on item.
- **The `..` range operator** (Help page `operators/precedence`; used in
  `int(f, x=a..b)`'s bounds, `seq`, and `for ... to ...`). Only meaningful in
  the contexts (definite integration bounds, loops, `seq`) this subset
  already defers, so deferred alongside them rather than added in isolation.
- **`op(i, e)`/`op(i..j, e)`/`nops(e)`** structural accessors (`op` Help
  page's own calling sequences) and the `union`/`intersect`/`minus`/`in`
  set-arithmetic infix operators (confirmed real, listed in
  `operators/precedence` alongside the arithmetic tiers). A richer
  list/set-operator surface, deferred alongside the loop/range family,
  mirroring how [MA08](MA08-reduce-language.md) §4 deferred Reduce's own
  richer list-operator surface (`eq`/`memq`/`member`/`where`).
  `Set`/`List`, once a real handler lands (§5), can gain these one at a
  time.
- **`FAIL`, Maple's third truth value.** Confirmed real: Maple's boolean
  logic is three-valued (`true`/`false`/`FAIL` — Help page
  `type/truefalseFAIL`), and every comparison/logical operator in §3 can, in
  real Maple, produce `FAIL` rather than `true`/`false` for an
  indeterminate relation. The shared `symbolic-vm` engine's boolean model
  (the pre-bound `True`/`False` symbols every CAS-family language here
  already reuses) is strictly two-valued — representing `FAIL` faithfully
  would need new engine surface, not just wiring a token — so this subset's
  `true`/`false` bridge to the shared symbols (§3) and `FAIL` is out of
  scope for now.
- **Definite/bounded/multi-variable `diff`/`int` forms**
  (`int(f, x=a..b)` definite integration, `int(f, [x, y])` multiple
  integration, `diff(f, x1, x2, ...)` multi-variable, `diff(f, x$n)`
  higher-order — all confirmed real calling sequences on the `diff`/`int`
  Help pages). MP-4's in-scope base is the one-argument `diff(f, x)`/
  `int(f, x)` forms only (bridged to the shared `D`/`Integrate` handlers);
  the richer forms are deferred to their own follow-on items, matching how
  [MA07](MA07-derive-language.md) §4 deferred Derive's own richer
  `LIM`/multi-variable/higher-order `DIF` forms rather than landing
  everything in one PR.
- **`solve`, `simplify`, `expand`, `factor`, `taylor`, `series`**, and the
  rest of the `cas-*`-backed function surface under Maple names. Confirmed
  real Maple functions; deferred to their own follow-on items, one head at a
  time, matching how [MA04](MA04-wolfram-language.md)'s W-22 and
  [MA07](MA07-derive-language.md)/[MA08](MA08-reduce-language.md)'s own
  calculus items land incrementally rather than all at once.
- **`patmatch`/`match`, Maple's pattern-matching library functions**
  (confirmed real, per §1 — the `patmatch`/`match` Help pages). Not a scope
  exclusion in the usual sense: once ordinary function calls are wired
  (MP-4), `patmatch(expr, pattern, 's')` already *parses* and evaluates like
  any other unresolved call. It is called out explicitly here — rather than
  silently omitted — because the brief for this spec specifically asked
  that Maple's pattern-matching story be verified rather than assumed to
  resemble Wolfram's/Reduce's: real Maple exposes no dedicated
  pattern/rule-object *surface grammar* the way Wolfram's `_`/`->`/`/.` or
  Reduce's `let` do, so there is nothing for `cas-pattern-matching`'s
  `Blank`/`Pattern`/`Rule`/`RuleDelayed` vocabulary to bridge to at the
  surface level in this subset, now or later — a library-function surface,
  not a grammar-level one.
- **Comments** (`#` line comments, `(* ... *)` nestable block comments —
  both confirmed real on the `comment` Help page) and **2-D "Math" input**
  notation (palette-driven typeset input, as opposed to plain-text "Maple
  input"). Not part of this grammar; this subset's programs are a flat
  sequence of `;`/`:`-terminated expression statements, assignments,
  arrow-operator definitions, and `if` statements, matching how
  MA03/MA04/MA07/MA08's own subsets all start minimal and all target the
  plain-text surface rather than any GUI-typeset one.

These are surface-syntax gaps only where the *engine* (rewrite +
differentiation + integration, already implemented for Macsyma/Wolfram/
Derive/Reduce) already supports the corresponding operation — each is a
grammar/lexer/wiring addition in a later item, not an engine change — except
`for`/`while` loops and `FAIL`'s three-valued logic, which are genuinely new
engine surface (a loop primitive / a non-binary boolean domain respectively),
called out above rather than lumped in with the others, the same
distinction [MA08](MA08-reduce-language.md) §4 draws for Reduce's `let`
rules and block procedures.

## §5 Reuse strategy

- **Frontend:** the grammar-tools framework, exactly as Macsyma/MATLAB/
  Wolfram/APL/J/Derive/Reduce use it. `maple.tokens`/`maple.grammar` compile
  to committed `_grammar.rs` in `maple-lexer`/`maple-parser` (MP-2/MP-3).
- **Lowering + engine (MP-4, ✅ done, `maple-runtime`):** the parsed tree lowers to
  [`symbolic_ir::IRNode`](../packages/rust/symbolic-ir) (surface operators,
  `:=`, the arrow operator, and list/set literals → canonical `Add`/`Sub`/
  `Mul`/`Div`/`Pow`/`Neg`/`Equal`/`NotEqual`/`Less`/`Greater`/`LessEqual`/
  `GreaterEqual`/`And`/`Or`/`Not`/`Assign`/`Define`/`If`/`List`/`Set` heads),
  evaluated by [`symbolic_vm::VM`](../packages/rust/symbolic-vm) over the
  **stock** [`SymbolicBackend`](../packages/rust/symbolic-vm) — reused
  directly, unchanged, with **no** Maple-specific `Backend` at all.

  This claim is verified directly against the source in this repo, not
  copied from [MA07](MA07-derive-language.md)'s or
  [MA08](MA08-reduce-language.md)'s own spec prose — per the lesson
  [MA08](MA08-reduce-language.md) §5 itself discloses (its original wording
  overclaimed that `List`/list-accessors/`CompoundExpression` were "already
  implemented" for the shared backend, when only `List` actually was).
  Grepping `code/packages/rust/symbolic-vm/src/handlers.rs`'s
  `build_handler_table` confirms handlers exist for `Add`/`Sub`/`Mul`/`Div`/
  `Pow`/`Neg`/`Inv`/`Abs`/the trig-and-hyperbolic-function family/`Equal`/
  `NotEqual`/`Less`/`Greater`/`LessEqual`/`GreaterEqual`/`And`/`Or`/`Not`/
  `If`/`Assign`/`Define`/`List`, plus — because `SymbolicBackend::new`
  always builds the table with `simplify: true`
  (`code/packages/rust/symbolic-vm/src/backends.rs`) — `D`/`Integrate`/
  `Factor`/`Apart`/`Assume`/`Forget`/`ForgetAll`. Grepping
  `code/packages/rust/symbolic-vm/src/backend.rs`'s `BaseBackend::new`
  confirms the held-heads set is exactly `{Assign, Define, If, Assume,
  Forget}`. There is **no** handler for a `Set` head anywhere in the shared
  table — unsurprising, since no language in this repo has asked for one
  before — so this subset's `{a, b, c}` set literal lowers to the
  structurally-correct `Set[a, b, c]` (arguments still evaluate) but
  evaluates as an unresolved call today, exactly the same disclosed gap
  [MA08](MA08-reduce-language.md) §5 documents for Reduce's own
  `CompoundExpression`/list-accessor heads, until a follow-on item adds a
  real handler (to the shared table, or to a narrowly-scoped Maple
  `Backend` — a decision for that later item, not this one).

  Also verified: introducing a brand-new canonical `Set` head does not
  collide with Wolfram's own surface-level use of the string `"Set"` (in
  `x = e` / `Set[x, e]`, per [MA04](MA04-wolfram-language.md) §7.1).
  Grepping `code/packages/rust/wolfram-runtime/src/lower.rs` shows that
  surface spelling is bridged straight to the canonical `Assign` head during
  lowering (`"Set" => symbolic_ir::ASSIGN`) and never itself persists as a
  literal `"Set"` `IRNode` the VM dispatches on — so Maple's `Set` head,
  which *does* persist as a real canonical head here, shares no runtime
  meaning with Wolfram's transient surface spelling of the same word.

  `diff`/`int` are thin calls into the same `cas-*`-backed `D`/`Integrate`
  handlers Derive's `DIF`/`INT` and Wolfram's `D`/`Integrate` already call
  under their own names — one function, four languages agreeing on its
  result, the same "reuse, not reimplementation" story §1 promises.
- **REPL (MP-4, ✅ done, `maple-repl`):** a single-threaded driver mirroring
  `reduce-repl`/`derive-repl`/`wolfram-repl`/`maxima-repl`; a plain
  (non-numbered) read-eval-print loop, matching real Maple's own
  interactive-session convention (§5.3 "Statement Separators" — a
  `;`/`:`-terminated statement, no `#n:`/`In[n]:=` numbering), the same
  convention [MA08](MA08-reduce-language.md) documents for `reduce-repl`.
  One addition beyond bracket balance: Maple's `if_expr` (unlike Reduce's)
  requires an explicit `end if`/`fi` closer, so `maple-repl`'s own
  continuation heuristic also tracks `if`/`end if`|`fi` block-keyword
  balance — closer in spirit to `matlab-repl`'s/`octave-repl`'s own
  keyword-block tracking than to any other CAS-family REPL in this repo.
  The **`maple` binary itself is not a separate `code/programs/rust/maple`
  crate** — verified directly against the sibling CAS-family languages
  (`reduce`, `derive`, `wolfram`): none of them has a `code/programs/rust/`
  entry either; each `-repl` crate's own `Cargo.toml` declares the binary
  directly via `[[bin]] name = "..."`, and `maple-repl` follows that exact,
  empirically-confirmed convention rather than the generic `code/programs/
  rust/<name>` shape this repo's `CLAUDE.md` "Project Structure" section
  describes for standalone programs in general.

## §6 References

Internal: [`HML00`](HML00-historical-math-languages-roadmap.md) (roadmap,
Wave 5), [`MA07`](MA07-derive-language.md)/[`MA08`](MA08-reduce-language.md)
(the two prior Wave-5 kickoffs this spec mirrors most closely — MA08
especially, both for its "verify against real source, not spec prose"
methodology, reused directly in §5 above, and for being the sibling item
this one closes Wave 5 alongside), [`MA03`](MA03-maxima-language.md)/
[`MA04`](MA04-wolfram-language.md) (the earlier symbolic-family kickoffs),
`symbolic-ir`, `symbolic-vm`, `cas-pattern-matching`.

External: Keith O. Geddes & Gaston H. Gonnet et al., *Maple* (Symbolic
Computation Group, University of Waterloo, 1980–82; now developed and sold
by Waterloo Maple Inc. / Maplesoft). The current Maplesoft online Help
system, consulted directly (not assumed from CAS-family resemblance) —
specifically:

- The **Maple Programming Guide** (L. Bernardin, P. Chin, P. DeMarco,
  K. O. Geddes, et al.): Chapter 3 "Maple Expressions" §3.9 "Arithmetic
  Expressions," §3.10 "Boolean and Relational Expressions," §3.11
  "Expressions for Data Structures"; Chapter 4 "Basic Data Structures" §4.3
  "Immutable Data Structures" (Lists, Sets); Chapter 5 "Maple Statements"
  §5.3 "Statement Separators," §5.5 "Assignments," §5.6 "Flow Control";
  Chapter 6 "Procedures" §6.2 "Defining and Executing Procedures," §6.3
  "Parameter Declarations," §6.5 "The Procedure Body." Reachable from
  <https://www.maplesoft.com/support/help/> under "Programming Guide."
- Standalone Maple Help topic pages consulted directly: `assignment` (`:=`),
  `equation` (relational operators, `=`/`<>`), `operators/precedence` (the
  full operator-precedence table), `operators/functional` (the arrow/
  functional-operator notation), `exprseq` (expression sequences), `set`
  (set vs. list semantics), `procedure` (the `proc ... end proc` syntax
  summary), `remember` (remember tables — the real meaning of
  `f(x) := expr`), `comment` (`#` and `(* ... *)`), `if` (`if`/`elif`/
  `else`/`end if`/`fi`), `do` (`for`/`while` loop syntax), `diff`, `int`,
  `patmatch`, `match`, `op` (`op`/`nops` calling sequences), `arithop`
  (arithmetic operators, the explicit-`*` requirement), and
  `type/truefalseFAIL` (three-valued boolean logic).
