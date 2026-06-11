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
//! ## Scope (C1)
//!
//! Scalar McCarthy — the entry function as a straight line of integer `const` /
//! `mov` / `ret`. Each register becomes an `int32` local; the entry computes an
//! `int32`, and a generated launcher prints it so a runner reads the result by
//! running. Every other op returns [`IIRClrError::UnsupportedOp`], so later slices
//! (cons → `newarr object`, predicates, `COND`, symbols, lambda) extend the op
//! match incrementally — `ilasm` already handles the metadata each will need.

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
            "const" => {
                let dest = instr.dest.as_deref().ok_or_else(|| IIRClrError::InvalidOperand {
                    function: f.name.clone(),
                    detail: "const must have a dest".to_string(),
                })?;
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
            // Later slices extend this match (predicates, COND, symbols, lambda).
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
    fn unsupported_op_is_rejected_not_emitted() {
        // A predicate (`call_builtin`) is C3 — C2 must reject it explicitly, not emit junk.
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
