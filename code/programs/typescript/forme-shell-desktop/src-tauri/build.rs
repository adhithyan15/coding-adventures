fn main() {
    #[cfg(feature = "desktop")]
    {
        let manifest = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
        let suffix = if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
            ".exe"
        } else {
            ""
        };
        let worker = manifest
            .parent()
            .unwrap()
            .join("dist/worker")
            .join(format!("forme-product-worker{suffix}"));
        let digest_path = std::path::PathBuf::from(format!("{}.sha256", worker.display()));
        println!("cargo:rerun-if-changed={}", worker.display());
        println!("cargo:rerun-if-changed={}", digest_path.display());
        let digest = std::fs::read_to_string(&digest_path)
            .expect("build the reviewed Forme product worker before compiling the desktop host");
        let digest = digest.trim();
        assert!(
            digest.len() == 64
                && digest
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
            "the generated Forme product-worker digest is malformed"
        );
        println!("cargo:rustc-env=FORME_PRODUCT_WORKER_SHA256={digest}");
        let launcher = manifest
            .parent()
            .unwrap()
            .join("dist/worker/forme-sandbox-macos");
        let launcher_digest_path =
            std::path::PathBuf::from(format!("{}.sha256", launcher.display()));
        println!("cargo:rerun-if-changed={}", launcher.display());
        println!("cargo:rerun-if-changed={}", launcher_digest_path.display());
        let launcher_digest = std::fs::read_to_string(&launcher_digest_path)
            .expect("build the reviewed Forme macOS sandbox launcher before compiling the host");
        let launcher_digest = launcher_digest.trim();
        assert!(
            launcher_digest.len() == 64
                && launcher_digest
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
            "the generated sandbox-launcher digest is malformed"
        );
        println!("cargo:rustc-env=FORME_SANDBOX_LAUNCHER_SHA256={launcher_digest}");
        let manifest = tauri_build::AppManifest::new().commands(&[
            "project_load",
            "project_compare_and_swap",
            "identity_create",
            "preview_build",
            "target_configure",
            "target_list",
            "target_publish",
            "workspace_dispose",
        ]);
        tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest))
            .expect("the Forme desktop Tauri context should build")
    }
}
