//! PIF-003a: the command line, the sign-off, the report and the small helpers.

mod common;

use common::*;
use pif_check::cli::{parse_args, run, Args};
use pif_check::control::{effective_chain, parse_chain, parse_signoff, Role};
use pif_check::json::Json;
use pif_check::{CheckError, Digest};

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| (*x).to_string()).collect()
}

/// A store with the prelude and a claim, and the chain file listing them.
fn setup() -> (Tmp, Vec<u8>, String, String) {
    let t = Tmp::new();
    let p = prelude();
    let c = claim_over_prelude(
        &[Digest::of(&p).hex()],
        "gates.and_comm",
        WORKED_DIGEST,
        "Pif.and_comm",
    );
    t.store(&p);
    t.store(c.as_bytes());
    let chain = format!(
        "pif-chain 1\nprelude sha256:{}\nclaim sha256:{}\n",
        Digest::of(&p).hex(),
        Digest::of(c.as_bytes()).hex()
    );
    let chain_path = t.put("chain.txt", chain.as_bytes());
    (t, p, c, chain_path)
}

#[test]
fn a_good_chain_exits_zero_with_a_json_report() {
    let (t, p, c, chain) = setup();
    let out = run(&s(&["--store", &t.0.to_string_lossy(), "--chain", &chain]));
    assert_eq!(out.code, 0, "{}", out.stdout);
    assert!(out.stderr.is_empty());
    assert!(out
        .stdout
        .starts_with("{\"ok\":true,\"base_environment\":\""));
    assert!(out
        .stdout
        .contains(&format!("\"digest\":\"{}\"", Digest::of(&p).hex())));
    assert!(out.stdout.contains(&format!(
        "\"digest\":\"{}\"",
        Digest::of(c.as_bytes()).hex()
    )));
    assert!(out.stdout.contains("\"claim\":\"gates.and_comm\""));
    assert!(out.stdout.contains("\"theorem\":\"Pif.and_comm\""));
    assert!(out.stdout.contains("\"allow_list\":\"not-provided\""));
    assert!(out.stdout.ends_with("}\n"));
    // A re-run is byte-identical: the report is a pure function of its inputs.
    let again = run(&s(&["--store", &t.0.to_string_lossy(), "--chain", &chain]));
    assert_eq!(out, again);
}

#[test]
fn an_allow_list_and_a_matching_signoff_are_honoured() {
    let (t, p, _c, chain) = setup();
    let base = Digest::env_start().fold(&Digest::of(&p));
    let allow = t.put(
        "allow.txt",
        format!("pif-allow 1\nsha256:{}\n", Digest::of(&p).hex()).as_bytes(),
    );
    let signoff = t.put(
        "signoff.txt",
        format!(
            "pif-signoff 1\nbase prelude sha256:{}\nbase-environment {}\n",
            Digest::of(&p).hex(),
            base.hex()
        )
        .as_bytes(),
    );
    let store = t.0.to_string_lossy().into_owned();
    let out = run(&s(&[
        "--store",
        &store,
        "--chain",
        &chain,
        "--signoff",
        &signoff,
        "--allow",
        &allow,
    ]));
    assert_eq!(out.code, 0, "{}", out.stdout);
    assert!(out.stdout.contains("\"allow_list\":\"checked\""));
    // A sign-off recording a different base environment digest is refused (E6).
    let bad = t.put(
        "signoff2.txt",
        format!(
            "pif-signoff 1\nbase prelude sha256:{}\nbase-environment {}\n",
            Digest::of(&p).hex(),
            Digest::of(b"other").hex()
        )
        .as_bytes(),
    );
    let out = run(&s(&[
        "--store",
        &store,
        "--chain",
        &chain,
        "--signoff",
        &bad,
    ]));
    assert_eq!(out.code, 1);
    assert!(out
        .stdout
        .contains("\"error\":\"SignoffBaseEnvironmentMismatch\""));
    // A sign-off whose base differs from the chain file's base is refused (E1).
    let other = t.put(
        "signoff3.txt",
        format!(
            "pif-signoff 1\nbase prelude sha256:{}\n",
            Digest::of(b"other").hex()
        )
        .as_bytes(),
    );
    let both = t.put(
        "chain2.txt",
        format!(
            "pif-chain 1\nprelude sha256:{}\nclaim sha256:{}\n",
            Digest::of(&p).hex(),
            Digest::of(b"c").hex()
        )
        .as_bytes(),
    );
    let out = run(&s(&[
        "--store",
        &store,
        "--chain",
        &both,
        "--signoff",
        &other,
    ]));
    assert!(
        out.stdout.contains("\"error\":\"SignoffChainMismatch\""),
        "{}",
        out.stdout
    );
}

