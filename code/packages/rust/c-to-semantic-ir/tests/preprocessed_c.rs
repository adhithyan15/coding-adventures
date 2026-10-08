//! The file-input C frontend consumes PREP01 tokens before parsing and lowering.

use coding_adventures_source_preprocessor::Bounds;
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};

static SEQ: AtomicUsize = AtomicUsize::new(0);

fn write_fresh(path: &std::path::Path, contents: &[u8]) {
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .unwrap()
        .write_all(contents)
        .unwrap();
}

#[test]
fn included_macro_reaches_executable_output() {
    if !std::process::Command::new("ruby")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
    {
        return;
    }
    let root = std::env::temp_dir().join(format!(
        "prep01_c_{}_{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&root).unwrap();
    write_fresh(
        &root.join("main.c"),
        b"#define ANSWER 7\n#if defined(ANSWER) && ANSWER > 0\n#include \"part.h\"\n#else\nint value(void) { return 0; }\n#endif\nint main(void) { printf(\"%d\\n\", value()); return 0; }\n",
    );
    write_fresh(
        &root.join("part.h"),
        b"int value(void) { return ANSWER; }\n",
    );

    let module = c_to_semantic_ir::compile_preprocessed_file(
        "main.c",
        [root.clone()],
        "preprocessed_c",
        Bounds::default(),
    )
    .unwrap();
    let ruby = semantic_ir_to_ruby::compile(&module).unwrap().source;
    let script = root.join("output.rb");
    write_fresh(&script, ruby.as_bytes());
    let output = std::process::Command::new("ruby")
        .arg(&script)
        .output()
        .unwrap();
    std::fs::remove_dir_all(&root).unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)
        .unwrap()
        .replace("\r\n", "\n");
    assert_eq!(stdout, "7\n");
}

#[test]
fn malformed_condition_reports_its_directive_location() {
    let root = std::env::temp_dir().join(format!(
        "prep01_c_bad_{}_{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&root).unwrap();
    write_fresh(
        &root.join("main.c"),
        b"#if 1 &&\nint main(void) { return 1; }\n#endif\n",
    );
    let error = c_to_semantic_ir::compile_preprocessed_file(
        "main.c",
        [root.clone()],
        "bad_condition",
        Bounds::default(),
    )
    .unwrap_err();
    std::fs::remove_dir_all(&root).unwrap();
    assert_eq!((error.line, error.column), (1, 1), "{error}");
}

#[test]
fn rooted_elif_selects_macro_branch_and_reports_active_condition_error() {
    let root = std::env::temp_dir().join(format!(
        "prep01_c_elif_{}_{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&root).unwrap();
    write_fresh(
        &root.join("good.c"),
        b"#define FLAG 7\n#if 0\nint value(void) { return 0; }\n#elif defined(FLAG) && FLAG == 7\nint value(void) { return FLAG; }\n#elif 1 / 0\nint value(void) { return 9; }\n#endif\n",
    );
    write_fresh(
        &root.join("bad.c"),
        b"#if 0\nint value(void) { return 0; }\n#elif 1 / 0\nint value(void) { return 1; }\n#endif\n",
    );
    let module = c_to_semantic_ir::compile_preprocessed_file(
        "good.c", [root.clone()], "elif_good", Bounds::default(),
    ).unwrap();
    let text = semantic_ir::print_module(&module);
    assert!(text.contains("(block (int 7))"), "{text}");
    let error = c_to_semantic_ir::compile_preprocessed_file(
        "bad.c", [root.clone()], "elif_bad", Bounds::default(),
    ).unwrap_err();
    std::fs::remove_dir_all(&root).unwrap();
    assert_eq!((error.line, error.column), (3, 1), "{error}");
}

#[test]
fn quoted_nested_header_prefers_its_own_directory() {
    let root = std::env::temp_dir().join(format!(
        "prep01_c_nested_{}_{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&root).unwrap();
    std::fs::create_dir(root.join("sub")).unwrap();
    write_fresh(
        &root.join("main.c"),
        b"#include \"sub/a.h\"\nint main(void) { return value(); }\n",
    );
    write_fresh(&root.join("sub/a.h"), b"#include \"b.h\"\n");
    write_fresh(&root.join("sub/b.h"), b"int value(void) { return 7; }\n");
    write_fresh(&root.join("b.h"), b"int value(void) { return 9; }\n");
    let module = c_to_semantic_ir::compile_preprocessed_file(
        "main.c",
        [root.clone()],
        "nested_headers",
        Bounds::default(),
    )
    .unwrap();
    std::fs::remove_dir_all(&root).unwrap();
    let text = semantic_ir::print_module(&module);
    assert!(text.contains("(block (int 7))"), "{text}");
    assert!(!text.contains("(block (int 9))"), "{text}");
}

#[test]
fn rooted_division_selects_branch_and_zero_divisor_keeps_location() {
    let root = std::env::temp_dir().join(format!(
        "prep01_c_divmod_{}_{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&root).unwrap();
    write_fresh(
        &root.join("good.c"),
        b"#define COUNT 9\n#if COUNT / 3\nint value(void) { return 7; }\n#else\nint value(void) { return 0; }\n#endif\n",
    );
    write_fresh(
        &root.join("bad.c"),
        b"#if 8 % 0\nint value(void) { return 1; }\n#endif\n",
    );

    let module = c_to_semantic_ir::compile_preprocessed_file(
        "good.c",
        [root.clone()],
        "divmod_good",
        Bounds::default(),
    )
    .unwrap();
    let text = semantic_ir::print_module(&module);
    assert!(text.contains("(block (int 7))"), "{text}");
    assert!(!text.contains("(block (int 0))"), "{text}");

    let error = c_to_semantic_ir::compile_preprocessed_file(
        "bad.c",
        [root.clone()],
        "divmod_bad",
        Bounds::default(),
    )
    .unwrap_err();
    std::fs::remove_dir_all(&root).unwrap();
    assert_eq!((error.line, error.column), (1, 1), "{error}");
    assert!(error.message.contains("divisor is zero"), "{error}");
}

#[test]
fn rooted_bitwise_selects_branch_and_out_of_range_operand_keeps_location() {
    let root = std::env::temp_dir().join(format!(
        "prep01_c_bitwise_{}_{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&root).unwrap();
    write_fresh(
        &root.join("good.c"),
        b"#define MASK 6\n#if MASK & 2\nint value(void) { return 7; }\n#else\nint value(void) { return 0; }\n#endif\n",
    );
    write_fresh(
        &root.join("bad.c"),
        b"#if 2147483648 | 1\nint value(void) { return 1; }\n#endif\n",
    );

    let module = c_to_semantic_ir::compile_preprocessed_file(
        "good.c",
        [root.clone()],
        "bitwise_good",
        Bounds::default(),
    )
    .unwrap();
    let text = semantic_ir::print_module(&module);
    assert!(text.contains("(block (int 7))"), "{text}");
    assert!(!text.contains("(block (int 0))"), "{text}");

    let error = c_to_semantic_ir::compile_preprocessed_file(
        "bad.c",
        [root.clone()],
        "bitwise_bad",
        Bounds::default(),
    )
    .unwrap_err();
    std::fs::remove_dir_all(&root).unwrap();
    assert_eq!((error.line, error.column), (1, 1), "{error}");
    assert!(error.message.contains("operand is out of range"), "{error}");
}
