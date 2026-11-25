//! # Warrant Rust Binding Schema and Validator (VER04d §5 / VER-P007)
//!
//! Enforces that real public exports correspond to abstract Warrant operations,
//! validating signature compatibility, freshness assertions, negative controls,
//! error normalizations, and closed failure codes (`W0501`, `W0502`).

use std::collections::{BTreeMap, BTreeSet};

use crate::checker::{CheckedWarrantModule, CheckerError, CheckerErrorKind, DiagnosticCode, Type};
use crate::driver::DriverRecord;
use crate::parser::Profile;

/// Closed error normalization rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorNormalization {
    pub kind: String, // e.g. "panic_to_error"
    pub error_case: String,
}

/// Binding specification for a single Warrant public operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpBinding {
    /// The `export=` callee, or `None` for a WX-F driven operation (which
    /// executes as a constructor/step/finish protocol instead of one call).
    /// Exactly one of `export` and `driver` is `Some` (VER04k 3.4 rule 1).
    pub export: Option<String>,
    /// The parsed `driver` record for a driven operation, attached after the
    /// whole file parses so `driver` lines may precede their `op` lines.
    pub driver: Option<crate::driver::DriverRecord>,
    pub static_params: Vec<String>,
    pub fresh: bool,
    pub error_normalization: Option<ErrorNormalization>,
    pub negative_controls: Vec<String>,
}

/// Complete host binding manifest for a Warrant module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingManifest {
    pub package: String,
    pub revision: String,
    pub bindings: BTreeMap<String, OpBinding>,
}

/// A host (Rust) type of a bound composite component (VER04j section 4.6).
///
/// ```text
/// u8 u16 u32 u64 i8 i16 i32 i64 usize     HostTy::Int
/// bool  bytes                              HostTy::Bool, HostTy::Bytes
/// bytes<32>                                HostTy::BytesN (the array `[u8; 32]`)
/// (u64,usize)                              HostTy::Tuple   (a Rust tuple)
/// opt<u8>  list<u32>                       Option<u8>, Vec<u32>
/// Tlv                                      HostTy::Named   (a declared struct/enum)
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostTy {
    Int(String),
    Bool,
    Bytes,
    /// `bytes<N>`: the fixed-length array `[u8; N]` (VER04j section 14.4).  It
    /// fits only a Warrant `Bytes<N>` with the same literal `N`.
    BytesN(u64),
    Tuple(Vec<HostTy>),
    Opt(Box<HostTy>),
    List(Box<HostTy>),
    Named(String),
}

/// One `name:type` field of a host struct or enum variant.  The name is a Rust
/// identifier or a decimal index (`0`, `1`), which reaches the fields of a
/// tuple struct or tuple variant (`P { 0: x }`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostField {
    pub name: String,
    pub ty: HostTy,
}

/// How a declared host type is built and observed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostShape {
    /// A struct: its fields, in the order of the Warrant record's fields.
    Struct(Vec<HostField>),
    /// An enum: its variants (`name`, fields), in the order of the Warrant
    /// variant's constructors; a variant is matched by name.
    Enum(Vec<(String, Vec<HostField>)>),
}

/// A `struct` or `enum` line of the bindings file: the host type (a Rust path)
/// that implements one Warrant record or variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostType {
    pub path: String,
    pub shape: HostShape,
}

/// `errors=PATH/Variant=CASE,...`: the host error enum of an export that returns
/// `Result<T, PATH>`, and which Warrant error case each (unit-like) variant is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostErrors {
    pub path: String,
    pub map: Vec<(String, String)>,
}

/// How one `inout` parameter is carried across the host boundary (VER04k
/// section 4.6): the `<form>` of `inout=<param>:<form>`.
///
/// ```text
/// slice        &mut [u8]                 Bytes, Bytes<N>      (length cannot change)
/// vec          &mut Vec<u8> / Vec<ht>    Bytes, List<T>       (any length afterwards)
/// array        &mut [u8; N]              Bytes<N>
/// int:u8       &mut u8 ... &mut usize    fixed-width, Nat
/// bool         &mut bool                 Bool
/// struct:Cell  &mut Cell                 a record, field by field
/// ```
///
/// The last four spellings are parsed so that a bindings file written for the
/// stateful-objects slice (VER04m) is a *refusal* with a clear message here, not
/// a syntax error; the generator reports them as not yet implemented (W0704).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InoutForm {
    Slice,
    Vec,
    Array,
    Int(String),
    Bool,
    Struct(String),
    Returned,
    SliceOfStruct(String),
    VecOfStruct(String),
    Codec(String),
}

impl InoutForm {
    /// The text the bindings file spells the form with.
    pub fn text(&self) -> String {
        match self {
            InoutForm::Slice => "slice".into(),
            InoutForm::Vec => "vec".into(),
            InoutForm::Array => "array".into(),
            InoutForm::Int(t) => format!("int:{t}"),
            InoutForm::Bool => "bool".into(),
            InoutForm::Struct(n) => format!("struct:{n}"),
            InoutForm::Returned => "returned".into(),
            InoutForm::SliceOfStruct(n) => format!("slice-of:struct:{n}"),
            InoutForm::VecOfStruct(n) => format!("vec-of:struct:{n}"),
            InoutForm::Codec(n) => format!("codec:{n}"),
        }
    }
}

/// Parse one host form (the text after the first colon of an `inout=` entry).
pub fn parse_inout_form(text: &str) -> Result<InoutForm, String> {
    let name = |n: &str| -> Result<String, String> {
        if is_rust_identifier(n) {
            Ok(n.to_string())
        } else {
            Err(format!("`{n}` is not a type name"))
        }
    };
    Ok(match text {
        "slice" => InoutForm::Slice,
        "vec" => InoutForm::Vec,
        "array" => InoutForm::Array,
        "bool" => InoutForm::Bool,
        "returned" => InoutForm::Returned,
        _ => {
            if let Some(t) = text.strip_prefix("int:") {
                if !HOST_INTS.contains(&t) {
                    return Err(format!("`int:{t}`: the host integer is one of {HOST_INTS:?}"));
                }
                InoutForm::Int(t.to_string())
            } else if let Some(n) = text.strip_prefix("slice-of:struct:") {
                InoutForm::SliceOfStruct(name(n)?)
            } else if let Some(n) = text.strip_prefix("vec-of:struct:") {
                InoutForm::VecOfStruct(name(n)?)
            } else if let Some(n) = text.strip_prefix("struct:") {
                InoutForm::Struct(name(n)?)
            } else if let Some(n) = text.strip_prefix("codec:") {
                InoutForm::Codec(name(n)?)
            } else {
                return Err(format!("`{text}` is not an inout host form"));
            }
        }
    })
}

/// `inout=buf:slice,n:int:u64`: the host form of each `inout` parameter, in the
/// order written.  The parameter is the text before the *first* colon.
fn parse_inout_entries(text: &str) -> Result<Vec<(String, InoutForm)>, String> {
    let mut out: Vec<(String, InoutForm)> = Vec::new();
    for entry in split_top(text) {
        let (param, form) = entry
            .split_once(':')
            .ok_or_else(|| format!("inout entry `{entry}` is not `<param>:<form>`"))?;
        if !is_rust_identifier(param) {
            return Err(format!("inout entry `{entry}`: `{param}` is not a parameter name"));
        }
        if out.iter().any(|(p, _)| p == param) {
            return Err(format!("inout parameter `{param}` is given a host form twice"));
        }
        if out.len() >= 4 {
            return Err("more than 4 inout entries".to_string());
        }
        out.push((param.to_string(), parse_inout_form(form).map_err(|e| format!("inout entry `{entry}`: {e}"))?));
    }
    if out.is_empty() {
        return Err("inout= needs at least one `<param>:<form>` entry".to_string());
    }
    Ok(out)
}

