# VOS01 -- The Component Contract Model

**Status:** Draft; nothing implemented. No crate, contract package or program
`.warrant` file exists yet. The Warrant listings below were type-checked with the
repository's `warrant-grammar` checker and the model-only lemmas were evaluated
over their whole domains. The bitmap implementation of section 5 was later run end
to end through the evidence generator as the acceptance spike of
[VOS02](VOS02-interface-contracts-and-profile-linking.md) section 13 (in the
interim form of VOS02 section 4.4, which today's generator needs); the toy
interface remains illustrative, and the real scheduler contract is VOS10
([backlog row VOS-010](VOS-BACKLOG.md)).

**Layer:** Program model, directly under the umbrella. Every VOS1x, VOS2x and
VOS4x spec cites this document for what a component is, what its contract
contains and what evidence each part earns.

**Parent:** [VOS00](VOS00-verified-os-program.md) (the verified OS program),
sections 4, 8, 9 and 13.

**Builds on:** [VER04](VER04-warrant-language.md) (Warrant v0),
[VER04h](VER04h-warrant-derived-evidence.md) (derived evidence),
[VER04j](VER04j-warrant-v1-extensions.md) (v1 integers, bit operations, records,
variants, bounds), [VER04m](VER04m-warrant-stateful-objects.md) (stateful
objects; labels and vacuity guards), [VER29](VER29-layer-contract-linking.md)
(contract ids and strength), [VER20](VER20-resource-timing-proofs.md) (bounds and
constant time), [VER08a](VER08a-rust-capability-analyzer.md) (capability
analyzer), [VER23a](VER23a-rust-effect-modules.md) (effect certificates) and
[VER30](VER30-warrant-expansion-roadmap.md) (what adopts today).

---

## 0. How to read this document

VOS00 fixed the *shape* of the program: eight structural rules and a contract
with eleven parts. This document makes each precise enough that two people
writing two components, or two implementations of one component, produce things
that fit without talking to each other. For every part of a contract it answers:
what does it mean, how is it written in Warrant **today**, what evidence and label
does it earn, and which toolchain row would make it stronger.

**Marks.** **today** means it exists on this branch, in the spec or code cited.
**NEW(row)** means it does not exist and the row names what closes it. The mark is
never dropped, so a reader always knows what is a plan. Rows (VOS00 section 12
orders them): `LP-012` per-kind case counts; `WX-G`/`WX-S1` `inout` and explicit
state (VER04m S1); `WX-SP1` functional-equivalence proofs; `WX-D1/D2/D3`
sequence types, `fold`/`map`/`all`, bounded unrolling; `ARR` fixed arrays of
non-byte elements in bindings (VER30 track A6) plus an arrays theory in the
solver; `RV32SYM` the symbolic RV32I executor (VER13 section 4.3, deliverable 2);
`VOS02` interface contracts and profile linking (backlog VOS-004, which carries
the VER29 amendment); `VOS30` proof route and solver capacity (VOS-028);
`VOS-owned` a check or tool this program builds.

**Two words used strictly.** An **instance** is one configuration: one set of
values for the size constants. `N = 8` and `N = 64` are different instances with
different documents and labels (VOS00 section 9.3); nothing proved at one is
carried to another. A **tier** is the *safety tier* (what system theorems may
use) or the *policy tier* (what a profile needs beyond that, VOS00 section 4.4);
each has its own claims and ids, so an implementation can discharge one without
the other.

---

## 1. The component shape

### 1.1 The step

A component is a state type and operations over it. Every operation has the shape

```text
op(State, In) -> (Out, State)
```

and obeys five laws, each there because a proof or a tool needs it.

1. **Total.** Defined for every `State` and `In` the types admit. *Why:* an
   undefined case in a kernel is a crash or an exploit; totality turns "the caller
   must not do that" into "the callee returns a defined answer" that a claim can
   pin (section 2.8).
2. **Errors are values.** A bad input yields an `Out` variant such as
   `OutOfRange`, never a panic or unwind. *Why:* a panic is a path to the panic
   handler, which no contract covers.
3. **Pure.** The result depends on the two arguments only: no clock, randomness,
   global or I/O. *Why:* this makes the binding `state=none` in the VER29 sense
   (VER29 section 6.2), so an `exhaustive` suite over complete binders is
   `exhaustive` without the hidden-state obligations of VER04m E15. VER30
   section 4.5 item 1 calls this form, state as arguments and results, "T0
   today" (`sr_latch` and `d_latch` are the worked case).
4. **Single owner.** `State` belongs to one component and to one field of the
   kernel record `K` (VOS00 rule 2). The step takes the state *by value* and
   returns the new one, so in Rust a stale copy is a compile error.
5. **Bounded.** The step terminates within a number of instructions that is a
   function of the configuration constants only (VOS00 rule 7; section 2.5).

The initial state is a constant function `init() -> State`. All operations of a
component share its one `State`. **Nondeterminism lives only in `-any`**
(section 3), and even there it enters through the state, so the signature above
does not change.

### 1.2 The Rust shape

The interface is a trait owned by the contract package: an associated `State`,
associated constants for the configuration, and one associated function per
operation, with no `self`.

```rust
// vos-ready-contract/src/lib.rs   (illustrative; see section 5)
#![no_std]
#![forbid(unsafe_code)]
#[derive(Copy, Clone, PartialEq, Eq)] pub enum Upd  { Changed, Unchanged, OutOfRange }
#[derive(Copy, Clone, PartialEq, Eq)] pub enum Pick { NoneReady, Member(u8) }

pub trait ReadySet {
    type State;                                   // owned by one field of K
    const N: u8;                                  // configuration, printed in labels
    fn init() -> Self::State;
    fn insert(s: Self::State, p: u8) -> (Upd, Self::State);
    fn remove(s: Self::State, p: u8) -> (Upd, Self::State);
    fn pick(s: Self::State) -> (Pick, Self::State);
}
```

`Profile` and `Kernel<P>` are **not** in the contract crate. A profile has one
associated type per interface, so no single contract crate can own it without
depending on every other contract crate; they live in the spine crate (or a profile
crate), which depends on all the contract crates (VOS-D052, VOS10 section 14):

```rust
// vos-spine/src/lib.rs   (illustrative)
// One associated type per interface; the profile crate binds each to ONE implementation.
pub trait Profile { type Ready: ReadySet; /* one line per interface */ }
pub struct Kernel<P: Profile> { ready: <P::Ready as ReadySet>::State, /* one field per component */ }
```

A spine call site reads `let (upd, ready) = <P::Ready as ReadySet>::insert(k.ready, t);`.
`Kernel<P>` is monomorphised per profile, so no `dyn` and no function pointer
exists; the call is UFCS, the one form VER29's lexical reach pass can name (VER29
section 15, limit 4); and `k.ready` is *moved*, so the spine cannot keep a copy of
the scheduler's state.

**Clarification of VOS00 section 4.5**, which says a call site "names its
implementation". Here it names the *interface* through the profile's associated
type, and one `impl Profile for ...` block names the implementation; that block
plus the profile manifest is the audit target. The spine source is then identical
for every profile, which rule 5 needs: a call site naming an implementation would
be edited on every swap.

### 1.3 Allowed vocabulary

`State`, `In` and `Out` use fixed-width integers, `bool`, arrays and structs of
those with constant lengths, and enums with fixed-width payloads; never `usize`,
floats, references, `Vec`, `String` or `Box` (`usize` only as a loop or array
index). *Why:* host evidence runs the source compiled for a 64-bit host, the image
for 32-bit RV32; a `usize` makes one source mean two things (VER29 section 5.1,
AC-7, already makes the compiler stack part of a claim).

### 1.4 What is forbidden, why, and what checks it

VOS00 states these as rules 2 and 7 and as the effect footprint of section 4.4.
The finding of this table is that **the existing analyzers check little of it**:
VER08a bans `unsafe`, inline assembly and foreign linkage (section 3.4 there) and
detects `std` capability use, but it classes `alloc`, `cell`, `vec`, `boxed` and
`thread_local` as pure (section 3.3), puts dynamic behaviour out of scope (gap
G-7) and knows nothing of loops, recursion or division. So most rows need one new,
VOS-owned lexical check in VER08a's style reusing its lexer, called here the
**shape check**; its rules are `SH1` to `SH15` so a diagnostic and a conformance
item can name them. VOS02 names the tool.

| Id | Rule | Why | Checked by |
|---|---|---|---|
| SH1 | `#![no_std]`; no `extern crate`, `alloc` or `std` | zero heap, zero OS authority; VER08a would pass `alloc::vec::Vec` | NEW(VOS-owned) |
| SH2 | No `unsafe`, inline asm or foreign linkage, **and no waiver** (only the HAL may hold VER08a exceptions) | memory safety is a type fact; a waiver lets the footprint read "proved" while `unsafe` exists (VER23a section 3.3) | **today** (VER08a `CAP002`) plus a no-exception rule |
| SH3 | **No `static` items at all** in a component crate, and no `thread_local!`. `const` items are allowed if they are plain data with no interior mutability, including through wrapper types (a `const` of a `Cell`-bearing type is refused) | rule 2: no global, no interior mutability | NEW(VOS-owned) |
| SH4 | No `dyn`, function-pointer types, closures or `Fn*` bounds | an indirect call is a transfer the image proof cannot follow; VER08a G-7 | NEW(VOS-owned) |
| SH5 | No recursion: the name-resolved call graph, UFCS included, is acyclic | bounded stack and step bound; rustc's lint catches only direct recursion | NEW(VOS-owned) |
| SH6 | The only loop is `for i in 0..K`, `K` a literal or a `const` path; no `while`, `loop` or iterator adaptors | the bound must be a configuration constant (rule 7); the one loop form of the VER25a subset | NEW(VOS-owned) |
| SH7 | No `/`, `%`, `/=`, `%=`, `div_*`, `rem_*`, `checked_div`, `checked_rem` | RV32I has no divide, so it is a library routine of data-dependent length, and divide-by-zero is a trap path (VER13 section 4.3 picks the base ISA for this reason); use shifts and masks | NEW(VOS-owned); `clippy::integer_division` as a second line |
| SH8 | No path to a panic: no `unwrap`, `expect`, `panic!`, `unreachable!`, `assert!`, `debug_assert!` (a `const _: () = assert!(..)` item is compile time and allowed); indexing only by a `for` variable over `0..K` where `K` is the array's own length, else `get` | a panic is the uncontracted exit of law 2; `debug_assert!` differs between the debug host build and the release image | NEW(VOS-owned); `clippy::unwrap_used`, `expect_used`, `panic`, `indexing_slicing` |
| SH9 | Overflowing arithmetic uses `wrapping_*`, `checked_*` or `saturating_*`, never bare `+ - *`. Every non-literal shift amount of `<<` or `>>` uses `wrapping_shl/shr` or `checked_shl/shr`, or is provably below the bit width (a literal, or guarded as in the `-any` listing's `c < N` plus a `const` assert that `N` fits the type) | debug overflow panics and release wraps: two builds, two functions | `clippy::arithmetic_side_effects` denied (**today** as a lint; the CI wiring is NEW(VOS-owned)) |
| SH10 | Fixed-width types only (section 1.3) | host and image compute one function | NEW(VOS-owned) |
| SH11 | No `f32`, `f64` | no float unit on RV32I; rounding and formatting differ by host | NEW(VOS-owned) |
| SH12 | Dependencies (normal, build and target-specific) are `core` and the interface's own contract crate; dev-dependencies are a **closed allowlist** (the generated harness's `coding_adventures_sha256` and `coding_adventures_test_bootstrap`, and contract crates), not ignored (VOS03 SH12). Warrant-tooling tests of a contract therefore live in a sibling class H crate (VOS03 section 1). A third-party dependency is an error, not the `WARN` of VER08a section 3.6 | VER08a does not analyse dependencies, so the footprint would not cover them (VER23a section 3.2) | NEW(VOS-owned) policy over **today's** report |
| SH13 | `alpha` and the state builder live in a `ghost` module behind a cargo feature the image build must not enable | they exist for evidence, not the kernel; features are pinned only at manifest level (VER29 section 15, limit 10) | NEW(VOS-owned) |
| SH14 | A `State` field is touched only by its owning component; the spine passes `k.field` only to operations of that field's interface | rule 2; privacy gives it inside a crate, the spine needs the UFCS reach pass | privacy **today**; spine check NEW(VER29 slice 4) |
| SH15 | No `macro_rules!` in a component; only pure `core` macros. Also banned: `include!`, `include_str!`, `include_bytes!`, `#[path]`, `#[link_section]`, `#[no_mangle]`, `#[export_name]`, a `build.rs`, and the `links` key in `Cargo.toml` | a macro, an included file, a relocated module or a build script puts code or symbols out of a lexical analyzer's sight (VER08a gap G-6 and `CAP003`) | NEW(VOS-owned) |

