//! # Integration Tests for tar
//!
//! These tests verify tar archive creation, listing, and extraction
//! using real files on the filesystem.

use unix_tools::tar_tool::*;
use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn temp_path(name: &str) -> String {
    std::env::temp_dir()
        .join(format!("tar_integ_{}", name))
        .to_string_lossy()
        .into_owned()
}

fn cleanup(path: &str) {
    let p = Path::new(path);
    if p.is_dir() {
        let _ = fs::remove_dir_all(p);
    } else {
        let _ = fs::remove_file(p);
    }
}

// ---------------------------------------------------------------------------
// Tests: Header encoding/decoding
// ---------------------------------------------------------------------------

#[cfg(test)]
mod header {
    use super::*;

    #[test]
    fn roundtrip() {
        let header = TarHeader {
            name: "hello.txt".to_string(),
            mode: 0o644,
            uid: 501,
            gid: 20,
            size: 13,
            mtime: 1700000000,
            typeflag: b'0',
            uname: "user".to_string(),
            gname: "staff".to_string(),
        };

        let bytes = header.to_bytes();
        let parsed = TarHeader::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.name, "hello.txt");
        assert_eq!(parsed.mode, 0o644);
        assert_eq!(parsed.uid, 501);
        assert_eq!(parsed.gid, 20);
        assert_eq!(parsed.size, 13);
        assert_eq!(parsed.typeflag, b'0');
    }

    #[test]
    fn zero_block_is_eof() {
        let block = [0u8; 512];
        assert!(TarHeader::from_bytes(&block).is_none());
    }

    #[test]
    fn directory_typeflag() {
        let header = TarHeader {
            name: "mydir/".to_string(),
            mode: 0o755,
            uid: 0,
            gid: 0,
            size: 0,
            mtime: 0,
            typeflag: b'5',
            uname: String::new(),
            gname: String::new(),
        };

        let bytes = header.to_bytes();
        let parsed = TarHeader::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.typeflag, b'5');
        assert!(parsed.name.contains("mydir"));
    }

    #[test]
    fn large_file_size() {
        let header = TarHeader {
            name: "big.bin".to_string(),
            mode: 0o644,
            uid: 0,
            gid: 0,
            size: 1048576, // 1 MB
            mtime: 0,
            typeflag: b'0',
            uname: String::new(),
            gname: String::new(),
        };

        let bytes = header.to_bytes();
        let parsed = TarHeader::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.size, 1048576);
    }
}

// ---------------------------------------------------------------------------
// Tests: Create and list
// ---------------------------------------------------------------------------

#[cfg(test)]
mod create_list {
    use super::*;

    #[test]
    fn create_single_file() {
        let dir = temp_path("create_single");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(format!("{}/test.txt", dir), "hello world").unwrap();

        let opts = TarOptions {
            directory: Some(dir.clone()),
            verbose: false,
        };
        let files = vec!["test.txt".to_string()];
        let (archive, _) = create_archive(&files, &opts).unwrap();

        let entries = list_archive(&archive, &TarOptions::default()).unwrap();
        assert_eq!(entries, vec!["test.txt"]);

        cleanup(&dir);
    }

    #[test]
    fn create_multiple_files() {
        let dir = temp_path("create_multi");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(format!("{}/a.txt", dir), "aaa").unwrap();
        fs::write(format!("{}/b.txt", dir), "bbb").unwrap();
        fs::write(format!("{}/c.txt", dir), "ccc").unwrap();

        let opts = TarOptions {
            directory: Some(dir.clone()),
            verbose: false,
        };
        let files = vec!["a.txt".into(), "b.txt".into(), "c.txt".into()];
        let (archive, _) = create_archive(&files, &opts).unwrap();

        let entries = list_archive(&archive, &TarOptions::default()).unwrap();
        assert_eq!(entries.len(), 3);
        assert!(entries.contains(&"a.txt".to_string()));
        assert!(entries.contains(&"b.txt".to_string()));
        assert!(entries.contains(&"c.txt".to_string()));

        cleanup(&dir);
    }

    #[test]
    fn create_with_directory() {
        let dir = temp_path("create_dir");
        cleanup(&dir);
        fs::create_dir_all(format!("{}/subdir", dir)).unwrap();
        fs::write(format!("{}/subdir/inner.txt", dir), "inside").unwrap();

        let opts = TarOptions {
            directory: Some(dir.clone()),
            verbose: true,
        };
        let files = vec!["subdir".to_string()];
        let (archive, messages) = create_archive(&files, &opts).unwrap();

        assert!(!messages.is_empty());
        let entries = list_archive(&archive, &TarOptions::default()).unwrap();
        assert!(entries.iter().any(|e| e.contains("subdir")));

        cleanup(&dir);
    }

    #[test]
    fn verbose_list_shows_permissions() {
        let dir = temp_path("verbose_list");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(format!("{}/file.txt", dir), "data").unwrap();

        let opts = TarOptions {
            directory: Some(dir.clone()),
            verbose: false,
        };
        let files = vec!["file.txt".to_string()];
        let (archive, _) = create_archive(&files, &opts).unwrap();

        let verbose_opts = TarOptions { verbose: true, ..Default::default() };
        let entries = list_archive(&archive, &verbose_opts).unwrap();
        assert!(entries[0].contains("rw"));

        cleanup(&dir);
    }