/// The most segments of an `export=` path (VER04k section 4.6).
pub const MAX_EXPORT_SEGMENTS: usize = 8;

/// The host signature of one operation (all parts optional; absent means the
/// default derived from the Warrant type).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OpHost {
    pub args: Option<Vec<HostTy>>,
    pub ret: Option<HostTy>,
    pub errors: Option<HostErrors>,
    /// `err_any=CASE` (VER04j section 14.4): the export returns `Result<T, E>`
    /// for an opaque `E`, and every `Err(_)` is the Warrant error case `CASE`.
    pub err_any: Option<String>,
    /// WX-G: the host form of each `inout` parameter (VER04k section 4.6).
    pub inout: Vec<(String, InoutForm)>,
}

/// Everything the bindings file says about host composites (VER04j section 4.6).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HostSpec {
    /// Declared host structs and enums by Warrant type name.
    pub types: BTreeMap<String, HostType>,
    /// Host signatures by operation name.
    pub ops: BTreeMap<String, OpHost>,
}

const HOST_INTS: [&str; 9] = ["u8", "u16", "u32", "u64", "i8", "i16", "i32", "i64", "usize"];
const MAX_HOST_DEPTH: usize = 8;
const MAX_HOST_ITEMS: usize = 256;

/// Whether a whole host spec could have come from `struct`/`enum`/`host`
/// lines: every spliced path, name and type re-checked. Generation runs
/// this on hand-built options; anything else would splice unchecked text
/// into generated source.
pub(crate) fn validate_host_spec(spec: &HostSpec) -> Result<(), String> {
    for (name, decl) in &spec.types {
        if !is_rust_identifier(name) {
            return Err(format!("host type name `{name}` is not an identifier"));
        }
        if !is_rust_path(&decl.path) {
            return Err(format!("host type `{name}` has a bad path"));
        }
        let mut fields = Vec::new();
        match &decl.shape {
            HostShape::Struct(fs) => fields.extend(fs.iter()),
            HostShape::Enum(vs) => {
                for (v, fs) in vs {
                    if !is_rust_identifier(v) {
                        return Err(format!("host variant `{v}` is not an identifier"));
                    }
                    fields.extend(fs.iter());
                }
            }
        }
        for f in fields {
            // Mirrors the line parser: an identifier or a positional index.
            let ok = is_rust_identifier(&f.name)
                || (!f.name.is_empty() && f.name.bytes().all(|b| b.is_ascii_digit()));
            if !ok {
                return Err(format!("host field `{}` is not an identifier or an index", f.name));
            }
            if !valid_host_ty(&f.ty) {
                return Err(format!("host field `{}` has a bad type", f.name));
            }
        }
    }
    for (op, h) in &spec.ops {
        if !is_rust_identifier(op) {
            return Err(format!("host op `{op}` is not an identifier"));
        }
        for t in h.args.iter().flatten().chain(h.ret.iter()) {
            if !valid_host_ty(t) {
                return Err(format!("host signature of `{op}` has a bad type"));
            }
        }
        if let Some(errors) = &h.errors {
            if !is_rust_path(&errors.path) {
                return Err(format!("host errors path of `{op}` is bad"));
            }
            for (variant, case) in &errors.map {
                if !is_rust_identifier(variant) || !is_rust_identifier(case) {
                    return Err(format!("host errors entry of `{op}` must name identifiers"));
                }
            }
        }
        if let Some(case) = &h.err_any {
            if !is_rust_identifier(case) {
                return Err(format!("host `err_any` of `{op}` is not an identifier"));
            }
        }
        for (param, form) in &h.inout {
            if !is_rust_identifier(param) {
                return Err(format!("host `inout` parameter `{param}` is not an identifier"));
            }
            if let InoutForm::Int(s) = form {
                if !HOST_INTS.contains(&s.as_str()) {
                    return Err(format!("host `inout` integer `{s}` is unknown"));
                }
            }
        }
    }
    Ok(())
}

/// Whether a host type could have come from [`parse_host_ty`]: closed int
/// vocabulary, identifier names, capped widths, tuples of 2 to 8, bounded
/// depth. Generation re-checks hand-built records with this; anything else
/// would splice unchecked text into generated source.
pub(crate) fn valid_host_ty(h: &HostTy) -> bool {
    fn go(h: &HostTy, depth: usize) -> bool {
        if depth > MAX_HOST_DEPTH {
            return false;
        }
        match h {
            HostTy::Int(s) => HOST_INTS.contains(&s.as_str()),
            HostTy::Bool | HostTy::Bytes => true,
            HostTy::BytesN(n) => *n <= crate::evidence::MAX_FIXED_WIDTH,
            HostTy::Tuple(hs) => {
                (2..=8).contains(&hs.len()) && hs.iter().all(|h| go(h, depth + 1))
            }
            HostTy::Opt(h) | HostTy::List(h) => go(h, depth + 1),
            HostTy::Named(n) => is_rust_identifier(n),
        }
    }
    go(h, 0)
}

