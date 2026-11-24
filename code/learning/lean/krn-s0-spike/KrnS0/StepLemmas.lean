import KrnS0.Semantics

/-!
# KRN-S0, part 2: step lemmas (VER11a 9.2, item 1)

One lemma per instruction or terminator kind: *given a side condition, `step`
is this state*.  Each is proved once, by unfolding `step`.  This is what the
generated per-point proofs apply, because `rfl` cannot: with a free variable
`i` and a free `n`, the branch condition `decide (i < n)` does not reduce, so
kernel reduction on a symbolic state gets stuck at the first `if`.  The
side-condition is the hypothesis; the lemma turns it into a rewrite.
-/

namespace KrnS0

variable {p : Program} {s : State} {fr : Frame} {rest : List Frame}

theorem step_inst {i : Inst} (hs : s.stack = fr :: rest)
    (hf : p.fetch fr.fn fr.block fr.ip = some (.inst i)) :
    step p s = stepInst s fr rest i := by
  simp only [step, hs, hf]

theorem step_term {t : Term} (hs : s.stack = fr :: rest)
    (hf : p.fetch fr.fn fr.block fr.ip = some (.term t)) :
    step p s = stepTerm s fr rest t := by
  simp only [step, hs, hf]

theorem step_nofetch (hs : s.stack = fr :: rest)
    (hf : p.fetch fr.fn fr.block fr.ip = none) : step p s = s.stuck := by
  simp only [step, hs, hf]

theorem step_empty (hs : s.stack = []) : step p s = s.stuck := by
  simp only [step, hs]

/-- The step lemma the spec names: a branch whose condition is true. -/
theorem step_branch_true {c : Expr} {t e : Nat} (hs : s.stack = fr :: rest)
    (hf : p.fetch fr.fn fr.block fr.ip = some (.term (.branch c t e)))
    (h : (c.eval fr.vars).toBool = true) :
    step p s = { s with
      stack := { fr with block := t, ip := 0 } :: rest
      trace := s.trace ++ [.branch fr.block t]
      steps := s.steps + costJump } := by
  simp only [step_term hs hf, stepTerm, h, if_true]

theorem step_branch_false {c : Expr} {t e : Nat} (hs : s.stack = fr :: rest)
    (hf : p.fetch fr.fn fr.block fr.ip = some (.term (.branch c t e)))
    (h : (c.eval fr.vars).toBool = false) :
    step p s = { s with
      stack := { fr with block := e, ip := 0 } :: rest
      trace := s.trace ++ [.branch fr.block e]
      steps := s.steps + costJump } := by
  simp only [step_term hs hf, stepTerm, h]
  simp

theorem step_jump {b : Nat} (hs : s.stack = fr :: rest)
    (hf : p.fetch fr.fn fr.block fr.ip = some (.term (.jump b))) :
    step p s = { s with
      stack := { fr with block := b, ip := 0 } :: rest
      trace := s.trace ++ [.branch fr.block b]
      steps := s.steps + costJump } := by
  simp only [step_term hs hf, stepTerm]

theorem step_let {x : Nat} {e : Expr} (hs : s.stack = fr :: rest)
    (hf : p.fetch fr.fn fr.block fr.ip = some (.inst (.let_ x e))) :
    step p s = { s with
      stack := (fr.set x (e.eval fr.vars)).next :: rest
      steps := s.steps + costAlu } := by
  simp only [step_inst hs hf, stepInst]

theorem step_load {x : Nat} {pe : Expr} {prov off : Nat} {a : Alloc}
    (hs : s.stack = fr :: rest)
    (hf : p.fetch fr.fn fr.block fr.ip = some (.inst (.load x pe)))
    (ha : access s.mem (pe.eval fr.vars) = some (prov, off, a)) :
    step p s = { s with
      stack := (fr.set x (a.cells off)).next :: rest
      trace := s.trace ++ [.access prov off false]
      steps := s.steps + costMem } := by
  simp only [step_inst hs hf, stepInst, ha]

theorem step_load_stuck {x : Nat} {pe : Expr}
    (hs : s.stack = fr :: rest)
    (hf : p.fetch fr.fn fr.block fr.ip = some (.inst (.load x pe)))
    (ha : access s.mem (pe.eval fr.vars) = none) :
    step p s = s.stuck := by
  simp only [step_inst hs hf, stepInst, ha]

