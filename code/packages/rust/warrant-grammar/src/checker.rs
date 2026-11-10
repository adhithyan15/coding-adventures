//! # Warrant Type Checker and Semantic Analysis (VER04d / VER-P007)
//!
//! Enforces scope rules, namespace separation, forward-reference resolution,
//! acyclic model definitions, strict subtyping (Nat <: Int, Bytes<N> <: Bytes),
//! operator typing, exhaustive matches, contract admissibility, and stable diagnostics
//! (`W0201`, `W0202`, `W0203`, `W0301`, `W0401`, `W0602`).

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt;

use crate::fixed::FixedTy;
use crate::parser::{
    AssumptionsDecl, BinaryOp, BoundDecl, Declaration, Expression, FieldDecl, InputDecl,
    LabelsDecl, Literal, MatchArm, ModelDecl, OpDecl, Pattern, ProjKey, QualifiedName, TypeExpr,
    UnaryOp, Version, WarrantFile, Width, MAX_PATTERN_DEPTH,
};
use crate::patterns::{self, PTy, Pat};

/// Most arms one `match` may have (VER04j section 4.3).
pub const MAX_MATCH_ARMS: usize = 256;

/// Names reserved in every document (VER04 section 2).
const V0_BUILTINS: [&str; 18] = [
    "Bool", "Int", "Nat", "U8", "U64", "Bytes", "List", "Option", "Result", "len", "at", "concat",
    "fixed", "div", "Some", "None", "Ok", "Err",
];

/// The six types WX-B adds; reserved only in v1 documents.
const V1_TYPES: [&str; 6] = ["U16", "U32", "I8", "I16", "I32", "I64"];

/// The v1 built-in *functions* (WX-B and WX-C).  Reserved, and in scope, only
/// in v1 documents (VER04j section 1.2): a v0 document may still define its
/// own model called `rem` or `shl` and keeps meaning that definition.
pub const V1_FUNCTIONS: [&str; 31] = [
    "widen",
    "wrap",
    "try",
    "to_int",
    "to_nat",
    "max_value",
    "min_value",
    "checked_add",
    "checked_sub",
    "checked_mul",
    "rem",
    "div_trunc",
    "rem_trunc",
    "unwrap_or",
    "bit_and",
    "bit_or",
    "bit_xor",
    "bit_not",
    "shl",
    "shr",
    "sar",
    "rotl",
    "rotr",
    "popcount",
    "clz",
    "ctz",
    "bit",
    "be_bytes",
    "le_bytes",
    "from_be",
    "from_le",
];

/// Is `name` a built-in reserved in a document of version `version`?
fn is_reserved_builtin(version: Version, name: &str) -> bool {
    V0_BUILTINS.contains(&name)
        || (version == Version::V1 && (V1_TYPES.contains(&name) || V1_FUNCTIONS.contains(&name)))
}

/// Defensive limits for semantic checking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckerLimits {
    pub max_work: usize,
    pub max_depth: usize,
}

impl Default for CheckerLimits {
    fn default() -> Self {
        Self {
            max_work: 1_000_000,
            max_depth: 64,
        }
    }
}

/// Closed diagnostic failure codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticCode {
    /// W0201: Duplicate symbol / test identity / contract declaration.
    W0201,
    /// W0202: Unbound or ambiguous name.
    W0202,
    /// W0203: Cyclic model / alias dependency or prohibited shadowing.
    W0203,
    /// W0301: Type, arity, width, constructor or match-exhaustiveness error.
    W0301,
    /// W0401: Missing or invalid input, bound, label or assumption contract.
    W0401,
    /// W0402: Hostile-input assumption, secrecy downgrade or ambiguous rejection.
    W0402,
    /// W0501: Invalid/missing export binding, constructor/observer or static instantiation.
    W0501,
    /// W0502: Failed binding negative control or fresh-output requirement.
    W0502,
    /// W0602: Incomplete mandatory solver/analyzer/observer evidence; budget exhaustion.
    W0602,
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::W0201 => write!(f, "W0201"),
            Self::W0202 => write!(f, "W0202"),
            Self::W0203 => write!(f, "W0203"),
            Self::W0301 => write!(f, "W0301"),
            Self::W0401 => write!(f, "W0401"),
            Self::W0402 => write!(f, "W0402"),
            Self::W0501 => write!(f, "W0501"),
            Self::W0502 => write!(f, "W0502"),
            Self::W0602 => write!(f, "W0602"),
        }
    }
}

/// Detailed error reason without leaking secret inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckerErrorKind {
    DuplicateSymbol(String),
    DuplicateTestName(String),
    DuplicateParameter(String),
    DuplicateContract(String),
    UnboundName(String),
    UnboundType(String),
    UnboundConstructor(String),
    CyclicDependency(String),
    ShadowedBinding(String),
    TypeMismatch {
        expected: String,
        actual: String,
    },
    ArityMismatch {
        expected: usize,
        actual: usize,
    },
    IncompatibleBranches {
        then_type: String,
        else_type: String,
    },
    IncompatibleMatchArms,
    NonExhaustiveMatch,
    ModelCallsOperation(String),
    InvalidBoundExpression,
    InvalidContract(String),
    MissingContract(String),
    MissingBinding(String),
    UnknownBoundOperation(String),
    InvalidErrorNormalization(String),
    MissingNegativeControl(String),
    MissingFreshRequirement(String),
    WorkBudgetExceeded,
    DepthBudgetExceeded,
    ArithmeticOverflow(String),
    /// WX-A: an arm every value of which earlier arms already take.
    RedundantArm,
    /// WX-A: `t.i` / `r.f` names no component or field of the type.
    NoSuchField(String),
    /// WX-A: projection of a variant that is not a record (use `match`).
    ProjectNonRecord(String),
    /// WX-A: a nullary constructor written with parentheses, `C()`.
    NullaryWithParens(String),
    /// WX-A: a match with more than [`MAX_MATCH_ARMS`] arms.
    TooManyArms,
}

/// Semantic checking error with source span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckerError {
    pub code: DiagnosticCode,
    pub kind: CheckerErrorKind,
    pub span: (usize, usize),
}

impl fmt::Display for CheckerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {:?}", self.code, self.kind)
    }
}

impl std::error::Error for CheckerError {}

/// Semantic types supported in the Warrant pure v0 profile.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Bool,
    Int,
    Nat,
    U8,
    U64,
    /// The six fixed-width types added by WX-B (v1 documents only).
    U16,
    U32,
    I8,
    I16,
    I32,
    I64,
    Bytes,
    FixedBytes(u64),
    FixedBytesParam(String),
    List(Box<Type>),
    Option(Box<Type>),
    Result(Box<Type>, String), // String is declared error type name
    ErrorVariant(String),      // Specific nominal error variant
    /// WX-A: `(T1, ..., Tn)`, structural (VER04j section 4.2).
    Tuple(Vec<Type>),
    /// WX-A: a declared record or variant, nominal, by type name.
    Adt(String),
}

impl Type {
    /// Format type representation for error messages.
    pub fn display_name(&self) -> String {
        match self {
            Self::Bool => "Bool".to_string(),
            Self::Int => "Int".to_string(),
            Self::Nat => "Nat".to_string(),
            Self::U8 => "U8".to_string(),
            Self::U64 => "U64".to_string(),
            Self::U16 => "U16".to_string(),
            Self::U32 => "U32".to_string(),
            Self::I8 => "I8".to_string(),
            Self::I16 => "I16".to_string(),
            Self::I32 => "I32".to_string(),
            Self::I64 => "I64".to_string(),
            Self::Bytes => "Bytes".to_string(),
            Self::FixedBytes(n) => format!("Bytes<{n}>"),
            Self::FixedBytesParam(p) => format!("Bytes<{p}>"),
            Self::List(inner) => format!("List<{}>", inner.display_name()),
            Self::Option(inner) => format!("Option<{}>", inner.display_name()),
            Self::Result(inner, err) => format!("Result<{}, {err}>", inner.display_name()),
            Self::ErrorVariant(v) => format!("ErrorVariant({v})"),
            Self::Tuple(ts) => {
                let items: Vec<String> = ts.iter().map(Type::display_name).collect();
                format!("({})", items.join(", "))
            }
            Self::Adt(n) => n.clone(),
        }
    }

    /// The fixed-width type this semantic type denotes, if it is one of the
    /// eight (`U8`, `U64` and the six v1 additions).
    pub fn fixed(&self) -> Option<FixedTy> {
        Some(match self {
            Self::U8 => FixedTy::U8,
            Self::U16 => FixedTy::U16,
            Self::U32 => FixedTy::U32,
            Self::U64 => FixedTy::U64,
            Self::I8 => FixedTy::I8,
            Self::I16 => FixedTy::I16,
            Self::I32 => FixedTy::I32,
            Self::I64 => FixedTy::I64,
            _ => return None,
        })
    }

    /// The semantic type of a fixed-width type.
    pub fn from_fixed(ty: FixedTy) -> Type {
        match ty {
            FixedTy::U8 => Type::U8,
            FixedTy::U16 => Type::U16,
            FixedTy::U32 => Type::U32,
            FixedTy::U64 => Type::U64,
            FixedTy::I8 => Type::I8,
            FixedTy::I16 => Type::I16,
            FixedTy::I32 => Type::I32,
            FixedTy::I64 => Type::I64,
        }
    }

    /// Checks if a value of type `self` can be supplied where `target` is expected.
    /// Strict subtyping rules in v0:
    /// - Nat <: Int
    /// - Bytes<N> <: Bytes
    pub fn is_assignable_to(&self, target: &Type) -> bool {
        if self == target {
            return true;
        }
        match (self, target) {
            (Type::Nat, Type::Int) => true,
            (Type::FixedBytes(_), Type::Bytes) => true,
            (Type::FixedBytesParam(_), Type::Bytes) => true,
            (Type::List(a), Type::List(b)) => a == b,
            (Type::Option(a), Type::Option(b)) => a.is_assignable_to(b),
            (Type::Result(ok_a, err_a), Type::Result(ok_b, err_b)) => {
                err_a == err_b && ok_a.is_assignable_to(ok_b)
            }
            _ => false,
        }
    }

    /// Computes the least upper bound (common supertype) under widening.
    pub fn common_type(a: &Type, b: &Type) -> Option<Type> {
        if a == b {
            return Some(a.clone());
        }
        match (a, b) {
            (Type::Nat, Type::Int) | (Type::Int, Type::Nat) => Some(Type::Int),
            (Type::FixedBytes(_), Type::Bytes) | (Type::Bytes, Type::FixedBytes(_)) => {
                Some(Type::Bytes)
            }
            (Type::FixedBytesParam(_), Type::Bytes) | (Type::Bytes, Type::FixedBytesParam(_)) => {
                Some(Type::Bytes)
            }
            (Type::FixedBytes(_), Type::FixedBytes(_)) => Some(Type::Bytes),
            (Type::FixedBytesParam(_), Type::FixedBytesParam(_)) => Some(Type::Bytes),
            (Type::Option(inner_a), Type::Option(inner_b)) => {
                Self::common_type(inner_a, inner_b).map(|t| Type::Option(Box::new(t)))
            }
            (Type::Result(ok_a, err_a), Type::Result(ok_b, err_b)) if err_a == err_b => {
                Self::common_type(ok_a, ok_b).map(|t| Type::Result(Box::new(t), err_a.clone()))
            }
            _ => None,
        }
    }
}

/// Checked operation metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedOp {
    pub name: String,
    pub static_params: Vec<String>,
    pub params: Vec<(String, Type)>,
    pub return_type: Type,
    pub is_fresh: bool,
    pub span: (usize, usize),
}

/// Checked pure model metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedModel {
    pub name: String,
    pub params: Vec<(String, Type)>,
    pub target_type: Type,
    pub body_type: Type,
    pub span: (usize, usize),
}

/// Checked error definition metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedError {
    pub name: String,
    pub variants: BTreeSet<String>,
    pub span: (usize, usize),
}

/// One constructor of a checked record or variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedCtor {
    pub name: String,
    pub fields: Vec<(String, Type)>,
}

/// A checked record (one constructor named like the type) or variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedAdt {
    pub name: String,
    pub is_record: bool,
    pub ctors: Vec<CheckedCtor>,
    pub span: (usize, usize),
}

/// Successfully checked Warrant module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedWarrantModule {
    pub module_name: String,
    pub type_aliases: BTreeMap<String, Type>,
    pub errors: BTreeMap<String, CheckedError>,
    pub models: BTreeMap<String, CheckedModel>,
    pub operations: BTreeMap<String, CheckedOp>,
    pub examples: Vec<String>,
    pub claims: Vec<String>,
}

/// A declared record or variant before its field types are resolved: whether it
/// is a record, its constructors with their field declarations, and its span.
type RawAdt<'a> = (bool, Vec<(String, Vec<&'a FieldDecl<'a>>)>, (usize, usize));

/// State tracking for semantic checking.
pub struct Checker<'a> {
    file: &'a WarrantFile<'a>,
    limits: CheckerLimits,
    work_count: usize,
    depth: usize,

    // Module namespace symbols
    type_aliases: BTreeMap<String, TypeExpr<'a>>,
    resolved_aliases: BTreeMap<String, Type>,
    errors: BTreeMap<String, (BTreeSet<String>, (usize, usize))>,
    models: BTreeMap<String, &'a ModelDecl<'a>>,
    operations: BTreeMap<String, &'a OpDecl<'a>>,

    // WX-A: declared records and variants.  `adt_raw` is filled by the catalog
    // pass (names and unresolved field types), `adts` once the field types are
    // resolved (pass 2b), `ctor_index` maps a constructor name to
    // (type name, constructor index).
    adt_raw: BTreeMap<String, RawAdt<'a>>,
    adts: BTreeMap<String, CheckedAdt>,
    ctor_index: BTreeMap<String, (String, usize)>,

    // Test namespace symbols
    test_symbols: HashSet<String>,

    // Contracts
    labels: BTreeMap<String, &'a LabelsDecl<'a>>,
    bounds: BTreeMap<String, &'a BoundDecl<'a>>,
    inputs: BTreeMap<String, &'a InputDecl<'a>>,
}

