use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("adj-lang-cli must remain below code/packages/rust")
        .to_path_buf()
}

fn assert_object_keys_are_sorted(value: &serde_json::Value) {
    match value {
        serde_json::Value::Array(items) => {
            for item in items {
                assert_object_keys_are_sorted(item);
            }
        }
        serde_json::Value::Object(items) => {
            let keys: Vec<_> = items.keys().collect();
            let mut sorted = keys.clone();
            sorted.sort_unstable();
            assert_eq!(keys, sorted, "JSON object keys must be lexically sorted");
            for item in items.values() {
                assert_object_keys_are_sorted(item);
            }
        }
        _ => {}
    }
}

#[test]
fn formula_audit_is_canonical_and_preserves_nested_formula_identity() {
    let root = repo_root();
    let program = "code/specs/data/adj-formula-stdlib/arithmetic/ratio.query.adj";
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_adj-formula-audit"))
            .current_dir(&root)
            .arg(program)
            .output()
            .expect("run formula audit")
    };

    let first = run();
    let second = run();
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert_eq!(
        first.stdout, second.stdout,
        "canonical output must be byte-stable"
    );
    assert!(first.stdout.ends_with(b"\n"));

    let audit: serde_json::Value = serde_json::from_slice(&first.stdout).expect("audit JSON");
    assert_object_keys_are_sorted(&audit);
    assert_eq!(audit["contract"], "adj-lang/formula_audit/v1");
    assert_eq!(audit["schema_version"], 1);
    assert!(audit.get("executions").is_none());
    let derivation = &audit["derivations"][0];
    assert_eq!(derivation["export"]["name"], "ratio");
    assert_eq!(derivation["formula_sequence"][0]["name"], "ratio");
    assert_eq!(derivation["formula_sequence"][1]["name"], "quotient");
    assert_eq!(derivation["result"]["f64_bits"], "3fe8000000000000");
    assert_eq!(derivation["result"]["exact_rational"]["numerator"], "3");
    assert_eq!(derivation["result"]["exact_rational"]["denominator"], "4");
    assert_eq!(
        derivation["verification"]["computation"]["status"],
        "rechecked"
    );
    assert_eq!(audit["imports"].as_array().expect("imports").len(), 2);
}

