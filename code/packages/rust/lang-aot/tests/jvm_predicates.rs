//! # JVM `ATOM`/`EQ`/`COND` on a real JVM (LANG77 / McCarthy W4, F3–F5).
//!
//! Builds on the W3b cons harness: compile a McCarthy program to a
//! `JvmClassFile`, inject a `main([Ljava/lang/String;)V` launcher that calls the
//! entry and `System.out.println`s the result, then run on a real `java`. The
//! predicates exercise the JVM lowering of the *shared* structural-pass builtins:
//! `pair?` → `instanceof Object[]`, `not` → logical not, `equal?` → unbox +
//! `if_icmpeq`, and the COND control flow (`jmp_if_false`/`is_null`).
//!
//! The launcher is **descriptor-aware**: a predicate result is `int` (`()I`), a
//! COND that selects an integer atom is `long` (`()J`) — we pick the matching
//! `println` overload so either runs.

use iir_to_jvm_class_file::serialize_jvm_class_file;
use jvm_class_file::{
    JvmCodeAttribute, JvmConstantPoolEntry, JvmMethodAttribute, JvmMethodInfo, ACC_PUBLIC,
    ACC_STATIC,
};
use lang_aot::{compile_source_to_jvm_class, Language};

fn java_available() -> bool {
    std::process::Command::new("java")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn cp_append(cp: &mut Vec<Option<JvmConstantPoolEntry>>, e: JvmConstantPoolEntry) -> u16 {
    cp.push(Some(e));
    (cp.len() - 1) as u16
}

/// Compile `source`, inject a launcher matching the entry's return type, run on
/// a real JVM, and return trimmed stdout (or `None` if `java` is absent).
fn run_on_java(source: &str, tag: &str) -> Option<String> {
    if !java_available() {
        return None;
    }
    let mut class = compile_source_to_jvm_class(Language::McCarthyLisp, source, "Main")
        .unwrap_or_else(|e| panic!("compile {source:?}: {e}"));

    // The entry's descriptor decides the launcher: `()I` → int / `println(I)V`,
    // `()J` → long / `println(J)V`.
    let entry_desc = class
        .methods
        .iter()
        .find(|m| m.name == "main")
        .expect("entry `main`")
        .descriptor
        .clone();
    let is_long = entry_desc == "()J";
    let (entry_d, pln_d) = if is_long { ("()J", "(J)V") } else { ("()I", "(I)V") };

    let (out_fieldref, println_ref, entry_ref) = {
        let cp = &mut class.constant_pool;
        let su = cp_append(cp, JvmConstantPoolEntry::Utf8("java/lang/System".into()));
        let sc = cp_append(cp, JvmConstantPoolEntry::Class { name_index: su });
        let ou = cp_append(cp, JvmConstantPoolEntry::Utf8("out".into()));
        let pd = cp_append(cp, JvmConstantPoolEntry::Utf8("Ljava/io/PrintStream;".into()));
        let on = cp_append(cp, JvmConstantPoolEntry::NameAndType { name_index: ou, descriptor_index: pd });
        let of = cp_append(cp, JvmConstantPoolEntry::Fieldref { class_index: sc, name_and_type_index: on });
        let pu = cp_append(cp, JvmConstantPoolEntry::Utf8("java/io/PrintStream".into()));
        let pc = cp_append(cp, JvmConstantPoolEntry::Class { name_index: pu });
        let lu = cp_append(cp, JvmConstantPoolEntry::Utf8("println".into()));
        let ld = cp_append(cp, JvmConstantPoolEntry::Utf8(pln_d.into()));
        let ln = cp_append(cp, JvmConstantPoolEntry::NameAndType { name_index: lu, descriptor_index: ld });
        let pr = cp_append(cp, JvmConstantPoolEntry::Methodref { class_index: pc, name_and_type_index: ln });
        let mu = cp_append(cp, JvmConstantPoolEntry::Utf8("Main".into()));
        let mc = cp_append(cp, JvmConstantPoolEntry::Class { name_index: mu });
        let nu = cp_append(cp, JvmConstantPoolEntry::Utf8("main".into()));
        let du = cp_append(cp, JvmConstantPoolEntry::Utf8(entry_d.into()));
        let nn = cp_append(cp, JvmConstantPoolEntry::NameAndType { name_index: nu, descriptor_index: du });
        let er = cp_append(cp, JvmConstantPoolEntry::Methodref { class_index: mc, name_and_type_index: nn });
        let _ = cp_append(cp, JvmConstantPoolEntry::Utf8("([Ljava/lang/String;)V".into()));
        (of, pr, er)
    };

    let [oh, ol] = out_fieldref.to_be_bytes();
    let [eh, el] = entry_ref.to_be_bytes();
    let [ph, pl] = println_ref.to_be_bytes();
    let main_code = vec![
        0xB2, oh, ol, // getstatic System.out
        0xB8, eh, el, // invokestatic Main.main()I|J
        0xB6, ph, pl, // invokevirtual println(I|J)V
        0xB1,         // return
    ];
    // long takes two stack slots, so PrintStream + long needs 3.
    let max_stack = if is_long { 3 } else { 2 };
    class.methods.push(JvmMethodInfo {
        access_flags: ACC_PUBLIC | ACC_STATIC,
        name: "main".into(),
        descriptor: "([Ljava/lang/String;)V".into(),
        attributes: vec![JvmMethodAttribute::Code(JvmCodeAttribute {
            name: "Code".into(),
            max_stack,
            max_locals: 1,
            code: main_code,
            nested_attributes: vec![],
        })],
    });

    let bytes = serialize_jvm_class_file(&class);
    let tmp = std::env::temp_dir().join(format!("mccarthy_w4_{tag}"));
    std::fs::create_dir_all(&tmp).expect("temp dir");
    std::fs::write(tmp.join("Main.class"), &bytes).expect("write Main.class");
    let out = std::process::Command::new("java")
        .arg("-Xverify:none").arg("-cp").arg(&tmp).arg("Main")
        .output().expect("run java");
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[test]
fn mccarthy_atom_eq_cond_run_on_real_jvm() {
    let Some(_) = run_on_java("(ATOM 5)", "probe") else {
        eprintln!("java absent — skipped");
        return;
    };
    // ATOM (= not pair?): an integer/symbol atom is an atom; a cons is not.
    assert_eq!(run_on_java("(ATOM 5)", "a").unwrap(), "1", "5 is an atom");
    assert_eq!(run_on_java("(ATOM (CONS 1 2))", "b").unwrap(), "0", "a cons is not an atom");
    // EQ on integer atoms. (Symbols — F6 — are W5: their interned ids live in a
    // high reserved range that needs `ldc`, which the JVM const path handles
    // separately; out of scope here.)
    assert_eq!(run_on_java("(EQ 5 5)", "c").unwrap(), "1", "5 = 5");
    assert_eq!(run_on_java("(EQ 5 6)", "d").unwrap(), "0", "5 ≠ 6");
    // COND: lisp truthiness + branch selection.
    assert_eq!(run_on_java("(COND ((EQ 1 1) 7) (5 9))", "g").unwrap(), "7", "first clause true");
    assert_eq!(run_on_java("(COND ((EQ 1 2) 7) (5 9))", "h").unwrap(), "9", "fall to second clause");
    assert_eq!(run_on_java("(COND ((ATOM (CONS 1 2)) 7) (5 9))", "i").unwrap(), "9", "pair? guard false");
}
