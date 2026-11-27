//! Loading the ordered environment chain (VER32 3.7, rules E1 to E6).
//!
//! # The idea
//!
//! A proof is checked against an *environment*: the declarations of every file before it.
//! The environment is not whatever the proof's author imports; it is the exact ordered list
//! the signed sign-off fixes (E1), and each file must name **all and only** the files before
//! it as its `import` lines (E2). So a chain is a line, never a diamond, and "which file is
//! loaded twice" cannot arise.
//!
//! ```text
//!   prelude ── adl ── (library) ── model ── (axioms) ──┬── claim
//!   └────────────── base chain (signed, E1, E6) ───────┘   └ last, not allow-listed (E4)
//! ```
//!
//! # What [`load_chain`] does, in order, for each file
//!
//! 1. resolve its digest to bytes and **verify the SHA-256** (E5; [`crate::resolver`]);
//! 2. check that a base file is on the allow-list, if one was given (E4): the entry binds the
//!    file's role as well as its digest;
//! 3. call the trusted reader with the correct [`FileClass`] and [`GateInputs`]: the imports
//!    it must have are the lowercase hex of every digest before it (E2, R-IMP-1), and no
//!    feature is allowed (the kernel implements none yet);
//! 4. fold its digest into the environment digests (8.3).
//!
//! It stops at the first failure and returns *no environment*: the reader's environment is
//! consumed by value, so a half-loaded chain cannot be used by mistake.
//!
//! The base environment digest is folded over the base chain only (everything but a claim
//! file); the full digest continues over the claim file. The signature binds the first, the
//! certificate records the second (E3, E6).

use crate::control::{ChainEntry, Role};
use crate::digest::Digest;
use crate::error::CheckError;
use crate::registry::Registry;
use crate::resolver::Resolver;
use lean_kernel::pif::{read, Accepted, GateInputs, Goal};
use lean_kernel::{Environment, Expr, ExprStore, Name};
use std::collections::{BTreeSet, HashSet};

/// What the gate hands the loader besides the chain itself.
#[derive(Default)]
pub struct LoadOptions<'a> {
    /// E4: digests the gate allows in the **base** chain. `None` means "not checked", and the
    /// report says so.
    pub allow: Option<&'a BTreeSet<(Role, Digest)>>,
    /// R-VER-1: `logic` versions the gate's pin file has withdrawn (gate configuration; an empty
    /// list withdraws none).
    pub withdrawn_logic: Vec<u32>,
    /// R-GOL-4 (gate-fed): the constants that occur in the goal terms of the claims, taken
    /// from the goals list. Only the claim file is read with it.
    pub goal_constants: HashSet<Name>,
    /// R-AX-1 (gate-fed): the axiom registry the claim file's axioms are matched against.
    /// `None` rejects every axiom of a claim file.
    pub registry: Option<&'a Registry>,
}

/// One file of the chain, as accepted.
pub struct LoadedFile {
    pub role: Role,
    pub digest: Digest,
    /// The environment digest *before* this file: its "environment prefix" (VER32 1.4 A2).
    pub prefix: Digest,
    /// The producer's profile (id, label): reported, never trusted.
    pub profile: (String, String),
    /// Every declaration the kernel accepted from this file, in file order.
    pub decls: Vec<Accepted>,
    /// Every goal record of this file.
    pub goals: Vec<Goal>,
}

/// A fully loaded chain.
pub struct Loaded {
    /// The environment after the whole chain.
    pub env: Environment,
    pub files: Vec<LoadedFile>,
    /// Folded over the base chain (what the signature binds).
    pub base_environment: Digest,
    /// Folded over the base chain and the claim file (what the certificate records).
    pub environment: Digest,
}

/// Load `chain` through `resolver`. The shape of the chain is checked by the caller
/// ([`crate::control::check_shape`]) before any file is touched.
pub fn load_chain(
    resolver: &Resolver,
    chain: &[ChainEntry],
    opts: &LoadOptions,
) -> Result<Loaded, CheckError> {
    let mut env = Environment::new();
    let mut files: Vec<LoadedFile> = Vec::with_capacity(chain.len());
    let mut state = Digest::env_start();
    let mut base_environment = state;
    let mut prior: Vec<String> = Vec::with_capacity(chain.len());
    let mut prior_ids = std::collections::HashSet::new();
    for (index, entry) in chain.iter().enumerate() {
        let bytes = resolver.resolve(&entry.digest)?;
        let is_base = entry.role != Role::Claim;
        if is_base
            && opts
                .allow
                .is_some_and(|a| !a.contains(&(entry.role, entry.digest)))
        {
            return Err(CheckError::NotAllowListed {
                index,
                digest: entry.digest.hex(),
            });
        }
        // A2: a registered statement matches only under the environment prefix it was
        // registered in, so the registry predicate is built per file with that file's prefix.
        let prefix_now = state;
        let registry_check = |name: &Name, ps: &[Name], ty: Expr, st: &ExprStore| {
            opts.registry
                .is_some_and(|r| r.matches_term(name, ps, ty, st, &prefix_now))
        };
        let gate = GateInputs {
            expected_imports: prior.clone(),
            class: entry.role.class(),
            withdrawn_logic: opts.withdrawn_logic.clone(),
            prior_claim_ids: prior_ids.clone(),
            goal_constants: if is_base {
                HashSet::new()
            } else {
                opts.goal_constants.clone()
            },
            axiom_registry: if is_base { None } else { Some(&registry_check) },
            ..GateInputs::default()
        };
        let role = entry.role.word();
        let report =
            read(env, &bytes, &gate).map_err(|error| CheckError::Read { index, role, error })?;
        env = report.env;
        let prefix = state;
        state = state.fold(&entry.digest);
        if is_base {
            base_environment = state;
        }
        prior.push(entry.digest.hex());
        prior_ids.extend(report.goals.iter().map(|g| g.claim_id.clone()));
        files.push(LoadedFile {
            role: entry.role,
            digest: entry.digest,
            prefix,
            profile: report.profile,
            decls: report.decls,
            goals: report.goals,
        });
    }
    Ok(Loaded {
        env,
        files,
        base_environment,
        environment: state,
    })
}
