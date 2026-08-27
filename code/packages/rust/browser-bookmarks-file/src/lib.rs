//! Versioned, crash-safe file persistence for browser bookmarks.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::{env, fmt};

use browser_bookmarks::{Bookmark, BookmarkCatalog, BookmarkRepository, BookmarkRepositoryError};
use serde::{Deserialize, Serialize};

pub const VERSION: &str = "0.1.0";
pub const SCHEMA_VERSION: u32 = 1;
pub const MAX_BOOKMARKS: usize = 10_000;
pub const MAX_DOCUMENT_BYTES: u64 = 8 * 1024 * 1024;
pub const PATH_OVERRIDE_ENV: &str = "VENTURE_BOOKMARKS_PATH";

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct BookmarkDocument {
    schema_version: u32,
    bookmarks: Vec<BookmarkRecord>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct BookmarkRecord {
    url: String,
    title: String,
}

/// JSON repository backed by one native profile file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileBookmarkRepository {
    path: PathBuf,
}

impl FileBookmarkRepository {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn read_document(&self) -> Result<Option<Vec<u8>>, BookmarkRepositoryError> {
        match fs::symlink_metadata(&self.path) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
                return Err(corrupt("bookmark path is not a regular file"));
            }
            Ok(metadata) if metadata.len() > MAX_DOCUMENT_BYTES => {
                return Err(corrupt("bookmark document exceeds the size limit"));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(unavailable(error)),
        }

        let mut file = File::open(&self.path).map_err(unavailable)?;
        let mut bytes = Vec::new();
        Read::by_ref(&mut file)
            .take(MAX_DOCUMENT_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(unavailable)?;
        if bytes.len() as u64 > MAX_DOCUMENT_BYTES {
            return Err(corrupt("bookmark document exceeds the size limit"));
        }
        Ok(Some(bytes))
    }

    fn decode(bytes: &[u8]) -> Result<BookmarkCatalog, BookmarkRepositoryError> {
        let document: BookmarkDocument =
            serde_json::from_slice(bytes).map_err(|error| corrupt(error.to_string()))?;
        if document.schema_version != SCHEMA_VERSION {
            return Err(BookmarkRepositoryError::UnsupportedSchema(
                document.schema_version,
            ));
        }
        if document.bookmarks.len() > MAX_BOOKMARKS {
            return Err(corrupt("bookmark count exceeds the limit"));
        }
        let entries = document
            .bookmarks
            .into_iter()
            .map(|record| {
                Bookmark::new(&record.url, record.title)
                    .map_err(|error| corrupt(format!("invalid bookmark URL: {error}")))
            })
            .collect::<Result<Vec<_>, _>>()?;
        BookmarkCatalog::from_entries(entries).map_err(|error| corrupt(error.to_string()))
    }

    fn encode(catalog: &BookmarkCatalog) -> Result<Vec<u8>, BookmarkRepositoryError> {
        if catalog.len() > MAX_BOOKMARKS {
            return Err(corrupt("bookmark count exceeds the limit"));
        }
        let document = BookmarkDocument {
            schema_version: SCHEMA_VERSION,
            bookmarks: catalog
                .entries()
                .iter()
                .map(|bookmark| BookmarkRecord {
                    url: bookmark.url().as_str().to_string(),
                    title: bookmark.title().to_string(),
                })
                .collect(),
        };
        let mut bytes =
            serde_json::to_vec_pretty(&document).map_err(|error| corrupt(error.to_string()))?;
        bytes.push(b'\n');
        if bytes.len() as u64 > MAX_DOCUMENT_BYTES {
            return Err(corrupt("bookmark document exceeds the size limit"));
        }
        Ok(bytes)
    }

    fn atomic_save(&self, bytes: &[u8]) -> Result<(), BookmarkRepositoryError> {
        let parent = self
            .path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .ok_or_else(|| corrupt("bookmark path has no parent directory"))?;
        fs::create_dir_all(parent).map_err(unavailable)?;
        let file_name = self
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| corrupt("bookmark path has no valid file name"))?;

        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temporary_path = parent.join(format!(
            ".{file_name}.tmp-{}-{sequence}",
            std::process::id()
        ));
        let result = (|| {
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut temporary = options.open(&temporary_path).map_err(unavailable)?;
            temporary.write_all(bytes).map_err(unavailable)?;
            temporary.sync_all().map_err(unavailable)?;
            drop(temporary);
            replace_file(&temporary_path, &self.path).map_err(unavailable)?;
            sync_directory(parent).map_err(unavailable)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary_path);
        }
        result
    }
}

impl BookmarkRepository for FileBookmarkRepository {
    fn load(&mut self) -> Result<BookmarkCatalog, BookmarkRepositoryError> {
        self.read_document()?
            .map_or_else(|| Ok(BookmarkCatalog::new()), |bytes| Self::decode(&bytes))
    }

    fn save(&mut self, catalog: &BookmarkCatalog) -> Result<(), BookmarkRepositoryError> {
        self.atomic_save(&Self::encode(catalog)?)
    }
}

