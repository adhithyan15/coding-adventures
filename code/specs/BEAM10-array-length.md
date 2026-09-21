# BEAM10 — `array_len` on BEAM: recovering an extent the substrate does not store

**Status:** proposed
**Backlog item:** VM-070 (see `LANG-VM-NON-ALGOL-BACKLOG.md`)
**Depends on:** BEAM04 (float arrays on `:ets`), BEAM06 (string arrays on `:ets`)
**Package:** `code/packages/rust/iir-to-beam`

---

## 1. Why this exists

`iir-to-beam` implements `alloc_array`, `array_get` and `array_set`. It does not
implement `array_len` at all — the op reaches the lowering and falls through to
`UnsupportedOp`.

VM-064 measured what that costs. Of 289 ALGOL 60 corpus programs, 252 already
lower to BEAM; of the 37 that refuse, **31 refuse for this one missing op**.
It is the single largest remaining obstacle between ALGOL and BEAM, and it is
not an ALGOL problem: `array_len` is implemented by `iir-to-llvm`,
`iir-to-wasm`, `iir-to-cil-bytecode`, `iir-to-jvm-class-file` and `vm-core`.
BEAM is the only backend missing it. ALGOL is merely the only frontend that
currently emits it, which is why the gap stayed invisible while the non-ALGOL
BEAM track completed.

This spec is shared-platform work. It requires no ALGOL change.

## 2. Why it is not a one-line addition

Two independent complications, either of which would produce silently wrong
code if implemented by analogy with the ops around it.

### 2.1 There are two array substrates, and they answer "how long?" differently

BEAM has no linear memory, so arrays are runtime objects. BEAM04/BEAM06 chose
two different objects for two different element types:

| IIR type | substrate | allocated by | stores a declared length? |
|---|---|---|---|
| `array<i64>` (and `u8`/`bool`) | `:atomics` | `atomics:new(N, [])` | **yes** — fixed size `N` |
| `array<f64>`, `array<str>` | `:ets` | `ets:new(farray, [])` | **no** — grows on insert |

That split was right for `alloc_array`/`array_get`/`array_set` (BEAM04 records
that the bit-syntax alternative needed an entire new operand-encoding
subsystem, while `:ets` needed zero new BEAM opcodes). But it means the obvious
implementation of `array_len` is wrong on one of the two:

`ets:info(Tab, size)` returns **the number of inserted entries, not the
declared length.** An ALGOL `array A[1:10]` with three elements written would
report `3`. That is worse than the current `UnsupportedOp`: a refusal is
visible, whereas `3` is plausible, is silently wrong, and is wrong on exactly
the substrate ALGOL `real` arrays use.

### 2.2 `array_len`'s `type_hint` is its *result* type, so it cannot dispatch like its neighbours

`array_get`/`array_set` dispatch between substrates on `instr.type_hint`,
because for those ops the hint is the **element** type (`"f64"`, `"str"`).
`array_len` does not work that way. Its hint is `"i64"` — the type of the
length it produces:

```rust
// algol-iir-compiler/src/lib.rs:1335 and :3730
IIRInstr::new("array_len", Some(length), vec![Operand::Var(slot)], "i64")
```

`aarch64-backend`'s fixture agrees (`heap("array_len", …, "i64")`), so this is
the established convention across the pipeline and not an ALGOL quirk. Copying
the `array_get` dispatch would read `"i64"`, conclude "not f64/str, therefore
`:atomics`", and emit `atomics:info/1` against an `:ets` table identifier.

Both `atomics:new/2` and `ets:new/2` return *references* in OTP 21+, so
`is_reference/1` cannot tell them apart at runtime either. The substrate has to
be recovered some other way.

### 2.3 All of the above confirmed on real `erl`, not reasoned about

OTP 27 on this machine, because every claim in 2.1 and 2.2 is a claim about
runtime behaviour and each one, if wrong, would invalidate the design:

```
atomics:info/1          = #{max => 9223372036854775807,memory => 120,
                            min => -9223372036854775808,size => 10}
maps:get(size, Info)    = 10
size after 1 of 10 put  = 10
ets:info(T,size) w/ 3/10= 3   (declared length was 10)
lookup_element atom key = 10
integer key still works = 2.0
ets:info(T,size) now    = 4   (length entry inflates it)
is_reference(atomics)   = true
is_reference(ets tid)   = true
atomics:info(ets tid)   = {error,badarg}
ets:info(atomics, size) = {error,badarg}
```

