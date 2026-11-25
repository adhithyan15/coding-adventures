//! PIF-003a: the resolver, the environment chain and the failures each must report exactly.

mod common;

use common::*;
use lean_kernel::pif::PifError;
use pif_check::chain::{load_chain, LoadOptions};
use pif_check::control::{check_shape, parse_allow, ChainEntry, Role};
use pif_check::resolver::{DirSource, Resolver, Source};
use pif_check::{CheckError, Digest};
use std::collections::BTreeSet;

fn load(files: &[&[u8]], chain: &[ChainEntry]) -> Result<pif_check::chain::Loaded, CheckError> {
    let m = mem(files);
    load_chain(&Resolver::new(&m), chain, &LoadOptions::default())
}

fn hex(b: &[u8]) -> String {
    Digest::of(b).hex()
}

#[test]
fn the_prelude_alone_loads_and_the_two_environment_digests_agree() {
    let p = prelude();
    let l = load(&[&p], &[entry(Role::Prelude, &p)]).unwrap();
    assert_eq!(l.files.len(), 1);
    assert!(
        l.files[0].decls.len() > 40,
        "the prelude declares many constants"
    );
    assert!(l.files[0].goals.is_empty());
    // No claim file: the base and full digests are the same fold over the one base file.
    assert_eq!(l.base_environment, l.environment);
    assert_eq!(
        l.base_environment,
        Digest::env_start().fold(&Digest::of(&p))
    );
    assert_eq!(l.files[0].prefix, Digest::env_start());
}

#[test]
fn the_prelude_plus_the_worked_theorem_as_a_claim_file_loads() {
    let p = prelude();
    let c = claim_over_prelude(&[hex(&p)], "gates.and_comm", WORKED_DIGEST, "Pif.and_comm");
    let chain = [entry(Role::Prelude, &p), entry(Role::Claim, c.as_bytes())];
    let l = load(&[&p, c.as_bytes()], &chain).unwrap();
    assert_eq!(l.files[1].goals.len(), 1);
    assert_eq!(l.files[1].goals[0].claim_id, "gates.and_comm");
    // E3: the base digest covers the prelude only; the full digest continues over the claim.
    assert_ne!(l.base_environment, l.environment);
    assert_eq!(
        l.base_environment,
        Digest::env_start().fold(&Digest::of(&p))
    );
    assert_eq!(
        l.environment,
        l.base_environment.fold(&Digest::of(c.as_bytes()))
    );
    // The claim's environment prefix is the base environment (VER32 1.4 A2).
    assert_eq!(l.files[1].prefix, l.base_environment);
}

#[test]
fn a_digest_mismatch_is_refused_before_the_reader_sees_the_bytes() {
    let p = prelude();
    let mut m = mem(&[&p]);
    // Plant a different file under the prelude's digest.
    m.0.insert(Digest::of(&p), b"pif 1\n".to_vec());
    let r = Resolver::new(&m);
    let err = r.resolve(&Digest::of(&p)).unwrap_err();
    assert_eq!(
        err,
        CheckError::DigestMismatch {
            expected: hex(&p),
            actual: hex(b"pif 1\n")
        }
    );
    let chain = [entry(Role::Prelude, &p)];
    let e = load_chain(&r, &chain, &LoadOptions::default())
        .err()
        .unwrap();
    assert_eq!(e.code(), "DigestMismatch");
}

#[test]
fn a_wrong_import_list_is_an_import_mismatch_from_the_reader() {
    let p = prelude();
    // The claim lists no import at all.
    let c = claim_over_prelude(&[], "gates.and_comm", WORKED_DIGEST, "Pif.and_comm");
    let chain = [entry(Role::Prelude, &p), entry(Role::Claim, c.as_bytes())];
    let e = load(&[&p, c.as_bytes()], &chain).err().unwrap();
    assert_eq!(
        e,
        CheckError::Read {
            index: 1,
            role: "claim",
            error: PifError::ImportMismatch
        }
    );
    // The claim lists an import that is not in the chain.
    let c = claim_over_prelude(
        &[hex(b"other")],
        "gates.and_comm",
        WORKED_DIGEST,
        "Pif.and_comm",
    );
    let chain = [entry(Role::Prelude, &p), entry(Role::Claim, c.as_bytes())];
    let e = load(&[&p, c.as_bytes()], &chain).err().unwrap();
    assert_eq!(
        e,
        CheckError::Read {
            index: 1,
            role: "claim",
            error: PifError::ImportMismatch
        }
    );
    // The claim lists the prelude twice.
    let c = claim_over_prelude(
        &[hex(&p), hex(&p)],
        "gates.and_comm",
        WORKED_DIGEST,
        "Pif.and_comm",
    );
    let chain = [entry(Role::Prelude, &p), entry(Role::Claim, c.as_bytes())];
    let e = load(&[&p, c.as_bytes()], &chain).err().unwrap();
    assert_eq!(
        e,
        CheckError::Read {
            index: 1,
            role: "claim",
            error: PifError::ImportMismatch
        }
    );
}

