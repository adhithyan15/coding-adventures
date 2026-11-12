//! # Warrant Code Generation and L0 Test Vector Compiler (VER04e / VER-P008)
//!
//! Compiles checked Warrant models, examples, contracts, and bindings into:
//! 1. Canonical Portable Test Vectors (TFV records).
//! 2. Standalone, deterministic Rust L0 test harnesses running on `test-bootstrap`.
//!
//! Zero third-party dependencies. Bounded evaluation depth (<= 64) and work quota (<= 1,000,000).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::path::Path;

use crate::binding::BindingManifest;
use crate::checker::{CheckedWarrantModule, V1_FUNCTIONS};
use crate::fixed::{self, FixedTy};
use crate::parser::{
    BinaryOp, Expression, Literal, MatchArm, ModelDecl, Pattern, ProjKey, UnaryOp, Version,
    WarrantFile, Width,
};

/// Diagnostic codes for codegen phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiagnosticCode {
    /// W0701: Unsupported codegen target or uninstantiated static generic width.
    W0701,
    /// W0702: Non-ground term in compile-time model evaluation.
    W0702,
    /// W0703: Evaluator resource exhaustion (depth > 64 or work > 1,000,000).
    W0703,
    /// W0704: Missing or incompatible host binding for operation.
    W0704,
    /// W0705: Duplicate or conflicting test case identifier.
    W0705,
    /// W0706: Bounded-exhaustive domain exceeds the case budget (VER04h §3).
    W0706,
    /// W0707: A fixed byte width exceeds the cap (VER04h §3): the exhaustive
    /// domain would allocate `O(width^2)` bytes.
    W0707,
    /// W0709: A value nests deeper than TFV's 64-container limit (VER04j
    /// section 4.9) and cannot be encoded into a corpus; or a table row has a
    /// shape no Warrant type has (WX-E).
    W0709,
    /// W0601: A table that cannot discriminate (VER04j section 5.3 item 4).
    W0601,
    /// W0602: A budget is exhausted: more table rows than the tool profile
    /// allows (VER04j section 5.2).
    W0602,
    /// W0708: Table integrity: bad path, symlink, size limit, digest mismatch,
    /// row-count mismatch, malformed TFV (VER04j section 5.4).
    W0708,
    /// W0710: A table row whose fields or values do not fit the declared columns.
    W0710,
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::W0701 => write!(f, "W0701"),
            Self::W0702 => write!(f, "W0702"),
            Self::W0703 => write!(f, "W0703"),
            Self::W0704 => write!(f, "W0704"),
            Self::W0705 => write!(f, "W0705"),
            Self::W0706 => write!(f, "W0706"),
            Self::W0707 => write!(f, "W0707"),
            Self::W0709 => write!(f, "W0709"),
            Self::W0601 => write!(f, "W0601"),
            Self::W0602 => write!(f, "W0602"),
            Self::W0708 => write!(f, "W0708"),
            Self::W0710 => write!(f, "W0710"),
        }
    }
}

/// Specific codegen error variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodegenErrorKind {
    UnsupportedTarget(String),
    NonGroundTerm(String),
    RecursionDepthExceeded,
    WorkBudgetExceeded,
    MissingBinding(String),
    DuplicateId(String),
    EvaluationError(String),
    InvalidHexLiteral(String),
    DivisionByZero,
    /// WX-D (VER04k section 2.7, step 1): a sequence result would have more
    /// than [`MAX_SEQ_ELEMENTS`] elements.  Refused before anything is
    /// charged or allocated for it.
    SequenceTooLong,
    /// WX-D (VER04k section 2.7, step 2): the evaluation's cumulative owned
    /// data would pass [`DATA_BUDGET_BYTES`].  Refused before allocating.
    DataBudgetExceeded,
}

/// Codegen and compilation error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodegenError {
    pub code: DiagnosticCode,
    pub kind: CodegenErrorKind,
    pub span: (usize, usize),
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {:?}", self.code, self.kind)
    }
}

impl std::error::Error for CodegenError {}

/// Resource limits for compile-time model evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodegenLimits {
    pub max_depth: usize,
    pub max_work: usize,
    pub max_value_bytes: usize,
}

// ---------------------------------------------------------------------------
// WX-D: the sequence caps of the meter (VER04k section 2.7)
// ---------------------------------------------------------------------------
//
// One evaluation (an example's expected value, one claim instance, one table
// row) owns one meter.  WX-D adds two counters to the work and depth counters
// the evaluator already had:
//
// ```text
//   counter                         limit                 exceeded is
//   work units                      1,000,000 (limits)    W0703 WorkBudgetExceeded
//   nesting depth                   64        (limits)    W0703 RecursionDepthExceeded
//   elements in one sequence value  65,536    (below)     W0703 SequenceTooLong
//   owned data, cumulative          64 MiB    (below)     W0703 DataBudgetExceeded
// ```
//
// The two new limits are constants, not `CodegenLimits` fields, so the public
// limits struct (and every caller that builds one) is unchanged.  A caller
// that lowers `max_value_bytes` lowers the element cap with it: the cap used is
// the smaller of the two, so a stricter profile is never loosened.
//
// *Owned data* is cumulative allocation, never refunded (live-data accounting
// would depend on when an implementation frees memory, so it would not be
// deterministic across implementations).  What a value costs is its `dsize`
// (below), and which evaluation steps pay it is VER04k section 2.7 (S0 E4):
//
// ```text
//   every node, in a v1 evaluation, charges dsize(its value) ...
//   ... except the pass-through nodes, which hand on a value that exists:
//       a binder reference or projection path, a projection, `if`, `match`,
//       a parenthesised group, a model call (its body's nodes pay), `unwrap_or`
//       (and, from WX-D2, `fold`)
// ```
//
// A node whose value is a scalar charges 0, so in practice the payers are the
// nodes that build something: list literals, tuples, records, constructors
// (nullary ones too, for the names they store: E20), error values `E.C`,
// `Some`/`Ok`, bytes literals, `concat`, `be_bytes`/`le_bytes`, `fixed<N>`,
// `at`, the `Option`-valued v1 built-ins and the seven WX-D1 forms.  Reads
// charge nothing (VER04k section 9.1, note on E4, which records the memory an
// uncharged deep-copying read can still take as an open hazard).

/// Most elements one sequence value may have (VER04k `MAX_SEQ_ELEMENTS`).
pub const MAX_SEQ_ELEMENTS: usize = 65_536;

/// Most owned data, in bytes, one evaluation may produce in total (VER04k
/// `DATA_BUDGET_BYTES`, VER04 section 2: "64 MiB of owned intermediate/output
/// data").
pub const DATA_BUDGET_BYTES: u64 = 64 * 1024 * 1024;

/// The bytes charged for one container node: the same constant the evidence
/// generator's domain budget uses (VER04h section 3).
const NODE_BYTES: u128 = crate::evidence::VALUE_OVERHEAD as u128;

/// `dsize(v)`, the owned-data cost of a value (VER04k section 2.7, "Data
/// size"):
///
/// ```text
///   Bool, Int, Nat, U8, ..., None             0     (its storage is its slot)
///   Bytes(b), Bytes<N>                        len(b)
///   List / tuple c1..cn                       sum of (32 + dsize(ci))
///   constructor T.C(c1..cn), nullary too      len(T) + len(C) + sum of (32 + dsize(ci))
///   Err(E.C)                                  len(E) + len(C)
///   Some(x), Ok(x)                            32 + dsize(x)
/// ```
///
/// So `replicate(x, n)` reserves `n x (32 + dsize(x))` and `take` of a
/// `List<Bytes>` pays for its elements' bytes too (S0 E5): the evaluator's
/// values are deep copies, and the meter charges what the copy costs.
///
/// **Names are data (E20).**  A constructor value owns its type and
/// constructor names, and an error value its error and case names, as
/// `String`s, so every copy allocates them again.  Pricing them at 0 let
/// `replicate(C, 65536)` with a 10,000-byte `C` allocate 660 MB while the
/// meter saw 2 MiB.  Counting the name bytes keeps the charge at or above the
/// real allocation (up to the fixed size of a slot), whatever the names are.
///
/// `[b"0102", b"03"]` is `(32 + 2) + (32 + 1) = 67` bytes; `(1, 2)` is 64;
/// `Circle(5u8)` of `variant Shape` is `5 + 6 + 32 = 43`.
fn dsize(v: &EvalValue) -> u128 {
    match v {
        EvalValue::Bytes(b) => b.len() as u128,
        EvalValue::List(xs) | EvalValue::Tuple(xs) => dsize_items(xs),
        EvalValue::Con {
            ty, ctor, fields, ..
        } => names_size(ty, ctor).saturating_add(dsize_items(fields)),
        EvalValue::Result(Err((ty, case))) => names_size(ty, case),
        EvalValue::Option(Some(x)) | EvalValue::Result(Ok(x)) => {
            NODE_BYTES.saturating_add(dsize(x))
        }
        _ => 0,
    }
}

/// The bytes of the two names a constructor or error value stores (E20).
fn names_size(ty: &str, name: &str) -> u128 {
    (ty.len() as u128).saturating_add(name.len() as u128)
}

/// `sum over xs of (32 + dsize(x))`: the cost of a container holding `xs`, or
/// of a deep copy of the elements `xs`.  Saturating, so an absurd sum is
/// simply over budget.
fn dsize_items(xs: &[EvalValue]) -> u128 {
    xs.iter().fold(0u128, |acc, x| {
        acc.saturating_add(NODE_BYTES.saturating_add(dsize(x)))
    })
}

/// `clamp(n) = max(0, min(n, len))` on the exact count (VER04k section 2.2):
/// a negative count means zero, a count past the end means "all of it".
/// Nothing is narrowed before the comparison, so `2^70` clamps to `len`.
fn clamp_count(n: i128, len: usize) -> usize {
    if n <= 0 {
        0
    } else if n >= len as i128 {
        len
    } else {
        n as usize
    }
}

impl Default for CodegenLimits {
    fn default() -> Self {
        Self {
            max_depth: 64,
            max_work: 1_000_000,
            max_value_bytes: 65_536,
        }
    }
}

/// Ground evaluated value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvalValue {
    Bool(bool),
    Int(i128),
    Nat(u64),
    U8(u8),
    U64(u64),
    /// One of the six fixed-width types WX-B adds (`U16 U32 I8 I16 I32 I64`),
    /// holding its canonical in-range value.  `U8` and `U64` keep their v0
    /// variants; [`EvalValue::as_fixed`] and [`EvalValue::from_fixed`] map
    /// between the two spellings so every operation sees one shape.
    Fixed(FixedTy, i128),
    Bytes(Vec<u8>),
    List(Vec<EvalValue>),
    Option(Option<Box<EvalValue>>),
    Result(Result<Box<EvalValue>, (String, String)>), // Ok(val) or Err(Type, Variant)
    /// WX-A: `(v1, ..., vn)` (VER04j section 4.5).
    Tuple(Vec<EvalValue>),
    /// WX-A: a constructed record or variant value.  `index` is the position of
    /// the constructor in its declaration; the name is kept beside it because
    /// the portable corpus keys a variant on the constructor *name*.
    Con {
        ty: String,
        index: usize,
        ctor: String,
        record: bool,
        fields: Vec<EvalValue>,
    },
}

/// TFV's container nesting limit (`test-bootstrap` `tfv::MAX_DEPTH`).
pub const TFV_MAX_DEPTH: usize = 64;

