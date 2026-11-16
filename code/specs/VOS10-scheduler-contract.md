# VOS10 -- The Scheduler Contract

**Status:** Draft. The contract package `code/packages/rust/vos-sched-contract`
(the Rust trait, the Warrant contract block, four obligation blocks and
`contract.vos`) and its test companion `vos-sched-contract-check` are committed
with this document, in the commit that follows it. No
scheduler implements it yet: `vos-sched-fp-bitmap` is VOS-011 and
`vos-sched-any` with `vos-sched-rr` are VOS-012. The Warrant text below is the
text of the package, copied by a script, and was type-checked and evaluated
with the repository's own Warrant tooling (section 12). The generated
per-implementation documents, and with them any evidence about compiled Rust,
wait for `vos-link gen-doc` (VOS-033).

**Layer:** Program model, the first real instance of the contract model. It is
what [VOS01](VOS01-component-contract-model.md) section 5 (the toy "ready set")
was written to prepare, in the document layout of
[VOS02](VOS02-interface-contracts-and-profile-linking.md).

**Parent:** [VOS00](VOS00-verified-os-program.md) sections 4.4, 4.8, 7, 8 and 11.

**Builds on:** VOS01 (anatomy, tiers, `-any`, the budget), VOS02 (R1-R9, the
fixed vocabulary, the interim projection form, ghost face), VOS03 (the shape
check), [VOS30](VOS30-proof-route-and-capacity.md) section 8.5 (X1 instance
size), [VER20](VER20-resource-timing-proofs.md) (bounds).

---

## 0. How to read this document

Marks are those of VOS01 section 0: **today** exists on this branch;
**NEW(row)** does not, and the row closes it. Every part of VOS00 section 4.4's
anatomy gets one section; the decisions that were open are collected in section
1 with their reasons, so a reviewer can disagree with a decision instead of
discovering it.

Two terms are used strictly. A **thread id** `t` is a number below `N`; thread
`t` **has priority `t`** and a larger number wins. A **choice** is what
`dispatch` returns: `Idle` or `Run(t)`.

---

## 1. Decisions

| # | Decision | Why (section) |
|---|---|---|
| S1 | The abstract state M is **only the ready set**: `N` Booleans. There is **no `current`** | 2 |
| S2 | Priority is the thread id; the thread table hands out ids in priority order | 2 |
| S3 | Four operations: `init`, `insert`, `remove`, `dispatch`. `dispatch` **removes** the thread it returns | 3 |
| S4 | Excluded on purpose: `pick` without removal, `yield`, `set_priority`, `current`, time, thread creation | 3.3 |
| S5 | `inv(M)` is total (`true`); init is the empty set; closure is claimed anyway | 4 |
| S6 | Safety tier: `dispatch` returns `Idle` or a ready thread and removes exactly that one. Policy tier: it returns the maximum ready thread | 5 |
| S7 | Round-robin and EDF implementations carry the safety tier only; round-robin is `mode=representation` | 5.3 |
| S8 | `insert` and `remove` of an id `>= N` answer `OutOfRange` and change nothing; `dispatch` has no precondition | 6 |
| S9 | One ceiling formula, `16 N + 32` steps, heap 0, depth 2; `init` gets `8 N + 16` | 7 |
| S10 | Instance `N = 8` for X1; the largest claim is 65,536 binder cases (16.4% of the 400,000 budget) | 10 |
| S11 | Sporadic and budget scheduling is a **separate later contract** (`vos-budget-contract`), not a tier of this one | 13 |
| S12 | Obligations use the interim projection vocabulary of VOS02 section 4.4; the canonical block is **field-agnostic** (no `canon_build_alpha`), so `any` is kept out of `canonical` by a rule | 9, 14 |

---

## 2. Abstract state M

**Meaning.** M answers one question: *which threads may run next?* It is the
smallest model every claim can be stated against (VOS01 section 2.1).

```text
  M = Ready(p0, p1, p2, p3, p4, p5, p6, p7)      p_i : Bool     "thread i is ready"
```

Eight Booleans. Equality is structural, so two states that mean the same set are
equal (a queue would not be: two insertion orders of one set would differ and
the contract would pin an order nobody needs). With `N = 8` there are 256
abstract states, so every claim over M can be enumerated (section 10).

**Why priority is the id (S2).** The task is a ready set over `N` thread ids
with *unique static* priorities. Unique means the priorities are a permutation
of the ids, so renaming the ids into priority order loses nothing: the thread
table (VOS-owned, a later leaf) owns the mapping from a kernel object to its id
and assigns ids in priority order at configuration time. The scheduler then
needs no priority table, M needs no array, and the bitmap scheduler of VOS-011
needs no lookup (VOS00 section 11, "fixed-priority bitmap scheduler"). A
priority *table* in the scheduler's state would multiply M by `N!` and push every
claim past the budget. Static priorities are therefore a **property of the
configuration**, stated once here, and a profile with dynamic priorities (priority
inheritance, ceilings) is a different interface (section 3.3).

**Why there is no `current` (S1).** Three reasons, in order of weight.

