use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

use ibm_system360_gatelevel::{gates, System360GateLevel};

fn rust_tokens(source: &str) -> Vec<String> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        } else if bytes[cursor..].starts_with(b"//") {
            cursor += 2;
            while cursor < bytes.len() && bytes[cursor] != b'\n' {
                cursor += 1;
            }
        } else if bytes[cursor..].starts_with(b"/*") {
            cursor += 2;
            let mut depth = 1;
            while cursor < bytes.len() && depth != 0 {
                if bytes[cursor..].starts_with(b"/*") {
                    depth += 1;
                    cursor += 2;
                } else if bytes[cursor..].starts_with(b"*/") {
                    depth -= 1;
                    cursor += 2;
                } else {
                    cursor += 1;
                }
            }
        } else if bytes[cursor] == b'"' {
            cursor += 1;
            while cursor < bytes.len() {
                if bytes[cursor] == b'\\' {
                    cursor += 2;
                } else if bytes[cursor] == b'"' {
                    cursor += 1;
                    break;
                } else {
                    cursor += 1;
                }
            }
            tokens.push("<string>".to_owned());
        } else if bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_' {
            let start = cursor;
            cursor += 1;
            while cursor < bytes.len()
                && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_')
            {
                cursor += 1;
            }
            tokens.push(source[start..cursor].to_owned());
        } else {
            tokens.push((bytes[cursor] as char).to_string());
            cursor += 1;
        }
    }
    tokens
}

fn matching_delimiter(
    tokens: &[String],
    open: usize,
    opening: &str,
    closing: &str,
) -> Result<usize, String> {
    let mut depth = 0;
    for (index, token) in tokens.iter().enumerate().skip(open) {
        if token == opening {
            depth += 1;
        } else if token == closing {
            depth -= 1;
            if depth == 0 {
                return Ok(index);
            }
        }
    }
    Err(format!("unclosed {opening} delimiter"))
}

fn function_body(tokens: &[String], name: &str) -> Result<Vec<String>, String> {
    let starts: Vec<_> = tokens
        .windows(2)
        .enumerate()
        .filter_map(|(index, pair)| (pair[0] == "fn" && pair[1] == name).then_some(index))
        .collect();
    if starts.len() != 1 {
        return Err(format!(
            "expected exactly one function named {name}, found {}",
            starts.len()
        ));
    }
    let open = (starts[0]..tokens.len())
        .find(|&index| tokens[index] == "{")
        .ok_or_else(|| format!("function {name} has no body"))?;
    let close = matching_delimiter(tokens, open, "{", "}")?;
    Ok(tokens[open + 1..close].to_vec())
}

fn require_exact_body(source: &str, function: &str, expected: &str) -> Result<(), String> {
    let actual = function_body(&rust_tokens(source), function)?;
    let expected = rust_tokens(expected);
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "{function} differs from its reviewed body\nactual: {actual:?}\nexpected: {expected:?}"
        ))
    }
}

fn token_digest(tokens: &[String]) -> u64 {
    tokens
        .iter()
        .fold(0xcbf2_9ce4_8422_2325, |mut hash, token| {
            for byte in token.bytes().chain(std::iter::once(0xff)) {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
            hash
        })
}

fn require_body_digest(source: &str, function: &str, expected: u64) -> Result<(), String> {
    let body = function_body(&rust_tokens(source), function)?;
    let actual = token_digest(&body);
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "{function} parsed body digest changed: {actual:#018x}, expected {expected:#018x}"
        ))
    }
}

fn const_initializer(source: &str, name: &str) -> Result<Vec<String>, String> {
    let tokens = rust_tokens(source);
    let starts: Vec<_> = tokens
        .windows(2)
        .enumerate()
        .filter_map(|(index, pair)| (pair == ["const", name]).then_some(index))
        .collect();
    if starts.len() != 1 {
        return Err(format!("expected one const {name}, found {}", starts.len()));
    }
    let equals = (starts[0]..tokens.len())
        .find(|&index| tokens[index] == "=")
        .ok_or_else(|| format!("const {name} has no initializer"))?;
    let semicolon = (equals..tokens.len())
        .find(|&index| tokens[index] == ";")
        .ok_or_else(|| format!("const {name} has no terminator"))?;
    Ok(tokens[equals + 1..semicolon].to_vec())
}