Reading down: `atomics:info/1`'s `size` is the **declared** length and stays
`10` after a write, so it is the right source (line 3 is the control — a
size that tracked writes would have read `1`). `ets:info/2`'s is **not**: three
writes into a ten-element array report `3`, exactly the silently-plausible
wrong answer 2.1 predicts. An atom key coexists with integer keys and reads
back, while integer indices keep working. `is_reference/1` is `true` for both
substrates, so runtime dispatch is genuinely impossible and section 3 is
necessary rather than merely convenient. And each substrate's info call raises
`badarg` on the other's handle, so a dispatch mistake fails at runtime rather
than returning a wrong number.

One consequence worth recording, since it is a change to observable state:
**the reserved entry inflates `ets:info(Tab, size)` by one** (`3` to `4`
above). Nothing in this backend calls `ets:info/2` — that is why 2.1 is a
problem rather than a bug — but anything added later that does must subtract
the length entry.

## 3. Design: a per-function substrate map

The defining instruction always carries the array type even though the *use*
does not. `alloc_array` is typed `array<f64>`; so is the `global_load` that
reads a global array back; function parameters carry their declared type in
`IIRFunction::params`. So the substrate is recoverable by a single forward
walk, which the lowering already performs:

1. Seed a `handle -> substrate` map from `f.params`, for every parameter whose
   declared type is an array type.
2. Walking instructions in order, after lowering each instruction, if it has a
   `dest` and its `type_hint` is an array type, record the substrate for that
   dest.
3. On `array_len`, look the source handle up in the map.
4. **If the handle is not in the map, refuse** with `UnsupportedOp` naming the
   handle and the reason. Guessing a substrate here is exactly the silently
   wrong outcome 2.1 rejects.

### 3.1 This rule was measured before it was written

A probe compiled every ALGOL corpus program to IIR and applied the rule above:

```
ALGOL programs emitting array_len: 31
  every array_len resolvable: 31
  at least one hole:          0
total array_len instructions: 202
resolution outcomes:
    123  resolved -> atomics
     79  resolved -> ets
```

Three things follow, and each changes a decision:

- **The rule is sufficient for the whole blocked set.** 31 of 31, with no
  handle arriving from somewhere the forward walk cannot see. The step-4
  refusal is a guard against future frontends, not a hole in this one.
- **Both substrates are required.** 79 of 202 `array_len` uses are on `:ets`.
  An `:atomics`-only implementation — the cheap option, and the one the backlog
  listed third — would have unblocked *none* of the 31 programs, because an
  ALGOL program with a `real` array typically also bounds-checks it. That
  option is dead on evidence rather than on taste.
- **A last-write-wins map is safe.** The probe also counted rebinding: a handle
  rebound to a different substrate would make the map order-dependent. There
  were zero conflicting rebinds *and zero rebinds of any kind* — IIR handles
  here are freshly-named temporaries, so a rebind conflict is impossible by
  construction, not merely unobserved. (The rebind counter is a control: with
  zero rebinds of any kind, "no conflicts" would otherwise be a claim the probe
  never had an opportunity to test.)

## 4. Lowering

### 4.1 `:atomics` — `maps:get(size, atomics:info(Ref))`

`atomics:info/1` returns a map carrying `size`, along with `max`, `min` and
`memory`. There is no `atomics:size/1`, so the map must be projected. This
backend has no map opcodes (`get_map_elements` has never been emitted), so the
projection is `maps:get/2` — an ordinary `call_ext` with an atom argument, the
same shape `math:sqrt/1` and `ets:lookup_element/3` already use. No new opcode.

```
  x0 = Ref            ; call_ext 1 atomics:info/1   -> x0 = the info map
  x1 = that map ; x0 = 'size'                       ; (staged, see 4.3)
  call_ext 2 maps:get/2                             -> x0 = N
```

`N` is exactly the IIR length: `alloc_array` passes the length straight to
`atomics:new/2`, and the 0-based-to-1-based `+1` adjustment lives in
`array_get`/`array_set`, not in the allocation. So no off-by-one correction is
needed here — and an off-by-one here would not raise, it would return a wrong
number, so this is stated rather than assumed.

### 4.2 `:ets` — a reserved atom key written at allocation

The `:ets` table cannot report an extent it was never told, so `alloc_array`
must record it. Today that path explicitly discards the length:

```rust
let _ = get_src!(instr, 0); // shape check only; size is unused
```

Instead, after `ets:new/2`, insert one extra entry pairing a reserved key with
`N`, and implement `array_len` as `ets:lookup_element(Tab, <reserved>, 2)` —
the same call `array_get` already uses, with a reserved key instead of an
integer index.

**The reserved key must be an atom, not `-1`.** A negative integer key would
also avoid collision with the `0..N-1` index space, but `BEAMOperand::i` takes
a `u64` and this backend has no way to encode a negative literal operand. An
atom key is directly expressible via the existing `BEAMOperand::a` (the
`farray` table-name atom already uses it), and cannot collide with an integer
index under any future index scheme, including one that admits negative
indices.