#[test]
fn the_base_comes_from_the_signoff_when_the_chain_lists_only_the_claim() {
    let (t, p, c, _) = setup();
    let only_claim = t.put(
        "claimonly.txt",
        format!(
            "pif-chain 1\nclaim sha256:{}\n",
            Digest::of(c.as_bytes()).hex()
        )
        .as_bytes(),
    );
    let signoff = t.put(
        "so.txt",
        format!(
            "pif-signoff 1\nbase prelude sha256:{}\n",
            Digest::of(&p).hex()
        )
        .as_bytes(),
    );
    let out = run(&s(&[
        "--store",
        &t.0.to_string_lossy(),
        "--chain",
        &only_claim,
        "--signoff",
        &signoff,
    ]));
    assert_eq!(out.code, 0, "{}", out.stdout);
}

#[test]
fn failures_exit_one_with_the_exact_error_code() {
    let (t, p, c, _) = setup();
    let store = t.0.to_string_lossy().into_owned();
    let run_chain = |text: String| {
        let path = t.put("c.txt", text.as_bytes());
        run(&s(&["--store", &store, "--chain", &path]))
    };
    let (dp, dc) = (Digest::of(&p).hex(), Digest::of(c.as_bytes()).hex());
    // missing file
    let o = run_chain(format!(
        "pif-chain 1\nprelude sha256:{}\n",
        Digest::of(b"nope").hex()
    ));
    assert_eq!(
        (o.code, o.stdout.contains("\"error\":\"FileMissing\"")),
        (1, true),
        "{}",
        o.stdout
    );
    // claim before the prelude / claim not last
    let o = run_chain(format!(
        "pif-chain 1\nclaim sha256:{dc}\nprelude sha256:{dp}\n"
    ));
    assert!(o.stdout.contains("\"error\":\"ClaimNotLast\""));
    // reordered roles
    let o = run_chain(format!(
        "pif-chain 1\nadl sha256:{dp}\nprelude sha256:{dp}\n"
    ));
    assert!(o.stdout.contains("\"error\":\"PreludeNotFirst\""));
    // a reader rejection names the file and the rule
    let o = run_chain(format!(
        "pif-chain 1\nprelude sha256:{dp}\nadl sha256:{dp}\n"
    ));
    assert!(o.stdout.contains("\"error\":\"ReadRejected\""));
    assert!(o.stdout.contains("ImportMismatch"), "{}", o.stdout);
    // a malformed chain file
    let o = run_chain("pif-chain 1\nprelude nonsense\n".to_string());
    assert!(o.stdout.contains("\"error\":\"BadControlFile\""));
    let o = run_chain("pif-chain 1\n".to_string());
    assert!(o.stdout.contains("\"error\":\"EmptyChain\""));
    let o = run_chain("pif-chain 1\nwidget sha256:aa\n".to_string());
    assert!(o.stdout.contains("\"error\":\"BadControlFile\""));
}

