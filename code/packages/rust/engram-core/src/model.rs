#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub lineage: Option<CardLineage>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct CardLineage {
    pub note_id: String,
    pub note_type_id: String,
    pub template_id: String,
    pub ordinal: u32,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub cloze_ordinal: Option<u32>,
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
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub cloze_ordinal: Option<u32>,
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
pub struct CardProgressSnapshot {
    pub card_id: String,
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
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub previous_active_session: Option<ActiveSessionState>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub sibling_progress_snapshots: Vec<CardProgressSnapshot>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct RatingCounts {
    pub again: usize,
    pub hard: usize,
    pub good: usize,
    pub easy: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ReviewHistorySummary {
    pub deck_id: String,
    pub reviewed_after: u64,
    pub reviewed_before: u64,
    pub total_reviews: usize,
    pub correct_reviews: usize,
    pub unique_cards: usize,
    pub rating_counts: RatingCounts,
    pub first_reviewed_at: Option<u64>,
    pub last_reviewed_at: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct DailyStudyLimitUsage {
    pub deck_id: String,
    pub day_start: u64,
    pub day_end: u64,
    pub new_cards_seen: usize,
    pub review_cards_seen: usize,
    pub remaining_new_cards: usize,
    pub remaining_reviews: usize,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum ExternalSourceTarget {
    Collection,
    Deck,
    NoteType,
    Note,
    Card,
    Review,
    Session,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ExternalSourceRecord {
    pub target: ExternalSourceTarget,
    pub target_id: String,
    pub source: String,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub original_id: Option<String>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "BTreeMap::is_empty")
    )]
    pub data: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct MediaAssetRecord {
    pub id: String,
    pub archive_name: String,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub filename: Option<String>,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase", default))]
pub struct DeckOptions {
    pub new_cards_per_day: u32,
    pub reviews_per_day: u32,
    pub learning_steps_minutes: Vec<u32>,
    pub relearning_steps_minutes: Vec<u32>,
    pub graduating_interval_days: u32,
    pub easy_interval_days: u32,
    pub lapse_interval_multiplier: f64,
}

impl Default for DeckOptions {
    fn default() -> Self {
        Self {
            new_cards_per_day: 20,
            reviews_per_day: 200,
            learning_steps_minutes: vec![1, 10],
            relearning_steps_minutes: vec![10],
            graduating_interval_days: 1,
            easy_interval_days: 4,
            lapse_interval_multiplier: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct DeckOptionsPreset {
    pub deck_id: String,
    pub options: DeckOptions,
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
    #[cfg_attr(feature = "serde", serde(default))]
    pub deck_options: Vec<DeckOptionsPreset>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub external_sources: Vec<ExternalSourceRecord>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub media_assets: Vec<MediaAssetRecord>,
    pub active_session: Option<ActiveSessionState>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeckStats {
    pub total: usize,
    pub new_count: usize,
    pub learning_count: usize,
    pub mastered_count: usize,
    pub due_count: usize,
    pub suspended_count: usize,
    pub buried_count: usize,
    pub average_ease_factor: f64,
}
