//! LANG83 PR6a on the BEAM backend: `write_f64_py` / `write_f64_js`.
//!
//! Both lower to `call 2, '__iir_write_f64'/2`, a local function the module
//! carries (see `lower.rs`, `mod lang83_f64`). It takes the shortest digits
//! from `erlang:float_to_list(F, [short])` and lays them out like Python or
//! JavaScript.
//!
//! There is no `erl` here (CI's BEAM column runs the real emulator), so these
//! tests EXECUTE the emitted function on a model of the BEAM opcodes and
//! library functions it uses. The model is strict where real BEAM is
//! unforgiving:
//!
//! * every `call`/`call_ext` clobbers every X register except the result;
//! * a `gc_bif` clobbers every X register at or above its Live count, except
//!   its destination;
//! * reading a clobbered X register or an uninitialised Y slot fails the test.
//!
//! The model's `float_to_list(F, [short])` is fed the same shortest digits in
//! several Erlang-plausible layouts (`1.0e16`, `1.0e+16`, `1e16`,
//! `10000000000000000.0`). For every finite value in the table the output
//! must be what host `python3` and `node` print.

use std::collections::HashMap;

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_beam::{lower_iir_to_beam, validate_for_beam, BEAMModule, IIRBeamConfig};
use ir_to_beam::{BEAMInstruction, BEAMOperand, BEAMTag};

const OP_LABEL: u8 = 1;
const OP_FUNC_INFO: u8 = 2;
const OP_CALL: u8 = 4;
const OP_CALL_EXT: u8 = 7;
const OP_ALLOCATE: u8 = 12;
const OP_DEALLOCATE: u8 = 18;
const OP_RETURN: u8 = 19;
const OP_IS_LT: u8 = 39;
const OP_IS_GE: u8 = 40;
const OP_IS_EQ_EXACT: u8 = 43;
const OP_IS_NIL: u8 = 52;
const OP_JUMP: u8 = 61;
const OP_MOVE: u8 = 64;
const OP_GET_LIST: u8 = 65;
const OP_GC_BIF1: u8 = 124;
const OP_GC_BIF2: u8 = 125;

fn v(n: &str) -> Operand {
    Operand::Var(n.into())
}

/// `main(x: f64) -> i64 { builtin(x); return 0 }`.
fn writer(builtin: &str) -> BEAMModule {
    let module = IIRModule {
        name: "floats".into(),
        functions: vec![IIRFunction::new(
            "main",
            vec![("x".into(), "f64".into())],
            "i64",
            vec![
                IIRInstr::new("call_builtin", None, vec![v(builtin), v("x")], "void"),
                IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "i64"),
                IIRInstr::new("ret", None, vec![v("z")], "i64"),
            ],
        )],
        entry_point: Some("main".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
        release: None,
    };
    assert!(validate_for_beam(&module).is_empty(), "{:?}", validate_for_beam(&module));
    lower_iir_to_beam(&module, &IIRBeamConfig::new("floats")).expect("lowers")
}

fn atom(m: &BEAMModule, idx: u64) -> &str {
    &m.atoms[idx as usize - 1]
}

fn import(m: &BEAMModule, idx: u64) -> String {
    let i = &m.imports[idx as usize];
    format!("{}:{}/{}", atom(m, u64::from(i.module_atom_index)), atom(m, u64::from(i.function_atom_index)), i.arity)
}

#[derive(Debug, Clone, PartialEq)]
enum Val {
    Int(i64),
    Float(f64),
    Atom(String),
    Nil,
    Cons(Box<Val>, Box<Val>),
    /// A register the last call or GC destroyed, or a never-written Y slot.
    Garbage,
}

fn list(items: impl IntoIterator<Item = Val>) -> Val {
    let items: Vec<Val> = items.into_iter().collect();
    items.into_iter().rev().fold(Val::Nil, |tail, h| Val::Cons(Box::new(h), Box::new(tail)))
}

fn chars(s: &str) -> Val {
    list(s.bytes().map(|b| Val::Int(i64::from(b))))
}

