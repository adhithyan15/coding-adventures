# MA13 — Axiom: the strongly-typed CAS with a category/domain type system

## Status

Design-only kickoff (**MA-13a**). Wave 7 of the historical math-languages
roadmap ([`HML00`](HML00-historical-math-languages-roadmap.md) §7) — the
**first** of Wave 7's two languages, ahead of Julia (subset), and the first
item run after Wave 6 (J, Scilab, Q, IDL) closed out. No `code/grammars/`
files and no crate land in this item — only the language design, so that the
answer to this spec's central question (§3) is fixed *before* any
lexer/parser/runtime code exists, exactly as MA07 fixed Derive's
`:=`-expression grammar, MA09 fixed Maple's three-aggregate-type trap, MA11
fixed Q's function-literal evaluator problem, and MA12 fixed IDL's
keyword-argument calling convention before their own implementation PRs.

HML00 §5's survey table gives Axiom one honestly-thin line: *"Strongly
typed CAS (category/type system) — the hardest; later."* Following the same
discipline every prior kickoff applied to its own one-line description (MA10
checking Scilab's `+` was genuinely not MATLAB's; MA11 discovering "K/Q" was
two languages; MA12 checking which IDL and which subscript era), this spec
does not accept "the hardest" as a vague warning to wave away. It checks —
against the original 1992 Jenks & Sutor Axiom book, read directly (see §1 for
which edition and why) — **which** Axiom this repo targets (§1), whether the
existing symbolic substrate (`symbolic-ir`/`symbolic-vm`/`cas-*`) covers
Axiom's value model (§2 — the answer is genuinely different from every prior
symbolic-family kickoff), and the **one genuinely new, hard problem** —
Axiom's category/domain type system itself — deciding exactly how much of it
a first cut should build (§3).

**Conclusion, stated plainly up front.** Axiom's *arithmetic/rewrite engine*
is the same reusable `symbolic-vm`/`cas-*` substrate every CAS-family
language in this repo already drives — no new math. What is genuinely new,
and unlike every prior symbolic-family kickoff (Maxima, Wolfram, Derive,
Reduce, Maple — each found "reuse the shared engine completely unchanged, only
a new frontend"), is that `symbolic-vm`'s `IRNode` carries **no domain/type
tag at all** — nothing in this repo's symbolic substrate has ever needed one,
verified directly against the source (§2). The first cut therefore adds a new
`axiom-runtime`-internal layer (a domain-tagged value, plus a **fixed,
non-extensible** table of built-in domains and categories) covering only the
**interactive-language, consumer view** of Axiom's type system (declare with
`:`, coerce with `::`, query membership with `has`) — deferring whole the
**library-language, producer view** (user-defined categories/domains/packages
via `Join`, conditional exports, and symbolic-domain-parameterized generic
functions) that is Axiom's actual claim to fame, exactly as the book's own
two-part structure (Chapters 0/1/2/5/6 for interactive users; Chapters 11-13
for library writers) already separates them (§3).

## §1 Why Axiom, and which Axiom — targeting the language, verified via FriCAS's continuation of the original Jenks & Sutor book

Axiom's own history already contains a fork question of exactly the kind
MA11 §1 had to resolve for K/Q and MA12 §1 had to resolve for IDL's
version era — so this spec resolves it the same way: by checking, not
assuming.

**The lineage.** IBM's Scratchpad project (James Griesmer, 1965, Fortran)
never reached public release. A second system, **Scratchpad II**, began in
1977 at IBM's Thomas J. Watson Research Center under Richard D. Jenks, with
Davenport, Trager, Yun, Miller, Sutor, Daly, and others; around 1990 IBM
commercialized it as **Axiom**, and later sold it to NAG (Numerical
Algorithms Group). In 2001 NAG withdrew Axiom from the market, and in 2002 it
was re-released under the Modified BSD license with Tim Daly as lead
developer. In 2007, a disagreement about project direction produced **two
forks** — **OpenAxiom** and **FriCAS** (forked by Waldek Hebisch, encouraged
by Daly) — while the original Axiom project continued independently under
Daly. All three lines are still nominally alive today (FriCAS is by far the
most actively released of the three, with stable releases into 2026).

**Why this doesn't fork the *language* the way K/Q did.** MA11 §1 had to pick
between K and Q because they are genuinely two different surface grammars
sharing one engine. Axiom/OpenAxiom/FriCAS are the opposite case: the 2007
split was a **project-governance** disagreement, not a language-design
split — all three share the same SPAD library language, the same interactive
language (`:=`/`==`, `if`-`then`-`else`, blocks), and the same category/
domain/package type-system architecture (categories, domains, dependent
typing via domain-parameterized constructors) inherited unchanged from the
pre-fork Axiom. So, unlike MA11's K-vs-Q decision, this spec has no
surface-grammar fork to pick a side of: **this spec targets Axiom the
language** — the interactive language and category/domain type system
described by the original Axiom book — which every one of the three current
projects still implements essentially unchanged.

