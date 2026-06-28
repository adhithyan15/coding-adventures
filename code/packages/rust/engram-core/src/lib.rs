//! Headless spaced-repetition engine for Engram.
//!
//! The crate is intentionally UI-free and platform-free. Web and native
//! frontends should pass timestamps, IDs, and persistence snapshots across a
//! small facade, then let this crate handle scheduling and state transitions.

#![forbid(unsafe_code)]

mod model;
mod queue;
mod reducer;
mod sm2;

pub use model::{
    ActiveSessionState, AppState, Card, CardProgress, Deck, DeckStats, Rating, Review, Session,
    SessionStatus,
};
pub use queue::{build_session_queue, get_deck_stats, is_deck_caught_up};
pub use reducer::{reduce, EngramCommand};
pub use sm2::{
    create_initial_progress, update_card_progress, INITIAL_EASE_FACTOR, MAX_EASE_FACTOR,
    MIN_EASE_FACTOR, ONE_DAY_MS,
};
