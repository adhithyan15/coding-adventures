//! # McCarthy on **real CoreCLR** — scalar (CLR-real C1).
//!
//! `compile_source_to_cil_text` emits textual CIL (`.il`); this harness assembles
//! it with the real `ilasm` into a loadable PE and runs it on real `dotnet`,
//! asserting the printed result. The CLR analog of `llvm_*` (clang) — the CLR
//! column is now verified on its **real runtime**, not only the in-repo simulator.
//!
//! Gated on `dotnet` + `ilasm` (skips gracefully when absent, like the other
//! external-tool backends). `ilasm` ships as the NuGet runtime pack
//! `runtime.<rid>.Microsoft.NETCore.ILAsm` (CI fetches it via `dotnet restore`);
//! locally it is found on `PATH` or in the NuGet package cache.

use lang_aot::{compile_source_to_cil_text, Language};
use std::path::PathBuf;
use std::process::Command;

fn dotnet_ok() -> bool {
    Command::new("dotnet").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

/// Find `ilasm`: PATH first, then the NuGet package cache. The cache layout is
/// `~/.nuget/packages/runtime.<rid>.microsoft.netcore.ilasm/<ver>/runtimes/<rid>/native/ilasm`,
/// so we locate the (small) `*ilasm*` package directory and walk only that subtree.
fn find_ilasm() -> Option<PathBuf> {
    if Command::new("ilasm").arg("/?").output().is_ok() {
        return Some(PathBuf::from("ilasm"));
    }
    let home = std::env::var_os("HOME")?;
    let pkgs = PathBuf::from(home).join(".nuget/packages");
    let pkg_dir = std::fs::read_dir(&pkgs)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.is_dir()
                && p.file_name()
                    .map(|n| n.to_string_lossy().to_ascii_lowercase().contains("ilasm"))
                    .unwrap_or(false)
        })?;
    let mut stack = vec![pkg_dir];
    let mut visited = 0usize;
    while let Some(dir) = stack.pop() {
        visited += 1;
        if visited > 5_000 {
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
/// returning the printed integer. `None` if the toolchain is unavailable.
fn run_on_real_clr(src: &str, tag: &str) -> Option<i64> {
    if !dotnet_ok() {
        return None;
    }
    let ilasm = find_ilasm()?;
    let il = compile_source_to_cil_text(Language::McCarthyLisp, src, "Main")
        .unwrap_or_else(|e| panic!("emit .il for {src:?}: {e}"));

    let dir = std::env::temp_dir().join(format!("clr_real_c1_{}_{tag}", std::process::id()));
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
        "ilasm failed for {src:?}: {}\n{}",
        String::from_utf8_lossy(&asm.stdout),
        String::from_utf8_lossy(&asm.stderr),
    );
    // Framework-dependent runtimeconfig so `dotnet Main.dll` resolves the shared FX.
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
    String::from_utf8_lossy(&out.stdout).trim().parse::<i64>().ok()
}

#[test]
fn mccarthy_scalar_runs_on_real_coreclr() {
    let Some(v) = run_on_real_clr("42", "n42") else {
        eprintln!("dotnet/ilasm absent — skipping real-CoreCLR scalar test");
        return;
    };
    assert_eq!(v, 42, "McCarthy `42` on real CoreCLR");
    assert_eq!(run_on_real_clr("0", "n0").unwrap(), 0);
    assert_eq!(run_on_real_clr("7", "n7").unwrap(), 7);
}
