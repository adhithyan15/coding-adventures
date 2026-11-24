import KrnS0.Relation

/-!
# KRN-S0, part 3: per-point obligations, on symbolic state

For every instruction and terminator kind, one lemma: *if the (decidable) side
condition on `lab` holds, then `R` is preserved by `step`*.  Each lemma is
proved once, over **symbolic** states constrained only by `R`, using the step
lemmas of `StepLemmas.lean`.  Nothing here mentions a concrete program, a
concrete length or a concrete loop index.

The side conditions are `abbrev`s over finite lists, so for a concrete program
`decide` discharges them by kernel reduction: instruction decoding and list
membership are exactly the part of the work that *does* reduce.  This is the
design change versus VER11a 9.2: the per-point proof is
`lock_load _ _ _ (of_decide_eq_true (by decide))`, not a generated tactic script.
-/

namespace KrnS0

variable {lab : Lab} {p : Program} {s₁ s₂ : State}

theorem R.ptr_top (hR : R lab s₁ s₂) {fr : Frame} {rest : List Frame}
    (h : s₁.stack = fr :: rest) : PtrOK lab s₁.mem fr :=
  hR.ptrs fr (by rw [h]; exact List.mem_cons_self)

/-- The fetch is the same on both sides, because `R` equates program points. -/
theorem fetch_same {fr₁ fr₂ : Frame} (hF : FrameR lab fr₁ fr₂) {o : Option Op}
    (hf : p.fetch fr₁.fn fr₁.block fr₁.ip = o) : p.fetch fr₂.fn fr₂.block fr₂.ip = o := by
  rw [← hF.fn, ← hF.block, ← hF.ip]; exact hf

/-! ## Conditions -/

abbrev LetCond (lab : Lab) (f b i x : Nat) (e : Expr) : Prop :=
  (∀ y ∈ (lab f b (i + 1)).pubs, y = x ∨ y ∈ (lab f b i).pubs) ∧
  (x ∈ (lab f b (i + 1)).pubs → ∀ z ∈ e.fv, z ∈ (lab f b i).pubs) ∧
  (∀ y ∈ (lab f b (i + 1)).pubPtr, y ≠ x ∧ y ∈ (lab f b i).pubPtr) ∧
  (∀ y ∈ (lab f b (i + 1)).secPtr, y ≠ x ∧ y ∈ (lab f b i).secPtr)

abbrev AllocCond (lab : Lab) (f b i x : Nat) (sz : Expr) (sec : Bool) : Prop :=
  (∀ y ∈ (lab f b (i + 1)).pubs, y = x ∨ y ∈ (lab f b i).pubs) ∧
  (∀ z ∈ sz.fv, z ∈ (lab f b i).pubs) ∧
  (∀ y ∈ (lab f b (i + 1)).pubPtr, y = x ∨ y ∈ (lab f b i).pubPtr) ∧
  (x ∈ (lab f b (i + 1)).pubPtr → sec = false) ∧
  (∀ y ∈ (lab f b (i + 1)).secPtr, y = x ∨ y ∈ (lab f b i).secPtr) ∧
  (x ∈ (lab f b (i + 1)).secPtr → sec = true)

/-- A load: the address is public; the result is public only if it is read
through a variable known to point into a public-tagged allocation. -/
abbrev LoadCond (lab : Lab) (f b i x : Nat) (pe : Expr) : Prop :=
  (∀ z ∈ pe.fv, z ∈ (lab f b i).pubs) ∧
  (∀ y ∈ (lab f b (i + 1)).pubs, y = x ∨ y ∈ (lab f b i).pubs) ∧
  (∀ y ∈ (lab f b (i + 1)).pubPtr, y ≠ x ∧ y ∈ (lab f b i).pubPtr) ∧
  (∀ y ∈ (lab f b (i + 1)).secPtr, y ≠ x ∧ y ∈ (lab f b i).secPtr) ∧
  (x ∈ (lab f b (i + 1)).pubs → ∃ q ∈ (lab f b i).pubPtr, pe = .var q)

