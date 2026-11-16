//! # Multi-Language Safety Profiles & CWE-208 Exclusion Tests
//!
//! Literate tests verifying:
//! 1. CWE-208 exclusion for non-cryptographic packages (`no_secret_inputs` safe subset).
//! 2. CWE-208 floor and proof requirement for cryptographic packages with constant-time claims.
//! 3. Memory safety evaluation (Family 1: CWE-416, 787, 125, 415, 476, 190) across:
//!    - `SafeRust`: static analysis proves zero unsafe / zero FFI reachability.
//!    - `GcManaged`: managed runtimes (TypeScript, Python, Go) enforce lifetime and bounds safety.
//!    - `ManualMemoryChecked`: manual memory management (C, C++, unsafe Rust) requires
//!      discharge via evidence/proofs and cannot be excluded by construction.
//! 4. Backwards compatibility of `PackageAuditContext` and `effective_safety_profile()`.

#![deny(unsafe_code)]

use std::path::Path;

use coding_adventures_ca_assurance_report::{
    build_weakness_matrix, verify_assurance_report, AssuranceReport, AuditEvidence,
    AuditProfile, CapabilitiesSummary, CatalogueRef, CheckError, CheckVerdict,
    ClaimEvidence, LanguageSafetyProfile, MatrixError, PackageAuditContext,
    PropertyEvidence, RungsSummary, WeaknessRegistry, WeaknessRow, WeaknessStatus,
};

fn base_profile() -> AuditProfile {
    AuditProfile::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../ct-compare"))
        .expect("ct-compare profile loads")
}

/// Helper to construct base `AuditEvidence` with step bounds satisfied.
fn base_evidence() -> AuditEvidence {
    AuditEvidence {
        unsafe_exceptions: vec![],
        step_bound_ids: vec!["warrant/m/proof/op/steps-bound".into()],
        step_bound_label: "bounded(8)".into(),
        alloc_ids: vec!["bench/a".into()],
        ct_proof_ids: vec![],
        ct_proof_label: "".into(),
        bench_ids: vec![],
        hw_counters: "not-live".into(),
        oob_ids: vec![],
        oob_label: "".into(),
        no_secret_operand_ids: vec!["warrant-spec::all-operands-public".into()],
        profile: base_profile(),
        all_ops_step_bounded: true,
        ..AuditEvidence::default()
    }
}

/// Helper to construct a base `PackageAuditContext` for non-crypto packages.
fn non_crypto_ctx() -> PackageAuditContext {
    PackageAuditContext {
        package_id: "rust/dsp-synth".into(),
        language_safety: LanguageSafetyProfile::SafeRust,
        is_safe_rust: true,
        has_secret_ops: false,
        constant_time_claims: vec![],
        declared_capabilities: vec![],
        has_wir_timing_proof: false,
        has_instruction_benchmarks: false,
        has_zero_alloc_proof: true,
        has_pbt_coverage: true,
        has_fuzz_coverage: true,
        has_eval_analyzer_evidence: false,
        has_prototype_analyzer_evidence: false,
        evidence: base_evidence(),
    }
}

/// Helper to construct a base `PackageAuditContext` for crypto packages with constant-time obligations.
fn crypto_ctx() -> PackageAuditContext {
    let mut ev = base_evidence();
    ev.ct_proof_ids = vec!["warrant/m/proof/op/constant-time".into()];
    ev.ct_proof_label = "bounded(64)".into();
    ev.bench_ids = vec!["bench/a".into()];
    ev.hw_counters = "pass".into();

    PackageAuditContext {
        package_id: "rust/crypto-ct-compare".into(),
        language_safety: LanguageSafetyProfile::SafeRust,
        is_safe_rust: true,
        has_secret_ops: true,
        constant_time_claims: vec!["claim/ct_u8_slice_eq".into()],
        declared_capabilities: vec![],
        has_wir_timing_proof: true,
        has_instruction_benchmarks: true,
        has_zero_alloc_proof: true,
        has_pbt_coverage: true,
        has_fuzz_coverage: true,
        has_eval_analyzer_evidence: false,
        has_prototype_analyzer_evidence: false,
        evidence: ev,
    }
}

// ---------------------------------------------------------------------------
// 1. CWE-208: Non-Crypto Exclusion vs Crypto Requirement
// ---------------------------------------------------------------------------