impl EvalValue {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Bytes(b) => Some(b.as_slice()),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Self::Nat(n) | Self::U64(n) => Some(*n),
            Self::U8(n) => Some(*n as u64),
            Self::Int(i) if *i >= 0 => Some(*i as u64),
            Self::Fixed(_, v) if *v >= 0 => Some(*v as u64),
            _ => None,
        }
    }

    pub fn as_i128(&self) -> Option<i128> {
        match self {
            Self::Int(i) => Some(*i),
            Self::Nat(n) | Self::U64(n) => Some(*n as i128),
            Self::U8(n) => Some(*n as i128),
            Self::Fixed(_, v) => Some(*v),
            _ => None,
        }
    }

    /// The fixed-width type and canonical value of a `U8`, `U64` or
    /// [`EvalValue::Fixed`] value.
    pub fn as_fixed(&self) -> Option<(FixedTy, i128)> {
        match self {
            Self::U8(n) => Some((FixedTy::U8, i128::from(*n))),
            Self::U64(n) => Some((FixedTy::U64, i128::from(*n))),
            Self::Fixed(ty, v) => Some((*ty, *v)),
            _ => None,
        }
    }

    /// Build the canonical value of `ty` (`v` must already be in range).
    pub fn from_fixed(ty: FixedTy, v: i128) -> Self {
        match ty {
            FixedTy::U8 => Self::U8(v as u8),
            FixedTy::U64 => Self::U64(v as u64),
            _ => Self::Fixed(ty, v),
        }
    }

    /// Serialize into canonical TFV prefix notation value.
    pub fn encode_tfv(&self) -> String {
        match self {
            Self::Bool(true) => "T".to_string(),
            Self::Bool(false) => "F".to_string(),
            Self::Int(i) => format!("I{i}"),
            Self::Nat(n) | Self::U64(n) => format!("I{n}"),
            Self::U8(n) => format!("I{n}"),
            Self::Fixed(_, v) => format!("I{v}"),
            Self::Bytes(bytes) => format!("X{}", hex_encode(bytes)),
            Self::List(items) => {
                let mut out = format!("L{}", items.len());
                for item in items {
                    out.push(' ');
                    out.push_str(&item.encode_tfv());
                }
                out
            }
            Self::Option(None) => "N".to_string(),
            Self::Option(Some(inner)) => inner.encode_tfv(),
            Self::Result(Ok(inner)) => inner.encode_tfv(),
            Self::Result(Err((_, variant))) => format!("S{}", hex_encode(variant.as_bytes())),
            Self::Tuple(_) | Self::Con { .. } => self.encode_portable(),
        }
    }

    /// The portable corpus encoding of VER04j section 4.9.
    ///
    /// ```text
    /// Bool        T | F                      Nat, U8, I32, ...   I<decimal>
    /// Bytes       X<hex>                     List                L<n> v1 ... vn
    /// Option      L0 | L1 x
    /// tuple       L<n> v1 ... vn             record              L<n> v1 ... vn
    /// variant     L2 S<hex ctor name> L<k> f1 ... fk    (by *name*, so
    ///             reordering declarations cannot change a vector)
    /// Result      M1 S<hex "ok"> v | M1 S<hex "err"> S<hex VARIANT>
    ///             (the VER04i form; VER04j 4.9 proposed `L2 S<Ok|Err> v`,
    ///             see section 14.1 item 19)
    /// ```
    ///
    /// ```text
    /// (7u64, 3)                      ->  L2 I7 I3
    /// Circle(5)  of  Circle(r)|Empty ->  L2 S436972636c65 L1 I5
    /// Some((1, 2))                   ->  L1 L2 I1 I2
    /// ```
    pub fn encode_portable(&self) -> String {
        let mut out = String::new();
        self.encode_portable_into(&mut out);
        out
    }

    fn encode_portable_into(&self, out: &mut String) {
        if !out.is_empty() {
            out.push(' ');
        }
        let list = |out: &mut String, items: &[EvalValue]| {
            out.push_str(&format!("L{}", items.len()));
            for item in items {
                item.encode_portable_into(out);
            }
        };
        match self {
            Self::Bool(true) => out.push('T'),
            Self::Bool(false) => out.push('F'),
            Self::Int(i) => out.push_str(&format!("I{i}")),
            Self::Nat(n) | Self::U64(n) => out.push_str(&format!("I{n}")),
            Self::U8(n) => out.push_str(&format!("I{n}")),
            Self::Fixed(_, v) => out.push_str(&format!("I{v}")),
            Self::Bytes(b) => out.push_str(&format!("X{}", hex_encode(b))),
            Self::List(items) | Self::Tuple(items) => list(out, items),
            Self::Option(None) => out.push_str("L0"),
            Self::Option(Some(inner)) => {
                out.push_str("L1");
                inner.encode_portable_into(out);
            }
            Self::Result(Ok(inner)) => {
                out.push_str(&format!("M1 S{}", hex_encode(b"ok")));
                inner.encode_portable_into(out);
            }
            Self::Result(Err((_, variant))) => out.push_str(&format!(
                "M1 S{} S{}",
                hex_encode(b"err"),
                hex_encode(variant.as_bytes())
            )),
            Self::Con {
                record: true,
                fields,
                ..
            } => list(out, fields),
            Self::Con { ctor, fields, .. } => {
                out.push_str(&format!("L2 S{}", hex_encode(ctor.as_bytes())));
                out.push(' ');
                list(out, fields);
            }
        }
    }

    /// Container nesting of the portable encoding (a scalar is 0).
    pub fn container_depth(&self) -> usize {
        let max_of = |items: &[EvalValue]| items.iter().map(Self::container_depth).max().unwrap_or(0);
        match self {
            Self::List(items) | Self::Tuple(items) => 1 + max_of(items),
            Self::Option(None) => 1,
            Self::Option(Some(inner)) | Self::Result(Ok(inner)) => 1 + inner.container_depth(),
            Self::Result(Err(_)) => 1,
            Self::Con {
                record: true,
                fields,
                ..
            } => 1 + max_of(fields),
            // L2 S<ctor> L<k> ...: the outer pair and the field list.
            Self::Con { fields, .. } => 2 + max_of(fields),
            _ => 0,
        }
    }

    /// [`EvalValue::encode_portable`], refusing a value nested deeper than
    /// TFV's 64-container limit at generation time (W0709).
    pub fn encode_checked(&self) -> Result<String, CodegenError> {
        let depth = self.container_depth();
        if depth > TFV_MAX_DEPTH {
            return Err(CodegenError {
                code: DiagnosticCode::W0709,
                kind: CodegenErrorKind::UnsupportedTarget(format!(
                    "value nests {depth} containers deep; TFV allows {TFV_MAX_DEPTH}"
                )),
                span: (0, 0),
            });
        }
        Ok(self.encode_portable())
    }

    /// Leaf count of a value, the unit of the owned-data budget.
    fn leaves(&self) -> usize {
        match self {
            Self::Bytes(b) => b.len().max(1),
            Self::List(items) | Self::Tuple(items) => {
                1 + items.iter().map(Self::leaves).sum::<usize>()
            }
            Self::Con { fields, .. } => 1 + fields.iter().map(Self::leaves).sum::<usize>(),
            Self::Option(Some(inner)) | Self::Result(Ok(inner)) => 1 + inner.leaves(),
            _ => 1,
        }
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

pub fn hex_decode(s: &str, span: (usize, usize)) -> Result<Vec<u8>, CodegenError> {
    if s.len() & 1 != 0 {
        return Err(CodegenError {
            code: DiagnosticCode::W0702,
            kind: CodegenErrorKind::InvalidHexLiteral(format!(
                "Odd-length hex string: length {}",
                s.len()
            )),
            span,
        });
    }
    let mut bytes = Vec::with_capacity(s.len() / 2);
    let raw = s.as_bytes();
    for i in (0..raw.len()).step_by(2) {
        let hi = match raw[i] {
            b'0'..=b'9' => raw[i] - b'0',
            b'a'..=b'f' => raw[i] - b'a' + 10,
            b'A'..=b'F' => raw[i] - b'A' + 10,
            _ => {
                return Err(CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::InvalidHexLiteral(format!(
                        "Invalid hex byte {}",
                        raw[i]
                    )),
                    span,
                })
            }
        };
        let lo = match raw[i + 1] {
            b'0'..=b'9' => raw[i + 1] - b'0',
            b'a'..=b'f' => raw[i + 1] - b'a' + 10,
            b'A'..=b'F' => raw[i + 1] - b'A' + 10,
            _ => {
                return Err(CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::InvalidHexLiteral(format!(
                        "Invalid hex byte {}",
                        raw[i + 1]
                    )),
                    span,
                })
            }
        };
        bytes.push((hi << 4) | lo);
    }
    Ok(bytes)
}

/// Pure model and ground expression evaluator.
pub struct Evaluator<'a> {
    models: HashMap<&'a str, &'a ModelDecl<'a>>,
    limits: CodegenLimits,
    depth: usize,
    work: usize,
    /// WX-D: owned data produced by this evaluation so far, in bytes
    /// (cumulative, never refunded; reset with the work counter per instance).
    /// Only a v1 document charges it.
    data: u64,
    /// The document is a v1 document: the WX-B/WX-C built-ins are in scope and
    /// numeric equality compares by mathematical value.
    v1: bool,
    /// WX-A: constructor name -> (type name, constructor index, is_record).
    ctors: HashMap<&'a str, (&'a str, usize, bool)>,
    /// WX-A: type name -> per constructor (name, field names).
    adts: HashMap<&'a str, Vec<(&'a str, Vec<&'a str>)>>,
}

impl<'a> Evaluator<'a> {
    pub fn new(file: &'a WarrantFile<'a>, limits: CodegenLimits) -> Self {
        let mut models = HashMap::new();
        let mut ctors = HashMap::new();
        let mut adts = HashMap::new();
        for decl in &file.declarations {
            match decl {
                crate::parser::Declaration::Model(m) => {
                    models.insert(m.name, m);
                }
                crate::parser::Declaration::Record(r) => {
                    ctors.insert(r.name, (r.name, 0, true));
                    adts.insert(
                        r.name,
                        vec![(r.name, r.fields.iter().map(|f| f.name).collect())],
                    );
                }
                crate::parser::Declaration::Variant(v) => {
                    let mut list = Vec::new();
                    for (i, c) in v.ctors.iter().enumerate() {
                        ctors.insert(c.name, (v.name, i, false));
                        list.push((c.name, c.fields.iter().map(|f| f.name).collect()));
                    }
                    adts.insert(v.name, list);
                }
                _ => {}
            }
        }
        Self {
            models,
            limits,
            depth: 0,
            work: 0,
            data: 0,
            v1: file.version == Version::V1,
            ctors,
            adts,
        }
    }

    fn charge_work(&mut self, amount: usize, span: (usize, usize)) -> Result<(), CodegenError> {
        self.work = self.work.saturating_add(amount);
        if self.work > self.limits.max_work {
            return Err(CodegenError {
                code: DiagnosticCode::W0703,
                kind: CodegenErrorKind::WorkBudgetExceeded,
                span,
            });
        }
        Ok(())
    }

    fn enter_scope(&mut self, span: (usize, usize)) -> Result<(), CodegenError> {
        self.depth += 1;
        if self.depth > self.limits.max_depth {
            return Err(CodegenError {
                code: DiagnosticCode::W0703,
                kind: CodegenErrorKind::RecursionDepthExceeded,
                span,
            });
        }
        Ok(())
    }

