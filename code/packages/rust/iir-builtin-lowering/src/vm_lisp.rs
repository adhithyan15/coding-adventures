//! # vm_lisp — the Lisp family's value rules for the generic VM (#18470, #18493).
//!
//! ## Why a VM needs its own version of `lisp_semantics`
//!
//! [`crate::lisp_semantics`] writes Lisp's rules — truthiness, `not`, … — as
//! IIR over **tagged words**, for the native, LLVM and McCarthy-JIT pipelines.
//! The `lang_matrix` VM and JIT columns do not use tagged words. They run the
//! structural lowering (`lower_heap_builtins`: a cons is an `alloc` and two
//! `field_store`s) on `vm-core`, where a value is a `vm_core::value::Value`
//! that already knows its own kind. Before this pass two Lisp facts were lost
//! on the way there:
//!
//! ```text
//!   source            reached vm-core as             so
//!   ---------------   ----------------------------   -----------------------------
//!   '()               const 0 : ref<LispyPair>        nil IS the integer 0
//!   'a                const 2^29+id : i32             a symbol IS an integer
//!   (if 0 1 2)        jmp_if_false 0                  0 is false (the VM's C rule)
//!   (not x) …         call_builtin "not"              UnknownOpcode — no handler
//! ```
//!
//! TW00 says only `#f` and nil are false: `(if 0 1 2)` is `1`, and `(null? 0)`
//! is `#f`. The VM cannot learn that rule — LANG83 PR7 forbids a VM builtin
//! that carries one language's semantics, and LANG85 §11.1 decision 4 puts the
//! fix "in the frontend's lowering; `vm-core` itself stays language-neutral".
//! So this pass, which a Lisp-family compiler runs before handing a module to
//! the VM, writes each rule as IIR over five **language-neutral** registry
//! primitives that `vm-core` lowers over its own value kinds:
//!
//! | primitive | vm-core answer |
//! |-----------|----------------|
//! | `dyn_tag x` | 0 integer, 1 nil, 2 symbol, 3 `#f`, 5 `#t`, 7 pair, −1 string / float |
//! | `dyn_is_nil x` | is `x` nil (never the integer 0)? |
//! | `dyn_is_pair x` | is `x` a cell? |
//! | `dyn_nil` | nil (`Value::Null`) |
//! | `dyn_symbol "name"` | the symbol `name` (`Value::Symbol`, one id per name) |
//!
//! ## The rewrites
//!
//! | Lisp-family IIR in | written out as |
//! |--------------------|----------------|
//! | `const 0 : ref<LispyPair>` (nil) | `call_builtin dyn_nil` |
//! | `const Var(name) : symbol` | `call_builtin dyn_symbol, "name"` |
//! | `is_null x` (`null?`) | `call_builtin dyn_is_nil x` |
//! | `call_builtin "pair?" x` | `call_builtin dyn_is_pair x` |
//! | `jmp_if_false c, L` / `jmp_if_true c, L`, `c` not a `bool` | truthiness of `c`, then the branch on a 0/1 |
//! | `call_builtin "not" x` | `(tag \| 2) = 3` — the kind is nil or `#f` |
//! | `call_builtin "number?" x` | `tag = 0` |
//! | `call_builtin "symbol?" x` | `tag = 2` |
//! | `call_builtin "boolean?" x` | `(tag = 3) + (tag = 5) ≠ 0` |
//! | `call_builtin "list?" x` | an inline walk down the `rest` fields (below) |
//! | `call_builtin "/" a b`, `"quotient"` | `div a b` (truncating) |
//! | `call_builtin "remainder" a b` | `mod a b` (truncating: the sign of `a`) |
//! | `call_builtin "modulo" a b` | `i64_mod_floor a b` (floored: the sign of `b`) |
//!
//! `tag` is `call_builtin dyn_tag x`. Everything else is a registry op the VM
//! already runs: `cmp_eq` / `cmp_ne`, `add`, `mul`, `field_load`, `mov`,
//! `const`, `jmp`, `jmp_if_false`, `label`.
//!
//! ### Truthiness
//!
//! A branch on a register some instruction typed `bool` (a comparison, a
//! predicate this pass wrote) is left alone: a `bool` is `#t` or `#f`, and the
//! VM's own rule agrees on those two. Anything else gets the rule first:
//!
//! ```text
//!   t = call_builtin dyn_tag, c     ; the kind of c
//!   o = or t, 2                     ; folds nil (1) onto #f (3)
//!   k = cmp_ne o, 3                 ; #t iff c is truthy
//!   jmp_if_false k, L               ; the branch now sees a real boolean
//! ```
//!
//! Setting bit 1 sends exactly the two false kinds to 3, and no other kind
//! there — every other tag either has bit 0 clear or bit 2 set:
//!
//! | `c` | `dyn_tag` | `tag \| 2` | `k` | the VM's own `is_truthy` (before) |
//! |-----|-----------|-----------|-----|-----------------------------------|
//! | `0`, `7` | 0 | 2 | **#t** | false for `0` |
//! | `""`, `1.5` | −1 | −1 | **#t** | false for `""` |
//! | nil | 1 | 3 | #f | false (as `Int(0)`) |
//! | `'a` | 2 | 2 | #t | true |
//! | `#f` | 3 | 3 | #f | false |
//! | `#t` | 5 | 7 | #t | true |
//! | a pair | 7 | 7 | #t | true |
//!
//! This is the rule `lisp_semantics` writes over the word — `(x ≠ 3) × (x ≠
//! 1)` — asked of the kind instead, because a VM value has no word to compare.
//! `not` is the same fold with the opposite test, `cmp_eq o, 3`.
//!
//! ### `list?`
//!
//! A proper list is nil, or a pair whose rest is a proper list. Twig cannot
//! build a cycle (it has no `set-cdr!`), so the walk ends:
//!
//! ```text
//!   cur = mov x
//! loop:
//!   t = call_builtin dyn_tag, cur
//!   n = cmp_eq t, 1 ; jmp_if_false n, notnil
//!   r = const #t ; jmp done                 ; reached nil: a list
//! notnil:
//!   p = cmp_eq t, 7 ; jmp_if_false p, no
//!   cur = field_load cur, 1 ; jmp loop      ; a pair: walk its rest
//! no:
//!   r = const #f                            ; an atom before nil: not a list
//! done:
//! ```
//!
//! It is inline, not a helper function, so it needs no reserved name; the
//! function it lands in gains a loop, and so runs on the JIT's interpreter
//! tier — as every function with a branch already does (`execute_with_jit`
//! compiles only fully typed functions, and a `label` is typed `void`).
//!
//! ## Division
//!
//! Twig emits `call_builtin "/"` (and `quotient`, `remainder`, `modulo`) when
//! an operand is not statically an integer — every division on a parameter.
//! The VM has no such builtins, so `(define (f x y) (/ x y))` stopped with
//! `UnknownOpcode`. Each becomes the registry op with the same rounding as
//! `dynval-runtime` (twig-vm): truncating `div` and `mod`, floored
//! `i64_mod_floor`. A zero divisor stops the program on the VM (`div` / `mod`
//! with a division-by-zero error, `i64_mod_floor` with `Trap.DivZero`), as
//! twig-vm raises.
//!
//! ## Who calls it
//!
//! Only a Lisp-family pipeline, on a module whose every value follows Lisp's
//! rules — it rewrites every branch in the module. It runs **after**
//! `lower_closures_to_heap` and `lower_heap_builtins` (which emit the nil
//! constants and `is_null`s it rewrites) and replaces
//! `intern_symbols_structural` for the module (a symbol literal it rewrites is
//! no longer a `const … : symbol`, so that pass then has nothing to do). The
//! `lang_matrix` VM and JIT columns run it on every Twig module.
//!
//! Running it twice changes nothing more: everything it writes is either a
//! primitive call it does not rewrite again or a branch on a register it typed
//! `bool`.

