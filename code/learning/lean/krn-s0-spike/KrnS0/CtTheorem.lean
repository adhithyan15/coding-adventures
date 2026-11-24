import KrnS0.Lockstep
import KrnS0.CtEq

/-!
# KRN-S0, part 4: `lockstep_obs` and constant time of `ct_eq_equal_length`

1. `lockstep_obs`, the generic theorem of VER11a 8.2, by induction on the step
   count.
2. The label table `ctLab` for `ct_eq_equal_length` (the part VER11a calls
   "generated and untrusted").
3. The loop-body block, point by point, using the obligation lemmas.
4. Constant time for every length.
-/

namespace KrnS0

variable {lab : Lab} {p : Program}

/-! ## The generic theorem (VER11a 8.2) -/

theorem R.running_iff {s₁ s₂ : State} (hR : R lab s₁ s₂) :
    s₁.status = .running ↔ s₂.status = .running := by
  have h := hR.kind
  constructor
  · intro h1
    rw [h1] at h
    cases hs : s₂.status <;> rw [hs] at h <;> first | rfl | cases h
  · intro h2
    rw [h2] at h
    cases hs : s₁.status <;> rw [hs] at h <;> first | rfl | cases h

theorem run_stop {s : State} {n : Nat} (h : ¬ s.status = .running) : run (n + 1) p s = s := by
  cases hs : s.status <;> first | exact absurd hs h | simp only [run, hs]

/-- Lockstep, proved once.  If one step preserves `R`, then `R` relates the
observations of the two runs for every step count. -/
theorem lockstep_obs (hL : ∀ s₁ s₂, R lab s₁ s₂ → R lab (step p s₁) (step p s₂)) :
    ∀ n s₁ s₂, R lab s₁ s₂ → obs (run n p s₁) = obs (run n p s₂) := by
  intro n
  induction n with
  | zero => intro s₁ s₂ hR; exact hR.obs_eq
  | succ n ih =>
    intro s₁ s₂ hR
    by_cases h1 : s₁.status = .running
    · have h2 := hR.running_iff.mp h1
      simp only [run, h1, h2]
      exact ih _ _ (hL _ _ hR)
    · have h2 : ¬ s₂.status = .running := fun h => h1 (hR.running_iff.mpr h)
      rw [run_stop h1, run_stop h2]
      exact hR.obs_eq

/-! ## The labels of `ct_eq_equal_length` -/

/-- The label table: for each program point, the public variables, the public
variables known to point into a public-tagged allocation (`i_ptr`, var 4) and
those known to point into a secret-tagged allocation (`acc_ptr`, var 2).
Secret variables (8, 10, 11, 12, 13, 15) never appear. -/
def ctLab : Lab := fun f b i =>
  match f, b, i with
  | 0, 0, 0 => ⟨[0, 1], [], []⟩
  | 0, 0, 1 => ⟨[0, 1], [], [2]⟩
  | 0, 0, 2 => ⟨[0, 1], [], [2]⟩
  | 0, 0, 3 => ⟨[0, 1, 3], [], [2]⟩
  | 0, 0, 4 => ⟨[0, 1, 3], [4], [2]⟩
  | 0, 0, 5 => ⟨[0, 1, 3], [4], [2]⟩
  | 0, 1, 0 => ⟨[0, 1, 3], [4], [2]⟩
  | 0, 1, 1 => ⟨[0, 1, 3, 5], [4], [2]⟩
  | 0, 2, 0 => ⟨[0, 1, 3, 5], [4], [2]⟩
  | 0, 2, 1 => ⟨[0, 1, 3, 5, 7], [4], [2]⟩
  | 0, 2, 2 => ⟨[0, 1, 3, 5, 7], [4], [2]⟩
  | 0, 2, 3 => ⟨[0, 1, 3, 5, 7, 9], [4], [2]⟩
  | 0, 2, 4 => ⟨[0, 1, 3, 5, 7, 9], [4], [2]⟩
  | 0, 2, 5 => ⟨[0, 1, 3, 5, 7, 9], [4], [2]⟩
  | 0, 2, 6 => ⟨[0, 1, 3, 5, 7, 9], [4], [2]⟩
  | 0, 2, 7 => ⟨[0, 1, 3, 5, 7, 9], [4], [2]⟩
  | 0, 2, 8 => ⟨[0, 1, 3, 5, 7, 9], [4], [2]⟩
  | 0, 2, 9 => ⟨[0, 1, 3, 5, 7, 9, 14], [4], [2]⟩
  | 0, 2, 10 => ⟨[0, 1, 3, 5, 7, 9, 14], [4], [2]⟩
  | 0, 3, 0 => ⟨[0, 1, 3, 5], [4], [2]⟩
  | 0, 3, 1 => ⟨[0, 1, 3, 5], [4], [2]⟩
  | 0, 3, 2 => ⟨[0, 1, 3, 5], [4], [2]⟩
  | _, _, _ => ⟨[], [], []⟩

