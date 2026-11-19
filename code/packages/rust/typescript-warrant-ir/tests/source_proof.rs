//! Source-bound smoke proof: actual TypeScript bytes -> real AST -> typed WIR -> SAT.
//!
//! The reference below is a per-output-bit mux. It is intentionally independent
//! of the source operator tree, and mutation tests prove that supported wrong
//! operators reach the same solver instead of being normalized away.

use bit_blaster::bv::{self, BitVec};
use coding_adventures_sha256::sha256_hex;
use coding_adventures_typescript_lexer::tokenize_typescript;
use coding_adventures_warrant_ir::sym::zero_extend;
use coding_adventures_warrant_ir::{
    BinaryOp, Program, Prover, SymMemory, SymVal, UnaryOp, VerificationVerdict, WirExpr,
    WirTerminator,
};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use typescript_warrant_ir::{lower_typescript_bytes, BridgeError};

const SOURCE: &[u8] = include_bytes!("fixtures/digital.ts");
const SOURCE_SHA256: &str = "097006897b58787ab52b30c8cf6c9e23d7974944d6f6914357fc935218975c7c";

#[derive(Clone, Copy)]
enum Helper {
    BitIsSet,
    SetBit,
    ClearBit,
}

impl Helper {
    fn name(self) -> &'static str {
        match self {
            Self::BitIsSet => "bit_is_set",
            Self::SetBit => "set_bit",
            Self::ClearBit => "clear_bit",
        }
    }
}

fn prove(program: &Program, helper: Helper) -> VerificationVerdict {
    let function = program
        .find_function(helper.name())
        .expect("the exact source helper must lower");
    let mut prover = Prover::new();
    let mask = prover.executor.create_input_bits("mask", 64);
    let slot6 = prover.executor.create_input_bits("slot", 6);
    let slot8 = zero_extend(&slot6, 8);

    prover.prove_equivalence(
        function,
        vec![SymVal::Bits(mask), SymVal::Bits(slot8)],
        SymMemory::new(),
        |aig, args, _| {
            let mask = args[0].to_bits()?;
            let slot = args[1].to_bits()?;
            let mut outputs = Vec::with_capacity(64);
            for bit in 0..64 {
                // Select each output bit by an independent slot equality. The
                // reference never shifts by the source's symbolic slot.
                let selected = bv::eq(aig, &slot, &BitVec::constant(8, bit as u128));
                let held = mask.bits[bit];
                match helper {
                    Helper::BitIsSet => outputs.push(aig.and(selected, held)),
                    Helper::SetBit => outputs.push(aig.or(selected, held)),
                    Helper::ClearBit => outputs.push(aig.and(!selected, held)),
                }
            }
            match helper {
                Helper::BitIsSet => Ok(SymVal::Bool(aig.or_all(&outputs))),
                Helper::SetBit | Helper::ClearBit => Ok(SymVal::Bits(BitVec { bits: outputs })),
            }
        },
    )
}

#[test]
fn fixture_bytes_parse_lower_typecheck_and_prove_all_three_helpers() {
    let expected_path = "code/packages/rust/typescript-warrant-ir/tests/fixtures/digital.ts";
    assert_eq!(source_relative_path(), expected_path);
    validate_fixture_binding(&source_fixture_path(), SOURCE, expected_path, SOURCE_SHA256)
        .expect("fixture path, included bytes, and SHA-256 must bind before proof");
    let program = lower_typescript_bytes(SOURCE)
        .unwrap_or_else(|error| panic!("{} must parse and lower: {error}", source_relative_path()));
    assert_eq!(program.functions.len(), 3);
    assert_actual_source_operators(&program);
    for helper in [Helper::BitIsSet, Helper::SetBit, Helper::ClearBit] {
        assert_eq!(
            prove(&program, helper),
            VerificationVerdict::Proved,
            "{}",
            helper.name()
        );
    }
    runtime_domain_replay();
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BindingFailure {
    Path,
    Bytes,
    Digest,
}

/// Test-harness gate run before any source lowering or proof query.
fn validate_fixture_binding(
    path: &Path,
    bytes: &[u8],
    expected_path: &str,
    expected_digest: &str,
) -> Result<(), BindingFailure> {
    let relative = path
        .strip_prefix(repository_root())
        .map_err(|_| BindingFailure::Path)?
        .to_string_lossy()
        .replace('\\', "/");
    if relative != expected_path {
        return Err(BindingFailure::Path);
    }
    if fs::read(path)
        .map_err(|_| BindingFailure::Bytes)?
        .as_slice()
        != bytes
    {
        return Err(BindingFailure::Bytes);
    }
    if sha256_hex(bytes) != expected_digest {
        return Err(BindingFailure::Digest);
    }
    Ok(())
}

fn repository_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("Rust package has repository root")
        .to_path_buf()
}

