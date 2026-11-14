//! # Warrant Rust Binding Schema and Validator (VER04d §5 / VER-P007)
//!
//! Enforces that real public exports correspond to abstract Warrant operations,
//! validating signature compatibility, freshness assertions, negative controls,
//! error normalizations, and closed failure codes (`W0501`, `W0502`).

use std::collections::BTreeMap;

use crate::checker::{CheckedWarrantModule, CheckerError, CheckerErrorKind, DiagnosticCode, Type};

/// Closed error normalization rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorNormalization {
    pub kind: String, // e.g. "panic_to_error"
    pub error_case: String,
}

/// Binding specification for a single Warrant public operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpBinding {
    pub export: String,
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
fn parse_host_errors(text: &str) -> Result<HostErrors, String> {
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
/// Keys other than `op` appear once.  `op` options: `export=` (required),
/// `static=A,B`, bare `fresh`, `error=CASE` (panic-to-error normalization),
/// `args=u8,u32` (Rust parameter types of a scalar export), `faults=a,b`
/// (required, at least one), `proofs_not_live=REASON` (the operation's proof
/// obligations are acknowledged as not runnable; see
/// [`EvidenceBindingFile::proofs_not_live`]).  `package` and every `export` must
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
                let mut static_params = Vec::new();
                let mut fresh = false;
                let mut error = None;
                let mut faults = Vec::new();
                let mut args = None;
                let mut not_live = None;
                let mut ret = None;
                let mut host_errors = None;
                let mut err_any = None;
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
                        "static" if !v.is_empty() => static_params = list(),
                        "fresh" if v.is_empty() => fresh = true,
                        "error" if !v.is_empty() => error = Some(v.to_string()),
                        "args" if !v.is_empty() => args = Some(list()),
                        "ret" if !v.is_empty() => ret = Some(parse_host_ty(v).map_err(at)?),
                        "errors" if !v.is_empty() => {
                            host_errors = Some(parse_host_errors(v).map_err(at)?)
                        }
                        "err_any" if is_rust_identifier(v) => err_any = Some(v.to_string()),
                        "faults" if !v.is_empty() => faults = list(),
                        "proofs_not_live" if !v.is_empty() => not_live = Some(v.to_string()),
                        _ => return Err(at(format!("op `{name}`: unknown or malformed option `{w}`"))),
                    }
                }
                let export = export.ok_or_else(|| at(format!("op `{name}` has no export=")))?;
                if !is_rust_identifier(&export) {
                    return Err(at(format!("op `{name}`: export `{export}` is not a Rust identifier")));
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
                if host_args.is_some() || ret.is_some() || host_errors.is_some() || err_any.is_some() {
                    hosts.ops.insert(
                        name.to_string(),
                        OpHost {
                            args: host_args,
                            ret,
                            errors: host_errors,
                            err_any,
                        },
                    );
                }
                let dup = bindings.insert(
                    name.to_string(),
                    OpBinding {
                        export,
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
            other => return Err(at(format!("unknown key `{other}`"))),
        }
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
