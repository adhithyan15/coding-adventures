//! LANG83 PR6a on the JVM backend: `write_f64_py` / `write_f64_js`.
//!
//! Both lower to `dload x; iconst_<layout>; invokestatic __iir_write_f64(DI)V`,
//! the class's own synthetic method, which takes the shortest digits from
//! `Double.toString` (JDK 19+) and lays them out like Python or JavaScript.
//! The structural tests pin that shape; when `java` is installed, the
//! execution tests run the class on a real JVM with the verifier on and
//! compare against strings printed by host `python3` and `node`.

use std::process::Command;

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_jvm_class_file::{lower_iir_to_jvm, serialize_jvm_class_file, IIRJvmConfig, JvmClassFile};
use jvm_class_file::{
    JvmCodeAttribute, JvmConstantPoolEntry as Cp, JvmMethodAttribute, JvmMethodInfo, ACC_PUBLIC, ACC_STATIC,
};

fn v(n: &str) -> Operand {
    Operand::Var(n.into())
}

/// `main() -> i64`: `builtin(x); print_str("\n")` for every value. (`print_str`
/// is `System.out.print`; `putchar` would need the `env.BFRuntime` host class.)
fn writes(builtin: &str, values: &[f64]) -> IIRModule {
    let mut body = vec![IIRInstr::new("str_const", Some("nl".into()), vec![Operand::Str("\n".into())], "str")];
    for (i, x) in values.iter().enumerate() {
        let name = format!("x{i}");
        body.push(IIRInstr::new("const", Some(name.clone()), vec![Operand::Float(*x)], "f64"));
        body.push(IIRInstr::new("call_builtin", None, vec![v(builtin), v(&name)], "void"));
        body.push(IIRInstr::new("print_str", None, vec![v("nl")], "void"));
    }
    body.push(IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "i64"));
    body.push(IIRInstr::new("ret", None, vec![v("z")], "i64"));
    IIRModule {
        name: "floats".into(),
        functions: vec![IIRFunction::new("main", vec![], "i64", body)],
        entry_point: Some("main".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    }
}

fn lower(m: &IIRModule) -> JvmClassFile {
    lower_iir_to_jvm(m, &IIRJvmConfig::new("Main")).expect("lowers")
}

fn utf8(cp: &[Option<Cp>], i: u16) -> String {
    match &cp[i as usize] {
        Some(Cp::Utf8(s)) => s.clone(),
        other => panic!("cp #{i} is not Utf8: {other:?}"),
    }
}