/-! ## The loop-body block, point by point (VER11a 8.4, block `B`)

Eleven program points, each a one-line application of the matching obligation
lemma.  The side condition is closed by `decide`: it is a statement about the
concrete label table and the concrete instruction. -/

section Body

variable {s₁ s₂ : State} {fr₁ : Frame} {rest₁ : List Frame}

/-- `fr₁` is at instruction `k` of the body block (block 2 of function 0). -/
def AtBody (fr : Frame) (k : Nat) : Prop := fr.fn = 0 ∧ fr.block = 2 ∧ fr.ip = k

theorem body_0_ptr_a (hR : R ctLab s₁ s₂) (h₁ : s₁.stack = fr₁ :: rest₁) (hp : AtBody fr₁ 0) :
    R ctLab (step ctEq s₁) (step ctEq s₂) := by
  obtain ⟨hf, hb, hi⟩ := hp
  exact lock_let (x := 7) (e := .ptrOff (.var 0) (.var 5)) hR h₁
    (by rw [hf, hb, hi]; rfl) (by rw [hf, hb, hi]; decide)

theorem body_1_load_va (hR : R ctLab s₁ s₂) (h₁ : s₁.stack = fr₁ :: rest₁) (hp : AtBody fr₁ 1) :
    R ctLab (step ctEq s₁) (step ctEq s₂) := by
  obtain ⟨hf, hb, hi⟩ := hp
  exact lock_load (x := 8) (pe := .var 7) hR h₁
    (by rw [hf, hb, hi]; rfl) (by rw [hf, hb, hi]; decide)

theorem body_2_ptr_b (hR : R ctLab s₁ s₂) (h₁ : s₁.stack = fr₁ :: rest₁) (hp : AtBody fr₁ 2) :
    R ctLab (step ctEq s₁) (step ctEq s₂) := by
  obtain ⟨hf, hb, hi⟩ := hp
  exact lock_let (x := 9) (e := .ptrOff (.var 1) (.var 5)) hR h₁
    (by rw [hf, hb, hi]; rfl) (by rw [hf, hb, hi]; decide)

theorem body_3_load_vb (hR : R ctLab s₁ s₂) (h₁ : s₁.stack = fr₁ :: rest₁) (hp : AtBody fr₁ 3) :
    R ctLab (step ctEq s₁) (step ctEq s₂) := by
  obtain ⟨hf, hb, hi⟩ := hp
  exact lock_load (x := 10) (pe := .var 9) hR h₁
    (by rw [hf, hb, hi]; rfl) (by rw [hf, hb, hi]; decide)

theorem body_4_load_acc (hR : R ctLab s₁ s₂) (h₁ : s₁.stack = fr₁ :: rest₁) (hp : AtBody fr₁ 4) :
    R ctLab (step ctEq s₁) (step ctEq s₂) := by
  obtain ⟨hf, hb, hi⟩ := hp
  exact lock_load (x := 11) (pe := .var 2) hR h₁
    (by rw [hf, hb, hi]; rfl) (by rw [hf, hb, hi]; decide)

