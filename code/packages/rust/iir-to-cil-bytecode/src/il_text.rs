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
//! ## Scope (C1–C5)
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
//! * **C4 — symbols:** no new ops. `intern_symbols_structural` lowers each `(QUOTE
//!   S)` to a *tagged integer id* (`A` → `0x20000000`, …), which on the CLR is just
//!   a boxed `System.Int32` atom — so `EQ`/`ATOM` on symbols reuse the C1–C3
//!   `const`/`box`/`equal?`/`pair?` path unchanged.
//! * **C5 — lambda / LABEL / recursion:** the module is now **multi-function**.
//!   Every IIR function becomes its own static `.method` (the entry → `MccarthyEntry`,
//!   the rest keep their hoisted names `lambda_<n>`/`label_<n>`); a `call` is a
//!   by-name `call <ret> <Class>::<m>(<argtys>)` (`ilasm` resolves the token), so
//!   self-recursive `LABEL` is just a method calling itself. Parameters live in
//!   `ldarg`/`starg` slots (locals stay in `ldloc`/`stloc`); `is_null` is `ldnull;
//!   ceq`; and a `field_*` whose array operand is statically `object` (a lambda
//!   parameter, not a freshly-`alloc`-ed `object[]`) gets a `castclass object[]`
//!   first — real CoreCLR's `ldelem.ref`/`stelem.ref` require an array on the stack,
//!   a constraint the lenient simulator never enforced.
//!
//! Every other op returns [`IIRClrError::UnsupportedOp`]; `ilasm` already owns the
//! metadata each emitted construct needs.

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

/// Validate that `name` is a safe CIL identifier before it is written **verbatim**
/// into the `.il` text (a `label`/branch target, or a `.method`/`call` name).
///
/// These names — unlike register operands, which are resolved to numeric
/// `V_<slot>`/`ldarg <n>` and never reach the text — come from `Operand::Var`
/// strings, i.e. arbitrary unbounded input. If one carried whitespace, newlines,
/// `}`, or CIL directives (`.entrypoint`, `.method`, `//` comments…), a hostile IIR
/// could inject arbitrary CIL into the assembled program. We fail **closed**: only
/// a non-empty run of `[A-Za-z0-9_$]` (a safe subset of legal CIL identifier
/// characters) is accepted. The binary emitter is immune by construction (numeric
/// offsets / tokens); this gives the textual emitter the same guarantee. Synthetic
/// names (`L_cond_next_<n>`, `lambda_<n>`, `label_<n>`, `main`) always pass.
fn checked_cil_ident<'a>(ctx: &str, name: &'a str) -> Result<&'a str, IIRClrError> {
    let valid = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$');
    if valid {
        Ok(name)
    } else {
        Err(IIRClrError::InvalidOperand {
            function: ctx.to_string(),
            detail: format!("CIL identifier {name:?} contains an illegal character"),
        })
    }
}

/// A `label`/branch target name, validated for the current function's context.
fn checked_label<'a>(f: &IIRFunction, name: &'a str) -> Result<&'a str, IIRClrError> {
    checked_cil_ident(&f.name, name)
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

/// The CIL local type for an IIR register, from the producing instruction's
/// `type_hint`. A cons cell (`ref<LispyPair>`) is a `System.Object[]`; a boxed
/// atom or a loaded field (`ref<any>`) is a `System.Object`; everything else is a
/// raw machine `int32` (the McCarthy CLR value model boxes ints into the cells).
fn cil_local_type(type_hint: &str) -> &'static str {
    if type_hint == "ref<LispyPair>" {
        "object[]"
    } else if type_hint.starts_with("ref<") {
        "object"
    } else if type_hint == "f64" {
        // ALGOL `real` (LANG-FULL E3): an IEEE-754 double. The `.locals`
        // declaration carries `float64`, and an `f64`-typed register is loaded/
        // stored with the same `ldloc`/`stloc` (CIL stack slots are typed by the
        // local signature, not the instruction). CIL's `add`/`sub`/`mul`/`div`
        // and `ceq`/`cgt`/`clt` are stack-type-overloaded, so they operate on
        // `float64` operands with no opcode change.
        "float64"
    } else if type_hint == "f32" {
        "float32"
    } else {
        "int32"
    }
}

/// Is `op` one of the six IIR comparison ops? Their result is always a 0/1
/// `int32` (CIL `ceq`/`cgt`/`clt`), independent of the operand width carried by
/// `type_hint` — which matters for `float64` operands (LANG-FULL E3).
fn is_comparison_op(op: &str) -> bool {
    matches!(
        op,
        "cmp_eq" | "cmp_ne" | "cmp_lt" | "cmp_le" | "cmp_gt" | "cmp_ge"
    )
}

/// The resolved entry-point function name — the module's `entry_point`, or `"main"`
/// as the fallback. Resolved **once** here and used everywhere (the early existence
/// check, the `MccarthyEntry` rename, `is_entry`, and the `call` callee) so the
/// launcher's hardcoded `call …::MccarthyEntry()` can never dangle on a `None`
/// `entry_point`.
fn entry_name(module: &IIRModule) -> &str {
    module.entry_point.as_deref().unwrap_or("main")
}

/// The CIL return type for an IIR function. The entry returns `int32` (the printed
/// program result); a hoisted lambda/label returns its IIR `return_type` mapped by
/// [`cil_local_type`] (McCarthy lambdas return a lisp value → `object`).
fn cil_ret_type(module: &IIRModule, f: &IIRFunction) -> &'static str {
    if f.name == entry_name(module) {
        "int32"
    } else {
        cil_local_type(&f.return_type)
    }
}

/// Where an IIR register lives in a CIL method body: a method **argument**
/// (`ldarg`/`starg`) or a **local** (`ldloc`/`stloc`). Before C5 every register was
/// a local; McCarthy lambda/LABEL functions take parameters, so a register can now
/// be an argument too.
struct RegHome {
    is_param: bool,
    slot: usize,
    ty: &'static str,
}

/// Per-function register allocation: parameters get argument slots (in declaration
/// order), every other distinct destination register gets a local slot (first-seen
/// order), each typed by [`cil_local_type`].
struct FnRegs {
    fn_name: String,
    homes: HashMap<String, RegHome>,
    local_tys: Vec<&'static str>,
}

impl FnRegs {
    fn build(f: &IIRFunction) -> FnRegs {
        let mut homes: HashMap<String, RegHome> = HashMap::new();
        for (i, (pname, pty)) in f.params.iter().enumerate() {
            homes.entry(pname.clone()).or_insert(RegHome {
                is_param: true,
                slot: i,
                ty: cil_local_type(pty),
            });
        }
        // A register that is the dest of `alloc_bytes` is a Brainfuck byte tape, an
        // `unsigned int8[]` — not the scalar `int32` its (concretised) `type_hint`
        // would suggest. Type it so the `.locals` declaration matches the `newarr
        // [System.Runtime]System.Byte` + `ldelem.u1`/`stelem.i1` access (LM-C Brainfuck).
        let tape_vars: std::collections::HashSet<&str> = f
            .instructions
            .iter()
            .filter(|i| i.op == "alloc_bytes")
            .filter_map(|i| i.dest.as_deref())
            .collect();
        let mut local_tys: Vec<&'static str> = Vec::new();
        for instr in &f.instructions {
            if let Some(dest) = &instr.dest {
                if !homes.contains_key(dest) {
                    let ty = if tape_vars.contains(dest.as_str()) {
                        "unsigned int8[]"
                    } else if is_comparison_op(&instr.op) {
                        // A comparison ALWAYS yields a 0/1 `int32` (`ceq`/`cgt`/
                        // `clt`), even over `float64` operands (whose `type_hint`
                        // is the operand width `f64`, not the result type). Typing
                        // the result `float64` would `stloc` an `int32` into a
                        // `float64` local — an ill-typed slot (LANG-FULL E3). For
                        // integer comparisons this is already `int32` via
                        // `cil_local_type`; floats are the case that needs the
                        // override.
                        "int32"
                    } else {
                        cil_local_type(&instr.type_hint)
                    };
                    homes.insert(
                        dest.clone(),
                        RegHome {
                            is_param: false,
                            slot: local_tys.len(),
                            ty,
                        },
                    );
                    local_tys.push(ty);
                }
            }
        }
        FnRegs {
            fn_name: f.name.clone(),
            homes,
            local_tys,
        }
    }