use std::collections::HashSet;

use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::IIRModule;

/// The frontends' nil literal type (`const 0 : ref<LispyPair>`).
const NIL_HINT: &str = "ref<LispyPair>";
/// The frontends' symbol literal type (`const Var(name) : symbol`).
const SYMBOL_HINT: &str = "symbol";
/// The type of a dynamic value this pass produces.
const ANY: &str = "any";
/// The type of a boolean this pass produces (a `Value::Bool` on the VM).
const BOOL: &str = "bool";
/// The machine-integer type of the arithmetic this pass emits.
const I64: &str = "i64";

/// The LANG83 PR7f tag numbers `dyn_tag` answers.
const TAG_INT: i64 = 0;
const TAG_NIL: i64 = 1;
const TAG_SYMBOL: i64 = 2;
const TAG_FALSE: i64 = 3;
const TAG_TRUE: i64 = 5;
const TAG_PAIR: i64 = 7;

/// The field a pair keeps its rest in (`lower_heap_builtins`: `cdr` is
/// `field_load x, 1`).
const REST_FIELD: i64 = 1;

/// Rewrite every function of a Lisp-family `module` for the generic VM — see
/// the module docs for the table.
///
/// ```
/// use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
/// use iir_builtin_lowering::vm_lisp::lower_lisp_for_vm;
///
/// // (if 0 1 2): a branch on the integer 0.
/// let mut m = IIRModule::new("m", "twig");
/// m.add_or_replace(IIRFunction::new("main", vec![], "any", vec![
///     IIRInstr::new("const", Some("c".into()), vec![Operand::Int(0)], "i64"),
///     IIRInstr::new("jmp_if_false", None, vec![Operand::Var("c".into()), Operand::Var("else".into())], "void"),
/// ]));
/// lower_lisp_for_vm(&mut m);
/// let ops: Vec<&str> = m.functions[0].instructions.iter().map(|i| i.op.as_str()).collect();
/// // The branch now tests the kind of `c`, not the number.
/// assert_eq!(ops, ["const", "call_builtin", "or", "cmp_ne", "jmp_if_false"]);
/// ```
pub fn lower_lisp_for_vm(module: &mut IIRModule) {
    for func in &mut module.functions {
        lower_function(func);
    }
}