theorem body_5_xor (hR : R ctLab s₁ s₂) (h₁ : s₁.stack = fr₁ :: rest₁) (hp : AtBody fr₁ 5) :
    R ctLab (step ctEq s₁) (step ctEq s₂) := by
  obtain ⟨hf, hb, hi⟩ := hp
  exact lock_let (x := 12) (e := .bxor (.var 8) (.var 10)) hR h₁
    (by rw [hf, hb, hi]; rfl) (by rw [hf, hb, hi]; decide)

theorem body_6_or (hR : R ctLab s₁ s₂) (h₁ : s₁.stack = fr₁ :: rest₁) (hp : AtBody fr₁ 6) :
    R ctLab (step ctEq s₁) (step ctEq s₂) := by
  obtain ⟨hf, hb, hi⟩ := hp
  exact lock_let (x := 13) (e := .bor (.var 11) (.var 12)) hR h₁
    (by rw [hf, hb, hi]; rfl) (by rw [hf, hb, hi]; decide)

theorem body_7_store_acc (hR : R ctLab s₁ s₂) (h₁ : s₁.stack = fr₁ :: rest₁)
    (hp : AtBody fr₁ 7) : R ctLab (step ctEq s₁) (step ctEq s₂) := by
  obtain ⟨hf, hb, hi⟩ := hp
  exact lock_store (pe := .var 2) (ve := .var 13) hR h₁
    (by rw [hf, hb, hi]; rfl) (by rw [hf, hb, hi]; decide)

theorem body_8_inc (hR : R ctLab s₁ s₂) (h₁ : s₁.stack = fr₁ :: rest₁) (hp : AtBody fr₁ 8) :
    R ctLab (step ctEq s₁) (step ctEq s₂) := by
  obtain ⟨hf, hb, hi⟩ := hp
  exact lock_let (x := 14) (e := .add (.var 5) (.const (.nat 1))) hR h₁
    (by rw [hf, hb, hi]; rfl) (by rw [hf, hb, hi]; decide)

theorem body_9_store_i (hR : R ctLab s₁ s₂) (h₁ : s₁.stack = fr₁ :: rest₁) (hp : AtBody fr₁ 9) :
    R ctLab (step ctEq s₁) (step ctEq s₂) := by
  obtain ⟨hf, hb, hi⟩ := hp
  exact lock_store (pe := .var 4) (ve := .var 14) hR h₁
    (by rw [hf, hb, hi]; rfl) (by rw [hf, hb, hi]; decide)