fn source_fixture_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/digital.ts")
}

fn source_relative_path() -> String {
    source_fixture_path()
        .strip_prefix(repository_root())
        .expect("fixture is inside repository root")
        .to_string_lossy()
        .replace('\\', "/")
}

#[test]
fn broken_fixture_path_digest_and_bytes_refuse_before_lowering() {
    let path = source_fixture_path();
    let correct_path = source_relative_path();
    assert_eq!(
        validate_fixture_binding(&path, SOURCE, &correct_path, SOURCE_SHA256),
        Ok(())
    );

    let wrong_path = "code/packages/rust/typescript-warrant-ir/tests/fixtures/not-digital.ts";
    assert_eq!(
        validate_fixture_binding(&path, SOURCE, wrong_path, SOURCE_SHA256),
        Err(BindingFailure::Path)
    );
    assert_eq!(
        validate_fixture_binding(&path, SOURCE, &correct_path, "0".repeat(64).as_str()),
        Err(BindingFailure::Digest)
    );
    let mut wrong_bytes = SOURCE.to_vec();
    wrong_bytes[0] ^= 1;
    assert_eq!(
        validate_fixture_binding(&path, &wrong_bytes, &correct_path, SOURCE_SHA256),
        Err(BindingFailure::Bytes)
    );
}

fn assert_actual_source_operators(program: &Program) {
    let bit_is_set = program.find_function("bit_is_set").unwrap();
    assert!(matches!(
        &bit_is_set.blocks[0].terminator,
        WirTerminator::Return(Some(WirExpr::Binary {
            op: BinaryOp::Ne,
            ..
        }))
    ));
    let set_bit = program.find_function("set_bit").unwrap();
    assert!(matches!(
        &set_bit.blocks[0].terminator,
        WirTerminator::Return(Some(WirExpr::Binary {
            op: BinaryOp::BitOr,
            ..
        }))
    ));
    let clear_bit = program.find_function("clear_bit").unwrap();
    assert!(matches!(
        &clear_bit.blocks[0].terminator,
        WirTerminator::Return(Some(WirExpr::Binary {
            op: BinaryOp::BitAnd,
            rhs,
            ..
        })) if matches!(rhs.as_ref(), WirExpr::Unary { op: UnaryOp::Not, .. })
    ));
    let shift = |expression: &WirExpr| match expression {
        WirExpr::Binary {
            op: BinaryOp::Shl,
            lhs,
            rhs,
        } => {
            matches!(
                lhs.as_ref(),
                WirExpr::ConstInt {
                    val: 1,
                    ty: coding_adventures_warrant_ir::WirType::U64
                }
            ) && matches!(
                rhs.as_ref(),
                WirExpr::ZExt { expr, target_ty: coding_adventures_warrant_ir::WirType::U64 }
                    if matches!(expr.as_ref(), WirExpr::Var(var) if var.0 == "slot")
            )
        }
        _ => false,
    };
    match &bit_is_set.blocks[0].terminator {
        WirTerminator::Return(Some(WirExpr::Binary { lhs, .. })) => match lhs.as_ref() {
            WirExpr::Binary { rhs, .. } => {
                assert!(shift(rhs), "actual mask shift tree must be retained")
            }
            _ => panic!("bit_is_set must lower mask AND shift"),
        },
        _ => panic!("bit_is_set must lower its source comparison"),
    }
    match &set_bit.blocks[0].terminator {
        WirTerminator::Return(Some(WirExpr::Binary { rhs, .. })) => {
            assert!(shift(rhs), "actual set-bit shift tree must be retained")
        }
        _ => panic!("set_bit must lower its source OR"),
    }
    match &clear_bit.blocks[0].terminator {
        WirTerminator::Return(Some(WirExpr::Binary { rhs, .. })) => match rhs.as_ref() {
            WirExpr::Unary { expr, .. } => {
                assert!(shift(expr), "actual clear-bit shift tree must be retained")
            }
            _ => panic!("clear_bit must lower NOT of source shift"),
        },
        _ => panic!("clear_bit must lower its source AND"),
    }
}

