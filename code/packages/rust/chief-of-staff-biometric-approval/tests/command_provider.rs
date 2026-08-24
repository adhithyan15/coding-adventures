use chief_of_staff_biometric_approval::{BiometricApprovalError, BiometricCommandProvider};
use chief_of_staff_tool_api::{ApprovalAssurance, PrivilegeTier};
use chief_of_staff_trust_checker::{
    AuthorizationBasis, TrustChecker, TrustCheckerError, TrustRequest, TrustResource,
};
use std::env;
use std::ffi::OsStr;
use std::io::{self, BufRead, Write};
use std::process;
use std::thread;
use std::time::{Duration, Instant};

fn main() {
    if env::var_os("CHIEF_APPROVAL_PROTOCOL").as_deref() == Some("2".as_ref()) {
        helper_main();
        return;
    }
    run_parent_tests();
}

fn run_parent_tests() {
    let executable = env::current_exe().expect("test executable is available");
    let approved = TrustChecker::new(BiometricCommandProvider::new(executable.clone()).unwrap())
        .authorize(&request("approve"))
        .unwrap();
    assert_eq!(
        approved.basis(),
        AuthorizationBasis::Approved(ApprovalAssurance::Biometric)
    );
    let denied = TrustChecker::new(BiometricCommandProvider::new(executable.clone()).unwrap())
        .authorize(&request("deny"));
    assert!(matches!(denied, Err(TrustCheckerError::Denied)));
    for request_id in ["weak", "malformed", "exit"] {
        let result = TrustChecker::new(BiometricCommandProvider::new(executable.clone()).unwrap())
            .authorize(&request(request_id));
        assert!(matches!(
            result,
            Err(TrustCheckerError::Provider(
                BiometricApprovalError::InvalidResponse
            ))
        ));
    }
    let oversized = TrustChecker::new(BiometricCommandProvider::new(executable.clone()).unwrap())
        .authorize(&request("oversized"));
    assert!(matches!(
        oversized,
        Err(TrustCheckerError::Provider(
            BiometricApprovalError::ResponseReadFailed
        ))
    ));

    let bulk = bulk_request("approve");
    let bulk_receipt =
        TrustChecker::new(BiometricCommandProvider::new(executable.clone()).unwrap())
            .authorize(&bulk)
            .unwrap();
    assert_eq!(
        bulk_receipt.basis(),
        AuthorizationBasis::Approved(ApprovalAssurance::Biometric)
    );

    let timeout_request = request("timeout");
    let started = Instant::now();
    let timeout = TrustChecker::new(BiometricCommandProvider::new(executable).unwrap())
        .authorize(&timeout_request);
    assert!(matches!(timeout, Err(TrustCheckerError::TimedOut)));
    assert!(started.elapsed() >= Duration::from_secs(29));
    assert!(started.elapsed() < Duration::from_secs(40));

    let missing = missing_executable();
    let spawn_result = TrustChecker::new(BiometricCommandProvider::new(missing).unwrap())
        .authorize(&request("approve"));
    assert!(matches!(
        spawn_result,
        Err(TrustCheckerError::Provider(
            BiometricApprovalError::SpawnFailed
        ))
    ));

    let tier3 = TrustRequest::new(
        "approve",
        "operator:local",
        vec![TrustResource::new("resource", PrivilegeTier::Tier3).unwrap()],
    )
    .unwrap();
    let executable = env::current_exe().unwrap();
    let result =
        TrustChecker::new(BiometricCommandProvider::new(executable).unwrap()).authorize(&tier3);
    assert!(matches!(
        result,
        Err(TrustCheckerError::Provider(
            BiometricApprovalError::UnsupportedRequirement
        ))
    ));
}

fn request(request_id: &str) -> TrustRequest {
    TrustRequest::new(
        request_id,
        "operator:local",
        vec![TrustResource::new("package:bank", PrivilegeTier::Tier2).unwrap()],
    )
    .unwrap()
}

fn bulk_request(request_id: &str) -> TrustRequest {
    TrustRequest::new(
        request_id,
        "operator:local",
        (0..1_026)
            .map(|index| {
                TrustResource::new(
                    format!("resource:{index:04}:{}", "x".repeat(300)),
                    if index == 1_025 {
                        PrivilegeTier::Tier2
                    } else {
                        PrivilegeTier::Tier0
                    },
                )
                .unwrap()
            })
            .collect(),
    )
    .unwrap()
}

/// The helper must observe ONLY the protocol variable the provider hands it.
///
/// `src/lib.rs` calls `Command::env_clear()` before spawning precisely so that a
/// Tier 2 biometric helper can never read the daemon's secrets, tokens, or paths.
/// This assertion is the test that guards that property: if `env_clear()` ever
/// regressed, `PATH`, `HOME`, and every `CARGO_*` variable would show up here
/// and trip it.
///
/// Exactly one additional name is tolerated, and only because the child sets it
/// on ITSELF. When this crate is built with `-C instrument-coverage` -- which is
/// what `cargo tarpaulin --engine llvm` does in order to measure coverage of
/// this very file -- compiler-rt's profile runtime runs a startup constructor
/// that calls `setenv("__LLVM_PROFILE_RT_INIT_ONCE", ...)` as a one-time-init
/// guard. That happens in the child, after `exec`, so no `env_clear()` on the
/// parent side can suppress it, and its presence is not evidence that the
/// parent's environment leaked. Uninstrumented production builds never link
/// that runtime and never contain the variable.
///
/// Tolerating this one exact name keeps the security assertion intact: any
/// genuinely inherited variable still fails the check.
fn is_expected_helper_variable(name: &OsStr) -> bool {
    name == "CHIEF_APPROVAL_PROTOCOL" || name == "__LLVM_PROFILE_RT_INIT_ONCE"
}

fn helper_main() {
    assert!(env::vars_os().all(|(name, _)| is_expected_helper_variable(&name)));
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    assert_eq!(
        lines.next().transpose().unwrap().as_deref(),
        Some("CHIEF-TIER2-BIOMETRIC/1")
    );
    let request_line = lines.next().transpose().unwrap().unwrap();
    let request_id = decode_hex(request_line.strip_prefix("request_id ").unwrap());
    while lines.next().transpose().unwrap().as_deref() != Some("end") {}
    print_decision("ready\n");
    match request_id.as_str() {
        "approve" => print_decision("approve biometric\n"),
        "deny" => print_decision("deny\n"),
        "weak" => print_decision("approve\n"),
        "malformed" => print_decision("maybe\n"),
        "oversized" => print_decision("xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx\n"),
        "exit" => {}
        "timeout" => thread::sleep(Duration::from_secs(45)),
        _ => process::exit(2),
    }
}

fn print_decision(decision: &str) {
    let mut stdout = io::stdout().lock();
    stdout.write_all(decision.as_bytes()).unwrap();
    stdout.flush().unwrap();
}

fn decode_hex(encoded: &str) -> String {
    let bytes = encoded
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| (hex_nibble(pair[0]) << 4) | hex_nibble(pair[1]))
        .collect::<Vec<_>>();
    String::from_utf8(bytes).unwrap()
}

fn hex_nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => panic!("non-canonical hex"),
    }
}

#[cfg(unix)]
fn missing_executable() -> std::path::PathBuf {
    std::path::PathBuf::from("/definitely/missing/chief-biometric-helper")
}

#[cfg(windows)]
fn missing_executable() -> std::path::PathBuf {
    std::path::PathBuf::from(r"C:\definitely\missing\chief-biometric-helper.exe")
}
