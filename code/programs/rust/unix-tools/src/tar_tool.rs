//! # tar — Tape Archive
//!
//! This module implements the business logic for the `tar` command.
//! `tar` creates, extracts, and lists archive files in the Unix tar
//! format.
//!
//! ## The Tar Format
//!
//! A tar archive is a sequence of 512-byte blocks:
//!
//! ```text
//!     ┌──────────────────────┐
//!     │   Header (512 bytes) │  ← File metadata (name, size, mode, etc.)
//!     ├──────────────────────┤
//!     │   Data blocks        │  ← File content, padded to 512-byte boundary
//!     │   (ceil(size/512)    │
//!     │    * 512 bytes)      │
//!     ├──────────────────────┤
//!     │   Header (512 bytes) │  ← Next file...
//!     ├──────────────────────┤
//!     │   Data blocks        │
//!     ├──────────────────────┤
//!     │   ... more files ... │
//!     ├──────────────────────┤
//!     │   End-of-archive     │  ← Two blocks of zeros (1024 bytes)
//!     └──────────────────────┘
//! ```
//!
//! ## Header Layout
//!
//! The 512-byte tar header has these fields:
//!
//! ```text
//!     Offset  Size  Field
//!     ──────  ────  ──────────────────────
//!       0     100   File name
//!     100       8   File mode (octal ASCII)
//!     108       8   Owner UID (octal ASCII)
//!     116       8   Group GID (octal ASCII)
//!     124      12   File size (octal ASCII)
//!     136      12   Modification time (octal, Unix epoch)
//!     148       8   Header checksum
//!     156       1   Type flag ('0'=file, '5'=dir)
//!     157     100   Link name
//!     257       6   Magic ("ustar")
//!     263       2   Version ("00")
//!     265      32   Owner name
//!     297      32   Group name
//!     329       8   Device major
//!     337       8   Device minor
//!     345     155   Prefix (for long filenames)
//!     500      12   Padding
//! ```
//!
//! ## Operations
//!
//! ```text
//!     Flag  Operation  Description
//!     ────  ─────────  ────────────────────────────────
//!     -c    Create     Pack files into an archive
//!     -x    Extract    Unpack files from an archive
//!     -t    List       Show archive contents without extracting
//! ```
//!
//! ## Flags
//!
//! ```text
//!     Flag          Field      Effect
//!     ────────────  ─────────  ──────────────────────────────
//!     -f FILE       file       Archive filename (default: stdout/stdin)
//!     -v            verbose    List files processed
//!     -C DIR        directory  Change to DIR before operating
//!     -z            gzip       (Stub) Compress with gzip
//!     -j            bzip2      (Stub) Compress with bzip2
//!     -J            xz         (Stub) Compress with xz
//! ```

use std::fs;
use std::path::Path;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Every tar block is exactly 512 bytes. This is baked into the format
/// specification and dates back to the block size of tape drives.
const BLOCK_SIZE: usize = 512;

/// The magic string that identifies a POSIX (UStar) tar header.
const USTAR_MAGIC: &[u8] = b"ustar";

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

/// Options controlling how `tar` operates.
#[derive(Debug, Clone, Default)]
pub struct TarOptions {
    /// Print each file as it's processed (-v).
    pub verbose: bool,
    /// Change to this directory before operating (-C).
    pub directory: Option<String>,
}

// ---------------------------------------------------------------------------
// Tar Header
// ---------------------------------------------------------------------------

/// A parsed tar header with all the metadata for one file.
#[derive(Debug, Clone)]
pub struct TarHeader {
    /// File name (relative path).
    pub name: String,
    /// File mode (Unix permissions).
    pub mode: u32,
    /// Owner UID.
    pub uid: u32,
    /// Group GID.
    pub gid: u32,
    /// File size in bytes.
    pub size: u64,
    /// Modification time (Unix timestamp).
    pub mtime: u64,
    /// Type flag: '0' or '\0' for regular file, '5' for directory.
    pub typeflag: u8,
    /// Owner name.
    pub uname: String,
    /// Group name.
    pub gname: String,
}

