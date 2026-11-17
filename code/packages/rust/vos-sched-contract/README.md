# vos-sched-contract

The **scheduler interface contract** of the verified-OS program (backlog VOS-010,
spec [`code/specs/VOS10-scheduler-contract.md`](../../../specs/VOS10-scheduler-contract.md)).

## What it is

A *contract package* (VOS02 section 2.2): the Rust trait every scheduler
implements, and the Warrant text that says what "implements" means. It contains no
scheduler.

```text
  state M       the ready set over thread ids 0..N (N = 8); thread t has priority t
  init()                  -> State         no thread is ready
  insert(s, t)   -> (Upd,  State)          thread t becomes ready
  remove(s, t)   -> (Upd,  State)          thread t stops being ready
  dispatch(s)    -> (Pick, State)          choose a ready thread AND remove it, or Idle
```

Two tiers of promise:

* **safety** (every profile): `dispatch` returns `Idle` or a ready thread and the
  new set is the old set minus exactly that thread;
* **policy** (fixed priority): it returns the highest-priority ready thread.

Round-robin and EDF schedulers carry the safety tier (and a round-robin, whose pointer `alpha` forgets, the **representation** tier of VOS02 2.7 in place of `canonical`). `vos-sched-any` (the
most-general scheduler) is the safety tier's witness.

## Where it fits

```text
 VOS01/VOS02 (the model)  ->  vos-sched-contract (this)  ->  vos-sched-fp-bitmap   VOS-011
                                                         ->  vos-sched-any, -rr   VOS-012
                                                         ->  the spine (VOS-026)
```

## Layout

| Path | What |
|---|---|
| `src/lib.rs` | `trait Sched`, `Upd`, `Pick`: declarations only, `no_std`, no dependency |
| `warrant/contract.part` | the contract block (models only): M, abstract operations, `inv`, tier relations, `pre_` models |
| `warrant/lemmas.part` | the nine model-level lemmas: **not embedded** in any document, evaluated by `vos-sched-contract-check` (VOS02 R1.4) |
| `warrant/obligations-{safety,policy,canonical,representation,any}.part` | the claims an implementation of that tier must carry, character for character |
| `contract.vos` | ceilings (steps, heap, depth per operation) and the instance, `N 8` |
| `vos.toml` | class K, kind `contract` (read by `vos-shape-check`) |

The `.part` files are copied verbatim into each implementation's generated
document by `vos-link gen-doc` (VOS-033, not built yet; VOS02 R2). Edit them only
as a contract change: every implementation regenerates and re-proves.

## Tests

* `cargo test -p vos-sched-contract` runs `tests/trait_usable.rs`: two plain Rust
  schedulers (a bitmap and a lowest-first scan) implement the trait, and the safety
  and policy statements are checked over every state and id at `N = 8` (the scan
  fails the policy on exactly 247 of 256 states).
* The Warrant text is parsed, type-checked and evaluated by the sibling crate
  `vos-sched-contract-check` (it needs the Warrant tooling, which the shape check's
  SH12 does not allow a class K crate to depend on).
* `cargo run -p vos-shape-check -- .` passes (class K, kind `contract`).

There is no coverage percentage worth quoting: the library has no function, only
two `#[derive]`d enums and a trait, and the compiler checks those. The tests that
matter exercise the declarations (implementability, name agreement with the Warrant
variants) and the meaning of the claims.

## Not here

Implementations (VOS-011, VOS-012), the spine and `Profile` (VOS-026), time-based
scheduling (a later `vos-budget-contract`, VOS10 section 13), and the generated
documents (VOS-033).
