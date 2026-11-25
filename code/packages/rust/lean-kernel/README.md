# lean-kernel

A Lean 4 compatible proof kernel, written from Lean's documented type theory, and the
**trusted reader of the Proof Interchange Format** ([VER32](../../../specs/VER32-proof-interchange-format.md),
PIF-002). This is **slices 1, 2 and 3a of KRN-002**: the term layer, the core of the type checker
(environment, `infer`/`check`, `whnf`, `is_def_eq` with beta, zeta and delta), and now
**inductive types**: the kernel checks a (single, non-nested) inductive declaration,
derives its recursor, and reduces recursors on constructors (iota), with K-like reduction,
proof irrelevance and eta for functions. That is enough to check the first real proofs:
Boolean truth tables, `Nat` arithmetic by computation, `Eq.symm`/`Eq.trans`, ex falso.

What is **not** implemented yet, and fails closed: mutual and nested inductives, structure
projections and structure eta, unit-like types, quotients, lazy delta hints (slice 3b), and
literals, which are `Unsupported` until the primitive `Nat`/`String` definitions can be
validated (a name check is not enough: an environment can define `Nat` as `False`). Anything
the checker cannot judge is rejected, never accepted.

## Where it fits

[VER11a](../../../specs/VER11a-lean4-proof-kernel.md) replaces the trivial miter
certificate of the verification stack with proofs checked by a small, independent Lean 4
kernel; [LEAN00](../../../specs/LEAN00-lean-on-the-generic-pipeline.md) puts Lean on the
generic language pipeline. The kernel is the trusted base of every such proof, so it has
no third-party dependencies (only [`bignum-core`](../bignum-core) for `Nat` literals), no
I/O and `#![forbid(unsafe_code)]`. VER11a section 4.2 budgets this part ("Expressions and
levels") at 1,200 non-blank, non-comment lines of Rust; the current source is about 1,080.
The type-checking row (1,500) is at about 700, the reduction row (2,200) at about 330
the inductive-types row (1,800) at about 530 and the PIF reader row ("Export reader", 600)
at about 585; `tests/ceiling.rs` enforces all five.

```
 names ──► levels ──► expressions (hash-consed, de Bruijn)  ◄── this crate, slice 1
                         │
                         ▼
   environment ──► infer / check, whnf / is_def_eq (beta, zeta, delta)  ◄── slice 2
                         │
                         ▼
   inductives + derived recursors, iota, K, proof irrelevance, eta  ◄── slice 3a (this one)
                         │
                         ▼
   mutual/nested inductives, structures, quotients, literals, lazy delta, export reader (3b+)
```

## What is here

| Module | Contents |
|--------|----------|
| `name` | `Name`: anonymous, string component, numeric component (`Nat.add`, `_uniq.12`). String and numeric components never compare equal. |
| `level` | `Level`: `zero`, `succ`, `max`, `imax`, `param`. `instantiate`, `normalize`, `is_equiv`, `is_geq`, `levels_equiv`. |
| `expr` | `ExprStore` (the interning table) and `Expr` handles: bvar, fvar, sort, const, app, lam, pi, let, lit (Nat via `BigInteger`, String), proj. `instantiate`, `instantiate_rev`, `abstract_fvars`, `lift_loose_bvars`, `lower_loose_bvars`, `instantiate_level_params`, `has_loose_bvar`. |
| `error` | `KernelError`: every way to be rejected, including `Unsupported` and `ResourceLimit`. |
| `lctx` | `LocalContext`: free variable to type and optional let-value (append-only, ids never reused). |
| `env` | `Environment` (owns the `ExprStore`) and `Declaration` (axiom, definition, theorem, opaque). `add_declaration` is the only way in and leaves the environment unchanged on error. |
| `checker` | `TypeChecker` state, `Limits`, caches. |
| `infer` | `infer`, `check`, `ensure_sort`, `ensure_pi`: the typing rules, written out beside the code. |
| `inductive` | `Environment::add_inductive(InductiveDecl)`: checks one non-mutual, non-nested inductive type (names, type telescope, shared parameters, constructor results, universe constraint, strict positivity), derives `T.rec` with its computation rules and large-elimination decision, and adds type, constructors and recursor atomically. `add_declaration` refuses `Inductive`/`Constructor`/`Recursor` kinds, so a recursor can never be forged. |
| `pif` | The trusted PIF reader: `pif::read(env, bytes, &GateInputs)` (see below). |
| `whnf` | `whnf`, `whnf_core`: beta, zeta, delta (definitions only), iota (recursor on a constructor, exact arity) and K-like reduction for `Eq`-style recursors. |
| `defeq` | `is_def_eq`: structural comparison, shared fresh binder variables, congruence, proof irrelevance, whnf retry, eta for functions. |

### Design points

* **Fail closed and bounded.** Every operation returns `Result<_, KernelError>`; call depth
  (200), term depth (1000), total steps (2M) and new expressions (4M, enforced inside each
  store traversal too) are explicit `Limits`, and every universe level in a term is at
  most 128 deep and 512 nodes as a tree; all report `ResourceLimit`. A handle from another
  store is `ForeignExpr`, not a panic. Verdict caches make an exponential delta chain linear.
* **Delta unfolds definitions only.** Theorem and opaque bodies are checked, then hidden;
  axioms have no value.
* **Recursors are derived, never supplied.** Large elimination follows Lean: into any `Sort`
  unless the type may be a `Prop` with several constructors or a non-propositional field
  that is not an index (`Or`, `Exists`: `Prop` only, because proof irrelevance would
  otherwise equate data). A `Prop` with one field-less constructor is K-like (`Eq`).
* **Optional rules never error.** Proof irrelevance and eta need a type; if a term cannot be
  typed they simply do not apply. They only ever add "yes" answers, and each is guarded by
  a negative test.
* **Known limit:** a Pi telescope deeper than about 128 binders exceeds the universe-level
  depth limit, so an inductive with more than about 120 constructors is `ResourceLimit`.

* **Hash-consing.** Building the same structure twice returns the same `Expr`; equality is
  a `u32` comparison. Each node caches its structural hash, its loose bound-variable range,
  `has_fvar` and `has_level_param`; operations skip any subterm those facts prove
  unaffected.
* **Binder names and binder info are metadata.** They live in `Meta<T>`, whose equality is
  always true and whose hash is empty, so `fun x => x` and `fun y => y` are one node.
* **Level equivalence is by normal form** (`max` flattened, sorted, deduplicated, numerals
  dropped when dominated, `succ` pushed through `max`, `imax` simplified when its right
  side is known zero or non-zero). `is_equiv` and `is_geq` are sound and deliberately not
  complete, like Lean's kernel.
* A handle used with a store that did not create it panics, because a silent mix-up would
  alias unrelated terms.

### Known limits and TODOs

* Bound-variable indices are capped at `MAX_BVAR_INDEX` (`u32::MAX / 2`); `try_bvar`
  returns an error (for the export reader to treat as malformed input) and `bvar` panics.
  Index arithmetic is checked and panics on overflow rather than wrapping.
* TODO: traversals are recursive, so very deep terms (long `App` spines, big list literals
  in Mathlib) can overflow the stack. A later slice moves them to an explicit work stack.
* Free-variable ids are issued only by the store (`fresh_fvar_id`, or `issue_fvar_id_at`
  for importers), and `fvar` panics on an id the store never issued. Level and
  expression instantiation return `ArityMismatch` rather than silently truncating.
  Level offsets are capped at `MAX_LEVEL_OFFSET` (2^16, `try_offset` / `try_of_nat`
  return errors), node and store counters are checked, and `Level` and `Name` drop
  iteratively so deep chains cannot overflow the stack.
* TODO: `Level` and `Name` equality, ordering and hashing are still recursive over deep
  structure; and `normalize`, `instantiate` and `geq_core` on levels are not memoised, so
  heavily shared levels can blow up exponentially. Memoise them in a later slice.
* TODO: level normalisation does not distribute `imax` over `max` the way Lean's kernel
  does. It is sound but may reject valid declarations; the L1-a corpus run against the
  official kernel will tell.

## Usage

```rust
use lean_kernel::{Binder, BinderInfo, ExprStore, Level, Name};

let mut st = ExprStore::new();
let ty = st.sort(&Level::of_nat(1));
let x = Binder { name: Name::from_dotted("x"), info: BinderInfo::Default };

// fun (x : Type) => x   and   fun (y : Type) => y   are the same node
let b0 = st.bvar(0);
let id_x = st.lam(x, ty, b0);
let y = Binder { name: Name::from_dotted("y"), info: BinderInfo::Implicit };
assert_eq!(id_x, st.lam(y, ty, b0));

// beta: (fun x => x) c  ==>  instantiate the body with c
let c = st.constant(&Name::from_dotted("c"), &[]);
assert_eq!(st.instantiate1(b0, c), c);

// max u v and max v u are the same universe
let (u, v) = (Level::param(&Name::from_dotted("u")), Level::param(&Name::from_dotted("v")));
assert!(Level::max(&u, &v).is_equiv(&Level::max(&v, &u)));
```

Declaring an inductive type (`tests/common/mod.rs` has a small term-building DSL):

```rust,ignore
// inductive Bool : Type | false : Bool | true : Bool
env.add_inductive(InductiveDecl {
    level_params: vec![],
    num_params: 0,
    types: vec![InductiveTypeDecl {
        name: Name::from_dotted("Bool"),
        ty: type_1,
        ctors: vec![
            ConstructorDecl { name: Name::from_dotted("Bool.false"), ty: bool_c },
            ConstructorDecl { name: Name::from_dotted("Bool.true"), ty: bool_c },
        ],
    }],
})?;
// env now has Bool, Bool.false, Bool.true and the derived Bool.rec.{u}
```

Checking a proof (a sketch of what the tests do):

```rust
use lean_kernel::{Declaration, Environment, Level, Name};

let mut env = Environment::new();
let prop = env.store.sort(&Level::zero());
let name = |s| Name::from_dotted(s);
env.add_declaration(Declaration::axiom(name("False"), vec![], prop)).unwrap();
let fls = env.store.constant(&name("False"), &[]);
// theorem bad : False := False   is rejected: `False : Prop`, not `False`
let bad = Declaration::theorem(name("bad"), vec![], fls, fls);
assert!(env.add_declaration(bad).is_err());
```

## The PIF reader (`pif`, VER32 PIF-002)

A Proof Interchange Format file is text; the kernel understands declarations. The reader
is the one place that turns the first into the second, so it is part of the trusted base:
a bug that made the kernel check something other than what the file says would be a
soundness bug. It is one file (`src/pif.rs`, at most 600 non-blank, non-comment lines, the
"Export reader" row of `tests/ceiling.rs`), has no I/O, no recursion on input-controlled
depth, no new dependency, and **fails closed**.

```rust
use lean_kernel::pif::{read, GateInputs};
use lean_kernel::Environment;

let bytes = std::fs::read("claim.pif")?;
// the default GateInputs are the strictest: a claim file, no imports, no axioms
let report = read(Environment::new(), &bytes, &GateInputs::default())?;
for d in &report.decls {
    println!("{:?} {} ", d.kind, d.name);
}
let env = report.env; // only returned when the WHOLE file was accepted
```

* **All or nothing.** `read` takes the `Environment` by value and returns it inside the
  `ReadReport` only on success; on any error the half-built environment is dropped.
* **The report** lists every accepted declaration in order (kind, name, type handle; an
  inductive block is the type, its constructors, then the derived `T.rec`) and every
  accepted goal. The kind is read back **from the environment**, not from the record, so a
  `theorem` record that became an axiom would show. PIF-007's differential compares this.
* **What the gate supplies** (`GateInputs`): the expected import chain (R-IMP-1), the
  allowed features (R-VER-2), withdrawn logic versions, the file class (`Library`,
  `Reviewed`, `Claim`), the axiom registry predicate (R-AX-1), the constants occurring in
  the goals (R-GOL-4), claim ids already used (R-GOL-2) and the limits (section 7).
* **The accepted language is the grammar's** (`code/grammars/pif/pif.tokens`,
  `pif.grammar`): ASCII only, one space between fields, byte-exact brackets, no bytes after
  `end <count>` and its LF.