/-- A store: the address is public; the value is public, or the store goes
through a variable known to point into a secret-tagged allocation. -/
abbrev StoreCond (lab : Lab) (f b i : Nat) (pe ve : Expr) : Prop :=
  (∀ z ∈ pe.fv, z ∈ (lab f b i).pubs) ∧
  (∀ y ∈ (lab f b (i + 1)).pubs, y ∈ (lab f b i).pubs) ∧
  (∀ y ∈ (lab f b (i + 1)).pubPtr, y ∈ (lab f b i).pubPtr) ∧
  (∀ y ∈ (lab f b (i + 1)).secPtr, y ∈ (lab f b i).secPtr) ∧
  ((∀ z ∈ ve.fv, z ∈ (lab f b i).pubs) ∨ ∃ q ∈ (lab f b i).secPtr, pe = .var q)

abbrev FreeCond (lab : Lab) (f b i : Nat) (pe : Expr) : Prop :=
  (∀ z ∈ pe.fv, z ∈ (lab f b i).pubs) ∧
  (∀ y ∈ (lab f b (i + 1)).pubs, y ∈ (lab f b i).pubs) ∧
  (∀ y ∈ (lab f b (i + 1)).pubPtr, y ∈ (lab f b i).pubPtr) ∧
  (∀ y ∈ (lab f b (i + 1)).secPtr, y ∈ (lab f b i).secPtr)

abbrev JumpCond (lab : Lab) (f b i t : Nat) : Prop :=
  (∀ y ∈ (lab f t 0).pubs, y ∈ (lab f b i).pubs) ∧
  (∀ y ∈ (lab f t 0).pubPtr, y ∈ (lab f b i).pubPtr) ∧
  (∀ y ∈ (lab f t 0).secPtr, y ∈ (lab f b i).secPtr)

abbrev BranchCond (lab : Lab) (f b i : Nat) (c : Expr) (t e : Nat) : Prop :=
  (∀ z ∈ c.fv, z ∈ (lab f b i).pubs) ∧ JumpCond lab f b i t ∧ JumpCond lab f b i e

/-! ## `let` -/

theorem lock_let {fr₁ : Frame} {rest₁ : List Frame} {x : Nat} {e : Expr}
    (hR : R lab s₁ s₂) (h₁ : s₁.stack = fr₁ :: rest₁)
    (hf : p.fetch fr₁.fn fr₁.block fr₁.ip = some (.inst (.let_ x e)))
    (hc : LetCond lab fr₁.fn fr₁.block fr₁.ip x e) : R lab (step p s₁) (step p s₂) := by
  obtain ⟨fr₂, rest₂, h₂, hF, hrest⟩ := hR.top h₁
  rw [step_let h₁ hf, step_let h₂ (fetch_same hF hf)]
  obtain ⟨c1, c2, c3, c4⟩ := hc
  refine hR.step_top h₁ hrest rfl rfl rfl
    (hF.set_next c1 (fun hxN => eval_pub (c2 hxN) hF.vars))
    hR.shape hR.agree hR.trace ?_ hR.allocs hR.heap hR.kind hR.fresh (MemExt.refl _) ?_
  · show s₁.steps + costAlu = s₂.steps + costAlu
    rw [hR.steps]
  · exact (hR.ptr_top h₁).set_next (MemExt.refl _)
      (fun y hy _ => (c3 y hy).2) (fun y hy _ => (c4 y hy).2)
      (fun h => absurd rfl (c3 _ h).1) (fun h => absurd rfl (c4 _ h).1)

/-! ## `alloc` -/