**Which edition of the documentation, and why.** The canonical primary
source is Jenks & Sutor, *"AXIOM — The Scientific Computation System"*
(Springer, 1992), released as open text in 2002 alongside the BSD
re-release. The original `axiom-developer.org` hosting did not resolve
directly for this spec's own verification pass (DNS failure at fetch time)
— the same "canonical host doesn't serve cleanly to a plain fetcher"
situation MA12 §7 disclosed for NV5's IDL documentation, resolved the same
way: by citing a well-known, honestly-disclosed continuation of the primary
source rather than a paraphrase. **This spec verified every syntax claim in
§2/§3/§4 directly against `https://fricas.github.io/book.pdf`** — a PDF whose
own first page states plainly: *"This is the original Axiom book from 1992
with title 'AXIOM—The Scientific Computation System' by Jenks and Sutor in
its adaptation for the FriCAS fork of the Axiom source code... It counts as
the official version for the FriCAS project... released in 2002 under the
modified BSD license."* This is not a paraphrase or a fork's own
reinterpretation — it is the same 1992 book, continuously regenerated from
the same original sources (last regenerated March 2026 as of this spec's
research pass), and it remains the single most current, most freely
fetchable, primary-source edition of the exact text this spec needs.

**Decision, stated plainly:** this spec, and every item under it, targets
**Axiom the language** (interactive language + category/domain type system),
verified against the FriCAS-hosted continuation of the original 1992 Jenks &
Sutor book. Crates and grammar files are named `axiom-*` /
`code/grammars/axiom/` throughout — matching HML00's own naming for this
wave item — **not** `fricas-*`; the FriCAS project is cited here purely as
this spec's documentation source, per the finding above that the language
itself is not fork-specific.

## §2 Substrate check: `symbolic-ir`/`symbolic-vm`/`cas-*` cover expression evaluation; none of them have any concept of domain, category, or a per-value type tag

Every prior symbolic-family kickoff in this repo — Wolfram (MA04), Derive
(MA07), Reduce (MA08), Maple (MA09) — reached the same finding: the surface
grammar is new, but the *engine* underneath (`symbolic_ir::IRNode`,
`symbolic_vm::VM` with the shared `SymbolicBackend`/`build_handler_table`,
and the `cas-*` algorithm crates) is reused **completely unchanged**, with no
custom `Backend` needed at all. This spec checked whether that finding
transfers a fifth time, directly against the source rather than assumed from
family resemblance — and it does **not** transfer completely, for a reason
specific to what makes Axiom Axiom.

**What is confirmed to transfer, verified directly against the source.**
`symbolic_ir::IRNode` (`Symbol(String)`, `Integer(i64)`, `Rational(i64,i64)`,
`Float(f64)`, `Str(String)`, `Apply(Box<IRApply>)`) is exactly the value
shape Axiom's own arithmetic needs — an `Integer`, a `Fraction(Integer)` (an
already-reduced rational pair), a `Float`, or a compound expression head
applied to arguments. Grepping
`code/packages/rust/symbolic-vm/src/handlers.rs`'s `build_handler_table`
confirms handlers already exist for `Add`/`Sub`/`Mul`/`Div`/`Pow`/`Neg`/`Inv`/
`Abs`/the full trig-and-hyperbolic family/`Equal`/`NotEqual`/`Less`/
`Greater`/`LessEqual`/`GreaterEqual`/`And`/`Or`/`Not`/`If`/`Assign`/`Define`/
`List`, plus (when `simplify: true`, which `SymbolicBackend::new` always
sets) `D`/`Integrate`/`Factor`/`Apart`/`Assume`/`Forget`/`ForgetAll` — and
`code/packages/rust/symbolic-vm/src/backend.rs`'s `BaseBackend::new` confirms
the held-heads set is `{Assign, Define, If, Assume, Forget}`. Axiom's
`Integer`/`Fraction(Integer)`/`Polynomial(Integer)` arithmetic is not new
math: computing `+`/`-`/`*`/`^` once a value's domain has been decided is
exactly the same evaluation this shared table already performs for
Macsyma/Wolfram/Derive/Reduce/Maple under their own names. One representation
subtlety worth being explicit about (since it shapes §3/§4's built-in domain
table): only `Fraction(Integer)` gets `symbolic-ir`'s packed native
`IRNode::Rational(i64, i64)` representation; a `Fraction` of anything richer
(a rational *function*, `Fraction(Polynomial(Integer))`) has no equivalent
packed form and would need to be represented the way every other CAS-family
language here already represents `a / b` in general — an ordinary
`Apply(Times, [a, Apply(Pow, [b, -1])])` tree, normalized by the same
`cas-simplify` machinery. This is a real, disclosed reason (not an arbitrary
one) for §4's decision to keep this cut's built-in `Fraction`/`Polynomial`
domains parameterized over `Integer` only, not over each other.

