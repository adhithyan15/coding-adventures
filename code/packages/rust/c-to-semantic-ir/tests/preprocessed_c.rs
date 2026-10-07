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
