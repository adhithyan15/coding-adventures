//! # c-to-semantic-ir — C (integer-core subset) → Semantic IR.
//!
//! The mirror of the Ruby/Python frontends for a **strict, typed** source
//! language, and the last piece of the C → SIR → Ruby initiative.  It parses C
//! with [`coding_adventures_c_parser`], then walks the CST assigning a concrete
//! `IntSpec` to every expression and inserting `Expr::Convert` nodes per C's
//! integer promotions / usual-arithmetic-conversions — so a C program's
//! width/wrap/truncate semantics survive the narrow waist and every backend
//! reproduces the program's results.
//!
//! Implements [SIR27](../../../specs/SIR27-c-to-semantic-ir.md).  Milestone 1:
//! functions, typed `+`/`-`/`*` arithmetic (with `Convert` insertion), casts,
//! declarations & assignments, `printf`, and `return`.

mod lower;

pub use lower::{compile, CLowerError};

/// Parse C `source` and lower it to a [`semantic_ir::Module`].
pub fn compile_source(source: &str, module_name: &str) -> Result<semantic_ir::Module, CLowerError> {
    let tree = coding_adventures_c_parser::try_parse_c(source).map_err(|msg| CLowerError {
        message: format!("C parse error: {msg}"),
        line: 0,
        column: 0,
    })?;
    compile(&tree, module_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static SEQ: AtomicUsize = AtomicUsize::new(0);

    /// A per-(process, call) unique stem so parallel tests never share a file.
    fn uniq(ext: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "c2sir_{}_{}{ext}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn lower(src: &str) -> semantic_ir::Module {
        compile_source(src, "test").expect("C lowering succeeded")
    }

    #[test]
    fn source_language_is_c_and_validates() {
        let m = lower("int main(void) { return 0; }");
        assert_eq!(m.metadata.source_language.as_deref(), Some("c"));
        assert!(semantic_ir::validate(&m).is_ok());
    }

    #[test]
    fn uint8_overflow_inserts_convert_chain() {
        // uint8_t c = 200 + 100  →  the assignment narrows to u8 and the add
        // promotes its u8 operands to i32, so the SIR text contains both a u8
        // and an i32 convert.
        let m = lower("int main(void) { uint8_t c = 200 + 100; return 0; }");
        let text = semantic_ir::print_module(&m);
        assert!(
            text.contains("(convert (int u8 wrap)"),
            "no u8 convert:\n{text}"
        );
        assert!(
            text.contains("(convert (int i32 ub)"),
            "no i32 promote:\n{text}"
        );
    }

    // ── end-to-end helpers ──────────────────────────────────────────────────

    fn run_ruby(m: &semantic_ir::Module) -> Option<String> {
        use std::io::Write;
        let src = semantic_ir_to_ruby::compile(m).ok()?.source;
        let path = uniq(".rb");
        std::fs::File::create(&path)
            .ok()?
            .write_all(src.as_bytes())
            .ok()?;
        let out = std::process::Command::new("ruby")
            .arg(&path)
            .output()
            .ok()?;
        let _ = std::fs::remove_file(&path);
        if !out.status.success() {
            panic!(
                "emitted ruby failed:\n{}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        Some(
            String::from_utf8_lossy(&out.stdout)
                .replace("\r\n", "\n")
                .trim_end()
                .to_string(),
        )
    }

    fn find_cc() -> Option<String> {
        if let Ok(cc) = std::env::var("SIR_CC") {
            if !cc.trim().is_empty() {
                return Some(cc);
            }
        }
        for c in ["cc", "clang", "gcc"] {
            if std::process::Command::new(c)
                .arg("--version")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                return Some(c.to_string());
            }
        }
        None
    }

    fn run_c(m: &semantic_ir::Module) -> Option<String> {
        use std::io::Write;
        let cc = find_cc()?;
        let src = semantic_ir_to_c::compile(m).ok()?.source;
        let cpath = uniq(".c");
        let exe = uniq(std::env::consts::EXE_SUFFIX);
        std::fs::File::create(&cpath)
            .ok()?
            .write_all(src.as_bytes())
            .ok()?;
        let ok = std::process::Command::new(&cc)
            .arg("-std=c99")
            .arg("-o")
            .arg(&exe)
            .arg(&cpath)
            .output()
            .ok()?;
        assert!(
            ok.status.success(),
            "emitted C failed to compile:\n{}\n{src}",
            String::from_utf8_lossy(&ok.stderr)
        );
        let run = std::process::Command::new(&exe).output().ok()?;
        let _ = std::fs::remove_file(&cpath);
        let _ = std::fs::remove_file(&exe);
        Some(
            String::from_utf8_lossy(&run.stdout)
                .replace("\r\n", "\n")
                .trim_end()
                .to_string(),
        )
    }

    /// The headline case: a C program and BOTH its translations agree on the
    /// uint8 overflow — 200 + 100 == 44 (mod 256).
    #[test]
    fn uint8_overflow_roundtrips_through_ruby_and_c() {
        let m = lower("int main(void) { uint8_t c = 200 + 100; printf(\"%d\\n\", c); return 0; }");
        if let Some(out) = run_ruby(&m) {
            assert_eq!(out, "44", "ruby");
        }
        if let Some(out) = run_c(&m) {
            assert_eq!(out, "44", "c");
        }
    }

    #[test]
    fn int32_overflow_roundtrips() {
        // (int32_t)(2000000000 + 2000000000): the operands are int (i32) so the
        // add is at i32 and wraps: 4000000000 - 2^32 = -294967296.
        let m = lower(
            "int main(void) { int32_t y = (int32_t)(2000000000 + 2000000000); printf(\"%d\\n\", y); return 0; }",
        );
        if let Some(out) = run_ruby(&m) {
            assert_eq!(out, "-294967296", "ruby");
        }
        if let Some(out) = run_c(&m) {
            assert_eq!(out, "-294967296", "c");
        }
    }

    #[test]
    fn function_calls_and_params_roundtrip() {
        let m = lower(
            "int add(int a, int b) { return a + b; }\n\
             int main(void) { printf(\"%d\\n\", add(2, 3)); return 0; }",
        );
        if let Some(out) = run_ruby(&m) {
            assert_eq!(out, "5", "ruby");
        }
        if let Some(out) = run_c(&m) {
            assert_eq!(out, "5", "c");
        }
    }
}