#[test]
fn source_operator_mutants_reach_sat_and_replay_in_node() {
    let source = std::str::from_utf8(SOURCE).unwrap();
    let mutants = [
        (Helper::BitIsSet, "!== 0n", "=== 0n", "bit_is_set"),
        (Helper::SetBit, "return mask |", "return mask &", "set_bit"),
    ];
    for (helper, healthy, mutant, _) in mutants {
        let bad_source = source.replacen(healthy, mutant, 1);
        assert_ne!(bad_source, source, "the source mutation must apply");
        let program = lower_typescript_bytes(bad_source.as_bytes()).unwrap_or_else(|error| {
            panic!("supported source mutant must be admitted and lowered: {error}")
        });
        let (mask, slot) = match prove(&program, helper) {
            VerificationVerdict::Falsified { counterexample } => {
                let mut mask = None;
                let mut slot = None;
                for (name, value) in counterexample {
                    match name.as_str() {
                        "mask" => mask = Some(value),
                        "slot" => slot = Some(value),
                        _ => {}
                    }
                }
                (mask.expect("mask witness"), slot.expect("slot witness"))
            }
            other => panic!("{} source mutant must be SAT, got {other:?}", helper.name()),
        };
        assert!(mask <= u64::MAX as u128, "mask witness is full-width u64");
        assert!(slot <= 63, "slot witness is inside the six-bit domain");
        replay_in_node(bad_source.as_bytes(), helper, mask, slot, true);
        replay_in_node(SOURCE, helper, mask, slot, false);
        eprintln!(
            "SAT source mutant {}: path={} (mutated bytes), sha256={}, mask={}, slot={}, witness replayed against mutant and healthy bytes",
            helper.name(),
            source_relative_path(),
            sha256_hex(bad_source.as_bytes()),
            mask,
            slot,
        );
    }
}

fn replay_in_node(source: &[u8], helper: Helper, mask: u128, slot: u128, expected_diff: bool) {
    static RUN_ID: AtomicUsize = AtomicUsize::new(0);
    let root = (0..100)
        .find_map(|attempt| {
            let unique = RUN_ID.fetch_add(1, Ordering::Relaxed);
            let candidate = std::env::temp_dir().join(format!(
                "typescript-warrant-ir-{}-{unique}-{attempt}",
                std::process::id()
            ));
            match fs::create_dir(&candidate) {
                Ok(()) => Some(candidate),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => None,
                Err(error) => panic!("create owned Node replay directory: {error}"),
            }
        })
        .expect("find an unused Node replay directory name");
    let module = root.join("digital.ts");
    fs::write(&module, source).expect("write exact source or mutant bytes");
    let script = r#"
import { pathToFileURL } from 'node:url';
const api = await import(pathToFileURL(process.env.TS_WIR_MODULE).href);
const mask = BigInt(process.env.TS_WIR_MASK);
const slot = Number(process.env.TS_WIR_SLOT);
const digits = mask.toString(2).padStart(64, '0').split('');
const index = 63 - slot;
const selected = digits[index] === '1';
let expected;
if (process.env.TS_WIR_HELPER === 'bit_is_set') expected = String(selected);
else {
  if (process.env.TS_WIR_HELPER === 'set_bit') digits[index] = '1';
  else digits[index] = '0';
  expected = BigInt('0b' + digits.join('')).toString();
}

const actualValue = api[process.env.TS_WIR_HELPER](mask, slot);
const actual = typeof actualValue === 'bigint' ? actualValue.toString() : String(actualValue);
console.log(JSON.stringify({actual, expected, differs: actual !== expected}));
"#;
    let mut node = Command::new("node");
    node.env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("TS_WIR_MODULE", &module)
        .env("TS_WIR_HELPER", helper.name())
        .env("TS_WIR_MASK", mask.to_string())
        .env("TS_WIR_SLOT", slot.to_string());
    #[cfg(windows)]
    if let Some(system_root) = std::env::var_os("SystemRoot") {
        node.env("SystemRoot", system_root);
    }
    let output = node
        .args([
            "--experimental-strip-types",
            "--input-type=module",
            "-e",
            script,
        ])
        .output()
        .unwrap_or_else(|error| panic!("run Node source replay: {error}"));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    fs::remove_dir_all(&root).expect("remove only this test's temporary replay directory");
    assert!(output.status.success(), "Node replay failed: {stderr}");
    let replay: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|error| panic!("Node replay must emit JSON: {error}; stdout={stdout}"));
    let actual = replay["actual"].as_str().expect("actual result string");
    let expected = replay["expected"]
        .as_str()
        .expect("reference result string");
    let differs = replay["differs"].as_bool().expect("difference boolean");
    assert_eq!(
        actual != expected,
        differs,
        "JSON result fields agree: {stdout}"
    );
    assert_eq!(
        differs, expected_diff,
        "Node should replay actual bytes against the independent reference: {stdout}"
    );
}

