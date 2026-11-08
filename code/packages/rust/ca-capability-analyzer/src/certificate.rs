//! # Effect Proof Certificate & Small Standalone Checker (VER24)
//!
//! Provides the canonical schema (`ver24-effect-cert-v1`), certificate generation,
//! and the independent small checker (VER00 §8.6.5) reading directly from source and manifest.

#![deny(unsafe_code)]

use std::fs;
use std::path::Path;

use crate::manifest::{load_manifest, JsonValue, MAX_MANIFEST_SIZE_BYTES};
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

impl EffectCertificate {
    /// Format certificate as canonical, deterministic JSON.
    pub fn to_json_string(&self) -> String {
        let mut out = String::new();
        out.push_str("{\n");
        out.push_str(&format!("  \"schema_version\": \"{}\",\n", self.schema_version));
        out.push_str(&format!("  \"package\": \"{}\",\n", self.package));
        out.push_str(&format!("  \"source_digest\": \"{}\",\n", self.source_digest));
        out.push_str(&format!("  \"manifest_digest\": \"{}\",\n", self.manifest_digest));

        // declared_effects
        out.push_str("  \"declared_effects\": [");
        if self.declared_effects.is_empty() {
            out.push_str("],\n");
        } else {
            out.push('\n');
            for (i, eff) in self.declared_effects.iter().enumerate() {
                out.push_str(&format!("    \"{}\"", eff));
                if i + 1 < self.declared_effects.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            out.push_str("  ],\n");
        }

        // entry_points
        out.push_str("  \"entry_points\": [");
        if self.entry_points.is_empty() {
            out.push_str("],\n");
        } else {
            out.push('\n');
            for (i, ep) in self.entry_points.iter().enumerate() {
                out.push_str("    {\n");
                out.push_str(&format!("      \"name\": \"{}\",\n", ep.name));
                let escaped_sig = ep.signature.replace('\"', "\\\"");
                out.push_str(&format!("      \"signature\": \"{}\",\n", escaped_sig));
                out.push_str("      \"effects\": [");
                if ep.effects.is_empty() {
                    out.push_str("]\n");
                } else {
                    out.push('\n');
                    for (j, eff) in ep.effects.iter().enumerate() {
                        out.push_str(&format!("        \"{}\"", eff));
                        if j + 1 < ep.effects.len() {
                            out.push(',');
                        }
                        out.push('\n');
                    }
                    out.push_str("      ]\n");
                }
                out.push_str("    }");
                if i + 1 < self.entry_points.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            out.push_str("  ],\n");
        }

        // unconditional_roots
        out.push_str("  \"unconditional_roots\": [");
        if self.unconditional_roots.is_empty() {
            out.push_str("],\n");
        } else {
            out.push('\n');
            for (i, root) in self.unconditional_roots.iter().enumerate() {
                out.push_str(&format!("    \"{}\"", root));
                if i + 1 < self.unconditional_roots.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            out.push_str("  ],\n");
        }

        // banned_construct_exceptions
        out.push_str("  \"banned_construct_exceptions\": [");
        if self.banned_construct_exceptions.is_empty() {
            out.push_str("],\n");
        } else {
            out.push('\n');
            for (i, ex) in self.banned_construct_exceptions.iter().enumerate() {
                out.push_str("    {\n");
                out.push_str(&format!("      \"construct\": \"{}\",\n", ex.construct));
                out.push_str(&format!("      \"language\": \"{}\",\n", ex.language));
                let escaped_just = ex.justification.replace('\"', "\\\"");
                out.push_str(&format!("      \"justification\": \"{}\"\n", escaped_just));
                out.push_str("    }");
                if i + 1 < self.banned_construct_exceptions.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            out.push_str("  ],\n");
        }

        out.push_str(&format!("  \"label\": \"{}\"\n", self.label));
        out.push_str("}\n");
        out
    }
}

/// Compute canonical SHA-256 digest of package sources.
/// Scans all `.rs` files under `src/` (excluding `capabilities.rs`).
/// Applies CRLF normalization and big-endian length-prefixed framing.
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
        let file_type = entry.file_type()
            .map_err(|e| format!("Failed to get file type for {}: {e}", path.display()))?;

        if file_type.is_symlink() {
            continue;
        }

        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if file_name.starts_with('.') || file_name == "target" || file_name == "node_modules" || file_name == ".venv" {
            continue;
        }

        if file_type.is_dir() {
            collect_rs_files(root, &path, out, depth + 1, total_bytes)?;
        } else if file_type.is_file() {
            if file_name.ends_with(".rs") && file_name != "capabilities.rs" {
                if out.len() >= MAX_SOURCE_FILES {
                    return Err(format!("Source file count exceeded limit of {MAX_SOURCE_FILES}"));
                }

                let meta = fs::metadata(&path)
                    .map_err(|e| format!("Failed to get metadata for {}: {e}", path.display()))?;
                if meta.len() > MAX_FILE_SIZE_BYTES {
                    return Err(format!("File {} exceeds size limit of {MAX_FILE_SIZE_BYTES} bytes", path.display()));
                }

                let raw = fs::read(&path)
                    .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
                let content = normalize_crlf(&raw);

                *total_bytes += content.len() as u64;
                if *total_bytes > MAX_TOTAL_SOURCE_BYTES {
                    return Err(format!("Total source bytes exceeded limit of {MAX_TOTAL_SOURCE_BYTES}"));
                }

                let rel = path.strip_prefix(root)
                    .map_err(|e| format!("Path strip prefix error: {e}"))?;
                let rel_str = rel.to_string_lossy().replace('\\', "/");
                out.push((rel_str, content));
            }
        }
    }
    Ok(())
}

/// Compute SHA-256 digest of `required_capabilities.json`.
/// Applies CRLF normalization for cross-platform stability.
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
        let normalized = normalize_crlf(&bytes);
        Ok(sha256_hex(&normalized))
    } else {
        Ok(sha256_hex(b""))
    }
}