theorem step_store {pe ve : Expr} {prov off : Nat} {a : Alloc}
    (hs : s.stack = fr :: rest)
    (hf : p.fetch fr.fn fr.block fr.ip = some (.inst (.store pe ve)))
    (ha : access s.mem (pe.eval fr.vars) = some (prov, off, a)) :
    step p s = { s with
      stack := fr.next :: rest
      mem := fun q => if q = prov then
        some { a with cells := fun k => if k = off then ve.eval fr.vars else a.cells k }
        else s.mem q
      trace := s.trace ++ [.access prov off true]
      steps := s.steps + costMem } := by
  simp only [step_inst hs hf, stepInst, ha]

theorem step_store_stuck {pe ve : Expr}
    (hs : s.stack = fr :: rest)
    (hf : p.fetch fr.fn fr.block fr.ip = some (.inst (.store pe ve)))
    (ha : access s.mem (pe.eval fr.vars) = none) :
    step p s = s.stuck := by
  simp only [step_inst hs hf, stepInst, ha]

theorem step_free {pe : Expr} {prov off : Nat} {a : Alloc}
    (hs : s.stack = fr :: rest)
    (hf : p.fetch fr.fn fr.block fr.ip = some (.inst (.free pe)))
    (ha : access s.mem (pe.eval fr.vars) = some (prov, off, a)) :
    step p s = { s with
      stack := fr.next :: rest
      mem := fun q => if q = prov then some { a with alive := false } else s.mem q
      steps := s.steps + costFree } := by
  simp only [step_inst hs hf, stepInst, ha]

theorem step_free_stuck {pe : Expr}
    (hs : s.stack = fr :: rest)
    (hf : p.fetch fr.fn fr.block fr.ip = some (.inst (.free pe)))
    (ha : access s.mem (pe.eval fr.vars) = none) :
    step p s = s.stuck := by
  simp only [step_inst hs hf, stepInst, ha]

theorem step_alloc {x : Nat} {sz : Expr} {k : AllocKind} {sec : Bool}
    (hs : s.stack = fr :: rest)
    (hf : p.fetch fr.fn fr.block fr.ip = some (.inst (.alloc x sz k sec))) :
    step p s = { s with
      stack := (fr.set x (.ptr s.allocs 0 (sz.eval fr.vars).toNat)).next :: rest
      mem := fun q => if q = s.allocs then
        some { size := (sz.eval fr.vars).toNat, alive := true, kind := k, secret := sec
               cells := fun _ => .nat 0 } else s.mem q
      allocs := s.allocs + 1
      heap := s.heap + k.heapBytes (sz.eval fr.vars).toNat
      steps := s.steps + costAlloc } := by
  rw [step_inst hs hf]; rfl

theorem step_ret_top {e : Expr} (hs : s.stack = [fr])
    (hf : p.fetch fr.fn fr.block fr.ip = some (.term (.ret e))) :
    step p s = { s with stack := [], status := .ret (e.eval fr.vars)
                        steps := s.steps + costRet } := by
  simp only [step_term hs hf, stepTerm]

theorem step_ret_pop {e : Expr} {g : Frame} {gs : List Frame}
    (hs : s.stack = fr :: g :: gs)
    (hf : p.fetch fr.fn fr.block fr.ip = some (.term (.ret e))) :
    step p s = { s with
      stack := fr.deliver g (e.eval fr.vars) :: gs
      steps := s.steps + costRet } := by
  rw [step_term hs hf]; rfl

theorem step_trap (hs : s.stack = fr :: rest)
    (hf : p.fetch fr.fn fr.block fr.ip = some (.term .trap)) :
    step p s = { s with status := .trap, steps := s.steps + costJump } := by
  simp only [step_term hs hf, stepTerm]

theorem step_unreachable (hs : s.stack = fr :: rest)
    (hf : p.fetch fr.fn fr.block fr.ip = some (.term .unreachable)) :
    step p s = s.stuck := by
  simp only [step_term hs hf, stepTerm]

end KrnS0
