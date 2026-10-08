//! The broker-root rules, against a real filesystem (D18S S-K5, P2.6c).
//!
//! Each test builds its own scratch tree:
//!
//! ```text
//!   <scratch>/
//!     outside/secret        what an escape would reach
//!     root/                 the broker root
//!       notes.md            an ordinary file
//!       ...                 whatever the test plants
//! ```

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod supported {
    use chief_of_staff_broker_roots::{Access, BrokerRoot, BrokerRootError};
    use std::fs;
    use std::io::{Read, Write};
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::symlink;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Scratch {
        base: PathBuf,
    }

    impl Scratch {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let base = std::env::temp_dir().join(format!(
                "broker-roots-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let _ = fs::remove_dir_all(&base);
            fs::create_dir_all(base.join("outside")).unwrap();
            fs::create_dir_all(base.join("root")).unwrap();
            fs::write(base.join("outside/secret"), b"the vault").unwrap();
            fs::write(base.join("root/notes.md"), b"today").unwrap();
            Self { base }
        }

        fn path(&self, relative: &str) -> PathBuf {
            self.base.join(relative)
        }

        fn root(&self) -> BrokerRoot {
            BrokerRoot::open(&self.path("root"), &[&self.path("outside")]).unwrap()
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.base);
        }
    }

    fn refused(result: Result<fs::File, BrokerRootError>) -> BrokerRootError {
        match result {
            Ok(_) => panic!("opened something it should have refused"),
            Err(error) => error,
        }
    }

    // ---- what is allowed --------------------------------------------------

    #[test]
    fn a_plain_file_beneath_the_root_opens_for_the_access_asked() {
        let scratch = Scratch::new();
        let root = scratch.root();
        assert_eq!(root.path(), fs::canonicalize(scratch.path("root")).unwrap());

        let mut text = String::new();
        let mut read = root.open_beneath("notes.md", Access::Read).unwrap();
        read.read_to_string(&mut text).unwrap();
        assert_eq!(text, "today");
        assert!(
            read.write_all(b"x").is_err(),
            "a read handle must not write"
        );

        let mut write = root.open_beneath("notes.md", Access::Write).unwrap();
        write.write_all(b"TOD").unwrap();
        let mut back = Vec::new();
        assert!(
            write.read_to_end(&mut back).is_err(),
            "a write handle must not read"
        );
        drop(write);
        // Opened without truncation: the rest of the file is still there.
        assert_eq!(fs::read(scratch.path("root/notes.md")).unwrap(), b"TODay");
    }

    #[test]
    fn a_file_in_a_subdirectory_opens() {
        let scratch = Scratch::new();
        fs::create_dir_all(scratch.path("root/a/b")).unwrap();
        fs::write(scratch.path("root/a/b/c.txt"), b"deep").unwrap();
        let mut text = String::new();
        scratch
            .root()
            .open_beneath("a/b/c.txt", Access::Read)
            .unwrap()
            .read_to_string(&mut text)
            .unwrap();
        assert_eq!(text, "deep");
    }

    #[test]
    fn the_returned_file_is_close_on_exec_and_blocking() {
        let scratch = Scratch::new();
        let file = scratch
            .root()
            .open_beneath("notes.md", Access::Read)
            .unwrap();
        // SAFETY: flag queries on a descriptor the test owns.
        let fd_flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFD) };
        let status = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) };
        assert!(fd_flags & libc::FD_CLOEXEC != 0);
        assert_eq!(status & libc::O_NONBLOCK, 0);
    }

    // ---- names that never reach the kernel --------------------------------

    #[test]
    fn dot_dot_absolute_and_malformed_names_are_refused_before_any_open() {
        let scratch = Scratch::new();
        let root = scratch.root();
        for name in [
            "../outside/secret",
            "a/../../outside/secret",
            "/etc/passwd",
            "",
            ".",
            "./notes.md",
            "a//notes.md",
            "notes.md/",
            "notes\0.md",
        ] {
            assert!(
                matches!(
                    refused(root.open_beneath(name, Access::Read)),
                    BrokerRootError::InvalidName(_)
                ),
                "{name:?}"
            );
        }
    }

    // ---- what the kernel refuses ------------------------------------------

    #[test]
    fn a_symlink_out_of_the_root_is_refused() {
        let scratch = Scratch::new();
        symlink(scratch.path("outside/secret"), scratch.path("root/link")).unwrap();
        symlink(scratch.path("outside"), scratch.path("root/dir")).unwrap();
        symlink("../outside/secret", scratch.path("root/relative")).unwrap();
        let root = scratch.root();
        for name in ["link", "dir/secret", "relative"] {
            for access in [Access::Read, Access::Write] {
                assert!(
                    matches!(
                        refused(root.open_beneath(name, access)),
                        BrokerRootError::Refused(_)
                    ),
                    "{name} {access:?}"
                );
            }
        }
        // And nothing was written through any of them.
        assert_eq!(
            fs::read(scratch.path("outside/secret")).unwrap(),
            b"the vault"
        );
    }

    #[test]
    fn a_symlink_that_stays_inside_the_root_is_refused_too() {
        // No symlink at all, rather than judging where one points: a link
        // that points inside today can be swapped to point outside tomorrow.
        let scratch = Scratch::new();
        symlink("notes.md", scratch.path("root/alias")).unwrap();
        assert!(matches!(
            refused(scratch.root().open_beneath("alias", Access::Read)),
            BrokerRootError::Refused(_)
        ));
    }

    #[test]
    fn a_symlink_to_proc_is_refused() {
        let scratch = Scratch::new();
        symlink("/proc/self/root/etc/passwd", scratch.path("root/magic")).unwrap();
        symlink("/proc/self", scratch.path("root/self")).unwrap();
        let root = scratch.root();
        for name in ["magic", "self/environ"] {
            assert!(
                matches!(
                    refused(root.open_beneath(name, Access::Read)),
                    BrokerRootError::Refused(_)
                ),
                "{name}"
            );
        }
    }

    #[test]
    fn a_missing_file_is_refused_and_not_created() {
        let scratch = Scratch::new();
        let root = scratch.root();
        for access in [Access::Read, Access::Write] {
            assert!(matches!(
                refused(root.open_beneath("absent.md", access)),
                BrokerRootError::Refused(_)
            ));
        }
        assert!(!scratch.path("root/absent.md").exists());
    }

    // ---- what is opened but not handed over -------------------------------

    #[test]
    fn a_directory_is_never_returned() {
        let scratch = Scratch::new();
        fs::create_dir(scratch.path("root/sub")).unwrap();
        assert_eq!(
            refused(scratch.root().open_beneath("sub", Access::Read)),
            BrokerRootError::NotRegularFile
        );
    }

    #[test]
    fn a_hard_link_to_a_file_outside_is_refused() {
        let scratch = Scratch::new();
        fs::hard_link(scratch.path("outside/secret"), scratch.path("root/linked")).unwrap();
        let root = scratch.root();
        for access in [Access::Read, Access::Write] {
            assert_eq!(
                refused(root.open_beneath("linked", access)),
                BrokerRootError::MultipleLinks
            );
        }
        assert_eq!(
            fs::read(scratch.path("outside/secret")).unwrap(),
            b"the vault"
        );
    }

    #[test]
    fn a_fifo_neither_hangs_the_broker_nor_is_returned() {
        let scratch = Scratch::new();
        let fifo = std::ffi::CString::new(scratch.path("root/pipe").as_os_str().as_encoded_bytes())
            .unwrap();
        // SAFETY: a NUL-terminated path.
        assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
        // With no writer, a blocking read-open would wait forever. The opens
        // run on a thread with a deadline, so a regression fails the test
        // rather than hanging it.
        let root = scratch.root();
        let (done, results) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let read = root.open_beneath("pipe", Access::Read).map(drop);
            let write = root.open_beneath("pipe", Access::Write).map(drop);
            let _ = done.send((read, write));
        });
        let (read, write) = results
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("opening a FIFO blocked the broker");
        // Opened non-blocking, it returns at once and is refused as not a file.
        assert_eq!(read, Err(BrokerRootError::NotRegularFile));
        // A non-blocking write-open with no reader fails outright (ENXIO).
        assert!(matches!(
            write,
            Err(BrokerRootError::Refused(_) | BrokerRootError::NotRegularFile)
        ));
    }

    // ---- root disjointness -------------------------------------------------

    fn overlap(root: &Path, never_grantable: &[&Path]) -> bool {
        matches!(
            BrokerRoot::open(root, never_grantable),
            Err(BrokerRootError::Overlap { .. })
        )
    }

    #[test]
    fn a_root_inside_a_never_grantable_path_is_refused() {
        let scratch = Scratch::new();
        fs::create_dir(scratch.path("outside/inner")).unwrap();
        assert!(overlap(
            &scratch.path("outside/inner"),
            &[&scratch.path("outside")]
        ));
        assert!(overlap(
            &scratch.path("outside"),
            &[&scratch.path("outside")]
        ));
    }

    #[test]
    fn a_root_containing_a_never_grantable_path_is_refused() {
        let scratch = Scratch::new();
        assert!(overlap(
            &scratch.path(""),
            &[&scratch.path("outside/secret")]
        ));
        // Not existing yet (a vault about to be created) still counts.
        assert!(overlap(
            &scratch.path("root"),
            &[&scratch.path("root/vault/not-yet/sealed.bin")]
        ));
    }

    #[test]
    fn a_symlink_cannot_disguise_an_overlap_either_way() {
        let scratch = Scratch::new();
        // The root named through a symlink that leads into the vault area.
        symlink(scratch.path("outside"), scratch.path("disguised-root")).unwrap();
        assert!(overlap(
            &scratch.path("disguised-root"),
            &[&scratch.path("outside")]
        ));
        // The never-grantable path named through a symlink into the root.
        symlink(scratch.path("root"), scratch.path("disguised-vault")).unwrap();
        assert!(overlap(
            &scratch.path("root"),
            &[&scratch.path("disguised-vault/sealed.bin")]
        ));
    }

    #[test]
    fn a_sibling_sharing_a_name_prefix_is_not_an_overlap() {
        // Containment is by whole path components: root2 is not inside root.
        let scratch = Scratch::new();
        fs::create_dir(scratch.path("root2")).unwrap();
        assert!(BrokerRoot::open(&scratch.path("root"), &[&scratch.path("root2")]).is_ok());
        assert!(BrokerRoot::open(&scratch.path("root2"), &[&scratch.path("root")]).is_ok());
    }

    #[test]
    fn a_root_must_be_an_existing_directory() {
        let scratch = Scratch::new();
        for root in ["missing", "root/notes.md"] {
            assert!(matches!(
                BrokerRoot::open(&scratch.path(root), &[]),
                Err(BrokerRootError::RootNotDirectory(_))
            ));
        }
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
#[test]
fn without_a_beneath_primitive_no_root_is_offered() {
    use chief_of_staff_broker_roots::{BrokerRoot, BrokerRootError};
    assert_eq!(
        BrokerRoot::open(&std::env::temp_dir(), &[]).unwrap_err(),
        BrokerRootError::Unsupported
    );
}