/// Split `text` on commas that are not inside `()` or `<>`.
pub fn split_top(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let (mut depth, mut start) = (0i32, 0usize);
    for (i, c) in text.char_indices() {
        match c {
            '(' | '<' => depth += 1,
            ')' | '>' => depth -= 1,
            ',' if depth == 0 => {
                out.push(&text[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&text[start..]);
    out
}

/// Parse one host type expression.
///
/// ```text
/// ht = "u8" | ... | "usize" | "bool" | "bytes" | "bytes<" N ">"
///    | "(" ht { "," ht } ")"  |  "opt<" ht ">"  |  "list<" ht ">"  |  NAME
/// ```
pub fn parse_host_ty(text: &str) -> Result<HostTy, String> {
    parse_host_ty_at(text.trim(), 0)
}

fn parse_host_ty_at(t: &str, depth: usize) -> Result<HostTy, String> {
    if depth > MAX_HOST_DEPTH {
        return Err(format!("host type `{t}` nests deeper than {MAX_HOST_DEPTH}"));
    }
    if HOST_INTS.contains(&t) {
        return Ok(HostTy::Int(t.to_string()));
    }
    match t {
        "bool" => return Ok(HostTy::Bool),
        "bytes" => return Ok(HostTy::Bytes),
        _ => {}
    }
    if let Some(inner) = t.strip_prefix("bytes<").and_then(|r| r.strip_suffix('>')) {
        // A plain decimal literal: no sign, no leading zero, no space, and at
        // most the generator's fixed-width cap (the width becomes a Rust array
        // type in generated source, so nothing else may reach it).
        let plain = !inner.is_empty()
            && inner.bytes().all(|b| b.is_ascii_digit())
            && (inner == "0" || !inner.starts_with('0'));
        let n = plain.then(|| inner.parse::<u64>().ok()).flatten();
        return match n {
            Some(n) if n <= crate::evidence::MAX_FIXED_WIDTH => Ok(HostTy::BytesN(n)),
            _ => Err(format!(
                "`{t}`: the width of bytes<N> is a decimal literal from 0 to {}",
                crate::evidence::MAX_FIXED_WIDTH
            )),
        };
    }
    if let Some(inner) = t.strip_prefix("opt<").and_then(|r| r.strip_suffix('>')) {
        return Ok(HostTy::Opt(Box::new(parse_host_ty_at(inner.trim(), depth + 1)?)));
    }
    if let Some(inner) = t.strip_prefix("list<").and_then(|r| r.strip_suffix('>')) {
        return Ok(HostTy::List(Box::new(parse_host_ty_at(inner.trim(), depth + 1)?)));
    }
    if let Some(inner) = t.strip_prefix('(').and_then(|r| r.strip_suffix(')')) {
        let parts = split_top(inner);
        if parts.len() < 2 || parts.len() > 8 {
            return Err(format!("host tuple `{t}` needs 2 to 8 components"));
        }
        let items: Result<Vec<_>, _> = parts
            .into_iter()
            .map(|p| parse_host_ty_at(p.trim(), depth + 1))
            .collect();
        return Ok(HostTy::Tuple(items?));
    }
    if is_rust_identifier(t) {
        return Ok(HostTy::Named(t.to_string()));
    }
    Err(format!("`{t}` is not a host type"))
}

/// `a::b::C`: identifier segments joined by `::`.
pub fn is_rust_path(text: &str) -> bool {
    !text.is_empty() && text.split("::").all(is_rust_identifier)
}

fn parse_host_fields(text: &str) -> Result<Vec<HostField>, String> {
    let mut out: Vec<HostField> = Vec::new();
    if text.trim().is_empty() {
        return Ok(out);
    }
    for part in split_top(text) {
        let (name, ty) = part
            .split_once(':')
            .ok_or_else(|| format!("field `{part}` must be name:type"))?;
        let name = name.trim();
        let ok = is_rust_identifier(name)
            || (!name.is_empty() && name.bytes().all(|b| b.is_ascii_digit()));
        if !ok {
            return Err(format!("field name `{name}` is not an identifier or an index"));
        }
        if out.iter().any(|f| f.name == name) {
            return Err(format!("field `{name}` appears twice"));
        }
        if out.len() >= MAX_HOST_ITEMS {
            return Err("too many fields".to_string());
        }
        out.push(HostField {
            name: name.to_string(),
            ty: parse_host_ty(ty)?,
        });
    }
    Ok(out)
}

fn host_names(t: &HostTy, out: &mut Vec<String>) {
    match t {
        HostTy::Named(n) => out.push(n.clone()),
        HostTy::Tuple(ts) => ts.iter().for_each(|t| host_names(t, out)),
        HostTy::Opt(t) | HostTy::List(t) => host_names(t, out),
        _ => {}
    }
}

/// Parse the value of `errors=`: `PATH/Variant=CASE,Variant2=CASE2`.
pub(crate) fn parse_host_errors(text: &str) -> Result<HostErrors, String> {
    let (path, rest) = text
        .split_once('/')
        .ok_or_else(|| format!("errors=`{text}` must be PATH/Variant=CASE,..."))?;
    if !is_rust_path(path) {
        return Err(format!("errors path `{path}` is not a Rust path"));
    }
    let mut map = Vec::new();
    for pair in rest.split(',') {
        let (variant, case) = pair
            .split_once('=')
            .ok_or_else(|| format!("errors entry `{pair}` must be Variant=CASE"))?;
        if !is_rust_identifier(variant) || !is_rust_identifier(case) {
            return Err(format!("errors entry `{pair}` must name identifiers"));
        }
        map.push((variant.to_string(), case.to_string()));
    }
    if map.is_empty() {
        return Err("errors= lists no variant".to_string());
    }
    Ok(HostErrors {
        path: path.to_string(),
        map,
    })
}

/// A parsed `evidence.bindings` file: the binding manifest plus every
/// generation parameter, so one data file (not Rust code) tells the evidence
/// generator how to bind a package (VER04 section 6, VER04h section 1).
///
/// ```text
/// # comment
/// package coding_adventures_ct_compare
/// revision 0.1.0
/// spec code/specs/ct-compare.warrant
/// output code/packages/rust/ct-compare/tests/generated/warrant_evidence.rs
/// seed 0x57415252414e5421
/// property_cases 4096
/// fuzz_iterations 5000
/// negctl_fuzz_iterations 1000
/// exhaustive_budget 400000
/// op eq export=ct_eq faults=invert,ignore-last-byte
/// op eq_fixed export=ct_eq_fixed static=N faults=invert,ignore-last-byte
/// op select_bytes export=ct_select_bytes fresh error=CT_SELECT_LENGTH_MISMATCH faults=swap-choice,missing-error
/// op power export=power args=u8,u32 faults=exp-mod-256 proofs_not_live=frontend-refuses-lib
/// ```
///
/// Keys other than `op` appear once.  `op` options: `export=` or bare
/// `driver` (exactly one, VER04k 3.4 rule 1),
/// `static=A,B`, bare `fresh`, `error=CASE` (panic-to-error normalization),
/// `args=u8,u32` (Rust parameter types of a scalar export), `faults=a,b`
/// (required, at least one), `proofs_not_live=REASON` (the operation's proof
/// obligations are acknowledged as not runnable; see
/// [`EvidenceBindingFile::proofs_not_live`]).  A `driver NAME k=v...` line
/// carries the WX-F record for a driver-flagged op (see
/// [`crate::driver::parse_driver_record`]).  `package` and every `export` must
/// be Rust identifiers, `exhaustive_budget` is at most [`MAX_EXHAUSTIVE_BUDGET`]
/// (2^26), and `spec`/`output` are plain relative paths that
/// [`EvidenceBindingFile::validate_paths`] pins to the package.  Anything unknown is an error: a typo must not
/// silently weaken the evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceBindingFile {
    pub manifest: BindingManifest,
    pub spec: String,
    pub output: String,
    pub seed: u64,
    pub property_cases: u64,
    pub fuzz_iterations: u64,
    pub negctl_fuzz_iterations: u64,
    pub exhaustive_budget: u64,
    pub arg_types: BTreeMap<String, Vec<String>>,
    /// Operations whose proof obligations the package *declares* it cannot
    /// run (`proofs_not_live=<reason>`), with the one-word reason.  A proof
    /// manifest may record `not-live` for an operation only if it is listed
    /// here: an acknowledged absence of proof is an opt-in made in this
    /// reviewed file, never something a manifest can claim by itself.
    pub proofs_not_live: BTreeMap<String, String>,
    /// WX-A host composites: the `struct`/`enum` lines and the `ret=` /
    /// `errors=` options (VER04j section 4.6).
    pub hosts: HostSpec,
    /// VER33 section 4.8: the `realizes=` / `embedding=` options and `panic`
    /// lines, by operation name.  Data only: nothing consumes it yet
    /// (ADL-001a is schema and validation); see [`Realization`].
    pub realizations: BTreeMap<String, Realization>,
}

/// The canonical embeddings a `realizes` binding may select (VER33 section 4.8).
///
/// ```text
/// int-by-value    every integer leaf (Int, Nat, U8..U64, I8..I64) is the
///                 model's `Int` of the same value; Bool is Bool
/// bytes-identity  every Bytes / Bytes<N> leaf is the model's Bytes, byte for
///                 byte; Bool is Bool
/// ```
///
/// `Option`, `Result`, `List` and tuples embed *componentwise* around whichever
/// leaf embedding is chosen, so they need no name of their own.  The set is
/// closed: there is no user-supplied conversion function.
pub const EMBEDDINGS: [&str; 2] = ["int-by-value", "bytes-identity"];

/// Most `panic` templates one realization may declare.  The set is finite and
/// reviewed (VER33 section 8.2); an unbounded list is not a reviewed list.
pub const MAX_PANIC_TEMPLATES: usize = 16;

/// Longest `panic` template, in bytes.
pub const MAX_PANIC_TEMPLATE_BYTES: usize = 256;

/// How an `op` says it implements a model function (VER33 section 4.8).
///
/// ```text
/// op nand_gate export=nand_gate faults=always_true realizes=logic_gates_model.nand_gate embedding=int-by-value
/// panic nand_gate a must be 0 or 1
/// panic nand_gate b must be 0 or 1
/// ```
///
/// `realizes` names a model function and *selects* an embedding; it does not
/// state the domain (the lane function's declared parameter types do), and it
/// is optional: without it the op has A0/A1 evidence only.  `panic_messages`
/// is the finite, declared set of the lane's panic identities that count as the
/// modelled rejection, as **exact templates**.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Realization {
    /// The model file's module name (the `warrant NAME` of an algorithm-profile document).
    pub module: String,
    /// The model function in that module.
    pub model: String,
    /// One of [`EMBEDDINGS`], kept as text so that validation can refuse an
    /// unknown one rather than the type system making it unrepresentable.
    pub embedding: String,
    /// Exact format templates (VER33 section 8.2, R5).
    pub panic_messages: Vec<String>,
}

/// Phrases that mark a panic as *compiler-generated*: an overflow, an index, an
/// `unwrap`, a division by zero, an allocation failure.  Such a panic can never
/// be the modelled rejection (VER33 section 8.2: "compiler-generated panics ...
/// can never be listed"), because it also fires on bugs the model knows nothing
/// about.  Matched case-insensitively as a substring of the *normalised*
/// template (whitespace runs collapsed), and again with the `{}` holes removed,
/// across the lanes' wordings (Rust, Go, Java, Python, JS/TS).
///
/// This list is a backstop, not the check: the long-term check is the lifter
/// confirming each template against a literal panic message in the lane source
/// (backlog ADL-032, ADL-033).
const COMPILER_GENERATED_PANICS: [&str; 38] = [
    "attempt to ",
    "with overflow",
    "index out of bounds",
    "out of bounds for length",
    "index out of range",
    "slice bounds out of range",
    "slice index",
    "range end index",
    "range start index",
    "byte index",
    "unwrap()",
    "result::unwrap",
    "option::unwrap",
    "on a `none` value",
    "on an `err` value",
    "divide by zero",
    "division by zero",
    "division or modulo by zero",
    "/ by zero",
    "remainder with a divisor of zero",
    "explicit panic",
    "internal error: entered unreachable",
    "not yet implemented",
    "capacity overflow",
    "already borrowed",
    "already mutably borrowed",
    "nil pointer",
    "nil map",
    "runtime error",
    "cannot read propert",
    "is not a function",
    "stack overflow",
    "assertion failed",
    "pop from empty list",
    "out of memory",
    "list index out of range",
    "is not defined",
    "is not iterable",
];

/// Characters a template must begin with, counted as ASCII alphanumerics before
/// the first `{}` hole: the **anchor rule** (security review of ADL-001a).
pub const MIN_PANIC_ANCHOR: usize = 8;

/// Why a `panic` template is refused, or `Ok` when it is an exact template.
///
/// ```text
/// "a must be 0 or 1"                 accepted: exact text
/// "a must be 0 or 1, got {}"         accepted: anchor, then a hole
/// "{}"  "{}: {}"  "a{}"  "{} !"      refused:  no anchor (a near-wildcard)
/// "x"   "short {}"                   refused:  anchor under 8 alphanumerics
/// ""    "   "                        refused:  empty
/// "bad *"  ".*"  "?"                 refused:  wildcard characters
/// NBSP, U+200B, bidi, accents        refused:  only printable ASCII (0x20-0x7E)
/// "attempt to add with overflow"     refused:  compiler-generated (W0807)
/// ```
///
/// **The anchor rule.** A template must begin with at least
/// [`MIN_PANIC_ANCHOR`] ASCII alphanumeric characters before its first `{}`
/// hole (spaces and punctuation may sit between them), so it cannot start with
/// a hole and a single literal character beside a hole is not enough.  Without
/// it `{}: {}` would match `index out of bounds: the len is 3 but the index is
/// 5`, defeating VER33 R5 ("exact templates, no wildcards").  Holes after the
/// anchor are allowed.
pub fn check_panic_template(text: &str) -> Result<(), String> {
    if text.trim().is_empty() {
        return Err("panic template is empty".to_string());
    }
    if text.len() > MAX_PANIC_TEMPLATE_BYTES {
        return Err(format!(
            "panic template is longer than {MAX_PANIC_TEMPLATE_BYTES} bytes"
        ));
    }
    // Printable ASCII only: this also refuses tabs and newlines, NBSP, zero-width
    // and bidi characters, and U+2028/2029, which could hide or reorder text.
    if text.chars().any(|c| !(' '..='~').contains(&c)) {
        return Err("panic template contains a character outside printable ASCII".to_string());
    }
    if text != text.trim() {
        return Err("panic template has leading or trailing whitespace".to_string());
    }
    if text.contains(['*', '?']) {
        return Err("panic template contains a wildcard character".to_string());
    }
    // `{}` is the only format hole; `{{` and `}}` escape the braces.
    let mut anchor = 0usize;
    let mut seen_hole = false;
    let mut literal = false;
    let mut stripped = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' if chars.peek() == Some(&'}') => {
                chars.next();
                seen_hole = true;
                stripped.push(' ');
            }
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
                literal = true;
                stripped.push('{');
            }
            '}' if chars.peek() == Some(&'}') => {
                chars.next();
                literal = true;
                stripped.push('}');
            }
            '{' | '}' => {
                return Err("panic template has a brace that is not `{}`, `{{` or `}}`".to_string())
            }
            c => {
                if !c.is_whitespace() {
                    literal = true;
                }
                if !seen_hole && c.is_ascii_alphanumeric() {
                    anchor += 1;
                }
                stripped.push(c);
            }
        }
    }
    if !literal {
        return Err("panic template is only format holes: a wildcard".to_string());
    }
    if anchor < MIN_PANIC_ANCHOR {
        return Err(format!(
            "panic template needs a literal anchor of at least {MIN_PANIC_ANCHOR} alphanumeric characters before its first `{{}}` hole"
        ));
    }
    let norm = |t: &str| {
        t.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase()
    };
    let (a, b) = (norm(text), norm(&stripped));
    if let Some(p) = COMPILER_GENERATED_PANICS
        .iter()
        .find(|p| a.contains(*p) || b.contains(*p))
    {
        return Err(format!(
            "panic template looks compiler-generated (contains `{p}`); only an authored panic can be the modelled rejection"
        ));
    }
    Ok(())
}