impl<'a> Checker<'a> {
    pub fn new(file: &'a WarrantFile<'a>, limits: CheckerLimits) -> Self {
        Self {
            file,
            limits,
            work_count: 0,
            depth: limits.max_depth,
            type_aliases: BTreeMap::new(),
            resolved_aliases: BTreeMap::new(),
            errors: BTreeMap::new(),
            models: BTreeMap::new(),
            operations: BTreeMap::new(),
            adt_raw: BTreeMap::new(),
            adts: BTreeMap::new(),
            ctor_index: BTreeMap::new(),
            test_symbols: HashSet::new(),
            labels: BTreeMap::new(),
            bounds: BTreeMap::new(),
            inputs: BTreeMap::new(),
        }
    }

    fn charge_work(&mut self, span: (usize, usize)) -> Result<(), CheckerError> {
        self.work_count += 1;
        if self.work_count > self.limits.max_work {
            return Err(CheckerError {
                code: DiagnosticCode::W0602,
                kind: CheckerErrorKind::WorkBudgetExceeded,
                span,
            });
        }
        Ok(())
    }

    fn enter_scope<F, R>(&mut self, span: (usize, usize), f: F) -> Result<R, CheckerError>
    where
        F: FnOnce(&mut Self) -> Result<R, CheckerError>,
    {
        if self.depth == 0 {
            return Err(CheckerError {
                code: DiagnosticCode::W0602,
                kind: CheckerErrorKind::DepthBudgetExceeded,
                span,
            });
        }
        self.depth -= 1;
        let res = f(self);
        self.depth += 1;
        res
    }

    /// Primary entry point: perform all checking passes.
    pub fn check(mut self) -> Result<CheckedWarrantModule, CheckerError> {
        // Pass 1: Catalog declarations into namespaces and detect duplicates
        self.catalog_declarations()?;

        // Pass 2: Resolve type aliases and detect cycles among types
        self.resolve_type_aliases()?;

        // Pass 2b (WX-A): resolve record/variant field types, reject cycles
        self.resolve_adts()?;

        // Pass 3: Validate model function dependencies (must be acyclic and not call operations)
        self.validate_model_dependencies()?;

        // Pass 4: Validate operations (parameters, refinements)
        let checked_ops = self.validate_operations()?;

        // Pass 5: Type-check model function bodies
        let checked_models = self.typecheck_models()?;

        // Pass 6: Type-check examples and claims
        self.typecheck_examples_and_claims()?;

        // Pass 7: Check contracts (labels, bounds, inputs, assumptions)
        self.check_contracts()?;

        // Construct CheckedWarrantModule
        let mut checked_errors = BTreeMap::new();
        for (name, (variants, span)) in &self.errors {
            checked_errors.insert(
                name.clone(),
                CheckedError {
                    name: name.clone(),
                    variants: variants.clone(),
                    span: *span,
                },
            );
        }

        let mut examples = Vec::new();
        let mut claims = Vec::new();
        for decl in &self.file.declarations {
            match decl {
                Declaration::Example(e) => examples.push(e.name.to_string()),
                Declaration::Claim(c) => claims.push(c.name.to_string()),
                _ => {}
            }
        }

        Ok(CheckedWarrantModule {
            module_name: self.file.name.to_string(),
            type_aliases: self.resolved_aliases,
            errors: checked_errors,
            models: checked_models,
            operations: checked_ops,
            examples,
            claims,
        })
    }

    // ── Pass 1: Catalog Declarations ────────────────────────────────────────

    fn catalog_declarations(&mut self) -> Result<(), CheckerError> {
        let mut module_symbols = HashSet::new();
        let mut seen_assumptions = HashSet::new();

        // Built-in types and callables reserved in module symbol namespace
        // (the v1 names only in a v1 document, VER04j section 1.2).
        for b in V0_BUILTINS {
            module_symbols.insert(b.to_string());
        }
        if self.file.version == Version::V1 {
            for b in V1_TYPES.iter().chain(V1_FUNCTIONS.iter()) {
                module_symbols.insert((*b).to_string());
            }
        }

        for decl in &self.file.declarations {
            self.charge_work((0, 0))?;
            match decl {
                Declaration::Model(m) => {
                    if !module_symbols.insert(m.name.to_string()) {
                        return Err(CheckerError {
                            code: DiagnosticCode::W0201,
                            kind: CheckerErrorKind::DuplicateSymbol(m.name.to_string()),
                            span: m.span,
                        });
                    }
                    if m.body.is_none() {
                        // Type alias: model Name = type
                        self.type_aliases
                            .insert(m.name.to_string(), m.target_type.clone());
                    } else {
                        // Pure model function: model Name(...) -> ... = expr
                        self.models.insert(m.name.to_string(), m);
                    }
                }
                Declaration::Error(e) => {
                    if !module_symbols.insert(e.name.to_string()) {
                        return Err(CheckerError {
                            code: DiagnosticCode::W0201,
                            kind: CheckerErrorKind::DuplicateSymbol(e.name.to_string()),
                            span: e.span,
                        });
                    }
                    let mut variants = BTreeSet::new();
                    for v in &e.variants {
                        if !variants.insert(v.to_string()) {
                            return Err(CheckerError {
                                code: DiagnosticCode::W0201,
                                kind: CheckerErrorKind::DuplicateSymbol(format!(
                                    "{}.{}",
                                    e.name, v
                                )),
                                span: e.span,
                            });
                        }
                    }
                    self.errors.insert(e.name.to_string(), (variants, e.span));
                }
                Declaration::Op(op) => {
                    if !module_symbols.insert(op.name.to_string()) {
                        return Err(CheckerError {
                            code: DiagnosticCode::W0201,
                            kind: CheckerErrorKind::DuplicateSymbol(op.name.to_string()),
                            span: op.span,
                        });
                    }
                    self.operations.insert(op.name.to_string(), op);
                }
                Declaration::Record(r) => {
                    if !module_symbols.insert(r.name.to_string()) {
                        return Err(CheckerError {
                            code: DiagnosticCode::W0201,
                            kind: CheckerErrorKind::DuplicateSymbol(r.name.to_string()),
                            span: r.span,
                        });
                    }
                    // A record's name is both its type and its one constructor.
                    self.ctor_index
                        .insert(r.name.to_string(), (r.name.to_string(), 0));
                    self.adt_raw.insert(
                        r.name.to_string(),
                        (
                            true,
                            vec![(r.name.to_string(), r.fields.iter().collect())],
                            r.span,
                        ),
                    );
                }
                Declaration::Variant(v) => {
                    if !module_symbols.insert(v.name.to_string()) {
                        return Err(CheckerError {
                            code: DiagnosticCode::W0201,
                            kind: CheckerErrorKind::DuplicateSymbol(v.name.to_string()),
                            span: v.span,
                        });
                    }
                    let mut ctors = Vec::new();
                    for (i, c) in v.ctors.iter().enumerate() {
                        if !module_symbols.insert(c.name.to_string()) {
                            return Err(CheckerError {
                                code: DiagnosticCode::W0201,
                                kind: CheckerErrorKind::DuplicateSymbol(c.name.to_string()),
                                span: c.span,
                            });
                        }
                        self.ctor_index
                            .insert(c.name.to_string(), (v.name.to_string(), i));
                        ctors.push((c.name.to_string(), c.fields.iter().collect()));
                    }
                    self.adt_raw
                        .insert(v.name.to_string(), (false, ctors, v.span));
                }
                Declaration::Example(ex) => {
                    if !self.test_symbols.insert(ex.name.to_string()) {
                        return Err(CheckerError {
                            code: DiagnosticCode::W0201,
                            kind: CheckerErrorKind::DuplicateTestName(ex.name.to_string()),
                            span: ex.span,
                        });
                    }
                }
                Declaration::Claim(cl) => {
                    if !self.test_symbols.insert(cl.name.to_string()) {
                        return Err(CheckerError {
                            code: DiagnosticCode::W0201,
                            kind: CheckerErrorKind::DuplicateTestName(cl.name.to_string()),
                            span: cl.span,
                        });
                    }
                }
                Declaration::Labels(lbl) => {
                    if self.labels.insert(lbl.name.to_string(), lbl).is_some() {
                        return Err(CheckerError {
                            code: DiagnosticCode::W0201,
                            kind: CheckerErrorKind::DuplicateContract(format!(
                                "labels {}",
                                lbl.name
                            )),
                            span: lbl.span,
                        });
                    }
                }
                Declaration::Bound(bnd) => {
                    if self.bounds.insert(bnd.name.to_string(), bnd).is_some() {
                        return Err(CheckerError {
                            code: DiagnosticCode::W0201,
                            kind: CheckerErrorKind::DuplicateContract(format!(
                                "bound {}",
                                bnd.name
                            )),
                            span: bnd.span,
                        });
                    }
                }
                Declaration::Input(inp) => {
                    let target_name = inp.target.segments.join(".");
                    if self.inputs.insert(target_name.clone(), inp).is_some() {
                        return Err(CheckerError {
                            code: DiagnosticCode::W0201,
                            kind: CheckerErrorKind::DuplicateContract(format!(
                                "input {target_name}"
                            )),
                            span: inp.span,
                        });
                    }
                }
                Declaration::Assumptions(a) => {
                    let (kind, span) = match a {
                        AssumptionsDecl::Platform { span, .. } => ("platform", *span),
                        AssumptionsDecl::Environment { span, .. } => ("environment", *span),
                    };
                    if !seen_assumptions.insert(kind) {
                        return Err(CheckerError {
                            code: DiagnosticCode::W0201,
                            kind: CheckerErrorKind::DuplicateContract(format!("assume {kind}")),
                            span,
                        });
                    }
                }
            }
        }
        Ok(())
    }

    // ── Pass 2: Type Alias Resolution and Cycle Detection ──────────────────

    fn resolve_type_aliases(&mut self) -> Result<(), CheckerError> {
        let alias_names: Vec<String> = self.type_aliases.keys().cloned().collect();

        // Detect dependency cycles among aliases using DFS
        for name in &alias_names {
            let mut visited = Vec::new();
            self.check_alias_cycle(name, &mut visited)?;
        }

        // Topologically resolve all aliases
        for name in &alias_names {
            let raw_type = self.type_aliases.get(name).unwrap().clone();
            let resolved = self.resolve_type(&raw_type, &[])?;
            self.resolved_aliases.insert(name.clone(), resolved);
        }

        Ok(())
    }

    fn check_alias_cycle(
        &mut self,
        current: &str,
        path: &mut Vec<String>,
    ) -> Result<(), CheckerError> {
        self.charge_work((0, 0))?;
        if let Some(pos) = path.iter().position(|x| x == current) {
            let cycle = path[pos..].join(" -> ") + " -> " + current;
            return Err(CheckerError {
                code: DiagnosticCode::W0203,
                kind: CheckerErrorKind::CyclicDependency(cycle),
                span: (0, 0),
            });
        }

        if let Some(ty_expr) = self.type_aliases.get(current).cloned() {
            path.push(current.to_string());
            let deps = self.collect_type_dependencies(&ty_expr);
            for dep in deps {
                if self.type_aliases.contains_key(&dep) {
                    self.enter_scope((0, 0), |this| this.check_alias_cycle(&dep, path))?;
                }
            }
            path.pop();
        }
        Ok(())
    }

    fn collect_type_dependencies(&self, ty: &TypeExpr<'a>) -> Vec<String> {
        let mut deps = Vec::new();
        match ty {
            TypeExpr::Named { name, .. } => deps.push((*name).to_string()),
            TypeExpr::List { inner, .. } | TypeExpr::Option { inner, .. } => {
                deps.extend(self.collect_type_dependencies(inner));
            }
            TypeExpr::Result {
                ok_type, err_name, ..
            } => {
                deps.extend(self.collect_type_dependencies(ok_type));
                deps.push((*err_name).to_string());
            }
            TypeExpr::Tuple { elems, .. } => {
                for e in elems {
                    deps.extend(self.collect_type_dependencies(e));
                }
            }
            TypeExpr::Bytes { .. } => {}
        }
        deps
    }