/// Rewrite one function. Rebuilds the instruction list, since most rewrites
/// expand one instruction into several.
///
/// `source_map` is kept in lockstep when it was: every instruction an
/// expansion writes carries the source position of the instruction it
/// replaced, so a debugger or coverage tool still lands on the right line.
fn lower_function(func: &mut IIRFunction) {
    let bools = bool_registers(func);
    let mut fresh = Fresh::new(func);
    let locs = (func.source_map.len() == func.instructions.len()).then(|| std::mem::take(&mut func.source_map));
    let mut out: Vec<IIRInstr> = Vec::with_capacity(func.instructions.len());
    let mut new_locs = Vec::new();
    for (i, instr) in std::mem::take(&mut func.instructions).into_iter().enumerate() {
        let before = out.len();
        rewrite(instr, &bools, &mut fresh, &mut out);
        if let Some(locs) = &locs {
            new_locs.extend(std::iter::repeat_n(locs[i], out.len() - before));
        }
    }
    func.instructions = out;
    if locs.is_some() {
        func.source_map = new_locs;
    }
}

/// The registers every definition of which is typed `bool`. A branch on one
/// of these needs no truthiness rule: it holds `#t` or `#f`. A parameter is
/// never one (its type says nothing about what a caller passed), and a
/// register with any non-`bool` definition is not either.
fn bool_registers(func: &IIRFunction) -> HashSet<String> {
    let mut bools: HashSet<String> = HashSet::new();
    let mut other: HashSet<String> = func.params.iter().map(|(n, _)| n.clone()).collect();
    for instr in &func.instructions {
        if let Some(dest) = &instr.dest {
            if instr.type_hint == BOOL {
                bools.insert(dest.clone());
            } else {
                other.insert(dest.clone());
            }
        }
    }
    bools.retain(|r| !other.contains(r));
    bools
}

