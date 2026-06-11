//! # Textual CIL (`.il`) emitter — the **real CoreCLR** path (CLR-real C1).
//!
//! The CLR backend's primary output ([`lower_iir_to_cil`](crate::lower_iir_to_cil))
//! is a [`CILProgramArtifact`](crate::CILProgramArtifact) — raw CIL method bodies
//! that the in-repo `clr-simulator` interprets. That gives zero-external-dep
//! verification, but the simulator can't *independently* catch a backend bug the
//! way a real runtime can.
//!
//! This module emits the **same program as textual CIL** (`.il`), which the real
//! `ilasm` assembles into a loadable PE assembly that runs on real CoreCLR
//! (`dotnet`). It is the exact CLR analog of [`iir-to-llvm`](../iir_to_llvm), which
//! emits textual LLVM IR and runs it through real `clang`: hand the *symbolic*
//! program to the real toolchain and let it own the metadata (PE headers, the
//! `#~`/`#Strings`/`#Blob` streams, token resolution). No hand-rolled metadata.
//!
//! ## Scope (C1–C4)
//!
//! * **C1 — scalar:** the entry function as a straight line of integer `const` /
//!   `mov` / `ret`; each register an `int32` local, a generated launcher prints
//!   the result so a runner reads it by running.
//! * **C2 — cons / car / cdr:** a cons cell is a 2-element `System.Object[]`
//!   (`alloc` → `newarr object`), atoms are `box`ed `System.Int32`, `field_*` are
//!   `stelem.ref`/`ldelem.ref`, with mixed-type locals (`object[]`/`object`/`int32`).
//! * **C3 — predicates + COND:** `pair?` → `isinst object[]; ldnull; ceq; ldc.i4.0;
//!   ceq`, `not` → `xor 1`, `equal?` → `unbox.any int32` ×2 + `ceq`; `COND` lowers
//!   to `label` (`<name>:`) / `jmp` (`br`) / `jmp_if_false` (`brfalse`), and a nil
//!   fall-through `const … : ref<…>` becomes `ldnull` (never `ldc.i4 0`).
//!
//! * **C4 — symbols:** no new ops. `intern_symbols_structural` lowers each `(QUOTE
//!   S)` to a *tagged integer id* (`A` → `0x20000000`, …), which on the CLR is just
//!   a boxed `System.Int32` atom — so `EQ`/`ATOM` on symbols reuse the C1–C3
//!   `const`/`box`/`equal?`/`pair?` path unchanged.
//!
//! Every other op returns [`IIRClrError::UnsupportedOp`], so the remaining slice
//! (lambda / LABEL) extends the op match incrementally — `ilasm` already handles
//! the metadata it will need.

use crate::lower::{IIRClrConfig, IIRClrError};
use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use std::collections::HashMap;
use std::fmt::Write as _;

/// The variable name at `instr.srcs[idx]`, or an `InvalidOperand` error.
fn var_src<'a>(
    f: &IIRFunction,
    instr: &'a IIRInstr,
    idx: usize,
    op: &str,
) -> Result<&'a str, IIRClrError> {
    match instr.srcs.get(idx) {
        Some(Operand::Var(s)) => Ok(s.as_str()),
        other => Err(IIRClrError::InvalidOperand {
            function: f.name.clone(),
            detail: format!("{op} src[{idx}] must be a variable, got {other:?}"),
        }),
    }
}

/// Validate a branch-target / label name before it is written **verbatim** into
/// the `.il` text that real `ilasm` assembles.
///
/// Unlike register operands — which never reach the output as names (they are
/// resolved to numeric `V_<slot>` indices) — a `label`/`jmp`/`jmp_if_*` target is
/// an `Operand::Var(String)`, an arbitrary unbounded string, emitted directly as
/// `<name>:` / `br <name>`. If that string carried whitespace, newlines, `}`, or
/// CIL directives (`.entrypoint`, `.method`, `//` comments…), a hostile IIR could
/// inject arbitrary CIL into the assembled program. We therefore fail **closed**:
/// only a non-empty run of `[A-Za-z0-9_$]` (a safe subset of legal CIL identifier
/// characters) is accepted; anything else is an `InvalidOperand`. The binary
/// emitter is immune by construction (it resolves labels to numeric offsets); this
/// gives the textual emitter the same guarantee. Synthetic COND labels
/// (`L_cond_next_<n>`) always pass; source-derived names (C4/C5 symbols, LABEL)
/// are checked here before they can reach `ilasm`.
fn checked_label<'a>(f: &IIRFunction, name: &'a str) -> Result<&'a str, IIRClrError> {
    let valid = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$');
    if valid {
        Ok(name)
    } else {
        Err(IIRClrError::InvalidOperand {
            function: f.name.clone(),
            detail: format!("label/branch target {name:?} is not a valid CIL identifier"),
        })
    }
}

