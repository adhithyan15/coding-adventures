# VER04 — Warrant v0: syntax, values, claims and bindings

**Status:** Specification, not an implemented compiler. VER-P005 defines the
contract for the Rust pilot. P006 supplies grammar/corpus/parser, P007 supplies
typing/bindings, and P008 supplies code generation. No parser, generated test,
solver result, source proof or release approval is delivered by this document.

**Parents:** [VER00 §4, §7.5–7.6](VER00-verified-shipping-program.md),
[VER26 §3–4](VER26-ct-compare-pilot.md).
**Foundation:** [TF01](TF01-bootstrap-harness.md),
[CT01](CT01-constant-time-compare.md).

## 1. Purpose and profile

A warrant justifies a claim. Here the justification is a typed formula with a
defined meaning, together with examples, input contracts and bindings to actual
public operations. English explains the formula; it does not supply executable
step definitions. Model functions describe values independently of a host lane.

The v0 **pure profile** covers CT01: byte equality, fixed-width equality,
selection and U64 equality. It supports examples, first-order claims, hostile
inputs, bounds, assumptions and secrecy declarations. Integration scenarios,
stateful models, sets/maps, floating point, strings, measured budgets and
capabilities need later profiles. Their syntax is rejected, never ignored.
This narrows the illustrative bitset sketch in VER00 §4.3 for the first parser;
it does not remove that program's later requirements.

One document supplies several evidence targets. An E1 execution of a claim
instance is not E3 property coverage or E5 proof. The report must distinguish
targets that exist from targets whose implementations are not live.

## 2. Source and identity

Sources are UTF-8 without BOM or NUL. LF is canonical; CRLF is normalized by
the source reader before lexing, with original-byte offsets retained. This is
a source-language rule, not a relaxation of TF01's LF-only TFV transport.
Keywords and identifiers are case sensitive. Identifiers match
`[A-Za-z_][A-Za-z0-9_]*`; reserved words cannot be identifiers. Dotted names
join identifiers without whitespace. `--` starts an end-of-line comment;
write subtraction of a negative as `a - -b`.

Reserved words are `warrant model error op example claim assume platform
environment none labels public secret input hostile rejects with ensures
bound fresh if then else match forall exists not and or true false`. Built-in
type, constructor and function names in §3–4 cannot be redeclared or shadowed;
they need not be lexical keywords. Metric/ensures names are contextual NAME
tokens constrained by their contract rules, not arbitrary extension points.

Indentation follows the existing lexer indentation mode: NEWLINE, INDENT and
DEDENT are synthetic tokens; parentheses and brackets suppress layout inside
them. Leading tabs and inconsistent dedents are errors. Two spaces are the
recommended indentation, not a separate semantic requirement. A newline ends
a declaration or single-expression block unless brackets remain open.

The first declaration is `warrant module_name`; its declarations form one
indented suite. Type and callable names share a unique module symbol namespace. Example and
claim names share a separate unique test-name namespace. Labels/bounds reference
an operation rather than introduce a symbol; each operation may have one of
each, and each operation/parameter pair has at most one input declaration.
Assume sections occur once per kind. Parameter/binder names are unique in their
scope and cannot shadow an outer binder. Resolve forward references, then reject cycles among model definitions
and type aliases. Neither declaration order nor file-system search resolves an
ambiguous name. Imports, includes and host paths do not exist in this profile.

A source span has a repository-relative path, half-open original UTF-8 byte
offsets, and one-based line/byte-column coordinates. Stable evidence identity
is `(module, block kind, declaration name)` paired with the exact source
revision/content identity. Line movement does not rename a claim; editing a
formula invalidates evidence for the old source. Generated helper identities
append documented role/case names, not registration-order indices.

Admission limits are 1 MiB of source bytes, 100,000 emitted tokens, 64 nested
expression/type nodes on any path, 4,096 declarations, 128 bytes per identifier
and 4,096 decimal digits per integer literal. Check before unbounded allocation
or recursion. An exceeded limit is an error, never truncated successful input.
These limits constrain source representation, not mathematical Int values.

