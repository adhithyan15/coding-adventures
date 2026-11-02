//! LANG83 PR5a on the BEAM backend: string failures are registered traps.
//!
//! | op           | before PR5a                               | PR5a                                   |
//! |--------------|-------------------------------------------|----------------------------------------|
//! | `str_index`  | `lists:nth` → `function_clause` (no kind) | `is_integer`, `0 ≤ i < length` guards → `erlang:error('Trap.Bounds')` |
//! | `str_slice`  | range guards → `erlang:error(badarg)`     | `is_integer` first, then the same guards → `'Trap.Bounds'` |
//! | `str_concat` | no ceiling                                | `length(A) + length(B) > Ceiling` → `'Trap.Overflow'` |
//!
//! `is_integer` comes first because Erlang orders numbers across types: a
//! float such as `1.5` would pass `0 ≤ i < length` (lesson "BEAM array indexes
//! need an integer guard before numeric bounds checks").
//!
//! There is no `erl` on the development hosts, so these tests pin the
//! instruction stream; CI's BEAM `lang_matrix` column runs the emulator.

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_beam::{lower_iir_to_beam, validate_for_beam, BEAMModule, IIRBeamConfig};
use ir_to_beam::{BEAMInstruction, BEAMTag};

const OP_LABEL: u8 = 1;
const OP_CALL_EXT: u8 = 7;
const OP_IS_LT: u8 = 39;
const OP_IS_GE: u8 = 40;
const OP_IS_INTEGER: u8 = 45;
const OP_MOVE: u8 = 64;

fn v(n: &str) -> Operand {
    Operand::Var(n.into())
}

fn lower(params: &[&str], body: Vec<IIRInstr>, ceiling: Option<u32>) -> BEAMModule {
    let params = params.iter().map(|p| (p.to_string(), "any".to_string())).collect();
    let mut module = IIRModule::new("lang83", "test");
    module.functions = vec![IIRFunction::new("f", params, "i64", body)];
    module.entry_point = Some("f".into());
    module.str_max_bytes = ceiling;
    assert!(validate_for_beam(&module).is_empty(), "{:?}", validate_for_beam(&module));
    lower_iir_to_beam(&module, &IIRBeamConfig::new("lang83")).expect("lowers")
}

fn atom(m: &BEAMModule, idx: u64) -> &str {
    &m.atoms[idx as usize - 1]
}

fn import(m: &BEAMModule, idx: u64) -> String {
    let i = &m.imports[idx as usize];
    format!("{}:{}/{}", atom(m, u64::from(i.module_atom_index)), atom(m, u64::from(i.function_atom_index)), i.arity)
}

/// Every `erlang:error/1` call, as `(pc, reason atom)`: the reason is moved
/// into x0 by the instruction just before the call.
fn raises(m: &BEAMModule) -> Vec<(usize, String)> {
    let code = &m.instructions;
    (1..code.len())
        .filter(|&pc| code[pc].opcode == OP_CALL_EXT && import(m, code[pc].operands[1].value) == "erlang:error/1")
        .map(|pc| {
            let mv = &code[pc - 1];
            assert_eq!(mv.opcode, OP_MOVE, "the reason is moved into x0 right before the call");
            assert_eq!(mv.operands[0].tag, BEAMTag::A);
            (pc, atom(m, mv.operands[0].value).to_string())
        })
        .collect()
}

/// The label number a `{f, L}` failure operand jumps to.
fn fail_label(i: &BEAMInstruction) -> u64 {
    assert_eq!(i.operands[0].tag, BEAMTag::F);
    i.operands[0].value
}

/// The pc of `label L`.
fn label_pc(m: &BEAMModule, l: u64) -> usize {
    m.instructions.iter().position(|i| i.opcode == OP_LABEL && i.operands[0].value == l).expect("label")
}

#[test]
fn str_index_guards_integer_and_range_then_raises_trap_bounds() {
    let m = lower(
        &["s", "i"],
        vec![
            IIRInstr::new("str_index", Some("b".into()), vec![v("s"), v("i")], "i64"),
            IIRInstr::new("ret", None, vec![v("b")], "i64"),
        ],
        None,
    );
    let raises = raises(&m);
    assert_eq!(raises.iter().map(|(_, a)| a.as_str()).collect::<Vec<_>>(), ["Trap.Bounds"]);
    let code = &m.instructions;
    let guards: Vec<&BEAMInstruction> =
        code.iter().filter(|i| matches!(i.opcode, OP_IS_INTEGER | OP_IS_GE | OP_IS_LT)).collect();
    assert_eq!(guards.iter().map(|g| g.opcode).collect::<Vec<_>>(), [OP_IS_INTEGER, OP_IS_GE, OP_IS_LT]);
    // Every guard fails to the label just before the raise.
    let trap_pc = raises[0].0 - 2;
    for g in guards {
        assert_eq!(label_pc(&m, fail_label(g)), trap_pc, "{g:?}");
    }
    // The guards all come before `lists:nth`.
    let nth = (0..code.len())
        .find(|&pc| code[pc].opcode == OP_CALL_EXT && import(&m, code[pc].operands[1].value) == "lists:nth/2")
        .expect("lists:nth");
    assert!(trap_pc < nth);
}

#[test]
fn str_slice_guards_integers_first_and_raises_trap_bounds_not_badarg() {
    let m = lower(
        &["s", "a", "b"],
        vec![
            IIRInstr::new("str_slice", Some("t".into()), vec![v("s"), v("a"), v("b")], "str"),
            IIRInstr::new("str_len", Some("n".into()), vec![v("t")], "i64"),
            IIRInstr::new("ret", None, vec![v("n")], "i64"),
        ],
        None,
    );
    assert_eq!(raises(&m).iter().map(|(_, a)| a.as_str()).collect::<Vec<_>>(), ["Trap.Bounds"]);
    let ops: Vec<u8> =
        m.instructions.iter().map(|i| i.opcode).filter(|o| matches!(*o, OP_IS_INTEGER | OP_IS_GE)).collect();
    assert_eq!(ops, [OP_IS_INTEGER, OP_IS_INTEGER, OP_IS_GE, OP_IS_GE, OP_IS_GE], "integers, then ranges");
}

#[test]
fn str_concat_raises_trap_overflow_past_the_module_ceiling() {
    for (ceiling, want) in [(Some(16), 16u64), (None, u64::from(interpreter_ir::STR_MAX_BYTES))] {
        let m = lower(
            &["a", "b"],
            vec![
                IIRInstr::new("str_concat", Some("c".into()), vec![v("a"), v("b")], "str"),
                IIRInstr::new("str_len", Some("n".into()), vec![v("c")], "i64"),
                IIRInstr::new("ret", None, vec![v("n")], "i64"),
            ],
            ceiling,
        );
        let raises = raises(&m);
        assert_eq!(raises.len(), 1);
        assert_eq!(raises[0].1, "Trap.Overflow");
        // `is_lt {f,OK} Ceiling Sum` falls through to the raise when Sum > Ceiling.
        let check = &m.instructions[raises[0].0 - 2];
        assert_eq!(check.opcode, OP_IS_LT);
        assert_eq!((check.operands[1].tag, check.operands[1].value), (BEAMTag::I, want));
        assert_eq!(label_pc(&m, fail_label(check)), raises[0].0 + 1, "OK is just past the raise");
    }
}