1. *Ownership (VOS00 rule 2).* "Which thread is running" is state the **spine**
   (and the thread table, for its context) owns. If the scheduler also held it,
   two leaves would hold one fact, and a cross-leaf invariant ("scheduler's
   current equals the thread table's current") would appear in every proof.
2. *The set is enough.* A preemptive fixed-priority scheduler needs only the
   ready threads and the running thread's priority; the spine composes the two
   (section 6.2): to preempt, put the running thread back with `insert` and
   `dispatch`; the answer is the running thread again if it is still the
   highest.
3. *Replaceability.* With `current` in M, every implementation must maintain it
   and every `-any` proof must carry it; without it, `-any` is a bitmap and an
   oracle.

The one thing the spine loses is a cheap "is anything more urgent than the
running thread" test without a state change. It pays two calls instead
(section 3.3, `peek`).

---

## 3. The operations

### 3.1 Shape and the four operations

Every operation is by-value, total, panic-free and bounded (VOS01 section 1.1):
`op(State, In) -> (Out, State)`. The Rust trait (`vos_sched_contract::Sched`):

```rust
#![no_std]
#![forbid(unsafe_code)]
pub enum Upd  { Changed, Unchanged, OutOfRange }   // derives Copy, Clone, Debug, PartialEq, Eq
pub enum Pick { Idle, Run(u8) }                    // likewise

pub trait Sched {
    type State;                                    // owned by one field of the kernel record
    const N: u8;                                   // configuration, printed in labels
    fn init() -> Self::State;
    fn insert(s: Self::State, t: u8) -> (Upd, Self::State);
    fn remove(s: Self::State, t: u8) -> (Upd, Self::State);
    fn dispatch(s: Self::State) -> (Pick, Self::State);
}
```

| Operation | Signature | Meaning | Why it is in the minimal set |
|---|---|---|---|
| `init` | `() -> State` | no thread is ready | every component has one (VOS01 section 1.1) |
| `insert` | `(State, t) -> (Upd, State)` | thread `t` becomes ready | a thread is created, woken, or put back after running |
| `remove` | `(State, t) -> (Upd, State)` | thread `t` stops being ready | a ready thread is suspended or exits; a thread that is *running* is simply never re-inserted, and `remove` of a non-member is a harmless `Unchanged` |
| `dispatch` | `(State) -> (Pick, State)` | choose a ready thread and take it out of the set, or `Idle` | the only way to *get* a thread; fused with removal so that "the chosen thread is running, hence not ready" holds by construction |

`Upd = Changed | Unchanged | OutOfRange`; `Pick = Idle | Run(t)`. Both are
`Copy` enums with fixed-width payloads (VOS01 section 1.3).

### 3.2 Why `dispatch` removes (S3)

VOS01's toy had a non-destructive `pick` and a separate `remove`, and its spine
needed a caller-side claim (`site_dequeue_pre`, VOS01 section 6.3) that the
picked thread is a member before it is removed. Fusing the two:

* removes one call site per kernel entry, hence one `site_` claim and one
  `pre_` obligation (the picked thread cannot fail to be a member);
* makes the spine invariant "the running thread is not ready" a *consequence of
  the contract* (`safe_dispatch` removes exactly the choice), not of a sequence
  of two calls the spine author must get right;
* matches VOS00 section 4.8 line 2, `(pick, K.sched) = sched::op(K.sched, ...)`,
  which already has one call returning the pick and the new state.

The cost is that `dispatch` has **no abstract successor function of M alone** in
the safety tier (any member may be chosen), so M gives the successor as a
function of the *choice*: `after(m, o)` is `m` for `Idle` and `m` without `t` for
`Run(t)`. Every obligation about `dispatch` is therefore about the pair
(choice, successor), and the claims in section 9 are written that way.

### 3.3 What is deliberately excluded (S4)

| Excluded | Reason | Where it goes |
|---|---|---|
| `pick` (peek, no removal) | Needed only to ask "would a dispatch change anything" without changing it. The spine can `insert` the running thread then `dispatch`: if the answer is the running thread, nothing observable changed. One more operation is one more op in every implementation, `-any`, ceiling table and `site_` claim | add only if a profile shows a measured need (VOS-D048) |
| `yield`, `rotate` | With unique priorities a yield *is* `insert` of the running thread followed by `dispatch`. Round-robin rotation is internal to a round-robin `dispatch` (representation state, section 5.3) | none |
| `set_priority` | Priorities are static (S2). Priority inheritance or a ceiling protocol needs a thread to change place in the order, which is a different M (a priority table) and a budget problem | a later interface, `vos-sched-contract` version 2 or a sibling (VOS-D048) |
| `current` / `running` | section 2 | spine |
| time, tick, budget, deadline | section 13 | `vos-budget-contract` |
| thread creation, thread state | The scheduler knows ids, not threads (VOS00 rule 1: no calls between leaves) | thread table |
| `count`, `is_empty` | `dispatch` answers `Idle` for the empty set; no caller needs a count | none |

---

## 4. `inv`, the initial state, and closure

`inv(M)` is `true`: every one of the 256 states is well formed, so there is no
ill-formed state to exclude (a Boolean per thread cannot be corrupted). `init`
returns the empty set. **Closure** says every operation maps an `inv` state to an
`inv` state; it is trivial for a total `inv`, but it is still stated, as the
lemma `closure_at_m` (a **trivial** lemma, evaluated but not counted as evidence), so that a later M with structure cannot change `inv`
without restating it (VOS01 section 2.2).

The implementation-level pair is the one VOS01 describes: `inv_impl(S)` is the
implementation's own invariant (for a bitmap, `true`: all 256 masks are valid;
for a one-cell-per-thread scan, "every cell is 0 or 1"), and the **canonical**
block of section 9.3 says that `init` and every step from a `build(m)` state
lands again in the image of `build`, so every *reachable* state is a `build` of an
abstract state and the obligations, which quantify over `build(M)`, speak about
every reachable state (VOS02 R5). Two facts are evidenced separately and both
are required by X1: **init** (`canon_init`) and **closure** (the `_post` claims).

---

## 5. The two tiers

### 5.1 The abstract relations

```text
  safe_dispatch(m, o, m2)   =  valid_pick(m, o)  and  m2 == after(m, o)
  policy_dispatch(m, o, m2) =  o == top(m)       and  m2 == after(m, o)

  valid_pick(m, Idle)   = true             after(m, Idle)   = m
  valid_pick(m, Run t)  = t < N and t in m after(m, Run t)  = m without t
  top(m) = Run(max of m)  for non-empty m,  Idle for the empty set
```

**Safety tier (every profile; what system theorems may use).** `dispatch` returns
`Idle` or a thread that was in the ready set, and the new set is the old set
minus exactly that thread (so the ready set only shrinks, by at most one
element, and a chosen thread is never ready afterwards). `insert` and `remove`
are deterministic and equal the abstract operations in *both* tiers (shape A of
VOS01 section 2.3); only `dispatch` is a relation (shape B).

**`Idle` is always permitted (VOS01 Q5).** The safety tier does not say "`Idle`
only if empty": that is liveness, and a most-general implementation must be able
to return `Idle` for any state (VOS01 section 3.4). A system theorem therefore
cannot conclude "a ready thread runs"; only "the thread that runs was ready".

**Policy tier (fixed priority).** `dispatch` returns `top(m)`: the highest-priority
ready thread, and `Idle` only for the empty set. `policy_dispatch` is the graph of
a function, so a policy implementation is a *deterministic* function of M
(shape A), and it implies the safety tier (lemmas `policy_implies_safety` and
`policy_admits_only_top`, which together prove the implication for every
`(o, m2)`: the policy relation admits one pair and that pair is safe).

### 5.2 Which profile needs which tier

| Profile (VOS00 section 11) | Needs | Why |
|---|---|---|
| Hard RTOS (fixed priority) | `policy` and `safety` | the fixed-priority policy is the profile's headline property |
| Flight (ARINC-653 style) | `policy` and `safety` for the partition-internal scheduler; the major-frame partition scheduler is a separate contract | noninterference over a deterministic scheduler (VOS01 section 3.4, Q8) |
| Soft real time | `safety` only | isolation is reused unchanged; budget enforcement is a different leaf (section 13) |
| GPOS, teaching | `safety` only | |

### 5.3 What other schedulers satisfy

