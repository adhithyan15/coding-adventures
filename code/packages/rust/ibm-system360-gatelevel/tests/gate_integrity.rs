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

#[allow(dead_code)]
mod compile_checked_hostile_shapes {
    use super::gates;

    pub fn raw_match(raw: &[u8]) -> u8 {
        match raw[0] {
            0x05..=0x07 => 1,
            _ => 0,
        }
    }

    pub fn raw_equality(raw: &[u8]) -> bool {
        raw[0] == 0x05
    }

    pub fn duplicate_decode(opcode: u8) -> ([u8; 256], [u8; 256]) {
        (gates::decode_byte(opcode), gates::decode_byte(opcode))
    }

    pub fn inactive_read(memory: &[u8], selected: bool) -> Option<u8> {
        let eager = memory[0];
        selected.then_some(eager)
    }

    pub fn post_overwrite(mut selected: u32, old: u32) -> u32 {
        selected = selected.wrapping_add(1);
        let _ = selected;
        old
    }
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
    let networks = include_str!("../src/gates.rs");
    let executor = include_str!("../src/executor.rs");
    let validator = include_str!("../src/lib.rs");

    let raw_match = executor.replacen(
        "let operation = authority.operation().ok_or(System360Error::Specification {",
        "let operation = match trace.raw[0] { _ => authority.operation() }.ok_or(System360Error::Specification {",
        1,
    );
    assert!(verify_decode_authority(networks, &raw_match, validator).is_err());

    let raw_range = executor.replacen(
        "let operation = authority.operation().ok_or(System360Error::Specification {",
        "let operation = if (0x40..=0x7f).contains(&trace.raw[0]) { authority.operation() } else { authority.operation() }.ok_or(System360Error::Specification {",
        1,
    );
    assert!(verify_decode_authority(networks, &raw_range, validator).is_err());

    let raw_equality = executor.replacen(
        "let operation = authority.operation().ok_or(System360Error::Specification {",
        "let operation = if trace.raw[0] == 0x05 { authority.operation() } else { authority.operation() }.ok_or(System360Error::Specification {",
        1,
    );
    assert!(verify_decode_authority(networks, &raw_equality, validator).is_err());

    let duplicate_decode = executor.replacen(
        "let lines = gates::decode_byte(opcode);",
        "let lines = gates::decode_byte(opcode); let _duplicate = gates::decode_byte(opcode);",
        1,
    );
    assert!(verify_decode_authority(networks, &duplicate_decode, validator).is_err());

    let dead_decode = executor.replacen(
        "let lines = gates::decode_byte(opcode);",
        "if false { let _ = gates::decode_byte(opcode); } let lines = [0; 256];",
        1,
    );
    assert!(verify_decode_authority(networks, &dead_decode, validator).is_err());

    let secondary_decode = executor.replacen(
        "let operation = authority.operation().ok_or(System360Error::Specification {",
        "let _secondary = gates::decode_byte(trace.raw[0]); let operation = authority.operation().ok_or(System360Error::Specification {",
        1,
    );
    assert!(verify_decode_authority(networks, &secondary_decode, validator).is_err());

    let inactive_access = executor.replacen(
        "let operation = authority.operation().ok_or(System360Error::Specification {",
        "let _inactive = before.memory[0]; let operation = authority.operation().ok_or(System360Error::Specification {",
        1,
    );
    assert!(verify_decode_authority(networks, &inactive_access, validator).is_err());

    let shadow = executor.replacen(
        "use crate::{gates, hfp::GateHfp, StepTrace, System360Error, System360State};",
        "use fake_gates as gates; use crate::{hfp::GateHfp, StepTrace, System360Error, System360State};",
        1,
    );
    assert!(verify_decode_authority(networks, &shadow, validator).is_err());

    let oracle_dispatch = validator.replacen(
        "let gate_next = executor::execute(&before, &trace, &authority)?;",
        "let gate_next = oracle_next.clone();",
        1,
    );
    assert!(verify_decode_authority(networks, executor, &oracle_dispatch).is_err());

    let post_overwrite = validator.replacen(
        "self.commit_transition(&gate_next);",
        "self.commit_transition(&oracle_next);",
        1,
    );
    assert!(verify_decode_authority(networks, executor, &post_overwrite).is_err());
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
