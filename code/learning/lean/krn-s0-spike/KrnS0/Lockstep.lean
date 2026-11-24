import KrnS0.Obligations

/-!
# KRN-S0, parts 3 and 4: the rest of the obligations, and the glue

Stores, frees and terminators (each proved once over symbolic states), the
per-point check `ok`, and `lockstep`: one step preserves `R` at *every* program
point provided every point passes `ok`.
-/

namespace KrnS0

variable {lab : Lab} {p : Program} {s₁ s₂ : State}

/-! ## `store` and `free` -/

theorem lock_store {fr₁ : Frame} {rest₁ : List Frame} {pe ve : Expr}
    (hR : R lab s₁ s₂) (h₁ : s₁.stack = fr₁ :: rest₁)
    (hf : p.fetch fr₁.fn fr₁.block fr₁.ip = some (.inst (.store pe ve)))
    (hc : StoreCond lab fr₁.fn fr₁.block fr₁.ip pe ve) : R lab (step p s₁) (step p s₂) := by
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
    rw [step_store_stuck h₁ hf hA, step_store_stuck h₂ hf₂ (by rw [← he]; exact hA₂)]
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
      rw [step_store h₁ hf hA, step_store h₂ hf₂ (by rw [← he]; exact hB)]
      have hext : MemExt s₁.mem (fun q => if q = prov then
          some { a₁ with cells := fun k => if k = off then ve.eval fr₁.vars else a₁.cells k }
          else s₁.mem q) := by
        intro q a ha
        by_cases hq : q = prov
        · have e : s₁.mem q = some a₁ := by rw [hq]; exact hm₁
          rw [e] at ha
          cases ha
          exact ⟨{ a₁ with cells := fun k => if k = off then ve.eval fr₁.vars else a₁.cells k },
            by simp only [hq, ↓reduceIte], rfl⟩
        · exact ⟨a, by simp only [hq, ↓reduceIte]; exact ha, rfl⟩
      refine hR.step_top h₁ hrest rfl rfl rfl (hF.next c2) ?_ ?_ ?_ ?_ hR.allocs hR.heap hR.kind
        ?_ hext ((hR.ptr_top h₁).same hext rfl c3 c4)
      · intro q
        show Option.map Alloc.shape (if q = prov then _ else s₁.mem q) =
          Option.map Alloc.shape (if q = prov then _ else s₂.mem q)
        by_cases hq : q = prov
        · simp only [hq, ↓reduceIte, Option.map_some]
          exact congrArg some hshape
        · simp only [hq, ↓reduceIte]
          exact hR.shape q
      · intro q b₁ b₂ hb₁ hb₂ hs k'
        change (if q = prov then _ else s₁.mem q) = some b₁ at hb₁
        change (if q = prov then _ else s₂.mem q) = some b₂ at hb₂
        by_cases hq : q = prov
        · simp only [hq, ↓reduceIte, Option.some.injEq] at hb₁ hb₂
          subst hb₁
          subst hb₂
          show (if k' = off then ve.eval fr₁.vars else a₁.cells k') =
            (if k' = off then ve.eval fr₂.vars else a₂.cells k')
          by_cases hk : k' = off
          · simp only [hk, ↓reduceIte]
            rcases c5 with hv | ⟨q', hq', rfl⟩
            · exact eval_pub hv hF.vars
            · have hsec := access_tag_sec (hR.ptr_top h₁) hq' hA
              have hfalse : true = false := hsec.symm.trans hs
              cases hfalse
          · simp only [hk, ↓reduceIte]
            exact hR.agree prov a₁ a₂ hm₁ hm₂ hs k'
        · simp only [hq, ↓reduceIte] at hb₁ hb₂
          exact hR.agree q b₁ b₂ hb₁ hb₂ hs k'
      · show s₁.trace ++ _ = s₂.trace ++ _
        rw [hR.trace]
      · show s₁.steps + costMem = s₂.steps + costMem
        rw [hR.steps]
      · intro q hq
        show (if q = prov then _ else s₁.mem q) = none
        by_cases hqp : q = prov
        · have h0 := hR.fresh q hq
          rw [hqp, hm₁] at h0
          cases h0
        · simp only [hqp, ↓reduceIte]
          exact hR.fresh q hq

