use quote::ToTokens;
use syn::{Expr, ExprMatch, Item, ItemFn, ItemMod, Stmt};

const UNSIGNED_ENTRY: &str = r#"
pub(crate) fn multiply_unsigned(a: u64, b: u64) -> QualifiedWideResult {
    qualify_unsigned(&u128_to_bits(multiply_unsigned_wide(a, b)))
}
"#;

const SIGNED_ENTRY: &str = r#"
pub(crate) fn multiply_signed(a: u64, b: u64) -> QualifiedWideResult {
    let a_negative = u64_to_bits(a)[63];
    let b_negative = u64_to_bits(b)[63];
    let a_magnitude = magnitude64(a, a_negative);
    let b_magnitude = magnitude64(b, b_negative);
    let product = multiply_unsigned_wide(a_magnitude, b_magnitude);
    let product = if xor_gate(a_negative, b_negative) == 0 {
        product as i128
    } else {
        negate128(product) as i128
    };
    qualify_signed(&u128_to_bits(product as u128))
}
"#;

const UNSIGNED_DIVISION_ENTRY: &str = r#"
pub(crate) fn divide_unsigned(dividend: u128, divisor: u64) -> Option<QualifiedDivision> {
    let (quotient, remainder) = divide_unsigned_wide(dividend, divisor)?;
    Some(QualifiedDivision {
        quotient: qualify_unsigned(&u128_to_bits(quotient)),
        remainder,
    })
}
"#;

const SIGNED_DIVISION_ENTRY: &str = r#"
pub(crate) fn divide_signed(dividend: u128, divisor: u64) -> Option<QualifiedDivision> {
    if zero_64(divisor) != 0 {
        return None;
    }
    let dividend_negative = u128_to_bits(dividend)[127];
    let divisor_negative = u64_to_bits(divisor)[63];
    let dividend_magnitude = if dividend_negative == 0 {
        dividend
    } else {
        negate128(dividend)
    };
    let divisor_magnitude = magnitude64(divisor, divisor_negative);
    let (quotient, remainder) = divide_unsigned_wide(dividend_magnitude, divisor_magnitude)?;
    let quotient = if xor_gate(dividend_negative, divisor_negative) == 0 {
        quotient
    } else {
        negate128(quotient)
    };
    let remainder = if dividend_negative == 0 {
        remainder
    } else {
        add_64(invert_64(remainder), 0, 1).0
    };
    Some(QualifiedDivision {
        quotient: qualify_signed(&u128_to_bits(quotient)),
        remainder,
    })
}
"#;

const UNSIGNED_QUALIFIER: &str = r#"
fn qualify_unsigned(bits: &[u8; 128]) -> QualifiedWideResult {
    let low = bits_to_u64(&bits[..64]);
    let high = bits_to_u64(&bits[64..]);
    let any_high = bits[64..]
        .iter()
        .copied()
        .fold(0, ::logic_gates::gates::or_gate);
    QualifiedWideResult {
        low,
        high,
        overflow: any_high,
    }
}
"#;

const SIGNED_QUALIFIER: &str = r#"
fn qualify_signed(bits: &[u8; 128]) -> QualifiedWideResult {
    let low = bits_to_u64(&bits[..64]);
    let high = bits_to_u64(&bits[64..]);
    let sign = bits[63];
    let differs_from_sign = bits[64..]
        .iter()
        .copied()
        .map(|bit| ::logic_gates::gates::xor_gate(bit, sign))
        .fold(0, ::logic_gates::gates::or_gate);
    QualifiedWideResult {
        low,
        high,
        overflow: differs_from_sign,
    }
}
"#;