Two image-level companions close the same rows at the other end, both
NEW(VOS-owned) and both inputs to `RV32SYM`: no path from any kernel entry to the
panic handler in the compiled image, and a static stack bound (no recursion, so
the maximum stack is the sum of frames along the deepest chain). The checks are
lexical, so they inherit VER08a section 8: a pass means "no occurrence the lexer
can see". SH12 and SH15 close the ways a lexical check is bypassed.

**SH16, a profile-gate rule (NEW(VOS-owned), backlog VOS-D009).** No `vos-*-any`
crate and no `ghost` feature may appear in the **resolved** dependency and
feature graph of any profile whose ship class is `image`. The gate checks the
resolution Cargo actually computes (`cargo metadata` or `cargo tree -e
features` for the image target), not the manifests, because feature unification
can enable `ghost` in a crate through an unrelated dependent. `-any` is
evidence-only (section 3.1), as are `alpha`, the builders and the oracle-write
path.

### 1.5 `alpha` is ghost code

Section 2.3 requires each implementation to export `alpha(ImplState) -> M`. It is
compiled in the evidence and monitor builds, never the shipped kernel (SH13), so
evidence is about the same *source* but not the same *binary*; the monitor
(VOS00 section 10) and later `RV32SYM` close that gap.

**A tension (open question Q3), resolved in VOS02 section 4.** Rule 2 wants `State`
fields private, but the generated adapter builds a by-value record from a `struct`
line, which needs public fields. VER04m E11's `codec` line builds through a function
and observes through a method, but is specified for `inout` state, and VER04n does
not apply (it forbids opaque values in results). Adopted: private fields, and a
`ghost`-gated public-field state image (`Image`) with forwarding wrappers that
destructure exhaustively, call the production trait once by UFCS and restructure
(new rules SH17 and SH18, VOS02 section 4.6). The Warrant document is written
against the image and does not change when a codec arrives (backlog VOS-D006,
VER backlog WX-SV).

### 1.6 Which rules bind which crate (classes)

SH1-SH16 exist so a component compiles to RV32 and can be proved on the
image. A crate that is not in an image has no such need, and the emulator
wraps audited simulators that use heap, `String` and `std::error`. So the
rules bind by class (EMU00 section 3.3 defines the emulator classes):

| Class | Crates | Rules |
|---|---|---|
| **K** (kernel) | every component, `-any`, spine and HAL-facing crate that can enter an image | SH1-SH16 in full, as above |
| **C**, **S** (emulator contract, spine) | `emu-core`, `emu-dev-*` | SH2-SH11 and SH13-SH15; no `alloc` in state or step; closed device enums, no `dyn`; empty effect certificate |
| **M** (model) | `emu-cpu-*` and the simulators they wrap | Exempt from SH1, SH10, SH12; `forbid(unsafe_code)`; no capability beyond memory |
| **P** (parser) | `emu-image` | `alloc` allowed; total, panic-free, bounds before allocation (EMU00 section 8.3) |
| **H** (host) | `emu-host` | `std` allowed; the only crate with capabilities |

*Why:* evidence over a reduced instance needs a total, bounded, by-value step
(classes C and S), while input handling and the trusted ISA models have a
different evidence route (EMU00 sections 8 and 11). The class is recorded in
the crate's README and read by the shape check (VOS-D002). Class K is the
default: a crate with no class tag is class K. SH16 is unchanged and applies
to class K and image-class profiles only: classes M, P and H never enter an
image, because the emulator runs the image rather than being part of it.

---

## 2. Contract anatomy

A contract is **one Warrant document per interface**, owned by a contract package
(`vos-ready-contract`). Elements 2.1 to 2.9 are the eleven parts of VOS00 section
4.4 (M and its operations share a subsection, as do the two tiers); 2.10 adds what
makes a contract linkable. Section 4 tabulates every element against the bar.

### 2.1 Abstract state M and the abstract operations

**Meaning.** M is the smallest mathematical model that all claims can be stated
against (for the ready set: *which priorities are ready*); the abstract operations
are total functions over M saying what each operation means, errors as values.

**Today.** A `record` for M and `model` functions for the operations. Models are
pure, total and cannot call a bound operation (VER04 section 4).

```text
  record Ready(p0: Bool, p1: Bool, p2: Bool, p3: Bool, p4: Bool, p5: Bool, p6: Bool, p7: Bool)
  model abs_insert(m: Ready, p: U8) -> (Upd, Ready) = ( ... )     -- section 5.2
```

M is **canonical**: equality is structural (VER04m section 3.5), so two abstract
states that mean the same must be equal (a set kept as an insertion-ordered list
would be wrong; eight Bools is right). M is also **small enough to enumerate** at
the reduced instance: the budget is 400,000 cases per claim (VER04h section 3;
VER04m E10).

**Evidence.** A claim over M and its models calls no subject operation. The
current generator rejects it (`extract_claim_cases` in `warrant-grammar` answers
W0702). VER04m V9 specifies a **lemma** manifest kind: run, excluded from every
subject count, **no published strength**. So everything at the level of M is a
lemma and never moves a VOS number; a contract id's strength comes from *subject*
claims about an implementation (section 2.3; for `-any`, section 3). Lemma
support is NEW(WX-S1).

**Limit.** A document is written for one instance (its record has eight fields
because `N = 8`). A family over `N` needs sequence types in models
(NEW(WX-D1/D2)) or a generator emitting one document per instance (the generated documents of
VOS02 section 2 are per instance; a template generator is later work).

### 2.2 `inv(M)`, the initial state and closure

