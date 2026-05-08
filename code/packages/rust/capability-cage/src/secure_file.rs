//! Secure-wrapper functions for the `fs` capability category.
//!
//! Every call validates the manifest first, then delegates to the
//! current [`Backend`]. The manifest check produces an
//! [`io::Error::other`]-wrapped [`CapabilityViolationError`] on
//! denial, so callers' existing `io::Result` propagation works.

use std::io;
use std::path::Path;

use crate::backend::current_backend;
use crate::category::{Action, Category};
use crate::errors::CapabilityViolationError;
use crate::manifest::Manifest;

/// Read the entire contents of `path`.
pub fn read_file(manifest: &Manifest, path: &Path) -> io::Result<Vec<u8>> {
    check(manifest, Action::Read, path)?;
    current_backend().read_file(path)
}

/// Write `data` to `path`, replacing any existing contents.
pub fn write_file(manifest: &Manifest, path: &Path, data: &[u8]) -> io::Result<()> {
    check(manifest, Action::Write, path)?;
    current_backend().write_file(path, data)
}

/// Create a new empty file at `path`. Fails if it already exists.
pub fn create_file(manifest: &Manifest, path: &Path) -> io::Result<()> {
    check(manifest, Action::Create, path)?;
    current_backend().create_file(path)
}

/// Remove the file at `path`.
pub fn delete_file(manifest: &Manifest, path: &Path) -> io::Result<()> {
    check(manifest, Action::Delete, path)?;
    current_backend().delete_file(path)
}

/// List the entries in the directory at `path`.
pub fn list_dir(manifest: &Manifest, path: &Path) -> io::Result<Vec<String>> {
    check(manifest, Action::List, path)?;
    current_backend().list_dir(path)
}

fn check(manifest: &Manifest, action: Action, path: &Path) -> io::Result<()> {
    let target = path.to_string_lossy();
    manifest
        .check(Category::Fs, action, target.as_ref())
        .map_err(violation_to_io)
}

fn violation_to_io(err: CapabilityViolationError) -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, err)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::backend::{with_backend, TestBackend, TestBackendCall};
    use crate::capability::Capability;

    fn manifest_with(category: Category, action: Action, target: &str) -> Manifest {
        Manifest::new(vec![
            Capability::new(category, action, target, "test").unwrap(),
        ])
    }

    #[test]
    fn read_file_calls_backend_when_allowed() {
        let backend = Arc::new(TestBackend::new().with_response("./allowed.txt", b"hello"));
        let _guard = with_backend(backend.clone());
        let m = manifest_with(Category::Fs, Action::Read, "./allowed.txt");
        let bytes = read_file(&m, Path::new("./allowed.txt")).unwrap();
        assert_eq!(bytes, b"hello");
    }

    #[test]
    fn read_file_denied_without_manifest_entry() {
        let backend = Arc::new(TestBackend::new());
        let _guard = with_backend(backend.clone());
        let m = Manifest::empty();
        let err = read_file(&m, Path::new("./forbidden.txt")).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
        let underlying = err.into_inner().expect("expected source error");
        let viol = underlying
            .downcast_ref::<CapabilityViolationError>()
            .expect("expected CapabilityViolationError");
        assert_eq!(viol.category, Category::Fs);
        assert_eq!(viol.action, Action::Read);
        assert_eq!(viol.target, "./forbidden.txt");
    }

    #[test]
    fn read_file_does_not_call_backend_when_denied() {
        let backend = Arc::new(TestBackend::new());
        let _guard = with_backend(backend.clone());
        let m = Manifest::empty();
        let _ = read_file(&m, Path::new("./forbidden.txt"));
        // Backend should never have been called.
        assert!(backend.calls().is_empty(), "backend was called on denial");
    }

    #[test]
    fn write_create_delete_round_trip() {
        let backend = Arc::new(TestBackend::new());
        let _guard = with_backend(backend.clone());
        let m = Manifest::new(vec![
            Capability::new(Category::Fs, Action::Write, "./out.txt", "t").unwrap(),
            Capability::new(Category::Fs, Action::Create, "./out.txt", "t").unwrap(),
            Capability::new(Category::Fs, Action::Delete, "./out.txt", "t").unwrap(),
        ]);
        write_file(&m, Path::new("./out.txt"), b"data").unwrap();
        create_file(&m, Path::new("./out.txt")).unwrap();
        delete_file(&m, Path::new("./out.txt")).unwrap();
        let calls = backend.calls();
        assert_eq!(calls.len(), 3);
        assert!(matches!(&calls[0], TestBackendCall::WriteFile(p, d) if p == Path::new("./out.txt") && d == b"data"));
        assert!(matches!(&calls[1], TestBackendCall::CreateFile(p) if p == Path::new("./out.txt")));
        assert!(matches!(&calls[2], TestBackendCall::DeleteFile(p) if p == Path::new("./out.txt")));
    }

    #[test]
    fn list_dir_with_manifest() {
        let backend = Arc::new(
            TestBackend::new().with_dir_response("./tmp", vec!["a".into(), "b".into()]),
        );
        let _guard = with_backend(backend);
        let m = manifest_with(Category::Fs, Action::List, "./tmp");
        let entries = list_dir(&m, Path::new("./tmp")).unwrap();
        assert_eq!(entries, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn glob_target_matches_specific_path() {
        let backend = Arc::new(TestBackend::new().with_response("./logs/2026.txt", b"x"));
        let _guard = with_backend(backend);
        let m = manifest_with(Category::Fs, Action::Read, "./logs/*.txt");
        assert!(read_file(&m, Path::new("./logs/2026.txt")).is_ok());
    }
}