/// Strip block comments, line comments, and string literals for accurate token discovery.
fn strip_comments_and_strings(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;

    while i < chars.len() {
        // Line comment
        if chars[i] == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            if i < chars.len() {
                out.push('\n');
                i += 1;
            }
            continue;
        }

        // Block comment
        if chars[i] == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
            i += 2;
            let mut depth = 1;
            while i + 1 < chars.len() && depth > 0 {
                if chars[i] == '/' && chars[i + 1] == '*' {
                    depth += 1;
                    i += 2;
                } else if chars[i] == '*' && chars[i + 1] == '/' {
                    depth -= 1;
                    i += 2;
                } else {
                    if chars[i] == '\n' {
                        out.push('\n');
                    }
                    i += 1;
                }
            }
            continue;
        }

        // Raw string r#"..."#
        if chars[i] == 'r' && (i + 1 < chars.len() && (chars[i + 1] == '#' || chars[i + 1] == '"')) {
            i += 1;
            let mut hashes = 0;
            while i < chars.len() && chars[i] == '#' {
                hashes += 1;
                i += 1;
            }
            if i < chars.len() && chars[i] == '"' {
                i += 1;
                // Look for closing "###
                while i < chars.len() {
                    if chars[i] == '"' {
                        let mut match_hashes = 0;
                        while i + 1 + match_hashes < chars.len() && chars[i + 1 + match_hashes] == '#' && match_hashes < hashes {
                            match_hashes += 1;
                        }
                        if match_hashes == hashes {
                            i += 1 + hashes;
                            break;
                        }
                    }
                    if chars[i] == '\n' {
                        out.push('\n');
                    }
                    i += 1;
                }
                out.push(' ');
                continue;
            }
        }

        // Regular string literal
        if chars[i] == '"' {
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    i += 2;
                } else {
                    if chars[i] == '\n' {
                        out.push('\n');
                    }
                    i += 1;
                }
            }
            if i < chars.len() && chars[i] == '"' {
                i += 1;
            }
            out.push(' ');
            continue;
        }

        out.push(chars[i]);
        i += 1;
    }

    out
}