Cost: one extra `ets:insert/2` per array allocation, O(1). The alternative
considered — pre-populating all `N` cells at `alloc_array` — is O(n) and is
**not** adopted here; see section 5.

### 4.3 The liveness rule this backend keeps violating

Every `call_ext` clobbers x-registers, so any op emitting one **must** be
bracketed by `save_live_across_imported_call!` /
`restore_live_across_imported_call!`, and arguments must be staged through
scratch registers above `live` before being moved down into `x0..xN` (a direct
move is a parallel-move hazard: writing `x1` can clobber a source still sitting
in `x1`).

Omitting this does not fail loudly. It silently corrupts live SSA values. It
has already happened three times in this backend — VM-D029, VM-D035 and issue
#15332 — so it is called out here as a build requirement rather than left to
review. Both new ops emit `call_ext`: `array_len` emits two on the `:atomics`
path (`atomics:info/1` then `maps:get/2`) or one on the `:ets` path
(`ets:lookup_element/3`), and the `alloc_array` `:ets` path gains one
(`ets:insert/2`) inside its existing save/restore bracket.

The `:atomics` path is the delicate one: it emits **two** `call_ext`s in
sequence, and the result of the first is an argument to the second. The
intermediate map lives in `x0` across the second call's argument setup, so it
must be staged, exactly as `array_get`'s three-argument `:ets` call stages its
operands.

## 5. What this deliberately does not fix

BEAM04 records a known limitation: `ets:new` does not pre-zero cells, so
reading an `array<f64>` element that was never written traps `badarg` instead
of returning `0.0`. Pre-populating the table at `alloc_array` would fix both
that and this spec's problem at once, which is the argument for doing them
together.

It is not adopted here, but the reason originally written in this section was
**wrong**, and the correction is worth more than the conclusion.

That reason was: "nothing in the blocked set needs it, because ALGOL's own
`emit_array_value_copy` allocates and then writes every element before any
read." The first half of that sentence is true of the *destination* array and
irrelevant. `emit_array_value_copy` also **reads every element of the source**,
and an ALGOL source array may be sparsely written — `real array A[1:3]` with
only `A[1]` and `A[3]` assigned has nothing at flat index 1. Passing such an
array by value therefore reads a cell that was never written, which on `:ets`
raises `badarg`/`badkey` rather than yielding `0.0`.

This was not caught by reading the code. It was caught by running the corpus on
real `erl`: six ALGOL programs trap in exactly this way, every one of them a
call-by-value pass of a sparse multi-dimensional `real` or `string` array. A
claim about what a corpus does needs the corpus to be run.

The measured position, before and after this spec's change (292 ALGOL rows,
compiled to `.beam` and executed on OTP 27, checked against each row's own
declared `Expect`):

| | before | after |
|---|---|---|
| run correctly on real `erl` | 254 | **277** |
| refused to compile (`array_len`) | 31 | 0 |
| trapped at runtime | 1 | 9 |
| unchanged either way [^1] | 6 | 6 |

[^1]: 4 rows whose function signatures hit `UnsupportedType`, 1 other compile
refusal, and 1 program that both prints and returns a value, which the
measuring harness scores as a mismatch. Listed so the columns sum to 292.

The accounting closes exactly: of the 31 programs that previously could not
compile, 23 now run correctly and 8 now trap. **No program that passed before
changed behaviour.** Of the 8, six are the pre-zero gap above; the other two
read `own` storage that was never initialised, which is a different gap in the
same family and is not an array problem at all.

So the honest summary of what this spec ships is: +23 correct programs, and 8
programs moved from a compile-time refusal to a runtime trap. That is not a
regression in any declared behaviour — no coverage-matrix row claims `Beam` for
ALGOL, so neither state is visible today — and a loud trap is a better failure
than a silently wrong length. But it is a real change in failure mode and is
recorded as one rather than rounded off.

Pre-population stays out of *this* spec because it is BEAM04's gap, not this
one's, and because it deserves its own design: the O(n) cost can be paid
entirely in `call_ext`s with no emitted loop — `lists:seq/2`,
`lists:duplicate/2`, `lists:zip/2`, then a single `ets:insert/2` of the whole
list — which is a different shape of change from anything here. It is logged
as its own backlog item with this evidence attached. The reserved length key
remains correct alongside it; the two are independent.

## 6. Tests

Each of these must be able to fail for the reason it names. Assertions are on
the emitted instruction sequence, not on the absence of an error.

1. `array_len` on an `array<i64>` emits `atomics:info/1` followed by
   `maps:get/2`, in that order.