/// The (int32-range-checked) integer literal at `instr.srcs[idx]`.
fn int_src(f: &IIRFunction, instr: &IIRInstr, idx: usize, op: &str) -> Result<i32, IIRClrError> {
    match instr.srcs.get(idx) {
        Some(Operand::Int(n)) => i32::try_from(*n).map_err(|_| IIRClrError::InvalidOperand {
            function: f.name.clone(),
            detail: format!("{op} index {n} out of int32 range"),
        }),
        other => Err(IIRClrError::InvalidOperand {
            function: f.name.clone(),
            detail: format!("{op} src[{idx}] must be an integer literal, got {other:?}"),
        }),
    }
}

/// Emit a complete, `ilasm`-assemblable `.il` source for `module`'s entry point.
///
/// The result defines one static class `<asm>Program` with two methods:
/// `MccarthyEntry()` (the lowered McCarthy program, returning `int32`) and `Run()`
/// (the `.entrypoint` launcher that prints the entry's result).
pub fn emit_il(module: &IIRModule, config: &IIRClrConfig) -> Result<String, IIRClrError> {
    let entry_name = module.entry_point.as_deref().unwrap_or("main");
    let entry = module
        .functions
        .iter()
        .find(|f| f.name == entry_name)
        .ok_or_else(|| IIRClrError::InvalidOperand {
            function: entry_name.to_string(),
            detail: "module has no entry-point function".to_string(),
        })?;

    let asm = &config.assembly_name;
    let mut il = String::new();
    // External assembly references the runtime + Console live in. `ilasm` resolves
    // these against the framework reference assemblies at assemble time.
    let _ = writeln!(il, ".assembly extern System.Runtime {{ .ver 9:0:0:0 }}");
    let _ = writeln!(il, ".assembly extern System.Console {{ .ver 9:0:0:0 }}");
    let _ = writeln!(il, ".assembly {asm} {{ }}");
    let _ = writeln!(il, ".module {asm}.dll");
    let _ = writeln!(
        il,
        ".class public auto ansi abstract sealed beforefieldinit {asm}Program \
         extends [System.Runtime]System.Object {{"
    );

    emit_entry_method(&mut il, entry, asm)?;

    // Launcher: `Console.WriteLine(MccarthyEntry())` so the result is observable by
    // running the assembly (matches how the BEAM/JVM e2e runners read a printout).
    let _ = writeln!(il, "  .method public static void Run() cil managed {{");
    let _ = writeln!(il, "    .entrypoint");
    let _ = writeln!(il, "    .maxstack 1");
    let _ = writeln!(il, "    call int32 {asm}Program::MccarthyEntry()");
    let _ = writeln!(
        il,
        "    call void [System.Console]System.Console::WriteLine(int32)"
    );
    let _ = writeln!(il, "    ret");
    let _ = writeln!(il, "  }}");
    let _ = writeln!(il, "}}");
    Ok(il)
}

/// The CIL local type for an IIR register, from the producing instruction's
/// `type_hint`. A cons cell (`ref<LispyPair>`) is a `System.Object[]`; a boxed
/// atom or a loaded field (`ref<any>`) is a `System.Object`; everything else is a
/// raw machine `int32` (the McCarthy CLR value model boxes ints into the cells).
fn cil_local_type(type_hint: &str) -> &'static str {
    if type_hint == "ref<LispyPair>" {
        "object[]"
    } else if type_hint.starts_with("ref<") {
        "object"
    } else {
        "int32"
    }
}

