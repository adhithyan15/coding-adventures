//! Strict scalar lowering has its own validation boundary. Legacy heap and
//! dynamically typed programs must never silently select this ABI.
use crate::{IIRClrConfig, IIRClrError};
use interpreter_ir::{IIRModule, Operand};
use ir_to_cil_bytecode::builder::CILBytecodeBuilder;
use ir_to_cil_bytecode::{CILMethodArtifact, CILProgramArtifact, SequentialCILTokenProvider};
use std::collections::HashMap;

fn invalid(function: &str, detail: &str) -> IIRClrError {
    IIRClrError::InvalidOperand {
        function: function.into(),
        detail: detail.into(),
    }
}
fn width(function: &str, hint: &str) -> Result<&'static str, IIRClrError> {
    match hint {
        "i32" => Ok("int32"),
        "i64" => Ok("int64"),
        _ => Err(IIRClrError::UnsupportedType {
            function: function.into(),
            type_hint: hint.into(),
        }),
    }
}
#[derive(Clone, Copy)]
struct Slot {
    ty: &'static str,
    index: u16,
    argument: bool,
}
fn load(b: &mut CILBytecodeBuilder, s: Slot) {
    // Parameter indices were checked against u8 before creating the slot.
    if s.argument {
        b.emit_ldarg(u8::try_from(s.index).expect("validated argument index"));
    } else {
        b.emit_ldloc(s.index);
    }
}

