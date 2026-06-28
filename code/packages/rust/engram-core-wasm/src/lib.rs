//! JSON facade over `engram-core`.
//!
//! This crate is deliberately not the raw `extern "C"` WASM ABI. It is the
//! testable contract layer that WASM, C-ABI, Electron, HTML, Qt, XAML, and
//! SwiftUI bindings can all share.

#![forbid(unsafe_code)]

use std::panic::{catch_unwind, AssertUnwindSafe};

use engram_core::{
    build_session_queue, generate_cards_for_note, get_deck_stats, reduce,
    search_cards as search_core_cards, AppState, Card, CardFlag, DeckOptions, Rating,
};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Default)]
pub struct EngramSession {
    state: AppState,
}

impl EngramSession {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> String {
        ok_with("state", &self.state)
    }

    pub fn load_snapshot(&mut self, snapshot_json: &str) -> String {
        catch_json(|| {
            let state: AppState = serde_json::from_str(snapshot_json)
                .map_err(|err| format!("invalid snapshot: {err}"))?;
            self.state = state;
            Ok(ok_with("state", &self.state))
        })
    }

    pub fn dispatch(&mut self, command_json: &str) -> String {
        catch_json(|| {
            let command: FacadeCommand = serde_json::from_str(command_json)
                .map_err(|err| format!("invalid command: {err}"))?;
            let command = command.into_core_command();
            self.state = reduce(&self.state, command);
            Ok(ok_with("state", &self.state))
        })
    }

    pub fn build_queue(&self, deck_id: &str, now: u64) -> String {
        catch_json(|| {
            let queue =
                build_session_queue(&self.state.cards, &self.state.card_progress, deck_id, now);
            Ok(ok_with("queue", &queue))
        })
    }

    pub fn deck_stats(&self, deck_id: &str, now: u64) -> String {
        catch_json(|| {
            let stats = get_deck_stats(&self.state.cards, &self.state.card_progress, deck_id, now);
            Ok(ok_with(
                "stats",
                &json!({
                    "total": stats.total,
                    "newCount": stats.new_count,
                    "learningCount": stats.learning_count,
                    "masteredCount": stats.mastered_count,
                    "dueCount": stats.due_count,
                    "averageEaseFactor": stats.average_ease_factor,
                }),
            ))
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

    pub fn search_cards(&self, query: &str, now: u64) -> String {
        catch_json(|| match search_core_cards(&self.state, query, now) {
            Ok(results) => Ok(ok_with("results", &results)),
            Err(error) => Ok(error_json_with_token(&error.message, &error.token)),
        })
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
    DeleteDeck {
        deck_id: String,
    },
    CreateCard {
        id: String,
        deck_id: String,
        front: String,
        back: String,
        created_at: u64,
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
            Self::DeleteDeck { deck_id } => engram_core::EngramCommand::DeleteDeck { deck_id },
            Self::CreateCard {
                id,
                deck_id,
                front,
                back,
                created_at,
            } => engram_core::EngramCommand::CreateCard {
                id,
                deck_id,
                front,
                back,
                created_at,
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
            } => match deck_options {
                Some(deck_options) => engram_core::EngramCommand::RateCardWithOptions {
                    review_id,
                    session_id,
                    card_id,
                    rating,
                    reviewed_at,
                    deck_options,
                },
                None => engram_core::EngramCommand::RateCard {
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

fn error_json(message: &str) -> String {
    json!({ "ok": false, "error": message }).to_string()
}

fn error_json_with_token(message: &str, token: &str) -> String {
    json!({ "ok": false, "error": message, "token": token }).to_string()
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
    fn build_queue_uses_loaded_state() {
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
        let value: Value = serde_json::from_str(&session.build_queue("deck", NOW)).unwrap();

        assert_eq!(value["ok"], true);
        assert_eq!(value["queue"][0]["id"], "card");
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

        let error: Value = serde_json::from_str(&session.search_cards("kind:review", NOW)).unwrap();

        assert_eq!(error["ok"], false);
        assert_eq!(error["token"], "kind:review");
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
                        "lapseIntervalMultiplier": 0.0
                    }
                }"#,
        ))
        .unwrap();

        assert_eq!(value["ok"], true);
        assert_eq!(value["state"]["cardProgress"][0]["state"], "learning");
        assert_eq!(value["state"]["cardProgress"][0]["learningStepIndex"], 1);
        assert_eq!(
            value["state"]["cardProgress"][0]["nextDueAt"],
            NOW + 20 * 60 * 1000
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
