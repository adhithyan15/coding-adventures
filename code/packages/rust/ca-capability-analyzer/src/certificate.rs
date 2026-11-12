//! # Effect Proof Certificate & Small Standalone Checker (VER24, VER23a)
//!
//! Provides the canonical schema (`ver24-effect-cert-v1`), certificate generation,
//! and the independent small checker (VER00 §8.6.5).
//!
//! ## The checker works by recomputation
//!
//! ```text
//!   certificate.json ──strict parse──► parsed
//!   package on disk  ──recompute────► expected      parsed == expected, field by field
//! ```
//!
//! Rebuilding the whole expected certificate from the package and comparing every
//! field means no field of the file is decorative: flip any one and verification
//! fails.  (The previous checker ignored `unconditional_roots`, exception
//! justifications, unknown fields, and matched entry points by name only.)
//!
//! ## What the hashes cover (VER23a §3.1)
//!
//! * `source_digest`: every `.rs` file under `src/` except exactly
//!   `src/capabilities.rs`; paths relative to `src/`, sorted bytewise; CRLF
//!   normalised; framed as `u64_be(len(path)) path u64_be(len(bytes)) bytes`.
//! * `manifest_digest`: `required_capabilities.json`, CRLF normalised (empty if absent).
//! * NOT covered: `Cargo.toml`, `build.rs`, files outside `src/`, data files.
//!   Verification therefore also re-runs the full analysis, which does cover them.

#![deny(unsafe_code)]

use std::fs;
use std::path::Path;

use crate::manifest::{load_manifest, JsonValue, ManifestData, MAX_MANIFEST_SIZE_BYTES};
use crate::parser::public_fn_signatures;
use crate::scanner::{MAX_DIR_DEPTH, MAX_FILE_SIZE_BYTES, MAX_SOURCE_FILES, MAX_TOTAL_SOURCE_BYTES};
use crate::sha256::{sha256_hex, Sha256};

pub const CERT_SCHEMA_VERSION: &str = "ver24-effect-cert-v1";

/// Normalize CRLF bytes to LF for deterministic cross-platform digest hashing.
pub fn normalize_crlf(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\r' && i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
            i += 1; // skip \r
        }
        out.push(bytes[i]);
        i += 1;
    }
    out
}

/// A certified public entry point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryPointCert {
    pub name: String,
    pub signature: String,
    pub effects: Vec<String>,
}

/// An audited banned construct exception in the certificate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BannedExceptionCert {
    pub construct: String,
    pub language: String,
    pub justification: String,
}

/// Canonical effect proof certificate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectCertificate {
    pub schema_version: String,
    pub package: String,
    pub source_digest: String,
    pub manifest_digest: String,
    pub declared_effects: Vec<String>,
    pub entry_points: Vec<EntryPointCert>,
    pub unconditional_roots: Vec<String>,
    pub banned_construct_exceptions: Vec<BannedExceptionCert>,
    pub label: String,
}