const MULTIPLY_CONSUMERS: &str = r#"
Instr::Imul { dst, src } => {
    let product = crate::gate_networks::multiply_signed(
        st.get(*dst),
        read_op(st, mem, src, next_ip, 8)?,
    );
    st.set(*dst, product.low);
    let truncated = product.overflow != 0;
    st.flags.cf = truncated;
    st.flags.of = truncated;
    Ok(Flow::Next)
}
Instr::ImulImm { dst, src, imm } => {
    let product = crate::gate_networks::multiply_signed(
        read_op(st, mem, src, next_ip, 8)?,
        *imm as u64,
    );
    st.set(*dst, product.low);
    let truncated = product.overflow != 0;
    st.flags.cf = truncated;
    st.flags.of = truncated;
    Ok(Flow::Next)
}
Instr::Mul { src } => {
    let source = read_op(st, mem, src, next_ip, 8)?;
    let product = crate::gate_networks::multiply_unsigned(st.get(Reg::Rax), source);
    st.set(Reg::Rax, product.low);
    st.set(Reg::Rdx, product.high);
    st.flags.cf = product.overflow != 0;
    st.flags.of = st.flags.cf;
    Ok(Flow::Next)
}
"#;

const DIVIDE_CONSUMER: &str = r#"
Instr::Div { divisor, signed } => {
    let d = read_op(st, mem, divisor, next_ip, 8)?;
    let lo = st.get(Reg::Rax);
    let hi = st.get(Reg::Rdx);
    let dividend = ((hi as u128) << 64) | (lo as u128);
    let (quot, rem) = if *signed {
        let division = crate::gate_networks::divide_signed(dividend, d)
            .ok_or(Trap::DivideError(instr_off as u64))?;
        if division.quotient.overflow != 0 {
            return Err(Trap::DivideError(instr_off as u64));
        }
        (division.quotient.low, division.remainder)
    } else {
        let division = crate::gate_networks::divide_unsigned(dividend, d)
            .ok_or(Trap::DivideError(instr_off as u64))?;
        if division.quotient.overflow != 0 {
            return Err(Trap::DivideError(instr_off as u64));
        }
        (division.quotient.low, division.remainder)
    };
    st.set(Reg::Rax, quot);
    st.set(Reg::Rdx, rem);
    Ok(Flow::Next)
}
"#;

fn canonical(node: &impl ToTokens) -> String {
    node.to_token_stream().to_string()
}

fn unique_top_level_function<'a>(file: &'a syn::File, name: &str) -> Result<&'a ItemFn, String> {
    let matches: Vec<_> = file
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Fn(function) if function.sig.ident == name => Some(function),
            _ => None,
        })
        .collect();
    if matches.len() != 1 {
        return Err(format!(
            "expected exactly one top-level function named {name}, found {}",
            matches.len()
        ));
    }
    Ok(matches[0])
}

fn unique_top_level_module<'a>(file: &'a syn::File, name: &str) -> Result<&'a ItemMod, String> {
    let matches: Vec<_> = file
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Mod(module) if module.ident == name => Some(module),
            _ => None,
        })
        .collect();
    if matches.len() != 1 {
        return Err(format!(
            "expected exactly one top-level module named {name}, found {}",
            matches.len()
        ));
    }
    Ok(matches[0])
}

fn validate_function(source: &str, name: &str, expected: &str) -> Result<(), String> {
    let file = syn::parse_file(source).map_err(|error| error.to_string())?;
    let actual = unique_top_level_function(&file, name)?;
    let expected: ItemFn = syn::parse_str(expected).map_err(|error| error.to_string())?;
    if canonical(actual) != canonical(&expected) {
        return Err(format!("top-level function {name} changed"));
    }
    Ok(())
}

fn validate_external_module(source: &str, name: &str) -> Result<(), String> {
    let file = syn::parse_file(source).map_err(|error| error.to_string())?;
    let actual = unique_top_level_module(&file, name)?;
    let expected: ItemMod =
        syn::parse_str(&format!("mod {name};")).map_err(|error| error.to_string())?;
    if canonical(actual) != canonical(&expected) {
        return Err(format!(
            "top-level module {name} must be the attribute-free external file binding"
        ));
    }
    Ok(())
}

fn parse_expected_arms(source: &str) -> Result<Vec<syn::Arm>, String> {
    let expression: Expr = syn::parse_str(&format!("match __contract_subject {{ {source} }}"))
        .map_err(|error| error.to_string())?;
    let Expr::Match(dispatch) = expression else {
        return Err("expected fixture did not parse as a match".to_owned());
    };
    Ok(dispatch.arms)
}

