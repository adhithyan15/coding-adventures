//! LANG83 PR6a on the CLR textual (`emit_il`) path: `write_f64_py` /
//! `write_f64_js`.
//!
//! Both lower to `call void <Asm>Program::__iir_write_f64(float64, int32)`, a
//! helper the module carries (see `il_text.rs`). It takes the shortest digits
//! from `double.ToString("R", InvariantCulture)` and lays them out like Python
//! or JavaScript.
//!
//! There is no `dotnet` here, so the helper is checked two ways:
//!
//! 1. A **stack-depth model** walks every control-flow path of its IL. Each
//!    label is reached with an empty stack, no instruction underflows, the
//!    peak fits `.maxstack`, and `ret` leaves nothing behind. CoreCLR rejects
//!    an inconsistent stack shape at a join even when `ilasm` accepts it
//!    (Perl 4a).
//! 2. An **IL interpreter model** executes the helper. It implements the dozen
//!    opcodes and the `System.*` members the helper uses. Its
//!    `double.ToString("R")` is fed the same digits in several layouts: .NET
//!    Core's own (`1E+16`, `1E-05`, `123.456`), all-scientific and all-fixed.
//!    For every value in the table below the output must be Python's `repr`
//!    and JavaScript's `String(x)`, as printed by host `python3` and `node`.
//!
//! CI's CLR column runs the real thing once a language row emits these
//! builtins.

use std::collections::HashMap;

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_cil_bytecode::{emit_il, lower_iir_to_cil, IIRClrConfig};

fn v(n: &str) -> Operand {
    Operand::Var(n.into())
}

/// `main() -> i64`: `builtin(x)` for every value.
fn writes(builtin: &str, values: &[f64]) -> IIRModule {
    let mut body = Vec::new();
    for (i, x) in values.iter().enumerate() {
        let name = format!("x{i}");
        body.push(IIRInstr::new("const", Some(name.clone()), vec![Operand::Float(*x)], "f64"));
        body.push(IIRInstr::new("call_builtin", None, vec![v(builtin), v(&name)], "void"));
    }
    body.push(IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "i64"));
    body.push(IIRInstr::new("ret", None, vec![v("z")], "i64"));
    let mut m = IIRModule::new("Main", "test");
    m.functions.push(IIRFunction::new("main", vec![], "i64", body));
    m.entry_point = Some("main".into());
    m
}

/// The helper's trimmed IL lines, from `.method` to its closing `}`.
fn helper_lines(il: &str) -> Vec<String> {
    let start = il.find(".method private static void __iir_write_f64").expect("the helper is emitted");
    let mut lines = Vec::new();
    for l in il[start..].lines() {
        let l = l.trim().to_string();
        let end = l == "}";
        lines.push(l);
        if end {
            break;
        }
    }
    lines
}

/// The instruction lines (no `.method`/`.maxstack`/`.locals`/braces).
fn body(lines: &[String]) -> Vec<String> {
    lines.iter().filter(|l| !l.starts_with('.') && *l != "{" && *l != "}").cloned().collect()
}

/// `(pops, pushes)` of a `call`/`callvirt`/`newobj` line, from its signature.
fn call_effect(line: &str) -> (usize, usize) {
    let op = line.split_whitespace().next().unwrap();
    let args = line.rsplit_once('(').unwrap().1.trim_end_matches(')');
    let n_args = if args.trim().is_empty() { 0 } else { args.split(',').count() };
    let instance = line.contains(" instance ");
    match op {
        "newobj" => (n_args, 1),
        _ => {
            let ret = line.split_whitespace().nth(if instance { 2 } else { 1 }).unwrap();
            (n_args + usize::from(instance), usize::from(ret != "void"))
        }
    }
}

const BRANCHES: &[&str] = &["beq", "bne.un", "blt", "bge", "bgt", "ble", "brfalse", "brtrue", "br"];

