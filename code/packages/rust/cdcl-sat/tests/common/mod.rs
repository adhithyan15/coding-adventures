//! Helpers shared by the integration tests: a deterministic generator,
//! random formulas, brute force, and pigeonhole instances.

#![allow(dead_code)] // each test file uses a different subset

use cdcl_sat::{Cnf, Lit, Var};

/// xorshift64: a tiny deterministic generator, so failures reproduce and the
/// crate needs no dependencies.
pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    pub fn lit(&mut self, vars: usize) -> Lit {
        Var::from_index(self.below(vars)).lit(self.next() & 1 == 1)
    }
}

/// A random formula over `vars` variables: mostly 3-literal clauses, with
/// some 1-, 2- and 4-literal ones so units and binaries are exercised too.
pub fn random_cnf(rng: &mut Rng, vars: usize, clauses: usize) -> Cnf {
    let mut cnf = Cnf::new(vars);
    for _ in 0..clauses {
        let width = match rng.below(10) {
            0 => 1,
            1 | 2 => 2,
            3 => 4,
            _ => 3,
        };
        let c: Vec<Lit> = (0..width).map(|_| rng.lit(vars)).collect();
        cnf.add_clause(&c);
    }
    cnf
}

/// Every assignment of `vars` variables, as bit patterns.
pub fn brute_force_sat(cnf: &Cnf, fixed: &[Lit]) -> bool {
    let n = cnf.num_vars;
    (0u64..(1 << n)).any(|bits| {
        let a: Vec<bool> = (0..n).map(|i| bits >> i & 1 == 1).collect();
        fixed.iter().all(|l| a[l.var().index()] != l.is_negated()) && cnf.is_satisfied_by(&a)
    })
}

/// PHP(n+1, n): n+1 pigeons in n holes, one pigeon per hole. Always UNSAT,
/// and exponentially hard for resolution, so it makes the solver learn,
/// backjump, restart and reduce its database for real.
pub fn pigeonhole(holes: usize) -> Cnf {
    let pigeons = holes + 1;
    let x = |p: usize, h: usize| Var::from_index(p * holes + h);
    let mut cnf = Cnf::new(pigeons * holes);
    for p in 0..pigeons {
        let c: Vec<Lit> = (0..holes).map(|h| x(p, h).positive()).collect();
        cnf.add_clause(&c);
    }
    for h in 0..holes {
        for p in 0..pigeons {
            for q in p + 1..pigeons {
                cnf.add_clause(&[x(p, h).negative(), x(q, h).negative()]);
            }
        }
    }
    cnf
}
