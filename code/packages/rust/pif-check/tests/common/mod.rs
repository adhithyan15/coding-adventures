//! Shared fixtures: the committed prelude, a claim file built with the untrusted writer, and
//! temporary stores. Everything here is test scaffolding; the gate re-checks every byte.
#![allow(dead_code)]

use pif_check::control::{ChainEntry, Role};
use pif_check::resolver::MemSource;
use pif_check::Digest;
use pif_writer::{BinderInfo::Default as D, Level, Writer};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

/// The committed prelude's bytes (what the generator emits, which a test elsewhere pins).
pub fn prelude() -> Vec<u8> {
    pif_prelude::generate().into_bytes()
}

/// The statement hint the worked example records.
pub const WORKED_DIGEST: &str = "bf1fa0a0d828b019965f46efa44ff2627bf54a59558a1bc2959b52c161f74676";

/// The VER32 4.6 theorem `Pif.and_comm` as a claim file over the prelude: the same statement
/// and proof as the worked example, but `Bool`, `Eq` and `Pif.and` come from the prelude, not
/// from this file. `imports` are the hex digests of the files before it. `goal_hint` is the
/// digest written in the `goal` record, and `claim` the claim id.
pub fn claim_over_prelude(
    imports: &[String],
    claim: &str,
    goal_hint: &str,
    thm_name: &str,
) -> String {
    let mut w = Writer::new("native", "pif-case-split 0.1")
        .unwrap()
        .ascii_names_only();
    for i in imports {
        w.import(i).unwrap();
    }
    let bool_n = w.name("Bool").unwrap();
    let (ff, tt) = (w.name("Bool.false").unwrap(), w.name("Bool.true").unwrap());
    let eq_n = w.name("Eq").unwrap();
    let refl = w.name("Eq.refl").unwrap();
    let and_n = w.name("Pif.and").unwrap();
    let rec_n = w.name("Bool.rec").unwrap();
    let (a, b) = (w.name("a").unwrap(), w.name("b").unwrap());
    let l1 = w.level_succ(Level::ZERO);
    let bool_t = w.constant(bool_n, &[]);
    let (v0, v1) = (w.var(0), w.var(1));
    let (fc, tc) = (w.constant(ff, &[]), w.constant(tt, &[]));
    let and_c = w.constant(and_n, &[]);
    let eq1 = w.constant(eq_n, &[l1]);
    let ab = w.app_many(and_c, &[v1, v0]);
    let and_b = w.app(and_c, v0);
    let ba = w.app(and_b, v1);
    let eq_bool = w.app(eq1, bool_t);
    let stmt = w.app_many(eq_bool, &[ab, ba]);
    let inner = w.pi(D, b, bool_t, stmt);
    let thm_ty = w.pi(D, a, bool_t, inner);
    let motive_a = w.lam(D, a, bool_t, inner);
    let rec0 = w.constant(rec_n, &[Level::ZERO]);
    let refl1 = w.constant(refl, &[l1]);
    let refl_b = w.app(refl1, bool_t);
    let (rf, rt) = (w.app(refl_b, fc), w.app(refl_b, tc));
    let and_f = w.app(and_c, fc);
    let and_fb = w.app(and_f, v0);
    let and_b_f = w.app(and_b, fc);
    let eq_f = w.app(eq_bool, and_fb);
    let eq_f = w.app(eq_f, and_b_f);
    let m_f = w.lam(D, b, bool_t, eq_f);
    let case_f = w.app_many(rec0, &[m_f, rf, rf, v0]);
    let case_f = w.lam(D, b, bool_t, case_f);
    let and_t = w.app(and_c, tc);
    let and_tb = w.app(and_t, v0);
    let and_b_t = w.app(and_b, tc);
    let eq_t = w.app(eq_bool, and_tb);
    let eq_t = w.app(eq_t, and_b_t);
    let m_t = w.lam(D, b, bool_t, eq_t);
    let case_t = w.app_many(rec0, &[m_t, rf, rt, v0]);
    let case_t = w.lam(D, b, bool_t, case_t);
    let body = w.app_many(rec0, &[motive_a, case_f, case_t, v0]);
    let thm_val = w.lam(D, a, bool_t, body);
    let thm = w.name(thm_name).unwrap();
    w.theorem(thm, &[], thm_ty, thm_val).unwrap();
    w.goal(claim, thm, goal_hint).unwrap();
    w.to_text()
}

/// A tiny reviewed file (an ADL layer, say) declaring one axiom `name : Bool`, importing `imports`.
pub fn layer(imports: &[String], name: &str) -> String {
    let mut w = Writer::new("native", "layer 0.1")
        .unwrap()
        .ascii_names_only();
    for i in imports {
        w.import(i).unwrap();
    }
    let bool_n = w.name("Bool").unwrap();
    let ty = w.constant(bool_n, &[]);
    let n = w.name(name).unwrap();
    w.axiom(n, &[], ty).unwrap();
    w.to_text()
}

pub fn entry(role: Role, bytes: &[u8]) -> ChainEntry {
    ChainEntry {
        role,
        digest: Digest::of(bytes),
    }
}

/// A store holding every file given.
pub fn mem(files: &[&[u8]]) -> MemSource {
    let mut m = MemSource::default();
    for f in files {
        m.add(f);
    }
    m
}

/// A fresh temporary directory, removed when dropped.
pub struct Tmp(pub PathBuf);

static COUNTER: AtomicU32 = AtomicU32::new(0);

impl Tmp {
    pub fn new() -> Tmp {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!("pif-check-test-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&p).unwrap();
        Tmp(p)
    }
    pub fn path(&self, name: &str) -> String {
        self.0.join(name).to_string_lossy().into_owned()
    }
    pub fn put(&self, name: &str, bytes: &[u8]) -> String {
        std::fs::write(self.0.join(name), bytes).unwrap();
        self.path(name)
    }
    /// File `bytes` in this directory as `<digest>.pif`.
    pub fn store(&self, bytes: &[u8]) -> Digest {
        let d = Digest::of(bytes);
        self.put(&format!("{}.pif", d.hex()), bytes);
        d
    }
}

impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