#[test]
fn formula_audit_rejects_duplicate_export_names_before_using_runtime_mapping() {
    let directory = std::env::temp_dir().join(format!(
        "adj_formula_audit_duplicate_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("create temporary source directory");
    let program = directory.join("duplicate.adj");
    fs::write(
        &program,
        "formulabook first {\n\
             formula answer(x) = x source \"first definition\"\n\
         }\n\
         formulabook second {\n\
             formula answer(x) = x source \"second definition\"\n\
         }\n\
         observe x(2)\n\
         ? answer(x)\n",
    )
    .expect("write duplicate program");

    let output = Command::new(env!("CARGO_BIN_EXE_adj-formula-audit"))
        .arg(&program)
        .output()
        .expect("run formula audit");
    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("duplicate formula export name: answer")
    );

    fs::remove_dir_all(directory).expect("remove temporary source directory");
}

#[test]
fn formula_audit_v2_records_failed_guard_and_withheld_body() {
    let directory = std::env::temp_dir().join(format!(
        "adj_formula_audit_abstention_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("create temporary source directory");
    let program = directory.join("abstention.adj");
    fs::write(
        &program,
        "formulabook guarded {\n\
             formula quotient(a, b) = a / b\n\
               requires nonzero(b)\n\
               source \"division domain\" trust authoritative\n\
         }\n\
         observe numerator(8)\n\
         observe denominator(0)\n\
         ? quotient(numerator, denominator)\n",
    )
    .expect("write guarded program");

    let output = Command::new(env!("CARGO_BIN_EXE_adj-formula-audit"))
        .arg(&program)
        .output()
        .expect("run formula audit");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let audit: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("negative v2 audit JSON");
    assert_eq!(audit["contract"], "adj-lang/formula_audit/v2");
    assert_eq!(audit["schema_version"], 2);
    assert!(audit.get("derivations").is_none());
    let execution = &audit["executions"][0];
    let guard = &execution["guards"][0];
    assert_eq!(guard["outcome"], "failed");
    assert_eq!(guard["precondition"]["index"], 0);
    assert_eq!(guard["precondition"]["predicate"], "nonzero");
    assert_eq!(guard["comparison"]["operator"], "not_equal");
    assert_eq!(
        guard["comparison"]["observed"]["exact_rational"],
        serde_json::json!({"denominator":"1", "numerator":"0"})
    );
    assert_eq!(guard["inputs"][0]["term"], "denominator(0)");
    assert_eq!(guard["verification"]["passed"], true);
    assert_eq!(guard["verification"]["fully_verified"], false);
    assert!(guard["inputs"][0].get("fact_id").is_none());
    assert!(guard["precondition"]["declaration"]["sha256"]
        .as_str()
        .is_some_and(|hash| hash.len() == 64));
    assert_eq!(execution["body"]["status"], "withheld");
    assert_eq!(execution["body"]["reason"], "precondition_failed");
    assert!(execution["body"].get("derivation").is_none());
    assert!(execution["body"].get("result").is_none());
    assert!(execution["body"].get("tree").is_none());

    fs::remove_dir_all(directory).expect("remove temporary source directory");
}

#[test]
fn formula_audit_v2_stops_source_replay_at_outer_guard_failure() {
    let directory = std::env::temp_dir().join(format!(
        "adj_formula_audit_outer_short_circuit_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("create temporary source directory");
    let program = directory.join("outer_short_circuit.adj");
    fs::write(
        &program,
        "formulabook guarded {\n\
             formula inner(y) = y requires nonzero(y)\n\
               source \"inner domain\" trust authoritative\n\
             formula outer(x) = inner(x + 1) requires nonzero(x)\n\
               source \"outer domain\" trust authoritative\n\
         }\n\
         observe value(0)\n\
         ? outer(value)\n",
    )
    .expect("write outer short-circuit program");

    let output = Command::new(env!("CARGO_BIN_EXE_adj-formula-audit"))
        .arg(&program)
        .output()
        .expect("run formula audit");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let audit: serde_json::Value = serde_json::from_slice(&output.stdout).expect("v2 audit JSON");
    let execution = &audit["executions"][0];
    assert_eq!(execution["formula_sequence"].as_array().unwrap().len(), 1);
    assert_eq!(execution["formula_sequence"][0]["name"], "outer");
    assert_eq!(execution["guards"].as_array().unwrap().len(), 1);
    assert_eq!(execution["guards"][0]["outcome"], "failed");
    assert_eq!(execution["body"]["status"], "withheld");

    fs::remove_dir_all(directory).expect("remove temporary source directory");
}

/// Built-in precedence itself — a formula body that calls `round_to` gets the
/// RUNTIME's rounding, and the audit's replayed formula sequence says so by
/// naming only the user formula that really ran.
///
/// This used to be asserted with a `formulabook` that also declared its own
/// `round_to(a, b) … requires nonzero(a)`, to pin down which one won. That
/// program no longer compiles (`LowerError::ReservedFormulaName` — see the
/// companion test below), because a declared guard that can never run is a hole
/// in the precondition contract rather than a precedence question. The precedence
/// property is real and still worth pinning, so it is asserted here without the
/// unreachable shadow definition.
#[test]
fn formula_audit_v2_preserves_builtin_precedence() {
    let directory = std::env::temp_dir().join(format!(
        "adj_formula_audit_builtin_precedence_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("create temporary source directory");
    let program = directory.join("builtin_precedence.adj");
    fs::write(
        &program,
        "formulabook guarded {\n\
             formula outer(x) = round_to(x, 0) requires nonzero(x)\n\
               source \"outer domain\" trust authoritative\n\
         }\n\
         observe value(2.4)\n\
         ? outer(value)\n",
    )
    .expect("write built-in precedence program");

    let output = Command::new(env!("CARGO_BIN_EXE_adj-formula-audit"))
        .arg(&program)
        .output()
        .expect("run formula audit");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let audit: serde_json::Value = serde_json::from_slice(&output.stdout).expect("v2 audit JSON");
    let execution = &audit["executions"][0];
    // `round_to` is answered by the runtime, so it never enters the replayed
    // formula sequence — only `outer` did.
    assert_eq!(execution["formula_sequence"].as_array().unwrap().len(), 1);
    assert_eq!(execution["formula_sequence"][0]["name"], "outer");
    assert_eq!(execution["body"]["status"], "evaluated");

    fs::remove_dir_all(directory).expect("remove temporary source directory");
}

/// The collision the audit no longer has to reason about. A `formulabook` that
/// declares a built-in's name is rejected at compile time, so the audit binary
/// fails loudly instead of emitting a witness for an execution in which
/// `requires nonzero(a)` was silently never evaluated.
#[test]
fn formula_audit_rejects_a_formulabook_that_shadows_a_builtin() {
    let directory = std::env::temp_dir().join(format!(
        "adj_formula_audit_builtin_collision_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("create temporary source directory");
    let program = directory.join("builtin_collision.adj");
    fs::write(
        &program,
        "formulabook guarded {\n\
             formula round_to(a, b) = a + b requires nonzero(a)\n\
               source \"shadowed export\" trust authoritative\n\
             formula outer(x) = round_to(x, 0) requires nonzero(x)\n\
               source \"outer domain\" trust authoritative\n\
         }\n\
         observe value(2.4)\n\
         ? outer(value)\n",
    )
    .expect("write built-in collision program");

    let output = Command::new(env!("CARGO_BIN_EXE_adj-formula-audit"))
        .arg(&program)
        .output()
        .expect("run formula audit");
    assert!(
        !output.status.success(),
        "a formula book shadowing a built-in must not produce an audit"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("ReservedFormulaName"),
        "expected a reserved-name rejection, got: {stderr}"
    );

    fs::remove_dir_all(directory).expect("remove temporary source directory");
}

#[test]
fn formula_audit_v2_records_passing_guard_and_evaluated_body_canonically() {
    let directory = std::env::temp_dir().join(format!(
        "adj_formula_audit_passing_guard_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&directory);
    let snapshots = directory.join("snapshots");
    fs::create_dir_all(&snapshots).expect("create temporary snapshot directory");
    let formula_bytes = b"Division uses a nonzero denominator.";
    let formula_snapshot = coding_adventures_sha256::sha256_hex(formula_bytes);
    fs::write(snapshots.join(&formula_snapshot), formula_bytes).expect("write formula snapshot");
    let input_bytes = b"numerator is 8. denominator is 2.";
    let input_snapshot = coding_adventures_sha256::sha256_hex(input_bytes);
    fs::write(snapshots.join(&input_snapshot), input_bytes).expect("write input snapshot");
    let program = directory.join("passing_guard.adj");
    fs::write(
        &program,
        format!(
            "formulabook guarded {{\n\
                 formula quotient(a, b) = a / b\n\
                   requires nonzero(b)\n\
                   quote \"Division uses a nonzero denominator.\" at 0 snapshot \"{formula_snapshot}\"\n\
                   source \"Division uses a nonzero denominator.\"\n\
                   locator \"https://example.test/division\" trust authoritative\n\
             }}\n\
             observe numerator(8)\n\
               quote \"numerator is 8.\" at 0 snapshot \"{input_snapshot}\"\n\
               source \"input fixture\" locator \"https://example.test/input\" trust authoritative\n\
             observe denominator(2)\n\
               quote \"denominator is 2.\" at 16 snapshot \"{input_snapshot}\"\n\
               source \"input fixture\" locator \"https://example.test/input\" trust authoritative\n\
             ? quotient(numerator, denominator)\n"
        ),
    )
    .expect("write guarded program");

    let run = || {
        Command::new(env!("CARGO_BIN_EXE_adj-formula-audit"))
            .arg("--snapshots")
            .arg(&snapshots)
            .arg(&program)
            .output()
            .expect("run formula audit")
    };
    let first = run();
    let second = run();
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert_eq!(first.stdout, second.stdout, "v2 output must be canonical");
    let audit: serde_json::Value =
        serde_json::from_slice(&first.stdout).expect("positive v2 audit JSON");
    assert_object_keys_are_sorted(&audit);
    assert_eq!(audit["contract"], "adj-lang/formula_audit/v2");
    let execution = &audit["executions"][0];
    assert_eq!(execution["guards"][0]["outcome"], "passed");
    assert_eq!(execution["guards"][0]["precondition"]["parameter"], "b");
    assert_eq!(
        execution["guards"][0]["plan"]["expression"],
        serde_json::json!({"kind":"reference", "name":"denominator"})
    );
    assert_eq!(
        execution["guards"][0]["comparison"]["observed"]["exact_rational"],
        serde_json::json!({"denominator":"1", "numerator":"2"})
    );
    assert_eq!(execution["guards"][0]["verification"]["passed"], true);
    assert_eq!(
        execution["guards"][0]["verification"]["fully_verified"],
        true
    );
    assert_eq!(execution["body"]["status"], "evaluated");
    assert_eq!(
        execution["body"]["derivation"]["result"]["exact_rational"],
        serde_json::json!({"denominator":"1", "numerator":"4"})
    );

    fs::write(
        snapshots.join(&formula_snapshot),
        b"incorrect formula bytes",
    )
    .expect("replace formula snapshot with mismatched bytes");
    let mismatched = run();
    assert!(mismatched.status.success());
    let mismatch_audit: serde_json::Value =
        serde_json::from_slice(&mismatched.stdout).expect("mismatch audit JSON");
    let verification = &mismatch_audit["executions"][0]["guards"][0]["verification"];
    assert_eq!(verification["fully_verified"], false);
    assert_eq!(verification["passed"], true);

    fs::remove_dir_all(directory).expect("remove temporary source directory");
}

#[test]
fn formula_audit_v2_keeps_earlier_pass_before_later_failure() {
    let directory = std::env::temp_dir().join(format!(
        "adj_formula_audit_guard_order_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("create temporary source directory");
    let program = directory.join("guard_order.adj");
    fs::write(
        &program,
        "formulabook guarded {\n\
             formula first(a, b) = a\n\
               requires nonzero(a), nonzero(b)\n\
               source \"two-input domain\" trust authoritative\n\
         }\n\
         observe left(2)\n\
         observe right(0)\n\
         ? first(left, right)\n",
    )
    .expect("write ordered guards program");

    let output = Command::new(env!("CARGO_BIN_EXE_adj-formula-audit"))
        .arg(&program)
        .output()
        .expect("run formula audit");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let audit: serde_json::Value = serde_json::from_slice(&output.stdout).expect("v2 audit JSON");
    let guards = audit["executions"][0]["guards"]
        .as_array()
        .expect("guard array");
    assert_eq!(guards.len(), 2);
    assert_eq!(guards[0]["outcome"], "passed");
    assert_eq!(guards[0]["precondition"]["index"], 0);
    assert_eq!(guards[1]["outcome"], "failed");
    assert_eq!(guards[1]["precondition"]["index"], 1);
    assert_eq!(audit["executions"][0]["body"]["status"], "withheld");

    fs::remove_dir_all(directory).expect("remove temporary source directory");
}

#[test]
fn formula_audit_v2_keeps_outer_guard_before_nested_failure() {
    let directory = std::env::temp_dir().join(format!(
        "adj_formula_audit_nested_guard_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("create temporary source directory");
    let program = directory.join("nested_guard.adj");
    fs::write(
        &program,
        "formulabook guarded {\n\
             formula inner(a, b) = a / b requires nonzero(b)\n\
               source \"inner domain\" trust authoritative\n\
             formula outer(a, b) = inner(a, b) requires nonzero(a)\n\
               source \"outer domain\" trust authoritative\n\
         }\n\
         observe numerator(8)\n\
         observe denominator(0)\n\
         ? outer(numerator, denominator)\n",
    )
    .expect("write nested guard program");

    let output = Command::new(env!("CARGO_BIN_EXE_adj-formula-audit"))
        .arg(&program)
        .output()
        .expect("run formula audit");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let audit: serde_json::Value = serde_json::from_slice(&output.stdout).expect("v2 audit JSON");
    let execution = &audit["executions"][0];
    assert_eq!(execution["formula_sequence"][0]["name"], "outer");
    assert_eq!(execution["formula_sequence"][1]["name"], "inner");
    assert_eq!(execution["guards"][0]["formula"]["name"], "outer");
    assert_eq!(execution["guards"][0]["outcome"], "passed");
    assert_eq!(execution["guards"][1]["formula"]["name"], "inner");
    assert_eq!(execution["guards"][1]["outcome"], "failed");
    assert_eq!(execution["body"]["status"], "withheld");

    fs::remove_dir_all(directory).expect("remove temporary source directory");
}

#[test]
fn formula_audit_v2_compares_exact_underflow_without_using_zero_f64() {
    let directory = std::env::temp_dir().join(format!(
        "adj_formula_audit_exact_underflow_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("create temporary source directory");
    let program = directory.join("exact_underflow.adj");
    fs::write(
        &program,
        "formulabook guarded {\n\
             formula identity(x) = x\n\
               requires nonzero(x)\n\
               source \"identity domain\" trust authoritative\n\
         }\n\
         observe tiny(1e-400)\n\
         ? identity(tiny)\n",
    )
    .expect("write exact-underflow program");

    let output = Command::new(env!("CARGO_BIN_EXE_adj-formula-audit"))
        .arg(&program)
        .output()
        .expect("run formula audit");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let audit: serde_json::Value = serde_json::from_slice(&output.stdout).expect("v2 audit JSON");
    let observed = &audit["executions"][0]["guards"][0]["comparison"]["observed"];
    assert_eq!(observed["f64_bits"], "0000000000000000");
    assert_ne!(observed["exact_rational"]["numerator"], "0");
    assert_eq!(audit["executions"][0]["guards"][0]["outcome"], "passed");
    assert_eq!(audit["executions"][0]["body"]["status"], "evaluated");

    fs::remove_dir_all(directory).expect("remove temporary source directory");
}

#[test]
fn formula_audit_v2_fails_closed_on_unresolved_guard() {
    let directory = std::env::temp_dir().join(format!(
        "adj_formula_audit_unresolved_guard_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("create temporary source directory");
    let program = directory.join("unresolved.adj");
    fs::write(
        &program,
        "formulabook guarded {\n\
             formula identity(x) = x requires nonzero(x)\n\
               source \"identity domain\" trust authoritative\n\
         }\n\
         ? identity(missing)\n",
    )
    .expect("write unresolved program");

    let output = Command::new(env!("CARGO_BIN_EXE_adj-formula-audit"))
        .arg(&program)
        .output()
        .expect("run formula audit");
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("contains an unresolved guard"));

    fs::remove_dir_all(directory).expect("remove temporary source directory");
}

#[test]
fn formula_audit_v2_witnesses_a_derived_guard_operand() {
    let directory = std::env::temp_dir().join(format!(
        "adj_formula_audit_derived_guard_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("create temporary source directory");
    let program = directory.join("derived.adj");
    fs::write(
        &program,
        "formulabook guarded {\n\
             formula identity(x) = x requires nonzero(x)\n\
               source \"identity domain\" trust authoritative\n\
         }\n\
         observe left(1)\n\
         observe right(1)\n\
         let total = left + right\n\
         ? identity(total)\n",
    )
    .expect("write derived guard program");

    let output = Command::new(env!("CARGO_BIN_EXE_adj-formula-audit"))
        .arg(&program)
        .output()
        .expect("run formula audit");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let audit: serde_json::Value = serde_json::from_slice(&output.stdout).expect("v2 audit JSON");
    let guard = &audit["executions"][0]["guards"][0];
    assert_eq!(guard["outcome"], "passed");
    assert_eq!(guard["inputs"].as_array().unwrap().len(), 2);
    assert_eq!(guard["tree"]["kind"], "derived_reference");
    assert_eq!(guard["tree"]["name"], "total");
    let derived = &guard["derived"];
    assert_eq!(derived["root_computation_id"], 0);
    assert_eq!(derived["computations"].as_array().unwrap().len(), 1);
    assert_eq!(derived["computations"][0]["computation_id"], 0);
    assert_eq!(derived["computations"][0]["name"], "total");
    assert_eq!(derived["computations"][0]["binding"]["name"], "total");
    assert_eq!(
        derived["computations"][0]["referenced_computation_ids"],
        serde_json::json!([])
    );
    assert_eq!(
        derived["verification"]["computation"]["status"],
        "rechecked"
    );
    assert_eq!(audit["executions"][0]["body"]["status"], "evaluated");

    fs::remove_dir_all(directory).expect("remove temporary source directory");
}

#[test]
fn formula_audit_v2_keeps_historical_computation_ids_through_rebinding() {
    let directory = std::env::temp_dir().join(format!(
        "adj_formula_audit_derived_rebinding_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("create temporary source directory");
    let program = directory.join("derived_rebinding.adj");
    let source = "formulabook guarded {\n\
             formula identity(x) = x requires nonzero(x)\n\
               source \"identity domain\" trust authoritative\n\
         }\n\
         observe left(1)\n\
         observe right(2)\n\
         let total = left + right\n\
         let answer = total + left\n\
         let total = right + right\n\
         ? identity(answer)\n";
    fs::write(&program, source)
    .expect("write derived rebinding program");

    let output = Command::new(env!("CARGO_BIN_EXE_adj-formula-audit"))
        .arg(&program)
        .output()
        .expect("run formula audit");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let audit: serde_json::Value = serde_json::from_slice(&output.stdout).expect("v2 audit JSON");
    let derived = &audit["executions"][0]["guards"][0]["derived"];
    assert_eq!(derived["root_computation_id"], 1);
    assert_eq!(derived["computations"][0]["computation_id"], 1);
    assert_eq!(derived["computations"][0]["name"], "answer");
    assert_eq!(
        derived["computations"][0]["referenced_computation_ids"],
        serde_json::json!([0])
    );
    assert_eq!(derived["computations"][1]["computation_id"], 0);
    assert_eq!(derived["computations"][1]["name"], "total");
    assert_ne!(derived["computations"][1]["computation_id"], 2);
    let historical = "let total = left + right";
    let start = source.find(historical).expect("historical binding");
    let expression_start = start + historical.find("left + right").unwrap();
    assert_eq!(
        derived["computations"][1]["binding"]["declaration"],
        serde_json::json!({
            "end": start + historical.len(),
            "sha256": coding_adventures_sha256::sha256_hex(historical.as_bytes()),
            "start": start,
        })
    );
    assert_eq!(
        derived["computations"][1]["binding"]["expression"],
        serde_json::json!({
            "end": expression_start + "left + right".len(),
            "sha256": coding_adventures_sha256::sha256_hex(b"left + right"),
            "start": expression_start,
        })
    );

    fs::remove_dir_all(directory).expect("remove temporary source directory");
}

#[test]
fn formula_audit_v2_skips_withheld_binding_when_assigning_source_identity() {
    let directory = std::env::temp_dir().join(format!(
        "adj_formula_audit_withheld_binding_origin_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("create temporary source directory");
    let program = directory.join("withheld_binding_origin.adj");
    let source = "formulabook guarded {\n\
             formula quotient(a, b) = a / b requires nonzero(b)\n\
               source \"division domain\" trust authoritative\n\
             formula identity(x) = x requires nonzero(x)\n\
               source \"identity domain\" trust authoritative\n\
         }\n\
         observe left(1)\n\
         observe right(2)\n\
         observe zero(0)\n\
         let total = quotient(left, zero)\n\
         let total = left + right\n\
         ? identity(total)\n";
    fs::write(&program, source).expect("write withheld binding source");

    let output = Command::new(env!("CARGO_BIN_EXE_adj-formula-audit"))
        .arg(&program)
        .output()
        .expect("run formula audit");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let audit: serde_json::Value = serde_json::from_slice(&output.stdout).expect("v2 audit JSON");
    let binding = &audit["executions"][0]["guards"][0]["derived"]["computations"][0]
        ["binding"];
    let expected = "let total = left + right";
    let start = source.find(expected).expect("successful binding");
    assert_eq!(binding["declaration"]["start"], start);
    assert_eq!(binding["declaration"]["end"], start + expected.len());
    assert_eq!(
        binding["declaration"]["sha256"],
        coding_adventures_sha256::sha256_hex(expected.as_bytes())
    );

    fs::remove_dir_all(directory).expect("remove temporary source directory");
}

#[test]
fn formula_audit_v2_binds_same_name_computations_across_files() {
    let directory = std::env::temp_dir().join(format!(
        "adj_formula_audit_cross_file_binding_origin_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("create temporary source directory");
    let dependency = directory.join("dependency.adj");
    let program = directory.join("root.adj");
    let dependency_source = "observe left(1)\nlet total = left + 1\n";
    let root_source = "import \"dependency.adj\"\n\
         formulabook guarded {\n\
             formula identity(x) = x requires nonzero(x)\n\
               source \"identity domain\" trust authoritative\n\
         }\n\
         let answer = total + left\n\
         let total = left + 3\n\
         ? identity(answer)\n";
    fs::write(&dependency, dependency_source).expect("write dependency source");
    fs::write(&program, root_source).expect("write root source");

    let output = Command::new(env!("CARGO_BIN_EXE_adj-formula-audit"))
        .arg(&program)
        .output()
        .expect("run formula audit");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let audit: serde_json::Value = serde_json::from_slice(&output.stdout).expect("v2 audit JSON");
    let computations = audit["executions"][0]["guards"][0]["derived"]["computations"]
        .as_array()
        .expect("computation closure");
    assert_eq!(computations[0]["name"], "answer");
    assert_eq!(
        computations[0]["binding"]["source_sha256"],
        coding_adventures_sha256::sha256_hex(root_source.as_bytes())
    );
    assert_eq!(computations[1]["name"], "total");
    assert_eq!(
        computations[1]["binding"]["source_sha256"],
        coding_adventures_sha256::sha256_hex(dependency_source.as_bytes())
    );

    fs::remove_dir_all(directory).expect("remove temporary source directory");
}

#[test]
fn formula_audit_v2_separates_derived_and_guard_formula_sources() {
    let directory = std::env::temp_dir().join(format!(
        "adj_formula_audit_derived_formula_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("create temporary source directory");
    let program = directory.join("derived_formula.adj");
    fs::write(
        &program,
        "formulabook guarded {\n\
             formula add(a, b) = a + b\n\
               source \"addition definition\" trust authoritative\n\
             formula identity(x) = x requires nonzero(x)\n\
               source \"identity domain\" trust authoritative\n\
         }\n\
         observe left(1)\n\
         observe right(2)\n\
         let total = add(left, right)\n\
         ? identity(total)\n",
    )
    .expect("write formula-derived guard program");

    let output = Command::new(env!("CARGO_BIN_EXE_adj-formula-audit"))
        .arg(&program)
        .output()
        .expect("run formula audit");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let audit: serde_json::Value = serde_json::from_slice(&output.stdout).expect("v2 audit JSON");
    let guard = &audit["executions"][0]["guards"][0];
    assert_eq!(guard["formula"]["name"], "identity");
    assert_eq!(
        guard["verification"]["formula_quote"]["identity"]["name"],
        "identity"
    );
    assert_eq!(
        guard["derived"]["formula_sequence"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(guard["derived"]["formula_sequence"][0]["name"], "add");
    assert_eq!(
        guard["derived"]["verification"]["formula_quotes"][0]["identity"]["name"],
        "add"
    );
    assert_eq!(
        guard["derived"]["verification"]["formula_quotes"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    fs::remove_dir_all(directory).expect("remove temporary source directory");
}

#[test]
fn formula_audit_v2_witnesses_a_failed_derived_guard_before_withholding() {
    let directory = std::env::temp_dir().join(format!(
        "adj_formula_audit_derived_zero_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("create temporary source directory");
    let program = directory.join("derived_zero.adj");
    fs::write(
        &program,
        "formulabook guarded {\n\
             formula identity(x) = x requires nonzero(x)\n\
               source \"identity domain\" trust authoritative\n\
         }\n\
         observe left(1)\n\
         observe right(1)\n\
         let total = left - right\n\
         ? identity(total)\n",
    )
    .expect("write zero derived guard program");

    let output = Command::new(env!("CARGO_BIN_EXE_adj-formula-audit"))
        .arg(&program)
        .output()
        .expect("run formula audit");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let audit: serde_json::Value = serde_json::from_slice(&output.stdout).expect("v2 audit JSON");
    let execution = &audit["executions"][0];
    assert_eq!(execution["guards"][0]["outcome"], "failed");
    assert_eq!(execution["guards"][0]["derived"]["root_computation_id"], 0);
    assert_eq!(execution["body"]["status"], "withheld");

    fs::remove_dir_all(directory).expect("remove temporary source directory");
}

#[test]
fn formula_audit_rejects_provenance_that_maps_to_multiple_exports() {
    let directory = std::env::temp_dir().join(format!(
        "adj_formula_audit_ambiguous_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("create temporary source directory");
    let program = directory.join("ambiguous.adj");
    fs::write(
        &program,
        "formulabook formulas {\n\
             formula first(x) = x source \"shared definition\"\n\
             formula second(x) = x source \"shared definition\"\n\
         }\n\
         observe x(2)\n\
         ? first(x)\n",
    )
    .expect("write ambiguous program");

    let output = Command::new(env!("CARGO_BIN_EXE_adj-formula-audit"))
        .arg(&program)
        .output()
        .expect("run formula audit");
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("formula provenance maps ambiguously to 2 exports: shared definition"));

    fs::remove_dir_all(directory).expect("remove temporary source directory");
}

#[test]
fn formula_audit_consumes_a_quoted_fact_from_an_imported_source() {
    let directory = std::env::temp_dir().join(format!(
        "adj_formula_audit_imported_fact_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&directory);
    let snapshots = directory.join("snapshots");
    fs::create_dir_all(&snapshots).expect("create temporary snapshot directory");
    let fact_bytes = b"imported value is 3.";
    let fact_snapshot = coding_adventures_sha256::sha256_hex(fact_bytes);
    fs::write(snapshots.join(&fact_snapshot), fact_bytes).expect("write fact snapshot");
    let formula_bytes = b"Double a value by multiplying it by 2.";
    let formula_snapshot = coding_adventures_sha256::sha256_hex(formula_bytes);
    fs::write(snapshots.join(&formula_snapshot), formula_bytes).expect("write formula snapshot");
    fs::write(
        directory.join("dependency.adj"),
        format!(
            "dictionary imported_vocab {{\n\
                 define imported : finding\n\
             }}\n\
             observe imported(3)\n\
               quote \"imported value is 3.\" at 0 snapshot \"{fact_snapshot}\"\n\
               source \"imported value fixture\"\n\
               locator \"https://example.test/imported-value\"\n\
               trust authoritative\n\
             formulabook imported_math {{\n\
               formula double(value) = value * 2\n\
                 quote \"Double a value by multiplying it by 2.\" at 0 snapshot \"{formula_snapshot}\"\n\
                 source \"Double a value by multiplying it by 2.\"\n\
                 locator \"https://example.test/double\"\n\
                 trust authoritative\n\
             }}\n"
        ),
    )
    .expect("write imported fact dependency");
    let query = directory.join("query.adj");
    fs::write(&query, "import \"dependency.adj\"\n? double(imported)\n")
        .expect("write imported fact query");

    let output = Command::new(env!("CARGO_BIN_EXE_adj-formula-audit"))
        .arg("--snapshots")
        .arg(&snapshots)
        .arg(&query)
        .output()
        .expect("run imported fact audit");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let audit: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("imported fact audit JSON");
    assert_eq!(audit["imports"].as_array().expect("imports").len(), 1);
    assert_eq!(audit["contract"], "adj-lang/formula_audit/v2");
    let derivation = &audit["executions"][0]["body"]["derivation"];
    assert_eq!(derivation["inputs"][0]["term"], "imported(3)");
    assert_eq!(
        derivation["inputs"][0]["provenance"]["quote"]["snapshot_sha256"],
        fact_snapshot
    );
    assert_eq!(derivation["result"]["f64_bits"], "4018000000000000");
    assert_eq!(derivation["verification"]["fully_verified"], true);

    fs::remove_dir_all(directory).expect("remove temporary source directory");
}

#[test]
fn formula_audit_accepts_a_zero_input_constant_formula() {
    let directory =
        std::env::temp_dir().join(format!("adj_formula_audit_constant_{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    let snapshots = directory.join("snapshots");
    fs::create_dir_all(&snapshots).expect("create temporary snapshot directory");
    let source_bytes = b"The answer is 42.";
    let snapshot = coding_adventures_sha256::sha256_hex(source_bytes);
    fs::write(snapshots.join(&snapshot), source_bytes).expect("write source snapshot");
    let program = directory.join("constant.adj");
    fs::write(
        &program,
        format!(
            "formulabook constants {{\n\
                 formula answer(seed) = 42\n\
                   quote \"The answer is 42.\" at 0 snapshot \"{snapshot}\"\n\
                   source \"The answer is 42.\"\n\
                   locator \"https://example.test/constant\"\n\
                   trust authoritative\n\
             }}\n\
             ? answer(0)\n"
        ),
    )
    .expect("write constant program");

    let output = Command::new(env!("CARGO_BIN_EXE_adj-formula-audit"))
        .arg("--snapshots")
        .arg(&snapshots)
        .arg(&program)
        .output()
        .expect("run formula audit");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let audit: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("constant audit JSON");
    assert_eq!(audit["contract"], "adj-lang/formula_audit/v2");
    let execution = &audit["executions"][0];
    assert!(execution["guards"].as_array().expect("guards").is_empty());
    assert_eq!(execution["body"]["status"], "evaluated");
    let derivation = &execution["body"]["derivation"];
    assert_eq!(derivation["export"]["name"], "answer");
    assert_eq!(derivation["inputs"], serde_json::json!([]));
    assert_eq!(
        derivation["verification"]["input_quotes"],
        serde_json::json!([])
    );
    assert_eq!(derivation["verification"]["fully_verified"], true);
    assert!(audit["imports"].as_array().expect("imports").is_empty());

    fs::remove_dir_all(directory).expect("remove temporary source directory");
}