All subsequent evaluation/checking/generation phases also need recorded work,
memory and stack budgets supplied by a versioned tool profile, not by the
untrusted document. V0's reference profile permits at most 1,000,000 charged
work units, 64 MiB of owned intermediate/output data, 64 active model/expression
frames, and 10,000 generated concrete instances per document. Charge traversed
bits/elements and expanded nodes, not merely a top-level function call; reject
an allocation or arithmetic expansion before it exceeds its budget. An acyclic
model can still expand exponentially, and a short Int expression can allocate
huge intermediates. Exhaustion produces incomplete checking (W0602), never a
truncated value, skipped case or proof. Solver/property-engine profiles may
define different explicit budgets in their own specifications; none changes
the formula's mathematical meaning or silently raises this compiler profile.

## 3. Values and types

| Type | Meaning |
|---|---|
| `Bool` | Exactly `true` or `false`; never an integer |
| `Int` | Mathematical signed integers; no overflow |
| `Nat` | The refinement of Int with value at least zero |
| `U8`, `U64` | Unsigned integers modulo 2^8 or 2^64 for arithmetic |
| `Bytes` | A finite sequence of U8 values |
| `Bytes<N>` | Bytes with exactly the static natural width N, including zero |
| `List<T>` | A finite homogeneous sequence |
| `Option<T>` | `None` or `Some(T)` |
| `Result<T,E>` | `Ok(T)` or `Err(E)`, with a declared finite error type E |

`model Name = type` introduces a transparent alias, not a new nominal type.
`error Name = CASE { | CASE }` declares a nonempty finite error type; constructors
are qualified as `Name.CASE`. Error constructor names are distinct within that
type. There is no implicit numeric-to-error or text-to-error conversion.

Unsigned decimal literals have no leading zero except `0`. Their default type
is Int; contextual Nat literals must be nonnegative. Suffixes `u8` and `u64`
select the corresponding type and require an in-range literal; a suffix does
not silently reduce an out-of-range value. `b"00ff"` is Bytes encoded by even,
lowercase hexadecimal digits, including `b""`. A contextual `Bytes<N>` literal
must have that width. Lists use `[e, ...]`; an empty list requires an expected
element type. There are no decimal fractions, NaN, text literals or host truth
coercions in v0. Unary minus on an Int/Nat expression produces Int.

Nat may widen to Int and Bytes<N> may forget its width to become Bytes. No
other implicit numeric conversion or dynamic-to-fixed width conversion exists.
`fixed<N>(bytes)` returns `Option<Bytes<N>>`; `Some` is possible exactly when
the length matches. N is a literal natural or an operation's static Nat
parameter, never a runtime Bytes length. A backend unable to instantiate N
rejects that target; it must not substitute the slice entry point.

Equality is structural and type-directed: sequence order matters, errors are
nominal, Bool differs from Int, and two values must have a common admitted
type. It never compares allocation identity. Freshness is a separate operation
contract (§6), not something value equality can establish.

## 4. Expressions and pure models

`model f(x: T, ...) -> R = expression` defines a pure, total model function.
Definitions may call only model functions and built-ins; they cannot call
bound subject operations. Recursive definitions, mutation, I/O, reflection,
arbitrary host code and dynamic evaluation are absent. Arguments are evaluated
left to right; `if`, `and`, `or` and `=>` evaluate only the required branch.
Each branch remains statically type checked even when unreachable.

Precedence, highest first: call/type application; unary `not`/minus;
`*`; `+`/`-`; comparisons/equality; `and`; `or`; `=>`; quantification.
Arithmetic operators are left associative; implication is right associative.
Comparisons cannot chain: write `0 <= i and i < n`.
Both operands of arithmetic have the same numeric type after Nat-to-Int
widening. Arithmetic on Int/Nat produces Int, including Nat subtraction;
comparison accepts that same widening. U8/U64 arithmetic wraps and returns
the same unsigned type; Int arithmetic is exact. Unsigned values
are ordered by their nonnegative representative. `if p then a else b` requires
Bool p and a common branch type. Boolean operations accept Bool only.

