/-
  Lean as the ORACLE for the PIF prelude (VER32 PIF-004b).

  This script is run by hand, never by the build.  For every inductive type the prelude
  declares under Lean's own name it prints one S-expression line holding exactly what Lean's
  kernel stores: the universe parameters, the number of parameters and indices, the type,
  each constructor with its type, and the derived recursor with its type.  Binder NAMES are
  dropped (the prelude spells them in the ASCII class); binder INFOS, universe parameter
  names and every level and application are kept.  `Not` is a definition in Lean, so its
  type and value are printed; the prelude calls it `Pif.Not`.

  Run:  oracle/run.sh      (writes oracle/expected.sexp, which is committed and reviewed)

  Grammar of the output (all on one line per declaration):
    level  ::= z | (succ l) | (max l l) | (imax l l) | (p NAME)
    expr   ::= (v N) | (s level) | (c NAME (level ...)) | (a e e)
             | (l INFO e e) | (pi INFO e e)          INFO is d | i | s | c
-/
import Lean
open Lean

/-- Print a universe parameter through `pm`: its own name, or its position for recursors. -/
def lvl (pm : Name → String) : Level → String
  | .zero => "z"
  | .succ l => s!"(succ {lvl pm l})"
  | .max a b => s!"(max {lvl pm a} {lvl pm b})"
  | .imax a b => s!"(imax {lvl pm a} {lvl pm b})"
  | .param n => s!"(p {pm n})"
  | .mvar _ => "?mvar"

def info : BinderInfo → String
  | .default => "d" | .implicit => "i" | .strictImplicit => "s" | .instImplicit => "c"

partial def ex (pm : Name → String) : Expr → String
  | .bvar i => s!"(v {i})"
  | .sort l => s!"(s {lvl pm l})"
  | .const n ls => s!"(c {n} ({" ".intercalate (ls.map (lvl pm))}))"
  | .app f a => s!"(a {ex pm f} {ex pm a})"
  | .lam _ t b bi => s!"(l {info bi} {ex pm t} {ex pm b})"
  | .forallE _ t b bi => s!"(pi {info bi} {ex pm t} {ex pm b})"
  | .mdata _ e => ex pm e
  | _ => "?unsupported"

def own (n : Name) : String := toString n

def posOf (lps : List Name) (n : Name) : String :=
  match lps.findIdx? (· == n) with
  | some i => s!"u{i}"
  | none => "?"

def lpsOf (ns : List Name) : String := "(" ++ " ".intercalate (ns.map toString) ++ ")"

def dumpInd (env : Environment) (n : Name) : IO Unit := do
  let some (.inductInfo i) := env.find? n | throw (IO.userError s!"{n} is not an inductive")
  let ctors ← i.ctors.mapM fun c => do
    let some ci := env.find? c | throw (IO.userError s!"missing {c}")
    pure s!"({c} {ex own ci.type})"
  let some r := env.find? (n ++ `rec) | throw (IO.userError s!"missing {n}.rec")
  IO.println s!"(ind {n} {lpsOf i.levelParams} {i.numParams} {i.numIndices} {ex own i.type} (ctors {" ".intercalate ctors}) (rec {r.levelParams.length} {ex (posOf r.levelParams) r.type}))"

def dumpDef (env : Environment) (n : Name) : IO Unit := do
  let some (.defnInfo d) := env.find? n | throw (IO.userError s!"{n} is not a definition")
  IO.println s!"(def {n} {lpsOf d.levelParams} {ex own d.type} {ex own d.value})"

def main : IO Unit := do
  initSearchPath (← findSysroot)
  let env ← importModules #[{ module := `Init }] {}
  for n in [`Bool, `Nat, `Int, `Eq, `And, `Or, `False, `True, `Exists, `Option, `List, `Prod, `Iff, `Nonempty] do
    dumpInd env n
  dumpDef env `Not