impl Realization {
    /// The shape rules that need no model module: a known embedding, plain
    /// identifiers, and every `panic` entry an exact, distinct template.
    /// Returns the diagnostic code with the reason.
    pub fn check_shape(&self, op: &str) -> Result<(), (DiagnosticCode, String)> {
        let w0806 = |m: String| (DiagnosticCode::W0806, m);
        if !EMBEDDINGS.contains(&self.embedding.as_str()) {
            return Err(w0806(format!(
                "op `{op}`: unknown embedding `{}` (the canonical set is {})",
                self.embedding,
                EMBEDDINGS.join(", ")
            )));
        }
        for (what, name) in [("module", &self.module), ("model function", &self.model)] {
            if !is_rust_identifier(name) {
                return Err(w0806(format!(
                    "op `{op}`: realized {what} `{name}` is not an identifier"
                )));
            }
        }
        if self.panic_messages.len() > MAX_PANIC_TEMPLATES {
            return Err((
                DiagnosticCode::W0807,
                format!("op `{op}`: more than {MAX_PANIC_TEMPLATES} panic templates"),
            ));
        }
        for (i, m) in self.panic_messages.iter().enumerate() {
            check_panic_template(m)
                .map_err(|e| (DiagnosticCode::W0807, format!("op `{op}`: {e}")))?;
            if self.panic_messages[..i].contains(m) {
                return Err((
                    DiagnosticCode::W0807,
                    format!("op `{op}`: panic template `{m}` is declared twice"),
                ));
            }
        }
        Ok(())
    }
}