/// Quote and escape a string as a JSON string literal (RFC 8259 §7).
///
/// The old writer escaped only `"`, so a justification ending in a backslash or
/// containing a newline produced invalid (or differently parsed) JSON.
pub fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Render `items` as a JSON array of strings at the given indent (in spaces).
fn string_array(items: &[String], indent: usize, trailing_comma: bool) -> String {
    let pad = " ".repeat(indent);
    let tail = if trailing_comma { "," } else { "" };
    if items.is_empty() {
        return format!("[]{tail}\n");
    }
    let mut out = String::from("[\n");
    for (i, it) in items.iter().enumerate() {
        out.push_str(&format!("{pad}  {}", json_str(it)));
        if i + 1 < items.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str(&format!("{pad}]{tail}\n"));
    out
}

impl EffectCertificate {
    /// Format certificate as canonical, deterministic JSON.
    pub fn to_json_string(&self) -> String {
        let mut out = String::new();
        out.push_str("{\n");
        out.push_str(&format!("  \"schema_version\": {},\n", json_str(&self.schema_version)));
        out.push_str(&format!("  \"package\": {},\n", json_str(&self.package)));
        out.push_str(&format!("  \"source_digest\": {},\n", json_str(&self.source_digest)));
        out.push_str(&format!("  \"manifest_digest\": {},\n", json_str(&self.manifest_digest)));
        out.push_str("  \"declared_effects\": ");
        out.push_str(&string_array(&self.declared_effects, 2, true));

        out.push_str("  \"entry_points\": ");
        if self.entry_points.is_empty() {
            out.push_str("[],\n");
        } else {
            out.push_str("[\n");
            for (i, ep) in self.entry_points.iter().enumerate() {
                out.push_str("    {\n");
                out.push_str(&format!("      \"name\": {},\n", json_str(&ep.name)));
                out.push_str(&format!("      \"signature\": {},\n", json_str(&ep.signature)));
                out.push_str("      \"effects\": ");
                out.push_str(&string_array(&ep.effects, 6, false));
                out.push_str("    }");
                if i + 1 < self.entry_points.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            out.push_str("  ],\n");
        }

        out.push_str("  \"unconditional_roots\": ");
        out.push_str(&string_array(&self.unconditional_roots, 2, true));

        out.push_str("  \"banned_construct_exceptions\": ");
        if self.banned_construct_exceptions.is_empty() {
            out.push_str("[],\n");
        } else {
            out.push_str("[\n");
            for (i, ex) in self.banned_construct_exceptions.iter().enumerate() {
                out.push_str("    {\n");
                out.push_str(&format!("      \"construct\": {},\n", json_str(&ex.construct)));
                out.push_str(&format!("      \"language\": {},\n", json_str(&ex.language)));
                out.push_str(&format!("      \"justification\": {}\n", json_str(&ex.justification)));
                out.push_str("    }");
                if i + 1 < self.banned_construct_exceptions.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            out.push_str("  ],\n");
        }

        out.push_str(&format!("  \"label\": {}\n", json_str(&self.label)));
        out.push_str("}\n");
        out
    }
}

/// Compute canonical SHA-256 digest of package sources (see module docs).
pub fn compute_source_digest(package_dir: &Path) -> Result<String, String> {
    let src_dir = package_dir.join("src");
    if !src_dir.exists() {
        return Err(format!("Source directory does not exist: {}", src_dir.display()));
    }

    let mut files = Vec::new();
    let mut total_bytes: u64 = 0;
    collect_rs_files(&src_dir, &src_dir, &mut files, 0, &mut total_bytes)?;
    files.sort_by(|a, b| a.0.cmp(&b.0));

    let mut hasher = Sha256::new();
    for (rel_path, content) in files {
        hasher.update(&(rel_path.len() as u64).to_be_bytes());
        hasher.update(rel_path.as_bytes());
        hasher.update(&(content.len() as u64).to_be_bytes());
        hasher.update(&content);
    }

    let digest = hasher.finalize();
    let mut s = String::with_capacity(64);
    for b in digest {
        use std::fmt::Write;
        let _ = write!(s, "{:02x}", b);
    }
    Ok(s)
}

fn collect_rs_files(
    root: &Path,
    current: &Path,
    out: &mut Vec<(String, Vec<u8>)>,
    depth: usize,
    total_bytes: &mut u64,
) -> Result<(), String> {
    if depth > MAX_DIR_DEPTH {
        return Err(format!("Directory depth exceeded maximum limit of {MAX_DIR_DEPTH}"));
    }

    let entries = fs::read_dir(current)
        .map_err(|e| format!("Failed to read directory {}: {e}", current.display()))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Directory entry error: {e}"))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|e| format!("Failed to get file type for {}: {e}", path.display()))?;

        // A symlink is refused, exactly as the scanner refuses it.  (It used to be
        // skipped silently here, so the digest and the analysis disagreed.)
        if file_type.is_symlink() {
            return Err(format!("symlink refused at {}", path.display()));
        }

        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        // Hidden directories are skipped (same rule as the scanner).
        if file_type.is_dir() {
            if file_name.starts_with('.') {
                continue;
            }
            collect_rs_files(root, &path, out, depth + 1, total_bytes)?;
            continue;
        }
        if !(file_type.is_file() && file_name.ends_with(".rs")) {
            continue;
        }

        let rel = path
            .strip_prefix(root)
            .map_err(|e| format!("Path strip prefix error: {e}"))?;
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        // Only the top-level generated module is exempt (VER23a §3.1).
        if rel_str == "capabilities.rs" {
            continue;
        }

        if out.len() >= MAX_SOURCE_FILES {
            return Err(format!("Source file count exceeded limit of {MAX_SOURCE_FILES}"));
        }
        let meta = fs::metadata(&path)
            .map_err(|e| format!("Failed to get metadata for {}: {e}", path.display()))?;
        if meta.len() > MAX_FILE_SIZE_BYTES {
            return Err(format!(
                "File {} exceeds size limit of {MAX_FILE_SIZE_BYTES} bytes",
                path.display()
            ));
        }
        let raw = fs::read(&path).map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
        let content = normalize_crlf(&raw);
        *total_bytes += content.len() as u64;
        if *total_bytes > MAX_TOTAL_SOURCE_BYTES {
            return Err(format!("Total source bytes exceeded limit of {MAX_TOTAL_SOURCE_BYTES}"));
        }
        out.push((rel_str, content));
    }
    Ok(())
}

/// Compute SHA-256 digest of `required_capabilities.json` (CRLF normalised).
pub fn compute_manifest_digest(package_dir: &Path) -> Result<String, String> {
    let manifest_path = package_dir.join("required_capabilities.json");
    if manifest_path.exists() {
        let meta = fs::metadata(&manifest_path)
            .map_err(|e| format!("Failed to get metadata for {}: {e}", manifest_path.display()))?;
        if meta.len() > MAX_MANIFEST_SIZE_BYTES {
            return Err(format!("Manifest exceeds maximum size limit of {MAX_MANIFEST_SIZE_BYTES}"));
        }
        let bytes = fs::read(&manifest_path)
            .map_err(|e| format!("Failed to read {}: {e}", manifest_path.display()))?;
        Ok(sha256_hex(&normalize_crlf(&bytes)))
    } else {
        Ok(sha256_hex(b""))
    }
}

/// Lexically discover public functions in `src/lib.rs` (VER23a §3.3).
///
/// `effects` is left empty here; [`generate_certificate`] fills in the sound
/// over-approximation (every declared effect).
pub fn discover_public_entry_points(package_dir: &Path) -> Result<Vec<EntryPointCert>, String> {
    let lib_rs = package_dir.join("src").join("lib.rs");
    if !lib_rs.exists() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(&lib_rs)
        .map_err(|e| format!("Failed to read {}: {e}", lib_rs.display()))?;
    let sigs = public_fn_signatures(&raw).map_err(|e| format!("{}: {e}", lib_rs.display()))?;
    Ok(sigs
        .into_iter()
        .map(|(name, signature)| EntryPointCert {
            name,
            signature,
            effects: Vec::new(),
        })
        .collect())
}

fn package_name(package_dir: &Path, manifest: &ManifestData) -> String {
    if manifest.is_explicit {
        manifest.package.clone()
    } else {
        format!(
            "rust/{}",
            package_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
        )
    }
}

fn declared_effects_of(manifest: &ManifestData) -> Vec<String> {
    manifest.capabilities.iter().map(|c| c.exact_string()).collect()
}

fn exceptions_of(manifest: &ManifestData) -> Vec<BannedExceptionCert> {
    let mut v: Vec<BannedExceptionCert> = manifest
        .banned_exceptions
        .values()
        .map(|e| BannedExceptionCert {
            construct: e.construct.clone(),
            language: e.language.clone(),
            justification: e.justification.clone(),
        })
        .collect();
    v.sort_by(|a, b| a.construct.cmp(&b.construct));
    v
}

/// Is `raw` (a `Cargo.toml` `[lib] path`) the default library root `src/lib.rs`?
/// `./src/lib.rs` and `src//lib.rs` are; anything else is not.
fn is_default_lib_root(raw: &str) -> bool {
    let p = raw.replace('\\', "/");
    let comps: Vec<&str> = p.split('/').filter(|c| !c.is_empty() && *c != ".").collect();
    !p.starts_with('/') && comps == ["src", "lib.rs"]
}

/// The certificate's entry points are the `pub fn`s of `src/lib.rs`, and its
/// `source_digest` covers `src/**/*.rs`.  Both describe the library only if
/// `src/lib.rs` *is* the library root.  `[lib] path = "src/real.rs"` (or a
/// root outside `src/`) compiles another file, so the certificate would list
/// a decoy's entry points (security review round 4, H5, `cert-nonrs.json`
/// listed `decoy` while `lib.txt` was compiled).  Such a package is refused
/// rather than certified from the wrong file.
fn require_default_lib_root(package_dir: &Path) -> Result<(), String> {
    let facts = crate::cargo::analyze_cargo_toml(package_dir)?;
    match facts.lib_path {
        Some(p) if !is_default_lib_root(&p) => Err(format!(
            "refusing certificate: Cargo.toml [lib] path = \"{p}\"; certificate entry points and source_digest are defined for the library root src/lib.rs only"
        )),
        _ => Ok(()),
    }
}

/// Recompute every certificate field from disk, without judging cleanliness.
fn expected_certificate(package_dir: &Path, manifest: &ManifestData) -> Result<EffectCertificate, String> {
    require_default_lib_root(package_dir)?;
    let declared_effects = declared_effects_of(manifest);
    let mut entry_points = discover_public_entry_points(package_dir)?;
    // No per-entry attribution exists (no call graph): record the sound
    // over-approximation that each entry may use any declared effect.
    for ep in &mut entry_points {
        ep.effects = declared_effects.clone();
    }
    Ok(EffectCertificate {
        schema_version: CERT_SCHEMA_VERSION.to_string(),
        package: package_name(package_dir, manifest),
        source_digest: compute_source_digest(package_dir)?,
        manifest_digest: compute_manifest_digest(package_dir)?,
        declared_effects,
        entry_points,
        unconditional_roots: Vec::new(),
        banned_construct_exceptions: exceptions_of(manifest),
        label: "proved".to_string(),
    })
}

/// Generate canonical effect certificate for target package.
///
/// Refuses (returns `Err`) unless the whole-package analysis is clean: a
/// certificate labelled `proved` must never exist for a package the analyzer
/// rejects.  (The previous version hard-coded the label and never analysed.)
pub fn generate_certificate(package_dir: &Path) -> Result<EffectCertificate, String> {
    let manifest = load_manifest(package_dir).map_err(|e| format!("Failed to load manifest: {e}"))?;
    let report = crate::analyze_package(package_dir, false)
        .map_err(|e| format!("Static capability analysis failed: {e}"))?;
    if !report.is_clean() {
        let first = report
            .violations
            .first()
            .map(|v| v.format_line())
            .unwrap_or_default();
        return Err(format!(
            "refusing to emit certificate: package analysis found {} violation(s); first: {first}",
            report.violations.len()
        ));
    }
    expected_certificate(package_dir, &manifest)
}

// ---------------------------------------------------------------------------
// Strict parsing
// ---------------------------------------------------------------------------

fn exact_object<'a>(
    val: &'a JsonValue,
    what: &str,
    allowed: &[&str],
) -> Result<&'a std::collections::HashMap<String, JsonValue>, String> {
    let obj = val
        .as_object()
        .ok_or_else(|| format!("{what} must be a JSON object"))?;
    for k in obj.keys() {
        if !allowed.contains(&k.as_str()) {
            return Err(format!("Unknown field '{k}' in {what}"));
        }
    }
    Ok(obj)
}