fn runtime_domain_replay() {
    static DOMAIN_RUN_ID: AtomicUsize = AtomicUsize::new(0);
    let root = (0..100)
        .find_map(|attempt| {
            let unique = DOMAIN_RUN_ID.fetch_add(1, Ordering::Relaxed);
            let candidate = std::env::temp_dir().join(format!(
                "typescript-warrant-ir-domain-{}-{unique}-{attempt}",
                std::process::id()
            ));
            match fs::create_dir(&candidate) {
                Ok(()) => Some(candidate),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => None,
                Err(error) => panic!("create owned Node domain directory: {error}"),
            }
        })
        .expect("find an unused Node domain directory name");
    let module = root.join("digital.ts");
    fs::write(&module, SOURCE).expect("write exact checked-in TypeScript fixture bytes");
    let script = r#"
import { pathToFileURL } from 'node:url';
const api = await import(pathToFileURL(process.env.TS_WIR_MODULE).href);
let cases = 0;
for (let slot = 0; slot < 64; slot += 1) {
  for (const held of [false, true]) {
    const bits = Array(64).fill('0');
    bits[63 - slot] = held ? '1' : '0';
    const mask = BigInt('0b' + bits.join(''));
    const expectedSet = [...bits];
    const expectedClear = [...bits];
    expectedSet[63 - slot] = '1';
    expectedClear[63 - slot] = '0';
    const expectedHeld = BigInt('0b' + expectedSet.join(''));
    const expectedReleased = BigInt('0b' + expectedClear.join(''));
    if (api.bit_is_set(mask, slot) !== held
        || api.set_bit(mask, slot) !== expectedHeld
        || api.clear_bit(mask, slot) !== expectedReleased) {
      throw new Error(`public helper mismatch at slot=${slot}, held=${held}`);
    }
    cases += 1;
  }
}
console.log(JSON.stringify({cases, passed: true}));
"#;
    let mut node = Command::new("node");
    node.env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("TS_WIR_MODULE", &module);
    #[cfg(windows)]
    if let Some(system_root) = std::env::var_os("SystemRoot") {
        node.env("SystemRoot", system_root);
    }
    let output = node
        .args([
            "--experimental-strip-types",
            "--input-type=module",
            "-e",
            script,
        ])
        .output()
        .expect("run exhaustive Node helper domain replay");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    fs::remove_dir_all(&root).expect("remove only this test's temporary domain directory");
    assert!(
        output.status.success(),
        "Node domain replay failed: {stderr}"
    );
    let result: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|error| panic!("domain replay must emit JSON: {error}; stdout={stdout}"));
    assert_eq!(result["cases"], 128);
    assert_eq!(result["passed"], true);
}
#[test]
fn admission_rejects_nonfinite_bigint_literals_and_non_slot_shifts() {
    let source = std::str::from_utf8(SOURCE).unwrap();
    for (needle, replacement) in [
        ("1n << BigInt(slot)", "2n << BigInt(slot)"),
        ("1n << BigInt(slot)", "1n << BigInt(mask)"),
        ("1n << BigInt(slot)", "1n << slot"),
        ("1n << BigInt(slot)", "(1n << BigInt(slot)) << BigInt(slot)"),
        ("return mask | (1n", "return mask << BigInt(slot)"),
        (
            "mask & ~(1n << BigInt(slot))",
            "~(mask & (1n << BigInt(slot)))",
        ),
        (
            "export function set_bit",
            "const BigInt = Number;\nexport function set_bit",
        ),
        ("mask: bigint", "mask: 'bigint'"),
        ("return (mask &", "return (\"mask\" &"),
        ("BigInt(slot)", "BigInt(slot + 0)"),
    ] {
        let bad = source.replacen(needle, replacement, 1);
        assert_ne!(bad, source);
        assert!(
            matches!(
                lower_typescript_bytes(bad.as_bytes()),
                Err(BridgeError::UnsupportedAst(_)
                    | BridgeError::InvalidNotContext
                    | BridgeError::TypeCheck(_)
                    | BridgeError::Parse(_))
            ),
            "mutation {needle:?} -> {replacement:?} must refuse"
        );
    }
}