    fn resolve_type(
        &mut self,
        ty: &TypeExpr<'a>,
        static_params: &[String],
    ) -> Result<Type, CheckerError> {
        let span = type_expr_span(ty);
        self.charge_work(span)?;
        self.enter_scope(span, |this| match ty {
            TypeExpr::Named { name, span } => match *name {
                "Bool" => Ok(Type::Bool),
                "Int" => Ok(Type::Int),
                "Nat" => Ok(Type::Nat),
                "U8" => Ok(Type::U8),
                "U64" => Ok(Type::U64),
                "Bytes" => Ok(Type::Bytes),
                // WX-B types exist only in a v1 document; in v0 they fall
                // through to the alias lookup and are unbound names (W0202).
                other
                    if this.file.version == Version::V1 && FixedTy::from_name(other).is_some() =>
                {
                    Ok(Type::from_fixed(FixedTy::from_name(other).unwrap()))
                }
                other => {
                    if let Some(resolved) = this.resolved_aliases.get(other) {
                        return Ok(resolved.clone());
                    }
                    if let Some(raw) = this.type_aliases.get(other).cloned() {
                        return this.resolve_type(&raw, static_params);
                    }
                    if this.errors.contains_key(other) {
                        return Ok(Type::ErrorVariant(other.to_string()));
                    }
                    if this.adt_raw.contains_key(other) {
                        return Ok(Type::Adt(other.to_string()));
                    }
                    Err(CheckerError {
                        code: DiagnosticCode::W0202,
                        kind: CheckerErrorKind::UnboundType(other.to_string()),
                        span: *span,
                    })
                }
            },
            TypeExpr::Bytes { width, span } => match width {
                None => Ok(Type::Bytes),
                Some(Width::Literal(n)) => Ok(Type::FixedBytes(*n)),
                Some(Width::Named(param)) => {
                    if static_params.contains(&param.to_string()) {
                        Ok(Type::FixedBytesParam(param.to_string()))
                    } else {
                        Err(CheckerError {
                            code: DiagnosticCode::W0202,
                            kind: CheckerErrorKind::UnboundName(param.to_string()),
                            span: *span,
                        })
                    }
                }
            },
            TypeExpr::Tuple { elems, .. } => {
                let mut ts = Vec::with_capacity(elems.len());
                for e in elems {
                    ts.push(this.resolve_type(e, static_params)?);
                }
                Ok(Type::Tuple(ts))
            }
            TypeExpr::List { inner, .. } => {
                let inner_ty = this.resolve_type(inner, static_params)?;
                Ok(Type::List(Box::new(inner_ty)))
            }
            TypeExpr::Option { inner, .. } => {
                let inner_ty = this.resolve_type(inner, static_params)?;
                Ok(Type::Option(Box::new(inner_ty)))
            }
            TypeExpr::Result {
                ok_type,
                err_name,
                span,
            } => {
                let ok_ty = this.resolve_type(ok_type, static_params)?;
                if !this.errors.contains_key(*err_name) {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0202,
                        kind: CheckerErrorKind::UnboundType((*err_name).to_string()),
                        span: *span,
                    });
                }
                Ok(Type::Result(Box::new(ok_ty), (*err_name).to_string()))
            }
        })
    }

    // ── Pass 2b (WX-A): records and variants ───────────────────────────────

    /// Resolve every field type, then require the "type mentions type"
    /// relation to be acyclic through tuples, records, variants, `List`,
    /// `Option`, `Result` and aliases (VER04j section 4.2): a cycle is W0203.
    /// Recursion would make a domain infinite and evaluation partial.
    fn resolve_adts(&mut self) -> Result<(), CheckerError> {
        let names: Vec<String> = self.adt_raw.keys().cloned().collect();
        for name in &names {
            let (is_record, ctors, span) = self.adt_raw[name].clone();
            let mut checked = Vec::new();
            for (cname, fields) in ctors {
                let mut fs = Vec::new();
                for f in fields {
                    let ty = self.resolve_type(&f.ty, &[])?;
                    if matches!(ty, Type::ErrorVariant(_)) {
                        // Error types are payload-free and finite; they are not
                        // value types (VER04j section 4.2).
                        return Err(CheckerError {
                            code: DiagnosticCode::W0301,
                            kind: CheckerErrorKind::TypeMismatch {
                                expected: "a value type".to_string(),
                                actual: ty.display_name(),
                            },
                            span: f.span,
                        });
                    }
                    fs.push((f.name.to_string(), ty));
                }
                checked.push(CheckedCtor {
                    name: cname,
                    fields: fs,
                });
            }
            self.adts.insert(
                name.clone(),
                CheckedAdt {
                    name: name.clone(),
                    is_record,
                    ctors: checked,
                    span,
                },
            );
        }
        let mut done = HashSet::new();
        for name in &names {
            let mut path = Vec::new();
            self.check_adt_cycle(name, &mut path, &mut done)?;
        }
        Ok(())
    }

    /// Depth-first search for a cycle in the mention graph; a type whose whole
    /// subtree is cycle-free is remembered in `done` so a diamond of records
    /// (each mentioning the previous one twice) costs linear, not exponential,
    /// work.
    fn check_adt_cycle(
        &mut self,
        current: &str,
        path: &mut Vec<String>,
        done: &mut HashSet<String>,
    ) -> Result<(), CheckerError> {
        self.charge_work((0, 0))?;
        if done.contains(current) {
            return Ok(());
        }
        if let Some(pos) = path.iter().position(|x| x == current) {
            let cycle = path[pos..].join(" -> ") + " -> " + current;
            let span = self.adts.get(current).map(|a| a.span).unwrap_or((0, 0));
            return Err(CheckerError {
                code: DiagnosticCode::W0203,
                kind: CheckerErrorKind::CyclicDependency(cycle),
                span,
            });
        }
        let mut mentioned = Vec::new();
        if let Some(adt) = self.adts.get(current) {
            for c in &adt.ctors {
                for (_, t) in &c.fields {
                    Self::collect_adts(t, &mut mentioned);
                }
            }
        }
        path.push(current.to_string());
        for m in mentioned {
            self.enter_scope((0, 0), |this| this.check_adt_cycle(&m, path, done))?;
        }
        path.pop();
        done.insert(current.to_string());
        Ok(())
    }

    fn collect_adts(t: &Type, out: &mut Vec<String>) {
        match t {
            Type::Adt(n) => out.push(n.clone()),
            Type::Tuple(ts) => ts.iter().for_each(|t| Self::collect_adts(t, out)),
            Type::List(i) | Type::Option(i) | Type::Result(i, _) => Self::collect_adts(i, out),
            _ => {}
        }
    }

    /// A binder (parameter, quantifier binder or pattern variable) may not
    /// spell a constructor (VER04j section 4.2): W0203.
    fn check_not_ctor(&self, name: &str, span: (usize, usize)) -> Result<(), CheckerError> {
        if self.ctor_index.contains_key(name) {
            return Err(CheckerError {
                code: DiagnosticCode::W0203,
                kind: CheckerErrorKind::ShadowedBinding(name.to_string()),
                span,
            });
        }
        Ok(())
    }

    // ── Pass 3: Model Dependencies and Acyclic Validation ──────────────────

    fn validate_model_dependencies(&mut self) -> Result<(), CheckerError> {
        let model_names: Vec<String> = self.models.keys().cloned().collect();

        // Build call graph for model functions
        let mut call_graph: HashMap<String, HashSet<String>> = HashMap::new();
        let model_entries: Vec<(String, Option<Expression<'a>>, (usize, usize))> = self
            .models
            .iter()
            .map(|(k, v)| (k.clone(), v.body.clone(), v.span))
            .collect();

        for (name, body_opt, span) in model_entries {
            let mut calls = HashSet::new();
            if let Some(body) = &body_opt {
                self.collect_function_calls(body, &mut calls)?;
            }
            // Check if any model calls an operation (strictly forbidden in pure models!)
            for called in &calls {
                if self.operations.contains_key(called) {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0301,
                        kind: CheckerErrorKind::ModelCallsOperation(called.clone()),
                        span,
                    });
                }
            }
            call_graph.insert(name, calls);
        }

        // Detect recursion or mutual recursion in model functions
        for name in &model_names {
            let mut visited = Vec::new();
            self.check_model_cycle(name, &call_graph, &mut visited)?;
        }

        Ok(())
    }

    fn collect_function_calls(
        &mut self,
        expr: &Expression<'a>,
        calls: &mut HashSet<String>,
    ) -> Result<(), CheckerError> {
        let span = body_span(expr);
        self.charge_work(span)?;
        self.enter_scope(span, |this| {
            match expr {
                Expression::Reference { name, args, .. } => {
                    if args.is_some() && name.segments.len() == 1 {
                        calls.insert(name.segments[0].to_string());
                    }
                    if let Some(args_list) = args {
                        for arg in args_list {
                            this.collect_function_calls(arg, calls)?;
                        }
                    }
                }
                Expression::Binary { left, right, .. } => {
                    this.collect_function_calls(left, calls)?;
                    this.collect_function_calls(right, calls)?;
                }
                Expression::Unary { operand, .. } | Expression::Grouped { inner: operand, .. } => {
                    this.collect_function_calls(operand, calls)?;
                }
                Expression::Conditional {
                    condition,
                    then_branch,
                    else_branch,
                    ..
                } => {
                    this.collect_function_calls(condition, calls)?;
                    this.collect_function_calls(then_branch, calls)?;
                    this.collect_function_calls(else_branch, calls)?;
                }
                Expression::Match { target, arms, .. } => {
                    this.collect_function_calls(target, calls)?;
                    for arm in arms {
                        this.collect_function_calls(&arm.body, calls)?;
                    }
                }
                Expression::Quantified { body, .. } => {
                    this.collect_function_calls(body, calls)?;
                }
                Expression::List { elements, .. } => {
                    for el in elements {
                        this.collect_function_calls(el, calls)?;
                    }
                }
                Expression::Tuple { elements, .. } => {
                    for el in elements {
                        this.collect_function_calls(el, calls)?;
                    }
                }
                Expression::Project { base, .. } => {
                    this.collect_function_calls(base, calls)?;
                }
                Expression::Literal { .. } => {}
            }
            Ok(())
        })
    }

    fn check_model_cycle(
        &mut self,
        current: &str,
        graph: &HashMap<String, HashSet<String>>,
        path: &mut Vec<String>,
    ) -> Result<(), CheckerError> {
        self.charge_work((0, 0))?;
        if let Some(pos) = path.iter().position(|x| x == current) {
            let cycle = path[pos..].join(" -> ") + " -> " + current;
            return Err(CheckerError {
                code: DiagnosticCode::W0203,
                kind: CheckerErrorKind::CyclicDependency(cycle),
                span: (0, 0),
            });
        }

        if let Some(neighbors) = graph.get(current) {
            path.push(current.to_string());
            for next in neighbors {
                if self.models.contains_key(next) {
                    self.enter_scope((0, 0), |this| this.check_model_cycle(next, graph, path))?;
                }
            }
            path.pop();
        }
        Ok(())
    }

    // ── Pass 4: Validate Operations ────────────────────────────────────────

    fn validate_operations(&mut self) -> Result<BTreeMap<String, CheckedOp>, CheckerError> {
        let mut checked_ops = BTreeMap::new();
        let op_decls: Vec<OpDecl<'a>> = self.operations.values().map(|op| (*op).clone()).collect();

        for op in op_decls {
            let mut static_params = Vec::new();
            for sp in &op.static_params {
                static_params.push(sp.name.to_string());
            }
            let mut params = Vec::new();
            let mut env = HashMap::new();
            for p in &op.params {
                if env.contains_key(p.name) {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0201,
                        kind: CheckerErrorKind::DuplicateParameter(p.name.to_string()),
                        span: p.span,
                    });
                }
                self.check_not_ctor(p.name, p.span)?;
                let ty = self.resolve_type(&p.param_type, &static_params)?;
                env.insert(p.name.to_string(), ty.clone());
                params.push((p.name.to_string(), ty));
            }
            for p in &op.params {
                if let Some(refinement) = &p.refinement {
                    let ref_ty =
                        self.typecheck_expression(refinement, &env, Some(&Type::Bool), false)?;
                    if ref_ty != Type::Bool {
                        return Err(CheckerError {
                            code: DiagnosticCode::W0301,
                            kind: CheckerErrorKind::TypeMismatch {
                                expected: "Bool".to_string(),
                                actual: ref_ty.display_name(),
                            },
                            span: body_span(refinement),
                        });
                    }
                }
            }
            let return_type = self.resolve_type(&op.return_type, &static_params)?;
            checked_ops.insert(
                op.name.to_string(),
                CheckedOp {
                    name: op.name.to_string(),
                    static_params,
                    params,
                    return_type,
                    is_fresh: op.is_fresh,
                    span: op.span,
                },
            );
        }
        Ok(checked_ops)
    }

    // ── Pass 5: Type-Check Models ──────────────────────────────────────────

    fn typecheck_models(&mut self) -> Result<BTreeMap<String, CheckedModel>, CheckerError> {
        let mut checked_models = BTreeMap::new();
        let model_decls: Vec<ModelDecl<'a>> = self.models.values().map(|m| (*m).clone()).collect();

        for model in model_decls {
            let mut params = Vec::new();
            let mut env: HashMap<String, Type> = HashMap::new();

            for p in &model.params {
                if env.contains_key(p.name) {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0201,
                        kind: CheckerErrorKind::DuplicateParameter(p.name.to_string()),
                        span: p.span,
                    });
                }
                self.check_not_ctor(p.name, p.span)?;
                let param_type = self.resolve_type(&p.param_type, &[])?;
                env.insert(p.name.to_string(), param_type.clone());
                params.push((p.name.to_string(), param_type));
            }

            for p in &model.params {
                if let Some(refinement) = &p.refinement {
                    let ref_ty =
                        self.typecheck_expression(refinement, &env, Some(&Type::Bool), false)?;
                    if ref_ty != Type::Bool {
                        return Err(CheckerError {
                            code: DiagnosticCode::W0301,
                            kind: CheckerErrorKind::TypeMismatch {
                                expected: "Bool".to_string(),
                                actual: ref_ty.display_name(),
                            },
                            span: body_span(refinement),
                        });
                    }
                }
            }

            let target_type = self.resolve_type(&model.target_type, &[])?;

            if let Some(body) = &model.body {
                let body_span = body_span(body);
                let body_type = self.typecheck_expression(body, &env, Some(&target_type), false)?;
                if !body_type.is_assignable_to(&target_type) {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0301,
                        kind: CheckerErrorKind::TypeMismatch {
                            expected: target_type.display_name(),
                            actual: body_type.display_name(),
                        },
                        span: body_span,
                    });
                }

                checked_models.insert(
                    model.name.to_string(),
                    CheckedModel {
                        name: model.name.to_string(),
                        params,
                        target_type,
                        body_type,
                        span: model.span,
                    },
                );
            }
        }

        Ok(checked_models)
    }

    // ── Pass 6: Type-Check Examples and Claims ─────────────────────────────

    fn typecheck_examples_and_claims(&mut self) -> Result<(), CheckerError> {
        for decl in &self.file.declarations {
            match decl {
                Declaration::Example(ex) => {
                    let env = HashMap::new();
                    let ty = self.typecheck_expression(&ex.body, &env, Some(&Type::Bool), true)?;
                    if ty != Type::Bool {
                        return Err(CheckerError {
                            code: DiagnosticCode::W0301,
                            kind: CheckerErrorKind::TypeMismatch {
                                expected: "Bool".to_string(),
                                actual: ty.display_name(),
                            },
                            span: ex.span,
                        });
                    }
                }
                Declaration::Claim(cl) => {
                    let env = HashMap::new();
                    let ty = self.typecheck_expression(&cl.body, &env, Some(&Type::Bool), true)?;
                    if ty != Type::Bool {
                        return Err(CheckerError {
                            code: DiagnosticCode::W0301,
                            kind: CheckerErrorKind::TypeMismatch {
                                expected: "Bool".to_string(),
                                actual: ty.display_name(),
                            },
                            span: cl.span,
                        });
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    // ── Pass 7: Check Contracts ────────────────────────────────────────────

    fn check_contracts(&mut self) -> Result<(), CheckerError> {
        // All operations with parameters must have secrecy labels
        for (op_name, op) in &self.operations {
            if !op.params.is_empty() && !self.labels.contains_key(op_name) {
                return Err(CheckerError {
                    code: DiagnosticCode::W0401,
                    kind: CheckerErrorKind::MissingContract(format!(
                        "Missing secrecy labels contract for operation {op_name}"
                    )),
                    span: op.span,
                });
            }
        }

        // Labels
        let label_list: Vec<(String, &'a LabelsDecl<'a>)> = self
            .labels
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        for (op_name, lbl) in &label_list {
            let op: &'a OpDecl<'a> = *self.operations.get(op_name).ok_or(CheckerError {
                code: DiagnosticCode::W0401,
                kind: CheckerErrorKind::MissingContract(format!("Unknown op {op_name} in labels")),
                span: lbl.span,
            })?;

            let op_param_names: HashSet<&str> = op.params.iter().map(|p| p.name).collect();
            let mut labeled_params = HashSet::new();

            for row in &lbl.rows {
                if row.name.segments.is_empty() || row.name.segments.len() > 2 {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0401,
                        kind: CheckerErrorKind::InvalidContract(format!(
                            "Label target cannot exceed 2 segments: {}",
                            row.name.segments.join(".")
                        )),
                        span: row.span,
                    });
                }
                let first_seg = row.name.segments[0];
                if !op_param_names.contains(first_seg) {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0401,
                        kind: CheckerErrorKind::InvalidContract(format!(
                            "Unknown parameter {first_seg} in labels for {op_name}"
                        )),
                        span: row.span,
                    });
                }
                if row.name.segments.len() == 2 {
                    let proj = row.name.segments[1];
                    let record_field = if self.file.version == Version::V1 {
                        let static_names: Vec<String> =
                            op.static_params.iter().map(|sp| sp.name.to_string()).collect();
                        let pty = op
                            .params
                            .iter()
                            .find(|p| p.name == first_seg)
                            .map(|p| p.param_type.clone());
                        match pty {
                            Some(t) => match self.resolve_type(&t, &static_names)? {
                                Type::Adt(n) => self.adts.get(&n).is_some_and(|a| {
                                    a.is_record
                                        && a.ctors[0].fields.iter().any(|(f, _)| f == proj)
                                }),
                                _ => false,
                            },
                            None => false,
                        }
                    } else {
                        false
                    };
                    if !record_field && proj != "contents" && proj != "length" {
                        return Err(CheckerError {
                            code: DiagnosticCode::W0401,
                            kind: CheckerErrorKind::InvalidContract(format!(
                                "Invalid projection .{proj}, expected .contents or .length"
                            )),
                            span: row.span,
                        });
                    }
                }
                labeled_params.insert(first_seg);
            }

            // All operation parameters must have an explicit secrecy label
            for param in &op.params {
                if !labeled_params.contains(param.name) {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0401,
                        kind: CheckerErrorKind::MissingContract(format!(
                            "Missing secrecy label for parameter {} in {}",
                            param.name, op_name
                        )),
                        span: lbl.span,
                    });
                }
            }
        }

        // Bounds
        let bounds_list: Vec<BoundDecl<'a>> = self.bounds.values().map(|b| (*b).clone()).collect();
        for bnd in bounds_list {
            let op_name = bnd.name;
            let op = self.operations.get(op_name).ok_or(CheckerError {
                code: DiagnosticCode::W0401,
                kind: CheckerErrorKind::MissingContract(format!("Unknown op {op_name} in bound")),
                span: bnd.span,
            })?;

            let op_params = op.params.clone();
            let op_static_params: Vec<String> = op
                .static_params
                .iter()
                .map(|sp| sp.name.to_string())
                .collect();
            let mut env = HashMap::new();
            for p in &op_params {
                let ty = self.resolve_type(&p.param_type, &op_static_params)?;
                env.insert(p.name.to_string(), ty);
            }

            for row in &bnd.rows {
                if row.name != "steps" && row.name != "heap" && row.name != "depth" {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0401,
                        kind: CheckerErrorKind::InvalidContract(format!(
                            "Invalid bound metric {}, expected steps, heap, or depth",
                            row.name
                        )),
                        span: row.span,
                    });
                }
                let bound_ty =
                    self.typecheck_expression(&row.bound, &env, Some(&Type::Int), false)?;
                if !bound_ty.is_assignable_to(&Type::Int) {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0301,
                        kind: CheckerErrorKind::InvalidBoundExpression,
                        span: row.span,
                    });
                }
            }
        }

        // Inputs
        let inputs_list: Vec<InputDecl<'a>> = self.inputs.values().map(|i| (*i).clone()).collect();
        for inp in inputs_list {
            let target_name = inp.target.segments.join(".");
            let parts: Vec<&str> = target_name.split('.').collect();
            if parts.len() != 2 {
                return Err(CheckerError {
                    code: DiagnosticCode::W0401,
                    kind: CheckerErrorKind::InvalidContract(format!(
                        "Invalid input target {target_name}"
                    )),
                    span: inp.span,
                });
            }
            let (op_name, param_name) = (parts[0], parts[1]);
            let (env, op_return_ty) = {
                let op = self.operations.get(op_name).ok_or(CheckerError {
                    code: DiagnosticCode::W0401,
                    kind: CheckerErrorKind::MissingContract(format!(
                        "Unknown op {op_name} in input"
                    )),
                    span: inp.span,
                })?;

                if !op.params.iter().any(|p| p.name == param_name) {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0401,
                        kind: CheckerErrorKind::MissingContract(format!(
                            "Unknown parameter {param_name} in input for {op_name}"
                        )),
                        span: inp.span,
                    });
                }

                let op_params = op.params.clone();
                let op_ret = op.return_type.clone();
                let op_static_params: Vec<String> = op
                    .static_params
                    .iter()
                    .map(|sp| sp.name.to_string())
                    .collect();
                let mut env = HashMap::new();
                for p in &op_params {
                    let ty = self.resolve_type(&p.param_type, &op_static_params)?;
                    env.insert(p.name.to_string(), ty);
                }
                let ret_ty = self.resolve_type(&op_ret, &op_static_params)?;
                (env, ret_ty)
            };

            // Strict validation of ensures flags
            for flag in &inp.ensures {
                if *flag != "no_panic" && *flag != "terminates" {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0401,
                        kind: CheckerErrorKind::InvalidContract(format!(
                            "Invalid ensures flag '{flag}', expected 'no_panic' or 'terminates'"
                        )),
                        span: inp.span,
                    });
                }
            }

            // Check rejections
            for rej in &inp.rejections {
                let cond_ty =
                    self.typecheck_expression(&rej.condition, &env, Some(&Type::Bool), false)?;
                if cond_ty != Type::Bool {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0301,
                        kind: CheckerErrorKind::TypeMismatch {
                            expected: "Bool".to_string(),
                            actual: cond_ty.display_name(),
                        },
                        span: rej.span,
                    });
                }

                // Verify rejection error variant belongs to op's Result error type
                if rej.error_variant.segments.len() != 2 {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0401,
                        kind: CheckerErrorKind::InvalidContract(
                            "Rejection error must be ErrorType.CASE".to_string(),
                        ),
                        span: rej.span,
                    });
                }
                let err_type_name = rej.error_variant.segments[0];
                let err_case = rej.error_variant.segments[1];

                match &op_return_ty {
                    Type::Result(_, expected_err_name) => {
                        if expected_err_name != err_type_name {
                            return Err(CheckerError {
                                code: DiagnosticCode::W0401,
                                kind: CheckerErrorKind::InvalidContract(format!(
                                    "Rejection error {err_type_name} does not match op return error type {expected_err_name}"
                                )),
                                span: rej.span,
                            });
                        }
                    }
                    _ => {
                        return Err(CheckerError {
                            code: DiagnosticCode::W0401,
                            kind: CheckerErrorKind::InvalidContract(format!(
                                "Op {op_name} does not return Result, cannot declare rejects"
                            )),
                            span: rej.span,
                        });
                    }
                }

                let (variants, _) = self.errors.get(err_type_name).ok_or(CheckerError {
                    code: DiagnosticCode::W0202,
                    kind: CheckerErrorKind::UnboundType(err_type_name.to_string()),
                    span: rej.span,
                })?;

                if !variants.contains(err_case) {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0202,
                        kind: CheckerErrorKind::UnboundConstructor(format!(
                            "{err_type_name}.{err_case}"
                        )),
                        span: rej.span,
                    });
                }
            }
        }

        // Assumptions
        for decl in &self.file.declarations {
            if let Declaration::Assumptions(AssumptionsDecl::Environment { rows, .. }) = decl {
                let env = HashMap::new();
                for row in rows {
                    let ty = self.typecheck_expression(&row.expr, &env, Some(&Type::Bool), true)?;
                    if ty != Type::Bool {
                        return Err(CheckerError {
                            code: DiagnosticCode::W0301,
                            kind: CheckerErrorKind::TypeMismatch {
                                expected: "Bool".to_string(),
                                actual: ty.display_name(),
                            },
                            span: row.span,
                        });
                    }
                }
            }
        }

        Ok(())
    }

    // ── Expression Type Checker ────────────────────────────────────────────

    fn typecheck_expression(
        &mut self,
        expr: &Expression<'a>,
        env: &HashMap<String, Type>,
        expected: Option<&Type>,
        allow_ops: bool,
    ) -> Result<Type, CheckerError> {
        let span = body_span(expr);
        self.charge_work(span)?;

        self.enter_scope(span, |this| match expr {
            Expression::Literal { lit, span } => this.typecheck_literal(lit, expected, *span),
            Expression::Reference {
                name,
                widths,
                args,
                span,
            } => this.typecheck_reference(
                name,
                widths,
                args.as_deref(),
                env,
                expected,
                allow_ops,
                *span,
            ),
            Expression::Grouped { inner, .. } => {
                this.typecheck_expression(inner, env, expected, allow_ops)
            }
            Expression::Unary { op, operand, span } => {
                let op_type = this.typecheck_expression(operand, env, None, allow_ops)?;
                match op {
                    UnaryOp::Neg => {
                        if op_type == Type::Int || op_type == Type::Nat {
                            Ok(Type::Int)
                        } else if op_type.fixed().is_some_and(FixedTy::signed) {
                            // VER04j section 2.3: wrapping negation of a
                            // signed value; an unsigned `-a` stays rejected
                            // (write `0uN - a`).
                            Ok(op_type)
                        } else {
                            Err(CheckerError {
                                code: DiagnosticCode::W0301,
                                kind: CheckerErrorKind::TypeMismatch {
                                    expected: "Int or Nat".to_string(),
                                    actual: op_type.display_name(),
                                },
                                span: *span,
                            })
                        }
                    }
                    UnaryOp::Not => {
                        if op_type == Type::Bool {
                            Ok(Type::Bool)
                        } else {
                            Err(CheckerError {
                                code: DiagnosticCode::W0301,
                                kind: CheckerErrorKind::TypeMismatch {
                                    expected: "Bool".to_string(),
                                    actual: op_type.display_name(),
                                },
                                span: *span,
                            })
                        }
                    }
                }
            }
            Expression::Binary {
                op,
                left,
                right,
                span,
            } => this.typecheck_binary(*op, left, right, env, allow_ops, *span),
            Expression::Conditional {
                condition,
                then_branch,
                else_branch,
                span,
            } => {
                let cond_ty =
                    this.typecheck_expression(condition, env, Some(&Type::Bool), allow_ops)?;
                if cond_ty != Type::Bool {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0301,
                        kind: CheckerErrorKind::TypeMismatch {
                            expected: "Bool".to_string(),
                            actual: cond_ty.display_name(),
                        },
                        span: *span,
                    });
                }
                let then_ty = this.typecheck_expression(then_branch, env, expected, allow_ops)?;
                let else_ty = this.typecheck_expression(else_branch, env, expected, allow_ops)?;

                Type::common_type(&then_ty, &else_ty).ok_or(CheckerError {
                    code: DiagnosticCode::W0301,
                    kind: CheckerErrorKind::IncompatibleBranches {
                        then_type: then_ty.display_name(),
                        else_type: else_ty.display_name(),
                    },
                    span: *span,
                })
            }
            Expression::Match { target, arms, span } => {
                this.typecheck_match(target, arms, env, expected, allow_ops, *span)
            }
            Expression::Quantified {
                is_forall: _,
                binders,
                body,
                span,
            } => {
                let mut local_env = env.clone();
                for b in binders {
                    if local_env.contains_key(b.name) {
                        return Err(CheckerError {
                            code: DiagnosticCode::W0203,
                            kind: CheckerErrorKind::ShadowedBinding(b.name.to_string()),
                            span: b.span,
                        });
                    }
                    this.check_not_ctor(b.name, b.span)?;
                    let b_type = this.resolve_type(&b.param_type, &[])?;
                    if let Some(refinement) = &b.refinement {
                        let mut ref_env = local_env.clone();
                        ref_env.insert(b.name.to_string(), b_type.clone());
                        let ref_ty = this.typecheck_expression(
                            refinement,
                            &ref_env,
                            Some(&Type::Bool),
                            allow_ops,
                        )?;
                        if ref_ty != Type::Bool {
                            return Err(CheckerError {
                                code: DiagnosticCode::W0301,
                                kind: CheckerErrorKind::TypeMismatch {
                                    expected: "Bool".to_string(),
                                    actual: ref_ty.display_name(),
                                },
                                span: b.span,
                            });
                        }
                    }
                    local_env.insert(b.name.to_string(), b_type);
                }
                let body_ty =
                    this.typecheck_expression(body, &local_env, Some(&Type::Bool), allow_ops)?;
                if body_ty != Type::Bool {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0301,
                        kind: CheckerErrorKind::TypeMismatch {
                            expected: "Bool".to_string(),
                            actual: body_ty.display_name(),
                        },
                        span: *span,
                    });
                }
                Ok(Type::Bool)
            }
            Expression::Tuple { elements, .. } => {
                let component_hints: Vec<Option<Type>> = match expected {
                    Some(Type::Tuple(ts)) if ts.len() == elements.len() => {
                        ts.iter().cloned().map(Some).collect()
                    }
                    _ => vec![None; elements.len()],
                };
                let mut types = Vec::with_capacity(elements.len());
                for (el, hint) in elements.iter().zip(&component_hints) {
                    types.push(this.typecheck_expression(el, env, hint.as_ref(), allow_ops)?);
                }
                Ok(Type::Tuple(types))
            }
            Expression::Project { base, key, span } => {
                let base_ty = this.typecheck_expression(base, env, None, allow_ops)?;
                this.project_type(&base_ty, *key, *span)
            }
            Expression::List { elements, span } => {
                if elements.is_empty() {
                    if let Some(Type::List(inner)) = expected {
                        return Ok(Type::List(inner.clone()));
                    }
                    return Err(CheckerError {
                        code: DiagnosticCode::W0301,
                        kind: CheckerErrorKind::TypeMismatch {
                            expected: "Non-empty list or contextual List<T>".to_string(),
                            actual: "Empty list".to_string(),
                        },
                        span: *span,
                    });
                }
                let elem_expected = match expected {
                    Some(Type::List(inner)) => Some(inner.as_ref()),
                    _ => None,
                };
                let first_ty =
                    this.typecheck_expression(&elements[0], env, elem_expected, allow_ops)?;
                let mut current_elem_ty = first_ty;
                for el in &elements[1..] {
                    let el_ty =
                        this.typecheck_expression(el, env, Some(&current_elem_ty), allow_ops)?;
                    current_elem_ty =
                        Type::common_type(&current_elem_ty, &el_ty).ok_or_else(|| {
                            CheckerError {
                                code: DiagnosticCode::W0301,
                                kind: CheckerErrorKind::TypeMismatch {
                                    expected: current_elem_ty.display_name(),
                                    actual: el_ty.display_name(),
                                },
                                span: *span,
                            }
                        })?;
                }
                Ok(Type::List(Box::new(current_elem_ty)))
            }
        })
    }

    /// The type of `base.key` (VER04j sections 4.3 and 4.4): `t.i` of a tuple
    /// with `i` in range, `r.f` of a record with a field `f`.  A variant that is
    /// not a record cannot be projected (use `match`).
    fn project_type(
        &self,
        base: &Type,
        key: ProjKey<'_>,
        span: (usize, usize),
    ) -> Result<Type, CheckerError> {
        let err = |kind: CheckerErrorKind| CheckerError {
            code: DiagnosticCode::W0301,
            kind,
            span,
        };
        match (base, key) {
            (Type::Tuple(ts), ProjKey::Index(i)) => ts.get(i).cloned().ok_or_else(|| {
                err(CheckerErrorKind::NoSuchField(format!(
                    "component {i} of a {}-tuple",
                    ts.len()
                )))
            }),
            (Type::Adt(name), ProjKey::Field(f)) => {
                let adt = self.adts.get(name).ok_or_else(|| {
                    err(CheckerErrorKind::NoSuchField(format!("{name}.{f}")))
                })?;
                if !adt.is_record {
                    return Err(err(CheckerErrorKind::ProjectNonRecord(name.clone())));
                }
                adt.ctors[0]
                    .fields
                    .iter()
                    .find(|(n, _)| n == f)
                    .map(|(_, t)| t.clone())
                    .ok_or_else(|| err(CheckerErrorKind::NoSuchField(format!("{name}.{f}"))))
            }
            (Type::Adt(name), ProjKey::Index(_)) => {
                if self.adts.get(name).is_some_and(|a| !a.is_record) {
                    Err(err(CheckerErrorKind::ProjectNonRecord(name.clone())))
                } else {
                    Err(err(CheckerErrorKind::TypeMismatch {
                        expected: "a tuple".to_string(),
                        actual: base.display_name(),
                    }))
                }
            }
            (Type::Tuple(_), ProjKey::Field(_)) => Err(err(CheckerErrorKind::TypeMismatch {
                expected: "a record".to_string(),
                actual: base.display_name(),
            })),
            _ => Err(err(CheckerErrorKind::TypeMismatch {
                expected: "a tuple or record".to_string(),
                actual: base.display_name(),
            })),
        }
    }

    /// Type a constructor application `C(e1, ..., en)` (or the bare nullary
    /// `C`): positional, each argument checked against its field with the v0
    /// subtyping (`Nat <= Int`, `Bytes<N> <= Bytes`) applied per argument.
    #[allow(clippy::too_many_arguments)]
    fn typecheck_constructor(
        &mut self,
        name: &str,
        widths: &[Width<'a>],
        args: Option<&[Expression<'a>]>,
        env: &HashMap<String, Type>,
        allow_ops: bool,
        span: (usize, usize),
    ) -> Result<Type, CheckerError> {
        let (ty_name, idx) = self.ctor_index[name].clone();
        let fields: Vec<Type> = self.adts[&ty_name].ctors[idx]
            .fields
            .iter()
            .map(|(_, t)| t.clone())
            .collect();
        let arity_err = |actual: usize| CheckerError {
            code: DiagnosticCode::W0301,
            kind: CheckerErrorKind::ArityMismatch {
                expected: fields.len(),
                actual,
            },
            span,
        };
        if !widths.is_empty() {
            return Err(arity_err(widths.len()));
        }
        let list: &[Expression<'a>] = match args {
            None if fields.is_empty() => &[],
            None => return Err(arity_err(0)),
            Some(l) if fields.is_empty() => {
                let _ = l;
                return Err(CheckerError {
                    code: DiagnosticCode::W0301,
                    kind: CheckerErrorKind::NullaryWithParens(name.to_string()),
                    span,
                });
            }
            Some(l) => l,
        };
        if list.len() != fields.len() {
            return Err(arity_err(list.len()));
        }
        for (arg, want) in list.iter().zip(&fields) {
            let got = self.typecheck_expression(arg, env, Some(want), allow_ops)?;
            if !got.is_assignable_to(want) {
                return Err(CheckerError {
                    code: DiagnosticCode::W0301,
                    kind: CheckerErrorKind::TypeMismatch {
                        expected: want.display_name(),
                        actual: got.display_name(),
                    },
                    span,
                });
            }
        }
        Ok(Type::Adt(ty_name))
    }

    fn typecheck_literal(
        &mut self,
        lit: &Literal<'a>,
        expected: Option<&Type>,
        _span: (usize, usize),
    ) -> Result<Type, CheckerError> {
        match lit {
            Literal::Bool(_) => Ok(Type::Bool),
            Literal::U8(_) => Ok(Type::U8),
            Literal::U64(_) => Ok(Type::U64),
            Literal::Fixed(ty, _) => Ok(Type::from_fixed(*ty)),
            Literal::Int(_) => {
                if let Some(Type::Nat) = expected {
                    Ok(Type::Nat)
                } else {
                    Ok(Type::Int)
                }
            }
            Literal::Bytes(hex) => {
                let byte_len = (hex.len() / 2) as u64;
                if let Some(Type::FixedBytes(n)) = expected {
                    if byte_len == *n {
                        return Ok(Type::FixedBytes(*n));
                    }
                }
                Ok(Type::FixedBytes(byte_len))
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn typecheck_reference(
        &mut self,
        name: &QualifiedName<'a>,
        widths: &[Width<'a>],
        args: Option<&[Expression<'a>]>,
        env: &HashMap<String, Type>,
        expected: Option<&Type>,
        allow_ops: bool,
        span: (usize, usize),
    ) -> Result<Type, CheckerError> {
        // WX-A: `x.f` with a binder head is projection (VER04j section 4.3);
        // it wins over an error constructor of the same spelling.
        if self.file.version == Version::V1 && name.segments.len() >= 2 {
            if let Some(head) = env.get(name.segments[0]) {
                if args.is_some() || !widths.is_empty() {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0301,
                        kind: CheckerErrorKind::ArityMismatch {
                            expected: 0,
                            actual: args.map_or(widths.len(), <[_]>::len),
                        },
                        span,
                    });
                }
                let mut ty = head.clone();
                for seg in &name.segments[1..] {
                    ty = self.project_type(&ty, ProjKey::Field(seg), span)?;
                }
                return Ok(ty);
            }
        }

        // Multi-segment name: could be ErrorName.VARIANT
        if name.segments.len() == 2 {
            let (err_name, variant) = (name.segments[0], name.segments[1]);
            if let Some((variants, _)) = self.errors.get(err_name) {
                if variants.contains(variant) {
                    return Ok(Type::ErrorVariant(format!("{err_name}.{variant}")));
                } else {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0202,
                        kind: CheckerErrorKind::UnboundConstructor(format!("{err_name}.{variant}")),
                        span,
                    });
                }
            }
        }

        let first = name.segments[0];

        // 1. Check local environment binders
        if let Some(ty) = env.get(first) {
            if args.is_none() && widths.is_empty() {
                return Ok(ty.clone());
            }
        }

        // 1b. WX-A constructors (a name is a constructor only if declared).
        if name.segments.len() == 1 && self.ctor_index.contains_key(first) {
            return self.typecheck_constructor(first, widths, args, env, allow_ops, span);
        }

        // 2a. v1 built-ins (WX-B/WX-C), in scope only in a v1 document.
        if self.file.version == Version::V1
            && name.segments.len() == 1
            && (V1_FUNCTIONS.contains(&first) || first == "div")
        {
            return self.typecheck_v1_builtin(first, widths, args, env, allow_ops, span);
        }

        // 2. Built-in constructors and functions
        match first {
            "Some" => {
                let args = args.ok_or(CheckerError {
                    code: DiagnosticCode::W0301,
                    kind: CheckerErrorKind::ArityMismatch {
                        expected: 1,
                        actual: 0,
                    },
                    span,
                })?;
                if args.len() != 1 {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0301,
                        kind: CheckerErrorKind::ArityMismatch {
                            expected: 1,
                            actual: args.len(),
                        },
                        span,
                    });
                }
                let inner_expected = match expected {
                    Some(Type::Option(inner)) => Some(inner.as_ref()),
                    _ => None,
                };
                let inner_ty =
                    self.typecheck_expression(&args[0], env, inner_expected, allow_ops)?;
                return Ok(Type::Option(Box::new(inner_ty)));
            }
            "None" => {
                if let Some(Type::Option(inner)) = expected {
                    return Ok(Type::Option(inner.clone()));
                }
                return Err(CheckerError {
                    code: DiagnosticCode::W0301,
                    kind: CheckerErrorKind::TypeMismatch {
                        expected: "Contextual Option<T>".to_string(),
                        actual: "None".to_string(),
                    },
                    span,
                });
            }
            "Ok" => {
                let args = args.ok_or(CheckerError {
                    code: DiagnosticCode::W0301,
                    kind: CheckerErrorKind::ArityMismatch {
                        expected: 1,
                        actual: 0,
                    },
                    span,
                })?;
                if args.len() != 1 {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0301,
                        kind: CheckerErrorKind::ArityMismatch {
                            expected: 1,
                            actual: args.len(),
                        },
                        span,
                    });
                }
                let (inner_expected, err_name) = match expected {
                    Some(Type::Result(inner, err)) => (Some(inner.as_ref()), err.clone()),
                    _ => (None, "UnknownError".to_string()),
                };
                let inner_ty =
                    self.typecheck_expression(&args[0], env, inner_expected, allow_ops)?;
                return Ok(Type::Result(Box::new(inner_ty), err_name));
            }
            "Err" => {
                let args = args.ok_or(CheckerError {
                    code: DiagnosticCode::W0301,
                    kind: CheckerErrorKind::ArityMismatch {
                        expected: 1,
                        actual: 0,
                    },
                    span,
                })?;
                if args.len() != 1 {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0301,
                        kind: CheckerErrorKind::ArityMismatch {
                            expected: 1,
                            actual: args.len(),
                        },
                        span,
                    });
                }
                let arg_ty = self.typecheck_expression(&args[0], env, None, allow_ops)?;
                if let Type::ErrorVariant(ev) = arg_ty {
                    let err_type = ev.split('.').next().unwrap_or(&ev).to_string();
                    let ok_type = match expected {
                        Some(Type::Result(ok, _)) => ok.clone(),
                        _ => Box::new(Type::Bool),
                    };
                    return Ok(Type::Result(ok_type, err_type));
                } else {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0301,
                        kind: CheckerErrorKind::TypeMismatch {
                            expected: "ErrorVariant".to_string(),
                            actual: arg_ty.display_name(),
                        },
                        span,
                    });
                }
            }
            "len" => {
                let args = args.ok_or(CheckerError {
                    code: DiagnosticCode::W0301,
                    kind: CheckerErrorKind::ArityMismatch {
                        expected: 1,
                        actual: 0,
                    },
                    span,
                })?;
                if args.len() != 1 {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0301,
                        kind: CheckerErrorKind::ArityMismatch {
                            expected: 1,
                            actual: args.len(),
                        },
                        span,
                    });
                }
                let arg_ty = self.typecheck_expression(&args[0], env, None, allow_ops)?;
                match arg_ty {
                    Type::Bytes
                    | Type::FixedBytes(_)
                    | Type::FixedBytesParam(_)
                    | Type::List(_) => {
                        return Ok(Type::Nat);
                    }
                    _ => {
                        return Err(CheckerError {
                            code: DiagnosticCode::W0301,
                            kind: CheckerErrorKind::TypeMismatch {
                                expected: "Bytes or List<T>".to_string(),
                                actual: arg_ty.display_name(),
                            },
                            span,
                        });
                    }
                }
            }
            "at" => {
                let args = args.ok_or(CheckerError {
                    code: DiagnosticCode::W0301,
                    kind: CheckerErrorKind::ArityMismatch {
                        expected: 2,
                        actual: 0,
                    },
                    span,
                })?;
                if args.len() != 2 {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0301,
                        kind: CheckerErrorKind::ArityMismatch {
                            expected: 2,
                            actual: args.len(),
                        },
                        span,
                    });
                }
                let seq_ty = self.typecheck_expression(&args[0], env, None, allow_ops)?;
                let idx_ty =
                    self.typecheck_expression(&args[1], env, Some(&Type::Nat), allow_ops)?;
                if !idx_ty.is_assignable_to(&Type::Nat) && idx_ty != Type::Int {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0301,
                        kind: CheckerErrorKind::TypeMismatch {
                            expected: "Nat or Int".to_string(),
                            actual: idx_ty.display_name(),
                        },
                        span,
                    });
                }
                match seq_ty {
                    Type::List(inner) => return Ok(Type::Option(inner)),
                    Type::Bytes | Type::FixedBytes(_) | Type::FixedBytesParam(_) => {
                        return Ok(Type::Option(Box::new(Type::U8)));
                    }
                    _ => {
                        return Err(CheckerError {
                            code: DiagnosticCode::W0301,
                            kind: CheckerErrorKind::TypeMismatch {
                                expected: "Sequence (Bytes or List<T>)".to_string(),
                                actual: seq_ty.display_name(),
                            },
                            span,
                        });
                    }
                }
            }
            "concat" => {
                let args = args.ok_or(CheckerError {
                    code: DiagnosticCode::W0301,
                    kind: CheckerErrorKind::ArityMismatch {
                        expected: 2,
                        actual: 0,
                    },
                    span,
                })?;
                if args.len() != 2 {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0301,
                        kind: CheckerErrorKind::ArityMismatch {
                            expected: 2,
                            actual: args.len(),
                        },
                        span,
                    });
                }
                let a_ty = self.typecheck_expression(&args[0], env, None, allow_ops)?;
                let b_ty = self.typecheck_expression(&args[1], env, None, allow_ops)?;
                match (&a_ty, &b_ty) {
                    (Type::FixedBytes(m), Type::FixedBytes(n)) => {
                        let sum = m.checked_add(*n).ok_or(CheckerError {
                            code: DiagnosticCode::W0301,
                            kind: CheckerErrorKind::ArithmeticOverflow(
                                "concat FixedBytes width overflow".to_string(),
                            ),
                            span,
                        })?;
                        return Ok(Type::FixedBytes(sum));
                    }
                    (Type::Bytes, Type::Bytes)
                    | (Type::FixedBytes(_), Type::Bytes)
                    | (Type::Bytes, Type::FixedBytes(_))
                    | (Type::FixedBytesParam(_), Type::Bytes)
                    | (Type::Bytes, Type::FixedBytesParam(_))
                    | (Type::FixedBytesParam(_), Type::FixedBytes(_))
                    | (Type::FixedBytes(_), Type::FixedBytesParam(_))
                    | (Type::FixedBytesParam(_), Type::FixedBytesParam(_)) => {
                        return Ok(Type::Bytes);
                    }
                    (Type::List(ia), Type::List(ib)) => {
                        let common = Type::common_type(ia, ib).ok_or(CheckerError {
                            code: DiagnosticCode::W0301,
                            kind: CheckerErrorKind::TypeMismatch {
                                expected: ia.display_name(),
                                actual: ib.display_name(),
                            },
                            span,
                        })?;
                        return Ok(Type::List(Box::new(common)));
                    }
                    _ => {
                        return Err(CheckerError {
                            code: DiagnosticCode::W0301,
                            kind: CheckerErrorKind::TypeMismatch {
                                expected: "Matching sequences".to_string(),
                                actual: format!("{}, {}", a_ty.display_name(), b_ty.display_name()),
                            },
                            span,
                        });
                    }
                }
            }
            "fixed" => {
                if widths.len() != 1 {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0301,
                        kind: CheckerErrorKind::ArityMismatch {
                            expected: 1,
                            actual: widths.len(),
                        },
                        span,
                    });
                }
                let args = args.ok_or(CheckerError {
                    code: DiagnosticCode::W0301,
                    kind: CheckerErrorKind::ArityMismatch {
                        expected: 1,
                        actual: 0,
                    },
                    span,
                })?;
                if args.len() != 1 {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0301,
                        kind: CheckerErrorKind::ArityMismatch {
                            expected: 1,
                            actual: args.len(),
                        },
                        span,
                    });
                }
                let b_ty = self.typecheck_expression(&args[0], env, None, allow_ops)?;
                if !b_ty.is_assignable_to(&Type::Bytes) {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0301,
                        kind: CheckerErrorKind::TypeMismatch {
                            expected: "Bytes".to_string(),
                            actual: b_ty.display_name(),
                        },
                        span,
                    });
                }
                let width_type = match &widths[0] {
                    Width::Literal(n) => Type::FixedBytes(*n),
                    Width::Named(name) => Type::FixedBytesParam(name.to_string()),
                };
                return Ok(Type::Option(Box::new(width_type)));
            }
            "div" => {
                let args = args.ok_or(CheckerError {
                    code: DiagnosticCode::W0301,
                    kind: CheckerErrorKind::ArityMismatch {
                        expected: 2,
                        actual: 0,
                    },
                    span,
                })?;
                if args.len() != 2 {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0301,
                        kind: CheckerErrorKind::ArityMismatch {
                            expected: 2,
                            actual: args.len(),
                        },
                        span,
                    });
                }
                let a_ty = self.typecheck_expression(&args[0], env, None, allow_ops)?;
                let b_ty = self.typecheck_expression(&args[1], env, None, allow_ops)?;
                if !a_ty.is_assignable_to(&Type::Int) || !b_ty.is_assignable_to(&Type::Int) {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0301,
                        kind: CheckerErrorKind::TypeMismatch {
                            expected: "Int or Nat".to_string(),
                            actual: format!("{}, {}", a_ty.display_name(), b_ty.display_name()),
                        },
                        span,
                    });
                }
                return Ok(Type::Option(Box::new(Type::Int)));
            }
            _ => {}
        }

        // 3. Model functions
        if let Some(model) = self.models.get(first).copied() {
            let actual_args = args.ok_or(CheckerError {
                code: DiagnosticCode::W0301,
                kind: CheckerErrorKind::ArityMismatch {
                    expected: model.params.len(),
                    actual: 0,
                },
                span,
            })?;
            if actual_args.len() != model.params.len() {
                return Err(CheckerError {
                    code: DiagnosticCode::W0301,
                    kind: CheckerErrorKind::ArityMismatch {
                        expected: model.params.len(),
                        actual: actual_args.len(),
                    },
                    span,
                });
            }
            let model_params = model.params.clone();
            let model_target_type = model.target_type.clone();
            for (arg_expr, param) in actual_args.iter().zip(&model_params) {
                let expected_param_ty = self.resolve_type(&param.param_type, &[])?;
                let actual_ty =
                    self.typecheck_expression(arg_expr, env, Some(&expected_param_ty), allow_ops)?;
                if !actual_ty.is_assignable_to(&expected_param_ty) {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0301,
                        kind: CheckerErrorKind::TypeMismatch {
                            expected: expected_param_ty.display_name(),
                            actual: actual_ty.display_name(),
                        },
                        span,
                    });
                }
            }
            return self.resolve_type(&model_target_type, &[]);
        }

        // 4. Public operations
        if let Some(op) = self.operations.get(first).copied() {
            if !allow_ops {
                return Err(CheckerError {
                    code: DiagnosticCode::W0301,
                    kind: CheckerErrorKind::ModelCallsOperation(first.to_string()),
                    span,
                });
            }
            let actual_args = args.ok_or(CheckerError {
                code: DiagnosticCode::W0301,
                kind: CheckerErrorKind::ArityMismatch {
                    expected: op.params.len(),
                    actual: 0,
                },
                span,
            })?;
            if actual_args.len() != op.params.len() {
                return Err(CheckerError {
                    code: DiagnosticCode::W0301,
                    kind: CheckerErrorKind::ArityMismatch {
                        expected: op.params.len(),
                        actual: actual_args.len(),
                    },
                    span,
                });
            }

            // Map static parameters
            let mut static_map: HashMap<String, u64> = HashMap::new();
            if widths.len() != op.static_params.len() {
                return Err(CheckerError {
                    code: DiagnosticCode::W0301,
                    kind: CheckerErrorKind::ArityMismatch {
                        expected: op.static_params.len(),
                        actual: widths.len(),
                    },
                    span,
                });
            }
            let op_static_params = op.static_params.clone();
            for (w, sp) in widths.iter().zip(&op_static_params) {
                match w {
                    Width::Literal(n) => {
                        static_map.insert(sp.name.to_string(), *n);
                    }
                    Width::Named(name) => {
                        return Err(CheckerError {
                            code: DiagnosticCode::W0202,
                            kind: CheckerErrorKind::UnboundName((*name).to_string()),
                            span,
                        });
                    }
                }
            }

            let op_params = op.params.clone();
            let op_ret = op.return_type.clone();
            for (arg_expr, param) in actual_args.iter().zip(&op_params) {
                let resolved_param_ty = self.instantiate_type(&param.param_type, &static_map)?;
                let actual_ty =
                    self.typecheck_expression(arg_expr, env, Some(&resolved_param_ty), allow_ops)?;
                if !actual_ty.is_assignable_to(&resolved_param_ty) {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0301,
                        kind: CheckerErrorKind::TypeMismatch {
                            expected: resolved_param_ty.display_name(),
                            actual: actual_ty.display_name(),
                        },
                        span,
                    });
                }
            }

            return self.instantiate_type(&op_ret, &static_map);
        }

        Err(CheckerError {
            code: DiagnosticCode::W0202,
            kind: CheckerErrorKind::UnboundName(first.to_string()),
            span,
        })
    }

    // ── WX-B / WX-C built-ins (VER04j sections 2.3, 3.2, 3.3) ──────────────

    /// Type a call to a v1 built-in function.  Every rule is a row of the
    /// specification tables; an operand that does not fit is W0301 and the
    /// result type is the table's `Result` column.
    ///
    /// ```text
    /// widen<T>(x)            x: F, range(x) within range(T)   -> T
    /// wrap<T>(x)             x: F | Int | Nat                 -> T
    /// try<T>(x)              x: F | Int | Nat                 -> Option<T>
    /// to_int(x) / to_nat(x)  x: F  /  x: unsigned F           -> Int / Nat
    /// max_value<T>()         (no operands)                    -> T
    /// checked_add(a, b)      T, T                             -> Option<T>
    /// div(a, b)  rem(a, b)   unsigned T, T                    -> Option<T>
    ///                        Int|Nat, Int|Nat                 -> Option<Int>
    ///                        signed: rejected (floor vs trunc is a trap)
    /// div_trunc rem_trunc    signed T, T                      -> Option<T>
    /// unwrap_or(o, d)        Option<T>, T                     -> T
    /// bit_and(a, b) ...      T, T                             -> T
    /// shl(a, n) shr rotl ... T, Nat                           -> T
    /// sar(a, n)              signed T, Nat                    -> T
    /// popcount(a) clz ctz    T                                -> Nat
    /// bit(a, i)              T, Nat                           -> Bool
    /// be_bytes(a) le_bytes   T                                -> Bytes<w/8>
    /// from_be<T>(b)          Bytes<w/8>                       -> T
    /// ```
    fn typecheck_v1_builtin(
        &mut self,
        name: &str,
        widths: &[Width<'a>],
        args: Option<&[Expression<'a>]>,
        env: &HashMap<String, Type>,
        allow_ops: bool,
        span: (usize, usize),
    ) -> Result<Type, CheckerError> {
        let mismatch = |expected: &str, actual: String| CheckerError {
            code: DiagnosticCode::W0301,
            kind: CheckerErrorKind::TypeMismatch {
                expected: expected.to_string(),
                actual,
            },
            span,
        };
        // The static type argument of `widen<T>`, `max_value<T>`, ...
        let static_ty = |widths: &[Width<'a>]| -> Result<FixedTy, CheckerError> {
            match widths {
                [Width::Named(n)] => FixedTy::from_name(n)
                    .ok_or_else(|| mismatch("fixed-width type argument", (*n).to_string())),
                [Width::Literal(n)] => {
                    Err(mismatch("fixed-width type argument", format!("{n}")))
                }
                other => Err(CheckerError {
                    code: DiagnosticCode::W0301,
                    kind: CheckerErrorKind::ArityMismatch {
                        expected: 1,
                        actual: other.len(),
                    },
                    span,
                }),
            }
        };
        let wants_static = matches!(
            name,
            "widen" | "wrap" | "try" | "max_value" | "min_value" | "from_be" | "from_le"
        );
        if !wants_static && !widths.is_empty() {
            return Err(CheckerError {
                code: DiagnosticCode::W0301,
                kind: CheckerErrorKind::ArityMismatch {
                    expected: 0,
                    actual: widths.len(),
                },
                span,
            });
        }
        let arity = match name {
            "max_value" | "min_value" => 0,
            "widen" | "wrap" | "try" | "to_int" | "to_nat" | "bit_not" | "popcount" | "clz"
            | "ctz" | "be_bytes" | "le_bytes" | "from_be" | "from_le" => 1,
            _ => 2,
        };
        let list = args.ok_or(CheckerError {
            code: DiagnosticCode::W0301,
            kind: CheckerErrorKind::ArityMismatch {
                expected: arity,
                actual: 0,
            },
            span,
        })?;
        if list.len() != arity {
            return Err(CheckerError {
                code: DiagnosticCode::W0301,
                kind: CheckerErrorKind::ArityMismatch {
                    expected: arity,
                    actual: list.len(),
                },
                span,
            });
        }

        match name {
            "max_value" | "min_value" => Ok(Type::from_fixed(static_ty(widths)?)),
            "widen" | "wrap" | "try" => {
                let target = static_ty(widths)?;
                let x = self.typecheck_expression(&list[0], env, None, allow_ops)?;
                let ok_operand = match x.fixed() {
                    Some(src) => name != "widen" || src.widens_to(target),
                    None => name != "widen" && x.is_assignable_to(&Type::Int),
                };
                if !ok_operand {
                    return Err(mismatch(
                        if name == "widen" {
                            "fixed-width operand whose range is contained in the target"
                        } else {
                            "fixed-width, Int or Nat operand"
                        },
                        x.display_name(),
                    ));
                }
                let t = Type::from_fixed(target);
                Ok(if name == "try" {
                    Type::Option(Box::new(t))
                } else {
                    t
                })
            }
            "to_int" | "to_nat" => {
                let x = self.typecheck_expression(&list[0], env, None, allow_ops)?;
                match x.fixed() {
                    Some(f) if name == "to_int" || !f.signed() => {
                        Ok(if name == "to_int" { Type::Int } else { Type::Nat })
                    }
                    _ => Err(mismatch(
                        if name == "to_int" {
                            "fixed-width operand"
                        } else {
                            "unsigned fixed-width operand"
                        },
                        x.display_name(),
                    )),
                }
            }
            "from_be" | "from_le" => {
                let target = static_ty(widths)?;
                let b = self.typecheck_expression(&list[0], env, None, allow_ops)?;
                match b {
                    // A dynamic Bytes is rejected on purpose: there is no
                    // implicit truncation or padding (use `fixed<N>` first).
                    Type::FixedBytes(n) if n == target.bytes() => Ok(Type::from_fixed(target)),
                    other => Err(mismatch(
                        &format!("Bytes<{}>", target.bytes()),
                        other.display_name(),
                    )),
                }
            }
            "unwrap_or" => {
                let o = self.typecheck_expression(&list[0], env, None, allow_ops)?;
                let Type::Option(inner) = o else {
                    return Err(mismatch("Option<T>", o.display_name()));
                };
                let d = self.typecheck_expression(&list[1], env, Some(&inner), allow_ops)?;
                if !d.is_assignable_to(&inner) {
                    return Err(mismatch(&inner.display_name(), d.display_name()));
                }
                Ok(*inner)
            }
            "div" | "rem" => {
                let a = self.typecheck_expression(&list[0], env, None, allow_ops)?;
                let b = self.typecheck_expression(&list[1], env, Some(&a), allow_ops)?;
                match (a.fixed(), b.fixed()) {
                    (Some(f), Some(g)) if f == g && !f.signed() => {
                        Ok(Type::Option(Box::new(a)))
                    }
                    (Some(f), Some(g)) if f == g => Err(mismatch(
                        "unsigned fixed-width operands (a signed model must say div_trunc/rem_trunc)",
                        a.display_name(),
                    )),
                    (None, None)
                        if a.is_assignable_to(&Type::Int) && b.is_assignable_to(&Type::Int) =>
                    {
                        Ok(Type::Option(Box::new(Type::Int)))
                    }
                    _ => Err(mismatch(
                        &format!("matching operands, got {}", a.display_name()),
                        b.display_name(),
                    )),
                }
            }
            "div_trunc" | "rem_trunc" | "checked_add" | "checked_sub" | "checked_mul"
            | "bit_and" | "bit_or" | "bit_xor" => {
                let a = self.typecheck_expression(&list[0], env, None, allow_ops)?;
                let b = self.typecheck_expression(&list[1], env, Some(&a), allow_ops)?;
                let f = a
                    .fixed()
                    .ok_or_else(|| mismatch("fixed-width operand", a.display_name()))?;
                if a != b {
                    return Err(mismatch(&a.display_name(), b.display_name()));
                }
                if name.ends_with("_trunc") && !f.signed() {
                    return Err(mismatch("signed fixed-width operands", a.display_name()));
                }
                Ok(if name.starts_with("bit_") {
                    a
                } else {
                    Type::Option(Box::new(a))
                })
            }
            "bit_not" | "popcount" | "clz" | "ctz" | "be_bytes" | "le_bytes" => {
                let a = self.typecheck_expression(&list[0], env, None, allow_ops)?;
                let f = a
                    .fixed()
                    .ok_or_else(|| mismatch("fixed-width operand", a.display_name()))?;
                Ok(match name {
                    "bit_not" => a,
                    "popcount" | "clz" | "ctz" => Type::Nat,
                    _ => Type::FixedBytes(f.bytes()),
                })
            }
            // shl shr sar rotl rotr bit: the count is a mathematical Nat.
            _ => {
                let a = self.typecheck_expression(&list[0], env, None, allow_ops)?;
                let n = self.typecheck_expression(&list[1], env, Some(&Type::Nat), allow_ops)?;
                let f = a
                    .fixed()
                    .ok_or_else(|| mismatch("fixed-width operand", a.display_name()))?;
                if n != Type::Nat {
                    return Err(mismatch("Nat shift count or bit index", n.display_name()));
                }
                if name == "sar" && !f.signed() {
                    return Err(mismatch(
                        "signed operand (a logical shift is spelled shr)",
                        a.display_name(),
                    ));
                }
                Ok(if name == "bit" { Type::Bool } else { a })
            }
        }
    }

    fn instantiate_type(
        &mut self,
        ty: &TypeExpr<'a>,
        static_map: &HashMap<String, u64>,
    ) -> Result<Type, CheckerError> {
        match ty {
            TypeExpr::Bytes {
                width: Some(Width::Named(name)),
                span,
            } => {
                if let Some(n) = static_map.get(*name) {
                    Ok(Type::FixedBytes(*n))
                } else {
                    Err(CheckerError {
                        code: DiagnosticCode::W0202,
                        kind: CheckerErrorKind::UnboundName(name.to_string()),
                        span: *span,
                    })
                }
            }
            TypeExpr::Tuple { elems, .. } => {
                let mut ts = Vec::with_capacity(elems.len());
                for e in elems {
                    ts.push(self.instantiate_type(e, static_map)?);
                }
                Ok(Type::Tuple(ts))
            }
            TypeExpr::List { inner, .. } => {
                let inner_ty = self.instantiate_type(inner, static_map)?;
                Ok(Type::List(Box::new(inner_ty)))
            }
            TypeExpr::Option { inner, .. } => {
                let inner_ty = self.instantiate_type(inner, static_map)?;
                Ok(Type::Option(Box::new(inner_ty)))
            }
            TypeExpr::Result {
                ok_type,
                err_name,
                span,
            } => {
                let ok_ty = self.instantiate_type(ok_type, static_map)?;
                if !self.errors.contains_key(*err_name) {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0202,
                        kind: CheckerErrorKind::UnboundType((*err_name).to_string()),
                        span: *span,
                    });
                }
                Ok(Type::Result(Box::new(ok_ty), (*err_name).to_string()))
            }
            _ => self.resolve_type(ty, &[]),
        }
    }

    fn typecheck_binary(
        &mut self,
        op: BinaryOp,
        left: &Expression<'a>,
        right: &Expression<'a>,
        env: &HashMap<String, Type>,
        allow_ops: bool,
        span: (usize, usize),
    ) -> Result<Type, CheckerError> {
        match op {
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul => {
                let l_ty = self.typecheck_expression(left, env, None, allow_ops)?;
                let r_ty = self.typecheck_expression(right, env, Some(&l_ty), allow_ops)?;

                if l_ty.is_assignable_to(&Type::Int) && r_ty.is_assignable_to(&Type::Int) {
                    Ok(Type::Int)
                } else if l_ty == Type::U8 && r_ty == Type::U8 {
                    Ok(Type::U8)
                } else if l_ty == Type::U64 && r_ty == Type::U64 {
                    Ok(Type::U64)
                } else if l_ty.fixed().is_some() && l_ty == r_ty {
                    // WX-B: same fixed-width type on both sides, wrapping.
                    Ok(l_ty)
                } else {
                    Err(CheckerError {
                        code: DiagnosticCode::W0301,
                        kind: CheckerErrorKind::TypeMismatch {
                            expected: format!(
                                "Matching numeric types (Int, U8, U64), got {}",
                                l_ty.display_name()
                            ),
                            actual: r_ty.display_name(),
                        },
                        span,
                    })
                }
            }
            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
                let l_ty = self.typecheck_expression(left, env, None, allow_ops)?;
                let r_ty = self.typecheck_expression(right, env, Some(&l_ty), allow_ops)?;

                if (l_ty.is_assignable_to(&Type::Int) && r_ty.is_assignable_to(&Type::Int))
                    || (l_ty == Type::U8 && r_ty == Type::U8)
                    || (l_ty == Type::U64 && r_ty == Type::U64)
                    || (l_ty.fixed().is_some() && l_ty == r_ty)
                {
                    Ok(Type::Bool)
                } else {
                    Err(CheckerError {
                        code: DiagnosticCode::W0301,
                        kind: CheckerErrorKind::TypeMismatch {
                            expected: format!("Comparable types, got {}", l_ty.display_name()),
                            actual: r_ty.display_name(),
                        },
                        span,
                    })
                }
            }
            BinaryOp::Eq | BinaryOp::NotEq => {
                let l_ty = self.typecheck_expression(left, env, None, allow_ops)?;
                let r_ty = self.typecheck_expression(right, env, Some(&l_ty), allow_ops)?;

                if Type::common_type(&l_ty, &r_ty).is_some() {
                    Ok(Type::Bool)
                } else {
                    Err(CheckerError {
                        code: DiagnosticCode::W0301,
                        kind: CheckerErrorKind::TypeMismatch {
                            expected: format!("Comparable type with {}", l_ty.display_name()),
                            actual: r_ty.display_name(),
                        },
                        span,
                    })
                }
            }
            BinaryOp::And | BinaryOp::Or | BinaryOp::Implies => {
                let l_ty = self.typecheck_expression(left, env, Some(&Type::Bool), allow_ops)?;
                let r_ty = self.typecheck_expression(right, env, Some(&Type::Bool), allow_ops)?;

                if l_ty == Type::Bool && r_ty == Type::Bool {
                    Ok(Type::Bool)
                } else {
                    Err(CheckerError {
                        code: DiagnosticCode::W0301,
                        kind: CheckerErrorKind::TypeMismatch {
                            expected: "Bool".to_string(),
                            actual: format!("{}, {}", l_ty.display_name(), r_ty.display_name()),
                        },
                        span,
                    })
                }
            }
        }
    }

    fn check_arm_binder(
        &self,
        binder: &str,
        span: (usize, usize),
        env: &HashMap<String, Type>,
    ) -> Result<(), CheckerError> {
        if env.contains_key(binder)
            || self.models.contains_key(binder)
            || self.operations.contains_key(binder)
            || self.errors.contains_key(binder)
            || self.ctor_index.contains_key(binder)
            || is_reserved_builtin(self.file.version, binder)
        {
            return Err(CheckerError {
                code: DiagnosticCode::W0203,
                kind: CheckerErrorKind::ShadowedBinding(binder.to_string()),
                span,
            });
        }
        Ok(())
    }

    fn typecheck_match(
        &mut self,
        target: &Expression<'a>,
        arms: &[MatchArm<'a>],
        env: &HashMap<String, Type>,
        expected: Option<&Type>,
        allow_ops: bool,
        span: (usize, usize),
    ) -> Result<Type, CheckerError> {
        if self.file.version == Version::V1 {
            return self.typecheck_match_v1(target, arms, env, expected, allow_ops, span);
        }
        let target_ty = self.typecheck_expression(target, env, None, allow_ops)?;

        match target_ty {
            Type::Option(inner) => {
                let mut has_none = false;
                let mut has_some = false;
                let mut arm_types = Vec::new();

                for arm in arms {
                    match &arm.pattern {
                        Pattern::None => {
                            has_none = true;
                            let arm_ty =
                                self.typecheck_expression(&arm.body, env, expected, allow_ops)?;
                            arm_types.push(arm_ty);
                        }
                        Pattern::Some(pinner) => {
                            let binder = pinner.as_binder().unwrap_or("_");
                            has_some = true;
                            self.check_arm_binder(binder, arm.span, env)?;
                            let mut local_env = env.clone();
                            local_env.insert(binder.to_string(), *inner.clone());
                            let arm_ty = self
                                .typecheck_expression(&arm.body, &local_env, expected, allow_ops)?;
                            arm_types.push(arm_ty);
                        }
                        _ => {
                            return Err(CheckerError {
                                code: DiagnosticCode::W0301,
                                kind: CheckerErrorKind::IncompatibleMatchArms,
                                span: arm.span,
                            });
                        }
                    }
                }

                if !has_none || !has_some {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0301,
                        kind: CheckerErrorKind::NonExhaustiveMatch,
                        span,
                    });
                }

                let mut unified = arm_types[0].clone();
                for at in &arm_types[1..] {
                    unified = Type::common_type(&unified, at).ok_or(CheckerError {
                        code: DiagnosticCode::W0301,
                        kind: CheckerErrorKind::IncompatibleMatchArms,
                        span,
                    })?;
                }
                Ok(unified)
            }
            Type::Result(ok_inner, err_type_name) => {
                let mut has_ok = false;
                let mut has_err = false;
                let mut arm_types = Vec::new();

                for arm in arms {
                    match &arm.pattern {
                        Pattern::Ok(pinner) => {
                            let binder = pinner.as_binder().unwrap_or("_");
                            has_ok = true;
                            self.check_arm_binder(binder, arm.span, env)?;
                            let mut local_env = env.clone();
                            local_env.insert(binder.to_string(), *ok_inner.clone());
                            let arm_ty = self
                                .typecheck_expression(&arm.body, &local_env, expected, allow_ops)?;
                            arm_types.push(arm_ty);
                        }
                        Pattern::Err(pinner) => {
                            let binder = pinner.as_binder().unwrap_or("_");
                            has_err = true;
                            self.check_arm_binder(binder, arm.span, env)?;
                            let mut local_env = env.clone();
                            local_env.insert(
                                binder.to_string(),
                                Type::ErrorVariant(err_type_name.clone()),
                            );
                            let arm_ty = self
                                .typecheck_expression(&arm.body, &local_env, expected, allow_ops)?;
                            arm_types.push(arm_ty);
                        }
                        _ => {
                            return Err(CheckerError {
                                code: DiagnosticCode::W0301,
                                kind: CheckerErrorKind::IncompatibleMatchArms,
                                span: arm.span,
                            });
                        }
                    }
                }

                if !has_ok || !has_err {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0301,
                        kind: CheckerErrorKind::NonExhaustiveMatch,
                        span,
                    });
                }

                let mut unified = arm_types[0].clone();
                for at in &arm_types[1..] {
                    unified = Type::common_type(&unified, at).ok_or(CheckerError {
                        code: DiagnosticCode::W0301,
                        kind: CheckerErrorKind::IncompatibleMatchArms,
                        span,
                    })?;
                }
                Ok(unified)
            }
            _ => Err(CheckerError {
                code: DiagnosticCode::W0301,
                kind: CheckerErrorKind::TypeMismatch {
                    expected: "Option or Result".to_string(),
                    actual: target_ty.display_name(),
                },
                span,
            }),
        }
    }
}