The core built-ins are:

| Form | Total semantics |
|---|---|
| `len(sequence)` | Nat number of elements |
| `at(sequence, i)` | Option of the element; None outside a Nat index range |
| `concat(a, b)` | Sequence concatenation of the same element type |
| `fixed<N>(b)` | Checked width conversion as in §3 |
| `div(a, b)` | Option<Int>; None if b=0, otherwise floor(a/b) |
| `Some`, `None`, `Ok`, `Err` | Typed constructors; context fixes omitted type arguments |

No unchecked indexing, `/`, implicit unwrap or implicit fixed-width narrowing
exists. Models needing an Option value use
`match e with None -> a | Some(x) -> b`; Result matches use
`Ok(x) -> a | Err(e) -> b`. Matches are exhaustive with one arm per constructor,
new arm binders are local, and all arm results have a common type. Constructor
applications and matches are not a mechanism for arbitrary host calls.

`forall x: T | p. q` and `exists x: T | p. q` have first-order meaning over
the entire domain of T with predicate p (omitted p means true). Forall means
every admitted assignment satisfies q; exists requires one. Multiple binders
are comma separated; a refinement may refer only to current/earlier binders.
Quantifier bodies/refinements are Bool and quantifier-bound variables have no
identity outside the expression. Finite lists are always finite values, but
their type's domain is not thereby finite. A sampling budget never changes a
quantifier's meaning. Unbounded quantifiers cannot be interpreted by silently
enumerating a finite prefix.

## 5. Declaration skeleton and a small example

The following is normative surface notation, not an implementation grammar.
P006 must express it with the repository's `.tokens`/`.grammar` tools, pin
parse trees and prove drift; a handwritten parser inside a backend is forbidden.
One expression block contains one Bool expression, which may contain calls,
matches and quantifiers. Long expressions continue inside parentheses.

```text
warrant ct_compare
  error SelectError = CT_SELECT_LENGTH_MISMATCH
  model same(a: Bytes, b: Bytes) -> Bool = a == b
  op eq(a: Bytes, b: Bytes) -> Bool
  op eq_fixed<N: Nat>(a: Bytes<N>, b: Bytes<N>) -> Bool
  op select_bytes(a: Bytes, b: Bytes, choice: Bool) -> Result<Bytes, SelectError> fresh
  op eq_u64(a: U64, b: U64) -> Bool

  assume platform:
    none
  assume environment:
    none

  labels eq:
    a.contents secret
    a.length public
    b.contents secret
    b.length public
  labels select_bytes:
    a.contents secret
    a.length public
    b.contents secret
    b.length public
    choice secret

  example empty_equal:
    eq(b"", b"") == true
  example select_left:
    select_bytes(b"00", b"ff", true) == Ok(b"00")
  example select_length_error:
    select_bytes(b"", b"00", false) == Err(SelectError.CT_SELECT_LENGTH_MISMATCH)
  example high_bit:
    eq_u64(9223372036854775808u64, 0u64) == false
  example fixed_empty:
    eq_fixed<0>(b"", b"") == true

  claim equal_agrees:
    forall a: Bytes, b: Bytes. eq(a, b) == same(a, b)

  input select_bytes.a:
    hostile
    rejects len(a) != len(b) with SelectError.CT_SELECT_LENGTH_MISMATCH
    ensures no_panic, terminates

  bound eq:
    steps <= 64 * len(a) + 512
    heap <= 2 * len(a)
    depth <= 32
```

The bound coefficients illustrate syntax, not a measured or proved CT01 bound.
This excerpt is not a complete pilot file: all four operations' parameters
need input contracts, their required bounds, and the remaining secret labels.
In particular `eq_fixed` contents and both `eq_u64` values must be secret.
The missing contracts prevent treating the excerpt as an accepted pilot spec.