#[test]
fn test_cwe208_non_crypto_package_is_excluded() {
    let registry = WeaknessRegistry::embedded();
    let ctx = non_crypto_ctx();

    let rows = build_weakness_matrix(&registry, &ctx).expect("matrix generation must succeed");
    let row_208 = rows.iter().find(|r| r.cwe == 208).expect("CWE-208 row");

    assert_eq!(row_208.status, WeaknessStatus::Excluded);
    assert_eq!(row_208.approved, None);
    assert!(row_208.reason.contains("no secret inputs"));
    assert!(row_208.evidence.contains(&"capability-audit::no-secret-inputs".to_string()));
}

#[test]
fn test_cwe208_crypto_package_requires_timing_proof() {
    let registry = WeaknessRegistry::embedded();
    let ctx = crypto_ctx();

    let rows = build_weakness_matrix(&registry, &ctx).expect("matrix generation must succeed");
    let row_208 = rows.iter().find(|r| r.cwe == 208).expect("CWE-208 row");

    // Crypto packages with constant_time_claims cannot be excluded:
    // They must require proof to gate-level and emit an unapproved exemption at WIR level.
    assert_eq!(row_208.status, WeaknessStatus::Exemption);
    assert_eq!(row_208.approved, Some(false));
    assert!(row_208.reason.contains("floor requires proof to gate-level CPU netlist"));
}

#[test]
fn test_cwe208_crypto_package_fails_closed_without_proofs() {
    let registry = WeaknessRegistry::embedded();
    let mut ctx = crypto_ctx();
    ctx.has_wir_timing_proof = false;
    ctx.has_instruction_benchmarks = false;
    ctx.evidence.ct_proof_ids.clear();

    let err = build_weakness_matrix(&registry, &ctx).expect_err("must fail closed as open row");
    assert!(matches!(err, MatrixError::OpenRow(msg) if msg.contains("CWE-208")));
}

#[test]
fn test_checker_rejects_illegal_cwe208_exclusion_for_crypto_package() {
    let registry = WeaknessRegistry::embedded();

    let report = AssuranceReport {
        package: "rust/crypto-timing-sensitive".into(),
        version: "1.0.0".into(),
        commit: "abcdef0123".into(),
        profile: "test".into(),
        profile_sha256: "00".repeat(32),
        verdict: "pass".into(),
        rungs: RungsSummary {
            e0: "pass".into(),
            e1: "pass".into(),
            e2: "pass".into(),
            e3: "pass".into(),
            e4: "pass".into(),
            e5: "pass".into(),
        },
        capabilities: CapabilitiesSummary {
            declared: vec![],
            analyzer: "pass".into(),
            certificate: "proved".into(),
            new_since_last_release: vec![],
        },
        claims: vec![ClaimEvidence {
            id: "claim/ct_compare".into(),
            symbol: "ct_compare".into(),
            example: "".into(),
            property: PropertyEvidence {
                cases: 100,
                status: "pass".into(),
            },
            exhaustive: None,
            fuzz: None,
            proof: None,
            evidence: vec!["proof/ct_compare/constant-time".into()],
        }],
        weaknesses: registry
            .classes
            .iter()
            .map(|c| {
                if c.cwe == 208 {
                    WeaknessRow {
                        cwe: 208,
                        name: c.name.clone(),
                        status: WeaknessStatus::Excluded, // ILLEGAL EXCLUSION
                        approved: None,
                        reason: "fraudulent no-secret claim".into(),
                        evidence: vec!["capability-audit::no-secret-inputs".into()],
                    }
                } else {
                    WeaknessRow {
                        cwe: c.cwe,
                        name: c.name.clone(),
                        status: WeaknessStatus::Excluded,
                        approved: None,
                        reason: "safe subset".into(),
                        evidence: vec![],
                    }
                }
            })
            .collect(),
        public_symbols_without_claims: vec![],
        exemptions: vec![],
        catalogue: CatalogueRef {
            schema: "warrant-assumption-catalogue-v1".into(),
            path: "code/assumptions/catalogue-v1.json".into(),
            sha256: "00".repeat(32),
            evidence_kinds_sha256: "00".repeat(32),
            non_claims_sha256: "00".repeat(32),
        },
        assumptions: vec![],
        closure: vec![],
    };

    let result = verify_assurance_report(&report, &registry);
    match result {
        Err(CheckError::InvalidExclusion(msg)) => {
            assert!(msg.contains("208"));
            assert!(msg.contains("cannot be excluded"));
        }
        other => panic!("expected CheckError::InvalidExclusion, got {:?}", other),
    }
}