/// The most cases an `exhaustive_budget` may ask the generated harness to
/// enumerate: 2^26.  gf256's three-binder field laws need 2^24; anything above
/// 2^26 is a harness that cannot finish in CI, and an unbounded value is a
/// denial-of-service lever on the generator's consumers.
pub const MAX_EXHAUSTIVE_BUDGET: u64 = 1 << 26;

/// Is `text` a plain Rust identifier, `[A-Za-z_][A-Za-z0-9_]*`?  Exports and
/// crate names are spliced into generated Rust source, so anything else (a
/// `;`, a quote, a path that escapes) must be refused before it gets there.
pub fn is_rust_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Is `text` an acceptable package directory name (the `<package>` argument
/// of `gen_evidence`)?  `[A-Za-z0-9][A-Za-z0-9_-]*`: no separators, no dots,
/// so it can never be absolute or contain `..`.
pub fn is_package_dir_name(text: &str) -> bool {
    let mut chars = text.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphanumeric())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// A repository-relative path made only of ordinary components: no leading
/// `/`, no drive letter, no backslash, no `.` or `..` component, no NUL.
fn plain_relative_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains(['\\', '\0', ':'])
        && path
            .split('/')
            .all(|c| !c.is_empty() && c != "." && c != ".." && c.chars().all(|ch| !ch.is_control()))
}

impl EvidenceBindingFile {
    /// Pin the file's `spec` and `output` to the package directory `package`
    /// (the `<package>` argument): `spec` must be `code/specs/<package>.warrant`
    /// and `output` a single `.rs` file directly under
    /// `code/packages/rust/<package>/tests/generated/`.  A bindings file can
    /// therefore not make the generator read some other document or write
    /// outside the package's generated-test directory.
    pub fn validate_paths(&self, package: &str) -> Result<(), String> {
        if !is_package_dir_name(package) {
            return Err(format!("package `{package}` must match [A-Za-z0-9][A-Za-z0-9_-]*"));
        }
        if !plain_relative_path(&self.spec) {
            return Err(format!("spec `{}` must be a plain relative path (no `..`, no absolute path)", self.spec));
        }
        if !plain_relative_path(&self.output) {
            return Err(format!("output `{}` must be a plain relative path (no `..`, no absolute path)", self.output));
        }
        let want_spec = format!("code/specs/{package}.warrant");
        if self.spec != want_spec {
            return Err(format!("spec `{}` must be `{want_spec}`", self.spec));
        }
        let dir = format!("code/packages/rust/{package}/tests/generated/");
        match self.output.strip_prefix(&dir) {
            Some(file) if !file.is_empty() && !file.contains('/') && file.ends_with(".rs") => Ok(()),
            _ => Err(format!("output `{}` must be a .rs file directly under `{dir}`", self.output)),
        }
    }
}

fn parse_u64(text: &str) -> Result<u64, String> {
    let t = text.replace('_', "");
    let parsed = match t.strip_prefix("0x") {
        Some(h) => u64::from_str_radix(h, 16),
        None => t.parse::<u64>(),
    };
    parsed.map_err(|_| format!("`{text}` is not an unsigned integer"))
}

