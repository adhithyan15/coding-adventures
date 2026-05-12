//! Backend trait and the three default implementations.
//!
//! The cage performs the manifest check, then delegates the actual OS
//! call to a [`Backend`] implementation. The default is [`OpenBackend`]
//! (calls stdlib). [`TestBackend`] records calls for tests; the
//! `host-runtime-rust` crate (future) installs its own backend that
//! routes secure calls over the host channel.
//!
//! V1 scope started with file-system methods. Environment access now has the
//! same backend seam. Network, process, time, and stdio methods land in
//! subsequent PRs.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, RwLock};

/// The contract the cage delegates OS work to.
///
/// Implementations are responsible only for performing the operation;
/// the manifest check happens before the call in the secure-wrapper
/// functions.
pub trait Backend: Send + Sync {
    fn read_file(&self, path: &Path) -> io::Result<Vec<u8>>;
    fn write_file(&self, path: &Path, data: &[u8]) -> io::Result<()>;
    fn create_file(&self, path: &Path) -> io::Result<()>;
    fn delete_file(&self, path: &Path) -> io::Result<()>;
    fn list_dir(&self, path: &Path) -> io::Result<Vec<String>>;
    fn read_env(&self, name: &str) -> io::Result<Option<String>>;
    fn write_env(&self, name: &str, value: &str) -> io::Result<()>;
}

/// The default backend. Delegates straight to [`std::fs`].
pub struct OpenBackend;

impl Backend for OpenBackend {
    fn read_file(&self, path: &Path) -> io::Result<Vec<u8>> {
        std::fs::read(path)
    }

    fn write_file(&self, path: &Path, data: &[u8]) -> io::Result<()> {
        std::fs::write(path, data)
    }

    fn create_file(&self, path: &Path) -> io::Result<()> {
        std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)
            .map(|_| ())
    }

    fn delete_file(&self, path: &Path) -> io::Result<()> {
        std::fs::remove_file(path)
    }

    fn list_dir(&self, path: &Path) -> io::Result<Vec<String>> {
        let mut names = Vec::new();
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            if let Some(name) = entry.file_name().to_str() {
                names.push(name.to_string());
            }
        }
        Ok(names)
    }

    fn read_env(&self, name: &str) -> io::Result<Option<String>> {
        match std::env::var(name) {
            Ok(value) => Ok(Some(value)),
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(std::env::VarError::NotUnicode(_)) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("environment variable {name} is not valid Unicode"),
            )),
        }
    }

    fn write_env(&self, name: &str, value: &str) -> io::Result<()> {
        std::env::set_var(name, value);
        Ok(())
    }
}

/// A backend that refuses every call with `PermissionDenied`.
///
/// Useful as a default for tests of pure-computation packages: any
/// accidental OS access fails loud.
pub struct DenyAllBackend;

impl Backend for DenyAllBackend {
    fn read_file(&self, _path: &Path) -> io::Result<Vec<u8>> {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "DenyAllBackend refused fs.read",
        ))
    }

    fn write_file(&self, _path: &Path, _data: &[u8]) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "DenyAllBackend refused fs.write",
        ))
    }

    fn create_file(&self, _path: &Path) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "DenyAllBackend refused fs.create",
        ))
    }

    fn delete_file(&self, _path: &Path) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "DenyAllBackend refused fs.delete",
        ))
    }

    fn list_dir(&self, _path: &Path) -> io::Result<Vec<String>> {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "DenyAllBackend refused fs.list",
        ))
    }

    fn read_env(&self, _name: &str) -> io::Result<Option<String>> {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "DenyAllBackend refused env.read",
        ))
    }

    fn write_env(&self, _name: &str, _value: &str) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "DenyAllBackend refused env.write",
        ))
    }
}

/// A backend that records every call into an internal log. Useful in
/// tests that assert on the sequence of secure operations.
///
/// Reads and lists return scripted responses provided via
/// [`TestBackend::with_response`]; if no response is scripted,
/// reads return empty bytes and lists return an empty vector. Writes
/// / creates / deletes always succeed and are recorded.
pub struct TestBackend {
    log: Mutex<Vec<TestBackendCall>>,
    file_responses: Mutex<Vec<(PathBuf, Vec<u8>)>>,
    dir_responses: Mutex<Vec<(PathBuf, Vec<String>)>>,
    env_responses: Mutex<Vec<(String, String)>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestBackendCall {
    ReadFile(PathBuf),
    WriteFile(PathBuf, Vec<u8>),
    CreateFile(PathBuf),
    DeleteFile(PathBuf),
    ListDir(PathBuf),
    ReadEnv(String),
    WriteEnv(String, String),
}

impl TestBackend {
    pub fn new() -> Self {
        Self {
            log: Mutex::new(Vec::new()),
            file_responses: Mutex::new(Vec::new()),
            dir_responses: Mutex::new(Vec::new()),
            env_responses: Mutex::new(Vec::new()),
        }
    }

