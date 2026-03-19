// SHA256 hashing for incremental change detection.
//
// # Why hashing?
//
// The core of incremental builds is change detection. If nothing changed
// in a package's source files, there is no reason to rebuild it. We detect
// changes by computing a SHA256 hash of all relevant source files and
// comparing it against the cached hash from the last build.
//
// # How hashing works
//
// The hashing algorithm is deterministic — given the same files with the
// same contents, it always produces the same hash. Here is the procedure:
//
//  1. Collect all source files in the package directory, filtered by the
//     language's relevant extensions. Always include BUILD files.
//  2. Sort the file list lexicographically by relative path. This ensures
//     that file ordering does not affect the hash.
//  3. SHA256-hash each file's contents individually.
//  4. Concatenate all individual hashes into one string.
//  5. SHA256-hash that concatenated string to produce the final hash.
//
// This two-level hashing means:
//   - Reordering files doesn't change the hash (we sort first).
//   - Adding or removing a file changes the hash.
//   - Modifying any file's contents changes the hash.
//
// # Dependency hashing
//
// A package should be rebuilt if any of its transitive dependencies changed.
// hash_deps takes a package's dependency information and produces a single
// hash representing the state of all its dependencies.

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::io::Read;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::discovery::Package;
use crate::graph::Graph;

// ---------------------------------------------------------------------------
// Extension maps
// ---------------------------------------------------------------------------

/// Maps languages to the file extensions that matter for change detection.
/// If any file with these extensions changes, the package needs rebuilding.
fn source_extensions(language: &str) -> HashSet<&'static str> {
    match language {
        "python" => [".py", ".toml", ".cfg"].iter().cloned().collect(),
        "ruby" => [".rb", ".gemspec"].iter().cloned().collect(),
        "go" => [".go"].iter().cloned().collect(),
        "rust" => [".rs", ".toml"].iter().cloned().collect(),
        _ => HashSet::new(),
    }
}

/// Maps languages to filenames that should always be included regardless
/// of their extension.
fn special_filenames(language: &str) -> HashSet<&'static str> {
    match language {
        "python" => HashSet::new(),
        "ruby" => ["Gemfile", "Rakefile"].iter().cloned().collect(),
        "go" => ["go.mod", "go.sum"].iter().cloned().collect(),
        "rust" => ["Cargo.toml", "Cargo.lock"].iter().cloned().collect(),
        _ => HashSet::new(),
    }
}

// ---------------------------------------------------------------------------
// File collection
// ---------------------------------------------------------------------------

/// Walks the package directory and returns all source files relevant to
/// the package's language. Files are sorted by their relative path for
/// deterministic hashing.
///
/// The collection rules:
///   - BUILD, BUILD_mac, BUILD_linux are always included.
///   - Files matching the language's extensions are included.
///   - Special filenames (go.mod, Gemfile, etc.) are included.
///   - Everything else is ignored.
fn collect_source_files(pkg: &Package) -> Vec<std::path::PathBuf> {
    let extensions = source_extensions(&pkg.language);
    let specials = special_filenames(&pkg.language);

    let mut files = Vec::new();

    // Walk the package directory recursively.
    walk_for_files(&pkg.path, &extensions, &specials, &mut files);

    // Sort by relative path for determinism. Two developers with different
    // absolute paths to the repo should get the same hash.
    files.sort_by(|a, b| {
        let rel_a = a.strip_prefix(&pkg.path).unwrap_or(a);
        let rel_b = b.strip_prefix(&pkg.path).unwrap_or(b);
        rel_a.cmp(rel_b)
    });

    files
}

