#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Deck {
    pub id: String,
    pub name: String,
    pub description: String,
    pub created_at: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Card {
    pub id: String,
    pub deck_id: String,
    pub front: String,
    pub back: String,
    pub created_at: u64,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CardProgress {
    pub card_id: String,
    pub interval: u32,
    pub ease_factor: f64,
    pub next_due_at: u64,
    pub times_seen: u32,
    pub times_correct: u32,
    pub times_incorrect: u32,
    pub last_seen_at: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum SessionStatus {
    Active,
    Completed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Session {
    pub id: String,
    pub deck_id: String,
    pub status: SessionStatus,
    pub started_at: u64,
    pub ended_at: Option<u64>,
    pub cards_reviewed: u32,
    pub cards_correct: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum Rating {
    Again,
    Hard,
    Good,
    Easy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Review {
    pub id: String,
    pub session_id: String,
    pub card_id: String,
    pub rating: Rating,
    pub reviewed_at: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ActiveSessionState {
    pub session_id: String,
    pub deck_id: String,
    pub queue: Vec<Card>,
    pub current_index: usize,
    pub revealed: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AppState {
    pub decks: Vec<Deck>,
    pub cards: Vec<Card>,
    pub card_progress: Vec<CardProgress>,
    pub sessions: Vec<Session>,
    pub reviews: Vec<Review>,
    pub active_session: Option<ActiveSessionState>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeckStats {
    pub total: usize,
    pub new_count: usize,
    pub learning_count: usize,
    pub mastered_count: usize,
    pub due_count: usize,
    pub average_ease_factor: f64,
}
