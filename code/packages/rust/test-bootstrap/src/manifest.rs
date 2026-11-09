//! # Evidence manifests (VER04h §8)
//!
//! An assurance report may only quote numbers that a run wrote down. This
//! module is the writing half: a tiny, dependency-free JSON value type and a
//! writer, shared by every harness that produces a manifest (the Warrant
//! generated test target, the proof driver, the microbenchmark target).
//!
//! Properties that matter for evidence:
//!
//! * **Deterministic bytes.** Object keys are sorted (`BTreeMap`), numbers are
//!   unsigned integers, there are no timestamps. Running the same suite twice
//!   writes the same file, so a checked-in manifest can be regenerated and
//!   diffed, and a hand edit shows up as drift.
//! * **No floating point.** Counts and seeds are integers; a seed that does not
//!   fit in a JSON-safe integer is written as a decimal *string* by the caller.
//! * **Safe strings.** Every control character is escaped, so a message can
//!   never break the document structure.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

/// A JSON value restricted to what manifests need.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Json {
    Bool(bool),
    Num(u64),
    Str(String),
    Arr(Vec<Json>),
    Obj(BTreeMap<String, Json>),
}

impl Json {
    /// Build an object from `(key, value)` pairs.
    pub fn obj<const N: usize>(pairs: [(&str, Json); N]) -> Json {
        Json::Obj(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }

    /// A string value.
    pub fn str(text: impl Into<String>) -> Json {
        Json::Str(text.into())
    }

    /// Render with two-space indentation and a trailing newline.
    pub fn render(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, 0);
        out.push('\n');
        out
    }

    fn write(&self, out: &mut String, depth: usize) {
        match self {
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Num(n) => out.push_str(&n.to_string()),
            Json::Str(s) => quote(s, out),
            Json::Arr(items) if items.is_empty() => out.push_str("[]"),
            Json::Arr(items) => {
                out.push_str("[\n");
                for (i, item) in items.iter().enumerate() {
                    indent(out, depth + 1);
                    item.write(out, depth + 1);
                    if i + 1 < items.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                indent(out, depth);
                out.push(']');
            }
            Json::Obj(map) if map.is_empty() => out.push_str("{}"),
            Json::Obj(map) => {
                out.push_str("{\n");
                for (i, (key, value)) in map.iter().enumerate() {
                    indent(out, depth + 1);
                    quote(key, out);
                    out.push_str(": ");
                    value.write(out, depth + 1);
                    if i + 1 < map.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                indent(out, depth);
                out.push('}');
            }
        }
    }
}

fn indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

fn quote(text: &str, out: &mut String) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// The path named by environment variable `var`, if set and non-empty.
///
/// Harnesses write a manifest only when asked to, so an ordinary
/// `cargo test` never modifies tracked files.
pub fn output_path(var: &str) -> Option<PathBuf> {
    match std::env::var_os(var) {
        Some(value) if !value.is_empty() => Some(PathBuf::from(value)),
        _ => None,
    }
}

/// Write `doc` to `path`, creating parent directories.
///
/// The write is hardened against a planted path (audit A4):
///
/// * a `path` that is a symlink is refused (`symlink_metadata` does not follow
///   it), so a manifest cannot be redirected onto some other file;
/// * the bytes go to a fresh temp file in the **same directory**, opened with
///   `create_new` (never follows or truncates a pre-existing entry), and the
///   temp file is then `rename`d over the target, so a reader never sees a
///   half-written manifest and the target's inode is replaced, not written
///   through.
pub fn write(path: &Path, doc: &Json) -> io::Result<()> {
    use std::io::Write as _;

    let parent = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => {
            std::fs::create_dir_all(p)?;
            p.to_path_buf()
        }
        _ => PathBuf::from("."),
    };
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("refusing to write through symlink {}", path.display()),
            ));
        }
        Ok(meta) if meta.is_dir() => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{} is a directory", path.display()),
            ));
        }
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"))?
        .to_string_lossy()
        .into_owned();
    let bytes = doc.render();
    for attempt in 0..16u32 {
        let tmp = parent.join(format!(".{name}.{}.{attempt}.tmp", std::process::id()));
        let mut file = match std::fs::OpenOptions::new().write(true).create_new(true).open(&tmp) {
            Ok(f) => f,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        };
        let result = file
            .write_all(bytes.as_bytes())
            .and_then(|()| file.sync_all())
            .and_then(|()| {
                drop(file);
                std::fs::rename(&tmp, path)
            });
        if result.is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
        return result;
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not create a unique temporary manifest file",
    ))
}

// ---------------------------------------------------------------------------
// Source binding (audit N2)
// ---------------------------------------------------------------------------