#[test]
fn test_non_crypto_package_reaches_pass_verdict_with_cwe208_excluded() {
    let registry = WeaknessRegistry::embedded();
    let ctx = non_crypto_ctx();
    let matrix = build_weakness_matrix(&registry, &ctx).expect("matrix generation must succeed");

    let report = AssuranceReport {
        package: "rust/dsp-synth".into(),
        version: "0.1.0".into(),
        commit: "abcdef0123".into(),
        profile: "test".into(),
        profile_sha256: "00".repeat(32),
        verdict: "pass".into(),
        rungs: RungsSummary {
            e0: "pass".into(),
            e1: "pass".into(),
            e2: "pass".into(),
            e3: "pass".into(),
            e4: "pass".into(),
            e5: "pass".into(),
        },
        capabilities: CapabilitiesSummary {
            declared: vec![],
            analyzer: "pass".into(),
            certificate: "proved".into(),
            new_since_last_release: vec![],
        },
        claims: vec![ClaimEvidence {
            id: "claim/dsp_filter".into(),
            symbol: "dsp_filter".into(),
            example: "pass".into(),
            property: PropertyEvidence {
                cases: 100,
                status: "pass".into(),
            },
            exhaustive: None,
            fuzz: None,
            proof: None,
            evidence: vec!["test/dsp_filter_pbt".into()],
        }],
        weaknesses: matrix,
        public_symbols_without_claims: vec![],
        exemptions: vec![],
        catalogue: CatalogueRef {
            schema: "warrant-assumption-catalogue-v1".into(),
            path: "code/assumptions/catalogue-v1.json".into(),
            sha256: "00".repeat(32),
            evidence_kinds_sha256: "00".repeat(32),
            non_claims_sha256: "00".repeat(32),
        },
        assumptions: vec![],
        closure: vec![],
    };

    let verdict = verify_assurance_report(&report, &registry).expect("non-crypto report must pass check_gate");
    assert_eq!(verdict, CheckVerdict::Pass);
}

// ---------------------------------------------------------------------------
// 2. Memory Safety: SafeRust vs GcManaged vs ManualMemoryChecked
// ---------------------------------------------------------------------------

#[test]
fn test_safe_rust_memory_safety_evaluation() {
    let registry = WeaknessRegistry::embedded();
    let ctx = PackageAuditContext {
        language_safety: LanguageSafetyProfile::SafeRust,
        is_safe_rust: true,
        ..non_crypto_ctx()
    };

    let rows = build_weakness_matrix(&registry, &ctx).expect("matrix generation must succeed");

    for cwe in [416, 787, 125, 415, 476, 190] {
        let row = rows.iter().find(|r| r.cwe == cwe).expect("row exists");
        assert_eq!(row.status, WeaknessStatus::Excluded, "CWE-{cwe} must be Excluded in SafeRust");
        assert!(row.reason.contains("safe Rust"), "CWE-{cwe} reason: {}", row.reason);
        assert!(row.evidence.contains(&"analyzer::no-unsafe-or-ffi".to_string()));
    }
}

#[test]
fn test_gc_managed_memory_safety_evaluation_typescript() {
    let registry = WeaknessRegistry::embedded();
    let ctx = PackageAuditContext {
        language_safety: LanguageSafetyProfile::GcManaged {
            language: "typescript".into(),
        },
        is_safe_rust: false,
        ..non_crypto_ctx()
    };

    let rows = build_weakness_matrix(&registry, &ctx).expect("matrix generation must succeed");

    for cwe in [416, 787, 125, 415, 476, 190] {
        let row = rows.iter().find(|r| r.cwe == cwe).expect("row exists");
        assert_eq!(row.status, WeaknessStatus::Excluded, "CWE-{cwe} must be Excluded in TypeScript");
        assert!(row.reason.contains("typescript"), "CWE-{cwe} reason: {}", row.reason);
        assert!(row.reason.contains("garbage-collected runtime"), "CWE-{cwe} reason: {}", row.reason);
        assert!(row.evidence.contains(&"runtime-memory-safety::typescript".to_string()));
    }
}

