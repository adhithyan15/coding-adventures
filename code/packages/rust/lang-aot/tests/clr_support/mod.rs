//! Shared harness for the **real CoreCLR** McCarthy tests (CLR-real chapter).
//!
//! `compile_source_to_cil_text` → write `.il` → assemble with real `ilasm` →
//! run on real `dotnet` → parse the printed integer. Gated on `dotnet` + `ilasm`
//! (returns `None`, i.e. *skip*, when either is absent — like the other
//! external-tool backends). Each CLR-real test file (`clr_real_scalar`,
//! `clr_real_cons`, …) `#[path]`-includes this module and asserts on the result.
//!
//! NOTE: this lives under `tests/clr_support/` (a subdirectory), so Cargo does not
//! compile it as its own test binary.

#![allow(dead_code)] // each test binary uses a subset of these helpers

use lang_aot::{compile_source_to_cil_text, Language};
use std::path::PathBuf;
use std::process::Command;

fn dotnet_ok() -> bool {
    Command::new("dotnet").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

/// Locate the `ilasm` binary: `PATH` first, then the NuGet package cache. The
/// assembler ships in the *runtime* pack `runtime.<rid>.microsoft.netcore.ilasm`
/// (the bare `microsoft.netcore.ilasm` is a ref-only package with **no** binary),
/// so we search **every** `*ilasm*` package directory — robust to `read_dir`
/// ordering — rather than the first match.
pub fn find_ilasm() -> Option<PathBuf> {
    if Command::new("ilasm").arg("/?").output().is_ok() {
        return Some(PathBuf::from("ilasm"));
    }
    let home = std::env::var_os("HOME")?;
    let pkgs = PathBuf::from(home).join(".nuget/packages");

    // Every package dir whose name mentions ilasm (handles the ref pack + the
    // runtime pack; only the latter actually contains the binary).
    let ilasm_pkgs: Vec<PathBuf> = std::fs::read_dir(&pkgs)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_dir()
                && p.file_name()
                    .map(|n| n.to_string_lossy().to_ascii_lowercase().contains("ilasm"))
                    .unwrap_or(false)
        })
        .collect();

    // Bounded DFS over those (small) package subtrees for an `ilasm` file.
    let mut stack = ilasm_pkgs;
    let mut visited = 0usize;
    while let Some(dir) = stack.pop() {
        visited += 1;
        if visited > 10_000 {
            break;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for e in entries.flatten() {
            let p = e.path();
            if e.file_name() == "ilasm" && p.is_file() {
                return Some(p);
            }
            if p.is_dir() {
                stack.push(p);
            }
        }
    }
    None
}

/// Compile McCarthy `src` → `.il` → real PE (`ilasm`) → run on real `dotnet`,
/// returning the printed integer. `None` if the toolchain is unavailable (skip).
pub fn run_on_real_clr(src: &str, tag: &str) -> Option<i64> {
    if !dotnet_ok() {
        return None;
    }
    let ilasm = find_ilasm()?;
    let il = compile_source_to_cil_text(Language::McCarthyLisp, src, "Main")
        .unwrap_or_else(|e| panic!("emit .il for {src:?}: {e}"));

    let dir = std::env::temp_dir().join(format!("clr_real_{}_{tag}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let il_path = dir.join("Main.il");
    std::fs::write(&il_path, &il).expect("write .il");
    let dll = dir.join("Main.dll");

    let asm = Command::new(&ilasm)
        .arg("-dll=false").arg("-exe")
        .arg(format!("-output={}", dll.display()))
        .arg(&il_path)
        .output()
        .expect("spawn ilasm");
    assert!(
        asm.status.success() && dll.exists(),
        "ilasm failed for {src:?}: {}\n{}\n--- emitted .il ---\n{il}",
        String::from_utf8_lossy(&asm.stdout),
        String::from_utf8_lossy(&asm.stderr),
    );
    std::fs::write(
        dir.join("Main.runtimeconfig.json"),
        r#"{ "runtimeOptions": { "tfm": "net9.0", "framework": { "name": "Microsoft.NETCore.App", "version": "9.0.0" } } }"#,
    )
    .expect("write runtimeconfig");

    let out = Command::new("dotnet").arg(&dll).output().expect("spawn dotnet");
    assert!(
        out.status.success(),
        "dotnet failed for {src:?}: {}",
        String::from_utf8_lossy(&out.stderr),
    );
    // The entry's `int32` result comes back on STDERR between `<<EXIT>>` and
    // `<</EXIT>>`, not on stdout. The launcher used to write it to stdout, but
    // only for programs that did NOT print — which made "prints AND returns a
    // value" inexpressible. Separate channels remove that conditional: stdout is
    // exclusively the program's own output now.
    //
    // The sentinels are load-bearing; the .NET host writes diagnostics to stderr
    // too, so the value has to be extracted rather than parsed from the whole
    // stream.
    let err = String::from_utf8_lossy(&out.stderr);
    err.split_once("<<EXIT>>")
        .and_then(|(_, rest)| rest.split_once("<</EXIT>>"))
        .and_then(|(v, _)| v.trim().parse::<i64>().ok())
}
