import KrnS0.CtTheorem

/-!
# KRN-S0: sabotage controls (VER11a 12.5)

The obligations must be able to *fail*.  A wrong label table, or a program
that branches on a secret, must be refused by the per-point check.  A proof
that cannot fail would prove nothing.
-/

namespace KrnS0

/-- A wrong label table: it calls `va` (var 8, `a[i]`, a secret byte) public
right after the load. -/
def ctLabBad : Lab := fun f b i =>
  match f, b, i with
  | 0, 2, 2 => ⟨[0, 1, 3, 5, 7, 8], [4], [2]⟩
  | f, b, i => ctLab f b i

/-- Sanity: the real table passes at the load of `va`. -/
example : LoadCond ctLab 0 2 1 8 (.var 7) := by decide

/-- The wrong table fails at that load: a load through a non-public-tagged
pointer cannot yield a public value. -/
example : ¬ LoadCond ctLabBad 0 2 1 8 (.var 7) := by decide

/-- ...and so the whole-program check fails, and no `ct_eq` proof exists for it. -/
example : ok ctEq ctLabBad 0 2 1 = false := by decide

/-- A branch on the secret byte `va` (the early-exit comparison) is refused. -/
example : ¬ BranchCond ctLab 0 2 2 (.var 8) 3 3 := by decide

/-- Storing a secret through the public-tagged slot is refused. -/
example : ¬ StoreCond ctLab 0 2 7 (.var 4) (.var 13) := by decide

end KrnS0
