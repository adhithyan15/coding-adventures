//! `meta::TREE_ENTRIES` must cover every path package of the probe's dependency closure
//! (VOS-D140): `Cargo.lock` pins registry crates but not the content of a path dependency, so a
//! path crate whose `src` and `Cargo.toml` are not recorded could change a recorded value without
//! staling the recording.

use std::path::Path;
use vos_capacity_probe::meta::{repo_root, TREE_ENTRIES};

/// The names of the packages in a `Cargo.lock` that have no `source` line: path packages.
fn path_packages(lock: &str) -> Vec<String> {
    let mut out = Vec::new();
    for block in lock.split("[[package]]").skip(1) {
        let name = block
            .lines()
            .find_map(|l| l.strip_prefix("name = \"")?.strip_suffix('"'))
            .expect("a package has a name");
        if !block.lines().any(|l| l.starts_with("source = ")) {
            out.push(name.to_string());
        }
    }
    out
}

#[test]
fn every_path_package_in_the_lockfile_has_its_src_and_manifest_recorded() {
    let lock =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.lock")).unwrap();
    let pkgs = path_packages(&lock);
    assert!(pkgs.len() > 10, "the lockfile parse found {pkgs:?}");
    let root = repo_root();
    for name in pkgs.iter().filter(|n| *n != "vos-capacity-probe") {
        // A package's directory is its name, or its name without the `coding_adventures_` prefix
        // and with `_` as `-` (the repository's convention).
        let alt = name
            .trim_start_matches("coding_adventures_")
            .replace('_', "-");
        let dir = [name.clone(), alt]
            .into_iter()
            .find(|d| {
                root.join("code/packages/rust")
                    .join(d)
                    .join("Cargo.toml")
                    .is_file()
            })
            .unwrap_or_else(|| panic!("no code/packages/rust directory for path package {name}"));
        for leaf in ["src", "Cargo.toml"] {
            let path = format!("code/packages/rust/{dir}/{leaf}");
            assert!(
                TREE_ENTRIES.iter().any(|(_, p)| *p == path),
                "path package {name} is in the probe's Cargo.lock but {path} is not in meta::TREE_ENTRIES"
            );
        }
        assert!(
            !root
                .join("code/packages/rust")
                .join(&dir)
                .join("build.rs")
                .exists(),
            "{dir} has a build.rs: add it to TREE_ENTRIES"
        );
    }
}

#[test]
fn entry_names_and_paths_are_unique_and_exist_in_git() {
    let mut names: Vec<_> = TREE_ENTRIES.iter().map(|(n, _)| *n).collect();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), TREE_ENTRIES.len(), "duplicate entry name");
    let root = repo_root();
    for (n, p) in TREE_ENTRIES {
        assert!(root.join(p).exists(), "{n}: {p} does not exist");
    }
}