impl TarHeader {
    /// Serialize this header into a 512-byte tar block.
    ///
    /// ## Encoding Rules
    ///
    /// Tar headers use a peculiar encoding:
    /// - Strings are NUL-padded (fill unused bytes with zeros)
    /// - Numbers are encoded as octal ASCII strings, NUL-terminated
    /// - The checksum is computed over the entire header with the
    ///   checksum field filled with spaces (0x20)
    ///
    /// ```text
    ///     Number 493 (0o755) → "0000755\0" (8 bytes of octal ASCII)
    /// ```
    pub fn to_bytes(&self) -> [u8; BLOCK_SIZE] {
        let mut header = [0u8; BLOCK_SIZE];

        // --- Name (offset 0, 100 bytes) ---
        write_string(&mut header[0..100], &self.name);

        // --- Mode (offset 100, 8 bytes) ---
        write_octal(&mut header[100..108], self.mode as u64, 7);

        // --- UID (offset 108, 8 bytes) ---
        write_octal(&mut header[108..116], self.uid as u64, 7);

        // --- GID (offset 116, 8 bytes) ---
        write_octal(&mut header[116..124], self.gid as u64, 7);

        // --- Size (offset 124, 12 bytes) ---
        write_octal(&mut header[124..136], self.size, 11);

        // --- Mtime (offset 136, 12 bytes) ---
        write_octal(&mut header[136..148], self.mtime, 11);

        // --- Checksum placeholder (offset 148, 8 bytes) ---
        // Fill with spaces — the checksum is computed over the header
        // with this field set to all spaces.
        header[148..156].copy_from_slice(b"        ");

        // --- Type flag (offset 156, 1 byte) ---
        header[156] = self.typeflag;

        // --- Magic (offset 257, 6 bytes) ---
        header[257..262].copy_from_slice(USTAR_MAGIC);

        // --- Version (offset 263, 2 bytes) ---
        header[263..265].copy_from_slice(b"00");

        // --- Owner name (offset 265, 32 bytes) ---
        write_string(&mut header[265..297], &self.uname);

        // --- Group name (offset 297, 32 bytes) ---
        write_string(&mut header[297..329], &self.gname);

        // --- Compute and write checksum ---
        let checksum: u32 = header.iter().map(|&b| b as u32).sum();
        write_octal(&mut header[148..156], checksum as u64, 7);

        header
    }

    /// Parse a 512-byte block into a TarHeader.
    ///
    /// Returns None if the block is all zeros (end-of-archive marker).
    pub fn from_bytes(block: &[u8; BLOCK_SIZE]) -> Option<Self> {
        // --- Check for end-of-archive (all zeros) ---
        if block.iter().all(|&b| b == 0) {
            return None;
        }

        let name = read_string(&block[0..100]);
        let mode = read_octal(&block[100..108]) as u32;
        let uid = read_octal(&block[108..116]) as u32;
        let gid = read_octal(&block[116..124]) as u32;
        let size = read_octal(&block[124..136]);
        let mtime = read_octal(&block[136..148]);
        let typeflag = block[156];
        let uname = read_string(&block[265..297]);
        let gname = read_string(&block[297..329]);

        // --- Handle prefix for long names (UStar extension) ---
        let prefix = read_string(&block[345..500]);
        let full_name = if prefix.is_empty() {
            name
        } else {
            format!("{}/{}", prefix, name)
        };

        Some(TarHeader {
            name: full_name,
            mode,
            uid,
            gid,
            size,
            mtime,
            typeflag,
            uname,
            gname,
        })
    }
}

// ---------------------------------------------------------------------------
// Binary Encoding Helpers
// ---------------------------------------------------------------------------

/// Write a string into a fixed-size field, NUL-padded.
fn write_string(field: &mut [u8], s: &str) {
    let bytes = s.as_bytes();
    let len = bytes.len().min(field.len() - 1);
    field[..len].copy_from_slice(&bytes[..len]);
    // Rest is already zeroed
}

/// Write a number as an octal ASCII string, NUL-terminated.
///
/// ```text
///     write_octal(field, 493, 7)  →  "0000755\0"
///     write_octal(field, 0, 7)    →  "0000000\0"
/// ```
fn write_octal(field: &mut [u8], value: u64, width: usize) {
    let s = format!("{:0>width$o}", value, width = width);
    let bytes = s.as_bytes();
    let len = bytes.len().min(field.len() - 1);
    field[..len].copy_from_slice(&bytes[..len]);
    if len < field.len() {
        field[len] = 0;
    }
}