    /// Script a response for a given path. The next [`Backend::read_file`]
    /// call against this path returns these bytes.
    pub fn with_response(self, path: impl Into<PathBuf>, data: impl Into<Vec<u8>>) -> Self {
        self.file_responses
            .lock()
            .unwrap()
            .push((path.into(), data.into()));
        self
    }

    /// Script a response for a directory listing.
    pub fn with_dir_response(self, path: impl Into<PathBuf>, entries: Vec<String>) -> Self {
        self.dir_responses
            .lock()
            .unwrap()
            .push((path.into(), entries));
        self
    }

    /// Script a response for an environment variable read.
    pub fn with_env_response(self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_responses
            .lock()
            .unwrap()
            .push((name.into(), value.into()));
        self
    }

    /// Snapshot the call log.
    pub fn calls(&self) -> Vec<TestBackendCall> {
        self.log.lock().unwrap().clone()
    }
}

impl Default for TestBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for TestBackend {
    fn read_file(&self, path: &Path) -> io::Result<Vec<u8>> {
        self.log
            .lock()
            .unwrap()
            .push(TestBackendCall::ReadFile(path.to_path_buf()));
        let data = self
            .file_responses
            .lock()
            .unwrap()
            .iter()
            .find(|(p, _)| p == path)
            .map(|(_, d)| d.clone())
            .unwrap_or_default();
        Ok(data)
    }

    fn write_file(&self, path: &Path, data: &[u8]) -> io::Result<()> {
        self.log.lock().unwrap().push(TestBackendCall::WriteFile(
            path.to_path_buf(),
            data.to_vec(),
        ));
        Ok(())
    }

    fn create_file(&self, path: &Path) -> io::Result<()> {
        self.log
            .lock()
            .unwrap()
            .push(TestBackendCall::CreateFile(path.to_path_buf()));
        Ok(())
    }

    fn delete_file(&self, path: &Path) -> io::Result<()> {
        self.log
            .lock()
            .unwrap()
            .push(TestBackendCall::DeleteFile(path.to_path_buf()));
        Ok(())
    }

    fn list_dir(&self, path: &Path) -> io::Result<Vec<String>> {
        self.log
            .lock()
            .unwrap()
            .push(TestBackendCall::ListDir(path.to_path_buf()));
        let entries = self
            .dir_responses
            .lock()
            .unwrap()
            .iter()
            .find(|(p, _)| p == path)
            .map(|(_, e)| e.clone())
            .unwrap_or_default();
        Ok(entries)
    }

    fn read_env(&self, name: &str) -> io::Result<Option<String>> {
        self.log
            .lock()
            .unwrap()
            .push(TestBackendCall::ReadEnv(name.to_string()));
        let value = self
            .env_responses
            .lock()
            .unwrap()
            .iter()
            .find(|(candidate, _)| candidate == name)
            .map(|(_, value)| value.clone());
        Ok(value)
    }