Declarations and required forms are:

| Form | Contents / scope |
|---|---|
| `warrant name` | Exactly once, followed by the declaration suite |
| `model name = type` | Alias; model-function form is defined in §4 |
| `error name = CASE ...` | Finite error type from §3 |
| `op name<N: Nat, ...>(parameters) -> type [fresh]` | Public subject signature; static parameters optional |
| `example name:` | One Bool expression with no free variables |
| `claim name:` | One Bool expression, with all logical variables explicitly bound |
| `assume platform:` | Dotted catalogue identifiers or the sole item `none` |
| `assume environment:` | `name: predicate` rows or the sole item `none` |
| `labels operation:` | `parameter[.contents\|.length] public\|secret` rows |
| `input operation.parameter:` | `hostile`, zero/more `rejects predicate with Error.CASE`, then `ensures` flags |
| `bound operation:` | One/more distinct `steps\|heap\|depth <= expression` rows |

Parameters are `name: type [| Bool refinement]`, comma separated. Refinements
on operation parameters are input admission predicates, not permission to
assume hostile data satisfies them: the input contract must specify rejection
outside the domain. Static Nat parameters are type-level compile-time values.
Subject calls may specify static arguments with `<N, ...>` and runtime values
with `(arguments)`; arity/types must match. No overload resolution, defaults,
implicit model dispatch or variadic calls exist.

The grammar-facing declaration shape is below. `expr` follows §4's precedence;
NAME/QNAME denote identifiers/dotted names, NAT a nonnegative decimal token,
and layout/EOF tokens come from the source reader. Semantic rules above impose
the named limits, namespaces and required contract fields. `list(rule)` means
one or more comma-separated instances; it is notation here, not a new grammar-
tools operator. Blank/comment-only lines do not create empty declarations.

```text
file       = "warrant" NAME NEWLINE INDENT { declaration } DEDENT EOF ;
declaration = alias | model_fn | error_type | operation | example | claim
            | assumptions | labels | input | bound ;
alias      = "model" NAME "=" type NEWLINE ;
model_fn   = "model" NAME "(" [ list(parameter) ] ")" "->" type "=" expr NEWLINE ;
error_type = "error" NAME "=" NAME { "|" NAME } NEWLINE ;
operation  = "op" NAME [ "<" list(static_parameter) ">" ]
             "(" [ list(parameter) ] ")" "->" type [ "fresh" ] NEWLINE ;
static_parameter = NAME ":" "Nat" ;
parameter  = NAME ":" type [ "|" expr ] ;
example    = "example" NAME ":" NEWLINE INDENT expr NEWLINE DEDENT ;
claim      = "claim" NAME ":" NEWLINE INDENT expr NEWLINE DEDENT ;
assumptions = "assume" ("platform" | "environment") ":" NEWLINE
              INDENT assumption_rows DEDENT ;
labels     = "labels" NAME ":" NEWLINE INDENT label_rows DEDENT ;
input      = "input" QNAME ":" NEWLINE INDENT "hostile" NEWLINE
             { "rejects" expr "with" QNAME NEWLINE }
             "ensures" list(NAME) NEWLINE DEDENT ;
bound      = "bound" NAME ":" NEWLINE INDENT bound_rows DEDENT ;
type       = primitive | NAME | "Bytes" [ "<" width ">" ]
           | ("List" | "Option") "<" type ">"
           | "Result" "<" type "," NAME ">" ;
width      = NAT | NAME ;
```

`primitive` is Bool/Int/Nat/U8/U64. An assumption suite is either the sole row
`none`, or nonempty platform identifier rows / environment `name: expr` rows.
A labels suite is nonempty path/classification rows; bound rows are nonempty
metric/`<=`/expression rows as in the table. These row lists terminate at DEDENT.
Expression atoms are literals, in-scope names, typed constructors, list
literals and parenthesized expressions; calls/static applications, conditionals,
exhaustive matches and quantifiers have exactly the forms in §3–4. A quantifier
separator `.` must be followed by whitespace/newline, distinguishing it from a
joined dotted name. No implicit semicolon, juxtaposition call or indentation-
free alternative is admitted by these descriptions.

