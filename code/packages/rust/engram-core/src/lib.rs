//! Headless spaced-repetition engine for Engram.
//!
//! The crate is intentionally UI-free and platform-free. Web and native
//! frontends should pass timestamps, IDs, and persistence snapshots across a
//! small facade, then let this crate handle scheduling and state transitions.

#![forbid(unsafe_code)]

mod csv;
mod history;
mod merge;
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
    export_cards_anki_basic_tsv, export_cards_csv, export_notes_anki_tsv,
    export_notes_anki_tsv_with_context, import_anki_basic_tsv, import_anki_notes_tsv,
    import_basic_cards_csv, import_cards_csv, AnkiBasicTsvExportOptions, AnkiNoteTsvImport,
    AnkiNoteTsvImportOptions, BasicCardCsvImportOptions, CsvError,
};
pub use history::summarize_review_history;
pub use merge::merge_app_states;
pub use model::{
    ActiveSessionState, AppState, Card, CardFlag, CardLineage, CardProgress, CardProgressSnapshot,
    CardState, CardTemplate, DailyStudyLimitUsage, Deck, DeckOptions, DeckOptionsPreset, DeckStats,
    ExternalSourceRecord, ExternalSourceTarget, FieldDef, GeneratedCard, LeechAction, LeechEvent,
    MediaAssetRecord, Note, NoteFieldValue, NoteType, Rating, RatingCounts, Review,
    ReviewHistorySummary, Session, SessionProgress, SessionStatus, TemplateRequirementMode,
};
pub use queue::{
    build_session_queue, build_session_queue_for_state_with_options,
    build_session_queue_with_daily_limits, build_session_queue_with_options, cards_in_deck_scope,
    deck_options_for_state, get_daily_study_limit_usage, get_deck_stats, get_deck_stats_for_state,
    is_deck_caught_up, notes_in_deck_scope,
};
pub use reducer::{reduce, EngramCommand};
pub use scheduler::schedule_review;
pub use search::{
    search_cards, search_cards_with_context, CardSearchResult, SearchContext, SearchError,
};
pub use session::get_active_session_progress;
pub use sm2::{
    create_initial_progress, update_card_progress, INITIAL_EASE_FACTOR, MAX_EASE_FACTOR,
    MIN_EASE_FACTOR, ONE_DAY_MS,
};
pub use snapshot::{
    create_engram_snapshot, restore_engram_snapshot, EngramSnapshot, SnapshotError,
    ENGRAM_SNAPSHOT_APP, ENGRAM_SNAPSHOT_VERSION,
};
pub use template::{
    generate_cards_for_note, materialize_generated_card, normalize_type_answer,
    rename_note_type_field, render_cloze_template, render_cloze_template_with_front_side,
    render_template, render_template_with_front_side, template_references_cloze,
    type_answer_matches, typed_answer_for_template, ClozeRenderSide, TypeAnswerSpec,
};