#[test]
fn a_missing_file_is_reported_with_its_digest() {
    let p = prelude();
    let c = claim_over_prelude(&[hex(&p)], "gates.and_comm", WORKED_DIGEST, "Pif.and_comm");
    let chain = [entry(Role::Prelude, &p), entry(Role::Claim, c.as_bytes())];
    let e = load(&[&p], &chain).err().unwrap();
    assert_eq!(e, CheckError::FileMissing(hex(c.as_bytes())));
}

#[test]
fn reordered_chains_fail_with_their_exact_errors() {
    let p = prelude();
    let a1 = layer(&[hex(&p)], "Adl.one");
    let a2 = layer(&[hex(&p), hex(a1.as_bytes())], "Adl.two");
    let c = claim_over_prelude(
        &[hex(&p), hex(a1.as_bytes()), hex(a2.as_bytes())],
        "gates.and_comm",
        WORKED_DIGEST,
        "Pif.and_comm",
    );
    let files: [&[u8]; 4] = [&p, a1.as_bytes(), a2.as_bytes(), c.as_bytes()];
    let (ep, e1, e2, ec) = (
        entry(Role::Prelude, &p),
        entry(Role::Adl, a1.as_bytes()),
        entry(Role::Adl, a2.as_bytes()),
        entry(Role::Claim, c.as_bytes()),
    );
    // The right order loads.
    assert!(load(&files, &[ep, e1, e2, ec]).is_ok());
    // Same roles, two ADL layers swapped: each names the wrong prefix.
    let e = load(&files, &[ep, e2, e1, ec]).err().unwrap();
    assert_eq!(
        e,
        CheckError::Read {
            index: 1,
            role: "adl",
            error: PifError::ImportMismatch
        }
    );
    // The prelude is not first.
    assert_eq!(check_shape(&[e1, ep]), Err(CheckError::PreludeNotFirst));
    // A role out of order: an ADL layer after the model.
    let model = entry(Role::Model, b"m");
    assert_eq!(
        check_shape(&[ep, model, e1]),
        Err(CheckError::RoleOrder { index: 2 })
    );
    // A once-only role twice.
    assert_eq!(
        check_shape(&[ep, model, model]),
        Err(CheckError::DuplicateRole { index: 2 })
    );
    assert_eq!(
        check_shape(&[ep, ep]),
        Err(CheckError::DuplicateRole { index: 1 })
    );
    // Several ADL layers are fine.
    assert_eq!(check_shape(&[ep, e1, e2, model, ec]), Ok(()));
    assert_eq!(check_shape(&[]), Err(CheckError::EmptyChain));
}

#[test]
fn a_claim_file_in_a_base_position_is_refused() {
    let p = prelude();
    let a1 = layer(&[hex(&p)], "Adl.one");
    let c = claim_over_prelude(&[hex(&p)], "gates.and_comm", WORKED_DIGEST, "Pif.and_comm");
    let (ep, e1, ec) = (
        entry(Role::Prelude, &p),
        entry(Role::Adl, a1.as_bytes()),
        entry(Role::Claim, c.as_bytes()),
    );
    assert_eq!(check_shape(&[ep, ec, e1]), Err(CheckError::ClaimNotLast));
    // A claim file *declared* as a library is read with the library rules and cannot sneak
    // past: the role decides the class, and a claim file read as `library` loses R-AX-1 only
    // if the chain says so. Here the shape check refuses the claim role mid-chain outright.
    assert_eq!(check_shape(&[ec, ep]), Err(CheckError::ClaimNotLast));
    assert_eq!(check_shape(&[ep, ec]), Ok(()));
}

#[test]
fn the_chain_is_at_most_65_files() {
    let p = prelude();
    let ep = entry(Role::Prelude, &p);
    let adl = entry(Role::Adl, b"x");
    let mut chain = vec![ep];
    chain.extend(std::iter::repeat_n(adl, 64));
    assert_eq!(check_shape(&chain), Ok(()));
    chain.push(adl);
    assert_eq!(check_shape(&chain), Err(CheckError::ChainTooLong));
}

