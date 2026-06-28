//! Headless spaced-repetition engine for Engram.
//!
//! The crate is intentionally UI-free and platform-free. Web and native
//! frontends should pass timestamps, IDs, and persistence snapshots across a
//! small facade, then let this crate handle scheduling and state transitions.

#![forbid(unsafe_code)]

mod csv;
mod history;
mod model;
mod queue;
mod reducer;
mod scheduler;
mod search;
mod session;
mod sm2;
mod snapshot;
mod template;

pub use csv::{
    export_cards_csv, import_basic_cards_csv, import_cards_csv, BasicCardCsvImportOptions, CsvError,
};
pub use history::summarize_review_history;
pub use model::{
    ActiveSessionState, AppState, Card, CardFlag, CardProgress, CardState, CardTemplate, Deck,
    DeckStats, FieldDef, GeneratedCard, Note, NoteFieldValue, NoteType, Rating, RatingCounts,
    Review, ReviewHistorySummary, Session, SessionProgress, SessionStatus,
};
pub use queue::{
    build_session_queue, build_session_queue_with_options, get_deck_stats, is_deck_caught_up,
};
pub use reducer::{reduce, EngramCommand};
pub use scheduler::{schedule_review, DeckOptions};
pub use search::{search_cards, CardSearchResult, SearchError};
pub use session::get_active_session_progress;
pub use sm2::{
    create_initial_progress, update_card_progress, INITIAL_EASE_FACTOR, MAX_EASE_FACTOR,
    MIN_EASE_FACTOR, ONE_DAY_MS,
};
pub use snapshot::{
    create_engram_snapshot, restore_engram_snapshot, EngramSnapshot, SnapshotError,
    ENGRAM_SNAPSHOT_APP, ENGRAM_SNAPSHOT_VERSION,
};
pub use template::{generate_cards_for_note, render_template};
