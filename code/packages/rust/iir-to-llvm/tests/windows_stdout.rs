//! The LANG83 byte stream must survive the Windows CRT's default text mode.
use iir_to_llvm::{lower_iir_to_llvm, IIRLlvmConfig};
use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};

fn output_module(builtin: &str) -> IIRModule {
    let mut module = IIRModule::new("stdout", "iir");
    module.entry_point = Some("main".into());
    module.add_or_replace(IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("const", Some("c".into()), vec![Operand::Int(10)], "i64"),
            IIRInstr::new(
                "call_builtin",
                None,
                vec![Operand::Var(builtin.into()), Operand::Var("c".into())],
                "void",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "i64"),
        ],
    ));
    module
}

#[test]
fn windows_output_initializes_binary_stdout_without_a_string_runtime() {
    for target in [
        "x86_64-pc-windows-msvc",
        "x86_64-pc-windows-gnu",
        "i686-w64-mingw32",
    ] {
        for builtin in ["print_i64", "write_i64", "putchar"] {
            let ll = lower_iir_to_llvm(
                &output_module(builtin),
                &IIRLlvmConfig::new("stdout").with_target(target),
            )
            .unwrap();
            assert!(
                ll.contains("@llvm.global_ctors = appending global"),
                "{target}: {builtin}"
            );
            assert!(ll.contains("call i32 @_setmode(i32 1, i32 32768)"), "{ll}");
        }
    }
}

#[test]
fn other_targets_and_windows_modules_without_output_need_no_crt_setup() {
    for target in ["x86_64-unknown-linux-gnu", "aarch64-apple-darwin"] {
        let ll = lower_iir_to_llvm(
            &output_module("putchar"),
            &IIRLlvmConfig::new("stdout").with_target(target),
        )
        .unwrap();
        assert!(!ll.contains("_setmode"));
        assert!(!ll.contains("@llvm.global_ctors"));
    }
    let ll = lower_iir_to_llvm(
        &IIRModule::new("pure", "iir"),
        &IIRLlvmConfig::new("pure").with_target("x86_64-pc-windows-msvc"),
    )
    .unwrap();
    assert!(!ll.contains("_setmode"));
}

#[test]
fn a_module_cannot_define_the_windows_stdout_constructor_name() {
    let mut module = output_module("putchar");
    module.functions.push(IIRFunction::new(
        "__iir_windows_stdout_binary",
        vec![],
        "void",
        vec![IIRInstr::new("ret_void", None, vec![], "void")],
    ));
    let result = lower_iir_to_llvm(
        &module,
        &IIRLlvmConfig::new("stdout").with_target("x86_64-pc-windows-msvc"),
    );
    assert!(
        result.is_err(),
        "duplicate generated definitions must be refused"
    );
}