/// `(pops, pushes)` of one instruction line; panics on anything unknown so a
/// new instruction cannot slip past the model.
fn effect(line: &str) -> (usize, usize) {
    let op = line.split_whitespace().next().unwrap();
    match op {
        "ldarg" | "ldloc" | "ldloca" | "ldstr" | "ldc.i4" | "ldc.i4.0" | "ldc.i4.1" | "ldc.i4.m1" | "ldc.r8" => (0, 1),
        "stloc" => (1, 0),
        "add" | "sub" => (2, 1),
        "beq" | "bne.un" | "blt" | "bge" | "bgt" | "ble" => (2, 0),
        "brfalse" | "brtrue" => (1, 0),
        "br" | "ret" => (0, 0),
        "call" | "callvirt" | "newobj" => call_effect(line),
        _ => panic!("stack model does not know {line:?}"),
    }
}

/// Walk every path; returns the peak depth.
fn assert_stack_shape(code: &[String]) -> usize {
    let labels: HashMap<&str, usize> = code
        .iter()
        .enumerate()
        .filter_map(|(i, l)| l.strip_suffix(':').map(|name| (name, i)))
        .collect();
    let mut depth_at: Vec<Option<usize>> = vec![None; code.len()];
    let mut work = vec![(0usize, 0usize)];
    let mut peak = 0;
    while let Some((pc, depth)) = work.pop() {
        assert!(pc < code.len(), "control falls off the end of the helper");
        match depth_at[pc] {
            Some(d) => {
                assert_eq!(d, depth, "inconsistent depth at {:?}", code[pc]);
                continue;
            }
            None => depth_at[pc] = Some(depth),
        }
        let line = &code[pc];
        if line.ends_with(':') {
            assert_eq!(depth, 0, "label {line} reached with a non-empty stack");
            work.push((pc + 1, depth));
            continue;
        }
        let (pops, pushes) = effect(line);
        assert!(depth >= pops, "stack underflow at {line:?}");
        let after = depth - pops + pushes;
        peak = peak.max(after);
        let op = line.split_whitespace().next().unwrap();
        if op == "ret" {
            assert_eq!(after, 0, "a void method returns with an empty stack");
            continue;
        }
        if BRANCHES.contains(&op) {
            assert_eq!(after, 0, "branch {line:?} must leave the stack empty");
            let target = line.split_whitespace().nth(1).unwrap();
            let at = *labels.get(target).unwrap_or_else(|| panic!("undefined label {target}"));
            work.push((at, after));
            if op == "br" {
                continue;
            }
        }
        work.push((pc + 1, after));
    }
    for (i, l) in code.iter().enumerate() {
        assert!(depth_at[i].is_some() || l.ends_with(':'), "unreachable instruction {l:?}");
    }
    peak
}

#[test]
fn helper_keeps_an_empty_stack_at_every_join_and_fits_maxstack() {
    let il = emit_il(&writes("write_f64_py", &[1.5]), &IIRClrConfig::new("Main")).expect("emit_il");
    let lines = helper_lines(&il);
    let declared: usize = lines
        .iter()
        .find_map(|l| l.strip_prefix(".maxstack "))
        .and_then(|n| n.trim().parse().ok())
        .expect(".maxstack");
    let peak = assert_stack_shape(&body(&lines));
    assert!(peak <= declared, "peak {peak} exceeds .maxstack {declared}");
    // Labels are unique, so the helper cannot capture a user label.
    let code = body(&lines);
    let mut labels: Vec<&String> = code.iter().filter(|l| l.ends_with(':')).collect();
    let total = labels.len();
    labels.dedup();
    assert_eq!(labels.len(), total);
}

