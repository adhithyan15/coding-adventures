//! Test-only durable-step instrumentation for the local composition root.
//!
//! # What this module is
//!
//! VLT-PM41 drills the local CLI by killing a *real* process at a chosen
//! durable write and then asking the *next* real process to recover. To make
//! "a chosen durable write" a well-defined thing, the composition root has to
//! name every point at which it makes something durable. This module is that
//! naming, and nothing else.
//!
//! # Why it costs the shipped binary nothing
//!
//! The module has two bodies selected by the optional `crash-injection`
//! feature:
//!
//! - **feature off** — the default, and the only configuration a released
//!   `vault-pm` is ever built in. `LocalBackend` is exactly
//!   `FsStorageBackend`, and each `around_*` combinator is an `#[inline]`
//!   function whose whole body is `action()`. No counter, no environment
//!   variable, no dependency: the crash-injection package is an *optional*
//!   dependency, so it is not even compiled.
//! - **feature on** — enabled only by this executable's dev-dependencies, so
//!   it exists in `cargo test` builds and nowhere else. `LocalBackend` gains a
//!   decorator that brackets every backend write, and the combinators bracket
//!   the two durable writes that do *not* go through the backend: the client
//!   configuration file and the portable-export artifact.
//!
//! Keeping the seam here rather than inside `vault-pm-application` matters:
//! the application layer is deliberately storage-agnostic and owns no
//! filesystem authority, so it is not the layer that knows what "durable"
//! means. The composition root is.

#[cfg(not(feature = "crash-injection"))]
mod imp {
    use coding_adventures_storage_fs::FsStorageBackend;
    use std::path::Path;

    /// The durable-write backend this host composes over its private roots.
    pub type LocalBackend = FsStorageBackend;

    /// Build the backend for one already permission-checked root.
    pub fn backend(root: &Path) -> LocalBackend {
        FsStorageBackend::new(root)
    }

    /// Run the first durable creation of the client configuration file.
    #[inline]
    pub fn around_config_create<T>(action: impl FnOnce() -> T) -> T {
        action()
    }

    /// Run one durable compare-and-exchange of the client configuration file.
    #[inline]
    pub fn around_config_replace<T>(action: impl FnOnce() -> T) -> T {
        action()
    }

    /// Run the durable creation of one encrypted portable-export artifact.
    #[inline]
    pub fn around_export_artifact<T>(action: impl FnOnce() -> T) -> T {
        action()
    }
}

#[cfg(feature = "crash-injection")]
mod imp {
    use coding_adventures_storage_fs::FsStorageBackend;
    use coding_adventures_vault_pm_crash_injection::{
        around, CrashInjectingStorageBackend, DurableStep,
    };
    use std::path::Path;

    /// The durable-write backend this host composes over its private roots.
    pub type LocalBackend = CrashInjectingStorageBackend<FsStorageBackend>;

    /// Build the instrumented backend for one permission-checked root.
    pub fn backend(root: &Path) -> LocalBackend {
        CrashInjectingStorageBackend::new(FsStorageBackend::new(root))
    }

    /// Run the first durable creation of the client configuration file.
    pub fn around_config_create<T>(action: impl FnOnce() -> T) -> T {
        around(DurableStep::ConfigCreate, action)
    }

    /// Run one durable compare-and-exchange of the client configuration file.
    pub fn around_config_replace<T>(action: impl FnOnce() -> T) -> T {
        around(DurableStep::ConfigReplace, action)
    }

    /// Run the durable creation of one encrypted portable-export artifact.
    pub fn around_export_artifact<T>(action: impl FnOnce() -> T) -> T {
        around(DurableStep::ExportArtifact, action)
    }
}

pub(crate) use imp::{
    around_config_create, around_config_replace, around_export_artifact, backend, LocalBackend,
};
