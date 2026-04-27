use std::path::PathBuf;

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    match target_os.as_str() {
        "macos" => {
            println!("cargo:rustc-cdylib-link-arg=-undefined");
            println!("cargo:rustc-cdylib-link-arg=dynamic_lookup");
        }
        "windows" => {
            let node_lib_dir = get_or_download_node_lib();
            println!("cargo:rustc-link-search=native={}", node_lib_dir.display());
            println!("cargo:rustc-link-lib=node");
        }
        _ => {}
    }
}

fn get_or_download_node_lib() -> PathBuf {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let node_lib_path = out_dir.join("node.lib");

    if node_lib_path.exists() {
        return out_dir;
    }

    let output = std::process::Command::new("node")
        .arg("--version")
        .output()
        .expect("failed to run `node --version` -- is Node.js installed?");

    let version = String::from_utf8(output.stdout)
        .expect("node --version output is not UTF-8")
        .trim()
        .to_string();

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

    let url = format!("https://nodejs.org/dist/{}/{}/node.lib", version, win_arch);

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
        let status = std::process::Command::new("curl")
            .args(["-sL", "-o", &node_lib_path.to_string_lossy(), &url])
            .status()
            .expect("failed to download node.lib with curl");

        if !status.success() {
            panic!(
                "Could not download node.lib from {}. Please download it manually and place it in {}",
                url,
                out_dir.display()
            );
        }
    }

    out_dir
}