#[test]
fn public_source_refusal_matrix_covers_unsupported_program_families() {
    let source = std::str::from_utf8(SOURCE).unwrap();
    let cases = [
        ("import", "import { external } from 'external';\n"),
        ("reexport", "\nexport { bit_is_set };"),
        ("ambient", "declare function external(): bigint;\n"),
        (
            "extra function",
            "\nexport function extra(): bigint { return 0n; }",
        ),
        ("duplicate helper", "export function set_bit"),
        ("shadow BigInt", "export function set_bit"),
        ("rebind BigInt", "BigInt = Number;\n"),
        ("alias BigInt", "const convert = BigInt;\n"),
    ];
    for (name, addition) in cases {
        let bad = match name {
            "duplicate helper" => {
                source.replacen("export function set_bit", "export function bit_is_set", 1)
            }
            "shadow BigInt" => format!("const BigInt = Number;\n{source}"),
            "reexport" | "extra function" => format!("{source}{addition}"),
            _ => format!("{addition}{source}"),
        };
        assert_ne!(bad, source, "{name} mutation must apply");
        assert!(
            lower_typescript_bytes(bad.as_bytes()).is_err(),
            "{name} must refuse"
        );
    }

    let replacements = [
        ("return (mask &", "return (missing &"),
        ("BigInt(slot)", "unknownConversion(slot)"),
        ("BigInt(slot)", "Number = BigInt(slot)"),
        ("(mask & (1n", "((mask as bigint) & (1n"),
        ("return (mask &", "return (mask! &"),
        (
            "return mask | (1n << BigInt(slot));",
            "let copy = mask; return copy | (1n << BigInt(slot));",
        ),
        (
            "return mask | (1n << BigInt(slot));",
            "if (slot) { return mask; } return mask | (1n << BigInt(slot));",
        ),
        (
            "return mask | (1n << BigInt(slot));",
            "for (;;) { } return mask | (1n << BigInt(slot));",
        ),
        (
            "mask: bigint, slot: number): bigint",
            "mask: bigint, slot: number)",
        ),
        (
            "mask: bigint, slot: number): bigint",
            "mask: bigint, slot: number): boolean",
        ),
        ("mask: bigint, slot: number", "mask: bigint, key: number"),
        ("mask & (1n << BigInt(slot))", "Number(mask) & Number(slot)"),
    ];
    for (index, (needle, replacement)) in replacements.into_iter().enumerate() {
        let bad = source.replacen(needle, replacement, 1);
        assert_ne!(bad, source, "source refusal case {index} must mutate bytes");
        assert!(
            lower_typescript_bytes(bad.as_bytes()).is_err(),
            "public unsupported source case {index} must refuse"
        );
    }
}