### Rules implemented (VER32 5.3)

R-LEX-1 to 4 (for `u32`/`u64` numerals), R-VER-1 to 3, R-IMP-1, R-IDX-1 to 3, R-NAM-1 to 3,
R-CNT-1 and 2, R-FEA-1, R-AX-1, R-DER-1, R-GOL-1 to 4, R-DEC-1 (the kernel's
`AlreadyDeclared`, forwarded) and R-LIM-1 for every limit that applies. Each rule is cited
at its code with what goes wrong if it is missing, and has a negative test asserting the
exact error kind (`tests/pif_*.rs`).

### Deferred, and failing closed

| Rule or record | Why | Behaviour now |
|---|---|---|
| R-PRM-1 to 4 and every `prim` record | needs KRN-002 slice 4 (primitive validation) | `Unsupported` |
| `enat`, `estr` (feature `lits`), R-LEX-4 for `nat` digits | needs slice 4 | `FeatureNotDeclared` (no feature can be declared) |
| `quot` (feature `quotient`) | needs slice 4 | `FeatureNotDeclared` |
| mutual and nested blocks (features `mutual`, `nested`) | needs slice 3b | `FeatureNotDeclared` |
| `eproj` | structure projections, slice 3b | `Unsupported` |
| any declared feature | the kernel implements none yet | `Unsupported` (after R-VER-2/3) |
| R-IMP-2 (`DigestMismatch`) | SHA-256 and the resolver are outside the 600 lines (VER32 5.1) | the resolver's job |
| the binding-CTE digest of an axiom statement (A2) | SHA-256, outside the reader | the gate supplies the predicate |

Tests: `pif_worked_example` (the VER32 4.6 example, whose commutativity theorem the kernel
checks end to end), `pif_lexical`, `pif_structure`, `pif_decls`, `pif_claims` and
`pif_hostile` (every section 7 limit, 2^n DAGs, every truncation, thousands of
deterministic mutations, all on a large-stack thread so a pass means an error was
returned).

## Testing

```bash
cargo test -p lean-kernel
```

Level normalisation is tested semantically: levels are evaluated over a grid of parameter
assignments to check that `normalize` never changes the value, `is_equiv` never equates
different functions, and `is_geq` never claims a false inequality. The de Bruijn
operations are checked against deliberately naive references on randomly generated terms,
plus hand-built cases for variable capture, off-by-one shifting and binder names leaking
into equality.
