//! LANG83 PR1b on the BEAM backend: `i64_div_floor`, `i64_mod_floor`, `trap`,
//! `write_i64`.
//!
//! There is no `erl` on the development hosts (CI's BEAM column runs the real
//! emulator), so besides pinning the instruction shapes these tests **execute
//! the emitted BEAM instructions** on a tiny model of the handful of opcodes
//! the floor-division sequence uses (`label`, `is_eq_exact`, `is_lt`, `jump`,
//! `move`, `gc_bif2` over `div`/`rem`/`+`/`-`, `call_ext erlang:error/1`,
//! `return`). Erlang integers are unbounded, so the model computes in `i128`
//! — exactly the property that makes `MIN div -1` a bignum on real BEAM, and
//! the reason the overflow edge must be tested explicitly.

use std::collections::HashMap;

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_beam::{lower_iir_to_beam, validate_for_beam, BEAMModule, IIRBeamConfig};
use ir_to_beam::{BEAMOperand, BEAMTag};

const OP_LABEL: u8 = 1;
const OP_FUNC_INFO: u8 = 2;
const OP_CALL_EXT: u8 = 7;
const OP_RETURN: u8 = 19;
const OP_IS_LT: u8 = 39;
const OP_IS_EQ_EXACT: u8 = 43;
const OP_JUMP: u8 = 61;
const OP_MOVE: u8 = 64;
const OP_GC_BIF2: u8 = 125;
const OP_TEST_HEAP: u8 = 16;
const OP_PUT_LIST: u8 = 69;

fn v(n: &str) -> Operand {
    Operand::Var(n.into())
}

