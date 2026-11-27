//! Security-review regressions for PIF-003a: vetting of the base, partial sign-offs, duplicate
//! goal ids, claim-less chains, role-bound allow-lists, withdrawn logic versions and control
//! files that are not regular files.

mod common;

use common::*;
use pif_check::cli::run;
use pif_check::control::{parse_allow, parse_signoff, Role};
use pif_check::{CheckError, Digest};

fn raw(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| (*x).to_string()).collect()
}

struct W {
    t: Tmp,
    store: String,
    chain: String,
    prelude_hex: String,
    base_env: String,
}

fn world() -> W {
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
    let chain = t.put(
        "chain.txt",
        format!(
            "pif-chain 1\nprelude sha256:{}\nclaim sha256:{}\n",
            Digest::of(&p).hex(),
            Digest::of(c.as_bytes()).hex()
        )
        .as_bytes(),
    );
    let store = t.0.to_string_lossy().into_owned();
    W {
        t,
        store,
        chain,
        prelude_hex: Digest::of(&p).hex(),
        base_env: Digest::env_start().fold(&Digest::of(&p)).hex(),
    }
}

fn signoff_text(w: &W) -> String {
    let agg =
        Digest::of(format!("gates.and_comm {WORKED_DIGEST} {}\n", w.base_env).as_bytes()).hex();
    format!(
        "pif-signoff 1\nbase prelude sha256:{}\nbase-environment {}\ngoal gates.and_comm {WORKED_DIGEST}\ngoals-sha256 {agg}\n",
        w.prelude_hex, w.base_env
    )
}

#[test]
fn finding_1_an_unvetted_base_is_refused_unless_explicitly_allowed() {
    let w = world();
    let base = ["--store", w.store.as_str(), "--chain", w.chain.as_str()];
    let with = |extra: &[&str]| raw(&[&base[..], extra].concat());
    // No allow-list, no sign-off: exit 3, ok false, nothing loaded.
    let out = run(&with(&[]));
    assert_eq!(out.code, 3, "{}", out.stdout);
    assert!(
        out.stdout
            .starts_with("{\"ok\":false,\"error\":\"Unvetted\""),
        "{}",
        out.stdout
    );
    // Explicitly unsafe: runs, and says so.
    let out = run(&with(&["--unsafe-no-allow"]));
    assert_eq!(out.code, 0, "{}", out.stdout);
    assert!(out.stdout.contains("\"base_trust\":\"unvetted\""));
    // Allow-listed.
    let allow = w.t.put(
        "allow.txt",
        format!("pif-allow 1\nprelude sha256:{}\n", w.prelude_hex).as_bytes(),
    );
    let out = run(&with(&["--allow", &allow]));
    assert_eq!(out.code, 0, "{}", out.stdout);
    assert!(out.stdout.contains("\"base_trust\":\"allow-listed\""));
    // Signed.
    let so = w.t.put("so.txt", signoff_text(&w).as_bytes());
    let out = run(&with(&["--signoff", &so]));
    assert_eq!(out.code, 0, "{}", out.stdout);
    assert!(out.stdout.contains("\"base_trust\":\"signed\""));
    // The refusal comes before any file is read: a missing chain file is still exit 3.
    let out = run(&raw(&["--store", "x", "--chain", "/nonexistent"]));
    assert_eq!(out.code, 3);
    // The flag may not be repeated.
    assert_eq!(
        run(&raw(&[
            "--store",
            "x",
            "--chain",
            "y",
            "--unsafe-no-allow",
            "--unsafe-no-allow"
        ]))
        .code,
        2
    );
}

#[test]
fn finding_2_a_partial_signoff_is_refused_and_the_report_says_what_was_checked() {
    let w = world();
    let full = signoff_text(&w);
    for drop in ["base-environment", "goal ", "goals-sha256"] {
        let partial: String = full
            .lines()
            .filter(|l| !l.starts_with(drop))
            .map(|l| format!("{l}\n"))
            .collect();
        let e = parse_signoff(&partial).err().unwrap();
        assert_eq!(e.code(), "BadControlFile", "{drop}");
    }
    // Only `base` lines (the review's example).
    let only_base = format!("pif-signoff 1\nbase prelude sha256:{}\n", w.prelude_hex);
    assert_eq!(
        parse_signoff(&only_base).err().map(|e| e.code()),
        Some("BadControlFile")
    );
    let so = w.t.put("only-base.txt", only_base.as_bytes());
    let out = run(&raw(&[
        "--store",
        &w.store,
        "--chain",
        &w.chain,
        "--signoff",
        &so,
    ]));
    assert_eq!(out.code, 1);
    assert!(out.stdout.contains("BadControlFile"), "{}", out.stdout);
    // A complete sign-off: the report has the `signoff` object (goal and aggregate checks
    // arrive with PIF-003b, and the report says `pending` until then).
    let so = w.t.put("full.txt", full.as_bytes());
    let out = run(&raw(&[
        "--store",
        &w.store,
        "--chain",
        &w.chain,
        "--signoff",
        &so,
    ]));
    assert_eq!(out.code, 0, "{}", out.stdout);
    assert!(
        out.stdout
            .contains("\"signoff\":{\"provided\":true,\"base_environment\":\"checked\""),
        "{}",
        out.stdout
    );
    let out = run(&raw(&[
        "--unsafe-no-allow",
        "--store",
        &w.store,
        "--chain",
        &w.chain,
    ]));
    assert!(out.stdout.contains("\"signoff\":{\"provided\":false,\"base_environment\":\"absent\",\"goals\":\"absent\",\"aggregate\":\"absent\"}"));
}

