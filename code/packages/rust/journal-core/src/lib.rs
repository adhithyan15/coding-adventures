#![forbid(unsafe_code)]
//! # journal-core
//!
//! The headless engine behind **Journal**, the fourth product of the Mosaic
//! program (#14415): dated entries across several journals, with tags, stars, a
//! timeline, "on this day" recall, and full-text search. The bar is Day One.
//!
//! ## Where this sits
//!
//! ```text
//! journal-core        ← you are here: model + commands + projections. Pure.
//!   └ journal-wasm    (J2b) JSON ABI for web, Electron, and every Mosaic host
//!       …consumed by the Mosaic journal-app (J3)
//! ```
//!
//! The same split as Trestle (`task-core`) and Engram (`engram-core`): the model is
//! written **once**, in Rust, and every one of Mosaic's backends reaches it through
//! the facade instead of re-implementing it in its own language.
//!
//! ## House rules
//!
//! - **Pure.** No I/O, no system clock, no randomness. The current time arrives as
//!   `now_ms: u64`; ids are minted by the host and passed in.
//! - **Zero dependencies by default.** `serde` is opt-in (the facade enables it);
//!   the only path dependency is `datetime-core` for calendar arithmetic.
//! - **Derived data is computed, never stored.** See [`projections`].
//!
//! ## A tour
//!
//! ```
//! use journal_core::{apply, Command, Date, EntryFilter, EntryId, JournalId, JournalState};
//! use journal_core::projections::{search, timeline};
//!
//! let mut state = JournalState::new(JournalId::from("personal"), "Personal", 0).unwrap();
//! apply(&mut state, Command::CreateEntry {
//!     id: EntryId::from("e1"),
//!     journal: JournalId::from("personal"),
//!     date: Date::parse_iso("2026-09-23").unwrap(),
//!     title: "First light".into(),
//!     body: "Walked to the lighthouse before breakfast.".into(),
//! }, 1_000).unwrap();
//!
//! let days = timeline(&state, &EntryFilter::default());
//! assert_eq!(days[0].date.to_iso(), "2026-09-23");
//!
//! let hits = search(&state, "LIGHTHOUSE", &EntryFilter::default());
//! assert_eq!(hits[0].entry.as_str(), "e1");
//! ```
//!
//! ## Module map
//!
//! - [`ids`](JournalId) — string-backed `JournalId` and `EntryId`.
//! - [`date`] — the civil [`Date`] an entry is about, with its ISO wire form.
//! - [`tag`] — [`Tag`]: a display string compared case-insensitively.
//! - [`model`](JournalState) — `Journal`, `Entry`, `JournalState`, and the limits.
//! - [`ops`] — [`Command`] and [`apply`]: validate, then write.
//! - [`projections`] — timeline, on-this-day, search, tag counts, month activity.
//!
//! The full design is `code/specs/journal-core.md`.

pub mod date;
mod ids;
mod model;
pub mod ops;
pub mod projections;
pub mod tag;
mod text;

pub use date::{Date, MAX_YEAR, MIN_YEAR};
pub use ids::*;
pub use model::*;
pub use ops::{apply, Command, OpError};
pub use projections::EntryFilter;
pub use tag::{Tag, TagError};