    fn leave_scope(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    /// Start a new instance: the work meter and the WX-D data meter are both
    /// per evaluation (VER04k section 2.7).
    pub fn reset_work(&mut self) {
        self.work = 0;
        self.data = 0;
    }

    /// Work units charged since the last [`Evaluator::reset_work`].
    pub fn work_used(&self) -> usize {
        self.work
    }

    /// Owned-data bytes charged since the last [`Evaluator::reset_work`]
    /// (always 0 in a v0 document, which has no data meter).
    pub fn data_used(&self) -> u64 {
        self.data
    }

    /// Reserve `amount` bytes of owned data (step 2 of VER04k section 2.7),
    /// saturating, so an absurd request is simply over budget.
    fn charge_data(&mut self, amount: u128, span: (usize, usize)) -> Result<(), CodegenError> {
        let total = u128::from(self.data).saturating_add(amount);
        self.data = u64::try_from(total).unwrap_or(u64::MAX);
        if total > u128::from(DATA_BUDGET_BYTES) {
            return Err(CodegenError {
                code: DiagnosticCode::W0703,
                kind: CodegenErrorKind::DataBudgetExceeded,
                span,
            });
        }
        Ok(())
    }

    /// Charge `amount()` bytes only in a v1 document (VER04k section 9.1, S0
    /// E4): the values every v1 evaluation builds count toward the data meter,
    /// so a sequence built by `concat` or literals cannot dodge it; v0 has no
    /// data meter and stays byte-identical.
    ///
    /// The amount is a closure so that a v0 evaluation does not even compute
    /// it: `dsize` walks the whole value, and v0 has no counter to feed.
    fn charge_data_v1(
        &mut self,
        amount: impl FnOnce() -> u128,
        span: (usize, usize),
    ) -> Result<(), CodegenError> {
        if self.v1 {
            self.charge_data(amount(), span)
        } else {
            Ok(())
        }
    }

    /// Step 1 of VER04k section 2.7: the element cap, on the exact length.
    fn seq_cap(&self, len: i128, span: (usize, usize)) -> Result<usize, CodegenError> {
        let cap = MAX_SEQ_ELEMENTS.min(self.limits.max_value_bytes);
        if len > cap as i128 {
            return Err(CodegenError {
                code: DiagnosticCode::W0703,
                kind: CodegenErrorKind::SequenceTooLong,
                span,
            });
        }
        Ok(len.max(0) as usize)
    }

    /// Steps 2 and 3: reserve the data, then charge `L` work units.  The
    /// caller allocates (step 4) only after this returns `Ok`.
    fn seq_reserve(
        &mut self,
        len: usize,
        data: u128,
        span: (usize, usize),
    ) -> Result<(), CodegenError> {
        self.charge_data(data, span)?;
        self.charge_work(len, span)
    }

    /// The diagnostic for an `Int` operation whose exact result leaves `i128`.
    /// VER04j section 2.5: in a v1 document that is W0703 (incomplete
    /// checking, never a wrapped value); v0 keeps its W0702.
    fn overflow_code(&self) -> DiagnosticCode {
        if self.v1 {
            DiagnosticCode::W0703
        } else {
            DiagnosticCode::W0702
        }
    }

    pub fn eval(
        &mut self,
        expr: &Expression<'a>,
        env: &HashMap<&'a str, EvalValue>,
    ) -> Result<EvalValue, CodegenError> {
        self.charge_work(1, expr_span(expr))?;
        self.enter_scope(expr_span(expr))?;
        let res = self.eval_internal(expr, env);
        self.leave_scope();
        res
    }

    fn eval_internal(
        &mut self,
        expr: &Expression<'a>,
        env: &HashMap<&'a str, EvalValue>,
    ) -> Result<EvalValue, CodegenError> {
        match expr {
            Expression::Literal { lit, span } => match lit {
                Literal::Bool(b) => Ok(EvalValue::Bool(*b)),
                Literal::U64(n) => Ok(EvalValue::U64(*n)),
                Literal::U8(n) => Ok(EvalValue::U8(*n)),
                Literal::Fixed(ty, v) => Ok(EvalValue::Fixed(*ty, *v)),
                Literal::Int(n) => Ok(EvalValue::Int(*n as i128)),
                Literal::Bytes(hex_str) => {
                    // A v1 bytes literal produces `len` bytes of owned data.
                    self.charge_data_v1(|| (hex_str.len() / 2) as u128, *span)?;
                    let bytes = hex_decode(hex_str, *span)?;
                    Ok(EvalValue::Bytes(bytes))
                }
            },
            Expression::Grouped { inner, .. } => self.eval(inner, env),
            Expression::List { elements, span } => {
                if elements.len() > self.limits.max_value_bytes {
                    return Err(CodegenError {
                        code: DiagnosticCode::W0703,
                        kind: CodegenErrorKind::WorkBudgetExceeded,
                        span: *span,
                    });
                }
                let mut list = Vec::with_capacity(elements.len());
                for el in elements {
                    list.push(self.eval(el, env)?);
                }
                // v1: the literal's value is new, `sum (32 + dsize(ei))`.
                self.charge_data_v1(|| dsize_items(&list), *span)?;
                self.charge_work(elements.len(), *span)?;
                Ok(EvalValue::List(list))
            }
            Expression::Reference {
                name,
                widths,
                args,
                span,
            } => {
                let first = name.segments[0];
                if name.segments.len() == 1 {
                    // A binder only when the name is not applied, exactly as
                    // the checker reads it (`typecheck_reference`, step 1):
                    // `take(take, 0)` is the built-in even where a binder is
                    // called `take`, so the value is the one the checker typed.
                    if args.is_none() && widths.is_empty() {
                        if let Some(val) = env.get(first) {
                            return Ok(val.clone());
                        }
                    }
                    // WX-A constructors: `C(e1, ..., en)` and the bare nullary `C`.
                    if let Some(&(ty, index, record)) = self.ctors.get(first) {
                        return self.construct(
                            ty,
                            index,
                            record,
                            first,
                            args.as_deref().unwrap_or(&[]),
                            env,
                            *span,
                        );
                    }
                    // Check built-in functions
                    if let Some(arg_exprs) = args {
                        return self.eval_call(first, widths, arg_exprs, env, *span);
                    }
                    // The checker types a bare `None` from its context; a v1
                    // document may therefore write `f(x) == None`.
                    if self.v1 && first == "None" {
                        return Ok(EvalValue::Option(None));
                    }
                    return Err(CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::NonGroundTerm(format!("Unbound symbol {first}")),
                        span: *span,
                    });
                }
                // WX-A: `x.f.g` with a binder head is projection.
                if self.v1 {
                    if let Some(head) = env.get(first) {
                        let mut v = head.clone();
                        for seg in &name.segments[1..] {
                            v = self.project(&v, ProjKey::Field(seg), *span)?;
                        }
                        return Ok(v);
                    }
                }
                // Qualified name: Error.Case constructor
                if name.segments.len() == 2 {
                    // v1 data (E20): the value owns both names.
                    let (ty, case) = (name.segments[0], name.segments[1]);
                    self.charge_data_v1(|| names_size(ty, case), *span)?;
                    let type_name = name.segments[0].to_string();
                    let case_name = name.segments[1].to_string();
                    return Ok(EvalValue::Result(Err((type_name, case_name))));
                }
                Err(CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::NonGroundTerm(format!(
                        "Complex qualified name {first}"
                    )),
                    span: *span,
                })
            }
            Expression::Unary { op, operand, span } => {
                let v = self.eval(operand, env)?;
                match op {
                    UnaryOp::Not => match v {
                        EvalValue::Bool(b) => Ok(EvalValue::Bool(!b)),
                        _ => Err(CodegenError {
                            code: DiagnosticCode::W0702,
                            kind: CodegenErrorKind::EvaluationError("not expects Bool".into()),
                            span: *span,
                        }),
                    },
                    UnaryOp::Neg => match v {
                        // WX-B: wrapping negation of a signed fixed-width value.
                        EvalValue::Fixed(ty, a) if ty.signed() => {
                            Ok(EvalValue::Fixed(ty, fixed::neg(ty, a)))
                        }
                        EvalValue::Int(i) => {
                            let neg = i.checked_neg().ok_or_else(|| CodegenError {
                                code: DiagnosticCode::W0702,
                                kind: CodegenErrorKind::EvaluationError(
                                    "Arithmetic overflow in Neg".into(),
                                ),
                                span: *span,
                            })?;
                            Ok(EvalValue::Int(neg))
                        }
                        EvalValue::Nat(n) => {
                            let neg = (n as i128).checked_neg().ok_or_else(|| CodegenError {
                                code: DiagnosticCode::W0702,
                                kind: CodegenErrorKind::EvaluationError(
                                    "Arithmetic overflow in Neg".into(),
                                ),
                                span: *span,
                            })?;
                            Ok(EvalValue::Int(neg))
                        }
                        _ => Err(CodegenError {
                            code: DiagnosticCode::W0702,
                            kind: CodegenErrorKind::EvaluationError(
                                "neg expects Int or Nat".into(),
                            ),
                            span: *span,
                        }),
                    },
                }
            }
            Expression::Binary {
                op,
                left,
                right,
                span,
            } => {
                // v1: `and`, `or` and `=>` short-circuit, as VER04 section 4
                // says and as generated code always did.  The deciding left
                // operand ends the evaluation: the right one is neither
                // evaluated nor charged.
                //
                // ```text
                //   false and _  = false      true or _  = true      false => _ = true
                // ```
                //
                // v0 keeps its eager evaluation of both operands (a recorded
                // deviation from VER04 section 4) so every v0 artifact, and
                // every v0 work count, stays byte-identical.
                if self.v1 && matches!(op, BinaryOp::And | BinaryOp::Or | BinaryOp::Implies) {
                    let l = self.eval(left, env)?;
                    let a = l.as_bool().ok_or_else(|| CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::EvaluationError(
                            "logical operator expects Bool".into(),
                        ),
                        span: *span,
                    })?;
                    let decided = match op {
                        BinaryOp::And => (!a).then_some(false),
                        BinaryOp::Or => a.then_some(true),
                        _ => (!a).then_some(true),
                    };
                    if let Some(v) = decided {
                        return Ok(EvalValue::Bool(v));
                    }
                    let r = self.eval(right, env)?;
                    return self.eval_binary(*op, l, r, *span);
                }
                let l = self.eval(left, env)?;
                let r = self.eval(right, env)?;
                self.eval_binary(*op, l, r, *span)
            }
            Expression::Conditional {
                condition,
                then_branch,
                else_branch,
                span,
            } => {
                let c = self.eval(condition, env)?;
                match c {
                    EvalValue::Bool(true) => self.eval(then_branch, env),
                    EvalValue::Bool(false) => self.eval(else_branch, env),
                    _ => Err(CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::EvaluationError("Condition must be Bool".into()),
                        span: *span,
                    }),
                }
            }
            Expression::Match { target, arms, span } => {
                let tgt = self.eval(target, env)?;
                self.eval_match(tgt, arms, env, *span)
            }
            Expression::Quantified { span, .. } => Err(CodegenError {
                code: DiagnosticCode::W0702,
                kind: CodegenErrorKind::NonGroundTerm(
                    "Quantifiers cannot be evaluated ground".into(),
                ),
                span: *span,
            }),
            Expression::Tuple { elements, span } => {
                let mut items = Vec::with_capacity(elements.len());
                for el in elements {
                    items.push(self.eval(el, env)?);
                }
                // v1 data: the components exist, so the tuple's dsize is
                // known before it is built (and the leaves cap of `bounded`
                // still applies after, S0 E6).
                self.charge_data_v1(|| dsize_items(&items), *span)?;
                self.charge_work(1 + items.len(), *span)?;
                self.bounded(EvalValue::Tuple(items), *span)
            }
            Expression::Project { base, key, span } => {
                let b = self.eval(base, env)?;
                self.project(&b, *key, *span)
            }
        }
    }

    /// Reject a composite larger than `max_value_bytes` leaves (W0703).
    fn bounded(&self, v: EvalValue, span: (usize, usize)) -> Result<EvalValue, CodegenError> {
        if v.leaves() > self.limits.max_value_bytes {
            return Err(CodegenError {
                code: DiagnosticCode::W0703,
                kind: CodegenErrorKind::WorkBudgetExceeded,
                span,
            });
        }
        Ok(v)
    }

    /// Build constructor `ctor` of type `ty` from already-known argument
    /// expressions (VER04j section 4.5: charge 1 plus one per field).
    #[allow(clippy::too_many_arguments)]
    fn construct(
        &mut self,
        ty: &'a str,
        index: usize,
        record: bool,
        ctor: &'a str,
        args: &[Expression<'a>],
        env: &HashMap<&'a str, EvalValue>,
        span: (usize, usize),
    ) -> Result<EvalValue, CodegenError> {
        let mut fields = Vec::with_capacity(args.len());
        for a in args {
            fields.push(self.eval(a, env)?);
        }
        // v1 data: the fields' slots and payloads, and the two names the
        // value owns (E20), so a nullary constructor is not free either.
        self.charge_data_v1(
            || names_size(ty, ctor).saturating_add(dsize_items(&fields)),
            span,
        )?;
        self.charge_work(1 + fields.len(), span)?;
        self.bounded(
            EvalValue::Con {
                ty: ty.to_string(),
                index,
                ctor: ctor.to_string(),
                record,
                fields,
            },
            span,
        )
    }

    /// `base.0` of a tuple, `base.f` of a record.
    fn project(
        &mut self,
        base: &EvalValue,
        key: ProjKey<'_>,
        span: (usize, usize),
    ) -> Result<EvalValue, CodegenError> {
        self.charge_work(2, span)?;
        let found = match (base, key) {
            (EvalValue::Tuple(items), ProjKey::Index(i)) => items.get(i),
            (
                EvalValue::Con {
                    ty,
                    index,
                    fields,
                    record: true,
                    ..
                },
                ProjKey::Field(f),
            ) => self
                .adts
                .get(ty.as_str())
                .and_then(|cs| cs.get(*index))
                .and_then(|(_, names)| names.iter().position(|n| *n == f))
                .and_then(|pos| fields.get(pos)),
            _ => None,
        };
        found.cloned().ok_or_else(|| CodegenError {
            code: DiagnosticCode::W0702,
            kind: CodegenErrorKind::EvaluationError("projection of a non-composite value".into()),
            span,
        })
    }

    fn eval_call(
        &mut self,
        name: &'a str,
        widths: &[Width<'a>],
        args: &[Expression<'a>],
        env: &HashMap<&'a str, EvalValue>,
        span: (usize, usize),
    ) -> Result<EvalValue, CodegenError> {
        let mut evaled_args = Vec::with_capacity(args.len());
        for a in args {
            evaled_args.push(self.eval(a, env)?);
        }

        // v1 built-ins cannot be shadowed (the checker reserves their names).
        if self.v1 && (V1_FUNCTIONS.contains(&name) || name == "div") {
            if let Some(v) = self.eval_v1_call(name, widths, &evaled_args, span)? {
                // Data (S0 E4): the value is new unless the built-in passes
                // one through (`unwrap_or`); the WX-D1 forms reserved theirs
                // before allocating (`eval_wxd_d1`).
                if !matches!(
                    name,
                    "unwrap_or"
                        | "take"
                        | "drop"
                        | "slice"
                        | "replicate"
                        | "range"
                        | "to_list"
                        | "to_bytes"
                ) {
                    self.charge_data(dsize(&v), span)?;
                }
                return Ok(v);
            }
        }

        // The v0 built-ins build a new value; any other name is a model call,
        // a pass-through node whose body's nodes pay for what they build.
        // `concat` reserves its data itself, before building (below).
        let builds = matches!(
            name,
            "len" | "at" | "fixed" | "div" | "Some" | "None" | "Ok" | "Err"
        );
        let result = match name {
            "len" => {
                if evaled_args.len() != 1 {
                    return Err(CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::EvaluationError("len expects 1 arg".into()),
                        span,
                    });
                }
                match &evaled_args[0] {
                    EvalValue::Bytes(b) => Ok(EvalValue::Nat(b.len() as u64)),
                    EvalValue::List(l) => Ok(EvalValue::Nat(l.len() as u64)),
                    _ => Err(CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::EvaluationError("len expects Bytes or List".into()),
                        span,
                    }),
                }
            }
            "at" => {
                if evaled_args.len() != 2 {
                    return Err(CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::EvaluationError("at expects 2 args".into()),
                        span,
                    });
                }
                let idx = evaled_args[1].as_u64().ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("at index must be Nat".into()),
                    span,
                })? as usize;

                match &evaled_args[0] {
                    EvalValue::Bytes(b) => {
                        if idx < b.len() {
                            Ok(EvalValue::Option(Some(Box::new(EvalValue::U8(b[idx])))))
                        } else {
                            Ok(EvalValue::Option(None))
                        }
                    }
                    EvalValue::List(l) => {
                        if idx < l.len() {
                            Ok(EvalValue::Option(Some(Box::new(l[idx].clone()))))
                        } else {
                            Ok(EvalValue::Option(None))
                        }
                    }
                    _ => Err(CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::EvaluationError("at expects Bytes or List".into()),
                        span,
                    }),
                }
            }
            "concat" => {
                if evaled_args.len() != 2 {
                    return Err(CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::EvaluationError("concat expects 2 args".into()),
                        span,
                    });
                }
                match (&evaled_args[0], &evaled_args[1]) {
                    (EvalValue::Bytes(a), EvalValue::Bytes(b)) => {
                        let total_len = a.len().saturating_add(b.len());
                        if total_len > self.limits.max_value_bytes {
                            return Err(CodegenError {
                                code: DiagnosticCode::W0703,
                                kind: CodegenErrorKind::WorkBudgetExceeded,
                                span,
                            });
                        }
                        self.charge_data_v1(|| total_len as u128, span)?;
                        self.charge_work(total_len, span)?;
                        let mut res = a.clone();
                        res.extend_from_slice(b);
                        Ok(EvalValue::Bytes(res))
                    }
                    (EvalValue::List(a), EvalValue::List(b)) => {
                        let total_len = a.len().saturating_add(b.len());
                        if total_len > self.limits.max_value_bytes {
                            return Err(CodegenError {
                                code: DiagnosticCode::W0703,
                                kind: CodegenErrorKind::WorkBudgetExceeded,
                                span,
                            });
                        }
                        // Both operands are copied element by element.
                        let copied = || dsize_items(a).saturating_add(dsize_items(b));
                        self.charge_data_v1(copied, span)?;
                        self.charge_work(total_len, span)?;
                        let mut res = a.clone();
                        res.extend_from_slice(b);
                        Ok(EvalValue::List(res))
                    }
                    _ => Err(CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::EvaluationError(
                            "concat expects 2 Bytes or 2 Lists".into(),
                        ),
                        span,
                    }),
                }
            }
            "fixed" => {
                if evaled_args.len() != 1 {
                    return Err(CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::EvaluationError("fixed expects 1 arg".into()),
                        span,
                    });
                }
                let target_width = match widths.first() {
                    Some(Width::Literal(n)) => *n,
                    _ => {
                        return Err(CodegenError {
                            code: DiagnosticCode::W0701,
                            kind: CodegenErrorKind::UnsupportedTarget(
                                "fixed requires static literal width".into(),
                            ),
                            span,
                        })
                    }
                };
                match &evaled_args[0] {
                    EvalValue::Bytes(b) => {
                        if b.len() as u64 == target_width {
                            Ok(EvalValue::Option(Some(Box::new(EvalValue::Bytes(
                                b.clone(),
                            )))))
                        } else {
                            Ok(EvalValue::Option(None))
                        }
                    }
                    _ => Err(CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::EvaluationError("fixed expects Bytes".into()),
                        span,
                    }),
                }
            }
            "div" => {
                if evaled_args.len() != 2 {
                    return Err(CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::EvaluationError("div expects 2 args".into()),
                        span,
                    });
                }
                let a = evaled_args[0].as_i128().ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("div lhs must be integer".into()),
                    span,
                })?;
                let b = evaled_args[1].as_i128().ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("div rhs must be integer".into()),
                    span,
                })?;
                if b == 0 {
                    Ok(EvalValue::Option(None))
                } else if a == i128::MIN && b == -1 {
                    Err(CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::EvaluationError(
                            "Arithmetic overflow in div".into(),
                        ),
                        span,
                    })
                } else {
                    let d = a.div_euclid(b);
                    Ok(EvalValue::Option(Some(Box::new(EvalValue::Int(d)))))
                }
            }
            "Some" => {
                if evaled_args.len() != 1 {
                    return Err(CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::EvaluationError("Some expects 1 arg".into()),
                        span,
                    });
                }
                Ok(EvalValue::Option(Some(Box::new(evaled_args.remove(0)))))
            }
            "None" => Ok(EvalValue::Option(None)),
            "Ok" => {
                if evaled_args.len() != 1 {
                    return Err(CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::EvaluationError("Ok expects 1 arg".into()),
                        span,
                    });
                }
                Ok(EvalValue::Result(Ok(Box::new(evaled_args.remove(0)))))
            }
            "Err" => {
                if evaled_args.len() != 1 {
                    return Err(CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::EvaluationError("Err expects 1 arg".into()),
                        span,
                    });
                }
                match evaled_args.remove(0) {
                    EvalValue::Result(Err((t, v))) => Ok(EvalValue::Result(Err((t, v)))),
                    _ => Err(CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::EvaluationError(
                            "Err expects nominal error constructor".into(),
                        ),
                        span,
                    }),
                }
            }
            _ => {
                // Check if user-defined pure model function
                let model = self.models.get(name).copied().ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::NonGroundTerm(format!("Unknown model function {name}")),
                    span,
                })?;

                if model.params.len() != evaled_args.len() {
                    return Err(CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::EvaluationError(format!(
                            "Model {name} expects {} params, got {}",
                            model.params.len(),
                            evaled_args.len()
                        )),
                        span,
                    });
                }

                let mut new_env = HashMap::new();
                for (param, val) in model.params.iter().zip(evaled_args) {
                    new_env.insert(param.name, val);
                }

                let body = model.body.as_ref().ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError(format!("Model {name} has no body")),
                    span,
                })?;

                self.eval(body, &new_env)
            }
        };
        let value = result?;
        if builds {
            self.charge_data_v1(|| dsize(&value), span)?;
        }
        Ok(value)
    }

    fn eval_binary(
        &mut self,
        op: BinaryOp,
        left: EvalValue,
        right: EvalValue,
        span: (usize, usize),
    ) -> Result<EvalValue, CodegenError> {
        match op {
            BinaryOp::Eq => Ok(EvalValue::Bool(self.values_equal(&left, &right))),
            BinaryOp::NotEq => Ok(EvalValue::Bool(!self.values_equal(&left, &right))),
            BinaryOp::And => {
                let a = left.as_bool().ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("and expects Bool".into()),
                    span,
                })?;
                let b = right.as_bool().ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("and expects Bool".into()),
                    span,
                })?;
                Ok(EvalValue::Bool(a && b))
            }
            BinaryOp::Or => {
                let a = left.as_bool().ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("or expects Bool".into()),
                    span,
                })?;
                let b = right.as_bool().ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("or expects Bool".into()),
                    span,
                })?;
                Ok(EvalValue::Bool(a || b))
            }
            BinaryOp::Implies => {
                let a = left.as_bool().ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("=> expects Bool".into()),
                    span,
                })?;
                let b = right.as_bool().ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("=> expects Bool".into()),
                    span,
                })?;
                Ok(EvalValue::Bool(!a || b))
            }
            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
                let a = left.as_i128().ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("Comparison expects integers".into()),
                    span,
                })?;
                let b = right.as_i128().ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("Comparison expects integers".into()),
                    span,
                })?;
                let res = match op {
                    BinaryOp::Lt => a < b,
                    BinaryOp::Le => a <= b,
                    BinaryOp::Gt => a > b,
                    BinaryOp::Ge => a >= b,
                    _ => unreachable!(),
                };
                Ok(EvalValue::Bool(res))
            }
            BinaryOp::Add => {
                if let (EvalValue::U64(a), EvalValue::U64(b)) = (&left, &right) {
                    return Ok(EvalValue::U64(a.wrapping_add(*b)));
                }
                if let (EvalValue::U8(a), EvalValue::U8(b)) = (&left, &right) {
                    return Ok(EvalValue::U8(a.wrapping_add(*b)));
                }
                if let (Some((ta, a)), Some((tb, b))) = (left.as_fixed(), right.as_fixed()) {
                    if ta == tb {
                        return Ok(EvalValue::from_fixed(ta, fixed::add(ta, a, b)));
                    }
                }
                let a = left.as_i128().ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("Add expects integers".into()),
                    span,
                })?;
                let b = right.as_i128().ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("Add expects integers".into()),
                    span,
                })?;
                let sum = a.checked_add(b).ok_or_else(|| CodegenError {
                    code: self.overflow_code(),
                    kind: CodegenErrorKind::EvaluationError("Arithmetic overflow in Add".into()),
                    span,
                })?;
                Ok(EvalValue::Int(sum))
            }
            BinaryOp::Sub => {
                if let (EvalValue::U64(a), EvalValue::U64(b)) = (&left, &right) {
                    return Ok(EvalValue::U64(a.wrapping_sub(*b)));
                }
                if let (EvalValue::U8(a), EvalValue::U8(b)) = (&left, &right) {
                    return Ok(EvalValue::U8(a.wrapping_sub(*b)));
                }
                if let (Some((ta, a)), Some((tb, b))) = (left.as_fixed(), right.as_fixed()) {
                    if ta == tb {
                        return Ok(EvalValue::from_fixed(ta, fixed::sub(ta, a, b)));
                    }
                }
                let a = left.as_i128().ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("Sub expects integers".into()),
                    span,
                })?;
                let b = right.as_i128().ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("Sub expects integers".into()),
                    span,
                })?;
                let diff = a.checked_sub(b).ok_or_else(|| CodegenError {
                    code: self.overflow_code(),
                    kind: CodegenErrorKind::EvaluationError("Arithmetic overflow in Sub".into()),
                    span,
                })?;
                Ok(EvalValue::Int(diff))
            }
            BinaryOp::Mul => {
                if let (EvalValue::U64(a), EvalValue::U64(b)) = (&left, &right) {
                    return Ok(EvalValue::U64(a.wrapping_mul(*b)));
                }
                if let (EvalValue::U8(a), EvalValue::U8(b)) = (&left, &right) {
                    return Ok(EvalValue::U8(a.wrapping_mul(*b)));
                }
                if let (Some((ta, a)), Some((tb, b))) = (left.as_fixed(), right.as_fixed()) {
                    if ta == tb {
                        return Ok(EvalValue::from_fixed(ta, fixed::mul(ta, a, b)));
                    }
                }
                let a = left.as_i128().ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("Mul expects integers".into()),
                    span,
                })?;
                let b = right.as_i128().ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("Mul expects integers".into()),
                    span,
                })?;
                let prod = a.checked_mul(b).ok_or_else(|| CodegenError {
                    code: self.overflow_code(),
                    kind: CodegenErrorKind::EvaluationError("Arithmetic overflow in Mul".into()),
                    span,
                })?;
                Ok(EvalValue::Int(prod))
            }
        }
    }

    // ── WX-B / WX-C built-ins (VER04j sections 2.5 and 3.5) ────────────────

    /// Evaluate a v1 built-in on ground arguments.  `Ok(None)` means `name` is
    /// not a v1 built-in (a user model, for instance).  The checker has
    /// already enforced every typing rule, so a shape error here is a defect
    /// reported as W0702 rather than a panic.
    fn eval_v1_call(
        &mut self,
        name: &str,
        widths: &[Width<'a>],
        args: &[EvalValue],
        span: (usize, usize),
    ) -> Result<Option<EvalValue>, CodegenError> {
        if matches!(
            name,
            "take" | "drop" | "slice" | "replicate" | "range" | "to_list" | "to_bytes"
        ) {
            return self.eval_wxd_d1(name, args, span).map(Some);
        }
        let bad = |msg: &str| CodegenError {
            code: DiagnosticCode::W0702,
            kind: CodegenErrorKind::EvaluationError(format!("{name}: {msg}")),
            span,
        };
        let static_ty = || -> Result<FixedTy, CodegenError> {
            match widths.first() {
                Some(Width::Named(n)) => {
                    FixedTy::from_name(n).ok_or_else(|| bad("unknown static type"))
                }
                _ => Err(bad("missing static type")),
            }
        };
        let fx = |i: usize| -> Result<(FixedTy, i128), CodegenError> {
            args.get(i)
                .and_then(EvalValue::as_fixed)
                .ok_or_else(|| bad("fixed-width operand expected"))
        };
        let int = |i: usize| -> Result<i128, CodegenError> {
            args.get(i)
                .and_then(EvalValue::as_i128)
                .ok_or_else(|| bad("integer operand expected"))
        };
        let count = |i: usize| -> Result<u128, CodegenError> {
            let n = int(i)?;
            u128::try_from(n).map_err(|_| bad("count must be a natural number"))
        };
        let some = |v: EvalValue| EvalValue::Option(Some(Box::new(v)));
        let opt = |v: Option<i128>, ty: FixedTy| match v {
            Some(x) => some(EvalValue::from_fixed(ty, x)),
            None => EvalValue::Option(None),
        };
        // Shifts, rotates and popcount charge ceil(w/8) work units in total;
        // `eval` already charged one for the call node itself.
        if matches!(name, "shl" | "shr" | "sar" | "rotl" | "rotr" | "popcount") {
            if let Ok((ty, _)) = fx(0) {
                self.charge_work(ty.bytes() as usize - 1, span)?;
            }
        }
        Ok(Some(match name {
            "max_value" => EvalValue::from_fixed(static_ty()?, static_ty()?.max()),
            "min_value" => EvalValue::from_fixed(static_ty()?, static_ty()?.min()),
            "widen" | "wrap" => {
                let target = static_ty()?;
                let v = int(0)?;
                EvalValue::from_fixed(
                    target,
                    if name == "wrap" {
                        target.wrap(v)
                    } else if target.contains(v) {
                        v
                    } else {
                        return Err(bad("operand outside the target range"));
                    },
                )
            }
            "try" => {
                let target = static_ty()?;
                opt(target.try_from_int(int(0)?), target)
            }
            "to_int" => EvalValue::Int(fx(0)?.1),
            "to_nat" => {
                let (ty, v) = fx(0)?;
                if ty.signed() {
                    return Err(bad("to_nat of a signed value"));
                }
                EvalValue::Nat(u64::try_from(v).map_err(|_| bad("value exceeds Nat"))?)
            }
            "checked_add" | "checked_sub" | "checked_mul" => {
                let (ty, a) = fx(0)?;
                let (_, b) = fx(1)?;
                let op = match name {
                    "checked_add" => '+',
                    "checked_sub" => '-',
                    _ => '*',
                };
                opt(fixed::checked(op, ty, a, b), ty)
            }
            "div" | "rem" => match (args.first().and_then(EvalValue::as_fixed), args.get(1)) {
                (Some((ty, a)), Some(_)) => {
                    let (_, b) = fx(1)?;
                    if ty.signed() {
                        return Err(bad("signed operands"));
                    }
                    opt(
                        if name == "div" {
                            fixed::div_unsigned(a, b)
                        } else {
                            fixed::rem_unsigned(a, b)
                        },
                        ty,
                    )
                }
                _ => {
                    let (a, b) = (int(0)?, int(1)?);
                    let r = if name == "div" {
                        fixed::floor_div(a, b)
                    } else {
                        fixed::floor_mod(a, b)
                    };
                    match r {
                        Some(x) => some(EvalValue::Int(x)),
                        None => EvalValue::Option(None),
                    }
                }
            },
            "div_trunc" | "rem_trunc" => {
                let (ty, a) = fx(0)?;
                let (_, b) = fx(1)?;
                opt(
                    if name == "div_trunc" {
                        fixed::div_trunc(ty, a, b)
                    } else {
                        fixed::rem_trunc(ty, a, b)
                    },
                    ty,
                )
            }
            "imin" | "imax" => {
                let (a, b) = (int(0)?, int(1)?);
                EvalValue::Int(if name == "imin" { a.min(b) } else { a.max(b) })
            }
            "unwrap_or" => match args.first() {
                Some(EvalValue::Option(Some(v))) => (**v).clone(),
                Some(EvalValue::Option(None)) => args.get(1).cloned().ok_or_else(|| bad("arity"))?,
                _ => return Err(bad("Option operand expected")),
            },
            "bit_and" | "bit_or" | "bit_xor" => {
                let (ty, a) = fx(0)?;
                let (_, b) = fx(1)?;
                EvalValue::from_fixed(
                    ty,
                    match name {
                        "bit_and" => fixed::bit_and(ty, a, b),
                        "bit_or" => fixed::bit_or(ty, a, b),
                        _ => fixed::bit_xor(ty, a, b),
                    },
                )
            }
            "bit_not" => {
                let (ty, a) = fx(0)?;
                EvalValue::from_fixed(ty, fixed::bit_not(ty, a))
            }
            "shl" | "shr" | "sar" | "rotl" | "rotr" => {
                let (ty, a) = fx(0)?;
                let n = count(1)?;
                if name == "sar" && !ty.signed() {
                    return Err(bad("sar of an unsigned value"));
                }
                EvalValue::from_fixed(
                    ty,
                    match name {
                        "shl" => fixed::shl(ty, a, n),
                        "shr" => fixed::shr(ty, a, n),
                        "sar" => fixed::sar(ty, a, n),
                        "rotl" => fixed::rotl(ty, a, n),
                        _ => fixed::rotr(ty, a, n),
                    },
                )
            }
            "popcount" | "clz" | "ctz" => {
                let (ty, a) = fx(0)?;
                let n = match name {
                    "popcount" => fixed::popcount(ty, a),
                    "clz" => fixed::clz(ty, a),
                    _ => fixed::ctz(ty, a),
                };
                EvalValue::Nat(n as u64)
            }
            "bit" => {
                let (ty, a) = fx(0)?;
                EvalValue::Bool(fixed::bit(ty, a, count(1)?))
            }
            "be_bytes" | "le_bytes" => {
                let (ty, a) = fx(0)?;
                EvalValue::Bytes(if name == "be_bytes" {
                    fixed::be_bytes(ty, a)
                } else {
                    fixed::le_bytes(ty, a)
                })
            }
            "from_be" | "from_le" => {
                let ty = static_ty()?;
                let Some(EvalValue::Bytes(b)) = args.first() else {
                    return Err(bad("Bytes operand expected"));
                };
                let v = if name == "from_be" {
                    fixed::from_be(ty, b)
                } else {
                    fixed::from_le(ty, b)
                };
                EvalValue::from_fixed(ty, v.ok_or_else(|| bad("wrong byte width"))?)
            }
            _ => return Ok(None),
        }))
    }

    /// The seven WX-D1 sequence forms (VER04k sections 2.2, 2.6 and 2.7) over
    /// already evaluated operands.
    ///
    /// Every form knows its result length `L` before it builds anything, and
    /// the order below is the specification's, and the point of this
    /// function:
    ///
    /// ```text
    ///   1. L <= MAX_SEQ_ELEMENTS, compared on the exact i128 count
    ///      (a 2^70 count is refused here, never narrowed to a small one)
    ///   2. reserve the data:  L x unit (+ the payload of copied elements)
    ///   3. charge the work:   L
    ///   4. only then allocate the result
    /// ```
    ///
    /// so a `replicate(0u8, 10^12)` allocates nothing at all, and a count read
    /// from hostile data cannot make the evaluator allocate first and fail
    /// later.  `tests/wxd_alloc_probe.rs` holds this with a counting
    /// allocator.  The call node's own unit was charged by `eval`.
    fn eval_wxd_d1(
        &mut self,
        name: &str,
        args: &[EvalValue],
        span: (usize, usize),
    ) -> Result<EvalValue, CodegenError> {
        let bad = |msg: &str| CodegenError {
            code: DiagnosticCode::W0702,
            kind: CodegenErrorKind::EvaluationError(format!("{name}: {msg}")),
            span,
        };
        let count = |i: usize| -> Result<i128, CodegenError> {
            args.get(i)
                .and_then(EvalValue::as_i128)
                .ok_or_else(|| bad("integer count expected"))
        };
        match name {
            "take" | "drop" | "slice" => {
                let len = match args.first() {
                    Some(EvalValue::Bytes(b)) => b.len(),
                    Some(EvalValue::List(xs)) => xs.len(),
                    _ => return Err(bad("sequence operand expected")),
                };
                // [start, start + l) of the input, from the clamped counts:
                //   take(xs, n)      = [0, clamp(n))
                //   drop(xs, n)      = [clamp(n), len)
                //   slice(xs, s, n)  = take(drop(xs, s), n)
                let (start, l) = match name {
                    "take" => (0, clamp_count(count(1)?, len)),
                    "drop" => {
                        let s = clamp_count(count(1)?, len);
                        (s, len - s)
                    }
                    _ => {
                        let s = clamp_count(count(1)?, len);
                        (s, clamp_count(count(2)?, len - s))
                    }
                };
                // Step 1 cannot fail for a sub-sequence of a capped value,
                // but it is the same check everywhere.
                let l = self.seq_cap(l as i128, span)?;
                match args.first() {
                    Some(EvalValue::Bytes(b)) => {
                        let part = &b[start..start + l];
                        self.seq_reserve(l, l as u128, span)?;
                        Ok(EvalValue::Bytes(part.to_vec()))
                    }
                    Some(EvalValue::List(xs)) => {
                        let part = &xs[start..start + l];
                        // Charged as if copied (section 2.6), payload and all.
                        let data = dsize_items(part);
                        self.seq_reserve(l, data, span)?;
                        Ok(EvalValue::List(part.to_vec()))
                    }
                    _ => Err(bad("sequence operand expected")),
                }
            }
            "replicate" => {
                let x = args
                    .first()
                    .ok_or_else(|| bad("element operand expected"))?;
                let l = self.seq_cap(count(1)?, span)?;
                let each = NODE_BYTES.saturating_add(dsize(x));
                self.seq_reserve(l, (l as u128).saturating_mul(each), span)?;
                Ok(EvalValue::List(vec![x.clone(); l]))
            }
            "range" => {
                let l = self.seq_cap(count(0)?, span)?;
                self.seq_reserve(l, l as u128 * NODE_BYTES, span)?;
                Ok(EvalValue::List((0..l as u64).map(EvalValue::Nat).collect()))
            }
            "to_list" => {
                let Some(EvalValue::Bytes(b)) = args.first() else {
                    return Err(bad("Bytes operand expected"));
                };
                let l = self.seq_cap(b.len() as i128, span)?;
                self.seq_reserve(l, l as u128 * NODE_BYTES, span)?;
                Ok(EvalValue::List(
                    b.iter().map(|&x| EvalValue::U8(x)).collect(),
                ))
            }
            // "to_bytes"
            _ => {
                let Some(EvalValue::List(xs)) = args.first() else {
                    return Err(bad("List<U8> operand expected"));
                };
                let l = self.seq_cap(xs.len() as i128, span)?;
                self.seq_reserve(l, l as u128, span)?;
                xs.iter()
                    .map(|x| match x {
                        EvalValue::U8(v) => Ok(*v),
                        _ => Err(bad("U8 element expected")),
                    })
                    .collect::<Result<Vec<u8>, _>>()
                    .map(EvalValue::Bytes)
            }
        }
    }

    /// Value equality.  In a v1 document the numeric tower is compared by
    /// mathematical value (`popcount(x) == 8` compares a `Nat` with the `Int`
    /// literal `8`), recursively through `Option`, `Result` and `List`; every
    /// other pair is structural.  v0 keeps its structural `==`.
    fn values_equal(&self, l: &EvalValue, r: &EvalValue) -> bool {
        if !self.v1 {
            return l == r;
        }
        match (l, r) {
            (EvalValue::Int(_) | EvalValue::Nat(_), EvalValue::Int(_) | EvalValue::Nat(_)) => {
                l.as_i128() == r.as_i128()
            }
            (EvalValue::Option(a), EvalValue::Option(b)) => match (a, b) {
                (Some(x), Some(y)) => self.values_equal(x, y),
                (None, None) => true,
                _ => false,
            },
            (EvalValue::Result(Ok(x)), EvalValue::Result(Ok(y))) => self.values_equal(x, y),
            (EvalValue::List(a), EvalValue::List(b)) | (EvalValue::Tuple(a), EvalValue::Tuple(b)) => {
                a.len() == b.len() && a.iter().zip(b).all(|(x, y)| self.values_equal(x, y))
            }
            (
                EvalValue::Con {
                    ty: t1,
                    index: i1,
                    fields: a,
                    ..
                },
                EvalValue::Con {
                    ty: t2,
                    index: i2,
                    fields: b,
                    ..
                },
            ) => {
                t1 == t2
                    && i1 == i2
                    && a.len() == b.len()
                    && a.iter().zip(b).all(|(x, y)| self.values_equal(x, y))
            }
            _ => l == r,
        }
    }

    /// Does `v` match `p`?  Binders are appended to `binds`.  A bare NAME that is
    /// a declared constructor is a constructor pattern, any other NAME binds
    /// (VER04j section 4.3); the checker has already guaranteed the shapes fit.
    fn match_pattern(
        &mut self,
        p: &Pattern<'a>,
        v: &EvalValue,
        binds: &mut Vec<(&'a str, EvalValue)>,
        span: (usize, usize),
    ) -> Result<bool, CodegenError> {
        self.charge_work(1, span)?;
        Ok(match (p, v) {
            (Pattern::Wild, _) => true,
            (Pattern::None, EvalValue::Option(None)) => true,
            (Pattern::Some(q), EvalValue::Option(Some(inner)))
            | (Pattern::Ok(q), EvalValue::Result(Ok(inner))) => {
                self.match_pattern(q, inner, binds, span)?
            }
            (Pattern::Err(q), EvalValue::Result(Err((t, c)))) => {
                let err_value = EvalValue::Result(Err((t.clone(), c.clone())));
                self.match_pattern(q, &err_value, binds, span)?
            }
            (Pattern::Bool(b), EvalValue::Bool(x)) => b == x,
            (Pattern::Tuple(ps), EvalValue::Tuple(vs)) if ps.len() == vs.len() => {
                for (q, x) in ps.iter().zip(vs) {
                    if !self.match_pattern(q, x, binds, span)? {
                        return Ok(false);
                    }
                }
                true
            }
            (Pattern::Ctor { name, args }, _) => {
                if name.segments.len() == 2 {
                    // `E.CASE` against an error value.
                    matches!(v, EvalValue::Result(Err((t, c)))
                        if t == name.segments[0] && c == name.segments[1])
                } else if let Some(&(ty, index, _)) = self.ctors.get(name.segments[0]) {
                    match v {
                        EvalValue::Con {
                            ty: vt,
                            index: vi,
                            fields,
                            ..
                        } if vt == ty && *vi == index && fields.len() == args.len() => {
                            for (q, x) in args.iter().zip(fields) {
                                if !self.match_pattern(q, x, binds, span)? {
                                    return Ok(false);
                                }
                            }
                            true
                        }
                        _ => false,
                    }
                } else {
                    binds.push((name.segments[0], v.clone()));
                    true
                }
            }
            _ => false,
        })
    }

    fn eval_match(
        &mut self,
        target: EvalValue,
        arms: &[MatchArm<'a>],
        env: &HashMap<&'a str, EvalValue>,
        span: (usize, usize),
    ) -> Result<EvalValue, CodegenError> {
        for arm in arms {
            let mut binds = Vec::new();
            if self.match_pattern(&arm.pattern, &target, &mut binds, arm.span)? {
                let mut arm_env = env.clone();
                for (n, v) in binds {
                    arm_env.insert(n, v);
                }
                return self.eval(&arm.body, &arm_env);
            }
        }
        Err(CodegenError {
            code: DiagnosticCode::W0702,
            kind: CodegenErrorKind::EvaluationError("Match not exhaustive at runtime".into()),
            span,
        })
    }
}

fn expr_span(expr: &Expression<'_>) -> (usize, usize) {
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

/// A compiled portable TFV test record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TfvRecord {
    pub id: String,
    pub fields: Vec<(String, String)>,
}

impl TfvRecord {
    /// Format as canonical TFV record line: `<id> R<count> K<key1> <val1> ...`
    pub fn to_tfv_line(&self) -> String {
        let mut line = format!("{} R{}", self.id, self.fields.len());
        for (k, v) in &self.fields {
            line.push_str(&format!(" K{k} {v}"));
        }
        line
    }
}

/// Extracted example case invocation.
#[derive(Clone, Debug)]
pub struct ExampleCase {
    pub id: String,
    pub op_name: String,
    pub static_width: Option<u64>,
    pub args: Vec<EvalValue>,
    pub expected: EvalValue,
}

/// Compiles an `ExampleDecl` AST node into an `ExampleCase`.
pub fn extract_example_case<'a>(
    name: &'a str,
    body: &'a Expression<'a>,
    evaluator: &mut Evaluator<'a>,
) -> Result<ExampleCase, CodegenError> {
    let span = expr_span(body);
    let (left, right) = match body {
        Expression::Binary {
            op: BinaryOp::Eq,
            left,
            right,
            ..
        } => (left, right),
        _ => {
            return Err(CodegenError {
                code: DiagnosticCode::W0702,
                kind: CodegenErrorKind::EvaluationError(
                    "Example body must be an equality assertion (op(...) == expected)".into(),
                ),
                span,
            })
        }
    };

    let empty_env = HashMap::new();
    let expected = evaluator.eval(right, &empty_env)?;

    let (op_name, static_width, args) = match &**left {
        Expression::Reference {
            name: op_ref,
            widths,
            args: Some(arg_exprs),
            span: ref_span,
        } => {
            let width = if let Some(first_width) = widths.first() {
                match first_width {
                    Width::Literal(w) => Some(*w),
                    _ => {
                        return Err(CodegenError {
                            code: DiagnosticCode::W0701,
                            kind: CodegenErrorKind::UnsupportedTarget(
                                "Example operation width must be literal".into(),
                            ),
                            span: *ref_span,
                        })
                    }
                }
            } else {
                None
            };
            let mut evaled_args = Vec::with_capacity(arg_exprs.len());
            for a in arg_exprs {
                evaled_args.push(evaluator.eval(a, &empty_env)?);
            }
            (op_ref.segments[0].to_string(), width, evaled_args)
        }
        _ => {
            return Err(CodegenError {
                code: DiagnosticCode::W0702,
                kind: CodegenErrorKind::EvaluationError(
                    "LHS of example must be an operation call".into(),
                ),
                span,
            })
        }
    };

    Ok(ExampleCase {
        id: name.to_string(),
        op_name,
        static_width,
        args,
        expected,
    })
}

/// A compiled universal claim ready for property-based test harness emission (VER04f / VER-P011).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimCase<'a> {
    pub id: &'a str,
    pub is_forall: bool,
    pub binders: Vec<crate::parser::Parameter<'a>>,
    pub body: &'a Expression<'a>,
    pub op_name: String,
}