## 6. Bind real public operations

Bindings are separate per-language artifacts. They identify the package,
public export, static instantiations, argument constructors, result observer,
error normalization, optional resource observers and their own exact revision.
Warrant sources contain no Rust paths, Python imports, shell snippets or
interpolated templates. Bindings name approved backend constructs; a future
binding reader must not evaluate arbitrary configuration as code.

CT01's Rust mapping must call the actual `ct_eq`, `ct_eq_fixed::<N>`,
`ct_select_bytes` and `ct_eq_u64` exports. The model symbol `select_bytes`
returns Result because CT01 has a semantic length error; its Rust export
returns Vec and panics on unequal lengths. Test-only normalization catches
that boundary only for the declared length-error case, suppresses native
panic text, and preserves unexpected panics as test failures. A panic on a
valid call cannot be turned into the expected semantic error.

`fresh` qualifies successful sequence results (directly or inside Result),
and requires a generated ownership check as well as value checks. For a
nonempty result, call the real export with distinguishable buffers, save input
values, mutate the returned result and check the passed inputs stay unchanged.
Include mutations that alias either input and show that each fails. For empty
results, no writable positions exist; equal empty-buffer addresses alone are
not an alias defect. Logical Bytes equality never substitutes for this check.
P007 defines a closed binding data schema; P008 supplies the actual generated
tests. Until those exist, freshness is a specification obligation, not evidence.

Every bound model/host type needs construction/observation round trips.
Additionally require distinct generated model values, an observer that
distinguishes them, and each built-in corrupted-subject control must make at
least one example/claim instance fail. A constant generator paired with a constant
observer is not vindicated by a successful round trip. Bindings cannot opt out
of negative controls or supply their own expected result for their own control.

Independently discover the shipped artifact's exports. Every real export must
be bound and each binding target must be a real export; instantiated generics
retain their export identity. A self-declared four-name manifest is useful
pilot input, not independent API-coverage evidence. Without the analyzer,
coverage is `not-live` and cannot authorize shipping. The binding cannot point
the tests at a convenient private helper/stub and claim coverage of the export.

## 7. Inputs, bounds, secrecy and assumptions

All public-operation runtime parameters are hostile by default. Every hostile
parameter needs an input declaration; every operation accepting one needs
steps, heap and depth bounds. An unsupported or omitted required contract
blocks complete-pilot acceptance; it is not an implicit exemption. The pure
profile admits no `trusted` downgrade or declassification approval syntax.
Publishing/approval remains blocked by VER00 §12.1.

Input rejection predicates see the operation's argument values, not `result`.
They are Bool; the specified error must belong to the operation's Result error
type. If two rejection predicates overlap they must name the same error, or
the contract is ambiguous and rejected. Outside the operation's admission
domain, rejection must be specified; on admitted inputs the contract cannot
demand an error. `ensures` admits only `no_panic` and `terminates` in v0.
No unknown flag is silently accepted. A declared, correctly normalized semantic
error is not an unexpected panic; its native implementation remains subject to
the binding checks in §6.

Bounds use mathematical integer expressions over parameters and `len`, never
secret-dependent host calls. They must be nonnegative on the admitted domain.
Steps count the selected execution model's steps, heap counts peak live bytes,
and depth counts maximum call-stack frames. Each observer names its model,
instrumentation and revision. WIR bounds and wall-clock time are distinct.
`CT01.expected_work` is logical input shape: it cannot serve as an observed
steps counter or a constant-time proof. Counts computed solely from input
length cannot attest that the production code visited those positions.
An observer must instrument the actual bound export; a constant observer or
input-shape calculator fails its own negative controls. Include subjects whose
result is unchanged while work/heap/depth changes, and require the relevant
observer to distinguish them. Ordinary value-result mutation checks alone do
not validate a resource observer. Without that independent control evidence,
the bound target stays blocked/not-live.