theorem lock_alloc {fr₁ : Frame} {rest₁ : List Frame} {x : Nat} {sz : Expr} {k : AllocKind}
    {sec : Bool}
    (hR : R lab s₁ s₂) (h₁ : s₁.stack = fr₁ :: rest₁)
    (hf : p.fetch fr₁.fn fr₁.block fr₁.ip = some (.inst (.alloc x sz k sec)))
    (hc : AllocCond lab fr₁.fn fr₁.block fr₁.ip x sz sec) : R lab (step p s₁) (step p s₂) := by
  obtain ⟨fr₂, rest₂, h₂, hF, hrest⟩ := hR.top h₁
  rw [step_alloc h₁ hf, step_alloc h₂ (fetch_same hF hf)]
  obtain ⟨c1, c2, c3, c4, c5, c6⟩ := hc
  have hn : (sz.eval fr₁.vars).toNat = (sz.eval fr₂.vars).toNat := by
    rw [eval_pub c2 hF.vars]
  have hid : s₂.allocs = s₁.allocs := hR.allocs.symm
  have hfr : s₁.mem s₁.allocs = none := hR.fresh _ (Nat.le_refl _)
  refine hR.step_top h₁ hrest rfl rfl rfl
    (hF.set_next c1 (fun _ => by show Val.ptr s₁.allocs 0 _ = Val.ptr s₂.allocs 0 _
                                 rw [hid, hn]))
    ?_ ?_ hR.trace ?_ ?_ ?_ hR.kind ?_ ?_ ?_
  · intro q
    show Option.map Alloc.shape (if q = s₁.allocs then _ else s₁.mem q) =
      Option.map Alloc.shape (if q = s₂.allocs then _ else s₂.mem q)
    rw [hid]
    by_cases hq : q = s₁.allocs
    · simp only [hq, ↓reduceIte, Option.map_some, Alloc.shape, hn]
    · simp only [hq, ↓reduceIte]
      exact hR.shape q
  · intro q a₁ a₂ ha₁ ha₂ hs k'
    change (if q = s₁.allocs then _ else s₁.mem q) = some a₁ at ha₁
    change (if q = s₂.allocs then _ else s₂.mem q) = some a₂ at ha₂
    rw [hid] at ha₂
    by_cases hq : q = s₁.allocs
    · simp only [hq, ↓reduceIte, Option.some.injEq] at ha₁ ha₂
      rw [← ha₁, ← ha₂]
    · simp only [hq, ↓reduceIte] at ha₁ ha₂
      exact hR.agree q a₁ a₂ ha₁ ha₂ hs k'
  · show s₁.steps + costAlloc = s₂.steps + costAlloc
    rw [hR.steps]
  · show s₁.allocs + 1 = s₂.allocs + 1
    rw [hid]
  · show s₁.heap + k.heapBytes _ = s₂.heap + k.heapBytes _
    rw [hR.heap, hn]
  · intro q hq
    have hne : q ≠ s₁.allocs := Nat.ne_of_gt hq
    show (if q = s₁.allocs then _ else s₁.mem q) = none
    simp only [hne, ↓reduceIte]
    exact hR.fresh q (Nat.le_of_succ_le hq)
  · intro q a ha
    refine ⟨a, ?_, rfl⟩
    have hne : q ≠ s₁.allocs := by
      intro hq; rw [hq, hfr] at ha; cases ha
    show (if q = s₁.allocs then _ else s₁.mem q) = some a
    simp only [hne, ↓reduceIte]
    exact ha
  · refine (hR.ptr_top h₁).set_next ?_ ?_ ?_ ?_ ?_
    · intro q a ha
      refine ⟨a, ?_, rfl⟩
      have hne : q ≠ s₁.allocs := by
        intro hq; rw [hq, hfr] at ha; cases ha
      show (if q = s₁.allocs then _ else s₁.mem q) = some a
      simp only [hne, ↓reduceIte]
      exact ha
    · intro y hy hne
      rcases c3 y hy with h | h
      · exact absurd h hne
      · exact h
    · intro y hy hne
      rcases c5 y hy with h | h
      · exact absurd h hne
      · exact h
    · intro hx
      refine ⟨s₁.allocs, 0, (sz.eval fr₁.vars).toNat,
        { size := (sz.eval fr₁.vars).toNat, alive := true, kind := k, secret := sec, cells := fun _ => .nat 0 }, rfl, ?_, c4 hx⟩
      show (if s₁.allocs = s₁.allocs then _ else _) = _
      simp only [↓reduceIte]
    · intro hx
      refine ⟨s₁.allocs, 0, (sz.eval fr₁.vars).toNat,
        { size := (sz.eval fr₁.vars).toNat, alive := true, kind := k, secret := sec, cells := fun _ => .nat 0 }, rfl, ?_, c6 hx⟩
      show (if s₁.allocs = s₁.allocs then _ else _) = _
      simp only [↓reduceIte]