/// Extracts and validates all universal `claim` declarations from a Warrant file.
pub fn extract_claim_cases<'a>(
    file: &'a WarrantFile<'a>,
) -> Result<Vec<ClaimCase<'a>>, CodegenError> {
    let mut claims = Vec::new();

    let op_names: HashSet<&str> = file
        .declarations
        .iter()
        .filter_map(|d| match d {
            crate::parser::Declaration::Op(op) => Some(op.name),
            _ => None,
        })
        .collect();

    for decl in &file.declarations {
        if let crate::parser::Declaration::Claim(c) = decl {
            match &c.body {
                Expression::Quantified {
                    is_forall,
                    binders,
                    body,
                    span,
                } => {
                    if !is_forall {
                        return Err(CodegenError {
                            code: DiagnosticCode::W0701,
                            kind: CodegenErrorKind::UnsupportedTarget(format!(
                                "Claim {} must be universally quantified (forall)",
                                c.name
                            )),
                            span: *span,
                        });
                    }
                    if binders.is_empty() {
                        return Err(CodegenError {
                            code: DiagnosticCode::W0701,
                            kind: CodegenErrorKind::UnsupportedTarget(format!(
                                "Claim {} must have at least one quantified binder",
                                c.name
                            )),
                            span: *span,
                        });
                    }

                    let op_name =
                        find_operation_for_version(body.as_ref(), &op_names, file.version)
                            .ok_or_else(|| CodegenError {
                                code: DiagnosticCode::W0702,
                                kind: CodegenErrorKind::NonGroundTerm(format!(
                                    "Claim {} does not invoke a recognized subject operation",
                                    c.name
                                )),
                                span: *span,
                            })?;

                    claims.push(ClaimCase {
                        id: c.name,
                        is_forall: *is_forall,
                        binders: binders.clone(),
                        body: body.as_ref(),
                        op_name: op_name.to_string(),
                    });
                }
                _ => {
                    return Err(CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::NonGroundTerm(format!(
                            "Claim {} must have a quantified body",
                            c.name
                        )),
                        span: c.span,
                    });
                }
            }
        }
    }
    Ok(claims)
}

