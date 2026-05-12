//! Secure-wrapper functions for the `env` capability category.
//!
//! These wrappers mirror [`crate::secure_file`]: every call checks the
//! manifest first, then delegates to the current backend. Environment variable
//! names are the manifest targets, so callers grant individual variables or
//! globbed variable families.

use std::io;

use crate::backend::current_backend;
use crate::category::{Action, Category};
use crate::errors::CapabilityViolationError;
use crate::manifest::Manifest;

/// Read one environment variable as Unicode.
///
/// Missing variables return `Ok(None)`. Non-Unicode variables are reported by
/// the backend as [`io::ErrorKind::InvalidData`].
pub fn read_var(manifest: &Manifest, name: &str) -> io::Result<Option<String>> {
    check(manifest, Action::Read, name)?;
    current_backend().read_env(name)
}

/// Set one environment variable.
pub fn write_var(manifest: &Manifest, name: &str, value: &str) -> io::Result<()> {
    check(manifest, Action::Write, name)?;
    current_backend().write_env(name, value)
}

fn check(manifest: &Manifest, action: Action, name: &str) -> io::Result<()> {
    manifest
        .check(Category::Env, action, name)
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

    fn manifest_with(action: Action, target: &str) -> Manifest {
        Manifest::new(vec![
            Capability::new(Category::Env, action, target, "test").unwrap()
        ])
    }

    #[test]
    fn read_var_calls_backend_when_allowed() {
        let backend = Arc::new(TestBackend::new().with_env_response("TOKEN", "secret"));
        let _guard = with_backend(backend.clone());
        let manifest = manifest_with(Action::Read, "TOKEN");

        let value = read_var(&manifest, "TOKEN").unwrap();

        assert_eq!(value, Some("secret".to_string()));
        assert!(matches!(&backend.calls()[0], TestBackendCall::ReadEnv(name) if name == "TOKEN"));
    }

    #[test]
    fn read_var_denied_without_manifest_entry() {
        let backend = Arc::new(TestBackend::new().with_env_response("TOKEN", "secret"));
        let _guard = with_backend(backend.clone());

        let err = read_var(&Manifest::empty(), "TOKEN").unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
        let underlying = err.into_inner().expect("expected source error");
        let violation = underlying
            .downcast_ref::<CapabilityViolationError>()
            .expect("expected CapabilityViolationError");
        assert_eq!(violation.category, Category::Env);
        assert_eq!(violation.action, Action::Read);
        assert_eq!(violation.target, "TOKEN");
        assert!(backend.calls().is_empty(), "backend was called on denial");
    }

    #[test]
    fn write_var_calls_backend_when_allowed() {
        let backend = Arc::new(TestBackend::new());
        let _guard = with_backend(backend.clone());
        let manifest = manifest_with(Action::Write, "MODE");

        write_var(&manifest, "MODE", "test").unwrap();

        assert!(
            matches!(&backend.calls()[0], TestBackendCall::WriteEnv(name, value) if name == "MODE" && value == "test")
        );
    }

    #[test]
    fn glob_target_matches_variable_family() {
        let backend = Arc::new(TestBackend::new().with_env_response("APP_TOKEN", "secret"));
        let _guard = with_backend(backend);
        let manifest = manifest_with(Action::Read, "APP_*");

        assert_eq!(
            read_var(&manifest, "APP_TOKEN").unwrap(),
            Some("secret".to_string())
        );
    }
}
