//! # JVM `LAMBDA`/`LABEL`/recursion on a real JVM (LANG77 / McCarthy W5b, F7).
//!
//! Completes the JVM backend. The frontend lifts each `LAMBDA`/`LABEL` to its
//! own method; the structural pass makes the call boundary uniform-anyref →
//! `Object` params/returns, with `invokestatic` for calls and recursion. The
//! pass also makes a `COND` **funnel** that can hold a reference (a cons, nil, or
//! a recursive call result) uniform — boxing its atom clauses — so a strict
//! backend like the JVM can give the result one type. Verified on a real `java`.

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
    let tmp = std::env::temp_dir().join(format!("mccarthy_w5b_{tag}"));
    std::fs::create_dir_all(&tmp).expect("temp dir");
    std::fs::write(tmp.join("Main.class"), &bytes).expect("write Main.class");
    let out = std::process::Command::new("java")
        .arg("-Xverify:none").arg("-cp").arg(&tmp).arg("Main")
        .output().expect("run java");
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[test]
fn mccarthy_lambda_and_recursion_run_on_real_jvm() {
    let Some(_) = run_on_java("((LAMBDA (X) X) 5)", "probe") else {
        eprintln!("java absent — skipped");
        return;
    };
    // Lambda application, multi-arg, a predicate and a cons inside a lambda.
    assert_eq!(run_on_java("((LAMBDA (X) X) 5)", "a").unwrap(), "5", "id lambda");
    assert_eq!(run_on_java("(CAR ((LAMBDA (X) (CONS X X)) 7))", "b").unwrap(), "7", "cons in a lambda");
    assert_eq!(run_on_java("(CDR ((LAMBDA (X Y) (CONS X Y)) 3 4))", "c").unwrap(), "4", "two-arg lambda");
    assert_eq!(run_on_java("((LAMBDA (X) (EQ X X)) 5)", "d").unwrap(), "1", "EQ inside a lambda");
    // A recursive LABEL walking a list to its atom tail.
    let rec = "((LABEL F (LAMBDA (L) (COND ((ATOM L) 99) ((EQ 1 1) (F (CDR L)))))) (CONS 1 (CONS 2 3)))";
    assert_eq!(run_on_java(rec, "e").unwrap(), "99", "recursive LABEL");
    // A COND funnel mixing an atom clause and a cons clause (the funnel-uniformity
    // case): the atom clause is taken here.
    assert_eq!(
        run_on_java("(COND ((ATOM 5) 7) ((EQ 1 1) (CONS 1 2)))", "f").unwrap(),
        "7",
        "mixed atom/cons COND funnel"
    );
}