#[test]
fn test_gc_managed_memory_safety_evaluation_python() {
    let registry = WeaknessRegistry::embedded();
    let ctx = PackageAuditContext {
        language_safety: LanguageSafetyProfile::GcManaged {
            language: "python".into(),
        },
        is_safe_rust: false,
        ..non_crypto_ctx()
    };

    let rows = build_weakness_matrix(&registry, &ctx).expect("matrix generation must succeed");

    for cwe in [416, 787, 125, 415, 476, 190] {
        let row = rows.iter().find(|r| r.cwe == cwe).expect("row exists");
        assert_eq!(row.status, WeaknessStatus::Excluded, "CWE-{cwe} must be Excluded in Python");
        assert!(row.reason.contains("python"), "CWE-{cwe} reason: {}", row.reason);
        assert!(row.evidence.contains(&"runtime-memory-safety::python".to_string()));
    }
}

#[test]
fn test_manual_memory_checked_with_bounds_proofs() {
    let registry = WeaknessRegistry::embedded();
    let mut ev = base_evidence();
    ev.oob_ids = vec!["proof/oob_check".into()];
    ev.oob_label = "bounded(128)".into();

    let ctx = PackageAuditContext {
        language_safety: LanguageSafetyProfile::ManualMemoryChecked {
            language: "c".into(),
        },
        is_safe_rust: false,
        evidence: ev,
        ..non_crypto_ctx()
    };

    let rows = build_weakness_matrix(&registry, &ctx).expect("matrix generation must succeed");

    // OOB reads (125) and writes (787) are Discharged on bounds proofs
    for cwe in [125, 787] {
        let row = rows.iter().find(|r| r.cwe == cwe).expect("row exists");
        assert_eq!(row.status, WeaknessStatus::Discharged, "CWE-{cwe} must be Discharged");
        assert!(row.reason.contains("bounds-checked by symbolic execution"));
        assert!(row.evidence.contains(&"proof/oob_check".to_string()));
    }

    // Remaining classes (416, 415, 476, 190) cannot be excluded: they are unapproved exemptions
    for cwe in [416, 415, 476, 190] {
        let row = rows.iter().find(|r| r.cwe == cwe).expect("row exists");
        assert_eq!(row.status, WeaknessStatus::Exemption, "CWE-{cwe} must be Exemption");
        assert_eq!(row.approved, Some(false));
        assert!(row.reason.contains("manual memory management (c)"));
    }
}

#[test]
fn test_manual_memory_checked_without_proofs_is_all_exemptions() {
    let registry = WeaknessRegistry::embedded();
    let ctx = PackageAuditContext {
        language_safety: LanguageSafetyProfile::ManualMemoryChecked {
            language: "cpp".into(),
        },
        is_safe_rust: false,
        evidence: base_evidence(),
        ..non_crypto_ctx()
    };

    let rows = build_weakness_matrix(&registry, &ctx).expect("matrix generation must succeed");

    // With no bounds proofs, all 6 memory safety CWEs must be unapproved exemptions, never excluded
    for cwe in [416, 787, 125, 415, 476, 190] {
        let row = rows.iter().find(|r| r.cwe == cwe).expect("row exists");
        assert_eq!(row.status, WeaknessStatus::Exemption, "CWE-{cwe} must be Exemption");
        assert_eq!(row.approved, Some(false));
        assert!(row.reason.contains("manual memory management (cpp)"));
    }
}

// ---------------------------------------------------------------------------
// 3. Profile helpers & Backwards Compatibility
// ---------------------------------------------------------------------------

#[test]
fn test_profile_helpers_and_effective_profile() {
    let p_safe = LanguageSafetyProfile::SafeRust;
    assert!(p_safe.is_safe_rust());
    assert!(!p_safe.is_gc_managed());
    assert!(!p_safe.is_manual_memory_checked());
    assert_eq!(p_safe.language_name(), "rust");

    let p_gc = LanguageSafetyProfile::GcManaged {
        language: "go".into(),
    };
    assert!(!p_gc.is_safe_rust());
    assert!(p_gc.is_gc_managed());
    assert!(!p_gc.is_manual_memory_checked());
    assert_eq!(p_gc.language_name(), "go");

    let p_manual = LanguageSafetyProfile::ManualMemoryChecked {
        language: "c".into(),
    };
    assert!(!p_manual.is_safe_rust());
    assert!(!p_manual.is_gc_managed());
    assert!(p_manual.is_manual_memory_checked());
    assert_eq!(p_manual.language_name(), "c");

    // Legacy context compatibility:
    // If language_safety is default SafeRust, but legacy is_safe_rust is false,
    // effective_safety_profile() resolves to ManualMemoryChecked("rust").
    let ctx_legacy = PackageAuditContext {
        language_safety: LanguageSafetyProfile::SafeRust,
        is_safe_rust: false,
        ..non_crypto_ctx()
    };
    assert_eq!(
        ctx_legacy.effective_safety_profile(),
        LanguageSafetyProfile::ManualMemoryChecked {
            language: "rust".into()
        }
    );

    // If language_safety is explicitly GcManaged, effective_safety_profile preserves it.
    let ctx_explicit = PackageAuditContext {
        language_safety: LanguageSafetyProfile::GcManaged {
            language: "python".into(),
        },
        is_safe_rust: false,
        ..non_crypto_ctx()
    };
    assert_eq!(
        ctx_explicit.effective_safety_profile(),
        LanguageSafetyProfile::GcManaged {
            language: "python".into()
        }
    );
}

