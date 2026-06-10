//! # JVM cons + run tests on a real JVM (LANG77 / McCarthy W3b).
//!
//! Where `jvm_emit.rs` runs *scalar* programs on the in-repo `jvm-simulator`,
//! cons cells are `Object[]` allocations the simulator cannot execute — so these
//! tests run the emitted class on a **real `java`** (Temurin via mise; the JVM
//! CI already uses). We compile a McCarthy program to a `JvmClassFile`, inject a
//! `main([Ljava/lang/String;)V` launcher that calls the entry method and
//! `System.out.println`s the `int` result, serialize, write `Main.class`, and
//! assert stdout.
//!
//! This proves the **value-model replication**: the *shared* structural passes
//! emit backend-agnostic `box`/`unbox`/`alloc`/`field_*`; the JVM backend lowers
//! them to `Integer.valueOf`/`intValue` + `Object[]` (where wasm uses
//! `i31ref`/`$LispyPair`).

use iir_to_jvm_class_file::serialize_jvm_class_file;
use jvm_class_file::{
    JvmCodeAttribute, JvmConstantPoolEntry, JvmMethodAttribute, JvmMethodInfo, ACC_PUBLIC,
    ACC_STATIC,
};
use lang_aot::{compile_source_to_jvm_class, Language};

/// Is `java` on the PATH?
fn java_available() -> bool {
    std::process::Command::new("java")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Append a CP entry, return its 1-based index.
fn cp_append(cp: &mut Vec<Option<JvmConstantPoolEntry>>, e: JvmConstantPoolEntry) -> u16 {
    cp.push(Some(e));
    (cp.len() - 1) as u16
}

/// Compile a McCarthy program, inject a `main` launcher that prints the entry's
/// `int` result, run it on a real JVM, and return the trimmed stdout.
fn compile_and_run_on_java(source: &str, dir_tag: &str) -> Option<String> {
    if !java_available() {
        return None; // skip gracefully where no JVM is present
    }
    let mut class = compile_source_to_jvm_class(Language::McCarthyLisp, source, "Main")
        .unwrap_or_else(|e| panic!("compile {source:?}: {e}"));

    // ── Inject the CP machinery for `System.out.println(I)` and a Methodref to
    //    the entry `Main.main()I`. Duplicate UTF-8/Class entries are legal. ──
    let (out_fieldref, println_ref, entry_ref) = {
        let cp = &mut class.constant_pool;

        let sys_utf8 = cp_append(cp, JvmConstantPoolEntry::Utf8("java/lang/System".into()));
        let sys_class = cp_append(cp, JvmConstantPoolEntry::Class { name_index: sys_utf8 });
        let out_utf8 = cp_append(cp, JvmConstantPoolEntry::Utf8("out".into()));
        let ps_desc = cp_append(cp, JvmConstantPoolEntry::Utf8("Ljava/io/PrintStream;".into()));
        let out_nat = cp_append(cp, JvmConstantPoolEntry::NameAndType { name_index: out_utf8, descriptor_index: ps_desc });
        let out_fieldref = cp_append(cp, JvmConstantPoolEntry::Fieldref { class_index: sys_class, name_and_type_index: out_nat });

        let ps_utf8 = cp_append(cp, JvmConstantPoolEntry::Utf8("java/io/PrintStream".into()));
        let ps_class = cp_append(cp, JvmConstantPoolEntry::Class { name_index: ps_utf8 });
        let pln_utf8 = cp_append(cp, JvmConstantPoolEntry::Utf8("println".into()));
        let pln_desc = cp_append(cp, JvmConstantPoolEntry::Utf8("(I)V".into()));
        let pln_nat = cp_append(cp, JvmConstantPoolEntry::NameAndType { name_index: pln_utf8, descriptor_index: pln_desc });
        let println_ref = cp_append(cp, JvmConstantPoolEntry::Methodref { class_index: ps_class, name_and_type_index: pln_nat });

        // Methodref → Main.main()I (the lowered entry method).
        let main_utf8 = cp_append(cp, JvmConstantPoolEntry::Utf8("Main".into()));
        let main_class = cp_append(cp, JvmConstantPoolEntry::Class { name_index: main_utf8 });
        let ent_name = cp_append(cp, JvmConstantPoolEntry::Utf8("main".into()));
        let ent_desc = cp_append(cp, JvmConstantPoolEntry::Utf8("()I".into()));
        let ent_nat = cp_append(cp, JvmConstantPoolEntry::NameAndType { name_index: ent_name, descriptor_index: ent_desc });
        let entry_ref = cp_append(cp, JvmConstantPoolEntry::Methodref { class_index: main_class, name_and_type_index: ent_nat });

        // The injected launcher's own name + descriptor (serializer looks them up).
        let _ = cp_append(cp, JvmConstantPoolEntry::Utf8("([Ljava/lang/String;)V".into()));
        (out_fieldref, println_ref, entry_ref)
    };

    // ── main([Ljava/lang/String;)V: getstatic out; invokestatic main()I;
    //    invokevirtual println(I)V; return. ──
    let [out_hi, out_lo] = out_fieldref.to_be_bytes();
    let [ent_hi, ent_lo] = entry_ref.to_be_bytes();
    let [pln_hi, pln_lo] = println_ref.to_be_bytes();
    let main_code = vec![
        0xB2, out_hi, out_lo, // getstatic System.out
        0xB8, ent_hi, ent_lo, // invokestatic Main.main()I  → int
        0xB6, pln_hi, pln_lo, // invokevirtual println(I)V
        0xB1,                 // return
    ];
    class.methods.push(JvmMethodInfo {
        access_flags: ACC_PUBLIC | ACC_STATIC,
        name: "main".into(),
        descriptor: "([Ljava/lang/String;)V".into(),
        attributes: vec![JvmMethodAttribute::Code(JvmCodeAttribute {
            name: "Code".into(),
            max_stack: 2,  // PrintStream + int
            max_locals: 1, // slot 0 = args
            code: main_code,
            nested_attributes: vec![],
        })],
    });

    let bytes = serialize_jvm_class_file(&class);
    let tmp = std::env::temp_dir().join(format!("mccarthy_w3b_{dir_tag}"));
    std::fs::create_dir_all(&tmp).expect("temp dir");
    std::fs::write(tmp.join("Main.class"), &bytes).expect("write Main.class");

    // `-Xverify:none`: the backend does not emit StackMapTables (Java 21 demands
    // them unless verification is off) — same flag the LANG36 round-trip uses.
    let out = std::process::Command::new("java")
        .arg("-Xverify:none").arg("-cp").arg(&tmp).arg("Main")
        .output().expect("run java");
    Some(format!(
        "{}|stderr:{}",
        String::from_utf8_lossy(&out.stdout).trim(),
        String::from_utf8_lossy(&out.stderr).trim()
    ))
}

fn run(source: &str, tag: &str) -> Option<String> {
    compile_and_run_on_java(source, tag).map(|s| s.split('|').next().unwrap().to_string())
}

#[test]
fn mccarthy_cons_car_cdr_run_on_real_jvm() {
    let Some(car) = run("(CAR (CONS 7 9))", "car") else { eprintln!("java absent — skipped"); return; };
    assert_eq!(car, "7", "(CAR (CONS 7 9)) → 7 on the JVM");
    assert_eq!(run("(CDR (CONS 7 9))", "cdr").unwrap(), "9", "(CDR (CONS 7 9)) → 9");
    // Nested: CAR of CDR.
    assert_eq!(run("(CAR (CDR (CONS 1 (CONS 2 3))))", "nested").unwrap(), "2", "car of cdr");
    // Scalar still works through the same path.
    assert_eq!(run("42", "scalar").unwrap(), "42", "scalar 42");
}
