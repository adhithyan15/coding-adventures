import KrnS0.Semantics

/-!
# KRN-S0: `ct_eq_equal_length` as WIR

A hand transcription of what `warrant-ir/src/lower.rs` produces for

```rust
fn ct_eq_equal_length(a: &[u8], b: &[u8]) -> bool {
    let mut acc: u8 = 0;
    for i in 0..a.len() { volatile_barrier::accumulate(&mut acc, &a[i], &b[i]); }
    volatile_barrier::read(&acc) == 0
}
```

Variable numbering (the embedding numbers variables, the kernel never compares
strings, VER11a 6.2):

| id | name | id | name |
|----|------|----|------|
| 0 | `a` (slice, secret contents) | 8 | `va`  (a[i], secret) |
| 1 | `b` (slice, secret contents) | 9 | `pb`  (b + i) |
| 2 | `acc_ptr` (stack slot, secret) | 10 | `vb` (b[i], secret) |
| 3 | `slice_len` | 11 | `acc_curr` (secret) |
| 4 | `i_ptr` (stack slot, public) | 12 | `diff` (secret) |
| 5 | `i` | 13 | `acc_next` (secret) |
| 7 | `pa`  (a + i) | 14 | `i_next` |
|   |       | 15 | `acc_res` (secret) |

Differences from the real lowering, all recorded in REPORT.md:
the real WIR has no explicit bounds-check branches (a load that is out of
bounds is `stuck`, which is what `access` models), integers do not wrap here,
and `a.len()` is `ptrLen a`.
-/

namespace KrnS0

open Expr Inst Term

def ctEqBlocks : List Block :=
  [ -- block 0: entry
    { insts := [ alloc 2 (const (.nat 1)) .stack true,        -- acc_ptr  (secret slot)
                 store (var 2) (const (.nat 0)),              -- acc := 0
                 let_ 3 (ptrLen (var 0)),                     -- slice_len
                 alloc 4 (const (.nat 8)) .stack false,       -- i_ptr    (public slot)
                 store (var 4) (const (.nat 0)) ]             -- i := 0
      term := jump 1 }
  , -- block 1: loop header
    { insts := [ load 5 (var 4) ]                             -- i
      term := branch (lt (var 5) (var 3)) 2 3 }
  , -- block 2: loop body (three volatile loads, one store, i := i + 1)
    { insts := [ let_ 7 (ptrOff (var 0) (var 5)),             -- &a[i]
                 load 8 (var 7),                              -- va
                 let_ 9 (ptrOff (var 1) (var 5)),             -- &b[i]
                 load 10 (var 9),                             -- vb
                 load 11 (var 2),                             -- acc_curr
                 let_ 12 (bxor (var 8) (var 10)),             -- diff
                 let_ 13 (bor (var 11) (var 12)),             -- acc_next
                 store (var 2) (var 13),                      -- acc := acc_next
                 let_ 14 (add (var 5) (const (.nat 1))),      -- i_next
                 store (var 4) (var 14) ]                     -- i := i_next
      term := jump 1 }
  , -- block 3: exit
    { insts := [ free (var 4),
                 load 15 (var 2) ]                            -- acc_res
      term := ret (eq (var 15) (const (.nat 0))) } ]

/-- The program: one function. -/
def ctEq : Program := { fns := [ { blocks := ctEqBlocks } ] }

end KrnS0
