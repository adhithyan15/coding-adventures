//! # sir-test-scratch
//!
//! Where the Semantic IR backends' execution tests put the files they make.
//!
//! Every `semantic-ir-to-*` backend is tested by *running* what it emits:
//! write the emitted program to a file, hand the file to the real toolchain
//! (`node`, `python3`, `go run`, `rustc`, `cc`, `ruby`), and compare what it
//! prints.  This crate owns the two shared resources those tests touch
//! outside their own process -- the temp directory and Go's build cache --
//! so that no other process can reach into a test while it runs.
//!
//! ## 1. Private scratch directories instead of names in the shared `/tmp`
//!
//! The tests used to build file names like
//!
//! ```text
//! /tmp/sir_go_twig_pc_car_<pid>_<counter>.go
//! ```
//!
//! directly in the system temp directory.  Process id plus a per-process
//! counter makes the name unique among *this* checkout's tests, but not
//! *private*: `/tmp` is shared by every user and every process on the host,
//! and the name is predictable, so anything else on the machine can create
//! it first.
//!
//! | what another process does first        | what the test then did                         |
//! |----------------------------------------|------------------------------------------------|
//! | creates the file                       | `fs::write` truncated and reused it            |
//! | plants a symlink under that name       | `fs::write` followed it and wrote *elsewhere*  |
//! | drops `json.py` beside a `.py` test    | `python3 /tmp/x.py` put `/tmp` on `sys.path`, so a later `import json` ran the planted file |
//!
//! [`Scratch`] replaces the shared directory with a fresh one per call:
//!
//! * **random name** -- `<tmp>/<prefix><random>`, chosen by [`tempfile`];
//! * **created atomically** -- `create_dir`, which fails rather than reuse a
//!   directory that already exists (a new random name is tried instead);
//! * **mode `0700`** on Unix -- no other user can list, create or replace
//!   anything inside;
//! * **removed on drop** -- the directory and everything in it, on every
//!   path out of the test, panics included.
//!
//! Files inside are opened with `create_new` ([`write_new`], [`create_new`]):
//! a file that is somehow already there is an error, never silently reused.
//!
//! Why per *call* and not one directory per *process*?  A per-process
//! directory would have to live in a `static`, and Rust never drops a
//! `static`, so the directory would outlive every test run -- one empty
//! directory left in `/tmp` per test binary per run.  A per-call guard is the
//! narrowest owner that is still dropped, and creating a directory costs
//! about as much as creating the file inside it.
//!
//! ## 2. A Go build cache nothing else deletes
//!
//! `go run prog.go` compiles `prog.go` against the standard library's
//! export data, which it keeps in the **build cache**
//! (`$GOCACHE`, by default `~/.cache/go-build`).  That cache is shared by
//! every Go command the user runs.  If another process deletes cache
//! entries while a test's `go run` is between "found `fmt` in the cache" and
//! "the compiler or linker opened it", the test fails with a *compile*
//! error that has nothing to do with the program:
//!
//! ```text
//! # command-line-arguments
//! .../link: cannot open file $GOCACHE/06/067a...-d: open $GOCACHE/06/067a...-d: no such file or directory
//! ```
//!
//! `go clean -cache` in another terminal does exactly that -- and is the
//! usual advice when that cache grows on a full disk.  This is the root cause
//! of the intermittent "failed to compile" in `semantic-ir-to-go`'s
//! `twig_pairs_and_closures.rs`: both of its tests failed in one run and
//! then passed fifteen reruns.  Running the test binary while a loop runs
//! `go clean -cache` reproduces it every time; see this crate's README.
//!
//! [`go_command`] returns a `go` [`Command`] whose `GOCACHE` is a directory
//! under this checkout's own cargo target directory ([`go_build_cache`]).
//! Go's cache is safe for any number of concurrent `go` processes; what it
//! cannot survive is deletion from outside, and nothing outside these tests
//! knows that directory.  It is reused across runs (so the standard library
//! is compiled once, not per test) and `cargo clean` removes it.

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::process::Command;

