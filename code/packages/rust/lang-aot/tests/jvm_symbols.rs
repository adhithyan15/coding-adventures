//! # JVM symbols on a real JVM (LANG77 / McCarthy W5a, F6).
//!
//! Symbols are interned to distinct integers in a high reserved range
//! (`SYMBOL_ID_BASE = 2²⁹`) — too large for `bipush`/`sipush`, so they exercise
//! the JVM backend's `ldc` + `CONSTANT_Integer` constant-pool path (W5a fixed an
//! invalid `ldc 0` placeholder that crashed real JVMs). With that, `EQ` on
//! symbols works: `(EQ 'X 'X)` → T, `(EQ 'X 'Y)` → nil.
//!
//! Verified on a real `java` via the same descriptor-aware launcher as the W4
//! predicate tests.

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

fn run_on_java(source: &str, tag: &str) -> Option<String> {
    if !java_available() {
        return None;
    }
    let mut class = compile_source_to_jvm_class(Language::McCarthyLisp, source, "Main")
        .unwrap_or_else(|e| panic!("compile {source:?}: {e}"));
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
    let max_stack = if is_long { 3 } else { 2 };
    class.methods.push(JvmMethodInfo {
        access_flags: ACC_PUBLIC | ACC_STATIC,
        name: "main".into(),
        descriptor: "([Ljava/lang/String;)V".into(),
        attributes: vec![JvmMethodAttribute::Code(JvmCodeAttribute {
            name: "Code".into(),
            max_stack,
            max_locals: 1,
            code: vec![0xB2, oh, ol, 0xB8, eh, el, 0xB6, ph, pl, 0xB1],
            nested_attributes: vec![],
        })],
    });

    let bytes = serialize_jvm_class_file(&class);
    let tmp = std::env::temp_dir().join(format!("mccarthy_w5a_{tag}"));
    std::fs::create_dir_all(&tmp).expect("temp dir");
    std::fs::write(tmp.join("Main.class"), &bytes).expect("write Main.class");
    let out = std::process::Command::new("java")
        .arg("-Xverify:none").arg("-cp").arg(&tmp).arg("Main")
        .output().expect("run java");
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[test]
fn mccarthy_symbols_run_on_real_jvm() {
    let Some(_) = run_on_java("(QUOTE X)", "probe") else {
        eprintln!("java absent — skipped");
        return;
    };
    // A bare symbol is its interned id — a large int via the `ldc` path (the W5a
    // fix; this used to crash the JVM at `constantTag`).
    assert_eq!(run_on_java("(QUOTE X)", "q").unwrap(), "536870912", "interned id of 'X");
    // EQ distinguishes symbols.
    assert_eq!(run_on_java("(EQ (QUOTE X) (QUOTE X))", "a").unwrap(), "1", "'X = 'X");
    assert_eq!(run_on_java("(EQ (QUOTE X) (QUOTE Y))", "b").unwrap(), "0", "'X ≠ 'Y");
    // A symbol is an atom (not a cons).
    assert_eq!(run_on_java("(ATOM (QUOTE X))", "c").unwrap(), "1", "'X is an atom");
    // Symbols are disjoint from integer atoms.
    assert_eq!(run_on_java("(EQ (QUOTE X) 5)", "d").unwrap(), "0", "a symbol ≠ an integer");
}