    #[test]
    fn empty_archive() {
        let archive = vec![0u8; 1024]; // Two zero blocks
        let entries = list_archive(&archive, &TarOptions::default()).unwrap();
        assert!(entries.is_empty());
    }
}

// ---------------------------------------------------------------------------
// Tests: Extract
// ---------------------------------------------------------------------------

#[cfg(test)]
mod extract {
    use super::*;

    #[test]
    fn extract_single_file() {
        let src = temp_path("extract_src");
        let dst = temp_path("extract_dst");
        cleanup(&src);
        cleanup(&dst);
        fs::create_dir_all(&src).unwrap();
        fs::write(format!("{}/data.txt", src), "extract me").unwrap();

        let opts = TarOptions {
            directory: Some(src.clone()),
            verbose: false,
        };
        let (archive, _) = create_archive(&["data.txt".into()], &opts).unwrap();

        fs::create_dir_all(&dst).unwrap();
        extract_archive(&archive, &dst, &TarOptions::default()).unwrap();

        let content = fs::read_to_string(format!("{}/data.txt", dst)).unwrap();
        assert_eq!(content, "extract me");

        cleanup(&src);
        cleanup(&dst);
    }

    #[test]
    fn extract_preserves_content() {
        let src = temp_path("preserve_src");
        let dst = temp_path("preserve_dst");
        cleanup(&src);
        cleanup(&dst);
        fs::create_dir_all(&src).unwrap();

        let content = "Line 1\nLine 2\nLine 3\n";
        fs::write(format!("{}/multi.txt", src), content).unwrap();

        let opts = TarOptions {
            directory: Some(src.clone()),
            verbose: false,
        };
        let (archive, _) = create_archive(&["multi.txt".into()], &opts).unwrap();

        fs::create_dir_all(&dst).unwrap();
        extract_archive(&archive, &dst, &TarOptions::default()).unwrap();

        let extracted = fs::read_to_string(format!("{}/multi.txt", dst)).unwrap();
        assert_eq!(extracted, content);

        cleanup(&src);
        cleanup(&dst);
    }

    #[test]
    fn extract_directory_structure() {
        let src = temp_path("dir_struct_src");
        let dst = temp_path("dir_struct_dst");
        cleanup(&src);
        cleanup(&dst);
        fs::create_dir_all(format!("{}/sub", src)).unwrap();
        fs::write(format!("{}/sub/nested.txt", src), "nested").unwrap();

        let opts = TarOptions {
            directory: Some(src.clone()),
            verbose: false,
        };
        let (archive, _) = create_archive(&["sub".into()], &opts).unwrap();

        fs::create_dir_all(&dst).unwrap();
        extract_archive(&archive, &dst, &TarOptions::default()).unwrap();

        assert!(Path::new(&format!("{}/sub", dst)).is_dir());
        let content = fs::read_to_string(format!("{}/sub/nested.txt", dst)).unwrap();
        assert_eq!(content, "nested");

        cleanup(&src);
        cleanup(&dst);
    }

    #[test]
    fn extract_verbose_reports_files() {
        let src = temp_path("verbose_extract_src");
        let dst = temp_path("verbose_extract_dst");
        cleanup(&src);
        cleanup(&dst);
        fs::create_dir_all(&src).unwrap();
        fs::write(format!("{}/v.txt", src), "verbose").unwrap();

        let opts = TarOptions {
            directory: Some(src.clone()),
            verbose: false,
        };
        let (archive, _) = create_archive(&["v.txt".into()], &opts).unwrap();

        fs::create_dir_all(&dst).unwrap();
        let verbose_opts = TarOptions { verbose: true, ..Default::default() };
        let messages = extract_archive(&archive, &dst, &verbose_opts).unwrap();
        assert!(messages.contains(&"v.txt".to_string()));

        cleanup(&src);
        cleanup(&dst);
    }
}

// ---------------------------------------------------------------------------
// Tests: Security
// ---------------------------------------------------------------------------

#[cfg(test)]
mod security {
    use super::*;

    #[test]
    fn rejects_path_traversal() {
        let header = TarHeader {
            name: "../../../etc/passwd".to_string(),
            mode: 0o644,
            uid: 0,
            gid: 0,
            size: 0,
            mtime: 0,
            typeflag: b'0',
            uname: String::new(),
            gname: String::new(),
        };

        let mut archive = Vec::new();
        archive.extend_from_slice(&header.to_bytes());
        archive.extend_from_slice(&[0u8; 1024]);

        let result = extract_archive(&archive, "/tmp/tar_integ_security", &TarOptions::default());
        assert!(result.is_err());
    }
}

// ---------------------------------------------------------------------------
// Tests: Error handling
// ---------------------------------------------------------------------------

#[cfg(test)]
mod errors {
    use super::*;

    #[test]
    fn create_nonexistent_file() {
        let opts = TarOptions::default();
        let result = create_archive(&["nonexistent_file_xyz.txt".into()], &opts);
        assert!(result.is_err());
    }
}
