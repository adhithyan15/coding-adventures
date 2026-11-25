//! The environment: the append-only list of everything the kernel has accepted.
//!
//! # What a declaration is
//!
//! ```text
//!   axiom      c.{us} : T                  assumed, never unfolded
//!   definition c.{us} : T := v  (hints)    unfolded by delta
//!   theorem    c.{us} : T := v             T must be a Prop; never unfolded
//!   opaque     c.{us} : T := v             checked like a definition, never unfolded
//!   inductive  T.{us} : ...                added by `add_inductive` only, never by
//!   constructor T.c.{us} : ...             `add_declaration`: the kernel derives the
//!   recursor   T.rec.{us'} : ...           recursor (see `inductive.rs`)
//! ```
//!
//! # The checks, and what each one prevents
//!
//! [`Environment::add_declaration`] runs these in order, and a failure leaves the
//! environment exactly as it was:
//!
//! 1. *The name is new.* Otherwise a later declaration could replace an earlier one and
//!    silently change what already-accepted theorems mean.
//! 2. *Universe parameters are distinct.* `{u u}` would make instantiation ambiguous.
//! 3. *Type and value are closed* (no loose bound variables, no free variables). A free
//!    variable would be an unchecked hypothesis smuggled into a global fact.
//! 4. *Every universe parameter used is declared* (enforced by the checker as it visits
//!    every `Sort` and constant). An undeclared `u` could be instantiated to anything
//!    later and would make `Sort u` mean different things at different places.
//! 5. *The type is a type*: its own type is a `Sort`. Otherwise `c : 5` would let a
//!    non-type stand where a type is needed.
//! 6. *The value has the declared type*: `infer(value)` is definitionally equal to the
//!    declared type. This is the heart of proof checking: for a theorem it is the
//!    statement "this proof proves this proposition".
//! 7. *A theorem's type is a proposition*: its sort is level-equivalent to `0`. Theorems
//!    are never unfolded and are proof-irrelevant, which is only sound for `Prop`.
//! 8. *Only constants already in the environment can be referenced.* This is not a
//!    separate pass: the checker rejects an unknown constant, and the declaration's own
//!    name is not added until it has passed, so self-reference (and hence non-terminating
//!    unfolding) is impossible.
//!
//! The environment owns the [`ExprStore`] that all its expressions live in, so a term
//! built for one environment cannot accidentally be mixed with another's.

use crate::checker::TypeChecker;
use crate::error::KernelError;
use crate::expr::{Expr, ExprStore};
use crate::level::Level;
use crate::name::Name;
use std::collections::{HashMap, HashSet};

/// How eagerly a definition should be unfolded when comparing terms. Stored now, used
/// by lazy delta reduction in a later slice.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ReducibilityHints {
    /// Never unfold when comparing (but whnf may).
    Opaque,
    /// Unfold as early as possible.
    Abbrev,
    /// Unfold according to height: a definition built from deeper ones has a larger
    /// number and is unfolded first.
    Regular(u32),
}

/// What kind of declaration this is, with its value if it has one.
#[derive(Clone, Debug)]
pub enum DeclKind {
    /// An assumption.
    Axiom,
    /// A definition; delta may unfold it.
    Definition {
        /// The body.
        value: Expr,
        /// Unfolding priority (unused until lazy delta).
        hints: ReducibilityHints,
    },
    /// A proved proposition; never unfolded.
    Theorem {
        /// The proof.
        value: Expr,
    },
    /// A definition whose body is hidden from delta.
    Opaque {
        /// The body (checked, then hidden).
        value: Expr,
    },
    /// An inductive type. Produced only by [`Environment::add_inductive`].
    Inductive(InductiveInfo),
    /// A constructor of an inductive type. Produced only by `add_inductive`.
    Constructor(ConstructorInfo),
    /// The recursor the kernel generated for an inductive type; carries its
    /// computation rules. Produced only by `add_inductive`.
    Recursor(RecursorInfo),
}

/// What the kernel records about an inductive type (see `inductive.rs`).
#[derive(Clone, Debug)]
pub struct InductiveInfo {
    /// Number of parameters (arguments shared by the type and every constructor).
    pub num_params: usize,
    /// Number of indices (the Pi binders of the type after the parameters).
    pub num_indices: usize,
    /// The constructor names, in declaration order.
    pub ctors: Vec<Name>,
    /// The name of the generated recursor (`T.rec`).
    pub rec: Name,
}

/// What the kernel records about a constructor.
#[derive(Clone, Debug)]
pub struct ConstructorInfo {
    /// The inductive type it builds.
    pub induct: Name,
    /// Its position among the type's constructors.
    pub index: usize,
    /// Number of parameters (same as the type's).
    pub num_params: usize,
    /// Number of fields (the arguments after the parameters).
    pub num_fields: usize,
}

/// One computation rule of a recursor: how it reduces on a given constructor.
#[derive(Clone, Debug)]
pub struct RecRule {
    /// The constructor this rule fires on.
    pub ctor: Name,
    /// Number of fields of that constructor.
    pub num_fields: usize,
    /// `fun params motive minors fields => minor_c fields (recursive results)`, closed,
    /// in the recursor's own universe parameters.
    pub rhs: Expr,
}

/// What the kernel records about a generated recursor.
#[derive(Clone, Debug)]
pub struct RecursorInfo {
    /// The inductive type it eliminates.
    pub induct: Name,
    /// Number of parameters.
    pub num_params: usize,
    /// Number of indices.
    pub num_indices: usize,
    /// Number of minor premises (one per constructor). There is always one motive.
    pub num_minors: usize,
    /// K-like: a `Prop` inductive with one constructor that has no fields (`Eq`).
    pub k: bool,
    /// May the motive land in any `Sort u` (true) or only `Prop` (false)?
    pub large_elim: bool,
    /// One rule per constructor.
    pub rules: Vec<RecRule>,
}

