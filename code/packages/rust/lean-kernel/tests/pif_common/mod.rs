//! A tiny PIF *producer* for the reader's tests: it numbers names, levels and
//! expressions for you and prints the lines. It is deliberately not clever: a test that
//! wants a malformed file writes the offending line by hand with [`B::line`].
#![allow(dead_code)]

use lean_kernel::pif::{read, GateInputs, PifError, ReadReport};
use lean_kernel::Environment;

/// A file under construction: header lines, body lines and the next index of each table.
pub struct B {
    pub header: Vec<String>,
    pub body: Vec<String>,
    pub nn: usize,
    pub nl: usize,
    pub ne: usize,
}

impl B {
    pub fn new() -> B {
        let header = ["pif 1", "logic 1", "profile native \"t\"", "features"];
        B {
            header: header.iter().map(|s| s.to_string()).collect(),
            body: vec![],
            nn: 0,
            nl: 0,
            ne: 0,
        }
    }

    /// A raw body line (it counts toward `end k`).
    pub fn line(&mut self, s: &str) -> &mut B {
        self.body.push(s.to_string());
        self
    }

    /// `nstr` extending `parent` with one component; returns the new name index.
    pub fn nstr(&mut self, parent: usize, comp: &str) -> usize {
        self.nn += 1;
        self.body
            .push(format!("nstr {} {} \"{}\"", self.nn, parent, comp));
        self.nn
    }

    /// A dotted name built from string components.
    pub fn name(&mut self, dotted: &str) -> usize {
        dotted.split('.').fold(0, |p, c| self.nstr(p, c))
    }

    pub fn lsucc(&mut self, l: usize) -> usize {
        self.nl += 1;
        self.body.push(format!("lsucc {} {}", self.nl, l));
        self.nl
    }

    pub fn lparam(&mut self, name: usize) -> usize {
        self.nl += 1;
        self.body.push(format!("lparam {} {}", self.nl, name));
        self.nl
    }

    fn e(&mut self, kw: &str, rest: &str) -> usize {
        self.ne += 1;
        self.body.push(format!("{kw} {} {rest}", self.ne));
        self.ne
    }

    pub fn sort(&mut self, l: usize) -> usize {
        self.e("esort", &l.to_string())
    }

    pub fn var(&mut self, i: usize) -> usize {
        self.e("evar", &i.to_string())
    }

    pub fn cnst(&mut self, name: usize, levels: &[usize]) -> usize {
        self.e("econst", &format!("{name} {}", Self::list(levels)))
    }

    pub fn app(&mut self, f: usize, a: usize) -> usize {
        self.e("eapp", &format!("{f} {a}"))
    }

    pub fn lam(&mut self, name: usize, ty: usize, body: usize) -> usize {
        self.e("elam", &format!("default {name} {ty} {body}"))
    }

    pub fn pi(&mut self, name: usize, ty: usize, body: usize) -> usize {
        self.e("epi", &format!("default {name} {ty} {body}"))
    }

    fn list(ids: &[usize]) -> String {
        let v: Vec<String> = ids.iter().map(|i| i.to_string()).collect();
        format!("[{}]", v.join(" "))
    }

    pub fn axiom(&mut self, name: usize, ps: &[usize], ty: usize) {
        self.body
            .push(format!("axiom {name} {} {ty}", Self::list(ps)));
    }

    pub fn theorem(&mut self, name: usize, ps: &[usize], ty: usize, val: usize) {
        self.body
            .push(format!("theorem {name} {} {ty} {val}", Self::list(ps)));
    }

    pub fn opaque(&mut self, name: usize, ps: &[usize], ty: usize, val: usize) {
        self.body
            .push(format!("opaque {name} {} {ty} {val}", Self::list(ps)));
    }

    pub fn def(&mut self, name: usize, ps: &[usize], ty: usize, val: usize, hint: &str) {
        self.body
            .push(format!("def {name} {} {ty} {val} {hint}", Self::list(ps)));
    }

    /// `inductive T : Type | T.mk : T`; returns (type name index, constructor name index).
    pub fn unit_type(&mut self, name: &str) -> (usize, usize) {
        let t = self.name(name);
        let mk = self.nstr(t, "mk");
        let one = self.lsucc(0);
        let ty = self.sort(one);
        let tc = self.cnst(t, &[]);
        self.body.push("inductive [] 0 1".to_string());
        self.body.push(format!("itype {t} {ty} 1"));
        self.body.push(format!("ictor {mk} {tc}"));
        (t, mk)
    }

    /// `axiom Name : Prop`; returns the name index and the index of `Name` as a constant.
    pub fn prop_axiom(&mut self, name: &str) -> (usize, usize) {
        let n = self.name(name);
        let prop = self.sort(0);
        self.axiom(n, &[], prop);
        let c = self.cnst(n, &[]);
        (n, c)
    }

    pub fn text(&self) -> String {
        let mut s = String::new();
        for l in self.header.iter().chain(&self.body) {
            s.push_str(l);
            s.push('\n');
        }
        s.push_str(&format!("end {}\n", self.body.len()));
        s
    }

    pub fn bytes(&self) -> Vec<u8> {
        self.text().into_bytes()
    }
}

/// Gate inputs for a library file: no claim-file rules, no ASCII class.
pub fn lib() -> GateInputs<'static> {
    GateInputs {
        class: lean_kernel::pif::FileClass::Library,
        ..GateInputs::default()
    }
}

pub fn ok(bytes: &[u8], g: &GateInputs) -> ReadReport {
    match read(Environment::new(), bytes, g) {
        Ok(r) => r,
        Err(e) => panic!(
            "expected the file to be accepted, got {e:?}\n{}",
            String::from_utf8_lossy(bytes)
        ),
    }
}

pub fn err(bytes: &[u8], g: &GateInputs) -> PifError {
    match read(Environment::new(), bytes, g) {
        Ok(_) => panic!("expected a rejection\n{}", String::from_utf8_lossy(bytes)),
        Err(e) => e,
    }
}

/// Read a file text under library rules and return the error.
pub fn lib_err(text: &str) -> PifError {
    err(text.as_bytes(), &lib())
}

/// A whole file with the given body lines (and the right `end k`).
pub fn file(body: &[&str]) -> String {
    let mut b = B::new();
    for l in body {
        b.line(l);
    }
    b.text()
}
