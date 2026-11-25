import KrnS0.StepLemmas

/-!
# KRN-S0, parts 3 and 4: the lockstep relation `R` (VER11a 8.3)

`R lab s₁ s₂` says "public things equal, traces equal so far".  `lab` is the
*generated* part (VER11a 8.3's `Public`): for every program point it lists

* `pub`    : variables whose value is public (equal in both runs);
* `pubPtr` : public variables that are also known to point into a
             **public-tagged** allocation (so a load through them is public);
* `secPtr` : public variables known to point into a **secret-tagged**
             allocation (so a store of a secret through them is fine).

The last two are not in VER11a 8.3; the spike needed them (REPORT.md, finding F3).
Everything in `lab` is untrusted: if it is wrong, an obligation fails.
-/

namespace KrnS0

/-- Pointwise relation on two lists (core has no `List.Forall₂`). -/
inductive Forall2 {α β : Type} (r : α → β → Prop) : List α → List β → Prop where
  | nil : Forall2 r [] []
  | cons {a b l₁ l₂} : r a b → Forall2 r l₁ l₂ → Forall2 r (a :: l₁) (b :: l₂)

structure Info where
  pub    : List Nat
  pubPtr : List Nat
  secPtr : List Nat

/-- Variables with equal values in both runs: all three lists. -/
def Info.pubs (i : Info) : List Nat := i.pub ++ i.pubPtr ++ i.secPtr

abbrev Lab := Nat → Nat → Nat → Info

def Alloc.shape (a : Alloc) : Nat × Bool × AllocKind × Bool :=
  (a.size, a.alive, a.kind, a.secret)

/-- Same allocations, with equal size, liveness, kind and tag. -/
def SameShape (m₁ m₂ : Mem) : Prop :=
  ∀ q, (m₁ q).map Alloc.shape = (m₂ q).map Alloc.shape

/-- Public-tagged allocations hold equal contents. -/
def AgreeOutside (m₁ m₂ : Mem) : Prop :=
  ∀ q a₁ a₂, m₁ q = some a₁ → m₂ q = some a₂ → a₁.secret = false →
    ∀ k, a₁.cells k = a₂.cells k

/-- Memory only grows or changes contents: every allocation stays, with its tag. -/
def MemExt (m m' : Mem) : Prop :=
  ∀ q a, m q = some a → ∃ a', m' q = some a' ∧ a'.secret = a.secret

structure FrameR (lab : Lab) (f₁ f₂ : Frame) : Prop where
  fn     : f₁.fn = f₂.fn
  block  : f₁.block = f₂.block
  ip     : f₁.ip = f₂.ip
  retDst : f₁.retDst = f₂.retDst
  vars   : ∀ y ∈ (lab f₁.fn f₁.block f₁.ip).pubs, f₁.vars y = f₂.vars y

/-- Public variables point where `lab` says they do (checked on run 1 only;
run 2 follows from `SameShape` and equal public values). -/
def PtrOK (lab : Lab) (m : Mem) (f : Frame) : Prop :=
  (∀ y ∈ (lab f.fn f.block f.ip).pubPtr, ∃ prov off len a,
      f.vars y = .ptr prov off len ∧ m prov = some a ∧ a.secret = false) ∧
  (∀ y ∈ (lab f.fn f.block f.ip).secPtr, ∃ prov off len a,
      f.vars y = .ptr prov off len ∧ m prov = some a ∧ a.secret = true)

/-- A callee's result variable is not one of the caller's public variables at
its resume point (call results are treated as secret; no contracts yet). -/
def RetFresh (lab : Lab) (f g : Frame) : Prop :=
  ∀ d, f.retDst = some d → d ∉ (lab g.fn g.block g.ip).pubs

def RetOK (lab : Lab) : List Frame → Prop
  | f :: g :: gs => RetFresh lab f g ∧ RetOK lab (g :: gs)
  | _ => True

structure R (lab : Lab) (s₁ s₂ : State) : Prop where
  kind   : s₁.status.kind = s₂.status.kind
  frames : Forall2 (FrameR lab) s₁.stack s₂.stack
  shape  : SameShape s₁.mem s₂.mem
  agree  : AgreeOutside s₁.mem s₂.mem
  trace  : s₁.trace = s₂.trace
  steps  : s₁.steps = s₂.steps
  allocs : s₁.allocs = s₂.allocs
  heap   : s₁.heap = s₂.heap
  fresh  : ∀ q, s₁.allocs ≤ q → s₁.mem q = none
  ptrs   : ∀ f ∈ s₁.stack, PtrOK lab s₁.mem f
  retok  : RetOK lab s₁.stack

/-! ## Expressions -/

theorem eval_congr {v₁ v₂ : Nat → Val} :
    ∀ (e : Expr), (∀ z ∈ e.fv, v₁ z = v₂ z) → e.eval v₁ = e.eval v₂
  | .const _, _ => rfl
  | .var x, h => h x (List.mem_singleton_self x)
  | .add a b, h => by
      have ha := eval_congr a (fun z hz => h z (List.mem_append_left _ hz))
      have hb := eval_congr b (fun z hz => h z (List.mem_append_right _ hz))
      simp only [Expr.eval, ha, hb]
  | .lt a b, h => by
      have ha := eval_congr a (fun z hz => h z (List.mem_append_left _ hz))
      have hb := eval_congr b (fun z hz => h z (List.mem_append_right _ hz))
      simp only [Expr.eval, ha, hb]
  | .eq a b, h => by
      have ha := eval_congr a (fun z hz => h z (List.mem_append_left _ hz))
      have hb := eval_congr b (fun z hz => h z (List.mem_append_right _ hz))
      simp only [Expr.eval, ha, hb]
  | .bxor a b, h => by
      have ha := eval_congr a (fun z hz => h z (List.mem_append_left _ hz))
      have hb := eval_congr b (fun z hz => h z (List.mem_append_right _ hz))
      simp only [Expr.eval, ha, hb]
  | .bor a b, h => by
      have ha := eval_congr a (fun z hz => h z (List.mem_append_left _ hz))
      have hb := eval_congr b (fun z hz => h z (List.mem_append_right _ hz))
      simp only [Expr.eval, ha, hb]
  | .ptrOff p o, h => by
      have ha := eval_congr p (fun z hz => h z (List.mem_append_left _ hz))
      have hb := eval_congr o (fun z hz => h z (List.mem_append_right _ hz))
      simp only [Expr.eval, ha, hb]
  | .ptrLen p, h => by
      have ha := eval_congr p h
      simp only [Expr.eval, ha]

/-- If every variable `e` reads is in `l`, and `l` is equal in both frames,
then `e` evaluates equally. -/
theorem eval_pub {v₁ v₂ : Nat → Val} {e : Expr} {l : List Nat}
    (hsub : ∀ z ∈ e.fv, z ∈ l) (hl : ∀ y ∈ l, v₁ y = v₂ y) : e.eval v₁ = e.eval v₂ :=
  eval_congr e (fun z hz => hl z (hsub z hz))

/-! ## Memory -/

theorem access_mem {m : Mem} {v : Val} {prov off : Nat} {a : Alloc}
    (h : access m v = some (prov, off, a)) : m prov = some a := by
  cases v with
  | ptr p o l =>
    simp only [access] at h
    cases hm : m p with
    | none => simp only [hm] at h; cases h
    | some b =>
      simp only [hm] at h
      split at h
      · split at h
        · cases h; exact hm
        · cases h
      · cases h
  | _ => simp only [access] at h; cases h

theorem access_prov {m : Mem} {p o l prov off : Nat} {a : Alloc}
    (h : access m (.ptr p o l) = some (prov, off, a)) : prov = p := by
  simp only [access] at h
  cases hm : m p with
  | none => simp only [hm] at h; cases h
  | some b =>
    simp only [hm] at h
    split at h
    · split at h
      · cases h; rfl
      · cases h
    · cases h

theorem access_shape {m₁ m₂ : Mem} (hS : SameShape m₁ m₂) (v : Val) :
    (access m₁ v).map (fun x => (x.1, x.2.1, x.2.2.shape)) =
    (access m₂ v).map (fun x => (x.1, x.2.1, x.2.2.shape)) := by
  cases v with
  | ptr p o l =>
    have h := hS p
    cases h1 : m₁ p with
    | none =>
      cases h2 : m₂ p with
      | none => simp only [access, h1, h2]
      | some b => simp only [h1, h2] at h; cases h
    | some a =>
      cases h2 : m₂ p with
      | none => simp only [h1, h2] at h; cases h
      | some b =>
        simp only [h1, h2, Option.map_some, Option.some.injEq] at h
        have hk : (a.size, a.alive, a.kind, a.secret) = (b.size, b.alive, b.kind, b.secret) := h
        simp only [Prod.mk.injEq] at hk
        obtain ⟨k1, k2, k3, k4⟩ := hk
        simp only [access, h1, h2, k1, k2]
        cases b.alive
        · simp
        · by_cases hl : o < b.size
          · simp [hl, Alloc.shape, k1, k2, k3, k4]
          · simp [hl]
  | _ => rfl

/-- Pointing through a variable `lab` calls public-tagged yields a public-tagged allocation. -/
theorem access_tag_pub {lab : Lab} {m : Mem} {fr : Frame} (hP : PtrOK lab m fr) {q : Nat}
    (hq : q ∈ (lab fr.fn fr.block fr.ip).pubPtr) {prov off : Nat} {a : Alloc}
    (h : access m ((Expr.var q).eval fr.vars) = some (prov, off, a)) : a.secret = false := by
  obtain ⟨p', o', l', a', hv, hm, hs⟩ := hP.1 q hq
  simp only [Expr.eval, hv] at h
  have e1 := access_prov h
  have e2 := access_mem h
  subst e1
  rw [hm] at e2
  cases e2
  exact hs

theorem access_tag_sec {lab : Lab} {m : Mem} {fr : Frame} (hP : PtrOK lab m fr) {q : Nat}
    (hq : q ∈ (lab fr.fn fr.block fr.ip).secPtr) {prov off : Nat} {a : Alloc}
    (h : access m ((Expr.var q).eval fr.vars) = some (prov, off, a)) : a.secret = true := by
  obtain ⟨p', o', l', a', hv, hm, hs⟩ := hP.2 q hq
  simp only [Expr.eval, hv] at h
  have e1 := access_prov h
  have e2 := access_mem h
  subst e1
  rw [hm] at e2
  cases e2
  exact hs

/-! ## Frames -/

theorem FrameR.set_next {lab : Lab} {f₁ f₂ : Frame} (hF : FrameR lab f₁ f₂) {x : Nat}
    {v₁ v₂ : Val}
    (hsub : ∀ y ∈ (lab f₁.fn f₁.block (f₁.ip + 1)).pubs,
      y = x ∨ y ∈ (lab f₁.fn f₁.block f₁.ip).pubs)
    (hx : x ∈ (lab f₁.fn f₁.block (f₁.ip + 1)).pubs → v₁ = v₂) :
    FrameR lab (f₁.set x v₁).next (f₂.set x v₂).next := by
  refine ⟨hF.fn, hF.block, ?_, hF.retDst, ?_⟩
  · show f₁.ip + 1 = f₂.ip + 1
    rw [hF.ip]
  · intro y hy
    change y ∈ (lab f₁.fn f₁.block (f₁.ip + 1)).pubs at hy
    show (if y = x then v₁ else f₁.vars y) = (if y = x then v₂ else f₂.vars y)
    by_cases hyx : y = x
    · subst hyx
      rw [if_pos rfl, if_pos rfl]
      exact hx hy
    · rw [if_neg hyx, if_neg hyx]
      rcases hsub y hy with h | h
      · exact absurd h hyx
      · exact hF.vars y h

theorem FrameR.next {lab : Lab} {f₁ f₂ : Frame} (hF : FrameR lab f₁ f₂)
    (hsub : ∀ y ∈ (lab f₁.fn f₁.block (f₁.ip + 1)).pubs, y ∈ (lab f₁.fn f₁.block f₁.ip).pubs) :
    FrameR lab f₁.next f₂.next := by
  refine ⟨hF.fn, hF.block, ?_, hF.retDst, ?_⟩
  · show f₁.ip + 1 = f₂.ip + 1
    rw [hF.ip]
  · intro y hy
    exact hF.vars y (hsub y hy)

theorem FrameR.jump {lab : Lab} {f₁ f₂ : Frame} (hF : FrameR lab f₁ f₂) (b : Nat)
    (hsub : ∀ y ∈ (lab f₁.fn b 0).pubs, y ∈ (lab f₁.fn f₁.block f₁.ip).pubs) :
    FrameR lab { f₁ with block := b, ip := 0 } { f₂ with block := b, ip := 0 } := by
  refine ⟨hF.fn, rfl, rfl, hF.retDst, ?_⟩
  intro y hy
  exact hF.vars y (hsub y hy)

/-! ## `PtrOK` -/

theorem PtrOK.set_next {lab : Lab} {m m' : Mem} {fr : Frame} (hP : PtrOK lab m fr)
    (hext : MemExt m m') {x : Nat} {v : Val}
    (h1 : ∀ y ∈ (lab fr.fn fr.block (fr.ip + 1)).pubPtr, y ≠ x →
      y ∈ (lab fr.fn fr.block fr.ip).pubPtr)
    (h2 : ∀ y ∈ (lab fr.fn fr.block (fr.ip + 1)).secPtr, y ≠ x →
      y ∈ (lab fr.fn fr.block fr.ip).secPtr)
    (hx1 : x ∈ (lab fr.fn fr.block (fr.ip + 1)).pubPtr →
      ∃ prov off len a, v = .ptr prov off len ∧ m' prov = some a ∧ a.secret = false)
    (hx2 : x ∈ (lab fr.fn fr.block (fr.ip + 1)).secPtr →
      ∃ prov off len a, v = .ptr prov off len ∧ m' prov = some a ∧ a.secret = true) :
    PtrOK lab m' (fr.set x v).next := by
  constructor
  · intro y hy
    change y ∈ (lab fr.fn fr.block (fr.ip + 1)).pubPtr at hy
    by_cases hyx : y = x
    · subst hyx
      obtain ⟨prov, off, len, a, hv, hm, hs⟩ := hx1 hy
      exact ⟨prov, off, len, a, by
        show (if y = y then v else fr.vars y) = _
        rw [if_pos rfl]; exact hv, hm, hs⟩
    · obtain ⟨prov, off, len, a, hv, hm, hs⟩ := hP.1 y (h1 y hy hyx)
      obtain ⟨a', ha', hs'⟩ := hext prov a hm
      exact ⟨prov, off, len, a', by
        show (if y = x then v else fr.vars y) = _
        rw [if_neg hyx]; exact hv, ha', hs'.trans hs⟩
  · intro y hy
    change y ∈ (lab fr.fn fr.block (fr.ip + 1)).secPtr at hy
    by_cases hyx : y = x
    · subst hyx
      obtain ⟨prov, off, len, a, hv, hm, hs⟩ := hx2 hy
      exact ⟨prov, off, len, a, by
        show (if y = y then v else fr.vars y) = _
        rw [if_pos rfl]; exact hv, hm, hs⟩
    · obtain ⟨prov, off, len, a, hv, hm, hs⟩ := hP.2 y (h2 y hy hyx)
      obtain ⟨a', ha', hs'⟩ := hext prov a hm
      exact ⟨prov, off, len, a', by
        show (if y = x then v else fr.vars y) = _
        rw [if_neg hyx]; exact hv, ha', hs'.trans hs⟩

/-- A frame with the same variables at another program point. -/
theorem PtrOK.same {lab : Lab} {m m' : Mem} {fr fr' : Frame} (hP : PtrOK lab m fr)
    (hext : MemExt m m') (hv : fr'.vars = fr.vars)
    (h1 : ∀ y ∈ (lab fr'.fn fr'.block fr'.ip).pubPtr, y ∈ (lab fr.fn fr.block fr.ip).pubPtr)
    (h2 : ∀ y ∈ (lab fr'.fn fr'.block fr'.ip).secPtr, y ∈ (lab fr.fn fr.block fr.ip).secPtr) :
    PtrOK lab m' fr' := by
  constructor
  · intro y hy
    obtain ⟨prov, off, len, a, hv', hm, hs⟩ := hP.1 y (h1 y hy)
    obtain ⟨a', ha', hs'⟩ := hext prov a hm
    exact ⟨prov, off, len, a', by rw [hv]; exact hv', ha', hs'.trans hs⟩
  · intro y hy
    obtain ⟨prov, off, len, a, hv', hm, hs⟩ := hP.2 y (h2 y hy)
    obtain ⟨a', ha', hs'⟩ := hext prov a hm
    exact ⟨prov, off, len, a', by rw [hv]; exact hv', ha', hs'.trans hs⟩

theorem PtrOK.mono {lab : Lab} {m m' : Mem} {fr : Frame} (hP : PtrOK lab m fr)
    (hext : MemExt m m') : PtrOK lab m' fr :=
  hP.same hext rfl (fun _ h => h) (fun _ h => h)

theorem MemExt.refl (m : Mem) : MemExt m m := fun _ a h => ⟨a, h, rfl⟩

/-! ## Assembling `R` after a step that replaces the top frame -/

theorem R.top {lab : Lab} {s₁ s₂ : State} (hR : R lab s₁ s₂) {fr₁ : Frame} {rest₁ : List Frame}
    (h₁ : s₁.stack = fr₁ :: rest₁) :
    ∃ fr₂ rest₂, s₂.stack = fr₂ :: rest₂ ∧ FrameR lab fr₁ fr₂ ∧
      Forall2 (FrameR lab) rest₁ rest₂ := by
  have h := hR.frames
  rw [h₁] at h
  cases h2 : s₂.stack with
  | nil => rw [h2] at h; cases h
  | cons f l =>
    rw [h2] at h
    cases h with
    | cons hf hr => exact ⟨f, l, rfl, hf, hr⟩

theorem R.of_status {lab : Lab} {s₁ s₂ s₁' s₂' : State} (hR : R lab s₁ s₂)
    (e1 : s₁'.stack = s₁.stack) (e2 : s₂'.stack = s₂.stack)
    (m1 : s₁'.mem = s₁.mem) (m2 : s₂'.mem = s₂.mem)
    (t1 : s₁'.trace = s₁.trace) (t2 : s₂'.trace = s₂.trace)
    (st1 : s₁'.steps = s₁.steps + c) (st2 : s₂'.steps = s₂.steps + c)
    (a1 : s₁'.allocs = s₁.allocs) (a2 : s₂'.allocs = s₂.allocs)
    (h1 : s₁'.heap = s₁.heap) (h2 : s₂'.heap = s₂.heap)
    (hk : s₁'.status.kind = s₂'.status.kind) : R lab s₁' s₂' := by
  refine ⟨hk, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩
  · rw [e1, e2]; exact hR.frames
  · rw [m1, m2]; exact hR.shape
  · rw [m1, m2]; exact hR.agree
  · rw [t1, t2]; exact hR.trace
  · rw [st1, st2, hR.steps]
  · rw [a1, a2]; exact hR.allocs
  · rw [h1, h2]; exact hR.heap
  · rw [m1, a1]; exact hR.fresh
  · rw [e1, m1]; exact hR.ptrs
  · rw [e1]; exact hR.retok

theorem RetOK.replace_top {lab : Lab} {f f' : Frame} {rest : List Frame}
    (h : RetOK lab (f :: rest)) (hr : f'.retDst = f.retDst) : RetOK lab (f' :: rest) := by
  cases rest with
  | nil => exact trivial
  | cons g gs =>
    refine ⟨?_, h.2⟩
    intro d hd
    exact h.1 d (hr ▸ hd)

/-- The workhorse: a step that replaces the top frame (keeping its result
variable) and keeps the rest of the stack. -/
theorem R.step_top {lab : Lab} {s₁ s₂ s₁' s₂' : State} (hR : R lab s₁ s₂)
    {fr₁ : Frame} {rest₁ rest₂ : List Frame} {fr₁' fr₂' : Frame}
    (h₁ : s₁.stack = fr₁ :: rest₁) (hrest : Forall2 (FrameR lab) rest₁ rest₂)
    (e₁ : s₁'.stack = fr₁' :: rest₁) (e₂ : s₂'.stack = fr₂' :: rest₂)
    (hret : fr₁'.retDst = fr₁.retDst)
    (hFR : FrameR lab fr₁' fr₂')
    (hm : SameShape s₁'.mem s₂'.mem) (ha : AgreeOutside s₁'.mem s₂'.mem)
    (ht : s₁'.trace = s₂'.trace) (hst : s₁'.steps = s₂'.steps)
    (hal : s₁'.allocs = s₂'.allocs) (hh : s₁'.heap = s₂'.heap)
    (hk : s₁'.status.kind = s₂'.status.kind)
    (hfresh : ∀ q, s₁'.allocs ≤ q → s₁'.mem q = none)
    (hext : MemExt s₁.mem s₁'.mem)
    (hptr : PtrOK lab s₁'.mem fr₁') : R lab s₁' s₂' := by
  refine ⟨hk, ?_, hm, ha, ht, hst, hal, hh, hfresh, ?_, ?_⟩
  · rw [e₁, e₂]; exact Forall2.cons hFR hrest
  · intro f hf
    rw [e₁] at hf
    rcases List.mem_cons.mp hf with rfl | hr
    · exact hptr
    · exact (hR.ptrs f (by rw [h₁]; exact List.mem_cons_of_mem _ hr)).mono hext
  · rw [e₁]
    have := hR.retok
    rw [h₁] at this
    exact this.replace_top hret

theorem R.stuck {lab : Lab} {s₁ s₂ : State} (hR : R lab s₁ s₂) : R lab s₁.stuck s₂.stuck :=
  hR.of_status (c := 0) rfl rfl rfl rfl rfl rfl (Nat.add_zero _).symm (Nat.add_zero _).symm
    rfl rfl rfl rfl rfl

/-- The observation of related states is equal (the base case of `lockstep_obs`). -/
theorem R.obs_eq {lab : Lab} {s₁ s₂ : State} (hR : R lab s₁ s₂) : obs s₁ = obs s₂ := by
  simp only [obs, hR.kind, hR.trace, hR.steps]

end KrnS0
