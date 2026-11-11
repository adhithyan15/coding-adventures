use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=src/perl_bridge_shim.c");
    println!("cargo:rerun-if-env-changed=PERL_BIN");
    println!("cargo:rerun-if-env-changed=CC");
    println!("cargo:rerun-if-env-changed=AR");

    if env::var("CARGO_CFG_TARGET_OS").ok().as_deref() == Some("windows") {
        build_windows();
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

// Strawberry's headers and import library describe the interpreter that loads
// the extension. Use its own compiler configuration, rather than compiling GNU
// Perl headers with the MSVC compiler selected for the Rust cdylib.
fn build_windows() {
    println!("cargo:rerun-if-env-changed=PATH");
    let perl =
        find_perl_bin().expect("perl-bridge requires native Strawberry Perl on PATH or PERL_BIN");
    assert_eq!(
        perl_config(&perl, "osname"),
        "MSWin32",
        "Windows XS extensions require native Strawberry Perl, not MSYS/Cygwin Perl"
    );
    let pointer_bytes = env::var("CARGO_CFG_TARGET_POINTER_WIDTH")
        .expect("target pointer width")
        .parse::<usize>()
        .expect("numeric pointer width")
        / 8;
    for key in ["ptrsize", "ivsize"] {
        assert_eq!(
            perl_config(&perl, key)
                .parse::<usize>()
                .expect("numeric Perl layout"),
            pointer_bytes,
            "Perl {key} must match the Rust target's pointer-sized IV/UV layout"
        );
    }
    assert_eq!(
        perl_config(&perl, "nvsize"),
        "8",
        "Perl NV must match Rust f64"
    );

    let core = PathBuf::from(perl_config(&perl, "archlibexp")).join("CORE");
    let import_name = perl_config(&perl, "libperl");
    let import_path = core.join(&import_name);
    assert!(
        import_path.is_file(),
        "Perl import library does not exist: {}",
        import_path.display()
    );
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR must be set"));
    let object = out_dir.join("perl_bridge_shim.o");
    let library = out_dir.join(
        if env::var("CARGO_CFG_TARGET_ENV").ok().as_deref() == Some("msvc") {
            "perl_bridge_shim.lib"
        } else {
            "libperl_bridge_shim.a"
        },
    );
    // quiet=>0 is deliberate: with quiet=>1 a failed compile only reports
    // "error building xs.dll from ...", which hid the compiler's own diagnostic
    // when perl/conduit and perl/irc-server-native failed on the first Windows
    // full build. Cargo shows this output only when the build script fails.
    run(Command::new(&perl).args([
        "-MExtUtils::CBuilder", "-e",
        "ExtUtils::CBuilder->new(quiet=>0)->compile(source=>$ARGV[0],object_file=>$ARGV[1]);",
        "src/perl_bridge_shim.c",
    ]).arg(&object), "compile Windows Perl bridge shim");
    let ar = env::var("AR").unwrap_or_else(|_| perl_config(&perl, "ar"));
    run(
        Command::new(ar).arg("crs").arg(&library).arg(&object),
        "archive Windows Perl bridge shim",
    );

    // GNU COFF archives are accepted by rust-lld on MSVC too. Verbatim keeps
    // Config's real import-library filename (e.g. libperl542.a), without
    // guessing a Perl version or replacing it with an invented .lib name.
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=perl_bridge_shim");
    println!("cargo:rustc-link-search=native={}", core.display());
    println!("cargo:rustc-link-lib=dylib:+verbatim={import_name}");
    println!("cargo:rerun-if-changed={}", import_path.display());
}

fn perl_config(perl: &str, key: &str) -> String {
    run_capture(perl, &["-MConfig", "-e", "print $Config{$ARGV[0]}", key])
        .unwrap_or_else(|| panic!("failed to query Perl Config{{{key}}}"))
        .trim()
        .to_string()
}

fn find_perl_bin() -> Option<String> {
    if let Ok(perl) = env::var("PERL_BIN") {
        let trimmed = perl.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    // Windows BUILD recipes load the DLL with the Perl on PATH. A mise shim
    // must not silently select another interpreter for compiling that DLL.
    if cfg!(windows) {
        return run_capture("perl", &["-e", "print $^X"]).map(|value| value.trim().to_string());
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