/// Parse an `evidence.bindings` file (see [`EvidenceBindingFile`]).
pub fn parse_evidence_bindings(text: &str) -> Result<EvidenceBindingFile, String> {
    let mut package = None;
    let mut revision = None;
    let mut spec = None;
    let mut output = None;
    let mut seed = None;
    let mut property_cases = None;
    let mut fuzz_iterations = None;
    let mut negctl_fuzz_iterations = None;
    let mut exhaustive_budget = None;
    let mut bindings = BTreeMap::new();
    let mut arg_types = BTreeMap::new();
    let mut proofs_not_live = BTreeMap::new();
    let mut hosts = HostSpec::default();
    let mut drivers: BTreeMap<String, DriverRecord> = BTreeMap::new();
    let mut driver_ops: BTreeSet<String> = BTreeSet::new();
    // VER33 4.8: per-op `realizes=` / `embedding=` and the `panic` lines.
    let mut realizes: BTreeMap<String, (String, String, Option<String>)> = BTreeMap::new();
    let mut panics: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (n, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let at = |m: String| format!("line {}: {m}", n + 1);
        let (key, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
        let rest = rest.trim();
        let once = |slot: &mut Option<String>| -> Result<(), String> {
            if slot.is_some() || rest.is_empty() {
                return Err(at(format!("`{key}` needs exactly one value, once")));
            }
            *slot = Some(rest.to_string());
            Ok(())
        };
        let num = |slot: &mut Option<u64>| -> Result<(), String> {
            if slot.is_some() {
                return Err(at(format!("`{key}` appears twice")));
            }
            *slot = Some(parse_u64(rest).map_err(at)?);
            Ok(())
        };
        match key {
            "package" => once(&mut package)?,
            "revision" => once(&mut revision)?,
            "spec" => once(&mut spec)?,
            "output" => once(&mut output)?,
            "seed" => num(&mut seed)?,
            "property_cases" => num(&mut property_cases)?,
            "fuzz_iterations" => num(&mut fuzz_iterations)?,
            "negctl_fuzz_iterations" => num(&mut negctl_fuzz_iterations)?,
            "exhaustive_budget" => num(&mut exhaustive_budget)?,
            "struct" | "enum" => {
                let mut words = rest.split_whitespace();
                let name = words.next().ok_or_else(|| at(format!("`{key}` needs a type name")))?;
                if !is_rust_identifier(name) {
                    return Err(at(format!("type name `{name}` is not an identifier")));
                }
                let (mut path, mut body) = (None, None);
                for w in words {
                    let (k, v) = w.split_once('=').unwrap_or((w, ""));
                    match (k, v.is_empty()) {
                        ("path", false) => path = Some(v.to_string()),
                        ("fields", false) if key == "struct" => body = Some(v.to_string()),
                        ("ctors", false) if key == "enum" => body = Some(v.to_string()),
                        _ => return Err(at(format!("{key} `{name}`: unknown or malformed option `{w}`"))),
                    }
                }
                let path = path.ok_or_else(|| at(format!("{key} `{name}` has no path=")))?;
                if !is_rust_path(&path) {
                    return Err(at(format!("path `{path}` is not a Rust path")));
                }
                let body = body.ok_or_else(|| {
                    at(format!("{key} `{name}` has no {}=", if key == "struct" { "fields" } else { "ctors" }))
                })?;
                let shape = if key == "struct" {
                    HostShape::Struct(parse_host_fields(&body).map_err(at)?)
                } else {
                    let mut ctors: Vec<(String, Vec<HostField>)> = Vec::new();
                    for part in split_top(&body) {
                        let (cname, fields) = match part.split_once('(') {
                            Some((c, f)) => (
                                c,
                                parse_host_fields(f.strip_suffix(')').ok_or_else(|| {
                                    at(format!("constructor `{part}` is missing its `)`"))
                                })?)
                                .map_err(at)?,
                            ),
                            None => (part, Vec::new()),
                        };
                        if !is_rust_identifier(cname) || ctors.iter().any(|(c, _)| c == cname) {
                            return Err(at(format!("constructor name `{cname}` is invalid or repeated")));
                        }
                        if ctors.len() >= MAX_HOST_ITEMS {
                            return Err(at("too many constructors".to_string()));
                        }
                        ctors.push((cname.to_string(), fields));
                    }
                    HostShape::Enum(ctors)
                };
                if hosts.types.insert(name.to_string(), HostType { path, shape }).is_some() {
                    return Err(at(format!("{key} `{name}` is declared twice")));
                }
            }
            "op" => {
                let mut words = rest.split_whitespace();
                let name = words.next().ok_or_else(|| at("`op` needs a name".into()))?;
                let mut export = None;
                let mut driver_flag = false;
                let mut static_params = Vec::new();
                let mut fresh = false;
                let mut error = None;
                let mut faults = Vec::new();
                let mut args = None;
                let mut not_live = None;
                let mut ret = None;
                let mut host_errors = None;
                let mut err_any = None;
                let mut inout: Vec<(String, InoutForm)> = Vec::new();
                let mut realizes_to: Option<(String, String)> = None;
                let mut embedding: Option<String> = None;
                for w in words {
                    let (k, v) = w.split_once('=').unwrap_or((w, ""));
                    let list = || -> Vec<String> {
                        split_top(v)
                            .into_iter()
                            .filter(|s| !s.is_empty())
                            .map(String::from)
                            .collect()
                    };
                    match k {
                        "export" if !v.is_empty() => export = Some(v.to_string()),
                        "driver" if v.is_empty() => driver_flag = true,
                        "static" if !v.is_empty() => static_params = list(),
                        "fresh" if v.is_empty() => fresh = true,
                        "error" if !v.is_empty() => error = Some(v.to_string()),
                        "args" if !v.is_empty() => args = Some(list()),
                        "ret" if !v.is_empty() => ret = Some(parse_host_ty(v).map_err(at)?),
                        "errors" if !v.is_empty() => {
                            host_errors = Some(parse_host_errors(v).map_err(at)?)
                        }
                        "err_any" if is_rust_identifier(v) => err_any = Some(v.to_string()),
                        "inout" if !v.is_empty() => {
                            if !inout.is_empty() {
                                return Err(at(format!("W0501: op `{name}`: `inout=` appears twice")));
                            }
                            inout = parse_inout_entries(v)
                                .map_err(|e| at(format!("W0501: op `{name}`: {e}")))?;
                        }
                        "realizes" if !v.is_empty() => {
                            let (m, f) = v.split_once('.').ok_or_else(|| {
                                at(format!(
                                    "W0806: op `{name}`: `realizes=` must be MODULE.MODEL"
                                ))
                            })?;
                            if realizes_to.is_some() {
                                return Err(at(format!(
                                    "W0806: op `{name}`: `realizes=` appears twice"
                                )));
                            }
                            realizes_to = Some((m.to_string(), f.to_string()));
                        }
                        "embedding" if !v.is_empty() => {
                            if embedding.is_some() {
                                return Err(at(format!(
                                    "W0806: op `{name}`: `embedding=` appears twice"
                                )));
                            }
                            embedding = Some(v.to_string());
                        }
                        "faults" if !v.is_empty() => faults = list(),
                        "proofs_not_live" if !v.is_empty() => not_live = Some(v.to_string()),
                        _ => return Err(at(format!("op `{name}`: unknown or malformed option `{w}`"))),
                    }
                }
                // VER04k 3.4 rule 1: exactly one of `export=` and bare `driver`.
                if export.is_some() && driver_flag {
                    return Err(at(format!(
                        "W0501: op `{name}`: `export=` and `driver` are mutually exclusive"
                    )));
                }
                if export.is_none() && !driver_flag {
                    return Err(at(format!(
                        "W0501: op `{name}` needs exactly one of `export=` and `driver`"
                    )));
                }
                if driver_flag {
                    driver_ops.insert(name.to_string());
                }
                match (realizes_to, embedding) {
                    (Some((m, f)), Some(e)) => {
                        realizes.insert(name.to_string(), (m, f, Some(e)));
                    }
                    (Some(_), None) => {
                        return Err(at(format!(
                            "W0806: op `{name}`: `realizes=` needs `embedding=` (a canonical embedding is always named)"
                        )));
                    }
                    (None, Some(_)) => {
                        return Err(at(format!(
                            "W0806: op `{name}`: `embedding=` without `realizes=`"
                        )));
                    }
                    (None, None) => {}
                }
                if let Some(export) = export.as_deref() {
                    if inout.is_empty() {
                        if !is_rust_identifier(export) {
                            return Err(at(format!("op `{name}`: export `{export}` is not a Rust identifier")));
                        }
                    } else if !is_rust_path(export) || export.split("::").count() > MAX_EXPORT_SEGMENTS {
                        // VER04k section 4.6: an inout operation may name a method or a
                        // function by a Rust path of at most 8 segments.
                        return Err(at(format!(
                            "W0501: op `{name}`: export `{export}` is not a Rust path of at most {MAX_EXPORT_SEGMENTS} segments"
                        )));
                    }
                }
                if let Some(reason) = not_live {
                    proofs_not_live.insert(name.to_string(), reason);
                }
                if faults.is_empty() {
                    return Err(at(format!("op `{name}` needs faults= (at least one)")));
                }
                let host_args = args.as_ref().and_then(|a| {
                    a.iter()
                        .map(|t| parse_host_ty(t).ok())
                        .collect::<Option<Vec<_>>>()
                });
                if let Some(a) = args {
                    arg_types.insert(name.to_string(), a);
                }
                if err_any.is_some() && (error.is_some() || host_errors.is_some()) {
                    return Err(at(format!(
                        "op `{name}`: err_any= excludes error= and errors="
                    )));
                }
                if host_args.is_some()
                    || ret.is_some()
                    || host_errors.is_some()
                    || err_any.is_some()
                    || !inout.is_empty()
                {
                    hosts.ops.insert(
                        name.to_string(),
                        OpHost {
                            args: host_args,
                            ret,
                            errors: host_errors,
                            err_any,
                            inout,
                        },
                    );
                }
                let dup = bindings.insert(
                    name.to_string(),
                    OpBinding {
                        export,
                        driver: None,
                        static_params,
                        fresh,
                        error_normalization: error.map(|c| ErrorNormalization {
                            kind: "panic_to_error".into(),
                            error_case: c,
                        }),
                        negative_controls: faults,
                    },
                );
                if dup.is_some() {
                    return Err(at(format!("op `{name}` is bound twice")));
                }
            }
            "panic" => {
                // `panic OP TEMPLATE...`: the template is the rest of the line,
                // verbatim apart from the trim the line already had.
                let (name, template) = rest.split_once(char::is_whitespace).ok_or_else(|| {
                    at("W0807: `panic` needs an operation name and a template".to_string())
                })?;
                let template = template.trim();
                check_panic_template(template)
                    .map_err(|e| at(format!("W0807: op `{name}`: {e}")))?;
                panics
                    .entry(name.to_string())
                    .or_default()
                    .push(template.to_string());
            }
            "driver" => {
                let mut words = rest.split_whitespace();
                let name = words
                    .next()
                    .ok_or_else(|| at("`driver` needs an operation name".into()))?;
                if !is_rust_identifier(name) {
                    return Err(at(format!(
                        "W0501: driver operation `{name}` is not an identifier"
                    )));
                }
                let rest_words: Vec<&str> = words.collect();
                let record = crate::driver::parse_driver_record(&rest_words)
                    .map_err(|e| at(format!("W0501: driver `{name}`: {e}")))?;
                if drivers.insert(name.to_string(), record).is_some() {
                    return Err(at(format!(
                        "W0501: driver `{name}` is declared twice"
                    )));
                }
            }
            other => return Err(at(format!("unknown key `{other}`"))),
        }
    }
    // VER04k 3.4 rule 1, cross-line half: every `driver` line names an
    // operation whose `op` line carries `driver`, and every such operation
    // has exactly one `driver` line.
    for name in drivers.keys() {
        let binding = bindings.get(name).ok_or_else(|| {
            format!("W0501: driver `{name}` names an unknown operation")
        })?;
        if binding.export.is_some() {
            return Err(format!(
                "W0501: driver `{name}` names operation `{name}`, whose `op` line lacks `driver`"
            ));
        }
    }
    for name in driver_ops {
        let record = drivers.remove(&name).ok_or_else(|| {
            format!("W0501: op `{name}` carries `driver` but has no `driver` line")
        })?;
        if let Some(binding) = bindings.get_mut(&name) {
            binding.driver = Some(record);
        }
    }
    // VER33 4.8, cross-line half: a `panic` line belongs to a realizing op, and
    // a realization's shape is checked as soon as the file is read.
    for name in panics.keys() {
        if !realizes.contains_key(name) {
            return Err(format!(
                "W0806: `panic` line for op `{name}`, which has no `realizes=`"
            ));
        }
    }
    let mut realizations = BTreeMap::new();
    for (name, (module, model, embedding)) in realizes {
        let r = Realization {
            module,
            model,
            embedding: embedding.unwrap_or_default(),
            panic_messages: panics.remove(&name).unwrap_or_default(),
        };
        r.check_shape(&name)
            .map_err(|(code, why)| format!("{code}: {why}"))?;
        realizations.insert(name, r);
    }
    let need = |v: Option<String>, k: &str| v.ok_or_else(|| format!("missing `{k}`"));
    let need_n = |v: Option<u64>, k: &str| v.ok_or_else(|| format!("missing `{k}`"));
    if bindings.is_empty() {
        return Err("no `op` lines".into());
    }
    // Every host type a signature or field names must be declared.
    let mut named = Vec::new();
    for t in hosts.types.values() {
        match &t.shape {
            HostShape::Struct(fs) => fs.iter().for_each(|f| host_names(&f.ty, &mut named)),
            HostShape::Enum(cs) => cs
                .iter()
                .flat_map(|(_, fs)| fs)
                .for_each(|f| host_names(&f.ty, &mut named)),
        }
    }
    for o in hosts.ops.values() {
        o.args.iter().flatten().for_each(|t| host_names(t, &mut named));
        o.ret.iter().for_each(|t| host_names(t, &mut named));
    }
    if let Some(missing) = named.iter().find(|n| !hosts.types.contains_key(*n)) {
        return Err(format!("host type `{missing}` is used but has no `struct` or `enum` line"));
    }
    let package = need(package, "package")?;
    if !is_rust_identifier(&package) {
        return Err(format!("package `{package}` is not a Rust identifier"));
    }
    let spec = need(spec, "spec")?;
    let output = need(output, "output")?;
    for (key, path) in [("spec", &spec), ("output", &output)] {
        if !plain_relative_path(path) {
            return Err(format!("{key} `{path}` must be a plain relative path (no `..`, no absolute path)"));
        }
    }
    let exhaustive_budget = exhaustive_budget.unwrap_or(400_000);
    if exhaustive_budget > MAX_EXHAUSTIVE_BUDGET {
        return Err(format!(
            "exhaustive_budget {exhaustive_budget} exceeds the maximum {MAX_EXHAUSTIVE_BUDGET} (2^26)"
        ));
    }
    Ok(EvidenceBindingFile {
        manifest: BindingManifest {
            package,
            revision: need(revision, "revision")?,
            bindings,
        },
        spec,
        output,
        seed: need_n(seed, "seed")?,
        property_cases: need_n(property_cases, "property_cases")?,
        fuzz_iterations: need_n(fuzz_iterations, "fuzz_iterations")?,
        negctl_fuzz_iterations: need_n(negctl_fuzz_iterations, "negctl_fuzz_iterations")?,
        exhaustive_budget,
        arg_types,
        proofs_not_live,
        hosts,
        realizations,
    })
}

/// Validates that a host binding manifest correctly and completely covers a checked Warrant module.
pub fn validate_bindings(
    module: &CheckedWarrantModule,
    manifest: &BindingManifest,
) -> Result<(), CheckerError> {
    // 1. Every op declared in the Warrant document must have a binding (W0501)
    for (op_name, op) in &module.operations {
        let binding = manifest.bindings.get(op_name).ok_or_else(|| CheckerError {
            code: DiagnosticCode::W0501,
            kind: CheckerErrorKind::MissingBinding(format!(
                "Op {op_name} is declared in Warrant but has no binding in manifest"
            )),
            span: op.span,
        })?;

        // 2. Freshness requirement: if op declared fresh, binding must assert fresh
        if op.is_fresh && !binding.fresh {
            return Err(CheckerError {
                code: DiagnosticCode::W0502,
                kind: CheckerErrorKind::MissingFreshRequirement(format!(
                    "Op {op_name} is declared fresh in Warrant, but binding does not specify fresh: true"
                )),
                span: op.span,
            });
        }

        // 3. Every bound op must specify at least one negative control (W0502)
        if binding.negative_controls.is_empty() {
            return Err(CheckerError {
                code: DiagnosticCode::W0502,
                kind: CheckerErrorKind::MissingNegativeControl(format!(
                    "Binding for {op_name} must declare at least one negative control"
                )),
                span: op.span,
            });
        }

        // 4. Static parameters must match
        if binding.static_params.len() != op.static_params.len() {
            return Err(CheckerError {
                code: DiagnosticCode::W0501,
                kind: CheckerErrorKind::InvalidContract(format!(
                    "Binding for {op_name} static param count ({}) != op static params ({})",
                    binding.static_params.len(),
                    op.static_params.len()
                )),
                span: op.span,
            });
        }

        // 5. Error normalization validation
        if let Some(norm) = &binding.error_normalization {
            match &op.return_type {
                Type::Result(_, err_name) => {
                    let err_decl = module.errors.get(err_name).ok_or_else(|| CheckerError {
                        code: DiagnosticCode::W0501,
                        kind: CheckerErrorKind::InvalidErrorNormalization(format!(
                            "Op {op_name} return error type {err_name} is not defined"
                        )),
                        span: op.span,
                    })?;

                    if !err_decl.variants.contains(&norm.error_case) {
                        return Err(CheckerError {
                            code: DiagnosticCode::W0501,
                            kind: CheckerErrorKind::InvalidErrorNormalization(format!(
                                "Normalized error case {} is not a variant of error type {err_name}",
                                norm.error_case
                            )),
                            span: op.span,
                        });
                    }
                }
                _ => {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0501,
                        kind: CheckerErrorKind::InvalidErrorNormalization(format!(
                            "Op {op_name} does not return Result, cannot declare error normalization"
                        )),
                        span: op.span,
                    });
                }
            }
        }
    }

    // 6. Manifest bindings cannot refer to unknown ops (W0501)
    for binding_op_name in manifest.bindings.keys() {
        if !module.operations.contains_key(binding_op_name) {
            return Err(CheckerError {
                code: DiagnosticCode::W0501,
                kind: CheckerErrorKind::UnknownBoundOperation(format!(
                    "Binding manifest specifies op {binding_op_name} not found in Warrant module"
                )),
                span: (0, 0),
            });
        }
    }

    Ok(())
}

