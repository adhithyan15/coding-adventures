# lean-kernel

A Lean 4 compatible proof kernel, written from Lean's documented type theory. This is
**slice 1 of KRN-002: the term layer only**. There is no type checker yet.

## Where it fits

[VER11a](../../../specs/VER11a-lean4-proof-kernel.md) replaces the trivial miter
certificate of the verification stack with proofs checked by a small, independent Lean 4
kernel; [LEAN00](../../../specs/LEAN00-lean-on-the-generic-pipeline.md) puts Lean on the
generic language pipeline. The kernel is the trusted base of every such proof, so it has
no third-party dependencies (only [`bignum-core`](../bignum-core) for `Nat` literals), no
I/O and `#![forbid(unsafe_code)]`. VER11a section 4.2 budgets this part ("Expressions and
levels") at 1,200 non-blank, non-comment lines of Rust; the current source is about 810.

```
 names ──► levels ──► expressions (hash-consed, de Bruijn)  ◄── this crate, slice 1
                         │
                         ▼
          infer / check, whnf / is_def_eq, inductives, literals, export reader  (later slices)
```

## What is here

| Module | Contents |
|--------|----------|
| `name` | `Name`: anonymous, string component, numeric component (`Nat.add`, `_uniq.12`). String and numeric components never compare equal. |
| `level` | `Level`: `zero`, `succ`, `max`, `imax`, `param`. `instantiate`, `normalize`, `is_equiv`, `is_geq`, `levels_equiv`. |
| `expr` | `ExprStore` (the interning table) and `Expr` handles: bvar, fvar, sort, const, app, lam, pi, let, lit (Nat via `BigInteger`, String), proj. `instantiate`, `instantiate_rev`, `abstract_fvars`, `lift_loose_bvars`, `lower_loose_bvars`, `instantiate_level_params`, `has_loose_bvar`. |

### Design points

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