/// WX-A pattern checking and typing (VER04j sections 4.3 and 4.4).
impl<'a> Checker<'a> {
    /// The abstract shape of `ty` for the usefulness algorithm.  Charged work:
    /// a deliberately explosive family of nested records stops at the budget.
    fn pty_of(&mut self, ty: &Type, span: (usize, usize)) -> Result<PTy, CheckerError> {
        self.charge_work(span)?;
        self.enter_scope(span, |this| {
            Ok(match ty {
                Type::Bool => PTy::Bool,
                Type::Option(t) => PTy::Option(Box::new(this.pty_of(t, span)?)),
                Type::Result(t, e) => {
                    let n = this.errors.get(e).map_or(0, |(v, _)| v.len());
                    PTy::Result(Box::new(this.pty_of(t, span)?), Box::new(PTy::ErrCases(n)))
                }
                Type::ErrorVariant(e) if !e.contains('.') => {
                    PTy::ErrCases(this.errors.get(e).map_or(0, |(v, _)| v.len()))
                }
                Type::Tuple(ts) => {
                    let mut out = Vec::with_capacity(ts.len());
                    for t in ts {
                        out.push(this.pty_of(t, span)?);
                    }
                    PTy::Tuple(out)
                }
                Type::Adt(name) => {
                    let ctors: Vec<Vec<Type>> = this.adts[name]
                        .ctors
                        .iter()
                        .map(|c| c.fields.iter().map(|(_, t)| t.clone()).collect())
                        .collect();
                    let mut out = Vec::with_capacity(ctors.len());
                    for fields in ctors {
                        let mut fs = Vec::with_capacity(fields.len());
                        for t in &fields {
                            fs.push(this.pty_of(t, span)?);
                        }
                        out.push(fs);
                    }
                    PTy::Data(out)
                }
                _ => PTy::Opaque,
            })
        })
    }