/// Emit `int32 MccarthyEntry()` from the entry function's instructions.
fn emit_entry_method(il: &mut String, f: &IIRFunction, _asm: &str) -> Result<(), IIRClrError> {
    // One local per distinct destination register, in first-seen order, typed from
    // the instruction that produces it (int32 / object / object[]).
    let mut slot_of: HashMap<&str, usize> = HashMap::new();
    let mut local_tys: Vec<&'static str> = Vec::new();
    for instr in &f.instructions {
        if let Some(dest) = &instr.dest {
            if !slot_of.contains_key(dest.as_str()) {
                slot_of.insert(dest.as_str(), local_tys.len());
                local_tys.push(cil_local_type(&instr.type_hint));
            }
        }
    }
    let slot = |name: &str| -> Result<usize, IIRClrError> {
        slot_of
            .get(name)
            .copied()
            .ok_or_else(|| IIRClrError::UndefinedVariable {
                function: f.name.clone(),
                name: name.to_string(),
            })
    };

    let _ = writeln!(il, "  .method public static int32 MccarthyEntry() cil managed {{");
    let _ = writeln!(il, "    .maxstack 8");
    if !local_tys.is_empty() {
        let locals: Vec<String> = local_tys
            .iter()
            .enumerate()
            .map(|(i, ty)| format!("{ty} V_{i}"))
            .collect();
        let _ = writeln!(il, "    .locals init ({})", locals.join(", "));
    }

    for instr in &f.instructions {
        match instr.op.as_str() {
            // const <dest> = Int(n)  →  ldc.i4 n; stloc V_<dest>
            //
            // A `const` whose *result type* is a reference (`ref<…>`) is the
            // McCarthy **nil** — an empty list is a null `object[]`. The structural
            // `COND` lowering emits `const <r> = 0 : ref<LispyPair>` for the
            // fall-through when no clause matched. Storing an `int32` into an
            // object-typed local is ill-typed CIL, so emit a genuine `ldnull` (the
            // canonical nil), never `ldc.i4 0`. (Mirrors the binary emitter's
            // `ldnull` nil case in `lower.rs`.)
            "const" => {
                let dest = instr.dest.as_deref().ok_or_else(|| IIRClrError::InvalidOperand {
                    function: f.name.clone(),
                    detail: "const must have a dest".to_string(),
                })?;
                if instr.type_hint.starts_with("ref<") {
                    // Only nil (0 / absent) is representable as a constant reference.
                    match instr.srcs.first() {
                        Some(Operand::Int(0)) | None => {}
                        other => {
                            return Err(IIRClrError::InvalidOperand {
                                function: f.name.clone(),
                                detail: format!(
                                    "const of reference type {:?} must be nil (0), got {other:?}",
                                    instr.type_hint
                                ),
                            })
                        }
                    }
                    let _ = writeln!(il, "    ldnull");
                    let _ = writeln!(il, "    stloc V_{}", slot(dest)?);
                } else {
                    let n = match instr.srcs.first() {
                        Some(Operand::Int(n)) => *n,
                        other => {
                            return Err(IIRClrError::InvalidOperand {
                                function: f.name.clone(),
                                detail: format!("const expects an integer literal, got {other:?}"),
                            })
                        }
                    };
                    // The CLR McCarthy model is `int32`; a scalar literal fits there.
                    let n32 = i32::try_from(n).map_err(|_| IIRClrError::InvalidOperand {
                        function: f.name.clone(),
                        detail: format!("integer literal {n} out of int32 range"),
                    })?;
                    let _ = writeln!(il, "    ldc.i4 {n32}");
                    let _ = writeln!(il, "    stloc V_{}", slot(dest)?);
                }
            }
            // mov <dest>, <src>  →  ldloc V_<src>; stloc V_<dest>
            "mov" => {
                let dest = instr.dest.as_deref().ok_or_else(|| IIRClrError::InvalidOperand {
                    function: f.name.clone(),
                    detail: "mov must have a dest".to_string(),
                })?;
                let src = match instr.srcs.first() {
                    Some(Operand::Var(s)) => s.as_str(),
                    other => {
                        return Err(IIRClrError::InvalidOperand {
                            function: f.name.clone(),
                            detail: format!("mov source must be a variable, got {other:?}"),
                        })
                    }
                };
                let _ = writeln!(il, "    ldloc V_{}", slot(src)?);
                let _ = writeln!(il, "    stloc V_{}", slot(dest)?);
            }
            // ret <src>  →  ldloc V_<src>; ret   (the entry returns int32)
            "ret" => {
                let src = match instr.srcs.first() {
                    Some(Operand::Var(s)) => s.as_str(),
                    other => {
                        return Err(IIRClrError::InvalidOperand {
                            function: f.name.clone(),
                            detail: format!("ret source must be a variable, got {other:?}"),
                        })
                    }
                };
                let _ = writeln!(il, "    ldloc V_{}", slot(src)?);
                let _ = writeln!(il, "    ret");
            }
            // alloc <dest> : ref<LispyPair>  →  a fresh 2-element System.Object[]
            //   ldc.i4.2; newarr [System.Runtime]System.Object; stloc V_<dest>
            // A McCarthy cons cell is a 2-slot reference array; the CLR GC owns it.
            "alloc" => {
                let dest = instr.dest.as_deref().ok_or_else(|| IIRClrError::InvalidOperand {
                    function: f.name.clone(),
                    detail: "alloc must have a dest".to_string(),
                })?;
                let _ = writeln!(il, "    ldc.i4.2");
                let _ = writeln!(il, "    newarr [System.Runtime]System.Object");
                let _ = writeln!(il, "    stloc V_{}", slot(dest)?);
            }
            // box <dest> = <src> : ref<any>  →  box a raw int32 atom into an object
            //   ldloc V_<src>; box [System.Runtime]System.Int32; stloc V_<dest>
            "box" => {
                let dest = instr.dest.as_deref().ok_or_else(|| IIRClrError::InvalidOperand {
                    function: f.name.clone(),
                    detail: "box must have a dest".to_string(),
                })?;
                let src = var_src(f, instr, 0, "box")?;
                let _ = writeln!(il, "    ldloc V_{}", slot(src)?);
                let _ = writeln!(il, "    box [System.Runtime]System.Int32");
                let _ = writeln!(il, "    stloc V_{}", slot(dest)?);
            }
            // unbox <dest> = <src> : i32  →  unbox an object back to a raw int32
            //   ldloc V_<src>; unbox.any [System.Runtime]System.Int32; stloc V_<dest>
            "unbox" => {
                let dest = instr.dest.as_deref().ok_or_else(|| IIRClrError::InvalidOperand {
                    function: f.name.clone(),
                    detail: "unbox must have a dest".to_string(),
                })?;
                let src = var_src(f, instr, 0, "unbox")?;
                let _ = writeln!(il, "    ldloc V_{}", slot(src)?);
                let _ = writeln!(il, "    unbox.any [System.Runtime]System.Int32");
                let _ = writeln!(il, "    stloc V_{}", slot(dest)?);
            }
            // field_store <arr>[<idx>] = <val>  (srcs = arr, Int(idx), val)
            //   ldloc V_<arr>; ldc.i4 <idx>; ldloc V_<val>; stelem.ref
            "field_store" => {
                let arr = var_src(f, instr, 0, "field_store")?;
                let idx = int_src(f, instr, 1, "field_store")?;
                let val = var_src(f, instr, 2, "field_store")?;
                let _ = writeln!(il, "    ldloc V_{}", slot(arr)?);
                let _ = writeln!(il, "    ldc.i4 {idx}");
                let _ = writeln!(il, "    ldloc V_{}", slot(val)?);
                let _ = writeln!(il, "    stelem.ref");
            }
            // field_load <dest> = <arr>[<idx>] : ref<any>  (srcs = arr, Int(idx))
            //   ldloc V_<arr>; ldc.i4 <idx>; ldelem.ref; stloc V_<dest>
            "field_load" => {
                let dest = instr.dest.as_deref().ok_or_else(|| IIRClrError::InvalidOperand {
                    function: f.name.clone(),
                    detail: "field_load must have a dest".to_string(),
                })?;
                let arr = var_src(f, instr, 0, "field_load")?;
                let idx = int_src(f, instr, 1, "field_load")?;
                let _ = writeln!(il, "    ldloc V_{}", slot(arr)?);
                let _ = writeln!(il, "    ldc.i4 {idx}");
                let _ = writeln!(il, "    ldelem.ref");
                let _ = writeln!(il, "    stloc V_{}", slot(dest)?);
            }
            // ── Control flow (COND lowers to label / jmp / jmp_if_*) ──────────
            //
            // McCarthy `COND` is lowered by the shared structural pass to a chain
            // of `jmp_if_false`/`jmp` over `label`s. CIL labels are not opcodes —
            // they are named positions in the byte stream — so a `label` emits a
            // `<name>:` anchor and the branches reference it by name. `ilasm`
            // resolves every name to the right offset.
            //
            // label <name>  →  `<name>:`
            "label" => {
                let name = checked_label(f, var_src(f, instr, 0, "label")?)?;
                let _ = writeln!(il, "  {name}:");
            }
            // jmp <label>  →  br <label>   (unconditional branch)
            "jmp" => {
                let label = checked_label(f, var_src(f, instr, 0, "jmp")?)?;
                let _ = writeln!(il, "    br {label}");
            }
            // jmp_if_false <cond>, <label>  →  ldloc cond; brfalse <label>
            "jmp_if_false" => {
                let cond = var_src(f, instr, 0, "jmp_if_false")?;
                let label = checked_label(f, var_src(f, instr, 1, "jmp_if_false")?)?;
                let _ = writeln!(il, "    ldloc V_{}", slot(cond)?);
                let _ = writeln!(il, "    brfalse {label}");
            }
            // jmp_if_true <cond>, <label>  →  ldloc cond; brtrue <label>
            "jmp_if_true" => {
                let cond = var_src(f, instr, 0, "jmp_if_true")?;
                let label = checked_label(f, var_src(f, instr, 1, "jmp_if_true")?)?;
                let _ = writeln!(il, "    ldloc V_{}", slot(cond)?);
                let _ = writeln!(il, "    brtrue {label}");
            }
            // ── McCarthy predicate primitives (call_builtin) ──────────────────
            //
            // The structural pass decomposes the source predicates into three
            // boolean builtins, each a small CIL idiom (the CLR twins of the JVM
            // `instanceof`/`ixor`/`if_icmpeq` and the wasm `ref.test`/`i32.eqz`/
            // `i32.eq`). Mirrors the binary emitter's `call_builtin` arm in
            // `lower.rs`.
            //
            // | builtin  | layout                            | CIL |
            // |----------|-----------------------------------|-----|
            // | `pair?`  | [Var("pair?"), Var(x)]; dest      | `ldloc x; isinst object[]; ldnull; ceq; ldc.i4.0; ceq` |
            // | `not`    | [Var("not"), Var(x)]; dest        | `ldloc x; ldc.i4.1; xor` |
            // | `equal?` | [Var("equal?"), Var(a), Var(b)]   | `ldloc a; unbox.any int32; ldloc b; unbox.any int32; ceq` |
            "call_builtin" => {
                let dest = instr.dest.as_deref().ok_or_else(|| IIRClrError::InvalidOperand {
                    function: f.name.clone(),
                    detail: "call_builtin must have a dest".to_string(),
                })?;
                let builtin = var_src(f, instr, 0, "call_builtin")?;
                match builtin {
                    // Is the (boxed) lisp value a cons cell? A cons is an `object[]`
                    // (heap ref); an atom is a boxed int; nil is null. `isinst`
                    // leaves the ref or null; the two `ceq`s turn that into a clean
                    // 1 (pair) / 0 (not): the first `ceq ldnull` answers "is it null
                    // (≠ pair)?", the second (`== 0`) flips it back to "was a pair".
                    "pair?" => {
                        let arg = var_src(f, instr, 1, "pair?")?;
                        let _ = writeln!(il, "    ldloc V_{}", slot(arg)?);
                        let _ = writeln!(il, "    isinst object[]");
                        let _ = writeln!(il, "    ldnull");
                        let _ = writeln!(il, "    ceq");
                        let _ = writeln!(il, "    ldc.i4.0");
                        let _ = writeln!(il, "    ceq");
                        let _ = writeln!(il, "    stloc V_{}", slot(dest)?);
                    }
                    // Logical not of a 0/1 bool: `x ^ 1`. (Distinct from a machine
                    // bitwise-complement `not`; this is McCarthy's boolean not.)
                    "not" => {
                        let arg = var_src(f, instr, 1, "not")?;
                        let _ = writeln!(il, "    ldloc V_{}", slot(arg)?);
                        let _ = writeln!(il, "    ldc.i4.1");
                        let _ = writeln!(il, "    xor");
                        let _ = writeln!(il, "    stloc V_{}", slot(dest)?);
                    }
                    // `EQ` on atoms: unbox both and compare. The structural pass
                    // guarantees both args are boxed atoms (symbols interned to ints,
                    // integers as ints), so identity reduces to integer equality.
                    "equal?" => {
                        let a = var_src(f, instr, 1, "equal?")?;
                        let b = var_src(f, instr, 2, "equal?")?;
                        let _ = writeln!(il, "    ldloc V_{}", slot(a)?);
                        let _ = writeln!(il, "    unbox.any [System.Runtime]System.Int32");
                        let _ = writeln!(il, "    ldloc V_{}", slot(b)?);
                        let _ = writeln!(il, "    unbox.any [System.Runtime]System.Int32");
                        let _ = writeln!(il, "    ceq");
                        let _ = writeln!(il, "    stloc V_{}", slot(dest)?);
                    }
                    other => {
                        return Err(IIRClrError::UnsupportedOp {
                            function: f.name.clone(),
                            op: format!("call_builtin {other:?}"),
                        })
                    }
                }
            }
            // Later slices extend this match (symbols, lambda).
            other => {
                return Err(IIRClrError::UnsupportedOp {
                    function: f.name.clone(),
                    op: other.to_string(),
                })
            }
        }
    }

    let _ = writeln!(il, "  }}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::{IIRFunction, IIRInstr, IIRModule};

    fn scalar_module(n: i64) -> IIRModule {
        let instrs = vec![
            IIRInstr::new("const", Some("v0".into()), vec![Operand::Int(n)], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "any"),
        ];
        let mut m = IIRModule::new("Main", "mccarthy-lisp");
        m.functions.push(IIRFunction::new("main", vec![], "any", instrs));
        m.entry_point = Some("main".into());
        m
    }

    #[test]
    fn scalar_emits_well_formed_il() {
        let il = emit_il(&scalar_module(42), &IIRClrConfig::new("Main")).unwrap();
        assert!(il.contains(".entrypoint"), "must declare a process entry point");
        assert!(il.contains("int32 MccarthyEntry()"), "entry method returns int32");
        assert!(il.contains("ldc.i4 42"), "loads the literal 42; got:\n{il}");
        assert!(il.contains("System.Console]System.Console::WriteLine(int32)"));
    }

    #[test]
    fn cons_car_emits_object_array_box_and_unbox() {
        // The lowered shape of `(CAR (CONS 7 9))` (C2): a 2-element object[] cons
        // cell, boxed int atoms, ldelem.ref for CAR, unbox.any for the int result.
        let instrs = vec![
            IIRInstr::new("const", Some("v0".into()), vec![Operand::Int(7)], "i32"),
            IIRInstr::new("const", Some("v1".into()), vec![Operand::Int(9)], "i32"),
            IIRInstr::new("alloc", Some("v2".into()), vec![], "ref<LispyPair>"),
            IIRInstr::new("box", Some("v0b".into()), vec![Operand::Var("v0".into())], "ref<any>"),
            IIRInstr::new(
                "field_store",
                None,
                vec![Operand::Var("v2".into()), Operand::Int(0), Operand::Var("v0b".into())],
                "void",
            ),
            IIRInstr::new("box", Some("v1b".into()), vec![Operand::Var("v1".into())], "ref<any>"),
            IIRInstr::new(
                "field_store",
                None,
                vec![Operand::Var("v2".into()), Operand::Int(1), Operand::Var("v1b".into())],
                "void",
            ),
            IIRInstr::new(
                "field_load",
                Some("v3".into()),
                vec![Operand::Var("v2".into()), Operand::Int(0)],
                "ref<any>",
            ),
            IIRInstr::new("unbox", Some("v3u".into()), vec![Operand::Var("v3".into())], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("v3u".into())], "i32"),
        ];
        let mut m = IIRModule::new("Main", "mccarthy-lisp");
        m.functions.push(IIRFunction::new("main", vec![], "i32", instrs));
        m.entry_point = Some("main".into());

        let il = emit_il(&m, &IIRClrConfig::new("Main")).unwrap();
        assert!(il.contains("newarr [System.Runtime]System.Object"), "cons → object[]:\n{il}");
        assert!(il.contains("box [System.Runtime]System.Int32"), "atoms are boxed");
        assert!(il.contains("stelem.ref"), "field_store → stelem.ref");
        assert!(il.contains("ldelem.ref"), "CAR → ldelem.ref");
        assert!(il.contains("unbox.any [System.Runtime]System.Int32"), "result unboxed");
        // The cons cell local is typed object[], the boxed atoms object, ints int32.
        assert!(il.contains("object[] V_2"), "cons cell local is object[]; got:\n{il}");
        assert!(il.contains("object V_3"), "loaded field local is object");
    }

    #[test]
    fn atom_emits_isinst_xor_predicate_chain() {
        // `(ATOM 7)` lowers to `not (pair? (box 7))`: box the int, test it against
        // object[], collapse to a 0/1 bool, then xor-1 to negate.
        let instrs = vec![
            IIRInstr::new("const", Some("v0".into()), vec![Operand::Int(7)], "i32"),
            IIRInstr::new("box", Some("v0b".into()), vec![Operand::Var("v0".into())], "ref<any>"),
            IIRInstr::new(
                "call_builtin",
                Some("v1".into()),
                vec![Operand::Var("pair?".into()), Operand::Var("v0b".into())],
                "bool",
            ),
            IIRInstr::new(
                "call_builtin",
                Some("v2".into()),
                vec![Operand::Var("not".into()), Operand::Var("v1".into())],
                "bool",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("v2".into())], "i32"),
        ];
        let mut m = IIRModule::new("Main", "mccarthy-lisp");
        m.functions.push(IIRFunction::new("main", vec![], "i32", instrs));
        m.entry_point = Some("main".into());

        let il = emit_il(&m, &IIRClrConfig::new("Main")).unwrap();
        // pair?: isinst object[]; ldnull; ceq; ldc.i4.0; ceq
        assert!(il.contains("isinst object[]"), "pair? → isinst object[]; got:\n{il}");
        assert!(il.contains("ceq"), "pair? collapses ref/null to a bool with ceq");
        // not: ldc.i4.1; xor
        assert!(il.contains("ldc.i4.1\n    xor"), "not → xor 1; got:\n{il}");
        // The bool register is a raw int32 local.
        assert!(il.contains("int32 V_"), "predicate result is an int32 local");
    }

    #[test]
    fn eq_emits_double_unbox_then_ceq() {
        // `(EQ 7 7)` lowers to `equal? (box 7) (box 7)`: unbox both, compare.
        let instrs = vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(7)], "i32"),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Int(7)], "i32"),
            IIRInstr::new("box", Some("ab".into()), vec![Operand::Var("a".into())], "ref<any>"),
            IIRInstr::new("box", Some("bb".into()), vec![Operand::Var("b".into())], "ref<any>"),
            IIRInstr::new(
                "call_builtin",
                Some("r".into()),
                vec![
                    Operand::Var("equal?".into()),
                    Operand::Var("ab".into()),
                    Operand::Var("bb".into()),
                ],
                "bool",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ];
        let mut m = IIRModule::new("Main", "mccarthy-lisp");
        m.functions.push(IIRFunction::new("main", vec![], "i32", instrs));
        m.entry_point = Some("main".into());

        let il = emit_il(&m, &IIRClrConfig::new("Main")).unwrap();
        assert_eq!(
            il.matches("unbox.any [System.Runtime]System.Int32").count(),
            2,
            "equal? unboxes both operands; got:\n{il}"
        );
        assert!(il.contains("ceq"), "equal? → ceq");
    }

    #[test]
    fn symbol_eq_emits_tagged_id_consts_unboxed_and_compared() {
        // `(EQ (QUOTE A) (QUOTE A))` after `intern_symbols_structural`: each symbol
        // is a *tagged integer id* (here `A` → 0x20000000 = 536870912), boxed as an
        // atom — the exact scalar/predicate shape C1–C3 already emit. Symbols need
        // NO new CIL ops: two equal `ldc.i4 <id>`, boxed, then `equal?`-unboxed.
        const SYM_A: i64 = 0x2000_0000; // 536870912, the interned id of `A`
        let instrs = vec![
            IIRInstr::new("const", Some("v0".into()), vec![Operand::Int(SYM_A)], "i32"),
            IIRInstr::new("const", Some("v1".into()), vec![Operand::Int(SYM_A)], "i32"),
            IIRInstr::new("box", Some("v0b".into()), vec![Operand::Var("v0".into())], "ref<any>"),
            IIRInstr::new("box", Some("v1b".into()), vec![Operand::Var("v1".into())], "ref<any>"),
            IIRInstr::new(
                "call_builtin",
                Some("r".into()),
                vec![
                    Operand::Var("equal?".into()),
                    Operand::Var("v0b".into()),
                    Operand::Var("v1b".into()),
                ],
                "bool",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ];
        let mut m = IIRModule::new("Main", "mccarthy-lisp");
        m.functions.push(IIRFunction::new("main", vec![], "i32", instrs));
        m.entry_point = Some("main".into());

        let il = emit_il(&m, &IIRClrConfig::new("Main")).unwrap();
        assert!(il.contains("ldc.i4 536870912"), "tagged symbol id is a const; got:\n{il}");
        assert!(il.contains("box [System.Runtime]System.Int32"), "symbol id boxed as an atom");
        assert_eq!(
            il.matches("unbox.any [System.Runtime]System.Int32").count(),
            2,
            "equal? unboxes both symbol ids"
        );
    }

    #[test]
    fn cond_emits_branches_labels_and_nil_fallthrough() {
        // A minimal COND skeleton: branch on a bool to a label, an unconditional
        // jump to the end, and a nil (`const 0 : ref<LispyPair>`) fall-through that
        // must become `ldnull` (not `ldc.i4 0`) so the object-typed local is sound.
        let instrs = vec![
            IIRInstr::new("const", Some("c".into()), vec![Operand::Int(1)], "bool"),
            IIRInstr::new(
                "jmp_if_false",
                None,
                vec![Operand::Var("c".into()), Operand::Var("L_next".into())],
                "void",
            ),
            IIRInstr::new("const", Some("r".into()), vec![Operand::Int(11)], "i32"),
            IIRInstr::new("jmp", None, vec![Operand::Var("L_end".into())], "void"),
            IIRInstr::new("label", None, vec![Operand::Var("L_next".into())], "void"),
            IIRInstr::new("const", Some("nil".into()), vec![Operand::Int(0)], "ref<LispyPair>"),
            IIRInstr::new("label", None, vec![Operand::Var("L_end".into())], "void"),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ];
        let mut m = IIRModule::new("Main", "mccarthy-lisp");
        m.functions.push(IIRFunction::new("main", vec![], "i32", instrs));
        m.entry_point = Some("main".into());

        let il = emit_il(&m, &IIRClrConfig::new("Main")).unwrap();
        assert!(il.contains("brfalse L_next"), "jmp_if_false → brfalse; got:\n{il}");
        assert!(il.contains("br L_end"), "jmp → br");
        assert!(il.contains("L_next:"), "label → named anchor");
        assert!(il.contains("L_end:"), "label → named anchor");
        // The nil const must be ldnull, and its local must be object[] (a list).
        assert!(il.contains("ldnull"), "const-of-ref-type nil → ldnull; got:\n{il}");
        assert!(il.contains("object[] V_"), "nil local is object[]");
        // It must NOT store an int 0 into the reference local.
        assert!(!il.contains("ldc.i4 0\n    stloc"), "nil must not be ldc.i4 0");
    }

    #[test]
    fn malicious_label_name_is_rejected_not_injected() {
        // A hostile IIR label carrying CIL directives / newlines must NOT reach the
        // `.il` text — `checked_label` fails closed on any non-identifier character.
        for bad in [
            "L_end\n    .entrypoint\n  ", // newline + directive injection
            "L }",                        // brace closes the method early
            "L_end // comment",           // line comment
            "",                           // empty
        ] {
            let instrs = vec![
                IIRInstr::new("label", None, vec![Operand::Var(bad.into())], "void"),
                IIRInstr::new("const", Some("v0".into()), vec![Operand::Int(1)], "i32"),
                IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
            ];
            let mut m = IIRModule::new("Main", "mccarthy-lisp");
            m.functions.push(IIRFunction::new("main", vec![], "i32", instrs));
            m.entry_point = Some("main".into());
            let err = emit_il(&m, &IIRClrConfig::new("Main")).unwrap_err();
            assert!(
                matches!(err, IIRClrError::InvalidOperand { .. }),
                "label {bad:?} must be rejected, got {err:?}"
            );
        }
        // A legitimate synthetic COND label still passes.
        let instrs = vec![
            IIRInstr::new("label", None, vec![Operand::Var("L_cond_next_1".into())], "void"),
            IIRInstr::new("const", Some("v0".into()), vec![Operand::Int(1)], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
        ];
        let mut m = IIRModule::new("Main", "mccarthy-lisp");
        m.functions.push(IIRFunction::new("main", vec![], "i32", instrs));
        m.entry_point = Some("main".into());
        assert!(emit_il(&m, &IIRClrConfig::new("Main")).unwrap().contains("L_cond_next_1:"));
    }

    #[test]
    fn const_of_reference_type_rejects_non_nil() {
        // A non-zero constant of reference type is not representable (only nil is a
        // constant ref) — reject rather than emit an ill-typed store.
        let instrs = vec![
            IIRInstr::new("const", Some("x".into()), vec![Operand::Int(5)], "ref<LispyPair>"),
            IIRInstr::new("ret", None, vec![Operand::Var("x".into())], "i32"),
        ];
        let mut m = IIRModule::new("Main", "mccarthy-lisp");
        m.functions.push(IIRFunction::new("main", vec![], "i32", instrs));
        m.entry_point = Some("main".into());
        let err = emit_il(&m, &IIRClrConfig::new("Main")).unwrap_err();
        assert!(matches!(err, IIRClrError::InvalidOperand { .. }));
    }

    #[test]
    fn unsupported_op_is_rejected_not_emitted() {
        // A `call_builtin` to a name outside the CLR whitelist (only pair?/not/
        // equal? are emittable today) must be rejected explicitly, not emit junk —
        // e.g. `lispy_cons` is a heap builtin handled structurally, never here.
        let mut m = scalar_module(1);
        m.functions[0].instructions.insert(
            0,
            IIRInstr::new(
                "call_builtin",
                Some("p".into()),
                vec![Operand::Var("lispy_cons".into())],
                "ref<LispyPair>",
            ),
        );
        let err = emit_il(&m, &IIRClrConfig::new("Main")).unwrap_err();
        assert!(matches!(err, IIRClrError::UnsupportedOp { .. }));
    }
}