fn req_str(obj: &std::collections::HashMap<String, JsonValue>, key: &str, what: &str) -> Result<String, String> {
    obj.get(key)
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| format!("Missing or non-string '{key}' in {what}"))
}

fn req_str_array(
    obj: &std::collections::HashMap<String, JsonValue>,
    key: &str,
    what: &str,
) -> Result<Vec<String>, String> {
    let arr = obj
        .get(key)
        .and_then(|v| v.as_array())
        .ok_or_else(|| format!("Missing or non-array '{key}' in {what}"))?;
    arr.iter()
        .map(|v| {
            v.as_str()
                .map(str::to_string)
                .ok_or_else(|| format!("'{key}' element in {what} must be string"))
        })
        .collect()
}

/// Parse effect certificate JSON into `EffectCertificate` (strict: exact field
/// set, exact types, no duplicate keys).
pub fn parse_certificate(json_str: &str) -> Result<EffectCertificate, String> {
    let val = JsonValue::parse(json_str).map_err(|e| format!("Invalid certificate JSON: {e}"))?;
    let obj = exact_object(
        &val,
        "certificate",
        &[
            "schema_version",
            "package",
            "source_digest",
            "manifest_digest",
            "declared_effects",
            "entry_points",
            "unconditional_roots",
            "banned_construct_exceptions",
            "label",
        ],
    )?;

    let schema_version = req_str(obj, "schema_version", "certificate")?;
    if schema_version != CERT_SCHEMA_VERSION {
        return Err(format!(
            "Unsupported certificate schema_version '{}' (expected '{}')",
            schema_version, CERT_SCHEMA_VERSION
        ));
    }

    let mut entry_points = Vec::new();
    let eps = obj
        .get("entry_points")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "Missing or non-array 'entry_points' in certificate".to_string())?;
    for item in eps {
        let o = exact_object(item, "entry_point", &["name", "signature", "effects"])?;
        entry_points.push(EntryPointCert {
            name: req_str(o, "name", "entry_point")?,
            signature: req_str(o, "signature", "entry_point")?,
            effects: req_str_array(o, "effects", "entry_point")?,
        });
    }

    let mut banned = Vec::new();
    let exs = obj
        .get("banned_construct_exceptions")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "Missing or non-array 'banned_construct_exceptions' in certificate".to_string())?;
    for item in exs {
        let o = exact_object(item, "banned exception", &["construct", "language", "justification"])?;
        banned.push(BannedExceptionCert {
            construct: req_str(o, "construct", "banned exception")?,
            language: req_str(o, "language", "banned exception")?,
            justification: req_str(o, "justification", "banned exception")?,
        });
    }

    Ok(EffectCertificate {
        schema_version,
        package: req_str(obj, "package", "certificate")?,
        source_digest: req_str(obj, "source_digest", "certificate")?,
        manifest_digest: req_str(obj, "manifest_digest", "certificate")?,
        declared_effects: req_str_array(obj, "declared_effects", "certificate")?,
        entry_points,
        unconditional_roots: req_str_array(obj, "unconditional_roots", "certificate")?,
        banned_construct_exceptions: banned,
        label: req_str(obj, "label", "certificate")?,
    })
}