**Meaning.** `inv` says a state is well formed; the initial state satisfies it;
closure says every operation maps `inv` states to `inv` states. Together they
make every *reachable* state satisfy `inv`; without closure the invariant is a
wish. There are two invariants: `inv(M)` over the abstract state ("no thread is
ready and blocked"), and `inv_impl(S)` over one implementation's concrete state
("every cell byte is 0 or 1"). The ready set's M is eight Bools, so its `inv` is
total and needs no claim; the linear scan of section 5.4 has a real `inv_impl`.

**Today.** `inv_impl` is a `model`; init is an `example`; closure is a conjunct
`inv_impl(post)` in every step claim, so the claim quantifies exactly the
invariant states and shows the successor is again one. Concrete states are
*built* from abstract binders (`build`), the by-value counterpart of VER04m's
reach class R2 (E6): a state set complete by construction. One reviewed fact
remains, that the builder reaches every invariant state, which `canon_alpha_build`
(section 2.3) evidences.

```text
  example init_is_the_empty_set:
    inv_impl(init()) and alpha(init()) == empty()
  -- in insert_safe, a third conjunct:   inv_impl(insert(build(...), p).1)
```

**Evidence and label.** Init: `example` (rank 0, VER29 section 6.1). Closure:
kind `full` (LP-012), rung `exhaustive` at the reduced instance. "Every reachable
state satisfies `inv`" is an inference from the two; VER04m section 7.2 defers
printing it (slice S3), so VOS prints two facts, "init evidenced, closure
evidenced", and X1 (section 4) requires both. **NEW(VOS-owned):** VER04m E8 prints
those lines for `state=explicit` operations only; a by-value operation is
`state=none` and gets none, so VOS adds the naming convention and report lines (a
small extension WX-S1 can absorb).

### 2.3 Refinement through `alpha`

**Meaning.** "Implements the contract" is a simulation. Each implementation
exports `alpha(ImplState) -> M`, and every concrete operation must, through
`alpha`, do what the abstract one does:

```text
          abs_op(m, in)
   m  ------------------->  (out, m')
   ^                          ^
   | alpha                    | alpha
   s  ------------------->  (out, s')
          impl_op(s, in)
```

**Shape A, outputs equal** (every deterministic operation, every policy-tier
operation):

```text
  forall s: Impl, i: In with inv_impl(s).
      impl_op(s, i).0        == abs_op(alpha(s), i).0           -- same output
  and alpha(impl_op(s, i).1) == abs_op(alpha(s), i).1           -- same abstract successor
  and inv_impl(impl_op(s, i).1)                                 -- closure
```

**Shape B, outputs within an allowed set** (the safety tier of `pick`). The tier
is a *relation* `safe_op(m, in, out, m')`, and the implementation need only land
inside it:

```text
  forall s: Impl, i: In with inv_impl(s).
      safe_op(alpha(s), i, impl_op(s, i).0, alpha(impl_op(s, i).1))  and  inv_impl(impl_op(s, i).1)
```

**Shape B for a destructive operation.** When the operation consumes what it returns
(a scheduler's `dispatch` chooses a thread and removes it), the abstract successor is
not a function of `m` alone: it depends on the *choice*. The relation then relates the
new state to the output, `safe_dispatch(m, o, m') = valid(m, o) and m' == after(m, o)`,
where `after(m, o)` is the abstract successor given the output `o` (the set without the
chosen thread). Refinement claims use the implementation's own output,
`safe_dispatch(alpha(s), out(s), alpha(op(s).1))`, and the canonical commuting claim
takes its successor from it too (`img(build(m)) == build(after(m, out(build(m))))`).
The toy's `pick` is a read, `m' == m`, the degenerate case. VOS10 section 3.2 is the
worked instance.

A is the case of B whose relation is `out == f(m, i) and m' == g(m, i)`; the spec
keeps both because A is what a newcomer reads at a glance and B is what `-any` is
built from. For a deterministic implementation a forward simulation is a full
refinement; a component needing a backward simulation or prophecy variable is not
in the pool.

**Shape C, `alpha` is onto.** A refinement is only as honest as `alpha`: one that
maps every state to the empty set lets an implementation that always returns
`NoneReady` "refine". The guard, required, is a claim that every abstract state
is the image of an invariant concrete state, plus `alpha` bound as a `state=none`
operation (a pure function):

```text
  claim canon_alpha_build:      -- VOS02 R5 (the canonical block); called alpha_covers_m before VOS02
    forall b0: Bool, ..., b7: Bool. alpha(build(Ready(b0, ..., b7))) == Ready(b0, ..., b7)
```

This is the VER04m V3 guard ("refinement that observes nothing"); with the step
claims it forces the implementation to simulate the whole abstract machine.

**Today.** All three shapes are ordinary by-value claims over record binders
(`game-clock.warrant` is a real by-value record step). The generator's fault
vocabulary (VER04h section 6) has none for tuple or variant results, so the
negative controls these claims need (return the input state; replace the output by
a constant variant) are NEW(WX-SV item 2, VOS02 section 9.3): `drop-state-update`,
faults inside tuple components and `const-variant:<Ctor>` (the acceptance spike of
VOS02 section 13 confirmed that no fault applies to a tuple result today).

**Evidence and label.** `full` kind, `exhaustive` rung at the reduced instance,
when every binder is complete (`Bool`, `U8`, VER29 section 6.2). They carry
**`stack: host-rustc`**: the host-compiled source refines the contract, nothing is
said of the RV32 image (section 4). `proved` needs a functional-equivalence
obligation, NEW(WX-SP1), and kernel Rust is outside the VER25a subset (no struct,
enum or match), so lowering an image is NEW(RV32SYM).

### 2.4 The safety tier and the policy tier

**Meaning.** The safety tier is the weakest statement of each operation that all
system theorems can be proved from; the policy tier strengthens it to what a
profile wants. For `pick`: safety is "returns a member or `NoneReady`, and the set
is unchanged" (every profile, every system theorem); policy is "returns the
**maximum** member, `NoneReady` only for the empty set" (hard RTOS and flight).

**Today.** Two relation models (`safe_pick`, `policy_pick`) and two claim sets per
implementation. The ordering is a contract lemma, `policy_implies_safety`, so a
policy tier cannot fall outside its safety tier.

**Evidence and label.** As section 2.3, per tier. The consequence: **an
implementation may carry one tier's evidence and not the other**, and the profile
manifest records which tier each interface needs (VOS02). The linear scan of
section 5.4 carries the safety tier only; as a policy claim its `pick` fails on 247
of the 256 states (those with two or more members), the visible content of "valid
for soft real time, invalid for hard real time". `-any` realises the safety tier
only, so system theorems cite only safety-tier ids. The convention `<op>_safe` and
`<op>_policy` makes that mechanical: VOS02 R1 and R4 (the obligations blocks).

### 2.5 Resource bounds

**Meaning.** For each operation: the maximum instructions, heap (zero) and stack
depth, as functions of the configuration constants. The contract states a
**ceiling**; an implementation declares a bound at or under it; the measured or
proved cost is at or under the declaration. *Why ceilings:* rule 8 composes timing
as a sum. An entry-level bound built from the leaves' *ceilings* stays valid when
a leaf is swapped inside its ceiling (section 5.8).

**Today.** `bound` rows (VER04 section 7), as in `game-clock.warrant`:

```text
  bound pick:
    steps <= 40       -- RV32 instructions, declared; the contract ceiling is 16 * N + 32
    heap <= 0
    depth <= 2
```

* `steps` is discharged by `BoundProver` over WIR for operations the front end can
  lower (VER04h section 7). Kernel Rust cannot be lowered, so the row is
  **declared and not live**, and a `not-live` row cannot be a resource id (VER29
  section 7.1). `heap` and `depth` are recorded `unchecked` (VER04h section 7.3);
  VOS gives `heap <= 0` a structural basis (SH1) and `depth` a static one (1.4).
* The **unit** matters (VER04 section 7: each observer names its model). VOS
  bounds are in **RV32 instructions, `uarch.*`-qualified** (rule 8); the proof
  observer is NEW(RV32SYM), the emulator measurement NEW(VOS-owned).

**Evidence and label.** Declared: none. Measured on the emulator: VER29 section
7.1 classes a bench observation as `example` even for a complete sweep, because it
is not a Warrant suite declaring its domain; lifting that is NEW(VOS-owned)
(backlog VOS-D004). `proved` or `bounded-proof(k=K)`: symbolic execution of the
compiled entry over all paths; a constant-bounded loop unrolls completely, which
needs no `Bounded(k)` caveat (VER20 section 4.2): NEW(RV32SYM).

### 2.6 Timing and flow labels

**Meaning.** Which inputs may influence how long an operation takes (and, for
noninterference, which outputs). State private to a partition is declared
`secret`, and the operation must then take the same time for every value of it.

**Today.** `labels` rows (`public`/`secret`, VER04 section 7); a `secret` row
yields a 2-safety constant-time obligation (VER20 section 2.3). Two facts shape
what VOS can say. **Vacuity (VER20 section 3):** a proof with no secret input is
rejected, so a component whose labels are all `public` makes **no** timing claim
and the report prints `timing: none claimed`, never "constant time". **Not
lowerable:** the obligation is `not-live` for kernel code, and a `not-live` timing
strength is not publishable (VER29 section 7.1), so timing contracts are withheld
until `RV32SYM` supplies the image-level self-composition (VER13 section 4.3,
deliverable 4) with the `uarch.*` assumptions named
(`uarch.cache.address-trace-suffices`, `uarch.speculation.none`,
`uarch.predictor.no-state-leak`, `uarch.shared-resources.none`).

```text
  labels pick:
    s public              -- the RTOS profile: no timing claim
  -- flight variant, for a ready set private to one partition:  s secret
  --   obliges equal instruction counts and equal addresses for every s
```

The label is part of the contract, so it also says which implementations are
acceptable: an early-exit scan could not carry `s secret`. **Evidence and label:**
none published before `RV32SYM`; an emulator sweep of equal counts over all 256
masks is an `example` (section 2.5). The pure profile has no declassification
(VER04 section 7), so a flow label never relaxes.

### 2.7 Effect footprint

**Meaning.** The component uses no capability at all: empty capability set,
`no_std`, no `unsafe`, no allocation, no dependency (SH1, SH2, SH12).

**Today.** `required_capabilities.json` absent or `capabilities: []` (pure
computation, VER08a section 2); the VER23a effect certificate, which
`generate_certificate` refuses to emit for a package the analyzer rejects and
whose checker re-verifies against the files on disk, labelled `proved` as a
**package-level lexical** claim carrying every VER08a section 8 limit (VER23a
section 3.2); and the shape check for what VER08a does not look at.

**Evidence and label.** `proved` (VER23a, lexical); the shape check records an
attestation beside the `capability-audit::*` ones (VER29 section 6.2 names
`capability-audit::rand-empty`), at no higher strength than the analyzer it
extends. **NEW(VER29, VQ5):** a capability id (`contract.<package>.capability.<category>`)
is `kind-not-yet-published` and its strength is open, so the footprint is checked
by the profile gate against each implementation's certificate and is not a linked
id yet.

### 2.8 Caller preconditions

**Meaning, and a subtlety.** Every operation is total (law 1), so no input is
undefined. A *precondition* is the predicate `pre_op(m, in)` under which the
operation's useful outcome occurs; the contract also says what happens outside
it, a defined error value with the state unchanged. The callee's obligation is
the whole function; the caller's is to show it never relies on the outside-domain
branch by accident.

**Today.**

```text
  model pre_remove(m: Ready, p: U8) -> Bool = p < 8u8 and mem(m, p)
  claim remove_outside_pre_changes_nothing:                          -- callee side, a lemma at M
    forall b0: Bool, ..., b7: Bool, p: U8. (
      not pre_remove(Ready(b0, ..., b7), p) => abs_remove(Ready(b0, ..., b7), p).1 == Ready(b0, ..., b7))
  input remove.p:
    hostile
    ensures no_panic, terminates
```

The `rejects ... with Error.CASE` form needs a `Result<_, E>`, and a by-value step
returns its error as a variant of `Out`; so rejection is a *claim* (the
refinement claim contains the `OutOfRange` branch) and the `input` row carries
`hostile` and `ensures`. **The caller side** is VER29 limit 9 and VQ12; rule 6
closes it by writing the obligation where the call is made (section 6).

### 2.9 Configuration constants

**Meaning.** The sizes a contract is instantiated at (`N`, thread counts,
priorities); a claim at `N = 8` says nothing at `N = 64` (VOS00 section 9.3).

**Today.** The Rust constant is an associated `const`. A Warrant document can
observe it only through a nullary export, pinned by an example:

```text
  op cfg_n() -> Nat
  example n_is_eight:
    cfg_n() == 8
```

**NEW(WX-SV item 4, VOS02 section 9.3):** printing. VOS00 section 9.3 requires every label to print the
configuration and VER04h manifests have no field for it. The pin makes the value
*checked* today; a report field makes it *printed*.

### 2.10 Identity, ids, and one document per implementation

A contract's ids are `contract.<package>.<claim>` for functional claims and the
three longer forms for timing, capability and resource (VER29 section 3.1). The
interface-id decision (D1) had to settle all four kinds, because a VOS contract
has bounds, a footprint and (for partitioned profiles) timing, and VER29 does not
publish the last three yet (section 13, slice 2). It is settled in
[VOS02](VOS02-interface-contracts-and-profile-linking.md): ids stay stock VER29,
published by the **implementation** (`contract.<impl-package>.<claim>`), and
VOS02 section 8 states, for each kind, what a validator checks now and what the
id would add later. Bounds and footprint are checked at the profile gate, not
linked by id, until those kinds publish.

VOS00 section 4.5 first said an implementation is a *binding* of the contract's
`.warrant`. Two facts make that impossible literally: the `.warrant` stem is the
package directory (VER29 section 3.1) and a document has no imports (VER29
section 15); and refinement claims mention the implementation's own concrete
state type. The design this spec adopts, specified in VOS02 sections 1 and 2:

* the contract package owns three kinds of text: the **contract block** (M,
  abstract operations, `inv`, tier relations, `pre_` models, lemmas), one
  **obligations block** per tier, and the ceilings and configuration names;
* each implementation package's document is **generated**: the contract block
  copied verbatim, then its hand-written **implementation block** (concrete
  records, bound operations, `alpha`, the builder, `bound` rows), then the
  obligations blocks it carries;
* the implementation block uses one **fixed vocabulary** (`Impl`, `init`, the
  interface's op names, `alpha`, `build`, `inv_impl`, `cfg_<name>`), so an
  obligation claim has identical tokens, and an identical `claim_sha256`, in every
  implementation: conformance is digest equality (VOS02 R3, R4);
* the copy is **generated and checked**: a regeneration check fails on drift, in
  the style of `--check-module` (VER23a section 2.1), and the document records the
  digests. The tool is NEW(VOS-033: `vos-link gen-doc` and
  `check-doc`, VOS02 section 10).

## 3. The most-general implementation, `vos-<iface>-any`

### 3.1 What it is for

VOS00 rule 4 gives three reasons; each is checkable here. **Cover:** a theorem
proved over `-any` holds for every safety-tier implementation, including unwritten
ones, provided `-any` exhibits *every* permitted behaviour and *nothing else*.
**Test the contract:** a theorem false over `-any` has a counterexample that is a
*permitted* behaviour, the sharpest signal that the contract is too weak
(section 3.5). **Cheap first swap:** `-any` is the first alternative
implementation of every interface (VOS-012). `-any` is **evidence-only**, not a
shippable swap: it is allowed in model and evidence profiles and never in an
`image` profile (SH16, section 1.4).

### 3.2 Construction: propose, validate, default

One recipe for every interface. `-any` carries an **oracle register** in its
state, sized to the output set (nine outputs need 4 bits, not 64). An operation
whose safety tier fixes the output (`insert`, `remove`) behaves exactly as the
model and **keeps the oracle and does not depend on it**: this is not a lexical rule, it is evidenced by the `any` block
(VOS10 section 9.3: the `insert_oracle_*` and `remove_oracle_*` claims, over every oracle value), with the kit rule CC11 requiring the
fault that attacks them (VOS04 5.6). The
oracle-write path (the harness setting `choice` before an entry) lives in the
crate's `ghost` module, never in the production API; `choice` itself is a private
field. A
nondeterministic operation **proposes** the output the oracle encodes,
**validates** it against the safety relation and **defaults** to a canonical safe
output otherwise:

```rust
// vos-ready-any/src/lib.rs  (illustrative; inside `impl ReadySet for AnyReady`)
pub struct AnyReady { mask: u8, choice: u8 }          // choice: the oracle register
const _: () = assert!(<AnyReady as ReadySet>::N <= 8);  // N fits the u8 mask
// ghost::set_oracle(s, c) is the only writer of `choice` (SH13, SH16)
fn pick(s: AnyReady) -> (Pick, AnyReady) {
    let c = s.choice;
    let ok = c < Self::N && s.mask.wrapping_shr(c as u32) & 1 != 0;  // propose Member(c); validate
    (if ok { Pick::Member(c) } else { Pick::NoneReady }, s)   // default NoneReady; oracle returned unchanged
}
```

*Why a register in the state, not an argument.* The trait signature must not
change, or the spine would change when `-any` is swapped in, and a theorem over
`-any` would not be about the spine that ships (rule 5). With the register in
`State`, `Kernel<P>` is the same code for every profile, and the system model
quantifies the oracle for free. The harness rewrites the register before every
kernel entry; a leaf called `k` times per entry gets `k` words. `alpha` drops it.
*Cost:* the oracle is part of the state space, so an 8-bit oracle multiplies a
system claim by 256 and a 4-bit one by 16 (section 5.6).

### 3.3 Soundness and coverage, and how coverage is itself evidenced

```text
  -- the fixed vocabulary of VOS02 R3; these are the `any` obligations block of VOS02 section 3.3
  claim pick_sound:        -- whatever the oracle says, the output is permitted
    forall mask: U8, c: U8. (
      safe_pick(alpha(Impl(mask, c)), pick(Impl(mask, c)).0, alpha(pick(Impl(mask, c)).1)) and
      pick(Impl(mask, c)).1.choice == c)
  claim pick_covers_members:   -- every permitted member is produced by some oracle value
    forall mask: U8, p: U8 | p < 8u8. (mem(alpha(Impl(mask, p)), p) => pick(Impl(mask, p)).0 == Member(p))
  claim pick_covers_none:      -- and so is the permitted "none"
    forall mask: U8. pick(Impl(mask, 255u8)).0 == NoneReady
```

Soundness puts the output set *inside* the safety tier; coverage makes it *all
of* it. Coverage uses a **witness**, the output's own encoding (member `p` by the
value `p`, "none" by 255), so no quantifier over the oracle is needed: a nested
`exists` over 256 oracle values inside 256 states would be a cost the budget rules
do not obviously charge (VER04h section 3 budgets binder products), and the
witness keeps every claim a plain binder product. Coverage is itself falsifiable:
an `-any` that never returned `NoneReady` would make theorems easier and nobody
would notice, so each coverage claim has a negative control, a sabotaged `-any`
that must be rejected (a variant-result fault the generator lacks: NEW(WX-SV item 2),
section 2.3). A simulation of the Rust above confirmed the equality at `N = 8`:
over all 256 states and 256 oracle values the output set is exactly the 9 outputs
the safety tier permits (the members plus `NoneReady`).

### 3.4 Why system theorems are evidenced over `-any`, and what transfers

*Argument.* If an implementation refines the safety tier (shape B), every step it
takes is permitted, and `-any` can take the same step by choosing the oracle that
encodes the output. So every behaviour of the component is a behaviour of `-any`,
and a property of all behaviours of `-any` is a property of the component.

*What transfers: single-trace properties.* Isolation, integrity, capability
confinement, bounded entries, "the running thread is in the ready set".
*What does not: hyperproperties.* Noninterference (flight and separation
profiles) relates two runs, and refinement can remove nondeterminism in a way
that introduces a leak: an implementation whose choice depends on secret state
refines an `-any` whose choice does not. So a noninterference theorem is evidenced
over the policy tier (a deterministic function, shape A), or the safety tier must
state how the choice may depend on the secret. VOS00's statement that `-any`
covers every implementation "for isolation, integrity, confinement" is true for
exactly the trace properties (backlog VOS-D007; VOS-031). *Nor liveness:* over
`-any` the scheduler may always answer `NoneReady`, so "every ready thread is
eventually picked" is false over `-any`. Progress is a policy property, evidenced
on components that carry the policy tier.

### 3.5 What a contract says when it is too weak, and too strong

**Too weak.** A system claim fails over `-any` at some `(state, oracle)`. The
report line, a diagnostic over the generator's counterexample (NEW(VOS30)), names
the claim, the counterexample and the safety row that permitted it:

```text
  system claim  site_dequeue_pre   FAILED over vos-ready-any
    at state ready={}, running=None, oracle=3
    the oracle proposes Member(3); safe_pick permits it (its membership test was deleted)
    -> the safety tier is too weak for this claim; strengthening it changes every
       implementation's obligation (VOS00 section 8: a contract change is a deliberate event)
```

Delete the `mem(m, p)` conjunct of `safe_pick` and `-any` may return a priority
that is not ready, so the dequeue precondition of section 6 fails with that very
state. The fix is a contract edit plus re-proving every implementation:
expensive on purpose, cheap to find early. **Too strong.** No real implementation
refines it: found when the first one is tried (VOS-012), as a refinement claim
failing on an `inv_impl` state for *every* implementation attempted. **Neither:** a
profile that also needs progress has a missing policy tier, not a weak contract;
its manifest requires the policy ids.

---

## 4. The X1, X2 and X3 bar applied to each contract element

VOS00 section 9.2: **X1** is `exhaustive` on a reduced configuration with closure
and init evidenced; **X2** is X1 plus property, fuzz and monitor evidence at the
shipped configuration; **X3** is `proved` on the shipped image, per instance. Cells
use only the labels of VER29 section 6.1 and the kinds of VER04m section 8.1.
Every cell inherits two reminders: **host-level evidence has `stack: host-rustc`**
(the image is covered by the monitor, `randomized`, and at X3 by `RV32SYM`), and
the printing rules of VOS00 section 9.3 (instance constants, manifest-derived
counts, "rests on `-any`").

| Element | X1 (now) | X2 (now) | X3 (later) | X3 waits on |
|---|---|---|---|---|
| Abstract state, abstract ops | lemmas run; no label | none to add | none | lemma kind NEW(WX-S1) |
| `inv`, init, closure | `exhaustive` closure conjunct plus `example` init | + `randomized` at the shipped instance (property/fuzz over host source); monitor on the image | `proved` closure of the compiled step | NEW(WX-SP1, RV32SYM), solver capacity NEW(VOS30) |
| Refinement (A, B, C) | `exhaustive` at `N = 8` | + `randomized` at shipped `N`; the monitor checks the image at every trap via `alpha` | `proved`: compiled function against the bit-blasted model | as above; arrays NEW(ARR) only for a state indexed by a runtime id (a thread table), not a bitmap |
| Safety tier | `exhaustive`; plus `-any` soundness and coverage | as above | `proved` | as above |
| Policy tier | `exhaustive` where carried | as above; monitor checks the policy per trap | `proved` | as above |
| Resource bounds | declared only (not live) | emulator measurement: `example` (VER29 section 7.1); structural zero heap | `proved` or `bounded-proof(k=K)` max path, RV32 instructions | NEW(RV32SYM), bound observer NEW(VOS-owned) |
| Timing and flow | none: withheld; vacuous if all `public` | equal-count sweep on the emulator: `example` | `proved` self-composition on the image | NEW(RV32SYM), `uarch.*`; VER13 section 4.3 deliverable 4 |
| Effect footprint | VER23a `proved` (lexical) plus shape check | same, plus image checks (no panic path, stack bound) | same, image checks as evidence | NEW(VOS-owned); id publication NEW(VER29, VQ5) |
| Caller preconditions | spine claims at a tiny instance over `-any` (section 6) | + monitor checks the precondition at every trap | `proved` over the compiled spine | NEW(VER29 slice 4, VQ12), NEW(RV32SYM) |
| Configuration | `example` pin | printed | printed | printed field NEW(WX-SV item 4) |

Read off three things. **X1 is reachable for the data-structure elements now**
and needs no new VER row beyond the lemma kind and `LP-012` (VOS00 section 9.3
already makes `LP-012` a hard prerequisite of shipping a label). **Resource,
timing and the image-level footprint wait on `RV32SYM`**; until then they are
declared, measured at `example`, or withheld, the honest reading of VOS00 section
11's "X2 near, X3 later". **Progress stays with the policy tier**: no system-level
liveness theorem reaches any bar over `-any`.

---

## 5. Worked example: a bitmap "ready set"

**Illustrative.** The toy shows every element of section 2 and the replacement
exercise of VOS00 section 8 on something small enough to enumerate; the real
scheduler contract is VOS10. Each Warrant listing was run through the
`warrant-grammar` parser and type checker as a document of its own (the contract
block plus one implementation block, with the `labels`, `input` and `bound` rows
the checker requires for every operation, shown once and then elided). The
model-only lemmas were evaluated over their whole domains, and the claims about
the three implementations were checked over their whole domains by an independent
Python simulation of the algorithms shown.

**VOS02 reconciliation.** The three implementations use the fixed vocabulary of
VOS02 R3 (`Impl`, `init`, the interface's op names, `alpha`, `build`, `inv_impl`,
`cfg_n`), not per-implementation prefixes. The listings are the **natural form**:
one tuple-returning operation per interface step. The step claims quantify over
`Impl(mask)` here and over `build(M)` in the contract package's obligations
blocks (VOS02 R5); for a canonical implementation the two are the same 256
states. VOS02 section 2.4 shows the generated document that actually ran. Today's
generator can bind no fault to a tuple result, so the run used the interim
projection form of VOS02 section 4.4; the evidence for the bitmap (10 exhaustive
claims, 328,960 cases, all passing) is in VOS02 section 13.2.

### 5.1 The interface

| Part | Statement |
|---|---|
| Abstract state M | the set of ready priorities among `N = 8`, `Ready(p0 .. p7)` |
| Operations | `insert(s, p)`, `remove(s, p)`: `Changed`, `Unchanged`, or `OutOfRange` if `p >= 8`. `pick(s)`: returns a priority, leaves the set as it is |
| `inv(M)` | total (all 256 states valid) |
| Safety tier | `pick` returns a member or `NoneReady`; the set is unchanged |
| Policy tier | `pick` returns the **maximum** member; `NoneReady` only for the empty set |
| Precondition | `pre_remove(m, p) = p < 8 and mem(m, p)`; outside it `remove` changes nothing |
| Ceilings | steps `<= 16 N + 32 = 160` per operation, heap 0, stack `<= 2` frames |
| Footprint, configuration | empty capability set, `no_std`, no `unsafe`, no dependency but the contract crate; `N = 8` |

### 5.2 The contract block

The `vos-ready-contract` document, and the first half of every implementation's.

```text
warrant vos_ready_set v1
  variant Upd = Changed | Unchanged | OutOfRange
  variant Pick = NoneReady | Member(p: U8)

  -- Abstract state M: the ready priorities. Eight Bools: structural equality is set equality.
  record Ready(p0: Bool, p1: Bool, p2: Bool, p3: Bool, p4: Bool, p5: Bool, p6: Bool, p7: Bool)
  model empty() -> Ready = Ready(false, false, false, false, false, false, false, false)
  model mem(m: Ready, i: U8) -> Bool = (
    (i == 0u8 and m.p0) or (i == 1u8 and m.p1) or (i == 2u8 and m.p2) or (i == 3u8 and m.p3) or
    (i == 4u8 and m.p4) or (i == 5u8 and m.p5) or (i == 6u8 and m.p6) or (i == 7u8 and m.p7))
  model set_at(cur: Bool, i: U8, k: U8, b: Bool) -> Bool = if i == k then b else cur
  model put(m: Ready, i: U8, b: Bool) -> Ready = (
    Ready(set_at(m.p0, i, 0u8, b), set_at(m.p1, i, 1u8, b), set_at(m.p2, i, 2u8, b), set_at(m.p3, i, 3u8, b),
          set_at(m.p4, i, 4u8, b), set_at(m.p5, i, 5u8, b), set_at(m.p6, i, 6u8, b), set_at(m.p7, i, 7u8, b)))

  -- Total abstract operations: every U8 index has a defined answer.
  model abs_insert(m: Ready, p: U8) -> (Upd, Ready) = (
    if p >= 8u8 then (OutOfRange, m)
    else (if mem(m, p) then (Unchanged, m) else (Changed, put(m, p, true))))
  model abs_remove(m: Ready, p: U8) -> (Upd, Ready) = (
    if p >= 8u8 then (OutOfRange, m)
    else (if mem(m, p) then (Changed, put(m, p, false)) else (Unchanged, m)))
  model top(m: Ready) -> Pick = (
    if m.p7 then Member(7u8) else (if m.p6 then Member(6u8) else (if m.p5 then Member(5u8) else
    (if m.p4 then Member(4u8) else (if m.p3 then Member(3u8) else (if m.p2 then Member(2u8) else
    (if m.p1 then Member(1u8) else (if m.p0 then Member(0u8) else NoneReady))))))))

  -- Caller precondition.
  model pre_remove(m: Ready, p: U8) -> Bool = p < 8u8 and mem(m, p)

  -- Safety tier: a RELATION over (state, output, new state); what system theorems may use.
  model safe_pick(m: Ready, o: Pick, m2: Ready) -> Bool = (
    m2 == m and (match o with NoneReady -> true | Member(p) -> p < 8u8 and mem(m, p)))
  -- Policy tier: a FUNCTION; what the fixed-priority profile adds.
  model policy_pick(m: Ready, o: Pick, m2: Ready) -> Bool = m2 == m and o == top(m)

  -- Lemmas (VER04m V9): no bound operation is called, so no subject evidence, no strength.
  claim policy_implies_safety:
    forall b0: Bool, b1: Bool, b2: Bool, b3: Bool, b4: Bool, b5: Bool, b6: Bool, b7: Bool. (
      policy_pick(Ready(b0, b1, b2, b3, b4, b5, b6, b7), top(Ready(b0, b1, b2, b3, b4, b5, b6, b7)), Ready(b0, b1, b2, b3, b4, b5, b6, b7)) =>
      safe_pick(Ready(b0, b1, b2, b3, b4, b5, b6, b7), top(Ready(b0, b1, b2, b3, b4, b5, b6, b7)), Ready(b0, b1, b2, b3, b4, b5, b6, b7)))
  claim remove_outside_pre_changes_nothing:
    forall b0: Bool, b1: Bool, b2: Bool, b3: Bool, b4: Bool, b5: Bool, b6: Bool, b7: Bool, p: U8. (
      not pre_remove(Ready(b0, b1, b2, b3, b4, b5, b6, b7), p) =>
      abs_remove(Ready(b0, b1, b2, b3, b4, b5, b6, b7), p).1 == Ready(b0, b1, b2, b3, b4, b5, b6, b7))
```

The two lemma claims at the end were evaluated over their whole domains (256 and
65,536 cases, no failure); a generator that rejects claims calling no operation
cannot run them yet (section 2.1).

### 5.3 Implementation 1: a bitmap with a leading-zero count

One byte, bit `i` set when priority `i` is ready; `pick` returns `7 - clz(mask)`.

```rust
// vos-ready-bitmap/src/lib.rs  (illustrative)
#![no_std]
#![forbid(unsafe_code)]
use vos_ready_contract::{Pick, ReadySet, Upd};
#[derive(Copy, Clone, PartialEq, Eq)] pub struct Bitmap { mask: u8 }   // private fields: rule 2
pub struct BitmapReady;
const _: () = assert!(BitmapReady::N == 8);          // N is the mask width; compile-time check

// Leading zeros as a three-stage binary search: branch-free, division-free, visible cost.
// u8::leading_zeros is avoided: on RV32I without bit-manipulation it may become a call into a
// compiler runtime routine outside the contract (`unbound-item`, VER29 section 8.3).
const fn clz8(x: u8) -> u8 {
    let e4 = (x >> 4 == 0) as u8;  let x = x.wrapping_shl((e4 << 2) as u32);
    let e2 = (x >> 6 == 0) as u8;  let x = x.wrapping_shl((e2 << 1) as u32);
    let e1 = (x >> 7 == 0) as u8;  let x = x.wrapping_shl(e1 as u32);
    (e4 << 2).wrapping_add(e2 << 1).wrapping_add(e1).wrapping_add((x == 0) as u8)
}
impl ReadySet for BitmapReady {
    type State = Bitmap;
    const N: u8 = 8;
    fn init() -> Bitmap { Bitmap { mask: 0 } }
    fn insert(s: Bitmap, p: u8) -> (Upd, Bitmap) {
        if p >= Self::N { return (Upd::OutOfRange, s); }
        let bit = 1u8.wrapping_shl(p as u32);
        if s.mask & bit != 0 { (Upd::Unchanged, s) } else { (Upd::Changed, Bitmap { mask: s.mask | bit }) }
    }
    fn remove(s: Bitmap, p: u8) -> (Upd, Bitmap) { /* symmetric: clear the bit */ }
    fn pick(s: Bitmap) -> (Pick, Bitmap) {
        if s.mask == 0 { (Pick::NoneReady, s) } else { (Pick::Member(7u8.wrapping_sub(clz8(s.mask))), s) }
    }
}
#[cfg(feature = "ghost")]                            // SH13: evidence build only
pub mod ghost { /* alpha: bit i -> p_i;  from_parts(mask) -> Bitmap */ }
```

Its implementation block, appended to the contract block of section 5.2:

```text
  -- Implementation 1: a bitmap; alpha reads bit i as "priority i is ready".
  -- The vocabulary is fixed for every implementation (VOS02 R3): Impl, init, the
  -- interface's op names, alpha, build, inv_impl, cfg_<name>.
  record Impl(mask: U8)
  op init() -> Impl
  op insert(s: Impl, p: U8) -> (Upd, Impl)
  op remove(s: Impl, p: U8) -> (Upd, Impl)
  op pick(s: Impl) -> (Pick, Impl)
  op alpha(s: Impl) -> Ready                  -- the implementation's ghost export
  op cfg_n() -> Nat                           -- pins the configuration constant

  -- The 256 concrete states, one per abstract state.
  model b2m(b: Bool, k: Nat) -> U8 = if b then shl(1u8, k) else 0u8
  model build(r: Ready) -> Impl = (
    Impl(bit_or(bit_or(bit_or(bit_or(bit_or(bit_or(bit_or(b2m(r.p0, 0), b2m(r.p1, 1)), b2m(r.p2, 2)), b2m(r.p3, 3)),
           b2m(r.p4, 4)), b2m(r.p5, 5)), b2m(r.p6, 6)), b2m(r.p7, 7))))
  model inv_impl(s: Impl) -> Bool = true      -- every one of the 256 masks is reachable

  example n_is_eight:
    cfg_n() == 8
  example init_is_the_empty_set:
    alpha(init()) == empty()

  -- alpha is onto M (shape C); the canonical block of VOS02 R5.
  claim canon_alpha_build:   -- was alpha_covers_m (VOS02 R5)
    forall b0: Bool, b1: Bool, b2: Bool, b3: Bool, b4: Bool, b5: Bool, b6: Bool, b7: Bool. (
      alpha(build(Ready(b0, b1, b2, b3, b4, b5, b6, b7))) == Ready(b0, b1, b2, b3, b4, b5, b6, b7))

  -- Shape A: insert and remove are deterministic in both tiers.
  claim insert_safe:
    forall mask: U8, p: U8. (
      insert(Impl(mask), p).0 == abs_insert(alpha(Impl(mask)), p).0 and
      alpha(insert(Impl(mask), p).1) == abs_insert(alpha(Impl(mask)), p).1)
  claim remove_safe:
    forall mask: U8, p: U8. (
      remove(Impl(mask), p).0 == abs_remove(alpha(Impl(mask)), p).0 and
      alpha(remove(Impl(mask), p).1) == abs_remove(alpha(Impl(mask)), p).1)

  -- Shape B (safety tier) and shape A (policy tier) for pick.
  claim pick_safe:
    forall mask: U8. (
      safe_pick(alpha(Impl(mask)), pick(Impl(mask)).0, alpha(pick(Impl(mask)).1)))
  claim pick_policy:
    forall mask: U8. (
      policy_pick(alpha(Impl(mask)), pick(Impl(mask)).0, alpha(pick(Impl(mask)).1)))

  -- ... then the assume, labels, input and bound rows of every operation
```

### 5.4 Implementation 2: a linear scan, safety tier only

One byte per priority (`[u8; 8]`, each 0 or 1); `pick` scans from 0 upward and
returns the **lowest** member. Simple, bounded by `N`, and **not** the policy tier.

```rust
// vos-ready-linear/src/lib.rs  (illustrative; insert and remove are as for the bitmap)
pub struct Linear { cells: [u8; 8] }                 // inv: every byte is 0 or 1
const LEN: usize = 8;                                // == ReadySet::N, checked at compile time
fn pick(s: Linear) -> (Pick, Linear) {
    let (mut out, mut found) = (Pick::NoneReady, false);
    for i in 0..LEN {                                // SH6: constant bound, the array's own length
        if !found && s.cells[i] != 0 { out = Pick::Member(i as u8); found = true; }
    }
    (out, s)
}
```

Its implementation block (`remove_safe` is `insert_safe` with `abs_remove`, and is
elided; the names are the fixed vocabulary of VOS02 R3, not `lin_`-prefixed). The 256 invariant states are *built* from eight Bool
binders (`cells_of`), because the generator's own domain for an eight-byte array
is a sample of 28 strings, not the 256 states that matter (VER04h section 3: a
`Bytes<N>`, `N >= 4`, binder is `4 + 3N` strings, kind `sampled`). The array is
bound as the WX-N `bytes<8>` host type (VER04j section 14.4); an array of `bool`
would need `ARR`.

```text
  -- Implementation 2: a linear scan returning the LOWEST ready priority (safety tier only).
  record Impl(cells: Bytes<8>)
  op init() -> Impl
  op insert(s: Impl, p: U8) -> (Upd, Impl)
  op remove(s: Impl, p: U8) -> (Upd, Impl)
  op pick(s: Impl) -> (Pick, Impl)
  op alpha(s: Impl) -> Ready
  op cfg_n() -> Nat

  model b2u(b: Bool) -> U8 = if b then 1u8 else 0u8
  -- The 256 invariant states, built from eight Bool binders.
  model cells_of(r: Ready) -> Bytes<8> = (
    match fixed<8>(concat(concat(concat(concat(concat(concat(concat(
        be_bytes(b2u(r.p0)), be_bytes(b2u(r.p1))), be_bytes(b2u(r.p2))), be_bytes(b2u(r.p3))),
        be_bytes(b2u(r.p4))), be_bytes(b2u(r.p5))), be_bytes(b2u(r.p6))), be_bytes(b2u(r.p7))))
    with Some(b) -> b | None -> b"0000000000000000")
  model build(r: Ready) -> Impl = Impl(cells_of(r))
  -- inv_impl: every byte is 0 or 1.
  model inv_impl(l: Impl) -> Bool = bit_and(from_be<U64>(l.cells), 18374403900871474942u64) == 0u64

  example n_is_eight:
    cfg_n() == 8
  example init_is_the_empty_set:
    inv_impl(init()) and alpha(init()) == empty()

  claim canon_alpha_build:
    forall b0: Bool, b1: Bool, b2: Bool, b3: Bool, b4: Bool, b5: Bool, b6: Bool, b7: Bool. (
      alpha(build(Ready(b0, b1, b2, b3, b4, b5, b6, b7))) == Ready(b0, b1, b2, b3, b4, b5, b6, b7))
  -- Each step claim carries the closure conjunct inv_impl(post).
  claim insert_safe:
    forall b0: Bool, b1: Bool, b2: Bool, b3: Bool, b4: Bool, b5: Bool, b6: Bool, b7: Bool, p: U8. (
      insert(build(Ready(b0, b1, b2, b3, b4, b5, b6, b7)), p).0 == abs_insert(Ready(b0, b1, b2, b3, b4, b5, b6, b7), p).0 and
      alpha(insert(build(Ready(b0, b1, b2, b3, b4, b5, b6, b7)), p).1) == abs_insert(Ready(b0, b1, b2, b3, b4, b5, b6, b7), p).1 and
      inv_impl(insert(build(Ready(b0, b1, b2, b3, b4, b5, b6, b7)), p).1))
  claim pick_safe:
    forall b0: Bool, b1: Bool, b2: Bool, b3: Bool, b4: Bool, b5: Bool, b6: Bool, b7: Bool. (
      safe_pick(alpha(build(Ready(b0, b1, b2, b3, b4, b5, b6, b7))),
                pick(build(Ready(b0, b1, b2, b3, b4, b5, b6, b7))).0,
                alpha(pick(build(Ready(b0, b1, b2, b3, b4, b5, b6, b7))).1) ) and
      inv_impl(pick(build(Ready(b0, b1, b2, b3, b4, b5, b6, b7))).1))

  -- ... then the assume, labels, input and bound rows of every operation
```

The claim it deliberately omits is `pick_policy`: written, it fails on 247 of the
256 states.

### 5.5 The most-general implementation

The bitmap plus the oracle register; construction and claims are those of
section 3 (`remove_safe` is elided like its twin); the names are the fixed vocabulary of VOS02 R3.

```text
  -- The oracle is a register in the state; every operation returns it unchanged.
  record Impl(mask: U8, choice: U8)
  op insert(s: Impl, p: U8) -> (Upd, Impl)
  op remove(s: Impl, p: U8) -> (Upd, Impl)
  op pick(s: Impl) -> (Pick, Impl)
  op alpha(s: Impl) -> Ready                -- drops the oracle register
  op init() -> Impl
  op cfg_n() -> Nat

  model build_with_oracle(r: Ready, c: U8) -> Impl = (
    Impl(bit_or(bit_or(bit_or(bit_or(bit_or(bit_or(bit_or(
      if r.p0 then 1u8 else 0u8, if r.p1 then 2u8 else 0u8), if r.p2 then 4u8 else 0u8), if r.p3 then 8u8 else 0u8),
      if r.p4 then 16u8 else 0u8), if r.p5 then 32u8 else 0u8), if r.p6 then 64u8 else 0u8), if r.p7 then 128u8 else 0u8), c))

  model build(r: Ready) -> Impl = build_with_oracle(r, 0u8)   -- VOS02 R3: oracle 0

  example n_is_eight:
    cfg_n() == 8

  -- SOUNDNESS: whatever the oracle says, the output is permitted.
  claim pick_sound:
    forall mask: U8, c: U8. (
      safe_pick(alpha(Impl(mask, c)), pick(Impl(mask, c)).0, alpha(pick(Impl(mask, c)).1)) and
      pick(Impl(mask, c)).1.choice == c)
  -- COVERAGE: every permitted output has a witness oracle (member p: p; none: 255).
  claim pick_covers_members:
    forall mask: U8, p: U8 | p < 8u8. (
      mem(alpha(Impl(mask, p)), p) => pick(Impl(mask, p)).0 == Member(p))
  claim pick_covers_none:
    forall mask: U8. pick(Impl(mask, 255u8)).0 == NoneReady
  -- insert and remove are deterministic, so -any is the model; oracle fixed at 0 (section 5.5).
  claim alpha_build_with_oracle:      -- impl-own, not an obligation: names -any's own state (VOS02 R6)
    forall b0: Bool, b1: Bool, b2: Bool, b3: Bool, b4: Bool, b5: Bool, b6: Bool, b7: Bool, c: U8. (
      alpha(build_with_oracle(Ready(b0, b1, b2, b3, b4, b5, b6, b7), c)) == Ready(b0, b1, b2, b3, b4, b5, b6, b7))
  claim insert_safe:                    -- the safety obligation, at oracle 0 (R5: build(M))
    forall mask: U8, p: U8. (
      insert(Impl(mask, 0u8), p).0 == abs_insert(alpha(Impl(mask, 0u8)), p).0 and
      alpha(insert(Impl(mask, 0u8), p).1) == abs_insert(alpha(Impl(mask, 0u8)), p).1)
  -- The oracle survives every operation.
  claim insert_keeps_the_oracle:      -- impl-own
    forall c: U8, p: U8. insert(Impl(0u8, c), p).1.choice == c

  -- ... then the assume, labels, input and bound rows of every operation
```

With state, oracle and index all exhaustive, `insert_safe` would be
`256 x 256 x 256 = 16,777,216` binder cases, 41.9 times the budget. So insert and
remove are evidenced at oracle 0, the frame fact is a separate claim, and the
independence of insert from the oracle is evidenced by the `any` block (VOS10 section 9.3), not by a lexical rule. The
budget forces a split and the split is written down.

### 5.6 The exhaustive case count at N = 8

Counted as VER04h section 3 and VER04m E10 count: the **binder product before
refinement** is what the budget bounds (400,000 per claim, 64 MiB of domain); the
**admitted** count is what the manifest reports. Every state is built from `Bool`
binders or a `U8`, so every binder is complete (VER29 section 6.2) and every claim
is kind `full`.

| Claim | Binders | Product | Admitted |
|---|---|---|---|
| **Contract lemmas** | | | |
| `policy_implies_safety` | 8 Bool | 256 | 256 |
| `remove_outside_pre_changes_nothing` | 8 Bool, `p: U8` | 65,536 | 65,536 |
| **Implementation 1 (bitmap)** | | | |
| `canon_alpha_build` | 8 Bool | 256 | 256 |
| `insert_safe`, `remove_safe` | `mask: U8`, `p: U8` | 65,536 each | 65,536 each |
| `pick_safe`, `pick_policy` | `mask: U8` | 256 each | 256 each |
| *enumerated total* (plus 2 `example`s) | | | **131,840** |
| **Implementation 2 (linear scan)** | | | |
| `canon_alpha_build`, `pick_safe` | 8 Bool | 256 each | 256 each |
| `insert_safe`, `remove_safe` (closure conjunct) | 8 Bool, `p: U8` | 65,536 each | 65,536 each |
| *enumerated total* (plus 2 `example`s) | | | **131,584** |
| **`-any`** | | | |
| `alpha_build_with_oracle` | 8 Bool, `c: U8` | 65,536 | 65,536 |
| `pick_sound` | `mask: U8`, `c: U8` | 65,536 | 65,536 |
| `pick_covers_members` | `mask: U8`, `p: U8` refined to `p < 8` | 65,536 | 2,048 |
| `pick_covers_none` | `mask: U8` | 256 | 256 |
| `insert_safe`, `remove_safe`, `insert_keeps_the_oracle` | two `U8` | 65,536 each | 65,536 each |
| *enumerated total* (plus 1 `example`) | | 393,472 | **329,984** |

The largest single claim is **65,536 binder cases, 16.4 percent of the budget**.
Where the budget ends:

| Variation | Binder product | Verdict |
|---|---|---|
| `insert` at `N = 10` (state `2^10`, `p: U8`) | `1,024 x 256 = 262,144` | inside |
| `insert` at `N = 11` | `2,048 x 256 = 524,288` | over: W0706 |
| `insert` at `N = 16` with a 4-bit index | `65,536 x 16 = 1,048,576` | over |
| `pick` alone at `N = 18` / `N = 19` | `262,144` / `524,288` | inside / over |
| any claim at `N = 64` | `2^64` | not enumerable: X2 and X3 only |
| `-any` insert with state, oracle, index all `U8` | `16,777,216` | over, 41.9 times |

So the reduced instance for the bitmap family is `N <= 10` for an operation with a
full `U8` index and `N <= 18` for an operation of the state alone; `N = 8` is
chosen because it is the eight priorities of VOS-011.

### 5.7 Bounds and footprint

The figures are **placeholders**: no measurement exists and the bound observer is
NEW(RV32SYM)/NEW(VOS-owned). They show the relation of ceiling to declaration,
not a cost.

| Operation | Ceiling (steps) | Bitmap declares | Linear declares |
|---|---|---|---|
| `insert`, `remove` | `16 N + 32 = 160` | `<= 24` | `<= 12 N + 8 = 104` |
| `pick` | `160` | `<= 40` | `<= 104` |
| heap, depth | `0`, `<= 2` | `0`, `<= 2` | `0`, `<= 2` |

Footprint of each: the VER23a `proved` certificate for an empty capability set,
shape-check results `SH1`-`SH15`, and the image checks of section 1.4. Timing is
`none claimed` (all labels `public`). The labels print the instance: for example
`vos-ready-bitmap N=8 refinement exhaustive (full, max 65,536/claim) stack
host-rustc, carries pick_policy`.

### 5.8 The swap: implementation 2 replaces implementation 1

For a profile that needs only the safety tier (soft real time), VOS00 section 8
says three things are re-proved. The precise list for this example:

**Regenerated, about implementation 2** (section 5.4, 5.6): the refinement claims
with closure conjuncts (131,584 cases), the safety claim `pick_safe`, the init
and configuration-pin `example`s, the declared bound compared with the ceiling (a
mechanical check), the effect certificate, shape check and `ghost`-feature check,
and the recorded contract-block digest.

**Regenerated mechanically for the image, not authored again** (the *image*
changes whenever any component does): the profile manifest row (the swap *is* a
manifest edit); the monitor run (`randomized`); the emulator instruction counts;
any X3 proof of the entry that calls `pick`.

**Not regenerated:** the contract document and lemmas; `vos-ready-any` and its
claims (they are about the contract); implementation 1's evidence (valid, unused);
every other component (rule 1); the spine invariant and call-site claims (over
abstract states, discharged from the contract, rules 3 and 6); every system
theorem (over `-any`, for trace properties, rule 4); the entry-level instruction
bound (built from ceilings, and implementation 2 is inside them).

**The swap fails mechanically in two cases.** A profile that requires the policy
tier (hard RTOS) rejects implementation 2 because it does not discharge
`pick_policy`. An implementation that must exceed a ceiling is a contract change,
affecting every implementation and profile (VOS00 section 8). The exercise
succeeds exactly when nothing outside the first two groups changes; a needed spine
edit is a finding about the contract (VOS00 risk 3), logged, not worked around.

### 5.9 What the toy does not show

A state indexed by a *runtime* id (a thread table), where arrays and solver
capacity matter (NEW(ARR), NEW(VOS30)); a nondeterministic state transition; a
`secret` timing label (needs `RV32SYM`); any instance above `N = 10`.

---

## 6. Spine-side obligations

The spine is the one component not replaceable under VOS00 section 8: it is where
the leaves meet, so it carries what no leaf can. Three obligations.

### 6.1 Cross-component invariants over abstract states (rule 3)

A statement that mentions two leaves ("the running thread is not in the ready
set") is stated **once, over a record of abstract states**, never over any
implementation's concrete state. The spine document defines the record, the
invariant, and an abstract step built from the leaves' abstract operations, with
each nondeterministic leaf replaced by its `-any` model (the oracle becomes an
input of the model). It is appended to the contract blocks of the leaves it uses.

```text
  -- A two-field toy kernel: the ready set and the running thread.
  record KAbs(ready: Ready, running: Option<U8>)

  -- Cross-component invariant over abstract states: the running thread is valid and not ready.
  model spine_inv(k: KAbs) -> Bool = (
    match k.running with None -> true | Some(r) -> r < 8u8 and not mem(k.ready, r))

  -- Entry "timer tick": requeue the running thread, pick, dequeue the pick, run it.
  model requeued(k: KAbs) -> Ready = (
    match k.running with None -> k.ready | Some(r) -> abs_insert(k.ready, r).1)
  -- The -any pick as a model (the oracle c is an input).
  model any_model_pick(m: Ready, c: U8) -> Pick = if c < 8u8 and mem(m, c) then Member(c) else NoneReady
  model tick(k: KAbs, c: U8) -> KAbs = (
    match any_model_pick(requeued(k), c) with
      NoneReady -> KAbs(requeued(k), None)
    | Member(p) -> KAbs(abs_remove(requeued(k), p).1, Some(p)))

  -- 16 Bool binders (set, running thread, oracle): 2^16 = 65,536 cases.
  model bits3(a: Bool, b: Bool, c: Bool) -> U8 = (if a then 4u8 else 0u8) + (if b then 2u8 else 0u8) + (if c then 1u8 else 0u8)
  model bits4(a: Bool, b: Bool, c: Bool, d: Bool) -> U8 = (if a then 8u8 else 0u8) + bits3(b, c, d)
  model kabs(b0: Bool, b1: Bool, b2: Bool, b3: Bool, b4: Bool, b5: Bool, b6: Bool, b7: Bool, s: Bool, r2: Bool, r1: Bool, r0: Bool) -> KAbs = (
    KAbs(Ready(b0, b1, b2, b3, b4, b5, b6, b7), if s then Some(bits3(r2, r1, r0)) else None))

  -- Caller-domain obligation at the dequeue call site (rule 6).
  claim site_dequeue_pre:
    (forall b0: Bool, b1: Bool, b2: Bool, b3: Bool, b4: Bool, b5: Bool, b6: Bool, b7: Bool,
            s: Bool, r2: Bool, r1: Bool, r0: Bool, c3: Bool, c2: Bool, c1: Bool, c0: Bool. (
      spine_inv(kabs(b0, b1, b2, b3, b4, b5, b6, b7, s, r2, r1, r0)) =>
      (match any_model_pick(requeued(kabs(b0, b1, b2, b3, b4, b5, b6, b7, s, r2, r1, r0)), bits4(c3, c2, c1, c0))
       with NoneReady -> true | Member(p) -> pre_remove(requeued(kabs(b0, b1, b2, b3, b4, b5, b6, b7, s, r2, r1, r0)), p))))
  -- Closure of the spine invariant under one entry, every oracle value.
  claim spine_inv_closes_under_tick:
    (forall b0: Bool, b1: Bool, b2: Bool, b3: Bool, b4: Bool, b5: Bool, b6: Bool, b7: Bool,
            s: Bool, r2: Bool, r1: Bool, r0: Bool, c3: Bool, c2: Bool, c1: Bool, c0: Bool. (
      spine_inv(kabs(b0, b1, b2, b3, b4, b5, b6, b7, s, r2, r1, r0)) =>
      spine_inv(tick(kabs(b0, b1, b2, b3, b4, b5, b6, b7, s, r2, r1, r0), bits4(c3, c2, c1, c0)))))

```

* The invariant mentions `Ready` and a thread id and nothing of `Bitmap`,
  `Linear` or `AnyReady`, so any implementation that refines the contract
  inherits it (rule 3). The closure claim quantifies every oracle value.
* **Budget.** The record is 8 + 4 + 4 Bool binders, `2^16 = 65,536` cases (49,152
  with the antecedent true; both claims were evaluated, no failure). With the
  running thread and oracle as `U8` binders it would be `2^8 x 2 x 256 x 256 =
  33,554,432`, 84 times the budget. Cross-component invariants are where the
  budget bites first, so **X1 for a spine claim is a tiny instance**, and `VOS30`
  measures solver capacity before the spine is scheduled (VOS00 risk 1).
* **These are lemmas.** A claim over `KAbs`, `tick` and `spine_inv` calls no bound
  operation (VER04m V9, section 2.1): no published strength, no subject count.
  The subject-level link is the **spine refinement claim**: the Rust `spine`
  function, bound as a by-value operation `(K, Trap) -> (K', Resume)`, refines
  `tick` through `alpha_k` (the leaves' `alpha`s field by field), evidenced over
  `-any` leaves at a tiny instance (X1) and by the monitor on the image (X2). That
  is where closure of `spine_inv` over the *code* is evidenced. A spine report
  prints the lemma and the refinement as two rows and never counts the lemma.

### 6.2 Frame and ownership (rule 2)

"The scheduler's operation changed nothing outside the scheduler's field" is true
by construction when each field has one owner and the spine passes `k.ready` only
to `<P::Ready as ReadySet>` operations (SH14). Two checks make that more than an
assertion: Rust privacy of the field, and a lexical check that every UFCS call in
the spine passes the field of its own interface, a use of VER29's reach pass
(VER29 section 8, slice 4): NEW(VER29 slice 4). Until then the spine review does
it by hand (section 7, item 16).

### 6.3 Caller-domain obligations at each call site (rule 6)

VER29 section 15 limit 9 says caller-side domains are never checked, and VQ12 asks
for a check. Rule 6 does not wait for a tool: **each spine call site carries a claim
that its arguments satisfy the contract's `pre_` model**, written in the spine's
document, named for the site, discharged from the safety tier and the spine
invariant. `site_dequeue_pre` in the block above is the form: "if the spine
invariant holds, the argument computed from the *safety-tier output of the
previous call* satisfies the next call's precondition". It cites the contract only
through `safe_pick` and `pre_remove`, so it stays true across a swap, and fails
with the counterexample of section 3.5 if `safe_pick` loses its membership test.

* **One claim per call site**, with a stable id (`site_<entry>_<step>`). A UFCS call
  in the spine source with no claim of that id is a conformance failure. The match
  is lexical: resolving the call is NEW(VER29 slice 4), the naming rule is VOS02 section 5.4.
* **The outside-domain branch is unreachable or handled.** The claim shows it
  unreachable. Where it cannot be (an input from user space) the spine handles the
  returned error value and a second claim shows the handler correct; totality
  guarantees the callee returned something defined.
* **A lemma unless the spine is bound** (section 6.1): the subject-level evidence
  that the code passes a valid argument is the spine refinement claim and the
  monitor, which check the real arguments through `alpha`.

---

## 7. Conformance checklist

A reviewer applies this to any VOS component PR (a leaf, an `-any`, a second
implementation or a spine). An item that does not apply is marked `n/a` with a
reason, never skipped (VER04m V5: a check that did not run is not counted as
passed).

**Shape**

1. `no_std`; no `unsafe`, inline assembly or foreign linkage; the VER08a manifest
   has **no** `banned_construct_exceptions` (SH1, SH2).
2. The shape check `SH1`-`SH15` passes with zero waivers; dependencies are `core`
   and the interface's contract crate only (SH12).
3. `State`, `In`, `Out` use fixed-width types only (SH10); every operation is
   `(State, In) -> (Out, State)`, total, panic-free (SH8), division-free (SH7),
   with constant loop bounds (SH6).
4. `State` is owned by one field of `K` with private fields (SH14), or the
   exception of Q3 is invoked and recorded; `alpha` and builders live behind
   `ghost`, which the image build does not enable (SH13).

**Contract**

5. The contract document has, in order: abstract state and operations, `inv`,
   `pre_` models, the safety relation, the policy relation if any, the ordering
   lemma, the configuration (section 2). An implementation document contains the
   contract block **verbatim**, its digest matches, the regeneration check passes
   (section 2.10).
6. `init` is an `example`; closure is a conjunct of every step claim (or a named
   claim); the report prints `init` and `closure` (section 2.2).
7. Refinement claims use shape A or B; a `canon_alpha_build` claim (the canonical block, VOS02 R5) exists;
   `alpha` is bound `state=none` (section 2.3).
8. Claims are named `<op>_safe` / `<op>_policy`; the implementation states which
   tiers it carries; the profile manifest requires only those (section 2.4).
9. `-any` only: soundness, coverage with a witness and a negative control, an
   oracle sized to the output set, no oracle read by a deterministic operation.

**Evidence**

10. Every binder is a complete type, states are built from `Bool`/`U8` binders,
    the largest claim is under the 400,000 budget and tabulated (section 5.6);
    anything sampled is `sampled` or `bounded-set`, never `exhaustive`.
11. The report prints the instance constants, per-kind counts, `stack: host-rustc`
    and `rests on -any`; no count is typed by hand.
12. Bounds are at or under the contract ceilings, in RV32 instructions, heap 0,
    with a stack depth; a measured value is `example` (section 2.5).
13. Timing is a `secret` label with a proof, or `timing: none claimed`; an
    all-`public` label set is never called constant time (section 2.6).
14. The VER23a certificate is `proved` and current (`--check-module` clean).
15. The PR states the bar it reaches (X1, X2 or X3) and the instance; a claim above
    X1 shows the property, fuzz or monitor run it rests on (section 4).

**Spine and system**

16. Cross-component statements are over abstract states only; every spine call
    site has a `site_` claim of the same id (section 6).
17. System theorems cite only safety-tier ids; a trace property is evidenced over
    `-any`, a hyperproperty is not claimed from `-any` alone (section 3.4).
18. A replacement PR lists exactly the regenerated items of section 5.8 and shows
    none of the "not regenerated" items changed.

**Honesty**

19. Every NEW row the PR leans on is named with the row that would close it; no
    label is printed that a missing row would be needed to earn.
20. Any defect found along the way is fixed or logged under "Discovered work".
21. No `vos-*-any` crate and no `ghost` feature is in the resolved graph of an
    `image` profile (SH16), checked on the resolved features, not the manifests;
    the oracle-write path is in `ghost` only.

---

## 8. Open questions

Each has a recommended answer, which this document assumes until decided.

| # | Question | Recommendation |
|---|---|---|
| Q1 | The contract document is per instance (section 2.1): parameterise by `N` or generate one per instance? | Generate one per instance (VOS02 section 2; a template generator is later work) until sequence types exist (NEW(WX-D1/D2)); a parameterised document would hide that a proof at `N = 8` says nothing at `N = 64` |
| Q2 | Is `alpha` a Rust export (ghost) or a Warrant model? | A Rust export: the abstraction function is part of the implementation's claim to refine. Its three guards are `state=none`, `canon_alpha_build` and human review; a Warrant-model `alpha` would still need an adapter from the Rust state, which is the same function |
| Q3 | State privacy (rule 2) versus the adapter, which builds a record from public fields? | Resolved in VOS02 section 4: private fields, and a `ghost`-gated public-field state image with forwarding wrappers (rules SH17 and SH18). The VER request that removes the wrappers is WX-SV (VOS02 section 9.3) |
| Q4 | Oracle as a state register (section 3.2) or an explicit parameter? | A register: the trait and spine stay identical for every profile (rule 5). The cost is an enumeration multiplier, managed by sizing the oracle |
| Q5 | Is "`NoneReady` only if the set is empty" safety or policy? | Policy. In the safety tier `-any` could not return `NoneReady` for a non-empty set and would hide a class of safe implementations. Liveness is a policy property (section 3.4) |
| Q6 | Closure as a named claim per operation or a conjunct of the refinement claim? | A conjunct, with the naming convention and report line of section 2.2; a separate claim doubles the count for no information when the builder already restricts to invariant states |
| Q7 | Unit of a bound: WIR steps or RV32 instructions? | RV32 instructions, `uarch.*`-qualified, observer named in each bound row (VER04 section 7 requires it) |
| Q8 | Noninterference and `-any`? | Not through refinement alone (section 3.4). VOS-031 decides between NI over deterministic policy-tier components and flow-labelled safety relations with a secret-dependent oracle; recommended: the first, for the first profile |
| Q9 | Contract-level lemmas have no published strength (VER04m V9); where does a contract id get strength? | From the implementation's own claims: ids stay `contract.<impl-package>.<claim>`, each implementation publishes its obligations, and conformance to the contract is digest equality of the claim text (VOS02 R1-R4). Nothing in VER29 sections 3, 5.1 or 6 changes; effective strength is the minimum over the implementations a profile uses, as VER29 section 6.3 already says. The amendment (VER29 slice 7, backlog LV-008) is only in the reach rules and the feature rule |
| Q10 | One shared contract document or a copy per implementation? | A generated, digest-checked copy per implementation (section 2.10, VOS02 R2): the toolchain ties a `.warrant` to a package and has no imports |
| Q11 | Spine call sites name the implementation (VOS00 section 4.5) or the interface through the profile? | The interface through the profile's associated type (section 1.2); the implementation is named once, in the profile crate |
| Q12 | When does a measurement count as `exhaustive`? A sweep over all 256 masks covers the whole instance | Today never: VER29 section 7.1 caps a bench observation at `example`. Ask for a bench-kind manifest entry that declares a complete domain (VOS-D004); until then print the sweep as `example` and say it was complete |

---

## 9. Findings against VOS00 and the toolchain

Found while making VOS00's anatomy precise. Each finding is logged in
[the backlog](VOS-BACKLOG.md) under "Discovered work". **F1-F9 were applied to
VOS00 in the same PR** (second commit): VOS00 now states each resolution
concisely and cites this document for the detail, so the two agree. The backlog
rows stay open where the fix is tooling, not text.

| # | Finding | Where | Backlog |
|---|---|---|---|
| F1 | VOS00 section 8 lists three items re-proved on a swap. It omits per-image artifacts (monitor run, measurements, X3 image proofs), the profile manifest and tier-eligibility check, and the role of contract ceilings in keeping the entry-level timing bound valid | 5.8 | VOS-D008 |
| F2 | VOS00 sections 4.5 and 4.7 call an implementation a "binding of" the contract's `.warrant`; a `.warrant` stem is a package directory (VER29 section 3.1) and there are no imports (section 15), so a per-implementation generated document and a drift check are needed | 2.10 | VOS-D003 |
| F3 | VOS00 section 4.7 discusses functional ids only; a VOS contract also needs resource, timing and capability ids, which VER29 does not publish yet (slice 2) | 2.10 | VOS-D003 |
| F4 | VOS00 section 9.2's X1 ("closure and init evidenced") relies on `closure:`/`init:` lines VER04m E8 prints for `state=explicit` operations only; a by-value operation is `state=none` and has none | 2.2 | VOS-D005 |
| F5 | VOS00 section 4.4's footprint ("no_std, no unsafe") and rules 2 and 7 have no checker: VER08a does not check `no_std`, classes `alloc` as pure, and knows nothing of loops, recursion, division, statics or indirect calls | 1.4 | VOS-D002 |
| F6 | VOS00 rule 4 and section 8 say `-any` covers every implementation for system theorems; true for single-trace properties, false for noninterference and liveness | 3.4 | VOS-D007 |
| F7 | Claims over models alone are rejected by the current generator (W0702) and by VER04m V9 carry no published strength; the abstract-level "closure" of VOS00 section 4.4 is a lemma, and a contract id's strength must come from subject claims | 2.1, 6.1 | VOS-D005 |
| F8 | VER29 section 7.1 caps a bench observation at `example`, so a complete emulator sweep of instruction counts cannot reach the label VOS00 section 9.2 suggests for resource evidence | 2.5 | VOS-D004 |
| F9 | Configuration constants (VOS00 section 9.3, printing rule 1) have no manifest field; the by-value private-state binding and the tuple- and variant-result faults the claims need do not exist | 1.5, 2.3, 2.9 | VOS-D006 |