/// Rewrite one instruction into `out` (unchanged if no rule applies).
fn rewrite(instr: IIRInstr, bools: &HashSet<String>, fresh: &mut Fresh, out: &mut Vec<IIRInstr>) {
    match instr.op.as_str() {
        // nil literal → dyn_nil
        "const" if instr.type_hint == NIL_HINT && instr.srcs == [Operand::Int(0)] => {
            out.push(retarget(instr, "call_builtin", vec![var("dyn_nil")], ANY));
        }
        // symbol literal → dyn_symbol "name"
        "const" if instr.type_hint == SYMBOL_HINT => match instr.srcs.as_slice() {
            [Operand::Var(name)] => {
                let srcs = vec![var("dyn_symbol"), Operand::Str(name.clone())];
                out.push(retarget(instr, "call_builtin", srcs, ANY));
            }
            _ => out.push(instr),
        },
        // null? → dyn_is_nil (is_null would call the integer 0 nil)
        "is_null" if instr.srcs.len() == 1 => {
            let srcs = vec![var("dyn_is_nil"), instr.srcs[0].clone()];
            out.push(retarget(instr, "call_builtin", srcs, BOOL));
        }
        "jmp_if_false" | "jmp_if_true" => rewrite_branch(instr, bools, fresh, out),
        "call_builtin" => rewrite_builtin(instr, fresh, out),
        _ => out.push(instr),
    }
}

/// A branch on anything but a `bool` first computes Lisp truthiness.
fn rewrite_branch(instr: IIRInstr, bools: &HashSet<String>, fresh: &mut Fresh, out: &mut Vec<IIRInstr>) {
    let cond = match instr.srcs.first() {
        Some(Operand::Var(c)) if !bools.contains(c) => Operand::Var(c.clone()),
        // A `bool` register, or an immediate (`Operand::Bool`) — nothing to do.
        _ => {
            out.push(instr);
            return;
        }
    };
    let truthy = fresh.name("truthy");
    emit_false_fold("cmp_ne", &truthy, cond, fresh, out);
    let mut branch = instr;
    branch.srcs[0] = Operand::Var(truthy);
    out.push(branch);
}

/// Emit `dest = cmp_op (dyn_tag x | 2), 3` — with `cmp_ne`, Lisp truthiness;
/// with `cmp_eq`, Lisp `not` (module docs, "Truthiness").
fn emit_false_fold(cmp_op: &str, dest: &str, x: Operand, fresh: &mut Fresh, out: &mut Vec<IIRInstr>) {
    let t = emit_tag(x, fresh, out);
    let folded = fresh.name("fold");
    out.push(IIRInstr::new("or", Some(folded.clone()), vec![var(&t), Operand::Int(0b10)], I64));
    out.push(cmp(cmp_op, dest, &folded, TAG_FALSE));
}

/// Emit `t = call_builtin dyn_tag, x`; return `t`.
fn emit_tag(x: Operand, fresh: &mut Fresh, out: &mut Vec<IIRInstr>) -> String {
    let t = fresh.name("tag");
    out.push(builtin(Some(&t), "dyn_tag", vec![x], ANY));
    t
}

/// Emit `dest = (tag = k1) + (tag = k2) ≠ 0` — "is the kind one of these two".
fn emit_tag_is_either(dest: &str, x: Operand, k1: i64, k2: i64, fresh: &mut Fresh, out: &mut Vec<IIRInstr>) {
    let t = emit_tag(x, fresh, out);
    let a = fresh.name("is");
    let b = fresh.name("is");
    let s = fresh.name("either");
    out.push(cmp("cmp_eq", &a, &t, k1));
    out.push(cmp("cmp_eq", &b, &t, k2));
    // The two tests are exclusive (one tag), so the sum is 0 or 1.
    out.push(IIRInstr::new("add", Some(s.clone()), vec![var(&a), var(&b)], I64));
    out.push(cmp("cmp_ne", dest, &s, 0));
}