    /// Check `pattern` against `ty`, collecting its binders, and return the
    /// abstract pattern.  `depth` is 1 at the top of an arm.
    fn check_pattern(
        &mut self,
        pattern: &Pattern<'a>,
        ty: &Type,
        env: &HashMap<String, Type>,
        binds: &mut Vec<(String, Type)>,
        arm_span: (usize, usize),
        depth: usize,
    ) -> Result<Pat, CheckerError> {
        self.charge_work(arm_span)?;
        if depth > MAX_PATTERN_DEPTH {
            return Err(CheckerError {
                code: DiagnosticCode::W0602,
                kind: CheckerErrorKind::DepthBudgetExceeded,
                span: arm_span,
            });
        }
        let mismatch = || CheckerError {
            code: DiagnosticCode::W0301,
            kind: CheckerErrorKind::IncompatibleMatchArms,
            span: arm_span,
        };
        match pattern {
            Pattern::Wild => Ok(Pat::Wild),
            Pattern::Bool(b) => {
                if *ty != Type::Bool {
                    return Err(mismatch());
                }
                Ok(Pat::Ctor(usize::from(*b), vec![]))
            }
            Pattern::None => match ty {
                Type::Option(_) => Ok(Pat::Ctor(0, vec![])),
                _ => Err(mismatch()),
            },
            Pattern::Some(inner) => match ty {
                Type::Option(t) => {
                    let p = self.check_pattern(inner, t, env, binds, arm_span, depth + 1)?;
                    Ok(Pat::Ctor(1, vec![p]))
                }
                _ => Err(mismatch()),
            },
            Pattern::Ok(inner) => match ty {
                Type::Result(t, _) => {
                    let p = self.check_pattern(inner, t, env, binds, arm_span, depth + 1)?;
                    Ok(Pat::Ctor(0, vec![p]))
                }
                _ => Err(mismatch()),
            },
            Pattern::Err(inner) => match ty {
                Type::Result(_, e) => {
                    let et = Type::ErrorVariant(e.clone());
                    let p = self.check_pattern(inner, &et, env, binds, arm_span, depth + 1)?;
                    Ok(Pat::Ctor(1, vec![p]))
                }
                _ => Err(mismatch()),
            },
            Pattern::Tuple(items) => match ty {
                Type::Tuple(ts) if ts.len() == items.len() => {
                    let mut out = Vec::with_capacity(items.len());
                    for (p, t) in items.iter().zip(ts) {
                        out.push(self.check_pattern(p, t, env, binds, arm_span, depth + 1)?);
                    }
                    Ok(Pat::Ctor(0, out))
                }
                _ => Err(mismatch()),
            },
            Pattern::Ctor { name, args } => {
                if name.segments.len() == 2 {
                    // `E.CASE`: a case of the scrutinee's error type.
                    let (en, case) = (name.segments[0], name.segments[1]);
                    let Type::ErrorVariant(want) = ty else {
                        return Err(mismatch());
                    };
                    if want != en || !args.is_empty() {
                        return Err(mismatch());
                    }
                    let (cases, _) = self.errors.get(en).ok_or_else(mismatch)?;
                    let idx = cases.iter().position(|c| c == case).ok_or(CheckerError {
                        code: DiagnosticCode::W0202,
                        kind: CheckerErrorKind::UnboundConstructor(format!("{en}.{case}")),
                        span: arm_span,
                    })?;
                    return Ok(Pat::Ctor(idx, vec![]));
                }
                if name.segments.len() != 1 {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0202,
                        kind: CheckerErrorKind::UnboundConstructor(name.segments.join(".")),
                        span: arm_span,
                    });
                }
                let n = name.segments[0];
                if let Some((tname, idx)) = self.ctor_index.get(n).cloned() {
                    if *ty != Type::Adt(tname.clone()) {
                        return Err(mismatch());
                    }
                    let fields: Vec<Type> = self.adts[&tname].ctors[idx]
                        .fields
                        .iter()
                        .map(|(_, t)| t.clone())
                        .collect();
                    if fields.len() != args.len() {
                        return Err(CheckerError {
                            code: DiagnosticCode::W0301,
                            kind: CheckerErrorKind::ArityMismatch {
                                expected: fields.len(),
                                actual: args.len(),
                            },
                            span: arm_span,
                        });
                    }
                    let mut out = Vec::with_capacity(args.len());
                    for (p, t) in args.iter().zip(&fields) {
                        out.push(self.check_pattern(p, t, env, binds, arm_span, depth + 1)?);
                    }
                    return Ok(Pat::Ctor(idx, out));
                }
                if !args.is_empty() {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0202,
                        kind: CheckerErrorKind::UnboundConstructor(n.to_string()),
                        span: arm_span,
                    });
                }
                // Any other NAME is a fresh binder.
                self.check_arm_binder(n, arm_span, env)?;
                if binds.iter().any(|(b, _)| b == n) {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0203,
                        kind: CheckerErrorKind::ShadowedBinding(n.to_string()),
                        span: arm_span,
                    });
                }
                binds.push((n.to_string(), ty.clone()));
                Ok(Pat::Wild)
            }
        }
    }

    /// `match` in a v1 document: nested patterns, the usefulness check for
    /// exhaustiveness and redundancy (W0301), bounded by 256 arms, pattern
    /// depth 8 and the checker's work budget (W0602).
    fn typecheck_match_v1(
        &mut self,
        target: &Expression<'a>,
        arms: &[MatchArm<'a>],
        env: &HashMap<String, Type>,
        expected: Option<&Type>,
        allow_ops: bool,
        span: (usize, usize),
    ) -> Result<Type, CheckerError> {
        if arms.len() > MAX_MATCH_ARMS {
            return Err(CheckerError {
                code: DiagnosticCode::W0602,
                kind: CheckerErrorKind::TooManyArms,
                span,
            });
        }
        let target_ty = self.typecheck_expression(target, env, None, allow_ops)?;
        let shape = self.pty_of(&target_ty, span)?;
        let mut rows: Vec<Pat> = Vec::new();
        let mut arm_types: Vec<Type> = Vec::new();
        let mut budget = self.limits.max_work.saturating_sub(self.work_count);
        for arm in arms {
            let mut binds = Vec::new();
            let pat = self.check_pattern(&arm.pattern, &target_ty, env, &mut binds, arm.span, 1)?;
            // Redundancy: this arm must be useful against the arms before it.
            let tys = std::slice::from_ref(&shape);
            let prior: Vec<Vec<Pat>> = rows.iter().map(|p| vec![p.clone()]).collect();
            let useful = patterns::useful(&prior, std::slice::from_ref(&pat), tys, &mut budget)
                .map_err(|_| CheckerError {
                    code: DiagnosticCode::W0602,
                    kind: CheckerErrorKind::WorkBudgetExceeded,
                    span: arm.span,
                })?;
            if !useful {
                return Err(CheckerError {
                    code: DiagnosticCode::W0301,
                    kind: CheckerErrorKind::RedundantArm,
                    span: arm.span,
                });
            }
            rows.push(pat);
            let mut local_env = env.clone();
            for (n, t) in binds {
                local_env.insert(n, t);
            }
            arm_types.push(self.typecheck_expression(&arm.body, &local_env, expected, allow_ops)?);
        }
        let report = patterns::analyze(&rows, &shape, &mut budget).map_err(|_| CheckerError {
            code: DiagnosticCode::W0602,
            kind: CheckerErrorKind::WorkBudgetExceeded,
            span,
        })?;
        if !report.exhaustive {
            return Err(CheckerError {
                code: DiagnosticCode::W0301,
                kind: CheckerErrorKind::NonExhaustiveMatch,
                span,
            });
        }
        let mut unified = arm_types[0].clone();
        for at in &arm_types[1..] {
            unified = Type::common_type(&unified, at).ok_or(CheckerError {
                code: DiagnosticCode::W0301,
                kind: CheckerErrorKind::IncompatibleMatchArms,
                span,
            })?;
        }
        Ok(unified)
    }
}

