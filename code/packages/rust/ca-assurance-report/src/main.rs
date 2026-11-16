//! # Assurance Report CLI (VER01 / VER15 / VER-P028)
//!
//! Evaluates package weakness matrix, generates `assurance_report.json`,
//! and verifies assurance reports fail-closed.

#![deny(unsafe_code)]

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::exit;

use coding_adventures_ca_assurance_report::{
    check_assumptions, generate_package_report, verify_assurance_report, verify_evidence, verify_profile,
    verify_rederivation, AssuranceReport, CheckVerdict, ProfileKind, Evidence, WeaknessRegistry,
};

/// `rust/<directory name>` and the `version = "..."` of the package's
/// `Cargo.toml`: the report names the package it was generated for, it does not
/// carry a hardcoded identity.
fn package_identity(dir: &std::path::Path) -> Result<(String, String), String> {
    let canon = dir.canonicalize().map_err(|e| format!("{}: {e}", dir.display()))?;
    let name = canon
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("package directory has no name")?
        .to_string();
    // Same size cap as every other input; the version comes from the
    // `[package]` table only.
    let cargo = coding_adventures_ca_assurance_report::evidence::read_capped(
        &canon.join("Cargo.toml"),
        coding_adventures_ca_assurance_report::evidence::MAX_MANIFEST_BYTES,
    )
    .map_err(|e| format!("{}/Cargo.toml: {}", canon.display(), e.0))?;
    let cargo = String::from_utf8(cargo).map_err(|_| format!("{}/Cargo.toml is not UTF-8", canon.display()))?;
    let version = coding_adventures_ca_assurance_report::evidence::package_version(&cargo).map_err(|e| e.0)?;
    Ok((format!("rust/{name}"), version))
}