* **Round-robin** (VOS-012's second implementation) satisfies the **safety tier
  only**: it returns some ready thread, rotates its pointer, and is not
  highest-priority-first. Its state has history (the rotation pointer) that `alpha`
  forgets, so it is **not canonical**: it declares `mode=representation`
  (VOS02 section 2.7), writes its own `dispatch_rep` claims over `inv_impl`
  states and a determinism claim, and does not carry the `canonical` block. The
  negative control the "history register" mutants (section 12)
  shows the canonical block catching exactly this.
* **Earliest-deadline-first** satisfies the safety tier only. Under this contract it
  can implement `Sched` only if its deadline arrives some other way than through
  these four operations, so a real EDF scheduler needs a deadline channel
  that this interface does not have (section 13, VOS-D048). It is *not* a
  tier of this contract.
* **A lowest-priority-first scan** (the toy of VOS01 section 5.4) satisfies safety
  and is canonical; the evaluation of section 12 shows it fails the policy claim on
  exactly 247 of 256 states.
* **`-any`** satisfies the safety tier and nothing more, by construction
  (VOS01 section 3).

---

## 6. Caller preconditions, error values, and the spine's use

**Errors are values.** `insert(s, t)` and `remove(s, t)` with `t >= N` answer
`OutOfRange` and return `s` unchanged (lemmas `insert_outside_pre_changes_nothing`,
`remove_outside_pre_changes_nothing`). The precondition models are
`pre_insert(m, t) = t < N` and `pre_remove(m, t) = t < N`. `dispatch` has no
precondition: it is defined for all 256 states. **No operation panics**, and the
`input` rows of the generated documents carry `hostile` and
`ensures no_panic, terminates`.

**The spine's obligations (rule 6).** The scheduler cannot tell whether `t` names
a live thread, only whether it is below `N`; the thread table owns liveness. The
spine therefore carries one `site_<entry>_<step>` claim per call of `insert` and
`remove` ("the argument satisfies `pre_insert`/`pre_remove`", discharged from
the thread-table invariant that every live id is below `N`) and **none for
`dispatch`**. A timer entry in the shape of VOS00 section 4.8:

```text
  1. (upd, sched) = insert(sched, running)        -- only if the running thread should stay runnable
  2. (pick, sched) = dispatch(sched)              -- the chosen thread is no longer ready
  3. running := pick (Idle -> the idle context)
```

The spine invariant "the running thread is not in the ready set" holds after
step 2 by the safety tier alone, for every implementation, including `-any`.

---

## 7. Resource ceilings

Ceilings are in RV32 instructions, `uarch.*`-qualified (VOS01 Q7), and are the
**maximum any implementation may declare** (VOS01 section 2.5). They are in
`contract.vos` (VOS02 section 2.5):

```text
vos-contract 1
name vos-sched-contract
iface sched
tier any
tier canonical
tier policy
tier safety
config N 8
ceiling dispatch depth 2
ceiling dispatch heap 0
ceiling dispatch steps 160
ceiling init depth 2
ceiling init heap 0
ceiling init steps 80
ceiling insert depth 2
ceiling insert heap 0
ceiling insert steps 160
ceiling remove depth 2
ceiling remove heap 0
ceiling remove steps 160
end
```

| Operation | Steps | Heap | Depth | Derivation |
|---|---|---|---|---|
| `init` | `8 N + 16` = 80 | 0 | 2 | a one-cell-per-thread scan clears `N` cells at about 4 instructions each, plus call overhead |
| `insert`, `remove` | `16 N + 32` = 160 | 0 | 2 | the slowest admissible implementation is a scan of `N` cells at about 12 instructions each (load, compare, branch, increment, bound) plus a store and the tuple return; 16 leaves a third of slack |
| `dispatch` | `16 N + 32` = 160 | 0 | 2 | a scan for the highest member and one clearing store, same bound |

The bitmap scheduler needs a few tens of instructions and declares much less
(VOS01 section 5.7). **These figures are placeholders:** there is no instruction
observer yet (RV32SYM, VOS-D004), so the ceiling is a derived design figure, not
a measurement. They are *conservative by construction*: an implementation that
needs more must change this file, which is a contract change affecting every
implementation (VOS00 section 8). The formula is by `N`, per VER20, so the file
is regenerated with the instance.

Zero heap is structural (`no_std`, SH1). Depth 2 is a call to the operation and
at most one helper: SH5 (no recursion) makes the maximum stack the sum of frames
along the deepest call chain, which VOS-028's image companion measures.

---

## 8. Effect footprint and configuration

**Footprint: empty.** `required_capabilities.json` has `capabilities: []`; the
crate is `no_std`, `forbid(unsafe_code)`, has no dependency (SH1, SH2, SH12) and
no `static` (SH3). The contract package passes `vos-shape-check` as class K,
kind `contract`; every implementation must too (VOS01 section 7).

**Configuration.** `N`, the number of thread ids, is the trait constant
`Sched::N` and the line `config N 8` in `contract.vos`. The Warrant text writes
`8` and `8u8` literally, because a Warrant document is per instance (VOS01 Q1);
the generated document pins it with `example n_is_eight: cfg_n() == CfgN(8u8)`
in the implementation block (VOS02 R3), and the profile crate repeats it in a
`const` assert. A claim at `N = 8` says nothing at another `N`.

---

## 9. The obligations, in VOS02's vocabulary

### 9.1 The fixed vocabulary and the interim form

The implementation block declares, in the names of VOS02 R3: `record Impl(...)`,
`op init() -> Impl`, the operations, `alpha`, `build`, `inv_impl` and
`cfg_n`. Today's evidence generator binds no fault to a tuple result, so the
interim projection form of VOS02 section 4.4 is the vocabulary:

| Natural form (after WX-SV) | Interim form used in this contract |
|---|---|
| `op insert(s, t) -> (Upd, Impl)` | `insert_out(s, t) -> Upd`, `insert_img(s, t) -> Impl` |
| `op remove(s, t) -> (Upd, Impl)` | `remove_out`, `remove_img` |
| `op dispatch(s) -> (Pick, Impl)` | `dispatch_out(s) -> Pick`, `dispatch_img(s) -> Impl` |
| `op alpha(s) -> Ready` | `alpha_w(s: Impl) -> AbsW` and `model dec(x: AbsW) -> Ready` |
| `op cfg_n() -> Nat` | `op cfg_n() -> CfgN` |

When WX-SV item 2 lands the obligation text changes (one claim per operation),
every digest changes, and that is the planned contract-text event of VOS02
section 4.4.

### 9.2 The contract block

The document header and the implementation block are not part of it. This is
`warrant/contract.part`, byte for byte:

```text
  variant Upd = Changed | Unchanged | OutOfRange
  variant Pick = Idle | Run(t: U8)

  -- Abstract state M: the ready set over the thread ids 0..7. Thread t has priority t, so a
  -- larger id wins. Eight Bools: structural equality is set equality (VOS01 section 2.1).
  record Ready(p0: Bool, p1: Bool, p2: Bool, p3: Bool, p4: Bool, p5: Bool, p6: Bool, p7: Bool)
  model ready_of(b0: Bool, b1: Bool, b2: Bool, b3: Bool, b4: Bool, b5: Bool, b6: Bool, b7: Bool) -> Ready = Ready(b0, b1, b2, b3, b4, b5, b6, b7)
  model empty() -> Ready = Ready(false, false, false, false, false, false, false, false)
  model mem(m: Ready, i: U8) -> Bool = (
    (i == 0u8 and m.p0) or (i == 1u8 and m.p1) or (i == 2u8 and m.p2) or (i == 3u8 and m.p3) or
    (i == 4u8 and m.p4) or (i == 5u8 and m.p5) or (i == 6u8 and m.p6) or (i == 7u8 and m.p7))
  model set_at(cur: Bool, i: U8, k: U8, b: Bool) -> Bool = if i == k then b else cur
  model put(m: Ready, i: U8, b: Bool) -> Ready = (
    Ready(set_at(m.p0, i, 0u8, b), set_at(m.p1, i, 1u8, b), set_at(m.p2, i, 2u8, b), set_at(m.p3, i, 3u8, b),
          set_at(m.p4, i, 4u8, b), set_at(m.p5, i, 5u8, b), set_at(m.p6, i, 6u8, b), set_at(m.p7, i, 7u8, b)))
  model subset(a: Ready, b: Ready) -> Bool = (
    (not a.p0 or b.p0) and (not a.p1 or b.p1) and (not a.p2 or b.p2) and (not a.p3 or b.p3) and
    (not a.p4 or b.p4) and (not a.p5 or b.p5) and (not a.p6 or b.p6) and (not a.p7 or b.p7))

  -- inv(M) is total: every one of the 256 states is well formed, so there is nothing to
  -- preserve by accident. It is still written down, and closure is still claimed below, so
  -- a later M with structure (queues, counts) cannot change inv without restating closure.
  model inv(m: Ready) -> Bool = true
  model init_m() -> Ready = empty()

  -- Total abstract operations: every U8 id has a defined answer.
  model abs_insert(m: Ready, t: U8) -> (Upd, Ready) = (
    if t >= 8u8 then (OutOfRange, m)
    else (if mem(m, t) then (Unchanged, m) else (Changed, put(m, t, true))))
  model abs_remove(m: Ready, t: U8) -> (Upd, Ready) = (
    if t >= 8u8 then (OutOfRange, m)
    else (if mem(m, t) then (Changed, put(m, t, false)) else (Unchanged, m)))

  -- dispatch is not a function of M in the safety tier (any ready thread, or Idle, may be
  -- chosen), so M gives it a successor as a function of the CHOICE: the chosen thread leaves.
  model after(m: Ready, o: Pick) -> Ready = (
    match o with Idle -> m | Run(t) -> put(m, t, false))
  model valid_pick(m: Ready, o: Pick) -> Bool = (
    match o with Idle -> true | Run(t) -> t < 8u8 and mem(m, t))
  -- The highest-priority ready thread (the largest id), or Idle for the empty set.
  model top(m: Ready) -> Pick = (
    if m.p7 then Run(7u8) else (if m.p6 then Run(6u8) else (if m.p5 then Run(5u8) else
    (if m.p4 then Run(4u8) else (if m.p3 then Run(3u8) else (if m.p2 then Run(2u8) else
    (if m.p1 then Run(1u8) else (if m.p0 then Run(0u8) else Idle))))))))
  -- A Pick from a byte: 255 is Idle, anything else names a thread (possibly an invalid one).
  -- Lemmas use it to range over every Pick without a variant binder.
  model pick_of(k: U8) -> Pick = if k == 255u8 then Idle else Run(k)

  -- Caller preconditions (VOS01 section 2.8): the id names a thread. Outside it the
  -- operation answers OutOfRange and changes nothing. dispatch has none.
  model pre_insert(m: Ready, t: U8) -> Bool = t < 8u8
  model pre_remove(m: Ready, t: U8) -> Bool = t < 8u8

  -- Safety tier: a RELATION over (state, output, new state). What system theorems may use.
  model safe_dispatch(m: Ready, o: Pick, m2: Ready) -> Bool = valid_pick(m, o) and m2 == after(m, o)
  -- Policy tier: the fixed-priority function. What the hard-RTOS and flight profiles add.
  model policy_dispatch(m: Ready, o: Pick, m2: Ready) -> Bool = o == top(m) and m2 == after(m, o)

  -- Lemmas (VER04m V9): no bound operation is called, so no subject evidence and no strength.
  -- closure_at_m is a TRIVIAL lemma (inv is `true`): it documents the shape, and is not counted as
  -- evidence of anything (VOS10 section 4).
  claim closure_at_m:
    forall b0: Bool, b1: Bool, b2: Bool, b3: Bool, b4: Bool, b5: Bool, b6: Bool, b7: Bool, t: U8. (
      inv(init_m()) and
      (inv(ready_of(b0, b1, b2, b3, b4, b5, b6, b7)) =>
        (inv(abs_insert(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), t).1) and
         inv(abs_remove(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), t).1) and
         inv(after(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), top(ready_of(b0, b1, b2, b3, b4, b5, b6, b7)))))))
  claim insert_outside_pre_changes_nothing:
    forall b0: Bool, b1: Bool, b2: Bool, b3: Bool, b4: Bool, b5: Bool, b6: Bool, b7: Bool, t: U8. (
      not pre_insert(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), t) =>
      (abs_insert(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), t).0 == OutOfRange and
       abs_insert(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), t).1 == ready_of(b0, b1, b2, b3, b4, b5, b6, b7)))
  claim remove_outside_pre_changes_nothing:
    forall b0: Bool, b1: Bool, b2: Bool, b3: Bool, b4: Bool, b5: Bool, b6: Bool, b7: Bool, t: U8. (
      not pre_remove(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), t) =>
      (abs_remove(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), t).0 == OutOfRange and
       abs_remove(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), t).1 == ready_of(b0, b1, b2, b3, b4, b5, b6, b7)))
  claim insert_then_remove_restores:
    forall b0: Bool, b1: Bool, b2: Bool, b3: Bool, b4: Bool, b5: Bool, b6: Bool, b7: Bool, t: U8. (
      (t < 8u8 and not mem(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), t)) =>
      (abs_insert(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), t).0 == Changed and
       abs_remove(abs_insert(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), t).1, t).1 == ready_of(b0, b1, b2, b3, b4, b5, b6, b7)))
  claim top_is_the_maximum:
    forall b0: Bool, b1: Bool, b2: Bool, b3: Bool, b4: Bool, b5: Bool, b6: Bool, b7: Bool, t: U8. (
      (mem(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), t) =>
        (match top(ready_of(b0, b1, b2, b3, b4, b5, b6, b7)) with Idle -> false | Run(h) -> t <= h)) and
      (match top(ready_of(b0, b1, b2, b3, b4, b5, b6, b7)) with
        Idle -> not mem(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), t)
      | Run(h) -> mem(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), h)))
  claim safety_is_idle_or_a_member:
    forall b0: Bool, b1: Bool, b2: Bool, b3: Bool, b4: Bool, b5: Bool, b6: Bool, b7: Bool, k: U8. (
      safe_dispatch(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), pick_of(k), after(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), pick_of(k))) ==
      (k == 255u8 or (k < 8u8 and mem(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), k))))
  claim safe_dispatch_only_removes_the_choice:
    forall b0: Bool, b1: Bool, b2: Bool, b3: Bool, b4: Bool, b5: Bool, b6: Bool, b7: Bool, k: U8. (
      valid_pick(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), pick_of(k)) =>
      (subset(after(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), pick_of(k)), ready_of(b0, b1, b2, b3, b4, b5, b6, b7)) and
       (match pick_of(k) with
          Idle -> after(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), pick_of(k)) == ready_of(b0, b1, b2, b3, b4, b5, b6, b7)
        | Run(h) -> after(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), pick_of(k)) == put(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), h, false) and not mem(after(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), pick_of(k)), h))))
  claim policy_implies_safety:
    forall b0: Bool, b1: Bool, b2: Bool, b3: Bool, b4: Bool, b5: Bool, b6: Bool, b7: Bool. (
      policy_dispatch(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), top(ready_of(b0, b1, b2, b3, b4, b5, b6, b7)), after(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), top(ready_of(b0, b1, b2, b3, b4, b5, b6, b7)))) =>
      safe_dispatch(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), top(ready_of(b0, b1, b2, b3, b4, b5, b6, b7)), after(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), top(ready_of(b0, b1, b2, b3, b4, b5, b6, b7)))))
  claim policy_admits_only_top:
    forall b0: Bool, b1: Bool, b2: Bool, b3: Bool, b4: Bool, b5: Bool, b6: Bool, b7: Bool, k: U8. (
      policy_dispatch(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), pick_of(k), after(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), pick_of(k))) =>
      pick_of(k) == top(ready_of(b0, b1, b2, b3, b4, b5, b6, b7)))
```

### 9.3 The obligation blocks

Four blocks, one per tier of `contract.vos` (VOS02 R1.2). A claim is quantified
over `build(m)` for the abstract `m` (R5), so its binders are `Bool` and `U8`
only, and every claim is *exhaustive* where the instance fits.

| Block | Claims | Shape | Carried by |
|---|---|---|---|
| `safety` | `insert_safe`, `remove_safe`, `dispatch_safe`, `safety_alpha_build` | A for `insert`/`remove`, B for `dispatch`, C for `alpha` | every implementation |
| `policy` | `dispatch_policy` | A (deterministic) | fixed-priority implementations |
| `canonical` | `canon_init`, `canon_insert_post`, `canon_insert_img`, `canon_remove_post`, `canon_remove_img`, `canon_dispatch_post`, `canon_dispatch_img` | shape C and closure | every `production` implementation with `mode=canonical` |
| `any` | `dispatch_sound`, `dispatch_covers_run`, `dispatch_covers_idle` | soundness and coverage | `kind = any` only |

**Safety** (`warrant/obligations-safety.part`):

```text
  -- insert and remove are deterministic in both tiers (shape A of VOS01 section 2.3).
  claim insert_safe:
    forall b0: Bool, b1: Bool, b2: Bool, b3: Bool, b4: Bool, b5: Bool, b6: Bool, b7: Bool, t: U8. (
      insert_out(build(ready_of(b0, b1, b2, b3, b4, b5, b6, b7)), t) == abs_insert(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), t).0 and
      dec(alpha_w(insert_img(build(ready_of(b0, b1, b2, b3, b4, b5, b6, b7)), t))) == abs_insert(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), t).1)
  claim remove_safe:
    forall b0: Bool, b1: Bool, b2: Bool, b3: Bool, b4: Bool, b5: Bool, b6: Bool, b7: Bool, t: U8. (
      remove_out(build(ready_of(b0, b1, b2, b3, b4, b5, b6, b7)), t) == abs_remove(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), t).0 and
      dec(alpha_w(remove_img(build(ready_of(b0, b1, b2, b3, b4, b5, b6, b7)), t))) == abs_remove(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), t).1)
  -- dispatch is a relation (shape B): any permitted choice, and the chosen thread leaves.
  claim dispatch_safe:
    forall b0: Bool, b1: Bool, b2: Bool, b3: Bool, b4: Bool, b5: Bool, b6: Bool, b7: Bool. (
      safe_dispatch(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), dispatch_out(build(ready_of(b0, b1, b2, b3, b4, b5, b6, b7))), dec(alpha_w(dispatch_img(build(ready_of(b0, b1, b2, b3, b4, b5, b6, b7)))))))
  -- alpha is onto M (shape C of VOS01 section 2.3): build and alpha agree on every abstract state.
  -- It pins build, alpha and dec for EVERY implementation: a representation-mode scheduler and -any
  -- carry the safety tier but not canonical, and would otherwise have an alpha nothing constrains.
  claim safety_alpha_build:
    forall b0: Bool, b1: Bool, b2: Bool, b3: Bool, b4: Bool, b5: Bool, b6: Bool, b7: Bool. dec(alpha_w(build(ready_of(b0, b1, b2, b3, b4, b5, b6, b7)))) == ready_of(b0, b1, b2, b3, b4, b5, b6, b7)
```

**Policy** (`warrant/obligations-policy.part`):

```text
  -- The fixed-priority policy: the highest-priority ready thread, Idle only for the empty set.
  claim dispatch_policy:
    forall b0: Bool, b1: Bool, b2: Bool, b3: Bool, b4: Bool, b5: Bool, b6: Bool, b7: Bool. (
      policy_dispatch(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), dispatch_out(build(ready_of(b0, b1, b2, b3, b4, b5, b6, b7))), dec(alpha_w(dispatch_img(build(ready_of(b0, b1, b2, b3, b4, b5, b6, b7)))))))
```

**Safety also holds `safety_alpha_build`** (`alpha(build(m)) == m`, shape C): it is
field-agnostic, and every implementation needs `build`, `alpha` and `dec` pinned,
including a representation-mode scheduler and `-any`, which do not carry `canonical`.

**Canonical** (`warrant/obligations-canonical.part`). `canon_init` states that
`init()` is `build(empty())` (an `example` would do, but an obligation cannot be
left out of an implementation block unnoticed); the `_post` claims state that the
successor is in the image of `build`; the `_img` claims state it commutes with the
abstract step, with the image operation as the claim's subject so a fault in it
has a negative control (VOS02 section 4.4). `canon_dispatch_img` takes the
abstract successor from the implementation's own choice, because the successor
depends on it:

```text
  -- The reachable states are exactly the image of build (VOS02 R5). alpha(build(m)) == m is not
  -- here: it is field-agnostic and every tier needs it, so it is safety_alpha_build. The commuting claims have
  -- the image operation as their subject, so a fault in it has a negative control (VOS02 4.4).
  claim canon_init:
    forall b: Bool. init() == build(empty())
  claim canon_insert_post:
    forall b0: Bool, b1: Bool, b2: Bool, b3: Bool, b4: Bool, b5: Bool, b6: Bool, b7: Bool, t: U8. build(dec(alpha_w(insert_img(build(ready_of(b0, b1, b2, b3, b4, b5, b6, b7)), t)))) == insert_img(build(ready_of(b0, b1, b2, b3, b4, b5, b6, b7)), t)
  claim canon_insert_img:
    forall b0: Bool, b1: Bool, b2: Bool, b3: Bool, b4: Bool, b5: Bool, b6: Bool, b7: Bool, t: U8. insert_img(build(ready_of(b0, b1, b2, b3, b4, b5, b6, b7)), t) == build(abs_insert(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), t).1)
  claim canon_remove_post:
    forall b0: Bool, b1: Bool, b2: Bool, b3: Bool, b4: Bool, b5: Bool, b6: Bool, b7: Bool, t: U8. build(dec(alpha_w(remove_img(build(ready_of(b0, b1, b2, b3, b4, b5, b6, b7)), t)))) == remove_img(build(ready_of(b0, b1, b2, b3, b4, b5, b6, b7)), t)
  claim canon_remove_img:
    forall b0: Bool, b1: Bool, b2: Bool, b3: Bool, b4: Bool, b5: Bool, b6: Bool, b7: Bool, t: U8. remove_img(build(ready_of(b0, b1, b2, b3, b4, b5, b6, b7)), t) == build(abs_remove(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), t).1)
  claim canon_dispatch_post:
    forall b0: Bool, b1: Bool, b2: Bool, b3: Bool, b4: Bool, b5: Bool, b6: Bool, b7: Bool. build(dec(alpha_w(dispatch_img(build(ready_of(b0, b1, b2, b3, b4, b5, b6, b7)))))) == dispatch_img(build(ready_of(b0, b1, b2, b3, b4, b5, b6, b7)))
  -- dispatch's successor depends on the choice, which is the implementation's own output.
  claim canon_dispatch_img:
    forall b0: Bool, b1: Bool, b2: Bool, b3: Bool, b4: Bool, b5: Bool, b6: Bool, b7: Bool. dispatch_img(build(ready_of(b0, b1, b2, b3, b4, b5, b6, b7))) == build(after(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), dispatch_out(build(ready_of(b0, b1, b2, b3, b4, b5, b6, b7)))))
```

**Any** (`warrant/obligations-any.part`). The oracle register is a field of
`Impl`, so these claims can only be carried by an implementation whose state has
`mask` and `choice` (VOS02 section 3.3). They are not quantified over `build(m)`:
the oracle is a binder.

```text
  -- The most-general scheduler (VOS01 section 3). Impl is Impl(mask: U8, choice: U8): a bitmap
  -- plus the oracle register that alpha drops. The oracle is a binder here, so these claims
  -- quantify over Impl directly and not over build(M) (VOS02 section 3.3).
  claim dispatch_sound:
    forall mask: U8, c: U8. (
      safe_dispatch(dec(alpha_w(Impl(mask, c))), dispatch_out(Impl(mask, c)), dec(alpha_w(dispatch_img(Impl(mask, c))))) and
      dispatch_img(Impl(mask, c)).choice == c)
  -- Coverage with a witness: thread t is produced by the oracle value t, Idle by 255.
  claim dispatch_covers_run:
    forall mask: U8, t: U8 | t < 8u8. (
      mem(dec(alpha_w(Impl(mask, t))), t) => dispatch_out(Impl(mask, t)) == Run(t))
  claim dispatch_covers_idle:
    forall mask: U8. dispatch_out(Impl(mask, 255u8)) == Idle
```

`dispatch_covers_run` uses a **witness** (thread `t` by oracle value `t`, `Idle` by
255) so that coverage needs no nested `exists` (VOS01 section 3.3). Its negative
controls (a sabotaged `-any` that must be rejected) need WX-SV item 2 in the real
generator; at the Warrant-model level they are the controls of section 12.

### 9.4 Claims that are not obligations

An implementation may add claims of its own (printed `impl-own`, VOS02 section
2.6): the any implementation adds `alpha_build_with_oracle` and
`insert_keeps_the_oracle`, which are not obligations because they name its own state.

---

## 10. Case counts at N = 8

Counted as VOS01 section 5.6 and VER04h section 3 count: the **binder product
before refinement** is what the 400,000-case budget (VOS30 section 8.5) bounds, the
**admitted** count is what the manifest reports. `ready` is 8 `Bool` binders
(256 states); a `Bool` binder doubles, a `U8` binder multiplies by 256. Every binder
is a complete type, so every claim is kind `full`. The counts below are the
counts the evaluation of section 12 measured, not typed by hand.

| Claim | Binders | Product | Admitted | % budget |
|---|---|---|---|---|
| **Contract-block lemmas** | | | | |
| `closure_at_m` | 8 Bool, `t: U8` | 65,536 | 65,536 | 16.38 |
| `insert_outside_pre_changes_nothing` | 8 Bool, `t: U8` | 65,536 | 65,536 | 16.38 |
| `remove_outside_pre_changes_nothing` | 8 Bool, `t: U8` | 65,536 | 65,536 | 16.38 |
| `insert_then_remove_restores` | 8 Bool, `t: U8` | 65,536 | 65,536 | 16.38 |
| `top_is_the_maximum` | 8 Bool, `t: U8` | 65,536 | 65,536 | 16.38 |
| `safety_is_idle_or_a_member` | 8 Bool, `k: U8` | 65,536 | 65,536 | 16.38 |
| `safe_dispatch_only_removes_the_choice` | 8 Bool, `k: U8` | 65,536 | 65,536 | 16.38 |
| `policy_implies_safety` | 8 Bool | 256 | 256 | 0.06 |
| `policy_admits_only_top` | 8 Bool, `k: U8` | 65,536 | 65,536 | 16.38 |
| **Safety** | | | | |
| `insert_safe`, `remove_safe` (each) | 8 Bool, `t: U8` | 65,536 | 65,536 | 16.38 |
| `dispatch_safe` | 8 Bool | 256 | 256 | 0.06 |
| `safety_alpha_build` | 8 Bool | 256 | 256 | 0.06 |
| **Policy** | | | | |
| `dispatch_policy` | 8 Bool | 256 | 256 | 0.06 |
| **Canonical** | | | | |
| `canon_init` | `b: Bool` | 2 | 2 | 0.00 |
| `canon_insert_post`, `canon_insert_img`, `canon_remove_post`, `canon_remove_img` (each) | 8 Bool, `t: U8` | 65,536 | 65,536 | 16.38 |
| `canon_dispatch_post`, `canon_dispatch_img` (each) | 8 Bool | 256 | 256 | 0.06 |
| **Any** | | | | |
| `dispatch_sound` | `mask: U8`, `c: U8` | 65,536 | 65,536 | 16.38 |
| `dispatch_covers_run` | `mask: U8`, `t: U8 | t < 8` | 65,536 | 2,048 | 16.38 |
| `dispatch_covers_idle` | `mask: U8` | 256 | 256 | 0.06 |

**The largest single claim is 65,536 binder cases, 16.4 percent of the budget**,
as in the toy. The obligation totals per implementation (enumerated cases):

| Implementation | Carries | Enumerated cases |
|---|---|---|
| fixed-priority bitmap (`production`) | safety, policy, canonical | **394,498** |
| lowest-first scan (safety, canonical) | safety, canonical | 131,584 + 262,658 = 394,242 |
| round-robin (safety only, `representation`) | safety, plus its own `_rep` claims | 131,584 + its own |
| `-any` | safety, any | **199,424** admitted (262,912 binder cases) |

Where the budget ends for this contract:

| Variation | Binder product | Verdict |
|---|---|---|
| `insert` at `N = 10` (state `2^10`, `t: U8`) | `1,024 x 256 = 262,144` | inside |
| `insert` at `N = 11` | `2,048 x 256 = 524,288` | over: W0706 |
| `dispatch` alone at `N = 18` / `N = 19` | `262,144` / `524,288` | inside / over |
| `-any` `dispatch_sound` (state `U8`, oracle `U8`) | `65,536` | inside |
| any claim at `N = 64` | `2^64` | not enumerable: X2 and X3 only |

So the reduced instance is `N <= 10` for the operations with a full `U8` id and
`N <= 18` for `dispatch`; `N = 8` is the instance the first scheduler needs, and
(VOS30 section 8.5) X3 for a bitmap is width-insensitive, so the X3 instance need not
equal it. A **thread-table-sized** scheduler state (an array indexed by a runtime id)
is out of this contract's reach: plan on `T <= 8..16` per obligation there.

---

## 11. What VOS-011 and VOS-012 must provide

### 11.1 VOS-011: `vos-sched-fp-bitmap` (kind `production`, mode `canonical`)

* `Sched` for a private `Bitmap { mask: u8 }` with `N = 8`; `dispatch` returns the
  highest set bit and clears it, with a branch-free leading-zero routine (the
  toy's `clz8`; `u8::leading_zeros` may become a runtime call, VOS01 section 5.3).
* Carries **safety, policy and canonical**: all twelve obligations, character for
  character (R4). `vos.toml`: `kind = "production"`, `evidence-features = ["ghost"]`.
* The ghost face of VOS02 section 4.2: a `ghost` module with a public-field
  `Image`, wrappers `vos_ghost_insert_out`, `_insert_img`, `_remove_out`,
  `_remove_img`, `_dispatch_out`, `_dispatch_img`, `_alpha_w`, `_cfg_n`, each an
  exhaustive destructure, one UFCS call, a restructure (SH17), re-exported at the
  crate root under the same feature (SH18). Its `evidence.bindings` names the
  `Impl`, `AbsW`, `CfgN`, `Upd`, `Pick` types and gives every `op` a fault the
  generator can apply (section 14: D021).
* Declares `bound` rows at or under the ceilings of section 7 and the
  `example n_is_eight` pin.
* Needs `vos-link gen-doc` (VOS-033), the shape check on the crate (VOS-D002 is done
  for the lexical half), and VOS-D032's `clippy::arithmetic_side_effects` wiring
  because SH9 is not lexically checkable.

### 11.2 VOS-012: `vos-sched-any` and `vos-sched-rr`

* **`vos-sched-any`** (kind `any`, mode `none`): the bitmap plus an oracle register
  `choice` (`Impl(mask, choice)`), `dispatch` proposing, validating and
  defaulting to `Idle`; the oracle-write path only in the `ghost` module; `insert`
  and `remove` never read the oracle (a lexical rule). Carries **safety and
  any**, never policy or canonical. Its `-any` block is the `obligations-any.part`
  above.
* **`vos-sched-rr`** (kind `production` or `reference`, mode `representation`): a
  rotating pointer, safety tier only, `dispatch_rep` claims and a determinism
  claim in its own `impl.part`.
* The two together are the replaceability exercise (VOS00 section 8): the policy
  claims, `vos-sched-fp-bitmap`'s evidence and the contract block are **not**
  regenerated.
* **Blockers in today's generator**, logged as discovered work: a byte-array state
  (`Bytes<8>`) in a round-robin would hit D025 (`fixed<8>`), and negative controls
  for the `-any` coverage claims need WX-SV item 2 (D021).

---

## 12. Validation performed in this PR

No scheduler exists, so no evidence about Rust is claimed. What was checked, with
the repository's Warrant tooling (`coding_adventures_warrant_grammar`: parser,
`check_warrant`, and the ground `Evaluator`), in
`code/packages/rust/vos-sched-contract-check/tests/warrant_contract.rs`:

0. **Where the checks live.** The shape check (VOS03 SH12) allows a class K crate no
   Warrant-tooling dependency, so the Warrant checks are in the sibling crate
   `vos-sched-contract-check` (class H), which joins the `.part` files into documents by
   string concatenation and calls the repository's `warrant-grammar` for everything
   else. The Rust-level checks are in the contract crate's own `tests/trait_usable.rs`.
1. **The contract block type-checks** alone, and its nine model-level lemmas hold
   over their **whole domains** (table above; no failure).
2. **A generated-shape document type-checks**: header, contract block, an
   implementation block with real `op` declarations and its `labels`, `input` and
   `bound` rows, and the safety, policy and canonical blocks. The `any` block
   type-checks against a two-field `Impl` and is *rejected* against the one-field
   bitmap `Impl`.
3. **Three implementations written as Warrant models**, independently of the
   abstract record (bit operations, a one-cell-per-thread scan, a bitmap with an
   oracle), are run against the obligations over every binder value: the
   fixed-priority bitmap passes all twelve; the lowest-first scan passes every claim
   but `dispatch_policy`, which it fails on exactly **247** of 256 states; the oracle
   model passes safety and `any`.
4. **Negative controls: a table of 19 mutants plus 9 lemma mutants**
   (`every_mutant_breaks_exactly_the_claims_it_should`,
   `every_lemma_has_a_mutant_that_breaks_it`, `every_obligation_has_a_control`). Each
   mutant is a textual change to one model, asserted to have changed it. A claim is
   evaluated for a mutant iff its own text names a mutated operation (a claim that does
   not name it cannot fail), and the set of evaluated claims that fail must equal the
   expected set **exactly**. What each control is for:

   | Claim (conjunct) | Control: a wrong implementation that breaks it | Exact set that fails |
   |---|---|---|
   | `insert_safe`, `_out` | `insert_out` without the range check | `insert_safe` |
   | `insert_safe`, `_img` | `insert_img` returns the state unchanged for `t < 8`; or sets bit 0 whatever `t` is | `insert_safe`, `canon_insert_img` |
   | `remove_safe`, `_out` | `remove_out` always `Unchanged` | `remove_safe` |
   | `remove_safe`, `_img` | `remove_img` returns the state unchanged; or clears every bit | `remove_safe`, `canon_remove_img` |
   | `dispatch_safe`, `_out` | `dispatch` answers `Run(0)` for the empty set (a thread that is not ready) | `dispatch_safe`, `dispatch_policy` |
   | `dispatch_safe`, `_img` | `dispatch_img` forgets to remove the chosen thread | `dispatch_safe`, `dispatch_policy`, `canon_dispatch_img` |
   | `safety_alpha_build` | `alpha` forgets the state; `build` ignores thread 7 | `alpha`: the four safety claims, `dispatch_policy` and the three `_post` claims; `build`: the four safety claims, `dispatch_policy` and `canon_insert_post`, `canon_insert_img` |
   | `dispatch_policy` | `dispatch` always `Idle`, state kept | `dispatch_policy` only |
   | `canon_init` | `init()` is not `build(empty())` | `canon_init` only |
   | `canon_insert_post`, `canon_remove_post`, `canon_dispatch_post` | a history register that `alpha` drops, moved by `insert_img`, `remove_img`, `dispatch_img` respectively (the oracle model) | the `_post` claim and its `_img` twin, never safety |
   | `canon_insert_img`, `canon_remove_img`, `canon_dispatch_img` | see the `_img` rows above | |
   | `dispatch_sound`, safe part | `any` skips the validation step | `dispatch_safe`, `dispatch_sound` |
   | `dispatch_sound`, `.choice == c` | `any`'s `dispatch_img` moves the oracle | `dispatch_sound` only |
   | `dispatch_covers_run` | `any` never proposes thread 3 | `dispatch_covers_run` only |
   | `dispatch_covers_idle` | `any` answers `Run(7)` for oracle 255 on a non-empty set | `dispatch_sound`, `dispatch_covers_idle` |
   | each of the nine lemmas | one mutation of the contract block per lemma | that lemma |

   Three honest limits. (a) The `_post` claims are *implied* by their `_img` twins plus
   `safety_alpha_build`: no mutant can break a `_post` claim alone (an `img` equal to a
   `build` value is in the image), so the controls assert the pair, and the `_post`
   claims earn their place only where `alpha` itself is wrong (the `alpha` mutant breaks
   the three `_post` claims and not the `_img` ones). (b) A `build` that ignores thread 7 is caught
   by the safety claims and, for `insert` only (it sets thread 7), by the canonical
   `insert` pair; the canonical `remove` and `dispatch` claims are blind to it, because
   they compare `build` with `build`. Only `safety_alpha_build` states what `build` and
   `alpha` mean, which is why it is a safety obligation. (c) `closure_at_m` is a *trivial* lemma
   (`inv` is `true`): its mutant makes `inv` non-trivial and breaks it, but the lemma
   is not evidence of anything and is not counted as such.
5. **The `.part` files obey the marker discipline** of VOS02 section 2.3 (ASCII, no
   control characters, no `-- vos02:`), and an obligations part contains claim
   declarations only. This is a stand-in for `vos-link gen-doc` (VOS-033), not the
   authority.
6. **The Rust trait crate and the Warrant text agree**: the enum constructors of `Upd`
   and `Pick` equal the Warrant variants (names, order, payload arity), through
   exhaustive `match`es without a wildcard, so a new Rust variant is a compile error.
   That the literal `8` of the text, `Sched::N` and `config N 8` in `contract.vos` agree
   is **not** checked: it needs `vos-manifest` (VOS-033, VOS-D049). Two plain Rust
   schedulers (a bitmap and a lowest-first scan) implement `Sched` and are checked over
   every state and id at `N = 8` against the same safety and policy statements; the scan
   fails the policy on exactly 247 states (`tests/trait_usable.rs`).

**Results.** See the report of the commit; the Warrant suite runs several million claim
evaluations in a few minutes of CPU, about a minute or two of wall time in a debug build.

**What this does not establish.** The evaluator runs Warrant models, not compiled
Rust: the claim that *Rust* refines the contract is VOS-011's evidence. The
generator was not run (no binding exists); that the claims bind (a subject operation
found, each `op` with an applicable fault, an unused binder in `canon_init`
accepted) is exactly what VOS-011 will find out, and each possible failure is
logged in section 14. The document header, markers and digests are
`vos-link gen-doc` (VOS-033); in this PR the test assembles the document by
joining strings.

**What remains for VOS-033** (logged as VOS-D049): assemble `code/specs/<impl>.warrant`
from these parts with markers and digests, the regeneration check, and a `contract.vos`
parser (`vos-manifest`) that cross-checks `contract.vos`'s `tier` lines against the
`obligations-*.part` files present (this package has all four), the `config` value
against `Sched::N`, the ceiling comparison against each implementation's `bound` rows,
and the kind rules of section 14 (F1); and fold the checks of this section into its tests.

---

## 13. Time: sporadic servers, budgets and deadlines

**Decision S11: a later, separate contract, not a tier here.**

Budget (sporadic-server) and deadline scheduling need *time*: a thread's budget is
consumed by elapsed ticks and replenished at a period, an EDF key is an absolute
deadline. Both are state this contract does not own. Four reasons for a separate
`vos-budget-contract`:

1. **Ownership (rule 2).** Budgets and deadlines are per-thread timing state; the
   thread table owns thread state and the time leaf owns the clock. Putting them in
   the scheduler would make `Sched` own a second copy of facts two other leaves hold.
2. **Composition without calls (rule 1).** The spine mediates: on a tick it asks the
   budget leaf `charge(budget, running, ticks)`; if the budget is exhausted it calls
   `remove` (or simply does not re-`insert`) on the scheduler, and when a budget is
   replenished it calls `insert`. The scheduler's contract does not change, so
   **every fixed-priority obligation and every system theorem over `-any` stays valid
   unchanged** (VOS00 section 11: "Isolation reused unchanged; budget enforcement").
   A policy tier in `Sched` carrying time would add a time input to every operation
   of every implementation, invalidating the evidence of VOS-011.
3. **The tier test (VOS01 section 2.4).** A tier is a strengthening of what the *same*
   operations promise. Budget enforcement is not "`dispatch` picks differently"; it is
   "a thread stops being ready when its budget runs out", an effect on `insert` and
   `remove` calls made by the spine.
4. **EDF.** EDF is the one scheduling policy that needs a per-thread key *inside* the
   scheduler. It is a `Sched` implementation that also needs a deadline channel
   (a deadline-carrying `insert`, or an `set_deadline`), which this four-operation
   interface does not have. Two honest options exist: a sibling interface
   (`vos-edf-contract`) or `Sched` version 2; neither is needed before the soft
   real-time profile, which VOS00 section 11 says draws on the *same safety tier*.
   Logged as VOS-D048.

So for soft real time the first deliverable is VOS-012's round-robin scheduler (safety
tier) plus, later, the budget leaf. A **timing label** on `dispatch` is
`public` for the RTOS profile (VOS01 section 2.6): no timing claim; a flight profile
with a partition-private ready set would declare `s secret` and the constant-time
obligation, which needs `RV32SYM` and is withheld until then.