#[test]
fn a_base_file_off_the_allow_list_is_refused_but_the_claim_need_not_be_on_it() {
    let p = prelude();
    let c = claim_over_prelude(&[hex(&p)], "gates.and_comm", WORKED_DIGEST, "Pif.and_comm");
    let chain = [entry(Role::Prelude, &p), entry(Role::Claim, c.as_bytes())];
    let m = mem(&[&p, c.as_bytes()]);
    let r = Resolver::new(&m);
    // E4: only the prelude is allow-listed; the claim file is not, by design.
    let allow: BTreeSet<(Role, Digest)> = [(Role::Prelude, Digest::of(&p))].into_iter().collect();
    assert!(load_chain(
        &r,
        &chain,
        &LoadOptions {
            allow: Some(&allow),
            ..LoadOptions::default()
        }
    )
    .is_ok());
    let none: BTreeSet<(Role, Digest)> = BTreeSet::new();
    let e = load_chain(
        &r,
        &chain,
        &LoadOptions {
            allow: Some(&none),
            ..LoadOptions::default()
        },
    )
    .err()
    .unwrap();
    assert_eq!(
        e,
        CheckError::NotAllowListed {
            index: 0,
            digest: hex(&p)
        }
    );
}

#[test]
fn the_directory_store_verifies_and_refuses_unsafe_entries() {
    let t = Tmp::new();
    let p = prelude();
    let d = t.store(&p);
    let src = DirSource::new(&t.0);
    let r = Resolver::new(&src);
    assert_eq!(r.resolve(&d).unwrap(), p);
    // A file whose content does not match its name.
    let wrong = Digest::of(b"wrong");
    t.put(&format!("{}.pif", wrong.hex()), b"not the right bytes");
    assert_eq!(r.resolve(&wrong).err().unwrap().code(), "DigestMismatch");
    // Absent.
    assert_eq!(
        r.resolve(&Digest::of(b"absent")).err().unwrap(),
        CheckError::FileMissing(hex(b"absent"))
    );
    // A directory where the file should be.
    let dir = Digest::of(b"dir");
    std::fs::create_dir(t.0.join(format!("{}.pif", dir.hex()))).unwrap();
    assert_eq!(
        src.get(&dir).err().unwrap(),
        CheckError::NotRegularFile(dir.hex())
    );
    // The size cap, before reading.
    let small = DirSource::new(&t.0).with_max_file_bytes(10);
    assert_eq!(
        small.get(&d).err().unwrap(),
        CheckError::FileTooLarge(d.hex())
    );
    // A name never contains a separator, whatever the digest.
    assert!(!src.path_for(&d).to_string_lossy().contains(".."));
}

#[cfg(unix)]
#[test]
fn a_symbolic_link_in_the_store_is_refused() {
    let t = Tmp::new();
    let p = prelude();
    let outside = t.put("elsewhere.bin", &p);
    let d = Digest::of(&p);
    std::os::unix::fs::symlink(outside, t.0.join(format!("{}.pif", d.hex()))).unwrap();
    let src = DirSource::new(&t.0);
    assert_eq!(
        src.get(&d).err().unwrap(),
        CheckError::NotRegularFile(d.hex())
    );
}

#[test]
fn an_unreadable_store_entry_is_an_io_error_not_a_panic() {
    // The store root is a file, so every path below it fails to open with a non-NotFound error.
    let t = Tmp::new();
    let f = t.put("root-is-a-file", b"x");
    let src = DirSource::new(f);
    let e = src.get(&Digest::of(b"x")).err().unwrap();
    assert!(
        matches!(e, CheckError::FileMissing(_) | CheckError::Io { .. }),
        "{e:?}"
    );
}

#[test]
fn control_files_are_strict() {
    let d = hex(b"x");
    assert!(parse_allow(&format!("pif-allow 1\nmodel sha256:{d}\n")).is_ok());
    for bad in [
        String::new(),
        format!(
            "pif-allow 1
model sha256:{d}"
        ), // no final LF
        format!("pif-allow 1\n\nsha256:{d}\n"), // blank line
        format!(
            "pif-allow 1
model sha256:{d} \n"
        ), // trailing space
        format!(
            "pif-allow 1
 model sha256:{d}\n"
        ), // leading space
        format!("pif-allow 2\nsha256:{d}\n"),   // wrong version
        format!(
            "pif-allow 1
model sha256:{}\n",
            d.to_uppercase()
        ), // upper case
        "pif-allow 1
model sha256:abc\n"
            .to_string(), // short
        format!("pif-allow 1\nmodel {d} {d}\n"), // two fields
        format!(
            "pif-allow 1
model sha256:{d}\r\n"
        ), // CR
        "pif-allow 1\n\u{e9}\n".to_string(),    // non-ASCII
    ] {
        assert_eq!(
            parse_allow(&bad).err().map(|e| e.code()),
            Some("BadControlFile"),
            "{bad:?}"
        );
    }
}