/// Largest single source file the binding will read.
pub const MAX_SOURCE_FILE_BYTES: u64 = 8 * 1024 * 1024;

/// Deepest `src` nesting the binding will walk.
const MAX_SOURCE_DEPTH: usize = 32;

/// Read every `*.rs` file under `src_dir` as `(relative path, bytes)`, sorted
/// by path. Paths are relative to `src_dir`, use `/` as separator, and are
/// valid UTF-8. A symlink anywhere in the tree, an unreadable entry, a
/// non-UTF-8 name, an oversized file or an excessively deep tree is an error:
/// the binding must cover exactly what the compiler would read, so anything
/// it cannot account for fails closed.
pub fn read_source_tree(src_dir: &Path) -> io::Result<Vec<(String, Vec<u8>)>> {
    use std::io::Read as _;

    fn bad(msg: String) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidData, msg)
    }
    fn walk(dir: &Path, rel: &str, depth: usize, out: &mut Vec<(String, Vec<u8>)>) -> io::Result<()> {
        if depth > MAX_SOURCE_DEPTH {
            return Err(bad(format!("{} is nested too deeply", dir.display())));
        }
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            entries.push(entry?);
        }
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| bad(format!("non-UTF-8 file name under {}", dir.display())))?;
            let path = entry.path();
            let rel_name = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };
            let ty = entry.file_type()?;
            if ty.is_symlink() {
                return Err(bad(format!("{} is a symlink", path.display())));
            } else if ty.is_dir() {
                walk(&path, &rel_name, depth + 1, out)?;
            } else if ty.is_file() && name.ends_with(".rs") {
                let mut bytes = Vec::new();
                std::fs::File::open(&path)?
                    .take(MAX_SOURCE_FILE_BYTES + 1)
                    .read_to_end(&mut bytes)?;
                if bytes.len() as u64 > MAX_SOURCE_FILE_BYTES {
                    return Err(bad(format!("{} is larger than the {MAX_SOURCE_FILE_BYTES}-byte limit", path.display())));
                }
                out.push((rel_name, bytes));
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    walk(src_dir, "", 0, &mut out)?;
    out.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    Ok(out)
}

/// The unambiguous byte framing that `source_sha256` digests (`warrant-src-v1`):
///
/// ```text
/// "warrant-src-v1\0"
/// u64-LE file count
/// for each file, sorted by path bytes:
///     u64-LE path length,    path bytes (relative to src/, `/` separators)
///     u64-LE content length, content bytes
/// ```
///
/// Every field is length-prefixed, so no two different file sets frame to the
/// same bytes (moving bytes between a path and its content, or between two
/// files, changes a length). `files` is re-sorted here so the framing never
/// depends on the caller's order.
pub fn frame_sources(files: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut sorted: Vec<&(String, Vec<u8>)> = files.iter().collect();
    sorted.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    let mut out = Vec::new();
    out.extend_from_slice(b"warrant-src-v1\0");
    out.extend_from_slice(&(sorted.len() as u64).to_le_bytes());
    for (path, bytes) in sorted {
        out.extend_from_slice(&(path.len() as u64).to_le_bytes());
        out.extend_from_slice(path.as_bytes());
        out.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        out.extend_from_slice(bytes);
    }
    out
}