#[test]
fn call_sites_pass_the_float_and_a_layout_flag_and_the_helper_is_emitted_once() {
    for (builtin, flag) in [("write_f64_py", "ldc.i4.0"), ("write_f64_js", "ldc.i4.1")] {
        let il = emit_il(&writes(builtin, &[1.5, -2.0]), &IIRClrConfig::new("Main")).expect("emit_il");
        let call = "call void MainProgram::__iir_write_f64(float64, int32)";
        assert_eq!(il.matches(call).count(), 2, "{builtin}: one call per write");
        assert_eq!(il.matches(".method private static void __iir_write_f64").count(), 1);
        let lines: Vec<&str> = il.lines().map(str::trim).collect();
        let at = lines.iter().position(|l| *l == call).unwrap();
        assert_eq!(lines[at - 1], flag, "{builtin}");
    }
    // No float printed, no helper.
    let none = writes("write_f64_py", &[]);
    assert!(!emit_il(&none, &IIRClrConfig::new("Main")).unwrap().contains("__iir_write_f64"));
}

#[test]
fn bad_operands_and_the_binary_path_are_refused() {
    let mut m = writes("write_f64_py", &[]);
    m.functions[0].instructions.insert(0, IIRInstr::new("const", Some("n".into()), vec![Operand::Int(3)], "i64"));
    m.functions[0].instructions.insert(1, IIRInstr::new("call_builtin", None, vec![v("write_f64_py"), v("n")], "void"));
    assert!(emit_il(&m, &IIRClrConfig::new("Main")).is_err(), "an integer is not a float");
    let mut clash = writes("write_f64_js", &[1.0]);
    clash.functions.push(IIRFunction::new("__iir_write_f64", vec![], "void", vec![IIRInstr::new("ret_void", None, vec![], "void")]));
    assert!(emit_il(&clash, &IIRClrConfig::new("Main")).is_err(), "the helper's name is reserved");
    // The binary path targets the 32-bit simulator and does not lower PR6a.
    assert!(lower_iir_to_cil(&writes("write_f64_py", &[1.5]), &IIRClrConfig::new("Main")).is_err());
}

// ── The IL interpreter model ────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Val {
    I(i64),
    F(f64),
    S(String),
    /// `ldloca` of a float64 local.
    Addr(String),
    Culture,
}

/// How the model's `double.ToString("R")` lays out the shortest digits.
#[derive(Clone, Copy, Debug)]
enum RStyle {
    /// .NET Core 3.0+: scientific when e >= 15 or e < -5, `E+XX` / `E-XX`
    /// with at least two exponent digits (`1E+16`, `1.5E-07`, `123.456`).
    DotNet,
    /// Always `d.dddE+X`.
    Scientific,
    /// Always plain positional digits (`10000000000000000`, `0.0000001`).
    Fixed,
}

/// Shortest digits `D` and exponent `e` of a value, read off its Python repr.
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

fn r_string(d: &str, e: i32, style: RStyle) -> String {
    let n = d.len() as i32;
    let sci = |pad: usize| {
        let mut s = d[..1].to_string();
        if n > 1 {
            s.push('.');
            s.push_str(&d[1..]);
        }
        let mag = e.unsigned_abs().to_string();
        format!("{s}E{}{}{mag}", if e < 0 { '-' } else { '+' }, "0".repeat(pad.saturating_sub(mag.len())))
    };
    let fixed = || {
        if e < 0 {
            format!("0.{}{d}", "0".repeat((-e - 1) as usize))
        } else if n <= e + 1 {
            format!("{d}{}", "0".repeat((e + 1 - n) as usize))
        } else {
            format!("{}.{}", &d[..(e + 1) as usize], &d[(e + 1) as usize..])
        }
    };
    match style {
        RStyle::DotNet if !(-5..15).contains(&e) => sci(2),
        RStyle::DotNet | RStyle::Fixed => fixed(),
        RStyle::Scientific => sci(1),
    }
}