fn to_vec(v: &Val) -> Vec<Val> {
    let mut out = Vec::new();
    let mut cur = v;
    loop {
        match cur {
            Val::Nil => return out,
            Val::Cons(h, t) => {
                out.push((**h).clone());
                cur = t;
            }
            other => panic!("not a proper list: {other:?}"),
        }
    }
}

fn to_string(v: &Val) -> String {
    to_vec(v)
        .iter()
        .map(|c| match c {
            Val::Int(n) => char::from(u8::try_from(*n).expect("a byte")),
            other => panic!("not a character: {other:?}"),
        })
        .collect()
}

/// Decode the ETF literals the helper uses: `NEW_FLOAT_EXT`, `STRING_EXT`
/// and `[short]` (`LIST_EXT` of `SMALL_ATOM_UTF8_EXT`).
fn decode_literal(etf: &[u8]) -> Val {
    assert_eq!(etf[0], 131, "ETF version tag");
    match etf[1] {
        70 => Val::Float(f64::from_be_bytes(etf[2..10].try_into().unwrap())),
        107 => chars(std::str::from_utf8(&etf[4..]).unwrap()),
        108 => {
            assert_eq!(&etf[2..6], &[0, 0, 0, 1]);
            assert_eq!(etf[6], 119);
            let len = etf[7] as usize;
            assert_eq!(etf[8 + len], 106, "proper list");
            list([Val::Atom(std::str::from_utf8(&etf[8..8 + len]).unwrap().to_string())])
        }
        tag => panic!("literal tag {tag} not modelled"),
    }
}

/// How the model's `float_to_list(F, [short])` lays out the shortest digits.
#[derive(Clone, Copy, Debug)]
enum Short {
    /// `d.ddde±X` with `.0` when there is one digit, `e-5` / `e16`.
    Sci,
    /// Like `Sci` but the exponent always has a sign (`e+16`).
    SciPlus,
    /// No `.0` padding at all (`1e16`, `5e-324`).
    Bare,
    /// Positional digits with a mandatory fraction (`10000000000000000.0`).
    Fixed,
}

fn short_string(d: &str, e: i32, neg: bool, style: Short) -> String {
    let n = d.len() as i32;
    let sign = if neg { "-" } else { "" };
    if d.is_empty() {
        return format!("{sign}0.0");
    }
    let mant = |pad_one: bool| {
        if n > 1 {
            format!("{}.{}", &d[..1], &d[1..])
        } else if pad_one {
            format!("{d}.0")
        } else {
            d.to_string()
        }
    };
    let body = match style {
        Short::Sci => format!("{}e{e}", mant(true)),
        Short::SciPlus => format!("{}e{}{}", mant(true), if e < 0 { "-" } else { "+" }, e.abs()),
        Short::Bare => format!("{}e{e}", mant(false)),
        Short::Fixed => {
            if e < 0 {
                format!("0.{}{d}", "0".repeat((-e - 1) as usize))
            } else if n <= e + 1 {
                format!("{d}{}.0", "0".repeat((e + 1 - n) as usize))
            } else {
                format!("{}.{}", &d[..(e + 1) as usize], &d[(e + 1) as usize..])
            }
        }
    };
    format!("{sign}{body}")
}

/// Digits and exponent of a value, from its Python repr.
fn digits_of(py: &str) -> (String, i32) {
    let s = py.trim_start_matches('-');
    let (mant, exp) = match s.split_once('e') {
        Some((m, e)) => (m, e.parse::<i32>().unwrap()),
        None => (s, 0),
    };
    let ilen = mant.split('.').next().unwrap().len() as i32;
    let all: String = mant.chars().filter(char::is_ascii_digit).collect();
    let d1 = all.trim_start_matches('0');
    let lz = (all.len() - d1.len()) as i32;
    (d1.trim_end_matches('0').to_string(), ilen - 1 - lz + exp)
}

