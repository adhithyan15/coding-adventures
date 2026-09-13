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
        assert!(source.contains("<thead style={{ position: \"sticky\", top: \"0px\", zIndex: 1 }}"));
        assert!(source.contains("height: \"60vh\""));

        // #15048 -- the inline cell editor must carry the geometry that
        // keeps an editing row the same height as a display row. That
        // geometry comes from `mosaic-pkg-grid`'s `cell-editor` part and
        // reaches here through `mosaic-pkg-sheet`; VisiCalc deliberately
        // does NOT override the part, because a part declared here would
        // REPLACE the package's wholesale rather than merge with it, and
        // an override that restated only some properties would silently
        // put the bare user-agent input back. The symptom is invisible in
        // review -- the grid just quietly refuses uniform viewport
        // capacity -- so pin it against the emitted source.
        let editor = source
            .split("<input")
            .find(|chunk| chunk.contains("value={editContent}"))
            .unwrap_or_else(|| panic!("{theme}: no cell editor in the emitted React"));
        for needle in [
            "padding: \"0px\"",
            "border: \"0px\"",
            "width: \"100%\"",
            "boxSizing: \"border-box\"",
        ] {
            assert!(
                editor.contains(needle),
                "{theme}: the cell editor lost `{needle}` -- an editing row will \
                 no longer match a display row. Emitted: {editor}"
            );
        }
    }
}