/// [`frame_sources`] of [`read_source_tree`]: the bytes every producer and the
/// checker hash (SHA-256, lowercase hex) to obtain `source_sha256`.
pub fn source_tree_framing(src_dir: &Path) -> io::Result<Vec<u8>> {
    Ok(frame_sources(&read_source_tree(src_dir)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_is_sorted_escaped_and_deterministic() {
        let doc = Json::obj([
            ("b", Json::Num(2)),
            ("a", Json::str("x\"y\n\u{1}")),
            ("c", Json::Arr(vec![Json::Bool(true), Json::Arr(vec![])])),
            ("d", Json::Obj(BTreeMap::new())),
        ]);
        let text = doc.render();
        assert_eq!(
            text,
            "{\n  \"a\": \"x\\\"y\\n\\u0001\",\n  \"b\": 2,\n  \"c\": [\n    true,\n    []\n  ],\n  \"d\": {}\n}\n"
        );
        assert_eq!(text, doc.render());
    }

    #[test]
    fn output_path_requires_a_non_empty_value() {
        assert_eq!(output_path("TB_MANIFEST_TEST_UNSET_VAR"), None);
        std::env::set_var("TB_MANIFEST_TEST_EMPTY_VAR", "");
        assert_eq!(output_path("TB_MANIFEST_TEST_EMPTY_VAR"), None);
        std::env::set_var("TB_MANIFEST_TEST_SET_VAR", "x/y.json");
        assert_eq!(
            output_path("TB_MANIFEST_TEST_SET_VAR"),
            Some(PathBuf::from("x/y.json"))
        );
    }

    #[test]
    fn write_creates_parent_directories() {
        let dir = std::env::temp_dir().join(format!("tb-manifest-{}", std::process::id()));
        let path = dir.join("nested/m.json");
        write(&path, &Json::obj([("k", Json::Num(1))])).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{\n  \"k\": 1\n}\n");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn write_replaces_atomically_and_leaves_no_temp_files() {
        let dir = std::env::temp_dir().join(format!("tb-manifest-atomic-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("m.json");
        write(&path, &Json::obj([("k", Json::Num(1))])).unwrap();
        write(&path, &Json::obj([("k", Json::Num(2))])).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{\n  \"k\": 2\n}\n");
        let names: Vec<_> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(names, vec![std::ffi::OsString::from("m.json")], "temp file must be renamed away");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn write_refuses_a_symlink_target_and_never_touches_what_it_points_at() {
        let dir = std::env::temp_dir().join(format!("tb-manifest-symlink-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let victim = dir.join("victim.txt");
        std::fs::write(&victim, "precious").unwrap();
        let link = dir.join("m.json");
        std::os::unix::fs::symlink(&victim, &link).unwrap();
        let err = write(&link, &Json::obj([("k", Json::Num(1))])).unwrap_err();
        assert!(err.to_string().contains("symlink"), "{err}");
        assert_eq!(std::fs::read_to_string(&victim).unwrap(), "precious");
        // A dangling symlink is refused as well (it would create its target).
        let dangling = dir.join("d.json");
        std::os::unix::fs::symlink(dir.join("nowhere"), &dangling).unwrap();
        assert!(write(&dangling, &Json::obj([])).is_err());
        assert!(!dir.join("nowhere").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn write_never_reuses_a_preexisting_temp_name() {
        let dir = std::env::temp_dir().join(format!("tb-manifest-tmp-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // Pre-plant the first temp name an attacker could predict.
        let planted = dir.join(format!(".m.json.{}.0.tmp", std::process::id()));
        std::fs::write(&planted, "attacker").unwrap();
        write(&dir.join("m.json"), &Json::obj([("k", Json::Num(1))])).unwrap();
        assert_eq!(std::fs::read_to_string(&planted).unwrap(), "attacker");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    fn sbox(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("tb-src-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("sub")).unwrap();
        d
    }

    #[test]
    fn framing_is_unambiguous_and_order_independent() {
        let a = vec![("a.rs".to_string(), b"xy".to_vec()), ("b.rs".to_string(), b"z".to_vec())];
        let b = vec![("b.rs".to_string(), b"z".to_vec()), ("a.rs".to_string(), b"xy".to_vec())];
        assert_eq!(frame_sources(&a), frame_sources(&b));
        // Moving a byte across the path/content or file/file boundary changes the framing.
        let c = vec![("a.rs".to_string(), b"x".to_vec()), ("yb.rs".to_string(), b"z".to_vec())];
        assert_ne!(frame_sources(&a), frame_sources(&c));
        let d = vec![("a.rsx".to_string(), b"y".to_vec()), ("b.rs".to_string(), b"z".to_vec())];
        assert_ne!(frame_sources(&a), frame_sources(&d));
    }

    #[test]
    fn source_tree_covers_every_rs_file_and_sees_edits() {
        let d = sbox("tree");
        std::fs::write(d.join("lib.rs"), "a").unwrap();
        std::fs::write(d.join("capabilities.rs"), "b").unwrap();
        std::fs::write(d.join("sub/deep.rs"), "c").unwrap();
        std::fs::write(d.join("notes.txt"), "ignored").unwrap();
        let files = read_source_tree(&d).unwrap();
        let names: Vec<&str> = files.iter().map(|f| f.0.as_str()).collect();
        assert_eq!(names, ["capabilities.rs", "lib.rs", "sub/deep.rs"]);
        let before = source_tree_framing(&d).unwrap();
        std::fs::write(d.join("capabilities.rs"), "B").unwrap();
        assert_ne!(before, source_tree_framing(&d).unwrap());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[cfg(unix)]
    #[test]
    fn source_tree_refuses_symlinks() {
        let d = sbox("link");
        std::fs::write(d.join("lib.rs"), "a").unwrap();
        std::os::unix::fs::symlink(d.join("lib.rs"), d.join("alias.rs")).unwrap();
        assert!(read_source_tree(&d).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }
}