**What does not transfer, and is the real substrate finding of this spec.**
Grepping `symbolic-ir`'s `IRNode` definition and every type in
`code/packages/rust/symbolic-ir/src/` turns up **no field, variant, or
concept anywhere resembling a domain, a category, or a per-value type
tag** — every `IRNode::Integer(i64)` is simply "an integer," with no
attached notion that it belongs to `Integer` as opposed to `PositiveInteger`
or `NonNegativeInteger`, and no way for a program to ask whether some
expression's domain `has Ring`. This is unsurprising: no prior language in
this repo's symbolic family has ever needed one — Macsyma/Wolfram/Derive/
Reduce/Maple are all, at the value-model level, a single flat universe of
untyped symbolic expressions, and their surface-level `:=`/`assignment`
constructs bind names to expressions, never to *domains*. Axiom is
different in exactly the respect HML00 flags: **every Axiom value belongs to
a domain, and reasoning about domains — declaring one (`:`), coercing to one
(`::`), and querying category membership (`has`) — is a first-class,
constantly-used part of even the most basic Axiom session** (§3, §4).

**The finding, stated plainly:** no `symbolic-ir`/`symbolic-vm`/`cas-*`
crate needs to change — the arithmetic/rewrite core is reused exactly as
Wolfram/Derive/Reduce/Maple already reuse it. But `axiom-runtime` needs an
entirely new layer sitting on top of that engine: its own domain-tagged
value (an `IRNode` paired with the `AxiomDomain` it currently belongs to) and
a fixed domain/category membership table (§3) it can consult for `::` and
`has`. This is the one piece of genuinely new *evaluator* design this spec's
family has not needed before — parallel in kind to how `q-runtime`'s `QFn`
(MA11 §2) and `idl-runtime`'s `IdlCallable` (MA12 §3) were each a
runtime-internal addition needed by one language's own novel feature, but
larger in scope here, because the new concept (domain/category identity
itself, threaded through every value and every operation) is more
fundamental than a callable representation.

## §3 The one genuinely new, hard problem: Axiom's category/domain type system — and the first-cut scoping decision

