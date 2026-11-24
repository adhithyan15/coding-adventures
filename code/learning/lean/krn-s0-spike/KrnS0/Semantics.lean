/-!
# KRN-S0, part 1: a minimal WIR in Lean (VER11a section 6.2 and 6.3)

This file is the *meaning* of a small subset of WIR.  It is an
instruction-level small-step machine with an explicit frame stack.  `run` is
defined by **structural** recursion on a step counter, so it reduces in the
kernel on concrete inputs; no well-founded recursion appears anywhere.

Simplifications against `warrant-ir` (all recorded in REPORT.md):

* values are `nat | bool | ptr | unit`; machine integers do not wrap and have
  no width, a load or store addresses one *cell* (offset), not a byte range;
* variables are total (`Nat -> Val`, unbound reads give `unit`) and ill-typed
  operators return a default instead of getting stuck.  The WIR type checker
  rules both out; the spike does not re-prove that;
* capability calls, dynamic values, tuples and overflow modes are absent.
-/

namespace KrnS0

/-! ## Values and expressions -/

inductive Val where
  | nat  (n : Nat)
  | bool (b : Bool)
  | ptr  (prov off len : Nat)     -- provenance, offset, length of the slice
  | unit
  deriving DecidableEq, Repr

def Val.toNat : Val → Nat
  | .nat n => n
  | .bool b => if b then 1 else 0
  | _ => 0

def Val.toBool : Val → Bool
  | .bool b => b
  | .nat n => n != 0
  | _ => false

inductive Expr where
  | const (v : Val)
  | var   (x : Nat)
  | add   (a b : Expr)
  | lt    (a b : Expr)
  | eq    (a b : Expr)
  | bxor  (a b : Expr)
  | bor   (a b : Expr)
  | ptrOff (p o : Expr)
  | ptrLen (p : Expr)

/-- The variables an expression reads. -/
def Expr.fv : Expr → List Nat
  | .const _ => []
  | .var x => [x]
  | .add a b => a.fv ++ b.fv
  | .lt a b => a.fv ++ b.fv
  | .eq a b => a.fv ++ b.fv
  | .bxor a b => a.fv ++ b.fv
  | .bor a b => a.fv ++ b.fv
  | .ptrOff p o => p.fv ++ o.fv
  | .ptrLen p => p.fv

def Expr.eval (vs : Nat → Val) : Expr → Val
  | .const v => v
  | .var x => vs x
  | .add a b => .nat ((a.eval vs).toNat + (b.eval vs).toNat)
  | .lt a b => .bool (decide ((a.eval vs).toNat < (b.eval vs).toNat))
  | .eq a b => .bool (decide ((a.eval vs).toNat = (b.eval vs).toNat))
  | .bxor a b => .nat ((a.eval vs).toNat ^^^ (b.eval vs).toNat)
  | .bor a b => .nat ((a.eval vs).toNat ||| (b.eval vs).toNat)
  | .ptrOff p o =>
      match p.eval vs with
      | .ptr prov off len => .ptr prov (off + (o.eval vs).toNat) len
      | v => v
  | .ptrLen p =>
      match p.eval vs with
      | .ptr _ _ len => .nat len
      | _ => .nat 0

/-! ## Memory (section 6.3) -/

inductive AllocKind where
  | stack | heap
  deriving DecidableEq, Repr

structure Alloc where
  size   : Nat
  alive  : Bool
  kind   : AllocKind
  secret : Bool              -- the tag: does this allocation hold a secret?
  cells  : Nat → Val

abbrev Mem := Nat → Option Alloc

/-- A pointer is usable when its allocation exists, is alive and the offset is
inside it.  Returns provenance, offset and the allocation. -/
def access (m : Mem) : Val → Option (Nat × Nat × Alloc)
  | .ptr prov off _ =>
      match m prov with
      | some a => if a.alive then (if off < a.size then some (prov, off, a) else none) else none
      | none => none
  | _ => none

/-! ## Programs -/

inductive Inst where
  | let_  (x : Nat) (e : Expr)
  | alloc (x : Nat) (size : Expr) (kind : AllocKind) (secret : Bool)
  | load  (x : Nat) (p : Expr)
  | store (p v : Expr)
  | free  (p : Expr)
  | call  (dst : Option Nat) (fn : Nat) (args : List Expr)

inductive Term where
  | ret    (e : Expr)
  | jump   (b : Nat)
  | branch (c : Expr) (t e : Nat)
  | trap
  | unreachable

structure Block where
  insts : List Inst
  term  : Term

structure Fn where
  blocks : List Block

structure Program where
  fns : List Fn

inductive Op where
  | inst (i : Inst)
  | term (t : Term)

/-- Program point `(f, b, i)`: instruction `i` of block `b`, or its terminator
when `i` is one past the last instruction. -/
def Program.fetch (p : Program) (f b i : Nat) : Option Op :=
  match p.fns[f]? with
  | none => none
  | some fn =>
    match fn.blocks[b]? with
    | none => none
    | some bl =>
      match bl.insts[i]? with
      | some ins => some (.inst ins)
      | none => if i = bl.insts.length then some (.term bl.term) else none

/-! ## States -/

structure Frame where
  fn : Nat
  block : Nat
  ip : Nat
  vars : Nat → Val
  retDst : Option Nat        -- where the caller wants the result