// ---------------------------------------------------------------------------
// 4. Fail-closed security reviews tests
// ---------------------------------------------------------------------------

#[test]
fn test_cwe208_fails_closed_when_all_operands_public_missing() {
    let registry = WeaknessRegistry::embedded();
    let mut ctx = non_crypto_ctx();
    // Clear all-operands-public attestation
    ctx.evidence.no_secret_operand_ids.clear();

    let err = build_weakness_matrix(&registry, &ctx).expect_err("must fail closed as OpenRow");
    assert!(matches!(err, MatrixError::OpenRow(msg) if msg.contains("CWE-208")));
}

#[test]
fn test_cwe208_fails_closed_when_has_secret_ops() {
    let registry = WeaknessRegistry::embedded();
    let mut ctx = non_crypto_ctx();
    // Even if no_secret_operand_ids is present, has_secret_ops must block exclusion
    ctx.has_secret_ops = true;

    let err = build_weakness_matrix(&registry, &ctx).expect_err("must fail closed as OpenRow");
    assert!(matches!(err, MatrixError::OpenRow(msg) if msg.contains("CWE-208")));
}

#[test]
fn test_gc_managed_fails_closed_on_family_1_when_unsafe_or_ffi_present() {
    let registry = WeaknessRegistry::embedded();
    let mut ev = base_evidence();
    ev.unsafe_exceptions = vec!["c-ffi-binding".into()];
    ev.oob_ids = vec!["proof/oob_bounds".into()];
    ev.oob_label = "bounded(64)".into();

    let ctx = PackageAuditContext {
        language_safety: LanguageSafetyProfile::GcManaged {
            language: "typescript".into(),
        },
        is_safe_rust: false,
        evidence: ev,
        ..non_crypto_ctx()
    };

    let rows = build_weakness_matrix(&registry, &ctx).expect("matrix generation must succeed");

    // With FFI/unsafe present, Family 1 is NOT excluded!
    for cwe in [416, 415, 476, 190] {
        let row = rows.iter().find(|r| r.cwe == cwe).expect("row exists");
        assert_eq!(row.status, WeaknessStatus::Exemption, "CWE-{cwe} must be Exemption");
        assert_eq!(row.approved, Some(false));
        assert!(row.reason.contains("unsafe/FFI exceptions"));
    }
    // 125 and 787 can be discharged on bounds proofs
    for cwe in [125, 787] {
        let row = rows.iter().find(|r| r.cwe == cwe).expect("row exists");
        assert_eq!(row.status, WeaknessStatus::Discharged, "CWE-{cwe} must be Discharged");
        assert!(row.reason.contains("bounds-checked by symbolic execution"));
    }
}

