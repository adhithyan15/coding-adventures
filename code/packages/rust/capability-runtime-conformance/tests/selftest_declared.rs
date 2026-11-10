//! Live self-test 3: *declared* capabilities widen the sandbox by exactly what
//! the manifest says, no more.
//!
//! * `time:read` allows a raw `clock_gettime` syscall (a pure package would be
//!   killed for it: see the `time-syscall` negative control).
//! * `fs:read /etc/passwd` allows opening that file only. seccomp permits
//!   `openat`; **Landlock** is what refuses `/etc/group` (EACCES), proving the
//!   path scoping works for real, and the observer sees no violation because
//!   `openat` is declared.

#[cfg(target_os = "linux")]
mod live {
    use capability_runtime_conformance::sabotage;
    use capability_runtime_conformance::{
        run_package_gate, Check, Exercise, GateConfig, PolicyOptions, Rings,
    };
    use std::io::Read;
    use std::path::PathBuf;

    const MANIFEST: &str = r#"{
      "version": 1,
      "package": "rust/rcg-selftest-declared",
      "capabilities": [
        {"category": "time", "action": "read", "target": "*",
         "justification": "Self-test reads the clock with a raw syscall."},
        {"category": "fs", "action": "read", "target": "/etc/passwd",
         "justification": "Self-test reads exactly one file to prove Landlock scoping."}
      ],
      "justification": "Synthetic package declaring time and one readable file."
    }"#;

    fn exercise() -> Exercise {
        Box::new(|| {
            // Declared: allowed.
            sabotage::attempt("time-syscall", "selftest");
            let mut text = String::new();
            let allowed = std::fs::File::open("/etc/passwd")
                .and_then(|mut f| f.read_to_string(&mut text))
                .is_ok()
                && text.contains("root");
            // Not declared: Landlock refuses although seccomp allows openat.
            let denied = matches!(
                std::fs::File::open("/etc/group"),
                Err(ref e) if e.kind() == std::io::ErrorKind::PermissionDenied
            );
            vec![
                Check {
                    name: "declared file is readable".into(),
                    ok: allowed,
                },
                Check {
                    name: "undeclared file is refused by Landlock".into(),
                    ok: denied,
                },
            ]
        })
    }

    pub fn run() {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("selftest-declared");
        let config = GateConfig {
            package: "rust/rcg-selftest-declared",
            manifest_json: MANIFEST,
            evidence_dir: Some(dir.clone()),
            prepare: || exercise(),
            options: PolicyOptions::default(),
            rings: Rings::MAIN_ONLY,
        };
        let code = run_package_gate(&config);
        let evidence = std::fs::read_to_string(dir.join("capability_runtime_evidence.json"))
            .expect("evidence file written");
        if evidence.contains("\"overall\": \"not-live\"") {
            return;
        }
        assert_eq!(
            code, 0,
            "declared capabilities must be honoured: {evidence}"
        );
        assert!(evidence.contains("\"overall\": \"pass\""), "{evidence}");
        assert!(evidence.contains("fs:read:/etc/passwd"));
        assert!(
            evidence.contains("\"clock_gettime\""),
            "the declared call is observed"
        );
        assert!(evidence.contains("\"openat\""));
        // Landlock must actually have been installed for the scoping claim.
        assert!(
            evidence.contains("\"landlock_enforced\": true"),
            "Landlock unavailable on this kernel; scoping unproven: {evidence}"
        );
    }
}

fn main() {
    #[cfg(target_os = "linux")]
    live::run();
    #[cfg(not(target_os = "linux"))]
    println!("not-live: runtime capability gate is Linux-only (RCG010)");
}