---

## 14. Findings against VOS01 and VOS02, and discovered work

Found while writing a real contract. F1 to F4 and the SH12 wording were **amended in VOS01, VOS02 and VOS03 in the same PR** (the reconciliation commit); the effect columns below describe the state before, and the backlog rows say what remains (F1: the `vos-link` implementation). Each is logged in the
[backlog](VOS-BACKLOG.md); where the fix is text, the spec that owns it is named.

| # | Finding | Effect here | Row |
|---|---|---|---|
| F1 | VOS02 section 2.4 lists `canon_build_alpha` (`forall mask: U8. build(dec(alpha_w(Impl(mask)))) == Impl(mask)`), which **names the fields of `Impl`**. R3 says the fields differ per implementation, so this claim cannot be one text with one digest (a one-cell-per-thread scan has no `mask`). Without it, the canonical block (steps from `build(M)`) **does not exclude `-any`**: the oracle is 0 in every `build` state and an oracle nothing moves is canonical. The evaluation proves it (`the_oracle_at_zero_is_canonical_so_kind_is_enforced_by_rule`). So VOS02 R6's argument "`any` cannot discharge `canonical` because `alpha` drops the oracle" is not an evidence fact for a field-agnostic block | This contract's canonical block omits `canon_build_alpha`; the exclusion of `any` from `canonical` is a **rule** `vos-link` must enforce (`kind = any` may not carry `canonical`), and SH16 (never in an image) is unchanged. VOS02 R5/R6 and section 2.4 need an amendment | VOS-D050 |
| F2 | VOS02 section 6.2's `IDENT = [a-z][a-z0-9-]{0,31}` (also used for `ceiling` operations in `contract.vos`) **forbids an underscore**, so an operation named `make_ready` or `set_priority` cannot have a ceiling. Rust and Warrant operation names are snake_case | Operation names here are single words (`init`, `insert`, `remove`, `dispatch`) so `contract.vos` parses. VOS02 section 2.5 should widen the ceiling operation name to `[a-z][a-z0-9_]*` | VOS-D051 |
| F3 | VOS01 section 1.2 puts `Profile` and `Kernel<P>` in the contract crate (`vos-ready-contract`). A profile has **one associated type per interface**, so it cannot live in any one contract crate without that crate depending on all the others | `vos-sched-contract` exports the trait and its two enums only; `Profile` and `Kernel<P>` are the spine's (VOS-026). VOS01 section 1.2 should say so | VOS-D052 |
| F4 | `dispatch` is **destructive**, so its safety tier has no abstract successor function of M alone; the toy's `pick` was a read. VOS01 section 2.3's shape B needs `m2` to be related to the *output* (`after(m, o)`), and canonical claims need the successor taken from the implementation's own output (`canon_dispatch_img`). VOS01 section 5 does not show a shape-B operation with an output-dependent successor | Written here (section 9); VOS01 section 2.3 could cite it | VOS-D053 |
| F5 | A claim with no use of its binders (`canon_init: forall b: Bool. init() == build(empty())`) is a convention not exercised by the generator; if VER04h rejects an unused binder, `canon_init` becomes an `example` in the implementation block and the obligation list loses an entry (a contract-text change) | unresolved until VOS-011 runs the generator | VOS-D054 |
| F6 | VER04h finds a claim's negative-control subject by a shallow left-to-right search (VOS02 section 13.3). The `any` claims (`dispatch_sound`) lead with `safe_dispatch(...)`, a non-operation call; the subject found may be `dispatch_out` or `alpha_w` depending on how arguments are searched. Whether each obligation has a control that can catch a fault in the operation it is *about* is not checked until a generator runs | unresolved until VOS-011/012 | VOS-D054 |
| F7 | VOS02 section 2.5 gives `contract.vos` a `tier` line for each of `any`, `canonical`, `policy`, `safety`; nothing says that a tier line and the files `warrant/obligations-<tier>.part` must correspond. A tier with no file, or a file with no tier, is a silent mismatch until `vos-manifest` exists. `contract.vos` has its own grammar and its parser is `vos-manifest`, so this PR does not read it (no ad-hoc parser) | `vos-manifest` / `gen-doc` (VOS-033) must cross-check tiers, files and `config` against `Sched::N` and the `8`s in the text | VOS-D049 |