    fn write_env(&self, name: &str, value: &str) -> io::Result<()> {
        self.log.lock().unwrap().push(TestBackendCall::WriteEnv(
            name.to_string(),
            value.to_string(),
        ));
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Process-wide default backend slot.
// ---------------------------------------------------------------------------

static DEFAULT_BACKEND: once_cell_shim::Lazy<RwLock<Arc<dyn Backend>>> =
    once_cell_shim::Lazy::new(|| RwLock::new(Arc::new(OpenBackend)));
static BACKEND_OVERRIDE_LOCK: once_cell_shim::Lazy<Mutex<()>> =
    once_cell_shim::Lazy::new(|| Mutex::new(()));

mod once_cell_shim {
    //! Tiny replacement for `once_cell::sync::Lazy` so we don't take an
    //! external dep just for this. Single-write semantics; first
    //! `force` wins.
    use std::sync::OnceLock;

    pub struct Lazy<T> {
        cell: OnceLock<T>,
        init: fn() -> T,
    }

    impl<T> Lazy<T> {
        pub const fn new(init: fn() -> T) -> Self {
            Self {
                cell: OnceLock::new(),
                init,
            }
        }

        fn force(&self) -> &T {
            self.cell.get_or_init(self.init)
        }
    }

    impl<T> std::ops::Deref for Lazy<T> {
        type Target = T;
        fn deref(&self) -> &T {
            self.force()
        }
    }
}

/// Returned by [`with_backend`]. Restores the previous backend on drop.
pub struct BackendGuard {
    previous: Arc<dyn Backend>,
    _override_guard: MutexGuard<'static, ()>,
}

impl Drop for BackendGuard {
    fn drop(&mut self) {
        if let Ok(mut slot) = DEFAULT_BACKEND.write() {
            *slot = Arc::clone(&self.previous);
        }
    }
}

/// Replace the process-wide default backend until the returned guard
/// is dropped.
///
/// Only one override can be active at a time. That keeps parallel Rust
/// tests from restoring each other's process-wide backend mid-call.
pub fn with_backend(backend: Arc<dyn Backend>) -> BackendGuard {
    let override_guard = BACKEND_OVERRIDE_LOCK
        .lock()
        .expect("backend override lock poisoned");
    let mut slot = DEFAULT_BACKEND
        .write()
        .expect("default backend lock poisoned");
    let previous = std::mem::replace(&mut *slot, backend);
    BackendGuard {
        previous,
        _override_guard: override_guard,
    }
}

/// Internal accessor for secure-wrapper modules.
pub(crate) fn current_backend() -> Arc<dyn Backend> {
    Arc::clone(
        &DEFAULT_BACKEND
            .read()
            .expect("default backend lock poisoned"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deny_all_refuses_everything() {
        let b = DenyAllBackend;
        assert_eq!(
            b.read_file(Path::new("./x")).unwrap_err().kind(),
            io::ErrorKind::PermissionDenied
        );
        assert_eq!(
            b.write_file(Path::new("./x"), b"y").unwrap_err().kind(),
            io::ErrorKind::PermissionDenied
        );
        assert_eq!(
            b.create_file(Path::new("./x")).unwrap_err().kind(),
            io::ErrorKind::PermissionDenied
        );
        assert_eq!(
            b.delete_file(Path::new("./x")).unwrap_err().kind(),
            io::ErrorKind::PermissionDenied
        );
        assert_eq!(
            b.list_dir(Path::new("./x")).unwrap_err().kind(),
            io::ErrorKind::PermissionDenied
        );
        assert_eq!(
            b.read_env("TOKEN").unwrap_err().kind(),
            io::ErrorKind::PermissionDenied
        );
        assert_eq!(
            b.write_env("TOKEN", "secret").unwrap_err().kind(),
            io::ErrorKind::PermissionDenied
        );
    }

    #[test]
    fn test_backend_records_and_returns_scripted_response() {
        let b = TestBackend::new().with_response("./foo", b"hello");
        let bytes = b.read_file(Path::new("./foo")).unwrap();
        assert_eq!(bytes, b"hello");
        let calls = b.calls();
        assert_eq!(calls.len(), 1);
        assert!(matches!(&calls[0], TestBackendCall::ReadFile(p) if p == Path::new("./foo")));
    }

    #[test]
    fn test_backend_unscripted_read_returns_empty() {
        let b = TestBackend::new();
        let bytes = b.read_file(Path::new("./missing")).unwrap();
        assert!(bytes.is_empty());
    }

    #[test]
    fn test_backend_records_writes() {
        let b = TestBackend::new();
        b.write_file(Path::new("./x"), b"data").unwrap();
        b.create_file(Path::new("./y")).unwrap();
        b.delete_file(Path::new("./z")).unwrap();
        let calls = b.calls();
        assert_eq!(calls.len(), 3);
        assert!(
            matches!(&calls[0], TestBackendCall::WriteFile(p, d) if p == Path::new("./x") && d == b"data")
        );
        assert!(matches!(&calls[1], TestBackendCall::CreateFile(p) if p == Path::new("./y")));
        assert!(matches!(&calls[2], TestBackendCall::DeleteFile(p) if p == Path::new("./z")));
    }

    #[test]
    fn test_backend_dir_listing() {
        let b = TestBackend::new().with_dir_response("./dir", vec!["a.txt".into(), "b.txt".into()]);
        let entries = b.list_dir(Path::new("./dir")).unwrap();
        assert_eq!(entries, vec!["a.txt".to_string(), "b.txt".to_string()]);
    }

    #[test]
    fn test_backend_records_env_access() {
        let b = TestBackend::new().with_env_response("TOKEN", "secret");
        assert_eq!(b.read_env("TOKEN").unwrap(), Some("secret".to_string()));
        b.write_env("MODE", "test").unwrap();

        let calls = b.calls();
        assert_eq!(calls.len(), 2);
        assert!(matches!(&calls[0], TestBackendCall::ReadEnv(name) if name == "TOKEN"));
        assert!(
            matches!(&calls[1], TestBackendCall::WriteEnv(name, value) if name == "MODE" && value == "test")
        );
    }
}