/// The source-level builtins this pass writes out. Anything else (`+`, `<`,
/// `=`, `equal?`, the output builtins …) is left for the VM.
fn rewrite_builtin(instr: IIRInstr, fresh: &mut Fresh, out: &mut Vec<IIRInstr>) {
    let name = match instr.srcs.first() {
        Some(Operand::Var(n)) => n.clone(),
        _ => {
            out.push(instr);
            return;
        }
    };
    let args = instr.srcs[1..].to_vec();
    // A predicate whose value is discarded has no destination; give the
    // expansion a fresh one so it stays well formed.
    let mut dest = || instr.dest.clone().unwrap_or_else(|| fresh.name("unused"));
    match (name.as_str(), args.as_slice()) {
        ("pair?", [x]) => out.push(retarget(instr, "call_builtin", vec![var("dyn_is_pair"), x.clone()], BOOL)),
        ("not", [x]) => {
            let d = dest();
            emit_false_fold("cmp_eq", &d, x.clone(), fresh, out);
        }
        ("boolean?", [x]) => {
            let d = dest();
            emit_tag_is_either(&d, x.clone(), TAG_FALSE, TAG_TRUE, fresh, out);
        }
        ("number?", [x]) => {
            let d = dest();
            let t = emit_tag(x.clone(), fresh, out);
            out.push(cmp("cmp_eq", &d, &t, TAG_INT));
        }
        ("symbol?", [x]) => {
            let d = dest();
            let t = emit_tag(x.clone(), fresh, out);
            out.push(cmp("cmp_eq", &d, &t, TAG_SYMBOL));
        }
        ("list?", [x]) => {
            let d = dest();
            emit_list_p(&d, x.clone(), fresh, out);
        }
        ("/" | "quotient", [a, b]) => out.push(retarget(instr, "div", vec![a.clone(), b.clone()], I64)),
        ("remainder", [a, b]) => out.push(retarget(instr, "mod", vec![a.clone(), b.clone()], I64)),
        ("modulo", [a, b]) => out.push(retarget(instr, "i64_mod_floor", vec![a.clone(), b.clone()], I64)),
        _ => out.push(instr),
    }
}

/// Turn `instr` into a different one-instruction form, keeping its
/// destination and the profiling fields it already carries.
fn retarget(mut instr: IIRInstr, op: &str, srcs: Vec<Operand>, ty: &str) -> IIRInstr {
    instr.op = op.to_string();
    instr.srcs = srcs;
    instr.type_hint = ty.to_string();
    instr
}

/// The inline proper-list walk (module docs, "`list?`").
fn emit_list_p(dest: &str, x: Operand, fresh: &mut Fresh, out: &mut Vec<IIRInstr>) {
    let cur = fresh.name("cur");
    let lp = fresh.name("loop");
    let notnil = fresh.name("notnil");
    let no = fresh.name("no");
    let done = fresh.name("done");
    out.push(IIRInstr::new("mov", Some(cur.clone()), vec![x], ANY));
    out.push(label(&lp));
    let t = emit_tag(var(&cur), fresh, out);
    let is_nil = fresh.name("isnil");
    out.push(cmp("cmp_eq", &is_nil, &t, TAG_NIL));
    out.push(IIRInstr::new("jmp_if_false", None, vec![var(&is_nil), var(&notnil)], "void"));
    out.push(IIRInstr::new("const", Some(dest.to_string()), vec![Operand::Bool(true)], BOOL));
    out.push(jmp(&done));
    out.push(label(&notnil));
    let is_pair = fresh.name("ispair");
    out.push(cmp("cmp_eq", &is_pair, &t, TAG_PAIR));
    out.push(IIRInstr::new("jmp_if_false", None, vec![var(&is_pair), var(&no)], "void"));
    out.push(IIRInstr::new(
        "field_load",
        Some(cur.clone()),
        vec![var(&cur), Operand::Int(REST_FIELD)],
        "ref<any>",
    ));
    out.push(jmp(&lp));
    out.push(label(&no));
    out.push(IIRInstr::new("const", Some(dest.to_string()), vec![Operand::Bool(false)], BOOL));
    out.push(label(&done));
}