#[test]
fn usage_errors_exit_two() {
    assert_eq!(run(&s(&[])).code, 2);
    assert_eq!(run(&s(&["--store", "x"])).code, 2);
    assert_eq!(run(&s(&["--store", "x", "--chain"])).code, 2);
    assert_eq!(
        run(&s(&["--store", "x", "--store", "y", "--chain", "z"])).code,
        2
    );
    assert_eq!(run(&s(&["--frob", "x"])).code, 2);
    let o = run(&s(&["--store", "x", "--chain", "z", "--signoff"]));
    assert_eq!(o.code, 2);
    assert!(o.stderr.contains("usage:"));
    assert_eq!(
        parse_args(&s(&["--store", "a", "--chain", "b", "--allow", "c"])),
        Ok(Args {
            store: "a".into(),
            chain: "b".into(),
            signoff: None,
            allow: Some("c".into())
        })
    );
}

#[test]
fn an_unreadable_or_non_utf8_control_file_is_a_refusal_not_a_panic() {
    let t = Tmp::new();
    let o = run(&s(&["--store", "x", "--chain", &t.path("missing.txt")]));
    assert_eq!(o.code, 1);
    assert!(o.stdout.contains("BadControlFile"));
    let bin = t.put("bin.txt", &[0xff, 0xfe, 0x00]);
    let o = run(&s(&["--store", "x", "--chain", &bin]));
    assert!(o.stdout.contains("is not UTF-8"));
    // A directory is not a readable file.
    let o = run(&s(&["--store", "x", "--chain", &t.0.to_string_lossy()]));
    assert_eq!(o.code, 1);
}

#[test]
fn the_binary_reports_through_its_exit_status() {
    let (t, _p, _c, chain) = setup();
    let bin = env!("CARGO_BIN_EXE_pif-check");
    let ok = std::process::Command::new(bin)
        .args(["--store", &t.0.to_string_lossy(), "--chain", &chain])
        .output()
        .unwrap();
    assert_eq!(ok.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&ok.stdout).starts_with("{\"ok\":true"));
    let bad = std::process::Command::new(bin)
        .args(["--store", "nowhere", "--chain", &t.path("none")])
        .output()
        .unwrap();
    assert_eq!(bad.status.code(), Some(1));
    let usage = std::process::Command::new(bin).output().unwrap();
    assert_eq!(usage.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&usage.stderr).contains("usage:"));
}

#[test]
fn signoff_parsing_and_effective_chain() {
    let (d1, d2) = (Digest::of(b"1").hex(), Digest::of(b"2").hex());
    let so = parse_signoff(&format!(
        "pif-signoff 1\nbase prelude {d1}\nbase model sha256:{d2}\nbase-environment {d1}\ngoal a.b {d2}\ngoals-sha256 {d1}\n"
    ))
    .unwrap();
    assert_eq!(so.base.len(), 2);
    assert_eq!(so.base[1].role, Role::Model);
    assert_eq!(
        so.goals,
        vec![("a.b".to_string(), Digest::parse(&d2).unwrap())]
    );
    assert_eq!(so.goals_sha256, Digest::parse(&d1));
    // The sign-off never records a claim file in its base.
    assert_eq!(
        parse_signoff(&format!("pif-signoff 1\nbase claim {d1}\n")).err(),
        Some(CheckError::ClaimNotLast)
    );
    for bad in [
        format!("pif-signoff 1\nbase widget {d1}\n"),
        format!("pif-signoff 1\nbase-environment {d1}\nbase-environment {d1}\n"),
        format!("pif-signoff 1\ngoals-sha256 {d1}\ngoals-sha256 {d1}\n"),
        "pif-signoff 1\nwhat\n".to_string(),
        "pif-signoff 1\ngoal a\n".to_string(),
    ] {
        assert_eq!(
            parse_signoff(&bad).err().map(|e| e.code()),
            Some("BadControlFile"),
            "{bad}"
        );
    }
    let many: String = (0..65).map(|_| format!("base adl {d1}\n")).collect();
    assert_eq!(
        parse_signoff(&format!("pif-signoff 1\n{many}")).err(),
        Some(CheckError::ChainTooLong)
    );
    let chain = parse_chain(&format!("pif-chain 1\nprelude {d1}\nmodel {d2}\n")).unwrap();
    assert_eq!(effective_chain(chain.clone(), None).unwrap(), chain);
    let many: String = (0..66).map(|_| format!("adl {d1}\n")).collect();
    assert_eq!(
        parse_chain(&format!("pif-chain 1\n{many}")).err(),
        Some(CheckError::ChainTooLong)
    );
}