Secrecy is independent of hostility. Bytes have separate contents and public
shape projections; labeling contents secret does not label length secret.
Scalar labels apply to the scalar value. A labels row asserts classification;
it cannot override stronger secrecy found by VER00's analyzer/registry.
Flow joins use `public < secret`; a public assertion contradicted by inferred
secrecy is an error, not declassification. Annotation alone cannot certify
crypto-relevance or the absence of other secrets. For CT01, compared contents,
U64 operands and choice are secret; lengths and fixed widths are public.
Any pilot constant-time target with no secret input fails vacuity checks.
Proving equal counters is insufficient: VER20 must compare branch decisions,
addresses/indices and step counts under self-composition.

Both assume sections are mandatory, even when their sole item is `none`.
Platform entries must resolve to the shared versioned VER13 catalogue and its
lane conformance evidence; packages cannot invent private platform axioms.
Environment entries are formal predicates with a tracked dependency set;
they cannot constrain hostile input, directly or transitively through a model
function/alias. Prose comments never enter a proof. Missing catalogue or taint
analysis prevents the corresponding guarantee from being labelled proved;
writing `none` cannot erase an assumption the implementation actually relies on.

## 8. Static checks and evidence production

Run lexical/layout checks, parse, resolve names, reject cycles, type check,
check contracts/labels, validate bindings, then perform solver checks.
Sort same-phase diagnostics by original source offset and diagnostic ID;
independent errors may be collected, but no output is promoted from an invalid
document. Diagnostics contain class/ID/span and bounded explanation. They must
not contain secret input, a private seed, a full reproducer or panic text.
Compiler diagnostics are a separate vocabulary from TF01's six failure IDs.


Diagnostic IDs are stable within v0; explanations may improve without changing
classification. P006/P007 cases pin the applicable phase, ID and offending span.
When an earlier phase cannot construct a later construct, report the earlier
error; a wrong result is not excused by a coincidental different rejection.

| ID | Phase / rejection |
|---|---|
| W0001 | Source admission limit, forbidden encoding or malformed layout |
| W0101 | Invalid token/literal or unsupported syntax/block |
| W0102 | Structurally incomplete declaration/expression |
| W0201 | Duplicate symbol/test identity/contract declaration |
| W0202 | Unbound or ambiguous name |
| W0203 | Cyclic model/alias dependency or prohibited shadowing |
| W0301 | Type, arity, width, constructor or match-exhaustiveness error |
| W0401 | Missing/invalid input, bound, label or assumption contract |
| W0402 | Hostile-input assumption, secrecy downgrade or ambiguous rejection |
| W0501 | Invalid/missing export binding, constructor/observer or static instantiation |
| W0502 | Failed binding negative control or fresh-output requirement |
| W0601 | Inconsistent assumptions/claims, vacuity or trivializing assumption |
| W0602 | Incomplete mandatory solver/analyzer/observer evidence; no checked status |


Before a spec is reported **checked**, its assumptions and claims must be
jointly consistent, claim antecedents/refinements reachable, examples
consistent with claims, and assumptions neither inconsistent nor trivially
discharging claims. Subject operations are typed uninterpreted functions in
these spec-level checks; model definitions retain their actual semantics.
Do not add the very goal being proved as an assumption of its own proof.
Reachability and consistency have different queries/evidence from goal proofs.

For consistency/reachability queries, `Unsat` rejects an inconsistent spec or
unreachable antecedent. A satisfying
witness establishes consistency/reachability, not a universal subject proof.
`Unknown`, timeout, unsupported theory or unavailable solver leaves checking
**incomplete** and blocks checked-spec/pilot acceptance. A parse/type-check
success can still be inspected, but it is never renamed checked/proved.
An UNSAT proof claim needs a checked certificate under VER10/11; a solver
boolean, unchecked theory lemma or abandoned search is not proof evidence.