/// Run `'__iir_write_f64'(F, Js)` on the model; returns what `io:put_chars`
/// printed. `short` is the model's `float_to_list(F, [short])`.
fn run_helper(m: &BEAMModule, f: f64, js: i64, short: &dyn Fn(f64) -> String) -> String {
    let code: &[BEAMInstruction] = &m.instructions;
    let labels: HashMap<u64, usize> =
        code.iter().enumerate().filter(|(_, i)| i.opcode == OP_LABEL).map(|(pc, i)| (i.operands[0].value, pc)).collect();
    let fi = code
        .iter()
        .position(|i| i.opcode == OP_FUNC_INFO && atom(m, i.operands[1].value) == "__iir_write_f64")
        .expect("the helper's func_info");
    assert_eq!(code[fi].operands[2].value, 2, "arity 2");
    let mut pc = fi + 2;
    let mut x: HashMap<u64, Val> = HashMap::from([(0, Val::Float(f)), (1, Val::Int(js))]);
    let mut y: Vec<Val> = Vec::new();
    let mut out = String::new();
    let lit = |idx: u64| decode_literal(&m.literals[idx as usize]);
    for _ in 0..100_000 {
        let ins = &code[pc];
        let ops = &ins.operands;
        pc += 1;
        // Operand reader: literal operands are two words (z:4, u:index).
        let read = |x: &HashMap<u64, Val>, y: &[Val], at: usize| -> (Val, usize) {
            let op = &ops[at];
            let val = match op.tag {
                BEAMTag::X => x.get(&op.value).cloned().unwrap_or(Val::Garbage),
                BEAMTag::Y => y.get(op.value as usize).cloned().unwrap_or_else(|| panic!("y{} outside the frame", op.value)),
                BEAMTag::I => Val::Int(op.value as i64),
                BEAMTag::A if op.value == 0 => Val::Nil,
                BEAMTag::A => Val::Atom(atom(m, op.value).to_string()),
                BEAMTag::Z => {
                    assert_eq!(op.value, 4, "only literal extended operands");
                    return (lit(ops[at + 1].value), at + 2);
                }
                other => panic!("operand tag {other:?} not modelled"),
            };
            assert_ne!(val, Val::Garbage, "read of a clobbered/uninitialised register at pc {}: {op:?}", pc - 1);
            (val, at + 1)
        };
        let write = |x: &mut HashMap<u64, Val>, y: &mut Vec<Val>, op: &BEAMOperand, val: Val| match op.tag {
            BEAMTag::X => {
                x.insert(op.value, val);
            }
            BEAMTag::Y => y[op.value as usize] = val,
            other => panic!("write to {other:?}"),
        };
        let int = |v: Val| match v {
            Val::Int(n) => n,
            other => panic!("expected an integer, got {other:?}"),
        };
        match ins.opcode {
            OP_LABEL => {}
            OP_ALLOCATE => y = vec![Val::Garbage; ops[0].value as usize],
            OP_DEALLOCATE => {
                assert_eq!(y.len() as u64, ops[0].value, "deallocate matches allocate");
                y.clear();
            }
            OP_MOVE => {
                let (val, at) = read(&x, &y, 0);
                write(&mut x, &mut y, &ops[at], val);
            }
            OP_GET_LIST => {
                let (val, _) = read(&x, &y, 0);
                let Val::Cons(h, t) = val else { panic!("get_list of a non-cons {val:?}") };
                write(&mut x, &mut y, &ops[1], *h);
                write(&mut x, &mut y, &ops[2], *t);
            }
            OP_JUMP => pc = labels[&ops[0].value],
            OP_IS_EQ_EXACT | OP_IS_LT | OP_IS_GE => {
                let (l, at) = read(&x, &y, 1);
                let (r, _) = read(&x, &y, at);
                let holds = match ins.opcode {
                    OP_IS_EQ_EXACT => l == r,
                    OP_IS_LT => int(l) < int(r),
                    _ => int(l) >= int(r),
                };
                if !holds {
                    pc = labels[&ops[0].value];
                }
            }
            OP_IS_NIL => {
                let (val, _) = read(&x, &y, 1);
                if val != Val::Nil {
                    pc = labels[&ops[0].value];
                }
            }
            OP_GC_BIF1 | OP_GC_BIF2 => {
                let live = ops[1].value;
                let name = import(m, ops[2].value);
                let (a, at) = read(&x, &y, 3);
                let (result, dst) = if ins.opcode == OP_GC_BIF1 {
                    assert_eq!(name, "erlang:length/1");
                    (Val::Int(to_vec(&a).len() as i64), &ops[at])
                } else {
                    let (b, at) = read(&x, &y, at);
                    let r = match name.as_str() {
                        "erlang:-/2" => int(a) - int(b),
                        "erlang:+/2" => int(a) + int(b),
                        other => panic!("bif {other} not modelled"),
                    };
                    (Val::Int(r), &ops[at])
                };
                // A GC may run: every X register at or above Live is lost.
                for (reg, val) in x.iter_mut() {
                    if *reg >= live {
                        *val = Val::Garbage;
                    }
                }
                write(&mut x, &mut y, dst, result);
            }
            OP_CALL_EXT => {
                let arity = ops[0].value;
                let args: Vec<Val> = (0..arity).map(|i| x.get(&i).cloned().unwrap_or(Val::Garbage)).collect();
                assert!(args.iter().all(|a| *a != Val::Garbage), "call_ext with a clobbered argument: {args:?}");
                let s = |v: &Val| to_string(v);
                let result = match import(m, ops[1].value).as_str() {
                    "erlang:float_to_list/2" => {
                        assert_eq!(args[1], list([Val::Atom("short".into())]));
                        let Val::Float(f) = args[0] else { panic!() };
                        chars(&short(f))
                    }
                    "string:split/2" => {
                        let (hay, pat) = (s(&args[0]), s(&args[1]));
                        match hay.split_once(pat.as_str()) {
                            Some((a, b)) => list([chars(a), chars(b)]),
                            None => list([chars(&hay)]),
                        }
                    }
                    "erlang:list_to_integer/1" => Val::Int(s(&args[0]).trim_start_matches('+').parse().unwrap()),
                    "erlang:++/2" => chars(&(s(&args[0]) + &s(&args[1]))),
                    "string:trim/3" => {
                        assert_eq!(s(&args[2]), "0");
                        let text = s(&args[0]);
                        chars(match &args[1] {
                            Val::Atom(dir) if dir == "leading" => text.trim_start_matches('0'),
                            Val::Atom(dir) if dir == "trailing" => text.trim_end_matches('0'),
                            other => panic!("trim direction {other:?}"),
                        })
                    }
                    "lists:duplicate/2" => {
                        let n = usize::try_from(int(args[0].clone())).expect("duplicate count >= 0");
                        list(std::iter::repeat_n(args[1].clone(), n))
                    }
                    "lists:sublist/2" => {
                        let n = usize::try_from(int(args[1].clone())).unwrap();
                        list(to_vec(&args[0]).into_iter().take(n))
                    }
                    "lists:nthtail/2" => {
                        let n = usize::try_from(int(args[0].clone())).unwrap();
                        let items = to_vec(&args[1]);
                        assert!(n <= items.len(), "nthtail past the end raises");
                        list(items.into_iter().skip(n))
                    }
                    "erlang:integer_to_list/1" => chars(&int(args[0].clone()).to_string()),
                    "io:put_chars/1" => {
                        out.push_str(&s(&args[0]));
                        Val::Atom("ok".into())
                    }
                    other => panic!("call_ext {other} not modelled"),
                };
                x.clear();
                x.insert(0, result);
            }
            OP_RETURN => {
                assert!(y.is_empty(), "return with a live frame");
                return out;
            }
            OP_CALL => panic!("the helper makes no local calls"),
            other => panic!("opcode {other} not modelled (pc {})", pc - 1),
        }
    }
    panic!("the helper did not terminate");
}

