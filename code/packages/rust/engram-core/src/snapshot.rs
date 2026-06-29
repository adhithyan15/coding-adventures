use crate::model::{
    AppState, Card, CardProgress, Deck, DeckOptionsPreset, ExternalSourceRecord, MediaAssetRecord,
    Note, NoteType, Review, Session,
};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub const ENGRAM_SNAPSHOT_APP: &str = "engram";
pub const ENGRAM_SNAPSHOT_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct EngramSnapshot {
    pub app: String,
    pub version: u32,
    pub exported_at: u64,
    pub decks: Vec<Deck>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub note_types: Vec<NoteType>,
    #[cfg_attr(feature = "serde", serde(default))]
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotError {
    pub message: String,
}

impl EngramSnapshot {
    pub fn from_state(state: &AppState, exported_at: u64) -> Self {
        Self {
            app: ENGRAM_SNAPSHOT_APP.to_string(),
            version: ENGRAM_SNAPSHOT_VERSION,
            exported_at,
            decks: state.decks.clone(),
            note_types: state.note_types.clone(),
            notes: state.notes.clone(),
            cards: state.cards.clone(),
            card_progress: state.card_progress.clone(),
            sessions: state.sessions.clone(),
            reviews: state.reviews.clone(),
            deck_options: state.deck_options.clone(),
            external_sources: state.external_sources.clone(),
            media_assets: state.media_assets.clone(),
        }
    }

    pub fn validate(&self) -> Result<(), SnapshotError> {
        if self.app != ENGRAM_SNAPSHOT_APP {
            return Err(SnapshotError {
                message: "The selected file is not an Engram backup.".to_string(),
            });
        }
        if self.version != ENGRAM_SNAPSHOT_VERSION {
            return Err(SnapshotError {
                message: format!("Unsupported Engram backup version: {}", self.version),
            });
        }
        Ok(())
    }

    pub fn into_state(self) -> Result<AppState, SnapshotError> {
        self.validate()?;
        Ok(AppState {
            decks: self.decks,
            note_types: self.note_types,
            notes: self.notes,
            cards: self.cards,
            card_progress: self.card_progress,
            sessions: self.sessions,
            reviews: self.reviews,
            deck_options: self.deck_options,
            external_sources: self.external_sources,
            media_assets: self.media_assets,
            active_session: None,
        })
    }
}

pub fn create_engram_snapshot(state: &AppState, exported_at: u64) -> EngramSnapshot {
    EngramSnapshot::from_state(state, exported_at)
}

pub fn restore_engram_snapshot(snapshot: EngramSnapshot) -> Result<AppState, SnapshotError> {
    snapshot.into_state()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        ActiveSessionState, CardTemplate, DeckOptions, DeckOptionsPreset, ExternalSourceRecord,
        ExternalSourceTarget, FieldDef, NoteFieldValue, SessionStatus, TemplateRequirementMode,
    };
    use std::collections::BTreeMap;

    const NOW: u64 = 1_700_000_000_000;

    fn state() -> AppState {
        AppState {
            decks: vec![Deck {
                id: "deck".to_string(),
                name: "Tamil".to_string(),
                description: "Script".to_string(),
                created_at: NOW,
            }],
            note_types: vec![NoteType {
                id: "basic".to_string(),
                name: "Basic".to_string(),
                fields: vec![FieldDef {
                    id: "front".to_string(),
                    name: "Front".to_string(),
                    required: true,
                    ordinal: 0,
                }],
                templates: vec![CardTemplate {
                    id: "forward".to_string(),
                    name: "Forward".to_string(),
                    front_template: "{{Front}}".to_string(),
                    back_template: "Answer".to_string(),
                    deck_id: None,
                    required_field_names: vec!["Front".to_string()],
                    requirement_mode: TemplateRequirementMode::All,
                    ordinal: 0,
                }],
                created_at: NOW,
                updated_at: NOW,
            }],
            notes: vec![Note {
                id: "note".to_string(),
                note_type_id: "basic".to_string(),
                deck_id: "deck".to_string(),
                fields: vec![NoteFieldValue {
                    field_id: "front".to_string(),
                    value: "letter-a".to_string(),
                }],
                tags: vec!["script".to_string()],
                created_at: NOW,
                updated_at: NOW,
            }],
            cards: vec![Card {
                id: "card".to_string(),
                deck_id: "deck".to_string(),
                front: "letter-a".to_string(),
                back: "a".to_string(),
                created_at: NOW,
                lineage: None,
            }],
            card_progress: Vec::new(),
            sessions: vec![Session {
                id: "session".to_string(),
                deck_id: "deck".to_string(),
                status: SessionStatus::Active,
                started_at: NOW,
                ended_at: None,
                cards_reviewed: 0,
                cards_correct: 0,
            }],
            reviews: Vec::new(),
            deck_options: vec![DeckOptionsPreset {
                deck_id: "deck".to_string(),
                options: DeckOptions {
                    new_cards_per_day: 8,
                    ..DeckOptions::default()
                },
            }],
            external_sources: vec![ExternalSourceRecord {
                target: ExternalSourceTarget::Note,
                target_id: "note".to_string(),
                source: "anki-v11".to_string(),
                original_id: Some("1000".to_string()),
                data: BTreeMap::from([("guid".to_string(), "stable-guid".to_string())]),
            }],
            media_assets: vec![MediaAssetRecord {
                id: "media:0".to_string(),
                archive_name: "0".to_string(),
                filename: Some("audio/hola.mp3".to_string()),
                data: b"mp3".to_vec(),
            }],
            active_session: Some(ActiveSessionState {
                session_id: "session".to_string(),
                deck_id: "deck".to_string(),
                queue: Vec::new(),
                current_index: 0,
                current_card_started_at: None,
                revealed: false,
            }),
        }
    }

    #[test]
    fn snapshot_from_state_copies_only_durable_collection_data() {
        let snapshot = create_engram_snapshot(&state(), NOW + 1);

        assert_eq!(snapshot.app, ENGRAM_SNAPSHOT_APP);
        assert_eq!(snapshot.version, ENGRAM_SNAPSHOT_VERSION);
        assert_eq!(snapshot.exported_at, NOW + 1);
        assert_eq!(snapshot.decks.len(), 1);
        assert_eq!(snapshot.note_types.len(), 1);
        assert_eq!(snapshot.notes.len(), 1);
        assert_eq!(snapshot.deck_options[0].options.new_cards_per_day, 8);
        assert_eq!(snapshot.external_sources.len(), 1);
        assert_eq!(snapshot.external_sources[0].source, "anki-v11");
        assert_eq!(
            snapshot.media_assets[0].filename.as_deref(),
            Some("audio/hola.mp3")
        );
    }

    #[test]
    fn restoring_snapshot_clears_active_session() {
        let snapshot = create_engram_snapshot(&state(), NOW + 1);

        let restored = restore_engram_snapshot(snapshot).unwrap();

        assert!(restored.active_session.is_none());
        assert_eq!(restored.decks[0].id, "deck");
        assert_eq!(restored.note_types[0].id, "basic");
        assert_eq!(restored.notes[0].id, "note");
        assert_eq!(restored.deck_options[0].deck_id, "deck");
        assert_eq!(restored.external_sources[0].target_id, "note");
        assert_eq!(restored.media_assets[0].data, b"mp3");
    }

    #[test]
    fn snapshot_validation_rejects_wrong_app_or_version() {
        let mut snapshot = create_engram_snapshot(&state(), NOW + 1);
        snapshot.app = "other".to_string();
        assert_eq!(
            restore_engram_snapshot(snapshot).unwrap_err().message,
            "The selected file is not an Engram backup."
        );

        let mut snapshot = create_engram_snapshot(&state(), NOW + 1);
        snapshot.version = 99;
        assert_eq!(
            restore_engram_snapshot(snapshot).unwrap_err().message,
            "Unsupported Engram backup version: 99"
        );
    }
}