/// Resolve Venture's bookmark file using the current native platform profile.
pub fn default_bookmark_path() -> Result<PathBuf, BookmarkRepositoryError> {
    if let Some(path) = env::var_os(PATH_OVERRIDE_ENV).filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(path));
    }

    #[cfg(target_os = "windows")]
    {
        let root = env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .ok_or_else(|| unavailable_message("LOCALAPPDATA is not set"))?;
        return Ok(root.join("Venture").join("bookmarks.json"));
    }

    #[cfg(target_os = "macos")]
    {
        let home = home_directory()?;
        return Ok(home
            .join("Library")
            .join("Application Support")
            .join("Venture")
            .join("bookmarks.json"));
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let root = env::var_os("XDG_DATA_HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .unwrap_or(home_directory()?.join(".local").join("share"));
        return Ok(root.join("venture").join("bookmarks.json"));
    }

    #[allow(unreachable_code)]
    Err(unavailable_message(
        "no profile path policy for this platform",
    ))
}

#[cfg(unix)]
fn home_directory() -> Result<PathBuf, BookmarkRepositoryError> {
    env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| unavailable_message("HOME is not set"))
}

#[cfg(not(unix))]
fn home_directory() -> Result<PathBuf, BookmarkRepositoryError> {
    Err(unavailable_message("HOME is unavailable"))
}

#[cfg(unix)]
fn replace_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::rename(source, destination)
}

#[cfg(windows)]
fn replace_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_REPLACE_EXISTING};

    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let replaced = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING,
        )
    };
    if replaced == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(any(unix, windows)))]
fn replace_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::rename(source, destination)
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> std::io::Result<()> {
    File::open(path)?.sync_all()
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

fn unavailable(error: impl fmt::Display) -> BookmarkRepositoryError {
    unavailable_message(error.to_string())
}

fn unavailable_message(message: impl Into<String>) -> BookmarkRepositoryError {
    BookmarkRepositoryError::Unavailable(message.into())
}

fn corrupt(message: impl Into<String>) -> BookmarkRepositoryError {
    BookmarkRepositoryError::Corrupt(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = env::temp_dir().join(format!(
                "browser-bookmarks-{label}-{}-{nonce}",
                std::process::id()
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn file(&self) -> PathBuf {
            self.0.join("profile").join("bookmarks.json")
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn missing_file_loads_empty_and_round_trips_in_order() {
        let directory = TestDirectory::new("round-trip");
        let mut repository = FileBookmarkRepository::new(directory.file());
        assert!(repository.load().unwrap().is_empty());

        let mut catalog = BookmarkCatalog::new();
        catalog
            .upsert("HTTP://EXAMPLE.TEST:80/one#intro", "One")
            .unwrap();
        catalog.upsert("http://example.test/two", "Two").unwrap();
        repository.save(&catalog).unwrap();
        assert_eq!(repository.load().unwrap(), catalog);

        let bytes = fs::read(repository.path()).unwrap();
        assert!(bytes.ends_with(b"\n"));
        assert!(String::from_utf8(bytes)
            .unwrap()
            .contains("http://example.test/one#intro"));
    }

    #[test]
    fn unsupported_schema_and_duplicate_canonical_urls_are_rejected() {
        let directory = TestDirectory::new("validation");
        let path = directory.file();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, r#"{"schema_version":2,"bookmarks":[]}"#).unwrap();
        let mut repository = FileBookmarkRepository::new(&path);
        assert_eq!(
            repository.load().unwrap_err(),
            BookmarkRepositoryError::UnsupportedSchema(2)
        );

        fs::write(
            &path,
            r#"{"schema_version":1,"bookmarks":[{"url":"HTTP://EXAMPLE.TEST:80/","title":"A"},{"url":"http://example.test/","title":"B"}]}"#,
        )
        .unwrap();
        assert!(matches!(
            repository.load(),
            Err(BookmarkRepositoryError::Corrupt(_))
        ));
    }

    #[test]
    fn malformed_or_oversized_documents_are_rejected_without_rewrite() {
        let directory = TestDirectory::new("corrupt");
        let path = directory.file();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"not-json").unwrap();
        let original = fs::read(&path).unwrap();
        let mut repository = FileBookmarkRepository::new(&path);
        assert!(repository.load().is_err());
        assert_eq!(fs::read(&path).unwrap(), original);

        let file = File::create(&path).unwrap();
        file.set_len(MAX_DOCUMENT_BYTES + 1).unwrap();
        assert!(repository.load().is_err());
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_are_rejected_and_saved_files_are_owner_only() {
        use std::os::unix::fs::{symlink, PermissionsExt};

        let directory = TestDirectory::new("permissions");
        let target = directory.0.join("target.json");
        fs::write(&target, r#"{"schema_version":1,"bookmarks":[]}"#).unwrap();
        let link = directory.0.join("link.json");
        symlink(&target, &link).unwrap();
        assert!(FileBookmarkRepository::new(link).load().is_err());

        let path = directory.file();
        let mut repository = FileBookmarkRepository::new(&path);
        repository.save(&BookmarkCatalog::new()).unwrap();
        assert_eq!(
            fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[test]
    fn rewrite_removes_temporary_files() {
        let directory = TestDirectory::new("temporary");
        let path = directory.file();
        let mut repository = FileBookmarkRepository::new(&path);
        repository.save(&BookmarkCatalog::new()).unwrap();
        repository.save(&BookmarkCatalog::new()).unwrap();
        let names = fs::read_dir(path.parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect::<Vec<_>>();
        assert_eq!(names, [path.file_name().unwrap()]);
    }
}