inductive Event where
  | branch (src dst : Nat)
  | access (prov off : Nat) (write : Bool)
  deriving DecidableEq, Repr

inductive Status where
  | running
  | ret (v : Val)
  | trap
  | stuck
  deriving DecidableEq, Repr

inductive Outcome where
  | running | ret | trap | stuck
  deriving DecidableEq, Repr

def Status.kind : Status → Outcome
  | .running => .running
  | .ret _ => .ret
  | .trap => .trap
  | .stuck => .stuck

structure State where
  stack  : List Frame
  status : Status
  mem    : Mem
  trace  : List Event
  steps  : Nat               -- accumulated cost, VER25 section 5.1
  allocs : Nat               -- next provenance id
  heap   : Nat               -- bytes allocated with kind `heap`

/-! ## The step function -/

def Frame.set (fr : Frame) (x : Nat) (v : Val) : Frame :=
  { fr with vars := fun y => if y = x then v else fr.vars y }

def Frame.next (fr : Frame) : Frame := { fr with ip := fr.ip + 1 }

def State.stuck (s : State) : State := { s with status := .stuck }

/-- Cost schedule.  ALU 1, branch/jump 1, load/store 2, call/return 2
(VER25 5.1); alloc 3 and free 2 (`cost.rs`, not in VER25 5.1). -/
def costAlu : Nat := 1
def costMem : Nat := 2
def costAlloc : Nat := 3
def costFree : Nat := 2
def costCall : Nat := 2
def costRet : Nat := 2
def costJump : Nat := 1

def stepInst (s : State) (fr : Frame) (rest : List Frame) : Inst → State
  | .let_ x e =>
      { s with stack := (fr.set x (e.eval fr.vars)).next :: rest, steps := s.steps + costAlu }
  | .alloc x sz k sec =>
      let n := (sz.eval fr.vars).toNat
      let id := s.allocs
      let a : Alloc := { size := n, alive := true, kind := k, secret := sec, cells := fun _ => .nat 0 }
      { s with
          stack := (fr.set x (.ptr id 0 n)).next :: rest
          mem := fun q => if q = id then some a else s.mem q
          allocs := s.allocs + 1
          heap := s.heap + (match k with | .heap => n | .stack => 0)
          steps := s.steps + costAlloc }
  | .load x pe =>
      match access s.mem (pe.eval fr.vars) with
      | none => s.stuck
      | some (prov, off, a) =>
          { s with
              stack := (fr.set x (a.cells off)).next :: rest
              trace := s.trace ++ [.access prov off false]
              steps := s.steps + costMem }
  | .store pe ve =>
      match access s.mem (pe.eval fr.vars) with
      | none => s.stuck
      | some (prov, off, a) =>
          let v := ve.eval fr.vars
          let a' : Alloc := { a with cells := fun k => if k = off then v else a.cells k }
          { s with
              stack := fr.next :: rest
              mem := fun q => if q = prov then some a' else s.mem q
              trace := s.trace ++ [.access prov off true]
              steps := s.steps + costMem }
  | .free pe =>
      match access s.mem (pe.eval fr.vars) with
      | none => s.stuck
      | some (prov, _, a) =>
          { s with
              stack := fr.next :: rest
              mem := fun q => if q = prov then some { a with alive := false } else s.mem q
              steps := s.steps + costFree }
  | .call dst f args =>
      let callee : Frame :=
        { fn := f, block := 0, ip := 0, retDst := dst
          vars := fun y => (args.map (·.eval fr.vars))[y]?.getD .unit }
      { s with stack := callee :: fr.next :: rest, steps := s.steps + costCall }

def stepTerm (s : State) (fr : Frame) (rest : List Frame) : Term → State
  | .ret e =>
      let v := e.eval fr.vars
      match rest with
      | [] => { s with stack := [], status := .ret v, steps := s.steps + costRet }
      | g :: gs =>
          let g' := match fr.retDst with
            | some d => g.set d v
            | none => g
          { s with stack := g' :: gs, steps := s.steps + costRet }
  | .jump b =>
      { s with
          stack := { fr with block := b, ip := 0 } :: rest
          trace := s.trace ++ [.branch fr.block b]
          steps := s.steps + costJump }
  | .branch c t e =>
      let b := if (c.eval fr.vars).toBool then t else e
      { s with
          stack := { fr with block := b, ip := 0 } :: rest
          trace := s.trace ++ [.branch fr.block b]
          steps := s.steps + costJump }
  | .trap => { s with status := .trap, steps := s.steps + costJump }
  | .unreachable => s.stuck

/-- One instruction or terminator of the top frame. -/
def step (p : Program) (s : State) : State :=
  match s.stack with
  | [] => s.stuck
  | fr :: rest =>
      match p.fetch fr.fn fr.block fr.ip with
      | none => s.stuck
      | some (.inst i) => stepInst s fr rest i
      | some (.term t) => stepTerm s fr rest t

/-- Structural recursion on the step counter (VER11a 6.2). -/
def run : Nat → Program → State → State
  | 0, _, s => s
  | n + 1, p, s =>
      match s.status with
      | .running => run n p (step p s)
      | _ => s

/-- What an observer sees (VER11a 6.2): the outcome kind, the event trace and
the accumulated cost.  The returned value is deliberately absent. -/
def obs (s : State) : Outcome × List Event × Nat := (s.status.kind, s.trace, s.steps)

end KrnS0