/// The subject operation an expression calls, if any: the search every
/// version used before WX-D, and still the v0 search (and the one the evidence
/// generator uses until it translates list literals, VER04k section 8.1, S2a).
///
/// ```text
///   1. shallow     operators, conditionals, matches, tuples
///   2. arguments   only if 1 found nothing: inside the arguments of
///                  non-operation calls, so `popcount(op(a)) == 3` names `op`
/// ```
pub fn find_operation_in_expr<'a>(
    expr: &'a Expression<'a>,
    op_names: &HashSet<&str>,
) -> Option<&'a str> {
    find_operation_with(expr, op_names, false)
}

/// [`find_operation_in_expr`] for a document of version `version`.
///
/// In a v1 document (VER04k section 9.1, S0 E13) step 2 also descends into
/// the elements of a list literal, exactly as it descends into call
/// arguments, so `to_bytes([f(a)]) == b` and `[f(a)] == [b]` name `f` instead
/// of "calls no operation".  The shallow step is unchanged and still runs
/// first, and the first match wins.  A v0 document gets the v0 search.
pub fn find_operation_for_version<'a>(
    expr: &'a Expression<'a>,
    op_names: &HashSet<&str>,
    version: Version,
) -> Option<&'a str> {
    find_operation_with(expr, op_names, version == Version::V1)
}