/// A declaration awaiting (or having passed) checking.
#[derive(Clone, Debug)]
pub struct Declaration {
    /// The global name.
    pub name: Name,
    /// The universe parameters, in order.
    pub level_params: Vec<Name>,
    /// The declared type.
    pub ty: Expr,
    /// The kind, with its value.
    pub kind: DeclKind,
}

impl Declaration {
    /// An axiom.
    pub fn axiom(name: Name, level_params: Vec<Name>, ty: Expr) -> Declaration {
        Declaration {
            name,
            level_params,
            ty,
            kind: DeclKind::Axiom,
        }
    }
    /// A definition.
    pub fn definition(
        name: Name,
        level_params: Vec<Name>,
        ty: Expr,
        value: Expr,
        hints: ReducibilityHints,
    ) -> Declaration {
        Declaration {
            name,
            level_params,
            ty,
            kind: DeclKind::Definition { value, hints },
        }
    }
    /// A theorem.
    pub fn theorem(name: Name, level_params: Vec<Name>, ty: Expr, value: Expr) -> Declaration {
        Declaration {
            name,
            level_params,
            ty,
            kind: DeclKind::Theorem { value },
        }
    }
    /// An opaque definition.
    pub fn opaque(name: Name, level_params: Vec<Name>, ty: Expr, value: Expr) -> Declaration {
        Declaration {
            name,
            level_params,
            ty,
            kind: DeclKind::Opaque { value },
        }
    }
    /// The value, if the declaration has one.
    pub fn value(&self) -> Option<Expr> {
        match &self.kind {
            DeclKind::Axiom
            | DeclKind::Inductive(_)
            | DeclKind::Constructor(_)
            | DeclKind::Recursor(_) => None,
            DeclKind::Definition { value, .. }
            | DeclKind::Theorem { value }
            | DeclKind::Opaque { value } => Some(*value),
        }
    }
}

/// The map from name to accepted declaration, together with the expression store.
pub struct Environment {
    /// The store all expressions of this environment live in. Build terms with it.
    pub store: ExprStore,
    pub(crate) decls: HashMap<Name, Declaration>,
}

impl Default for Environment {
    fn default() -> Self {
        Environment::new()
    }
}

impl Environment {
    /// An empty environment.
    pub fn new() -> Environment {
        Environment {
            store: ExprStore::new(),
            decls: HashMap::new(),
        }
    }
    /// Look up an accepted declaration.
    pub fn get(&self, name: &Name) -> Option<&Declaration> {
        self.decls.get(name)
    }
    /// Is the name declared?
    pub fn contains(&self, name: &Name) -> bool {
        self.decls.contains_key(name)
    }
    /// Number of accepted declarations.
    pub fn len(&self) -> usize {
        self.decls.len()
    }
    /// Is the environment empty?
    pub fn is_empty(&self) -> bool {
        self.decls.is_empty()
    }

    /// A type checker over this environment, for terms that may mention the given
    /// universe parameters (use `&[]` for closed terms).
    pub fn type_checker(&mut self, level_params: &[Name]) -> TypeChecker<'_> {
        TypeChecker::new(&mut self.store, &self.decls, level_params.to_vec())
    }

    /// Check a declaration and, only if every check passes, append it. On any error the
    /// environment is unchanged (apart from harmless extra interned expressions).
    pub fn add_declaration(&mut self, decl: Declaration) -> Result<(), KernelError> {
        // An inductive type, a constructor or a recursor may only come from
        // `add_inductive`, which checks the whole block and *derives* the recursor. A
        // forged `Recursor` would be an arbitrary axiom with computation rules.
        if matches!(
            decl.kind,
            DeclKind::Inductive(_) | DeclKind::Constructor(_) | DeclKind::Recursor(_)
        ) {
            return Err(KernelError::Unsupported(
                "inductive declarations are only accepted through add_inductive",
            ));
        }
        // 1. the name is new
        if self.decls.contains_key(&decl.name) {
            return Err(KernelError::AlreadyDeclared(decl.name));
        }
        // 2. universe parameters are distinct
        let mut seen = HashSet::new();
        for p in &decl.level_params {
            if !seen.insert(p) {
                return Err(KernelError::DuplicateLevelParam(p.clone()));
            }
        }
        // 3. closed: loose bound variables first (they are never meaningful), then
        //    free variables
        let value = decl.value();
        // 0. handles from another store would panic when inspected: refuse them first
        for e in std::iter::once(decl.ty).chain(value) {
            if !self.store.owns(e) {
                return Err(KernelError::ForeignExpr);
            }
        }
        for e in std::iter::once(decl.ty).chain(value) {
            if self.store.has_loose_bvars(e) {
                return Err(KernelError::LooseBVar(self.store.loose_bvar_range(e) - 1));
            }
            if self.store.has_fvar(e) {
                return Err(KernelError::FreeVarInDeclaration);
            }
        }
        // 4-8. well-typedness; the checker is dropped before the environment changes
        {
            let mut tc = self.type_checker(&decl.level_params);
            let sort_of_ty = tc.infer(decl.ty)?;
            let level = tc.ensure_sort(sort_of_ty)?;
            if matches!(decl.kind, DeclKind::Theorem { .. }) && !level.is_equiv(&Level::zero()) {
                return Err(KernelError::TheoremNotProp(decl.ty));
            }
            if let Some(v) = value {
                let found = tc.infer(v)?;
                if !tc.is_def_eq(found, decl.ty)? {
                    return Err(KernelError::TypeMismatch {
                        expected: decl.ty,
                        found,
                    });
                }
            }
        }
        self.decls.insert(decl.name.clone(), decl);
        Ok(())
    }
}