fn direct_instruction_dispatch(function: &ItemFn) -> Result<&ExprMatch, String> {
    if function
        .attrs
        .iter()
        .any(|attribute| !attribute.path().is_ident("doc"))
    {
        return Err("exec_one carries a non-documentation attribute".to_owned());
    }
    let [Stmt::Expr(Expr::Match(dispatch), None)] = function.block.stmts.as_slice() else {
        return Err("exec_one must consist solely of its instruction match".to_owned());
    };
    if canonical(dispatch.expr.as_ref()) != "ins" {
        return Err("exec_one no longer dispatches directly on ins".to_owned());
    }
    if dispatch.arms.len() != 47 {
        return Err(format!(
            "expected all 47 direct instruction arms, found {}",
            dispatch.arms.len()
        ));
    }
    for arm in &dispatch.arms {
        if !arm.attrs.is_empty() || arm.guard.is_some() {
            return Err("exec_one arms must not use attributes or guards".to_owned());
        }
        if !matches!(
            arm.pat,
            syn::Pat::Path(_) | syn::Pat::Struct(_) | syn::Pat::TupleStruct(_)
        ) {
            return Err("exec_one arms must directly name one Instr variant".to_owned());
        }
    }
    Ok(dispatch)
}

fn validate_instruction_arms(source: &str, expected: &str) -> Result<(), String> {
    let file = syn::parse_file(source).map_err(|error| error.to_string())?;
    let function = unique_top_level_function(&file, "exec_one")?;
    let dispatch = direct_instruction_dispatch(function)?;
    for expected_arm in parse_expected_arms(expected)? {
        let pattern = canonical(&expected_arm.pat);
        let matches: Vec<_> = dispatch
            .arms
            .iter()
            .filter(|arm| canonical(&arm.pat) == pattern)
            .collect();
        if matches.len() != 1 {
            return Err(format!(
                "expected exactly one exec_one arm for {pattern}, found {}",
                matches.len()
            ));
        }
        if canonical(matches[0]) != canonical(&expected_arm) {
            return Err(format!("exec_one arm for {pattern} changed"));
        }
    }
    Ok(())
}

fn validate_gate_contracts(source: &str) -> Result<(), String> {
    for (name, expected) in [
        ("multiply_unsigned", UNSIGNED_ENTRY),
        ("multiply_signed", SIGNED_ENTRY),
        ("divide_unsigned", UNSIGNED_DIVISION_ENTRY),
        ("divide_signed", SIGNED_DIVISION_ENTRY),
        ("qualify_unsigned", UNSIGNED_QUALIFIER),
        ("qualify_signed", SIGNED_QUALIFIER),
    ] {
        validate_function(source, name, expected)?;
    }
    Ok(())
}

fn validate_execution_contracts(source: &str) -> Result<(), String> {
    validate_instruction_arms(source, MULTIPLY_CONSUMERS)?;
    validate_instruction_arms(source, DIVIDE_CONSUMER)
}

fn validate_production_routing(source: &str) -> Result<(), String> {
    validate_external_module(source, "execution")?;
    validate_external_module(source, "gate_networks")
}

fn replace_once(source: &str, old: &str, new: &str) -> String {
    assert_eq!(
        source.matches(old).count(),
        1,
        "fixture mutation must be exact"
    );
    source.replacen(old, new, 1)
}

#[test]
fn production_qualification_call_paths_match_the_reviewed_contract() {
    let crate_root = include_str!("../src/lib.rs");
    validate_production_routing(crate_root).expect("production module routing contract");

    let gates = include_str!("../src/gate_networks.rs");
    validate_gate_contracts(gates).expect("gate qualification contract");

    let execution = include_str!("../src/execution.rs");
    validate_execution_contracts(execution).expect("instruction consumer contract");
}