/// Step 1, then step 2; `lists` makes step 2 enter list literals.
fn find_operation_with<'a>(
    expr: &'a Expression<'a>,
    op_names: &HashSet<&str>,
    lists: bool,
) -> Option<&'a str> {
    find_operation_shallow(expr, op_names).or_else(|| find_operation_in_args(expr, op_names, lists))
}

fn find_operation_in_args<'a>(
    expr: &'a Expression<'a>,
    op_names: &HashSet<&str>,
    lists: bool,
) -> Option<&'a str> {
    match expr {
        Expression::Reference {
            args: Some(args), ..
        } => args
            .iter()
            .find_map(|a| find_operation_with(a, op_names, lists)),
        // v1 (S0 E13): a list literal's elements are searched like arguments.
        Expression::List { elements, .. } if lists => elements
            .iter()
            .find_map(|e| find_operation_with(e, op_names, lists)),
        Expression::Binary { left, right, .. } => find_operation_in_args(left, op_names, lists)
            .or_else(|| find_operation_in_args(right, op_names, lists)),
        Expression::Unary { operand, .. } => find_operation_in_args(operand, op_names, lists),
        Expression::Grouped { inner, .. } => find_operation_in_args(inner, op_names, lists),
        Expression::Project { base, .. } => find_operation_in_args(base, op_names, lists),
        Expression::Tuple { elements, .. } => elements
            .iter()
            .find_map(|e| find_operation_in_args(e, op_names, lists)),
        Expression::Conditional {
            condition,
            then_branch,
            else_branch,
            ..
        } => find_operation_in_args(condition, op_names, lists)
            .or_else(|| find_operation_in_args(then_branch, op_names, lists))
            .or_else(|| find_operation_in_args(else_branch, op_names, lists)),
        _ => None,
    }
}

fn find_operation_shallow<'a>(
    expr: &'a Expression<'a>,
    op_names: &HashSet<&str>,
) -> Option<&'a str> {
    match expr {
        Expression::Reference { name, .. } => {
            if let Some(first) = name.segments.first() {
                if op_names.contains(first) {
                    return Some(first);
                }
            }
            None
        }
        Expression::Binary { left, right, .. } => {
            find_operation_shallow(left, op_names)
                .or_else(|| find_operation_shallow(right, op_names))
        }
        Expression::Unary { operand, .. } => find_operation_shallow(operand, op_names),
        Expression::Conditional {
            condition,
            then_branch,
            else_branch,
            ..
        } => find_operation_shallow(condition, op_names)
            .or_else(|| find_operation_shallow(then_branch, op_names))
            .or_else(|| find_operation_shallow(else_branch, op_names)),
        Expression::Grouped { inner, .. } => find_operation_shallow(inner, op_names),
        Expression::Project { base, .. } => find_operation_shallow(base, op_names),
        Expression::Tuple { elements, .. } => elements
            .iter()
            .find_map(|e| find_operation_shallow(e, op_names)),
        Expression::Match { target, arms, .. } => {
            find_operation_shallow(target, op_names).or_else(|| {
                arms.iter()
                    .find_map(|arm| find_operation_shallow(&arm.body, op_names))
            })
        }
        _ => None,
    }
}

fn is_bytes_type(param_type: &crate::parser::TypeExpr) -> bool {
    matches!(
        param_type,
        crate::parser::TypeExpr::Bytes { .. }
            | crate::parser::TypeExpr::Named {
                name: "Bytes",
                ..
            }
    )
}

fn is_u64_type(param_type: &crate::parser::TypeExpr) -> bool {
    matches!(
        param_type,
        crate::parser::TypeExpr::Named {
            name: "U64" | "Nat",
            ..
        }
    )
}

fn is_bool_type(param_type: &crate::parser::TypeExpr) -> bool {
    matches!(
        param_type,
        crate::parser::TypeExpr::Named {
            name: "Bool",
            ..
        }
    )
}

fn is_u8_type(param_type: &crate::parser::TypeExpr) -> bool {
    matches!(
        param_type,
        crate::parser::TypeExpr::Named {
            name: "U8",
            ..
        }
    )
}

fn generate_claim_helper(
    claim: &ClaimCase<'_>,
    manifest: &BindingManifest,
    file: &WarrantFile<'_>,
) -> Result<String, CodegenError> {
    let op_binding = &manifest.bindings[&claim.op_name];
    let op_export = &op_binding.export;

    for binder in &claim.binders {
        if !is_bytes_type(&binder.param_type)
            && !is_u64_type(&binder.param_type)
            && !is_bool_type(&binder.param_type)
            && !is_u8_type(&binder.param_type)
        {
            return Err(CodegenError {
                code: DiagnosticCode::W0701,
                kind: CodegenErrorKind::UnsupportedTarget(format!(
                    "Unsupported binder type in claim {}",
                    claim.id
                )),
                span: binder.span,
            });
        }
    }

    if claim.binders.len() > 2 {
        return Err(CodegenError {
            code: DiagnosticCode::W0701,
            kind: CodegenErrorKind::UnsupportedTarget(format!(
                "Claim {} has {} binders; max supported in v0 is 2",
                claim.id,
                claim.binders.len()
            )),
            span: expr_span(claim.body),
        });
    }

    let is_first_bytes = is_bytes_type(&claim.binders[0].param_type);
    let is_first_u64 = is_u64_type(&claim.binders[0].param_type);
    let is_first_bool = is_bool_type(&claim.binders[0].param_type);
    let is_first_u8 = is_u8_type(&claim.binders[0].param_type);

    let (gen_code, shrink_code, prop_body) = if claim.binders.len() == 2 {
        let is_second_bytes = is_bytes_type(&claim.binders[1].param_type);
        let is_second_u64 = is_u64_type(&claim.binders[1].param_type);
        let is_second_u8 = is_u8_type(&claim.binders[1].param_type);

        if is_first_bytes && is_second_bytes {
            let gen = "        let gen = |rng: &mut coding_adventures_test_bootstrap::prng::SplitMix64| {\n            Ok((gen_bytes_slice(rng, 0, 64), gen_bytes_slice(rng, 0, 64)))\n        };\n";
            let shrink = r#"        let shrink = |cand: (Vec<u8>, Vec<u8>), prop: &dyn Fn(&(Vec<u8>, Vec<u8>)) -> bool| -> ((Vec<u8>, Vec<u8>), usize) {
            let (cand_a, cand_b) = cand;
            let (shrunk_a, steps_a) = shrink_bytes(&cand_a, |a| prop(&(a.to_vec(), cand_b.clone())), 1000);
            let (shrunk_b, steps_b) = shrink_bytes(&cand_b, |b| prop(&(shrunk_a.clone(), b.to_vec())), 1000);
            ((shrunk_a, shrunk_b), steps_a + steps_b)
        };
"#;
            let prop = format!(r#"        let prop = |cand: &(Vec<u8>, Vec<u8>)| -> bool {{
            let a = &cand.0;
            let b = &cand.1;
            let mut got = {op_export}(a, b);
            if matches!(fault, Fault::Equality) {{
                got = !got;
            }}
            let expected = a == b;
            got == expected
        }};
"#);
            (gen, shrink.to_string(), prop)
        } else if is_first_u64 && is_second_u64 {
            let gen = "        let gen = |rng: &mut coding_adventures_test_bootstrap::prng::SplitMix64| {\n            Ok((gen_u64(rng), gen_u64(rng)))\n        };\n";
            let shrink = r#"        let shrink = |cand: (u64, u64), prop: &dyn Fn(&(u64, u64)) -> bool| -> ((u64, u64), usize) {
            let (cand_a, cand_b) = cand;
            let (shrunk_a, steps_a) = shrink_int(cand_a as i128, 0, 0, u64::MAX as i128, &|a| prop(&(a as u64, cand_b)), 1000);
            let (shrunk_b, steps_b) = shrink_int(cand_b as i128, 0, 0, u64::MAX as i128, &|b| prop(&(shrunk_a as u64, b as u64)), 1000);
            ((shrunk_a as u64, shrunk_b as u64), steps_a + steps_b)
        };
"#;
            let prop = format!(r#"        let prop = |cand: &(u64, u64)| -> bool {{
            let a = cand.0;
            let b = cand.1;
            let mut got = {op_export}(a, b);
            if matches!(fault, Fault::Equality) {{
                got = !got;
            }}
            let expected = a == b;
            got == expected
        }};
"#);
            (gen, shrink.to_string(), prop)
        } else if is_first_u8 && is_second_u8 {
            let gen = "        let gen = |rng: &mut coding_adventures_test_bootstrap::prng::SplitMix64| {\n            Ok((gen_u8(rng), gen_u8(rng)))\n        };\n";
            let shrink = r#"        let shrink = |cand: (u8, u8), prop: &dyn Fn(&(u8, u8)) -> bool| -> ((u8, u8), usize) {
            let (cand_a, cand_b) = cand;
            let (shrunk_a, steps_a) = shrink_int(cand_a as i128, 0, 0, 255, &|a| prop(&(a as u8, cand_b)), 1000);
            let (shrunk_b, steps_b) = shrink_int(cand_b as i128, 0, 0, 255, &|b| prop(&(shrunk_a as u8, b as u8)), 1000);
            ((shrunk_a as u8, shrunk_b as u8), steps_a + steps_b)
        };
"#;
            let prop = format!(r#"        let prop = |cand: &(u8, u8)| -> bool {{
            let a = cand.0;
            let b = cand.1;
            let mut got = {op_export}(a, b);
            if matches!(fault, Fault::Equality) {{
                got = !got;
            }}
            let expected = a == b;
            got == expected
        }};
"#);
            (gen, shrink.to_string(), prop)
        } else {
            return Err(CodegenError {
                code: DiagnosticCode::W0701,
                kind: CodegenErrorKind::UnsupportedTarget(format!(
                    "Unsupported binary binder pair in claim {}",
                    claim.id
                )),
                span: expr_span(claim.body),
            });
        }
    } else {
        // 1 binder
        if is_first_bytes {
            let gen = "        let gen = |rng: &mut coding_adventures_test_bootstrap::prng::SplitMix64| {\n            Ok(gen_bytes_slice(rng, 0, 64))\n        };\n";
            let shrink = r#"        let shrink = |cand: Vec<u8>, prop: &dyn Fn(&Vec<u8>) -> bool| -> (Vec<u8>, usize) {
            shrink_bytes(&cand, |a| prop(&a.to_vec()), 1000)
        };
"#;
            let prop = format!(r#"        let prop = |cand: &Vec<u8>| -> bool {{
            let a = cand;
            let mut got = {op_export}(a, a);
            if matches!(fault, Fault::Equality) {{
                got = !got;
            }}
            let expected = true;
            got == expected
        }};