// (bits, python3 repr, node String) — generated by host python3 3.13.16 and node v22.22.0.
const FLOAT_TABLE: &[(u64, &str, &str)] = &[
    (0x0000000000000000, "0.0", "0"),
    (0x8000000000000000, "-0.0", "0"),
    (0x7FF0000000000000, "inf", "Infinity"),
    (0xFFF0000000000000, "-inf", "-Infinity"),
    (0x7FF8000000000000, "nan", "NaN"),
    (0x3FF0000000000000, "1.0", "1"),
    (0x4059000000000000, "100.0", "100"),
    (0x3FB999999999999A, "0.1", "0.1"),
    (0x430C6BF526340000, "1000000000000000.0", "1000000000000000"),
    (0x4341C37937E08000, "1e+16", "10000000000000000"),
    (0x4415AF1D78B58C40, "1e+20", "100000000000000000000"),
    (0x444B1AE4D6E2EF50, "1e+21", "1e+21"),
    (0x3F1A36E2EB1C432D, "0.0001", "0.0001"),
    (0x3EE4F8B588E368F1, "1e-05", "0.00001"),
    (0x3EB0C6F7A0B5ED8D, "1e-06", "0.000001"),
    (0x3E7AD7F29ABCAF48, "1e-07", "1e-7"),
    (0x3E8421F5F40D8376, "1.5e-07", "1.5e-7"),
    (0x405EDD2F1A9FBE77, "123.456", "123.456"),
    (0xC004000000000000, "-2.5", "-2.5"),
    (0x0000000000000001, "5e-324", "5e-324"),
    (0x0010000000000000, "2.2250738585072014e-308", "2.2250738585072014e-308"),
    (0x7FEFFFFFFFFFFFFF, "1.7976931348623157e+308", "1.7976931348623157e+308"),
    (0x3E60000000000000, "2.9802322387695312e-08", "2.9802322387695312e-8"),
    (0x4340000000000001, "9007199254740994.0", "9007199254740994"),
    (0x44C52D02C7E14AF6, "2e+23", "2e+23"),
    (0x3FD3333333333334, "0.30000000000000004", "0.30000000000000004"),
    (0x430E77EA4376F602, "1072012167405248.2", "1072012167405248.2"),
    (0x437B69B4BA630F35, "1.2345678901234568e+17", "123456789012345680"),
    (0x81A56E1FC2F8F359, "-1e-300", "-1e-300"),
    (0x4011666666666666, "4.35", "4.35"),
    (0xD802DB897F041728, "-9.287824536391492e+115", "-9.287824536391492e+115"),
    (0x75BD5125DB54E35F, "1.4086255999159193e+259", "1.4086255999159193e+259"),
    (0x20CBF68A16B4AA27, "1.0678165209952348e-150", "1.0678165209952348e-150"),
    (0x08CE66D960E04456, "2.9464053386017267e-266", "2.9464053386017267e-266"),
    (0xF3C0A0E50C95D61B, "-3.7204833670429256e+249", "-3.7204833670429256e+249"),
    (0x36539DC5A658D453, "5.368806067798381e-47", "5.368806067798381e-47"),
    (0x2C80A15F306E7B43, "2.491454630318929e-94", "2.491454630318929e-94"),
    (0x152D01E64F17B9BA, "1.1293852148835315e-206", "1.1293852148835315e-206"),
    (0xFA231E76D1930214, "-2.1690580031879166e+280", "-2.1690580031879166e+280"),
    (0x691ED3920DFB649F, "2.3043086018535117e+198", "2.3043086018535117e+198"),
    (0x5A54D551274E53BE, "1.4102484161097585e+127", "1.4102484161097585e+127"),
    (0x8F8D17F44D22AC04, "-9.15019639745717e-234", "-9.15019639745717e-234"),
    (0x33C1B3F1CFC3ED46, "2.2033094014034217e-59", "2.2033094014034217e-59"),
    (0xC93BF9A679B68A27, "-6.238677282146316e+44", "-6.238677282146316e+44"),
    (0xB50A7691A04B8761, "-3.4536034687305475e-53", "-3.4536034687305475e-53"),
    (0xED9EDE68E6A24F73, "-1.0896736081856203e+220", "-1.0896736081856203e+220"),
    (0xD24149293AD63D44, "-1.7193245445503562e+88", "-1.7193245445503562e+88"),
    (0xF7242ACA7222B702, "-8.128506383860937e+265", "-8.128506383860937e+265"),
    (0x0B5BBCCE2AB0CC72, "5.911413105048698e-254", "5.911413105048698e-254"),
    (0x07CCF96BC815D3AE, "4.2847637478370544e-271", "4.2847637478370544e-271"),
    (0xF7AC0685051557DA, "-2.8917384894280522e+268", "-2.8917384894280522e+268"),
    (0x777125B3C1C4578B, "2.2116238141801078e+267", "2.2116238141801078e+267"),
    (0x62F46C925963D9C7, "4.817488535775391e+168", "4.817488535775391e+168"),
    (0xB93B967192FADFF5, "-5.313192071926005e-33", "-5.313192071926005e-33"),
    (0x8CE38FEE2FB932C4, "-1.3989167825577705e-246", "-1.3989167825577705e-246"),
    (0xDF92355AEE87C2E6, "-2.3841306779525437e+152", "-2.3841306779525437e+152"),
    (0x7A2CEC265132D40E, "3.2812642341384877e+280", "3.2812642341384877e+280"),
    (0xCBE782C1746553A4, "-4.6118581223499435e+57", "-4.6118581223499435e+57"),
    (0xDEF0DF69F03C6967, "-2.1574598139070555e+149", "-2.1574598139070555e+149"),
    (0xD8F1B39298A733C6, "-2.856848852987085e+120", "-2.856848852987085e+120"),
    (0x70D0CDBC524A32C7, "2.67140895794731e+235", "2.67140895794731e+235"),
    (0x22D05E9A3AE8FD30, "5.369557981821192e-141", "5.369557981821192e-141"),
    (0xC0826248E92243B8, "-588.2856009175221", "-588.2856009175221"),
    (0x684BDBB4B7B2484B, "2.542030062404305e+194", "2.542030062404305e+194"),
    (0x82AA9609A0ED9BF6, "-8.1303389302721e-296", "-8.1303389302721e-296"),
    (0x2F977CC78DF3596C, "1.9808722431475594e-79", "1.9808722431475594e-79"),
    (0x91B2D5E5EBF7C6BC, "-2.03545399911944e-223", "-2.03545399911944e-223"),
    (0x033F737120A4CDF7, "4.924452245226261e-293", "4.924452245226261e-293"),
    (0x713A83505E7F5BD2, "2.6975814893955876e+237", "2.6975814893955876e+237"),
    (0x304A2209D00A3D4E, "4.5137733331146937e-76", "4.5137733331146937e-76"),
    (0x40B6EACCCCCCCCCD, "5866.8", "5866.8"),
    (0xC072B4EB1C432CA5, "-299.3074", "-299.3074"),
    (0xC0C217C666666666, "-9263.55", "-9263.55"),
    (0x40A970BE157D3FC7, "3256.3712577", "3256.3712577"),
    (0x40C22F2D7F62B6AE, "9310.35545", "9310.35545"),
    (0xC0AC380000000000, "-3612.0", "-3612"),
    (0xC0B1C209BA5E353F, "-4546.038", "-4546.038"),
    (0x40B1F47F2495E17E, "4596.496652", "4596.496652"),
    (0x40C009BA1A8CDEA0, "8211.45393525", "8211.45393525"),
    (0x40AC7211EB851EB8, "3641.035", "3641.035"),
    (0x4093D172474538EF, "1268.3616", "1268.3616"),
    (0x40A9740000000000, "3258.0", "3258"),
    (0x40BAE5A5810624DD, "6885.6465", "6885.6465"),
    (0x40C3265CF12C27A6, "9804.72611", "9804.72611"),
    (0x40B775A786C22681, "6005.6544", "6005.6544"),
    (0xC0911F3333333333, "-1095.8", "-1095.8"),
    (0x40B3F76083126E98, "5111.377", "5111.377"),
    (0xC0BF710000000000, "-8049.0", "-8049"),
    (0x409BAD95810624DD, "1771.396", "1771.396"),
    (0x40B99E157689CA19, "6558.08384", "6558.08384"),
];