**What the real type system actually is, grounded directly in the book.**
Axiom organizes every object under a **domain of computation** (a "domain"):
`Integer` denotes "the class of integers," `Float` denotes floating-point
numbers, and so on. Domains are built by **domain constructors**:
`Polynomial(Integer)` ("polynomials over the integers"), `Matrix(Float)`,
`List(Matrix(Polynomial(Complex(Fraction(Integer)))))` — arbitrarily deep
nesting, with no restriction on constructor arity. A domain can be refined to
a **subdomain** via a *membership predicate*: `PositiveInteger` is the
subdomain of `Integer` satisfying `x > 0`; "any domain is a subdomain of
itself." Domains are themselves objects with a type, and the type of a
domain is called a **category** — `Ring` (constants `0`/`1`, operations
`+`/`-`/`*`, satisfying ring axioms), `Field`, `OrderedSet`, `IntegralDomain`,
and so on, forming a directed-acyclic hierarchy. A domain must **assert**
which categories it belongs to (category membership is not inferred merely
from having the right-named operations — real Axiom's own worked example is
that `Boolean` under APL-style `0`/`1` encoding of `and`/`or` as `*`/`+` would
export every `Ring` operation syntactically, yet is correctly *not* asserted
to be a `Ring`, since the additive-inverse axiom fails). The infix `has`
operator queries this: `Polynomial(Integer) has Ring` is `true`;
`Matrix(Integer) has Ring` is `false` (matrix `+`/`-`/`*` are undefined for
arbitrary shapes). Domains and categories are themselves defined by ordinary
Axiom programs: a domain is `Name(...): Exports == Implementation`; a
category is `Name(...): Category == Exports`, where `Exports` can `Join`
several parent categories (`Join(OrderedSet, IntegralDomain, ...) with ...`)
and can assert **conditional** exports (`Ring with ... if R has Field then
Field ...`, real Axiom syntax, confirmed directly). Functions can be written
generically over an abstract category-bound domain variable — `R: Ring;
power: (R, NonNegativeInteger): R -> R; power(x, n) == x ** n` — and the
interpreter builds domain towers dynamically at runtime from user input
(entering `matrix [[x + %i, 0], [1, -2]]` causes the interpreter to load
`Matrix`, `Polynomial`, `Fraction`, and `Integer` from the library and build
the tower "matrices of polynomials of complex integers" on the fly, with
categories "policing" which constructor arguments are meaningful —
`Polynomial(String)` is rejected outright as "not a valid type," confirmed
directly against the book's own worked example). A **package** is a
domain-like construct whose operations only take and return *other* domains'
objects rather than having any objects of its own (`PolynomialFunctions2`,
the vehicle for factorization/integration/equation-solving/limits). Finally,
`Record(selector: type, ...)` and `Union(type, ...)` give heterogeneous
aggregate and sum types (with or without named selectors, `case` to test a
branch, `retractIfCan` to attempt narrowing), and `Any` is an
escape-hatch domain (an internally tagged `[value, domain-badge]` pair) for
mixed-type collections.

**This is enormous, and HML00 is right to flag it as the hardest.** The
mechanism above — user-declarable categories, user-declarable domains,
`Join`, conditional exports, packages, symbolic-domain-parameterized generic
functions, `Record`/`Union`/`Any` — is not a corner of Axiom's syntax; it is
the entire subject of the book's Part III ("Advanced Topics," specifically
its own recommended-reading Chapters 11–13 on writing new domains,
categories, and packages), representing a purpose-built compiler (SPAD) and
a library of over a hundred domains and hundreds of categories built up over
more than a decade by a large research team. Reconstructing that whole
mechanism as a first cut is exactly the scope HML00 §5 warns against ("the
hardest; later").

**The book's own structure draws the responsible cut line for us.**
Chapters 0–2 (and 5–6) — "Introduction to FriCAS," "An Overview of FriCAS,"
"Using Types and Modes," "Introduction to the FriCAS Interactive Language,"
"User-Defined Functions, Macros and Rules" — teach the **interactive
language**: being a **consumer** of an already-built, fixed universe of
domains and categories. A consumer *declares* a variable's type (`a :
PositiveInteger`), *coerces* a value to another domain (`3 :: Fraction
Integer`), *queries* category membership (`Polynomial(Integer) has Ring`),
and watches the interpreter build domain towers and retract/simplify results
to the least-general domain that still holds the value (`x + 3 - x` stays
`Polynomial(Integer)` even though it simplifies to the *value* `3`, "so no
information is lost" — confirmed verbatim). None of this requires writing a
single new domain or category. Chapters 11–13 — squarely "for further
information ... where these ideas are explained in greater detail," per the
book's own Chapter 0 closing line — teach the **library/programming
language**: being a **producer**, defining brand-new categories and domains
yourself. This is precisely the line this spec draws.

**Decision, stated plainly.** The first cut implements the **consumer view
only**, over a **small, fixed, non-extensible table of built-in domains and
categories** hard-wired into `axiom-runtime` — not a general category-algebra
engine. Concretely:

- **In scope:** the `:` declaration syntax, the `::` coercion operator, and
  the `has` category-membership query — all resolved against a fixed,
  hard-coded lookup table (not a computed `Join`/conditional-export
  algebra), covering a small built-in domain set (§4) and a small built-in
  category set (`Ring`, `OrderedSet` — enough to make the book's own
  worked `has Ring`/`has Field`-style queries meaningful over this cut's
  domains without needing `Field`'s own richer conditional-export logic,
  which is deferred, see below). Subdomain membership (`PositiveInteger`
  as "`Integer` where `x > 0`") is implemented the same way the book
  describes it conceptually — a predicate function checked at coercion
  time — rather than as a generative subdomain-definition mechanism a user
  could extend.
- **Deferred whole, exactly as MA11 §4 deferred Q's own tables (Q's actual
  "reason for existing") as "real, substantial follow-on work, not a corner
  being permanently cut":** user-definable categories (`Name(...): Category
  == Exports`), user-definable domains (`Name(...): Exports ==
  Implementation`), `Join`, conditional exports (`if R has X then ...`),
  packages, and symbolic-domain-parameterized generic function signatures
  (writing a function generic over an abstract `R: Ring` type variable, as
  opposed to writing it against one of this cut's fixed concrete domains).
  This is, plainly stated, Axiom's actual claim to fame — the *producer*
  side of the category/domain system is deferred whole, not the least
  interesting part of it. `Record`/`Union`/`Any` (a separate, real,
  heterogeneous/dynamic-aggregate value-model addition, structurally
  parallel to the "deferred whole" treatment Scilab's `list`/`tlist`/
  `mlist` (MA10 §1) and IDL's structures (MA12 §4) each got) are deferred
  alongside them.

This is deliberately the smallest slice still recognizably **Axiom** and not
merely "Derive or Maple again with different spelling": unlike every other
CAS-family language in this repo, a program in this first cut can write
`a : PositiveInteger`, ask `Polynomial(Integer) has Ring` and get `true`, and
watch a coercion genuinely fail with an Axiom-shaped error
(`Cannot convert right-hand side of assignment ... to an object of the type
Integer of the left-hand side`, confirmed verbatim from the book) — the
category/domain vocabulary is real and load-bearing in this cut, not a
grammar footnote — while the *generative* half of the same system (which is
what would require reconstructing something close to the SPAD compiler
itself) is honestly deferred to its own future item.

**One further, disclosed divergence: `=` defaults to an `Equation`, not
`Boolean`, in real Axiom.** Confirmed directly: "by default, the equal sign
`=` creates an equation" — `x + 1 = y` has type `Equation(Polynomial(Integer))`,
not `Boolean`, *except* inside an `if` predicate (where FriCAS "places a
default target type of `Boolean` on the predicate," so `=` need not be
qualified there) or when an explicit `Boolean` target type is supplied. Every
other comparison this cut needs (`<`, `<=`, `>`, `>=`, and Axiom's own
not-equal spelling `~=`, confirmed directly — not Maple's `<>`, not
Wolfram's `!=`) already default to `Boolean` in real Axiom with no such
wrinkle. This cut, like every sibling CAS-family language in this repo,
lowers a bare `=` straight to the shared boolean-producing `Equal` handler
(§2) unconditionally — the same simplification Reduce/Derive/Maple/Wolfram
all make — which means this cut does **not** reproduce real Axiom's default
`Equation`-object behavior for a bare top-level `a = b`. This is called out
explicitly, as a real and disclosed divergence rather than a silently
dropped one, per this repo's honesty-about-subsets convention; a genuine
`Equation` domain is deferred alongside `Record`/`Union`/`Any` above.

## §4 Language scope (the historical core)

In scope for the first cut — a faithful "textbook Axiom session" subset,
following the same honesty-about-subsets convention as every language here,
grounded directly in the FriCAS-hosted Axiom book (§1):

| Surface | Meaning | Lowers to |
|---------|---------|-----------|
| `123`, `1.5` | integer / float literal, domain-inferred as `PositiveInteger`/`Integer`/`Float` | `Integer` / `Float` (§2) |
| `1/3` | exact rational, domain `Fraction(Integer)` | `IRNode::Rational` (§2) |
| `x`, `foo` | symbol/variable | `Symbol` |
| `"hello"` | string literal, domain `String` | `Str` |
| `f(a, b)`, `f a` (single-argument, paren-optional — confirmed real, `factorial 7`, `ff z`) | function call | `Apply` |
| `[a, b, c]` | list, domain `List(T)` | `List[a, b, c]` (existing shared `List` handler, §2) |
| `a + b`, `a - b`, `a * b`, `a / b`, `a ^ b` / `a ** b` (both spellings real, confirmed — mirroring [Reduce](MA08-reduce-language.md)'s own `^`/`**` pair, MA08 §3) | arithmetic, precedence `^`/`**` (highest) → `*`/`/` → `+`/`-` (lowest), confirmed directly from the book's own basic-tour precedence statement | `Pow`/`Mul`/`Div`/`Add`/`Sub` |
| `a = b` | equality — **lowers straight to `Equal`/Boolean in this cut** (§3's disclosed divergence from real Axiom's default `Equation`) | `Equal` |
| `a ~= b`, `a < b`, `a <= b`, `a > b`, `a >= b` | comparison (real Axiom spelling — `~=` not `<>`/`!=`) | `NotEqual`/`Less`/`LessEqual`/`Greater`/`GreaterEqual` |
| `x := e` | immediate assignment (evaluate `e` now, bind) | `Assign[x, e]` |
| `f(x: T, ...): T == e`, undeclared `f x == e` | function definition (held body; undeclared form is duck-typed per call, evaluated rather than "recompiled" since this is an interpreter, not Axiom's own compiler) | `Define[f, [x, ...], e]` |
| `if p then e1 else e2` | conditional (`p` coerced to `Boolean`; missing `else` — deferred, see below) | `If[p, e1, e2]` |
| `( e1; e2; ...; eN )` | a parenthesised, semicolon-separated block; value is the last expression's value | sequencing over the existing evaluation order — no early-exit `=>` this cut (deferred below) |
| `a : T`, `(a, b, c) : T` | declaration, restricting the domain a name may hold | consulted at assignment/coercion time against the fixed domain table (§3) |
| `e :: T` | coercion to domain `T` | consulted against the fixed domain table; errors with the book's own confirmed error shape if impossible |
| `D has C` | category-membership query (`D`, `C` restricted to the fixed built-in tables, §3) | a `Boolean` result from the fixed lookup table, not a computed `Join` |
| `( … )` | grouping | — |

**Built-in domains, fixed and non-extensible (§3):** `Boolean`, `Integer`,
`PositiveInteger` (subdomain: `x > 0`), `NonNegativeInteger` (subdomain:
`x >= 0`), `Float`, `String`, `Fraction(Integer)`, `Polynomial(Integer)`,
`List(T)` for `T` among the domains just listed. Deliberately **not**
parameterized over each other beyond this (no `Polynomial(Fraction(Integer))`,
no `Fraction(Polynomial(Integer))`, no `Complex(T)`, no `Matrix(T)`/
`SquareMatrix` this cut) — keeping the fixed table genuinely small and finite
is itself part of the scoping decision (§3): general recursive
constructor-composition is exactly the "producer-side" generality being
deferred, not an oversight.

**Built-in categories, fixed and non-extensible (§3):** `Ring` (asserted for
`Integer`, `Fraction(Integer)`, `Polynomial(Integer)`) and `OrderedSet`
(asserted for `Integer`, `Float`, `PositiveInteger`, `NonNegativeInteger`) —
enough to make this cut's own `has` queries real and checkable
(`Polynomial(Integer) has Ring` → `true`; `List(Integer) has Ring` → `false`,
mirroring the book's own confirmed `List(Integer) has Ring` example) without
needing `Field`'s richer conditional-export machinery.

**Deferred (post-MA-13), each a follow-on item exactly as every sibling
language here deferred its own harder extras:**

- **User-defined categories and domains** (`Name(...): Category == Exports`,
  `Name(...): Exports == Implementation`), `Join`, conditional exports
  (`if R has Field then Field ...`), packages, and symbolic-domain-
  parameterized generic function signatures (`R: Ring` as a function-signature
  type variable) — the headline deferral (§3): the entire *producer* side of
  the category/domain system, Axiom's actual reason for existing, deferred
  whole as real, substantial future work, not a corner permanently cut.
- **`Record`/`Union`/`Any`** — heterogeneous aggregate and sum types, and the
  dynamic `Any` escape hatch (confirmed real, Chapter 2 §§2.4–2.6). A real,
  separate value-model addition, the same category as Scilab's `list`/
  `tlist`/`mlist` (MA10 §1) and IDL's structures (MA12 §4) — deferred
  alongside the type-system producer side, its own future item.
- **`Matrix`/`SquareMatrix`, `Complex`, and richer domain-constructor
  nesting** beyond §4's fixed table — deferred alongside real matrix/complex
  *algebra*, matching how [Derive](MA07-derive-language.md) D-5 deferred
  actual matrix algebra to its own later item.
- **A genuine `Equation` domain** for bare `=` (§3's disclosed divergence) —
  deferred; this cut's `=` always produces `Boolean`.
- **Macros** (`macro name == body`, `macro name(args) == body`) — confirmed
  real, a purely textual, unhygienic, untyped substitution mechanism
  (Chapter 6 §6.2), explicitly **unrelated** to the category/domain type
  system this spec's own central decision is about. Deferred as ordinary
  further-out interactive-language surface, not part of this spec's headline
  finding.
- **Delayed assignment of a plain variable** (`a == 1`, dependency-tracked
  and automatically recompiled when a dependency is redefined — confirmed
  real, Chapter 5 §5.1, distinct from `==`'s function-definition role).
  This cut's `==` is in scope **only** in the function-definition form
  (`f(...) == e` / `f x == e`); the bare-variable delayed/dependency-tracked
  rule form has no clean `symbolic-vm` analogue (it is a session/
  interpreter caching concern, not a type-system one) and is deferred.
- **Block early-exit (`=>`)**, **piecewise/multi-clause function definitions**
  (the book's own first meaty tutorial example — `p(0) == 1`, `p(1) == x`,
  `p(n) == ...` dispatching on the *value* of the argument, not just its
  domain — confirmed real, a genuinely separate, non-trivial dispatch
  mechanism no other CAS-family language in this repo's own subset has: write
  the equivalent nested `if` in this cut instead, mirroring how
  [Derive](MA07-derive-language.md) §4 deferred prime-notation sugar over an
  already-expressible base form), **anonymous "maps-to" functions** (`+->`),
  **list comprehensions** (`[e for x in a..b]`), **`for`/`while` iteration
  and streams** — all confirmed real, all deferred as further, separate
  interactive-language surface, matching how [Maple](MA09-maple-language.md)
  §4 deferred its own `for`/`while` loop family for the identical reason (no
  existing `symbolic-vm` loop-primitive handler for the CAS-family side of
  this repo's shared engine).
- **Package-calling `$` and target-type `@`** — confirmed real
  (`content(2)$Polynomial(Integer)`, `(2 = 3)@Boolean`) but tightly coupled to
  packages (deferred above) and to `=`'s target-type-sensitive `Equation`/
  `Boolean` duality (deferred above, §3); `::` alone covers this cut's
  coercion needs. Deferred.
- **HyperDoc, graphics, Fortran/TeX/MathML output formats, `.input`-file
  pile syntax, and `)`-prefixed system commands** (`)abbreviation`, `)what`,
  `)clear all`, …) — session/tooling surface, not language surface; not part
  of this grammar, matching how MA07/MA08/MA09 all excluded worksheet-level
  conveniences from their own first cuts.

## §5 Reuse strategy

- **Frontend:** the grammar-tools framework, exactly as every other
  frontend in this repo, per
  [`feedback_no_handwritten_lexers_parsers`]. `code/grammars/axiom/
  axiom.tokens` + `axiom.grammar` compile to committed `_grammar.rs` in
  `axiom-lexer`/`axiom-parser` (MA-13b/c). Axiom's arithmetic/comparison/
  logic surface is an ordinary infix expression grammar, closer in overall
  shape to Reduce/Derive/Maple's `head(args)`-style CAS grammars than to any
  array-family grammar in this repo — the genuinely new productions are the
  declaration (`:`), coercion (`::`), and category-query (`has`) syntax
  fixed by §3/§4, none of which any prior symbolic-family grammar here has
  needed.
- **Lowering + engine (MA-13d):** the parsed tree lowers to
  [`symbolic_ir::IRNode`](../packages/rust/symbolic-ir) exactly as Wolfram/
  Derive/Reduce/Maple already do (surface operators → canonical `Add`/`Sub`/
  `Mul`/`Div`/`Pow`/`Neg`/`Equal`/`NotEqual`/`Less`/`Greater`/`LessEqual`/
  `GreaterEqual`/`Assign`/`Define`/`If`/`List` heads, all confirmed already
  handled by the shared `SymbolicBackend`, §2) — reused **unchanged**, with
  no Axiom-specific `Backend` needed for arithmetic itself. On top,
  `axiom-runtime` adds its own new layer (the one piece of genuinely new
  evaluator design this spec fixes, §2/§3): an `AxiomValue` pairing an
  `IRNode` with its current `AxiomDomain` tag (an enum over exactly §4's
  fixed domain table), and a fixed `AxiomDomain × AxiomCategory → bool`
  lookup table `has` and `::` consult — evaluated entirely within
  `axiom-runtime`'s own dispatcher, never inside `symbolic-vm` itself, the
  same "runtime-internal, not a shared-crate change" shape MA11 §2's `QFn`
  and MA12 §3's `IdlCallable` already established for their own novel
  features.
- **REPL (MA-13d):** `axiom-repl`, mirroring `derive-repl`'s numbered-prompt
  convention rather than Reduce/Maple's plain unnumbered loop — real
  Axiom's own interactive prompt is itself numbered (confirmed directly:
  `(1) ->`, incrementing per computation step, with `%`/`%%(n)` referring
  back to prior results by step number, Chapter 1 §1.3.2), so `axiom-repl`
  should track and display that step counter, the closest match among this
  repo's existing CAS REPLs.
- **`symbolic-ir`/`symbolic-vm`/`cas-*`**: reused for arithmetic/rewriting
  exactly as Macsyma/Wolfram/Derive/Reduce/Maple already do (§2) — the
  substrate needs **no** changes; `array-runtime`/`matrix-runtime` are
  **irrelevant** here, matching HML00 §2's own CAS-vs-array substrate split
  (Axiom is Stream B, symbolic CAS, not Stream A, numerical/array).
- **`HML01`'s `-to-semantic-ir` convention**: per
  [`HML01`](HML01-math-to-semantic-ir.md) §2's amended per-language pattern
  and every prior kickoff's precedent, `axiom-to-semantic-ir` is built
  **alongside** the runtime in this same wave, not bolted on afterward.
  Lowering the arithmetic/comparison/assignment/function-definition surface
  is ordinary, reusing whatever `Expr` vocabulary Wolfram's/Derive's/Reduce's/
  Maple's own `-to-semantic-ir` crates already established for the shared
  symbolic domain. Lowering `:`/`::`/`has` — Axiom-specific surface with no
  analogue in any prior symbolic-family frontend's own SIR lowering — is an
  **open question left to that later item**, the same "depends on what the
  shared IR already has by the time the frontend starts" deferral MA11 §5 and
  MA12 §5 each made for their own open lowering questions, not resolved
  here.

## §6 Crate layout and rollout (one item = one PR)

```
axiom-lexer/          src/{lib.rs, _grammar.rs}   ← MA-13b (+ code/grammars/axiom/axiom.tokens)
axiom-parser/         src/{lib.rs, _grammar.rs}   ← MA-13c (+ code/grammars/axiom/axiom.grammar)
axiom-runtime/        src/{lib.rs, eval.rs, value.rs, domains.rs, builtins.rs}   ← MA-13d
axiom-repl/           src/{lib.rs, main.rs}       ← MA-13d (the `axiom` binary)
axiom-to-semantic-ir/ src/{lib.rs, lower.rs}      ← MA-13e
```

- **MA-13a — this spec.** The which-Axiom decision (§1), the substrate check
  finding `symbolic-ir`/`symbolic-vm`/`cas-*` need no changes but carry no
  domain/category concept at all (§2), and the one genuinely new,
  hard problem — Axiom's category/domain type system, scoped to a fixed,
  non-extensible, consumer-only subset (§3) — fixed before any
  lexer/parser/runtime code lands.
- **MA-13b — `axiom-lexer`.** `axiom.tokens`: the arithmetic/comparison
  operator set including the `~=` not-equal and `**`/`^` power spellings
  (§4), `:`/`::`/`has` as their own tokens (not reused from any prior
  frontend's punctuation, since no prior symbolic-family grammar here has
  needed them), string/integer/float literals, and reserved words
  (`if`/`then`/`else`).
- **MA-13c — `axiom-parser`.** `axiom.grammar`: the ordinary infix
  expression cascade (§4's precedence table); the declaration (`name : T`),
  coercion (`e :: T`), and category-query (`D has C`) productions — the
  genuinely new grammar work this spec's own §3 scopes; `:=` immediate
  assignment and `==` function definition/held-body forms; `if`-`then`-
  `else`; the parenthesised `;`-separated block. Should ship with an
  explicit `MAX_RULE_DEPTH`, measured the same way every prior
  `*-parser`'s own (twice-corrected, per their `CHANGELOG.md`s)
  measure-don't-assume methodology requires, against this grammar's own
  native-stack floor.
- **MA-13d — `axiom-runtime` + `axiom-repl` + the `axiom` binary.** The
  fixed `AxiomDomain`/`AxiomCategory` table and the `has`/`::`-consulting
  dispatcher (§2/§3) sitting over the reused, unchanged `symbolic-vm`
  arithmetic engine (§5); the §4 in-scope surface; the numbered-prompt
  `axiom-repl` (§5).
- **MA-13e — `axiom-to-semantic-ir`**, built alongside per `HML01` §2 /
  every prior precedent (§5), with the `:`/`::`/`has` lowering question
  left open for that item to resolve.
- **Next**: Wave 7's second and final language, Julia (subset), once
  Axiom's kickoff has landed. Axiom's own deferred surfaces — user-defined
  categories/domains/packages (Axiom's own actual "reason for existing"),
  `Record`/`Union`/`Any`, piecewise function definitions, macros,
  comprehensions/loops/streams — are each their own fresh design pass and
  future item, per HML00 §7's wave discipline, not a rubber stamp on this
  spec's own scoping.

## §7 References

Internal: [`HML00`](HML00-historical-math-languages-roadmap.md) (§5 survey —
the "strongly typed CAS (category/type system) — the hardest; later." line
this spec resolves; §7 Wave 7, whose first language this is),
[`HML01`](HML01-math-to-semantic-ir.md) (the `-to-semantic-ir` built-alongside
convention adopted at MA-13e), [`MA04`](MA04-wolfram-language.md)/
[`MA07`](MA07-derive-language.md)/[`MA08`](MA08-reduce-language.md)/
[`MA09`](MA09-maple-language.md) (the four prior symbolic-family kickoffs,
each of which found "`symbolic-vm`/`cas-*` reused completely unchanged" — the
finding this spec is the first to partially break, per §2), [`MA11`](MA11-q-language.md)
(Q — the "defer the language's own actual reason for existing, whole, as
real future work" precedent §3 follows for Axiom's producer-side type
system, and the runtime-internal-not-shared-crate precedent for `QFn` §2/§5
follow for `AxiomValue`), [`MA12`](MA12-idl-language.md) (IDL — the
"cite a well-known continuation when the canonical host doesn't resolve
directly, stated honestly" precedent §1 follows, and the "open SIR lowering
question left to the later item" precedent §5 follows), `symbolic-ir`,
`symbolic-vm`, `cas-simplify`, `cas-pattern-matching`.

External, verified directly (not assumed from the family's own or HML00's
one-line description): Jenks, R. D. & Sutor, R. S. (with Bronstein,
Burge, Daly, Davenport, Dewar, Gianni, Grabmeier, Morrison, Sit, Steinbach,
Trager, Watt, Williamson, and others), *"AXIOM — The Scientific Computation
System"* (Springer-Verlag, 1992; released as open text in 2002 under the
Modified BSD license) — consulted via its FriCAS-hosted, continuously
regenerated adaptation at `https://fricas.github.io/book.pdf` (per §1's own
finding that this is the same 1992 text, not a fork's reinterpretation),
specifically: Chapter 0 "Introduction to FriCAS" §§0.1–0.10 (the
domain/category/package nutshell, pp. 3–17 — `Name(...): Exports ==
Implementation`, `Name(...): Category == Exports`, `Join`, conditional
exports, symbolic-domain function signatures, "Domains Belong to Categories
by Assertion," the `Boolean`-is-not-a-`Ring` worked counter-example); Chapter
1 "An Overview of FriCAS" §§1.1–1.16 (pp. 21–61 — arithmetic precedence,
`%`/`%%(n)` step-history references, `PositiveInteger`/type basics, the
piecewise Legendre-polynomial example, streams, matrices, pattern-matching
`rule`, `radicalSolve`); Chapter 2 "Using Types and Modes" §§2.1–2.7 (pp.
65–98 — domain constructors, subdomains via membership predicate, the
`Polynomial(String)`-is-invalid worked example, `has`, declarations,
`Record`, `Union` with/without selectors, the `Any` domain, `::` conversion
vs. coercion); Chapter 5 "Introduction to the FriCAS Interactive Language"
§§5.1–5.3 (pp. 123–130 — `:=` immediate vs. `==` delayed assignment,
blocks/piles, `if`-`then`-`else`, the `=`-defaults-to-`Equation` /
`~=` not-equal facts); Chapter 6 "User-Defined Functions, Macros and Rules"
§§6.1–6.9 (pp. 149–158 — macros as textual substitution, function type
declarations, one-line functions, declared-vs-undeclared/polymorphic
functions, overloading). Wikipedia, "Axiom (computer algebra system)" and
"FriCAS" (the Scratchpad I/II → commercial Axiom → NAG → 2001 withdrawal/2002
BSD re-release → 2007 OpenAxiom/FriCAS fork lineage, §1); Grokipedia, "Axiom
(computer algebra system)" (cross-checked against the book directly rather
than relied on alone, per this repo's verification discipline).