/// The leaves of a type under an embedding, and whether the embedding admits
/// them.  Containers (`List`, `Option`, `Result`, tuples) embed componentwise.
fn embeds(op_ty: &Type, model_ty: &Type, embedding: &str) -> bool {
    let int = |t: &Type| {
        matches!(
            t,
            Type::Int
                | Type::Nat
                | Type::U8
                | Type::U16
                | Type::U32
                | Type::U64
                | Type::I8
                | Type::I16
                | Type::I32
                | Type::I64
        )
    };
    match (op_ty, model_ty) {
        (Type::Bool, Type::Bool) => true,
        (a, Type::Int) if embedding == "int-by-value" => int(a),
        (Type::Bytes | Type::FixedBytes(_), Type::Bytes) if embedding == "bytes-identity" => true,
        (Type::FixedBytes(a), Type::FixedBytes(b)) if embedding == "bytes-identity" => a == b,
        (Type::List(a), Type::List(b)) | (Type::Option(a), Type::Option(b)) => {
            embeds(a, b, embedding)
        }
        (Type::Result(a, ea), Type::Result(b, eb)) => ea == eb && embeds(a, b, embedding),
        (Type::Tuple(a), Type::Tuple(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(x, y)| embeds(x, y, embedding))
        }
        _ => false,
    }
}