#[test]
fn digests_parse_one_spelling_and_fold_as_documented() {
    let d = Digest::of(b"abc");
    assert_eq!(
        d.hex(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(Digest::parse(&format!("sha256:{}", d.hex())), Some(d));
    assert_eq!(Digest::parse(&d.hex()), Some(d));
    assert_eq!(d.to_string(), d.hex());
    for bad in [
        "",
        "sha256:",
        "sha256:sha256:ba78",
        &d.hex().to_uppercase(),
        &d.hex()[1..],
        "sha256: ".trim(),
    ] {
        assert_eq!(Digest::parse(bad), None, "{bad}");
    }
    let g = format!("{}g", &d.hex()[..63]);
    assert_eq!(Digest::parse(&g), None);
    // H0 and the fold: sha256("pif-env-1\n") and sha256(H0 || file) over raw bytes.
    assert_eq!(Digest::env_start(), Digest::of(b"pif-env-1\n"));
    let mut raw = Digest::env_start().0.to_vec();
    raw.extend_from_slice(&d.0);
    assert_eq!(Digest::env_start().fold(&d), Digest::of(&raw));
}

#[test]
fn json_is_ascii_escaped_and_ordered() {
    let j = Json::Obj(vec![
        ("a", Json::s("q\"\\\n\u{1}\u{e9}\u{1F600}")),
        (
            "b",
            Json::Arr(vec![Json::Num(7), Json::Bool(false), Json::Bool(true)]),
        ),
    ]);
    assert_eq!(
        j.render(),
        "{\"a\":\"q\\\"\\\\\\u000a\\u0001\\u00e9\\ud83d\\ude00\",\"b\":[7,false,true]}"
    );
}

#[test]
fn error_details_are_one_line_and_every_code_is_distinct() {
    use lean_kernel::pif::PifError;
    let errs = vec![
        CheckError::BadDigestSyntax("x".into()),
        CheckError::FileMissing("d".into()),
        CheckError::DigestMismatch {
            expected: "a".into(),
            actual: "b".into(),
        },
        CheckError::FileTooLarge("d".into()),
        CheckError::NotRegularFile("d".into()),
        CheckError::Io {
            digest: "d".into(),
            kind: "k".into(),
        },
        CheckError::BadControlFile {
            what: "chain",
            line: 1,
            why: "w",
        },
        CheckError::EmptyChain,
        CheckError::ChainTooLong,
        CheckError::PreludeNotFirst,
        CheckError::ClaimNotLast,
        CheckError::RoleOrder { index: 1 },
        CheckError::DuplicateRole { index: 1 },
        CheckError::NotAllowListed {
            index: 0,
            digest: "d".into(),
        },
        CheckError::SignoffChainMismatch,
        CheckError::SignoffBaseEnvironmentMismatch {
            expected: "a".into(),
            actual: "b".into(),
        },
        CheckError::Read {
            index: 0,
            role: "prelude",
            error: PifError::ImportMismatch,
        },
    ];
    let codes: std::collections::BTreeSet<_> = errs.iter().map(CheckError::code).collect();
    assert_eq!(codes.len(), errs.len());
    assert!(errs
        .iter()
        .all(|e| !e.detail().contains('\n') && !e.detail().is_empty()));
}