/-! ## `load` -/

theorem lock_load {fr₁ : Frame} {rest₁ : List Frame} {x : Nat} {pe : Expr}
    (hR : R lab s₁ s₂) (h₁ : s₁.stack = fr₁ :: rest₁)
    (hf : p.fetch fr₁.fn fr₁.block fr₁.ip = some (.inst (.load x pe)))
    (hc : LoadCond lab fr₁.fn fr₁.block fr₁.ip x pe) : R lab (step p s₁) (step p s₂) := by
  obtain ⟨fr₂, rest₂, h₂, hF, hrest⟩ := hR.top h₁
  have hf₂ := fetch_same hF hf
  obtain ⟨c1, c2, c3, c4, c5⟩ := hc
  have he : pe.eval fr₁.vars = pe.eval fr₂.vars := eval_pub c1 hF.vars
  have hsh := access_shape hR.shape (pe.eval fr₁.vars)
  cases hA : access s₁.mem (pe.eval fr₁.vars) with
  | none =>
    have hA₂ : access s₂.mem (pe.eval fr₁.vars) = none := by
      rw [hA] at hsh
      cases hB : access s₂.mem (pe.eval fr₁.vars) with
      | none => rfl
      | some w => rw [hB] at hsh; cases hsh
    rw [step_load_stuck h₁ hf hA, step_load_stuck h₂ hf₂ (by rw [← he]; exact hA₂)]
    exact hR.stuck
  | some w =>
    obtain ⟨prov, off, a₁⟩ := w
    cases hB : access s₂.mem (pe.eval fr₁.vars) with
    | none => rw [hA, hB] at hsh; cases hsh
    | some w₂ =>
      obtain ⟨prov₂, off₂, a₂⟩ := w₂
      rw [hA, hB] at hsh
      simp only [Option.map_some, Option.some.injEq, Prod.mk.injEq] at hsh
      obtain ⟨rfl, rfl, hshape⟩ := hsh
      have hm₁ := access_mem hA
      have hm₂ := access_mem hB
      rw [step_load h₁ hf hA, step_load h₂ hf₂ (by rw [← he]; exact hB)]
      refine hR.step_top h₁ hrest rfl rfl rfl
        (hF.set_next c2 ?_) hR.shape hR.agree ?_ ?_ hR.allocs hR.heap hR.kind hR.fresh
        (MemExt.refl _) ?_
      · intro hxN
        obtain ⟨q, hq, rfl⟩ := c5 hxN
        have hsec := access_tag_pub (hR.ptr_top h₁) hq hA
        exact hR.agree prov a₁ a₂ hm₁ hm₂ hsec off
      · show s₁.trace ++ _ = s₂.trace ++ _
        rw [hR.trace]
      · show s₁.steps + costMem = s₂.steps + costMem
        rw [hR.steps]
      · exact (hR.ptr_top h₁).set_next (MemExt.refl _)
          (fun y hy _ => (c3 y hy).2) (fun y hy _ => (c4 y hy).2)
          (fun h => absurd rfl (c3 _ h).1) (fun h => absurd rfl (c4 _ h).1)

end KrnS0