/// A private directory for one test call's files, removed (with everything
/// in it) when the guard drops.  See the crate documentation for why.
///
/// ```
/// let scratch = sir_test_scratch::Scratch::new("doc_");
/// let src = scratch.write("prog.txt", "hello").expect("write");
/// assert_eq!(std::fs::read_to_string(&src).unwrap(), "hello");
/// let dir = scratch.path().to_path_buf();
/// drop(scratch);
/// assert!(!dir.exists(), "dropping the guard removes the directory");
/// ```
#[derive(Debug)]
pub struct Scratch {
    dir: tempfile::TempDir,
}

impl Scratch {
    /// Create a fresh private directory in the system temp directory, named
    /// `<prefix><random>`.  The prefix only says, in a directory listing,
    /// which test made it.
    ///
    /// # Panics
    ///
    /// When the directory cannot be created.  This is test scaffolding: a
    /// test that cannot make its scratch directory cannot run, and the panic
    /// says why.
    pub fn new(prefix: &str) -> Scratch {
        let mut builder = tempfile::Builder::new();
        builder.prefix(prefix);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            builder.permissions(std::fs::Permissions::from_mode(0o700));
        }
        let dir = builder
            .tempdir()
            .unwrap_or_else(|e| panic!("create a private scratch directory `{prefix}…`: {e}"));
        Scratch { dir }
    }

    /// The directory itself.
    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// The path of the file `name` inside the directory.
    ///
    /// # Panics
    ///
    /// When `name` is not a single plain file name (it is empty, absolute,
    /// or contains a separator, `.` or `..`): every file a test makes
    /// belongs *in* its scratch directory, never beside it.
    pub fn join(&self, name: &str) -> PathBuf {
        let mut parts = Path::new(name).components();
        let plain = matches!(parts.next(), Some(Component::Normal(_))) && parts.next().is_none();
        assert!(plain, "scratch file name `{name}` must be a single plain file name");
        self.dir.path().join(name)
    }

    /// Create the file `name` inside the directory with `create_new` and
    /// write `contents` to it; returns its path.
    pub fn write(&self, name: &str, contents: impl AsRef<[u8]>) -> io::Result<PathBuf> {
        let path = self.join(name);
        write_new(&path, contents)?;
        Ok(path)
    }
}

/// Like [`std::fs::write`], except that the file must not exist yet.
///
/// `fs::write` opens with `create` + `truncate`: an existing file is reused
/// and a symlink is followed.  Here the open uses `create_new`
/// (`O_CREAT | O_EXCL`), which fails with `AlreadyExists` instead -- and
/// `O_EXCL` also refuses to follow a symlink in the final component.
pub fn write_new(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> io::Result<()> {
    create_new(path)?.write_all(contents.as_ref())
}

/// Like [`File::create`], except that the file must not exist yet (see
/// [`write_new`]).
pub fn create_new(path: impl AsRef<Path>) -> io::Result<File> {
    OpenOptions::new().write(true).create_new(true).open(path)
}

/// The Go build cache these tests use: `<profile dir>/sir-go-build-cache`,
/// where `<profile dir>` is the cargo target directory's `debug/` (or
/// `release/`, …) that holds the running test binary.
///
/// A test binary lives at `<target>/<profile>/deps/<name>-<hash>` (Cargo up to
/// 1.97) or at `<target>/<profile>/build/<name>/<hash>/out/<name>-<hash>`
/// (Cargo 1.100's build-dir layout), and a binary built by `cargo build` at
/// `<target>/<profile>/<name>`; all three map to
/// `<target>/<profile>/sir-go-build-cache`.  `None` when the running
/// executable's path is unknown; [`go_command`] then leaves `GOCACHE` alone.
pub fn go_build_cache() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let profile_dir = if dir.file_name().is_some_and(|n| n == "deps") {
        dir.parent()?
    } else if dir.file_name().is_some_and(|n| n == "out") {
        // `out` / `<hash>` / `<name>` / `build` / `<profile>`: only when the
        // path really has that shape, so a plain binary that happens to sit
        // in a directory called `out` is not mistaken for it.
        let build = dir.parent()?.parent()?.parent()?;
        if build.file_name().is_some_and(|n| n == "build") {
            build.parent()?
        } else {
            dir
        }
    } else {
        dir
    };
    Some(profile_dir.join("sir-go-build-cache"))
}

