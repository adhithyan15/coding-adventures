//! # Rust Capability Module Generator (VER23)
//!
//! Generates sealed `src/capabilities.rs` from `required_capabilities.json`.
//! Implements "you cannot name what you did not declare" (VER00 §8.6).

#![deny(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};

use crate::manifest::{load_manifest, Capability, ManifestData};

/// Generate the canonical source code for `src/capabilities.rs`.
pub fn generate_module_source(package_name: &str, manifest: Option<&ManifestData>) -> String {
    let clean_pkg_name: String = package_name
        .chars()
        .filter(|c| c.is_ascii_graphic() && *c != '\n' && *c != '\r')
        .collect();
    let mut out = String::new();
    out.push_str("// Generated from required_capabilities.json. Do not edit.\n");
    out.push_str(&format!("// Package: {}\n", clean_pkg_name));

    match manifest {
        None => {
            out.push_str("// Declared capabilities: none (pure computation)\n");
            out.push_str("// Proof label: proved\n\n");
            out.push_str("#![deny(unsafe_code)]\n\n");
            out.push_str("/// Pure computation witness token.\n");
            out.push_str("///\n");
            out.push_str("/// This package declares zero ambient capabilities.\n");
            out.push_str("/// No I/O, network, filesystem, process, environment, or threading handles\n");
            out.push_str("/// are exposed by this module.\n");
            out.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n");
            out.push_str("pub struct PureCapabilities;\n");
        }
        Some(m) if m.capabilities.is_empty() => {
            out.push_str("// Declared capabilities: none (pure computation)\n");
            out.push_str("// Proof label: proved\n\n");
            out.push_str("#![deny(unsafe_code)]\n\n");
            out.push_str("/// Pure computation witness token.\n");
            out.push_str("///\n");
            out.push_str("/// This package declares zero ambient capabilities in required_capabilities.json.\n");
            out.push_str("/// No I/O, network, filesystem, process, environment, or threading handles\n");
            out.push_str("/// are exposed by this module.\n");
            out.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n");
            out.push_str("pub struct PureCapabilities;\n");
        }
        Some(m) => {
            out.push_str("// Declared capabilities:\n");
            for cap in &m.capabilities {
                out.push_str(&format!("//   - {}:{}:{}\n", cap.category, cap.action, cap.target));
            }
            out.push_str("\n#![deny(unsafe_code)]\n\n");
            out.push_str("/// Package capability authority token.\n");
            out.push_str("///\n");
            out.push_str("/// Exposes only the capabilities explicitly authorized in required_capabilities.json.\n");
            out.push_str("#[derive(Debug)]\n");
            out.push_str("pub struct PackageCapabilities {\n");
            for (idx, cap) in m.capabilities.iter().enumerate() {
                let field_name = sanitize_ident(&format!("cap_{}_{}_{}", cap.category, cap.action, idx));
                out.push_str(&format!("    pub(crate) {}: {},\n", field_name, capability_type_name(cap)));
            }
            out.push_str("}\n\n");

            // Define individual capability witness tokens
            for cap in &m.capabilities {
                let type_name = capability_type_name(cap);
                out.push_str(&format!("/// Authority witness for `{}:{}:{}`.\n", cap.category, cap.action, cap.target));
                out.push_str("#[derive(Debug, Clone, Copy)]\n");
                out.push_str(&format!("pub struct {};\n\n", type_name));
            }
        }
    }

    out
}

fn sanitize_ident(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "ident".to_string()
    } else {
        out
    }
}

fn capability_type_name(cap: &Capability) -> String {
    let cat = to_camel_case(&cap.category);
    let act = to_camel_case(&cap.action);
    let tgt = if cap.target == "*" {
        "All".to_string()
    } else {
        to_camel_case(&cap.target)
    };
    format!("{cat}{act}{tgt}Cap")
}

fn to_camel_case(s: &str) -> String {
    let mut out = String::new();
    let mut capitalize_next = true;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            if capitalize_next {
                out.push(c.to_ascii_uppercase());
                capitalize_next = false;
            } else {
                out.push(c);
            }
        } else {
            capitalize_next = true;
        }
    }
    if out.is_empty() {
        "Witness".to_string()
    } else {
        out
    }
}