/// Run the helper's IL for `x` with layout flag `js`. `shortest` gives the
/// model's `ToString("R")` digits for |x|.
fn run_helper(code: &[String], x: f64, js: bool, r: &dyn Fn(f64) -> String) -> String {
    let labels: HashMap<&str, usize> =
        code.iter().enumerate().filter_map(|(i, l)| l.strip_suffix(':').map(|n| (n, i))).collect();
    let mut locals: HashMap<String, Val> = HashMap::new();
    let args = [Val::F(x), Val::I(i64::from(js))];
    let mut stack: Vec<Val> = Vec::new();
    let mut out = String::new();
    let mut pc = 0;
    let mut steps = 0;
    let int = |v: Val| match v {
        Val::I(n) => n,
        other => panic!("expected an int, got {other:?}"),
    };
    let string = |v: Val| match v {
        Val::S(s) => s,
        other => panic!("expected a string, got {other:?}"),
    };
    loop {
        steps += 1;
        assert!(steps < 10_000, "the helper does not terminate");
        let line = code[pc].clone();
        pc += 1;
        if line.ends_with(':') {
            continue;
        }
        let mut words = line.splitn(2, ' ');
        let op = words.next().unwrap();
        let rest = words.next().unwrap_or("");
        let jump = |cond: bool, pc: &mut usize| {
            if cond {
                *pc = labels[rest];
            }
        };
        match op {
            "ldarg" => stack.push(args[rest.parse::<usize>().unwrap()].clone()),
            "ldloc" => stack.push(locals.get(rest).cloned().unwrap_or(Val::I(0))),
            "ldloca" => stack.push(Val::Addr(rest.to_string())),
            "stloc" => {
                let v = stack.pop().unwrap();
                locals.insert(rest.to_string(), v);
            }
            "ldstr" => stack.push(Val::S(rest.trim_matches('"').to_string())),
            "ldc.i4" => stack.push(Val::I(rest.parse().unwrap())),
            "ldc.i4.0" => stack.push(Val::I(0)),
            "ldc.i4.1" => stack.push(Val::I(1)),
            "ldc.i4.m1" => stack.push(Val::I(-1)),
            "ldc.r8" => stack.push(Val::F(rest.parse().unwrap())),
            "add" | "sub" => {
                let b = int(stack.pop().unwrap());
                let a = int(stack.pop().unwrap());
                stack.push(Val::I(if op == "add" { a + b } else { a - b }));
            }
            "beq" | "bne.un" | "blt" | "bge" | "bgt" | "ble" => {
                let b = stack.pop().unwrap();
                let a = stack.pop().unwrap();
                let (x, y) = match (a, b) {
                    (Val::F(x), Val::F(y)) => (x, y),
                    (Val::I(x), Val::I(y)) => (x as f64, y as f64),
                    other => panic!("compare of {other:?}"),
                };
                let cond = match op {
                    "beq" => x == y,
                    "bne.un" => x != y || x.is_nan() || y.is_nan(),
                    "blt" => x < y,
                    "bge" => x >= y,
                    "bgt" => x > y,
                    _ => x <= y,
                };
                jump(cond, &mut pc);
            }
            "brfalse" | "brtrue" => {
                let v = int(stack.pop().unwrap());
                jump((v != 0) == (op == "brtrue"), &mut pc);
            }
            "br" => jump(true, &mut pc),
            "ret" => return out,
            "call" | "callvirt" | "newobj" => {
                let member = line.rsplit_once("::").unwrap().1;
                let (n_pop, _) = call_effect(&line);
                let mut argv: Vec<Val> = stack.split_off(stack.len() - n_pop);
                let result = match member {
                    "Write(string)" => {
                        out.push_str(&string(argv.remove(0)));
                        None
                    }
                    "Write(int32)" => {
                        out.push_str(&int(argv.remove(0)).to_string());
                        None
                    }
                    "IsNegative(float64)" => match argv[0] {
                        Val::F(f) => Some(Val::I(i64::from(f.is_sign_negative()))),
                        _ => panic!(),
                    },
                    "IsInfinity(float64)" => match argv[0] {
                        Val::F(f) => Some(Val::I(i64::from(f.is_infinite()))),
                        _ => panic!(),
                    },
                    "Abs(float64)" => match argv[0] {
                        Val::F(f) => Some(Val::F(f.abs())),
                        _ => panic!(),
                    },
                    "get_InvariantCulture()" => Some(Val::Culture),
                    "ToString(string, class [System.Runtime]System.IFormatProvider)" => {
                        let Val::Addr(local) = &argv[0] else { panic!("ToString needs the local's address") };
                        assert_eq!(argv[1], Val::S("R".into()));
                        assert_eq!(argv[2], Val::Culture, "always the invariant culture");
                        let Some(Val::F(f)) = locals.get(local) else { panic!("not a float local") };
                        Some(Val::S(r(*f)))
                    }
                    "IndexOf(char)" => {
                        let c = int(argv.pop().unwrap()) as u8 as char;
                        let s = string(argv.pop().unwrap());
                        Some(Val::I(s.find(c).map_or(-1, |i| i as i64)))
                    }
                    "Substring(int32)" => {
                        let i = int(argv.pop().unwrap()) as usize;
                        Some(Val::S(string(argv.pop().unwrap())[i..].to_string()))
                    }
                    "Substring(int32, int32)" => {
                        let len = int(argv.pop().unwrap()) as usize;
                        let i = int(argv.pop().unwrap()) as usize;
                        Some(Val::S(string(argv.pop().unwrap())[i..i + len].to_string()))
                    }
                    "Remove(int32, int32)" => {
                        let len = int(argv.pop().unwrap()) as usize;
                        let i = int(argv.pop().unwrap()) as usize;
                        let mut s = string(argv.pop().unwrap());
                        s.replace_range(i..i + len, "");
                        Some(Val::S(s))
                    }
                    "get_Length()" => Some(Val::I(string(argv.pop().unwrap()).len() as i64)),
                    "TrimStart(char)" | "TrimEnd(char)" => {
                        let c = int(argv.pop().unwrap()) as u8 as char;
                        let s = string(argv.pop().unwrap());
                        Some(Val::S(if member.starts_with("TrimStart") {
                            s.trim_start_matches(c).to_string()
                        } else {
                            s.trim_end_matches(c).to_string()
                        }))
                    }
                    "Parse(string, class [System.Runtime]System.IFormatProvider)" => {
                        assert_eq!(argv[1], Val::Culture);
                        let s = string(argv.remove(0));
                        Some(Val::I(s.trim_start_matches('+').parse().unwrap()))
                    }
                    ".ctor(char, int32)" => {
                        let n = int(argv.pop().unwrap());
                        let c = int(argv.pop().unwrap()) as u8 as char;
                        Some(Val::S(c.to_string().repeat(usize::try_from(n).expect("count >= 0"))))
                    }
                    other => panic!("model does not know {other}"),
                };
                stack.extend(result);
            }
            other => panic!("model does not know opcode {other}"),
        }
    }
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
fn helper_lays_out_every_r_string_style_like_python_and_javascript() {
    let il = emit_il(&writes("write_f64_py", &[1.5]), &IIRClrConfig::new("Main")).expect("emit_il");
    let code = body(&helper_lines(&il));
    // Digits for |x|, from the oracle's own Python repr (so the model's
    // `ToString("R")` is exactly as shortest as .NET's).
    let by_bits: HashMap<u64, &str> = FLOAT_TABLE.iter().map(|(b, py, _)| (b & !(1 << 63), *py)).collect();
    for style in [RStyle::DotNet, RStyle::Scientific, RStyle::Fixed] {
        let r = |ax: f64| {
            let (d, e) = digits_of(by_bits[&ax.to_bits()]);
            r_string(&d, e, style)
        };
        for &(bits, py, js) in FLOAT_TABLE {
            let x = f64::from_bits(bits);
            assert_eq!(run_helper(&code, x, false, &r), py, "{style:?} python {x:e}");
            assert_eq!(run_helper(&code, x, true, &r), js, "{style:?} javascript {x:e}");
        }
    }
    // The model's .NET style is the real one for the classic cases.
    assert_eq!(r_string("1", 16, RStyle::DotNet), "1E+16");
    assert_eq!(r_string("15", -7, RStyle::DotNet), "1.5E-07");
    assert_eq!(r_string("123456", 2, RStyle::DotNet), "123.456");
    assert_eq!(r_string("5", -324, RStyle::DotNet), "5E-324");
}
