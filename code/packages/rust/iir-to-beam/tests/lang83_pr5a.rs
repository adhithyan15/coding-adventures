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
const OP_GC_BIF1: u8 = 124;
const OP_GC_BIF2: u8 = 125;

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

// ── The trapping `length/1` save area ────────────────────────────────────────
//
// On a real VM (OTP 22+) the loader rewrites `gc_bif1 Fail Live length/1 Src
// Dst` into
//
//   i_length_setup Live Src     ; x[Live] = Src, x[Live+1] = 0, x[Live+2] = Src
//   i_length Fail Live Dst      ; walk (may yield and resume), then write Dst
//
// so `length` can pause half-way down a long list. Those three registers are
// overwritten BEFORE `Dst` is written: `Live` is a clobber line, not a mere GC
// hint. The first str_concat ceiling check computed `length(left)` into
// x[live] and then ran `length(right)` with the same `Live`, so the setup
// stored the string `right` over it and real `erl` raised
// `badarith erlang:'+'("K", 1)` (CI on PR #18799, three real-`erl` tests).
//
// There is no `erl` here, so this pins the rule on the instruction stream: in
// the straight-line code after each `length`, no register of its save area
// other than `Dst` may be READ before something writes it again.

/// Registers `ins` reads and the one it writes, for the opcodes that appear
/// in the straight-line string expansions. `None` ends the scan: a label,
/// jump or call changes the control flow or clobbers everything anyway.
fn reads_writes(ins: &BEAMInstruction) -> Option<(Vec<u64>, Option<u64>)> {
    let x = |i: usize| (ins.operands[i].tag == BEAMTag::X).then_some(ins.operands[i].value);
    match ins.opcode {
        OP_MOVE => Some((x(0).into_iter().collect(), x(1))),
        OP_GC_BIF1 => Some((x(3).into_iter().collect(), x(4))),
        OP_GC_BIF2 => Some(([x(3), x(4)].into_iter().flatten().collect(), x(5))),
        OP_IS_LT | OP_IS_GE => Some(([x(1), x(2)].into_iter().flatten().collect(), None)),
        OP_IS_INTEGER => Some((x(1).into_iter().collect(), None)),
        _ => None,
    }
}

/// Fail if any `length` call's save area is read after the call clobbered it.
fn assert_length_save_area_never_read(m: &BEAMModule) {
    let code = &m.instructions;
    let mut seen = 0;
    for (pc, ins) in code.iter().enumerate() {
        if ins.opcode != OP_GC_BIF1 || import(m, ins.operands[2].value) != "erlang:length/1" {
            continue;
        }
        seen += 1;
        let live = ins.operands[1].value;
        let dst = ins.operands[4].value;
        // Registers still holding the setup's junk.
        let mut junk: Vec<u64> = (live..live + 3).filter(|&r| r != dst).collect();
        for later in &code[pc + 1..] {
            let Some((reads, write)) = reads_writes(later) else { break };
            for r in &reads {
                assert!(
                    !junk.contains(r),
                    "pc {pc}: `length` with Live={live} clobbered x{r}, which {later:?} then reads"
                );
            }
            if let Some(w) = write {
                junk.retain(|&r| r != w);
            }
        }
    }
    assert!(seen > 0, "the module computes at least one length");
}

#[test]
fn length_save_area_never_holds_a_value_still_needed() {
    let concat = vec![
        IIRInstr::new("str_concat", Some("c".into()), vec![v("a"), v("b")], "str"),
        IIRInstr::new("str_len", Some("n".into()), vec![v("c")], "i64"),
        IIRInstr::new("ret", None, vec![v("n")], "i64"),
    ];
    let index = vec![
        IIRInstr::new("str_index", Some("n".into()), vec![v("a"), v("b")], "i64"),
        IIRInstr::new("ret", None, vec![v("n")], "i64"),
    ];
    let slice = vec![
        IIRInstr::new("str_slice", Some("t".into()), vec![v("a"), v("b"), v("c")], "str"),
        IIRInstr::new("str_len", Some("n".into()), vec![v("t")], "i64"),
        IIRInstr::new("ret", None, vec![v("n")], "i64"),
    ];
    for (params, body) in [(&["a", "b"][..], concat), (&["a", "b"][..], index), (&["a", "b", "c"][..], slice)] {
        assert_length_save_area_never_read(&lower(params, body, None));
    }
}

#[test]
fn str_concat_second_length_declares_the_first_result_live() {
    let m = lower(
        &["a", "b"],
        vec![
            IIRInstr::new("str_concat", Some("c".into()), vec![v("a"), v("b")], "str"),
            IIRInstr::new("ret", None, vec![v("c")], "i64"),
        ],
        None,
    );
    let lengths: Vec<&BEAMInstruction> = m
        .instructions
        .iter()
        .filter(|i| i.opcode == OP_GC_BIF1 && import(&m, i.operands[2].value) == "erlang:length/1")
        .collect();
    assert_eq!(lengths.len(), 2, "length(a) and length(b)");
    let first_dst = lengths[0].operands[4].value;
    let second_live = lengths[1].operands[1].value;
    assert!(
        second_live > first_dst,
        "length(b)'s save area (x{second_live}..) must start above length(a)'s result x{first_dst}"
    );
}