/// Recursively walks a directory collecting source files.
fn walk_for_files(
    dir: &Path,
    extensions: &HashSet<&str>,
    specials: &HashSet<&str>,
    files: &mut Vec<std::path::PathBuf>,
) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_for_files(&path, extensions, specials, files);
        } else if path.is_file() {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            // Always include BUILD files — they define how the package is built.
            if name == "BUILD" || name == "BUILD_mac" || name == "BUILD_linux" {
                files.push(path);
                continue;
            }

            // Check if the file extension matches.
            if let Some(ext) = path.extension() {
                let ext_str = format!(".{}", ext.to_string_lossy());
                if extensions.contains(ext_str.as_str()) {
                    files.push(path);
                    continue;
                }
            }

            // Check special filenames.
            if specials.contains(name.as_str()) {
                files.push(path);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Hashing functions
// ---------------------------------------------------------------------------

/// Computes the SHA256 hex digest of a single file's contents.
/// We read in chunks to handle large files without loading them
/// entirely into memory.
fn hash_file(path: &Path) -> Result<String, std::io::Error> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192]; // 8KB chunks, same as Go implementation.

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// Computes a SHA256 hash representing all source files in the package.
/// The hash changes if any source file is added, removed, or modified.
///
/// If the package has no source files, we hash the empty string for
/// consistency — every package gets a hash, even empty ones.
pub fn hash_package(pkg: &Package) -> String {
    let files = collect_source_files(pkg);

    if files.is_empty() {
        // No source files — hash the empty string.
        let hash = Sha256::digest(b"");
        return format!("{:x}", hash);
    }

    // Hash each file individually, concatenate all hashes, hash again.
    // This two-level scheme means the final hash changes if any file
    // changes, is added, or is removed.
    let mut file_hashes = Vec::new();
    for f in &files {
        let fh = match hash_file(f) {
            Ok(h) => h,
            // If we can't read a file, use a sentinel to ensure the hash
            // differs from the cached version, triggering a rebuild.
            Err(_) => "error-reading-file".to_string(),
        };
        file_hashes.push(fh);
    }

    let combined = file_hashes.join("");
    let hash = Sha256::digest(combined.as_bytes());
    format!("{:x}", hash)
}

/// Computes a SHA256 hash of all transitive dependency hashes.
///
/// If any transitive dependency's source files changed, this hash will
/// change too, triggering a rebuild of the dependent package. This is
/// how we propagate changes through the dependency tree.
///
/// In our graph, edges go dep -> pkg (dependency points to dependent).
/// So a package's dependencies are found by following reverse edges
/// (predecessors). We walk backwards (reverse edges) from the package.
pub fn hash_deps(
    package_name: &str,
    graph: &Graph,
    package_hashes: &HashMap<String, String>,
) -> String {
    if !graph.has_node(package_name) {
        let hash = Sha256::digest(b"");
        return format!("{:x}", hash);
    }

    // Collect all transitive dependencies (packages this one depends on).
    let transitive_deps = collect_transitive_predecessors(package_name, graph);

    if transitive_deps.is_empty() {
        let hash = Sha256::digest(b"");
        return format!("{:x}", hash);
    }

    // Sort for determinism, concatenate hashes, hash again.
    let mut sorted: Vec<&String> = transitive_deps.iter().collect();
    sorted.sort();

    let mut combined = String::new();
    for dep in sorted {
        if let Some(h) = package_hashes.get(dep.as_str()) {
            combined.push_str(h);
        }
    }

    let hash = Sha256::digest(combined.as_bytes());
    format!("{:x}", hash)
}

/// Walks backwards through the graph from the given node, collecting
/// all nodes it transitively depends on.
///
/// In our graph, edge A->B means "B depends on A". So to find everything
/// that package_name depends on, we follow predecessors (reverse edges).
pub fn collect_transitive_predecessors(node: &str, graph: &Graph) -> HashSet<String> {
    let mut visited = HashSet::new();

    let preds = match graph.predecessors(node) {
        Ok(p) => p,
        Err(_) => return visited,
    };

    // BFS through predecessors.
    let mut queue: VecDeque<String> = VecDeque::new();
    for p in &preds {
        visited.insert(p.clone());
        queue.push_back(p.clone());
    }

    while let Some(current) = queue.pop_front() {
        let more_preds = match graph.predecessors(&current) {
            Ok(p) => p,
            Err(_) => continue,
        };
        for pred in more_preds {
            if visited.insert(pred.clone()) {
                queue.push_back(pred);
            }
        }
    }

    visited
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_hash_file() {
        let dir = std::env::temp_dir().join(format!(
            "build_tool_hashfile_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let file = dir.join("test.py");
        fs::write(&file, "print('hello')").unwrap();

        let hash = hash_file(&file).unwrap();
        // SHA256 hashes are 64 hex characters.
        assert_eq!(hash.len(), 64);

        // Same content should produce same hash.
        let hash2 = hash_file(&file).unwrap();
        assert_eq!(hash, hash2);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_hash_package_empty() {
        let pkg = Package {
            name: "test/empty".to_string(),
            path: PathBuf::from("/nonexistent/path"),
            build_commands: vec![],
            language: "python".to_string(),
        };

        let hash = hash_package(&pkg);
        // Should be the hash of the empty string.
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_hash_package_deterministic() {
        let dir = std::env::temp_dir().join(format!(
            "build_tool_hashpkg_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        fs::write(dir.join("main.py"), "print('hello')").unwrap();
        fs::write(dir.join("BUILD"), "pytest").unwrap();

        let pkg = Package {
            name: "python/test-pkg".to_string(),
            path: dir.clone(),
            build_commands: vec!["pytest".to_string()],
            language: "python".to_string(),
        };

        let hash1 = hash_package(&pkg);
        let hash2 = hash_package(&pkg);
        assert_eq!(hash1, hash2); // Deterministic.

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_hash_deps_no_deps() {
        let mut graph = Graph::new();
        graph.add_node("A");

        let hashes: HashMap<String, String> = HashMap::new();
        let hash = hash_deps("A", &graph, &hashes);
        // No dependencies means hash of empty string.
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_hash_deps_with_deps() {
        let mut graph = Graph::new();
        graph.add_edge("A", "B"); // B depends on A

        let mut hashes = HashMap::new();
        hashes.insert("A".to_string(), "abc123".to_string());
        hashes.insert("B".to_string(), "def456".to_string());

        let hash = hash_deps("B", &graph, &hashes);
        assert_eq!(hash.len(), 64);

        // Hash should change if dependency hash changes.
        hashes.insert("A".to_string(), "changed".to_string());
        let hash2 = hash_deps("B", &graph, &hashes);
        assert_ne!(hash, hash2);
    }

    #[test]
    fn test_collect_transitive_predecessors() {
        let mut graph = Graph::new();
        graph.add_edge("A", "B");
        graph.add_edge("B", "C");

        let preds = collect_transitive_predecessors("C", &graph);
        assert!(preds.contains("A"));
        assert!(preds.contains("B"));
        assert!(!preds.contains("C"));
    }
}