fn require_const_digest(source: &str, name: &str, expected: u64) -> Result<(), String> {
    let actual = token_digest(&const_initializer(source, name)?);
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "{name} parsed initializer digest changed: {actual:#018x}, expected {expected:#018x}"
        ))
    }
}

fn require_external_module(source: &str, name: &str, public: bool) -> Result<(), String> {
    let syntax = syn::parse_file(source).map_err(|error| format!("invalid Rust: {error}"))?;
    let modules: Vec<_> = syntax
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Mod(module) if module.ident == name => Some(module),
            _ => None,
        })
        .collect();
    if modules.len() != 1 {
        return Err(format!("expected one top-level {name} module"));
    }
    let module = modules[0];
    let visibility_matches = matches!(
        (&module.vis, public),
        (syn::Visibility::Public(_), true) | (syn::Visibility::Inherited, false)
    );
    if !visibility_matches
        || !module.attrs.is_empty()
        || module.content.is_some()
        || module.semi.is_none()
    {
        return Err(format!("{name} is not the reviewed external module"));
    }
    Ok(())
}

fn contains(tokens: &[String], pattern: &str) -> bool {
    let pattern = rust_tokens(pattern);
    tokens
        .windows(pattern.len())
        .any(|window| window == pattern.as_slice())
}

fn candidate_lines(tokens: &[String]) -> Result<Vec<usize>, String> {
    let mut lines = Vec::new();
    for index in 0..tokens.len().saturating_sub(5) {
        if tokens[index] == "candidate" && tokens[index + 1] == "!" && tokens[index + 2] == "(" {
            let literal = tokens[index + 3].trim_start_matches("0x");
            let radix = if tokens[index + 3].starts_with("0x") {
                16
            } else {
                10
            };
            lines.push(
                usize::from_str_radix(literal, radix)
                    .map_err(|error| format!("invalid candidate line: {error}"))?,
            );
        }
    }
    Ok(lines)
}