/// Every `Methodref` in the pool as `Class.name:descriptor`.
fn methodrefs(class: &JvmClassFile) -> Vec<String> {
    let cp = &class.constant_pool;
    cp.iter()
        .filter_map(|e| match e {
            Some(Cp::Methodref { class_index, name_and_type_index }) => {
                let Some(Cp::Class { name_index }) = &cp[*class_index as usize] else { return None };
                let Some(Cp::NameAndType { name_index: n, descriptor_index: d }) = &cp[*name_and_type_index as usize]
                else {
                    return None;
                };
                Some(format!("{}.{}:{}", utf8(cp, *name_index), utf8(cp, *n), utf8(cp, *d)))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn the_class_gets_one_private_static_float_writer() {
    let class = lower(&writes("write_f64_py", &[1.5, 2.5]));
    let helpers: Vec<&JvmMethodInfo> = class.methods.iter().filter(|m| m.name == "__iir_write_f64").collect();
    assert_eq!(helpers.len(), 1, "one helper however many call sites");
    assert_eq!(helpers[0].descriptor, "(DI)V");
    assert_eq!(helpers[0].access_flags, 0x0002 | ACC_STATIC, "private static");
    let refs = methodrefs(&class);
    for needed in [
        "java/lang/Double.toString:(D)Ljava/lang/String;",
        "java/math/BigDecimal.stripTrailingZeros:()Ljava/math/BigDecimal;",
        "java/math/BigDecimal.toPlainString:()Ljava/lang/String;",
        "java/io/PrintStream.print:(Ljava/lang/String;)V",
        "Main.__iir_write_f64:(DI)V",
    ] {
        assert!(refs.iter().any(|r| r == needed), "missing {needed} in {refs:?}");
    }
    // A module that prints no float carries no helper.
    let plain = IIRModule { functions: vec![IIRFunction::new("main", vec![], "i64", vec![
        IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "i64"),
        IIRInstr::new("ret", None, vec![v("z")], "i64"),
    ])], ..writes("write_f64_py", &[]) };
    assert!(lower(&plain).methods.iter().all(|m| m.name != "__iir_write_f64"));
}

#[test]
fn call_site_loads_the_double_then_the_layout_flag() {
    for (builtin, flag) in [("write_f64_py", 0x03u8), ("write_f64_js", 0x04u8)] {
        let class = lower(&writes(builtin, &[1.5]));
        let main = class.methods.iter().find(|m| m.name == "main").unwrap();
        let JvmMethodAttribute::Code(code) = &main.attributes[0] else { panic!("no Code") };
        let call = code.code.windows(2).position(|w| w == [flag, 0xB8]).expect("iconst_<layout>; invokestatic");
        assert!(call > 0, "{builtin}");
    }
}

#[test]
fn an_integer_operand_is_rejected() {
    let m = IIRModule {
        functions: vec![IIRFunction::new("main", vec![], "i64", vec![
            IIRInstr::new("const", Some("n".into()), vec![Operand::Int(1)], "i64"),
            IIRInstr::new("call_builtin", None, vec![v("write_f64_py"), v("n")], "void"),
            IIRInstr::new("ret", None, vec![v("n")], "i64"),
        ])],
        ..writes("write_f64_py", &[])
    };
    assert!(lower_iir_to_jvm(&m, &IIRJvmConfig::new("Main")).is_err());
}

#[test]
fn a_function_named_like_the_helper_is_refused() {
    let mut m = writes("write_f64_js", &[1.0]);
    m.functions.push(IIRFunction::new("__iir_write_f64", vec![], "void", vec![IIRInstr::new("ret_void", None, vec![], "void")]));
    assert!(lower_iir_to_jvm(&m, &IIRJvmConfig::new("Main")).is_err());
}

fn java_ok() -> bool {
    Command::new("java").arg("-version").output().map(|o| o.status.success()).unwrap_or(false)
}

/// Add `public static void main(String[])` that calls the IIR `main()J` and
/// drops its result, write `Main.class`, and run it with the verifier on.
fn run_on_java(mut class: JvmClassFile, tag: &str) -> Option<String> {
    if !java_ok() {
        eprintln!("java not available — skipping {tag}");
        return None;
    }
    let cp = &mut class.constant_pool;
    let mut add = |e: Cp| {
        cp.push(Some(e));
        (cp.len() - 1) as u16
    };
    let class_utf8 = add(Cp::Utf8("Main".into()));
    let class_ref = add(Cp::Class { name_index: class_utf8 });
    let name = add(Cp::Utf8("main".into()));
    let desc = add(Cp::Utf8("()J".into()));
    let nat = add(Cp::NameAndType { name_index: name, descriptor_index: desc });
    let entry = add(Cp::Methodref { class_index: class_ref, name_and_type_index: nat });
    let _ = add(Cp::Utf8("([Ljava/lang/String;)V".into()));
    let [hi, lo] = entry.to_be_bytes();
    class.methods.push(JvmMethodInfo {
        access_flags: ACC_PUBLIC | ACC_STATIC,
        name: "main".into(),
        descriptor: "([Ljava/lang/String;)V".into(),
        attributes: vec![JvmMethodAttribute::Code(JvmCodeAttribute {
            name: "Code".into(),
            max_stack: 2,
            max_locals: 1,
            code: vec![0xB8, hi, lo, 0x58, 0xB1], // invokestatic main()J; pop2; return
            nested_attributes: vec![],
        })],
    });
    let dir = std::env::temp_dir().join(format!("lang83_pr6a_jvm_{tag}_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("mkdir");
    std::fs::write(dir.join("Main.class"), serialize_jvm_class_file(&class)).expect("write class");
    let out = Command::new("java").env_remove("JAVA_TOOL_OPTIONS").arg("-cp").arg(&dir).arg("Main").output().expect("java");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(out.status.success(), "{tag}: java failed: {}", String::from_utf8_lossy(&out.stderr));
    Some(String::from_utf8(out.stdout).expect("ASCII"))
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
fn python_layout_on_a_real_jvm_matches_host_python3() {
    let values: Vec<f64> = FLOAT_TABLE.iter().map(|(b, _, _)| f64::from_bits(*b)).collect();
    let Some(out) = run_on_java(lower(&writes("write_f64_py", &values)), "py") else { return };
    let want: String = FLOAT_TABLE.iter().map(|(_, py, _)| format!("{py}\n")).collect();
    assert_eq!(out, want);
}

#[test]
fn javascript_layout_on_a_real_jvm_matches_host_node() {
    let values: Vec<f64> = FLOAT_TABLE.iter().map(|(b, _, _)| f64::from_bits(*b)).collect();
    let Some(out) = run_on_java(lower(&writes("write_f64_js", &values)), "js") else { return };
    let want: String = FLOAT_TABLE.iter().map(|(_, _, js)| format!("{js}\n")).collect();
    assert_eq!(out, want);
}