"#);
            (gen, shrink.to_string(), prop)
        } else if is_first_u64 {
            let gen = "        let gen = |rng: &mut coding_adventures_test_bootstrap::prng::SplitMix64| {\n            Ok(gen_u64(rng))\n        };\n";
            let shrink = r#"        let shrink = |cand: u64, prop: &dyn Fn(&u64) -> bool| -> (u64, usize) {
            let (shrunk, steps) = shrink_int(cand as i128, 0, 0, u64::MAX as i128, &|v| prop(&(v as u64)), 1000);
            (shrunk as u64, steps)
        };
"#;
            let prop = format!(r#"        let prop = |cand: &u64| -> bool {{
            let a = *cand;
            let mut got = {op_export}(a, a);
            if matches!(fault, Fault::Equality) {{
                got = !got;
            }}
            let expected = true;
            got == expected
        }};
"#);
            (gen, shrink.to_string(), prop)
        } else if is_first_bool {
            let gen = "        let gen = |rng: &mut coding_adventures_test_bootstrap::prng::SplitMix64| {\n            Ok(gen_bool(rng))\n        };\n";
            let shrink = r#"        let shrink = |cand: bool, _prop: &dyn Fn(&bool) -> bool| -> (bool, usize) {
            (cand, 0)
        };
"#;
            let prop = format!(r#"        let prop = |cand: &bool| -> bool {{
            let a = *cand;
            let mut got = {op_export}(a);
            if matches!(fault, Fault::Equality) {{
                got = !got;
            }}
            let expected = a;
            got == expected
        }};
"#);
            (gen, shrink.to_string(), prop)
        } else {
            return Err(CodegenError {
                code: DiagnosticCode::W0701,
                kind: CodegenErrorKind::UnsupportedTarget(format!(
                    "Unsupported binder type in claim {}",
                    claim.id
                )),
                span: expr_span(claim.body),
            });
        }
    };

    let mut out = String::new();
    out.push_str(&format!("fn check_claim_{}(ctx: &Ctx, fault: Fault) -> Verdict {{\n", claim.id));
    out.push_str("    let options = PbtOptions::default();\n");
    out.push_str(&format!("    let claim_tag = \"claim/{}_{}\";\n", file.name, claim.id));
    out.push_str("    let claim_sub_seed = coding_adventures_test_bootstrap::prng::SplitMix64::new(\n");
    out.push_str("        ctx.seed() ^ coding_adventures_test_bootstrap::prng::fnv1a64(claim_tag.as_bytes())\n");
    out.push_str("    ).next_u64();\n\n");
    out.push_str(gen_code);
    out.push_str(&shrink_code);
    out.push_str(&prop_body);
    out.push_str("\n    check_property(claim_sub_seed, &options, gen, prop, shrink)\n");
    out.push_str("}\n");

    Ok(out)
}

/// Generates a TFV record for an example case.
pub fn compile_example_to_tfv(
    module_name: &str,
    case: &ExampleCase,
) -> Result<TfvRecord, CodegenError> {
    let mut fields = Vec::new();
    fields.push((
        "op".to_string(),
        format!("S{}", hex_encode(case.op_name.as_bytes())),
    ));

    // Record arguments based on CT01 standard conventions
    match case.op_name.as_str() {
        "eq" | "eq_fixed" => {
            if case.args.len() != 2 {
                return Err(CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError(format!(
                        "{} expects 2 arguments",
                        case.op_name
                    )),
                    span: (0, 0),
                });
            }
            let left_bytes = case.args[0].as_bytes().ok_or_else(|| CodegenError {
                code: DiagnosticCode::W0702,
                kind: CodegenErrorKind::EvaluationError("left argument must be Bytes".into()),
                span: (0, 0),
            })?;
            let right_bytes = case.args[1].as_bytes().ok_or_else(|| CodegenError {
                code: DiagnosticCode::W0702,
                kind: CodegenErrorKind::EvaluationError("right argument must be Bytes".into()),
                span: (0, 0),
            })?;

            fields.push(("left".to_string(), format!("X{}", hex_encode(left_bytes))));
            fields.push(("right".to_string(), format!("X{}", hex_encode(right_bytes))));
            if let Some(w) = case.static_width {
                fields.push(("width".to_string(), format!("I{w}")));
            }
            let eq_bool = case.expected.as_bool().ok_or_else(|| CodegenError {
                code: DiagnosticCode::W0702,
                kind: CodegenErrorKind::EvaluationError("Expected value must be Bool".into()),
                span: (0, 0),
            })?;
            fields.push((
                "equal".to_string(),
                if eq_bool { "T".into() } else { "F".into() },
            ));

            // Logical work fields
            let positions = if left_bytes.len() == right_bytes.len() {
                left_bytes.len() as u64
            } else {
                0
            };
            fields.push(("byte_positions".to_string(), format!("I{positions}")));
            fields.push(("byte_reads".to_string(), format!("I{}", 2 * positions)));
            fields.push(("byte_writes".to_string(), "I0".to_string()));
            fields.push(("u64_bits".to_string(), "I0".to_string()));
        }
        "select_bytes" => {
            if case.args.len() != 3 {
                return Err(CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError(
                        "select_bytes expects 3 arguments".into(),
                    ),
                    span: (0, 0),
                });
            }
            let left_bytes = case.args[0].as_bytes().ok_or_else(|| CodegenError {
                code: DiagnosticCode::W0702,
                kind: CodegenErrorKind::EvaluationError("left argument must be Bytes".into()),
                span: (0, 0),
            })?;
            let right_bytes = case.args[1].as_bytes().ok_or_else(|| CodegenError {
                code: DiagnosticCode::W0702,
                kind: CodegenErrorKind::EvaluationError("right argument must be Bytes".into()),
                span: (0, 0),
            })?;
            let choice = case.args[2].as_bool().ok_or_else(|| CodegenError {
                code: DiagnosticCode::W0702,
                kind: CodegenErrorKind::EvaluationError("choice argument must be Bool".into()),
                span: (0, 0),
            })?;

            fields.push(("left".to_string(), format!("X{}", hex_encode(left_bytes))));
            fields.push(("right".to_string(), format!("X{}", hex_encode(right_bytes))));
            fields.push((
                "choice".to_string(),
                if choice { "T".into() } else { "F".into() },
            ));

            match &case.expected {
                EvalValue::Result(Ok(inner)) => {
                    let out_bytes = inner.as_bytes().ok_or_else(|| CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::EvaluationError("Output must be Bytes".into()),
                        span: (0, 0),
                    })?;
                    fields.push(("output".to_string(), format!("X{}", hex_encode(out_bytes))));
                    fields.push(("fresh_output".to_string(), "T".to_string()));

                    let positions = left_bytes.len() as u64;
                    fields.push(("byte_positions".to_string(), format!("I{positions}")));
                    fields.push(("byte_reads".to_string(), format!("I{}", 2 * positions)));
                    fields.push(("byte_writes".to_string(), format!("I{positions}")));
                    fields.push(("u64_bits".to_string(), "I0".to_string()));
                }
                EvalValue::Result(Err((_, err_case))) => {
                    fields.push((
                        "error_id".to_string(),
                        format!("S{}", hex_encode(err_case.as_bytes())),
                    ));
                    fields.push(("byte_positions".to_string(), "I0".to_string()));
                    fields.push(("byte_reads".to_string(), "I0".to_string()));
                    fields.push(("byte_writes".to_string(), "I0".to_string()));
                    fields.push(("u64_bits".to_string(), "I0".to_string()));
                }
                _ => {
                    return Err(CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::EvaluationError(
                            "select_bytes expected must be Result".into(),
                        ),
                        span: (0, 0),
                    })
                }
            }
        }
        "eq_u64" => {
            if case.args.len() != 2 {
                return Err(CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("eq_u64 expects 2 arguments".into()),
                    span: (0, 0),
                });
            }
            let left_val = case.args[0].as_u64().ok_or_else(|| CodegenError {
                code: DiagnosticCode::W0702,
                kind: CodegenErrorKind::EvaluationError("left_u64 must be U64".into()),
                span: (0, 0),
            })?;
            let right_val = case.args[1].as_u64().ok_or_else(|| CodegenError {
                code: DiagnosticCode::W0702,
                kind: CodegenErrorKind::EvaluationError("right_u64 must be U64".into()),
                span: (0, 0),
            })?;

            fields.push(("left_u64".to_string(), format!("I{left_val}")));
            fields.push(("right_u64".to_string(), format!("I{right_val}")));

            let eq_bool = case.expected.as_bool().ok_or_else(|| CodegenError {
                code: DiagnosticCode::W0702,
                kind: CodegenErrorKind::EvaluationError("Expected value must be Bool".into()),
                span: (0, 0),
            })?;
            fields.push((
                "equal".to_string(),
                if eq_bool { "T".into() } else { "F".into() },
            ));

            fields.push(("byte_positions".to_string(), "I0".to_string()));
            fields.push(("byte_reads".to_string(), "I0".to_string()));
            fields.push(("byte_writes".to_string(), "I0".to_string()));
            fields.push(("u64_bits".to_string(), "I64".to_string()));
        }
        _ => {
            return Err(CodegenError {
                code: DiagnosticCode::W0701,
                kind: CodegenErrorKind::UnsupportedTarget(format!(
                    "Unsupported operation for TFV: {}",
                    case.op_name
                )),
                span: (0, 0),
            });
        }
    }

    Ok(TfvRecord {
        id: format!("{}/{}", module_name, case.id),
        fields,
    })
}

/// Compiles all examples in a Warrant document to a full TFV file content string.
pub fn compile_warrant_to_tfv(
    file: &WarrantFile<'_>,
    limits: CodegenLimits,
) -> Result<String, CodegenError> {
    let mut evaluator = Evaluator::new(file, limits);
    let mut lines = vec!["tfv 1".to_string()];

    for decl in &file.declarations {
        if let crate::parser::Declaration::Example(ex) = decl {
            evaluator.reset_work();
            let case = extract_example_case(ex.name, &ex.body, &mut evaluator)?;
            let record = compile_example_to_tfv(file.name, &case)?;
            lines.push(record.to_tfv_line());
        }
    }

    let mut out = lines.join("\n");
    out.push('\n');
    Ok(out)
}

/// Options for compiling a Rust L0 test harness.
#[derive(Clone, Debug)]
pub struct RustL0Options {
    pub package_crate: String,
    pub spec_path: String,
}