#[test]
fn helper_lays_out_every_short_style_like_python_and_javascript() {
    let m = writer("write_f64_py");
    let by_bits: HashMap<u64, &str> = FLOAT_TABLE.iter().map(|(b, py, _)| (*b, *py)).collect();
    for style in [Short::Sci, Short::SciPlus, Short::Bare, Short::Fixed] {
        let short = |f: f64| {
            let (d, e) = if f == 0.0 { (String::new(), 0) } else { digits_of(by_bits[&f.to_bits()]) };
            short_string(&d, e, f.is_sign_negative(), style)
        };
        // BEAM has no inf or nan floats: those rows cannot reach the helper.
        for &(bits, py, js) in FLOAT_TABLE.iter().filter(|(b, _, _)| f64::from_bits(*b).is_finite()) {
            let f = f64::from_bits(bits);
            assert_eq!(run_helper(&m, f, 0, &short), py, "{style:?} python {f:e}");
            assert_eq!(run_helper(&m, f, 1, &short), js, "{style:?} javascript {f:e}");
        }
    }
}

#[test]
fn call_site_passes_the_float_and_layout_then_calls_the_local_helper() {
    for (builtin, layout) in [("write_f64_py", 0), ("write_f64_js", 1)] {
        let m = writer(builtin);
        let entry = m
            .instructions
            .iter()
            .position(|i| i.opcode == OP_FUNC_INFO && atom(&m, i.operands[1].value) == "__iir_write_f64")
            .map(|fi| m.instructions[fi + 1].operands[0].value)
            .unwrap();
        let call = m.instructions.iter().position(|i| i.opcode == OP_CALL && i.operands[1].value == entry).expect("local call");
        assert_eq!(m.instructions[call].operands[0].value, 2, "arity 2");
        let set_layout = &m.instructions[call - 1];
        assert_eq!((set_layout.opcode, set_layout.operands[0].value, set_layout.operands[1].value), (OP_MOVE, layout, 1));
        // Exported, so the Code chunk's function count (= exports) covers it.
        let helper = m.exports.iter().find(|e| atom(&m, u64::from(e.function_atom_index)) == "__iir_write_f64");
        assert!(matches!(helper, Some(e) if e.arity == 2 && u64::from(e.label) == entry));
        assert_eq!(m.exports.len(), m.instructions.iter().filter(|i| i.opcode == OP_FUNC_INFO).count());
        assert_eq!(m.instructions.iter().filter(|i| i.opcode == OP_FUNC_INFO).count(), 2, "main + one helper");
    }
}

