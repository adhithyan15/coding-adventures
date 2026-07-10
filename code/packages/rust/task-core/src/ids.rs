//! Typed entity identifiers.
//!
//! Every entity is addressed by a stable, string-backed id. We use **strings**
//! (canonical UUID-v7 text, minted by the facade) rather than a 128-bit integer for
//! one deliberate reason: this core is consumed from JavaScript through WebAssembly,
//! and JSON numbers larger than 2^53 lose precision in JS. A string id round-trips
//! losslessly on every platform.
//!
//! **Ids are not generated here.** Generating a UUID v7 needs a clock and a random
//! source; this crate is intentionally clock-free and deterministic, so the *facade*
//! (`task-core-wasm`, which the host feeds `now` and entropy) mints ids and passes
//! them in on the create-commands. Within the core an id is just an opaque, ordered
//! key — see [`TaskId::from_raw`].
//!
//! Each entity gets its *own* newtype (`TaskId`, `ResourceId`, …) so the compiler
//! rejects passing a resource id where a task id is expected.

/// Define a string-backed id newtype with the standard trait set and optional serde.
///
/// `serde(transparent)` makes the id serialise as a bare string (`"018f…"`), not a
/// wrapper object, keeping the JSON contract clean for host adapters.
macro_rules! id_type {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(transparent))]
        pub struct $name(pub String);

        impl $name {
            /// Wrap an externally-minted id string (the facade mints UUID v7s).
            pub fn from_raw(s: impl Into<String>) -> Self {
                Self(s.into())
            }
            /// Borrow the id as a string slice.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl core::fmt::Display for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_string())
            }
        }
        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(s)
            }
        }
    };
}

id_type!(
    /// Identifies a whole project (one open document).
    ProjectId
);
id_type!(
    /// Identifies a task — the central entity.
    TaskId
);
id_type!(
    /// Identifies a resource (a person, machine, material, or cost).
    ResourceId
);
id_type!(
    /// Identifies a working-time calendar.
    CalendarId
);
id_type!(
    /// Identifies a user-defined custom field.
    FieldId
);
id_type!(
    /// Identifies a captured baseline (a schedule snapshot).
    BaselineId
);
id_type!(
    /// Identifies a saved view (a projection descriptor).
    ViewId
);
id_type!(
    /// Identifies a dependency or generic link.
    LinkId
);
id_type!(
    /// Identifies a workflow (a status state machine).
    WorkflowId
);
id_type!(
    /// Identifies a status within a workflow.
    StatusId
);