/// Discover public functions in `src/lib.rs` robustly across formatting styles.
pub fn discover_public_entry_points(package_dir: &Path) -> Result<Vec<EntryPointCert>, String> {
    let lib_rs = package_dir.join("src").join("lib.rs");
    if !lib_rs.exists() {
        return Ok(Vec::new());
    }

    let raw = fs::read_to_string(&lib_rs)
        .map_err(|e| format!("Failed to read {}: {e}", lib_rs.display()))?;

    let cleaned = strip_comments_and_strings(&raw);
    let words: Vec<&str> = cleaned.split_whitespace().collect();

    let mut entry_points = Vec::new();
    let mut i = 0;

    while i < words.len() {
        if words[i] == "pub" {
            // Check for visibility modifiers: pub(crate), pub(super), etc.
            if i + 1 < words.len() && words[i + 1].starts_with('(') {
                i += 1;
                continue;
            }

            // Look ahead for "fn"
            let mut j = i + 1;
            let mut is_fn = false;
            while j < words.len() && j <= i + 6 {
                if words[j] == "fn" {
                    is_fn = true;
                    break;
                }
                if matches!(words[j], "const" | "unsafe" | "async") || words[j].starts_with("extern") {
                    j += 1;
                } else {
                    break;
                }
            }

            if is_fn && j + 1 < words.len() {
                // Next word is function name (or name with generics/parentheses)
                let raw_name = words[j + 1];
                let fn_name: String = raw_name
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();

                if !fn_name.is_empty() {
                    // Extract signature from i to end of params / return type
                    let mut sig_words = Vec::new();
                    let mut k = i;
                    let mut parens = 0;
                    let mut seen_open_paren = false;

                    while k < words.len() {
                        let w = words[k];
                        if w.contains('{') || w.contains(';') {
                            let clean_w = w.split('{').next().unwrap_or("").split(';').next().unwrap_or("");
                            if !clean_w.is_empty() {
                                sig_words.push(clean_w);
                            }
                            break;
                        }
                        sig_words.push(w);
                        for c in w.chars() {
                            if c == '(' {
                                parens += 1;
                                seen_open_paren = true;
                            } else if c == ')' {
                                parens -= 1;
                            }
                        }
                        if seen_open_paren && parens == 0 && (k + 1 < words.len() && (words[k + 1] == "{" || words[k + 1] == ";" || words[k + 1] == "where")) {
                            break;
                        }
                        k += 1;
                    }

                    let sig = sig_words.join(" ");
                    entry_points.push(EntryPointCert {
                        name: fn_name,
                        signature: sig,
                        effects: Vec::new(),
                    });
                }
                i = j + 1;
            }
        }
        i += 1;
    }

    Ok(entry_points)
}

/// Generate canonical effect certificate for target package.
pub fn generate_certificate(package_dir: &Path) -> Result<EffectCertificate, String> {
    let manifest = load_manifest(package_dir).map_err(|e| format!("Failed to load manifest: {e}"))?;
    let pkg_name = if manifest.is_explicit {
        manifest.package.clone()
    } else {
        format!(
            "rust/{}",
            package_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
        )
    };

    let source_digest = compute_source_digest(package_dir)?;
    let manifest_digest = compute_manifest_digest(package_dir)?;

    let declared_effects: Vec<String> = manifest
        .capabilities
        .iter()
        .map(|c| c.exact_string())
        .collect();

    let mut entry_points = discover_public_entry_points(package_dir)?;
    // If package has empty capabilities, all entry points have empty effects
    if declared_effects.is_empty() {
        for ep in &mut entry_points {
            ep.effects.clear();
        }
    }

    let mut banned_exceptions: Vec<BannedExceptionCert> = manifest
        .banned_exceptions
        .values()
        .map(|e| BannedExceptionCert {
            construct: e.construct.clone(),
            language: e.language.clone(),
            justification: e.justification.clone(),
        })
        .collect();
    banned_exceptions.sort_by(|a, b| a.construct.cmp(&b.construct));

    Ok(EffectCertificate {
        schema_version: CERT_SCHEMA_VERSION.to_string(),
        package: pkg_name,
        source_digest,
        manifest_digest,
        declared_effects,
        entry_points,
        unconditional_roots: Vec::new(),
        banned_construct_exceptions: banned_exceptions,
        label: "proved".to_string(),
    })
}