/// Verification verdict produced by the small checker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertVerificationResult {
    pub package: String,
    pub label: String,
    pub source_digest: String,
    pub manifest_digest: String,
    pub entry_points_count: usize,
    pub declared_effects_count: usize,
}

/// Small Standalone Certificate Checker (VER00 §8.6.5, VER24, VER23a §4.1).
///
/// Verifies the certificate directly against the target package on disk by
/// recomputing the expected certificate and requiring equality.
pub fn verify_certificate(package_dir: &Path, cert_path: &Path) -> Result<CertVerificationResult, String> {
    if !cert_path.exists() {
        return Err(format!("Certificate file does not exist: {}", cert_path.display()));
    }

    let cert_meta = fs::metadata(cert_path)
        .map_err(|e| format!("Failed to read metadata for {}: {e}", cert_path.display()))?;
    if cert_meta.len() > MAX_MANIFEST_SIZE_BYTES {
        return Err(format!(
            "Certificate exceeds maximum allowed size of {MAX_MANIFEST_SIZE_BYTES} bytes"
        ));
    }

    let cert_content = fs::read_to_string(cert_path)
        .map_err(|e| format!("Failed to read certificate {}: {e}", cert_path.display()))?;

    // 1. Strict parse.
    let cert = parse_certificate(&cert_content)?;

    // 2. Generated capability module on disk matches manifest.
    match crate::generator::check_module_on_disk(package_dir) {
        Ok(true) => {}
        Ok(false) => {
            return Err(
                "Generated capabilities module 'src/capabilities.rs' is missing or differs from manifest"
                    .to_string(),
            );
        }
        Err(e) => return Err(format!("Failed to verify capabilities module: {e}")),
    }

    let manifest = load_manifest(package_dir).map_err(|e| format!("Failed to load manifest: {e}"))?;
    let expected = expected_certificate(package_dir, &manifest)?;

    // 3. Digests.
    if expected.manifest_digest != cert.manifest_digest {
        return Err(format!(
            "Certificate manifest_digest mismatch: expected '{}', computed '{}'",
            cert.manifest_digest, expected.manifest_digest
        ));
    }
    if expected.source_digest != cert.source_digest {
        return Err(format!(
            "Certificate source_digest mismatch: expected '{}', computed '{}'",
            cert.source_digest, expected.source_digest
        ));
    }

    // 4. Package and declared effects.
    if cert.package != expected.package {
        return Err(format!(
            "Certificate package '{}' does not match manifest package '{}'",
            cert.package, expected.package
        ));
    }
    if cert.declared_effects != expected.declared_effects {
        return Err(format!(
            "Certificate declared_effects mismatch: cert declared {:?}, manifest declared {:?}",
            cert.declared_effects, expected.declared_effects
        ));
    }

    // 5. Entry points, positionally.
    if expected.entry_points.len() != cert.entry_points.len() {
        return Err(format!(
            "Entry points count mismatch: found {} in source, certificate declared {}",
            expected.entry_points.len(),
            cert.entry_points.len()
        ));
    }
    for (src_ep, cert_ep) in expected.entry_points.iter().zip(cert.entry_points.iter()) {
        if src_ep.name != cert_ep.name {
            let listed = cert.entry_points.iter().any(|c| c.name == src_ep.name);
            return Err(if listed {
                format!(
                    "Entry point order mismatch: source has '{}' where certificate has '{}'",
                    src_ep.name, cert_ep.name
                )
            } else {
                format!("Uncertified public entry point '{}' found in source", src_ep.name)
            });
        }
        if src_ep.signature != cert_ep.signature {
            return Err(format!(
                "Entry point '{}' signature mismatch: source has '{}', cert declared '{}'",
                src_ep.name, src_ep.signature, cert_ep.signature
            ));
        }
        if cert_ep.effects != src_ep.effects {
            if expected.declared_effects.is_empty() {
                return Err(format!(
                    "Pure package entry point '{}' claims non-empty effects: {:?}",
                    cert_ep.name, cert_ep.effects
                ));
            }
            return Err(format!(
                "Entry point '{}' effects {:?} must equal declared_effects {:?} (no per-entry attribution exists)",
                cert_ep.name, cert_ep.effects, src_ep.effects
            ));
        }
    }

    // 6. Roots and banned-construct exceptions (including justification text).
    if cert.unconditional_roots != expected.unconditional_roots {
        return Err(format!(
            "unconditional_roots must be empty (nothing computes roots), certificate has {:?}",
            cert.unconditional_roots
        ));
    }
    let mut cert_ex = cert.banned_construct_exceptions.clone();
    cert_ex.sort_by(|a, b| a.construct.cmp(&b.construct));
    if cert_ex.len() != expected.banned_construct_exceptions.len() {
        return Err(format!(
            "Banned construct exceptions count mismatch: manifest has {}, cert has {}",
            expected.banned_construct_exceptions.len(),
            cert_ex.len()
        ));
    }
    for (c, m) in cert_ex.iter().zip(expected.banned_construct_exceptions.iter()) {
        if c != m {
            return Err(format!(
                "Certificate declares unverified banned exception '{}:{}' (differs from manifest entry '{}:{}')",
                c.language, c.construct, m.language, m.construct
            ));
        }
    }

    // 7. Whole-package static analysis must be clean.
    let report = crate::analyze_package(package_dir, false)
        .map_err(|e| format!("Static capability analysis failed: {e}"))?;
    if !report.is_clean() {
        return Err(format!(
            "Package static capability analysis failed with {} violations",
            report.violations.len()
        ));
    }

    // 8. Honest label.
    if cert.label != expected.label {
        return Err(format!(
            "Invalid certificate label '{}' (expected 'proved' for verified capability confinement)",
            cert.label
        ));
    }

    Ok(CertVerificationResult {
        package: cert.package,
        label: cert.label,
        source_digest: cert.source_digest,
        manifest_digest: cert.manifest_digest,
        entry_points_count: cert.entry_points.len(),
        declared_effects_count: cert.declared_effects.len(),
    })
}
