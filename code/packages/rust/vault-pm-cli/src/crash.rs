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
//!   *not* go through the backend: the client configuration file, the
//!   portable-export artifact, and the exported attachment file.
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
//!
//! # The KDF policy override
//!
//! Same feature split, one more seam: [`kdf_policy_override`]. VLT-PM41 §8.1
//! records why it exists — in short, `vault-pm-cli-drill`'s crash/fault
//! matrix drives *production* Argon2id derivations through real killed
//! processes, and the landing-point count and clean/resumable classification
//! a cell lands in are both pure functions of how many durable writes a
//! ceremony performs, never of how expensive the KDF was to derive. Reading
//! this only when `crash-injection` is compiled in means the override is
//! read-only-by-construction dead code in the shipped `vault-pm` — the same
//! guarantee [`around_config_create`] and friends already have.

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

    /// Run the durable creation of one exported attachment file.
    #[inline]
    pub fn around_attachment_artifact<T>(action: impl FnOnce() -> T) -> T {
        action()
    }

    /// The product build never reads an override: no counter, no
    /// environment variable, no dependency, exactly like the `around_*`
    /// combinators above.
    #[inline]
    pub fn kdf_policy_override() -> Option<(u32, u32, u8)> {
        None
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

    /// Run the durable creation of one exported attachment file.
    pub fn around_attachment_artifact<T>(action: impl FnOnce() -> T) -> T {
        around(DurableStep::AttachmentArtifact, action)
    }

    /// Read `VAULT_PM_DRILL_KDF_{MEMORY_KIB,ITERATIONS,LANES}`, all three or
    /// none — a partial override would silently fall back to a mix of the
    /// caller's chosen cost and the production default, which is a strength
    /// nobody asked for and would defeat the whole point of documenting the
    /// tradeoff explicitly. Malformed values panic rather than silently
    /// falling back, on the same principle `VAULT_PM_CRASH_AT` parsing uses:
    /// this variable exists for one caller (the drill's own test harness),
    /// so a bad value is this crate's bug, not a hostile environment to
    /// tolerate.
    pub fn kdf_policy_override() -> Option<(u32, u32, u8)> {
        let memory_kib = std::env::var("VAULT_PM_DRILL_KDF_MEMORY_KIB").ok()?;
        let iterations = std::env::var("VAULT_PM_DRILL_KDF_ITERATIONS")
            .expect("VAULT_PM_DRILL_KDF_MEMORY_KIB set without VAULT_PM_DRILL_KDF_ITERATIONS");
        let lanes = std::env::var("VAULT_PM_DRILL_KDF_LANES")
            .expect("VAULT_PM_DRILL_KDF_MEMORY_KIB set without VAULT_PM_DRILL_KDF_LANES");
        Some((
            memory_kib
                .parse()
                .expect("VAULT_PM_DRILL_KDF_MEMORY_KIB must be a decimal u32"),
            iterations
                .parse()
                .expect("VAULT_PM_DRILL_KDF_ITERATIONS must be a decimal u32"),
            lanes
                .parse()
                .expect("VAULT_PM_DRILL_KDF_LANES must be a decimal u8"),
        ))
    }
}

pub(crate) use imp::{
    around_attachment_artifact, around_config_create, around_config_replace,
    around_export_artifact, backend, kdf_policy_override, LocalBackend,
};