    fn home(&self, name: &str) -> Result<&RegHome, IIRClrError> {
        self.homes
            .get(name)
            .ok_or_else(|| IIRClrError::UndefinedVariable {
                function: self.fn_name.clone(),
                name: name.to_string(),
            })
    }
}

/// Emit a load (`ldarg`/`ldloc`) of `name` onto the CIL stack.
fn load_var(il: &mut String, regs: &FnRegs, name: &str) -> Result<(), IIRClrError> {
    let h = regs.home(name)?;
    if h.is_param {
        let _ = writeln!(il, "    ldarg {}", h.slot);
    } else {
        let _ = writeln!(il, "    ldloc V_{}", h.slot);
    }
    Ok(())
}

/// Emit `ldc.i4 <mask>; and` to wrap a just-computed narrow-width result back
/// into its declared width — the textual-`.il` twin of the bytecode path's
/// [`crate::lower::emit_narrow_width_mask`].
///
/// LANG-FULL **E2 — register width & wrap**.  A CIL `add`/`mul`/`shl`/… runs on
/// a full 32-bit slot, so `200u8 + 100u8` lands as `300` on the stack; the
/// `u8` contract requires it to wrap to `300 & 0xFF = 44`.  After the op we
/// AND-mask the result down to the width:
///
/// | type_hint | emits                | wraps                         |
/// |-----------|----------------------|-------------------------------|
/// | `u4`      | `ldc.i4 0xF; and`    | `15u4 + 1u4` → `0`            |
/// | `u8`      | `ldc.i4 0xFF; and`   | `200u8 + 100u8` → `44`        |
/// | `u16`     | `ldc.i4 0xFFFF; and` | `~0u16` → `65535`            |
/// | `u32`,`i32`,`i64`,… | *(nothing)* | the 32-bit op already wraps   |
///
/// A positive mask + `and` (not `conv.u1`, which would sign-extend) keeps the
/// unsigned widths unsigned — identical semantics to the JVM `iand` and wasm
/// `i32.and` masks.
fn emit_narrow_width_mask(il: &mut String, type_hint: &str) {
    let mask: i64 = match type_hint {
        "u4" => 0xF,
        "u8" => 0xFF,
        "u16" => 0xFFFF,
        _ => return, // u32/i32 wrap via the 32-bit op; wider/signed unchanged
    };
    let _ = writeln!(il, "    ldc.i4 0x{mask:X}");
    let _ = writeln!(il, "    and");
}

/// Emit a store (`starg`/`stloc`) popping the top of the CIL stack into `name`.
fn store_var(il: &mut String, regs: &FnRegs, name: &str) -> Result<(), IIRClrError> {
    let h = regs.home(name)?;
    if h.is_param {
        let _ = writeln!(il, "    starg {}", h.slot);
    } else {
        let _ = writeln!(il, "    stloc V_{}", h.slot);
    }
    Ok(())
}

/// Emit a complete, `ilasm`-assemblable `.il` source for `module`.
///
/// The result defines one static class `<asm>Program` whose methods are: every IIR
/// function (the entry as `MccarthyEntry()` returning `int32`, each hoisted
/// lambda/label as `<name>(…)`), plus the `Run()` `.entrypoint` launcher that prints
/// `MccarthyEntry()`'s result so a runner reads it by running.
pub fn emit_il(module: &IIRModule, config: &IIRClrConfig) -> Result<String, IIRClrError> {
    let entry = entry_name(module);
    if !module.functions.iter().any(|f| f.name == entry) {
        return Err(IIRClrError::InvalidOperand {
            function: entry.to_string(),
            detail: "module has no entry-point function".to_string(),
        });
    }

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

    // Every IIR function becomes a static method. Order is irrelevant — `ilasm`
    // resolves `call`s by name across the whole class, so forward references and
    // self-recursion just work.
    for f in &module.functions {
        emit_method(&mut il, module, f, asm)?;
    }

    // Launcher. For an **expression** program the result is the program — so we
    // `Console.WriteLine(MccarthyEntry())` to make it observable by running the
    // assembly (matches how the BEAM/JVM e2e runners read a printout). For an **I/O**
    // program (one that calls `print_i64` — Dartmouth BASIC's `PRINT`) the program has
    // already written its own output as a side effect inside `MccarthyEntry`, so the
    // launcher merely runs it and **discards** the (unused) `int32` return with `pop`,
    // rather than printing it a second time. (The entry's CIL return type is always
    // `int32`, so the `call`/`pop` are well-typed either way.)
    // A program "prints" (writes its own stdout as a side effect) if it calls
    // `print_i64` (Dartmouth BASIC's `PRINT`) **or** `putchar` (Brainfuck's `.`).
    // For such a program the launcher discards `MccarthyEntry`'s result instead of
    // `Console.WriteLine`-ing it — otherwise a Brainfuck program would print both its
    // own output and its (meaningless) `int32` exit value (a double-print).
    let prints = module.functions.iter().any(|f| {
        f.instructions.iter().any(|i| {
            i.op == "call_builtin"
                && matches!(i.srcs.first(),
                    Some(Operand::Var(n)) if n == "print_i64" || n == "putchar")
        })
    });
    let _ = writeln!(il, "  .method public static void Run() cil managed {{");
    let _ = writeln!(il, "    .entrypoint");
    let _ = writeln!(il, "    .maxstack 1");
    let _ = writeln!(il, "    call int32 {asm}Program::MccarthyEntry()");
    if prints {
        let _ = writeln!(il, "    pop");
    } else {
        let _ = writeln!(
            il,
            "    call void [System.Console]System.Console::WriteLine(int32)"
        );
    }
    let _ = writeln!(il, "    ret");
    let _ = writeln!(il, "  }}");
    let _ = writeln!(il, "}}");
    Ok(il)
}

