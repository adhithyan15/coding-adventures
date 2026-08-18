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
//! - **feature off** — the default, and the only configuration the product
//!   executable `code/programs/rust/vault-pm-cli` is ever built in.
//!   `LocalBackend` is exactly `FsStorageBackend`, and each `around_*`
//!   combinator is an `#[inline]` function whose whole body is `action()`. No
//!   counter, no environment variable, no dependency: the crash-injection
//!   package is an *optional* dependency, so it is not even compiled.
//! - **feature on** — enabled by exactly one crate,
//!   `code/programs/rust/vault-pm-cli-drill`, through its ordinary
//!   `[dependencies]`. `LocalBackend` gains a decorator that brackets every
//!   backend write, and the combinators bracket the two durable writes that do
//!   *not* go through the backend: the client configuration file and the
//!   portable-export artifact.
//!
//! Do not be tempted to reach the feature from the product crate's
//! `dev-dependencies` instead of splitting the crate. Cargo resolves features
//! per package across a build graph, so `cargo build --all-targets` would pull
//! them in and uplift an instrumented binary to `target/release/vault-pm` —
//! the path a packaging step copies from. Declaring no feature is in turn
//! necessary and *not sufficient*, because `--features <dep>/<feature>` reaches
//! a direct dependency's features regardless, which is why the product's
//! `main.rs` asserts on [`crate::CRASH_INJECTION_COMPILED`] as well.
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