/// `go` with its build cache set to [`go_build_cache`], so that no
/// `go clean -cache` (or anything else outside these tests) can delete a
/// cache entry while a test is compiling against it.  See the crate
/// documentation for the failure this prevents.
pub fn go_command() -> Command {
    let mut cmd = Command::new("go");
    if let Some(cache) = go_build_cache() {
        cmd.env("GOCACHE", cache);
    }
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scratch_directory_is_fresh_private_and_removed_on_drop() {
        let a = Scratch::new("sir_scratch_test_");
        let b = Scratch::new("sir_scratch_test_");
        assert_ne!(a.path(), b.path(), "every call gets its own directory");
        assert!(a.path().is_dir());
        assert!(a.path().starts_with(std::env::temp_dir()));
        let name = a.path().file_name().unwrap().to_string_lossy().into_owned();
        assert!(name.starts_with("sir_scratch_test_"), "{name}");
        assert!(name.len() > "sir_scratch_test_".len(), "a random suffix follows the prefix");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(a.path()).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o700, "only the owner may enter: {mode:o}");
        }
        let file = a.write("prog.go", "package main").unwrap();
        let dir = a.path().to_path_buf();
        drop(a);
        assert!(!file.exists() && !dir.exists(), "the directory and its files are gone");
    }

    #[test]
    fn a_scratch_directory_is_removed_when_its_test_panics() {
        let dir = std::panic::catch_unwind(|| {
            let s = Scratch::new("sir_scratch_test_");
            s.write("left.txt", "x").unwrap();
            let dir = s.path().to_path_buf();
            std::panic::panic_any(dir);
        })
        .expect_err("the closure panics");
        let dir = dir.downcast::<PathBuf>().expect("the panic carries the path");
        assert!(!dir.exists(), "unwinding drops the guard");
    }

    #[test]
    fn write_new_refuses_an_existing_file() {
        let s = Scratch::new("sir_scratch_test_");
        let p = s.write("once.txt", "first").unwrap();
        let err = write_new(&p, "second").expect_err("the file already exists");
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "first", "never truncated");
        assert_eq!(s.write("once.txt", "again").unwrap_err().kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(create_new(&p).unwrap_err().kind(), io::ErrorKind::AlreadyExists);
    }

    #[cfg(unix)]
    #[test]
    fn write_new_does_not_follow_a_planted_symlink() {
        let s = Scratch::new("sir_scratch_test_");
        let target = s.join("target.txt");
        std::os::unix::fs::symlink(&target, s.join("planted.txt")).unwrap();
        assert!(write_new(s.join("planted.txt"), "x").is_err());
        assert!(!target.exists(), "nothing was written through the link");
    }

    #[test]
    fn join_accepts_only_a_plain_file_name() {
        let s = Scratch::new("sir_scratch_test_");
        assert_eq!(s.join("a.go"), s.path().join("a.go"));
        for bad in ["", "/etc/passwd", "../a.go", "sub/a.go", ".", ".."] {
            let caught = std::panic::catch_unwind(|| s.join(bad));
            assert!(caught.is_err(), "`{bad}` must be refused");
        }
    }

    #[test]
    fn the_go_build_cache_lives_in_the_target_profile_directory() {
        let cache = go_build_cache().expect("a test binary knows its own path");
        assert!(cache.ends_with("sir-go-build-cache"), "{}", cache.display());
        // The cache sits in the profile directory that holds this test binary,
        // whichever Cargo layout put it there (`deps/` through Cargo 1.97,
        // `build/<name>/<hash>/out/` from 1.100): the directory is an ancestor
        // of the running executable and has Cargo's `deps` or `build` inside it.
        let exe = std::env::current_exe().unwrap();
        let profile = cache.parent().unwrap();
        assert!(exe.starts_with(profile), "{} is not under {}", exe.display(), profile.display());
        assert!(
            profile.join("deps").is_dir() || profile.join("build").is_dir(),
            "{} does not look like a cargo profile directory",
            profile.display()
        );
        let cmd = go_command();
        let gocache = cmd.get_envs().find(|(k, _)| *k == "GOCACHE").and_then(|(_, v)| v);
        assert_eq!(gocache, Some(cache.as_os_str()));
    }
}