/// Emit one static `.method` for IIR function `f`.
fn emit_method(
    il: &mut String,
    module: &IIRModule,
    f: &IIRFunction,
    asm: &str,
) -> Result<(), IIRClrError> {
    let is_entry = f.name == entry_name(module);
    // The entry's CIL name is the fixed, safe `MccarthyEntry`; a hoisted function's
    // name is emitted verbatim, so validate it (injection guard).
    let method_name = if is_entry {
        "MccarthyEntry".to_string()
    } else {
        checked_cil_ident(&f.name, &f.name)?.to_string()
    };
    let ret_ty = cil_ret_type(module, f);
    let regs = FnRegs::build(f);

    // Method signature: parameters as `<ty> A_<i>` (the name is cosmetic — the body
    // addresses them by `ldarg <i>`).
    let sig: Vec<String> = f
        .params
        .iter()
        .enumerate()
        .map(|(i, (_, pty))| format!("{} A_{i}", cil_local_type(pty)))
        .collect();
    let _ = writeln!(
        il,
        "  .method public static {ret_ty} {method_name}({}) cil managed {{",
        sig.join(", ")
    );

    // A `call` pushes all its arguments before the call instruction, so the stack
    // can grow to (arg count + a little working space). Keep a floor of 8 (covers
    // the predicate/cons idioms) and grow for arg-heavy calls.
    let max_call_args = f
        .instructions
        .iter()
        .filter(|i| i.op == "call")
        .map(|i| i.srcs.len().saturating_sub(1))
        .max()
        .unwrap_or(0);
    let _ = writeln!(il, "    .maxstack {}", 8usize.max(max_call_args + 2));

    if !regs.local_tys.is_empty() {
        let locals: Vec<String> = regs
            .local_tys
            .iter()
            .enumerate()
            .map(|(i, ty)| format!("{ty} V_{i}"))
            .collect();
        let _ = writeln!(il, "    .locals init ({})", locals.join(", "));
    }

    for instr in &f.instructions {
        match instr.op.as_str() {
            // const <dest> = Int(n)  →  ldc.i4 n; st<dest>
            //
            // A `const` whose *result type* is a reference (`ref<…>`) is the
            // McCarthy **nil** — an empty list is a null `object[]`. The structural
            // `COND` lowering emits `const <r> = 0 : ref<LispyPair>` for the
            // fall-through when no clause matched. Storing an `int32` into an
            // object-typed local is ill-typed CIL, so emit a genuine `ldnull` (the
            // canonical nil), never `ldc.i4 0`. (Mirrors the binary emitter.)
            "const" => {
                let dest = instr.dest.as_deref().ok_or_else(|| IIRClrError::InvalidOperand {
                    function: f.name.clone(),
                    detail: "const must have a dest".to_string(),
                })?;
                if instr.type_hint.starts_with("ref<") {
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
                    store_var(il, &regs, dest)?;
                } else if let Some(Operand::Float(v)) = instr.srcs.first() {
                    // ALGOL `real` literal (LANG-FULL E3): push an IEEE-754 float.
                    // We emit the *raw little-endian bytes* (`ldc.r8 (b0 b1 … b7)`)
                    // rather than a decimal so the constant round-trips bit-exactly
                    // — a decimal like `0.1` would be re-parsed by ilasm and could
                    // differ from the IIR value. `f32` uses the 4-byte `ldc.r4`.
                    if instr.type_hint == "f32" {
                        let bytes = (*v as f32).to_le_bytes();
                        let hex: Vec<String> = bytes.iter().map(|b| format!("{b:02X}")).collect();
                        let _ = writeln!(il, "    ldc.r4 ({})", hex.join(" "));
                    } else {
                        let bytes = v.to_le_bytes();
                        let hex: Vec<String> = bytes.iter().map(|b| format!("{b:02X}")).collect();
                        let _ = writeln!(il, "    ldc.r8 ({})", hex.join(" "));
                    }
                    store_var(il, &regs, dest)?;
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
                    let n32 = i32::try_from(n).map_err(|_| IIRClrError::InvalidOperand {
                        function: f.name.clone(),
                        detail: format!("integer literal {n} out of int32 range"),
                    })?;
                    let _ = writeln!(il, "    ldc.i4 {n32}");
                    store_var(il, &regs, dest)?;
                }
            }
            // mov <dest>, <src>  →  ld<src>; st<dest>
            "mov" => {
                let dest = instr.dest.as_deref().ok_or_else(|| IIRClrError::InvalidOperand {
                    function: f.name.clone(),
                    detail: "mov must have a dest".to_string(),
                })?;
                let src = var_src(f, instr, 0, "mov")?;
                load_var(il, &regs, src)?;
                store_var(il, &regs, dest)?;
            }
            // ret <src>  →  ld<src>; ret
            "ret" => {
                let src = var_src(f, instr, 0, "ret")?;
                load_var(il, &regs, src)?;
                let _ = writeln!(il, "    ret");
            }
            // alloc <dest> : ref<LispyPair>  →  a fresh 2-element System.Object[]
            //   ldc.i4.2; newarr [System.Runtime]System.Object; st<dest>
            "alloc" => {
                let dest = instr.dest.as_deref().ok_or_else(|| IIRClrError::InvalidOperand {
                    function: f.name.clone(),
                    detail: "alloc must have a dest".to_string(),
                })?;
                let _ = writeln!(il, "    ldc.i4.2");
                let _ = writeln!(il, "    newarr [System.Runtime]System.Object");
                store_var(il, &regs, dest)?;
            }
            // ── alloc_bytes <dest> <- <size>  (LM-C Brainfuck) ───────────────
            //
            // The Brainfuck tape: an `unsigned int8[]` of `size` bytes, zero-filled
            // by `newarr`. `dest` (the tape base) is an array-typed local (see
            // `FnRegs::build`); `load_byte`/`store_byte` index it directly.
            //   ld<size>; newarr [System.Runtime]System.Byte; st<dest>
            "alloc_bytes" => {
                let dest = instr.dest.as_deref().ok_or_else(|| IIRClrError::InvalidOperand {
                    function: f.name.clone(),
                    detail: "alloc_bytes must have a dest".to_string(),
                })?;
                let size = var_src(f, instr, 0, "alloc_bytes")?;
                load_var(il, &regs, size)?;
                let _ = writeln!(il, "    newarr [System.Runtime]System.Byte");
                store_var(il, &regs, dest)?;
            }
            // ── load_byte <dest> <- <base>, <idx>  (LM-C Brainfuck) ──────────
            //
            // Read one tape cell, zero-extended to `int32`. `ldelem.u1` loads an
            // unsigned byte (so a cell value 200 reads as 200, not -56). The
            // (concretised) `int32` index addresses the array directly.
            //   ld<base>; ld<idx>; ldelem.u1; st<dest>
            "load_byte" => {
                let dest = instr.dest.as_deref().ok_or_else(|| IIRClrError::InvalidOperand {
                    function: f.name.clone(),
                    detail: "load_byte must have a dest".to_string(),
                })?;
                let base = var_src(f, instr, 0, "load_byte")?;
                let idx = var_src(f, instr, 1, "load_byte")?;
                load_var(il, &regs, base)?;
                load_var(il, &regs, idx)?;
                let _ = writeln!(il, "    ldelem.u1");
                store_var(il, &regs, dest)?;
            }
            // ── store_byte <base>, <idx>, <val>  (no dest; LM-C Brainfuck) ───
            //
            // Write the low byte of `val` into `tape[idx]`. `stelem.i1` truncates
            // the `int32` value to a byte, which is exactly Brainfuck's 8-bit cell
            // wrap-around (`255 + 1 == 0`).
            //   ld<base>; ld<idx>; ld<val>; stelem.i1
            "store_byte" => {
                if instr.dest.is_some() {
                    return Err(IIRClrError::InvalidOperand {
                        function: f.name.clone(),
                        detail: "store_byte must not have a dest".to_string(),
                    });
                }
                let base = var_src(f, instr, 0, "store_byte")?;
                let idx = var_src(f, instr, 1, "store_byte")?;
                let val = var_src(f, instr, 2, "store_byte")?;
                load_var(il, &regs, base)?;
                load_var(il, &regs, idx)?;
                load_var(il, &regs, val)?;
                let _ = writeln!(il, "    stelem.i1");
            }
            // box <dest> = <src>  →  ld<src>; box [System.Runtime]System.Int32; st<dest>
            "box" => {
                let dest = instr.dest.as_deref().ok_or_else(|| IIRClrError::InvalidOperand {
                    function: f.name.clone(),
                    detail: "box must have a dest".to_string(),
                })?;
                let src = var_src(f, instr, 0, "box")?;
                load_var(il, &regs, src)?;
                let _ = writeln!(il, "    box [System.Runtime]System.Int32");
                store_var(il, &regs, dest)?;
            }
            // unbox <dest> = <src>  →  ld<src>; unbox.any [System.Runtime]System.Int32; st<dest>
            "unbox" => {
                let dest = instr.dest.as_deref().ok_or_else(|| IIRClrError::InvalidOperand {
                    function: f.name.clone(),
                    detail: "unbox must have a dest".to_string(),
                })?;
                let src = var_src(f, instr, 0, "unbox")?;
                load_var(il, &regs, src)?;
                let _ = writeln!(il, "    unbox.any [System.Runtime]System.Int32");
                store_var(il, &regs, dest)?;
            }
            // field_store <arr>[<idx>] = <val>  (srcs = arr, Int(idx), val)
            //   ld<arr>; [castclass object[]]; ldc.i4 <idx>; ld<val>; stelem.ref
            //
            // If `arr` is statically typed `object` (a lambda parameter, not a
            // freshly-`alloc`-ed `object[]`), insert a `castclass object[]` so real
            // CoreCLR's `stelem.ref` sees an array on the stack.
            "field_store" => {
                let arr = var_src(f, instr, 0, "field_store")?;
                let idx = int_src(f, instr, 1, "field_store")?;
                let val = var_src(f, instr, 2, "field_store")?;
                let needs_cast = regs.home(arr)?.ty != "object[]";
                load_var(il, &regs, arr)?;
                if needs_cast {
                    let _ = writeln!(il, "    castclass object[]");
                }
                let _ = writeln!(il, "    ldc.i4 {idx}");
                load_var(il, &regs, val)?;
                let _ = writeln!(il, "    stelem.ref");
            }
            // field_load <dest> = <arr>[<idx>]  (srcs = arr, Int(idx))
            //   ld<arr>; [castclass object[]]; ldc.i4 <idx>; ldelem.ref; st<dest>
            "field_load" => {
                let dest = instr.dest.as_deref().ok_or_else(|| IIRClrError::InvalidOperand {
                    function: f.name.clone(),
                    detail: "field_load must have a dest".to_string(),
                })?;
                let arr = var_src(f, instr, 0, "field_load")?;
                let idx = int_src(f, instr, 1, "field_load")?;
                let needs_cast = regs.home(arr)?.ty != "object[]";
                load_var(il, &regs, arr)?;
                if needs_cast {
                    let _ = writeln!(il, "    castclass object[]");
                }
                let _ = writeln!(il, "    ldc.i4 {idx}");
                let _ = writeln!(il, "    ldelem.ref");
                store_var(il, &regs, dest)?;
            }
            // is_null <dest> = <src>  →  ld<src>; ldnull; ceq; st<dest>
            "is_null" => {
                let dest = instr.dest.as_deref().ok_or_else(|| IIRClrError::InvalidOperand {
                    function: f.name.clone(),
                    detail: "is_null must have a dest".to_string(),
                })?;
                let src = var_src(f, instr, 0, "is_null")?;
                load_var(il, &regs, src)?;
                let _ = writeln!(il, "    ldnull");
                let _ = writeln!(il, "    ceq");
                store_var(il, &regs, dest)?;
            }
            // ── Control flow (COND lowers to label / jmp / jmp_if_*) ──────────
            //
            // CIL labels are not opcodes — they are named positions in the byte
            // stream — so a `label` emits a `<name>:` anchor and the branches
            // reference it by name. `ilasm` resolves every name to the right offset.
            "label" => {
                let name = checked_label(f, var_src(f, instr, 0, "label")?)?;
                let _ = writeln!(il, "  {name}:");
            }
            // jmp <label>  →  br <label>
            "jmp" => {
                let label = checked_label(f, var_src(f, instr, 0, "jmp")?)?;
                let _ = writeln!(il, "    br {label}");
            }
            // jmp_if_false <cond>, <label>  →  ld<cond>; brfalse <label>
            "jmp_if_false" => {
                let cond = var_src(f, instr, 0, "jmp_if_false")?;
                let label = checked_label(f, var_src(f, instr, 1, "jmp_if_false")?)?;
                load_var(il, &regs, cond)?;
                let _ = writeln!(il, "    brfalse {label}");
            }
            // jmp_if_true <cond>, <label>  →  ld<cond>; brtrue <label>
            "jmp_if_true" => {
                let cond = var_src(f, instr, 0, "jmp_if_true")?;
                let label = checked_label(f, var_src(f, instr, 1, "jmp_if_true")?)?;
                load_var(il, &regs, cond)?;
                let _ = writeln!(il, "    brtrue {label}");
            }
            // ── call <dest> = <fn>(<args…>)  (srcs = fn_name, arg0, arg1, …) ──
            //
            // A same-module call. The callee is found by name; its CIL signature is
            // derived from its IIR params/return type. We emit a by-name
            // `call <ret> <Class>::<m>(<argtys>)` and let `ilasm` resolve the token —
            // self-recursive `LABEL` is just a method calling itself.
            "call" => {
                let dest = instr.dest.as_deref().ok_or_else(|| IIRClrError::InvalidOperand {
                    function: f.name.clone(),
                    detail: "call must have a dest".to_string(),
                })?;
                let callee = var_src(f, instr, 0, "call")?;
                let callee_fn = module
                    .functions
                    .iter()
                    .find(|g| g.name == callee)
                    .ok_or_else(|| IIRClrError::UndefinedLabel {
                        function: f.name.clone(),
                        label: callee.to_string(),
                    })?;
                // Push arguments in order.
                for (k, src) in instr.srcs.iter().enumerate().skip(1) {
                    match src {
                        Operand::Var(n) => load_var(il, &regs, n)?,
                        Operand::Int(n) => {
                            let n32 = i32::try_from(*n).map_err(|_| IIRClrError::InvalidOperand {
                                function: f.name.clone(),
                                detail: format!("call arg {n} out of int32 range"),
                            })?;
                            let _ = writeln!(il, "    ldc.i4 {n32}");
                        }
                        Operand::Bool(b) => {
                            let _ = writeln!(il, "    ldc.i4.{}", if *b { 1 } else { 0 });
                        }
                        other => {
                            return Err(IIRClrError::InvalidOperand {
                                function: f.name.clone(),
                                detail: format!("call arg[{k}] unsupported operand {other:?}"),
                            })
                        }
                    }
                }
                let callee_ret = cil_ret_type(module, callee_fn);
                let arg_tys: Vec<&'static str> = callee_fn
                    .params
                    .iter()
                    .map(|(_, t)| cil_local_type(t))
                    .collect();
                // Validate the callee's emitted name (the entry's fixed name is safe).
                let callee_method = if callee == entry_name(module) {
                    "MccarthyEntry".to_string()
                } else {
                    checked_cil_ident(&f.name, callee)?.to_string()
                };
                let _ = writeln!(
                    il,
                    "    call {callee_ret} {asm}Program::{callee_method}({})",
                    arg_tys.join(", ")
                );
                store_var(il, &regs, dest)?;
            }
            // ── McCarthy predicate primitives (call_builtin) ──────────────────
            //
            // | builtin     | CIL |
            // |-------------|-----|
            // | `pair?`     | `ld x; isinst object[]; ldnull; ceq; ldc.i4.0; ceq` |
            // | `not`       | `ld x; ldc.i4.1; xor` |
            // | `equal?`    | `ld a; unbox.any int32; ld b; unbox.any int32; ceq` |
            // | `print_i64` | `ld val; call void Console::WriteLine(int32)`  (no dest) |
            "call_builtin" => {
                let builtin = var_src(f, instr, 0, "call_builtin")?;
                // `print_i64` is the I/O primitive Dartmouth BASIC's `PRINT` lowers to.
                // Unlike the predicate builtins it has **no dest** (it's a side effect),
                // so handle it before the dest lookup. The value is loaded and handed to
                // `System.Console.WriteLine(int32)` — the CLR analogue of the wasm
                // `env.__print_i64` host import / JVM `env.BasicRuntime.println(J)V`.
                // (Scalar concretization has lowered the value to `int32`, and
                // `Console.WriteLine` has an `int32` overload, so no `conv.i8` is needed.)
                if builtin == "print_i64" {
                    let val = var_src(f, instr, 1, "print_i64")?;
                    load_var(il, &regs, val)?;
                    let _ = writeln!(
                        il,
                        "    call void [System.Console]System.Console::WriteLine(int32)"
                    );
                    continue;
                }
                // `putchar` — Brainfuck's `.`. Also a dest-less side effect, so handle
                // it before the dest lookup. Write the cell's low byte as a *character*
                // (not its decimal value): mask to 8 bits, widen to the 16-bit `char`,
                // and call `Console.Write(char)` — so `.` of 65 emits `A`, not `65`.
                // The CLR analogue of LLVM's libc `putchar` / wasm's `env.putchar`.
                if builtin == "putchar" {
                    let val = var_src(f, instr, 1, "putchar")?;
                    load_var(il, &regs, val)?;
                    let _ = writeln!(il, "    ldc.i4 0xFF");
                    let _ = writeln!(il, "    and");
                    let _ = writeln!(il, "    conv.u2");
                    let _ = writeln!(
                        il,
                        "    call void [System.Console]System.Console::Write(char)"
                    );
                    continue;
                }
                let dest = instr.dest.as_deref().ok_or_else(|| IIRClrError::InvalidOperand {
                    function: f.name.clone(),
                    detail: "call_builtin must have a dest".to_string(),
                })?;
                match builtin {
                    "pair?" => {
                        let arg = var_src(f, instr, 1, "pair?")?;
                        load_var(il, &regs, arg)?;
                        let _ = writeln!(il, "    isinst object[]");
                        let _ = writeln!(il, "    ldnull");
                        let _ = writeln!(il, "    ceq");
                        let _ = writeln!(il, "    ldc.i4.0");
                        let _ = writeln!(il, "    ceq");
                        store_var(il, &regs, dest)?;
                    }
                    "not" => {
                        let arg = var_src(f, instr, 1, "not")?;
                        load_var(il, &regs, arg)?;
                        let _ = writeln!(il, "    ldc.i4.1");
                        let _ = writeln!(il, "    xor");
                        store_var(il, &regs, dest)?;
                    }
                    "equal?" => {
                        let a = var_src(f, instr, 1, "equal?")?;
                        let b = var_src(f, instr, 2, "equal?")?;
                        load_var(il, &regs, a)?;
                        let _ = writeln!(il, "    unbox.any [System.Runtime]System.Int32");
                        load_var(il, &regs, b)?;
                        let _ = writeln!(il, "    unbox.any [System.Runtime]System.Int32");
                        let _ = writeln!(il, "    ceq");
                        store_var(il, &regs, dest)?;
                    }
                    // `getchar` — Brainfuck's `,`. Read one character from stdin.
                    // `Console.Read()` returns the next char as an `int32`, or `-1` at
                    // EOF; the result is stored raw (a later `store_byte` truncates it
                    // to the 8-bit cell — EOF lands as `0xFF`, the conventional BF
                    // behaviour). The CLR analogue of libc `getchar` / wasm `env.getchar`.
                    "getchar" => {
                        let _ = writeln!(
                            il,
                            "    call int32 [System.Console]System.Console::Read()"
                        );
                        store_var(il, &regs, dest)?;
                    }
                    other => {
                        return Err(IIRClrError::UnsupportedOp {
                            function: f.name.clone(),
                            op: format!("call_builtin {other:?}"),
                        })
                    }
                }
            }
            // ── Binary integer arithmetic ─────────────────────────────────────
            //
            // `dest = <op>(a, b)` with both operands scalar `int32` — the scalar
            // concretization pass has already lowered every scalar value to i32, so
            // we load both and emit the single CIL arithmetic opcode.
            //
            // | IIR op | CIL  | note                          |
            // |--------|------|-------------------------------|
            // | add    | add  | integer addition              |
            // | sub    | sub  | integer subtraction           |
            // | mul    | mul  | integer multiplication        |
            // | div    | div  | signed integer division       |
            // | mod    | rem  | signed remainder (CIL `rem`)  |
            //
            // (CoreCLR's `div`/`rem` raise on divide-by-zero, matching the other
            // backends' trap-on-zero behaviour — no guard needed here.)
            // Unary bitwise NOT (`~`, LANG-FULL N3) → the CIL `not` opcode (one's
            // complement), then the E2 narrow mask so a `u4`/`u8`/`u16` result is the
            // width's complement, not the full register's: `~0u8 = 255` (`-1 & 0xFF`),
            // `~15u4 = 0`. This is the unary IIR `not` op (one source operand) — distinct
            // from the lispy `call_builtin "not"` (boolean negate) handled above.
            "not" => {
                let dest = instr.dest.as_deref().ok_or_else(|| IIRClrError::InvalidOperand {
                    function: f.name.clone(),
                    detail: "not must have a dest".into(),
                })?;
                let a = var_src(f, instr, 0, "not")?;
                load_var(il, &regs, a)?;
                let _ = writeln!(il, "    not");
                emit_narrow_width_mask(il, &instr.type_hint);
                store_var(il, &regs, dest)?;
            }
            // Bitwise `and`/`or`/`xor` map to the identically-named CIL opcodes
            // (LANG-FULL N3). `shl`/`shr` are the CIL shift ops; included here so
            // the textual path matches the bytecode path's binary-op coverage.
            "add" | "sub" | "mul" | "div" | "mod" | "and" | "or" | "xor" | "shl" | "shr" => {
                let dest = instr.dest.as_deref().ok_or_else(|| IIRClrError::InvalidOperand {
                    function: f.name.clone(),
                    detail: format!("{} must have a dest", instr.op),
                })?;
                let a = var_src(f, instr, 0, instr.op.as_str())?;
                let b = var_src(f, instr, 1, instr.op.as_str())?;
                load_var(il, &regs, a)?;
                load_var(il, &regs, b)?;
                let cil = match instr.op.as_str() {
                    "add" => "add",
                    "sub" => "sub",
                    "mul" => "mul",
                    "div" => "div",
                    "mod" => "rem",
                    "and" => "and",
                    "or" => "or",
                    "xor" => "xor",
                    "shl" => "shl",
                    "shr" => "shr",
                    _ => unreachable!(),
                };
                let _ = writeln!(il, "    {cil}");
                // E2: wrap a narrow `u4`/`u8`/`u16` result mod-2ⁿ
                // (`200u8+100u8=44`); a no-op for u32/i32/i64.
                emit_narrow_width_mask(il, &instr.type_hint);
                store_var(il, &regs, dest)?;
            }
            // ── Integer comparisons → a 0/1 `int32` result ────────────────────
            //
            // CIL only has `ceq` / `clt` / `cgt`; the other three relations are the
            // logical negation of one of those — negate a boolean by `ldc.i4.0; ceq`
            // (i.e. "== 0"). The 0/1 result feeds either a `st<dest>` or directly a
            // `brfalse`/`brtrue` (Oct's `if x == 1` does exactly this).
            //
            // | IIR op | CIL                | meaning              |
            // |--------|--------------------|----------------------|
            // | cmp_eq | ceq                | a == b               |
            // | cmp_ne | ceq; ldc.i4.0; ceq | !(a == b)            |
            // | cmp_lt | clt                | a <  b               |
            // | cmp_gt | cgt                | a >  b               |
            // | cmp_le | cgt; ldc.i4.0; ceq | !(a >  b)  ⇔  a ≤ b  |
            // | cmp_ge | clt; ldc.i4.0; ceq | !(a <  b)  ⇔  a ≥ b  |
            "cmp_eq" | "cmp_ne" | "cmp_lt" | "cmp_le" | "cmp_gt" | "cmp_ge" => {
                let dest = instr.dest.as_deref().ok_or_else(|| IIRClrError::InvalidOperand {
                    function: f.name.clone(),
                    detail: format!("{} must have a dest", instr.op),
                })?;
                let a = var_src(f, instr, 0, instr.op.as_str())?;
                let b = var_src(f, instr, 1, instr.op.as_str())?;
                load_var(il, &regs, a)?;
                load_var(il, &regs, b)?;
                match instr.op.as_str() {
                    "cmp_eq" => {
                        let _ = writeln!(il, "    ceq");
                    }
                    "cmp_lt" => {
                        let _ = writeln!(il, "    clt");
                    }
                    "cmp_gt" => {
                        let _ = writeln!(il, "    cgt");
                    }
                    "cmp_ne" => {
                        let _ = writeln!(il, "    ceq");
                        let _ = writeln!(il, "    ldc.i4.0");
                        let _ = writeln!(il, "    ceq");
                    }
                    "cmp_le" => {
                        let _ = writeln!(il, "    cgt");
                        let _ = writeln!(il, "    ldc.i4.0");
                        let _ = writeln!(il, "    ceq");
                    }
                    "cmp_ge" => {
                        let _ = writeln!(il, "    clt");
                        let _ = writeln!(il, "    ldc.i4.0");
                        let _ = writeln!(il, "    ceq");
                    }
                    _ => unreachable!(),
                }
                store_var(il, &regs, dest)?;
            }
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

    /// Build a one-function module `c = <op>(a, b); ret c` over two `i32` constants.
    fn binop_module(op: &str) -> IIRModule {
        let instrs = vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(17)], "i32"),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Int(5)], "i32"),
            IIRInstr::new(
                op,
                Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "i32"),
        ];
        let mut m = IIRModule::new("Main", "test");
        m.functions.push(IIRFunction::new("main", vec![], "i32", instrs));
        m.entry_point = Some("main".into());
        m
    }

    #[test]
    fn binary_arithmetic_emits_cil_opcodes() {
        // Each IIR arithmetic op lowers to a single CIL opcode (note `mod` → `rem`).
        for (op, cil) in
            [("add", "add"), ("sub", "sub"), ("mul", "mul"), ("div", "div"), ("mod", "rem")]
        {
            let il = emit_il(&binop_module(op), &IIRClrConfig::new("Main")).unwrap();
            assert!(
                il.lines().any(|l| l.trim() == cil),
                "op {op:?} must emit a bare `{cil}` instruction; got:\n{il}"
            );
        }
    }

    #[test]
    fn bitwise_ops_emit_cil_opcodes() {
        // LANG-FULL N3: the textual `.il` path must emit the bitwise CIL opcodes
        // (the bytecode path already did) so Nib `& | ^` runs on real CoreCLR.
        for (op, cil) in [("and", "and"), ("or", "or"), ("xor", "xor")] {
            let il = emit_il(&binop_module(op), &IIRClrConfig::new("Main")).unwrap();
            assert!(
                il.lines().any(|l| l.trim() == cil),
                "op {op:?} must emit a bare `{cil}` instruction; got:\n{il}"
            );
        }
    }

    #[test]
    fn unary_not_emits_cil_not_then_masks() {
        // LANG-FULL N3: the textual `.il` path must lower the unary `not` op (Nib `~`)
        // to the CIL `not` opcode (the bytecode path already did), followed by the E2
        // narrow mask for a `u8` width — so `~0u8 = 255` on real CoreCLR, not the
        // register's all-ones. (The lispy `call_builtin "not"` is a different path.)
        let instrs = vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(0)], "i32"),
            IIRInstr::new("not", Some("c".into()), vec![Operand::Var("a".into())], "u8"),
            IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "u8"),
        ];
        let mut m = IIRModule::new("Main", "test");
        m.functions.push(IIRFunction::new("main", vec![], "u8", instrs));
        m.entry_point = Some("main".into());
        let il = emit_il(&m, &IIRClrConfig::new("Main")).unwrap();
        let lines: Vec<&str> = il.lines().map(|l| l.trim()).collect();
        let not_at = lines.iter().position(|l| *l == "not").expect("emits a bare `not`");
        // The mask is `ldc.i4 0xFF` (or `ldc.i4 255`) then `and` immediately after.
        assert!(
            lines[not_at + 1..].iter().take(2).any(|l| *l == "and"),
            "u8 not must be followed by the `and` mask; got:\n{il}"
        );
    }

    /// Build `c = <op>(a, b); ret c` with a chosen result-`type_hint` width,
    /// so the E2 narrow-width mask fires on the binary op (the operand `const`s
    /// stay `i32`; only the op's `type_hint` selects the width).
    fn binop_module_typed(op: &str, hint: &str) -> IIRModule {
        let instrs = vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(200)], "i32"),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Int(100)], "i32"),
            IIRInstr::new(
                op,
                Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                hint,
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("c".into())], hint),
        ];
        let mut m = IIRModule::new("Main", "test");
        m.functions.push(IIRFunction::new("main", vec![], hint, instrs));
        m.entry_point = Some("main".into());
        m
    }

    /// LANG-FULL E2 integration regression: a narrow `u8` op whose **operands
    /// are `i64`** — the shape a real frontend emits (Nib materialises every
    /// const/let as i64 and carries the narrow width only on the op). Unlike the
    /// wasm/jvm backends (which had to grow an i64 register model so a narrow op
    /// wouldn't trap over i64 operands), the CIL backend is **uniformly int32**
    /// (`cil_local_type` maps every scalar — incl. `i64` — to `int32`, and
    /// `const` emits `ldc.i4`). So the i64 consts collapse to int32, the add is
    /// int32, and the `ldc.i4 0xFF; and` mask is int32-consistent — no rework
    /// needed. This test locks that in: the IL has NO `int64`/`ldc.i8`, and the
    /// u8 add still wraps via the mask. (`200u8 + 100u8` → `44` on real dotnet.)
    #[test]
    fn e2_u8_op_over_i64_operands_stays_int32() {
        let f = IIRFunction::new("main", vec![], "i64", vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(200)], "i64"),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Int(100)], "i64"),
            IIRInstr::new("add", Some("x".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "u8"),
            IIRInstr::new("ret", None, vec![Operand::Var("x".into())], "i64"),
        ]);
        let mut m = IIRModule::new("Main", "nib");
        m.functions.push(f);
        m.entry_point = Some("main".into());
        let il = emit_il(&m, &IIRClrConfig::new("Main")).unwrap();
        assert!(!il.contains("int64") && !il.contains("ldc.i8"),
            "CIL is uniformly int32 — no int64 from an i64-hinted operand; got:\n{il}");
        let lines: Vec<&str> = il.lines().map(|l| l.trim()).collect();
        let add_at = lines.iter().position(|l| *l == "add").expect("emits add");
        assert_eq!(lines[add_at + 1], "ldc.i4 0xFF", "u8 add still masks over i64-collapsed operands");
        assert_eq!(lines[add_at + 2], "and");
    }

    #[test]
    fn e2_narrow_width_add_masks_result() {
        // LANG-FULL E2: a `u8` add wraps mod-256 — the textual `.il` must emit
        // `add` immediately followed by `ldc.i4 0xFF; and` so `200u8+100u8=44`.
        let il = emit_il(&binop_module_typed("add", "u8"), &IIRClrConfig::new("Main")).unwrap();
        let lines: Vec<&str> = il.lines().map(|l| l.trim()).collect();
        let add_at = lines.iter().position(|l| *l == "add").expect("emits add");
        assert_eq!(lines[add_at + 1], "ldc.i4 0xFF", "u8 add → push 0xFF mask; got:\n{il}");
        assert_eq!(lines[add_at + 2], "and", "u8 add → `and` the mask; got:\n{il}");
    }

    #[test]
    fn e2_narrow_width_masks_match_hint() {
        // u4 → 0xF, u8 → 0xFF, u16 → 0xFFFF.
        for (hint, mask) in [("u4", "0xF"), ("u8", "0xFF"), ("u16", "0xFFFF")] {
            let il = emit_il(&binop_module_typed("mul", hint), &IIRClrConfig::new("Main")).unwrap();
            assert!(
                il.lines().any(|l| l.trim() == format!("ldc.i4 {mask}")),
                "{hint} mul must mask with `ldc.i4 {mask}`; got:\n{il}"
            );
            assert!(il.lines().any(|l| l.trim() == "and"), "{hint} mul must `and`; got:\n{il}");
        }
    }

    #[test]
    fn e2_wide_widths_are_not_masked() {
        // u32/i32 already wrap mod-2³² via the 32-bit op; i64 is wider — none of
        // them gets a mask, so the IL is byte-for-byte the legacy output.
        for hint in ["i32", "u32", "i64"] {
            let il = emit_il(&binop_module_typed("add", hint), &IIRClrConfig::new("Main")).unwrap();
            let masked = il.lines().any(|l| {
                let t = l.trim();
                t == "ldc.i4 0xFF" || t == "ldc.i4 0xFFFF" || t == "ldc.i4 0xF"
            });
            assert!(!masked, "{hint} add must NOT emit a width mask; got:\n{il}");
        }
    }

    #[test]
    fn print_i64_emits_console_writeline_and_discards_result() {
        // A printing program: `print_i64(7); ret 0` — BASIC's `PRINT 7` shape.
        let instrs = vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(7)], "i32"),
            IIRInstr::new(
                "call_builtin",
                None,
                vec![Operand::Var("print_i64".into()), Operand::Var("v".into())],
                "void",
            ),
            IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("z".into())], "i32"),
        ];
        let mut m = IIRModule::new("Main", "test");
        m.functions.push(IIRFunction::new("main", vec![], "i32", instrs));
        m.entry_point = Some("main".into());
        let il = emit_il(&m, &IIRClrConfig::new("Main")).unwrap();
        // The value is written via Console.WriteLine(int32) inside the entry method…
        assert!(
            il.contains("call void [System.Console]System.Console::WriteLine(int32)"),
            "print_i64 must call Console.WriteLine(int32); got:\n{il}"
        );
        // …and the launcher must DISCARD (pop) the entry's result, not re-print it:
        // for a printing program there is exactly one WriteLine and a `pop`.
        assert_eq!(
            il.matches("System.Console::WriteLine(int32)").count(),
            1,
            "a printing program prints exactly once (no double-print); got:\n{il}"
        );
        let launcher = il.split(".entrypoint").nth(1).expect("a launcher");
        assert!(launcher.contains("pop"), "launcher discards the entry result; got:\n{il}");
    }

    #[test]
    fn comparisons_emit_cil_opcodes() {
        // The three primitive relations are single opcodes.
        for (op, cil) in [("cmp_eq", "ceq"), ("cmp_lt", "clt"), ("cmp_gt", "cgt")] {
            let il = emit_il(&binop_module(op), &IIRClrConfig::new("Main")).unwrap();
            assert!(
                il.lines().any(|l| l.trim() == cil),
                "op {op:?} must emit a bare `{cil}` instruction; got:\n{il}"
            );
        }
        // The negated relations build on a primitive then `ldc.i4.0; ceq` (i.e. "== 0").
        for (op, base) in [("cmp_ne", "ceq"), ("cmp_le", "cgt"), ("cmp_ge", "clt")] {
            let il = emit_il(&binop_module(op), &IIRClrConfig::new("Main")).unwrap();
            assert!(il.lines().any(|l| l.trim() == base), "op {op:?} builds on `{base}`; got:\n{il}");
            assert!(
                il.contains("ldc.i4.0"),
                "negated relation {op:?} pushes 0 to invert; got:\n{il}"
            );
        }
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
        // The cons array is statically object[], so NO castclass is needed.
        assert!(!il.contains("castclass"), "alloc-ed cons needs no castclass; got:\n{il}");
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
        assert!(il.contains("isinst object[]"), "pair? → isinst object[]; got:\n{il}");
        assert!(il.contains("ceq"), "pair? collapses ref/null to a bool with ceq");
        assert!(il.contains("ldc.i4.1\n    xor"), "not → xor 1; got:\n{il}");
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
        assert!(il.contains("ldnull"), "const-of-ref-type nil → ldnull; got:\n{il}");
        assert!(il.contains("object[] V_"), "nil local is object[]");
        assert!(!il.contains("ldc.i4 0\n    stloc"), "nil must not be ldc.i4 0");
    }

    #[test]
    fn lambda_emits_second_method_param_ldarg_and_call() {
        // `((LAMBDA (X) (CAR X)) (CONS 7 9))`: a second method `lambda_0(object)`
        // that reads param 0 (ldarg), casts it to object[] (param is statically
        // object), CARs it; `main` builds the cons and `call`s lambda_0 by name.
        let lambda = IIRFunction::new(
            "lambda_0",
            vec![("X".into(), "ref<any>".into())],
            "ref<any>",
            vec![
                IIRInstr::new(
                    "field_load",
                    Some("v0".into()),
                    vec![Operand::Var("X".into()), Operand::Int(0)],
                    "ref<any>",
                ),
                IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "ref<any>"),
            ],
        );
        let main = IIRFunction::new(
            "main",
            vec![],
            "i32",
            vec![
                IIRInstr::new("const", Some("a".into()), vec![Operand::Int(7)], "i32"),
                IIRInstr::new("alloc", Some("c".into()), vec![], "ref<LispyPair>"),
                IIRInstr::new("box", Some("ab".into()), vec![Operand::Var("a".into())], "ref<any>"),
                IIRInstr::new(
                    "field_store",
                    None,
                    vec![Operand::Var("c".into()), Operand::Int(0), Operand::Var("ab".into())],
                    "void",
                ),
                IIRInstr::new(
                    "call",
                    Some("r".into()),
                    vec![Operand::Var("lambda_0".into()), Operand::Var("c".into())],
                    "ref<any>",
                ),
                IIRInstr::new("unbox", Some("ru".into()), vec![Operand::Var("r".into())], "i32"),
                IIRInstr::new("ret", None, vec![Operand::Var("ru".into())], "i32"),
            ],
        );
        let mut m = IIRModule::new("Main", "mccarthy-lisp");
        m.functions.push(lambda);
        m.functions.push(main);
        m.entry_point = Some("main".into());

        let il = emit_il(&m, &IIRClrConfig::new("Main")).unwrap();
        // A distinct lambda method with one object parameter.
        assert!(
            il.contains("object lambda_0(object A_0)"),
            "lambda emitted as its own method; got:\n{il}"
        );
        // The parameter is read with ldarg, not ldloc.
        assert!(il.contains("ldarg 0"), "param read via ldarg; got:\n{il}");
        // The object-typed param must be cast to object[] before ldelem.ref.
        assert!(il.contains("castclass object[]"), "object param cast before CAR; got:\n{il}");
        // main calls lambda_0 by name with the right signature.
        assert!(
            il.contains("call object MainProgram::lambda_0(object)"),
            "by-name call; got:\n{il}"
        );
    }

    #[test]
    fn recursive_label_calls_itself_by_name() {
        // A self-recursive LABEL function: `label_0` calls `label_0`. The call is
        // resolved by name, so recursion needs no special handling.
        let label = IIRFunction::new(
            "label_0",
            vec![("X".into(), "ref<any>".into())],
            "ref<any>",
            vec![
                IIRInstr::new(
                    "field_load",
                    Some("h".into()),
                    vec![Operand::Var("X".into()), Operand::Int(0)],
                    "ref<any>",
                ),
                IIRInstr::new(
                    "call",
                    Some("r".into()),
                    vec![Operand::Var("label_0".into()), Operand::Var("h".into())],
                    "ref<any>",
                ),
                IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "ref<any>"),
            ],
        );
        let main = IIRFunction::new(
            "main",
            vec![],
            "i32",
            vec![
                IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "i32"),
                IIRInstr::new("ret", None, vec![Operand::Var("z".into())], "i32"),
            ],
        );
        let mut m = IIRModule::new("Main", "mccarthy-lisp");
        m.functions.push(label);
        m.functions.push(main);
        m.entry_point = Some("main".into());

        let il = emit_il(&m, &IIRClrConfig::new("Main")).unwrap();
        assert!(
            il.contains("call object MainProgram::label_0(object)"),
            "self-recursive call by name; got:\n{il}"
        );
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
    fn malicious_function_name_is_rejected_not_injected() {
        // A hostile *function* name flows into `.method`/`call` text — it must be
        // rejected by the same identifier whitelist, never emitted verbatim.
        let evil = IIRFunction::new(
            "f\n  .method public static void Hacked() cil managed { ret }\n  .method x",
            vec![],
            "ref<any>",
            vec![IIRInstr::new("ret", None, vec![Operand::Var("X".into())], "ref<any>")],
        );
        let main = IIRFunction::new(
            "main",
            vec![],
            "i32",
            vec![
                IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "i32"),
                IIRInstr::new("ret", None, vec![Operand::Var("z".into())], "i32"),
            ],
        );
        let mut m = IIRModule::new("Main", "mccarthy-lisp");
        m.functions.push(evil);
        m.functions.push(main);
        m.entry_point = Some("main".into());
        let err = emit_il(&m, &IIRClrConfig::new("Main")).unwrap_err();
        assert!(matches!(err, IIRClrError::InvalidOperand { .. }), "got {err:?}");
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
    fn none_entry_point_falls_back_to_main_and_names_mccarthy_entry() {
        // With `entry_point = None`, the emitter resolves the entry to `"main"`
        // *consistently* — `main` is renamed to `MccarthyEntry` so the launcher's
        // hardcoded `call …::MccarthyEntry()` resolves (never dangles).
        let instrs = vec![
            IIRInstr::new("const", Some("v0".into()), vec![Operand::Int(42)], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
        ];
        let mut m = IIRModule::new("Main", "mccarthy-lisp");
        m.functions.push(IIRFunction::new("main", vec![], "i32", instrs));
        m.entry_point = None; // <- the edge case

        let il = emit_il(&m, &IIRClrConfig::new("Main")).unwrap();
        assert!(il.contains("int32 MccarthyEntry()"), "main renamed to MccarthyEntry; got:\n{il}");
        assert!(il.contains("call int32 MainProgram::MccarthyEntry()"), "launcher resolves");
    }

    #[test]
    fn call_to_unknown_function_is_rejected() {
        // A `call` to a function not in the module must be rejected, not emit a
        // dangling by-name call that `ilasm` would fail on later.
        let instrs = vec![
            IIRInstr::new(
                "call",
                Some("r".into()),
                vec![Operand::Var("nonexistent".into())],
                "ref<any>",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ];
        let mut m = IIRModule::new("Main", "mccarthy-lisp");
        m.functions.push(IIRFunction::new("main", vec![], "i32", instrs));
        m.entry_point = Some("main".into());
        let err = emit_il(&m, &IIRClrConfig::new("Main")).unwrap_err();
        assert!(matches!(err, IIRClrError::UndefinedLabel { .. }), "got {err:?}");
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

    // ── Byte-tape ops + putchar (LANG-MATRIX LM-C Brainfuck) ─────────────────

    /// A Brainfuck-shaped program: allocate a tape, store a byte, load it, print it
    /// with `putchar`. The `.il` declares an `unsigned int8[]` tape local and uses
    /// `newarr Byte` / `ldelem.u1` / `stelem.i1` to access it; `.` becomes
    /// `Console::Write(char)`; the launcher discards the entry result (no double-print).
    #[test]
    fn brainfuck_byte_tape_and_putchar_lower_to_cil() {
        let instrs = vec![
            IIRInstr::new("const", Some("size".into()), vec![Operand::Int(30_000)], "i32"),
            IIRInstr::new("alloc_bytes", Some("tape".into()), vec![Operand::Var("size".into())], "i32"),
            IIRInstr::new("const", Some("idx".into()), vec![Operand::Int(0)], "i32"),
            IIRInstr::new("const", Some("val".into()), vec![Operand::Int(65)], "i32"),
            IIRInstr::new("store_byte", None, vec![
                Operand::Var("tape".into()), Operand::Var("idx".into()), Operand::Var("val".into()),
            ], "i32"),
            IIRInstr::new("load_byte", Some("got".into()), vec![
                Operand::Var("tape".into()), Operand::Var("idx".into()),
            ], "i32"),
            IIRInstr::new("call_builtin", None, vec![
                Operand::Var("putchar".into()), Operand::Var("got".into()),
            ], "void"),
            IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("z".into())], "i32"),
        ];
        let mut m = IIRModule::new("Main", "test");
        m.functions.push(IIRFunction::new("main", vec![], "i32", instrs));
        m.entry_point = Some("main".into());
        let il = emit_il(&m, &IIRClrConfig::new("Main")).unwrap();

        assert!(il.contains("unsigned int8[] V_"), "the tape is an unsigned int8[] local; got:\n{il}");
        assert!(il.contains("newarr [System.Runtime]System.Byte"), "alloc_bytes → newarr Byte; got:\n{il}");
        assert!(il.contains("ldelem.u1"), "load_byte → ldelem.u1 (unsigned); got:\n{il}");
        assert!(il.contains("stelem.i1"), "store_byte → stelem.i1; got:\n{il}");
        assert!(
            il.contains("call void [System.Console]System.Console::Write(char)"),
            "putchar writes a char (so `.` of 65 is `A`, not `65`); got:\n{il}"
        );
        // A putchar program "prints" → the launcher discards the entry result.
        let launcher = il.split(".entrypoint").nth(1).expect("a launcher");
        assert!(launcher.contains("pop"), "launcher discards the entry result; got:\n{il}");
        assert!(
            !launcher.contains("WriteLine"),
            "a printing (putchar) program must not re-print the exit value; got:\n{il}"
        );
    }

    /// `getchar` reads a character via `Console::Read()`.
    #[test]
    fn getchar_reads_via_console_read() {
        let instrs = vec![
            IIRInstr::new("call_builtin", Some("c".into()), vec![Operand::Var("getchar".into())], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "i32"),
        ];
        let mut m = IIRModule::new("Main", "test");
        m.functions.push(IIRFunction::new("main", vec![], "i32", instrs));
        m.entry_point = Some("main".into());
        let il = emit_il(&m, &IIRClrConfig::new("Main")).unwrap();
        assert!(
            il.contains("call int32 [System.Console]System.Console::Read()"),
            "getchar → Console.Read(); got:\n{il}"
        );
    }

    /// `store_byte` with a dest is rejected — it produces no value.
    #[test]
    fn store_byte_with_dest_is_rejected() {
        let instrs = vec![
            IIRInstr::new("const", Some("size".into()), vec![Operand::Int(8)], "i32"),
            IIRInstr::new("alloc_bytes", Some("tape".into()), vec![Operand::Var("size".into())], "i32"),
            IIRInstr::new("const", Some("idx".into()), vec![Operand::Int(0)], "i32"),
            IIRInstr::new("const", Some("val".into()), vec![Operand::Int(1)], "i32"),
            IIRInstr::new("store_byte", Some("oops".into()), vec![
                Operand::Var("tape".into()), Operand::Var("idx".into()), Operand::Var("val".into()),
            ], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("idx".into())], "i32"),
        ];
        let mut m = IIRModule::new("Main", "test");
        m.functions.push(IIRFunction::new("main", vec![], "i32", instrs));
        m.entry_point = Some("main".into());
        let err = emit_il(&m, &IIRClrConfig::new("Main")).unwrap_err();
        assert!(matches!(err, IIRClrError::InvalidOperand { .. }), "store_byte must not have a dest");
    }

    // ── f64 (double) support — LANG-FULL E3 (ALGOL `real`) ───────────

    /// An `f64` program: a `real` local seeded with `2.5`, multiplied by `2.0`,
    /// compared `== 5.0`, folding to an integer exit code. The `.il` must declare
    /// the real registers as `float64`, push their literals with `ldc.r8`, and
    /// use the stack-type-overloaded `mul`/`ceq` (no opcode change for doubles).
    #[test]
    fn f64_program_emits_float64_locals_ldc_r8_and_mul() {
        let instrs = vec![
            IIRInstr::new("const", Some("r".into()), vec![Operand::Float(2.5)], "f64"),
            IIRInstr::new("const", Some("two".into()), vec![Operand::Float(2.0)], "f64"),
            IIRInstr::new("mul", Some("p".into()),
                vec![Operand::Var("r".into()), Operand::Var("two".into())], "f64"),
            IIRInstr::new("const", Some("five".into()), vec![Operand::Float(5.0)], "f64"),
            IIRInstr::new("cmp_eq", Some("eq".into()),
                vec![Operand::Var("p".into()), Operand::Var("five".into())], "f64"),
            IIRInstr::new("ret", None, vec![Operand::Var("eq".into())], "i32"),
        ];
        let mut m = IIRModule::new("Main", "test");
        m.functions.push(IIRFunction::new("main", vec![], "i32", instrs));
        m.entry_point = Some("main".into());
        let il = emit_il(&m, &IIRClrConfig::new("Main")).unwrap();
        assert!(il.contains("float64 V_"), "real registers must be float64 locals; got:\n{il}");
        assert!(il.contains("ldc.r8 ("), "f64 literals must use ldc.r8 (byte form); got:\n{il}");
        assert!(il.contains("    mul\n"), "f64 multiply uses the overloaded `mul`; got:\n{il}");
        assert!(il.contains("    ceq\n"), "f64 equality uses the overloaded `ceq`; got:\n{il}");
        // The comparison result is an int32 (the exit-code fold), not a float.
        assert!(il.contains("int32 V_"), "the cmp_eq result is an int32 local; got:\n{il}");
    }

    /// `ldc.r8` uses the exact little-endian IEEE-754 bytes so a `real` constant
    /// round-trips bit-for-bit (2.0 → `00 00 00 00 00 00 00 40`).
    #[test]
    fn f64_constant_uses_exact_byte_form() {
        let instrs = vec![
            IIRInstr::new("const", Some("two".into()), vec![Operand::Float(2.0)], "f64"),
            IIRInstr::new("ret", None, vec![Operand::Var("two".into())], "i32"),
        ];
        let mut m = IIRModule::new("Main", "test");
        m.functions.push(IIRFunction::new("main", vec![], "i32", instrs));
        m.entry_point = Some("main".into());
        let il = emit_il(&m, &IIRClrConfig::new("Main")).unwrap();
        assert!(il.contains("ldc.r8 (00 00 00 00 00 00 00 40)"),
            "2.0 should be the exact IEEE-754 LE bytes; got:\n{il}");
    }
}