fn verify_decode_authority(networks: &str, executor: &str, validator: &str) -> Result<(), String> {
    syn::parse_file(networks).map_err(|error| format!("gates do not parse: {error}"))?;
    syn::parse_file(executor).map_err(|error| format!("executor does not parse: {error}"))?;
    syn::parse_file(validator).map_err(|error| format!("validator does not parse: {error}"))?;
    require_external_module(validator, "executor", false)?;
    require_external_module(validator, "gates", true)?;

    // These are hashes of parsed Rust tokens, not source bytes.  They retain
    // the former exact-body locks without making harmless whitespace or
    // comment changes significant, while covering the complete authority,
    // router, validator, branch/add/compare, and public commit chain.
    for (function, digest) in [
        ("decode_byte", 0x96cb_0cae_2d05_2ad9),
        ("add_24", 0xa04f_a7e8_e3af_49b0),
        ("add_32", 0x3d11_3f86_3d34_d438),
        ("branch_selected", 0x7a7a_e79a_1519_1977),
        ("index_branch_selected", 0x9c31_cac5_60a3_4171),
        ("signed_compare_condition_code", 0x4e5c_8f7d_9b74_a639),
    ] {
        require_body_digest(networks, function, digest)?;
    }
    require_const_digest(executor, "CANDIDATES", 0x22b1_e37c_9a05_f100)?;
    for (function, digest) in [
        ("new", 0x93b8_b823_037f_bec8),
        ("transport_width", 0x2040_ffc0_ac40_1d6b),
        ("operation", 0x9ac2_8adf_5c72_d0b1),
        ("format_matches_operation", 0x4ad6_bb6c_d234_da31),
        ("execute", 0xd23f_a2ec_eb10_29a9),
        ("execute_rr", 0x0eff_489c_cc30_d39b),
        ("execute_rx", 0x0aca_3dbb_e40d_8499),
        ("execute_rs", 0x5604_80ab_c44a_4da4),
        ("execute_si", 0xc0c5_9ca1_85bf_6bdc),
        ("execute_ss", 0xc14c_e106_eeee_a468),
    ] {
        require_body_digest(executor, function, digest)?;
    }
    for (function, digest) in [
        ("decode_transition", 0x7de3_9596_a584_63e1),
        ("validate_gate_transition", 0xe541_8847_1eff_e819),
        ("validate_control_transition", 0x63cb_8bbd_bdfe_0fda),
        ("commit_transition", 0x5fb6_3d73_0d97_42f6),
        ("step", 0x49e8_10bf_f82d_108f),
        ("run", 0xf70a_d777_3fa6_49be),
        ("execute", 0x5050_4f02_0534_5e98),
    ] {
        require_body_digest(validator, function, digest)?;
    }

    require_exact_body(
        networks,
        "decode_byte",
        "let inputs:[u8;8]=std::array::from_fn(|bit|(value>>bit)&1);
         std::array::from_fn(|line|{(0..8).fold(1,|selected,bit|{
             let expected=((line>>bit)&1)as u8;
             let signal=if expected==1{inputs[bit]}else{not_gate(inputs[bit])};
             and_gate(selected,signal)
         })})",
    )?;
    require_exact_body(
        networks,
        "index_branch_selected",
        "let low=compare_condition_code&1;
         let high_bit=(compare_condition_code>>1)&1;
         let high=and_gate(high_bit,not_gate(low));
         or_gate(and_gate(bxh_select,high),and_gate(bxle_select,not_gate(high)),)",
    )?;
    require_exact_body(
        validator,
        "step",
        r#"let before=self.get_state();
        let(authority,fetched)=decode_transition(&before)?;
        let mut transition=System360Simulator::new();
        transition.restore(before.clone())?;
        let trace=transition.step()?;
        let oracle_next=transition.get_state();
        if trace.raw!=fetched||usize::from(trace.width)!=fetched.len(){
            return Err(System360Error::Specification{reason:"reason",});
        }
        let gate_next=executor::execute(&before,&trace,&authority)?;
        validate_gate_transition(&before,&gate_next,&trace,&authority)?;
        if gate_next!=oracle_next{return Err(System360Error::Specification{reason:"reason",});}
        self.commit_transition(&gate_next);
        Ok(trace)"#,
    )?;

    let executor_tokens = rust_tokens(executor);
    let validator_tokens = rust_tokens(validator);
    let network_tokens = rust_tokens(networks);
    if !contains(
        &executor_tokens,
        "use crate::{gates,hfp::GateHfp,StepTrace,System360Error,System360State}",
    ) {
        return Err("executor does not bind the reviewed gate module".to_owned());
    }
    let decode_mentions = [&executor_tokens, &validator_tokens, &network_tokens]
        .into_iter()
        .map(|tokens| {
            tokens
                .windows(2)
                .filter(|pair| pair[0] == "decode_byte" && pair[1] == "(")
                .count()
        })
        .sum::<usize>();
    if decode_mentions != 2 {
        return Err(format!(
            "expected one decode definition and one transition-boundary call, found {decode_mentions}"
        ));
    }
    if !contains(&executor_tokens, "let lines=gates::decode_byte(opcode)")
        || !contains(&executor_tokens, "gate_cardinality(&formats)")
        || !contains(&executor_tokens, "legal_multiple=or_gate")
        || !contains(&executor_tokens, "let operation=authority.operation()")
        || !contains(&executor_tokens, "match operation")
    {
        return Err("decode-to-operation authority chain changed".to_owned());
    }
    if contains(&executor_tokens, "raw[0]")
        || contains(&executor_tokens, "trace.raw[0]")
        || contains(&validator_tokens, "match opcode")
        || contains(&validator_tokens, "matches!(opcode")
    {
        return Err("raw opcode control escaped the transition decoder".to_owned());
    }
    let execute_body = function_body(&executor_tokens, "execute")?;
    let authorization = execute_body
        .windows(5)
        .position(|window| window == rust_tokens("authority.operation()").as_slice())
        .ok_or_else(|| "executor does not obtain the authorized operation".to_owned())?;
    let first_storage = ["before.memory[", "next.memory["]
        .into_iter()
        .filter_map(|pattern| {
            let pattern = rust_tokens(pattern);
            execute_body
                .windows(pattern.len())
                .position(|window| window == pattern.as_slice())
        })
        .min();
    if first_storage.is_some_and(|storage| storage < authorization) {
        return Err("storage is observed before operation authorization".to_owned());
    }
    if !execute_body.ends_with(&rust_tokens("Ok(next)")) {
        return Err("executor has control after the selected operation result".to_owned());
    }

    let lines = candidate_lines(&executor_tokens)?;
    let mut unique = lines.clone();
    unique.sort_unstable();
    unique.dedup();
    if lines.len() != 104 || unique.len() != lines.len() {
        return Err(format!(
            "candidate table must contain 104 unique legal lines, found {} / {} unique",
            lines.len(),
            unique.len()
        ));
    }
    Ok(())
}