// --- small IIR builders -----------------------------------------------------

fn var(name: &str) -> Operand {
    Operand::Var(name.to_string())
}

fn builtin(dest: Option<&str>, name: &str, args: Vec<Operand>, ty: &str) -> IIRInstr {
    let mut srcs = vec![var(name)];
    srcs.extend(args);
    IIRInstr::new("call_builtin", dest.map(str::to_string), srcs, ty)
}

fn cmp(op: &str, dest: &str, a: &str, k: i64) -> IIRInstr {
    IIRInstr::new(op, Some(dest.to_string()), vec![var(a), Operand::Int(k)], BOOL)
}

fn label(name: &str) -> IIRInstr {
    IIRInstr::new("label", None, vec![var(name)], "void")
}

fn jmp(name: &str) -> IIRInstr {
    IIRInstr::new("jmp", None, vec![var(name)], "void")
}

/// Register and label names this pass invents in one function:
/// `__vm{n}_{what}`, skipping every name the function already uses.
///
/// Twig reserves the whole `__` namespace for its compiler, so a program
/// cannot spell one of these; another pass's temporaries could, which is why
/// the names already present (parameters, destinations and every `Var`
/// operand, labels included) are skipped rather than trusted to differ.
struct Fresh {
    used: HashSet<String>,
    next: usize,
}

impl Fresh {
    fn new(func: &IIRFunction) -> Self {
        let mut used: HashSet<String> = func.params.iter().map(|(n, _)| n.clone()).collect();
        for instr in &func.instructions {
            if let Some(d) = &instr.dest {
                used.insert(d.clone());
            }
            for s in &instr.srcs {
                if let Operand::Var(v) = s {
                    used.insert(v.clone());
                }
            }
        }
        Fresh { used, next: 0 }
    }

