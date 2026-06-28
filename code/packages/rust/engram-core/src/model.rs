#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Deck {
    pub id: String,
    pub name: String,
    pub description: String,
    pub created_at: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Card {
    pub id: String,
    pub deck_id: String,
    pub front: String,
    pub back: String,
    pub created_at: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct FieldDef {
    pub id: String,
    pub name: String,
    pub required: bool,
    pub ordinal: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct CardTemplate {
    pub id: String,
    pub name: String,
    pub front_template: String,
    pub back_template: String,
    pub required_field_names: Vec<String>,
    pub ordinal: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct NoteType {
    pub id: String,
    pub name: String,
    pub fields: Vec<FieldDef>,
    pub templates: Vec<CardTemplate>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct NoteFieldValue {
    pub field_id: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Note {
    pub id: String,
    pub note_type_id: String,
    pub deck_id: String,
    pub fields: Vec<NoteFieldValue>,
    pub tags: Vec<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct GeneratedCard {
    pub id: String,
    pub note_id: String,
    pub note_type_id: String,
    pub template_id: String,
    pub deck_id: String,
    pub ordinal: u32,
    pub front: String,
    pub back: String,
    pub tags: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum CardState {
    Learning,
    Review,
    Relearning,
    Suspended,
    Buried,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum CardFlag {
    Red,
    Orange,
    Green,
    Blue,
    Pink,
    Turquoise,
    Purple,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct CardProgress {
    pub card_id: String,
    pub state: CardState,
    pub interval: u32,
    pub ease_factor: f64,
    pub next_due_at: u64,
    pub learning_step_index: Option<u32>,
    pub buried_until: Option<u64>,
    pub suspended_at: Option<u64>,
    pub times_seen: u32,
    pub times_correct: u32,
    pub times_incorrect: u32,
    pub last_seen_at: u64,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub flag: Option<CardFlag>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub marked_at: Option<u64>,
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
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
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

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Review {
    pub id: String,
    pub session_id: String,
    pub card_id: String,
    pub rating: Rating,
    pub reviewed_at: u64,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub previous_progress: Option<CardProgress>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub resulting_progress: Option<CardProgress>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ActiveSessionState {
    pub session_id: String,
    pub deck_id: String,
    pub queue: Vec<Card>,
    pub current_index: usize,
    pub revealed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SessionProgress {
    pub session_id: String,
    pub deck_id: String,
    pub total_cards: usize,
    pub current_index: usize,
    pub current_position: usize,
    pub remaining_cards: usize,
    pub cards_reviewed: u32,
    pub cards_correct: u32,
    pub revealed: bool,
    pub completed: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct AppState {
    pub decks: Vec<Deck>,
    pub note_types: Vec<NoteType>,
    pub notes: Vec<Note>,
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