| Block | Required generated work, with honest limits |
|---|---|
| example | Plain named L0 test calling real bound exports and checking Bool outcome |
| claim | L0 concrete instances plus E3 target and E5 obligation; samples do not discharge the formula |
| input | L0 valid/rejection boundaries and unexpected-panic checks, E4 target and termination/rejection obligations |
| bound | L0 checks using actual declared observers and E5 obligations; absent observers remain blocked/not-live |
| labels | Static/taint checks and VER20 self-composition obligations; labels alone prove nothing |

Generated L0 tests use TF01 ordering/sub-seeds and registered diagnostics.
Missing required bindings/observers cannot compile to literal-true assertions
or successful `ok` placeholders: block that target, report `not-live`, and
make complete-pilot acceptance nonzero. If execution reaches an invalid adapter
configuration, use registered `corpus-malformed`; false normalized outcomes use
`corpus-mismatch`, and unexpected subject panics use `panic`.

Termination cannot be inferred from a single returned call. It needs an
admitted bounded evaluator/observer or the later proof; lack of one stays
not-live. New/changed claims, generators, targets and engine versions must run
privately against shipped releases before public enablement (VER26 §10).
Public failure IDs alone do not hide a publicly recomputable seed. P005
authorizes no private-store integration or public randomized campaign.

Emission is deterministic for the same source, binding/tool revisions, target
profile and declared seed. Commit generated source and provenance; regeneration
must compare bytes and reject drift. Unsupported targets must be listed, not
silently dropped. In every report, E1/E3/E4 execution, measured counters,
`proved-over-model`, `bounded(k)` and implementation-linked proof are separate.

## 9. Acceptance inventory and implementation order

The companion [design inventory](fixtures/warrant-v0/README.md) names positive
and negative cases and the phase/diagnostic they must exercise. It is design
data in this PR, not an executed conformance corpus. P006 materializes parsing
cases in JSON plus generated TFV; harnesses read TFV, not JSON. P007 extends
them with typing/binding negatives; P008 proves real-export execution and
mutation failures. Each checker must fail deliberate corruptions of what it
claims to validate. A corpus containing only accepted programs is insufficient.

The implementation sequence is parser/corpus → type checker/binding schema →
Rust generation → property/runtime support → source-linked proof tooling.
Do not bundle a solver, a handwritten parser or a release gate into P005.
Spec review must verify every example is either a complete accepted program or
explicitly identified as an excerpt, and every rejection has a stable class
and source span. Cross-language adoption remains owned by package parity.

## 10. Specification change record

- Initial v0 pure-profile contract after verified bootstrap, CT01 and failure-ID
  boundary deliveries. Chooses significant indentation from the parent sketch.
- Makes Option-returning indexing/division explicit; no implicit partial
  operations. Separates logical Bytes equality from fresh-output ownership.
- Defines hostile-input checks, secrecy projections and fail-closed incomplete
  solver/binding/observer evidence without claiming these tools already exist.
- VER04k §9.1 E21 (WX-D1 S1 security review), a v0 deviation: a reference of
  more than one segment is only `E.CASE` (a declared error and one of its
  cases) or, in v1, a projection from a binder in scope. Any other dotted name
  is W0202 in v0 too. Such v0 documents used to check by the name's first
  segment but evaluated to a forged error value; none is in the repository.

## 11. Grammar artifacts and syntax-corpus staging (P006)

P006 is delivered in bounded slices. **P006a** supplies canonical
`code/grammars/warrant/warrant.tokens` / `warrant.grammar`, compiled Rust grammar
data in `code/packages/rust/warrant-grammar`, and a data-only syntax corpus.
Its public functions return TokenGrammar/ParserGrammar data; they do not accept
untrusted source or claim to be a protected Warrant compiler front end.
**P006b** supplies the guarded source reader/parser, original UTF-8 byte spans,
admission/work budgets and multi-shape parser-stack validation. Rule-call depth
in the shared parser is not Warrant AST depth; a small successful tooling probe
does not discharge this obligation. Typing, bindings and semantic checking
remain P007+. No slice may report another slice's missing evidence as live.

