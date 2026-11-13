// build.rs -- link Ruby's DLL on Windows
//
// font-parser-ruby is a Ruby extension: a cdylib that `require` loads into a
// running interpreter.  The C API functions it calls (rb_define_module,
// rb_hash_aset, ...) are provided by that interpreter, so each platform needs a
// different answer at link time:
//
// - macOS: `-undefined dynamic_lookup` -- resolve symbols when Ruby loads us.
// - Linux: nothing -- ELF shared objects may leave symbols undefined.
// - Windows: a PE/COFF DLL must name the module behind every import, and the
//   functions live in Ruby's own DLL (`x64-ucrt-ruby340.dll` for RubyInstaller
//   3.4).  RubyInstaller ships a MinGW import library (`.dll.a`), which
//   MSVC's link.exe cannot read, so we write a module-definition (.def) file
//   naming the DLL and every `rb_*` symbol this crate declares, and have
//   MSVC's `lib.exe` turn it into an import library.  (On the GNU target,
//   MinGW's ld reads RubyInstaller's `.dll.a` directly and none of this is
//   needed.)
//
// The symbol list is not hand-maintained: it is scanned from the `extern "C"`
// declarations in ruby-bridge and in this crate, so it cannot drift from the
// code.  A symbol that is listed but never called costs nothing; a symbol that
// is called but missing from the DLL fails the link loudly.

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=../ruby-bridge/src/lib.rs");
    println!("cargo:rerun-if-env-changed=RUBY");

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    match target_os.as_str() {
        "macos" => {
            println!("cargo:rustc-cdylib-link-arg=-undefined");
            println!("cargo:rustc-cdylib-link-arg=dynamic_lookup");
        }
        "windows" => link_ruby_import_library(),
        _ => {
            // Linux and other ELF targets: nothing to do.
        }
    }
}

/// The `rb_*` symbols declared by `extern "C"` blocks in `source`, with a flag
/// that is true for `static` (data) symbols such as `rb_cObject`.
///
/// Only declarations are matched (`fn rb_x(` / `static rb_x:`, optionally
/// `pub`); `unsafe extern "C" fn rb_load(` -- a function this crate *defines* --
/// starts with `unsafe` and is skipped.
fn scan_ruby_symbols(source: &str) -> Vec<(String, bool)> {
    let mut found: Vec<(String, bool)> = Vec::new();
    for line in source.lines() {
        let line = line.trim_start();
        let line = line.strip_prefix("pub ").unwrap_or(line);
        let (is_data, rest) = if let Some(rest) = line.strip_prefix("fn rb_") {
            (false, rest)
        } else if let Some(rest) = line.strip_prefix("static rb_") {
            (true, rest)
        } else {
            continue;
        };
        let tail: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if tail.is_empty() {
            continue;
        }
        let name = format!("rb_{tail}");
        if !found.iter().any(|(n, _)| *n == name) {
            found.push((name, is_data));
        }
    }
    found.sort();
    found
}

/// The text of a module-definition file that exports `symbols` from `dll_name`.
fn def_file_contents(dll_name: &str, symbols: &[(String, bool)]) -> String {
    let mut text = format!("LIBRARY {dll_name}\nEXPORTS\n");
    for (name, is_data) in symbols {
        text.push_str("    ");
        text.push_str(name);
        if *is_data {
            text.push_str(" DATA");
        }
        text.push('\n');
    }
    text
}