/// Parse effect certificate JSON into `EffectCertificate`.
pub fn parse_certificate(json_str: &str) -> Result<EffectCertificate, String> {
    let val = JsonValue::parse(json_str).map_err(|e| format!("Invalid certificate JSON: {e}"))?;

    let schema_version = val.get("schema_version")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'schema_version' in certificate".to_string())?
        .to_string();

    if schema_version != CERT_SCHEMA_VERSION {
        return Err(format!(
            "Unsupported certificate schema_version '{}' (expected '{}')",
            schema_version, CERT_SCHEMA_VERSION
        ));
    }

    let package = val.get("package")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'package' in certificate".to_string())?
        .to_string();

    let source_digest = val.get("source_digest")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'source_digest' in certificate".to_string())?
        .to_string();

    let manifest_digest = val.get("manifest_digest")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'manifest_digest' in certificate".to_string())?
        .to_string();

    let declared_effects = if let Some(arr) = val.get("declared_effects").and_then(|v| v.as_array()) {
        let mut list = Vec::new();
        for item in arr {
            let s = item.as_str().ok_or_else(|| "declared_effects element must be string".to_string())?;
            list.push(s.to_string());
        }
        list
    } else {
        Vec::new()
    };

    let entry_points = if let Some(arr) = val.get("entry_points").and_then(|v| v.as_array()) {
        let mut list = Vec::new();
        for item in arr {
            let name = item.get("name").and_then(|v| v.as_str()).ok_or_else(|| "entry_point missing 'name'".to_string())?.to_string();
            let signature = item.get("signature").and_then(|v| v.as_str()).ok_or_else(|| "entry_point missing 'signature'".to_string())?.to_string();
            let mut effects = Vec::new();
            if let Some(eff_arr) = item.get("effects").and_then(|v| v.as_array()) {
                for eff in eff_arr {
                    let eff_s = eff.as_str().ok_or_else(|| "effect must be string".to_string())?;
                    effects.push(eff_s.to_string());
                }
            }
            list.push(EntryPointCert { name, signature, effects });
        }
        list
    } else {
        Vec::new()
    };

    let unconditional_roots = if let Some(arr) = val.get("unconditional_roots").and_then(|v| v.as_array()) {
        let mut list = Vec::new();
        for item in arr {
            let s = item.as_str().ok_or_else(|| "unconditional_roots element must be string".to_string())?;
            list.push(s.to_string());
        }
        list
    } else {
        Vec::new()
    };

    let banned_exceptions = if let Some(arr) = val.get("banned_construct_exceptions").and_then(|v| v.as_array()) {
        let mut list = Vec::new();
        for item in arr {
            let construct = item.get("construct").and_then(|v| v.as_str()).ok_or_else(|| "missing 'construct'".to_string())?.to_string();
            let language = item.get("language").and_then(|v| v.as_str()).ok_or_else(|| "missing 'language'".to_string())?.to_string();
            let justification = item.get("justification").and_then(|v| v.as_str()).ok_or_else(|| "missing 'justification'".to_string())?.to_string();
            list.push(BannedExceptionCert { construct, language, justification });
        }
        list
    } else {
        Vec::new()
    };

    let label = val.get("label")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'label' in certificate".to_string())?
        .to_string();

    Ok(EffectCertificate {
        schema_version,
        package,
        source_digest,
        manifest_digest,
        declared_effects,
        entry_points,
        unconditional_roots,
        banned_construct_exceptions: banned_exceptions,
        label,
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

/// Small Standalone Certificate Checker (VER00 §8.6.5, VER24).
///
/// Verifies the certificate directly against the target package on disk.
/// Does NOT rely on external summaries or blind trust.
pub fn verify_certificate(package_dir: &Path, cert_path: &Path) -> Result<CertVerificationResult, String> {
    if !cert_path.exists() {
        return Err(format!("Certificate file does not exist: {}", cert_path.display()));
    }

    let cert_meta = fs::metadata(cert_path)
        .map_err(|e| format!("Failed to read metadata for {}: {e}", cert_path.display()))?;
    if cert_meta.len() > MAX_MANIFEST_SIZE_BYTES {
        return Err(format!("Certificate exceeds maximum allowed size of {MAX_MANIFEST_SIZE_BYTES} bytes"));
    }

    let cert_content = fs::read_to_string(cert_path)
        .map_err(|e| format!("Failed to read certificate {}: {e}", cert_path.display()))?;

    let cert = parse_certificate(&cert_content)?;

    // 0. Verify generated capability module on disk matches manifest (Blocker 2)
    match crate::generator::check_module_on_disk(package_dir) {
        Ok(true) => {}
        Ok(false) => {
            return Err("Generated capabilities module 'src/capabilities.rs' is missing or differs from manifest".to_string());
        }
        Err(e) => {
            return Err(format!("Failed to verify capabilities module: {e}"));
        }
    }

    // 1. Verify manifest digest (with CRLF normalization)
    let actual_manifest_digest = compute_manifest_digest(package_dir)?;
    if actual_manifest_digest != cert.manifest_digest {
        return Err(format!(
            "Certificate manifest_digest mismatch: expected '{}', computed '{}'",
            cert.manifest_digest, actual_manifest_digest
        ));
    }

    // 2. Verify source digest (with length framing and CRLF normalization)
    let actual_source_digest = compute_source_digest(package_dir)?;
    if actual_source_digest != cert.source_digest {
        return Err(format!(
            "Certificate source_digest mismatch: expected '{}', computed '{}'",
            cert.source_digest, actual_source_digest
        ));
    }

    // 3. Verify manifest contents and declared effects
    let manifest = load_manifest(package_dir).map_err(|e| format!("Failed to load manifest: {e}"))?;
    let actual_pkg_name = if manifest.is_explicit {
        manifest.package.clone()
    } else {
        format!(
            "rust/{}",
            package_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
        )
    };

    if cert.package != actual_pkg_name {
        return Err(format!(
            "Certificate package '{}' does not match manifest package '{}'",
            cert.package, actual_pkg_name
        ));
    }

    let actual_declared_effects: Vec<String> = manifest
        .capabilities
        .iter()
        .map(|c| c.exact_string())
        .collect();

    if cert.declared_effects != actual_declared_effects {
        return Err(format!(
            "Certificate declared_effects mismatch: cert declared {:?}, manifest declared {:?}",
            cert.declared_effects, actual_declared_effects
        ));
    }

    // 4. Verify public entry points in lib.rs (bidirectional match)
    let actual_entry_points = discover_public_entry_points(package_dir)?;
    if actual_entry_points.len() != cert.entry_points.len() {
        return Err(format!(
            "Entry points count mismatch: found {} in source, certificate declared {}",
            actual_entry_points.len(), cert.entry_points.len()
        ));
    }

    for ep in &actual_entry_points {
        let cert_ep = cert.entry_points.iter().find(|c| c.name == ep.name);
        match cert_ep {
            None => {
                return Err(format!(
                    "Uncertified public entry point '{}' found in source",
                    ep.name
                ));
            }
            Some(c) => {
                if c.signature != ep.signature {
                    return Err(format!(
                        "Entry point '{}' signature mismatch: source has '{}', cert declared '{}'",
                        ep.name, ep.signature, c.signature
                    ));
                }
                for eff in &c.effects {
                    if !cert.declared_effects.contains(eff) {
                        return Err(format!(
                            "Entry point '{}' declares effect '{}' not present in declared_effects",
                            c.name, eff
                        ));
                    }
                }
            }
        }
    }

    // If pure package, all entry points must have empty effects
    if cert.declared_effects.is_empty() {
        for ep in &cert.entry_points {
            if !ep.effects.is_empty() {
                return Err(format!(
                    "Pure package entry point '{}' claims non-empty effects: {:?}",
                    ep.name, ep.effects
                ));
            }
        }
    }

    // 5. Verify banned construct exceptions match manifest
    if cert.banned_construct_exceptions.len() != manifest.banned_exceptions.len() {
        return Err(format!(
            "Banned construct exceptions count mismatch: manifest has {}, cert has {}",
            manifest.banned_exceptions.len(),
            cert.banned_construct_exceptions.len()
        ));
    }
    for ex in &cert.banned_construct_exceptions {
        let key = format!("{}:{}", ex.language.to_lowercase(), ex.construct.to_lowercase());
        if !manifest.banned_exceptions.contains_key(&key) {
            return Err(format!(
                "Certificate declares unverified banned exception '{}:{}'",
                ex.language, ex.construct
            ));
        }
    }

    // 6. Run full package static analysis to confirm zero undeclared capabilities
    let report = crate::analyze_package(package_dir, false)
        .map_err(|e| format!("Static capability analysis failed: {e}"))?;
    if !report.is_clean() {
        return Err(format!(
            "Package static capability analysis failed with {} violations",
            report.violations.len()
        ));
    }

    // 7. Verify honest label
    if cert.label != "proved" {
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