/// Lower only explicitly typed straight-line i32/i64 scalar functions.
///
/// Unsupported operations or inconsistent signatures return an error, never a
/// legacy fallback. Integer immediates retain the CLR01 i32 range restriction;
/// i64 intermediates nevertheless execute at their declared full width.
pub fn lower_typed_scalars_to_cil(
    module: &IIRModule,
    _config: &IIRClrConfig,
) -> Result<CILProgramArtifact, IIRClrError> {
    if module.functions.len() > 0xFFFFFF {
        return Err(invalid("<module>", "too many MethodDef rows"));
    }
    let mut functions = HashMap::new();
    // Check every signature first so forward and recursive calls resolve using
    // the same stable method order used by the final token provider.
    for (i, f) in module.functions.iter().enumerate() {
        if f.name.is_empty() || functions.insert(f.name.as_str(), i).is_some() {
            return Err(invalid(&f.name, "empty or duplicate function name"));
        }
        width(&f.name, &f.return_type)?;
        if f.params.len() > 256 {
            return Err(invalid(&f.name, "too many parameters"));
        }
        for (_, ty) in &f.params {
            width(&f.name, ty)?;
        }
    }
    if let Some(entry) = &module.entry_point {
        if !functions.contains_key(entry.as_str()) {
            return Err(invalid("<module>", "undefined entry point"));
        }
    }
    let mut methods = Vec::new();
    for f in &module.functions {
        let mut slots: HashMap<&str, Slot> = HashMap::new();
        for (i, (name, hint)) in f.params.iter().enumerate() {
            let slot = Slot {
                ty: width(&f.name, hint)?,
                index: u16::try_from(i)
                    .map_err(|_| invalid(&f.name, "parameter index overflow"))?,
                argument: true,
            };
            if name.is_empty() || slots.insert(name, slot).is_some() {
                return Err(invalid(&f.name, "empty or duplicate parameter"));
            }
        }
        if f.instructions.last().map(|i| i.op.as_str()) != Some("ret") {
            return Err(invalid(&f.name, "function must end with ret"));
        }
        let mut b = CILBytecodeBuilder::new();
        let mut locals = Vec::new();
        for (position, ins) in f.instructions.iter().enumerate() {
            let ty = width(&f.name, &ins.type_hint)?;
            let op = ins.op.as_str();
            let arity = match op {
                "const" | "mov" | "neg" | "ret" => 1,
                "add" | "sub" | "mul" | "div" | "and" | "or" | "xor" => 2,
                "call" => ins.srcs.len(),
                _ => {
                    return Err(IIRClrError::UnsupportedOp {
                        function: f.name.clone(),
                        op: ins.op.clone(),
                    })
                }
            };
            if ins.srcs.len() != arity {
                return Err(invalid(&f.name, "incorrect operand count"));
            }
            if op == "ret" {
                if ins.dest.is_some()
                    || position + 1 != f.instructions.len()
                    || ty != width(&f.name, &f.return_type)?
                {
                    return Err(invalid(&f.name, "invalid return shape or width"));
                }
            } else {
                let dest = ins
                    .dest
                    .as_deref()
                    .ok_or_else(|| invalid(&f.name, "missing destination"))?;
                if dest.is_empty() || slots.contains_key(dest) {
                    return Err(invalid(&f.name, "empty or duplicate destination"));
                }
                if locals.len() >= 256 {
                    return Err(invalid(&f.name, "too many locals"));
                }
            }
            let operand = |src: &Operand, expected: &str| -> Result<Slot, IIRClrError> {
                let Operand::Var(name) = src else {
                    return Err(invalid(&f.name, "expected variable operand"));
                };
                let slot = slots
                    .get(name.as_str())
                    .copied()
                    .ok_or_else(|| invalid(&f.name, "undefined or forward variable"))?;
                if slot.ty != expected {
                    return Err(invalid(&f.name, "operand width mismatch"));
                }
                Ok(slot)
            };
            match op {
                "const" => {
                    let Operand::Int(n) = ins.srcs[0] else {
                        return Err(invalid(&f.name, "expected integer literal"));
                    };
                    let narrow = i32::try_from(n)
                        .map_err(|_| invalid(&f.name, "integer immediate exceeds CLR01 range"))?;
                    if ty == "int64" {
                        b.emit_ldc_i8(n);
                    } else {
                        b.emit_ldc_i4(narrow);
                    }
                }
                "call" => {
                    let Some(Operand::Var(name)) = ins.srcs.first() else {
                        return Err(invalid(&f.name, "missing direct callee"));
                    };
                    let index = *functions
                        .get(name.as_str())
                        .ok_or_else(|| invalid(&f.name, "undefined direct callee"))?;
                    let callee = &module.functions[index];
                    if ins.srcs.len() - 1 != callee.params.len()
                        || ty != width(&f.name, &callee.return_type)?
                    {
                        return Err(invalid(&f.name, "call signature mismatch"));
                    }
                    for (src, (_, hint)) in ins.srcs[1..].iter().zip(&callee.params) {
                        load(&mut b, operand(src, width(&f.name, hint)?)?);
                    }
                    let row = u32::try_from(index + 1)
                        .map_err(|_| invalid(&f.name, "method index overflow"))?;
                    b.emit_call(0x06000000 | row);
                }
                _ => {
                    for src in &ins.srcs {
                        load(&mut b, operand(src, ty)?);
                    }
                    match op {
                        "mov" => {}
                        "ret" => b.emit_ret(),
                        "neg" => b.emit_raw(vec![0x65]),
                        "add" => b.emit_add(),
                        "sub" => b.emit_sub(),
                        "mul" => b.emit_mul(),
                        "div" => b.emit_div(),
                        "and" => b.emit_and(),
                        "or" => b.emit_or(),
                        "xor" => b.emit_xor(),
                        _ => unreachable!("validated opcode"),
                    }
                }
            }
            if op != "ret" {
                let index = u16::try_from(locals.len())
                    .map_err(|_| invalid(&f.name, "local index overflow"))?;
                b.emit_stloc(index);
                locals.push(ty.to_string());
                slots.insert(
                    ins.dest.as_deref().expect("validated destination"),
                    Slot {
                        ty,
                        index,
                        argument: false,
                    },
                );
            }
        }
        methods.push(CILMethodArtifact {
            name: f.name.clone(),
            body: b.assemble().map_err(|e| IIRClrError::AssemblyError {
                function: f.name.clone(),
                detail: format!("{e:?}"),
            })?,
            max_stack: 256,
            local_types: locals,
            parameter_types: f
                .params
                .iter()
                .map(|(_, h)| width(&f.name, h).map(str::to_string))
                .collect::<Result<_, _>>()?,
            return_type: width(&f.name, &f.return_type)?,
        });
    }
    let labels: Vec<&str> = module.functions.iter().map(|f| f.name.as_str()).collect();
    Ok(CILProgramArtifact {
        entry_label: module
            .entry_point
            .clone()
            .unwrap_or_else(|| labels.first().copied().unwrap_or("").into()),
        methods,
        data_offsets: HashMap::new(),
        data_size: 0,
        helper_specs: vec![],
        token_provider: Box::new(SequentialCILTokenProvider::new(&labels)),
    })
}
