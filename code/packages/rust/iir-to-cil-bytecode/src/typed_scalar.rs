//! Strict scalar lowering has its own validation boundary. Legacy heap and
//! dynamically typed programs must never silently select this ABI.
use crate::{IIRClrConfig, IIRClrError};
use interpreter_ir::{IIRFunction, IIRModule, Operand};
use ir_to_cil_bytecode::builder::CILBytecodeBuilder;
use ir_to_cil_bytecode::CILBranchKind;
use ir_to_cil_bytecode::{CILMethodArtifact, CILProgramArtifact, SequentialCILTokenProvider};
use std::collections::{HashMap, HashSet};

const BASIC_INPUT_I64_TOKEN: u32 = 0x0A00_0006;
const BASIC_INPUT_MORE_TOKEN: u32 = 0x0A00_0007;
const BASIC_INPUT_STR_TOKEN: u32 = 0x0A00_0008;

fn invalid(function: &str, detail: &str) -> IIRClrError {
    IIRClrError::InvalidOperand {
        function: function.into(),
        detail: detail.into(),
    }
}
// Logical types remain distinct even when the evaluation stack representation
// is shared. In particular, a normalized boolean is not an arithmetic i32.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ScalarType {
    I32,
    I64,
    Bool,
    Str,
}
impl ScalarType {
    fn metadata(self) -> &'static str {
        match self {
            Self::I64 => "int64",
            Self::I32 | Self::Bool => "int32",
            Self::Str => "string",
        }
    }
}
fn width(function: &str, hint: &str) -> Result<ScalarType, IIRClrError> {
    match hint {
        "i32" => Ok(ScalarType::I32),
        "i64" => Ok(ScalarType::I64),
        "bool" => Ok(ScalarType::Bool),
        "str" => Ok(ScalarType::Str),
        _ => Err(IIRClrError::UnsupportedType {
            function: function.into(),
            type_hint: hint.into(),
        }),
    }
}
#[derive(Clone, Copy)]
struct Slot {
    ty: ScalarType,
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

// Forward edges let us prove assignment with one pass: a join receives only
// names present on every predecessor, never the union of names seen in text.
fn validate_flow(f: &IIRFunction) -> Result<(), IIRClrError> {
    let n = f.instructions.len();
    if n == 0 {
        return Err(invalid(&f.name, "empty function"));
    }
    // Bound assignment sets before propagation. Otherwise malformed functions
    // with thousands of destinations cause quadratic copying before refusal.
    if f.instructions
        .iter()
        .filter(|ins| ins.dest.is_some())
        .take(257)
        .count()
        > 256
    {
        return Err(invalid(&f.name, "too many locals"));
    }
    let mut labels = HashMap::new();
    for (i, ins) in f.instructions.iter().enumerate() {
        if matches!(
            ins.op.as_str(),
            "label" | "jmp" | "jmp_if_true" | "jmp_if_false"
        ) {
            let count = if matches!(ins.op.as_str(), "jmp_if_true" | "jmp_if_false") {
                2
            } else {
                1
            };
            if ins.dest.is_some()
                || ins.type_hint != "void"
                || ins.srcs.len() != count
                || ins
                    .srcs
                    .iter()
                    .any(|s| !matches!(s, Operand::Var(v) if !v.is_empty()))
            {
                return Err(invalid(&f.name, "invalid control shape or hint"));
            }
            if ins.op == "label" {
                let Operand::Var(name) = &ins.srcs[0] else {
                    unreachable!()
                };
                if labels.insert(name.as_str(), i).is_some() {
                    return Err(invalid(&f.name, "duplicate label"));
                }
            }
        }
    }
    let mut incoming: Vec<Option<HashSet<&str>>> = vec![None; n];
    incoming[0] = Some(f.params.iter().map(|(name, _)| name.as_str()).collect());
    for (i, ins) in f.instructions.iter().enumerate() {
        let mut assigned = incoming[i]
            .take()
            .ok_or_else(|| invalid(&f.name, "unreachable instruction"))?;
        let op = ins.op.as_str();
        let reads: &[Operand] = match op {
            "const" | "label" | "jmp" => &[],
            "jmp_if_true" | "jmp_if_false" => &ins.srcs[..1],
            "call" | "call_builtin" => ins.srcs.get(1..).unwrap_or(&[]),
            _ => &ins.srcs,
        };
        for src in reads {
            if let Operand::Var(name) = src {
                if !assigned.contains(name.as_str()) {
                    return Err(invalid(
                        &f.name,
                        "undefined or forward variable: not definitely assigned",
                    ));
                }
            }
        }
        if let Some(dest) = &ins.dest {
            assigned.insert(dest.as_str());
        }
        let mut successors = Vec::new();
        if matches!(op, "jmp" | "jmp_if_true" | "jmp_if_false") {
            let Operand::Var(target) = ins.srcs.last().expect("validated control shape") else {
                unreachable!()
            };
            let target = *labels
                .get(target.as_str())
                .ok_or_else(|| invalid(&f.name, "undefined label"))?;
            if target <= i {
                return Err(invalid(&f.name, "branch target must be forward"));
            }
            successors.push(target);
        }
        if !matches!(op, "jmp" | "ret") {
            if i + 1 == n {
                return Err(invalid(
                    &f.name,
                    "function must end with ret: control falls off",
                ));
            }
            successors.push(i + 1);
        }
        for next in successors {
            match &mut incoming[next] {
                Some(previous) => previous.retain(|name| assigned.contains(name)),
                slot @ None => *slot = Some(assigned.clone()),
            }
        }
    }
    Ok(())
}

/// Lower explicitly typed i32/i64/bool/string functions with forward-only control flow.
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
        validate_flow(f)?;
        let mut b = CILBytecodeBuilder::new();
        let mut locals = Vec::new();
        for ins in &f.instructions {
            let op = ins.op.as_str();
            // Control instructions have void hints and do not produce locals.
            // Shape, target and assignment checks have already completed.
            if matches!(op, "label" | "jmp" | "jmp_if_true" | "jmp_if_false") {
                let Operand::Var(target) = ins.srcs.last().expect("validated control shape") else {
                    unreachable!()
                };
                if op == "label" {
                    b.mark(target);
                } else {
                    let kind = match op {
                        "jmp" => CILBranchKind::Always,
                        "jmp_if_true" => CILBranchKind::True,
                        _ => CILBranchKind::False,
                    };
                    if op != "jmp" {
                        let Operand::Var(condition) = &ins.srcs[0] else {
                            unreachable!()
                        };
                        let slot = slots
                            .get(condition.as_str())
                            .copied()
                            .ok_or_else(|| invalid(&f.name, "undefined condition"))?;
                        if slot.ty != ScalarType::Bool {
                            return Err(invalid(&f.name, "branch condition must be bool"));
                        }
                        load(&mut b, slot);
                    }
                    b.emit_branch(kind, target, false);
                }
                continue;
            }
            let ty = width(&f.name, &ins.type_hint)?;
            let arity = match op {
                "const" | "mov" | "neg" | "ret" | "call_builtin" => 1,
                "add" | "sub" | "mul" | "div" | "and" | "or" | "xor" | "cmp_eq" | "cmp_ne"
                | "cmp_lt" | "cmp_le" | "cmp_gt" | "cmp_ge" => 2,
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
                if ins.dest.is_some() || ty != width(&f.name, &f.return_type)? {
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
            let operand = |src: &Operand, expected: ScalarType| -> Result<Slot, IIRClrError> {
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
                "const" if ty == ScalarType::Bool => {
                    let Operand::Bool(value) = ins.srcs[0] else {
                        return Err(invalid(&f.name, "expected boolean literal"));
                    };
                    b.emit_ldc_i4(i32::from(value));
                }
                "const" if ty == ScalarType::Str => {
                    return Err(invalid(&f.name, "encoded string constants are unsupported"));
                }
                "const" => {
                    let Operand::Int(n) = ins.srcs[0] else {
                        return Err(invalid(&f.name, "expected integer literal"));
                    };
                    let narrow = i32::try_from(n)
                        .map_err(|_| invalid(&f.name, "integer immediate exceeds CLR01 range"))?;
                    if ty == ScalarType::I64 {
                        b.emit_ldc_i8(n);
                    } else {
                        b.emit_ldc_i4(narrow);
                    }
                }
                "cmp_eq" | "cmp_ne" | "cmp_lt" | "cmp_le" | "cmp_gt" | "cmp_ge" => {
                    if ty != ScalarType::Bool {
                        return Err(invalid(&f.name, "comparison result must be bool"));
                    }
                    let Operand::Var(name) = &ins.srcs[0] else {
                        return Err(invalid(&f.name, "expected variable operand"));
                    };
                    let input = slots
                        .get(name.as_str())
                        .copied()
                        .ok_or_else(|| invalid(&f.name, "undefined or forward variable"))?;
                    if matches!(input.ty, ScalarType::Bool | ScalarType::Str) {
                        return Err(invalid(&f.name, "comparison requires integer operands"));
                    }
                    let right = operand(&ins.srcs[1], input.ty)?;
                    load(&mut b, input);
                    load(&mut b, right);
                    match op {
                        "cmp_eq" | "cmp_ne" => b.emit_ceq(),
                        "cmp_lt" | "cmp_ge" => b.emit_clt(),
                        "cmp_gt" | "cmp_le" => b.emit_cgt(),
                        _ => unreachable!("comparison opcode"),
                    }
                    // Even an i64 comparison produces an i32 boolean. Invert
                    // that result at its actual stack width, never with ldc.i8.
                    if matches!(op, "cmp_ne" | "cmp_le" | "cmp_ge") {
                        b.emit_ldc_i4(0);
                        b.emit_ceq();
                    }
                }
                "call_builtin" => {
                    let Some(Operand::Var(name)) = ins.srcs.first() else {
                        return Err(invalid(&f.name, "missing input builtin name"));
                    };
                    let token = match name.as_str() {
                        "input_i64" if ty == ScalarType::I64 => BASIC_INPUT_I64_TOKEN,
                        "input_more" if ty == ScalarType::I64 => BASIC_INPUT_MORE_TOKEN,
                        "input_str" if ty == ScalarType::Str => BASIC_INPUT_STR_TOKEN,
                        "input_i64" | "input_more" => {
                            return Err(invalid(&f.name, "integer input result must be i64"))
                        }
                        "input_str" => {
                            return Err(invalid(&f.name, "string input result must be str"))
                        }
                        _ => {
                            return Err(IIRClrError::UnsupportedOp {
                                function: f.name.clone(),
                                op: format!("call_builtin {name:?}"),
                            })
                        }
                    };
                    b.emit_call(token);
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
                    if ty == ScalarType::Bool && !matches!(op, "mov" | "ret") {
                        return Err(invalid(&f.name, "boolean arithmetic is unsupported"));
                    }
                    if ty == ScalarType::Str && !matches!(op, "mov" | "ret") {
                        return Err(invalid(&f.name, "string arithmetic is unsupported"));
                    }
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
                locals.push(ty.metadata().to_string());
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
                .map(|(_, h)| width(&f.name, h).map(|ty| ty.metadata().to_string()))
                .collect::<Result<_, _>>()?,
            return_type: width(&f.name, &f.return_type)?.metadata(),
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