theorem body_10_jump (hR : R ctLab s₁ s₂) (h₁ : s₁.stack = fr₁ :: rest₁) (hp : AtBody fr₁ 10) :
    R ctLab (step ctEq s₁) (step ctEq s₂) := by
  obtain ⟨hf, hb, hi⟩ := hp
  exact lock_jump (b' := 1) hR h₁
    (by rw [hf, hb, hi]; rfl) (by rw [hf, hb, hi]; decide)

end Body

/-! ## Every point, and constant time for every length -/

/-- Outside the program's finite shape, `fetch` is `none`. -/
theorem fetch_none_of_bound {F B I : Nat} (hF : p.fns.length ≤ F)
    (hB : ∀ fn ∈ p.fns, fn.blocks.length ≤ B)
    (hI : ∀ fn ∈ p.fns, ∀ bl ∈ fn.blocks, bl.insts.length < I) {f b i : Nat}
    (h : F ≤ f ∨ B ≤ b ∨ I ≤ i) : p.fetch f b i = none := by
  cases hf : p.fns[f]? with
  | none => simp only [Program.fetch, hf]
  | some fn =>
    have hmem : fn ∈ p.fns := List.mem_of_getElem? hf
    have hflen : f < p.fns.length := (List.getElem?_eq_some_iff.mp hf).1
    cases hb : fn.blocks[b]? with
    | none => simp only [Program.fetch, hf, hb]
    | some bl =>
      have hbmem : bl ∈ fn.blocks := List.mem_of_getElem? hb
      have hblen : b < fn.blocks.length := (List.getElem?_eq_some_iff.mp hb).1
      have h1 := hB fn hmem
      have h2 := hI fn hmem bl hbmem
      cases hi : bl.insts[i]? with
      | some ins =>
        have := (List.getElem?_eq_some_iff.mp hi).1
        exfalso
        omega
      | none =>
        have hnl : ¬ i = bl.insts.length := by omega
        simp only [Program.fetch, hf, hb, hi, hnl, ↓reduceIte]

/-- `omega` on this goal pulls in `Classical.choice`; this version does not. -/
theorem out_of_bounds {f b i : Nat} (h : ¬ (f < 1 ∧ b < 4 ∧ i < 12)) :
    1 ≤ f ∨ 4 ≤ b ∨ 12 ≤ i := by
  by_cases h1 : f < 1
  · by_cases h2 : b < 4
    · by_cases h3 : i < 12
      · exact absurd ⟨h1, h2, h3⟩ h
      · exact Or.inr (Or.inr (Nat.le_of_not_lt h3))
    · exact Or.inr (Or.inl (Nat.le_of_not_lt h2))
  · exact Or.inl (Nat.le_of_not_lt h1)

theorem ctEq_checked : ∀ f b i, ok ctEq ctLab f b i = true := by
  intro f b i
  by_cases h : f < 1 ∧ b < 4 ∧ i < 12
  · obtain ⟨hf, hb, hi⟩ := h
    have key : ∀ f < 1, ∀ b < 4, ∀ i < 12, ok ctEq ctLab f b i = true := by decide
    exact key f hf b hb i hi
  · have hnone : ctEq.fetch f b i = none :=
      fetch_none_of_bound (F := 1) (B := 4) (I := 12) (by decide) (by decide) (by decide)
        (out_of_bounds h)
    simp only [ok, hnone]

/-- Entry state: `a` and `b` are the two slice arguments (public provenance,
offset and length; secret contents). -/
def entryState (m : Mem) (allocs : Nat) (a b : Val) : State :=
  { stack := [{ fn := 0, block := 0, ip := 0, retDst := none
                vars := fun x => if x = 0 then a else if x = 1 then b else .unit }]
    status := .running, mem := m, trace := [], steps := 0, allocs := allocs, heap := 0 }

theorem R_entry {m₁ m₂ : Mem} {allocs : Nat} {a b : Val} (hshape : SameShape m₁ m₂)
    (hagree : AgreeOutside m₁ m₂) (hfresh : ∀ q, allocs ≤ q → m₁ q = none) :
    R ctLab (entryState m₁ allocs a b) (entryState m₂ allocs a b) := by
  refine ⟨rfl, Forall2.cons ⟨rfl, rfl, rfl, rfl, fun _ _ => rfl⟩ Forall2.nil, hshape, hagree,
    rfl, rfl, rfl, rfl, hfresh, ?_, trivial⟩
  intro f hf
  have hf' : f = _ := List.mem_singleton.mp hf
  subst hf'
  constructor
  · intro y hy
    change y ∈ ([] : List Nat) at hy
    cases hy
  · intro y hy
    change y ∈ ([] : List Nat) at hy
    cases hy

/-- **Constant time of `ct_eq_equal_length`, for every length.**
Two memories that differ only in secret-tagged allocations (`SameShape`,
`AgreeOutside`), the same slice arguments `a` and `b` (so equal provenance,
offset and *length*, whatever the length is), and fresh allocation ids: the two
runs have equal observations (outcome kind, event trace and accumulated cost)
after every number of steps. -/
theorem ct_eq_equal_length_ct {m₁ m₂ : Mem} {allocs : Nat} {a b : Val}
    (hshape : SameShape m₁ m₂) (hagree : AgreeOutside m₁ m₂)
    (hfresh : ∀ q, allocs ≤ q → m₁ q = none) :
    ∀ n, obs (run n ctEq (entryState m₁ allocs a b)) =
         obs (run n ctEq (entryState m₂ allocs a b)) :=
  fun n => lockstep_obs (fun _ _ hR => lockstep ctEq_checked hR) n _ _
    (R_entry hshape hagree hfresh)

#print axioms ct_eq_equal_length_ct

end KrnS0
