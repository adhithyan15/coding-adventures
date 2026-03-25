// build.rs -- Linker configuration for Node.js N-API native addon
// ================================================================
//
// Node.js N-API addons are shared libraries (.dll/.so/.dylib) that
// import symbols from the Node.js runtime. The symbols (napi_create_string,
// napi_create_function, etc.) are resolved at load time by the dynamic linker.
//
// Each platform handles "undefined symbols in a shared library" differently:
//
// - **Linux**: ELF shared libraries allow undefined symbols by default.
//   The linker defers resolution to load time. No flags needed.
//
// - **macOS**: Mach-O requires `-undefined dynamic_lookup` to allow
//   undefined symbols that will be resolved when loaded into a process.
//
// - **Windows**: MSVC's link.exe requires ALL symbols resolved at link
//   time. We must link against `node.lib`, the import library shipped
//   with Node.js headers. This tells the linker "these symbols exist in
//   node.exe and will be available at runtime". We download it from the
//   official Node.js release if not already cached.

use std::path::PathBuf;

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    match target_os.as_str() {
        "macos" => {
            // macOS: allow undefined symbols to be resolved at load time.
            println!("cargo:rustc-cdylib-link-arg=-undefined");
            println!("cargo:rustc-cdylib-link-arg=dynamic_lookup");
        }
        "windows" => {
            // Windows: we need node.lib to resolve N-API symbols at link time.
            // First, try to find it via node-gyp's cache or download it.
            let node_lib_dir = get_or_download_node_lib();
            println!("cargo:rustc-link-search=native={}", node_lib_dir.display());
            println!("cargo:rustc-link-lib=node");
        }
        _ => {
            // Linux and others: ELF allows undefined symbols by default.
        }
    }
}

/// Get the directory containing node.lib, downloading it if necessary.
///
/// On Windows, Node.js native addons need to link against node.lib -- an
/// import library that tells the linker "these symbols (napi_create_string,
/// napi_wrap, etc.) will be provided by node.exe at runtime."
///
/// We download it from the official Node.js release artifacts.
/// The file is cached in the target directory so it's only downloaded once.
fn get_or_download_node_lib() -> PathBuf {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let node_lib_path = out_dir.join("node.lib");

    if node_lib_path.exists() {
        return out_dir;
    }

    // Get the Node.js version by running `node --version`.
    let output = std::process::Command::new("node")
        .arg("--version")
        .output()
        .expect("failed to run `node --version` -- is Node.js installed?");

    let version = String::from_utf8(output.stdout)
        .expect("node --version output is not UTF-8")
        .trim()
        .to_string();

    // Determine architecture.
    let arch_output = std::process::Command::new("node")
        .arg("-e")
        .arg("process.stdout.write(process.arch)")
        .output()
        .expect("failed to get node arch");
    let arch = String::from_utf8(arch_output.stdout).unwrap();

    let win_arch = match arch.as_str() {
        "x64" => "win-x64",
        "arm64" => "win-arm64",
        "ia32" => "win-x86",
        _ => "win-x64",
    };

    let url = format!(
        "https://nodejs.org/dist/{}/{}/node.lib",
        version, win_arch
    );

    println!("cargo:warning=Downloading node.lib from {}", url);

    // Use PowerShell to download (available on all Windows systems).
    let status = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!(
                "Invoke-WebRequest -Uri '{}' -OutFile '{}'",
                url,
                node_lib_path.display()
            ),
        ])
        .status()
        .expect("failed to run PowerShell to download node.lib");

    if !status.success() {
        // Fallback: try curl (available on Windows 10+).
        let status = std::process::Command::new("curl")
            .args(["-sL", "-o", &node_lib_path.to_string_lossy(), &url])
            .status()
            .expect("failed to download node.lib with curl");

        if !status.success() {
            panic!(
                "Could not download node.lib from {}. \
                 Please download it manually and place it in {}",
                url,
                out_dir.display()
            );
        }
    }

    out_dir
}