    fn name(&mut self, what: &str) -> String {
        loop {
            self.next += 1;
            let candidate = format!("__vm{}_{what}", self.next);
            if self.used.insert(candidate.clone()) {
                return candidate;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ins(op: &str, dest: Option<&str>, srcs: Vec<Operand>, ty: &str) -> IIRInstr {
        IIRInstr::new(op, dest.map(str::to_string), srcs, ty)
    }

    fn lowered(params: Vec<(&str, &str)>, body: Vec<IIRInstr>) -> IIRFunction {
        let params = params.into_iter().map(|(n, t)| (n.to_string(), t.to_string())).collect();
        let mut m = IIRModule::new("m", "twig");
        m.add_or_replace(IIRFunction::new("f", params, "any", body));
        lower_lisp_for_vm(&mut m);
        m.functions.remove(0)
    }

    /// What an instruction *is* (IIRInstr has no `PartialEq`: its profiling
    /// fields are not part of its meaning).
    fn shape(instrs: &[IIRInstr]) -> Vec<(String, Option<String>, Vec<Operand>, String)> {
        instrs.iter().map(|i| (i.op.clone(), i.dest.clone(), i.srcs.clone(), i.type_hint.clone())).collect()
    }

    fn ops(f: &IIRFunction) -> Vec<&str> {
        f.instructions.iter().map(|i| i.op.as_str()).collect()
    }

    fn builtin_names(f: &IIRFunction) -> Vec<String> {
        f.instructions
            .iter()
            .filter(|i| i.op == "call_builtin")
            .filter_map(|i| i.srcs.first().and_then(|s| s.as_var()).map(str::to_string))
            .collect()
    }

    #[test]
    fn nil_and_symbol_literals_become_the_primitives() {
        let f = lowered(vec![], vec![
            ins("const", Some("n"), vec![Operand::Int(0)], NIL_HINT),
            ins("const", Some("s"), vec![var("abc")], SYMBOL_HINT),
            ins("const", Some("z"), vec![Operand::Int(0)], "i64"),
        ]);
        assert_eq!(builtin_names(&f), ["dyn_nil", "dyn_symbol"]);
        assert_eq!(f.instructions[1].srcs[1], Operand::Str("abc".into()), "the name, not a register");
        assert_eq!(
            shape(&f.instructions[2..]),
            shape(&[ins("const", Some("z"), vec![Operand::Int(0)], "i64")]),
            "an integer 0 is untouched"
        );
    }

    #[test]
    fn null_p_and_pair_p_use_the_kind_aware_primitives() {
        let f = lowered(vec![("x", "any")], vec![
            ins("is_null", Some("a"), vec![var("x")], "bool"),
            ins("call_builtin", Some("b"), vec![var("pair?"), var("x")], "any"),
        ]);
        assert_eq!(builtin_names(&f), ["dyn_is_nil", "dyn_is_pair"]);
    }

    #[test]
    fn a_branch_on_a_bool_is_left_alone() {
        let body = vec![
            ins("cmp_lt", Some("c"), vec![var("x"), Operand::Int(0)], "bool"),
            ins("jmp_if_false", None, vec![var("c"), var("L")], "void"),
        ];
        let f = lowered(vec![("x", "i64")], body.clone());
        assert_eq!(shape(&f.instructions), shape(&body));
    }

    #[test]
    fn a_branch_on_anything_else_gets_truthiness() {
        // The parameter, a register with one non-bool definition among bool
        // ones, and an i64: each is wrapped.
        for (params, body) in [
            (vec![("x", "any")], vec![ins("jmp_if_false", None, vec![var("x"), var("L")], "void")]),
            (vec![("x", "bool")], vec![ins("jmp_if_true", None, vec![var("x"), var("L")], "void")]),
            (
                vec![],
                vec![
                    ins("const", Some("c"), vec![Operand::Bool(true)], "bool"),
                    ins("const", Some("c"), vec![Operand::Int(0)], "i64"),
                    ins("jmp_if_false", None, vec![var("c"), var("L")], "void"),
                ],
            ),
        ] {
            let f = lowered(params, body);
            let n = f.instructions.len();
            assert_eq!(&ops(&f)[n - 4..n - 1], ["call_builtin", "or", "cmp_ne"]);
            let branch = &f.instructions[n - 1];
            assert!(branch.op.starts_with("jmp_if_"));
            assert_eq!(branch.srcs[0], var(f.instructions[n - 2].dest.as_deref().unwrap()));
            assert_eq!(branch.srcs[1], var("L"), "the target is kept");
        }
    }

    #[test]
    fn predicates_and_not_expand_inline() {
        for (name, want) in [
            ("number?", vec!["call_builtin", "cmp_eq"]),
            ("symbol?", vec!["call_builtin", "cmp_eq"]),
            ("not", vec!["call_builtin", "or", "cmp_eq"]),
            ("boolean?", vec!["call_builtin", "cmp_eq", "cmp_eq", "add", "cmp_ne"]),
        ] {
            let f = lowered(vec![("x", "any")], vec![ins("call_builtin", Some("r"), vec![var(name), var("x")], "any")]);
            assert_eq!(ops(&f), want, "{name}");
            assert_eq!(builtin_names(&f), ["dyn_tag"], "{name}: only the neutral primitive is called");
            let last = f.instructions.last().unwrap();
            assert_eq!((last.dest.as_deref(), last.type_hint.as_str()), (Some("r"), BOOL), "{name}");
        }
    }

    #[test]
    fn list_p_is_a_loop_over_the_rest_field() {
        let f = lowered(vec![("x", "any")], vec![ins("call_builtin", Some("r"), vec![var("list?"), var("x")], "any")]);
        assert_eq!(builtin_names(&f), ["dyn_tag"]);
        let rest = f.instructions.iter().find(|i| i.op == "field_load").expect("walks the list");
        assert_eq!(rest.srcs[1], Operand::Int(REST_FIELD));
        assert_eq!(f.instructions.iter().filter(|i| i.op == "label").count(), 4);
        assert_eq!(f.instructions.iter().filter(|i| i.dest.as_deref() == Some("r")).count(), 2, "#t and #f writers");
    }

    #[test]
    fn division_names_become_registry_ops() {
        for (name, op) in [("/", "div"), ("quotient", "div"), ("remainder", "mod"), ("modulo", "i64_mod_floor")] {
            let f = lowered(vec![("a", "any"), ("b", "any")], vec![ins(
                "call_builtin",
                Some("r"),
                vec![var(name), var("a"), var("b")],
                "any",
            )]);
            assert_eq!(shape(&f.instructions), shape(&[ins(op, Some("r"), vec![var("a"), var("b")], I64)]), "{name}");
        }
    }

    #[test]
    fn other_builtins_and_wrong_arities_are_left_alone() {
        for srcs in [
            vec![var("+"), var("a"), var("b")],
            vec![var("equal?"), var("a"), var("b")],
            vec![var("not"), var("a"), var("b")],
            vec![var("/"), var("a")],
            vec![var("print_i64"), var("a")],
        ] {
            let body = vec![ins("call_builtin", Some("r"), srcs, "any")];
            assert_eq!(shape(&lowered(vec![("a", "any"), ("b", "any")], body.clone()).instructions), shape(&body));
        }
    }

    #[test]
    fn source_map_stays_in_lockstep() {
        use interpreter_ir::SourceLoc;
        let mut f = IIRFunction::new("f", vec![("x".into(), "any".into())], "any", vec![
            ins("call_builtin", Some("r"), vec![var("not"), var("x")], "any"),
            ins("ret", None, vec![var("r")], "any"),
        ]);
        f.source_map = vec![SourceLoc::new(3, 1), SourceLoc::new(4, 1)];
        let mut m = IIRModule::new("m", "twig");
        m.add_or_replace(f);
        lower_lisp_for_vm(&mut m);
        let f = &m.functions[0];
        assert_eq!(f.source_map.len(), f.instructions.len());
        let n = f.instructions.len();
        assert!(f.source_map[..n - 1].iter().all(|l| *l == SourceLoc::new(3, 1)), "the expansion is line 3");
        assert_eq!(f.source_map[n - 1], SourceLoc::new(4, 1));
    }

    #[test]
    fn invented_names_skip_every_name_already_in_the_function() {
        let f = lowered(vec![("__vm1_tag", "any")], vec![
            ins("const", Some("__vm2_notnil"), vec![Operand::Int(1)], "i64"),
            ins("jmp_if_false", None, vec![var("__vm1_tag"), var("__vm3_notfalse")], "void"),
        ]);
        let dests: Vec<&str> = f.instructions.iter().filter_map(|i| i.dest.as_deref()).collect();
        assert_eq!(dests.iter().filter(|d| **d == "__vm1_tag").count(), 0, "the parameter is never overwritten");
        assert_eq!(dests.iter().filter(|d| **d == "__vm2_notnil").count(), 1, "only the program's own write");
        assert!(!dests.contains(&"__vm3_notfalse"), "a label name is taken too");
    }

    #[test]
    fn lowering_twice_changes_nothing_more() {
        // Every rewrite at once, including a branch on a non-bool — whose
        // truthiness register must not be wrapped a second time (the integer
        // 0 a wrapped `k` would be is *truthy*, which would flip the branch).
        let body = vec![
            ins("const", Some("n"), vec![Operand::Int(0)], NIL_HINT),
            ins("const", Some("s"), vec![var("a")], SYMBOL_HINT),
            ins("is_null", Some("z"), vec![var("n")], "bool"),
            ins("call_builtin", Some("r"), vec![var("not"), var("n")], "any"),
            ins("call_builtin", Some("l"), vec![var("list?"), var("n")], "any"),
            ins("call_builtin", Some("q"), vec![var("modulo"), var("n"), var("n")], "any"),
            ins("jmp_if_false", None, vec![var("r"), var("L")], "void"),
            ins("jmp_if_false", None, vec![var("s"), var("L")], "void"),
        ];
        let once = lowered(vec![], body);
        let mut m = IIRModule::new("m", "twig");
        m.add_or_replace(once.clone());
        lower_lisp_for_vm(&mut m);
        assert_eq!(shape(&m.functions[0].instructions), shape(&once.instructions));
    }
}