theorem lock_free {fr₁ : Frame} {rest₁ : List Frame} {pe : Expr}
    (hR : R lab s₁ s₂) (h₁ : s₁.stack = fr₁ :: rest₁)
    (hf : p.fetch fr₁.fn fr₁.block fr₁.ip = some (.inst (.free pe)))
    (hc : FreeCond lab fr₁.fn fr₁.block fr₁.ip pe) : R lab (step p s₁) (step p s₂) := by
  obtain ⟨fr₂, rest₂, h₂, hF, hrest⟩ := hR.top h₁
  have hf₂ := fetch_same hF hf
  obtain ⟨c1, c2, c3, c4⟩ := hc
  have he : pe.eval fr₁.vars = pe.eval fr₂.vars := eval_pub c1 hF.vars
  have hsh := access_shape hR.shape (pe.eval fr₁.vars)
  cases hA : access s₁.mem (pe.eval fr₁.vars) with
  | none =>
    have hA₂ : access s₂.mem (pe.eval fr₁.vars) = none := by
      rw [hA] at hsh
      cases hB : access s₂.mem (pe.eval fr₁.vars) with
      | none => rfl
      | some w => rw [hB] at hsh; cases hsh
    rw [step_free_stuck h₁ hf hA, step_free_stuck h₂ hf₂ (by rw [← he]; exact hA₂)]
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
      rw [step_free h₁ hf hA, step_free h₂ hf₂ (by rw [← he]; exact hB)]
      have hext : MemExt s₁.mem (fun q => if q = prov then some { a₁ with alive := false }
          else s₁.mem q) := by
        intro q a ha
        by_cases hq : q = prov
        · have e : s₁.mem q = some a₁ := by rw [hq]; exact hm₁
          rw [e] at ha
          cases ha
          exact ⟨{ a₁ with alive := false }, by simp only [hq, ↓reduceIte], rfl⟩
        · exact ⟨a, by simp only [hq, ↓reduceIte]; exact ha, rfl⟩
      refine hR.step_top h₁ hrest rfl rfl rfl (hF.next c2) ?_ ?_ hR.trace ?_ hR.allocs hR.heap
        hR.kind ?_ hext ((hR.ptr_top h₁).same hext rfl c3 c4)
      · intro q
        show Option.map Alloc.shape (if q = prov then _ else s₁.mem q) =
          Option.map Alloc.shape (if q = prov then _ else s₂.mem q)
        by_cases hq : q = prov
        · simp only [hq, ↓reduceIte, Option.map_some]
          have hk : (a₁.size, a₁.alive, a₁.kind, a₁.secret) =
              (a₂.size, a₂.alive, a₂.kind, a₂.secret) := hshape
          simp only [Prod.mk.injEq] at hk
          obtain ⟨k1, k2, k3, k4⟩ := hk
          simp only [Alloc.shape, k1, k3, k4]
        · simp only [hq, ↓reduceIte]
          exact hR.shape q
      · intro q b₁ b₂ hb₁ hb₂ hs k'
        change (if q = prov then _ else s₁.mem q) = some b₁ at hb₁
        change (if q = prov then _ else s₂.mem q) = some b₂ at hb₂
        by_cases hq : q = prov
        · simp only [hq, ↓reduceIte, Option.some.injEq] at hb₁ hb₂
          subst hb₁
          subst hb₂
          exact hR.agree prov a₁ a₂ hm₁ hm₂ hs k'
        · simp only [hq, ↓reduceIte] at hb₁ hb₂
          exact hR.agree q b₁ b₂ hb₁ hb₂ hs k'
      · show s₁.steps + costFree = s₂.steps + costFree
        rw [hR.steps]
      · intro q hq
        show (if q = prov then _ else s₁.mem q) = none
        by_cases hqp : q = prov
        · have h0 := hR.fresh q hq
          rw [hqp, hm₁] at h0
          cases h0
        · simp only [hqp, ↓reduceIte]
          exact hR.fresh q hq

/-! ## Terminators -/