#[test]
fn test_gc_managed_cwe94_and_cwe1321_fail_closed_for_typescript() {
    let registry = WeaknessRegistry::embedded();

    // Case A: TypeScript without analyzer evidence -> both CWE-94 and CWE-1321 must be unapproved Exemptions
    let ctx_unproven = PackageAuditContext {
        language_safety: LanguageSafetyProfile::GcManaged {
            language: "typescript".into(),
        },
        is_safe_rust: false,
        has_eval_analyzer_evidence: false,
        has_prototype_analyzer_evidence: false,
        ..non_crypto_ctx()
    };

    let rows_unproven = build_weakness_matrix(&registry, &ctx_unproven).expect("matrix generation succeeds");
    let row_94 = rows_unproven.iter().find(|r| r.cwe == 94).expect("CWE-94 row");
    assert_eq!(row_94.status, WeaknessStatus::Exemption, "CWE-94 without evidence must not be Excluded");
    assert_eq!(row_94.approved, Some(false));

    let row_1321 = rows_unproven.iter().find(|r| r.cwe == 1321).expect("CWE-1321 row");
    assert_eq!(row_1321.status, WeaknessStatus::Exemption, "CWE-1321 without evidence must not be Excluded");
    assert_eq!(row_1321.approved, Some(false));

    // Case B: TypeScript with analyzer evidence -> both CWE-94 and CWE-1321 are Discharged
    let ctx_proven = PackageAuditContext {
        language_safety: LanguageSafetyProfile::GcManaged {
            language: "typescript".into(),
        },
        is_safe_rust: false,
        has_eval_analyzer_evidence: true,
        has_prototype_analyzer_evidence: true,
        ..non_crypto_ctx()
    };

    let rows_proven = build_weakness_matrix(&registry, &ctx_proven).expect("matrix generation succeeds");
    let row_94_proven = rows_proven.iter().find(|r| r.cwe == 94).expect("CWE-94 row");
    assert_eq!(row_94_proven.status, WeaknessStatus::Discharged);
    assert!(row_94_proven.evidence.contains(&"analyzer::no-dynamic-eval".to_string()));

    let row_1321_proven = rows_proven.iter().find(|r| r.cwe == 1321).expect("CWE-1321 row");
    assert_eq!(row_1321_proven.status, WeaknessStatus::Discharged);
    assert!(row_1321_proven.evidence.contains(&"analyzer::no-prototype-pollution".to_string()));
}

#[test]
fn test_gc_managed_cwe94_fails_closed_for_python_and_cwe1321_is_excluded() {
    let registry = WeaknessRegistry::embedded();

    // Python without eval analyzer evidence: CWE-94 is Exemption, but CWE-1321 is Excluded (no JS prototype chain)
    let ctx_py = PackageAuditContext {
        language_safety: LanguageSafetyProfile::GcManaged {
            language: "python".into(),
        },
        is_safe_rust: false,
        has_eval_analyzer_evidence: false,
        ..non_crypto_ctx()
    };

    let rows = build_weakness_matrix(&registry, &ctx_py).expect("matrix generation succeeds");
    let row_94 = rows.iter().find(|r| r.cwe == 94).expect("CWE-94 row");
    assert_eq!(row_94.status, WeaknessStatus::Exemption);
    assert_eq!(row_94.approved, Some(false));

    let row_1321 = rows.iter().find(|r| r.cwe == 1321).expect("CWE-1321 row");
    assert_eq!(row_1321.status, WeaknessStatus::Excluded);
    assert!(row_1321.reason.contains("no object prototypes"));

    // Python with eval analyzer evidence: CWE-94 is Discharged
    let ctx_py_proven = PackageAuditContext {
        language_safety: LanguageSafetyProfile::GcManaged {
            language: "python".into(),
        },
        is_safe_rust: false,
        has_eval_analyzer_evidence: true,
        ..non_crypto_ctx()
    };
    let rows_proven = build_weakness_matrix(&registry, &ctx_py_proven).expect("matrix generation succeeds");
    let row_94_p = rows_proven.iter().find(|r| r.cwe == 94).expect("CWE-94 row");
    assert_eq!(row_94_p.status, WeaknessStatus::Discharged);
}

#[test]
fn test_manual_memory_checked_evidence_cleanliness() {
    let registry = WeaknessRegistry::embedded();

    // Pure C code without effect-certificate unsafe exceptions
    let ctx = PackageAuditContext {
        language_safety: LanguageSafetyProfile::ManualMemoryChecked {
            language: "c".into(),
        },
        is_safe_rust: false,
        evidence: base_evidence(), // no unsafe_exceptions
        ..non_crypto_ctx()
    };

    let rows = build_weakness_matrix(&registry, &ctx).expect("matrix generation succeeds");
    let row_416 = rows.iter().find(|r| r.cwe == 416).unwrap();
    // Evidence must NOT cite effect-certificate::unsafe-exception for pure C
    assert!(!row_416.evidence.contains(&"effect-certificate::unsafe-exception".to_string()));
}

#[test]
fn test_package_audit_context_default_is_fail_closed() {
    let default_ctx = PackageAuditContext::default();
    assert!(!default_ctx.is_safe_rust);
    assert!(matches!(
        default_ctx.language_safety,
        LanguageSafetyProfile::ManualMemoryChecked { ref language } if language == "unknown"
    ));
}