---

## 15. The package

```text
code/packages/rust/vos-sched-contract/
    Cargo.toml  vos.toml              class K, kind contract; no dependencies
    src/lib.rs                        trait Sched, Upd, Pick
    warrant/contract.part             section 9.2
    warrant/obligations-{safety,policy,canonical,any}.part      section 9.3
    contract.vos                      section 7 (ceilings, N = 8)
    tests/trait_usable.rs             the Rust-level checks of section 12 (item 6)
    required_capabilities.json        empty
    BUILD  BUILD_windows  README.md  CHANGELOG.md

code/packages/rust/vos-sched-contract-check/                   class H, test-only
    tests/warrant_contract.rs         section 12 (items 0 to 5), and the control table
    tests/fixtures/impl-*.part        three implementations as Warrant models, and the bound-operation block
```

The contract crate has no Rust code beyond declarations: two enums and a trait, which
the compiler checks. There is no function to cover, so a line-coverage percentage is not
meaningful (no coverage tool is installed on the build host either); the trait is
exercised by two implementations in `tests/trait_usable.rs` so that it is known to be
implementable, and its names are checked against the Warrant variants.

---

## 16. Sources

[VOS00](VOS00-verified-os-program.md) sections 4.4, 4.8, 7, 8, 11;
[VOS01](VOS01-component-contract-model.md) sections 1 to 6;
[VOS02](VOS02-interface-contracts-and-profile-linking.md) R1 to R9, sections 2, 3,
4, 13;
[VOS03](VOS03-component-shape-checker.md);
[VOS30](VOS30-proof-route-and-capacity.md) section 8.5;
[VER04j](VER04j-warrant-v1-extensions.md); [VER04m](VER04m-warrant-stateful-objects.md);
[VER20](VER20-resource-timing-proofs.md).