theorem lock_goto {fr₁ fr₂ : Frame} {rest₁ rest₂ : List Frame} {b' : Nat}
    (hR : R lab s₁ s₂) (h₁ : s₁.stack = fr₁ :: rest₁)
    (hrest : Forall2 (FrameR lab) rest₁ rest₂) (hF : FrameR lab fr₁ fr₂)
    (hj : JumpCond lab fr₁.fn fr₁.block fr₁.ip b') :
    R lab
      { s₁ with
        stack := { fr₁ with block := b', ip := 0 } :: rest₁
        trace := s₁.trace ++ [.branch fr₁.block b']
        steps := s₁.steps + costJump }
      { s₂ with
        stack := { fr₂ with block := b', ip := 0 } :: rest₂
        trace := s₂.trace ++ [.branch fr₂.block b']
        steps := s₂.steps + costJump } := by
  refine hR.step_top h₁ hrest rfl rfl rfl (hF.jump b' hj.1) hR.shape hR.agree ?_ ?_
    hR.allocs hR.heap hR.kind hR.fresh (MemExt.refl _)
    ((hR.ptr_top h₁).same (MemExt.refl _) rfl hj.2.1 hj.2.2)
  · show s₁.trace ++ [.branch fr₁.block b'] = s₂.trace ++ [.branch fr₂.block b']
    rw [hR.trace, hF.block]
  · show s₁.steps + costJump = s₂.steps + costJump
    rw [hR.steps]

theorem lock_jump {fr₁ : Frame} {rest₁ : List Frame} {b' : Nat}
    (hR : R lab s₁ s₂) (h₁ : s₁.stack = fr₁ :: rest₁)
    (hf : p.fetch fr₁.fn fr₁.block fr₁.ip = some (.term (.jump b')))
    (hc : JumpCond lab fr₁.fn fr₁.block fr₁.ip b') : R lab (step p s₁) (step p s₂) := by
  obtain ⟨fr₂, rest₂, h₂, hF, hrest⟩ := hR.top h₁
  rw [step_jump h₁ hf, step_jump h₂ (fetch_same hF hf)]
  exact lock_goto hR h₁ hrest hF hc

theorem lock_branch {fr₁ : Frame} {rest₁ : List Frame} {c : Expr} {t e : Nat}
    (hR : R lab s₁ s₂) (h₁ : s₁.stack = fr₁ :: rest₁)
    (hf : p.fetch fr₁.fn fr₁.block fr₁.ip = some (.term (.branch c t e)))
    (hc : BranchCond lab fr₁.fn fr₁.block fr₁.ip c t e) : R lab (step p s₁) (step p s₂) := by
  obtain ⟨fr₂, rest₂, h₂, hF, hrest⟩ := hR.top h₁
  have hf₂ := fetch_same hF hf
  obtain ⟨c1, c2, c3⟩ := hc
  have he : c.eval fr₁.vars = c.eval fr₂.vars := eval_pub c1 hF.vars
  by_cases hb : (c.eval fr₁.vars).toBool = true
  · have hb₂ : (c.eval fr₂.vars).toBool = true := by rw [← he]; exact hb
    rw [step_branch_true h₁ hf hb, step_branch_true h₂ hf₂ hb₂]
    exact lock_goto hR h₁ hrest hF c2
  · have hb' : (c.eval fr₁.vars).toBool = false := Bool.eq_false_iff.mpr hb
    have hb₂ : (c.eval fr₂.vars).toBool = false := by rw [← he]; exact hb'
    rw [step_branch_false h₁ hf hb', step_branch_false h₂ hf₂ hb₂]
    exact lock_goto hR h₁ hrest hF c3

theorem FrameR.deliver {fr₁ fr₂ g₁ g₂ : Frame} (hF : FrameR lab fr₁ fr₂) (hg : FrameR lab g₁ g₂)
    (hfresh : RetFresh lab fr₁ g₁) (v₁ v₂ : Val) :
    FrameR lab (fr₁.deliver g₁ v₁) (fr₂.deliver g₂ v₂) := by
  unfold Frame.deliver
  rcases hd : fr₁.retDst with _ | d
  · have hd₂ : fr₂.retDst = none := by rw [← hF.retDst]; exact hd
    rw [hd₂]
    exact hg
  · have hd₂ : fr₂.retDst = some d := by rw [← hF.retDst]; exact hd
    rw [hd₂]
    refine ⟨hg.fn, hg.block, hg.ip, hg.retDst, ?_⟩
    intro y hy
    have hyd : y ≠ d := fun h => hfresh d hd (h ▸ hy)
    show (if y = d then v₁ else g₁.vars y) = (if y = d then v₂ else g₂.vars y)
    simp only [hyd, ↓reduceIte]
    exact hg.vars y hy

theorem PtrOK.deliver {m : Mem} {fr g : Frame} (hP : PtrOK lab m g) (hfresh : RetFresh lab fr g)
    (v : Val) : PtrOK lab m (fr.deliver g v) := by
  unfold Frame.deliver
  rcases hd : fr.retDst with _ | d
  · exact hP
  · constructor
    · intro y hy
      have hyd : y ≠ d := fun h =>
        hfresh d hd (h ▸ List.mem_append_left _ (List.mem_append_right _ hy))
      obtain ⟨prov, off, len, a, hv, hm, hs⟩ := hP.1 y hy
      exact ⟨prov, off, len, a, by
        show (if y = d then v else g.vars y) = _
        simp only [hyd, ↓reduceIte]; exact hv, hm, hs⟩
    · intro y hy
      have hyd : y ≠ d := fun h => hfresh d hd (h ▸ List.mem_append_right _ hy)
      obtain ⟨prov, off, len, a, hv, hm, hs⟩ := hP.2 y hy
      exact ⟨prov, off, len, a, by
        show (if y = d then v else g.vars y) = _
        simp only [hyd, ↓reduceIte]; exact hv, hm, hs⟩

/-- `ret` needs no condition: the returned value is not observed.  The caller's
result variable is not public (that is `RetOK`, part of `R`). -/
theorem lock_ret {fr₁ : Frame} {rest₁ : List Frame} {e : Expr}
    (hR : R lab s₁ s₂) (h₁ : s₁.stack = fr₁ :: rest₁)
    (hf : p.fetch fr₁.fn fr₁.block fr₁.ip = some (.term (.ret e))) :
    R lab (step p s₁) (step p s₂) := by
  obtain ⟨fr₂, rest₂, h₂, hF, hrest⟩ := hR.top h₁
  have hf₂ := fetch_same hF hf
  cases hrest with
  | nil =>
    rw [step_ret_top h₁ hf, step_ret_top h₂ hf₂]
    refine ⟨rfl, Forall2.nil, hR.shape, hR.agree, hR.trace, ?_, hR.allocs, hR.heap, hR.fresh,
      (fun f hf => by cases hf), trivial⟩
    show s₁.steps + costRet = s₂.steps + costRet
    rw [hR.steps]
  | @cons g₁ g₂ gs₁ gs₂ hg hgs =>
    rw [step_ret_pop h₁ hf, step_ret_pop h₂ hf₂]
    have hrt := hR.retok
    rw [h₁] at hrt
    refine ⟨hR.kind, Forall2.cons (hF.deliver hg hrt.1 _ _) hgs, hR.shape, hR.agree, hR.trace, ?_,
      hR.allocs, hR.heap, hR.fresh, ?_, ?_⟩
    · show s₁.steps + costRet = s₂.steps + costRet
      rw [hR.steps]
    · intro f hfm
      rcases List.mem_cons.mp hfm with rfl | hm
      · exact (hR.ptrs g₁ (by rw [h₁]; exact List.mem_cons_of_mem _ List.mem_cons_self)).deliver
          hrt.1 _
      · exact hR.ptrs f (by rw [h₁]; exact List.mem_cons_of_mem _ (List.mem_cons_of_mem _ hm))
    · refine hrt.2.replace_top ?_
      unfold Frame.deliver
      cases fr₁.retDst <;> rfl

theorem lock_trap {fr₁ : Frame} {rest₁ : List Frame}
    (hR : R lab s₁ s₂) (h₁ : s₁.stack = fr₁ :: rest₁)
    (hf : p.fetch fr₁.fn fr₁.block fr₁.ip = some (.term .trap)) :
    R lab (step p s₁) (step p s₂) := by
  obtain ⟨fr₂, rest₂, h₂, hF, hrest⟩ := hR.top h₁
  rw [step_trap h₁ hf, step_trap h₂ (fetch_same hF hf)]
  exact hR.of_status (c := costJump) rfl rfl rfl rfl rfl rfl rfl rfl rfl rfl rfl rfl rfl

theorem lock_unreachable {fr₁ : Frame} {rest₁ : List Frame}
    (hR : R lab s₁ s₂) (h₁ : s₁.stack = fr₁ :: rest₁)
    (hf : p.fetch fr₁.fn fr₁.block fr₁.ip = some (.term .unreachable)) :
    R lab (step p s₁) (step p s₂) := by
  obtain ⟨fr₂, rest₂, h₂, hF, hrest⟩ := hR.top h₁
  rw [step_unreachable h₁ hf, step_unreachable h₂ (fetch_same hF hf)]
  exact hR.stuck

/-! ## Gluing: the per-point check, and `lockstep` -/

/-- The per-point check.  `call` is not supported by the spike, so a program
containing one cannot be certified here. -/
def ok (p : Program) (lab : Lab) (f b i : Nat) : Bool :=
  match p.fetch f b i with
  | none => true
  | some (.inst (.let_ x e)) => decide (LetCond lab f b i x e)
  | some (.inst (.alloc x sz _ sec)) => decide (AllocCond lab f b i x sz sec)
  | some (.inst (.load x pe)) => decide (LoadCond lab f b i x pe)
  | some (.inst (.store pe ve)) => decide (StoreCond lab f b i pe ve)
  | some (.inst (.free pe)) => decide (FreeCond lab f b i pe)
  | some (.inst (.call _ _ _)) => false
  | some (.term (.ret _)) => true
  | some (.term (.jump t)) => decide (JumpCond lab f b i t)
  | some (.term (.branch c t e)) => decide (BranchCond lab f b i c t e)
  | some (.term .trap) => true
  | some (.term .unreachable) => true

/-- One step preserves `R`, at *every* program point, provided every point
passes `ok`.  Proved once, by cases on what `fetch` returns. -/
theorem lockstep (hc : ∀ f b i, ok p lab f b i = true) (hR : R lab s₁ s₂) :
    R lab (step p s₁) (step p s₂) := by
  cases h₁ : s₁.stack with
  | nil =>
    have h₂ : s₂.stack = [] := by
      have h := hR.frames
      rw [h₁] at h
      cases h₂ : s₂.stack with
      | nil => rfl
      | cons f l => rw [h₂] at h; cases h
    rw [step_empty h₁, step_empty h₂]
    exact hR.stuck
  | cons fr₁ rest₁ =>
    obtain ⟨fr₂, rest₂, h₂, hF, hrest⟩ := hR.top h₁
    have hok := hc fr₁.fn fr₁.block fr₁.ip
    cases hf : p.fetch fr₁.fn fr₁.block fr₁.ip with
    | none =>
      rw [step_nofetch h₁ hf, step_nofetch h₂ (fetch_same hF hf)]
      exact hR.stuck
    | some op =>
      simp only [ok, hf] at hok
      cases op with
      | inst i =>
        cases i with
        | let_ x e => exact lock_let hR h₁ hf (of_decide_eq_true hok)
        | alloc x sz k sec => exact lock_alloc hR h₁ hf (of_decide_eq_true hok)
        | load x pe => exact lock_load hR h₁ hf (of_decide_eq_true hok)
        | store pe ve => exact lock_store hR h₁ hf (of_decide_eq_true hok)
        | free pe => exact lock_free hR h₁ hf (of_decide_eq_true hok)
        | call d f args => cases hok
      | term t =>
        cases t with
        | ret e => exact lock_ret hR h₁ hf
        | jump b' => exact lock_jump hR h₁ hf (of_decide_eq_true hok)
        | branch c t e => exact lock_branch hR h₁ hf (of_decide_eq_true hok)
        | trap => exact lock_trap hR h₁ hf
        | unreachable => exact lock_unreachable hR h₁ hf

end KrnS0