#[test]
fn finding_3_duplicate_and_malformed_goal_ids_in_a_signoff_are_refused() {
    let w = world();
    let d = Digest::of(b"x").hex();
    let base = format!(
        "pif-signoff 1\nbase prelude sha256:{}\nbase-environment {}\ngoals-sha256 {d}\n",
        w.prelude_hex, w.base_env
    );
    assert!(parse_signoff(&format!("{base}goal a {d}\ngoal b {d}\n")).is_ok());
    for bad in [
        format!("{base}goal a {d}\ngoal a {d}\n"),
        format!("{base}goal a/b {d}\n"),
        format!("{base}goal {} {d}\n", "x".repeat(129)),
        format!("{base}goal a {d}\ngoal a {}\n", Digest::of(b"y").hex()),
    ] {
        assert_eq!(
            parse_signoff(&bad).err().map(|e| e.code()),
            Some("BadControlFile"),
            "{bad}"
        );
    }
}

#[test]
fn finding_4_a_chain_without_a_claim_file_is_refused_unless_allowed() {
    let t = Tmp::new();
    let p = prelude();
    t.store(&p);
    let chain = t.put(
        "c.txt",
        format!("pif-chain 1\nprelude sha256:{}\n", Digest::of(&p).hex()).as_bytes(),
    );
    let store = t.0.to_string_lossy().into_owned();
    let out = run(&raw(&[
        "--unsafe-no-allow",
        "--store",
        &store,
        "--chain",
        &chain,
    ]));
    assert_eq!(out.code, 1);
    assert!(
        out.stdout.contains("\"error\":\"NoClaimFile\""),
        "{}",
        out.stdout
    );
    let out = run(&raw(&[
        "--unsafe-no-allow",
        "--allow-no-claim",
        "--store",
        &store,
        "--chain",
        &chain,
    ]));
    assert_eq!(out.code, 0, "{}", out.stdout);
}

#[test]
fn finding_5_the_allow_list_binds_roles() {
    let d = Digest::of(b"m").hex();
    let ok = parse_allow(&format!(
        "pif-allow 1\nmodel sha256:{d}\nlibrary sha256:{d}\n"
    ))
    .unwrap();
    assert!(ok.contains(&(Role::Model, Digest::parse(&d).unwrap())));
    assert!(ok.contains(&(Role::Library, Digest::parse(&d).unwrap())));
    // A bare digest, an unknown role, and a claim entry are refused.
    for bad in [
        format!("pif-allow 1\nsha256:{d}\n"),
        format!("pif-allow 1\nwidget sha256:{d}\n"),
    ] {
        assert_eq!(
            parse_allow(&bad).err().map(|e| e.code()),
            Some("BadControlFile")
        );
    }
    assert_eq!(
        parse_allow(&format!("pif-allow 1\nclaim sha256:{d}\n")).err(),
        Some(CheckError::ClaimNotLast)
    );
    // A file allow-listed as the prelude may not be loaded under another role.
    let w = world();
    let allow = w.t.put(
        "allow.txt",
        format!("pif-allow 1\nlibrary sha256:{}\n", w.prelude_hex).as_bytes(),
    );
    let out = run(&raw(&[
        "--store", &w.store, "--chain", &w.chain, "--allow", &allow,
    ]));
    assert_eq!(out.code, 1);
    assert!(
        out.stdout.contains("\"error\":\"NotAllowListed\""),
        "{}",
        out.stdout
    );
}

#[test]
fn finding_7_withdrawn_logic_versions_reach_the_reader() {
    let w = world();
    let args = |extra: &[&str]| {
        raw(&[
            &[
                "--unsafe-no-allow",
                "--store",
                w.store.as_str(),
                "--chain",
                w.chain.as_str(),
            ][..],
            extra,
        ]
        .concat())
    };
    let out = run(&args(&["--withdrawn-logic", "1"]));
    assert_eq!(out.code, 1, "{}", out.stdout);
    assert!(
        out.stdout.contains("WithdrawnLogicVersion"),
        "{}",
        out.stdout
    );
    assert_eq!(run(&args(&["--withdrawn-logic", "2,3"])).code, 0);
    assert_eq!(run(&args(&["--withdrawn-logic", "x"])).code, 2);
}

#[cfg(unix)]
#[test]
fn finding_8_control_files_must_be_regular_files() {
    // Imported here: this test is Unix-only, so a top-level import is
    // unused (and a `-D warnings` error) on Windows.
    use pif_check::cli::read_text;
    let t = Tmp::new();
    // A FIFO would block an open forever: refused without opening.
    let fifo = t.path("fifo");
    assert!(std::process::Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .unwrap()
        .success());
    let (tx, rx) = std::sync::mpsc::channel();
    let f = fifo.clone();
    std::thread::spawn(move || {
        let _ = tx.send(read_text("chain", &f));
    });
    let r = rx
        .recv_timeout(std::time::Duration::from_secs(10))
        .expect("read_text must not block on a FIFO");
    assert_eq!(r.err().map(|e| e.code()), Some("BadControlFile"));
    // A symbolic link is refused, and so is a directory.
    let real = t.put("real.txt", b"pif-chain 1\n");
    let link = t.path("link.txt");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    assert_eq!(
        read_text("chain", &link).err().map(|e| e.code()),
        Some("BadControlFile")
    );
    assert!(read_text("chain", &real).is_ok());
    assert!(read_text("chain", &t.0.to_string_lossy()).is_err());
}