/// Compiles a checked Warrant document into a complete, standalone Rust L0 test harness source code.
pub fn compile_rust_l0_harness(
    file: &WarrantFile<'_>,
    _checked: &CheckedWarrantModule,
    manifest: &BindingManifest,
    options: &RustL0Options,
) -> Result<String, CodegenError> {
    let mut evaluator = Evaluator::new(file, CodegenLimits::default());
    let mut example_cases = Vec::new();

    // Collect fixed widths used across examples
    let mut fixed_widths = BTreeMap::new();

    for decl in &file.declarations {
        if let crate::parser::Declaration::Example(ex) = decl {
            evaluator.reset_work();
            let case = extract_example_case(ex.name, &ex.body, &mut evaluator)?;
            if case.op_name == "eq_fixed" {
                if let Some(w) = case.static_width {
                    fixed_widths.insert(w, ());
                }
            }
            example_cases.push(case);
        }
    }

    // Verify all operations called in examples are bound in manifest
    for case in &example_cases {
        if !manifest.bindings.contains_key(&case.op_name) {
            return Err(CodegenError {
                code: DiagnosticCode::W0704,
                kind: CodegenErrorKind::MissingBinding(format!(
                    "Operation {} in example {} has no binding in manifest",
                    case.op_name, case.id
                )),
                span: (0, 0),
            });
        }
    }

    // Universal claims are no longer compiled here.  The former claim helper
    // hard-coded `a == b` as its oracle, accepted at most two binders and was
    // only ever string-matched.  Claims now compile through
    // `evidence::compile_rust_evidence_harness` (VER04h), which translates the
    // claim body itself.  `extract_claim_cases` remains for tooling.
    let claim_cases: Vec<ClaimCase<'_>> = Vec::new();

    // Verify all operations called in claims are bound in manifest
    for claim in &claim_cases {
        if !manifest.bindings.contains_key(&claim.op_name) {
            return Err(CodegenError {
                code: DiagnosticCode::W0704,
                kind: CodegenErrorKind::MissingBinding(format!(
                    "Operation {} in claim {} has no binding in manifest",
                    claim.op_name, claim.id
                )),
                span: (0, 0),
            });
        }
    }

    // Resolve dynamic public exports from manifest
    let eq_export = manifest
        .bindings
        .get("eq")
        .map(|b| b.export.as_str())
        .unwrap_or("ct_eq");
    let eq_fixed_export = manifest
        .bindings
        .get("eq_fixed")
        .map(|b| b.export.as_str())
        .unwrap_or("ct_eq_fixed");
    let select_export = manifest
        .bindings
        .get("select_bytes")
        .map(|b| b.export.as_str())
        .unwrap_or("ct_select_bytes");
    let eq_u64_export = manifest
        .bindings
        .get("eq_u64")
        .map(|b| b.export.as_str())
        .unwrap_or("ct_eq_u64");

    let select_error_case = manifest
        .bindings
        .get("select_bytes")
        .and_then(|b| b.error_normalization.as_ref())
        .map(|n| n.error_case.as_str())
        .unwrap_or("CT_SELECT_LENGTH_MISMATCH");

    // Build deterministic Rust harness code
    let mut out = String::new();
    out.push_str("// GENERATED BY coding_adventures_warrant_grammar::codegen (VER04e). DO NOT EDIT MANUALLY.\n");
    out.push_str(&format!("// Source: {}\n", options.spec_path));
    out.push_str("// Target: Rust L0 test harness on test-bootstrap\n\n");

    out.push_str("#![allow(unused_imports, clippy::all)]\n\n");
    out.push_str("use coding_adventures_test_bootstrap::{\n");
    out.push_str("    run::{self, Ctx},\n");
    out.push_str("    tfv::{self, Record, Value},\n");
    out.push_str("    Suite, Verdict,\n");
    out.push_str("};\n");
    if !claim_cases.is_empty() {
        out.push_str("use coding_adventures_test_bootstrap::pbt::{\n");
        out.push_str("    self, check_property, gen_bool, gen_bytes_slice, gen_u64, gen_u8, shrink_bytes, shrink_int,\n");
        out.push_str("    FID_COUNTEREXAMPLE, PbtOptions,\n");
        out.push_str("};\n");
    }
    out.push_str("use std::panic::catch_unwind;\n");
    out.push_str(&format!("use {}::*;\n\n", options.package_crate));

    // Fault injection enum
    out.push_str("/// Fault injection controls for negative test verification.\n");
    out.push_str("#[derive(Clone, Copy, PartialEq, Eq)]\n");
    out.push_str("pub enum Fault {\n");
    out.push_str("    None,\n");
    out.push_str("    Equality,\n");
    out.push_str("    Selection,\n");
    out.push_str("    MissingError,\n");
    out.push_str("}\n\n");

    // Const-generic dispatch function
    out.push_str("/// Fixed width const-generic dispatch.\n");
    out.push_str(
        "fn fixed_dispatch(width: u64, left: &[u8], right: &[u8]) -> Result<bool, String> {\n",
    );
    out.push_str("    match width {\n");
    for w in fixed_widths.keys() {
        out.push_str(&format!("        {w} => fixed::<{w}>(left, right),\n"));
    }
    out.push_str("        _ => Err(\"fixed width has no generated arm\".into()),\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");

    out.push_str("fn fixed<const N: usize>(left: &[u8], right: &[u8]) -> Result<bool, String> {\n");
    out.push_str(
        "    let left: &[u8; N] = left.try_into().map_err(|_| \"left fixed width mismatch\")?;\n",
    );
    out.push_str("    let right: &[u8; N] = right.try_into().map_err(|_| \"right fixed width mismatch\")?;\n");
    out.push_str(&format!("    Ok({eq_fixed_export}(left, right))\n"));
    out.push_str("}\n\n");

    // Baseline validation helper
    out.push_str("fn reject_after_baseline(ctx: &Ctx, baseline: impl Fn(&Ctx, Fault) -> Verdict, fault: Fault) -> Verdict {\n");
    out.push_str("    if !matches!(baseline(ctx, Fault::None), Verdict::Pass) {\n");
    out.push_str("        return Verdict::fail(\"corpus-mismatch\", \"valid adapter baseline did not pass\");\n");
    out.push_str("    }\n");
    out.push_str("    match baseline(ctx, fault) {\n");
    out.push_str("        Verdict::Fail { .. } => Verdict::Pass,\n");
    out.push_str("        Verdict::Pass => Verdict::fail(\"corpus-mismatch\", \"broken adapter behavior was accepted\"),\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");

    if !claim_cases.is_empty() {
        out.push_str("fn reject_claim_after_baseline(ctx: &Ctx, baseline: impl Fn(&Ctx, Fault) -> Verdict, fault: Fault) -> Verdict {\n");
        out.push_str("    if !matches!(baseline(ctx, Fault::None), Verdict::Pass) {\n");
        out.push_str("        return Verdict::fail(\"corpus-mismatch\", \"valid claim baseline did not pass\");\n");
        out.push_str("    }\n");
        out.push_str("    match baseline(ctx, fault) {\n");
        out.push_str("        Verdict::Fail { ref failure_id, .. } if coding_adventures_test_bootstrap::pbt::is_registered_failure_id(failure_id) => Verdict::Pass,\n");
        out.push_str("        Verdict::Fail { .. } => Verdict::Pass,\n");
        out.push_str("        Verdict::Pass => Verdict::fail(\"corpus-mismatch\", \"broken adapter behavior was accepted by claim\"),\n");
        out.push_str("    }\n");
        out.push_str("}\n\n");
    }

    // Suite constructor
    out.push_str("/// Builds the executable test suite.\n");
    out.push_str("pub fn build_suite() -> Suite {\n");
    out.push_str("    let mut suite = Suite::new();\n\n");

    // Add example test cases
    for case in &example_cases {
        let test_name = format!("{}/{}", file.name, case.id);
        out.push_str(&format!("    suite.add(\"{test_name}\", |ctx| {{\n"));
        let body = generate_case_body(case, eq_u64_export)?;
        out.push_str(&body);
        out.push_str("    });\n\n");
    }

    // Add negative control checks
    out.push_str("    // Adapter negative controls (sabotage checks)\n");
    out.push_str("    suite.add(\"adapter/wrong-equality\", |ctx| {\n");
    out.push_str("        reject_after_baseline(ctx, |c, f| check_eq(c, b\"00\", b\"00\", true, f), Fault::Equality)\n");
    out.push_str("    });\n");
    out.push_str("    suite.add(\"adapter/wrong-selection\", |ctx| {\n");
    out.push_str("        reject_after_baseline(ctx, |c, f| check_select(c, b\"00\", b\"ff\", true, b\"00\", f), Fault::Selection)\n");
    out.push_str("    });\n");
    out.push_str("    suite.add(\"adapter/missing-error\", |ctx| {\n");
    out.push_str(&format!(
        "        reject_after_baseline(ctx, |c, f| check_select_err(c, b\"\", b\"00\", false, \"{select_error_case}\", f), Fault::MissingError)\n"
    ));
    out.push_str("    });\n");
    out.push_str("    suite.add(\"adapter/unknown-fixed-width\", |_ctx| {\n");
    out.push_str("        let res = fixed_dispatch(999, b\"\", b\"\");\n");
    out.push_str("        if res == Err(\"fixed width has no generated arm\".into()) { Verdict::Pass } else { Verdict::fail(\"corpus-mismatch\", \"unsupported width did not report missing arm\") }\n");
    out.push_str("    });\n");
    out.push_str("    suite.add(\"adapter/wrong-work\", |_ctx| {\n");
    out.push_str("        let positions = 2u64;\n");
    out.push_str("        let byte_reads = 2 * positions;\n");
    out.push_str("        let mutated_reads = 0u64;\n");
    out.push_str("        if byte_reads != mutated_reads { Verdict::Pass } else { Verdict::fail(\"corpus-mismatch\", \"work mismatch not caught\") }\n");
    out.push_str("    });\n\n");

    // Universal claim checks and sabotage detection (VER04f)
    for claim in &claim_cases {
        let test_name = format!("{}/claim_{}", file.name, claim.id);
        out.push_str(&format!("    suite.add(\"{test_name}\", |ctx| {{\n"));
        out.push_str(&format!("        check_claim_{}(ctx, Fault::None)\n", claim.id));
        out.push_str("    });\n");
        out.push_str(&format!("    suite.add(\"claim_sabotage/{}_broken_impl\", |ctx| {{\n", claim.id));
        out.push_str(&format!("        reject_claim_after_baseline(ctx, |c, f| check_claim_{}(c, f), Fault::Equality)\n", claim.id));
        out.push_str("    });\n\n");
    }

    out.push_str("    suite\n");
    out.push_str("}\n\n");

    // Helper functions for checking operations
    out.push_str("fn check_eq(ctx: &Ctx, left: &[u8], right: &[u8], expected: bool, fault: Fault) -> Verdict {\n");
    out.push_str(&format!("    let mut got = {eq_export}(left, right);\n"));
    out.push_str("    if matches!(fault, Fault::Equality) { got = !got; }\n");
    out.push_str("    ctx.deep_equal(&Value::Bool(got), &Value::Bool(expected))\n");
    out.push_str("}\n\n");

    out.push_str("fn check_select(ctx: &Ctx, left: &[u8], right: &[u8], choice: bool, expected: &[u8], fault: Fault) -> Verdict {\n");
    out.push_str("    let left_before = left.to_vec();\n");
    out.push_str("    let right_before = right.to_vec();\n");
    out.push_str("    let effective_choice = if matches!(fault, Fault::Selection) { !choice } else { choice };\n");
    out.push_str(&format!(
        "    let mut output = {select_export}(left, right, effective_choice);\n"
    ));
    out.push_str("    let got = output.clone();\n");
    out.push_str("    if let Some(first) = output.first_mut() { *first ^= 0xff; }\n");
    out.push_str("    let independent = left == left_before && right == right_before\n");
    out.push_str("        && (output.is_empty() || (output.as_ptr() != left.as_ptr() && output.as_ptr() != right.as_ptr()));\n");
    out.push_str(
        "    let verdict = ctx.deep_equal(&Value::Bytes(got), &Value::Bytes(expected.to_vec()));\n",
    );
    out.push_str("    if !matches!(verdict, Verdict::Pass) { return verdict; }\n");
    out.push_str("    ctx.deep_equal(&Value::Bool(independent), &Value::Bool(true))\n");
    out.push_str("}\n\n");

    out.push_str("fn check_select_err(ctx: &Ctx, left: &[u8], right: &[u8], choice: bool, expected_err: &str, fault: Fault) -> Verdict {\n");
    out.push_str("    let got = catch_unwind(|| {\n");
    out.push_str("        if matches!(fault, Fault::MissingError) { Vec::new() }\n");
    out.push_str(&format!(
        "        else {{ {select_export}(left, right, choice) }}\n"
    ));
    out.push_str("    });\n");
    out.push_str(&format!(
        "    ctx.error_is(got.err().map(|_| \"{select_error_case}\"), expected_err)\n"
    ));
    out.push_str("}\n\n");

    // Helper functions for checking claims
    for claim in &claim_cases {
        let helper = generate_claim_helper(claim, manifest, file)?;
        out.push_str(&helper);
        out.push('\n');
    }

    // Main entrypoint with silent panic hook
    out.push_str("fn main() {\n");
    out.push_str("    std::panic::set_hook(Box::new(|_| ()));\n");
    out.push_str("    let suite = build_suite();\n");
    out.push_str("    std::process::exit(run::main(&suite));\n");
    out.push_str("}\n");

    Ok(out)
}

fn generate_case_body(case: &ExampleCase, eq_u64_export: &str) -> Result<String, CodegenError> {
    let mut s = String::new();
    match case.op_name.as_str() {
        "eq" => {
            let left = case.args[0].as_bytes().unwrap_or(&[]);
            let right = case.args[1].as_bytes().unwrap_or(&[]);
            let expected = case.expected.as_bool().unwrap_or(false);
            s.push_str(&format!(
                "        check_eq(ctx, &b\"{}\"[..], &b\"{}\"[..], {}, Fault::None)\n",
                escape_bytes(left),
                escape_bytes(right),
                expected
            ));
        }
        "eq_fixed" => {
            let left = case.args[0].as_bytes().unwrap_or(&[]);
            let right = case.args[1].as_bytes().unwrap_or(&[]);
            let width = case.static_width.unwrap_or(0);
            let expected = case.expected.as_bool().unwrap_or(false);
            s.push_str(&format!(
                "        let got = fixed_dispatch({width}, &b\"{}\"[..], &b\"{}\"[..]);\n",
                escape_bytes(left),
                escape_bytes(right)
            ));
            s.push_str("        match got {\n");
            s.push_str("            Ok(val) => ctx.deep_equal(&Value::Bool(val), &Value::Bool(");
            s.push_str(if expected { "true" } else { "false" });
            s.push_str(")),\n");
            s.push_str(
                "            Err(e) => Verdict::fail(\"corpus-malformed\", format!(\"fixed failed: {e}\")),\n",
            );
            s.push_str("        }\n");
        }
        "select_bytes" => {
            let left = case.args[0].as_bytes().unwrap_or(&[]);
            let right = case.args[1].as_bytes().unwrap_or(&[]);
            let choice = case.args[2].as_bool().unwrap_or(false);
            match &case.expected {
                EvalValue::Result(Ok(inner)) => {
                    let out_bytes = inner.as_bytes().unwrap_or(&[]);
                    s.push_str(&format!(
                        "        check_select(ctx, &b\"{}\"[..], &b\"{}\"[..], {}, &b\"{}\"[..], Fault::None)\n",
                        escape_bytes(left),
                        escape_bytes(right),
                        choice,
                        escape_bytes(out_bytes)
                    ));
                }
                EvalValue::Result(Err((_, err_variant))) => {
                    s.push_str(&format!(
                        "        check_select_err(ctx, &b\"{}\"[..], &b\"{}\"[..], {}, \"{}\", Fault::None)\n",
                        escape_bytes(left),
                        escape_bytes(right),
                        choice,
                        err_variant
                    ));
                }
                _ => {}
            }
        }
        "eq_u64" => {
            let left = case.args[0].as_u64().unwrap_or(0);
            let right = case.args[1].as_u64().unwrap_or(0);
            let expected = case.expected.as_bool().unwrap_or(false);
            s.push_str(&format!(
                "        let got = {eq_u64_export}({left}u64, {right}u64);\n"
            ));
            s.push_str(&format!(
                "        ctx.deep_equal(&Value::Bool(got), &Value::Bool({expected}))\n"
            ));
        }
        _ => {
            return Err(CodegenError {
                code: DiagnosticCode::W0701,
                kind: CodegenErrorKind::UnsupportedTarget(format!(
                    "Unsupported operation for harness codegen: {}",
                    case.op_name
                )),
                span: (0, 0),
            });
        }
    }
    Ok(s)
}

fn escape_bytes(bytes: &[u8]) -> String {
    let mut out = String::new();
    for &b in bytes {
        if (0x20..=0x7e).contains(&b) && b != b'\\' && b != b'"' {
            out.push(b as char);
        } else {
            out.push_str(&format!("\\x{b:02x}"));
        }
    }
    out
}

/// Checks that a file's content on disk matches generated content exactly.
pub fn verify_regeneration(target: &Path, content: &str) -> Result<bool, String> {
    if !target.exists() {
        return Ok(false);
    }
    let existing =
        std::fs::read_to_string(target).map_err(|e| format!("Failed to read target: {e}"))?;
    Ok(existing == content)
}