#[test]
fn return_line_terminators_are_refused_even_when_the_parser_accepts_them() {
    let source = std::str::from_utf8(SOURCE).unwrap();
    for (terminator, separator) in [
        ("LF", "\n"),
        ("CR", "\r"),
        ("CRLF", "\r\n"),
        ("comment-LF", "/*\n*/"),
        ("comment-CR", "/*\r*/"),
    ] {
        let bad = source.replacen("return (mask", &format!("return{separator} (mask"), 1);
        assert_ne!(bad, source);
        assert!(
            coding_adventures_typescript_parser::parse_typescript(&bad, "ts5.8").is_ok(),
            "test confirms upstream grammar parser acceptance for {terminator}"
        );
        assert!(
            matches!(
                lower_typescript_bytes(bad.as_bytes()),
                Err(BridgeError::RestrictedProductionLineTerminator)
            ),
            "return line terminator {terminator} must not be lowered as an expression"
        );
    }
    for terminator in ['\u{2028}', '\u{2029}'] {
        let bad = source.replacen("return (mask", &format!("return{terminator}(mask"), 1);
        assert!(matches!(
            lower_typescript_bytes(bad.as_bytes()),
            Err(BridgeError::Parse(_))
        ));
    }
    let unicode_comment = source.replacen("return (mask", "return /* λ */ (mask", 1);
    assert!(lower_typescript_bytes(unicode_comment.as_bytes()).is_ok());
    let unicode_before_return = source.replacen("return (mask", "/* λ */ return (mask", 1);
    assert!(lower_typescript_bytes(unicode_before_return.as_bytes()).is_ok());
}

#[test]
fn top_level_extra_statements_and_token_budget_are_refused() {
    let source = std::str::from_utf8(SOURCE).unwrap();
    let trailing = format!("{source}\nconst extra = 0n;");
    assert!(matches!(
        lower_typescript_bytes(trailing.as_bytes()),
        Err(BridgeError::UnsupportedAst(_))
    ));
    let token_heavy = format!("{source}{}", ";".repeat(64));
    assert!(matches!(
        lower_typescript_bytes(token_heavy.as_bytes()),
        Err(BridgeError::TooManyTokens { .. })
    ));
    let oversized = vec![b' '; typescript_warrant_ir::MAX_SOURCE_BYTES + 1];
    assert!(matches!(
        lower_typescript_bytes(&oversized),
        Err(BridgeError::SourceTooLarge { .. })
    ));
}

#[test]
fn exact_source_and_token_caps_reach_the_next_pipeline_stage() {
    let mut exact_bytes = SOURCE.to_vec();
    exact_bytes.resize(typescript_warrant_ir::MAX_SOURCE_BYTES, b' ');
    assert_eq!(exact_bytes.len(), typescript_warrant_ir::MAX_SOURCE_BYTES);
    assert!(lower_typescript_bytes(&exact_bytes).is_ok());

    let source = std::str::from_utf8(SOURCE).unwrap();
    let initial_tokens = tokenize_typescript(source, typescript_warrant_ir::TYPESCRIPT_VERSION)
        .unwrap()
        .len();
    assert!(initial_tokens < typescript_warrant_ir::MAX_TOKENS);
    let at_cap = format!(
        "{source}{}",
        ";".repeat(typescript_warrant_ir::MAX_TOKENS - initial_tokens)
    );
    assert_eq!(
        tokenize_typescript(&at_cap, typescript_warrant_ir::TYPESCRIPT_VERSION)
            .unwrap()
            .len(),
        typescript_warrant_ir::MAX_TOKENS
    );
    assert!(!matches!(
        lower_typescript_bytes(at_cap.as_bytes()),
        Err(BridgeError::TooManyTokens { .. })
    ), "exactly MAX_TOKENS must pass the token-budget gate even though the added top-level statements are later refused");

    let over_cap = format!("{at_cap};");
    assert!(matches!(
        lower_typescript_bytes(over_cap.as_bytes()),
        Err(BridgeError::TooManyTokens {
            actual,
            limit: typescript_warrant_ir::MAX_TOKENS
        }) if actual == typescript_warrant_ir::MAX_TOKENS + 1
    ));
}

#[test]
fn source_path_is_repository_relative_and_has_no_partial_mutation() {
    assert!(source_relative_path().starts_with("code/packages/rust/typescript-warrant-ir/"));
    let before = SOURCE.to_vec();
    let mut invalid = before.clone();
    invalid.extend(std::iter::repeat_n(
        b' ',
        typescript_warrant_ir::MAX_SOURCE_BYTES + 1,
    ));
    assert!(matches!(
        lower_typescript_bytes(&invalid),
        Err(BridgeError::SourceTooLarge { .. })
    ));
    assert_eq!(SOURCE, before.as_slice());
}

#[test]
fn lowered_programs_are_one_return_each() {
    let program = lower_typescript_bytes(SOURCE).unwrap();
    for function in program.functions {
        assert!(matches!(
            function.blocks[0].terminator,
            WirTerminator::Return(Some(WirExpr::Binary { .. }))
        ));
    }
}