fn replace_once(source: &str, anchor: &str, replacement: &str) -> String {
    assert_eq!(
        source.matches(anchor).count(),
        1,
        "mutation anchor: {anchor}"
    );
    source.replacen(anchor, replacement, 1)
}

static MUTANT_ID: AtomicUsize = AtomicUsize::new(0);

struct MutantCrate(PathBuf);

impl Drop for MutantCrate {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn toml_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "\\\\")
}

fn mutant_target() -> PathBuf {
    std::env::temp_dir().join(format!(
        "system360-authority-mutant-target-{}",
        std::process::id()
    ))
}

fn compile_mutant(executor: &str, validator: &str) {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rust_packages = manifest_dir.parent().expect("Rust package parent");
    let id = MUTANT_ID.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "system360-authority-mutant-{}-{id}",
        std::process::id()
    ));
    let guard = MutantCrate(root.clone());
    fs::create_dir_all(root.join("src")).expect("create mutant source directory");
    let manifest = format!(
        r#"[package]
name = "system360-authority-mutant-{id}"
version = "0.0.0"
edition = "2021"

[dependencies]
arithmetic = {{ path = "{}" }}
logic-gates = {{ path = "{}" }}
ibm-system360-simulator = {{ path = "{}" }}
"#,
        toml_path(&rust_packages.join("arithmetic")),
        toml_path(&rust_packages.join("logic-gates")),
        toml_path(&rust_packages.join("ibm-system360-simulator")),
    );
    fs::write(root.join("Cargo.toml"), manifest).expect("write mutant manifest");
    for file in ["gates.rs", "hfp.rs", "state.rs"] {
        fs::copy(
            manifest_dir.join("src").join(file),
            root.join("src").join(file),
        )
        .expect("copy production source");
    }
    fs::write(root.join("src/executor.rs"), executor).expect("write mutant executor");
    fs::write(root.join("src/lib.rs"), validator).expect("write mutant validator");
    let output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args(["check", "--quiet", "--manifest-path"])
        .arg(root.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", mutant_target())
        .output()
        .expect("run cargo check for hostile mutant");
    assert!(
        output.status.success(),
        "hostile mutation must compile before the verifier rejects it:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    drop(guard);
}

fn assert_executor_mutant_rejected(networks: &str, executor: &str, validator: &str) {
    compile_mutant(executor, validator);
    assert!(verify_decode_authority(networks, executor, validator).is_err());
}

fn assert_validator_mutant_rejected(networks: &str, executor: &str, validator: &str) {
    compile_mutant(executor, validator);
    assert!(verify_decode_authority(networks, executor, validator).is_err());
}

#[test]
fn branch_predicates_cover_every_selector_wire_and_boundary() {
    for mask in 0_u8..16 {
        for condition_code in 0_u8..4 {
            assert_eq!(
                gates::branch_selected(mask, condition_code),
                u8::from(mask & (8 >> condition_code) != 0)
            );
        }
    }
    for target_register in 0_u8..16 {
        for count in [0, 1, 2, u32::MAX, 0x8000_0000] {
            assert_eq!(
                gates::count_branch_selected(count, target_register),
                u8::from(count != 0 && target_register != 0)
            );
        }
    }
    for comparison in 0_u8..4 {
        assert_eq!(
            gates::index_branch_selected(1, 0, comparison),
            u8::from(comparison == 2)
        );
        assert_eq!(
            gates::index_branch_selected(0, 1, comparison),
            u8::from(comparison != 2)
        );
    }
    assert_eq!(gates::index_branch_selected(0, 0, 2), 0);
}

#[test]
fn production_decode_and_dispatch_match_the_reviewed_authority_chain() {
    verify_decode_authority(
        include_str!("../src/gates.rs"),
        include_str!("../src/executor.rs"),
        include_str!("../src/lib.rs"),
    )
    .expect("production decode authority must match the reviewed structure");
}

#[test]
fn compile_checked_hostile_dispatch_shapes_are_rejected() {
    let _target_guard = MutantCrate(mutant_target());
    let networks = include_str!("../src/gates.rs");
    let executor = include_str!("../src/executor.rs");
    let validator = include_str!("../src/lib.rs");

    let raw_match = replace_once(
        executor,
        "let operation = authority.operation().ok_or(System360Error::Specification {",
        "let operation = match trace.raw[0] { _ => authority.operation() }.ok_or(System360Error::Specification {",
    );
    assert_executor_mutant_rejected(networks, &raw_match, validator);

    let raw_range = replace_once(
        executor,
        "let operation = authority.operation().ok_or(System360Error::Specification {",
        "let operation = if (0x40..=0x7f).contains(&trace.raw[0]) { authority.operation() } else { authority.operation() }.ok_or(System360Error::Specification {",
    );
    assert_executor_mutant_rejected(networks, &raw_range, validator);

    let raw_equality = replace_once(
        executor,
        "let operation = authority.operation().ok_or(System360Error::Specification {",
        "let operation = if trace.raw[0] == 0x05 { authority.operation() } else { authority.operation() }.ok_or(System360Error::Specification {",
    );
    assert_executor_mutant_rejected(networks, &raw_equality, validator);

    let helper_indirection = replace_once(
        executor,
        "pub(crate) fn execute(\n",
        "fn operation_from_trace(trace: &StepTrace, authority: &DecodeAuthority) -> Option<Operation> { let _ = trace.raw.first(); authority.operation() }\npub(crate) fn execute(\n",
    );
    let helper_indirection = replace_once(
        &helper_indirection,
        "let operation = authority.operation().ok_or(System360Error::Specification {",
        "let operation = operation_from_trace(trace, authority).ok_or(System360Error::Specification {",
    );
    assert_executor_mutant_rejected(networks, &helper_indirection, validator);

    let duplicate_decode = replace_once(
        executor,
        "let lines = gates::decode_byte(opcode);",
        "let lines = gates::decode_byte(opcode); let _duplicate = gates::decode_byte(opcode);",
    );
    assert_executor_mutant_rejected(networks, &duplicate_decode, validator);

    let dead_decode = replace_once(
        executor,
        "let lines = gates::decode_byte(opcode);",
        "if false { let _ = gates::decode_byte(opcode); } let lines = [0; 256];",
    );
    assert_executor_mutant_rejected(networks, &dead_decode, validator);

    let secondary_decode = replace_once(
        executor,
        "let operation = authority.operation().ok_or(System360Error::Specification {",
        "let _secondary = gates::decode_byte(trace.raw[0]); let operation = authority.operation().ok_or(System360Error::Specification {",
    );
    assert_executor_mutant_rejected(networks, &secondary_decode, validator);

    let inactive_access = replace_once(
        executor,
        "let operation = authority.operation().ok_or(System360Error::Specification {",
        "let _inactive = before.memory[0]; let operation = authority.operation().ok_or(System360Error::Specification {",
    );
    assert_executor_mutant_rejected(networks, &inactive_access, validator);

    let shadow = replace_once(
        executor,
        "use crate::{gates, hfp::GateHfp, StepTrace, System360Error, System360State};",
        "mod fake_gates { pub(crate) use crate::gates::*; } use fake_gates as gates; use crate::{hfp::GateHfp, StepTrace, System360Error, System360State};",
    );
    assert_executor_mutant_rejected(networks, &shadow, validator);

    let post_authorization_overwrite = replace_once(
        executor,
        "if authority.transport_width() != Some(trace.raw.len())",
        "let operation = if trace.raw.first().copied() == Some(0x05) { Operation::Balr } else { operation }; if authority.transport_width() != Some(trace.raw.len())",
    );
    assert_executor_mutant_rejected(networks, &post_authorization_overwrite, validator);

    let oracle_dispatch = replace_once(
        validator,
        "let gate_next = executor::execute(&before, &trace, &authority)?;",
        "let gate_next = oracle_next.clone();",
    );
    assert_validator_mutant_rejected(networks, executor, &oracle_dispatch);

    let post_overwrite = replace_once(
        validator,
        "self.commit_transition(&gate_next);",
        "self.commit_transition(&oracle_next);",
    );
    assert_validator_mutant_rejected(networks, executor, &post_overwrite);
}

#[test]
fn inactive_storage_candidates_are_not_evaluated() {
    let mut simulator = System360GateLevel::new();
    simulator
        .load(&[0x41, 0x00, 0x1f, 0xff], 0)
        .expect("load LA");
    simulator
        .write_gpr(1, 0x00ff_f000)
        .expect("install high base");
    simulator
        .step()
        .expect("LA must not evaluate storage candidates");
    assert_eq!(simulator.read_gpr(0).expect("read R0"), 0x00ff_ffff);
}
