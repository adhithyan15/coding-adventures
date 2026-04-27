fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    println!("cargo:warning=perl-native extensions assume non-threaded Perl (no MULTIPLICITY). If your Perl was compiled with usethreads, this extension will cause memory corruption.");

    match target_os.as_str() {
        "macos" => {
            println!("cargo:rustc-cdylib-link-arg=-undefined");
            println!("cargo:rustc-cdylib-link-arg=dynamic_lookup");
        }
        "windows" => {
            if let Some((lib_dir, lib_name)) = find_perl_lib() {
                println!("cargo:rustc-link-search=native={}", lib_dir);
                println!("cargo:rustc-link-lib={}", lib_name);
            }
        }
        _ => {}
    }
}

fn find_perl_lib() -> Option<(String, String)> {
    let perl = find_exe("perl")?;
    let parent = std::path::Path::new(&perl).parent()?;
    let lib_dir = parent.parent()?.join("lib");

    for entry in std::fs::read_dir(&lib_dir).ok()? {
        let entry = entry.ok()?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if (name_str.starts_with("perl") && name_str.ends_with(".lib"))
            || (name_str.starts_with("perl") && name_str.ends_with(".dll.a"))
        {
            let lib_name = name_str
                .trim_end_matches(".lib")
                .trim_end_matches(".dll.a")
                .to_string();
            return Some((lib_dir.to_string_lossy().to_string(), lib_name));
        }
    }
    None
}

fn find_exe(name: &str) -> Option<String> {
    let output = std::process::Command::new("where")
        .arg(name)
        .output()
        .ok()?;
    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        if !path.is_empty() {
            return Some(path);
        }
    }
    None
}