#[test]
fn structural_contract_rejects_alternate_production_module_routing() {
    let crate_root = include_str!("../src/lib.rs");

    let path_alias = replace_once(
        crate_root,
        "mod execution;",
        "#[path = \"execution.rs\"]\nmod execution;",
    );
    syn::parse_file(&path_alias).expect("path-attribute fixture remains valid Rust syntax");
    assert!(validate_production_routing(&path_alias).is_err());

    let inline_replacement = replace_once(crate_root, "mod gate_networks;", "mod gate_networks {}");
    syn::parse_file(&inline_replacement).expect("inline fixture remains valid Rust syntax");
    assert!(validate_production_routing(&inline_replacement).is_err());

    let inactive_duplicate = replace_once(
        crate_root,
        "mod gate_networks;",
        "#[cfg(any())]\nmod gate_networks;\nmod gate_networks;",
    );
    syn::parse_file(&inactive_duplicate)
        .expect("inactive duplicate fixture remains valid Rust syntax");
    assert!(validate_production_routing(&inactive_duplicate).is_err());

    let dead_reviewed_module = replace_once(
        crate_root,
        "mod execution;",
        "#[cfg(any())]\nmod execution;\nmod execution {\n    pub fn exec_one() {}\n}",
    );
    syn::parse_file(&dead_reviewed_module)
        .expect("dead-reviewed-module fixture remains valid Rust syntax");
    assert!(validate_production_routing(&dead_reviewed_module).is_err());
}

#[test]
fn structural_contract_rejects_helper_shadow_and_iterator_bypasses() {
    let gates = include_str!("../src/gate_networks.rs");
    let unsigned_shadow = UNSIGNED_QUALIFIER.replace(
        "QualifiedWideResult {",
        "let any_high = host_nonzero(bits); QualifiedWideResult {",
    );
    let unsigned_sum = UNSIGNED_QUALIFIER.replace(
        ".iter()\n        .copied()\n        .fold(0, ::logic_gates::gates::or_gate)",
        ".iter().copied().sum::<u8>().min(1)",
    );
    let signed_shadow = SIGNED_QUALIFIER.replace(
        "QualifiedWideResult {",
        "let differs_from_sign = bits[64..].iter().filter(|bit| **bit != sign).count().min(1) as u8; QualifiedWideResult {",
    );
    assert!(
        validate_gate_contracts(&replace_once(gates, UNSIGNED_QUALIFIER, &unsigned_shadow))
            .is_err()
    );
    assert!(
        validate_gate_contracts(&replace_once(gates, UNSIGNED_QUALIFIER, &unsigned_sum)).is_err()
    );
    assert!(
        validate_gate_contracts(&replace_once(gates, SIGNED_QUALIFIER, &signed_shadow)).is_err()
    );
}

#[test]
fn structural_contract_rejects_dead_anchors_and_host_wide_consumers() {
    let execution = include_str!("../src/execution.rs");
    let multiply_bypass = execution.replacen(
        "let truncated = product.overflow != 0;",
        "let truncated = product.high.count_ones() != 0; let _anchor = product.overflow != 0;",
        1,
    );
    let divide_bypass = execution.replacen(
        "if division.quotient.overflow != 0 {",
        "let _anchor = division.quotient.overflow != 0; if division.quotient.high > 0 {",
        1,
    );
    assert_ne!(multiply_bypass, execution);
    assert_ne!(divide_bypass, execution);
    assert!(validate_execution_contracts(&multiply_bypass).is_err());
    assert!(validate_execution_contracts(&divide_bypass).is_err());
}

#[test]
fn structural_contract_rejects_inactive_decoy_and_active_duplicate() {
    let gates = include_str!("../src/gate_networks.rs");
    let inactive_decoy = replace_once(
        gates,
        "fn qualify_unsigned(bits: &[u8; 128]) -> QualifiedWideResult {",
        "#[cfg(any())]\nfn qualify_unsigned(bits: &[u8; 128]) -> QualifiedWideResult {",
    );
    let bypass = format!(
        r#"{inactive_decoy}
fn qualify_unsigned(bits: &[u8; 128]) -> QualifiedWideResult {{
    let low = bits_to_u64(&bits[..64]);
    let high = bits_to_u64(&bits[64..]);
    QualifiedWideResult {{
        low,
        high,
        overflow: u8::from(high != 0),
    }}
}}
"#
    );
    syn::parse_file(&bypass).expect("hostile fixture remains valid Rust syntax");
    let error = validate_gate_contracts(&bypass).expect_err("duplicate must be rejected");
    assert!(error.contains("exactly one top-level function named qualify_unsigned"));
}