fn link_ruby_import_library() {
    let ruby = std::env::var("RUBY").unwrap_or_else(|_| "ruby".to_string());
    // Missing prerequisites are a warning, not a panic: `cargo check` and
    // `cargo clippy` never link and must keep working on a machine with no Ruby.
    // Only an actual link needs the import library, and it fails right after.
    let Some(so_name) = ruby_config(&ruby, "RUBY_SO_NAME") else {
        println!("cargo:warning=font-parser-ruby: `{ruby}` did not report RbConfig RUBY_SO_NAME; put ruby on PATH or set RUBY; linking will fail without it");
        return;
    };

    // The GNU target (x86_64-pc-windows-gnu) links with MinGW's ld, which reads
    // RubyInstaller's own `lib<so_name>.dll.a` directly: no .def, no lib.exe.
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu") {
        if let Some(libdir) = ruby_config(&ruby, "libdir") {
            println!("cargo:rustc-link-search=native={libdir}");
        }
        println!("cargo:rustc-link-lib=dylib={so_name}");
        return;
    }

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let mut symbols = Vec::new();
    for source in [
        manifest_dir.join("src/lib.rs"),
        manifest_dir.join("../ruby-bridge/src/lib.rs"),
    ] {
        let text = std::fs::read_to_string(&source)
            .unwrap_or_else(|e| panic!("font-parser-ruby: cannot read {}: {e}", source.display()));
        for symbol in scan_ruby_symbols(&text) {
            if !symbols.contains(&symbol) {
                symbols.push(symbol);
            }
        }
    }
    symbols.sort();

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let def_path = out_dir.join(format!("{so_name}.def"));
    let lib_path = out_dir.join(format!("{so_name}.lib"));
    std::fs::write(&def_path, def_file_contents(&format!("{so_name}.dll"), &symbols))
        .expect("font-parser-ruby: cannot write the .def file");

    let Some(lib_exe) = find_lib_exe() else {
        println!("cargo:warning=font-parser-ruby: MSVC lib.exe not found on PATH or via vswhere; install the Visual Studio C++ tools; linking will fail without it");
        return;
    };
    let output = Command::new(&lib_exe)
        .arg(format!("/def:{}", def_path.display()))
        .arg(format!("/out:{}", lib_path.display()))
        .arg("/machine:x64")
        .arg("/nologo")
        .output()
        .unwrap_or_else(|e| panic!("font-parser-ruby: cannot run {}: {e}", lib_exe.display()));
    if !output.status.success() || !lib_path.exists() {
        panic!(
            "font-parser-ruby: {} failed:\n{}{}",
            lib_exe.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    println!("cargo:warning=font-parser-ruby: linking {so_name}.dll ({} symbols) via {}", symbols.len(), lib_exe.display());
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=dylib={so_name}");
}

fn ruby_config(ruby: &str, key: &str) -> Option<String> {
    let output = Command::new(ruby)
        .args(["-e", &format!("print RbConfig::CONFIG['{key}']")])
        .output()
        .ok()?;
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if output.status.success() && !value.is_empty() {
        Some(value)
    } else {
        None
    }
}

/// MSVC's `lib.exe`: on PATH if a developer prompt set it up, otherwise found
/// through vswhere, exactly as `_windows_cargo.sh` finds `link.exe`.  `lib.exe`
/// with `/def:` needs no INCLUDE/LIB environment, only its own directory.
fn find_lib_exe() -> Option<PathBuf> {
    if Command::new("lib.exe")
        .arg("/?")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return Some(PathBuf::from("lib.exe"));
    }

    let program_files = std::env::var("ProgramFiles(x86)")
        .unwrap_or_else(|_| r"C:\Program Files (x86)".to_string());
    let vswhere = Path::new(&program_files).join(r"Microsoft Visual Studio\Installer\vswhere.exe");
    let output = Command::new(vswhere)
        .args([
            "-latest",
            "-products",
            "*",
            "-requires",
            "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
            "-property",
            "installationPath",
        ])
        .output()
        .ok()?;
    let install = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if install.is_empty() {
        return None;
    }

    let mut versions: Vec<PathBuf> = std::fs::read_dir(Path::new(&install).join(r"VC\Tools\MSVC"))
        .ok()?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .collect();
    versions.sort();
    versions
        .into_iter()
        .rev()
        .map(|v| v.join(r"bin\Hostx64\x64\lib.exe"))
        .find(|p| p.exists())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_declarations_not_definitions() {
        let src = r#"
extern "C" {
    pub fn rb_define_module(name: *const c_char) -> VALUE;
    fn rb_intern(name: *const c_char) -> usize;
    pub static rb_cObject: VALUE;
}
unsafe extern "C" fn rb_load(data: VALUE) -> VALUE { 0 }
pub fn define_module(name: &str) -> VALUE { 0 }
"#;
        assert_eq!(
            scan_ruby_symbols(src),
            vec![
                ("rb_cObject".to_string(), true),
                ("rb_define_module".to_string(), false),
                ("rb_intern".to_string(), false),
            ]
        );
    }

    #[test]
    fn def_file_names_the_dll_and_marks_data() {
        let text = def_file_contents("x64-ucrt-ruby340.dll", &[("rb_a".into(), false), ("rb_cB".into(), true)]);
        assert_eq!(text, "LIBRARY x64-ucrt-ruby340.dll\nEXPORTS\n    rb_a\n    rb_cB DATA\n");
    }
}
