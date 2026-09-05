use std::{fs, path::PathBuf, process::Command};
#[test]
fn root_package_and_both_themes_compile() {
    let package = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rust = package.join("../../../packages/rust/Cargo.toml");
    let output = package.join("target/package-check");
    for theme in ["light", "dark"] {
        let destination = output.join(theme);
        let result = Command::new("cargo").args(["run", "--quiet", "--manifest-path"])
            .arg(&rust).args(["-p", "mosaic-compile", "--", "pkg"])
            .arg(&package).args(["--backend", "react", "--theme", theme, "--output"])
            .arg(&destination).output().expect("run package compiler");
        assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
        let source = fs::read_to_string(destination.join("react/VisiCalc.tsx")).unwrap();
        assert!(source.contains("gridNavigate"));
        assert!(source.contains("gridSelectedRow"));
        assert!(source.contains("newWorkbook"));
        assert!(source.contains("minHeight: \"100vh\""));
    }
}
