use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=src/perl_bridge_shim.c");
    println!("cargo:rerun-if-env-changed=PERL_BIN");
    println!("cargo:rerun-if-env-changed=CC");
    println!("cargo:rerun-if-env-changed=AR");

    if env::var("CARGO_CFG_TARGET_OS").ok().as_deref() == Some("windows") {
        return;
    }

    let perl = find_perl_bin().expect("perl-bridge requires a Perl interpreter to build");
    let cc = env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let ar = env::var("AR").unwrap_or_else(|_| "ar".to_string());
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR must be set"));
    let object_path = out_dir.join("perl_bridge_shim.o");
    let library_path = out_dir.join("libperl_bridge_shim.a");
    let mut compile = Command::new(&cc);

    compile
        .arg("-c")
        .arg("-fPIC")
        .arg("src/perl_bridge_shim.c")
        .arg("-o")
        .arg(&object_path);

    for flag in perl_ccopts(&perl) {
        compile.arg(flag);
    }

    run(&mut compile, "compile perl bridge shim");

    let mut archive = Command::new(&ar);
    archive.arg("crus").arg(&library_path).arg(&object_path);
    run(&mut archive, "archive perl bridge shim");

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=perl_bridge_shim");
}

fn find_perl_bin() -> Option<String> {
    if let Ok(perl) = env::var("PERL_BIN") {
        let trimmed = perl.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    if let Some(path) = run_capture("mise", &["which", "perl"]) {
        let trimmed = path.trim();
        if !trimmed.is_empty() && Path::new(trimmed).exists() {
            return Some(trimmed.to_string());
        }
    }

    run_capture("perl", &["-e", "print $^X"]).map(|value| value.trim().to_string())
}

fn perl_ccopts(perl: &str) -> Vec<String> {
    let output = Command::new(perl)
        .args(["-MExtUtils::Embed", "-e", "ccopts()"])
        .output()
        .expect("failed to query perl compiler flags");

    if !output.status.success() {
        panic!("failed to query perl compiler flags");
    }

    String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

fn run_capture(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).to_string())
}

fn run(command: &mut Command, description: &str) {
    let status = command
        .status()
        .unwrap_or_else(|error| panic!("failed to {}: {}", description, error));

    if !status.success() {
        panic!("failed to {}", description);
    }
}