/// Read a NUL-terminated string from a fixed-size field.
fn read_string(field: &[u8]) -> String {
    let end = field.iter().position(|&b| b == 0).unwrap_or(field.len());
    String::from_utf8_lossy(&field[..end]).trim().to_string()
}

/// Read an octal number from a NUL-terminated ASCII field.
///
/// ```text
///     "0000755\0" → 493
///     "0000000\0" → 0
/// ```
fn read_octal(field: &[u8]) -> u64 {
    let s = read_string(field);
    u64::from_str_radix(s.trim(), 8).unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Create Archive
// ---------------------------------------------------------------------------

/// Create a tar archive from a list of files/directories.
///
/// ## Algorithm
///
/// ```text
///     For each input path:
///         1. If it's a file:
///             a. Write header block (512 bytes)
///             b. Write file data blocks (padded to 512)
///         2. If it's a directory:
///             a. Write directory header (size = 0, typeflag = '5')
///             b. Recurse into contents
///     Finally: Write two blocks of zeros (end-of-archive)
/// ```
pub fn create_archive(
    files: &[String],
    opts: &TarOptions,
) -> Result<(Vec<u8>, Vec<String>), String> {
    let mut archive = Vec::new();
    let mut messages = Vec::new();

    let base_dir = opts.directory.as_deref().unwrap_or(".");

    for file in files {
        let full_path = Path::new(base_dir).join(file);
        add_to_archive(&full_path, file, &mut archive, &mut messages, opts)?;
    }

    // --- End-of-archive marker: two blocks of zeros ---
    archive.extend_from_slice(&[0u8; BLOCK_SIZE * 2]);

    Ok((archive, messages))
}

/// Add a single file or directory to the archive.
fn add_to_archive(
    full_path: &Path,
    archive_name: &str,
    archive: &mut Vec<u8>,
    messages: &mut Vec<String>,
    opts: &TarOptions,
) -> Result<(), String> {
    if !full_path.exists() {
        return Err(format!("tar: {}: No such file or directory", archive_name));
    }

    let metadata = fs::metadata(full_path)
        .map_err(|e| format!("tar: {}: {}", archive_name, e))?;

    if metadata.is_dir() {
        // --- Directory entry ---
        let dir_name = if archive_name.ends_with('/') {
            archive_name.to_string()
        } else {
            format!("{}/", archive_name)
        };

        let header = TarHeader {
            name: dir_name.clone(),
            #[cfg(unix)]
            mode: metadata.permissions().mode() & 0o7777,
            #[cfg(not(unix))]
            mode: 0o755,
            uid: 0,
            gid: 0,
            size: 0,
            mtime: metadata.modified()
                .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs())
                .unwrap_or(0),
            typeflag: b'5',
            uname: String::new(),
            gname: String::new(),
        };

        archive.extend_from_slice(&header.to_bytes());

        if opts.verbose {
            messages.push(dir_name);
        }

        // --- Recurse into directory contents ---
        let mut entries: Vec<_> = fs::read_dir(full_path)
            .map_err(|e| format!("tar: {}: {}", archive_name, e))?
            .filter_map(|e| e.ok())
            .collect();
        entries.sort_by_key(|e| e.file_name());

        for entry in entries {
            let child_name = format!(
                "{}/{}",
                archive_name.trim_end_matches('/'),
                entry.file_name().to_string_lossy()
            );
            add_to_archive(&entry.path(), &child_name, archive, messages, opts)?;
        }
    } else {
        // --- Regular file entry ---
        let content = fs::read(full_path)
            .map_err(|e| format!("tar: {}: {}", archive_name, e))?;

        let header = TarHeader {
            name: archive_name.to_string(),
            #[cfg(unix)]
            mode: metadata.permissions().mode() & 0o7777,
            #[cfg(not(unix))]
            mode: 0o644,
            uid: 0,
            gid: 0,
            size: content.len() as u64,
            mtime: metadata.modified()
                .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs())
                .unwrap_or(0),
            typeflag: b'0',
            uname: String::new(),
            gname: String::new(),
        };

        archive.extend_from_slice(&header.to_bytes());
        archive.extend_from_slice(&content);

        // --- Pad to 512-byte boundary ---
        let remainder = content.len() % BLOCK_SIZE;
        if remainder != 0 {
            let padding = BLOCK_SIZE - remainder;
            archive.extend_from_slice(&vec![0u8; padding]);
        }

        if opts.verbose {
            messages.push(archive_name.to_string());
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// List Archive
// ---------------------------------------------------------------------------

/// List the contents of a tar archive.
///
/// Reads headers one by one, skipping over data blocks, and collects
/// the file names (and optionally metadata) for display.
pub fn list_archive(
    archive_data: &[u8],
    opts: &TarOptions,
) -> Result<Vec<String>, String> {
    let mut entries = Vec::new();
    let mut offset = 0;

    while offset + BLOCK_SIZE <= archive_data.len() {
        let block: [u8; BLOCK_SIZE] = archive_data[offset..offset + BLOCK_SIZE]
            .try_into()
            .map_err(|_| "tar: unexpected end of archive".to_string())?;

        let header = match TarHeader::from_bytes(&block) {
            Some(h) => h,
            None => break, // End-of-archive
        };

        if opts.verbose {
            entries.push(format!(
                "{} {:>8} {}",
                format_tar_mode(header.mode, header.typeflag),
                header.size,
                header.name
            ));
        } else {
            entries.push(header.name.clone());
        }

        offset += BLOCK_SIZE;

        // --- Skip data blocks ---
        if header.size > 0 {
            let data_blocks = ((header.size as usize) + BLOCK_SIZE - 1) / BLOCK_SIZE;
            offset += data_blocks * BLOCK_SIZE;
        }
    }

    Ok(entries)
}

/// Format permissions for tar -tv output.
fn format_tar_mode(mode: u32, typeflag: u8) -> String {
    let type_char = match typeflag {
        b'5' => 'd',
        b'2' => 'l',
        _ => '-',
    };

    let mut s = String::with_capacity(10);
    s.push(type_char);

    for shift in [6, 3, 0] {
        let bits = (mode >> shift) & 7;
        s.push(if bits & 4 != 0 { 'r' } else { '-' });
        s.push(if bits & 2 != 0 { 'w' } else { '-' });
        s.push(if bits & 1 != 0 { 'x' } else { '-' });
    }

    s
}

// ---------------------------------------------------------------------------
// Extract Archive
// ---------------------------------------------------------------------------

/// Extract files from a tar archive.
///
/// ## Algorithm
///
/// ```text
///     For each header in archive:
///         1. Read the header block
///         2. If typeflag is '5' (directory): create the directory
///         3. If typeflag is '0' or '\0' (file):
///             a. Read `size` bytes of content
///             b. Create parent directories
///             c. Write the file
///             d. Set permissions (Unix only)
///         4. Skip to next block boundary
/// ```
pub fn extract_archive(
    archive_data: &[u8],
    dest_dir: &str,
    opts: &TarOptions,
) -> Result<Vec<String>, String> {
    let mut messages = Vec::new();
    let mut offset = 0;
    let dest = Path::new(dest_dir);

    while offset + BLOCK_SIZE <= archive_data.len() {
        let block: [u8; BLOCK_SIZE] = archive_data[offset..offset + BLOCK_SIZE]
            .try_into()
            .map_err(|_| "tar: unexpected end of archive".to_string())?;

        let header = match TarHeader::from_bytes(&block) {
            Some(h) => h,
            None => break,
        };

        offset += BLOCK_SIZE;

        // --- Security: prevent path traversal ---
        // Reject paths that try to escape the destination directory
        // with ".." components.
        if header.name.contains("..") {
            return Err(format!(
                "tar: refusing to extract '{}': path contains '..'",
                header.name
            ));
        }

        let target = dest.join(&header.name);

        match header.typeflag {
            b'5' => {
                // --- Directory ---
                fs::create_dir_all(&target)
                    .map_err(|e| format!("tar: {}: {}", header.name, e))?;

                #[cfg(unix)]
                {
                    let perms = fs::Permissions::from_mode(header.mode);
                    let _ = fs::set_permissions(&target, perms);
                }

                if opts.verbose {
                    messages.push(header.name.clone());
                }
            }
            b'0' | 0 => {
                // --- Regular file ---
                let size = header.size as usize;

                if offset + size > archive_data.len() {
                    return Err(format!(
                        "tar: unexpected end of archive for '{}'",
                        header.name
                    ));
                }

                let content = &archive_data[offset..offset + size];

                // Create parent directories
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| format!("tar: {}: {}", header.name, e))?;
                }

                // Write the file
                fs::write(&target, content)
                    .map_err(|e| format!("tar: {}: {}", header.name, e))?;

                #[cfg(unix)]
                {
                    let perms = fs::Permissions::from_mode(header.mode);
                    let _ = fs::set_permissions(&target, perms);
                }

                if opts.verbose {
                    messages.push(header.name.clone());
                }

                // Skip to next block boundary
                let data_blocks = (size + BLOCK_SIZE - 1) / BLOCK_SIZE;
                offset += data_blocks * BLOCK_SIZE;
            }
            _ => {
                // Skip unknown types
                let data_blocks = ((header.size as usize) + BLOCK_SIZE - 1) / BLOCK_SIZE;
                offset += data_blocks * BLOCK_SIZE;
            }
        }
    }

    Ok(messages)
}

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Header encoding/decoding ---

    #[test]
    fn header_roundtrip() {
        let header = TarHeader {
            name: "test.txt".to_string(),
            mode: 0o644,
            uid: 1000,
            gid: 1000,
            size: 42,
            mtime: 1700000000,
            typeflag: b'0',
            uname: "user".to_string(),
            gname: "group".to_string(),
        };

        let bytes = header.to_bytes();
        let parsed = TarHeader::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.name, "test.txt");
        assert_eq!(parsed.mode, 0o644);
        assert_eq!(parsed.uid, 1000);
        assert_eq!(parsed.gid, 1000);
        assert_eq!(parsed.size, 42);
        assert_eq!(parsed.typeflag, b'0');
        assert_eq!(parsed.uname, "user");
        assert_eq!(parsed.gname, "group");
    }

    #[test]
    fn zero_block_is_end_of_archive() {
        let block = [0u8; BLOCK_SIZE];
        assert!(TarHeader::from_bytes(&block).is_none());
    }

    #[test]
    fn directory_header_has_typeflag_5() {
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
        assert_eq!(parsed.name, "mydir/");
    }

    // --- Binary encoding helpers ---

    #[test]
    fn write_and_read_octal() {
        let mut field = [0u8; 8];
        write_octal(&mut field, 0o755, 7);
        let value = read_octal(&field);
        assert_eq!(value, 0o755);
    }

    #[test]
    fn write_and_read_string() {
        let mut field = [0u8; 100];
        write_string(&mut field, "hello.txt");
        let value = read_string(&field);
        assert_eq!(value, "hello.txt");
    }

    #[test]
    fn write_string_truncates() {
        let mut field = [0u8; 5];
        write_string(&mut field, "longername");
        let value = read_string(&field);
        assert_eq!(value, "long"); // 4 chars + NUL
    }

    // --- Create + List roundtrip ---

    #[test]
    fn create_and_list_archive() {
        let dir = std::env::temp_dir().join("tar_test_create_list");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("file1.txt"), "hello").unwrap();
        fs::write(dir.join("file2.txt"), "world").unwrap();

        let opts = TarOptions {
            directory: Some(dir.to_string_lossy().into_owned()),
            verbose: false,
        };

        let files = vec!["file1.txt".to_string(), "file2.txt".to_string()];
        let (archive, _) = create_archive(&files, &opts).unwrap();

        let entries = list_archive(&archive, &TarOptions::default()).unwrap();
        assert!(entries.contains(&"file1.txt".to_string()));
        assert!(entries.contains(&"file2.txt".to_string()));

        let _ = fs::remove_dir_all(&dir);
    }

    // --- Create + Extract roundtrip ---

    #[test]
    fn create_and_extract_archive() {
        let src_dir = std::env::temp_dir().join("tar_test_create_src");
        let dst_dir = std::env::temp_dir().join("tar_test_create_dst");
        let _ = fs::remove_dir_all(&src_dir);
        let _ = fs::remove_dir_all(&dst_dir);
        fs::create_dir_all(&src_dir).unwrap();

        fs::write(src_dir.join("test.txt"), "content here").unwrap();

        let opts = TarOptions {
            directory: Some(src_dir.to_string_lossy().into_owned()),
            verbose: false,
        };

        let files = vec!["test.txt".to_string()];
        let (archive, _) = create_archive(&files, &opts).unwrap();

        fs::create_dir_all(&dst_dir).unwrap();
        let _ = extract_archive(&archive, &dst_dir.to_string_lossy(), &TarOptions::default());

        let extracted = fs::read_to_string(dst_dir.join("test.txt")).unwrap();
        assert_eq!(extracted, "content here");

        let _ = fs::remove_dir_all(&src_dir);
        let _ = fs::remove_dir_all(&dst_dir);
    }

    // --- Create directory archive ---

    #[test]
    fn create_archive_with_directory() {
        let dir = std::env::temp_dir().join("tar_test_dir_archive");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("subdir")).unwrap();
        fs::write(dir.join("subdir/inner.txt"), "inner content").unwrap();

        let opts = TarOptions {
            directory: Some(dir.to_string_lossy().into_owned()),
            verbose: true,
        };

        let files = vec!["subdir".to_string()];
        let (archive, messages) = create_archive(&files, &opts).unwrap();

        assert!(messages.iter().any(|m| m.contains("subdir")));

        let entries = list_archive(&archive, &TarOptions::default()).unwrap();
        assert!(entries.iter().any(|e| e.contains("subdir")));

        let _ = fs::remove_dir_all(&dir);
    }

    // --- Verbose list ---

    #[test]
    fn list_verbose_shows_permissions() {
        let header = TarHeader {
            name: "test.txt".to_string(),
            mode: 0o644,
            uid: 0,
            gid: 0,
            size: 100,
            mtime: 0,
            typeflag: b'0',
            uname: String::new(),
            gname: String::new(),
        };

        let mut archive = Vec::new();
        archive.extend_from_slice(&header.to_bytes());
        // Add data blocks
        let data = vec![0u8; BLOCK_SIZE]; // one block of zeros
        archive.extend_from_slice(&data);
        archive.extend_from_slice(&[0u8; BLOCK_SIZE * 2]); // end marker

        let opts = TarOptions { verbose: true, ..Default::default() };
        let entries = list_archive(&archive, &opts).unwrap();
        assert!(entries[0].contains("rw-r--r--"));
    }

    // --- Path traversal prevention ---

    #[test]
    fn extract_rejects_path_traversal() {
        let header = TarHeader {
            name: "../etc/passwd".to_string(),
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
        archive.extend_from_slice(&[0u8; BLOCK_SIZE * 2]);

        let result = extract_archive(&archive, "/tmp/tar_test_traversal", &TarOptions::default());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains(".."));
    }

    // --- format_tar_mode ---

    #[test]
    fn format_mode_regular_file() {
        assert_eq!(format_tar_mode(0o755, b'0'), "-rwxr-xr-x");
    }

    #[test]
    fn format_mode_directory() {
        assert_eq!(format_tar_mode(0o755, b'5'), "drwxr-xr-x");
    }

    // --- Empty archive ---

    #[test]
    fn list_empty_archive() {
        let archive = vec![0u8; BLOCK_SIZE * 2];
        let entries = list_archive(&archive, &TarOptions::default()).unwrap();
        assert!(entries.is_empty());
    }

    // --- Nonexistent file ---

    #[test]
    fn create_archive_nonexistent_file() {
        let opts = TarOptions::default();
        let files = vec!["definitely_not_a_real_file_xyz.txt".to_string()];
        let result = create_archive(&files, &opts);
        assert!(result.is_err());
    }

    // --- read_octal edge cases ---

    #[test]
    fn read_octal_zero() {
        let field = b"0000000\0";
        assert_eq!(read_octal(field), 0);
    }

    #[test]
    fn read_octal_max() {
        let field = b"77777777777\0";
        assert_eq!(read_octal(field), 0o77777777777);
    }
}