fn body_span(expr: &Expression<'_>) -> (usize, usize) {
    match expr {
        Expression::Quantified { span, .. }
        | Expression::Conditional { span, .. }
        | Expression::Match { span, .. }
        | Expression::Binary { span, .. }
        | Expression::Unary { span, .. }
        | Expression::Literal { span, .. }
        | Expression::List { span, .. }
        | Expression::Reference { span, .. }
        | Expression::Grouped { span, .. }
        | Expression::Tuple { span, .. }
        | Expression::Project { span, .. } => *span,
    }
}

fn type_expr_span(ty: &TypeExpr<'_>) -> (usize, usize) {
    match ty {
        TypeExpr::Bytes { span, .. }
        | TypeExpr::List { span, .. }
        | TypeExpr::Option { span, .. }
        | TypeExpr::Result { span, .. }
        | TypeExpr::Tuple { span, .. }
        | TypeExpr::Named { span, .. } => *span,
    }
}

/// Convenience driver to check an admitted Warrant AST with default limits.
pub fn check_warrant<'a>(file: &'a WarrantFile<'a>) -> Result<CheckedWarrantModule, CheckerError> {
    check_warrant_with_limits(file, CheckerLimits::default())
}

/// Convenience driver to check an admitted Warrant AST with custom limits.
pub fn check_warrant_with_limits<'a>(
    file: &'a WarrantFile<'a>,
    limits: CheckerLimits,
) -> Result<CheckedWarrantModule, CheckerError> {
    Checker::new(file, limits).check()
}
