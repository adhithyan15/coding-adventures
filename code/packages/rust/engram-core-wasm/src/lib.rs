//! JSON facade over `engram-core`.
//!
//! This crate is deliberately not the raw `extern "C"` WASM ABI. It is the
//! testable contract layer that WASM, C-ABI, Electron, HTML, Qt, XAML, and
//! SwiftUI bindings can all share.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};

#[cfg(not(target_arch = "wasm32"))]
use engram_anki_package::{
    read_v11_collection_as_engram_state, write_legacy_apkg_from_engram_state,
};
use engram_core::{
    analyze_media_references, build_session_queue_for_state_with_options,
    build_session_queue_with_daily_limits, cards_in_deck_scope, create_engram_snapshot,
    deck_options_for_state, empty_filtered_deck as empty_core_filtered_deck,
    export_cards_anki_basic_tsv, export_cards_csv, export_notes_anki_tsv_with_context,
    generate_cards_for_note, get_active_session_progress, get_daily_study_limit_usage,
    get_deck_stats_for_state, import_anki_basic_tsv, import_anki_notes_tsv, import_basic_cards_csv,
    import_cards_csv, materialize_generated_card, merge_app_states, notes_in_deck_scope,
    rebuild_filtered_deck as rebuild_core_filtered_deck, reduce, rename_note_type_field,
    restore_engram_snapshot, search_cards as search_core_cards, search_cards_with_context,
    summarize_review_history, type_answer_matches, typed_answer_for_template,
    AnkiBasicTsvExportOptions, AnkiNoteTsvImport, AnkiNoteTsvImportOptions, AppState,
    BasicCardCsvImportOptions, Card, CardFlag, CardLineage, CardProgress, CardSearchResult,
    CardState, ClozeRenderSide, DeckOptions, EngramSnapshot, ExternalSourceRecord,
    ExternalSourceTarget, LeechAction, MediaAssetRecord, Note, NoteFieldValue, Rating,
    SearchContext, TypeAnswerSpec,
};
use serde::Deserialize;
use serde_json::{json, Value};

const DEFAULT_BROWSER_QUERY: &str = "is:due OR is:new";
const DEFAULT_CUSTOM_STUDY_LIMIT: usize = 100;
const BROWSER_FILTER_ALL: &str = "All";
const BROWSER_FILTER_OPTIONS: [&str; 7] = [
    BROWSER_FILTER_ALL,
    "New",
    "Due",
    "Learning",
    "Review",
    "Suspended",
    "Buried",
];
const ANKI_TYPE_NEW: i64 = 0;
const ANKI_TYPE_LEARN: i64 = 1;
const ANKI_TYPE_REVIEW: i64 = 2;
const ANKI_TYPE_RELEARN: i64 = 3;
const ANKI_QUEUE_SCHED_BURIED: i64 = -3;
const ANKI_QUEUE_USER_BURIED: i64 = -2;
const ANKI_QUEUE_SUSPENDED: i64 = -1;
const ANKI_QUEUE_NEW: i64 = 0;
const ANKI_QUEUE_REVIEW: i64 = 2;
const DEMO_SNAPSHOT_JSON: &str = r#"{
  "decks": [
    {
      "id": "tamil-script",
      "name": "Tamil::Script and Roots",
      "description": "Native Tamil script, family words, and Dravidian roots.",
      "createdAt": 1700000000000
    },
    {
      "id": "hindi-devanagari",
      "name": "Hindi::Devanagari",
      "description": "Hindi script and Indo-Aryan vocabulary.",
      "createdAt": 1700000000000
    },
    {
      "id": "kannada-script",
      "name": "Kannada::Script",
      "description": "Kannada letters and South Indian cognates.",
      "createdAt": 1700000000000
    },
    {
      "id": "spanish-latin-roots",
      "name": "Spanish::Latin Roots",
      "description": "Spanish words with Latin and English connections.",
      "createdAt": 1700000000000
    }
  ],
  "noteTypes": [
    {
      "id": "basic-story",
      "name": "Basic Grammar Story",
      "fields": [
        {"id": "front", "name": "Front", "required": true, "ordinal": 0},
        {"id": "back", "name": "Back", "required": true, "ordinal": 1},
        {"id": "story", "name": "Story", "required": false, "ordinal": 2}
      ],
      "templates": [
        {
          "id": "forward",
          "name": "Forward",
          "frontTemplate": "{{Front}}",
          "backTemplate": "{{Back}}\n\n{{Story}}",
          "requiredFieldNames": ["Front"],
          "ordinal": 0
        }
      ],
      "createdAt": 1700000000000,
      "updatedAt": 1700000000000
    }
  ],
  "notes": [
    {
      "id": "note-tamil-amma",
      "noteTypeId": "basic-story",
      "deckId": "tamil-script",
      "fields": [
        {"fieldId": "front", "value": "அம்மா / amma"},
        {"fieldId": "back", "value": "mother"},
        {"fieldId": "story", "value": "A family word shared across many Dravidian languages; compare Tamil amma and Kannada amma."}
      ],
      "tags": ["tamil", "dravidian", "family"],
      "createdAt": 1700000000000,
      "updatedAt": 1700000000000
    },
    {
      "id": "note-tamil-uyir",
      "noteTypeId": "basic-story",
      "deckId": "tamil-script",
      "fields": [
        {"fieldId": "front", "value": "உயிர் / uyir"},
        {"fieldId": "back", "value": "life; vowel"},
        {"fieldId": "story", "value": "Tamil grammar calls vowels uyir ezhuthu, literally life letters."}
      ],
      "tags": ["tamil", "script", "grammar"],
      "createdAt": 1700000000001,
      "updatedAt": 1700000000001
    },
    {
      "id": "note-hindi-namaste",
      "noteTypeId": "basic-story",
      "deckId": "hindi-devanagari",
      "fields": [
        {"fieldId": "front", "value": "नमस्ते / namaste"},
        {"fieldId": "back", "value": "hello; I bow to you"},
        {"fieldId": "story", "value": "From Sanskrit namas, a bow or reverence, plus te, to you."}
      ],
      "tags": ["hindi", "sanskrit", "greeting"],
      "createdAt": 1700000000002,
      "updatedAt": 1700000000002
    },
    {
      "id": "note-kannada-amma",
      "noteTypeId": "basic-story",
      "deckId": "kannada-script",
      "fields": [
        {"fieldId": "front", "value": "ಅಮ್ಮ / amma"},
        {"fieldId": "back", "value": "mother"},
        {"fieldId": "story", "value": "A Kannada cognate that lets the Tamil amma story travel sideways across South India."}
      ],
      "tags": ["kannada", "dravidian", "family"],
      "createdAt": 1700000000003,
      "updatedAt": 1700000000003
    },
    {
      "id": "note-spanish-hablar",
      "noteTypeId": "basic-story",
      "deckId": "spanish-latin-roots",
      "fields": [
        {"fieldId": "front", "value": "hablar"},
        {"fieldId": "back", "value": "to speak"},
        {"fieldId": "story", "value": "From Latin fabulari, to converse; a cousin of English fable and French fable."}
      ],
      "tags": ["spanish", "latin", "etymology"],
      "createdAt": 1700000000004,
      "updatedAt": 1700000000004
    }
  ],
  "cards": [
    {
      "id": "card-tamil-amma",
      "deckId": "tamil-script",
      "front": "அம்மா / amma",
      "back": "mother",
      "createdAt": 1700000000000,
      "lineage": {"noteId": "note-tamil-amma", "noteTypeId": "basic-story", "templateId": "forward", "ordinal": 0}
    },
    {
      "id": "card-tamil-uyir",
      "deckId": "tamil-script",
      "front": "உயிர் / uyir",
      "back": "life; vowel",
      "createdAt": 1700000000001,
      "lineage": {"noteId": "note-tamil-uyir", "noteTypeId": "basic-story", "templateId": "forward", "ordinal": 0}
    },
    {
      "id": "card-hindi-namaste",
      "deckId": "hindi-devanagari",
      "front": "नमस्ते / namaste",
      "back": "hello; I bow to you",
      "createdAt": 1700000000002,
      "lineage": {"noteId": "note-hindi-namaste", "noteTypeId": "basic-story", "templateId": "forward", "ordinal": 0}
    },
    {
      "id": "card-kannada-amma",
      "deckId": "kannada-script",
      "front": "ಅಮ್ಮ / amma",
      "back": "mother",
      "createdAt": 1700000000003,
      "lineage": {"noteId": "note-kannada-amma", "noteTypeId": "basic-story", "templateId": "forward", "ordinal": 0}
    },
    {
      "id": "card-spanish-hablar",
      "deckId": "spanish-latin-roots",
      "front": "hablar",
      "back": "to speak",
      "createdAt": 1700000000004,
      "lineage": {"noteId": "note-spanish-hablar", "noteTypeId": "basic-story", "templateId": "forward", "ordinal": 0}
    }
  ],
  "cardProgress": [
    {
      "cardId": "card-tamil-amma",
      "state": "review",
      "interval": 3,
      "easeFactor": 2.5,
      "nextDueAt": 1699999999900,
      "learningStepIndex": null,
      "buriedUntil": null,
      "suspendedAt": null,
      "timesSeen": 2,
      "timesCorrect": 2,
      "timesIncorrect": 0,
      "lastSeenAt": 1699999990000
    }
  ],
  "sessions": [],
  "reviews": [
    {
      "id": "review-demo-good",
      "sessionId": "demo-session",
      "cardId": "card-tamil-amma",
      "rating": "good",
      "reviewedAt": 1699999990000
    },
    {
      "id": "review-demo-again",
      "sessionId": "demo-session",
      "cardId": "card-spanish-hablar",
      "rating": "again",
      "reviewedAt": 1699999980000
    }
  ],
  "deckOptions": [
    {
      "deckId": "tamil-script",
      "options": {
        "newCardsPerDay": 12,
        "reviewsPerDay": 80,
        "learningStepsMinutes": [1, 10],
        "relearningStepsMinutes": [10],
        "graduatingIntervalDays": 2,
        "easyIntervalDays": 5,
        "initialEaseFactor": 2.8,
        "maximumIntervalDays": 90,
        "reviewIntervalModifier": 0.75,
        "hardIntervalMultiplier": 1.4,
        "easyBonusMultiplier": 1.6,
        "lapseIntervalMultiplier": 0.5,
        "leechThreshold": 6,
        "desiredRetention": 0.92,
        "fsrsParameters": [0.1, 1.2, 2.3],
        "fsrsParameterSearch": "tag:tamil is:review",
        "ignoreReviewHistoryBefore": "2024-01-02",
        "historicalRetention": 0.86,
        "easyDaysPercentages": [1.0, 0.9, 0.8, 1.1, 1.2, 1.0, 0.95],
        "leechAction": "suspend",
        "buryNewSiblings": false,
        "buryReviewSiblings": true,
        "buryInterdayLearningSiblings": false
      }
    }
  ],
  "externalSources": [],
  "mediaAssets": [],
  "activeSession": null
}"#;

#[derive(Default)]
pub struct EngramSession {
    state: AppState,
    selected_deck_id: Option<String>,
    active_screen: EngramAppScreen,
    browser: BrowserSessionState,
    review: ReviewSessionState,
    editor: NoteEditorSessionState,
    note_type_editor: NoteTypeEditorSessionState,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum EngramAppScreen {
    #[default]
    Decks,
    Study,
    Browse,
    Add,
    Stats,
    Options,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BrowserSessionState {
    query: String,
    filter: String,
    tag_edit: String,
    filter_open: bool,
    flag_picker_open: bool,
    selected_index: usize,
    custom_study_limit: usize,
    custom_study_reschedule: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ReviewSessionState {
    typed_answer_card_id: Option<String>,
    typed_answer: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct NoteEditorSessionState {
    selected_field_index: usize,
    draft_note_id: Option<String>,
    draft_note_type_id: Option<String>,
    draft_deck_id: Option<String>,
    draft_created_at: Option<u64>,
    draft_is_new: bool,
    confirm_delete: bool,
    draft_fields: HashMap<String, String>,
    draft_tags: Option<String>,
}

impl NoteEditorSessionState {
    fn reset(&mut self) {
        *self = Self::default();
    }

    fn reset_for_note(&mut self, note_id: &str) {
        if self.draft_note_id.as_deref() != Some(note_id) {
            self.draft_note_id = Some(note_id.to_string());
            self.draft_note_type_id = None;
            self.draft_deck_id = None;
            self.draft_created_at = None;
            self.draft_is_new = false;
            self.confirm_delete = false;
            self.draft_fields.clear();
            self.draft_tags = None;
        }
    }

    fn start_new(&mut self, note_id: String, note_type_id: String, deck_id: String, now: u64) {
        self.selected_field_index = 0;
        self.draft_note_id = Some(note_id);
        self.draft_note_type_id = (!note_type_id.is_empty()).then_some(note_type_id);
        self.draft_deck_id = (!deck_id.is_empty()).then_some(deck_id);
        self.draft_created_at = Some(now);
        self.draft_is_new = true;
        self.confirm_delete = false;
        self.draft_fields.clear();
        self.draft_tags = Some(String::new());
    }

    fn set_selected_field_index(&mut self, index: usize) {
        self.selected_field_index = index;
        self.confirm_delete = false;
    }

    fn set_note_type(&mut self, note_id: &str, note_type_id: String) {
        self.reset_for_note_if_needed(note_id);
        self.draft_note_type_id = Some(note_type_id);
        self.confirm_delete = false;
        self.draft_fields.clear();
    }

    fn set_deck(&mut self, note_id: &str, deck_id: String) {
        self.reset_for_note_if_needed(note_id);
        self.draft_deck_id = Some(deck_id);
        self.confirm_delete = false;
    }

    fn set_field_value(&mut self, note_id: &str, field_id: &str, value: String) {
        self.reset_for_note_if_needed(note_id);
        self.confirm_delete = false;
        self.draft_fields.insert(field_id.to_string(), value);
    }

    fn set_tags(&mut self, note_id: &str, value: String) {
        self.reset_for_note_if_needed(note_id);
        self.confirm_delete = false;
        self.draft_tags = Some(value);
    }

    fn ask_delete_confirmation(&mut self) {
        self.confirm_delete = true;
    }

    fn reset_for_note_if_needed(&mut self, note_id: &str) {
        if self.draft_note_id.as_deref() != Some(note_id) {
            self.reset_for_note(note_id);
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct NoteTypeEditorSessionState {
    selected_index: usize,
    selected_field_index: usize,
    selected_template_index: usize,
    draft_note_type_id: Option<String>,
    draft_name: Option<String>,
    draft_stylesheet: Option<String>,
    draft_field_names: HashMap<String, String>,
    draft_field_required: HashMap<String, bool>,
    draft_template_names: HashMap<String, String>,
    draft_template_fronts: HashMap<String, String>,
    draft_template_backs: HashMap<String, String>,
    draft_created_at: Option<u64>,
    draft_is_new: bool,
    confirm_delete: bool,
}

impl NoteTypeEditorSessionState {
    fn reset(&mut self) {
        *self = Self::default();
    }

    fn select_index(&mut self, index: usize) {
        self.selected_index = index;
        self.selected_field_index = 0;
        self.selected_template_index = 0;
        self.draft_note_type_id = None;
        self.draft_name = None;
        self.draft_stylesheet = None;
        self.draft_field_names.clear();
        self.draft_field_required.clear();
        self.draft_template_names.clear();
        self.draft_template_fronts.clear();
        self.draft_template_backs.clear();
        self.draft_created_at = None;
        self.draft_is_new = false;
        self.confirm_delete = false;
    }

    fn start_new(&mut self, now: u64) {
        self.selected_index = usize::MAX;
        self.selected_field_index = 0;
        self.selected_template_index = 0;
        self.draft_note_type_id = Some(format!("note-type-{now}"));
        self.draft_name = Some("Basic".to_string());
        self.draft_stylesheet = Some(String::new());
        self.draft_field_names.clear();
        self.draft_field_required.clear();
        self.draft_template_names.clear();
        self.draft_template_fronts.clear();
        self.draft_template_backs.clear();
        self.draft_created_at = Some(now);
        self.draft_is_new = true;
        self.confirm_delete = false;
    }

    fn ensure_selected_draft(&mut self, note_type_id: &str) {
        if self.draft_note_type_id.as_deref() != Some(note_type_id) {
            self.draft_note_type_id = Some(note_type_id.to_string());
            self.draft_name = None;
            self.draft_stylesheet = None;
            self.draft_field_names.clear();
            self.draft_field_required.clear();
            self.draft_template_names.clear();
            self.draft_template_fronts.clear();
            self.draft_template_backs.clear();
            self.draft_created_at = None;
            self.draft_is_new = false;
            self.confirm_delete = false;
        }
    }

    fn set_selected_field_index(&mut self, index: usize) {
        self.selected_field_index = index;
        self.confirm_delete = false;
    }

    fn set_selected_template_index(&mut self, index: usize) {
        self.selected_template_index = index;
        self.confirm_delete = false;
    }

    fn set_name(&mut self, note_type_id: &str, value: String) {
        self.ensure_selected_draft(note_type_id);
        self.confirm_delete = false;
        self.draft_name = Some(value);
    }

    fn set_stylesheet(&mut self, note_type_id: &str, value: String) {
        self.ensure_selected_draft(note_type_id);
        self.confirm_delete = false;
        self.draft_stylesheet = Some(value);
    }

    fn set_field_name(&mut self, note_type_id: &str, field_id: &str, value: String) {
        self.ensure_selected_draft(note_type_id);
        self.confirm_delete = false;
        self.draft_field_names.insert(field_id.to_string(), value);
    }

    fn set_field_required(&mut self, note_type_id: &str, field_id: &str, required: bool) {
        self.ensure_selected_draft(note_type_id);
        self.confirm_delete = false;
        self.draft_field_required
            .insert(field_id.to_string(), required);
    }

    fn set_template_name(&mut self, note_type_id: &str, template_id: &str, value: String) {
        self.ensure_selected_draft(note_type_id);
        self.confirm_delete = false;
        self.draft_template_names
            .insert(template_id.to_string(), value);
    }

    fn set_template_front(&mut self, note_type_id: &str, template_id: &str, value: String) {
        self.ensure_selected_draft(note_type_id);
        self.confirm_delete = false;
        self.draft_template_fronts
            .insert(template_id.to_string(), value);
    }

    fn set_template_back(&mut self, note_type_id: &str, template_id: &str, value: String) {
        self.ensure_selected_draft(note_type_id);
        self.confirm_delete = false;
        self.draft_template_backs
            .insert(template_id.to_string(), value);
    }

    fn ask_delete_confirmation(&mut self) {
        self.confirm_delete = true;
    }
}

impl ReviewSessionState {
    fn typed_answer_for_card(&self, card_id: &str) -> &str {
        if self.typed_answer_card_id.as_deref() == Some(card_id) {
            &self.typed_answer
        } else {
            ""
        }
    }

    fn set_typed_answer(&mut self, card_id: String, value: String) {
        self.typed_answer_card_id = Some(card_id);
        self.typed_answer = value;
    }

    fn clear_card(&mut self, card_id: &str) {
        if self.typed_answer_card_id.as_deref() == Some(card_id) {
            self.typed_answer_card_id = None;
            self.typed_answer.clear();
        }
    }
}

impl Default for BrowserSessionState {
    fn default() -> Self {
        Self {
            query: String::new(),
            filter: String::new(),
            tag_edit: String::new(),
            filter_open: false,
            flag_picker_open: false,
            selected_index: 0,
            custom_study_limit: DEFAULT_CUSTOM_STUDY_LIMIT,
            custom_study_reschedule: true,
        }
    }
}

impl BrowserSessionState {
    fn active_query(&self) -> &str {
        let query = self.query.trim();
        if query.is_empty() {
            DEFAULT_BROWSER_QUERY
        } else {
            query
        }
    }

    fn effective_query(&self) -> String {
        compose_browser_filter_query(&self.query, self.active_filter())
    }

    fn active_filter(&self) -> &str {
        normalize_browser_filter_label(&self.filter)
    }

    fn set_query(&mut self, query: String) {
        self.query = query;
        self.selected_index = 0;
    }

    fn toggle_filter(&mut self) {
        self.filter_open = !self.filter_open;
    }

    fn close_filter(&mut self) {
        self.filter_open = false;
    }

    fn set_filter(&mut self, value: String) {
        self.filter = normalize_browser_filter_label(&value).to_string();
        self.filter_open = false;
        self.selected_index = 0;
    }

    fn set_tag_edit(&mut self, value: String) {
        self.tag_edit = value;
    }

    fn active_tag_edit(&self) -> String {
        self.tag_edit.trim().to_string()
    }

    fn toggle_flag_picker(&mut self) {
        self.flag_picker_open = !self.flag_picker_open;
    }

    fn close_flag_picker(&mut self) {
        self.flag_picker_open = false;
    }

    fn set_selected_index(&mut self, value: f64) -> Result<(), String> {
        if !value.is_finite() {
            return Err("browser result index must be a finite number".to_string());
        }
        if value < 0.0 {
            return Err("browser result index must be non-negative".to_string());
        }
        if value > usize::MAX as f64 {
            return Err("browser result index is too large".to_string());
        }
        self.selected_index = value.round() as usize;
        Ok(())
    }

    fn set_custom_study_limit(&mut self, value: f64) -> Result<(), String> {
        if !value.is_finite() {
            return Err("custom study card limit must be a finite number".to_string());
        }
        if value < 0.0 {
            return Err("custom study card limit must be non-negative".to_string());
        }
        if value > usize::MAX as f64 {
            return Err("custom study card limit is too large".to_string());
        }
        self.custom_study_limit = value.round() as usize;
        Ok(())
    }

    fn set_custom_study_reschedule(&mut self, checked: bool) {
        self.custom_study_reschedule = checked;
    }
}

impl EngramSession {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_demo() -> Self {
        Self {
            state: serde_json::from_str(DEMO_SNAPSHOT_JSON)
                .expect("built-in Engram demo snapshot must be valid AppState JSON"),
            ..Self::default()
        }
    }

    pub fn demo_snapshot_json() -> &'static str {
        DEMO_SNAPSHOT_JSON
    }

    pub fn state(&self) -> &AppState {
        &self.state
    }

    fn selected_deck_id(&self, deck_id: &str) -> String {
        selected_deck_id_with_override(&self.state, deck_id, self.selected_deck_id.as_deref())
    }

    fn set_selected_deck_index(&mut self, value: f64) -> Result<(), String> {
        let index = parse_nonnegative_index(value, "deck")?;
        let deck_id = self
            .state
            .decks
            .get(index)
            .map(|deck| deck.id.clone())
            .ok_or_else(|| "cannot select missing deck".to_string())?;
        self.selected_deck_id = Some(deck_id);
        self.browser.selected_index = 0;
        self.editor.reset();
        Ok(())
    }

    fn set_selected_deck_value(&mut self, value: &str) -> Result<(), String> {
        let value = value.trim();
        let deck_id = self
            .state
            .decks
            .iter()
            .find(|deck| deck.id == value || deck.name == value)
            .map(|deck| deck.id.clone())
            .ok_or_else(|| "cannot select missing deck".to_string())?;
        self.selected_deck_id = Some(deck_id);
        self.browser.selected_index = 0;
        self.editor.reset();
        Ok(())
    }

    pub fn snapshot(&self) -> String {
        ok_with("state", &self.state)
    }

    pub fn load_snapshot(&mut self, snapshot_json: &str) -> String {
        catch_json(|| {
            let state: AppState = serde_json::from_str(snapshot_json)
                .map_err(|err| format!("invalid snapshot: {err}"))?;
            self.state = state;
            self.selected_deck_id = None;
            self.browser = BrowserSessionState::default();
            self.review = ReviewSessionState::default();
            self.editor = NoteEditorSessionState::default();
            self.note_type_editor = NoteTypeEditorSessionState::default();
            Ok(ok_with("state", &self.state))
        })
    }

    pub fn export_backup(&self, exported_at: u64) -> String {
        catch_json(|| {
            let snapshot = create_engram_snapshot(&self.state, exported_at);
            Ok(ok_with("snapshot", &snapshot))
        })
    }

    pub fn import_backup(&mut self, snapshot_json: &str) -> String {
        catch_json(|| {
            let snapshot: EngramSnapshot = serde_json::from_str(snapshot_json)
                .map_err(|err| format!("invalid backup: {err}"))?;
            self.state =
                restore_engram_snapshot(snapshot).map_err(|err| err.message.to_string())?;
            self.selected_deck_id = None;
            self.browser = BrowserSessionState::default();
            self.review = ReviewSessionState::default();
            self.editor = NoteEditorSessionState::default();
            self.note_type_editor = NoteTypeEditorSessionState::default();
            Ok(ok_with("state", &self.state))
        })
    }

    pub fn dispatch(&mut self, command_json: &str) -> String {
        catch_json(|| {
            let command: FacadeCommand = serde_json::from_str(command_json)
                .map_err(|err| format!("invalid command: {err}"))?;
            let resets_browser = matches!(command, FacadeCommand::LoadState { .. });
            let command = command.into_core_command();
            self.state = reduce(&self.state, command);
            if resets_browser {
                self.selected_deck_id = None;
                self.browser = BrowserSessionState::default();
                self.review = ReviewSessionState::default();
                self.editor = NoteEditorSessionState::default();
                self.note_type_editor = NoteTypeEditorSessionState::default();
            }
            Ok(ok_with("state", &self.state))
        })
    }

    pub fn build_queue(&self, deck_id: &str, now: u64) -> String {
        catch_json(|| {
            let options = deck_options_for_state(&self.state, deck_id);
            let queue =
                build_session_queue_for_state_with_options(&self.state, deck_id, now, &options);
            Ok(ok_with("queue", &queue))
        })
    }

    pub fn daily_limit_usage(
        &self,
        deck_id: &str,
        day_start: u64,
        day_end: u64,
        deck_options_json: &str,
    ) -> String {
        catch_json(|| {
            let options = parse_deck_options(deck_options_json, &self.state, deck_id)?;
            let usage =
                get_daily_study_limit_usage(&self.state, deck_id, day_start, day_end, &options);
            Ok(ok_with("usage", &usage))
        })
    }

    pub fn build_queue_with_daily_limits(
        &self,
        deck_id: &str,
        now: u64,
        day_start: u64,
        day_end: u64,
        deck_options_json: &str,
    ) -> String {
        catch_json(|| {
            let options = parse_deck_options(deck_options_json, &self.state, deck_id)?;
            let queue = build_session_queue_with_daily_limits(
                &self.state,
                deck_id,
                now,
                day_start,
                day_end,
                &options,
            );
            Ok(ok_with("queue", &queue))
        })
    }

    pub fn deck_stats(&self, deck_id: &str, now: u64) -> String {
        catch_json(|| {
            let stats = get_deck_stats_for_state(&self.state, deck_id, now);
            Ok(ok_with(
                "stats",
                &json!({
                    "total": stats.total,
                    "newCount": stats.new_count,
                    "learningCount": stats.learning_count,
                    "masteredCount": stats.mastered_count,
                    "dueCount": stats.due_count,
                    "suspendedCount": stats.suspended_count,
                    "buriedCount": stats.buried_count,
                    "averageEaseFactor": stats.average_ease_factor,
                }),
            ))
        })
    }

    pub fn empty_filtered_deck(&mut self, deck_id: &str) -> String {
        catch_json(|| {
            self.state = empty_core_filtered_deck(&self.state, deck_id);
            Ok(ok_with("state", &self.state))
        })
    }

    pub fn rebuild_filtered_deck(
        &mut self,
        deck_id: &str,
        search: &str,
        limit: usize,
        reschedule: bool,
        rebuilt_at: u64,
    ) -> String {
        catch_json(|| {
            self.state = rebuild_core_filtered_deck(
                &self.state,
                deck_id,
                search,
                limit,
                reschedule,
                rebuilt_at,
            )
            .map_err(|error| error.message)?;
            Ok(ok_with("state", &self.state))
        })
    }

    pub fn session_progress(&self) -> String {
        catch_json(|| {
            let progress = get_active_session_progress(&self.state);
            Ok(ok_with("progress", &progress))
        })
    }

    pub fn engram_app_props(&self, deck_id: &str, now: u64) -> String {
        catch_json(|| {
            let props = engram_app_props_for_state(
                &self.state,
                deck_id,
                self.selected_deck_id.as_deref(),
                now,
                self.active_screen,
                &self.browser,
                &self.review,
                &self.editor,
                &self.note_type_editor,
            );
            Ok(ok_with("props", &props))
        })
    }

    pub fn engram_browser_props(&self, query: &str, now: u64) -> String {
        catch_json(|| {
            match engram_browser_props_for_state(
                &self.state,
                query,
                &compose_browser_filter_query(query, BROWSER_FILTER_ALL),
                BROWSER_FILTER_ALL,
                now,
                0,
                None,
                false,
                false,
            ) {
                Ok(props) => Ok(ok_with("props", &props)),
                Err(error) => Ok(error_json_with_token(&error.message, &error.token)),
            }
        })
    }

    pub fn handle_engram_app_event(&mut self, event: &str, deck_id: &str, now: u64) -> String {
        catch_json(|| {
            let parsed = parse_engram_app_event(event)?;
            let selected_deck_context = self.selected_deck_id(deck_id);
            match parsed.kind {
                EngramAppEvent::ShowScreen(screen) => {
                    self.active_screen = screen;
                }
                EngramAppEvent::SelectDeck => {
                    if let Some(value) = parsed.number_value {
                        self.set_selected_deck_index(value)?;
                    } else if let Some(value) = parsed.text_value.as_deref() {
                        self.set_selected_deck_value(value)?;
                    } else {
                        return Err("onSelectDeck is missing a deck value".to_string());
                    }
                }
                EngramAppEvent::Reveal => {
                    self.state = reduce(&self.state, engram_core::EngramCommand::RevealCurrentCard);
                }
                EngramAppEvent::Undo => {
                    let session_id = active_session_id(&self.state, "undo")?;
                    self.state = reduce(
                        &self.state,
                        engram_core::EngramCommand::UndoLastReview { session_id },
                    );
                }
                EngramAppEvent::BuryCard => {
                    let card_id = current_active_card_id(&self.state, "bury")?;
                    self.state = reduce(
                        &self.state,
                        engram_core::EngramCommand::BuryCard {
                            card_id,
                            buried_at: now,
                            buried_until: now.saturating_add(engram_core::ONE_DAY_MS),
                        },
                    );
                }
                EngramAppEvent::BurySiblings => {
                    let card_id = current_active_card_id(&self.state, "bury siblings")?;
                    self.state = reduce(
                        &self.state,
                        engram_core::EngramCommand::BuryCardSiblings {
                            card_id,
                            buried_at: now,
                            buried_until: now.saturating_add(engram_core::ONE_DAY_MS),
                        },
                    );
                }
                EngramAppEvent::SuspendCard => {
                    let card_id = current_active_card_id(&self.state, "suspend")?;
                    self.state = suspend_or_unsuspend_card(&self.state, card_id, now);
                }
                EngramAppEvent::ToggleMark => {
                    let card_id = current_active_card_id(&self.state, "mark")?;
                    self.state = mark_or_unmark_card(&self.state, card_id, now);
                }
                EngramAppEvent::TypeAnswerChange => {
                    let card_id = current_active_card_id(&self.state, "type an answer")?;
                    let value = parsed.text_value.clone().ok_or_else(|| {
                        format!("{} is missing text value", parsed.kind.canonical_name())
                    })?;
                    self.review.set_typed_answer(card_id, value);
                }
                EngramAppEvent::DeckOptionsChange(field) => {
                    let selected_deck_id = selected_deck_context.clone();
                    if selected_deck_id.is_empty() {
                        return Err("cannot update deck options without a deck".to_string());
                    }
                    let mut options = deck_options_for_state(&self.state, &selected_deck_id);
                    match field {
                        DeckOptionField::LearningStepsMinutes
                        | DeckOptionField::RelearningStepsMinutes
                        | DeckOptionField::LeechAction
                        | DeckOptionField::FsrsParameters
                        | DeckOptionField::FsrsParameterSearch
                        | DeckOptionField::IgnoreReviewHistoryBefore
                        | DeckOptionField::EasyDaysPercentages => {
                            let value = parsed.text_value.as_deref().ok_or_else(|| {
                                format!("{} is missing text value", parsed.kind.canonical_name())
                            })?;
                            apply_deck_option_text_change(&mut options, field, value)?;
                        }
                        DeckOptionField::BuryNewSiblings
                        | DeckOptionField::BuryReviewSiblings
                        | DeckOptionField::BuryInterdayLearningSiblings => {
                            let checked = parsed.bool_value.ok_or_else(|| {
                                format!("{} is missing checked value", parsed.kind.canonical_name())
                            })?;
                            apply_deck_option_bool_change(&mut options, field, checked)?;
                        }
                        _ => {
                            let value = parsed.number_value.ok_or_else(|| {
                                format!("{} is missing numeric value", parsed.kind.canonical_name())
                            })?;
                            apply_deck_option_number_change(&mut options, field, value)?;
                        }
                    }
                    self.state = reduce(
                        &self.state,
                        engram_core::EngramCommand::SetDeckOptions {
                            deck_id: selected_deck_id,
                            options,
                        },
                    );
                }
                EngramAppEvent::Rate(rating) => {
                    let active_session = self
                        .state
                        .active_session
                        .clone()
                        .ok_or_else(|| "cannot rate without an active session".to_string())?;
                    let card = active_session
                        .queue
                        .get(active_session.current_index)
                        .ok_or_else(|| "cannot rate without a current card".to_string())?;
                    let session_id = active_session.session_id.clone();
                    let card_id = card.id.clone();
                    let review_id = format!(
                        "engram-app::{}::{}::{now}::{}",
                        session_id,
                        card_id,
                        rating_label(rating)
                    );
                    let deck_options = deck_options_for_state(&self.state, &active_session.deck_id);
                    self.state = reduce(
                        &self.state,
                        engram_core::EngramCommand::RateCardWithOptions {
                            review_id,
                            session_id,
                            card_id: card_id.clone(),
                            rating,
                            reviewed_at: now,
                            deck_options,
                        },
                    );
                    self.state = reduce(&self.state, engram_core::EngramCommand::AdvanceSession);
                    self.review.clear_card(&card_id);
                }
                EngramAppEvent::BrowserToggleSuspendSelected => {
                    let card_id = required_browser_event_card_id(
                        &self.state,
                        &self.browser,
                        parsed.card_id.clone(),
                        "toggle suspend",
                        now,
                        Some(selected_deck_context.as_str()),
                    )?;
                    self.state = suspend_or_unsuspend_card(&self.state, card_id, now);
                }
                EngramAppEvent::BrowserToggleMarkSelected => {
                    let card_id = required_browser_event_card_id(
                        &self.state,
                        &self.browser,
                        parsed.card_id.clone(),
                        "mark",
                        now,
                        Some(selected_deck_context.as_str()),
                    )?;
                    self.state = mark_or_unmark_card(&self.state, card_id, now);
                }
                EngramAppEvent::BrowserToggleFlagPicker => {
                    self.browser.toggle_flag_picker();
                }
                EngramAppEvent::BrowserSetFlagSelected => {
                    let card_id = required_browser_event_card_id(
                        &self.state,
                        &self.browser,
                        parsed.card_id.clone(),
                        "set flag on",
                        now,
                        Some(selected_deck_context.as_str()),
                    )?;
                    let flag = browser_event_flag_value(
                        parsed.text_value.as_deref(),
                        parsed.number_value,
                    )?;
                    self.state = reduce(
                        &self.state,
                        engram_core::EngramCommand::SetCardFlag {
                            card_id,
                            flag,
                            flagged_at: now,
                        },
                    );
                    self.browser.close_flag_picker();
                }
                EngramAppEvent::BrowserTagEditChange => {
                    if let Some(value) = parsed.text_value.clone() {
                        self.browser.set_tag_edit(value);
                    }
                }
                EngramAppEvent::BrowserAddTagSelected => {
                    let card_id = required_browser_event_card_id(
                        &self.state,
                        &self.browser,
                        parsed.card_id.clone(),
                        "add tag to",
                        now,
                        Some(selected_deck_context.as_str()),
                    )?;
                    let tag = browser_event_tag_value(
                        &self.browser,
                        parsed.text_value.as_deref(),
                        "add",
                    )?;
                    self.state = reduce(
                        &self.state,
                        engram_core::EngramCommand::AddCardTags {
                            card_ids: vec![card_id],
                            tags: vec![tag],
                            updated_at: now,
                        },
                    );
                }
                EngramAppEvent::BrowserRemoveTagSelected => {
                    let card_id = required_browser_event_card_id(
                        &self.state,
                        &self.browser,
                        parsed.card_id.clone(),
                        "remove tag from",
                        now,
                        Some(selected_deck_context.as_str()),
                    )?;
                    let tag = browser_event_tag_value(
                        &self.browser,
                        parsed.text_value.as_deref(),
                        "remove",
                    )?;
                    self.state = reduce(
                        &self.state,
                        engram_core::EngramCommand::RemoveCardTags {
                            card_ids: vec![card_id],
                            tags: vec![tag],
                            updated_at: now,
                        },
                    );
                }
                EngramAppEvent::BrowserCustomStudyLimitChange => {
                    let value = parsed.number_value.ok_or_else(|| {
                        "onBrowserCustomStudyLimitChange is missing a numeric value".to_string()
                    })?;
                    self.browser.set_custom_study_limit(value)?;
                }
                EngramAppEvent::BrowserCustomStudyRescheduleChange => {
                    let checked = parsed.bool_value.ok_or_else(|| {
                        "onBrowserCustomStudyRescheduleChange is missing a checked value"
                            .to_string()
                    })?;
                    self.browser.set_custom_study_reschedule(checked);
                }
                EngramAppEvent::BrowserRebuildFilteredDeck => {
                    if selected_deck_context.is_empty() {
                        return Err("cannot rebuild filtered deck without a deck".to_string());
                    }
                    self.state = rebuild_core_filtered_deck(
                        &self.state,
                        &selected_deck_context,
                        &self.browser.effective_query(),
                        self.browser.custom_study_limit,
                        self.browser.custom_study_reschedule,
                        now,
                    )
                    .map_err(|err| err.message.to_string())?;
                    self.browser.selected_index = 0;
                    self.editor.reset();
                }
                EngramAppEvent::BrowserEmptyFilteredDeck => {
                    if selected_deck_context.is_empty() {
                        return Err("cannot empty filtered deck without a deck".to_string());
                    }
                    self.state = empty_core_filtered_deck(&self.state, &selected_deck_context);
                    self.browser.selected_index = 0;
                    self.editor.reset();
                }
                EngramAppEvent::PruneUnusedMedia => {
                    let asset_ids = analyze_media_references(&self.state).unreferenced_asset_ids;
                    if !asset_ids.is_empty() {
                        self.state = reduce(
                            &self.state,
                            engram_core::EngramCommand::DeleteMediaAssets { asset_ids },
                        );
                    }
                }
                EngramAppEvent::BrowserQueryChange => {
                    if let Some(value) = parsed.text_value.clone() {
                        self.browser.set_query(value);
                    }
                }
                EngramAppEvent::BrowserToggleFilter => {
                    self.browser.toggle_filter();
                }
                EngramAppEvent::BrowserSetFilter => {
                    let value = parsed
                        .text_value
                        .clone()
                        .ok_or_else(|| "onBrowserSetFilter is missing a value".to_string())?;
                    self.browser.set_filter(value);
                }
                EngramAppEvent::BrowserSearch => {
                    if let Some(value) = parsed.text_value.clone() {
                        self.browser.set_query(value);
                    }
                    self.browser.close_filter();
                }
                EngramAppEvent::BrowserSelectResult => {
                    let value = parsed
                        .number_value
                        .ok_or_else(|| "onBrowserSelectResult is missing an index".to_string())?;
                    self.browser.set_selected_index(value)?;
                    self.editor.reset();
                }
                EngramAppEvent::NoteEditorSelectNoteType => {
                    let value = parsed.number_value.ok_or_else(|| {
                        "onNoteEditorSelectNoteType is missing an index".to_string()
                    })?;
                    let index = parse_nonnegative_index(value, "note type")?;
                    let note_type_id = self
                        .state
                        .note_types
                        .get(index)
                        .map(|note_type| note_type.id.clone())
                        .ok_or_else(|| "cannot select missing note type".to_string())?;
                    let selection = note_editor_selection(
                        &self.state,
                        &self.browser,
                        &self.editor,
                        None,
                        now,
                        Some(selected_deck_context.as_str()),
                    );
                    if selection.note_id.is_empty() {
                        return Err("cannot select note type without a note draft".to_string());
                    }
                    self.editor.set_note_type(&selection.note_id, note_type_id);
                    self.editor.set_selected_field_index(0);
                }
                EngramAppEvent::NoteEditorSelectDeck => {
                    let value = parsed
                        .number_value
                        .ok_or_else(|| "onNoteEditorSelectDeck is missing an index".to_string())?;
                    let index = parse_nonnegative_index(value, "deck")?;
                    let deck_id = self
                        .state
                        .decks
                        .get(index)
                        .map(|deck| deck.id.clone())
                        .ok_or_else(|| "cannot select missing deck".to_string())?;
                    let selection = note_editor_selection(
                        &self.state,
                        &self.browser,
                        &self.editor,
                        None,
                        now,
                        Some(selected_deck_context.as_str()),
                    );
                    if selection.note_id.is_empty() {
                        return Err("cannot select deck without a note draft".to_string());
                    }
                    self.editor.set_deck(&selection.note_id, deck_id);
                }
                EngramAppEvent::NoteEditorSelectField => {
                    let value = parsed
                        .number_value
                        .ok_or_else(|| "onNoteEditorSelectField is missing an index".to_string())?;
                    self.editor
                        .set_selected_field_index(parse_nonnegative_index(value, "note field")?);
                }
                EngramAppEvent::NoteEditorFieldValueChange => {
                    let value = parsed.text_value.clone().ok_or_else(|| {
                        "onNoteEditorFieldValueChange is missing a value".to_string()
                    })?;
                    let selection = note_editor_selection(
                        &self.state,
                        &self.browser,
                        &self.editor,
                        None,
                        now,
                        Some(selected_deck_context.as_str()),
                    );
                    let (field_id, note_id) = note_editor_selected_field(&selection, &self.editor)
                        .map(|field| (field.id.clone(), selection.note_id.clone()))
                        .ok_or_else(|| {
                            "cannot edit note field without a selected note field".to_string()
                        })?;
                    self.editor.set_field_value(&note_id, &field_id, value);
                }
                EngramAppEvent::NoteEditorTagsChange => {
                    let value = parsed
                        .text_value
                        .clone()
                        .ok_or_else(|| "onNoteEditorTagsChange is missing a value".to_string())?;
                    let selection = note_editor_selection(
                        &self.state,
                        &self.browser,
                        &self.editor,
                        None,
                        now,
                        Some(selected_deck_context.as_str()),
                    );
                    if selection.note_id.is_empty() {
                        return Err("cannot edit tags without a selected note".to_string());
                    }
                    self.editor.set_tags(&selection.note_id, value);
                }
                EngramAppEvent::NoteEditorSaveNote => {
                    let selection = note_editor_selection(
                        &self.state,
                        &self.browser,
                        &self.editor,
                        None,
                        now,
                        Some(selected_deck_context.as_str()),
                    );
                    let note =
                        note_from_editor_selection(&self.state, &selection, &self.editor, now)?;
                    self.state = reduce(
                        &self.state,
                        engram_core::EngramCommand::UpsertNote {
                            note,
                            materialize_cards_at: Some(now),
                        },
                    );
                    self.editor.reset();
                }
                EngramAppEvent::NoteEditorDeleteNote => {
                    if self.editor.draft_is_new {
                        self.editor.reset();
                    } else if !self.editor.confirm_delete {
                        self.editor.ask_delete_confirmation();
                    } else {
                        let selection = note_editor_selection(
                            &self.state,
                            &self.browser,
                            &self.editor,
                            None,
                            now,
                            Some(selected_deck_context.as_str()),
                        );
                        if selection.note_id.is_empty() {
                            return Err("cannot delete note without a selected note".to_string());
                        }
                        self.state = reduce(
                            &self.state,
                            engram_core::EngramCommand::DeleteNote {
                                note_id: selection.note_id,
                            },
                        );
                        self.editor.reset();
                    }
                }
                EngramAppEvent::NoteEditorCancel => {
                    self.editor.reset();
                }
                EngramAppEvent::NoteTypeEditorSelectNoteType => {
                    let value = parsed.number_value.ok_or_else(|| {
                        "onNoteTypeEditorSelectNoteType is missing an index".to_string()
                    })?;
                    let index = parse_nonnegative_index(value, "note type")?;
                    if !(self.note_type_editor.draft_is_new && index == self.state.note_types.len())
                    {
                        self.note_type_editor.select_index(index);
                    }
                }
                EngramAppEvent::NoteTypeEditorSelectField => {
                    let value = parsed.number_value.ok_or_else(|| {
                        "onNoteTypeEditorSelectField is missing an index".to_string()
                    })?;
                    self.note_type_editor
                        .set_selected_field_index(parse_nonnegative_index(
                            value,
                            "note type field",
                        )?);
                }
                EngramAppEvent::NoteTypeEditorSelectTemplate => {
                    let value = parsed.number_value.ok_or_else(|| {
                        "onNoteTypeEditorSelectTemplate is missing an index".to_string()
                    })?;
                    self.note_type_editor
                        .set_selected_template_index(parse_nonnegative_index(
                            value,
                            "note type template",
                        )?);
                }
                EngramAppEvent::NoteTypeEditorNameChange => {
                    let value = parsed.text_value.clone().ok_or_else(|| {
                        "onNoteTypeEditorNameChange is missing a value".to_string()
                    })?;
                    let note_type_id =
                        note_type_editor_selected_id(&self.state, &self.note_type_editor, now)
                            .ok_or_else(|| {
                                "cannot edit note type name without a selected note type"
                                    .to_string()
                            })?;
                    self.note_type_editor.set_name(&note_type_id, value);
                }
                EngramAppEvent::NoteTypeEditorFieldNameChange => {
                    let value = parsed.text_value.clone().ok_or_else(|| {
                        "onNoteTypeEditorFieldNameChange is missing a value".to_string()
                    })?;
                    let (note_type_id, field_id) = note_type_editor_selected_field_id(
                        &self.state,
                        &self.note_type_editor,
                        now,
                    )
                    .ok_or_else(|| {
                        "cannot edit note type field without a selected field".to_string()
                    })?;
                    self.note_type_editor
                        .set_field_name(&note_type_id, &field_id, value);
                }
                EngramAppEvent::NoteTypeEditorFieldRequiredChange => {
                    let required = parsed.bool_value.ok_or_else(|| {
                        "onNoteTypeEditorFieldRequiredChange is missing a checked value".to_string()
                    })?;
                    let (note_type_id, field_id) = note_type_editor_selected_field_id(
                        &self.state,
                        &self.note_type_editor,
                        now,
                    )
                    .ok_or_else(|| {
                        "cannot edit note type field without a selected field".to_string()
                    })?;
                    self.note_type_editor
                        .set_field_required(&note_type_id, &field_id, required);
                }
                EngramAppEvent::NoteTypeEditorTemplateNameChange => {
                    let value = parsed.text_value.clone().ok_or_else(|| {
                        "onNoteTypeEditorTemplateNameChange is missing a value".to_string()
                    })?;
                    let (note_type_id, template_id) = note_type_editor_selected_template_id(
                        &self.state,
                        &self.note_type_editor,
                        now,
                    )
                    .ok_or_else(|| {
                        "cannot edit note type template without a selected template".to_string()
                    })?;
                    self.note_type_editor
                        .set_template_name(&note_type_id, &template_id, value);
                }
                EngramAppEvent::NoteTypeEditorFrontTemplateChange => {
                    let value = parsed.text_value.clone().ok_or_else(|| {
                        "onNoteTypeEditorFrontTemplateChange is missing a value".to_string()
                    })?;
                    let (note_type_id, template_id) = note_type_editor_selected_template_id(
                        &self.state,
                        &self.note_type_editor,
                        now,
                    )
                    .ok_or_else(|| {
                        "cannot edit note type template without a selected template".to_string()
                    })?;
                    self.note_type_editor
                        .set_template_front(&note_type_id, &template_id, value);
                }
                EngramAppEvent::NoteTypeEditorBackTemplateChange => {
                    let value = parsed.text_value.clone().ok_or_else(|| {
                        "onNoteTypeEditorBackTemplateChange is missing a value".to_string()
                    })?;
                    let (note_type_id, template_id) = note_type_editor_selected_template_id(
                        &self.state,
                        &self.note_type_editor,
                        now,
                    )
                    .ok_or_else(|| {
                        "cannot edit note type template without a selected template".to_string()
                    })?;
                    self.note_type_editor
                        .set_template_back(&note_type_id, &template_id, value);
                }
                EngramAppEvent::NoteTypeEditorStylesheetChange => {
                    let value = parsed.text_value.clone().ok_or_else(|| {
                        "onNoteTypeEditorStylesheetChange is missing a value".to_string()
                    })?;
                    let note_type_id =
                        note_type_editor_selected_id(&self.state, &self.note_type_editor, now)
                            .ok_or_else(|| {
                                "cannot edit note type stylesheet without a selected note type"
                                    .to_string()
                            })?;
                    self.note_type_editor.set_stylesheet(&note_type_id, value);
                }
                EngramAppEvent::NoteTypeEditorNewNoteType => {
                    self.note_type_editor.start_new(now);
                }
                EngramAppEvent::NoteTypeEditorSaveNoteType => {
                    let note_type =
                        note_type_from_editor_selection(&self.state, &self.note_type_editor, now)?;
                    self.state = reduce(
                        &self.state,
                        engram_core::EngramCommand::UpsertNoteType {
                            note_type,
                            materialize_cards_at: Some(now),
                        },
                    );
                    self.note_type_editor.reset();
                }
                EngramAppEvent::NoteTypeEditorDeleteNoteType => {
                    let note_type_id =
                        note_type_editor_selected_id(&self.state, &self.note_type_editor, now)
                            .ok_or_else(|| {
                                "cannot delete note type without a selected note type".to_string()
                            })?;
                    if self.note_type_editor.draft_is_new {
                        self.note_type_editor.reset();
                    } else if !self.note_type_editor.confirm_delete {
                        self.note_type_editor.ask_delete_confirmation();
                    } else if self
                        .state
                        .note_types
                        .iter()
                        .any(|note_type| note_type.id == note_type_id)
                    {
                        self.state = reduce(
                            &self.state,
                            engram_core::EngramCommand::DeleteNoteType { note_type_id },
                        );
                        self.note_type_editor.reset();
                    }
                }
                EngramAppEvent::NoteTypeEditorCancel => {
                    self.note_type_editor.reset();
                }
                EngramAppEvent::SaveNote => {
                    let note = note_from_app_event(&parsed, &self.state, deck_id, now)?;
                    self.state = reduce(
                        &self.state,
                        engram_core::EngramCommand::UpsertNote {
                            note,
                            materialize_cards_at: Some(now),
                        },
                    );
                }
                EngramAppEvent::SaveNoteType => {
                    let note_type = note_type_from_app_event(&parsed, &self.state, now)?;
                    self.state = reduce(
                        &self.state,
                        engram_core::EngramCommand::UpsertNoteType {
                            note_type,
                            materialize_cards_at: Some(now),
                        },
                    );
                }
                EngramAppEvent::DeleteNote => {
                    if let Some(note_id) = explicit_note_id_from_app_event(&parsed, &self.state) {
                        self.state = reduce(
                            &self.state,
                            engram_core::EngramCommand::DeleteNote { note_id },
                        );
                    }
                }
                EngramAppEvent::DeleteNoteType => {
                    if let Some(note_type_id) = explicit_note_type_id_from_app_event(&parsed) {
                        self.state = reduce(
                            &self.state,
                            engram_core::EngramCommand::DeleteNoteType { note_type_id },
                        );
                    }
                }
                EngramAppEvent::AddNoteType => {
                    self.note_type_editor.start_new(now);
                    self.active_screen = EngramAppScreen::Options;
                }
                EngramAppEvent::AddNote => {
                    let selected_deck = self.selected_deck_id(deck_id);
                    let note_type_id = self
                        .state
                        .note_types
                        .first()
                        .map(|note_type| note_type.id.clone())
                        .unwrap_or_default();
                    self.editor.start_new(
                        unique_note_id(&self.state, now),
                        note_type_id,
                        selected_deck,
                        now,
                    );
                    self.active_screen = EngramAppScreen::Add;
                }
                EngramAppEvent::BrowserEditSelected => {
                    if let Some(card_id) = parsed.card_id.as_deref() {
                        if !select_browser_card_id(
                            &self.state,
                            &mut self.browser,
                            card_id,
                            now,
                            Some(selected_deck_context.as_str()),
                        ) {
                            self.browser.set_query(format!("cid:{card_id}"));
                        }
                    }
                    let selection = browser_selected_card_details(
                        &self.state,
                        &self.browser,
                        parsed.card_id.as_deref(),
                        now,
                        Some(selected_deck_context.as_str()),
                    );
                    if selection.note_id.is_empty() {
                        self.editor.reset();
                    } else {
                        self.editor.reset_for_note(&selection.note_id);
                        self.editor.set_selected_field_index(0);
                    }
                    self.active_screen = EngramAppScreen::Add;
                }
                EngramAppEvent::BrowserOpenSelected
                | EngramAppEvent::ImportAnki
                | EngramAppEvent::ExportAnki => {}
            }

            let host_intent = host_intent_for_event(
                &parsed,
                &self.state,
                deck_id,
                self.selected_deck_id.as_deref(),
                now,
                &self.browser,
            );
            let props = engram_app_props_for_state(
                &self.state,
                deck_id,
                self.selected_deck_id.as_deref(),
                now,
                self.active_screen,
                &self.browser,
                &self.review,
                &self.editor,
                &self.note_type_editor,
            );
            Ok(json!({
                "ok": true,
                "event": parsed.kind.canonical_name(),
                "hostIntent": host_intent,
                "state": self.state,
                "props": props,
            })
            .to_string())
        })
    }

    pub fn review_history(
        &self,
        deck_id: &str,
        reviewed_after: u64,
        reviewed_before: u64,
    ) -> String {
        catch_json(|| {
            let summary =
                summarize_review_history(&self.state, deck_id, reviewed_after, reviewed_before);
            Ok(ok_with("history", &summary))
        })
    }

    pub fn generated_cards(&self, note_type_id: &str, note_id: &str) -> String {
        catch_json(|| {
            let note_type = self
                .state
                .note_types
                .iter()
                .find(|note_type| note_type.id == note_type_id)
                .ok_or_else(|| format!("unknown note type: {note_type_id}"))?;
            let note = self
                .state
                .notes
                .iter()
                .find(|note| note.id == note_id)
                .ok_or_else(|| format!("unknown note: {note_id}"))?;
            let cards = generate_cards_for_note(note_type, note);
            Ok(ok_with("cards", &cards))
        })
    }

    pub fn materialized_cards(&self, note_type_id: &str, note_id: &str, created_at: u64) -> String {
        catch_json(|| {
            let note_type = self
                .state
                .note_types
                .iter()
                .find(|note_type| note_type.id == note_type_id)
                .ok_or_else(|| format!("unknown note type: {note_type_id}"))?;
            let note = self
                .state
                .notes
                .iter()
                .find(|note| note.id == note_id)
                .ok_or_else(|| format!("unknown note: {note_id}"))?;
            let cards: Vec<Card> = generate_cards_for_note(note_type, note)
                .iter()
                .map(|generated| materialize_generated_card(generated, created_at))
                .collect();
            Ok(ok_with("cards", &cards))
        })
    }

    pub fn search_cards(&self, query: &str, now: u64) -> String {
        catch_json(|| match search_core_cards(&self.state, query, now) {
            Ok(results) => Ok(ok_with("results", &results)),
            Err(error) => Ok(error_json_with_token(&error.message, &error.token)),
        })
    }

    pub fn export_cards_csv(&self, deck_id: &str) -> String {
        catch_json(|| {
            let cards = cards_in_deck_scope(&self.state, deck_id);
            Ok(ok_with("csv", &export_cards_csv(&cards)))
        })
    }

    pub fn export_anki_basic_tsv(
        &self,
        deck_id: &str,
        deck_name: &str,
        note_type_name: &str,
        html: bool,
    ) -> String {
        catch_json(|| {
            let cards = cards_in_deck_scope(&self.state, deck_id);
            let options = AnkiBasicTsvExportOptions {
                deck_name: deck_name.to_string(),
                note_type_name: note_type_name.to_string(),
                html,
                include_headers: true,
            };
            Ok(ok_with(
                "tsv",
                &export_cards_anki_basic_tsv(&cards, &options),
            ))
        })
    }

    pub fn export_anki_notes_tsv(
        &self,
        note_type_id: &str,
        deck_id: &str,
        deck_name: &str,
        note_type_name: &str,
        html: bool,
    ) -> String {
        catch_json(|| {
            let note_type = self
                .state
                .note_types
                .iter()
                .find(|note_type| note_type.id == note_type_id)
                .ok_or_else(|| format!("unknown note type: {note_type_id}"))?;
            let notes: Vec<_> = notes_in_deck_scope(&self.state, deck_id)
                .into_iter()
                .filter(|note| note.note_type_id == note_type_id)
                .collect();
            let options = AnkiBasicTsvExportOptions {
                deck_name: deck_name.to_string(),
                note_type_name: note_type_name.to_string(),
                html,
                include_headers: true,
            };
            Ok(ok_with(
                "tsv",
                &export_notes_anki_tsv_with_context(
                    note_type,
                    &notes,
                    &self.state.decks,
                    &self.state.external_sources,
                    &options,
                ),
            ))
        })
    }

    pub fn parse_cards_csv(&self, csv: &str) -> String {
        catch_json(|| match import_cards_csv(csv) {
            Ok(cards) => Ok(ok_with("cards", &cards)),
            Err(error) => Ok(error_json_with_row(&error.message, error.row)),
        })
    }

    pub fn parse_basic_cards_csv(
        &self,
        csv: &str,
        deck_id: &str,
        id_prefix: &str,
        created_at: u64,
    ) -> String {
        catch_json(|| {
            let options = BasicCardCsvImportOptions {
                deck_id: deck_id.to_string(),
                id_prefix: id_prefix.to_string(),
                created_at,
            };
            match import_basic_cards_csv(csv, &options) {
                Ok(cards) => Ok(ok_with("cards", &cards)),
                Err(error) => Ok(error_json_with_row(&error.message, error.row)),
            }
        })
    }

    pub fn parse_anki_basic_tsv(
        &self,
        tsv: &str,
        deck_id: &str,
        id_prefix: &str,
        created_at: u64,
    ) -> String {
        catch_json(|| {
            let options = BasicCardCsvImportOptions {
                deck_id: deck_id.to_string(),
                id_prefix: id_prefix.to_string(),
                created_at,
            };
            match import_anki_basic_tsv(tsv, &options) {
                Ok(cards) => Ok(ok_with("cards", &cards)),
                Err(error) => Ok(error_json_with_row(&error.message, error.row)),
            }
        })
    }

    pub fn parse_anki_notes_tsv(
        &self,
        tsv: &str,
        deck_id: &str,
        note_type_id: &str,
        note_type_name: &str,
        note_id_prefix: &str,
        created_at: u64,
    ) -> String {
        catch_json(|| {
            let options = AnkiNoteTsvImportOptions {
                deck_id: deck_id.to_string(),
                note_type_id: note_type_id.to_string(),
                note_type_name: note_type_name.to_string(),
                note_id_prefix: note_id_prefix.to_string(),
                created_at,
            };
            match import_anki_notes_tsv(tsv, &options) {
                Ok(imported) => Ok(ok_with("import", &imported)),
                Err(error) => Ok(error_json_with_row(&error.message, error.row)),
            }
        })
    }

    pub fn merge_anki_notes_tsv(
        &mut self,
        tsv: &str,
        deck_id: &str,
        note_type_id: &str,
        note_type_name: &str,
        note_id_prefix: &str,
        created_at: u64,
    ) -> String {
        catch_json(|| {
            let options = AnkiNoteTsvImportOptions {
                deck_id: deck_id.to_string(),
                note_type_id: note_type_id.to_string(),
                note_type_name: note_type_name.to_string(),
                note_id_prefix: note_id_prefix.to_string(),
                created_at,
            };
            match import_anki_notes_tsv(tsv, &options) {
                Ok(imported) => {
                    let imported_state = app_state_from_anki_note_tsv_import(imported);
                    self.state = merge_app_states(&self.state, imported_state);
                    self.browser = BrowserSessionState::default();
                    self.review = ReviewSessionState::default();
                    self.editor = NoteEditorSessionState::default();
                    self.note_type_editor = NoteTypeEditorSessionState::default();
                    Ok(ok_with("state", &self.state))
                }
                Err(error) => Ok(error_json_with_row(&error.message, error.row)),
            }
        })
    }

    pub fn export_anki_apkg(&self) -> String {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = self;
            return error_json("Anki APKG export is handled by native hosts for WASM shells");
        }

        #[cfg(not(target_arch = "wasm32"))]
        catch_json(
            || match write_legacy_apkg_from_engram_state(&self.state, &[]) {
                Ok(apkg) => Ok(ok_with("apkg", &apkg)),
                Err(error) => Ok(error_json(&error.message)),
            },
        )
    }

    pub fn merge_anki_apkg(&mut self, bytes: &[u8]) -> String {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = bytes;
            return error_json("Anki APKG import is handled by native hosts for WASM shells");
        }

        #[cfg(not(target_arch = "wasm32"))]
        catch_json(|| match read_v11_collection_as_engram_state(bytes) {
            Ok(imported_state) => {
                self.state = merge_app_states(&self.state, imported_state);
                self.browser = BrowserSessionState::default();
                self.review = ReviewSessionState::default();
                self.editor = NoteEditorSessionState::default();
                self.note_type_editor = NoteTypeEditorSessionState::default();
                Ok(ok_with("state", &self.state))
            }
            Err(error) => Ok(error_json(&error.message)),
        })
    }
}

fn app_state_from_anki_note_tsv_import(imported: AnkiNoteTsvImport) -> AppState {
    AppState {
        note_types: imported.note_types,
        notes: imported.notes,
        cards: imported.cards,
        external_sources: imported.external_sources,
        ..AppState::default()
    }
}

// Aggregates the whole app view-state into the props object handed to the
// renderer; the parameters mirror the discrete pieces of UI state (deck,
// selection, clock, screen, …) rather than being worth bundling into a struct.
#[allow(clippy::too_many_arguments)]
fn engram_app_props_for_state(
    state: &AppState,
    deck_id: &str,
    selected_deck_override: Option<&str>,
    now: u64,
    active_screen: EngramAppScreen,
    browser: &BrowserSessionState,
    review: &ReviewSessionState,
    editor: &NoteEditorSessionState,
    note_type_editor: &NoteTypeEditorSessionState,
) -> Value {
    let selected_deck_id = selected_deck_id_with_override(state, deck_id, selected_deck_override);
    let deck = state.decks.iter().find(|deck| deck.id == selected_deck_id);
    let deck_name = deck
        .map(|deck| deck.name.clone())
        .filter(|name| !name.is_empty())
        .or_else(|| (!selected_deck_id.is_empty()).then(|| selected_deck_id.clone()))
        .unwrap_or_else(|| "Deck".to_string());
    let stats = get_deck_stats_for_state(state, selected_deck_id.as_str(), now);
    let deck_options = deck_options_for_state(state, selected_deck_id.as_str());
    let review_history =
        summarize_review_history(state, selected_deck_id.as_str(), 0, now.saturating_add(1));
    let progress = get_active_session_progress(state);
    let active_card = state
        .active_session
        .as_ref()
        .and_then(|active| active.queue.get(active.current_index));
    let answer_visible = state
        .active_session
        .as_ref()
        .is_some_and(|active| active.revealed);
    let type_answer_spec = active_card.and_then(|card| active_type_answer_spec(state, card));
    let type_answer_value = active_card
        .map(|card| review.typed_answer_for_card(&card.id).to_string())
        .unwrap_or_default();
    let type_answer_correct = type_answer_spec
        .as_ref()
        .is_some_and(|spec| answer_visible && type_answer_matches(&type_answer_value, spec));
    let type_answer_expected = type_answer_spec
        .as_ref()
        .filter(|_| answer_visible)
        .map(|spec| spec.expected.clone())
        .unwrap_or_default();
    let type_answer_comparison = type_answer_spec
        .as_ref()
        .filter(|_| answer_visible)
        .map(|spec| format_type_answer_comparison(&type_answer_value, spec, type_answer_correct))
        .unwrap_or_default();
    let type_answer_placeholder = type_answer_spec
        .as_ref()
        .map(|spec| format!("Type {}", spec.field_name))
        .unwrap_or_else(|| "Type your answer".to_string());
    let type_answer_field = type_answer_spec
        .as_ref()
        .map(|spec| spec.field_name.clone())
        .unwrap_or_default();
    let type_answer_ignore_combining = type_answer_spec
        .as_ref()
        .is_some_and(|spec| spec.ignore_combining);
    let active_progress = active_card.and_then(|card| {
        state
            .card_progress
            .iter()
            .find(|progress| progress.card_id == card.id)
    });
    let mark_label = if active_progress
        .and_then(|progress| progress.marked_at)
        .is_some()
    {
        "Unmark"
    } else {
        "Mark"
    };
    let hidden_count = stats.suspended_count + stats.buried_count;
    let media_analysis = analyze_media_references(state);
    let deck_names = state
        .decks
        .iter()
        .map(|deck| {
            if deck.name.trim().is_empty() {
                deck.id.clone()
            } else {
                deck.name.clone()
            }
        })
        .collect::<Vec<_>>();
    let browser_props = engram_browser_props_for_state(
        state,
        browser.active_query(),
        &browser.effective_query(),
        browser.active_filter(),
        now,
        browser.selected_index,
        Some(selected_deck_id.as_str()),
        browser.filter_open,
        browser.flag_picker_open,
    )
    .unwrap_or_else(|_| {
        fallback_browser_props_for_state(
            state,
            browser.selected_index,
            browser.active_filter(),
            browser.filter_open,
            browser.flag_picker_open,
        )
    });
    let (current_value, remaining_value, correct_value, total_value, progress_label) =
        if let Some(progress) = &progress {
            (
                format!("{} / {}", progress.current_position, progress.total_cards),
                progress.remaining_cards.to_string(),
                progress.cards_correct.to_string(),
                progress.total_cards.to_string(),
                format!(
                    "Card {} of {}",
                    progress.current_position, progress.total_cards
                ),
            )
        } else {
            (
                "0 / 0".to_string(),
                "0".to_string(),
                "0".to_string(),
                "0".to_string(),
                "No active session".to_string(),
            )
        };

    let mut props = json!({
        "app-title": "Engram",
        "show-decks-screen": active_screen == EngramAppScreen::Decks,
        "show-study-screen": active_screen == EngramAppScreen::Study,
        "show-browse-screen": active_screen == EngramAppScreen::Browse,
        "show-add-screen": active_screen == EngramAppScreen::Add,
        "show-stats-screen": active_screen == EngramAppScreen::Stats,
        "show-options-screen": active_screen == EngramAppScreen::Options,
        "deck-name": deck_name,
        "deck-list-label": "Decks",
        "deck-names": deck_names,
        "deck-stats-label": "Deck stats",
        "deck-total-label": "Total",
        "deck-total-value": stats.total.to_string(),
        "deck-new-label": "New",
        "deck-new-value": stats.new_count.to_string(),
        "deck-due-label": "Due",
        "deck-due-value": stats.due_count.to_string(),
        "deck-learning-label": "Learning",
        "deck-learning-value": stats.learning_count.to_string(),
        "deck-hidden-label": "Hidden",
        "deck-hidden-value": hidden_count.to_string(),
        "prompt-label": "Prompt",
        "prompt": active_card.map(|card| card.front.as_str()).unwrap_or("No cards queued"),
        "answer-label": "Answer",
        "answer": active_card.map(|card| card.back.as_str()).unwrap_or_default(),
        "answer-visible": answer_visible,
        "progress-label": progress_label,
        "current-label": "Current",
        "current-value": current_value,
        "remaining-label": "Remaining",
        "remaining-value": remaining_value,
        "correct-label": "Correct",
        "correct-value": correct_value,
        "total-label": "Total",
        "total-value": total_value,
        "action-undo-label": "Undo",
        "action-bury-card-label": "Bury card",
        "action-bury-siblings-label": "Bury siblings",
        "action-suspend-card-label": "Suspend",
        "action-mark-label": mark_label,
    });

    let props_object = props
        .as_object_mut()
        .expect("Engram app props literal must be a JSON object");
    props_object.insert("host-status-visible".to_string(), Value::Bool(false));
    props_object.insert("host-status-kind".to_string(), Value::String(String::new()));
    props_object.insert(
        "host-status-label".to_string(),
        Value::String(String::new()),
    );
    props_object.insert(
        "host-status-message".to_string(),
        Value::String(String::new()),
    );
    props_object.insert(
        "type-answer-active".to_string(),
        Value::Bool(type_answer_spec.is_some()),
    );
    props_object.insert(
        "type-answer-label".to_string(),
        Value::String("Type answer".to_string()),
    );
    props_object.insert(
        "type-answer-value".to_string(),
        Value::String(type_answer_value),
    );
    props_object.insert(
        "type-answer-placeholder".to_string(),
        Value::String(type_answer_placeholder),
    );
    props_object.insert(
        "type-answer-field".to_string(),
        Value::String(type_answer_field),
    );
    props_object.insert(
        "type-answer-expected".to_string(),
        Value::String(type_answer_expected),
    );
    props_object.insert(
        "type-answer-comparison-label".to_string(),
        Value::String("Typed answer".to_string()),
    );
    props_object.insert(
        "type-answer-comparison-value".to_string(),
        Value::String(type_answer_comparison),
    );
    props_object.insert(
        "type-answer-correct".to_string(),
        Value::Bool(type_answer_correct),
    );
    props_object.insert(
        "type-answer-ignore-combining".to_string(),
        Value::Bool(type_answer_ignore_combining),
    );
    {
        let mut insert_prop = |key: &str, value: String| {
            props_object.insert(key.to_string(), Value::String(value));
        };
        insert_prop("collection-label", "Collection".to_string());
        insert_prop("collection-note-count-label", "Notes".to_string());
        insert_prop("collection-note-count-value", state.notes.len().to_string());
        insert_prop("collection-note-type-count-label", "Note types".to_string());
        insert_prop(
            "collection-note-type-count-value",
            state.note_types.len().to_string(),
        );
        insert_prop("collection-media-count-label", "Media".to_string());
        insert_prop(
            "collection-media-count-value",
            state.media_assets.len().to_string(),
        );
        insert_prop(
            "collection-referenced-media-label",
            "Referenced".to_string(),
        );
        insert_prop(
            "collection-referenced-media-value",
            media_analysis.referenced_filenames.len().to_string(),
        );
        insert_prop("collection-missing-media-label", "Missing".to_string());
        insert_prop(
            "collection-missing-media-value",
            media_analysis.missing_filenames.len().to_string(),
        );
        insert_prop("collection-unused-media-label", "Unused".to_string());
        insert_prop(
            "collection-unused-media-value",
            media_analysis.unreferenced_asset_ids.len().to_string(),
        );
        insert_prop(
            "collection-prune-unused-media-label",
            "Prune unused media".to_string(),
        );
        insert_prop("collection-import-label", "Import Anki".to_string());
        insert_prop("collection-export-label", "Export Anki".to_string());
        insert_prop("collection-add-note-label", "Add note".to_string());
        insert_prop(
            "collection-add-note-type-label",
            "Add note type".to_string(),
        );
        insert_prop("collection-delete-note-label", "Delete note".to_string());
        insert_prop(
            "collection-delete-note-type-label",
            "Delete note type".to_string(),
        );
    }
    props_object.insert(
        "collection-missing-media-filenames".to_string(),
        serde_json::to_value(&media_analysis.missing_filenames).unwrap_or(Value::Null),
    );
    props_object.insert(
        "collection-unused-media-asset-ids".to_string(),
        serde_json::to_value(&media_analysis.unreferenced_asset_ids).unwrap_or(Value::Null),
    );
    {
        props_object.insert(
            "deck-options-settings-label".to_string(),
            Value::String("Deck options".to_string()),
        );
        props_object.insert(
            "deck-options-learning-steps-label".to_string(),
            Value::String("Learning steps".to_string()),
        );
        props_object.insert(
            "deck-options-learning-steps-value".to_string(),
            Value::String(format_step_minutes(&deck_options.learning_steps_minutes)),
        );
        props_object.insert(
            "deck-options-relearning-steps-label".to_string(),
            Value::String("Relearning steps".to_string()),
        );
        props_object.insert(
            "deck-options-relearning-steps-value".to_string(),
            Value::String(format_step_minutes(&deck_options.relearning_steps_minutes)),
        );
        props_object.insert(
            "deck-options-new-cards-label".to_string(),
            Value::String("New cards/day".to_string()),
        );
        props_object.insert(
            "deck-options-new-cards-value".to_string(),
            Value::from(deck_options.new_cards_per_day),
        );
        props_object.insert(
            "deck-options-reviews-label".to_string(),
            Value::String("Reviews/day".to_string()),
        );
        props_object.insert(
            "deck-options-reviews-value".to_string(),
            Value::from(deck_options.reviews_per_day),
        );
        props_object.insert(
            "deck-options-graduating-interval-label".to_string(),
            Value::String("Graduating interval".to_string()),
        );
        props_object.insert(
            "deck-options-graduating-interval-value".to_string(),
            Value::from(deck_options.graduating_interval_days),
        );
        props_object.insert(
            "deck-options-easy-interval-label".to_string(),
            Value::String("Easy interval".to_string()),
        );
        props_object.insert(
            "deck-options-easy-interval-value".to_string(),
            Value::from(deck_options.easy_interval_days),
        );
        props_object.insert(
            "deck-options-initial-ease-label".to_string(),
            Value::String("Initial ease".to_string()),
        );
        props_object.insert(
            "deck-options-initial-ease-value".to_string(),
            Value::from(deck_options.initial_ease_factor),
        );
        props_object.insert(
            "deck-options-maximum-interval-label".to_string(),
            Value::String("Maximum interval".to_string()),
        );
        props_object.insert(
            "deck-options-maximum-interval-value".to_string(),
            Value::from(deck_options.maximum_interval_days),
        );
        props_object.insert(
            "deck-options-interval-modifier-label".to_string(),
            Value::String("Interval modifier".to_string()),
        );
        props_object.insert(
            "deck-options-interval-modifier-value".to_string(),
            Value::from(deck_options.review_interval_modifier),
        );
        props_object.insert(
            "deck-options-hard-multiplier-label".to_string(),
            Value::String("Hard multiplier".to_string()),
        );
        props_object.insert(
            "deck-options-hard-multiplier-value".to_string(),
            Value::from(deck_options.hard_interval_multiplier),
        );
        props_object.insert(
            "deck-options-easy-bonus-label".to_string(),
            Value::String("Easy bonus".to_string()),
        );
        props_object.insert(
            "deck-options-easy-bonus-value".to_string(),
            Value::from(deck_options.easy_bonus_multiplier),
        );
        props_object.insert(
            "deck-options-lapse-multiplier-label".to_string(),
            Value::String("Lapse multiplier".to_string()),
        );
        props_object.insert(
            "deck-options-lapse-multiplier-value".to_string(),
            Value::from(deck_options.lapse_interval_multiplier),
        );
        props_object.insert(
            "deck-options-leech-threshold-label".to_string(),
            Value::String("Leech threshold".to_string()),
        );
        props_object.insert(
            "deck-options-leech-threshold-value".to_string(),
            Value::from(deck_options.leech_threshold),
        );
        props_object.insert(
            "deck-options-desired-retention-label".to_string(),
            Value::String("Desired retention".to_string()),
        );
        props_object.insert(
            "deck-options-desired-retention-value".to_string(),
            Value::from(deck_options.desired_retention),
        );
        props_object.insert(
            "deck-options-fsrs-parameters-label".to_string(),
            Value::String("FSRS parameters".to_string()),
        );
        props_object.insert(
            "deck-options-fsrs-parameters-value".to_string(),
            Value::String(format_f64_list(&deck_options.fsrs_parameters)),
        );
        props_object.insert(
            "deck-options-fsrs-search-label".to_string(),
            Value::String("FSRS search".to_string()),
        );
        props_object.insert(
            "deck-options-fsrs-search-value".to_string(),
            Value::String(deck_options.fsrs_parameter_search.clone()),
        );
        props_object.insert(
            "deck-options-ignore-review-history-before-label".to_string(),
            Value::String("Ignore reviews before".to_string()),
        );
        props_object.insert(
            "deck-options-ignore-review-history-before-value".to_string(),
            Value::String(deck_options.ignore_review_history_before.clone()),
        );
        props_object.insert(
            "deck-options-historical-retention-label".to_string(),
            Value::String("Historical retention".to_string()),
        );
        props_object.insert(
            "deck-options-historical-retention-value".to_string(),
            Value::from(deck_options.historical_retention),
        );
        props_object.insert(
            "deck-options-easy-days-percentages-label".to_string(),
            Value::String("Easy day factors".to_string()),
        );
        props_object.insert(
            "deck-options-easy-days-percentages-value".to_string(),
            Value::String(format_f64_list(&deck_options.easy_days_percentages)),
        );
        props_object.insert(
            "deck-options-leech-action-label".to_string(),
            Value::String("Leech action".to_string()),
        );
        props_object.insert(
            "deck-options-leech-action-suspend-label".to_string(),
            Value::String("Suspend".to_string()),
        );
        props_object.insert(
            "deck-options-leech-action-suspend-value".to_string(),
            Value::Bool(deck_options.leech_action == LeechAction::Suspend),
        );
        props_object.insert(
            "deck-options-leech-action-tag-only-label".to_string(),
            Value::String("Tag only".to_string()),
        );
        props_object.insert(
            "deck-options-leech-action-tag-only-value".to_string(),
            Value::Bool(deck_options.leech_action == LeechAction::TagOnly),
        );
        props_object.insert(
            "deck-options-bury-new-siblings-label".to_string(),
            Value::String("Bury new siblings".to_string()),
        );
        props_object.insert(
            "deck-options-bury-new-siblings-value".to_string(),
            Value::Bool(deck_options.bury_new_siblings),
        );
        props_object.insert(
            "deck-options-bury-review-siblings-label".to_string(),
            Value::String("Bury review siblings".to_string()),
        );
        props_object.insert(
            "deck-options-bury-review-siblings-value".to_string(),
            Value::Bool(deck_options.bury_review_siblings),
        );
        props_object.insert(
            "deck-options-bury-interday-learning-siblings-label".to_string(),
            Value::String("Bury interday learning siblings".to_string()),
        );
        props_object.insert(
            "deck-options-bury-interday-learning-siblings-value".to_string(),
            Value::Bool(deck_options.bury_interday_learning_siblings),
        );
    }
    {
        props_object.insert(
            "history-label".to_string(),
            Value::String("Review history".to_string()),
        );
        props_object.insert(
            "history-window-label".to_string(),
            Value::String("Lifetime".to_string()),
        );
        props_object.insert(
            "history-total-label".to_string(),
            Value::String("Reviews".to_string()),
        );
        props_object.insert(
            "history-total-value".to_string(),
            Value::String(review_history.total_reviews.to_string()),
        );
        props_object.insert(
            "history-correct-label".to_string(),
            Value::String("Correct".to_string()),
        );
        props_object.insert(
            "history-correct-value".to_string(),
            Value::String(review_history.correct_reviews.to_string()),
        );
        props_object.insert(
            "history-unique-label".to_string(),
            Value::String("Cards".to_string()),
        );
        props_object.insert(
            "history-unique-value".to_string(),
            Value::String(review_history.unique_cards.to_string()),
        );
        props_object.insert(
            "history-accuracy-label".to_string(),
            Value::String("Accuracy".to_string()),
        );
        props_object.insert(
            "history-accuracy-value".to_string(),
            Value::String(format_review_accuracy(
                review_history.correct_reviews,
                review_history.total_reviews,
            )),
        );
        props_object.insert(
            "history-again-label".to_string(),
            Value::String("Again".to_string()),
        );
        props_object.insert(
            "history-again-value".to_string(),
            Value::String(review_history.rating_counts.again.to_string()),
        );
        props_object.insert(
            "history-hard-label".to_string(),
            Value::String("Hard".to_string()),
        );
        props_object.insert(
            "history-hard-value".to_string(),
            Value::String(review_history.rating_counts.hard.to_string()),
        );
        props_object.insert(
            "history-good-label".to_string(),
            Value::String("Good".to_string()),
        );
        props_object.insert(
            "history-good-value".to_string(),
            Value::String(review_history.rating_counts.good.to_string()),
        );
        props_object.insert(
            "history-easy-label".to_string(),
            Value::String("Easy".to_string()),
        );
        props_object.insert(
            "history-easy-value".to_string(),
            Value::String(review_history.rating_counts.easy.to_string()),
        );
        props_object.insert(
            "history-first-label".to_string(),
            Value::String("First".to_string()),
        );
        props_object.insert(
            "history-first-value".to_string(),
            Value::String(format_optional_timestamp(review_history.first_reviewed_at)),
        );
        props_object.insert(
            "history-last-label".to_string(),
            Value::String("Last".to_string()),
        );
        props_object.insert(
            "history-last-value".to_string(),
            Value::String(format_optional_timestamp(review_history.last_reviewed_at)),
        );
    }
    if let Some(browser_object) = browser_props.as_object() {
        for (key, value) in browser_object {
            props_object.insert(key.clone(), value.clone());
        }
    }
    props_object.insert(
        "browser-tag-edit-label".to_string(),
        Value::String("Tags".to_string()),
    );
    props_object.insert(
        "browser-tag-edit".to_string(),
        Value::String(browser.tag_edit.clone()),
    );
    props_object.insert(
        "browser-tag-edit-placeholder".to_string(),
        Value::String("grammar script".to_string()),
    );
    props_object.insert(
        "browser-add-tag-label".to_string(),
        Value::String("Add tag".to_string()),
    );
    props_object.insert(
        "browser-remove-tag-label".to_string(),
        Value::String("Remove tag".to_string()),
    );
    props_object.insert(
        "browser-custom-study-label".to_string(),
        Value::String("Custom study".to_string()),
    );
    props_object.insert(
        "browser-custom-study-limit-label".to_string(),
        Value::String("Cards".to_string()),
    );
    props_object.insert(
        "browser-custom-study-limit-value".to_string(),
        Value::from(browser.custom_study_limit as u64),
    );
    props_object.insert(
        "browser-custom-study-reschedule-label".to_string(),
        Value::String("Reschedule reviews".to_string()),
    );
    props_object.insert(
        "browser-custom-study-reschedule-value".to_string(),
        Value::Bool(browser.custom_study_reschedule),
    );
    props_object.insert(
        "browser-custom-study-rebuild-label".to_string(),
        Value::String("Rebuild".to_string()),
    );
    props_object.insert(
        "browser-custom-study-empty-label".to_string(),
        Value::String("Empty".to_string()),
    );
    insert_note_editor_props(
        props_object,
        state,
        browser,
        editor,
        now,
        Some(selected_deck_id.as_str()),
    );
    insert_note_type_editor_props(props_object, state, note_type_editor, now);

    props
}

fn format_step_minutes(steps: &[u32]) -> String {
    steps
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_f64_list(values: &[f64]) -> String {
    values
        .iter()
        .filter(|value| value.is_finite())
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_review_accuracy(correct_reviews: usize, total_reviews: usize) -> String {
    if total_reviews == 0 {
        return "0%".to_string();
    }
    let percentage = (correct_reviews as f64 / total_reviews as f64) * 100.0;
    format!("{}%", percentage.round() as u32)
}

fn format_optional_timestamp(timestamp: Option<u64>) -> String {
    timestamp
        .map(|value| value.to_string())
        .unwrap_or_else(|| "Never".to_string())
}

fn index_of_id<T>(items: &[T], selected_id: &str, id_for: impl Fn(&T) -> &str) -> Option<usize> {
    if selected_id.is_empty() {
        return None;
    }
    items.iter().position(|item| id_for(item) == selected_id)
}

fn insert_note_editor_props(
    props: &mut serde_json::Map<String, Value>,
    state: &AppState,
    browser: &BrowserSessionState,
    editor: &NoteEditorSessionState,
    now: u64,
    current_deck_id: Option<&str>,
) {
    let selection = note_editor_selection(state, browser, editor, None, now, current_deck_id);
    let note_type_names = state
        .note_types
        .iter()
        .map(|note_type| note_type.name.clone())
        .collect::<Vec<_>>();
    let deck_names = state
        .decks
        .iter()
        .map(|deck| {
            if deck.name.trim().is_empty() {
                deck.id.clone()
            } else {
                deck.name.clone()
            }
        })
        .collect::<Vec<_>>();
    let selected_note_type_index = index_of_id(
        &state.note_types,
        selection.note_type_id.as_str(),
        |note_type| note_type.id.as_str(),
    );
    let selected_deck_index = index_of_id(&state.decks, selection.deck_id.as_str(), |deck| {
        deck.id.as_str()
    });
    let field_labels = selection
        .fields
        .iter()
        .map(|field| {
            if field.required {
                format!("{} *", field.name)
            } else {
                field.name.clone()
            }
        })
        .collect::<Vec<_>>();
    let selected_field_index = note_editor_selected_field_index(&selection, editor);
    let selected_field = selected_field_index.and_then(|index| selection.fields.get(index));
    let draft_active = editor.draft_note_id.as_deref() == Some(selection.note_id.as_str());
    let selected_field_value = selected_field
        .map(|field| {
            if draft_active {
                editor
                    .draft_fields
                    .get(field.id.as_str())
                    .cloned()
                    .unwrap_or_else(|| field.value.clone())
            } else {
                field.value.clone()
            }
        })
        .unwrap_or_default();
    let tags_value = if draft_active {
        editor
            .draft_tags
            .clone()
            .unwrap_or_else(|| selection.note_tags.join(" "))
    } else {
        selection.note_tags.join(" ")
    };

    props.insert(
        "note-editor-label".to_string(),
        Value::String(if editor.draft_is_new {
            "Add note".to_string()
        } else {
            "Note editor".to_string()
        }),
    );
    props.insert(
        "note-editor-note-id-label".to_string(),
        Value::String("Note".to_string()),
    );
    props.insert(
        "note-editor-note-id-value".to_string(),
        Value::String(selection.note_id.clone()),
    );
    props.insert(
        "note-editor-note-type-label".to_string(),
        Value::String("Type".to_string()),
    );
    props.insert(
        "note-editor-note-type-value".to_string(),
        Value::String(selection.note_type_name.clone()),
    );
    props.insert(
        "note-editor-note-type-options-label".to_string(),
        Value::String("Note types".to_string()),
    );
    props.insert(
        "note-editor-note-type-names".to_string(),
        json!(note_type_names),
    );
    props.insert(
        "note-editor-selected-note-type-index".to_string(),
        selected_note_type_index.map_or(Value::from(-1), |index| Value::from(index as i64)),
    );
    props.insert(
        "note-editor-deck-label".to_string(),
        Value::String("Deck".to_string()),
    );
    props.insert(
        "note-editor-deck-value".to_string(),
        Value::String(selection.deck_name.clone()),
    );
    props.insert(
        "note-editor-deck-options-label".to_string(),
        Value::String("Decks".to_string()),
    );
    props.insert("note-editor-deck-names".to_string(), json!(deck_names));
    props.insert(
        "note-editor-selected-deck-index".to_string(),
        selected_deck_index.map_or(Value::from(-1), |index| Value::from(index as i64)),
    );
    props.insert(
        "note-editor-fields-label".to_string(),
        Value::String("Fields".to_string()),
    );
    props.insert("note-editor-field-labels".to_string(), json!(field_labels));
    props.insert(
        "note-editor-selected-field-index".to_string(),
        selected_field_index.map_or(Value::from(-1), |index| Value::from(index as i64)),
    );
    props.insert(
        "note-editor-selected-field-label".to_string(),
        Value::String(
            selected_field
                .map(|field| field.name.clone())
                .unwrap_or_default(),
        ),
    );
    props.insert(
        "note-editor-selected-field-value".to_string(),
        Value::String(selected_field_value),
    );
    props.insert(
        "note-editor-selected-field-placeholder".to_string(),
        Value::String(
            selected_field
                .map(|field| field.name.clone())
                .unwrap_or_else(|| "Select a field".to_string()),
        ),
    );
    props.insert(
        "note-editor-tags-label".to_string(),
        Value::String("Tags".to_string()),
    );
    props.insert(
        "note-editor-tags-value".to_string(),
        Value::String(tags_value),
    );
    props.insert(
        "note-editor-tags-placeholder".to_string(),
        Value::String("grammar script".to_string()),
    );
    props.insert(
        "note-editor-save-label".to_string(),
        Value::String("Save note".to_string()),
    );
    props.insert(
        "note-editor-delete-label".to_string(),
        Value::String(if editor.draft_is_new {
            "Discard draft".to_string()
        } else if editor.confirm_delete {
            "Confirm delete".to_string()
        } else {
            "Delete note".to_string()
        }),
    );
    props.insert(
        "note-editor-cancel-label".to_string(),
        Value::String("Cancel".to_string()),
    );
}

fn insert_note_type_editor_props(
    props: &mut serde_json::Map<String, Value>,
    state: &AppState,
    editor: &NoteTypeEditorSessionState,
    now: u64,
) {
    let mut note_type_names = state
        .note_types
        .iter()
        .map(|note_type| note_type.name.clone())
        .collect::<Vec<_>>();
    if editor.draft_is_new {
        note_type_names.push(format!(
            "{} (new)",
            editor
                .draft_name
                .as_deref()
                .filter(|name| !name.trim().is_empty())
                .unwrap_or("Basic")
        ));
    }

    let selected_index = note_type_editor_selected_index(state, editor);
    let selected_note_type = note_type_editor_selected_note_type(state, editor, now);
    let selected_field_index = selected_note_type
        .as_ref()
        .and_then(|note_type| note_type_editor_selected_field_index(note_type, editor));
    let selected_field = selected_note_type
        .as_ref()
        .and_then(|note_type| selected_field_index.and_then(|index| note_type.fields.get(index)));
    let selected_template_index = selected_note_type
        .as_ref()
        .and_then(|note_type| note_type_editor_selected_template_index(note_type, editor));
    let selected_template = selected_note_type.as_ref().and_then(|note_type| {
        selected_template_index.and_then(|index| note_type.templates.get(index))
    });
    let field_labels = selected_note_type
        .as_ref()
        .map(|note_type| {
            note_type
                .fields
                .iter()
                .map(|field| {
                    let required = if field.required { " *" } else { "" };
                    format!("{} {}{}", field.ordinal + 1, field.name, required)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let template_labels = selected_note_type
        .as_ref()
        .map(|note_type| {
            note_type
                .templates
                .iter()
                .map(|template| format!("{} {}", template.ordinal + 1, template.name))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    props.insert(
        "note-type-editor-label".to_string(),
        Value::String("Note type editor".to_string()),
    );
    props.insert(
        "note-type-editor-note-types-label".to_string(),
        Value::String("Note types".to_string()),
    );
    props.insert(
        "note-type-editor-note-type-names".to_string(),
        json!(note_type_names),
    );
    props.insert(
        "note-type-editor-selected-note-type-index".to_string(),
        selected_index.map_or(Value::from(-1), |index| Value::from(index as i64)),
    );
    props.insert(
        "note-type-editor-note-type-id-label".to_string(),
        Value::String("Model".to_string()),
    );
    props.insert(
        "note-type-editor-note-type-id-value".to_string(),
        Value::String(
            selected_note_type
                .as_ref()
                .map(|note_type| note_type.id.clone())
                .unwrap_or_default(),
        ),
    );
    props.insert(
        "note-type-editor-name-label".to_string(),
        Value::String("Name".to_string()),
    );
    props.insert(
        "note-type-editor-name-value".to_string(),
        Value::String(
            selected_note_type
                .as_ref()
                .map(|note_type| note_type.name.clone())
                .unwrap_or_default(),
        ),
    );
    props.insert(
        "note-type-editor-name-placeholder".to_string(),
        Value::String("Basic".to_string()),
    );
    props.insert(
        "note-type-editor-fields-label".to_string(),
        Value::String("Fields".to_string()),
    );
    props.insert(
        "note-type-editor-field-labels".to_string(),
        json!(field_labels),
    );
    props.insert(
        "note-type-editor-selected-field-index".to_string(),
        selected_field_index.map_or(Value::from(-1), |index| Value::from(index as i64)),
    );
    props.insert(
        "note-type-editor-field-name-label".to_string(),
        Value::String("Field name".to_string()),
    );
    props.insert(
        "note-type-editor-field-name-value".to_string(),
        Value::String(
            selected_field
                .map(|field| field.name.clone())
                .unwrap_or_default(),
        ),
    );
    props.insert(
        "note-type-editor-field-name-placeholder".to_string(),
        Value::String("Front".to_string()),
    );
    props.insert(
        "note-type-editor-field-required-label".to_string(),
        Value::String("Required".to_string()),
    );
    props.insert(
        "note-type-editor-field-required-value".to_string(),
        Value::Bool(selected_field.is_some_and(|field| field.required)),
    );
    props.insert(
        "note-type-editor-templates-label".to_string(),
        Value::String("Templates".to_string()),
    );
    props.insert(
        "note-type-editor-template-labels".to_string(),
        json!(template_labels),
    );
    props.insert(
        "note-type-editor-selected-template-index".to_string(),
        json!(selected_template_index.unwrap_or(0)),
    );
    props.insert(
        "note-type-editor-template-name-label".to_string(),
        Value::String("Template name".to_string()),
    );
    props.insert(
        "note-type-editor-template-name-value".to_string(),
        Value::String(
            selected_template
                .map(|template| template.name.clone())
                .unwrap_or_default(),
        ),
    );
    props.insert(
        "note-type-editor-template-name-placeholder".to_string(),
        Value::String("Forward".to_string()),
    );
    props.insert(
        "note-type-editor-front-template-label".to_string(),
        Value::String("Front template".to_string()),
    );
    props.insert(
        "note-type-editor-front-template-value".to_string(),
        Value::String(
            selected_template
                .map(|template| template.front_template.clone())
                .unwrap_or_default(),
        ),
    );
    props.insert(
        "note-type-editor-front-template-placeholder".to_string(),
        Value::String("{{Front}}".to_string()),
    );
    props.insert(
        "note-type-editor-back-template-label".to_string(),
        Value::String("Back template".to_string()),
    );
    props.insert(
        "note-type-editor-back-template-value".to_string(),
        Value::String(
            selected_template
                .map(|template| template.back_template.clone())
                .unwrap_or_default(),
        ),
    );
    props.insert(
        "note-type-editor-back-template-placeholder".to_string(),
        Value::String("{{Back}}".to_string()),
    );
    props.insert(
        "note-type-editor-stylesheet-label".to_string(),
        Value::String("Card style".to_string()),
    );
    props.insert(
        "note-type-editor-stylesheet-value".to_string(),
        Value::String(
            selected_note_type
                .as_ref()
                .and_then(|note_type| note_type.stylesheet.clone())
                .unwrap_or_default(),
        ),
    );
    props.insert(
        "note-type-editor-stylesheet-placeholder".to_string(),
        Value::String(".card { font-family: sans-serif; }".to_string()),
    );
    props.insert(
        "note-type-editor-new-label".to_string(),
        Value::String("New type".to_string()),
    );
    props.insert(
        "note-type-editor-save-label".to_string(),
        Value::String("Save type".to_string()),
    );
    props.insert(
        "note-type-editor-delete-label".to_string(),
        Value::String(if editor.draft_is_new {
            "Discard type".to_string()
        } else if editor.confirm_delete {
            "Confirm delete".to_string()
        } else {
            "Delete type".to_string()
        }),
    );
    props.insert(
        "note-type-editor-cancel-label".to_string(),
        Value::String("Cancel".to_string()),
    );
}

// As with engram_app_props_for_state, this flattens the browser view-state
// (queries, filter, …) into a props object; the parameter list mirrors that
// state directly.
#[allow(clippy::too_many_arguments)]
fn engram_browser_props_for_state(
    state: &AppState,
    display_query: &str,
    effective_query: &str,
    filter: &str,
    now: u64,
    selected_index: usize,
    current_deck_id: Option<&str>,
    filter_open: bool,
    flag_picker_open: bool,
) -> Result<Value, engram_core::SearchError> {
    let query = normalize_browser_query(display_query);
    let effective_query = normalize_browser_query(effective_query);
    let results = if current_deck_id.is_some() {
        search_cards_with_context(
            state,
            &effective_query,
            now,
            SearchContext {
                current_deck_id,
                ..SearchContext::default()
            },
        )?
    } else {
        search_core_cards(state, &effective_query, now)?
    };
    let card_sources_by_id = browser_card_sources_by_id(state);
    let collection_created_at_days = browser_collection_created_at_days(state);
    let rows = results
        .iter()
        .take(20)
        .map(|result| {
            BrowserRow::from_search_result(
                result,
                now,
                card_sources_by_id
                    .get(result.card.id.as_str())
                    .map_or(&[] as &[&ExternalSourceRecord], Vec::as_slice),
                collection_created_at_days,
            )
        })
        .collect::<Vec<_>>();

    Ok(browser_props_from_rows(
        query,
        filter,
        rows,
        results.len(),
        selected_index,
        filter_open,
        flag_picker_open,
    ))
}

fn fallback_browser_props_for_state(
    state: &AppState,
    selected_index: usize,
    filter: &str,
    filter_open: bool,
    flag_picker_open: bool,
) -> Value {
    let card_sources_by_id = browser_card_sources_by_id(state);
    let collection_created_at_days = browser_collection_created_at_days(state);
    let rows = state
        .cards
        .iter()
        .take(20)
        .map(|card| {
            BrowserRow::from_card(
                card,
                None,
                0,
                card_sources_by_id
                    .get(card.id.as_str())
                    .map_or(&[] as &[&ExternalSourceRecord], Vec::as_slice),
                collection_created_at_days,
            )
        })
        .collect::<Vec<_>>();
    browser_props_from_rows(
        DEFAULT_BROWSER_QUERY.to_string(),
        filter,
        rows,
        state.cards.len(),
        selected_index,
        filter_open,
        flag_picker_open,
    )
}

fn normalize_browser_query(query: &str) -> String {
    let query = query.trim();
    if query.is_empty() {
        DEFAULT_BROWSER_QUERY.to_string()
    } else {
        query.to_string()
    }
}

fn normalize_browser_filter_label(value: &str) -> &'static str {
    match value.trim().to_ascii_lowercase().as_str() {
        "new" | "is:new" => "New",
        "due" | "is:due" => "Due",
        "learn" | "learning" | "is:learn" | "is:learning" => "Learning",
        "review" | "is:review" => "Review",
        "suspended" | "suspend" | "is:suspended" => "Suspended",
        "buried" | "bury" | "is:buried" => "Buried",
        _ => BROWSER_FILTER_ALL,
    }
}

fn browser_filter_clause(filter: &str) -> Option<&'static str> {
    match normalize_browser_filter_label(filter) {
        "New" => Some("is:new"),
        "Due" => Some("is:due"),
        "Learning" => Some("is:learn"),
        "Review" => Some("is:review"),
        "Suspended" => Some("is:suspended"),
        "Buried" => Some("is:buried"),
        _ => None,
    }
}

fn compose_browser_filter_query(query: &str, filter: &str) -> String {
    let query = query.trim();
    let Some(clause) = browser_filter_clause(filter) else {
        return normalize_browser_query(query);
    };
    if query.is_empty() {
        clause.to_string()
    } else {
        format!("({query}) {clause}")
    }
}

fn browser_props_from_rows(
    query: String,
    filter: &str,
    rows: Vec<BrowserRow>,
    total_results: usize,
    requested_selected_index: usize,
    filter_open: bool,
    flag_picker_open: bool,
) -> Value {
    let visible = rows.len();
    let summary = match total_results {
        0 => "No matching cards".to_string(),
        1 if visible == 1 => "1 matching card".to_string(),
        total if visible == total => format!("{total} matching cards"),
        total => format!("Showing {visible} of {total} matching cards"),
    };
    let selected_index = if rows.is_empty() {
        -1
    } else {
        requested_selected_index.min(rows.len() - 1) as i64
    };
    let selected_row = if selected_index >= 0 {
        rows.get(selected_index as usize)
    } else {
        None
    };
    let labels = rows.iter().map(|row| row.label.clone()).collect::<Vec<_>>();
    let card_ids = rows
        .iter()
        .map(|row| row.card_id.clone())
        .collect::<Vec<_>>();
    let note_ids = rows
        .iter()
        .map(|row| row.note_id.clone())
        .collect::<Vec<_>>();
    let template_ids = rows
        .iter()
        .map(|row| row.template_id.clone())
        .collect::<Vec<_>>();
    let states = rows.iter().map(|row| row.state.clone()).collect::<Vec<_>>();
    let flags = rows.iter().map(|row| row.flag.clone()).collect::<Vec<_>>();

    json!({
        "browser-label": "Card browser",
        "browser-query-label": "Search",
        "browser-query": query,
        "browser-query-placeholder": "deck:tamil tag:script is:due",
        "browser-filter-label": "State",
        "browser-filter-value": normalize_browser_filter_label(filter),
        "browser-filter-options": BROWSER_FILTER_OPTIONS,
        "browser-filter-placeholder": BROWSER_FILTER_ALL,
        "browser-filter-open": filter_open,
        "browser-search-label": "Search",
        "browser-results-label": "Results",
        "browser-results-summary": summary,
        "browser-results": labels,
        "browser-result-card-ids": card_ids,
        "browser-result-note-ids": note_ids,
        "browser-result-template-ids": template_ids,
        "browser-result-states": states,
        "browser-result-flags": flags,
        "browser-selected-index": selected_index,
        "browser-selected-card-id": selected_row.map_or("", |row| row.card_id.as_str()),
        "browser-selected-note-id": selected_row.map_or("", |row| row.note_id.as_str()),
        "browser-selected-template-id": selected_row.map_or("", |row| row.template_id.as_str()),
        "browser-selected-state": selected_row.map_or("", |row| row.state.as_str()),
        "browser-selected-flag": selected_row.map_or("none", |row| row.flag.as_str()),
        "browser-open-label": "Open",
        "browser-edit-label": "Edit",
        "browser-suspend-label": "Suspend",
        "browser-mark-label": "Mark",
        "browser-flag-label": "Flag",
        "browser-flag-value": selected_row.map_or("none", |row| row.flag.as_str()),
        "browser-flag-options": ["none", "red", "orange", "green", "blue", "pink", "turquoise", "purple"],
        "browser-flag-placeholder": "none",
        "browser-flag-open": flag_picker_open,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BrowserRow {
    label: String,
    card_id: String,
    note_id: String,
    template_id: String,
    state: String,
    flag: String,
}

impl BrowserRow {
    fn from_search_result(
        result: &CardSearchResult,
        now: u64,
        card_sources: &[&ExternalSourceRecord],
        collection_created_at_days: Option<i64>,
    ) -> Self {
        Self::from_card(
            &result.card,
            result.progress.as_ref(),
            now,
            card_sources,
            collection_created_at_days,
        )
    }

    fn from_card(
        card: &Card,
        progress: Option<&CardProgress>,
        now: u64,
        card_sources: &[&ExternalSourceRecord],
        collection_created_at_days: Option<i64>,
    ) -> Self {
        let lineage = card.lineage.as_ref();
        let fallback_lineage = card.id.split_once("::");
        Self {
            label: format_browser_card_row(card),
            card_id: card.id.clone(),
            note_id: lineage
                .map(|lineage| lineage.note_id.clone())
                .or_else(|| fallback_lineage.map(|(note_id, _)| note_id.to_string()))
                .unwrap_or_default(),
            template_id: lineage
                .map(|lineage| lineage.template_id.clone())
                .or_else(|| fallback_lineage.map(|(_, template_id)| template_id.to_string()))
                .unwrap_or_default(),
            state: browser_card_state(progress, card_sources, collection_created_at_days, now)
                .to_string(),
            flag: browser_card_flag(progress, card_sources).to_string(),
        }
    }
}

fn format_browser_card_row(card: &Card) -> String {
    format!("{} -> {}", card.front, card.back)
}

fn browser_card_state(
    progress: Option<&CardProgress>,
    card_sources: &[&ExternalSourceRecord],
    collection_created_at_days: Option<i64>,
    now: u64,
) -> &'static str {
    if progress.is_none_or(browser_progress_is_new_overlay) {
        if let Some(state) =
            imported_anki_browser_card_state(card_sources, collection_created_at_days, now)
        {
            return state;
        }
    }

    let Some(progress) = progress else {
        return "new";
    };
    if progress.suspended_at.is_some() || progress.state == CardState::Suspended {
        return "suspended";
    }
    if progress.buried_until.is_some_and(|until| until > now) || progress.state == CardState::Buried
    {
        return "buried";
    }
    match progress.state {
        CardState::Learning => "learning",
        CardState::Review if progress.next_due_at <= now => "due",
        CardState::Review => "review",
        CardState::Relearning => "relearning",
        CardState::Suspended => "suspended",
        CardState::Buried => "buried",
    }
}

fn browser_progress_is_new_overlay(progress: &CardProgress) -> bool {
    progress.state == CardState::Review
        && progress.interval == 0
        && progress.learning_step_index.is_none()
        && progress.buried_until.is_none()
        && progress.suspended_at.is_none()
        && progress.times_seen == 0
        && progress.times_correct == 0
        && progress.times_incorrect == 0
}

fn browser_card_flag(
    progress: Option<&CardProgress>,
    card_sources: &[&ExternalSourceRecord],
) -> &'static str {
    imported_anki_browser_card_flag(card_sources)
        .or_else(|| {
            progress
                .and_then(|progress| progress.flag)
                .map(card_flag_label)
        })
        .unwrap_or("none")
}

fn browser_card_sources_by_id(
    state: &AppState,
) -> HashMap<&str, Vec<&ExternalSourceRecord>> {
    let mut sources_by_id: HashMap<&str, Vec<&ExternalSourceRecord>> = HashMap::new();
    for source in &state.external_sources {
        if source.target == ExternalSourceTarget::Card && source.source == "anki-v11" {
            sources_by_id
                .entry(source.target_id.as_str())
                .or_default()
                .push(source);
        }
    }
    sources_by_id
}

fn browser_collection_created_at_days(state: &AppState) -> Option<i64> {
    state
        .external_sources
        .iter()
        .find(|source| source.target == ExternalSourceTarget::Collection)
        .and_then(|source| browser_source_i64(source, "createdAtDays"))
}

fn imported_anki_browser_card_state(
    card_sources: &[&ExternalSourceRecord],
    collection_created_at_days: Option<i64>,
    now: u64,
) -> Option<&'static str> {
    card_sources.iter().find_map(|source| {
        let kind = browser_source_i64(source, "kind")?;
        let queue = browser_source_i64(source, "queue")?;
        if queue == ANKI_QUEUE_SUSPENDED {
            return Some("suspended");
        }
        if matches!(queue, ANKI_QUEUE_USER_BURIED | ANKI_QUEUE_SCHED_BURIED) {
            return Some("buried");
        }
        if kind == ANKI_TYPE_NEW && queue == ANKI_QUEUE_NEW {
            return Some("new");
        }

        match kind {
            ANKI_TYPE_LEARN => Some("learning"),
            ANKI_TYPE_RELEARN => Some("relearning"),
            ANKI_TYPE_REVIEW => {
                if imported_anki_browser_review_is_due(source, collection_created_at_days, now)
                    .unwrap_or(false)
                {
                    Some("due")
                } else {
                    Some("review")
                }
            }
            _ => None,
        }
    })
}

fn imported_anki_browser_review_is_due(
    source: &ExternalSourceRecord,
    collection_created_at_days: Option<i64>,
    now: u64,
) -> Option<bool> {
    if browser_source_i64(source, "queue")? != ANKI_QUEUE_REVIEW {
        return None;
    }
    let due = browser_source_i64(source, "originalDue")
        .filter(|due| *due != 0)
        .or_else(|| browser_source_i64(source, "due"))?;
    let today = i64::try_from(now / engram_core::ONE_DAY_MS)
        .ok()?
        .saturating_sub(collection_created_at_days?);
    Some(due <= today)
}

fn imported_anki_browser_card_flag(card_sources: &[&ExternalSourceRecord]) -> Option<&'static str> {
    card_sources
        .iter()
        .find_map(|source| browser_source_i64(source, "flags").map(anki_browser_card_flag_label))
}

fn anki_browser_card_flag_label(flags: i64) -> &'static str {
    match flags & 0b111 {
        1 => "red",
        2 => "orange",
        3 => "green",
        4 => "blue",
        5 => "pink",
        6 => "turquoise",
        7 => "purple",
        _ => "none",
    }
}

fn browser_source_i64(source: &ExternalSourceRecord, key: &str) -> Option<i64> {
    source.data.get(key)?.parse().ok()
}

fn card_flag_label(flag: CardFlag) -> &'static str {
    match flag {
        CardFlag::Red => "red",
        CardFlag::Orange => "orange",
        CardFlag::Green => "green",
        CardFlag::Blue => "blue",
        CardFlag::Pink => "pink",
        CardFlag::Turquoise => "turquoise",
        CardFlag::Purple => "purple",
    }
}

fn selected_deck_id_with_override(
    state: &AppState,
    deck_id: &str,
    selected_deck_override: Option<&str>,
) -> String {
    if !deck_id.is_empty() {
        return deck_id.to_string();
    }
    if let Some(selected_deck_id) = selected_deck_override
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .filter(|selected| state.decks.iter().any(|deck| deck.id == *selected))
    {
        return selected_deck_id.to_string();
    }
    state
        .active_session
        .as_ref()
        .map(|active| active.deck_id.clone())
        .or_else(|| state.decks.first().map(|deck| deck.id.clone()))
        .unwrap_or_default()
}

fn unique_note_id(state: &AppState, now: u64) -> String {
    let base = format!("note-{now}");
    if state.notes.iter().all(|note| note.id != base) {
        return base;
    }

    let mut suffix = 1_u32;
    loop {
        let candidate = format!("{base}-{suffix}");
        if state.notes.iter().all(|note| note.id != candidate) {
            return candidate;
        }
        suffix += 1;
    }
}

fn active_type_answer_spec(state: &AppState, card: &Card) -> Option<TypeAnswerSpec> {
    let lineage = card.lineage.as_ref()?;
    let note = state.notes.iter().find(|note| note.id == lineage.note_id)?;
    let note_type = state
        .note_types
        .iter()
        .find(|note_type| note_type.id == lineage.note_type_id)?;
    let template = note_type
        .templates
        .iter()
        .find(|template| template.id == lineage.template_id)
        .or_else(|| {
            note_type
                .templates
                .iter()
                .find(|template| template.ordinal == lineage.ordinal)
        })?;
    let field_values = lineaged_card_field_values(state, note_type, note, template, card);
    let cloze_context = lineage
        .cloze_ordinal
        .map(|ordinal| (ordinal, ClozeRenderSide::Question));

    typed_answer_for_template(&template.front_template, &field_values, cloze_context)
}

fn lineaged_card_field_values(
    state: &AppState,
    note_type: &engram_core::NoteType,
    note: &engram_core::Note,
    template: &engram_core::CardTemplate,
    card: &Card,
) -> HashMap<String, String> {
    let field_names_by_id: HashMap<&str, &str> = note_type
        .fields
        .iter()
        .map(|field| (field.id.as_str(), field.name.as_str()))
        .collect();
    let mut field_values: HashMap<String, String> = note
        .fields
        .iter()
        .filter_map(|value| {
            field_names_by_id
                .get(value.field_id.as_str())
                .map(|name| ((*name).to_string(), value.value.clone()))
        })
        .collect();

    let deck_name = state
        .decks
        .iter()
        .find(|deck| deck.id == card.deck_id)
        .map(|deck| deck.name.as_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or(card.deck_id.as_str());
    field_values
        .entry("Tags".to_string())
        .or_insert_with(|| note.tags.join(" "));
    field_values
        .entry("Type".to_string())
        .or_insert_with(|| note_type.name.clone());
    field_values
        .entry("Deck".to_string())
        .or_insert_with(|| deck_name.to_string());
    field_values
        .entry("Subdeck".to_string())
        .or_insert_with(|| subdeck_name(deck_name).to_string());
    field_values
        .entry("Card".to_string())
        .or_insert_with(|| template.name.clone());
    field_values
        .entry("CardFlag".to_string())
        .or_insert_with(|| "flag0".to_string());
    field_values
        .entry("CardID".to_string())
        .or_insert_with(|| card.id.clone());

    field_values
}

fn subdeck_name(deck_name: &str) -> &str {
    deck_name
        .rsplit_once("::")
        .map_or(deck_name, |(_, subdeck)| subdeck)
}

fn format_type_answer_comparison(value: &str, spec: &TypeAnswerSpec, correct: bool) -> String {
    if value.trim().is_empty() {
        return format!("Expected: {}", spec.expected);
    }

    let result = if correct { "Correct" } else { "Needs review" };
    format!("You: {value} | Expected: {} | {result}", spec.expected)
}

fn host_intent_for_event(
    parsed: &ParsedEngramAppEvent,
    state: &AppState,
    deck_id: &str,
    selected_deck_override: Option<&str>,
    now: u64,
    browser: &BrowserSessionState,
) -> Option<Value> {
    let event = parsed.kind;
    let selected_deck = selected_deck_id_with_override(state, deck_id, selected_deck_override);
    let base = |intent_type: &str| {
        json!({
            "type": intent_type,
            "event": event.canonical_name(),
            "deckId": selected_deck,
            "createdAt": now,
        })
    };

    match event {
        EngramAppEvent::ImportAnki => Some(json!({
            "type": "importAnki",
            "event": event.canonical_name(),
            "deckId": selected_deck,
            "createdAt": now,
            "accept": [".apkg", ".colpkg"],
        })),
        EngramAppEvent::ExportAnki => Some(json!({
            "type": "exportAnki",
            "event": event.canonical_name(),
            "deckId": selected_deck,
            "createdAt": now,
            "extension": ".apkg",
            "extensions": [".apkg", ".colpkg"],
        })),
        EngramAppEvent::BrowserOpenSelected => {
            let selection = browser_selected_card_details(
                state,
                browser,
                parsed.card_id.as_deref(),
                now,
                Some(selected_deck.as_str()),
            );
            Some(browser_card_host_intent(
                "openCard",
                event,
                selected_deck,
                now,
                selection,
            ))
        }
        EngramAppEvent::BrowserEditSelected => None,
        EngramAppEvent::AddNote => None,
        EngramAppEvent::SaveNote => None,
        EngramAppEvent::AddNoteType => None,
        EngramAppEvent::SaveNoteType => None,
        EngramAppEvent::DeleteNote if explicit_note_id_from_app_event(parsed, state).is_some() => {
            None
        }
        EngramAppEvent::DeleteNote => Some(base("deleteNote")),
        EngramAppEvent::DeleteNoteType
            if explicit_note_type_id_from_app_event(parsed).is_some() =>
        {
            None
        }
        EngramAppEvent::DeleteNoteType => Some(base("deleteNoteType")),
        _ => None,
    }
}

fn default_note_type_model(now: u64) -> engram_core::NoteType {
    engram_core::NoteType {
        id: format!("note-type-{now}"),
        name: "Basic".to_string(),
        fields: vec![
            engram_core::FieldDef {
                id: "front".to_string(),
                name: "Front".to_string(),
                required: true,
                ordinal: 0,
            },
            engram_core::FieldDef {
                id: "back".to_string(),
                name: "Back".to_string(),
                required: true,
                ordinal: 1,
            },
        ],
        templates: vec![engram_core::CardTemplate {
            id: "forward".to_string(),
            name: "Forward".to_string(),
            front_template: "{{Front}}".to_string(),
            back_template: "{{Back}}".to_string(),
            deck_id: None,
            required_field_names: vec!["Front".to_string()],
            requirement_mode: engram_core::TemplateRequirementMode::All,
            ordinal: 0,
        }],
        stylesheet: None,
        created_at: now,
        updated_at: now,
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct BrowserSelection {
    card_id: String,
    note_id: String,
    note_type_id: String,
    note_type_name: String,
    template_id: String,
    template_name: String,
    template_ordinal: Option<u32>,
    cloze_ordinal: Option<u32>,
    deck_id: String,
    deck_name: String,
    state: String,
    card_front: String,
    card_back: String,
    note_tags: Vec<String>,
    fields: Vec<BrowserSelectionField>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct BrowserSelectionField {
    id: String,
    name: String,
    value: String,
    required: bool,
    ordinal: u32,
}

impl From<BrowserRow> for BrowserSelection {
    fn from(row: BrowserRow) -> Self {
        Self {
            card_id: row.card_id,
            note_id: row.note_id,
            template_id: row.template_id,
            state: row.state,
            ..Self::default()
        }
    }
}

fn browser_card_host_intent(
    intent_type: &str,
    event: EngramAppEvent,
    deck_id: String,
    now: u64,
    selection: BrowserSelection,
) -> Value {
    let fields = selection
        .fields
        .iter()
        .map(|field| {
            json!({
                "id": field.id,
                "name": field.name,
                "value": field.value,
                "required": field.required,
                "ordinal": field.ordinal,
            })
        })
        .collect::<Vec<_>>();

    json!({
        "type": intent_type,
        "event": event.canonical_name(),
        "deckId": deck_id,
        "createdAt": now,
        "cardId": selection.card_id,
        "noteId": selection.note_id,
        "noteTypeId": selection.note_type_id,
        "noteTypeName": selection.note_type_name,
        "templateId": selection.template_id,
        "templateName": selection.template_name,
        "templateOrdinal": selection.template_ordinal,
        "clozeOrdinal": selection.cloze_ordinal,
        "cardDeckId": selection.deck_id,
        "deckName": selection.deck_name,
        "state": selection.state,
        "cardFront": selection.card_front,
        "cardBack": selection.card_back,
        "tags": selection.note_tags,
        "fields": fields,
    })
}

fn browser_selected_card_details(
    state: &AppState,
    browser: &BrowserSessionState,
    explicit_card_id: Option<&str>,
    now: u64,
    current_deck_id: Option<&str>,
) -> BrowserSelection {
    let card_sources_by_id = browser_card_sources_by_id(state);
    let collection_created_at_days = browser_collection_created_at_days(state);
    if let Some(card_id) = explicit_card_id.filter(|card_id| !card_id.trim().is_empty()) {
        let row = state
            .cards
            .iter()
            .find(|card| card.id == card_id)
            .map(|card| {
                BrowserRow::from_card(
                    card,
                    state
                        .card_progress
                        .iter()
                        .find(|progress| progress.card_id == card.id),
                    now,
                    card_sources_by_id
                        .get(card.id.as_str())
                        .map_or(&[] as &[&ExternalSourceRecord], Vec::as_slice),
                    collection_created_at_days,
                )
            });
        let selection = row
            .map(BrowserSelection::from)
            .unwrap_or_else(|| BrowserSelection {
                card_id: card_id.to_string(),
                ..BrowserSelection::default()
            });
        return enrich_browser_selection(state, selection);
    }

    selected_browser_row(state, browser, now, current_deck_id)
        .or_else(|| {
            let results = if current_deck_id.is_some() {
                search_cards_with_context(
                    state,
                    DEFAULT_BROWSER_QUERY,
                    now,
                    SearchContext {
                        current_deck_id,
                        ..SearchContext::default()
                    },
                )
            } else {
                search_core_cards(state, DEFAULT_BROWSER_QUERY, now)
            };
            results.ok().and_then(|results| {
                results.first().map(|result| {
                    BrowserRow::from_search_result(
                        result,
                        now,
                        card_sources_by_id
                            .get(result.card.id.as_str())
                            .map_or(&[] as &[&ExternalSourceRecord], Vec::as_slice),
                        collection_created_at_days,
                    )
                })
            })
        })
        .or_else(|| {
            state.cards.first().map(|card| {
                BrowserRow::from_card(
                    card,
                    state
                        .card_progress
                        .iter()
                        .find(|progress| progress.card_id == card.id),
                    now,
                    card_sources_by_id
                        .get(card.id.as_str())
                        .map_or(&[] as &[&ExternalSourceRecord], Vec::as_slice),
                    collection_created_at_days,
                )
            })
        })
        .map(BrowserSelection::from)
        .map(|selection| enrich_browser_selection(state, selection))
        .unwrap_or_default()
}

fn note_editor_selection(
    state: &AppState,
    browser: &BrowserSessionState,
    editor: &NoteEditorSessionState,
    explicit_card_id: Option<&str>,
    now: u64,
    current_deck_id: Option<&str>,
) -> BrowserSelection {
    if editor.draft_is_new {
        return new_note_editor_selection(state, editor, now, current_deck_id);
    }

    let selection =
        browser_selected_card_details(state, browser, explicit_card_id, now, current_deck_id);
    apply_note_editor_draft_overrides(state, selection, editor)
}

fn new_note_editor_selection(
    state: &AppState,
    editor: &NoteEditorSessionState,
    now: u64,
    current_deck_id: Option<&str>,
) -> BrowserSelection {
    let note_id = editor
        .draft_note_id
        .clone()
        .unwrap_or_else(|| unique_note_id(state, now));
    let note_type_id = editor
        .draft_note_type_id
        .clone()
        .or_else(|| {
            state
                .note_types
                .first()
                .map(|note_type| note_type.id.clone())
        })
        .unwrap_or_default();
    let deck_id = editor
        .draft_deck_id
        .clone()
        .or_else(|| current_deck_id.map(str::to_string))
        .or_else(|| state.decks.first().map(|deck| deck.id.clone()))
        .unwrap_or_default();

    enrich_browser_selection(
        state,
        BrowserSelection {
            note_id,
            note_type_id,
            deck_id,
            ..BrowserSelection::default()
        },
    )
}

fn apply_note_editor_draft_overrides(
    state: &AppState,
    mut selection: BrowserSelection,
    editor: &NoteEditorSessionState,
) -> BrowserSelection {
    if editor.draft_note_id.as_deref() != Some(selection.note_id.as_str()) {
        return selection;
    }
    if let Some(note_type_id) = editor.draft_note_type_id.as_ref() {
        selection.note_type_id = note_type_id.clone();
    }
    if let Some(deck_id) = editor.draft_deck_id.as_ref() {
        selection.deck_id = deck_id.clone();
    }
    enrich_browser_selection(state, selection)
}

fn enrich_browser_selection(state: &AppState, mut selection: BrowserSelection) -> BrowserSelection {
    let card = state.cards.iter().find(|card| card.id == selection.card_id);
    if let Some(card) = card {
        if selection.deck_id.is_empty() {
            selection.deck_id = card.deck_id.clone();
        }
        selection.card_front = card.front.clone();
        selection.card_back = card.back.clone();
        if let Some(lineage) = card.lineage.as_ref() {
            if selection.note_id.is_empty() {
                selection.note_id = lineage.note_id.clone();
            }
            if selection.note_type_id.is_empty() {
                selection.note_type_id = lineage.note_type_id.clone();
            }
            if selection.template_id.is_empty() {
                selection.template_id = lineage.template_id.clone();
            }
            selection.template_ordinal = Some(lineage.ordinal);
            selection.cloze_ordinal = lineage.cloze_ordinal;
        }
    }

    let note = state.notes.iter().find(|note| note.id == selection.note_id);
    if let Some(note) = note {
        if selection.note_type_id.is_empty() {
            selection.note_type_id = note.note_type_id.clone();
        }
        if selection.deck_id.is_empty() {
            selection.deck_id = note.deck_id.clone();
        }
        selection.note_tags = note.tags.clone();
    }

    let note_type = state
        .note_types
        .iter()
        .find(|note_type| note_type.id == selection.note_type_id);
    if let Some(note_type) = note_type {
        selection.note_type_name = note_type.name.clone();
        let template = note_type
            .templates
            .iter()
            .find(|template| template.id == selection.template_id)
            .or_else(|| {
                selection.template_ordinal.and_then(|ordinal| {
                    note_type
                        .templates
                        .iter()
                        .find(|template| template.ordinal == ordinal)
                })
            });
        if let Some(template) = template {
            if selection.template_id.is_empty() {
                selection.template_id = template.id.clone();
            }
            selection.template_name = template.name.clone();
            selection.template_ordinal = Some(template.ordinal);
        }

        let values_by_field_id = note
            .map(|note| {
                note.fields
                    .iter()
                    .map(|field| (field.field_id.as_str(), field.value.as_str()))
                    .collect::<HashMap<_, _>>()
            })
            .unwrap_or_default();
        selection.fields = note_type
            .fields
            .iter()
            .map(|field| BrowserSelectionField {
                id: field.id.clone(),
                name: field.name.clone(),
                value: values_by_field_id
                    .get(field.id.as_str())
                    .copied()
                    .unwrap_or_default()
                    .to_string(),
                required: field.required,
                ordinal: field.ordinal,
            })
            .collect();
    } else if let Some(note) = note {
        selection.fields = note
            .fields
            .iter()
            .enumerate()
            .map(|(index, field)| BrowserSelectionField {
                id: field.field_id.clone(),
                name: field.field_id.clone(),
                value: field.value.clone(),
                required: false,
                ordinal: index as u32,
            })
            .collect();
    }

    selection.deck_name = state
        .decks
        .iter()
        .find(|deck| deck.id == selection.deck_id)
        .map(|deck| deck.name.clone())
        .unwrap_or_else(|| selection.deck_id.clone());

    selection
}

fn selected_browser_row(
    state: &AppState,
    browser: &BrowserSessionState,
    now: u64,
    current_deck_id: Option<&str>,
) -> Option<BrowserRow> {
    let rows = browser_rows_for_state(state, browser, now, current_deck_id)?;
    if rows.is_empty() {
        None
    } else {
        rows.get(browser.selected_index.min(rows.len() - 1))
            .cloned()
    }
}

fn select_browser_card_id(
    state: &AppState,
    browser: &mut BrowserSessionState,
    card_id: &str,
    now: u64,
    current_deck_id: Option<&str>,
) -> bool {
    let Some(rows) = browser_rows_for_state(state, browser, now, current_deck_id) else {
        return false;
    };
    if let Some((index, _)) = rows
        .iter()
        .enumerate()
        .find(|(_, row)| row.card_id == card_id)
    {
        browser.selected_index = index;
        true
    } else {
        false
    }
}

fn browser_rows_for_state(
    state: &AppState,
    browser: &BrowserSessionState,
    now: u64,
    current_deck_id: Option<&str>,
) -> Option<Vec<BrowserRow>> {
    let query = browser.effective_query();
    let results = if current_deck_id.is_some() {
        search_cards_with_context(
            state,
            &query,
            now,
            SearchContext {
                current_deck_id,
                ..SearchContext::default()
            },
        )
    } else {
        search_core_cards(state, &query, now)
    };
    results.ok().map(|results| {
        let card_sources_by_id = browser_card_sources_by_id(state);
        let collection_created_at_days = browser_collection_created_at_days(state);
        results
            .iter()
            .take(20)
            .map(|result| {
                BrowserRow::from_search_result(
                    result,
                    now,
                    card_sources_by_id
                        .get(result.card.id.as_str())
                        .map_or(&[] as &[&ExternalSourceRecord], Vec::as_slice),
                    collection_created_at_days,
                )
            })
            .collect()
    })
}

fn note_editor_selected_field_index(
    selection: &BrowserSelection,
    editor: &NoteEditorSessionState,
) -> Option<usize> {
    if selection.fields.is_empty() {
        None
    } else {
        Some(editor.selected_field_index.min(selection.fields.len() - 1))
    }
}

fn note_editor_selected_field<'a>(
    selection: &'a BrowserSelection,
    editor: &NoteEditorSessionState,
) -> Option<&'a BrowserSelectionField> {
    note_editor_selected_field_index(selection, editor)
        .and_then(|index| selection.fields.get(index))
}

fn note_from_editor_selection(
    state: &AppState,
    selection: &BrowserSelection,
    editor: &NoteEditorSessionState,
    now: u64,
) -> Result<Note, String> {
    if selection.note_id.is_empty() {
        return Err("cannot save note without a selected note".to_string());
    }
    let existing_note = state.notes.iter().find(|note| note.id == selection.note_id);
    let note_type_id = if !selection.note_type_id.is_empty() {
        selection.note_type_id.clone()
    } else {
        existing_note
            .map(|note| note.note_type_id.clone())
            .ok_or_else(|| "selected note has no note type".to_string())?
    };
    if !state
        .note_types
        .iter()
        .any(|note_type| note_type.id == note_type_id)
    {
        return Err("selected note type does not exist".to_string());
    }
    let deck_id = if !selection.deck_id.is_empty() {
        selection.deck_id.clone()
    } else {
        existing_note
            .map(|note| note.deck_id.clone())
            .unwrap_or_default()
    };
    let draft_active = editor.draft_note_id.as_deref() == Some(selection.note_id.as_str());
    let fields = if selection.fields.is_empty() {
        existing_note
            .map(|note| note.fields.clone())
            .unwrap_or_default()
    } else {
        selection
            .fields
            .iter()
            .map(|field| NoteFieldValue {
                field_id: field.id.clone(),
                value: if draft_active {
                    editor
                        .draft_fields
                        .get(field.id.as_str())
                        .cloned()
                        .unwrap_or_else(|| field.value.clone())
                } else {
                    field.value.clone()
                },
            })
            .collect()
    };
    let tags = if draft_active {
        editor
            .draft_tags
            .as_ref()
            .map(|tags| normalize_note_tags(tags.split_whitespace().map(str::to_string).collect()))
            .unwrap_or_else(|| selection.note_tags.clone())
    } else {
        selection.note_tags.clone()
    };

    Ok(Note {
        id: selection.note_id.clone(),
        note_type_id,
        deck_id,
        fields,
        tags,
        created_at: existing_note
            .map(|note| note.created_at)
            .or(editor.draft_created_at)
            .unwrap_or(now),
        updated_at: now,
    })
}

fn note_type_editor_selected_index(
    state: &AppState,
    editor: &NoteTypeEditorSessionState,
) -> Option<usize> {
    if editor.draft_is_new {
        Some(state.note_types.len())
    } else if state.note_types.is_empty() {
        None
    } else {
        Some(editor.selected_index.min(state.note_types.len() - 1))
    }
}

fn note_type_editor_selected_id(
    state: &AppState,
    editor: &NoteTypeEditorSessionState,
    now: u64,
) -> Option<String> {
    note_type_editor_selected_note_type(state, editor, now).map(|note_type| note_type.id)
}

fn note_type_editor_selected_field_id(
    state: &AppState,
    editor: &NoteTypeEditorSessionState,
    now: u64,
) -> Option<(String, String)> {
    let note_type = note_type_editor_selected_note_type(state, editor, now)?;
    let field_index = note_type_editor_selected_field_index(&note_type, editor)?;
    let field_id = note_type.fields.get(field_index)?.id.clone();
    Some((note_type.id, field_id))
}

fn note_type_editor_selected_template_id(
    state: &AppState,
    editor: &NoteTypeEditorSessionState,
    now: u64,
) -> Option<(String, String)> {
    let note_type = note_type_editor_selected_note_type(state, editor, now)?;
    let template_index = note_type_editor_selected_template_index(&note_type, editor)?;
    let template_id = note_type.templates.get(template_index)?.id.clone();
    Some((note_type.id, template_id))
}

fn note_type_editor_selected_field_index(
    note_type: &engram_core::NoteType,
    editor: &NoteTypeEditorSessionState,
) -> Option<usize> {
    if note_type.fields.is_empty() {
        None
    } else {
        Some(editor.selected_field_index.min(note_type.fields.len() - 1))
    }
}

fn note_type_editor_selected_template_index(
    note_type: &engram_core::NoteType,
    editor: &NoteTypeEditorSessionState,
) -> Option<usize> {
    if note_type.templates.is_empty() {
        None
    } else {
        Some(
            editor
                .selected_template_index
                .min(note_type.templates.len() - 1),
        )
    }
}

fn note_type_editor_selected_note_type(
    state: &AppState,
    editor: &NoteTypeEditorSessionState,
    now: u64,
) -> Option<engram_core::NoteType> {
    let mut note_type = if editor.draft_is_new {
        let mut draft = default_note_type_model(editor.draft_created_at.unwrap_or(now));
        if let Some(note_type_id) = editor.draft_note_type_id.as_ref() {
            draft.id = note_type_id.clone();
        }
        draft
    } else {
        let index = note_type_editor_selected_index(state, editor)?;
        state.note_types.get(index)?.clone()
    };

    if editor.draft_note_type_id.as_deref() == Some(note_type.id.as_str()) || editor.draft_is_new {
        if let Some(name) = editor.draft_name.as_ref() {
            note_type.name = name.clone();
        }
        if let Some(stylesheet) = editor.draft_stylesheet.as_ref() {
            note_type.stylesheet = (!stylesheet.trim().is_empty()).then(|| stylesheet.clone());
        }
        for (field_id, name) in &editor.draft_field_names {
            note_type = rename_note_type_field(&note_type, field_id, name, now);
        }
        for field in &mut note_type.fields {
            if let Some(required) = editor.draft_field_required.get(field.id.as_str()) {
                field.required = *required;
            }
        }
        for template in &mut note_type.templates {
            if let Some(name) = editor.draft_template_names.get(template.id.as_str()) {
                template.name = name.clone();
            }
            if let Some(front) = editor.draft_template_fronts.get(template.id.as_str()) {
                template.front_template = front.clone();
            }
            if let Some(back) = editor.draft_template_backs.get(template.id.as_str()) {
                template.back_template = back.clone();
            }
        }
    }

    Some(note_type)
}

fn note_type_from_editor_selection(
    state: &AppState,
    editor: &NoteTypeEditorSessionState,
    now: u64,
) -> Result<engram_core::NoteType, String> {
    let mut note_type = note_type_editor_selected_note_type(state, editor, now)
        .ok_or_else(|| "cannot save note type without a selected note type".to_string())?;
    if note_type.name.trim().is_empty() {
        return Err("onNoteTypeEditorSaveNoteType is missing a name".to_string());
    }
    note_type.updated_at = now;
    Ok(note_type)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EngramAppEvent {
    ShowScreen(EngramAppScreen),
    SelectDeck,
    Reveal,
    Undo,
    BuryCard,
    BurySiblings,
    SuspendCard,
    ToggleMark,
    TypeAnswerChange,
    DeckOptionsChange(DeckOptionField),
    Rate(Rating),
    BrowserQueryChange,
    BrowserToggleFilter,
    BrowserSetFilter,
    BrowserSearch,
    BrowserSelectResult,
    BrowserOpenSelected,
    BrowserEditSelected,
    BrowserToggleSuspendSelected,
    BrowserToggleMarkSelected,
    BrowserToggleFlagPicker,
    BrowserSetFlagSelected,
    BrowserTagEditChange,
    BrowserAddTagSelected,
    BrowserRemoveTagSelected,
    BrowserCustomStudyLimitChange,
    BrowserCustomStudyRescheduleChange,
    BrowserRebuildFilteredDeck,
    BrowserEmptyFilteredDeck,
    PruneUnusedMedia,
    NoteEditorSelectNoteType,
    NoteEditorSelectDeck,
    NoteEditorSelectField,
    NoteEditorFieldValueChange,
    NoteEditorTagsChange,
    NoteEditorSaveNote,
    NoteEditorDeleteNote,
    NoteEditorCancel,
    NoteTypeEditorSelectNoteType,
    NoteTypeEditorSelectField,
    NoteTypeEditorSelectTemplate,
    NoteTypeEditorNameChange,
    NoteTypeEditorFieldNameChange,
    NoteTypeEditorFieldRequiredChange,
    NoteTypeEditorTemplateNameChange,
    NoteTypeEditorFrontTemplateChange,
    NoteTypeEditorBackTemplateChange,
    NoteTypeEditorStylesheetChange,
    NoteTypeEditorNewNoteType,
    NoteTypeEditorSaveNoteType,
    NoteTypeEditorDeleteNoteType,
    NoteTypeEditorCancel,
    ImportAnki,
    ExportAnki,
    AddNote,
    SaveNote,
    AddNoteType,
    SaveNoteType,
    DeleteNote,
    DeleteNoteType,
}

impl EngramAppEvent {
    fn canonical_name(self) -> &'static str {
        match self {
            Self::ShowScreen(EngramAppScreen::Decks) => "onShowDecks",
            Self::ShowScreen(EngramAppScreen::Study) => "onShowStudy",
            Self::ShowScreen(EngramAppScreen::Browse) => "onShowBrowse",
            Self::ShowScreen(EngramAppScreen::Add) => "onShowAdd",
            Self::ShowScreen(EngramAppScreen::Stats) => "onShowStats",
            Self::ShowScreen(EngramAppScreen::Options) => "onShowOptions",
            Self::SelectDeck => "onSelectDeck",
            Self::Reveal => "onReveal",
            Self::Undo => "onUndo",
            Self::BuryCard => "onBuryCard",
            Self::BurySiblings => "onBurySiblings",
            Self::SuspendCard => "onSuspendCard",
            Self::ToggleMark => "onToggleMark",
            Self::TypeAnswerChange => "onTypeAnswerChange",
            Self::DeckOptionsChange(DeckOptionField::LearningStepsMinutes) => {
                "onDeckOptionsLearningStepsChange"
            }
            Self::DeckOptionsChange(DeckOptionField::RelearningStepsMinutes) => {
                "onDeckOptionsRelearningStepsChange"
            }
            Self::DeckOptionsChange(DeckOptionField::NewCardsPerDay) => {
                "onDeckOptionsNewCardsChange"
            }
            Self::DeckOptionsChange(DeckOptionField::ReviewsPerDay) => "onDeckOptionsReviewsChange",
            Self::DeckOptionsChange(DeckOptionField::GraduatingIntervalDays) => {
                "onDeckOptionsGraduatingIntervalChange"
            }
            Self::DeckOptionsChange(DeckOptionField::EasyIntervalDays) => {
                "onDeckOptionsEasyIntervalChange"
            }
            Self::DeckOptionsChange(DeckOptionField::InitialEaseFactor) => {
                "onDeckOptionsInitialEaseChange"
            }
            Self::DeckOptionsChange(DeckOptionField::MaximumIntervalDays) => {
                "onDeckOptionsMaximumIntervalChange"
            }
            Self::DeckOptionsChange(DeckOptionField::ReviewIntervalModifier) => {
                "onDeckOptionsIntervalModifierChange"
            }
            Self::DeckOptionsChange(DeckOptionField::HardIntervalMultiplier) => {
                "onDeckOptionsHardMultiplierChange"
            }
            Self::DeckOptionsChange(DeckOptionField::EasyBonusMultiplier) => {
                "onDeckOptionsEasyBonusChange"
            }
            Self::DeckOptionsChange(DeckOptionField::LapseIntervalMultiplier) => {
                "onDeckOptionsLapseMultiplierChange"
            }
            Self::DeckOptionsChange(DeckOptionField::LeechThreshold) => {
                "onDeckOptionsLeechThresholdChange"
            }
            Self::DeckOptionsChange(DeckOptionField::DesiredRetention) => {
                "onDeckOptionsDesiredRetentionChange"
            }
            Self::DeckOptionsChange(DeckOptionField::FsrsParameters) => {
                "onDeckOptionsFsrsParametersChange"
            }
            Self::DeckOptionsChange(DeckOptionField::FsrsParameterSearch) => {
                "onDeckOptionsFsrsSearchChange"
            }
            Self::DeckOptionsChange(DeckOptionField::IgnoreReviewHistoryBefore) => {
                "onDeckOptionsIgnoreReviewHistoryBeforeChange"
            }
            Self::DeckOptionsChange(DeckOptionField::HistoricalRetention) => {
                "onDeckOptionsHistoricalRetentionChange"
            }
            Self::DeckOptionsChange(DeckOptionField::EasyDaysPercentages) => {
                "onDeckOptionsEasyDaysPercentagesChange"
            }
            Self::DeckOptionsChange(DeckOptionField::LeechAction) => {
                "onDeckOptionsLeechActionChange"
            }
            Self::DeckOptionsChange(DeckOptionField::BuryNewSiblings) => {
                "onDeckOptionsBuryNewSiblingsChange"
            }
            Self::DeckOptionsChange(DeckOptionField::BuryReviewSiblings) => {
                "onDeckOptionsBuryReviewSiblingsChange"
            }
            Self::DeckOptionsChange(DeckOptionField::BuryInterdayLearningSiblings) => {
                "onDeckOptionsBuryInterdayLearningSiblingsChange"
            }
            Self::Rate(Rating::Again) => "onAgain",
            Self::Rate(Rating::Hard) => "onHard",
            Self::Rate(Rating::Good) => "onGood",
            Self::Rate(Rating::Easy) => "onEasy",
            Self::BrowserQueryChange => "onBrowserQueryChange",
            Self::BrowserToggleFilter => "onBrowserToggleFilter",
            Self::BrowserSetFilter => "onBrowserSetFilter",
            Self::BrowserSearch => "onBrowserSearch",
            Self::BrowserSelectResult => "onBrowserSelectResult",
            Self::BrowserOpenSelected => "onBrowserOpenSelected",
            Self::BrowserEditSelected => "onBrowserEditSelected",
            Self::BrowserToggleSuspendSelected => "onBrowserToggleSuspendSelected",
            Self::BrowserToggleMarkSelected => "onBrowserToggleMarkSelected",
            Self::BrowserToggleFlagPicker => "onBrowserToggleFlagPicker",
            Self::BrowserSetFlagSelected => "onBrowserSetFlagSelected",
            Self::BrowserTagEditChange => "onBrowserTagEditChange",
            Self::BrowserAddTagSelected => "onBrowserAddTagSelected",
            Self::BrowserRemoveTagSelected => "onBrowserRemoveTagSelected",
            Self::BrowserCustomStudyLimitChange => "onBrowserCustomStudyLimitChange",
            Self::BrowserCustomStudyRescheduleChange => "onBrowserCustomStudyRescheduleChange",
            Self::BrowserRebuildFilteredDeck => "onBrowserRebuildFilteredDeck",
            Self::BrowserEmptyFilteredDeck => "onBrowserEmptyFilteredDeck",
            Self::PruneUnusedMedia => "onPruneUnusedMedia",
            Self::NoteEditorSelectNoteType => "onNoteEditorSelectNoteType",
            Self::NoteEditorSelectDeck => "onNoteEditorSelectDeck",
            Self::NoteEditorSelectField => "onNoteEditorSelectField",
            Self::NoteEditorFieldValueChange => "onNoteEditorFieldValueChange",
            Self::NoteEditorTagsChange => "onNoteEditorTagsChange",
            Self::NoteEditorSaveNote => "onNoteEditorSaveNote",
            Self::NoteEditorDeleteNote => "onNoteEditorDeleteNote",
            Self::NoteEditorCancel => "onNoteEditorCancel",
            Self::NoteTypeEditorSelectNoteType => "onNoteTypeEditorSelectNoteType",
            Self::NoteTypeEditorSelectField => "onNoteTypeEditorSelectField",
            Self::NoteTypeEditorSelectTemplate => "onNoteTypeEditorSelectTemplate",
            Self::NoteTypeEditorNameChange => "onNoteTypeEditorNameChange",
            Self::NoteTypeEditorFieldNameChange => "onNoteTypeEditorFieldNameChange",
            Self::NoteTypeEditorFieldRequiredChange => "onNoteTypeEditorFieldRequiredChange",
            Self::NoteTypeEditorTemplateNameChange => "onNoteTypeEditorTemplateNameChange",
            Self::NoteTypeEditorFrontTemplateChange => "onNoteTypeEditorFrontTemplateChange",
            Self::NoteTypeEditorBackTemplateChange => "onNoteTypeEditorBackTemplateChange",
            Self::NoteTypeEditorStylesheetChange => "onNoteTypeEditorStylesheetChange",
            Self::NoteTypeEditorNewNoteType => "onNoteTypeEditorNewNoteType",
            Self::NoteTypeEditorSaveNoteType => "onNoteTypeEditorSaveNoteType",
            Self::NoteTypeEditorDeleteNoteType => "onNoteTypeEditorDeleteNoteType",
            Self::NoteTypeEditorCancel => "onNoteTypeEditorCancel",
            Self::ImportAnki => "onImportAnki",
            Self::ExportAnki => "onExportAnki",
            Self::AddNote => "onAddNote",
            Self::SaveNote => "onSaveNote",
            Self::AddNoteType => "onAddNoteType",
            Self::SaveNoteType => "onSaveNoteType",
            Self::DeleteNote => "onDeleteNote",
            Self::DeleteNoteType => "onDeleteNoteType",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DeckOptionField {
    LearningStepsMinutes,
    RelearningStepsMinutes,
    NewCardsPerDay,
    ReviewsPerDay,
    GraduatingIntervalDays,
    EasyIntervalDays,
    InitialEaseFactor,
    MaximumIntervalDays,
    ReviewIntervalModifier,
    HardIntervalMultiplier,
    EasyBonusMultiplier,
    LapseIntervalMultiplier,
    LeechThreshold,
    DesiredRetention,
    FsrsParameters,
    FsrsParameterSearch,
    IgnoreReviewHistoryBefore,
    HistoricalRetention,
    EasyDaysPercentages,
    LeechAction,
    BuryNewSiblings,
    BuryReviewSiblings,
    BuryInterdayLearningSiblings,
}

#[derive(Clone, Debug, PartialEq)]
struct ParsedEngramAppEvent {
    kind: EngramAppEvent,
    card_id: Option<String>,
    number_value: Option<f64>,
    text_value: Option<String>,
    bool_value: Option<bool>,
    payload: Option<Value>,
}

fn parse_engram_app_event(event: &str) -> Result<ParsedEngramAppEvent, String> {
    let event = event.trim();
    if let Ok(value) = serde_json::from_str::<Value>(event) {
        if let Some(event_name) = value.as_str() {
            return parse_engram_app_event_name(event_name, None, None, None, None);
        }
        let event_name = value
            .get("event")
            .or_else(|| value.get("name"))
            .or_else(|| value.get("type"))
            .and_then(Value::as_str)
            .ok_or_else(|| "Engram app event object is missing an event name".to_string())?;
        let card_id = value
            .get("cardId")
            .or_else(|| value.get("selectedCardId"))
            .and_then(Value::as_str)
            .map(str::to_string);
        let event_value = value.get("value");
        let number_value = event_value.and_then(parse_json_number_value).or_else(|| {
            value
                .get("index")
                .or_else(|| value.get("selectedIndex"))
                .and_then(parse_json_number_value)
        });
        let text_value = event_value.and_then(parse_json_text_value).or_else(|| {
            value
                .get("query")
                .or_else(|| value.get("text"))
                .and_then(parse_json_text_value)
        });
        let bool_value = value
            .get("checked")
            .and_then(parse_json_bool_value)
            .or_else(|| event_value.and_then(parse_json_bool_value));
        let mut parsed =
            parse_engram_app_event_name(event_name, card_id, number_value, text_value, bool_value)?;
        parsed.payload = Some(value);
        return Ok(parsed);
    }

    let (event_name, card_id) = split_event_card_id(event);
    parse_engram_app_event_name(event_name, card_id, None, None, None)
}

fn split_event_card_id(event: &str) -> (&str, Option<String>) {
    event
        .split_once('|')
        .or_else(|| event.split_once(':'))
        .map(|(event_name, card_id)| {
            let card_id = card_id.trim();
            (
                event_name.trim(),
                (!card_id.is_empty()).then(|| card_id.to_string()),
            )
        })
        .unwrap_or((event, None))
}

fn parse_engram_app_event_name(
    event_name: &str,
    card_id: Option<String>,
    number_value: Option<f64>,
    text_value: Option<String>,
    bool_value: Option<bool>,
) -> Result<ParsedEngramAppEvent, String> {
    let lowered = event_name.trim().to_ascii_lowercase();
    let parsed = |kind| {
        Ok(parsed_event(
            kind,
            card_id.clone(),
            number_value,
            text_value.clone(),
            bool_value,
        ))
    };
    match lowered.strip_prefix("on").unwrap_or(&lowered) {
        "showdecks" | "show-decks" | "show_decks" | "decks" => {
            parsed(EngramAppEvent::ShowScreen(EngramAppScreen::Decks))
        }
        "showstudy" | "show-study" | "show_study" | "study" => {
            parsed(EngramAppEvent::ShowScreen(EngramAppScreen::Study))
        }
        "showbrowse" | "show-browse" | "show_browse" | "browse" => {
            parsed(EngramAppEvent::ShowScreen(EngramAppScreen::Browse))
        }
        "showadd" | "show-add" | "show_add" | "addscreen" | "add-screen" | "add_screen" => {
            parsed(EngramAppEvent::ShowScreen(EngramAppScreen::Add))
        }
        "showstats" | "show-stats" | "show_stats" | "stats" => {
            parsed(EngramAppEvent::ShowScreen(EngramAppScreen::Stats))
        }
        "showoptions" | "show-options" | "show_options" | "options" => {
            parsed(EngramAppEvent::ShowScreen(EngramAppScreen::Options))
        }
        "selectdeck" | "select-deck" | "select_deck" | "deckselect" | "deck-select"
        | "deck_select" => parsed(EngramAppEvent::SelectDeck),
        "reveal" => parsed(EngramAppEvent::Reveal),
        "undo" => parsed(EngramAppEvent::Undo),
        "burycard" | "bury-card" | "bury_card" => parsed(EngramAppEvent::BuryCard),
        "burysiblings" | "bury-siblings" | "bury_siblings" | "burynote" | "bury-note"
        | "bury_note" => parsed(EngramAppEvent::BurySiblings),
        "suspendcard" | "suspend-card" | "suspend_card" | "suspend" => {
            parsed(EngramAppEvent::SuspendCard)
        }
        "togglemark" | "toggle-mark" | "toggle_mark" | "mark" => parsed(EngramAppEvent::ToggleMark),
        "typeanswerchange" | "type-answer-change" | "type_answer_change" => {
            parsed(EngramAppEvent::TypeAnswerChange)
        }
        "deckoptionslearningstepschange"
        | "deck-options-learning-steps-change"
        | "deck_options_learning_steps_change" => parsed(EngramAppEvent::DeckOptionsChange(
            DeckOptionField::LearningStepsMinutes,
        )),
        "deckoptionsrelearningstepschange"
        | "deck-options-relearning-steps-change"
        | "deck_options_relearning_steps_change" => parsed(EngramAppEvent::DeckOptionsChange(
            DeckOptionField::RelearningStepsMinutes,
        )),
        "deckoptionsnewcardschange"
        | "deck-options-new-cards-change"
        | "deck_options_new_cards_change" => parsed(EngramAppEvent::DeckOptionsChange(
            DeckOptionField::NewCardsPerDay,
        )),
        "deckoptionsreviewschange"
        | "deck-options-reviews-change"
        | "deck_options_reviews_change" => parsed(EngramAppEvent::DeckOptionsChange(
            DeckOptionField::ReviewsPerDay,
        )),
        "deckoptionsgraduatingintervalchange"
        | "deck-options-graduating-interval-change"
        | "deck_options_graduating_interval_change" => parsed(EngramAppEvent::DeckOptionsChange(
            DeckOptionField::GraduatingIntervalDays,
        )),
        "deckoptionseasyintervalchange"
        | "deck-options-easy-interval-change"
        | "deck_options_easy_interval_change" => parsed(EngramAppEvent::DeckOptionsChange(
            DeckOptionField::EasyIntervalDays,
        )),
        "deckoptionsinitialeasechange"
        | "deck-options-initial-ease-change"
        | "deck_options_initial_ease_change"
        | "deckoptionsinitialfactorchange"
        | "deck-options-initial-factor-change"
        | "deck_options_initial_factor_change" => parsed(EngramAppEvent::DeckOptionsChange(
            DeckOptionField::InitialEaseFactor,
        )),
        "deckoptionsmaximumintervalchange"
        | "deck-options-maximum-interval-change"
        | "deck_options_maximum_interval_change" => parsed(EngramAppEvent::DeckOptionsChange(
            DeckOptionField::MaximumIntervalDays,
        )),
        "deckoptionsintervalmodifierchange"
        | "deck-options-interval-modifier-change"
        | "deck_options_interval_modifier_change" => parsed(EngramAppEvent::DeckOptionsChange(
            DeckOptionField::ReviewIntervalModifier,
        )),
        "deckoptionshardmultiplierchange"
        | "deck-options-hard-multiplier-change"
        | "deck_options_hard_multiplier_change" => parsed(EngramAppEvent::DeckOptionsChange(
            DeckOptionField::HardIntervalMultiplier,
        )),
        "deckoptionseasybonuschange"
        | "deck-options-easy-bonus-change"
        | "deck_options_easy_bonus_change" => parsed(EngramAppEvent::DeckOptionsChange(
            DeckOptionField::EasyBonusMultiplier,
        )),
        "deckoptionslapsemultiplierchange"
        | "deck-options-lapse-multiplier-change"
        | "deck_options_lapse_multiplier_change" => parsed(EngramAppEvent::DeckOptionsChange(
            DeckOptionField::LapseIntervalMultiplier,
        )),
        "deckoptionsleechthresholdchange"
        | "deck-options-leech-threshold-change"
        | "deck_options_leech_threshold_change" => parsed(EngramAppEvent::DeckOptionsChange(
            DeckOptionField::LeechThreshold,
        )),
        "deckoptionsdesiredretentionchange"
        | "deck-options-desired-retention-change"
        | "deck_options_desired_retention_change" => parsed(EngramAppEvent::DeckOptionsChange(
            DeckOptionField::DesiredRetention,
        )),
        "deckoptionsfsrsparameterschange"
        | "deck-options-fsrs-parameters-change"
        | "deck_options_fsrs_parameters_change"
        | "deckoptionsfsrsparamschange"
        | "deck-options-fsrs-params-change"
        | "deck_options_fsrs_params_change" => parsed(EngramAppEvent::DeckOptionsChange(
            DeckOptionField::FsrsParameters,
        )),
        "deckoptionsfsrssearchchange"
        | "deck-options-fsrs-search-change"
        | "deck_options_fsrs_search_change"
        | "deckoptionsfsrsparametersearchchange"
        | "deck-options-fsrs-parameter-search-change"
        | "deck_options_fsrs_parameter_search_change" => parsed(EngramAppEvent::DeckOptionsChange(
            DeckOptionField::FsrsParameterSearch,
        )),
        "deckoptionsignorereviewhistorybeforechange"
        | "deck-options-ignore-review-history-before-change"
        | "deck_options_ignore_review_history_before_change" => parsed(
            EngramAppEvent::DeckOptionsChange(DeckOptionField::IgnoreReviewHistoryBefore),
        ),
        "deckoptionshistoricalretentionchange"
        | "deck-options-historical-retention-change"
        | "deck_options_historical_retention_change"
        | "deckoptionssm2retentionchange"
        | "deck-options-sm2-retention-change"
        | "deck_options_sm2_retention_change" => parsed(EngramAppEvent::DeckOptionsChange(
            DeckOptionField::HistoricalRetention,
        )),
        "deckoptionseasydayspercentageschange"
        | "deck-options-easy-days-percentages-change"
        | "deck_options_easy_days_percentages_change" => parsed(EngramAppEvent::DeckOptionsChange(
            DeckOptionField::EasyDaysPercentages,
        )),
        "deckoptionsleechactionchange"
        | "deck-options-leech-action-change"
        | "deck_options_leech_action_change" => parsed(EngramAppEvent::DeckOptionsChange(
            DeckOptionField::LeechAction,
        )),
        "deckoptionsburynewsiblingschange"
        | "deck-options-bury-new-siblings-change"
        | "deck_options_bury_new_siblings_change" => parsed(EngramAppEvent::DeckOptionsChange(
            DeckOptionField::BuryNewSiblings,
        )),
        "deckoptionsburyreviewsiblingschange"
        | "deck-options-bury-review-siblings-change"
        | "deck_options_bury_review_siblings_change" => parsed(EngramAppEvent::DeckOptionsChange(
            DeckOptionField::BuryReviewSiblings,
        )),
        "deckoptionsburyinterdaylearningsiblingschange"
        | "deck-options-bury-interday-learning-siblings-change"
        | "deck_options_bury_interday_learning_siblings_change" => parsed(
            EngramAppEvent::DeckOptionsChange(DeckOptionField::BuryInterdayLearningSiblings),
        ),
        "again" => parsed(EngramAppEvent::Rate(Rating::Again)),
        "hard" => parsed(EngramAppEvent::Rate(Rating::Hard)),
        "good" => parsed(EngramAppEvent::Rate(Rating::Good)),
        "easy" => parsed(EngramAppEvent::Rate(Rating::Easy)),
        "browserquerychange" | "browser-query-change" | "browser_query_change" => {
            parsed(EngramAppEvent::BrowserQueryChange)
        }
        "browsertogglefilter" | "browser-toggle-filter" | "browser_toggle_filter" => {
            parsed(EngramAppEvent::BrowserToggleFilter)
        }
        "browsersetfilter" | "browser-set-filter" | "browser_set_filter" => {
            parsed(EngramAppEvent::BrowserSetFilter)
        }
        "browsersearch" | "browser-search" | "browser_search" => {
            parsed(EngramAppEvent::BrowserSearch)
        }
        "browserselectresult" | "browser-select-result" | "browser_select_result" => {
            parsed(EngramAppEvent::BrowserSelectResult)
        }
        "browseropenselected" | "browser-open-selected" | "browser_open_selected" => {
            parsed(EngramAppEvent::BrowserOpenSelected)
        }
        "browsereditselected" | "browser-edit-selected" | "browser_edit_selected" => {
            parsed(EngramAppEvent::BrowserEditSelected)
        }
        "browsertogglesuspendselected"
        | "browser-toggle-suspend-selected"
        | "browser_toggle_suspend_selected" => parsed(EngramAppEvent::BrowserToggleSuspendSelected),
        "browsertogglemarkselected"
        | "browser-toggle-mark-selected"
        | "browser_toggle_mark_selected" => parsed(EngramAppEvent::BrowserToggleMarkSelected),
        "browsertoggleflagpicker" | "browser-toggle-flag-picker" | "browser_toggle_flag_picker" => {
            parsed(EngramAppEvent::BrowserToggleFlagPicker)
        }
        "browsersetflagselected"
        | "browser-set-flag-selected"
        | "browser_set_flag_selected"
        | "browserflagchange"
        | "browser-flag-change"
        | "browser_flag_change" => parsed(EngramAppEvent::BrowserSetFlagSelected),
        "browsertageditchange" | "browser-tag-edit-change" | "browser_tag_edit_change" => {
            parsed(EngramAppEvent::BrowserTagEditChange)
        }
        "browseraddtagselected" | "browser-add-tag-selected" | "browser_add_tag_selected" => {
            parsed(EngramAppEvent::BrowserAddTagSelected)
        }
        "browserremovetagselected"
        | "browser-remove-tag-selected"
        | "browser_remove_tag_selected" => parsed(EngramAppEvent::BrowserRemoveTagSelected),
        "browsercustomstudylimitchange"
        | "browser-custom-study-limit-change"
        | "browser_custom_study_limit_change" => {
            parsed(EngramAppEvent::BrowserCustomStudyLimitChange)
        }
        "browsercustomstudyreschedulechange"
        | "browser-custom-study-reschedule-change"
        | "browser_custom_study_reschedule_change" => {
            parsed(EngramAppEvent::BrowserCustomStudyRescheduleChange)
        }
        "browserrebuildfiltereddeck"
        | "browser-rebuild-filtered-deck"
        | "browser_rebuild_filtered_deck"
        | "browserrebuildcustomstudy"
        | "browser-rebuild-custom-study"
        | "browser_rebuild_custom_study" => parsed(EngramAppEvent::BrowserRebuildFilteredDeck),
        "browseremptyfiltereddeck"
        | "browser-empty-filtered-deck"
        | "browser_empty_filtered_deck"
        | "browseremptycustomstudy"
        | "browser-empty-custom-study"
        | "browser_empty_custom_study" => parsed(EngramAppEvent::BrowserEmptyFilteredDeck),
        "pruneunusedmedia"
        | "prune-unused-media"
        | "prune_unused_media"
        | "collectionpruneunusedmedia"
        | "collection-prune-unused-media"
        | "collection_prune_unused_media" => parsed(EngramAppEvent::PruneUnusedMedia),
        "noteeditorselectnotetype"
        | "note-editor-select-note-type"
        | "note_editor_select_note_type" => parsed(EngramAppEvent::NoteEditorSelectNoteType),
        "noteeditorselectdeck" | "note-editor-select-deck" | "note_editor_select_deck" => {
            parsed(EngramAppEvent::NoteEditorSelectDeck)
        }
        "noteeditorselectfield" | "note-editor-select-field" | "note_editor_select_field" => {
            parsed(EngramAppEvent::NoteEditorSelectField)
        }
        "noteeditorfieldvaluechange"
        | "note-editor-field-value-change"
        | "note_editor_field_value_change" => parsed(EngramAppEvent::NoteEditorFieldValueChange),
        "noteeditortagschange" | "note-editor-tags-change" | "note_editor_tags_change" => {
            parsed(EngramAppEvent::NoteEditorTagsChange)
        }
        "noteeditorsavenote" | "note-editor-save-note" | "note_editor_save_note" => {
            parsed(EngramAppEvent::NoteEditorSaveNote)
        }
        "noteeditordeletenote" | "note-editor-delete-note" | "note_editor_delete_note" => {
            parsed(EngramAppEvent::NoteEditorDeleteNote)
        }
        "noteeditorcancel" | "note-editor-cancel" | "note_editor_cancel" => {
            parsed(EngramAppEvent::NoteEditorCancel)
        }
        "notetypeeditorselectnotetype"
        | "note-type-editor-select-note-type"
        | "note_type_editor_select_note_type" => {
            parsed(EngramAppEvent::NoteTypeEditorSelectNoteType)
        }
        "notetypeeditorselectfield"
        | "note-type-editor-select-field"
        | "note_type_editor_select_field" => parsed(EngramAppEvent::NoteTypeEditorSelectField),
        "notetypeeditorselecttemplate"
        | "note-type-editor-select-template"
        | "note_type_editor_select_template" => {
            parsed(EngramAppEvent::NoteTypeEditorSelectTemplate)
        }
        "notetypeeditornamechange"
        | "note-type-editor-name-change"
        | "note_type_editor_name_change" => parsed(EngramAppEvent::NoteTypeEditorNameChange),
        "notetypeeditorfieldnamechange"
        | "note-type-editor-field-name-change"
        | "note_type_editor_field_name_change" => {
            parsed(EngramAppEvent::NoteTypeEditorFieldNameChange)
        }
        "notetypeeditorfieldrequiredchange"
        | "note-type-editor-field-required-change"
        | "note_type_editor_field_required_change" => {
            parsed(EngramAppEvent::NoteTypeEditorFieldRequiredChange)
        }
        "notetypeeditortemplatenamechange"
        | "note-type-editor-template-name-change"
        | "note_type_editor_template_name_change" => {
            parsed(EngramAppEvent::NoteTypeEditorTemplateNameChange)
        }
        "notetypeeditorfronttemplatechange"
        | "note-type-editor-front-template-change"
        | "note_type_editor_front_template_change" => {
            parsed(EngramAppEvent::NoteTypeEditorFrontTemplateChange)
        }
        "notetypeeditorbacktemplatechange"
        | "note-type-editor-back-template-change"
        | "note_type_editor_back_template_change" => {
            parsed(EngramAppEvent::NoteTypeEditorBackTemplateChange)
        }
        "notetypeeditorstylesheetchange"
        | "note-type-editor-stylesheet-change"
        | "note_type_editor_stylesheet_change" => {
            parsed(EngramAppEvent::NoteTypeEditorStylesheetChange)
        }
        "notetypeeditornewnotetype"
        | "note-type-editor-new-note-type"
        | "note_type_editor_new_note_type" => parsed(EngramAppEvent::NoteTypeEditorNewNoteType),
        "notetypeeditorsavenotetype"
        | "note-type-editor-save-note-type"
        | "note_type_editor_save_note_type" => parsed(EngramAppEvent::NoteTypeEditorSaveNoteType),
        "notetypeeditordeletenotetype"
        | "note-type-editor-delete-note-type"
        | "note_type_editor_delete_note_type" => {
            parsed(EngramAppEvent::NoteTypeEditorDeleteNoteType)
        }
        "notetypeeditorcancel" | "note-type-editor-cancel" | "note_type_editor_cancel" => {
            parsed(EngramAppEvent::NoteTypeEditorCancel)
        }
        "importanki" | "import-anki" | "import_anki" => parsed(EngramAppEvent::ImportAnki),
        "exportanki" | "export-anki" | "export_anki" => parsed(EngramAppEvent::ExportAnki),
        "addnote" | "add-note" | "add_note" => parsed(EngramAppEvent::AddNote),
        "savenote" | "save-note" | "save_note" | "notesave" | "note-save" | "note_save"
        | "upsertnote" | "upsert-note" | "upsert_note" => parsed(EngramAppEvent::SaveNote),
        "addnotetype" | "add-note-type" | "add_note_type" => parsed(EngramAppEvent::AddNoteType),
        "savenotetype" | "save-note-type" | "save_note_type" | "notetypesave"
        | "note-type-save" | "note_type_save" | "upsertnotetype" | "upsert-note-type"
        | "upsert_note_type" => parsed(EngramAppEvent::SaveNoteType),
        "deletenote" | "delete-note" | "delete_note" => parsed(EngramAppEvent::DeleteNote),
        "deletenotetype" | "delete-note-type" | "delete_note_type" => {
            parsed(EngramAppEvent::DeleteNoteType)
        }
        _ => Err(format!("unknown Engram app event: {event_name}")),
    }
}

fn parsed_event(
    kind: EngramAppEvent,
    card_id: Option<String>,
    number_value: Option<f64>,
    text_value: Option<String>,
    bool_value: Option<bool>,
) -> ParsedEngramAppEvent {
    ParsedEngramAppEvent {
        kind,
        card_id,
        number_value,
        text_value,
        bool_value,
        payload: None,
    }
}

fn note_from_app_event(
    parsed: &ParsedEngramAppEvent,
    state: &AppState,
    deck_id: &str,
    now: u64,
) -> Result<Note, String> {
    let payload = parsed
        .payload
        .as_ref()
        .ok_or_else(|| "onSaveNote requires a JSON payload".to_string())?;
    let note_payload = payload.get("note").unwrap_or(payload);
    let note_id = explicit_note_id_from_app_event(parsed, state)
        .ok_or_else(|| "onSaveNote is missing a noteId".to_string())?;
    let existing_note = state.notes.iter().find(|note| note.id == note_id);
    let note_type_id = string_field(note_payload, &["noteTypeId", "note_type_id"])
        .or_else(|| existing_note.map(|note| note.note_type_id.clone()))
        .ok_or_else(|| "onSaveNote is missing a noteTypeId".to_string())?;
    let deck_id = string_field(note_payload, &["deckId", "deck_id"])
        .or_else(|| existing_note.map(|note| note.deck_id.clone()))
        .unwrap_or_else(|| deck_id.to_string());
    let note_type = state
        .note_types
        .iter()
        .find(|note_type| note_type.id == note_type_id);

    let fields = note_fields_from_payload(note_payload, existing_note, note_type)?;
    let tags = note_payload
        .get("tags")
        .and_then(parse_note_tags_value)
        .or_else(|| existing_note.map(|note| note.tags.clone()))
        .unwrap_or_default();
    let created_at = integer_field(note_payload, &["createdAt", "created_at"])
        .or_else(|| existing_note.map(|note| note.created_at))
        .unwrap_or(now);
    let updated_at = integer_field(note_payload, &["updatedAt", "updated_at"]).unwrap_or(now);

    Ok(Note {
        id: note_id,
        note_type_id,
        deck_id,
        fields,
        tags,
        created_at,
        updated_at,
    })
}

fn explicit_note_id_from_app_event(
    parsed: &ParsedEngramAppEvent,
    state: &AppState,
) -> Option<String> {
    let payload = parsed.payload.as_ref()?;
    let note_payload = payload.get("note").unwrap_or(payload);
    string_field(note_payload, &["noteId", "note_id", "id"]).or_else(|| {
        parsed.card_id.as_ref().and_then(|card_id| {
            state
                .cards
                .iter()
                .find(|card| card.id == *card_id)
                .and_then(|card| card.lineage.as_ref())
                .map(|lineage| lineage.note_id.clone())
        })
    })
}

fn note_type_from_app_event(
    parsed: &ParsedEngramAppEvent,
    state: &AppState,
    now: u64,
) -> Result<engram_core::NoteType, String> {
    let payload = parsed
        .payload
        .as_ref()
        .ok_or_else(|| "onSaveNoteType requires a JSON payload".to_string())?;
    let note_type_payload = payload.get("noteType").unwrap_or(payload);
    let note_type_id = explicit_note_type_id_from_app_event(parsed)
        .ok_or_else(|| "onSaveNoteType is missing a noteTypeId".to_string())?;
    let existing = state
        .note_types
        .iter()
        .find(|note_type| note_type.id == note_type_id);
    let mut note_type = existing.cloned().unwrap_or(engram_core::NoteType {
        id: note_type_id.clone(),
        name: String::new(),
        fields: Vec::new(),
        templates: Vec::new(),
        stylesheet: None,
        created_at: now,
        updated_at: now,
    });

    note_type.id = note_type_id;
    if let Some(name) = string_field(note_type_payload, &["name"]) {
        note_type.name = name;
    }
    if note_type.name.trim().is_empty() {
        return Err("onSaveNoteType is missing a name".to_string());
    }
    if let Some(fields) = note_type_payload.get("fields") {
        note_type.fields = serde_json::from_value::<Vec<engram_core::FieldDef>>(fields.clone())
            .map_err(|error| format!("invalid note type fields: {error}"))?;
    }
    if let Some(templates) = note_type_payload.get("templates") {
        note_type.templates =
            serde_json::from_value::<Vec<engram_core::CardTemplate>>(templates.clone())
                .map_err(|error| format!("invalid note type templates: {error}"))?;
    }
    if let Some(stylesheet) = note_type_payload.get("stylesheet") {
        note_type.stylesheet = match stylesheet {
            Value::Null => None,
            Value::String(value) if value.trim().is_empty() => None,
            Value::String(value) => Some(value.clone()),
            _ => return Err("note type stylesheet must be a string or null".to_string()),
        };
    }
    note_type.created_at = integer_field(note_type_payload, &["createdAt", "created_at"])
        .unwrap_or(note_type.created_at);
    note_type.updated_at =
        integer_field(note_type_payload, &["updatedAt", "updated_at"]).unwrap_or(now);

    Ok(note_type)
}

fn explicit_note_type_id_from_app_event(parsed: &ParsedEngramAppEvent) -> Option<String> {
    let payload = parsed.payload.as_ref()?;
    let note_type_payload = payload.get("noteType").unwrap_or(payload);
    string_field(
        note_type_payload,
        &["noteTypeId", "note_type_id", "modelId", "model_id", "id"],
    )
}

fn note_fields_from_payload(
    note_payload: &Value,
    existing_note: Option<&Note>,
    note_type: Option<&engram_core::NoteType>,
) -> Result<Vec<NoteFieldValue>, String> {
    let mut values = existing_note
        .map(|note| {
            note.fields
                .iter()
                .map(|field| (field.field_id.clone(), field.value.clone()))
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();

    if let Some(fields_value) = note_payload.get("fields") {
        apply_note_field_updates(&mut values, fields_value, note_type)?;
    }

    let mut field_ids = note_type
        .map(|note_type| {
            note_type
                .fields
                .iter()
                .map(|field| field.id.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| {
            existing_note
                .map(|note| {
                    note.fields
                        .iter()
                        .map(|field| field.field_id.clone())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        });
    for field_id in values.keys() {
        if !field_ids.iter().any(|existing| existing == field_id) {
            field_ids.push(field_id.clone());
        }
    }

    Ok(field_ids
        .into_iter()
        .map(|field_id| NoteFieldValue {
            value: values.remove(&field_id).unwrap_or_default(),
            field_id,
        })
        .collect())
}

fn apply_note_field_updates(
    values: &mut HashMap<String, String>,
    fields_value: &Value,
    note_type: Option<&engram_core::NoteType>,
) -> Result<(), String> {
    match fields_value {
        Value::Array(fields) => {
            for field in fields {
                let field_id = note_field_id_from_value(field, note_type)
                    .ok_or_else(|| "note field is missing an id or name".to_string())?;
                let value = field
                    .get("value")
                    .and_then(parse_json_text_value)
                    .unwrap_or_default();
                values.insert(field_id, value);
            }
            Ok(())
        }
        Value::Object(fields) => {
            for (field_id, value) in fields {
                let field_id = resolve_note_field_id(field_id, note_type);
                values.insert(field_id, parse_json_text_value(value).unwrap_or_default());
            }
            Ok(())
        }
        _ => Err("onSaveNote fields must be an array or object".to_string()),
    }
}

fn note_field_id_from_value(
    field: &Value,
    note_type: Option<&engram_core::NoteType>,
) -> Option<String> {
    string_field(field, &["fieldId", "field_id", "id"]).or_else(|| {
        string_field(field, &["name"]).map(|name| resolve_note_field_id(&name, note_type))
    })
}

fn resolve_note_field_id(raw: &str, note_type: Option<&engram_core::NoteType>) -> String {
    let trimmed = raw.trim();
    if let Some(note_type) = note_type {
        if let Some(field) = note_type.fields.iter().find(|field| field.id == trimmed) {
            return field.id.clone();
        }
        if let Some(field) = note_type
            .fields
            .iter()
            .find(|field| field.name.eq_ignore_ascii_case(trimmed))
        {
            return field.id.clone();
        }
    }
    trimmed.to_string()
}

fn parse_note_tags_value(value: &Value) -> Option<Vec<String>> {
    let tags = match value {
        Value::Array(values) => values
            .iter()
            .filter_map(parse_json_text_value)
            .collect::<Vec<_>>(),
        Value::String(raw) => raw.split_whitespace().map(str::to_string).collect(),
        Value::Null => Vec::new(),
        _ => return None,
    };
    Some(normalize_note_tags(tags))
}

fn normalize_note_tags(tags: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::new();
    for tag in tags {
        let tag = tag.trim();
        if tag.is_empty() {
            continue;
        }
        if normalized
            .iter()
            .any(|existing: &String| existing.eq_ignore_ascii_case(tag))
        {
            continue;
        }
        normalized.push(tag.to_string());
    }
    normalized
}

fn string_field(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(*key))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn integer_field(value: &Value, keys: &[&str]) -> Option<u64> {
    keys.iter()
        .find_map(|key| value.get(*key))
        .and_then(|value| {
            value
                .as_u64()
                .or_else(|| value.as_str()?.trim().parse().ok())
        })
}

fn parse_nonnegative_index(value: f64, label: &str) -> Result<usize, String> {
    if !value.is_finite() {
        return Err(format!("{label} index must be a finite number"));
    }
    if value < 0.0 {
        return Err(format!("{label} index must be non-negative"));
    }
    if value > usize::MAX as f64 {
        return Err(format!("{label} index is too large"));
    }
    Ok(value.round() as usize)
}

fn parse_json_number_value(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|raw| raw.trim().parse().ok()))
}

fn parse_json_text_value(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(str::to_string)
        .or_else(|| value.as_f64().map(|number| number.to_string()))
}

fn parse_json_bool_value(value: &Value) -> Option<bool> {
    value.as_bool().or_else(|| {
        value
            .as_str()
            .and_then(|raw| match raw.trim().to_ascii_lowercase().as_str() {
                "true" | "1" | "yes" | "on" => Some(true),
                "false" | "0" | "no" | "off" => Some(false),
                _ => None,
            })
    })
}

fn apply_deck_option_text_change(
    options: &mut DeckOptions,
    field: DeckOptionField,
    value: &str,
) -> Result<(), String> {
    match field {
        DeckOptionField::LearningStepsMinutes => {
            options.learning_steps_minutes = parse_step_minutes(value, "learning steps")?;
        }
        DeckOptionField::RelearningStepsMinutes => {
            options.relearning_steps_minutes = parse_step_minutes(value, "relearning steps")?;
        }
        DeckOptionField::LeechAction => {
            options.leech_action = parse_leech_action(value)?;
        }
        DeckOptionField::FsrsParameters => {
            options.fsrs_parameters = parse_f64_list(value, "FSRS parameters")?;
        }
        DeckOptionField::FsrsParameterSearch => {
            options.fsrs_parameter_search = value.trim().to_string();
        }
        DeckOptionField::IgnoreReviewHistoryBefore => {
            options.ignore_review_history_before = value.trim().to_string();
        }
        DeckOptionField::EasyDaysPercentages => {
            options.easy_days_percentages = parse_f64_list(value, "easy day factors")?;
        }
        DeckOptionField::BuryNewSiblings
        | DeckOptionField::BuryReviewSiblings
        | DeckOptionField::BuryInterdayLearningSiblings => {
            return Err("deck option field does not accept text values".to_string());
        }
        _ => return Err("deck option field does not accept text values".to_string()),
    }
    Ok(())
}

fn parse_leech_action(value: &str) -> Result<LeechAction, String> {
    let normalized = value
        .trim()
        .to_ascii_lowercase()
        .replace(['_', ' '], "-");
    match normalized.as_str() {
        "suspend" | "0" => Ok(LeechAction::Suspend),
        "tag-only" | "tagonly" | "tag" | "1" => Ok(LeechAction::TagOnly),
        _ => Err("leech action must be suspend or tag-only".to_string()),
    }
}

fn parse_step_minutes(value: &str, label: &str) -> Result<Vec<u32>, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    trimmed
        .split(|ch: char| ch == ',' || ch == ';' || ch.is_whitespace())
        .filter(|part| !part.trim().is_empty())
        .map(|part| {
            part.parse::<u32>()
                .map_err(|_| format!("{label} must be whole minutes separated by commas"))
        })
        .collect()
}

fn parse_f64_list(value: &str, label: &str) -> Result<Vec<f64>, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    trimmed
        .split(|ch: char| ch == ',' || ch == ';' || ch.is_whitespace())
        .filter(|part| !part.trim().is_empty())
        .map(|part| {
            let value = part
                .parse::<f64>()
                .map_err(|_| format!("{label} must be numbers separated by commas"))?;
            if value.is_finite() {
                Ok(value)
            } else {
                Err(format!("{label} must contain finite numbers"))
            }
        })
        .collect()
}

fn apply_deck_option_number_change(
    options: &mut DeckOptions,
    field: DeckOptionField,
    value: f64,
) -> Result<(), String> {
    match field {
        DeckOptionField::LearningStepsMinutes
        | DeckOptionField::RelearningStepsMinutes
        | DeckOptionField::LeechAction
        | DeckOptionField::FsrsParameters
        | DeckOptionField::FsrsParameterSearch
        | DeckOptionField::IgnoreReviewHistoryBefore
        | DeckOptionField::EasyDaysPercentages
        | DeckOptionField::BuryNewSiblings
        | DeckOptionField::BuryReviewSiblings
        | DeckOptionField::BuryInterdayLearningSiblings => {
            return Err("deck option field does not accept numeric values".to_string());
        }
        DeckOptionField::NewCardsPerDay => {
            options.new_cards_per_day = deck_option_count(value, "new cards/day")?;
        }
        DeckOptionField::ReviewsPerDay => {
            options.reviews_per_day = deck_option_count(value, "reviews/day")?;
        }
        DeckOptionField::GraduatingIntervalDays => {
            options.graduating_interval_days = deck_option_days(value, "graduating interval")?;
        }
        DeckOptionField::EasyIntervalDays => {
            options.easy_interval_days = deck_option_days(value, "easy interval")?;
        }
        DeckOptionField::InitialEaseFactor => {
            options.initial_ease_factor = deck_option_positive_multiplier(value, "initial ease")?;
        }
        DeckOptionField::MaximumIntervalDays => {
            options.maximum_interval_days = deck_option_days(value, "maximum interval")?;
        }
        DeckOptionField::ReviewIntervalModifier => {
            options.review_interval_modifier =
                deck_option_positive_multiplier(value, "interval modifier")?;
        }
        DeckOptionField::HardIntervalMultiplier => {
            options.hard_interval_multiplier =
                deck_option_positive_multiplier(value, "hard multiplier")?;
        }
        DeckOptionField::EasyBonusMultiplier => {
            options.easy_bonus_multiplier = deck_option_positive_multiplier(value, "easy bonus")?;
        }
        DeckOptionField::LapseIntervalMultiplier => {
            options.lapse_interval_multiplier =
                deck_option_non_negative_number(value, "lapse multiplier")?;
        }
        DeckOptionField::LeechThreshold => {
            options.leech_threshold = deck_option_count(value, "leech threshold")?;
        }
        DeckOptionField::DesiredRetention => {
            options.desired_retention = deck_option_retention(value, "desired retention")?;
        }
        DeckOptionField::HistoricalRetention => {
            options.historical_retention = deck_option_retention(value, "historical retention")?;
        }
    }
    Ok(())
}

fn apply_deck_option_bool_change(
    options: &mut DeckOptions,
    field: DeckOptionField,
    checked: bool,
) -> Result<(), String> {
    match field {
        DeckOptionField::BuryNewSiblings => options.bury_new_siblings = checked,
        DeckOptionField::BuryReviewSiblings => options.bury_review_siblings = checked,
        DeckOptionField::BuryInterdayLearningSiblings => {
            options.bury_interday_learning_siblings = checked;
        }
        _ => return Err("deck option field does not accept checked values".to_string()),
    }
    Ok(())
}

fn deck_option_count(value: f64, label: &str) -> Result<u32, String> {
    deck_option_u32(value, 0, label)
}

fn deck_option_days(value: f64, label: &str) -> Result<u32, String> {
    deck_option_u32(value, 1, label)
}

fn deck_option_u32(value: f64, min: u32, label: &str) -> Result<u32, String> {
    if !value.is_finite() {
        return Err(format!("{label} must be a finite number"));
    }
    if value < 0.0 {
        return Err(format!("{label} must be non-negative"));
    }
    let rounded = value.round().max(min as f64);
    if rounded > u32::MAX as f64 {
        return Err(format!("{label} is too large"));
    }
    Ok(rounded as u32)
}

fn deck_option_positive_multiplier(value: f64, label: &str) -> Result<f64, String> {
    if !value.is_finite() {
        return Err(format!("{label} must be a finite number"));
    }
    if value <= 0.0 {
        return Err(format!("{label} must be greater than zero"));
    }
    Ok(value)
}

fn deck_option_non_negative_number(value: f64, label: &str) -> Result<f64, String> {
    if !value.is_finite() {
        return Err(format!("{label} must be a finite number"));
    }
    if value < 0.0 {
        return Err(format!("{label} must be non-negative"));
    }
    Ok(value)
}

fn deck_option_retention(value: f64, label: &str) -> Result<f64, String> {
    if !value.is_finite() {
        return Err(format!("{label} must be a finite number"));
    }
    if !(0.0..=1.0).contains(&value) || value == 0.0 {
        return Err(format!("{label} must be between 0 and 1"));
    }
    Ok(value)
}

fn required_browser_event_card_id(
    state: &AppState,
    browser: &BrowserSessionState,
    card_id: Option<String>,
    action: &str,
    now: u64,
    current_deck_id: Option<&str>,
) -> Result<String, String> {
    let card_id = card_id
        .map(|card_id| card_id.trim().to_string())
        .filter(|card_id| !card_id.is_empty())
        .or_else(|| {
            selected_browser_row(state, browser, now, current_deck_id).map(|row| row.card_id)
        })
        .ok_or_else(|| format!("cannot {action} browser row without a card id"))?;
    if state.cards.iter().any(|card| card.id == card_id) {
        Ok(card_id)
    } else {
        Err(format!("cannot {action} unknown browser card: {card_id}"))
    }
}

fn browser_event_tag_value(
    browser: &BrowserSessionState,
    explicit_value: Option<&str>,
    action: &str,
) -> Result<String, String> {
    let tag = explicit_value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| browser.active_tag_edit());
    if tag.is_empty() {
        Err(format!("cannot {action} empty browser tag"))
    } else {
        Ok(tag)
    }
}

fn browser_event_flag_value(
    text_value: Option<&str>,
    number_value: Option<f64>,
) -> Result<Option<CardFlag>, String> {
    if let Some(value) = text_value {
        return parse_browser_flag_text(value);
    }
    if let Some(value) = number_value {
        return parse_browser_flag_number(value);
    }
    Ok(None)
}

fn parse_browser_flag_text(value: &str) -> Result<Option<CardFlag>, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "" | "0" | "none" | "no flag" | "clear" | "unflagged" => Ok(None),
        "1" | "red" => Ok(Some(CardFlag::Red)),
        "2" | "orange" => Ok(Some(CardFlag::Orange)),
        "3" | "green" => Ok(Some(CardFlag::Green)),
        "4" | "blue" => Ok(Some(CardFlag::Blue)),
        "5" | "pink" => Ok(Some(CardFlag::Pink)),
        "6" | "turquoise" => Ok(Some(CardFlag::Turquoise)),
        "7" | "purple" => Ok(Some(CardFlag::Purple)),
        other => Err(format!("unknown browser card flag: {other}")),
    }
}

fn parse_browser_flag_number(value: f64) -> Result<Option<CardFlag>, String> {
    if !value.is_finite() {
        return Err("browser card flag must be finite".to_string());
    }
    if value.fract() != 0.0 {
        return Err("browser card flag number must be an integer".to_string());
    }
    match value as i64 {
        0 => Ok(None),
        1 => Ok(Some(CardFlag::Red)),
        2 => Ok(Some(CardFlag::Orange)),
        3 => Ok(Some(CardFlag::Green)),
        4 => Ok(Some(CardFlag::Blue)),
        5 => Ok(Some(CardFlag::Pink)),
        6 => Ok(Some(CardFlag::Turquoise)),
        7 => Ok(Some(CardFlag::Purple)),
        other => Err(format!("unknown browser card flag number: {other}")),
    }
}

fn mark_or_unmark_card(state: &AppState, card_id: String, now: u64) -> AppState {
    if card_is_marked(state, &card_id) {
        reduce(state, engram_core::EngramCommand::UnmarkCard { card_id })
    } else {
        reduce(
            state,
            engram_core::EngramCommand::MarkCard {
                card_id,
                marked_at: now,
            },
        )
    }
}

fn card_is_marked(state: &AppState, card_id: &str) -> bool {
    state
        .card_progress
        .iter()
        .find(|progress| progress.card_id == card_id)
        .and_then(|progress| progress.marked_at)
        .is_some()
}

fn suspend_or_unsuspend_card(state: &AppState, card_id: String, now: u64) -> AppState {
    if card_is_suspended(state, &card_id) {
        reduce(state, engram_core::EngramCommand::UnsuspendCard { card_id })
    } else {
        reduce(
            state,
            engram_core::EngramCommand::SuspendCard {
                card_id,
                suspended_at: now,
            },
        )
    }
}

fn card_is_suspended(state: &AppState, card_id: &str) -> bool {
    state
        .card_progress
        .iter()
        .find(|progress| progress.card_id == card_id)
        .is_some_and(|progress| {
            progress.suspended_at.is_some() || progress.state == CardState::Suspended
        })
}

fn active_session_id(state: &AppState, action: &str) -> Result<String, String> {
    state
        .active_session
        .as_ref()
        .map(|active| active.session_id.clone())
        .ok_or_else(|| format!("cannot {action} without an active session"))
}

fn current_active_card_id(state: &AppState, action: &str) -> Result<String, String> {
    let active_session = state
        .active_session
        .as_ref()
        .ok_or_else(|| format!("cannot {action} without an active session"))?;
    active_session
        .queue
        .get(active_session.current_index)
        .map(|card| card.id.clone())
        .ok_or_else(|| format!("cannot {action} without a current card"))
}

fn rating_label(rating: Rating) -> &'static str {
    match rating {
        Rating::Again => "again",
        Rating::Hard => "hard",
        Rating::Good => "good",
        Rating::Easy => "easy",
    }
}

#[derive(Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum FacadeCommand {
    LoadState {
        state: AppState,
    },
    CreateDeck {
        id: String,
        name: String,
        description: String,
        created_at: u64,
    },
    UpdateDeck {
        deck_id: String,
        name: String,
        description: String,
    },
    SetDeckOptions {
        deck_id: String,
        options: DeckOptions,
    },
    DeleteDeck {
        deck_id: String,
    },
    UpsertNoteType {
        note_type: engram_core::NoteType,
        #[serde(default)]
        materialize_cards_at: Option<u64>,
    },
    DeleteNoteType {
        note_type_id: String,
    },
    UpsertNote {
        note: engram_core::Note,
        #[serde(default)]
        materialize_cards_at: Option<u64>,
    },
    DeleteNote {
        note_id: String,
    },
    AddNoteTags {
        note_ids: Vec<String>,
        tags: Vec<String>,
        updated_at: u64,
    },
    RemoveNoteTags {
        note_ids: Vec<String>,
        tags: Vec<String>,
        updated_at: u64,
    },
    AddCardTags {
        card_ids: Vec<String>,
        tags: Vec<String>,
        updated_at: u64,
    },
    RemoveCardTags {
        card_ids: Vec<String>,
        tags: Vec<String>,
        updated_at: u64,
    },
    RenameNoteTypeField {
        note_type_id: String,
        field_id: String,
        name: String,
        updated_at: u64,
    },
    UpsertMediaAsset {
        asset: MediaAssetRecord,
    },
    DeleteMediaAsset {
        asset_id: String,
    },
    DeleteMediaAssets {
        asset_ids: Vec<String>,
    },
    CreateCard {
        id: String,
        deck_id: String,
        front: String,
        back: String,
        created_at: u64,
        #[serde(default)]
        lineage: Option<CardLineage>,
    },
    UpdateCard {
        card_id: String,
        front: String,
        back: String,
    },
    DeleteCard {
        card_id: String,
    },
    SuspendCard {
        card_id: String,
        suspended_at: u64,
    },
    UnsuspendCard {
        card_id: String,
    },
    BuryCard {
        card_id: String,
        buried_at: u64,
        buried_until: u64,
    },
    BuryCardSiblings {
        card_id: String,
        buried_at: u64,
        buried_until: u64,
    },
    UnburyCard {
        card_id: String,
    },
    SetCardFlag {
        card_id: String,
        flag: Option<CardFlag>,
        flagged_at: u64,
    },
    MarkCard {
        card_id: String,
        marked_at: u64,
    },
    UnmarkCard {
        card_id: String,
    },
    StartSession {
        session_id: String,
        deck_id: String,
        queue: Vec<Card>,
        started_at: u64,
    },
    RevealCurrentCard,
    RateCard {
        review_id: String,
        session_id: String,
        card_id: String,
        rating: Rating,
        reviewed_at: u64,
        #[serde(default)]
        deck_options: Option<DeckOptions>,
        #[serde(default)]
        bury_siblings_until: Option<u64>,
    },
    UndoLastReview {
        session_id: String,
    },
    AdvanceSession,
    CompleteSession {
        session_id: String,
        ended_at: u64,
    },
}

impl FacadeCommand {
    fn into_core_command(self) -> engram_core::EngramCommand {
        match self {
            Self::LoadState { state } => engram_core::EngramCommand::LoadState(state),
            Self::CreateDeck {
                id,
                name,
                description,
                created_at,
            } => engram_core::EngramCommand::CreateDeck {
                id,
                name,
                description,
                created_at,
            },
            Self::UpdateDeck {
                deck_id,
                name,
                description,
            } => engram_core::EngramCommand::UpdateDeck {
                deck_id,
                name,
                description,
            },
            Self::SetDeckOptions { deck_id, options } => {
                engram_core::EngramCommand::SetDeckOptions { deck_id, options }
            }
            Self::DeleteDeck { deck_id } => engram_core::EngramCommand::DeleteDeck { deck_id },
            Self::UpsertNoteType {
                note_type,
                materialize_cards_at,
            } => engram_core::EngramCommand::UpsertNoteType {
                note_type,
                materialize_cards_at,
            },
            Self::DeleteNoteType { note_type_id } => {
                engram_core::EngramCommand::DeleteNoteType { note_type_id }
            }
            Self::UpsertNote {
                note,
                materialize_cards_at,
            } => engram_core::EngramCommand::UpsertNote {
                note,
                materialize_cards_at,
            },
            Self::DeleteNote { note_id } => engram_core::EngramCommand::DeleteNote { note_id },
            Self::AddNoteTags {
                note_ids,
                tags,
                updated_at,
            } => engram_core::EngramCommand::AddNoteTags {
                note_ids,
                tags,
                updated_at,
            },
            Self::RemoveNoteTags {
                note_ids,
                tags,
                updated_at,
            } => engram_core::EngramCommand::RemoveNoteTags {
                note_ids,
                tags,
                updated_at,
            },
            Self::AddCardTags {
                card_ids,
                tags,
                updated_at,
            } => engram_core::EngramCommand::AddCardTags {
                card_ids,
                tags,
                updated_at,
            },
            Self::RemoveCardTags {
                card_ids,
                tags,
                updated_at,
            } => engram_core::EngramCommand::RemoveCardTags {
                card_ids,
                tags,
                updated_at,
            },
            Self::RenameNoteTypeField {
                note_type_id,
                field_id,
                name,
                updated_at,
            } => engram_core::EngramCommand::RenameNoteTypeField {
                note_type_id,
                field_id,
                name,
                updated_at,
            },
            Self::UpsertMediaAsset { asset } => {
                engram_core::EngramCommand::UpsertMediaAsset { asset }
            }
            Self::DeleteMediaAsset { asset_id } => {
                engram_core::EngramCommand::DeleteMediaAsset { asset_id }
            }
            Self::DeleteMediaAssets { asset_ids } => {
                engram_core::EngramCommand::DeleteMediaAssets { asset_ids }
            }
            Self::CreateCard {
                id,
                deck_id,
                front,
                back,
                created_at,
                lineage,
            } => engram_core::EngramCommand::CreateCard {
                id,
                deck_id,
                front,
                back,
                created_at,
                lineage,
            },
            Self::UpdateCard {
                card_id,
                front,
                back,
            } => engram_core::EngramCommand::UpdateCard {
                card_id,
                front,
                back,
            },
            Self::DeleteCard { card_id } => engram_core::EngramCommand::DeleteCard { card_id },
            Self::SuspendCard {
                card_id,
                suspended_at,
            } => engram_core::EngramCommand::SuspendCard {
                card_id,
                suspended_at,
            },
            Self::UnsuspendCard { card_id } => {
                engram_core::EngramCommand::UnsuspendCard { card_id }
            }
            Self::BuryCard {
                card_id,
                buried_at,
                buried_until,
            } => engram_core::EngramCommand::BuryCard {
                card_id,
                buried_at,
                buried_until,
            },
            Self::BuryCardSiblings {
                card_id,
                buried_at,
                buried_until,
            } => engram_core::EngramCommand::BuryCardSiblings {
                card_id,
                buried_at,
                buried_until,
            },
            Self::UnburyCard { card_id } => engram_core::EngramCommand::UnburyCard { card_id },
            Self::SetCardFlag {
                card_id,
                flag,
                flagged_at,
            } => engram_core::EngramCommand::SetCardFlag {
                card_id,
                flag,
                flagged_at,
            },
            Self::MarkCard { card_id, marked_at } => {
                engram_core::EngramCommand::MarkCard { card_id, marked_at }
            }
            Self::UnmarkCard { card_id } => engram_core::EngramCommand::UnmarkCard { card_id },
            Self::StartSession {
                session_id,
                deck_id,
                queue,
                started_at,
            } => engram_core::EngramCommand::StartSession {
                session_id,
                deck_id,
                queue,
                started_at,
            },
            Self::RevealCurrentCard => engram_core::EngramCommand::RevealCurrentCard,
            Self::RateCard {
                review_id,
                session_id,
                card_id,
                rating,
                reviewed_at,
                deck_options,
                bury_siblings_until,
            } => match (deck_options, bury_siblings_until) {
                (Some(deck_options), Some(buried_until)) => {
                    engram_core::EngramCommand::RateCardWithOptionsAndBurySiblings {
                        review_id,
                        session_id,
                        card_id,
                        rating,
                        reviewed_at,
                        deck_options,
                        buried_until,
                    }
                }
                (Some(deck_options), None) => engram_core::EngramCommand::RateCardWithOptions {
                    review_id,
                    session_id,
                    card_id,
                    rating,
                    reviewed_at,
                    deck_options,
                },
                (None, Some(buried_until)) => engram_core::EngramCommand::RateCardAndBurySiblings {
                    review_id,
                    session_id,
                    card_id,
                    rating,
                    reviewed_at,
                    buried_until,
                },
                (None, None) => engram_core::EngramCommand::RateCard {
                    review_id,
                    session_id,
                    card_id,
                    rating,
                    reviewed_at,
                },
            },
            Self::UndoLastReview { session_id } => {
                engram_core::EngramCommand::UndoLastReview { session_id }
            }
            Self::AdvanceSession => engram_core::EngramCommand::AdvanceSession,
            Self::CompleteSession {
                session_id,
                ended_at,
            } => engram_core::EngramCommand::CompleteSession {
                session_id,
                ended_at,
            },
        }
    }
}

fn catch_json(run: impl FnOnce() -> Result<String, String>) -> String {
    match catch_unwind(AssertUnwindSafe(run)) {
        Ok(Ok(value)) => value,
        Ok(Err(message)) => error_json(&message),
        Err(_) => error_json("engram core panic"),
    }
}

fn ok_with(key: &str, value: &impl serde::Serialize) -> String {
    let mut object = serde_json::Map::new();
    object.insert("ok".to_string(), Value::Bool(true));
    object.insert(
        key.to_string(),
        serde_json::to_value(value).unwrap_or(Value::Null),
    );
    Value::Object(object).to_string()
}

fn parse_deck_options(
    deck_options_json: &str,
    state: &AppState,
    deck_id: &str,
) -> Result<DeckOptions, String> {
    if deck_options_json.trim().is_empty() {
        return Ok(deck_options_for_state(state, deck_id));
    }

    serde_json::from_str(deck_options_json).map_err(|err| format!("invalid deck options: {err}"))
}

fn error_json(message: &str) -> String {
    json!({ "ok": false, "error": message }).to_string()
}

fn error_json_with_token(message: &str, token: &str) -> String {
    json!({ "ok": false, "error": message, "token": token }).to_string()
}

fn error_json_with_row(message: &str, row: Option<usize>) -> String {
    json!({ "ok": false, "error": message, "row": row }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const NOW: u64 = 1_700_000_000_000;

    #[test]
    fn dispatch_create_deck_returns_camel_case_state_json() {
        let mut session = EngramSession::new();
        let result = session.dispatch(
            r#"{
                "type": "createDeck",
                "id": "deck",
                "name": "Tamil",
                "description": "Script",
                "createdAt": 1700000000000
            }"#,
        );
        let value: Value = serde_json::from_str(&result).unwrap();

        assert_eq!(value["ok"], true);
        assert_eq!(value["state"]["decks"][0]["createdAt"], NOW);
        assert!(value["state"].get("cardProgress").is_some());
    }

    #[test]
    fn dispatch_set_deck_options_persists_partial_camel_case_options() {
        let mut session = EngramSession::new();
        let value: Value = serde_json::from_str(&session.dispatch(
            r#"{
                "type": "setDeckOptions",
                "deckId": "deck",
                "options": {
                    "newCardsPerDay": 12,
                    "maximumIntervalDays": 90,
                    "initialEaseFactor": 2.8,
                    "reviewIntervalModifier": 0.75,
                    "hardIntervalMultiplier": 1.4,
                    "easyBonusMultiplier": 1.6
                }
            }"#,
        ))
        .unwrap();

        assert_eq!(value["ok"], true);
        assert_eq!(value["state"]["deckOptions"][0]["deckId"], "deck");
        assert_eq!(
            value["state"]["deckOptions"][0]["options"]["newCardsPerDay"],
            12
        );
        assert_eq!(
            value["state"]["deckOptions"][0]["options"]["maximumIntervalDays"],
            90
        );
        assert_eq!(
            value["state"]["deckOptions"][0]["options"]["initialEaseFactor"],
            2.8
        );
        assert_eq!(
            value["state"]["deckOptions"][0]["options"]["reviewIntervalModifier"],
            0.75
        );
        assert_eq!(
            value["state"]["deckOptions"][0]["options"]["learningStepsMinutes"],
            json!([1, 10])
        );
        assert_eq!(
            value["state"]["deckOptions"][0]["options"]["buryNewSiblings"],
            true
        );
        assert_eq!(
            value["state"]["deckOptions"][0]["options"]["buryReviewSiblings"],
            true
        );
    }

    #[test]
    fn filtered_deck_facade_methods_rebuild_and_empty_shared_state() {
        let mut session = EngramSession::new();
        session.load_snapshot(
            r#"{
                "decks": [
                    {"id": "spanish", "name": "Spanish", "description": "", "createdAt": 1700000000000},
                    {"id": "filtered", "name": "Filtered::Today", "description": "Custom study", "createdAt": 1700000000000}
                ],
                "noteTypes": [],
                "notes": [],
                "cards": [
                    {"id": "card", "deckId": "spanish", "front": "hola", "back": "hello", "createdAt": 1700000000000}
                ],
                "cardProgress": [],
                "sessions": [],
                "reviews": [],
                "deckOptions": [],
                "externalSources": [],
                "mediaAssets": [],
                "activeSession": null
            }"#,
        );

        let rebuilt: Value = serde_json::from_str(&session.rebuild_filtered_deck(
            "filtered",
            "deck:Spanish",
            10,
            false,
            NOW,
        ))
        .unwrap();

        assert_eq!(rebuilt["ok"], true);
        assert_eq!(rebuilt["state"]["cards"][0]["deckId"], "filtered");
        assert_eq!(rebuilt["state"]["externalSources"][0]["data"]["dyn"], "1");
        assert_eq!(
            rebuilt["state"]["externalSources"][1]["data"]["originalDeckId"],
            "spanish"
        );

        let emptied: Value =
            serde_json::from_str(&session.empty_filtered_deck("filtered")).unwrap();
        assert_eq!(emptied["ok"], true);
        assert_eq!(emptied["state"]["cards"][0]["deckId"], "spanish");
        assert_eq!(
            emptied["state"]["externalSources"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn dispatch_media_asset_commands_use_shared_state_contract() {
        let mut session = EngramSession::new();
        let value: Value = serde_json::from_str(&session.dispatch(
            r#"{
                "type": "upsertMediaAsset",
                "asset": {
                    "id": "anki-media:0",
                    "archiveName": "0",
                    "filename": "audio/hola.mp3",
                    "data": [109, 112, 51]
                }
            }"#,
        ))
        .unwrap();
        assert_eq!(value["ok"], true);
        assert_eq!(
            value["state"]["mediaAssets"][0]["filename"],
            "audio/hola.mp3"
        );

        let value: Value = serde_json::from_str(&session.dispatch(
            r#"{
                "type": "upsertMediaAsset",
                "asset": {
                    "id": "anki-media:1",
                    "archiveName": "1",
                    "filename": "images/card.png",
                    "data": [112, 110, 103]
                }
            }"#,
        ))
        .unwrap();
        assert_eq!(value["state"]["mediaAssets"].as_array().unwrap().len(), 2);

        let value: Value = serde_json::from_str(&session.dispatch(
            r#"{
                "type": "upsertMediaAsset",
                "asset": {
                    "id": "anki-media:0",
                    "archiveName": "0",
                    "filename": "audio/hola-v2.mp3",
                    "data": [118, 50]
                }
            }"#,
        ))
        .unwrap();
        assert_eq!(
            value["state"]["mediaAssets"][0]["filename"],
            "audio/hola-v2.mp3"
        );
        assert_eq!(value["state"]["mediaAssets"][0]["data"], json!([118, 50]));

        let value: Value = serde_json::from_str(&session.dispatch(
            r#"{
                "type": "deleteMediaAssets",
                "assetIds": ["anki-media:1", "missing"]
            }"#,
        ))
        .unwrap();
        assert_eq!(value["state"]["mediaAssets"].as_array().unwrap().len(), 1);
        assert_eq!(value["state"]["mediaAssets"][0]["id"], "anki-media:0");

        let value: Value = serde_json::from_str(&session.dispatch(
            r#"{
                "type": "deleteMediaAsset",
                "assetId": "anki-media:0"
            }"#,
        ))
        .unwrap();
        assert!(value["state"]["mediaAssets"].as_array().unwrap().is_empty());
    }

    #[test]
    fn dispatch_media_delete_commands_prune_media_source_records() {
        let mut session = EngramSession::new();
        let loaded: Value = serde_json::from_str(&session.load_snapshot(
            r#"{
                "decks": [],
                "noteTypes": [],
                "notes": [],
                "cards": [],
                "cardProgress": [],
                "sessions": [],
                "reviews": [],
                "deckOptions": [],
                "externalSources": [
                    {"target":"media","targetId":"anki-media:0","source":"anki-v11","originalId":"0","data":{}},
                    {"target":"media","targetId":"anki-media:1","source":"anki-v11","originalId":"1","data":{}},
                    {"target":"note","targetId":"note","source":"anki-v11","originalId":"10","data":{}}
                ],
                "mediaAssets": [
                    {"id":"anki-media:0","archiveName":"0","filename":"audio/hola.mp3","data":[109,112,51]},
                    {"id":"anki-media:1","archiveName":"1","filename":"images/card.png","data":[112,110,103]}
                ],
                "activeSession": null
            }"#,
        ))
        .unwrap();
        assert_eq!(loaded["ok"], true);

        let value: Value = serde_json::from_str(&session.dispatch(
            r#"{
                "type": "deleteMediaAssets",
                "assetIds": ["anki-media:1", "missing"]
            }"#,
        ))
        .unwrap();
        assert_eq!(value["state"]["mediaAssets"].as_array().unwrap().len(), 1);
        assert!(value["state"]["externalSources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|source| { source["target"] == "media" && source["targetId"] == "anki-media:0" }));
        assert!(!value["state"]["externalSources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|source| { source["target"] == "media" && source["targetId"] == "anki-media:1" }));

        let value: Value = serde_json::from_str(&session.dispatch(
            r#"{
                "type": "deleteMediaAsset",
                "assetId": "anki-media:0"
            }"#,
        ))
        .unwrap();
        assert!(value["state"]["mediaAssets"].as_array().unwrap().is_empty());
        assert!(!value["state"]["externalSources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|source| source["target"] == "media"));
        assert!(value["state"]["externalSources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|source| source["target"] == "note" && source["targetId"] == "note"));
    }

    #[test]
    fn dispatch_rename_note_type_field_migrates_templates() {
        let mut session = EngramSession::new();
        session.load_snapshot(
            r#"{
                "decks": [],
                "noteTypes": [{
                    "id": "basic",
                    "name": "Basic",
                    "fields": [
                        {"id": "front", "name": "Front", "required": true, "ordinal": 0},
                        {"id": "back", "name": "Back", "required": true, "ordinal": 1}
                    ],
                    "templates": [{
                        "id": "forward",
                        "name": "Forward",
                        "frontTemplate": "{{Front}}",
                        "backTemplate": "{{Back}}",
                        "requiredFieldNames": ["Front", "Back"],
                        "ordinal": 0
                    }],
                    "createdAt": 1700000000000,
                    "updatedAt": 1700000000000
                }],
                "notes": [],
                "cards": [],
                "cardProgress": [],
                "sessions": [],
                "reviews": [],
                "activeSession": null
            }"#,
        );

        let value: Value = serde_json::from_str(&session.dispatch(
            r#"{
                "type": "renameNoteTypeField",
                "noteTypeId": "basic",
                "fieldId": "front",
                "name": "Prompt",
                "updatedAt": 1700000000001
            }"#,
        ))
        .unwrap();

        assert_eq!(value["ok"], true);
        assert_eq!(
            value["state"]["noteTypes"][0]["fields"][0]["name"],
            "Prompt"
        );
        assert_eq!(
            value["state"]["noteTypes"][0]["templates"][0]["frontTemplate"],
            "{{Prompt}}"
        );
        assert_eq!(
            value["state"]["noteTypes"][0]["templates"][0]["requiredFieldNames"][0],
            "Prompt"
        );
        assert_eq!(value["state"]["noteTypes"][0]["updatedAt"], NOW + 1);
    }

    #[test]
    fn dispatch_upsert_and_delete_note_type_syncs_notes_and_cards() {
        let mut session = EngramSession::new();
        session.load_snapshot(
            r#"{
                "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
                "noteTypes": [],
                "notes": [{
                    "id": "note",
                    "noteTypeId": "basic",
                    "deckId": "deck",
                    "fields": [
                        {"fieldId": "front", "value": "amma"},
                        {"fieldId": "back", "value": "mother"}
                    ],
                    "tags": ["tamil"],
                    "createdAt": 1700000000000,
                    "updatedAt": 1700000000000
                }],
                "cards": [],
                "cardProgress": [],
                "sessions": [],
                "reviews": [],
                "activeSession": null
            }"#,
        );

        let created: Value = serde_json::from_str(&session.dispatch(
            r#"{
                "type": "upsertNoteType",
                "noteType": {
                    "id": "basic",
                    "name": "Basic",
                    "fields": [
                        {"id": "front", "name": "Front", "required": true, "ordinal": 0},
                        {"id": "back", "name": "Back", "required": true, "ordinal": 1}
                    ],
                    "templates": [{
                        "id": "forward",
                        "name": "Forward",
                        "frontTemplate": "{{Front}}",
                        "backTemplate": "{{Back}}",
                        "requiredFieldNames": ["Front", "Back"],
                        "ordinal": 0
                    }],
                    "createdAt": 1700000000000,
                    "updatedAt": 1700000000000
                },
                "materializeCardsAt": 1700000000001
            }"#,
        ))
        .unwrap();

        assert_eq!(created["ok"], true);
        assert_eq!(created["state"]["noteTypes"][0]["id"], "basic");
        assert_eq!(created["state"]["cards"][0]["id"], "note::forward");
        assert_eq!(created["state"]["cards"][0]["front"], "amma");
        assert_eq!(created["state"]["cards"][0]["createdAt"], NOW + 1);
        assert_eq!(
            created["state"]["cards"][0]["lineage"]["templateId"],
            "forward"
        );

        let deleted: Value = serde_json::from_str(&session.dispatch(
            r#"{
                "type": "deleteNoteType",
                "noteTypeId": "basic"
            }"#,
        ))
        .unwrap();

        assert_eq!(deleted["ok"], true);
        assert!(deleted["state"]["noteTypes"].as_array().unwrap().is_empty());
        assert!(deleted["state"]["notes"].as_array().unwrap().is_empty());
        assert!(deleted["state"]["cards"].as_array().unwrap().is_empty());
    }

    #[test]
    fn dispatch_upsert_and_delete_note_syncs_generated_cards() {
        let mut session = EngramSession::new();
        session.load_snapshot(
            r#"{
                "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
                "noteTypes": [{
                    "id": "basic",
                    "name": "Basic",
                    "fields": [
                        {"id": "front", "name": "Front", "required": true, "ordinal": 0},
                        {"id": "back", "name": "Back", "required": true, "ordinal": 1}
                    ],
                    "templates": [{
                        "id": "forward",
                        "name": "Forward",
                        "frontTemplate": "{{Front}}",
                        "backTemplate": "{{Back}}",
                        "requiredFieldNames": ["Front", "Back"],
                        "ordinal": 0
                    }],
                    "createdAt": 1700000000000,
                    "updatedAt": 1700000000000
                }],
                "notes": [],
                "cards": [],
                "cardProgress": [],
                "sessions": [],
                "reviews": [],
                "activeSession": null
            }"#,
        );

        let created: Value = serde_json::from_str(&session.dispatch(
            r#"{
                "type": "upsertNote",
                "note": {
                    "id": "note",
                    "noteTypeId": "basic",
                    "deckId": "deck",
                    "fields": [
                        {"fieldId": "front", "value": "amma"},
                        {"fieldId": "back", "value": "mother"}
                    ],
                    "tags": ["tamil"],
                    "createdAt": 1700000000000,
                    "updatedAt": 1700000000000
                },
                "materializeCardsAt": 1700000000001
            }"#,
        ))
        .unwrap();

        assert_eq!(created["ok"], true);
        assert_eq!(created["state"]["notes"][0]["id"], "note");
        assert_eq!(created["state"]["cards"][0]["id"], "note::forward");
        assert_eq!(created["state"]["cards"][0]["front"], "amma");
        assert_eq!(
            created["state"]["cards"][0]["lineage"]["templateId"],
            "forward"
        );

        let deleted: Value = serde_json::from_str(&session.dispatch(
            r#"{
                "type": "deleteNote",
                "noteId": "note"
            }"#,
        ))
        .unwrap();

        assert_eq!(deleted["ok"], true);
        assert!(deleted["state"]["notes"].as_array().unwrap().is_empty());
        assert!(deleted["state"]["cards"].as_array().unwrap().is_empty());
    }

    #[test]
    fn dispatch_tag_commands_mutate_lineaged_note_tags() {
        let mut session = EngramSession::new();
        session.load_snapshot(
            r#"{
                "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
                "noteTypes": [],
                "notes": [{
                    "id": "note",
                    "noteTypeId": "basic",
                    "deckId": "deck",
                    "fields": [
                        {"fieldId": "front", "value": "amma"},
                        {"fieldId": "back", "value": "mother"}
                    ],
                    "tags": ["tamil"],
                    "createdAt": 1700000000000,
                    "updatedAt": 1700000000000
                }],
                "cards": [
                    {
                        "id": "note::forward",
                        "deckId": "deck",
                        "front": "amma",
                        "back": "mother",
                        "createdAt": 1700000000000,
                        "lineage": {
                            "noteId": "note",
                            "noteTypeId": "basic",
                            "templateId": "forward",
                            "ordinal": 0
                        }
                    },
                    {
                        "id": "standalone",
                        "deckId": "deck",
                        "front": "one",
                        "back": "1",
                        "createdAt": 1700000000000
                    }
                ],
                "cardProgress": [],
                "sessions": [],
                "reviews": [],
                "activeSession": null
            }"#,
        );

        let tagged: Value = serde_json::from_str(&session.dispatch(
            r#"{
                "type": "addNoteTags",
                "noteIds": ["note"],
                "tags": ["script tamil", "SCRIPT"],
                "updatedAt": 1700000000001
            }"#,
        ))
        .unwrap();

        assert_eq!(tagged["ok"], true);
        assert_eq!(
            tagged["state"]["notes"][0]["tags"],
            json!(["tamil", "script"])
        );
        assert_eq!(tagged["state"]["notes"][0]["updatedAt"], NOW + 1);

        let card_tagged: Value = serde_json::from_str(&session.dispatch(
            r#"{
                "type": "addCardTags",
                "cardIds": ["note::forward", "standalone"],
                "tags": ["grammar"],
                "updatedAt": 1700000000002
            }"#,
        ))
        .unwrap();

        assert_eq!(
            card_tagged["state"]["notes"][0]["tags"],
            json!(["tamil", "script", "grammar"])
        );
        assert_eq!(card_tagged["state"]["notes"][0]["updatedAt"], NOW + 2);

        let card_removed: Value = serde_json::from_str(&session.dispatch(
            r#"{
                "type": "removeCardTags",
                "cardIds": ["note::forward"],
                "tags": ["TAMIL missing"],
                "updatedAt": 1700000000003
            }"#,
        ))
        .unwrap();

        assert_eq!(
            card_removed["state"]["notes"][0]["tags"],
            json!(["script", "grammar"])
        );
        assert_eq!(card_removed["state"]["notes"][0]["updatedAt"], NOW + 3);

        let note_removed: Value = serde_json::from_str(&session.dispatch(
            r#"{
                "type": "removeNoteTags",
                "noteIds": ["note"],
                "tags": ["SCRIPT"],
                "updatedAt": 1700000000004
            }"#,
        ))
        .unwrap();

        assert_eq!(
            note_removed["state"]["notes"][0]["tags"],
            json!(["grammar"])
        );
        assert_eq!(note_removed["state"]["notes"][0]["updatedAt"], NOW + 4);
    }

    #[test]
    fn snapshot_round_trips_through_load_snapshot() {
        let mut session = EngramSession::new();
        session.dispatch(
            r#"{
                "type": "createDeck",
                "id": "deck",
                "name": "Tamil",
                "description": "Script",
                "createdAt": 1700000000000
            }"#,
        );
        let snapshot_result: Value = serde_json::from_str(&session.snapshot()).unwrap();
        let snapshot = snapshot_result["state"].to_string();

        let mut restored = EngramSession::new();
        let loaded: Value = serde_json::from_str(&restored.load_snapshot(&snapshot)).unwrap();

        assert_eq!(loaded["ok"], true);
        assert_eq!(loaded["state"]["decks"][0]["id"], "deck");
    }

    #[test]
    fn export_backup_uses_versioned_engram_shape() {
        let mut session = EngramSession::new();
        session.dispatch(
            r#"{
                "type": "createDeck",
                "id": "deck",
                "name": "Tamil",
                "description": "Script",
                "createdAt": 1700000000000
            }"#,
        );
        session.dispatch(
            r#"{
                "type": "startSession",
                "sessionId": "session",
                "deckId": "deck",
                "queue": [],
                "startedAt": 1700000000000
            }"#,
        );

        let value: Value = serde_json::from_str(&session.export_backup(NOW + 1)).unwrap();

        assert_eq!(value["ok"], true);
        assert_eq!(value["snapshot"]["app"], "engram");
        assert_eq!(value["snapshot"]["version"], 1);
        assert_eq!(value["snapshot"]["exportedAt"], NOW + 1);
        assert_eq!(value["snapshot"]["decks"][0]["id"], "deck");
        assert!(value["snapshot"].get("activeSession").is_none());
    }

    #[test]
    fn import_backup_accepts_existing_web_backup_shape() {
        let mut session = EngramSession::new();
        let backup = r#"{
            "app": "engram",
            "version": 1,
            "exportedAt": 1700000000001,
            "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
            "cards": [{"id":"card","deckId":"deck","front":"letter-a","back":"a","createdAt":1700000000000}],
            "cardProgress": [],
            "sessions": [],
            "reviews": []
        }"#;

        let value: Value = serde_json::from_str(&session.import_backup(backup)).unwrap();

        assert_eq!(value["ok"], true);
        assert_eq!(value["state"]["decks"][0]["id"], "deck");
        assert_eq!(value["state"]["cards"][0]["front"], "letter-a");
        assert!(value["state"]["noteTypes"].as_array().unwrap().is_empty());
        assert!(value["state"]["notes"].as_array().unwrap().is_empty());
        assert_eq!(value["state"]["activeSession"], Value::Null);
    }

    #[test]
    fn import_backup_rejects_wrong_app_or_version() {
        let mut session = EngramSession::new();
        let wrong_app = r#"{
            "app": "other",
            "version": 1,
            "exportedAt": 1700000000001,
            "decks": [],
            "cards": [],
            "cardProgress": [],
            "sessions": [],
            "reviews": []
        }"#;
        let value: Value = serde_json::from_str(&session.import_backup(wrong_app)).unwrap();
        assert_eq!(value["ok"], false);
        assert_eq!(value["error"], "The selected file is not an Engram backup.");

        let wrong_version = r#"{
            "app": "engram",
            "version": 99,
            "exportedAt": 1700000000001,
            "decks": [],
            "cards": [],
            "cardProgress": [],
            "sessions": [],
            "reviews": []
        }"#;
        let value: Value = serde_json::from_str(&session.import_backup(wrong_version)).unwrap();
        assert_eq!(value["ok"], false);
        assert_eq!(value["error"], "Unsupported Engram backup version: 99");
    }

    #[test]
    fn build_queue_uses_loaded_state() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
            "noteTypes": [],
            "notes": [],
            "cards": [
                {"id":"card","deckId":"deck","front":"letter-a","back":"a","createdAt":1700000000000},
                {"id":"card-2","deckId":"deck","front":"letter-aa","back":"aa","createdAt":1700000000001}
            ],
            "cardProgress": [],
            "sessions": [],
            "reviews": [],
            "deckOptions": [{
                "deckId": "deck",
                "options": {
                    "newCardsPerDay": 1,
                    "reviewsPerDay": 200,
                    "learningStepsMinutes": [1, 10],
                    "relearningStepsMinutes": [10],
                    "graduatingIntervalDays": 1,
                    "easyIntervalDays": 4,
                    "lapseIntervalMultiplier": 0.0
                }
            }],
            "activeSession": null
        }"#;

        session.load_snapshot(snapshot);
        let value: Value = serde_json::from_str(&session.build_queue("deck", NOW)).unwrap();

        assert_eq!(value["ok"], true);
        assert_eq!(value["queue"][0]["id"], "card");
        assert_eq!(value["queue"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn build_queue_uses_imported_anki_new_card_positions() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
            "noteTypes": [],
            "notes": [],
            "cards": [
                {"id":"native-new","deckId":"deck","front":"native","back":"n","createdAt":1700000000000},
                {"id":"anki-late","deckId":"deck","front":"late","back":"l","createdAt":1700000000001},
                {"id":"due","deckId":"deck","front":"due","back":"d","createdAt":1700000000002},
                {"id":"anki-early","deckId":"deck","front":"early","back":"e","createdAt":1700000000003}
            ],
            "cardProgress": [{
                "cardId":"due",
                "state":"review",
                "interval":3,
                "easeFactor":2.5,
                "nextDueAt":1699999999900,
                "learningStepIndex":null,
                "buriedUntil":null,
                "suspendedAt":null,
                "timesSeen":1,
                "timesCorrect":1,
                "timesIncorrect":0,
                "lastSeenAt":1699999990000
            }],
            "sessions": [],
            "reviews": [],
            "deckOptions": [{
                "deckId": "deck",
                "options": {
                    "newCardsPerDay": 3,
                    "reviewsPerDay": 1,
                    "learningStepsMinutes": [1, 10],
                    "relearningStepsMinutes": [10],
                    "graduatingIntervalDays": 1,
                    "easyIntervalDays": 4,
                    "lapseIntervalMultiplier": 0.0
                }
            }],
            "externalSources": [
                {
                    "target":"card",
                    "targetId":"anki-late",
                    "source":"anki-v11",
                    "originalId":"anki-late",
                    "data":{"kind":"0","queue":"0","due":"200"}
                },
                {
                    "target":"card",
                    "targetId":"anki-early",
                    "source":"anki-v11",
                    "originalId":"anki-early",
                    "data":{"kind":"0","queue":"0","due":"25"}
                }
            ],
            "activeSession": null
        }"#;

        session.load_snapshot(snapshot);
        let value: Value = serde_json::from_str(&session.build_queue("deck", NOW)).unwrap();
        let ids: Vec<_> = value["queue"]
            .as_array()
            .unwrap()
            .iter()
            .map(|card| card["id"].as_str().unwrap())
            .collect();

        assert_eq!(ids, vec!["due", "anki-early", "anki-late", "native-new"]);
    }

    #[test]
    fn build_queue_uses_imported_anki_review_schedules() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
            "noteTypes": [],
            "notes": [],
            "cards": [
                {"id":"native-new","deckId":"deck","front":"native","back":"n","createdAt":1700000000000},
                {"id":"review-due","deckId":"deck","front":"due","back":"d","createdAt":1700000000001},
                {"id":"review-future","deckId":"deck","front":"future","back":"f","createdAt":1700000000002}
            ],
            "cardProgress": [],
            "sessions": [],
            "reviews": [],
            "deckOptions": [{
                "deckId": "deck",
                "options": {
                    "newCardsPerDay": 1,
                    "reviewsPerDay": 2,
                    "learningStepsMinutes": [1, 10],
                    "relearningStepsMinutes": [10],
                    "graduatingIntervalDays": 1,
                    "easyIntervalDays": 4,
                    "lapseIntervalMultiplier": 0.0
                }
            }],
            "externalSources": [
                {
                    "target":"collection",
                    "targetId":"collection",
                    "source":"anki-v11",
                    "originalId":"1",
                    "data":{"createdAtDays":"19475"}
                },
                {
                    "target":"card",
                    "targetId":"review-due",
                    "source":"anki-v11",
                    "originalId":"review-due",
                    "data":{"kind":"2","queue":"2","due":"200","interval":"7","factor":"2500"}
                },
                {
                    "target":"card",
                    "targetId":"review-future",
                    "source":"anki-v11",
                    "originalId":"review-future",
                    "data":{"kind":"2","queue":"2","due":"203","interval":"30","factor":"2500"}
                }
            ],
            "activeSession": null
        }"#;

        session.load_snapshot(snapshot);
        let value: Value = serde_json::from_str(&session.build_queue("deck", NOW)).unwrap();
        let ids: Vec<_> = value["queue"]
            .as_array()
            .unwrap()
            .iter()
            .map(|card| card["id"].as_str().unwrap())
            .collect();

        assert_eq!(ids, vec!["review-due", "native-new"]);
    }

    #[test]
    fn parent_deck_queue_and_stats_include_child_decks() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [
                {"id":"parent","name":"Tamil","description":"Script","createdAt":1700000000000},
                {"id":"child","name":"Tamil::Verbs","description":"Grammar","createdAt":1700000000001},
                {"id":"sibling","name":"Spanish","description":"Other","createdAt":1700000000002}
            ],
            "noteTypes": [],
            "notes": [],
            "cards": [
                {"id":"parent-due","deckId":"parent","front":"root","back":"r","createdAt":1700000000000},
                {"id":"child-due","deckId":"child","front":"verb","back":"v","createdAt":1700000000001},
                {"id":"child-new","deckId":"child","front":"fresh","back":"f","createdAt":1700000000002},
                {"id":"sibling-due","deckId":"sibling","front":"otro","back":"other","createdAt":1700000000003}
            ],
            "cardProgress": [
                {
                    "cardId":"parent-due",
                    "state":"review",
                    "interval":3,
                    "easeFactor":2.5,
                    "nextDueAt":1699999999900,
                    "learningStepIndex":null,
                    "buriedUntil":null,
                    "suspendedAt":null,
                    "timesSeen":1,
                    "timesCorrect":1,
                    "timesIncorrect":0,
                    "lastSeenAt":1699999990000
                },
                {
                    "cardId":"child-due",
                    "state":"review",
                    "interval":3,
                    "easeFactor":2.5,
                    "nextDueAt":1699999999950,
                    "learningStepIndex":null,
                    "buriedUntil":null,
                    "suspendedAt":null,
                    "timesSeen":1,
                    "timesCorrect":1,
                    "timesIncorrect":0,
                    "lastSeenAt":1699999990000
                },
                {
                    "cardId":"sibling-due",
                    "state":"review",
                    "interval":3,
                    "easeFactor":2.5,
                    "nextDueAt":1699999999800,
                    "learningStepIndex":null,
                    "buriedUntil":null,
                    "suspendedAt":null,
                    "timesSeen":1,
                    "timesCorrect":1,
                    "timesIncorrect":0,
                    "lastSeenAt":1699999990000
                }
            ],
            "sessions": [],
            "reviews": [],
            "deckOptions": [{
                "deckId": "parent",
                "options": {
                    "newCardsPerDay": 2,
                    "reviewsPerDay": 3,
                    "learningStepsMinutes": [1, 10],
                    "relearningStepsMinutes": [10],
                    "graduatingIntervalDays": 1,
                    "easyIntervalDays": 4,
                    "lapseIntervalMultiplier": 0.0
                }
            }],
            "activeSession": null
        }"#;

        session.load_snapshot(snapshot);
        let queue: Value = serde_json::from_str(&session.build_queue("parent", NOW)).unwrap();
        let ids: Vec<_> = queue["queue"]
            .as_array()
            .unwrap()
            .iter()
            .map(|card| card["id"].as_str().unwrap())
            .collect();

        assert_eq!(ids, vec!["parent-due", "child-due", "child-new"]);

        let stats: Value = serde_json::from_str(&session.deck_stats("parent", NOW)).unwrap();
        assert_eq!(stats["stats"]["total"], 3);
        assert_eq!(stats["stats"]["dueCount"], 2);
        assert_eq!(stats["stats"]["newCount"], 1);
    }

    #[test]
    fn deck_stats_reports_suspended_and_buried_counts() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
            "noteTypes": [],
            "notes": [],
            "cards": [
                {"id":"due","deckId":"deck","front":"due","back":"1","createdAt":1700000000000},
                {"id":"suspended","deckId":"deck","front":"hidden","back":"s","createdAt":1700000000000},
                {"id":"buried","deckId":"deck","front":"hidden","back":"b","createdAt":1700000000000}
            ],
            "cardProgress": [
                {
                    "cardId":"due",
                    "state":"review",
                    "interval":3,
                    "easeFactor":2.5,
                    "nextDueAt":1699999999900,
                    "learningStepIndex":null,
                    "buriedUntil":null,
                    "suspendedAt":null,
                    "timesSeen":1,
                    "timesCorrect":1,
                    "timesIncorrect":0,
                    "lastSeenAt":1699999990000
                },
                {
                    "cardId":"suspended",
                    "state":"review",
                    "interval":3,
                    "easeFactor":2.5,
                    "nextDueAt":1699999999900,
                    "learningStepIndex":null,
                    "buriedUntil":null,
                    "suspendedAt":1700000000000,
                    "timesSeen":1,
                    "timesCorrect":1,
                    "timesIncorrect":0,
                    "lastSeenAt":1699999990000
                },
                {
                    "cardId":"buried",
                    "state":"review",
                    "interval":3,
                    "easeFactor":2.5,
                    "nextDueAt":1699999999900,
                    "learningStepIndex":null,
                    "buriedUntil":1700000060000,
                    "suspendedAt":null,
                    "timesSeen":1,
                    "timesCorrect":1,
                    "timesIncorrect":0,
                    "lastSeenAt":1699999990000
                }
            ],
            "sessions": [],
            "reviews": [],
            "deckOptions": [{
                "deckId": "deck",
                "options": {
                    "newCardsPerDay": 12,
                    "reviewsPerDay": 80,
                    "learningStepsMinutes": [1, 10],
                    "relearningStepsMinutes": [10],
                    "graduatingIntervalDays": 2,
                    "easyIntervalDays": 5,
                    "initialEaseFactor": 2.8,
                    "maximumIntervalDays": 90,
                    "reviewIntervalModifier": 0.75,
                    "hardIntervalMultiplier": 1.4,
                    "easyBonusMultiplier": 1.6,
                    "lapseIntervalMultiplier": 0.5,
                    "buryNewSiblings": false,
                    "buryReviewSiblings": true,
                    "buryInterdayLearningSiblings": false
                }
            }],
            "activeSession": null
        }"#;

        session.load_snapshot(snapshot);
        let value: Value = serde_json::from_str(&session.deck_stats("deck", NOW)).unwrap();

        assert_eq!(value["ok"], true);
        assert_eq!(value["stats"]["dueCount"], 1);
        assert_eq!(value["stats"]["suspendedCount"], 1);
        assert_eq!(value["stats"]["buriedCount"], 1);
    }

    #[test]
    fn daily_limits_report_usage_and_trim_queue() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
            "noteTypes": [],
            "notes": [],
            "cards": [
                {"id":"due-1","deckId":"deck","front":"due","back":"1","createdAt":1700000000000},
                {"id":"reviewed-today","deckId":"deck","front":"seen","back":"review","createdAt":1700000000000},
                {"id":"new-1","deckId":"deck","front":"seen","back":"new","createdAt":1700000000000},
                {"id":"new-2","deckId":"deck","front":"fresh","back":"2","createdAt":1700000000000},
                {"id":"new-3","deckId":"deck","front":"fresh","back":"3","createdAt":1700000000000}
            ],
            "cardProgress": [
                {
                    "cardId":"due-1",
                    "state":"review",
                    "interval":3,
                    "easeFactor":2.5,
                    "nextDueAt":1699999999900,
                    "learningStepIndex":null,
                    "buriedUntil":null,
                    "suspendedAt":null,
                    "timesSeen":1,
                    "timesCorrect":1,
                    "timesIncorrect":0,
                    "lastSeenAt":1699999990000
                },
                {
                    "cardId":"reviewed-today",
                    "state":"review",
                    "interval":3,
                    "easeFactor":2.5,
                    "nextDueAt":1700000060000,
                    "learningStepIndex":null,
                    "buriedUntil":null,
                    "suspendedAt":null,
                    "timesSeen":2,
                    "timesCorrect":2,
                    "timesIncorrect":0,
                    "lastSeenAt":1700000000020
                },
                {
                    "cardId":"new-1",
                    "state":"learning",
                    "interval":0,
                    "easeFactor":2.5,
                    "nextDueAt":1700000060000,
                    "learningStepIndex":0,
                    "buriedUntil":null,
                    "suspendedAt":null,
                    "timesSeen":1,
                    "timesCorrect":1,
                    "timesIncorrect":0,
                    "lastSeenAt":1700000000010
                }
            ],
            "sessions": [],
            "reviews": [
                {
                    "id":"new",
                    "sessionId":"session",
                    "cardId":"new-1",
                    "rating":"good",
                    "reviewedAt":1700000000010
                },
                {
                    "id":"review",
                    "sessionId":"session",
                    "cardId":"reviewed-today",
                    "rating":"good",
                    "reviewedAt":1700000000020,
                    "previousProgress":{
                        "cardId":"reviewed-today",
                        "state":"review",
                        "interval":3,
                        "easeFactor":2.5,
                        "nextDueAt":1699999999900,
                        "learningStepIndex":null,
                        "buriedUntil":null,
                        "suspendedAt":null,
                        "timesSeen":1,
                        "timesCorrect":1,
                        "timesIncorrect":0,
                        "lastSeenAt":1699999990000
                    }
                }
            ],
            "activeSession": null
        }"#;
        let options = r#"{"newCardsPerDay":2,"reviewsPerDay":2}"#;

        session.load_snapshot(snapshot);
        let usage: Value =
            serde_json::from_str(&session.daily_limit_usage("deck", NOW, NOW + 100, options))
                .unwrap();

        assert_eq!(usage["ok"], true);
        assert_eq!(usage["usage"]["newCardsSeen"], 1);
        assert_eq!(usage["usage"]["reviewCardsSeen"], 1);
        assert_eq!(usage["usage"]["remainingNewCards"], 1);
        assert_eq!(usage["usage"]["remainingReviews"], 1);

        let queue: Value = serde_json::from_str(&session.build_queue_with_daily_limits(
            "deck",
            NOW,
            NOW,
            NOW + 100,
            options,
        ))
        .unwrap();

        assert_eq!(queue["ok"], true);
        assert_eq!(queue["queue"][0]["id"], "due-1");
        assert_eq!(queue["queue"][1]["id"], "new-2");
        assert_eq!(queue["queue"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn session_progress_reports_active_review_counts() {
        let mut session = EngramSession::new();
        let empty: Value = serde_json::from_str(&session.session_progress()).unwrap();
        assert_eq!(empty["ok"], true);
        assert_eq!(empty["progress"], Value::Null);

        let snapshot = r#"{
            "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
            "noteTypes": [],
            "notes": [],
            "cards": [
                {"id":"card","deckId":"deck","front":"letter-a","back":"a","createdAt":1700000000000},
                {"id":"other","deckId":"deck","front":"letter-aa","back":"aa","createdAt":1700000000000}
            ],
            "cardProgress": [],
            "sessions": [],
            "reviews": [],
            "activeSession": null
        }"#;

        session.load_snapshot(snapshot);
        session.dispatch(
            r#"{
                "type": "startSession",
                "sessionId": "session",
                "deckId": "deck",
                "queue": [
                    {"id":"card","deckId":"deck","front":"letter-a","back":"a","createdAt":1700000000000},
                    {"id":"other","deckId":"deck","front":"letter-aa","back":"aa","createdAt":1700000000000}
                ],
                "startedAt": 1700000000000
            }"#,
        );
        session.dispatch(r#"{"type": "revealCurrentCard"}"#);
        session.dispatch(
            r#"{
                "type": "rateCard",
                "reviewId": "review",
                "sessionId": "session",
                "cardId": "card",
                "rating": "good",
                "reviewedAt": 1700000000000
            }"#,
        );
        session.dispatch(r#"{"type": "advanceSession"}"#);

        let value: Value = serde_json::from_str(&session.session_progress()).unwrap();

        assert_eq!(value["ok"], true);
        assert_eq!(value["progress"]["sessionId"], "session");
        assert_eq!(value["progress"]["deckId"], "deck");
        assert_eq!(value["progress"]["totalCards"], 2);
        assert_eq!(value["progress"]["currentIndex"], 1);
        assert_eq!(value["progress"]["currentPosition"], 2);
        assert_eq!(value["progress"]["remainingCards"], 1);
        assert_eq!(value["progress"]["cardsReviewed"], 1);
        assert_eq!(value["progress"]["cardsCorrect"], 1);
        assert_eq!(value["progress"]["revealed"], false);
        assert_eq!(value["progress"]["completed"], false);
    }

    #[test]
    fn engram_app_props_shape_matches_mosaic_slots() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
            "noteTypes": [{
                "id": "basic",
                "name": "Basic",
                "fields": [],
                "templates": [],
                "createdAt": 1700000000000,
                "updatedAt": 1700000000000
            }],
            "notes": [{
                "id": "note",
                "noteTypeId": "basic",
                "deckId": "deck",
                "fields": [],
                "tags": [],
                "createdAt": 1700000000000,
                "updatedAt": 1700000000000
            }],
            "cards": [
                {"id":"card","deckId":"deck","front":"letter-a","back":"a","createdAt":1700000000000},
                {"id":"other","deckId":"deck","front":"letter-aa","back":"aa","createdAt":1700000000000}
            ],
            "cardProgress": [],
            "mediaAssets": [
                {"id":"media:0","archiveName":"0","filename":"audio/amma.mp3","data":[109,112,51]}
            ],
            "sessions": [],
            "reviews": [
                {
                    "id":"review-good",
                    "sessionId":"session",
                    "cardId":"card",
                    "rating":"good",
                    "reviewedAt":1699999999900
                },
                {
                    "id":"review-again",
                    "sessionId":"session",
                    "cardId":"other",
                    "rating":"again",
                    "reviewedAt":1699999999950
                }
            ],
            "deckOptions": [{
                "deckId": "deck",
                "options": {
                    "newCardsPerDay": 12,
                    "reviewsPerDay": 80,
                    "learningStepsMinutes": [1, 10],
                    "relearningStepsMinutes": [10],
                    "graduatingIntervalDays": 2,
                    "easyIntervalDays": 5,
                    "initialEaseFactor": 2.8,
                    "maximumIntervalDays": 90,
                    "reviewIntervalModifier": 0.75,
                    "hardIntervalMultiplier": 1.4,
                    "easyBonusMultiplier": 1.6,
                    "lapseIntervalMultiplier": 0.5,
                    "leechThreshold": 6,
                    "desiredRetention": 0.92,
                    "fsrsParameters": [0.1, 1.2, 2.3],
                    "fsrsParameterSearch": "preset:\"Tamil\" -is:suspended",
                    "ignoreReviewHistoryBefore": "2024-01-02",
                    "historicalRetention": 0.86,
                    "easyDaysPercentages": [1.0, 0.9, 0.8, 1.1, 1.2, 1.0, 0.95],
                    "leechAction": "suspend",
                    "buryNewSiblings": false,
                    "buryReviewSiblings": true,
                    "buryInterdayLearningSiblings": false
                }
            }],
            "activeSession": null
        }"#;

        session.load_snapshot(snapshot);
        session.dispatch(
            r#"{
                "type": "startSession",
                "sessionId": "session",
                "deckId": "deck",
                "queue": [
                    {"id":"card","deckId":"deck","front":"letter-a","back":"a","createdAt":1700000000000},
                    {"id":"other","deckId":"deck","front":"letter-aa","back":"aa","createdAt":1700000000000}
                ],
                "startedAt": 1700000000000
            }"#,
        );
        session.dispatch(r#"{"type": "revealCurrentCard"}"#);

        let value: Value = serde_json::from_str(&session.engram_app_props("deck", NOW)).unwrap();

        assert_eq!(value["ok"], true);
        assert_eq!(value["props"]["app-title"], "Engram");
        assert_eq!(value["props"]["show-decks-screen"], true);
        assert_eq!(value["props"]["show-study-screen"], false);
        assert_eq!(value["props"]["show-browse-screen"], false);
        assert_eq!(value["props"]["show-add-screen"], false);
        assert_eq!(value["props"]["show-stats-screen"], false);
        assert_eq!(value["props"]["show-options-screen"], false);
        assert_eq!(value["props"]["deck-name"], "Tamil");
        assert_eq!(value["props"]["deck-total-value"], "2");
        assert_eq!(value["props"]["deck-new-value"], "2");
        assert_eq!(
            value["props"]["deck-options-settings-label"],
            "Deck options"
        );
        assert_eq!(value["props"]["deck-options-learning-steps-value"], "1, 10");
        assert_eq!(value["props"]["deck-options-relearning-steps-value"], "10");
        assert_eq!(value["props"]["deck-options-new-cards-value"], 12);
        assert_eq!(value["props"]["deck-options-reviews-value"], 80);
        assert_eq!(value["props"]["deck-options-graduating-interval-value"], 2);
        assert_eq!(value["props"]["deck-options-easy-interval-value"], 5);
        assert_eq!(value["props"]["deck-options-initial-ease-value"], 2.8);
        assert_eq!(value["props"]["deck-options-maximum-interval-value"], 90);
        assert_eq!(value["props"]["deck-options-interval-modifier-value"], 0.75);
        assert_eq!(value["props"]["deck-options-hard-multiplier-value"], 1.4);
        assert_eq!(value["props"]["deck-options-easy-bonus-value"], 1.6);
        assert_eq!(value["props"]["deck-options-lapse-multiplier-value"], 0.5);
        assert_eq!(value["props"]["deck-options-leech-threshold-value"], 6);
        assert_eq!(value["props"]["deck-options-desired-retention-value"], 0.92);
        assert_eq!(
            value["props"]["deck-options-fsrs-parameters-value"],
            "0.1, 1.2, 2.3"
        );
        assert_eq!(
            value["props"]["deck-options-fsrs-search-value"],
            "preset:\"Tamil\" -is:suspended"
        );
        assert_eq!(
            value["props"]["deck-options-ignore-review-history-before-value"],
            "2024-01-02"
        );
        assert_eq!(
            value["props"]["deck-options-historical-retention-value"],
            0.86
        );
        assert_eq!(
            value["props"]["deck-options-easy-days-percentages-value"],
            "1, 0.9, 0.8, 1.1, 1.2, 1, 0.95"
        );
        assert_eq!(
            value["props"]["deck-options-leech-action-label"],
            "Leech action"
        );
        assert_eq!(
            value["props"]["deck-options-leech-action-suspend-value"],
            true
        );
        assert_eq!(
            value["props"]["deck-options-leech-action-tag-only-value"],
            false
        );
        assert_eq!(
            value["props"]["deck-options-bury-new-siblings-label"],
            "Bury new siblings"
        );
        assert_eq!(
            value["props"]["deck-options-bury-new-siblings-value"],
            false
        );
        assert_eq!(
            value["props"]["deck-options-bury-review-siblings-value"],
            true
        );
        assert_eq!(
            value["props"]["deck-options-bury-interday-learning-siblings-value"],
            false
        );
        assert_eq!(value["props"]["history-label"], "Review history");
        assert_eq!(value["props"]["history-window-label"], "Lifetime");
        assert_eq!(value["props"]["history-total-value"], "2");
        assert_eq!(value["props"]["history-correct-value"], "1");
        assert_eq!(value["props"]["history-unique-value"], "2");
        assert_eq!(value["props"]["history-accuracy-value"], "50%");
        assert_eq!(value["props"]["history-again-value"], "1");
        assert_eq!(value["props"]["history-good-value"], "1");
        assert_eq!(value["props"]["history-first-value"], "1699999999900");
        assert_eq!(value["props"]["history-last-value"], "1699999999950");
        assert_eq!(value["props"]["collection-label"], "Collection");
        assert_eq!(value["props"]["collection-note-count-value"], "1");
        assert_eq!(value["props"]["collection-note-type-count-value"], "1");
        assert_eq!(value["props"]["collection-media-count-value"], "1");
        assert_eq!(value["props"]["collection-referenced-media-value"], "0");
        assert_eq!(value["props"]["collection-missing-media-value"], "0");
        assert_eq!(
            value["props"]["collection-missing-media-filenames"],
            json!([])
        );
        assert_eq!(value["props"]["collection-unused-media-value"], "1");
        assert_eq!(
            value["props"]["collection-unused-media-asset-ids"],
            json!(["media:0"])
        );
        assert_eq!(
            value["props"]["collection-prune-unused-media-label"],
            "Prune unused media"
        );
        assert_eq!(value["props"]["collection-import-label"], "Import Anki");
        assert_eq!(value["props"]["collection-export-label"], "Export Anki");
        assert_eq!(value["props"]["collection-add-note-label"], "Add note");
        assert_eq!(
            value["props"]["collection-add-note-type-label"],
            "Add note type"
        );
        assert_eq!(
            value["props"]["collection-delete-note-label"],
            "Delete note"
        );
        assert_eq!(
            value["props"]["collection-delete-note-type-label"],
            "Delete note type"
        );
        assert_eq!(value["props"]["browser-label"], "Card browser");
        assert_eq!(value["props"]["browser-query"], "is:due OR is:new");
        assert_eq!(value["props"]["browser-filter-label"], "State");
        assert_eq!(value["props"]["browser-filter-value"], "All");
        assert_eq!(
            value["props"]["browser-filter-options"],
            json!([
                "All",
                "New",
                "Due",
                "Learning",
                "Review",
                "Suspended",
                "Buried"
            ])
        );
        assert_eq!(value["props"]["browser-filter-open"], false);
        assert_eq!(
            value["props"]["browser-results-summary"],
            "2 matching cards"
        );
        assert_eq!(
            value["props"]["browser-results"],
            json!(["letter-a -> a", "letter-aa -> aa"])
        );
        assert_eq!(
            value["props"]["browser-result-card-ids"],
            json!(["card", "other"])
        );
        assert_eq!(
            value["props"]["browser-result-states"],
            json!(["new", "new"])
        );
        assert_eq!(value["props"]["browser-selected-index"], 0);
        assert_eq!(value["props"]["browser-selected-card-id"], "card");
        assert_eq!(value["props"]["browser-selected-state"], "new");
        assert_eq!(value["props"]["prompt"], "letter-a");
        assert_eq!(value["props"]["answer"], "a");
        assert_eq!(value["props"]["answer-visible"], true);
        assert_eq!(value["props"]["type-answer-active"], false);
        assert_eq!(value["props"]["type-answer-value"], "");
        assert_eq!(value["props"]["type-answer-expected"], "");
        assert_eq!(value["props"]["type-answer-correct"], false);
        assert_eq!(value["props"]["current-value"], "1 / 2");
        assert_eq!(value["props"]["remaining-value"], "2");
        assert_eq!(value["props"]["total-value"], "2");
        assert_eq!(value["props"]["action-undo-label"], "Undo");
        assert_eq!(value["props"]["action-bury-card-label"], "Bury card");
        assert_eq!(
            value["props"]["action-bury-siblings-label"],
            "Bury siblings"
        );
        assert_eq!(value["props"]["action-suspend-card-label"], "Suspend");
        assert_eq!(value["props"]["action-mark-label"], "Mark");
    }

    #[test]
    fn demo_session_lights_up_engram_app_props() {
        let session = EngramSession::new_demo();
        let value: Value = serde_json::from_str(&session.engram_app_props("", NOW)).unwrap();

        assert_eq!(value["ok"], true);
        assert_eq!(value["props"]["deck-name"], "Tamil::Script and Roots");
        assert_eq!(value["props"]["deck-total-value"], "2");
        assert_eq!(value["props"]["deck-new-value"], "1");
        assert_eq!(value["props"]["deck-due-value"], "1");
        assert_eq!(value["props"]["collection-note-count-value"], "5");
        assert_eq!(value["props"]["collection-note-type-count-value"], "1");
        assert_eq!(value["props"]["collection-media-count-value"], "0");
        assert_eq!(
            value["props"]["deck-names"],
            json!([
                "Tamil::Script and Roots",
                "Hindi::Devanagari",
                "Kannada::Script",
                "Spanish::Latin Roots"
            ])
        );
        assert_eq!(
            value["props"]["browser-result-card-ids"],
            json!([
                "card-tamil-amma",
                "card-tamil-uyir",
                "card-hindi-namaste",
                "card-kannada-amma",
                "card-spanish-hablar"
            ])
        );
    }

    #[test]
    fn engram_app_screen_events_drive_single_visible_surface() {
        let mut session = EngramSession::new();

        let initial: Value = serde_json::from_str(&session.engram_app_props("", NOW)).unwrap();
        assert_eq!(initial["props"]["show-decks-screen"], true);
        assert_eq!(initial["props"]["show-browse-screen"], false);

        let browse: Value =
            serde_json::from_str(&session.handle_engram_app_event("onShowBrowse", "", NOW))
                .unwrap();
        assert_eq!(browse["ok"], true);
        assert_eq!(browse["event"], "onShowBrowse");
        assert_eq!(browse["props"]["show-decks-screen"], false);
        assert_eq!(browse["props"]["show-browse-screen"], true);
        assert_eq!(browse["props"]["show-add-screen"], false);

        let study: Value =
            serde_json::from_str(&session.handle_engram_app_event("show-study", "", NOW + 1))
                .unwrap();
        assert_eq!(study["event"], "onShowStudy");
        assert_eq!(study["props"]["show-study-screen"], true);
        assert_eq!(study["props"]["show-browse-screen"], false);

        let add: Value =
            serde_json::from_str(&session.handle_engram_app_event("onAddNote", "", NOW + 2))
                .unwrap();
        assert_eq!(add["event"], "onAddNote");
        assert_eq!(add["props"]["show-add-screen"], true);
        assert_eq!(add["props"]["show-study-screen"], false);

        let options: Value =
            serde_json::from_str(&session.handle_engram_app_event("onAddNoteType", "", NOW + 3))
                .unwrap();
        assert_eq!(options["event"], "onAddNoteType");
        assert_eq!(options["props"]["show-options-screen"], true);
        assert_eq!(options["props"]["show-add-screen"], false);
    }

    #[test]
    fn engram_app_prunes_unused_media_assets_from_shared_state() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [{"id":"deck","name":"Spanish","description":"Media","createdAt":1700000000000}],
            "noteTypes": [],
            "notes": [],
            "cards": [
                {
                    "id":"card",
                    "deckId":"deck",
                    "front":"hola [sound:audio/hola.mp3] <img src=\"missing.png\">",
                    "back":"hello",
                    "createdAt":1700000000000
                }
            ],
            "cardProgress": [],
            "mediaAssets": [
                {"id":"media:audio","archiveName":"0","filename":"audio/hola.mp3","data":[109,112,51]},
                {"id":"media:unused","archiveName":"1","filename":"unused.png","data":[112,110,103]}
            ],
            "sessions": [],
            "reviews": [],
            "activeSession": null
        }"#;

        session.load_snapshot(snapshot);

        let pruned: Value = serde_json::from_str(&session.handle_engram_app_event(
            "onPruneUnusedMedia",
            "deck",
            NOW,
        ))
        .unwrap();

        assert_eq!(pruned["ok"], true);
        assert_eq!(pruned["event"], "onPruneUnusedMedia");
        assert_eq!(pruned["hostIntent"], Value::Null);
        assert_eq!(
            pruned["state"]["mediaAssets"],
            json!([
                {"id":"media:audio","archiveName":"0","filename":"audio/hola.mp3","data":[109,112,51]}
            ])
        );
        assert_eq!(pruned["props"]["collection-media-count-value"], "1");
        assert_eq!(pruned["props"]["collection-referenced-media-value"], "2");
        assert_eq!(pruned["props"]["collection-unused-media-value"], "0");
        assert_eq!(
            pruned["props"]["collection-unused-media-asset-ids"],
            json!([])
        );
        assert_eq!(pruned["props"]["collection-missing-media-value"], "1");
        assert_eq!(
            pruned["props"]["collection-missing-media-filenames"],
            json!(["missing.png"])
        );
    }

    #[test]
    fn engram_app_props_tracks_type_answer_without_leaking_expected_value() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [{"id":"deck","name":"Spanish","description":"Roots","createdAt":1700000000000}],
            "noteTypes": [{
                "id": "basic",
                "name": "Basic",
                "fields": [
                    {"id":"front","name":"Front","required":true,"ordinal":0},
                    {"id":"back","name":"Back","required":true,"ordinal":1}
                ],
                "templates": [{
                    "id":"forward",
                    "name":"Forward",
                    "frontTemplate":"{{Front}}{{type:nc:Back}}",
                    "backTemplate":"{{FrontSide}}<hr>{{Back}}",
                    "requiredFieldNames":["Front"],
                    "ordinal":0
                }],
                "createdAt": 1700000000000,
                "updatedAt": 1700000000000
            }],
            "notes": [{
                "id": "note",
                "noteTypeId": "basic",
                "deckId": "deck",
                "fields": [
                    {"fieldId":"front","value":"coffee"},
                    {"fieldId":"back","value":"caf\u00e9"}
                ],
                "tags": ["spanish","latin"],
                "createdAt": 1700000000000,
                "updatedAt": 1700000000000
            }],
            "cards": [{
                "id":"note::forward",
                "deckId":"deck",
                "front":"coffee[type answer: Back]",
                "back":"coffee[type answer: Back]<hr>caf\u00e9",
                "createdAt":1700000000000,
                "lineage":{"noteId":"note","noteTypeId":"basic","templateId":"forward","ordinal":0}
            }],
            "cardProgress": [],
            "sessions": [],
            "reviews": [],
            "activeSession": null
        }"#;

        session.load_snapshot(snapshot);
        session.dispatch(
            r#"{
                "type": "startSession",
                "sessionId": "session",
                "deckId": "deck",
                "queue": [{
                    "id":"note::forward",
                    "deckId":"deck",
                    "front":"coffee[type answer: Back]",
                    "back":"coffee[type answer: Back]<hr>caf\u00e9",
                    "createdAt":1700000000000,
                    "lineage":{"noteId":"note","noteTypeId":"basic","templateId":"forward","ordinal":0}
                }],
                "startedAt": 1700000000000
            }"#,
        );

        let initial: Value = serde_json::from_str(&session.engram_app_props("deck", NOW)).unwrap();
        assert_eq!(initial["props"]["type-answer-active"], true);
        assert_eq!(initial["props"]["type-answer-field"], "Back");
        assert_eq!(initial["props"]["type-answer-value"], "");
        assert_eq!(initial["props"]["type-answer-expected"], "");
        assert_eq!(initial["props"]["type-answer-ignore-combining"], true);
        assert_eq!(initial["props"]["answer-visible"], false);
        assert!(!initial["props"]["prompt"].as_str().unwrap().contains("caf"));

        let changed: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onTypeAnswerChange","value":"cafe"}"#,
            "deck",
            NOW,
        ))
        .unwrap();
        assert_eq!(changed["ok"], true);
        assert_eq!(changed["event"], "onTypeAnswerChange");
        assert_eq!(changed["props"]["type-answer-value"], "cafe");
        assert_eq!(changed["props"]["type-answer-expected"], "");
        assert_eq!(changed["props"]["type-answer-correct"], false);

        let revealed: Value =
            serde_json::from_str(&session.handle_engram_app_event("onReveal", "deck", NOW + 1))
                .unwrap();
        assert_eq!(revealed["props"]["answer-visible"], true);
        assert_eq!(revealed["props"]["type-answer-expected"], "caf\u{e9}");
        assert_eq!(revealed["props"]["type-answer-correct"], true);
        assert!(revealed["props"]["type-answer-comparison-value"]
            .as_str()
            .unwrap()
            .contains("Correct"));
    }

    #[test]
    fn handle_engram_app_event_reveals_rates_and_advances_shared_props() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
            "noteTypes": [],
            "notes": [],
            "cards": [
                {"id":"card","deckId":"deck","front":"letter-a","back":"a","createdAt":1700000000000},
                {"id":"other","deckId":"deck","front":"letter-aa","back":"aa","createdAt":1700000000000}
            ],
            "cardProgress": [],
            "sessions": [],
            "reviews": [],
            "activeSession": null
        }"#;

        session.load_snapshot(snapshot);
        session.dispatch(
            r#"{
                "type": "startSession",
                "sessionId": "session",
                "deckId": "deck",
                "queue": [
                    {"id":"card","deckId":"deck","front":"letter-a","back":"a","createdAt":1700000000000},
                    {"id":"other","deckId":"deck","front":"letter-aa","back":"aa","createdAt":1700000000000}
                ],
                "startedAt": 1700000000000
            }"#,
        );

        let revealed: Value =
            serde_json::from_str(&session.handle_engram_app_event("onReveal", "deck", NOW))
                .unwrap();
        assert_eq!(revealed["ok"], true);
        assert_eq!(revealed["event"], "onReveal");
        assert_eq!(revealed["props"]["prompt"], "letter-a");
        assert_eq!(revealed["props"]["answer-visible"], true);

        let rated: Value =
            serde_json::from_str(&session.handle_engram_app_event("good", "deck", NOW + 1))
                .unwrap();
        assert_eq!(rated["ok"], true);
        assert_eq!(rated["event"], "onGood");
        assert_eq!(rated["props"]["prompt"], "letter-aa");
        assert_eq!(rated["props"]["answer-visible"], false);
        assert_eq!(rated["props"]["current-value"], "2 / 2");
        assert_eq!(rated["state"]["reviews"][0]["cardId"], "card");
        assert_eq!(rated["state"]["reviews"][0]["rating"], "good");
        assert_eq!(rated["state"]["sessions"][0]["cardsReviewed"], 1);
        assert_eq!(rated["state"]["sessions"][0]["cardsCorrect"], 1);

        let undone: Value =
            serde_json::from_str(&session.handle_engram_app_event("onUndo", "deck", NOW + 2))
                .unwrap();
        assert_eq!(undone["ok"], true);
        assert_eq!(undone["event"], "onUndo");
        assert!(undone["state"]["reviews"].as_array().unwrap().is_empty());
        assert_eq!(undone["props"]["prompt"], "letter-a");
        assert_eq!(undone["props"]["answer-visible"], true);
        assert_eq!(undone["state"]["sessions"][0]["cardsReviewed"], 0);
        assert_eq!(undone["state"]["sessions"][0]["cardsCorrect"], 0);
    }

    #[test]
    fn handle_engram_app_rate_uses_deck_sibling_bury_options() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
            "noteTypes": [],
            "notes": [],
            "cards": [
                {
                    "id":"note::forward",
                    "deckId":"deck",
                    "front":"letter-a",
                    "back":"a",
                    "createdAt":1700000000000,
                    "lineage":{"noteId":"note","noteTypeId":"basic","templateId":"forward","ordinal":0}
                },
                {
                    "id":"note::reverse",
                    "deckId":"deck",
                    "front":"a",
                    "back":"letter-a",
                    "createdAt":1700000000000,
                    "lineage":{"noteId":"note","noteTypeId":"basic","templateId":"reverse","ordinal":1}
                },
                {
                    "id":"other::forward",
                    "deckId":"deck",
                    "front":"letter-b",
                    "back":"b",
                    "createdAt":1700000000000,
                    "lineage":{"noteId":"other","noteTypeId":"basic","templateId":"forward","ordinal":0}
                }
            ],
            "cardProgress": [],
            "sessions": [],
            "reviews": [],
            "deckOptions": [{
                "deckId": "deck",
                "options": {
                    "buryNewSiblings": true,
                    "buryReviewSiblings": false,
                    "buryInterdayLearningSiblings": false
                }
            }],
            "activeSession": null
        }"#;

        session.load_snapshot(snapshot);
        session.dispatch(
            r#"{
                "type": "startSession",
                "sessionId": "session",
                "deckId": "deck",
                "queue": [
                    {
                        "id":"note::forward",
                        "deckId":"deck",
                        "front":"letter-a",
                        "back":"a",
                        "createdAt":1700000000000,
                        "lineage":{"noteId":"note","noteTypeId":"basic","templateId":"forward","ordinal":0}
                    },
                    {
                        "id":"note::reverse",
                        "deckId":"deck",
                        "front":"a",
                        "back":"letter-a",
                        "createdAt":1700000000000,
                        "lineage":{"noteId":"note","noteTypeId":"basic","templateId":"reverse","ordinal":1}
                    },
                    {
                        "id":"other::forward",
                        "deckId":"deck",
                        "front":"letter-b",
                        "back":"b",
                        "createdAt":1700000000000,
                        "lineage":{"noteId":"other","noteTypeId":"basic","templateId":"forward","ordinal":0}
                    }
                ],
                "startedAt": 1700000000000
            }"#,
        );

        let rated: Value =
            serde_json::from_str(&session.handle_engram_app_event("good", "deck", NOW)).unwrap();
        assert_eq!(rated["ok"], true);
        assert_eq!(rated["event"], "onGood");
        assert_eq!(
            rated["state"]["reviews"][0]["siblingProgressSnapshots"][0]["cardId"],
            "note::reverse"
        );
        assert_eq!(
            rated["state"]["cardProgress"]
                .as_array()
                .unwrap()
                .iter()
                .find(|progress| progress["cardId"] == "note::reverse")
                .and_then(|progress| progress["buriedUntil"].as_u64()),
            Some(NOW + engram_core::ONE_DAY_MS)
        );
        let queue_ids: Vec<_> = rated["state"]["activeSession"]["queue"]
            .as_array()
            .unwrap()
            .iter()
            .map(|card| card["id"].as_str().unwrap())
            .collect();
        assert_eq!(queue_ids, vec!["note::forward", "other::forward"]);
        assert_eq!(rated["props"]["prompt"], "letter-b");

        let undone: Value =
            serde_json::from_str(&session.handle_engram_app_event("undo", "deck", NOW + 1))
                .unwrap();
        assert_eq!(undone["ok"], true);
        assert!(undone["state"]["reviews"].as_array().unwrap().is_empty());
        assert!(undone["state"]["cardProgress"]
            .as_array()
            .unwrap()
            .is_empty());
        let restored_ids: Vec<_> = undone["state"]["activeSession"]["queue"]
            .as_array()
            .unwrap()
            .iter()
            .map(|card| card["id"].as_str().unwrap())
            .collect();
        assert_eq!(
            restored_ids,
            vec!["note::forward", "note::reverse", "other::forward"]
        );
    }

    #[test]
    fn handle_engram_app_review_actions_mark_bury_and_suspend_current_cards() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
            "noteTypes": [],
            "notes": [],
            "cards": [
                {
                    "id":"note::forward",
                    "deckId":"deck",
                    "front":"letter-a",
                    "back":"a",
                    "createdAt":1700000000000,
                    "lineage":{"noteId":"note","noteTypeId":"basic","templateId":"forward","ordinal":0}
                },
                {
                    "id":"note::reverse",
                    "deckId":"deck",
                    "front":"a",
                    "back":"letter-a",
                    "createdAt":1700000000000,
                    "lineage":{"noteId":"note","noteTypeId":"basic","templateId":"reverse","ordinal":1}
                },
                {"id":"other","deckId":"deck","front":"letter-aa","back":"aa","createdAt":1700000000000}
            ],
            "cardProgress": [],
            "sessions": [],
            "reviews": [],
            "activeSession": null
        }"#;

        session.load_snapshot(snapshot);
        session.dispatch(
            r#"{
                "type": "startSession",
                "sessionId": "session",
                "deckId": "deck",
                "queue": [
                    {
                        "id":"note::forward",
                        "deckId":"deck",
                        "front":"letter-a",
                        "back":"a",
                        "createdAt":1700000000000,
                        "lineage":{"noteId":"note","noteTypeId":"basic","templateId":"forward","ordinal":0}
                    },
                    {
                        "id":"note::reverse",
                        "deckId":"deck",
                        "front":"a",
                        "back":"letter-a",
                        "createdAt":1700000000000,
                        "lineage":{"noteId":"note","noteTypeId":"basic","templateId":"reverse","ordinal":1}
                    },
                    {"id":"other","deckId":"deck","front":"letter-aa","back":"aa","createdAt":1700000000000}
                ],
                "startedAt": 1700000000000
            }"#,
        );

        let marked: Value =
            serde_json::from_str(&session.handle_engram_app_event("toggle-mark", "deck", NOW))
                .unwrap();
        assert_eq!(marked["ok"], true);
        assert_eq!(marked["event"], "onToggleMark");
        assert_eq!(
            marked["state"]["cardProgress"][0]["cardId"],
            "note::forward"
        );
        assert_eq!(marked["state"]["cardProgress"][0]["markedAt"], NOW);
        assert_eq!(marked["props"]["action-mark-label"], "Unmark");

        let buried_sibling: Value = serde_json::from_str(&session.handle_engram_app_event(
            "onBurySiblings",
            "deck",
            NOW + 1,
        ))
        .unwrap();
        assert_eq!(buried_sibling["ok"], true);
        assert_eq!(buried_sibling["event"], "onBurySiblings");
        assert!(buried_sibling["state"]["cardProgress"]
            .as_array()
            .unwrap()
            .iter()
            .any(|progress| progress["cardId"] == "note::reverse"
                && progress["buriedUntil"] == NOW + 1 + engram_core::ONE_DAY_MS));
        let queue = buried_sibling["state"]["activeSession"]["queue"]
            .as_array()
            .unwrap();
        assert_eq!(queue.len(), 2);
        assert_eq!(queue[0]["id"], "note::forward");
        assert_eq!(queue[1]["id"], "other");

        let buried_current: Value =
            serde_json::from_str(&session.handle_engram_app_event("bury-card", "deck", NOW + 2))
                .unwrap();
        assert_eq!(buried_current["ok"], true);
        assert_eq!(buried_current["event"], "onBuryCard");
        let queue = buried_current["state"]["activeSession"]["queue"]
            .as_array()
            .unwrap();
        assert_eq!(queue.len(), 1);
        assert_eq!(queue[0]["id"], "other");
        assert_eq!(buried_current["props"]["prompt"], "letter-aa");

        let suspended: Value = serde_json::from_str(&session.handle_engram_app_event(
            "onSuspendCard",
            "deck",
            NOW + 3,
        ))
        .unwrap();
        assert_eq!(suspended["ok"], true);
        assert_eq!(suspended["event"], "onSuspendCard");
        assert!(suspended["state"]["cardProgress"]
            .as_array()
            .unwrap()
            .iter()
            .any(|progress| progress["cardId"] == "other" && progress["suspendedAt"] == NOW + 3));
        assert!(suspended["state"]["activeSession"]["queue"]
            .as_array()
            .unwrap()
            .is_empty());
        assert_eq!(suspended["props"]["prompt"], "No cards queued");
    }

    #[test]
    fn handle_engram_app_browser_events_target_selected_card_ids() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
            "noteTypes": [{
                "id": "basic",
                "name": "Basic",
                "fields": [
                    {"id": "front", "name": "Front", "required": true, "ordinal": 0},
                    {"id": "back", "name": "Back", "required": true, "ordinal": 1}
                ],
                "templates": [
                    {
                        "id": "forward",
                        "name": "Forward",
                        "frontTemplate": "{{Front}}",
                        "backTemplate": "{{Back}}",
                        "requiredFieldNames": ["Front"],
                        "ordinal": 0
                    },
                    {
                        "id": "reverse",
                        "name": "Reverse",
                        "frontTemplate": "{{Back}}",
                        "backTemplate": "{{Front}}",
                        "requiredFieldNames": ["Back"],
                        "ordinal": 1
                    }
                ],
                "createdAt": 1700000000000,
                "updatedAt": 1700000000000
            }],
            "notes": [{
                "id": "note",
                "noteTypeId": "basic",
                "deckId": "deck",
                "fields": [
                    {"fieldId": "front", "value": "letter-aa"},
                    {"fieldId": "back", "value": "aa"}
                ],
                "tags": ["tamil"],
                "createdAt": 1700000000000,
                "updatedAt": 1700000000000
            }],
            "cards": [
                {"id":"card","deckId":"deck","front":"letter-a","back":"a","createdAt":1700000000000},
                {
                    "id":"other",
                    "deckId":"deck",
                    "front":"letter-aa",
                    "back":"aa",
                    "createdAt":1700000000000,
                    "lineage": {
                        "noteId": "note",
                        "noteTypeId": "basic",
                        "templateId": "reverse",
                        "ordinal": 1
                    }
                }
            ],
            "cardProgress": [],
            "sessions": [],
            "reviews": [],
            "activeSession": null
        }"#;

        session.load_snapshot(snapshot);

        let search: Value =
            serde_json::from_str(&session.handle_engram_app_event("onBrowserSearch", "deck", NOW))
                .unwrap();
        assert_eq!(search["ok"], true);
        assert_eq!(search["event"], "onBrowserSearch");
        assert_eq!(search["props"]["browser-selected-card-id"], "card");

        let selected: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onBrowserSelectResult","index":1}"#,
            "deck",
            NOW,
        ))
        .unwrap();
        assert_eq!(selected["ok"], true);
        assert_eq!(selected["event"], "onBrowserSelectResult");
        assert_eq!(selected["props"]["browser-selected-index"], 1);
        assert_eq!(selected["props"]["browser-selected-card-id"], "other");

        let open: Value = serde_json::from_str(&session.handle_engram_app_event(
            "onBrowserOpenSelected",
            "deck",
            NOW,
        ))
        .unwrap();
        assert_eq!(open["ok"], true);
        assert_eq!(open["hostIntent"]["type"], "openCard");
        assert_eq!(open["hostIntent"]["cardId"], "other");
        assert_eq!(open["hostIntent"]["noteId"], "note");
        assert_eq!(open["hostIntent"]["noteTypeId"], "basic");
        assert_eq!(open["hostIntent"]["noteTypeName"], "Basic");
        assert_eq!(open["hostIntent"]["templateId"], "reverse");
        assert_eq!(open["hostIntent"]["templateName"], "Reverse");
        assert_eq!(open["hostIntent"]["templateOrdinal"], 1);
        assert_eq!(open["hostIntent"]["cardDeckId"], "deck");
        assert_eq!(open["hostIntent"]["deckName"], "Tamil");
        assert_eq!(open["hostIntent"]["state"], "new");
        assert_eq!(open["hostIntent"]["cardFront"], "letter-aa");
        assert_eq!(open["hostIntent"]["cardBack"], "aa");
        assert_eq!(open["hostIntent"]["tags"], json!(["tamil"]));
        assert_eq!(
            open["hostIntent"]["fields"],
            json!([
                {
                    "id": "front",
                    "name": "Front",
                    "value": "letter-aa",
                    "required": true,
                    "ordinal": 0
                },
                {
                    "id": "back",
                    "name": "Back",
                    "value": "aa",
                    "required": true,
                    "ordinal": 1
                }
            ])
        );

        let edit: Value = serde_json::from_str(&session.handle_engram_app_event(
            "onBrowserEditSelected",
            "deck",
            NOW,
        ))
        .unwrap();
        assert_eq!(edit["ok"], true);
        assert_eq!(edit["hostIntent"], Value::Null);
        assert_eq!(edit["props"]["note-editor-note-id-value"], "note");
        assert_eq!(edit["props"]["note-editor-note-type-value"], "Basic");
        assert_eq!(edit["props"]["note-editor-selected-field-index"], 0);
        assert_eq!(edit["props"]["note-editor-selected-field-label"], "Front");
        assert_eq!(
            edit["props"]["note-editor-selected-field-value"],
            "letter-aa"
        );
        assert_eq!(edit["props"]["note-editor-tags-value"], "tamil");

        let tag_draft: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onBrowserTagEditChange","value":"script grammar"}"#,
            "deck",
            NOW + 1,
        ))
        .unwrap();
        assert_eq!(tag_draft["ok"], true);
        assert_eq!(tag_draft["event"], "onBrowserTagEditChange");
        assert_eq!(tag_draft["props"]["browser-tag-edit"], "script grammar");

        let tagged: Value = serde_json::from_str(&session.handle_engram_app_event(
            "onBrowserAddTagSelected",
            "deck",
            NOW + 2,
        ))
        .unwrap();
        assert_eq!(tagged["ok"], true);
        assert_eq!(tagged["event"], "onBrowserAddTagSelected");
        assert_eq!(
            tagged["state"]["notes"][0]["tags"],
            json!(["tamil", "script", "grammar"])
        );
        assert_eq!(tagged["state"]["notes"][0]["updatedAt"], NOW + 2);

        let explicit_tagged: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onBrowserAddTagSelected","selectedCardId":"other","value":"roots"}"#,
            "deck",
            NOW + 3,
        ))
        .unwrap();
        assert_eq!(
            explicit_tagged["state"]["notes"][0]["tags"],
            json!(["tamil", "script", "grammar", "roots"])
        );
        assert_eq!(explicit_tagged["state"]["notes"][0]["updatedAt"], NOW + 3);

        let removed_tag: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onBrowserRemoveTagSelected","selectedCardId":"other","value":"TAMIL roots"}"#,
            "deck",
            NOW + 4,
        ))
        .unwrap();
        assert_eq!(
            removed_tag["state"]["notes"][0]["tags"],
            json!(["script", "grammar"])
        );
        assert_eq!(removed_tag["state"]["notes"][0]["updatedAt"], NOW + 4);

        let marked: Value = serde_json::from_str(&session.handle_engram_app_event(
            "onBrowserToggleMarkSelected",
            "deck",
            NOW + 5,
        ))
        .unwrap();
        assert_eq!(marked["ok"], true);
        assert_eq!(marked["event"], "onBrowserToggleMarkSelected");
        assert_eq!(marked["state"]["cardProgress"][0]["cardId"], "other");
        assert_eq!(marked["state"]["cardProgress"][0]["markedAt"], NOW + 5);

        let unmarked: Value = serde_json::from_str(&session.handle_engram_app_event(
            "onBrowserToggleMarkSelected",
            "deck",
            NOW + 6,
        ))
        .unwrap();
        assert_eq!(unmarked["ok"], true);
        assert!(unmarked["state"]["cardProgress"]
            .as_array()
            .unwrap()
            .is_empty());

        let suspended: Value = serde_json::from_str(&session.handle_engram_app_event(
            "onBrowserToggleSuspendSelected",
            "deck",
            NOW + 7,
        ))
        .unwrap();
        assert_eq!(suspended["ok"], true);
        assert_eq!(suspended["event"], "onBrowserToggleSuspendSelected");
        assert_eq!(suspended["state"]["cardProgress"][0]["cardId"], "other");
        assert_eq!(
            suspended["state"]["cardProgress"][0]["suspendedAt"],
            NOW + 7
        );

        let unsuspended: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onBrowserToggleSuspendSelected","selectedCardId":"other"}"#,
            "deck",
            NOW + 8,
        ))
        .unwrap();
        assert_eq!(unsuspended["ok"], true);
        assert!(unsuspended["state"]["cardProgress"]
            .as_array()
            .unwrap()
            .is_empty());

        let flag_picker: Value = serde_json::from_str(&session.handle_engram_app_event(
            "onBrowserToggleFlagPicker",
            "deck",
            NOW + 9,
        ))
        .unwrap();
        assert_eq!(flag_picker["ok"], true);
        assert_eq!(flag_picker["event"], "onBrowserToggleFlagPicker");
        assert_eq!(flag_picker["props"]["browser-flag-open"], true);

        let flagged: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onBrowserSetFlagSelected","value":"blue"}"#,
            "deck",
            NOW + 10,
        ))
        .unwrap();
        assert_eq!(flagged["ok"], true);
        assert_eq!(flagged["event"], "onBrowserSetFlagSelected");
        assert_eq!(flagged["state"]["cardProgress"][0]["cardId"], "other");
        assert_eq!(flagged["state"]["cardProgress"][0]["flag"], "blue");
        assert_eq!(
            flagged["props"]["browser-result-flags"],
            json!(["none", "blue"])
        );
        assert_eq!(flagged["props"]["browser-selected-flag"], "blue");
        assert_eq!(flagged["props"]["browser-flag-value"], "blue");
        assert_eq!(flagged["props"]["browser-flag-open"], false);

        let cleared_flag: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onBrowserSetFlagSelected","selectedCardId":"other","value":0}"#,
            "deck",
            NOW + 11,
        ))
        .unwrap();
        assert_eq!(cleared_flag["ok"], true);
        assert!(cleared_flag["state"]["cardProgress"]
            .as_array()
            .unwrap()
            .is_empty());
        assert_eq!(cleared_flag["props"]["browser-selected-flag"], "none");
        assert_eq!(cleared_flag["props"]["browser-flag-value"], "none");

        let explicit_edit: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onBrowserEditSelected","selectedCardId":"card"}"#,
            "deck",
            NOW + 12,
        ))
        .unwrap();
        assert_eq!(explicit_edit["ok"], true);
        assert_eq!(explicit_edit["hostIntent"], Value::Null);
        assert_eq!(explicit_edit["props"]["browser-selected-card-id"], "card");
        assert_eq!(explicit_edit["props"]["note-editor-note-id-value"], "");
        assert_eq!(
            explicit_edit["props"]["note-editor-selected-field-value"],
            ""
        );

        let query_change: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onBrowserQueryChange","value":"cid:other"}"#,
            "deck",
            NOW + 13,
        ))
        .unwrap();
        assert_eq!(query_change["ok"], true);
        assert_eq!(query_change["props"]["browser-query"], "cid:other");
        assert_eq!(query_change["props"]["browser-selected-index"], 0);
        assert_eq!(query_change["props"]["browser-selected-card-id"], "other");

        let mut empty_session = EngramSession::new();
        empty_session.load_snapshot(
            r#"{
                "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
                "noteTypes": [],
                "notes": [],
                "cards": [],
                "cardProgress": [],
                "sessions": [],
                "reviews": [],
                "activeSession": null
            }"#,
        );
        let missing_card: Value = serde_json::from_str(&empty_session.handle_engram_app_event(
            "onBrowserToggleMarkSelected",
            "deck",
            NOW + 7,
        ))
        .unwrap();
        assert_eq!(missing_card["ok"], false);
        assert_eq!(
            missing_card["error"],
            "cannot mark browser row without a card id"
        );
    }

    #[test]
    fn handle_engram_app_collection_events_round_trip_as_host_intents() {
        let mut session = EngramSession::new();
        session.load_snapshot(
            r#"{
                "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
                "noteTypes": [{
                    "id": "basic",
                    "name": "Basic",
                    "fields": [
                        {"id": "front", "name": "Front", "required": true, "ordinal": 0},
                        {"id": "back", "name": "Back", "required": true, "ordinal": 1}
                    ],
                    "templates": [{
                        "id": "forward",
                        "name": "Forward",
                        "frontTemplate": "{{Front}}",
                        "backTemplate": "{{Back}}",
                        "requiredFieldNames": ["Front"],
                        "ordinal": 0
                    }],
                    "createdAt": 1700000000000,
                    "updatedAt": 1700000000000
                }],
                "notes": [{
                    "id": "note",
                    "noteTypeId": "basic",
                    "deckId": "deck",
                    "fields": [],
                    "tags": [],
                    "createdAt": 1700000000000,
                    "updatedAt": 1700000000000
                }],
                "cards": [],
                "cardProgress": [],
                "mediaAssets": [],
                "sessions": [],
                "reviews": [],
                "activeSession": null
            }"#,
        );

        for (event, canonical, intent_type) in [
            ("onImportAnki", "onImportAnki", "importAnki"),
            ("export-anki", "onExportAnki", "exportAnki"),
            ("delete-note", "onDeleteNote", "deleteNote"),
            ("delete-note-type", "onDeleteNoteType", "deleteNoteType"),
        ] {
            let value: Value =
                serde_json::from_str(&session.handle_engram_app_event(event, "deck", NOW)).unwrap();
            assert_eq!(value["ok"], true);
            assert_eq!(value["event"], canonical);
            assert_eq!(value["hostIntent"]["type"], intent_type);
            assert_eq!(value["hostIntent"]["event"], canonical);
            assert_eq!(value["hostIntent"]["deckId"], "deck");
            assert_eq!(value["hostIntent"]["createdAt"], NOW);
            assert_eq!(value["state"]["notes"].as_array().unwrap().len(), 1);
            assert_eq!(value["props"]["collection-note-count-value"], "1");
            assert_eq!(value["props"]["collection-note-type-count-value"], "1");
        }

        let import: Value =
            serde_json::from_str(&session.handle_engram_app_event("onImportAnki", "deck", NOW))
                .unwrap();
        assert_eq!(import["hostIntent"]["accept"], json!([".apkg", ".colpkg"]));

        let export: Value =
            serde_json::from_str(&session.handle_engram_app_event("onExportAnki", "deck", NOW))
                .unwrap();
        assert_eq!(export["hostIntent"]["extension"], ".apkg");
        assert_eq!(
            export["hostIntent"]["extensions"],
            json!([".apkg", ".colpkg"])
        );

        let add_note: Value =
            serde_json::from_str(&session.handle_engram_app_event("onAddNote", "deck", NOW))
                .unwrap();
        assert_eq!(add_note["ok"], true);
        assert_eq!(add_note["event"], "onAddNote");
        assert_eq!(add_note["hostIntent"], Value::Null);
        assert_eq!(add_note["props"]["note-editor-label"], "Add note");
        assert_eq!(
            add_note["props"]["note-editor-note-id-value"],
            "note-1700000000000"
        );
        assert_eq!(
            add_note["props"]["note-editor-note-type-names"],
            json!(["Basic"])
        );
        assert_eq!(add_note["props"]["note-editor-selected-note-type-index"], 0);
        assert_eq!(
            add_note["props"]["note-editor-deck-names"],
            json!(["Tamil"])
        );
        assert_eq!(add_note["props"]["note-editor-selected-deck-index"], 0);
        assert_eq!(
            add_note["props"]["note-editor-field-labels"],
            json!(["Front *", "Back *"])
        );
        assert_eq!(add_note["props"]["note-editor-selected-field-value"], "");

        let add_note_type: Value =
            serde_json::from_str(&session.handle_engram_app_event("onAddNoteType", "deck", NOW))
                .unwrap();
        assert_eq!(add_note_type["ok"], true);
        assert_eq!(add_note_type["event"], "onAddNoteType");
        assert_eq!(add_note_type["hostIntent"], Value::Null);
        assert_eq!(
            add_note_type["props"]["note-type-editor-note-type-id-value"],
            "note-type-1700000000000"
        );
        assert_eq!(
            add_note_type["props"]["note-type-editor-name-value"],
            "Basic"
        );
        assert_eq!(
            add_note_type["props"]["note-type-editor-field-labels"],
            json!(["1 Front *", "2 Back *"])
        );
        assert_eq!(
            add_note_type["props"]["note-type-editor-template-labels"],
            json!(["1 Forward"])
        );
    }

    #[test]
    fn handle_engram_app_save_and_delete_note_events_update_shared_state() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
            "noteTypes": [{
                "id": "basic",
                "name": "Basic",
                "fields": [
                    {"id": "front", "name": "Front", "required": true, "ordinal": 0},
                    {"id": "back", "name": "Back", "required": true, "ordinal": 1}
                ],
                "templates": [
                    {
                        "id": "forward",
                        "name": "Forward",
                        "frontTemplate": "{{Front}}",
                        "backTemplate": "{{Back}}",
                        "requiredFieldNames": ["Front"],
                        "ordinal": 0
                    },
                    {
                        "id": "reverse",
                        "name": "Reverse",
                        "frontTemplate": "{{Back}}",
                        "backTemplate": "{{Front}}",
                        "requiredFieldNames": ["Back"],
                        "ordinal": 1
                    }
                ],
                "createdAt": 1700000000000,
                "updatedAt": 1700000000000
            }],
            "notes": [{
                "id": "note",
                "noteTypeId": "basic",
                "deckId": "deck",
                "fields": [
                    {"fieldId": "front", "value": "letter-aa"},
                    {"fieldId": "back", "value": "aa"}
                ],
                "tags": ["tamil"],
                "createdAt": 1700000000000,
                "updatedAt": 1700000000000
            }],
            "cards": [
                {
                    "id":"note::forward",
                    "deckId":"deck",
                    "front":"letter-aa",
                    "back":"aa",
                    "createdAt":1700000000000,
                    "lineage": {
                        "noteId": "note",
                        "noteTypeId": "basic",
                        "templateId": "forward",
                        "ordinal": 0
                    }
                },
                {
                    "id":"note::reverse",
                    "deckId":"deck",
                    "front":"aa",
                    "back":"letter-aa",
                    "createdAt":1700000000000,
                    "lineage": {
                        "noteId": "note",
                        "noteTypeId": "basic",
                        "templateId": "reverse",
                        "ordinal": 1
                    }
                }
            ],
            "cardProgress": [],
            "sessions": [],
            "reviews": [],
            "activeSession": null
        }"#;

        session.load_snapshot(snapshot);

        let saved: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{
                "event":"onSaveNote",
                "selectedCardId":"note::reverse",
                "fields": {
                    "Front": "letter-aaa",
                    "back": "aaa"
                },
                "tags": "tamil vowel tamil"
            }"#,
            "deck",
            NOW + 1,
        ))
        .unwrap();
        assert_eq!(saved["ok"], true);
        assert_eq!(saved["event"], "onSaveNote");
        assert_eq!(saved["hostIntent"], Value::Null);
        assert_eq!(saved["state"]["notes"][0]["id"], "note");
        assert_eq!(saved["state"]["notes"][0]["updatedAt"], NOW + 1);
        assert_eq!(
            saved["state"]["notes"][0]["fields"],
            json!([
                {"fieldId": "front", "value": "letter-aaa"},
                {"fieldId": "back", "value": "aaa"}
            ])
        );
        assert_eq!(
            saved["state"]["notes"][0]["tags"],
            json!(["tamil", "vowel"])
        );
        assert!(saved["state"]["cards"]
            .as_array()
            .unwrap()
            .iter()
            .any(|card| {
                card["id"] == "note::forward"
                    && card["front"] == "letter-aaa"
                    && card["back"] == "aaa"
            }));
        assert!(saved["state"]["cards"]
            .as_array()
            .unwrap()
            .iter()
            .any(|card| {
                card["id"] == "note::reverse"
                    && card["front"] == "aaa"
                    && card["back"] == "letter-aaa"
            }));

        let deleted: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onDeleteNote","noteId":"note"}"#,
            "deck",
            NOW + 2,
        ))
        .unwrap();
        assert_eq!(deleted["ok"], true);
        assert_eq!(deleted["event"], "onDeleteNote");
        assert_eq!(deleted["hostIntent"], Value::Null);
        assert!(deleted["state"]["notes"].as_array().unwrap().is_empty());
        assert!(deleted["state"]["cards"].as_array().unwrap().is_empty());
    }

    #[test]
    fn handle_engram_app_note_editor_events_save_selected_browser_note() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
            "noteTypes": [{
                "id": "basic",
                "name": "Basic",
                "fields": [
                    {"id": "front", "name": "Front", "required": true, "ordinal": 0},
                    {"id": "back", "name": "Back", "required": true, "ordinal": 1}
                ],
                "templates": [
                    {
                        "id": "forward",
                        "name": "Forward",
                        "frontTemplate": "{{Front}}",
                        "backTemplate": "{{Back}}",
                        "requiredFieldNames": ["Front"],
                        "ordinal": 0
                    },
                    {
                        "id": "reverse",
                        "name": "Reverse",
                        "frontTemplate": "{{Back}}",
                        "backTemplate": "{{Front}}",
                        "requiredFieldNames": ["Back"],
                        "ordinal": 1
                    }
                ],
                "createdAt": 1700000000000,
                "updatedAt": 1700000000000
            }],
            "notes": [{
                "id": "note",
                "noteTypeId": "basic",
                "deckId": "deck",
                "fields": [
                    {"fieldId": "front", "value": "letter-aa"},
                    {"fieldId": "back", "value": "aa"}
                ],
                "tags": ["tamil"],
                "createdAt": 1700000000000,
                "updatedAt": 1700000000000
            }],
            "cards": [
                {
                    "id":"note::forward",
                    "deckId":"deck",
                    "front":"letter-aa",
                    "back":"aa",
                    "createdAt":1700000000000,
                    "lineage": {
                        "noteId": "note",
                        "noteTypeId": "basic",
                        "templateId": "forward",
                        "ordinal": 0
                    }
                },
                {
                    "id":"note::reverse",
                    "deckId":"deck",
                    "front":"aa",
                    "back":"letter-aa",
                    "createdAt":1700000000000,
                    "lineage": {
                        "noteId": "note",
                        "noteTypeId": "basic",
                        "templateId": "reverse",
                        "ordinal": 1
                    }
                }
            ],
            "cardProgress": [],
            "sessions": [],
            "reviews": [],
            "activeSession": null
        }"#;

        session.load_snapshot(snapshot);

        let initial: Value = serde_json::from_str(&session.engram_app_props("deck", NOW)).unwrap();
        assert_eq!(
            initial["props"]["note-editor-field-labels"],
            json!(["Front *", "Back *"])
        );
        assert_eq!(initial["props"]["note-editor-selected-field-index"], 0);
        assert_eq!(
            initial["props"]["note-editor-selected-field-value"],
            "letter-aa"
        );
        assert_eq!(initial["props"]["note-editor-tags-value"], "tamil");

        let selected_back: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onNoteEditorSelectField","index":1}"#,
            "deck",
            NOW + 1,
        ))
        .unwrap();
        assert_eq!(selected_back["ok"], true);
        assert_eq!(selected_back["event"], "onNoteEditorSelectField");
        assert_eq!(
            selected_back["props"]["note-editor-selected-field-label"],
            "Back"
        );

        let edited_back: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onNoteEditorFieldValueChange","value":"aaa"}"#,
            "deck",
            NOW + 2,
        ))
        .unwrap();
        assert_eq!(
            edited_back["props"]["note-editor-selected-field-value"],
            "aaa"
        );

        let edited_tags: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onNoteEditorTagsChange","value":"tamil vowel tamil"}"#,
            "deck",
            NOW + 3,
        ))
        .unwrap();
        assert_eq!(
            edited_tags["props"]["note-editor-tags-value"],
            "tamil vowel tamil"
        );

        let saved: Value = serde_json::from_str(&session.handle_engram_app_event(
            "onNoteEditorSaveNote",
            "deck",
            NOW + 4,
        ))
        .unwrap();
        assert_eq!(saved["ok"], true);
        assert_eq!(saved["event"], "onNoteEditorSaveNote");
        assert_eq!(saved["state"]["notes"][0]["updatedAt"], NOW + 4);
        assert_eq!(
            saved["state"]["notes"][0]["fields"],
            json!([
                {"fieldId": "front", "value": "letter-aa"},
                {"fieldId": "back", "value": "aaa"}
            ])
        );
        assert_eq!(
            saved["state"]["notes"][0]["tags"],
            json!(["tamil", "vowel"])
        );
        assert!(saved["state"]["cards"]
            .as_array()
            .unwrap()
            .iter()
            .any(|card| {
                card["id"] == "note::reverse"
                    && card["front"] == "aaa"
                    && card["back"] == "letter-aa"
            }));

        let confirm_delete: Value = serde_json::from_str(&session.handle_engram_app_event(
            "onNoteEditorDeleteNote",
            "deck",
            NOW + 5,
        ))
        .unwrap();
        assert_eq!(confirm_delete["ok"], true);
        assert_eq!(confirm_delete["event"], "onNoteEditorDeleteNote");
        assert_eq!(
            confirm_delete["props"]["note-editor-delete-label"],
            "Confirm delete"
        );
        assert_eq!(
            confirm_delete["state"]["notes"].as_array().unwrap().len(),
            1
        );

        let deleted: Value = serde_json::from_str(&session.handle_engram_app_event(
            "onNoteEditorDeleteNote",
            "deck",
            NOW + 6,
        ))
        .unwrap();
        assert_eq!(deleted["ok"], true);
        assert!(deleted["state"]["notes"].as_array().unwrap().is_empty());
        assert!(deleted["state"]["cards"].as_array().unwrap().is_empty());
    }

    #[test]
    fn handle_engram_app_add_note_uses_shared_note_editor_draft() {
        let mut session = EngramSession::new();
        session.load_snapshot(
            r#"{
                "decks": [
                    {"id":"tamil","name":"Tamil","description":"Script","createdAt":1700000000000},
                    {"id":"spanish","name":"Spanish","description":"Words","createdAt":1700000000000}
                ],
                "noteTypes": [
                    {
                        "id": "basic",
                        "name": "Basic",
                        "fields": [
                            {"id": "front", "name": "Front", "required": true, "ordinal": 0},
                            {"id": "back", "name": "Back", "required": true, "ordinal": 1}
                        ],
                        "templates": [{
                            "id": "forward",
                            "name": "Forward",
                            "frontTemplate": "{{Front}}",
                            "backTemplate": "{{Back}}",
                            "requiredFieldNames": ["Front"],
                            "ordinal": 0
                        }],
                        "createdAt": 1700000000000,
                        "updatedAt": 1700000000000
                    },
                    {
                        "id": "spanish-basic",
                        "name": "Spanish basic",
                        "fields": [
                            {"id": "prompt", "name": "Prompt", "required": true, "ordinal": 0},
                            {"id": "answer", "name": "Answer", "required": true, "ordinal": 1}
                        ],
                        "templates": [{
                            "id": "forward",
                            "name": "Forward",
                            "frontTemplate": "{{Prompt}}",
                            "backTemplate": "{{Answer}}",
                            "requiredFieldNames": ["Prompt"],
                            "ordinal": 0
                        }],
                        "createdAt": 1700000000000,
                        "updatedAt": 1700000000000
                    }
                ],
                "notes": [],
                "cards": [],
                "cardProgress": [],
                "sessions": [],
                "reviews": [],
                "activeSession": null
            }"#,
        );

        let draft: Value =
            serde_json::from_str(&session.handle_engram_app_event("onAddNote", "tamil", NOW))
                .unwrap();
        assert_eq!(draft["ok"], true);
        assert_eq!(draft["hostIntent"], Value::Null);
        assert_eq!(draft["props"]["note-editor-label"], "Add note");
        assert_eq!(
            draft["props"]["note-editor-note-type-names"],
            json!(["Basic", "Spanish basic"])
        );
        assert_eq!(
            draft["props"]["note-editor-field-labels"],
            json!(["Front *", "Back *"])
        );
        assert_eq!(draft["props"]["note-editor-selected-deck-index"], 0);

        let selected_model: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onNoteEditorSelectNoteType","index":1}"#,
            "tamil",
            NOW + 1,
        ))
        .unwrap();
        assert_eq!(selected_model["ok"], true);
        assert_eq!(
            selected_model["props"]["note-editor-note-type-value"],
            "Spanish basic"
        );
        assert_eq!(
            selected_model["props"]["note-editor-field-labels"],
            json!(["Prompt *", "Answer *"])
        );

        let selected_deck: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onNoteEditorSelectDeck","index":1}"#,
            "tamil",
            NOW + 2,
        ))
        .unwrap();
        assert_eq!(selected_deck["ok"], true);
        assert_eq!(selected_deck["props"]["note-editor-deck-value"], "Spanish");
        assert_eq!(selected_deck["props"]["note-editor-selected-deck-index"], 1);

        let edited_prompt: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onNoteEditorFieldValueChange","value":"hola"}"#,
            "tamil",
            NOW + 3,
        ))
        .unwrap();
        assert_eq!(
            edited_prompt["props"]["note-editor-selected-field-value"],
            "hola"
        );

        let selected_answer: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onNoteEditorSelectField","index":1}"#,
            "tamil",
            NOW + 4,
        ))
        .unwrap();
        assert_eq!(
            selected_answer["props"]["note-editor-selected-field-label"],
            "Answer"
        );

        let edited_answer: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onNoteEditorFieldValueChange","value":"hello"}"#,
            "tamil",
            NOW + 5,
        ))
        .unwrap();
        assert_eq!(
            edited_answer["props"]["note-editor-selected-field-value"],
            "hello"
        );

        let tagged: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onNoteEditorTagsChange","value":"spanish etymology spanish"}"#,
            "tamil",
            NOW + 6,
        ))
        .unwrap();
        assert_eq!(
            tagged["props"]["note-editor-tags-value"],
            "spanish etymology spanish"
        );

        let saved: Value = serde_json::from_str(&session.handle_engram_app_event(
            "onNoteEditorSaveNote",
            "tamil",
            NOW + 7,
        ))
        .unwrap();
        assert_eq!(saved["ok"], true);
        assert_eq!(saved["hostIntent"], Value::Null);
        assert_eq!(saved["state"]["notes"].as_array().unwrap().len(), 1);
        assert_eq!(saved["state"]["notes"][0]["id"], "note-1700000000000");
        assert_eq!(saved["state"]["notes"][0]["noteTypeId"], "spanish-basic");
        assert_eq!(saved["state"]["notes"][0]["deckId"], "spanish");
        assert_eq!(saved["state"]["notes"][0]["createdAt"], NOW);
        assert_eq!(saved["state"]["notes"][0]["updatedAt"], NOW + 7);
        assert_eq!(
            saved["state"]["notes"][0]["fields"],
            json!([
                {"fieldId": "prompt", "value": "hola"},
                {"fieldId": "answer", "value": "hello"}
            ])
        );
        assert_eq!(
            saved["state"]["notes"][0]["tags"],
            json!(["spanish", "etymology"])
        );
        assert!(saved["state"]["cards"]
            .as_array()
            .unwrap()
            .iter()
            .any(|card| {
                card["id"] == "note-1700000000000::forward"
                    && card["deckId"] == "spanish"
                    && card["front"] == "hola"
                    && card["back"] == "hello"
            }));
    }

    #[test]
    fn handle_engram_app_save_and_delete_note_type_events_update_shared_state() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
            "noteTypes": [{
                "id": "basic",
                "name": "Basic",
                "fields": [
                    {"id": "front", "name": "Front", "required": true, "ordinal": 0},
                    {"id": "back", "name": "Back", "required": true, "ordinal": 1}
                ],
                "templates": [
                    {
                        "id": "forward",
                        "name": "Forward",
                        "frontTemplate": "{{Front}}",
                        "backTemplate": "{{Back}}",
                        "requiredFieldNames": ["Front"],
                        "ordinal": 0
                    }
                ],
                "createdAt": 1700000000000,
                "updatedAt": 1700000000000
            }],
            "notes": [{
                "id": "note",
                "noteTypeId": "basic",
                "deckId": "deck",
                "fields": [
                    {"fieldId": "front", "value": "letter-aa"},
                    {"fieldId": "back", "value": "aa"}
                ],
                "tags": ["tamil"],
                "createdAt": 1700000000000,
                "updatedAt": 1700000000000
            }],
            "cards": [
                {
                    "id":"note::forward",
                    "deckId":"deck",
                    "front":"letter-aa",
                    "back":"aa",
                    "createdAt":1700000000000,
                    "lineage": {
                        "noteId": "note",
                        "noteTypeId": "basic",
                        "templateId": "forward",
                        "ordinal": 0
                    }
                }
            ],
            "cardProgress": [],
            "sessions": [],
            "reviews": [],
            "activeSession": null
        }"#;

        session.load_snapshot(snapshot);

        let saved: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{
                "event": "onSaveNoteType",
                "noteType": {
                    "id": "basic",
                    "name": "Basic updated",
                    "fields": [
                        {"id": "front", "name": "Prompt", "required": true, "ordinal": 0},
                        {"id": "back", "name": "Answer", "required": true, "ordinal": 1}
                    ],
                    "templates": [
                        {
                            "id": "forward",
                            "name": "Forward",
                            "frontTemplate": "{{Prompt}}",
                            "backTemplate": "{{Answer}}",
                            "requiredFieldNames": ["Prompt"],
                            "ordinal": 0
                        },
                        {
                            "id": "reverse",
                            "name": "Reverse",
                            "frontTemplate": "{{Answer}}",
                            "backTemplate": "{{Prompt}}",
                            "requiredFieldNames": ["Answer"],
                            "ordinal": 1
                        }
                    ],
                    "stylesheet": ".card { color: red; }"
                }
            }"#,
            "deck",
            NOW + 1,
        ))
        .unwrap();
        assert_eq!(saved["ok"], true);
        assert_eq!(saved["event"], "onSaveNoteType");
        assert_eq!(saved["hostIntent"], Value::Null);
        assert_eq!(saved["state"]["noteTypes"][0]["name"], "Basic updated");
        assert_eq!(
            saved["state"]["noteTypes"][0]["fields"][0]["name"],
            "Prompt"
        );
        assert_eq!(
            saved["state"]["noteTypes"][0]["stylesheet"],
            ".card { color: red; }"
        );
        assert!(saved["state"]["cards"]
            .as_array()
            .unwrap()
            .iter()
            .any(|card| {
                card["id"] == "note::forward"
                    && card["front"] == "letter-aa"
                    && card["back"] == "aa"
            }));
        assert!(saved["state"]["cards"]
            .as_array()
            .unwrap()
            .iter()
            .any(|card| {
                card["id"] == "note::reverse"
                    && card["front"] == "aa"
                    && card["back"] == "letter-aa"
            }));

        let deleted: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onDeleteNoteType","noteTypeId":"basic"}"#,
            "deck",
            NOW + 2,
        ))
        .unwrap();
        assert_eq!(deleted["ok"], true);
        assert_eq!(deleted["event"], "onDeleteNoteType");
        assert_eq!(deleted["hostIntent"], Value::Null);
        assert!(deleted["state"]["noteTypes"].as_array().unwrap().is_empty());
        assert!(deleted["state"]["notes"].as_array().unwrap().is_empty());
        assert!(deleted["state"]["cards"].as_array().unwrap().is_empty());
    }

    #[test]
    fn handle_engram_app_note_type_editor_events_save_shared_models() {
        let mut session = EngramSession::new();
        session.load_snapshot(
            r#"{
                "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
                "noteTypes": [{
                    "id": "basic",
                    "name": "Basic",
                    "fields": [
                        {"id": "front", "name": "Front", "required": true, "ordinal": 0},
                        {"id": "back", "name": "Back", "required": true, "ordinal": 1}
                    ],
                    "templates": [{
                        "id": "forward",
                        "name": "Forward",
                        "frontTemplate": "{{Front}}",
                        "backTemplate": "{{Back}}",
                        "requiredFieldNames": ["Front"],
                        "ordinal": 0
                    }],
                    "createdAt": 1700000000000,
                    "updatedAt": 1700000000000
                }],
                "notes": [],
                "cards": [],
                "cardProgress": [],
                "sessions": [],
                "reviews": [],
                "activeSession": null
            }"#,
        );

        let initial: Value = serde_json::from_str(&session.engram_app_props("deck", NOW)).unwrap();
        assert_eq!(
            initial["props"]["note-type-editor-note-type-names"],
            json!(["Basic"])
        );
        assert_eq!(
            initial["props"]["note-type-editor-field-labels"],
            json!(["1 Front *", "2 Back *"])
        );
        assert_eq!(initial["props"]["note-type-editor-selected-field-index"], 0);
        assert_eq!(
            initial["props"]["note-type-editor-field-name-value"],
            "Front"
        );
        assert_eq!(
            initial["props"]["note-type-editor-field-required-value"],
            true
        );
        assert_eq!(
            initial["props"]["note-type-editor-template-labels"],
            json!(["1 Forward"])
        );
        assert_eq!(
            initial["props"]["note-type-editor-selected-template-index"],
            0
        );
        assert_eq!(
            initial["props"]["note-type-editor-template-name-value"],
            "Forward"
        );
        assert_eq!(
            initial["props"]["note-type-editor-front-template-value"],
            "{{Front}}"
        );
        assert_eq!(
            initial["props"]["note-type-editor-back-template-value"],
            "{{Back}}"
        );

        let renamed: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onNoteTypeEditorNameChange","value":"Tamil Script"}"#,
            "deck",
            NOW + 1,
        ))
        .unwrap();
        assert_eq!(renamed["ok"], true);
        assert_eq!(
            renamed["props"]["note-type-editor-name-value"],
            "Tamil Script"
        );

        let field_renamed: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onNoteTypeEditorFieldNameChange","value":"Prompt"}"#,
            "deck",
            NOW + 2,
        ))
        .unwrap();
        assert_eq!(
            field_renamed["props"]["note-type-editor-field-name-value"],
            "Prompt"
        );
        assert_eq!(
            field_renamed["props"]["note-type-editor-field-labels"],
            json!(["1 Prompt *", "2 Back *"])
        );

        let field_optional: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onNoteTypeEditorFieldRequiredChange","checked":false}"#,
            "deck",
            NOW + 3,
        ))
        .unwrap();
        assert_eq!(
            field_optional["props"]["note-type-editor-field-required-value"],
            false
        );
        assert_eq!(
            field_optional["props"]["note-type-editor-field-labels"],
            json!(["1 Prompt", "2 Back *"])
        );

        let template_named: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onNoteTypeEditorTemplateNameChange","value":"Forward with hint"}"#,
            "deck",
            NOW + 4,
        ))
        .unwrap();
        assert_eq!(
            template_named["props"]["note-type-editor-template-name-value"],
            "Forward with hint"
        );
        assert_eq!(
            template_named["props"]["note-type-editor-template-labels"],
            json!(["1 Forward with hint"])
        );

        let front_template: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onNoteTypeEditorFrontTemplateChange","value":"{{Prompt}}<br>{{hint:Back}}"}"#,
            "deck",
            NOW + 5,
        ))
        .unwrap();
        assert_eq!(
            front_template["props"]["note-type-editor-front-template-value"],
            "{{Prompt}}<br>{{hint:Back}}"
        );

        let back_template: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onNoteTypeEditorBackTemplateChange","value":"{{FrontSide}}<hr>{{Back}}"}"#,
            "deck",
            NOW + 6,
        ))
        .unwrap();
        assert_eq!(
            back_template["props"]["note-type-editor-back-template-value"],
            "{{FrontSide}}<hr>{{Back}}"
        );

        let styled: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onNoteTypeEditorStylesheetChange","value":".card { color: teal; }"}"#,
            "deck",
            NOW + 7,
        ))
        .unwrap();
        assert_eq!(
            styled["props"]["note-type-editor-stylesheet-value"],
            ".card { color: teal; }"
        );

        let saved_existing: Value = serde_json::from_str(&session.handle_engram_app_event(
            "onNoteTypeEditorSaveNoteType",
            "deck",
            NOW + 8,
        ))
        .unwrap();
        assert_eq!(saved_existing["ok"], true);
        assert_eq!(saved_existing["hostIntent"], Value::Null);
        assert_eq!(
            saved_existing["state"]["noteTypes"][0]["name"],
            "Tamil Script"
        );
        assert_eq!(
            saved_existing["state"]["noteTypes"][0]["stylesheet"],
            ".card { color: teal; }"
        );
        assert_eq!(
            saved_existing["state"]["noteTypes"][0]["updatedAt"],
            NOW + 8
        );
        assert_eq!(
            saved_existing["state"]["noteTypes"][0]["fields"][0]["name"],
            "Prompt"
        );
        assert_eq!(
            saved_existing["state"]["noteTypes"][0]["fields"][0]["required"],
            false
        );
        assert_eq!(
            saved_existing["state"]["noteTypes"][0]["templates"][0]["name"],
            "Forward with hint"
        );
        assert_eq!(
            saved_existing["state"]["noteTypes"][0]["templates"][0]["frontTemplate"],
            "{{Prompt}}<br>{{hint:Back}}"
        );
        assert_eq!(
            saved_existing["state"]["noteTypes"][0]["templates"][0]["backTemplate"],
            "{{FrontSide}}<hr>{{Back}}"
        );
        assert_eq!(
            saved_existing["state"]["noteTypes"][0]["templates"][0]["requiredFieldNames"],
            json!(["Prompt"])
        );

        let new_draft: Value = serde_json::from_str(&session.handle_engram_app_event(
            "onAddNoteType",
            "deck",
            NOW + 9,
        ))
        .unwrap();
        assert_eq!(new_draft["hostIntent"], Value::Null);
        assert_eq!(
            new_draft["props"]["note-type-editor-note-type-names"],
            json!(["Tamil Script", "Basic (new)"])
        );
        assert_eq!(
            new_draft["props"]["note-type-editor-selected-note-type-index"],
            1
        );
        assert_eq!(
            new_draft["props"]["note-type-editor-note-type-id-value"],
            "note-type-1700000000009"
        );

        let renamed_new: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onNoteTypeEditorNameChange","value":"Tamil Reverse"}"#,
            "deck",
            NOW + 10,
        ))
        .unwrap();
        assert_eq!(
            renamed_new["props"]["note-type-editor-name-value"],
            "Tamil Reverse"
        );

        let saved_new: Value = serde_json::from_str(&session.handle_engram_app_event(
            "onNoteTypeEditorSaveNoteType",
            "deck",
            NOW + 11,
        ))
        .unwrap();
        assert_eq!(saved_new["state"]["noteTypes"].as_array().unwrap().len(), 2);
        assert_eq!(
            saved_new["state"]["noteTypes"][1]["id"],
            "note-type-1700000000009"
        );
        assert_eq!(saved_new["state"]["noteTypes"][1]["name"], "Tamil Reverse");
        assert_eq!(
            saved_new["state"]["noteTypes"][1]["templates"][0]["frontTemplate"],
            "{{Front}}"
        );

        serde_json::from_str::<Value>(&session.handle_engram_app_event(
            r#"{"event":"onNoteTypeEditorSelectNoteType","index":1}"#,
            "deck",
            NOW + 12,
        ))
        .unwrap();
        let deleted_new: Value = serde_json::from_str(&session.handle_engram_app_event(
            "onNoteTypeEditorDeleteNoteType",
            "deck",
            NOW + 13,
        ))
        .unwrap();
        assert_eq!(deleted_new["ok"], true);
        assert_eq!(
            deleted_new["props"]["note-type-editor-delete-label"],
            "Confirm delete"
        );
        assert_eq!(
            deleted_new["state"]["noteTypes"].as_array().unwrap().len(),
            2
        );

        let deleted_new: Value = serde_json::from_str(&session.handle_engram_app_event(
            "onNoteTypeEditorDeleteNoteType",
            "deck",
            NOW + 14,
        ))
        .unwrap();
        assert_eq!(
            deleted_new["state"]["noteTypes"].as_array().unwrap().len(),
            1
        );
        assert_eq!(deleted_new["state"]["noteTypes"][0]["id"], "basic");
    }

    #[test]
    fn handle_engram_app_deck_option_events_persist_shared_options() {
        let mut session = EngramSession::new();
        session.load_snapshot(
            r#"{
                "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
                "noteTypes": [],
                "notes": [],
                "cards": [],
                "cardProgress": [],
                "sessions": [],
                "reviews": [],
                "deckOptions": [{
                    "deckId": "deck",
                    "options": {
                        "newCardsPerDay": 12,
                        "reviewsPerDay": 80,
                        "learningStepsMinutes": [3, 30],
                        "relearningStepsMinutes": [5],
                        "graduatingIntervalDays": 2,
                        "easyIntervalDays": 5,
                        "initialEaseFactor": 2.5,
                        "maximumIntervalDays": 180,
                        "reviewIntervalModifier": 1.1,
                        "hardIntervalMultiplier": 1.2,
                        "easyBonusMultiplier": 1.4,
                        "lapseIntervalMultiplier": 0.25,
                        "leechThreshold": 8,
                        "leechAction": "tagOnly",
                        "buryNewSiblings": true,
                        "buryReviewSiblings": true,
                        "buryInterdayLearningSiblings": true
                    }
                }],
                "activeSession": null
            }"#,
        );

        let new_cards: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"type":"deckOptionsNewCardsChange","value":7}"#,
            "deck",
            NOW,
        ))
        .unwrap();
        assert_eq!(new_cards["ok"], true);
        assert_eq!(new_cards["event"], "onDeckOptionsNewCardsChange");
        assert_eq!(
            new_cards["state"]["deckOptions"][0]["options"]["newCardsPerDay"],
            7
        );
        assert_eq!(
            new_cards["state"]["deckOptions"][0]["options"]["reviewsPerDay"],
            80
        );
        assert_eq!(new_cards["props"]["deck-options-new-cards-value"], 7);

        let interval_modifier: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onDeckOptionsIntervalModifierChange","value":"1.25"}"#,
            "deck",
            NOW,
        ))
        .unwrap();
        assert_eq!(interval_modifier["ok"], true);
        assert_eq!(
            interval_modifier["event"],
            "onDeckOptionsIntervalModifierChange"
        );
        assert_eq!(
            interval_modifier["state"]["deckOptions"][0]["options"]["reviewIntervalModifier"],
            1.25
        );
        assert_eq!(
            interval_modifier["props"]["deck-options-interval-modifier-value"],
            1.25
        );

        let initial_ease: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onDeckOptionsInitialEaseChange","value":"2.8"}"#,
            "deck",
            NOW,
        ))
        .unwrap();
        assert_eq!(initial_ease["ok"], true);
        assert_eq!(initial_ease["event"], "onDeckOptionsInitialEaseChange");
        assert_eq!(
            initial_ease["state"]["deckOptions"][0]["options"]["initialEaseFactor"],
            2.8
        );
        assert_eq!(
            initial_ease["props"]["deck-options-initial-ease-value"],
            2.8
        );

        let learning_steps: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"type":"deckOptionsLearningStepsChange","value":"2, 20 60"}"#,
            "deck",
            NOW,
        ))
        .unwrap();
        assert_eq!(learning_steps["ok"], true);
        assert_eq!(learning_steps["event"], "onDeckOptionsLearningStepsChange");
        assert_eq!(
            learning_steps["state"]["deckOptions"][0]["options"]["learningStepsMinutes"],
            json!([2, 20, 60])
        );
        assert_eq!(
            learning_steps["props"]["deck-options-learning-steps-value"],
            "2, 20, 60"
        );

        let relearning_steps: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onDeckOptionsRelearningStepsChange","value":"15; 45"}"#,
            "deck",
            NOW,
        ))
        .unwrap();
        assert_eq!(relearning_steps["ok"], true);
        assert_eq!(
            relearning_steps["event"],
            "onDeckOptionsRelearningStepsChange"
        );
        assert_eq!(
            relearning_steps["state"]["deckOptions"][0]["options"]["relearningStepsMinutes"],
            json!([15, 45])
        );
        assert_eq!(
            relearning_steps["props"]["deck-options-relearning-steps-value"],
            "15, 45"
        );

        let leech_threshold: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"type":"deckOptionsLeechThresholdChange","value":4}"#,
            "deck",
            NOW,
        ))
        .unwrap();
        assert_eq!(leech_threshold["ok"], true);
        assert_eq!(
            leech_threshold["event"],
            "onDeckOptionsLeechThresholdChange"
        );
        assert_eq!(
            leech_threshold["state"]["deckOptions"][0]["options"]["leechThreshold"],
            4
        );
        assert_eq!(
            leech_threshold["props"]["deck-options-leech-threshold-value"],
            4
        );

        let desired_retention: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"type":"deckOptionsDesiredRetentionChange","value":0.93}"#,
            "deck",
            NOW,
        ))
        .unwrap();
        assert_eq!(desired_retention["ok"], true);
        assert_eq!(
            desired_retention["event"],
            "onDeckOptionsDesiredRetentionChange"
        );
        assert_eq!(
            desired_retention["state"]["deckOptions"][0]["options"]["desiredRetention"],
            0.93
        );
        assert_eq!(
            desired_retention["props"]["deck-options-desired-retention-value"],
            0.93
        );

        let fsrs_parameters: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"type":"deckOptionsFsrsParametersChange","value":"0.1, 1.2 2.3"}"#,
            "deck",
            NOW,
        ))
        .unwrap();
        assert_eq!(fsrs_parameters["ok"], true);
        assert_eq!(
            fsrs_parameters["event"],
            "onDeckOptionsFsrsParametersChange"
        );
        assert_eq!(
            fsrs_parameters["state"]["deckOptions"][0]["options"]["fsrsParameters"],
            json!([0.1, 1.2, 2.3])
        );
        assert_eq!(
            fsrs_parameters["props"]["deck-options-fsrs-parameters-value"],
            "0.1, 1.2, 2.3"
        );

        let fsrs_search: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"type":"deckOptionsFsrsSearchChange","value":"preset:\"Tamil\" -is:suspended"}"#,
            "deck",
            NOW,
        ))
        .unwrap();
        assert_eq!(fsrs_search["ok"], true);
        assert_eq!(fsrs_search["event"], "onDeckOptionsFsrsSearchChange");
        assert_eq!(
            fsrs_search["state"]["deckOptions"][0]["options"]["fsrsParameterSearch"],
            "preset:\"Tamil\" -is:suspended"
        );
        assert_eq!(
            fsrs_search["props"]["deck-options-fsrs-search-value"],
            "preset:\"Tamil\" -is:suspended"
        );

        let ignore_before: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"type":"deckOptionsIgnoreReviewHistoryBeforeChange","value":"2024-01-02"}"#,
            "deck",
            NOW,
        ))
        .unwrap();
        assert_eq!(ignore_before["ok"], true);
        assert_eq!(
            ignore_before["event"],
            "onDeckOptionsIgnoreReviewHistoryBeforeChange"
        );
        assert_eq!(
            ignore_before["state"]["deckOptions"][0]["options"]["ignoreReviewHistoryBefore"],
            "2024-01-02"
        );
        assert_eq!(
            ignore_before["props"]["deck-options-ignore-review-history-before-value"],
            "2024-01-02"
        );

        let historical_retention: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"type":"deckOptionsHistoricalRetentionChange","value":0.86}"#,
            "deck",
            NOW,
        ))
        .unwrap();
        assert_eq!(historical_retention["ok"], true);
        assert_eq!(
            historical_retention["event"],
            "onDeckOptionsHistoricalRetentionChange"
        );
        assert_eq!(
            historical_retention["state"]["deckOptions"][0]["options"]["historicalRetention"],
            0.86
        );
        assert_eq!(
            historical_retention["props"]["deck-options-historical-retention-value"],
            0.86
        );

        let easy_days: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"type":"deckOptionsEasyDaysPercentagesChange","value":"1, 0.9, 0.8"}"#,
            "deck",
            NOW,
        ))
        .unwrap();
        assert_eq!(easy_days["ok"], true);
        assert_eq!(easy_days["event"], "onDeckOptionsEasyDaysPercentagesChange");
        assert_eq!(
            easy_days["state"]["deckOptions"][0]["options"]["easyDaysPercentages"],
            json!([1.0, 0.9, 0.8])
        );
        assert_eq!(
            easy_days["props"]["deck-options-easy-days-percentages-value"],
            "1, 0.9, 0.8"
        );

        let leech_action: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onDeckOptionsLeechActionChange","value":"suspend"}"#,
            "deck",
            NOW,
        ))
        .unwrap();
        assert_eq!(leech_action["ok"], true);
        assert_eq!(leech_action["event"], "onDeckOptionsLeechActionChange");
        assert_eq!(
            leech_action["state"]["deckOptions"][0]["options"]["leechAction"],
            "suspend"
        );
        assert_eq!(
            leech_action["props"]["deck-options-leech-action-suspend-value"],
            true
        );
        assert_eq!(
            leech_action["props"]["deck-options-leech-action-tag-only-value"],
            false
        );

        let bury_new: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"type":"deckOptionsBuryNewSiblingsChange","checked":false}"#,
            "deck",
            NOW,
        ))
        .unwrap();
        assert_eq!(bury_new["ok"], true);
        assert_eq!(bury_new["event"], "onDeckOptionsBuryNewSiblingsChange");
        assert_eq!(
            bury_new["state"]["deckOptions"][0]["options"]["buryNewSiblings"],
            false
        );
        assert_eq!(
            bury_new["props"]["deck-options-bury-new-siblings-value"],
            false
        );

        let bury_interday: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onDeckOptionsBuryInterdayLearningSiblingsChange","value":"off"}"#,
            "deck",
            NOW,
        ))
        .unwrap();
        assert_eq!(bury_interday["ok"], true);
        assert_eq!(
            bury_interday["event"],
            "onDeckOptionsBuryInterdayLearningSiblingsChange"
        );
        assert_eq!(
            bury_interday["state"]["deckOptions"][0]["options"]["buryInterdayLearningSiblings"],
            false
        );
        assert_eq!(
            bury_interday["props"]["deck-options-bury-interday-learning-siblings-value"],
            false
        );

        let invalid: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"type":"deckOptionsEasyBonusChange"}"#,
            "deck",
            NOW,
        ))
        .unwrap();
        assert_eq!(invalid["ok"], false);
        assert_eq!(
            invalid["error"],
            "onDeckOptionsEasyBonusChange is missing numeric value"
        );

        let invalid_steps: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"type":"deckOptionsLearningStepsChange","value":"soon"}"#,
            "deck",
            NOW,
        ))
        .unwrap();
        assert_eq!(invalid_steps["ok"], false);
        assert_eq!(
            invalid_steps["error"],
            "learning steps must be whole minutes separated by commas"
        );

        let invalid_leech_action: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"type":"deckOptionsLeechActionChange","value":"delete"}"#,
            "deck",
            NOW,
        ))
        .unwrap();
        assert_eq!(invalid_leech_action["ok"], false);
        assert_eq!(
            invalid_leech_action["error"],
            "leech action must be suspend or tag-only"
        );

        let invalid_bool: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"type":"deckOptionsBuryReviewSiblingsChange"}"#,
            "deck",
            NOW,
        ))
        .unwrap();
        assert_eq!(invalid_bool["ok"], false);
        assert_eq!(
            invalid_bool["error"],
            "onDeckOptionsBuryReviewSiblingsChange is missing checked value"
        );
    }

    #[test]
    fn handle_engram_app_event_rejects_unknown_events_and_missing_active_session() {
        let mut session = EngramSession::new();

        let unknown: Value =
            serde_json::from_str(&session.handle_engram_app_event("onDance", "", NOW)).unwrap();
        assert_eq!(unknown["ok"], false);
        assert_eq!(unknown["error"], "unknown Engram app event: onDance");

        let rated: Value =
            serde_json::from_str(&session.handle_engram_app_event("onGood", "", NOW)).unwrap();
        assert_eq!(rated["ok"], false);
        assert_eq!(rated["error"], "cannot rate without an active session");
    }

    #[test]
    fn review_history_reports_rating_summary_for_range() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [
                {"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000},
                {"id":"child","name":"Tamil::Verbs","description":"Grammar","createdAt":1700000000000},
                {"id":"other","name":"Spanish","description":"Words","createdAt":1700000000000}
            ],
            "noteTypes": [],
            "notes": [],
            "cards": [
                {"id":"card","deckId":"deck","front":"letter-a","back":"a","createdAt":1700000000000},
                {"id":"child-card","deckId":"child","front":"padi","back":"study","createdAt":1700000000000},
                {"id":"other-card","deckId":"other","front":"hola","back":"hello","createdAt":1700000000000}
            ],
            "cardProgress": [],
            "sessions": [],
            "reviews": [],
            "activeSession": null
        }"#;

        session.load_snapshot(snapshot);
        session.dispatch(
            r#"{
                "type": "startSession",
                "sessionId": "session",
                "deckId": "deck",
                "queue": [{"id":"card","deckId":"deck","front":"letter-a","back":"a","createdAt":1700000000000}],
                "startedAt": 1700000000000
            }"#,
        );
        for (review_id, card_id, rating, reviewed_at) in [
            ("again", "card", "again", NOW + 10),
            ("good", "card", "good", NOW + 20),
            ("hard-child", "child-card", "hard", NOW + 25),
            ("easy-other", "other-card", "easy", NOW + 30),
        ] {
            session.dispatch(&format!(
                r#"{{
                    "type": "rateCard",
                    "reviewId": "{review_id}",
                    "sessionId": "session",
                    "cardId": "{card_id}",
                    "rating": "{rating}",
                    "reviewedAt": {reviewed_at}
                }}"#
            ));
        }

        let value: Value =
            serde_json::from_str(&session.review_history("deck", NOW, NOW + 30)).unwrap();

        assert_eq!(value["ok"], true);
        assert_eq!(value["history"]["deckId"], "deck");
        assert_eq!(value["history"]["totalReviews"], 3);
        assert_eq!(value["history"]["correctReviews"], 2);
        assert_eq!(value["history"]["uniqueCards"], 2);
        assert_eq!(value["history"]["ratingCounts"]["again"], 1);
        assert_eq!(value["history"]["ratingCounts"]["good"], 1);
        assert_eq!(value["history"]["ratingCounts"]["hard"], 1);
        assert_eq!(value["history"]["firstReviewedAt"], NOW + 10);
        assert_eq!(value["history"]["lastReviewedAt"], NOW + 25);
    }

    #[test]
    fn generated_cards_uses_note_type_templates() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
            "noteTypes": [{
                "id": "basic",
                "name": "Basic",
                "fields": [
                    {"id": "front", "name": "Front", "required": true, "ordinal": 0},
                    {"id": "back", "name": "Back", "required": true, "ordinal": 1}
                ],
                "templates": [{
                    "id": "forward",
                    "name": "Forward",
                    "frontTemplate": "{{Front}}",
                    "backTemplate": "{{Back}}",
                    "requiredFieldNames": ["Front", "Back"],
                    "ordinal": 0
                }],
                "createdAt": 1700000000000,
                "updatedAt": 1700000000000
            }],
            "notes": [{
                "id": "note",
                "noteTypeId": "basic",
                "deckId": "deck",
                "fields": [
                    {"fieldId": "front", "value": "letter-a"},
                    {"fieldId": "back", "value": "a"}
                ],
                "tags": ["tamil"],
                "createdAt": 1700000000000,
                "updatedAt": 1700000000000
            }],
            "cards": [],
            "cardProgress": [],
            "sessions": [],
            "reviews": [],
            "activeSession": null
        }"#;

        session.load_snapshot(snapshot);
        let value: Value = serde_json::from_str(&session.generated_cards("basic", "note")).unwrap();

        assert_eq!(value["ok"], true);
        assert_eq!(value["cards"][0]["front"], "letter-a");
        assert_eq!(value["cards"][0]["back"], "a");

        let materialized: Value =
            serde_json::from_str(&session.materialized_cards("basic", "note", NOW + 1)).unwrap();

        assert_eq!(materialized["ok"], true);
        assert_eq!(materialized["cards"][0]["id"], "note::forward");
        assert_eq!(materialized["cards"][0]["createdAt"], NOW + 1);
        assert_eq!(materialized["cards"][0]["lineage"]["noteId"], "note");
        assert_eq!(materialized["cards"][0]["lineage"]["templateId"], "forward");
    }

    #[test]
    fn generated_cards_expose_cloze_ordinals() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [{"id":"deck","name":"Spanish","description":"Grammar","createdAt":1700000000000}],
            "noteTypes": [{
                "id": "cloze",
                "name": "Cloze",
                "fields": [
                    {"id": "text", "name": "Text", "required": true, "ordinal": 0},
                    {"id": "extra", "name": "Extra", "required": false, "ordinal": 1}
                ],
                "templates": [{
                    "id": "cloze",
                    "name": "Cloze",
                    "frontTemplate": "{{cloze:Text}}",
                    "backTemplate": "{{cloze:Text}}<hr>{{Extra}}",
                    "requiredFieldNames": ["Text"],
                    "ordinal": 0
                }],
                "createdAt": 1700000000000,
                "updatedAt": 1700000000000
            }],
            "notes": [{
                "id": "note",
                "noteTypeId": "cloze",
                "deckId": "deck",
                "fields": [
                    {"fieldId": "text", "value": "A {{c1::root::base}} plus a {{c2::suffix}}."},
                    {"fieldId": "extra", "value": "etymology"}
                ],
                "tags": ["grammar"],
                "createdAt": 1700000000000,
                "updatedAt": 1700000000000
            }],
            "cards": [],
            "cardProgress": [],
            "sessions": [],
            "reviews": [],
            "activeSession": null
        }"#;

        session.load_snapshot(snapshot);
        let value: Value = serde_json::from_str(&session.generated_cards("cloze", "note")).unwrap();

        assert_eq!(value["ok"], true);
        assert_eq!(value["cards"].as_array().unwrap().len(), 2);
        assert_eq!(value["cards"][0]["id"], "note::cloze::c1");
        assert_eq!(value["cards"][0]["clozeOrdinal"], 1);
        assert_eq!(value["cards"][0]["front"], "A [base] plus a suffix.");
        assert_eq!(value["cards"][1]["id"], "note::cloze::c2");
        assert_eq!(value["cards"][1]["clozeOrdinal"], 2);
        assert_eq!(value["cards"][1]["front"], "A root plus a [...].");

        let materialized: Value =
            serde_json::from_str(&session.materialized_cards("cloze", "note", NOW + 1)).unwrap();

        assert_eq!(materialized["ok"], true);
        assert_eq!(materialized["cards"][0]["lineage"]["templateId"], "cloze");
        assert_eq!(materialized["cards"][0]["lineage"]["clozeOrdinal"], 1);
    }

    #[test]
    fn search_cards_returns_core_browser_results() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
            "noteTypes": [],
            "notes": [{
                "id": "note",
                "noteTypeId": "basic",
                "deckId": "deck",
                "fields": [{"fieldId": "front", "value": "uyir letter"}],
                "tags": ["script", "tamil"],
                "createdAt": 1700000000000,
                "updatedAt": 1700000000000
            }],
            "cards": [
                {"id":"note::forward","deckId":"deck","front":"letter-a","back":"a","createdAt":1700000000000},
                {"id":"other","deckId":"deck","front":"number-one","back":"one","createdAt":1700000000000}
            ],
            "cardProgress": [{
                "cardId": "note::forward",
                "state": "review",
                "interval": 1,
                "easeFactor": 2.5,
                "nextDueAt": 1699999999999,
                "learningStepIndex": null,
                "buriedUntil": null,
                "suspendedAt": null,
                "timesSeen": 1,
                "timesCorrect": 1,
                "timesIncorrect": 0,
                "lastSeenAt": 1699913600000,
                "flag": "blue",
                "markedAt": 1700000000000
            }],
            "sessions": [],
            "reviews": [],
            "activeSession": null
        }"#;

        session.load_snapshot(snapshot);
        let value: Value = serde_json::from_str(
            &session.search_cards("deck:tamil tag:script is:due is:marked flag:blue", NOW),
        )
        .unwrap();

        assert_eq!(value["ok"], true);
        assert_eq!(value["results"][0]["card"]["id"], "note::forward");
        assert_eq!(value["results"][0]["progress"]["flag"], "blue");

        let browser_props: Value = serde_json::from_str(
            &session.engram_browser_props("deck:tamil tag:script cid:note::forward", NOW),
        )
        .unwrap();

        assert_eq!(browser_props["ok"], true);
        assert_eq!(
            browser_props["props"]["browser-query"],
            "deck:tamil tag:script cid:note::forward"
        );
        assert_eq!(
            browser_props["props"]["browser-results"],
            json!(["letter-a -> a"])
        );
        assert_eq!(
            browser_props["props"]["browser-result-card-ids"],
            json!(["note::forward"])
        );
        assert_eq!(
            browser_props["props"]["browser-result-note-ids"],
            json!(["note"])
        );
        assert_eq!(
            browser_props["props"]["browser-result-template-ids"],
            json!(["forward"])
        );
        assert_eq!(
            browser_props["props"]["browser-result-states"],
            json!(["due"])
        );
        assert_eq!(
            browser_props["props"]["browser-result-flags"],
            json!(["blue"])
        );
        assert_eq!(
            browser_props["props"]["browser-results-summary"],
            "1 matching card"
        );
        assert_eq!(browser_props["props"]["browser-selected-index"], 0);
        assert_eq!(
            browser_props["props"]["browser-selected-card-id"],
            "note::forward"
        );
        assert_eq!(browser_props["props"]["browser-selected-note-id"], "note");
        assert_eq!(
            browser_props["props"]["browser-selected-template-id"],
            "forward"
        );
        assert_eq!(browser_props["props"]["browser-selected-state"], "due");
        assert_eq!(browser_props["props"]["browser-selected-flag"], "blue");
        assert_eq!(browser_props["props"]["browser-flag-value"], "blue");
        assert_eq!(
            browser_props["props"]["browser-flag-options"],
            json!([
                "none",
                "red",
                "orange",
                "green",
                "blue",
                "pink",
                "turquoise",
                "purple"
            ])
        );
        assert_eq!(browser_props["props"]["browser-flag-open"], false);

        let empty_query_props: Value =
            serde_json::from_str(&session.engram_browser_props("", NOW)).unwrap();
        assert_eq!(empty_query_props["ok"], true);
        assert_eq!(
            empty_query_props["props"]["browser-query"],
            "is:due OR is:new"
        );

        let empty_field_search: Value =
            serde_json::from_str(&session.search_cards("kind:review", NOW)).unwrap();

        assert_eq!(empty_field_search["ok"], true);
        assert_eq!(empty_field_search["results"], json!([]));

        let empty_field_browser_props: Value =
            serde_json::from_str(&session.engram_browser_props("kind:review", NOW)).unwrap();
        assert_eq!(empty_field_browser_props["ok"], true);
        assert_eq!(
            empty_field_browser_props["props"]["browser-results-summary"],
            "No matching cards"
        );
    }

    #[test]
    fn browser_props_label_imported_anki_state_and_flags() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
            "noteTypes": [],
            "notes": [],
            "cards": [
                {"id":"review-due","deckId":"deck","front":"due","back":"d","createdAt":1700000000000},
                {"id":"suspended","deckId":"deck","front":"suspended","back":"s","createdAt":1700000000001}
            ],
            "cardProgress": [],
            "sessions": [],
            "reviews": [],
            "externalSources": [
                {
                    "target":"collection",
                    "targetId":"collection",
                    "source":"anki-v11",
                    "originalId":"1",
                    "data":{"createdAtDays":"19475"}
                },
                {
                    "target":"card",
                    "targetId":"review-due",
                    "source":"anki-v11",
                    "originalId":"review-due",
                    "data":{"kind":"2","queue":"2","due":"200","flags":"4"}
                },
                {
                    "target":"card",
                    "targetId":"suspended",
                    "source":"anki-v11",
                    "originalId":"suspended",
                    "data":{"kind":"2","queue":"-1","due":"200","flags":"1"}
                }
            ],
            "activeSession": null
        }"#;

        session.load_snapshot(snapshot);
        let due_props: Value =
            serde_json::from_str(&session.engram_browser_props("cid:review-due", NOW)).unwrap();
        assert_eq!(due_props["ok"], true);
        assert_eq!(due_props["props"]["browser-result-states"], json!(["due"]));
        assert_eq!(due_props["props"]["browser-result-flags"], json!(["blue"]));
        assert_eq!(due_props["props"]["browser-selected-state"], "due");
        assert_eq!(due_props["props"]["browser-selected-flag"], "blue");

        let suspended_props: Value =
            serde_json::from_str(&session.engram_browser_props("cid:suspended", NOW)).unwrap();
        assert_eq!(
            suspended_props["props"]["browser-selected-state"],
            "suspended"
        );
        assert_eq!(suspended_props["props"]["browser-selected-flag"], "red");
    }

    #[test]
    fn app_browser_state_filter_composes_with_query() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
            "noteTypes": [],
            "notes": [],
            "cards": [
                {"id":"due-card","deckId":"deck","front":"uyir vowel","back":"letter","createdAt":1700000000000},
                {"id":"new-card","deckId":"deck","front":"mei consonant","back":"letter","createdAt":1700000000000}
            ],
            "cardProgress": [{
                "cardId": "due-card",
                "state": "review",
                "interval": 1,
                "easeFactor": 2.5,
                "nextDueAt": 1699999999999,
                "learningStepIndex": null,
                "buriedUntil": null,
                "suspendedAt": null,
                "timesSeen": 1,
                "timesCorrect": 1,
                "timesIncorrect": 0,
                "lastSeenAt": 1699913600000,
                "flag": null,
                "markedAt": null
            }],
            "sessions": [],
            "reviews": [],
            "activeSession": null
        }"#;

        session.load_snapshot(snapshot);

        let opened: Value = serde_json::from_str(&session.handle_engram_app_event(
            "onBrowserToggleFilter",
            "deck",
            NOW,
        ))
        .unwrap();
        assert_eq!(opened["ok"], true);
        assert_eq!(opened["event"], "onBrowserToggleFilter");
        assert_eq!(opened["props"]["browser-filter-open"], true);

        let due: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onBrowserSetFilter","value":"Due"}"#,
            "deck",
            NOW,
        ))
        .unwrap();
        assert_eq!(due["ok"], true);
        assert_eq!(due["event"], "onBrowserSetFilter");
        assert_eq!(due["props"]["browser-filter-value"], "Due");
        assert_eq!(due["props"]["browser-filter-open"], false);
        assert_eq!(due["props"]["browser-result-card-ids"], json!(["due-card"]));

        let query: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onBrowserQueryChange","value":"mei"}"#,
            "deck",
            NOW,
        ))
        .unwrap();
        assert_eq!(query["props"]["browser-filter-value"], "Due");
        assert_eq!(
            query["props"]["browser-results-summary"],
            "No matching cards"
        );

        let new: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onBrowserSetFilter","value":"new"}"#,
            "deck",
            NOW,
        ))
        .unwrap();
        assert_eq!(new["props"]["browser-filter-value"], "New");
        assert_eq!(new["props"]["browser-result-card-ids"], json!(["new-card"]));
    }

    #[test]
    fn app_browser_custom_study_events_rebuild_and_empty_filtered_deck() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [
                {"id":"source","name":"Spanish","description":"Words","createdAt":1700000000000},
                {"id":"filtered","name":"Custom Study","description":"Filtered deck","createdAt":1700000000000}
            ],
            "noteTypes": [],
            "notes": [],
            "cards": [
                {"id":"card-one","deckId":"source","front":"madre","back":"mother","createdAt":1700000000000},
                {"id":"card-two","deckId":"source","front":"padre","back":"father","createdAt":1700000000000}
            ],
            "cardProgress": [],
            "sessions": [],
            "reviews": [],
            "deckOptions": [],
            "externalSources": [],
            "mediaAssets": [],
            "activeSession": null
        }"#;

        session.load_snapshot(snapshot);

        let initial: Value = serde_json::from_str(&session.engram_app_props("filtered", NOW))
            .expect("initial props should parse");
        assert_eq!(
            initial["props"]["browser-custom-study-limit-value"],
            json!(DEFAULT_CUSTOM_STUDY_LIMIT)
        );
        assert_eq!(
            initial["props"]["browser-custom-study-reschedule-value"],
            true
        );

        let limited: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onBrowserCustomStudyLimitChange","value":1}"#,
            "filtered",
            NOW + 1,
        ))
        .unwrap();
        assert_eq!(limited["ok"], true);
        assert_eq!(
            limited["props"]["browser-custom-study-limit-value"],
            json!(1)
        );

        let reschedule: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onBrowserCustomStudyRescheduleChange","checked":false}"#,
            "filtered",
            NOW + 2,
        ))
        .unwrap();
        assert_eq!(reschedule["ok"], true);
        assert_eq!(
            reschedule["props"]["browser-custom-study-reschedule-value"],
            false
        );

        serde_json::from_str::<Value>(&session.handle_engram_app_event(
            r#"{"event":"onBrowserQueryChange","value":"deck:source"}"#,
            "filtered",
            NOW + 3,
        ))
        .unwrap();
        let rebuilt: Value = serde_json::from_str(&session.handle_engram_app_event(
            "browser-rebuild-custom-study",
            "filtered",
            NOW + 4,
        ))
        .unwrap();

        assert_eq!(rebuilt["ok"], true);
        assert_eq!(rebuilt["event"], "onBrowserRebuildFilteredDeck");
        let cards = rebuilt["state"]["cards"].as_array().unwrap();
        assert_eq!(
            cards.iter().find(|card| card["id"] == "card-one").unwrap()["deckId"],
            "filtered"
        );
        assert_eq!(
            cards.iter().find(|card| card["id"] == "card-two").unwrap()["deckId"],
            "source"
        );

        let sources = rebuilt["state"]["externalSources"].as_array().unwrap();
        let deck_source = sources
            .iter()
            .find(|source| source["target"] == "deck" && source["targetId"] == "filtered")
            .expect("filtered deck source should be tracked");
        assert_eq!(deck_source["data"]["dyn"], "1");
        assert_eq!(deck_source["data"]["search"], "deck:source");
        assert_eq!(deck_source["data"]["limit"], "1");
        assert_eq!(deck_source["data"]["resched"], "false");
        assert_eq!(deck_source["data"]["rebuiltAt"], (NOW + 4).to_string());
        let card_source = sources
            .iter()
            .find(|source| source["target"] == "card" && source["targetId"] == "card-one")
            .expect("moved card source should keep original deck");
        assert_eq!(card_source["data"]["originalDeckId"], "source");

        let emptied: Value = serde_json::from_str(&session.handle_engram_app_event(
            "onBrowserEmptyFilteredDeck",
            "filtered",
            NOW + 5,
        ))
        .unwrap();
        assert_eq!(emptied["ok"], true);
        assert_eq!(emptied["event"], "onBrowserEmptyFilteredDeck");
        assert!(emptied["state"]["cards"]
            .as_array()
            .unwrap()
            .iter()
            .all(|card| card["deckId"] == "source"));
        assert!(!emptied["state"]["externalSources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|source| source["target"] == "card"));
    }

    #[test]
    fn app_browser_deck_current_uses_selected_deck_context() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [
                {"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000},
                {"id":"child","name":"Tamil::Verbs","description":"Grammar","createdAt":1700000000000},
                {"id":"other-deck","name":"Spanish","description":"Words","createdAt":1700000000000}
            ],
            "noteTypes": [],
            "notes": [],
            "cards": [
                {"id":"tamil-card","deckId":"deck","front":"amma","back":"mother","createdAt":1700000000000},
                {"id":"spanish-card","deckId":"other-deck","front":"madre","back":"mother","createdAt":1700000000000}
            ],
            "cardProgress": [],
            "sessions": [],
            "reviews": [],
            "activeSession": null
        }"#;

        session.load_snapshot(snapshot);

        let context_free: Value =
            serde_json::from_str(&session.engram_browser_props("deck:current", NOW)).unwrap();
        assert_eq!(context_free["ok"], true);
        assert_eq!(
            context_free["props"]["browser-results-summary"],
            "No matching cards"
        );

        let query_change: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onBrowserQueryChange","value":"deck:current"}"#,
            "deck",
            NOW,
        ))
        .unwrap();
        assert_eq!(query_change["ok"], true);
        assert_eq!(
            query_change["props"]["browser-result-card-ids"],
            json!(["tamil-card"])
        );

        let other_deck_props: Value =
            serde_json::from_str(&session.engram_app_props("other-deck", NOW)).unwrap();
        assert_eq!(other_deck_props["ok"], true);
        assert_eq!(
            other_deck_props["props"]["browser-result-card-ids"],
            json!(["spanish-card"])
        );

        let open_intent: Value = serde_json::from_str(&session.handle_engram_app_event(
            "onBrowserOpenSelected",
            "other-deck",
            NOW,
        ))
        .unwrap();
        assert_eq!(open_intent["ok"], true);
        assert_eq!(open_intent["hostIntent"]["cardId"], "spanish-card");
    }

    #[test]
    fn app_select_deck_event_updates_shared_deck_context() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [
                {"id":"tamil","name":"Tamil","description":"Script","createdAt":1700000000000},
                {"id":"spanish","name":"Spanish","description":"Words","createdAt":1700000000000}
            ],
            "noteTypes": [],
            "notes": [],
            "cards": [
                {"id":"tamil-card-a","deckId":"tamil","front":"amma","back":"mother","createdAt":1700000000000},
                {"id":"tamil-card-b","deckId":"tamil","front":"appa","back":"father","createdAt":1700000000000},
                {"id":"spanish-card","deckId":"spanish","front":"madre","back":"mother","createdAt":1700000000000}
            ],
            "cardProgress": [],
            "sessions": [],
            "reviews": [],
            "activeSession": null
        }"#;

        session.load_snapshot(snapshot);

        let initial: Value = serde_json::from_str(&session.engram_app_props("", NOW)).unwrap();
        assert_eq!(initial["props"]["deck-name"], "Tamil");
        assert_eq!(initial["props"]["deck-names"], json!(["Tamil", "Spanish"]));
        assert_eq!(initial["props"]["deck-total-value"], "2");

        let selected: Value = serde_json::from_str(&session.handle_engram_app_event(
            r#"{"event":"onSelectDeck","value":"Spanish"}"#,
            "",
            NOW + 1,
        ))
        .unwrap();
        assert_eq!(selected["ok"], true);
        assert_eq!(selected["event"], "onSelectDeck");
        assert_eq!(selected["props"]["deck-name"], "Spanish");
        assert_eq!(selected["props"]["deck-total-value"], "1");

        let persisted: Value =
            serde_json::from_str(&session.engram_app_props("", NOW + 2)).unwrap();
        assert_eq!(persisted["props"]["deck-name"], "Spanish");

        let import_intent: Value =
            serde_json::from_str(&session.handle_engram_app_event("onImportAnki", "", NOW + 3))
                .unwrap();
        assert_eq!(import_intent["hostIntent"]["deckId"], "spanish");

        let explicit_tamil: Value =
            serde_json::from_str(&session.engram_app_props("tamil", NOW + 4)).unwrap();
        assert_eq!(explicit_tamil["props"]["deck-name"], "Tamil");
        assert_eq!(explicit_tamil["props"]["deck-total-value"], "2");
    }

    #[test]
    fn export_and_parse_cards_csv() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [
                {"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000},
                {"id":"child","name":"Tamil::Verbs","description":"Grammar","createdAt":1700000000000},
                {"id":"other-deck","name":"Spanish","description":"Words","createdAt":1700000000000}
            ],
            "noteTypes": [],
            "notes": [],
            "cards": [
                {"id":"card","deckId":"deck","front":"letter, \"a\"","back":"line one\nline two","createdAt":1700000000000},
                {"id":"child-card","deckId":"child","front":"padi","back":"study","createdAt":1700000000000},
                {"id":"other","deckId":"other-deck","front":"hola","back":"hello","createdAt":1700000000000}
            ],
            "cardProgress": [],
            "sessions": [],
            "reviews": [],
            "activeSession": null
        }"#;

        session.load_snapshot(snapshot);
        let exported: Value = serde_json::from_str(&session.export_cards_csv("deck")).unwrap();

        assert_eq!(exported["ok"], true);
        let csv = exported["csv"].as_str().unwrap();
        assert!(csv.contains("\"letter, \"\"a\"\"\""));
        assert!(csv.contains("child-card,child,padi,study"), "{csv}");
        assert!(!csv.contains("other-deck"));

        let parsed: Value = serde_json::from_str(&session.parse_cards_csv(csv)).unwrap();
        assert_eq!(parsed["ok"], true);
        assert_eq!(parsed["cards"][0]["id"], "card");
        assert_eq!(parsed["cards"][0]["front"], "letter, \"a\"");
        assert_eq!(parsed["cards"][1]["id"], "child-card");

        let error: Value =
            serde_json::from_str(&session.parse_cards_csv("front,back\nx,y\n")).unwrap();
        assert_eq!(error["ok"], false);
        assert_eq!(error["row"], 1);
    }

    #[test]
    fn export_anki_basic_tsv_uses_anki_text_headers() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [
                {"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000},
                {"id":"child","name":"Tamil::Verbs","description":"Grammar","createdAt":1700000000000}
            ],
            "noteTypes": [],
            "notes": [],
            "cards": [
                {"id":"card","deckId":"deck","front":"letter\t\"a\"","back":"line one\nline two","createdAt":1700000000000},
                {"id":"child-card","deckId":"child","front":"padi","back":"study","createdAt":1700000000000}
            ],
            "cardProgress": [],
            "sessions": [],
            "reviews": [],
            "activeSession": null
        }"#;

        session.load_snapshot(snapshot);
        let exported: Value = serde_json::from_str(&session.export_anki_basic_tsv(
            "deck",
            "Tamil::Script",
            "Basic",
            false,
        ))
        .unwrap();

        assert_eq!(exported["ok"], true);
        let tsv = exported["tsv"].as_str().unwrap();
        assert!(tsv.starts_with(
            "#separator:tab\n#html:false\n#notetype:Basic\n#deck:Tamil::Script\n#columns:Front\tBack\n"
        ));
        assert!(tsv.contains("\"letter\t\"\"a\"\"\"\t\"line one\nline two\"\n"));
        assert!(tsv.contains("padi\tstudy\n"));
    }

    #[test]
    fn export_anki_notes_tsv_uses_note_fields_and_tags() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [
                {"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000},
                {"id":"child","name":"Tamil::Verbs","description":"Grammar","createdAt":1700000000000}
            ],
            "noteTypes": [{
                "id": "basic",
                "name": "Basic",
                "fields": [
                    {"id": "front", "name": "Front", "required": true, "ordinal": 0},
                    {"id": "back", "name": "Back", "required": true, "ordinal": 1}
                ],
                "templates": [{
                    "id": "forward",
                    "name": "Forward",
                    "frontTemplate": "{{Front}}",
                    "backTemplate": "{{Back}}",
                    "requiredFieldNames": ["Front", "Back"],
                    "ordinal": 0
                }],
                "createdAt": 1700000000000,
                "updatedAt": 1700000000000
            }],
            "notes": [{
                "id": "note",
                "noteTypeId": "basic",
                "deckId": "deck",
                "fields": [
                    {"fieldId": "front", "value": "letter\t\"a\""},
                    {"fieldId": "back", "value": "line one\nline two"}
                ],
                "tags": ["tamil", "script"],
                "createdAt": 1700000000000,
                "updatedAt": 1700000000000
            }, {
                "id": "child-note",
                "noteTypeId": "basic",
                "deckId": "child",
                "fields": [
                    {"fieldId": "front", "value": "padi"},
                    {"fieldId": "back", "value": "study"}
                ],
                "tags": ["tamil", "verb"],
                "createdAt": 1700000000000,
                "updatedAt": 1700000000000
            }],
            "cards": [],
            "cardProgress": [],
            "sessions": [],
            "reviews": [],
            "externalSources": [{
                "target": "note",
                "targetId": "note",
                "source": "anki-text",
                "originalId": "stable-guid",
                "data": {"guid": "stable-guid"}
            }],
            "activeSession": null
        }"#;

        session.load_snapshot(snapshot);
        let exported: Value = serde_json::from_str(
            &session.export_anki_notes_tsv("basic", "deck", "Tamil", "", false),
        )
        .unwrap();

        assert_eq!(exported["ok"], true);
        let tsv = exported["tsv"].as_str().unwrap();
        assert!(
            tsv.starts_with(
                "#separator:tab\n#html:false\n#notetype:Basic\n#deck:Tamil\n#guid column:3\n#deck column:4\n#columns:Front\tBack\tGuid\tDeck\tTags\n"
            ),
            "got:\n{tsv}"
        );
        assert!(tsv.contains(
            "\"letter\t\"\"a\"\"\"\t\"line one\nline two\"\tstable-guid\tTamil\ttamil script\n"
        ));
        assert!(tsv.contains("padi\tstudy\t\tTamil::Verbs\ttamil verb\n"));
    }

    #[test]
    fn parse_basic_cards_csv_generates_deterministic_cards() {
        let session = EngramSession::new();
        let value: Value = serde_json::from_str(&session.parse_basic_cards_csv(
            "front,back\nletter-a,a\nletter-aa,aa\n",
            "deck",
            "import",
            NOW,
        ))
        .unwrap();

        assert_eq!(value["ok"], true);
        assert_eq!(value["cards"][0]["id"], "import-1");
        assert_eq!(value["cards"][0]["deckId"], "deck");
        assert_eq!(value["cards"][0]["createdAt"], NOW);
        assert_eq!(value["cards"][1]["id"], "import-2");

        let error: Value = serde_json::from_str(&session.parse_basic_cards_csv(
            "front,back\nfront\n",
            "deck",
            "import",
            NOW,
        ))
        .unwrap();
        assert_eq!(error["ok"], false);
        assert_eq!(error["row"], 2);
    }

    #[test]
    fn parse_anki_basic_tsv_generates_deterministic_cards() {
        let session = EngramSession::new();
        let value: Value = serde_json::from_str(&session.parse_anki_basic_tsv(
            "#separator:tab\n#html:false\n#columns:Front\tBack\nletter-a\ta\n\"hello\tfriend\"\t\"line one\nline two\"\n",
            "deck",
            "anki",
            NOW,
        ))
        .unwrap();

        assert_eq!(value["ok"], true);
        assert_eq!(value["cards"][0]["id"], "anki-1");
        assert_eq!(value["cards"][0]["front"], "letter-a");
        assert_eq!(value["cards"][1]["id"], "anki-2");
        assert_eq!(value["cards"][1]["front"], "hello\tfriend");
        assert_eq!(value["cards"][1]["back"], "line one\nline two");
    }

    #[test]
    fn parse_anki_notes_tsv_generates_note_model_and_cards() {
        let session = EngramSession::new();
        let value: Value = serde_json::from_str(&session.parse_anki_notes_tsv(
            "#separator:tab\n#notetype:Basic (and reversed card)\n#columns:Front\tBack\tTags\nhola\thello\tspanish common\n",
            "deck",
            "basic-reversed",
            "",
            "note",
            NOW,
        ))
        .unwrap();

        assert_eq!(value["ok"], true);
        assert_eq!(value["import"]["noteTypes"][0]["id"], "basic-reversed");
        assert_eq!(
            value["import"]["noteTypes"][0]["name"],
            "Basic (and reversed card)"
        );
        assert_eq!(value["import"]["notes"][0]["id"], "note-1");
        assert_eq!(value["import"]["notes"][0]["tags"][0], "spanish");
        assert_eq!(value["import"]["cards"].as_array().unwrap().len(), 2);
        assert_eq!(value["import"]["cards"][0]["id"], "note-1::forward");
        assert_eq!(value["import"]["cards"][1]["id"], "note-1::reverse");
        assert_eq!(value["import"]["cards"][1]["lineage"]["noteId"], "note-1");

        let error: Value = serde_json::from_str(&session.parse_anki_notes_tsv(
            "#separator:tab\n#columns:Front\tTags\nhola\tspanish\n",
            "deck",
            "basic",
            "Basic",
            "note",
            NOW,
        ))
        .unwrap();

        assert_eq!(error["ok"], false);
        assert_eq!(error["row"], 1);
    }

    #[test]
    fn merge_anki_notes_tsv_merges_note_model_and_cards_into_session() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [{"id":"deck","name":"Spanish","description":"Words","createdAt":1700000000000}],
            "noteTypes": [],
            "notes": [],
            "cards": [{"id":"local-card","deckId":"deck","front":"amma","back":"mother","createdAt":1700000000000}],
            "cardProgress": [],
            "sessions": [],
            "reviews": [],
            "activeSession": null
        }"#;
        session.load_snapshot(snapshot);

        let value: Value = serde_json::from_str(&session.merge_anki_notes_tsv(
            "#separator:tab\n#notetype:Basic (and reversed card)\n#guid column:3\n#columns:Front\tBack\tGuid\nhola\thello\tguid-123\n",
            "deck",
            "basic-reversed",
            "",
            "note",
            NOW,
        ))
        .unwrap();

        assert_eq!(value["ok"], true);
        assert_eq!(value["state"]["noteTypes"][0]["id"], "basic-reversed");
        assert_eq!(value["state"]["notes"][0]["id"], "note-1");
        let card_ids = value["state"]["cards"]
            .as_array()
            .unwrap()
            .iter()
            .map(|card| card["id"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert!(card_ids.contains(&"local-card"));
        assert!(card_ids.contains(&"note-1::forward"));
        assert!(card_ids.contains(&"note-1::reverse"));
        assert!(value["state"]["externalSources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|source| {
                source["target"] == "note"
                    && source["targetId"] == "note-1"
                    && source["source"] == "anki-text"
                    && source["originalId"] == "guid-123"
            }));
    }

    #[test]
    fn export_and_merge_anki_apkg_round_trip_through_json_facade() {
        let source = EngramSession::new_demo();
        let exported: Value = serde_json::from_str(&source.export_anki_apkg()).unwrap();
        assert_eq!(exported["ok"], true);
        let apkg = exported["apkg"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_u64().unwrap() as u8)
            .collect::<Vec<_>>();
        assert!(!apkg.is_empty());

        let mut target = EngramSession::new();
        let merged: Value = serde_json::from_str(&target.merge_anki_apkg(&apkg)).unwrap();
        assert_eq!(merged["ok"], true);
        assert!(merged["state"]["decks"].as_array().unwrap().len() >= 4);
        assert!(merged["state"]["cards"].as_array().unwrap().len() >= 5);
        assert!(merged["state"]["externalSources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|source| source["source"] == "anki-v11"));
    }

    #[test]
    fn parse_anki_notes_tsv_honors_html_header() {
        let session = EngramSession::new();
        let plain: Value = serde_json::from_str(&session.parse_anki_notes_tsv(
            "#separator:tab\n#html:false\n#notetype:Basic\n#columns:Front\tBack\n<b>hola</b>\t\"mother & aunt\"\n",
            "deck",
            "basic",
            "Basic",
            "note",
            NOW,
        ))
        .unwrap();

        assert_eq!(plain["ok"], true);
        assert_eq!(
            plain["import"]["notes"][0]["fields"][0]["value"],
            "&lt;b&gt;hola&lt;/b&gt;"
        );
        assert_eq!(
            plain["import"]["notes"][0]["fields"][1]["value"],
            "mother &amp; aunt"
        );
        assert_eq!(
            plain["import"]["cards"][0]["front"],
            "&lt;b&gt;hola&lt;/b&gt;"
        );
        assert_eq!(plain["import"]["cards"][0]["back"], "mother &amp; aunt");

        let html: Value = serde_json::from_str(&session.parse_anki_notes_tsv(
            "#separator:tab\n#html:true\n#notetype:Basic\n#columns:Front\tBack\n<b>hola</b>\t\"mother & aunt\"\n",
            "deck",
            "basic",
            "Basic",
            "note",
            NOW,
        ))
        .unwrap();

        assert_eq!(html["ok"], true);
        assert_eq!(
            html["import"]["notes"][0]["fields"][0]["value"],
            "<b>hola</b>"
        );
        assert_eq!(
            html["import"]["notes"][0]["fields"][1]["value"],
            "mother & aunt"
        );
        assert_eq!(html["import"]["cards"][0]["front"], "<b>hola</b>");
        assert_eq!(html["import"]["cards"][0]["back"], "mother & aunt");
    }

    #[test]
    fn parse_anki_notes_tsv_preserves_basic_type_answer_template() {
        let session = EngramSession::new();
        let value: Value = serde_json::from_str(&session.parse_anki_notes_tsv(
            "#separator:tab\n#notetype:Basic (type in the answer)\n#columns:Front\tBack\tTags\nhola\thello\tspanish\n",
            "deck",
            "",
            "",
            "note",
            NOW,
        ))
        .unwrap();

        assert_eq!(value["ok"], true);
        assert_eq!(
            value["import"]["noteTypes"][0]["id"],
            "basic-type-in-the-answer"
        );
        assert_eq!(
            value["import"]["noteTypes"][0]["templates"][0]["frontTemplate"],
            "{{Front}}{{type:Back}}"
        );
        assert_eq!(
            value["import"]["noteTypes"][0]["templates"][0]["backTemplate"],
            "{{FrontSide}}<hr id=answer>{{Back}}"
        );
        assert_eq!(
            value["import"]["cards"][0]["front"],
            "hola[type answer: Back]"
        );
    }

    #[test]
    fn parse_anki_notes_tsv_honors_optional_reversed_cards() {
        let session = EngramSession::new();
        let value: Value = serde_json::from_str(&session.parse_anki_notes_tsv(
            "#separator:tab\n#notetype:Basic (optional reversed card)\n#columns:Front\tBack\tAdd Reverse\tTags\nhola\thello\ty\tspanish\namma\tmother\t\ttamil\n",
            "deck",
            "",
            "",
            "note",
            NOW,
        ))
        .unwrap();

        assert_eq!(value["ok"], true);
        assert_eq!(
            value["import"]["noteTypes"][0]["fields"][2]["id"],
            "add-reverse"
        );
        assert_eq!(value["import"]["cards"].as_array().unwrap().len(), 3);
        assert_eq!(value["import"]["cards"][1]["id"], "note-1::reverse");
        assert_eq!(value["import"]["cards"][2]["id"], "note-2::forward");
    }

    #[test]
    fn parse_anki_notes_tsv_preserves_guid_sources() {
        let session = EngramSession::new();
        let value: Value = serde_json::from_str(&session.parse_anki_notes_tsv(
            "#separator:tab\n#notetype:Basic\n#guid column:3\n#columns:Front\tBack\tStableGuid\nhola\thello\tguid-123\n",
            "deck",
            "",
            "",
            "note",
            NOW,
        ))
        .unwrap();

        assert_eq!(value["ok"], true);
        assert_eq!(value["import"]["externalSources"][0]["target"], "note");
        assert_eq!(value["import"]["externalSources"][0]["targetId"], "note-1");
        assert_eq!(value["import"]["externalSources"][0]["source"], "anki-text");
        assert_eq!(
            value["import"]["externalSources"][0]["originalId"],
            "guid-123"
        );
    }

    #[test]
    fn parse_anki_notes_tsv_preserves_note_type_column_models() {
        let session = EngramSession::new();
        let value: Value = serde_json::from_str(&session.parse_anki_notes_tsv(
            "#separator:tab\n#notetype column:3\n#columns:Prompt\tAnswer\tModel\nhola\thello\tBasic (type in the answer)\namma\tmother\tBasic (and reversed card)\n",
            "deck",
            "",
            "",
            "note",
            NOW,
        ))
        .unwrap();

        assert_eq!(value["ok"], true);
        assert_eq!(value["import"]["noteTypes"].as_array().unwrap().len(), 2);
        assert_eq!(
            value["import"]["noteTypes"][0]["id"],
            "basic-type-in-the-answer"
        );
        assert_eq!(
            value["import"]["noteTypes"][1]["id"],
            "basic-and-reversed-card"
        );
        assert_eq!(
            value["import"]["notes"][0]["noteTypeId"],
            "basic-type-in-the-answer"
        );
        assert_eq!(
            value["import"]["notes"][1]["noteTypeId"],
            "basic-and-reversed-card"
        );
        assert_eq!(value["import"]["notes"][0]["fields"][0]["fieldId"], "front");
        assert_eq!(value["import"]["notes"][0]["fields"][0]["value"], "hola");
        assert_eq!(value["import"]["notes"][0]["fields"][1]["fieldId"], "back");
        assert_eq!(value["import"]["notes"][0]["fields"][1]["value"], "hello");
        assert_eq!(value["import"]["cards"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn parse_anki_notes_tsv_generates_cloze_model_and_cards() {
        let session = EngramSession::new();
        let value: Value = serde_json::from_str(&session.parse_anki_notes_tsv(
            "#separator:tab\n#notetype:Cloze\n#columns:Text\tExtra\tTags\n\"A {{c1::root::base}} plus {{c2::suffix}}\"\tetymology\tgrammar\n",
            "deck",
            "",
            "",
            "cloze-note",
            NOW,
        ))
        .unwrap();

        assert_eq!(value["ok"], true);
        assert_eq!(value["import"]["noteTypes"][0]["id"], "cloze");
        assert_eq!(
            value["import"]["noteTypes"][0]["templates"][0]["id"],
            "cloze"
        );
        assert_eq!(value["import"]["notes"][0]["fields"][0]["fieldId"], "text");
        assert_eq!(value["import"]["notes"][0]["fields"][1]["fieldId"], "extra");
        assert_eq!(value["import"]["cards"].as_array().unwrap().len(), 2);
        assert_eq!(value["import"]["cards"][0]["id"], "cloze-note-1::cloze::c1");
        assert_eq!(value["import"]["cards"][0]["clozeOrdinal"], Value::Null);
        assert_eq!(value["import"]["cards"][0]["lineage"]["clozeOrdinal"], 1);
    }

    #[test]
    fn parse_anki_notes_tsv_preserves_custom_note_type_columns() {
        let session = EngramSession::new();
        let value: Value = serde_json::from_str(&session.parse_anki_notes_tsv(
            "#separator:tab\n#notetype:Basic Grammar Story\n#columns:Infinitive\tRoot\tCognate\tTags\nhablar\tfabl-\tfable\tspanish latin\n",
            "deck",
            "",
            "",
            "custom-note",
            NOW,
        ))
        .unwrap();

        assert_eq!(value["ok"], true);
        assert_eq!(value["import"]["noteTypes"][0]["id"], "basic-grammar-story");
        assert_eq!(
            value["import"]["noteTypes"][0]["fields"][0]["id"],
            "infinitive"
        );
        assert_eq!(value["import"]["noteTypes"][0]["fields"][1]["id"], "root");
        assert_eq!(value["import"]["notes"][0]["fields"][0]["value"], "hablar");
        assert_eq!(value["import"]["notes"][0]["tags"][0], "spanish");
        assert!(value["import"]["cards"].as_array().unwrap().is_empty());
    }

    #[test]
    fn dispatch_rate_card_accepts_deck_options() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
            "noteTypes": [],
            "notes": [],
            "cards": [{"id":"card","deckId":"deck","front":"letter-a","back":"a","createdAt":1700000000000}],
            "cardProgress": [],
            "sessions": [],
            "reviews": [],
            "activeSession": null
        }"#;

        session.load_snapshot(snapshot);
        session.dispatch(
            r#"{
                "type": "startSession",
                "sessionId": "session",
                "deckId": "deck",
                "queue": [{"id":"card","deckId":"deck","front":"letter-a","back":"a","createdAt":1700000000000}],
                "startedAt": 1700000000000
            }"#,
        );
        let value: Value = serde_json::from_str(&session.dispatch(
            r#"{
                    "type": "rateCard",
                    "reviewId": "review",
                    "sessionId": "session",
                    "cardId": "card",
                    "rating": "good",
                    "reviewedAt": 1700000000000,
                    "deckOptions": {
                        "newCardsPerDay": 20,
                        "reviewsPerDay": 200,
                        "learningStepsMinutes": [2, 20],
                        "relearningStepsMinutes": [10],
                        "graduatingIntervalDays": 1,
                        "easyIntervalDays": 4,
                        "initialEaseFactor": 2.8,
                        "lapseIntervalMultiplier": 0.0
                    }
                }"#,
        ))
        .unwrap();

        assert_eq!(value["ok"], true);
        assert_eq!(value["state"]["cardProgress"][0]["state"], "learning");
        assert_eq!(value["state"]["cardProgress"][0]["easeFactor"], 2.8);
        assert_eq!(value["state"]["cardProgress"][0]["learningStepIndex"], 1);
        assert_eq!(
            value["state"]["cardProgress"][0]["nextDueAt"],
            NOW + 20 * 60 * 1000
        );
    }

    #[test]
    fn dispatch_rate_card_accepts_interval_deck_options() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
            "noteTypes": [],
            "notes": [],
            "cards": [{"id":"card","deckId":"deck","front":"letter-a","back":"a","createdAt":1700000000000}],
            "cardProgress": [{
                "cardId": "card",
                "state": "review",
                "interval": 10,
                "easeFactor": 2.5,
                "nextDueAt": 1699999999000,
                "learningStepIndex": null,
                "buriedUntil": null,
                "suspendedAt": null,
                "timesSeen": 1,
                "timesCorrect": 1,
                "timesIncorrect": 0,
                "lastSeenAt": 1699913600000
            }],
            "sessions": [],
            "reviews": [],
            "activeSession": null
        }"#;

        session.load_snapshot(snapshot);
        session.dispatch(
            r#"{
                "type": "startSession",
                "sessionId": "session",
                "deckId": "deck",
                "queue": [{"id":"card","deckId":"deck","front":"letter-a","back":"a","createdAt":1700000000000}],
                "startedAt": 1700000000000
            }"#,
        );
        let value: Value = serde_json::from_str(&session.dispatch(
            r#"{
                    "type": "rateCard",
                    "reviewId": "review",
                    "sessionId": "session",
                    "cardId": "card",
                    "rating": "good",
                    "reviewedAt": 1700000000000,
                    "deckOptions": {
                        "maximumIntervalDays": 2,
                        "reviewIntervalModifier": 10.0
                    }
                }"#,
        ))
        .unwrap();

        assert_eq!(value["ok"], true);
        assert_eq!(value["state"]["cardProgress"][0]["interval"], 2);
        assert_eq!(
            value["state"]["cardProgress"][0]["nextDueAt"],
            NOW + 2 * 24 * 60 * 60 * 1000
        );
    }

    #[test]
    fn dispatch_suspend_and_unsuspend_card() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
            "noteTypes": [],
            "notes": [],
            "cards": [{"id":"card","deckId":"deck","front":"letter-a","back":"a","createdAt":1700000000000}],
            "cardProgress": [],
            "sessions": [],
            "reviews": [],
            "activeSession": null
        }"#;

        session.load_snapshot(snapshot);
        let suspended: Value = serde_json::from_str(&session.dispatch(
            r#"{
                    "type": "suspendCard",
                    "cardId": "card",
                    "suspendedAt": 1700000000000
                }"#,
        ))
        .unwrap();

        assert_eq!(suspended["ok"], true);
        assert_eq!(suspended["state"]["cardProgress"][0]["cardId"], "card");
        assert_eq!(suspended["state"]["cardProgress"][0]["suspendedAt"], NOW);

        let unsuspended: Value = serde_json::from_str(&session.dispatch(
            r#"{
                    "type": "unsuspendCard",
                    "cardId": "card"
                }"#,
        ))
        .unwrap();

        assert_eq!(unsuspended["ok"], true);
        assert!(unsuspended["state"]["cardProgress"]
            .as_array()
            .unwrap()
            .is_empty());
    }

    #[test]
    fn dispatch_bury_and_unbury_card() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
            "noteTypes": [],
            "notes": [],
            "cards": [{"id":"card","deckId":"deck","front":"letter-a","back":"a","createdAt":1700000000000}],
            "cardProgress": [],
            "sessions": [],
            "reviews": [],
            "activeSession": null
        }"#;

        session.load_snapshot(snapshot);
        let buried: Value = serde_json::from_str(&session.dispatch(
            r#"{
                    "type": "buryCard",
                    "cardId": "card",
                    "buriedAt": 1700000000000,
                    "buriedUntil": 1700086400000
                }"#,
        ))
        .unwrap();

        assert_eq!(buried["ok"], true);
        assert_eq!(
            buried["state"]["cardProgress"][0]["buriedUntil"],
            NOW + 86_400_000
        );

        let unburied: Value = serde_json::from_str(&session.dispatch(
            r#"{
                    "type": "unburyCard",
                    "cardId": "card"
                }"#,
        ))
        .unwrap();

        assert_eq!(unburied["ok"], true);
        assert!(unburied["state"]["cardProgress"]
            .as_array()
            .unwrap()
            .is_empty());
    }

    #[test]
    fn dispatch_bury_card_siblings_uses_card_lineage() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
            "noteTypes": [],
            "notes": [],
            "cards": [
                {
                    "id":"note::forward",
                    "deckId":"deck",
                    "front":"letter-a",
                    "back":"a",
                    "createdAt":1700000000000,
                    "lineage":{"noteId":"note","noteTypeId":"basic","templateId":"forward","ordinal":0}
                },
                {
                    "id":"note::reverse",
                    "deckId":"deck",
                    "front":"a",
                    "back":"letter-a",
                    "createdAt":1700000000000,
                    "lineage":{"noteId":"note","noteTypeId":"basic","templateId":"reverse","ordinal":1}
                },
                {
                    "id":"other::forward",
                    "deckId":"deck",
                    "front":"letter-aa",
                    "back":"aa",
                    "createdAt":1700000000000,
                    "lineage":{"noteId":"other","noteTypeId":"basic","templateId":"forward","ordinal":0}
                }
            ],
            "cardProgress": [],
            "sessions": [],
            "reviews": [],
            "activeSession": null
        }"#;

        session.load_snapshot(snapshot);
        session.dispatch(
            r#"{
                "type": "startSession",
                "sessionId": "session",
                "deckId": "deck",
                "queue": [
                    {
                        "id":"note::forward",
                        "deckId":"deck",
                        "front":"letter-a",
                        "back":"a",
                        "createdAt":1700000000000,
                        "lineage":{"noteId":"note","noteTypeId":"basic","templateId":"forward","ordinal":0}
                    },
                    {
                        "id":"note::reverse",
                        "deckId":"deck",
                        "front":"a",
                        "back":"letter-a",
                        "createdAt":1700000000000,
                        "lineage":{"noteId":"note","noteTypeId":"basic","templateId":"reverse","ordinal":1}
                    },
                    {
                        "id":"other::forward",
                        "deckId":"deck",
                        "front":"letter-aa",
                        "back":"aa",
                        "createdAt":1700000000000,
                        "lineage":{"noteId":"other","noteTypeId":"basic","templateId":"forward","ordinal":0}
                    }
                ],
                "startedAt": 1700000000000
            }"#,
        );

        let buried: Value = serde_json::from_str(&session.dispatch(
            r#"{
                "type": "buryCardSiblings",
                "cardId": "note::forward",
                "buriedAt": 1700000000000,
                "buriedUntil": 1700086400000
            }"#,
        ))
        .unwrap();

        assert_eq!(buried["ok"], true);
        assert_eq!(
            buried["state"]["cardProgress"][0]["cardId"],
            "note::reverse"
        );
        assert_eq!(
            buried["state"]["cardProgress"][0]["buriedUntil"],
            NOW + 86_400_000
        );
        let queue = buried["state"]["activeSession"]["queue"]
            .as_array()
            .unwrap();
        assert_eq!(queue.len(), 2);
        assert_eq!(queue[0]["id"], "note::forward");
        assert_eq!(queue[1]["id"], "other::forward");
    }

    #[test]
    fn dispatch_rate_card_can_bury_siblings_atomically() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
            "noteTypes": [],
            "notes": [],
            "cards": [
                {
                    "id":"note::forward",
                    "deckId":"deck",
                    "front":"letter-a",
                    "back":"a",
                    "createdAt":1700000000000,
                    "lineage":{"noteId":"note","noteTypeId":"basic","templateId":"forward","ordinal":0}
                },
                {
                    "id":"note::reverse",
                    "deckId":"deck",
                    "front":"a",
                    "back":"letter-a",
                    "createdAt":1700000000000,
                    "lineage":{"noteId":"note","noteTypeId":"basic","templateId":"reverse","ordinal":1}
                }
            ],
            "cardProgress": [],
            "sessions": [],
            "reviews": [],
            "activeSession": null
        }"#;

        session.load_snapshot(snapshot);
        session.dispatch(
            r#"{
                "type": "startSession",
                "sessionId": "session",
                "deckId": "deck",
                "queue": [
                    {
                        "id":"note::forward",
                        "deckId":"deck",
                        "front":"letter-a",
                        "back":"a",
                        "createdAt":1700000000000,
                        "lineage":{"noteId":"note","noteTypeId":"basic","templateId":"forward","ordinal":0}
                    },
                    {
                        "id":"note::reverse",
                        "deckId":"deck",
                        "front":"a",
                        "back":"letter-a",
                        "createdAt":1700000000000,
                        "lineage":{"noteId":"note","noteTypeId":"basic","templateId":"reverse","ordinal":1}
                    }
                ],
                "startedAt": 1700000000000
            }"#,
        );
        session.dispatch(r#"{"type":"revealCurrentCard"}"#);

        let reviewed: Value = serde_json::from_str(&session.dispatch(
            r#"{
                "type": "rateCard",
                "reviewId": "review",
                "sessionId": "session",
                "cardId": "note::forward",
                "rating": "good",
                "reviewedAt": 1700000000000,
                "burySiblingsUntil": 1700086400000
            }"#,
        ))
        .unwrap();

        assert_eq!(reviewed["ok"], true);
        assert_eq!(
            reviewed["state"]["reviews"][0]["siblingProgressSnapshots"][0]["cardId"],
            "note::reverse"
        );
        assert_eq!(
            reviewed["state"]["cardProgress"][1]["buriedUntil"],
            NOW + 86_400_000
        );
        assert_eq!(
            reviewed["state"]["activeSession"]["queue"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn dispatch_flag_and_mark_card() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
            "noteTypes": [],
            "notes": [],
            "cards": [{"id":"card","deckId":"deck","front":"letter-a","back":"a","createdAt":1700000000000}],
            "cardProgress": [],
            "sessions": [],
            "reviews": [],
            "activeSession": null
        }"#;

        session.load_snapshot(snapshot);
        let flagged: Value = serde_json::from_str(&session.dispatch(
            r#"{
                    "type": "setCardFlag",
                    "cardId": "card",
                    "flag": "turquoise",
                    "flaggedAt": 1700000000000
                }"#,
        ))
        .unwrap();

        assert_eq!(flagged["ok"], true);
        assert_eq!(flagged["state"]["cardProgress"][0]["flag"], "turquoise");

        let marked: Value = serde_json::from_str(&session.dispatch(
            r#"{
                    "type": "markCard",
                    "cardId": "card",
                    "markedAt": 1700000000001
                }"#,
        ))
        .unwrap();

        assert_eq!(marked["ok"], true);
        assert_eq!(marked["state"]["cardProgress"][0]["flag"], "turquoise");
        assert_eq!(marked["state"]["cardProgress"][0]["markedAt"], NOW + 1);

        let unflagged: Value = serde_json::from_str(&session.dispatch(
            r#"{
                    "type": "setCardFlag",
                    "cardId": "card",
                    "flag": null,
                    "flaggedAt": 1700000000002
                }"#,
        ))
        .unwrap();

        assert_eq!(unflagged["ok"], true);
        assert!(unflagged["state"]["cardProgress"][0].get("flag").is_none());
        assert_eq!(unflagged["state"]["cardProgress"][0]["markedAt"], NOW + 1);

        let unmarked: Value = serde_json::from_str(&session.dispatch(
            r#"{
                    "type": "unmarkCard",
                    "cardId": "card"
                }"#,
        ))
        .unwrap();

        assert_eq!(unmarked["ok"], true);
        assert!(unmarked["state"]["cardProgress"]
            .as_array()
            .unwrap()
            .is_empty());
    }

    #[test]
    fn load_snapshot_accepts_progress_without_flag_or_mark_fields() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
            "noteTypes": [],
            "notes": [],
            "cards": [{"id":"card","deckId":"deck","front":"letter-a","back":"a","createdAt":1700000000000}],
            "cardProgress": [{
                "cardId": "card",
                "state": "review",
                "interval": 1,
                "easeFactor": 2.5,
                "nextDueAt": 1700000000000,
                "learningStepIndex": null,
                "buriedUntil": null,
                "suspendedAt": null,
                "timesSeen": 1,
                "timesCorrect": 1,
                "timesIncorrect": 0,
                "lastSeenAt": 1699913600000
            }],
            "sessions": [],
            "reviews": [],
            "activeSession": null
        }"#;

        let loaded: Value = serde_json::from_str(&session.load_snapshot(snapshot)).unwrap();

        assert_eq!(loaded["ok"], true);
        assert!(loaded["state"]["cardProgress"][0].get("flag").is_none());
        assert!(loaded["state"]["cardProgress"][0].get("markedAt").is_none());
    }

    #[test]
    fn dispatch_undo_last_review_restores_previous_state() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
            "noteTypes": [],
            "notes": [],
            "cards": [{"id":"card","deckId":"deck","front":"letter-a","back":"a","createdAt":1700000000000}],
            "cardProgress": [],
            "sessions": [],
            "reviews": [],
            "activeSession": null
        }"#;

        session.load_snapshot(snapshot);
        session.dispatch(
            r#"{
                "type": "startSession",
                "sessionId": "session",
                "deckId": "deck",
                "queue": [{"id":"card","deckId":"deck","front":"letter-a","back":"a","createdAt":1700000000000}],
                "startedAt": 1700000000000
            }"#,
        );
        session.dispatch(r#"{"type": "revealCurrentCard"}"#);
        session.dispatch(
            r#"{
                "type": "rateCard",
                "reviewId": "review",
                "sessionId": "session",
                "cardId": "card",
                "rating": "good",
                "reviewedAt": 1700000000000
            }"#,
        );
        session.dispatch(r#"{"type": "advanceSession"}"#);

        let undone: Value = serde_json::from_str(&session.dispatch(
            r#"{
                    "type": "undoLastReview",
                    "sessionId": "session"
                }"#,
        ))
        .unwrap();

        assert_eq!(undone["ok"], true);
        assert!(undone["state"]["cardProgress"]
            .as_array()
            .unwrap()
            .is_empty());
        assert!(undone["state"]["reviews"].as_array().unwrap().is_empty());
        assert_eq!(undone["state"]["sessions"][0]["cardsReviewed"], 0);
        assert_eq!(undone["state"]["activeSession"]["currentIndex"], 0);
        assert_eq!(undone["state"]["activeSession"]["revealed"], true);
    }

    #[test]
    fn invalid_json_returns_error_instead_of_panicking() {
        let mut session = EngramSession::new();
        let value: Value = serde_json::from_str(&session.dispatch("{not-json")).unwrap();

        assert_eq!(value["ok"], false);
        assert!(value["error"].as_str().unwrap().contains("invalid command"));
    }
}