Canonical grammar compilation uses the existing grammar-tools emitter.
Regeneration compares emitted bytes, and tests compare runtime-loaded and
compiled grammar behavior. No handwritten parser is introduced by the artifact
package. Conditionals, matches and quantifiers are compound expression forms;
parenthesize them when used as arithmetic/Boolean operands. A match arm's `|`
belongs to the nearest unparenthesized match; parentheses delimit an inner
match when returning to an outer arm list. These rules make the previously
sketched compound-expression boundary explicit before grammar implementation.

The syntax corpus is `code/specs/fixtures/warrant-syntax-v1/cases.json` with
exact root fields `format` (the string `warrant-syntax-v1`) and `cases` (a list).
Each case has exactly `id`, `feature`, `source`, `accept` and `witnesses`:

- `id`: unique TF01 record identifier, at most 128 ASCII bytes;
- `feature`: a declared syntax-feature group, not a semantic evidence label;
- `source`: UTF-8 Warrant source, at most 4 KiB per case;
- `accept`: an actual JSON Bool, the independently authored syntax verdict;
- `witnesses`: distinct exact subtree witnesses for accepted cases, empty for
  rejected cases. A witness is `rule_name=token values joined by one space`,
  omitting layout/EOF leaves. Match whole entries, never substrings. Curate
  operator/subtree witnesses that distinguish precedence and grouping; a root
  `file` name or acceptance Bool alone cannot establish those properties.

The corpus has at most 128 cases and a 2-MiB serialized JSON limit. Reject
unknown fields, duplicate keys/IDs, invalid strings/Bool kinds, missing positive
or negative baselines, and missing required feature coverage before generation.
The emitter does not derive expected verdicts/witnesses by running the subject
parser. It projects each authored case to TFV fields `source` (text), `accept`
(Bool) and `witnesses` (list of text), preserving source bytes and LF transport.
Runtime tests embed/read TFV rather than parse JSON. A byte-level regeneration
test, malformed-corpus controls and a deliberately wrong-precedence grammar
must demonstrate that the guards can fail; the L0 sabotage run must be nonzero.

This is syntax-level evidence only. Type-invalid but syntactically valid
examples remain parser positives, while later-phase corpus entries belong to
their own checker. Public error IDs/spans, source normalization and the full
hostile-source budgets are not implemented by an artifact-only package's test
adapter. Its bounded test use of the shared parser must not be advertised as
complete source-frontend safety or full VER04 conformance.

## 12. Bounded source-stage delivery

P006b is divided into source admission/normalization (b1), exact bounded
significant-layout origins (b2) and the guarded parser (b3). The additive
[VER04a source contract](VER04a-warrant-source-admission.md) specifies b1's
UTF-8 admission, distinct normalized buffer, original byte/line mapping and
fixed W0001 rejection. It explicitly rejects bare CR and treats only LF/CRLF
as source line endings. A successful admitted buffer is not a parsed or checked
document.

The additive [VER04b layout contract](VER04b-warrant-layout-capture.md) specifies
b2's bounded significant-layout capture and synthetic token origin mapping over
admitted source documents, ensuring synthetic INDENT/DEDENT/NEWLINE tokens possess
exact zero-width origins without fictitious positive-width spans.

The additive [VER04c guarded parser contract](VER04c-guarded-warrant-parser.md)
specifies b3's bounded AST construction over layout token streams, enforcing explicit
node quotas (`max_nodes`) and recursion stack guards (`max_depth` ≤ 64) against
stack exhaustion. The shared CV04a standard-mode partition contract remains unchanged;
its indentation refusal must not be treated as complete layout capture.