/// Target path for `src/capabilities.rs` within a package directory.
pub fn capabilities_module_path(package_dir: &Path) -> PathBuf {
    package_dir.join("src").join("capabilities.rs")
}

/// Check whether on-disk `src/capabilities.rs` matches the expected generated module.
pub fn check_module_on_disk(package_dir: &Path) -> Result<bool, String> {
    let manifest = load_manifest(package_dir).map_err(|e| format!("Failed to load manifest: {e}"))?;
    let pkg_name = if manifest.is_explicit {
        manifest.package.as_str()
    } else {
        package_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("pure-computation")
    };

    let expected = generate_module_source(pkg_name, Some(&manifest));
    let path = capabilities_module_path(package_dir);
    if !path.exists() {
        return Ok(false);
    }

    let actual = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;

    // Normalize CRLF
    let actual_normalized = actual.replace("\r\n", "\n");
    let expected_normalized = expected.replace("\r\n", "\n");

    Ok(actual_normalized == expected_normalized)
}

/// Write the generated `src/capabilities.rs` to disk.
pub fn write_module_to_disk(package_dir: &Path) -> Result<(), String> {
    let manifest = load_manifest(package_dir).map_err(|e| format!("Failed to load manifest: {e}"))?;
    let pkg_name = if manifest.is_explicit {
        manifest.package.as_str()
    } else {
        package_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("pure-computation")
    };

    let content = generate_module_source(pkg_name, Some(&manifest));
    let src_dir = package_dir.join("src");
    // Refuse symlinks before writing (security review round 3, LOW).
    // `fs::write` follows them: a package whose `src` (or
    // `src/capabilities.rs`) is a symlink would make `--generate-module`
    // overwrite a file outside the package.  The scanner refuses symlinks for
    // the same reason; `symlink_metadata` looks at the link itself.  A path
    // that does not exist yet is fine and is created below.
    refuse_symlink(&src_dir)?;
    if !src_dir.exists() {
        fs::create_dir_all(&src_dir)
            .map_err(|e| format!("Failed to create src dir {}: {e}", src_dir.display()))?;
    }

    let target_path = capabilities_module_path(package_dir);
    refuse_symlink(&target_path)?;
    replace_file(&src_dir, &target_path, content.as_bytes())
}

/// Replace `target` (in directory `dir`) with `bytes` without writing through it.
///
/// `fs::write` opens the existing file and truncates it *in place*.  If
/// `src/capabilities.rs` is a hard link to a file outside the package, that
/// outside file is what gets overwritten (security review round 4, LOW, case
/// `generator-hardlink`); a hard link cannot be told apart from an ordinary
/// file by `symlink_metadata`.  So the module is written to a new temporary
/// file in the same directory, opened with `create_new` (it fails rather than
/// follows if the name exists, a symlink included; the next name is tried),
/// and `rename`d over the target.  `rename` replaces the directory entry, so
/// the old inode - and any other name it has - is left untouched.
///
/// The temporary name is not a `.rs` file, so neither the scanner nor the
/// certificate digest ever reads it.  If writing fails, it is *not* deleted
/// (this crate declares no `fs:delete` capability); the error names it.
fn replace_file(dir: &Path, target: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::{ErrorKind, Write};
    for n in 0..64u32 {
        let tmp = dir.join(format!(".capabilities.rs.tmp{n}"));
        let mut file = match fs::OpenOptions::new().write(true).create_new(true).open(&tmp) {
            Ok(f) => f,
            Err(e) if e.kind() == ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("Failed to create {}: {e}", tmp.display())),
        };
        return file
            .write_all(bytes)
            .and_then(|()| file.sync_all())
            .and_then(|()| fs::rename(&tmp, target))
            .map_err(|e| format!("Failed to write {} (temporary file {} left behind): {e}", target.display(), tmp.display()));
    }
    Err(format!("Failed to write {}: no free temporary file name in {}", target.display(), dir.display()))
}

/// `Err` if `path` exists and is a symbolic link (not followed).
fn refuse_symlink(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(m) if m.file_type().is_symlink() => Err(format!("refusing to write through symlink {}", path.display())),
        _ => Ok(()),
    }
}