2. `array_len` on an `array<f64>` emits `ets:lookup_element/3` and **not**
   `atomics:info/1` — the negative half is the point, since 2.2's naive
   dispatch would emit `atomics:info/1` here and produce a `badarg` at runtime.
3. Same for `array<str>`, which shares the `:ets` substrate.
4. `alloc_array` on the `:ets` path emits an `ets:insert/2` carrying the length
   operand — pinning that the length source stopped being discarded.
5. `array_len` on a handle arriving as a **function parameter** resolves from
   `IIRFunction::params` (the seed in section 3 step 1), not from a defining
   instruction.
6. `array_len` on a handle arriving via `global_load` resolves from that
   instruction's `type_hint`.
7. `array_len` on an unknown handle refuses with `UnsupportedOp` naming the
   handle — the step-4 guard, tested positively rather than assumed.
8. `array_len` appears in the list of ops that emit a `call_ext`, asserted
   against that list directly.

   **This test replaces the one originally planned here**, which was "every
   new `call_ext` sits inside a save/restore bracket, asserted structurally
   against the emitted sequence". That test would have passed while the code
   was broken. The bracket *was* present and correct; what was empty was the
   set of variables it had to save, because `array_len` was missing from the
   op list in 4.3. Every one of the end-to-end programs below died with
   `{badarith,[{erlang,'-',[1,[]]}]}` until it was added — the fourth time
   this backend has been bitten by this class, and the first time despite a
   spec section written specifically to prevent it.

   The lesson generalises past this op: a structural assertion about the shape
   of emitted code cannot see a data-flow fact the shape depends on. Asserting
   membership in the list is the assertion that can actually fail.
9. End-to-end **on real `erl`**, not merely lowering: ALGOL programs on each
   substrate and both together, each with a hand-computed expected value so a
   wrong length shows up as a wrong number rather than only as a crash.

   At least one case must pass an array **by value** (`value a; integer array
   a`). That is the only construct in which `array_len`'s value is *observed*
   rather than compared — it becomes the copy count in
   `emit_array_value_copy`. Every other case merely feeds a bounds check,
   which a too-LARGE length sails straight through. Without a by-value case
   the suite is insensitive in one direction, which is how it would pass while
   `array_len` over-reported.

## 7. Open

Execution on real `erl` is a separate promotion step, as it has been for every
BEAM feature (BEAM01). Lowering successfully is not evidence of running
correctly, and no coverage-matrix row should be promoted to declare `Beam` on
the strength of this spec alone. VM-064's 252-program figure is likewise a
"worth attempting" number, not a cell count.

## Appendix A — the verification script

Reproduce 2.3 with `erlc vm070check.erl && erl -noshell -s vm070check main`.
Kept here rather than as a checked-in `.erl`, because the repo has no Erlang
package and a lone source file would be an artifact nothing builds or owns.

```erlang
-module(vm070check).
-export([main/0]).
main() ->
    %% 1. atomics:info/1 shape, and whether size is the DECLARED length.
    A = atomics:new(10, []),
    Info = atomics:info(A),
    io:format("atomics:info/1          = ~p~n", [Info]),
    io:format("maps:get(size, Info)    = ~p~n", [maps:get(size, Info)]),
    atomics:put(A, 1, 42),
    io:format("size after 1 of 10 put  = ~p~n", [maps:get(size, atomics:info(A))]),

    %% 2. The claim in BEAM10 2.1: ets:info(Tab,size) counts ENTRIES.
    T = ets:new(farray, []),
    ets:insert(T, {0, 1.0}), ets:insert(T, {1, 2.0}), ets:insert(T, {2, 3.0}),
    io:format("ets:info(T,size) w/ 3/10= ~p   (declared length was 10)~n",
              [ets:info(T, size)]),

    %% 3. An ATOM key alongside integer keys, and reading it back.
    ets:insert(T, {'$array_len', 10}),
    io:format("lookup_element atom key = ~p~n", [ets:lookup_element(T, '$array_len', 2)]),
    io:format("integer key still works = ~p~n", [ets:lookup_element(T, 1, 2)]),
    io:format("ets:info(T,size) now    = ~p   (length entry inflates it)~n",
              [ets:info(T, size)]),

    %% 4. Can is_reference/1 tell the two substrates apart?
    io:format("is_reference(atomics)   = ~p~n", [is_reference(A)]),
    io:format("is_reference(ets tid)   = ~p~n", [is_reference(T)]),

    %% 5. Cross-substrate calls must fail, which is why dispatch must be right.
    io:format("atomics:info(ets tid)   = ~p~n",
              [try atomics:info(T) catch C1:E1 -> {C1, E1} end]),
    io:format("ets:info(atomics, size) = ~p~n",
              [try ets:info(A, size) catch C2:E2 -> {C2, E2} end]),
    halt(0).
```
