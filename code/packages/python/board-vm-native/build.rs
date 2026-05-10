fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    match target_os.as_str() {
        "macos" => {
            println!("cargo:rustc-cdylib-link-arg=-undefined");
            println!("cargo:rustc-cdylib-link-arg=dynamic_lookup");
        }
        "windows" => {
            let python = std::env::var("PYO3_PYTHON")
                .or_else(|_| std::env::var("PYTHON_SYS_EXECUTABLE"))
                .or_else(|_| which_python())
                .unwrap_or_else(|_| "python".to_string());

            let script = "import sys, os; prefix = sys.base_prefix; v = sys.version_info; \
                          lib_dir = os.path.join(prefix, 'libs'); \
                          lib_name = f'python{v.major}{v.minor}'; \
                          print(f'{lib_dir}|{lib_name}')";
            if let Ok(output) = std::process::Command::new(&python)
                .args(["-c", script])
                .output()
            {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if let Some((lib_dir, lib_name)) = stdout.split_once('|') {
                    println!("cargo:rustc-link-search=native={}", lib_dir);
                    println!("cargo:rustc-link-lib={}", lib_name);
                }
            }
        }
        _ => {}
    }
}

fn which_python() -> Result<String, std::env::VarError> {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let finder = if target_os == "windows" {
        "where"
    } else {
        "which"
    };
    for name in &["python3", "python"] {
        if let Ok(output) = std::process::Command::new(finder).arg(name).output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if !path.is_empty() {
                    return Ok(path);
                }
            }
        }
    }
    Err(std::env::VarError::NotPresent)
}
