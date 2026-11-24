import KrnS0.CtEq

/-!
# KRN-S0, part 5: a concrete witness by kernel reduction

`ct_eq_equal_length` on two two-byte slices that are equal, run to `ret`.
Everything is closed by `decide`/`rfl`: the kernel decodes instructions from
the program constant, runs the cost schedule and builds the trace.
-/

namespace KrnS0

/-- Memory holding two allocations: `a` (id 0) and `b` (id 1). -/
def witnessMem (a0 a1 b0 b1 : Nat) : Mem := fun q =>
  if q = 0 then
    some { size := 2, alive := true, kind := .heap, secret := true
           cells := fun k => if k = 0 then .nat a0 else .nat a1 }
  else if q = 1 then
    some { size := 2, alive := true, kind := .heap, secret := true
           cells := fun k => if k = 0 then .nat b0 else .nat b1 }
  else none

/-- Entry state of `ct_eq_equal_length`: both slices are `(prov, 0, 2)`. -/
def initState (m : Mem) (allocs : Nat) : State :=
  { stack := [{ fn := 0, block := 0, ip := 0, retDst := none
                vars := fun x => if x = 0 then .ptr 0 0 2 else if x = 1 then .ptr 1 0 2 else .unit }]
    status := .running, mem := m, trace := [], steps := 0, allocs := allocs, heap := 0 }

/-- Equal inputs: the run reaches `ret true` after exactly 59 cost units. -/
theorem witness_equal :
    (run 100 ctEq (initState (witnessMem 5 9 5 9) 2)).status = .ret (.bool true) ∧
    (run 100 ctEq (initState (witnessMem 5 9 5 9) 2)).steps = 59 := by
  decide

/-- Different secret bytes: `ret false`, and the observation is the same. -/
theorem witness_differ :
    (run 100 ctEq (initState (witnessMem 5 9 6 9) 2)).status = .ret (.bool false) := by
  decide

theorem witness_obs_equal :
    obs (run 100 ctEq (initState (witnessMem 5 9 5 9) 2)) =
    obs (run 100 ctEq (initState (witnessMem 5 9 6 7) 2)) := by
  decide

end KrnS0