#[test]
fn a_module_without_float_output_is_unchanged() {
    let module = IIRModule {
        name: "plain".into(),
        functions: vec![IIRFunction::new("main", vec![], "i64", vec![
            IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new("ret", None, vec![v("z")], "i64"),
        ])],
        entry_point: Some("main".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
        release: None,
    };
    let m = lower_iir_to_beam(&module, &IIRBeamConfig::new("plain")).unwrap();
    assert!(!m.atoms.iter().any(|a| a == "__iir_write_f64" || a == "float_to_list" || a == "nthtail"));
    assert!(m.literals.is_empty());
}

#[test]
fn a_function_named_like_the_helper_is_refused() {
    let mut module = IIRModule {
        name: "clash".into(),
        functions: vec![IIRFunction::new("main", vec![("x".into(), "f64".into())], "i64", vec![
            IIRInstr::new("call_builtin", None, vec![v("write_f64_js"), v("x")], "void"),
            IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new("ret", None, vec![v("z")], "i64"),
        ])],
        entry_point: Some("main".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
        release: None,
    };
    module.functions.push(IIRFunction::new("__iir_write_f64", vec![], "i64", vec![
        IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "i64"),
        IIRInstr::new("ret", None, vec![v("z")], "i64"),
    ]));
    assert!(lower_iir_to_beam(&module, &IIRBeamConfig::new("clash")).is_err());
}