fn print_usage() {
    eprintln!("Usage: ca-assurance-report-rs [OPTIONS]");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --package-dir <DIR>    Package directory holding evidence/ (default: the report's directory, or .)");
    eprintln!("  --generate             Generate and emit assurance_report.json");
    eprintln!("  --profile <NAME>       Optional pin: the `name` the package's assurance-profile.json must declare.");
    eprintln!("  --profile-kind <K>     Optional pin: the `kind` (transform | decompressor) the profile file must declare;");
    eprintln!("                         a BUILD line pins it so the file's author cannot flip it and regenerate");
    eprintln!("                         The profile itself is data: <package-dir>/assurance-profile.json (VER04h section 9");
    eprintln!("                         item 22), hash-bound into the report as profile_sha256; there is no built-in profile");
    eprintln!("  --check <FILE>         Verify an existing assurance_report.json fail-closed");
    eprintln!("  --allow-unverified     Accept `unverified::` evidence ids (static attestations no tool checks);");
    eprintln!("                         without it --check fails on any such id");
    eprintln!("                         (temporary; tracked by backlog item WD-008; every accepted id is printed)");
    eprintln!("  --strict               Fail on VER13 rule AC-10 (a catalogue entry naming the package that");
    eprintln!("                         no claim uses) instead of printing a warning");
    eprintln!();
    eprintln!("The VER13 assumption catalogue (code/assumptions/) is found above the package directory;");
    eprintln!("--check fails closed when the report's catalogue pin differs from it (AC-14).");
    eprintln!("  --registry <FILE>      Path to weakness-registry-v1.json");
    eprintln!("  --commit <SHA>         Commit SHA to record in generated report");
    eprintln!("  --help                 Display this help message");
}

/// Read the report with the same size cap as the manifests.
fn read_report(path: &std::path::Path) -> Result<String, String> {
    let bytes = coding_adventures_ca_assurance_report::evidence::read_capped(
        path,
        coding_adventures_ca_assurance_report::evidence::MAX_MANIFEST_BYTES,
    )
    .map_err(|e| e.0)?;
    String::from_utf8(bytes).map_err(|_| "report is not UTF-8".to_string())
}

fn find_default_registry() -> WeaknessRegistry {
    // Try local relative path to specs
    let candidates = [
        PathBuf::from("code/specs/data/weakness-registry-v1.json"),
        PathBuf::from("../../../specs/data/weakness-registry-v1.json"),
        PathBuf::from("../../../../specs/data/weakness-registry-v1.json"),
        PathBuf::from("../../specs/data/weakness-registry-v1.json"),
    ];

    for path in &candidates {
        if path.is_file() {
            if let Ok(reg) = WeaknessRegistry::load_from_file(path) {
                return reg;
            }
        }
    }

    // Fall back to embedded snapshot
    WeaknessRegistry::embedded()
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut package_dir: Option<PathBuf> = None;
    let mut check_file: Option<PathBuf> = None;
    let mut generate = false;
    let mut allow_unverified = false;
    let mut strict = false;
    let mut registry_file: Option<PathBuf> = None;
    let mut commit_sha = "HEAD".to_string();
    // Set only by an explicit `--profile`: the profile file's name must equal it.
    let mut profile_requested: Option<String> = None;
    let mut profile_kind_requested: Option<ProfileKind> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--package-dir" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Error: --package-dir requires an argument");
                    exit(2);
                }
                package_dir = Some(PathBuf::from(&args[i]));
            }
            "--check" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Error: --check requires an argument");
                    exit(2);
                }
                check_file = Some(PathBuf::from(&args[i]));
            }
            "--generate" => {
                generate = true;
            }
            "--allow-unverified" => {
                allow_unverified = true;
            }
            "--strict" => {
                strict = true;
            }
            "--profile" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Error: --profile requires an argument");
                    exit(2);
                }
                profile_requested = Some(args[i].clone());
            }
            "--profile-kind" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Error: --profile-kind requires an argument");
                    exit(2);
                }
                profile_kind_requested = match args[i].parse() {
                    Ok(k) => Some(k),
                    Err(e) => {
                        eprintln!("Error: {e}");
                        exit(2);
                    }
                };
            }
            "--registry" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Error: --registry requires an argument");
                    exit(2);
                }
                registry_file = Some(PathBuf::from(&args[i]));
            }
            "--commit" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Error: --commit requires an argument");
                    exit(2);
                }
                commit_sha = args[i].clone();
            }
            "--help" | "-h" => {
                print_usage();
                exit(0);
            }
            other => {
                eprintln!("Error: unknown option '{other}'");
                print_usage();
                exit(2);
            }
        }
        i += 1;
    }

    let registry = if let Some(path) = registry_file {
        match WeaknessRegistry::load_from_file(&path) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Error loading registry from {}: {e}", path.display());
                exit(2);
            }
        }
    } else {
        find_default_registry()
    };

    if let Some(file_path) = check_file {
        let content = match read_report(&file_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error reading report file {}: {e}", file_path.display());
                exit(2);
            }
        };

        let report = match AssuranceReport::from_json_str(&content) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Error parsing report JSON: {e}");
                exit(1);
            }
        };

        // Cross-check the report against the evidence manifests (VER04h §9).
        // The package directory is the report's own directory unless given.
        let dir = package_dir.clone().unwrap_or_else(|| {
            match file_path.parent() {
                Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
                _ => PathBuf::from("."),
            }
        });
        let evidence = match Evidence::load(&dir) {
            Ok(ev) => ev,
            Err(e) => {
                eprintln!("ASSURANCE VERIFICATION FAILED: {e}");
                exit(1);
            }
        };
        if let Err(e) = verify_profile(&report, profile_requested.as_deref()) {
            eprintln!("ASSURANCE VERIFICATION FAILED: {e}");
            exit(1);
        }
        // The pin is also held to the file itself (name and kind): BUILD may only assert which
        // profile it means, and what kind of package it is.
        if let Err(e) = evidence.profile.verify_pin(profile_requested.as_deref(), profile_kind_requested) {
            eprintln!("ASSURANCE VERIFICATION FAILED: {e}");
            exit(1);
        }
        match verify_evidence(&report, &evidence) {
            Ok(unverified) => {
                if !unverified.is_empty() {
                    // The honest default is to refuse: an `unverified::` id is a
                    // sentence somebody wrote, not evidence any tool produced.
                    if !allow_unverified {
                        eprintln!(
                            "ASSURANCE VERIFICATION FAILED: {} evidence id(s) are static attestations no tool checks \
                             (pass --allow-unverified to accept them): {}",
                            unverified.len(),
                            unverified.join(", ")
                        );
                        exit(1);
                    }
                    // Name every accepted id: a bare count hides which sentences
                    // were taken on trust.  The flag is temporary (backlog WD-008).
                    println!(
                        "note: {} evidence id(s) are static attestations no tool checks \
                         (accepted by --allow-unverified, a temporary flag tracked by backlog item WD-008):",
                        unverified.len()
                    );
                    for id in &unverified {
                        println!("  accepted unverified: {id}");
                    }
                }
            }
            Err(e) => {
                eprintln!("ASSURANCE VERIFICATION FAILED: {e}");
                exit(1);
            }
        }

        // VER13 AC-10: entries naming this package that nothing in the report
        // uses are flagged for removal (VER00 section 7.5.2): a warning by
        // default, an error under --strict.  (verify_evidence already ran
        // every other assumption rule.)
        // AR-001: the matrix (and so every reason sentence, including the ones the profile
        // supplies) is re-derived from the evidence and the bound profile.
        if let Err(e) = verify_rederivation(&report, &evidence, &registry) {
            eprintln!("ASSURANCE VERIFICATION FAILED: {e}");
            exit(1);
        }

        match check_assumptions(&report, &evidence, strict) {
            Ok(summary) => {
                for id in &summary.unused {
                    println!("warning: AC-10 UnusedAssumption: {id} names {} but matches no claim", report.package);
                }
            }
            Err(e) => {
                eprintln!("ASSURANCE VERIFICATION FAILED: {e}");
                exit(1);
            }
        }

        match verify_assurance_report(&report, &registry) {
            Ok(verdict) => {
                match verdict {
                    CheckVerdict::Pass => {
                        println!("ASSURANCE VERDICT: PASS [Package {}]", report.package);
                    }
                    CheckVerdict::ReportOnly(reason) => {
                        println!(
                            "ASSURANCE VERDICT: REPORT-ONLY [Package {}] ({})",
                            report.package, reason
                        );
                    }
                }
                exit(0);
            }
            Err(e) => {
                eprintln!("ASSURANCE VERIFICATION FAILED: {e}");
                exit(1);
            }
        }
    }

    if generate {
        let package_dir = package_dir.unwrap_or_else(|| PathBuf::from("."));
        let target_file = package_dir.join("assurance_report.json");
        let evidence = match Evidence::load(&package_dir) {
            Ok(ev) => ev,
            Err(e) => {
                eprintln!("Error: {e}");
                exit(1);
            }
        };
        if let Err(e) = evidence.profile.verify_pin(profile_requested.as_deref(), profile_kind_requested) {
            eprintln!("Error: {e}");
            exit(2);
        }
        let (package_id, version) = match package_identity(&package_dir) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Error: {e}");
                exit(1);
            }
        };
        let report = match generate_package_report(
            &package_id,
            &version,
            &commit_sha,
            &registry,
            &evidence,
        ) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Error generating report: {e}");
                exit(1);
            }
        };

        // Self-check before writing: shape, then evidence cross-check.
        if let Err(e) = verify_assurance_report(&report, &registry) {
            eprintln!("Generated report failed internal verification: {e}");
            exit(1);
        }
        if let Err(e) = verify_evidence(&report, &evidence) {
            eprintln!("Generated report failed evidence verification: {e}");
            exit(1);
        }
        if let Err(e) = verify_rederivation(&report, &evidence, &registry) {
            eprintln!("Generated report failed re-derivation: {e}");
            exit(1);
        }

        let json_str = report.to_json_string();
        if let Err(e) = fs::write(&target_file, format!("{json_str}\n")) {
            eprintln!("Error writing {}: {e}", target_file.display());
            exit(2);
        }

        println!("Generated assurance report at {}", target_file.display());
        println!("Verdict: {}", report.verdict);
        exit(0);
    }

    print_usage();
    exit(2);
}