/// A checked model file offered to [`validate_realizations`], with the profile
/// its header declared.  The profile travels beside the checked module, not in
/// it: `CheckedWarrantModule`'s `Debug` output is pinned byte for byte by the
/// v0 compatibility proofs, so it must not grow a field for a pure document.
#[derive(Debug, Clone, Copy)]
pub struct ModelModule<'a> {
    module: &'a CheckedWarrantModule,
    profile: Profile,
}

impl<'a> ModelModule<'a> {
    /// Offer `module`, the checked form of `file`: the profile is read from the
    /// parsed file itself, never supplied by the caller, so a pure document
    /// cannot be presented as an algorithm one.  A pair whose names differ is
    /// treated as pure (and so refused by [`validate_realizations`]).
    pub fn new(file: &crate::parser::WarrantFile<'_>, module: &'a CheckedWarrantModule) -> Self {
        Self {
            module,
            profile: if file.name == module.module_name {
                file.profile
            } else {
                Profile::Pure
            },
        }
    }
}

/// Validates every `realizes` binding of `op_module` against the model modules
/// (VER33 section 4.8).  Schema and validation only: nothing uses the result.
///
/// For each realizing op the validator refuses (all `W0806` unless noted):
///
/// ```text
/// an op the document does not declare
/// an unknown embedding, or a non-identifier module/model name
/// a model module that is not provided, or is not `profile algorithm`
/// a model function the module does not define
/// a different number of parameters
/// a parameter or result type the embedding does not map onto the model's
/// an empty, wildcard, duplicate or compiler-generated panic template  (W0807)
/// ```
pub fn validate_realizations(
    op_module: &CheckedWarrantModule,
    realizations: &BTreeMap<String, Realization>,
    model_modules: &[ModelModule<'_>],
) -> Result<(), CheckerError> {
    let fail = |code: DiagnosticCode, msg: String, span: (usize, usize)| CheckerError {
        code,
        kind: CheckerErrorKind::InvalidRealization(msg),
        span,
    };
    for (i, m) in model_modules.iter().enumerate() {
        if model_modules[..i]
            .iter()
            .any(|o| o.module.module_name == m.module.module_name)
        {
            return Err(fail(
                DiagnosticCode::W0806,
                format!("model module `{}` is offered twice", m.module.module_name),
                (0, 0),
            ));
        }
    }
    for (op_name, r) in realizations {
        let op = op_module.operations.get(op_name).ok_or_else(|| {
            fail(
                DiagnosticCode::W0806,
                format!("realization names op `{op_name}`, which the document does not declare"),
                (0, 0),
            )
        })?;
        r.check_shape(op_name)
            .map_err(|(code, msg)| fail(code, msg, op.span))?;
        let module = model_modules
            .iter()
            .find(|m| m.module.module_name == r.module)
            .ok_or_else(|| {
                fail(
                    DiagnosticCode::W0806,
                    format!(
                        "op `{op_name}` realizes module `{}`, which was not provided",
                        r.module
                    ),
                    op.span,
                )
            })?;
        if module.profile != Profile::Algorithm {
            return Err(fail(
                DiagnosticCode::W0806,
                format!(
                    "op `{op_name}`: module `{}` is not in `profile algorithm`",
                    r.module
                ),
                op.span,
            ));
        }
        let model = module.module.models.get(&r.model).ok_or_else(|| {
            fail(
                DiagnosticCode::W0806,
                format!(
                    "op `{op_name}`: module `{}` has no model function `{}`",
                    r.module, r.model
                ),
                op.span,
            )
        })?;
        if model.params.len() != op.params.len() {
            return Err(fail(
                DiagnosticCode::W0806,
                format!(
                    "op `{op_name}` has {} parameters but model `{}` has {}",
                    op.params.len(),
                    r.model,
                    model.params.len()
                ),
                op.span,
            ));
        }
        for (i, ((pn, op_ty), (_, model_ty))) in op.params.iter().zip(&model.params).enumerate() {
            if !embeds(op_ty, model_ty, &r.embedding) {
                return Err(fail(
                    DiagnosticCode::W0806,
                    format!(
                        "op `{op_name}` parameter {i} `{pn}`: `{}` does not embed into model `{}` under `{}`",
                        op_ty.display_name(),
                        model_ty.display_name(),
                        r.embedding
                    ),
                    op.span,
                ));
            }
        }
        if !embeds(&op.return_type, &model.target_type, &r.embedding) {
            return Err(fail(
                DiagnosticCode::W0806,
                format!(
                    "op `{op_name}` result `{}` does not embed into model result `{}` under `{}`",
                    op.return_type.display_name(),
                    model.target_type.display_name(),
                    r.embedding
                ),
                op.span,
            ));
        }
    }
    Ok(())
}
