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
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub deck_id: Option<String>,
    pub required_field_names: Vec<String>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "TemplateRequirementMode::is_all")
    )]
    pub requirement_mode: TemplateRequirementMode,
    pub ordinal: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum TemplateRequirementMode {
    #[default]
    All,
    Any,
}

impl TemplateRequirementMode {
    pub fn is_all(&self) -> bool {
        matches!(self, Self::All)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct NoteType {
    pub id: String,
    pub name: String,
    pub fields: Vec<FieldDef>,
    pub templates: Vec<CardTemplate>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub stylesheet: Option<String>,
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum LeechAction {
    Suspend,
    #[default]
    TagOnly,
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
    pub fsrs_stability: Option<f64>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub fsrs_difficulty: Option<f64>,
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

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct LeechEvent {
    pub action: LeechAction,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub note_id: Option<String>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub previous_note_tags: Option<Vec<String>>,
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
    pub answer_time_ms: Option<u32>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub leech_event: Option<LeechEvent>,
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
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub current_card_started_at: Option<u64>,
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
    Media,
    Session,
    Deleted,
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
    /// The asset's bytes.
    ///
    /// Serialised as base64 rather than through the derive, which emits a JSON
    /// array of decimal numbers: 4.6 wire bytes per byte of media against
    /// base64's 1.33. On `wasm32` that difference is why `engram-anki-package`
    /// carries a media ceiling at all (#13671).
    ///
    /// Reading accepts BOTH encodings. Snapshots already exist on users'
    /// machines at `~/.engram/mosaic-snapshot.v1.json` holding the numeric
    /// array, and a decoder that only understood base64 would fail to load
    /// every one of them.
    #[cfg_attr(feature = "serde", serde(with = "media_data"))]
    pub data: Vec<u8>,
}

/// Base64 on write, base64 *or* the legacy numeric array on read.
#[cfg(feature = "serde")]
mod media_data {
    use coding_adventures_base64::{decode, encode, STANDARD};
    use serde::de::{Error, SeqAccess, Visitor};
    use serde::{Deserializer, Serializer};
    use std::fmt;

    pub fn serialize<S: Serializer>(data: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&encode(data, &STANDARD))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        deserializer.deserialize_any(MediaDataVisitor)
    }

    struct MediaDataVisitor;

    impl<'de> Visitor<'de> for MediaDataVisitor {
        type Value = Vec<u8>;

        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("a base64 string, or an array of byte values (legacy)")
        }

        fn visit_str<E: Error>(self, value: &str) -> Result<Self::Value, E> {
            decode(value, &STANDARD).map_err(E::custom)
        }

        /// The pre-base64 encoding, still on disk in every existing snapshot.
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut out = Vec::with_capacity(seq.size_hint().unwrap_or(0));
            while let Some(byte) = seq.next_element::<u8>()? {
                out.push(byte);
            }
            Ok(out)
        }
    }
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
    pub initial_ease_factor: f64,
    pub maximum_interval_days: u32,
    pub review_interval_modifier: f64,
    pub hard_interval_multiplier: f64,
    pub easy_bonus_multiplier: f64,
    pub lapse_interval_multiplier: f64,
    pub leech_threshold: u32,
    pub leech_action: LeechAction,
    pub bury_new_siblings: bool,
    pub bury_review_siblings: bool,
    pub bury_interday_learning_siblings: bool,
    pub desired_retention: f64,
    pub fsrs_parameters: Vec<f64>,
    pub fsrs_parameter_search: String,
    pub ignore_review_history_before: String,
    pub historical_retention: f64,
    pub easy_days_percentages: Vec<f64>,
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
            initial_ease_factor: 2.5,
            maximum_interval_days: 36_500,
            review_interval_modifier: 1.0,
            hard_interval_multiplier: 1.2,
            easy_bonus_multiplier: 1.3,
            lapse_interval_multiplier: 0.0,
            leech_threshold: 8,
            leech_action: LeechAction::TagOnly,
            bury_new_siblings: true,
            bury_review_siblings: true,
            bury_interday_learning_siblings: true,
            desired_retention: 0.9,
            fsrs_parameters: Vec::new(),
            fsrs_parameter_search: String::new(),
            ignore_review_history_before: String::new(),
            historical_retention: 0.9,
            easy_days_percentages: vec![1.0; 7],
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

#[cfg(all(test, feature = "serde"))]
mod media_data_tests {
    use super::MediaAssetRecord;

    fn record(data: Vec<u8>) -> MediaAssetRecord {
        MediaAssetRecord {
            id: "asset-1".into(),
            archive_name: "cat.png".into(),
            filename: Some("cat.png".into()),
            data,
        }
    }

    /// Media now travels as base64, not a JSON array of decimal numbers.
    #[test]
    fn media_serialises_as_base64() {
        let json = serde_json::to_string(&record(b"foobar".to_vec())).unwrap();
        assert!(
            json.contains(r#""data":"Zm9vYmFy""#),
            "expected base64, got {json}"
        );
    }

    /// Snapshots already on disk hold the numeric array, and must still load.
    ///
    /// `~/.engram/mosaic-snapshot.v1.json` exists on users' machines. A decoder
    /// that only understood base64 would fail to read every one of them, which
    /// reads to the user as losing their collection.
    #[test]
    fn a_legacy_numeric_array_snapshot_still_loads() {
        let legacy = r#"{
            "id": "asset-1",
            "archiveName": "cat.png",
            "filename": "cat.png",
            "data": [102, 111, 111, 98, 97, 114]
        }"#;
        let parsed: MediaAssetRecord = serde_json::from_str(legacy).unwrap();
        assert_eq!(parsed.data, b"foobar");
    }

    #[test]
    fn both_encodings_round_trip_to_the_same_record() {
        let original = record((0..=255u8).collect());
        let json = serde_json::to_string(&original).unwrap();
        let back: MediaAssetRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(back, original);

        // And the legacy spelling of the same bytes lands in the same place.
        let numbers: Vec<String> = original.data.iter().map(|b| b.to_string()).collect();
        let legacy = format!(
            r#"{{"id":"asset-1","archiveName":"cat.png","filename":"cat.png","data":[{}]}}"#,
            numbers.join(",")
        );
        let from_legacy: MediaAssetRecord = serde_json::from_str(&legacy).unwrap();
        assert_eq!(from_legacy, original);
    }

    #[test]
    fn empty_media_round_trips() {
        let json = serde_json::to_string(&record(Vec::new())).unwrap();
        assert!(json.contains(r#""data":"""#), "{json}");
        let back: MediaAssetRecord = serde_json::from_str(&json).unwrap();
        assert!(back.data.is_empty());
    }

    /// A corrupt payload is an error, not silently-empty media.
    #[test]
    fn malformed_base64_is_rejected() {
        let bad = r#"{"id":"a","archiveName":"b","filename":null,"data":"not base64!"}"#;
        assert!(serde_json::from_str::<MediaAssetRecord>(bad).is_err());
    }

    /// The wire size difference, measured rather than described.
    ///
    /// Both encodings of the SAME record are built and compared, so the number
    /// comes from the serialiser rather than from arithmetic about offsets.
    #[test]
    fn base64_is_smaller_than_the_array_encoding_it_replaced() {
        let data: Vec<u8> = (0..3000u32).map(|i| (i % 251) as u8).collect();
        let bytes = data.len();

        let base64_json = serde_json::to_string(&record(data.clone())).unwrap();

        // The legacy spelling of the identical record.
        let numbers: Vec<String> = data.iter().map(|b| b.to_string()).collect();
        let legacy_json = format!(
            r#"{{"id":"asset-1","archiveName":"cat.png","filename":"cat.png","data":[{}]}}"#,
            numbers.join(",")
        );

        // Both must decode to the same record, or the comparison is between
        // two different things.
        let from_base64: MediaAssetRecord = serde_json::from_str(&base64_json).unwrap();
        let from_legacy: MediaAssetRecord = serde_json::from_str(&legacy_json).unwrap();
        assert_eq!(from_base64, from_legacy);

        let base64_ratio = base64_json.len() as f64 / bytes as f64;
        let legacy_ratio = legacy_json.len() as f64 / bytes as f64;
        assert!(
            base64_ratio < 1.5 && legacy_ratio > 3.0,
            "expected ~1.33 vs ~4.6 wire bytes per media byte, \
             got {base64_ratio:.2} vs {legacy_ratio:.2}"
        );
    }
}