fn lower(name: &str, params: Vec<(String, String)>, ret: &str, body: Vec<IIRInstr>) -> BEAMModule {
    let module = IIRModule {
        name: "lang83".into(),
        functions: vec![IIRFunction::new(name, params, ret, body)],
        entry_point: Some(name.into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    assert!(validate_for_beam(&module).is_empty(), "{:?}", validate_for_beam(&module));
    lower_iir_to_beam(&module, &IIRBeamConfig::new("lang83")).expect("lowers")
}

fn atom(m: &BEAMModule, idx: u64) -> &str {
    &m.atoms[idx as usize - 1] // 1-based; 0 is nil
}

/// `module:function/arity` of import-table row `idx`.
fn import(m: &BEAMModule, idx: u64) -> String {
    let i = &m.imports[idx as usize];
    format!(
        "{}:{}/{}",
        atom(m, u64::from(i.module_atom_index)),
        atom(m, u64::from(i.function_atom_index)),
        i.arity
    )
}

#[derive(Debug, Clone, PartialEq)]
enum Val {
    Int(i128),
    Nil,
    Atom(String),
}

/// How the modelled function ended.
#[derive(Debug, PartialEq)]
enum Outcome {
    Returned(i128),
    Raised(String),
}

/// Run the single function in `m` with `args` in x0, x1, … on the opcode model.
fn run(m: &BEAMModule, args: &[i64]) -> Outcome {
    let code = &m.instructions;
    let labels: HashMap<u64, usize> = code
        .iter()
        .enumerate()
        .filter(|(_, i)| i.opcode == OP_LABEL)
        .map(|(pc, i)| (i.operands[0].value, pc))
        .collect();
    // Start just after the func_info's following label (the entry label).
    let fi = code.iter().position(|i| i.opcode == OP_FUNC_INFO).expect("func_info");
    let mut pc = fi + 2;
    let mut x: HashMap<u64, Val> = args.iter().enumerate().map(|(i, a)| (i as u64, Val::Int(i128::from(*a)))).collect();
    let read = |x: &HashMap<u64, Val>, op: &BEAMOperand| -> Val {
        match op.tag {
            BEAMTag::X => x.get(&op.value).cloned().unwrap_or_else(|| panic!("read of unset x{}", op.value)),
            BEAMTag::I => Val::Int(i128::from(op.value as i64)),
            BEAMTag::A if op.value == 0 => Val::Nil,
            BEAMTag::A => Val::Atom(atom(m, op.value).to_string()),
            other => panic!("operand tag {other:?} not modelled"),
        }
    };
    let int = |v: Val| match v {
        Val::Int(n) => n,
        other => panic!("arithmetic on {other:?}"),
    };
    for _ in 0..10_000 {
        let ins = &code[pc];
        let ops = &ins.operands;
        pc += 1;
        match ins.opcode {
            OP_LABEL => {}
            OP_MOVE => {
                let val = read(&x, &ops[0]);
                x.insert(ops[1].value, val);
            }
            OP_JUMP => pc = labels[&ops[0].value],
            OP_IS_EQ_EXACT | OP_IS_LT => {
                let (l, r) = (read(&x, &ops[1]), read(&x, &ops[2]));
                let holds = if ins.opcode == OP_IS_EQ_EXACT { l == r } else { int(l) < int(r) };
                if !holds {
                    pc = labels[&ops[0].value];
                }
            }
            OP_GC_BIF2 => {
                let (l, r) = (int(read(&x, &ops[3])), int(read(&x, &ops[4])));
                let result = match import(m, ops[2].value).as_str() {
                    "erlang:div/2" => {
                        assert_ne!(r, 0, "erlang:div by zero raises badarith — the guard must prevent it");
                        l / r // Erlang `div` truncates, like Rust's `/`
                    }
                    "erlang:rem/2" => {
                        assert_ne!(r, 0, "erlang:rem by zero raises badarith — the guard must prevent it");
                        l % r
                    }
                    "erlang:+/2" => l + r,
                    "erlang:-/2" => l - r,
                    other => panic!("bif {other} not modelled"),
                };
                // Every gc_bif must count the floor temp as live: a register at
                // or above `live` would not survive a GC.
                assert!(ops[5].value < ops[1].value || ops[5].value == 0, "dest x{} is outside live={}", ops[5].value, ops[1].value);
                x.insert(ops[5].value, Val::Int(result));
            }
            OP_CALL_EXT => {
                assert_eq!(import(m, ops[1].value), "erlang:error/1");
                match read(&x, &BEAMOperand { tag: BEAMTag::X, value: 0 }) {
                    Val::Atom(a) => return Outcome::Raised(a),
                    other => panic!("erlang:error with {other:?}"),
                }
            }
            OP_RETURN => return Outcome::Returned(int(read(&x, &BEAMOperand { tag: BEAMTag::X, value: 0 }))),
            other => panic!("opcode {other} not modelled (pc {})", pc - 1),
        }
    }
    panic!("did not terminate");
}

fn floor_fn(op: &str) -> BEAMModule {
    lower(
        "f",
        vec![("a".into(), "i64".into()), ("b".into(), "i64".into())],
        "i64",
        vec![
            IIRInstr::new(op, Some("q".into()), vec![v("a"), v("b")], "i64"),
            IIRInstr::new("ret", None, vec![v("q")], "i64"),
        ],
    )
}

#[test]
fn i64_div_floor_follows_the_sign_table_and_traps_on_the_edges() {
    let m = floor_fn("i64_div_floor");
    for (a, b, q) in [(7, 2, 3), (-7, 2, -4), (7, -2, -4), (-7, -2, 3), (6, -3, -2), (0, 5, 0), (i64::MIN, 1, i64::MIN), (i64::MAX, -1, -i64::MAX)] {
        assert_eq!(run(&m, &[a, b]), Outcome::Returned(i128::from(q)), "{a} // {b}");
    }
    assert_eq!(run(&m, &[7, 0]), Outcome::Raised("Trap.DivZero".into()));
    assert_eq!(run(&m, &[i64::MIN, -1]), Outcome::Raised("Trap.Overflow".into()), "never the bignum 2^63");
}

#[test]
fn i64_mod_floor_follows_the_sign_table_and_traps_only_on_zero() {
    let m = floor_fn("i64_mod_floor");
    for (a, b, r) in [(7, 2, 1), (-7, 2, 1), (7, -2, -1), (-7, -2, -1), (6, -3, 0), (i64::MIN, -1, 0), (i64::MIN, 1, 0), (i64::MIN, 7, 6)] {
        assert_eq!(run(&m, &[a, b]), Outcome::Returned(i128::from(r)), "{a} % {b}");
    }
    assert_eq!(run(&m, &[7, 0]), Outcome::Raised("Trap.DivZero".into()));
}

#[test]
fn floor_dest_may_alias_an_operand() {
    // `a = a // b`: the quotient is computed after the last read of the temp.
    let m = lower(
        "f",
        vec![("a".into(), "i64".into()), ("b".into(), "i64".into())],
        "i64",
        vec![
            IIRInstr::new("i64_div_floor", Some("a".into()), vec![v("a"), v("b")], "i64"),
            IIRInstr::new("ret", None, vec![v("a")], "i64"),
        ],
    );
    assert_eq!(run(&m, &[-7, 2]), Outcome::Returned(-4));
    assert_eq!(run(&m, &[7, -2]), Outcome::Returned(-4));
}

#[test]
fn trap_raises_the_kind_atom() {
    let m = lower(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("trap", None, vec![Operand::Str("Trap.User".into())], "void"),
            IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new("ret", None, vec![v("z")], "i64"),
        ],
    );
    assert_eq!(run(&m, &[]), Outcome::Raised("Trap.User".into()));
    let bad = IIRModule {
        name: "lang83".into(),
        functions: vec![IIRFunction::new(
            "main",
            vec![],
            "void",
            vec![IIRInstr::new("trap", None, vec![Operand::Str("Nope".into())], "void")],
        )],
        entry_point: Some("main".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    assert!(lower_iir_to_beam(&bad, &IIRBeamConfig::new("lang83")).is_err());
}

#[test]
fn write_i64_formats_without_a_newline_through_the_io_server() {
    let m = lower(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("const", Some("n".into()), vec![Operand::Int(-5)], "i64"),
            IIRInstr::new("call_builtin", None, vec![v("write_i64"), v("n")], "void"),
            IIRInstr::new("ret", None, vec![v("n")], "i64"),
        ],
    );
    let code = &m.instructions;
    let call = code.iter().position(|i| i.opcode == OP_CALL_EXT).expect("call_ext");
    assert_eq!(import(&m, code[call].operands[1].value), "io:format/2");
    // test_heap 6 for [X] ++ "~w"; then the format string is built "~", "w".
    let heap = code.iter().position(|i| i.opcode == OP_TEST_HEAP).expect("test_heap");
    assert_eq!(code[heap].operands[0].value, 6);
    let chars: Vec<u64> = code[heap..call]
        .iter()
        .filter(|i| i.opcode == OP_PUT_LIST && i.operands[0].tag == BEAMTag::I)
        .map(|i| i.operands[0].value)
        .collect();
    assert_eq!(chars, [u64::from(b'w'), u64::from(b'~')], "\"~w\" — no ~n");
}

/// Security review (LANG83 PR1b, LOW): the floor expansion's scratch register
/// used to be looked up by the name `__lang83_floor_tmp` and reused if a user
/// variable already had that name — so the expansion clobbered it
/// (`__lang83_floor_tmp = 42; q = i64_div_floor(7, 2)` left the variable
/// holding the remainder, 1). The scratch register is now anonymous; a user
/// variable of exactly that name keeps its value.
#[test]
fn user_variable_named_like_the_floor_temp_is_not_clobbered() {
    for op in ["i64_div_floor", "i64_mod_floor"] {
        let m = lower(
            "f",
            vec![("a".into(), "i64".into()), ("b".into(), "i64".into())],
            "i64",
            vec![
                IIRInstr::new("const", Some("__lang83_floor_tmp".into()), vec![Operand::Int(42)], "i64"),
                IIRInstr::new(op, Some("q".into()), vec![v("a"), v("b")], "i64"),
                IIRInstr::new("add", Some("r".into()), vec![v("q"), v("__lang83_floor_tmp")], "i64"),
                IIRInstr::new("ret", None, vec![v("r")], "i64"),
            ],
        );
        // 7 // 2 = 3 and 7 mod 2 = 1, plus the untouched 42. (-7, 2) takes the
        // adjust path, where the remainder register is read twice.
        let (q, q_neg) = if op == "i64_div_floor" { (3, -4) } else { (1, 1) };
        assert_eq!(run(&m, &[7, 2]), Outcome::Returned(q + 42), "{op}(7, 2)");
        assert_eq!(run(&m, &[-7, 2]), Outcome::Returned(q_neg + 42), "{op}(-7, 2)");
    }
}
