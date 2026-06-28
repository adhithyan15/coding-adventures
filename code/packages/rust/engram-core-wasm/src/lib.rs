//! JSON facade over `engram-core`.
//!
//! This crate is deliberately not the raw `extern "C"` WASM ABI. It is the
//! testable contract layer that WASM, C-ABI, Electron, HTML, Qt, XAML, and
//! SwiftUI bindings can all share.

#![forbid(unsafe_code)]

use std::panic::{catch_unwind, AssertUnwindSafe};

use engram_core::{
    build_session_queue, build_session_queue_with_daily_limits, create_engram_snapshot,
    export_cards_anki_basic_tsv, export_cards_csv, generate_cards_for_note,
    get_active_session_progress, get_daily_study_limit_usage, get_deck_stats,
    import_basic_cards_csv, import_cards_csv, reduce, restore_engram_snapshot,
    search_cards as search_core_cards, summarize_review_history, AnkiBasicTsvExportOptions,
    AppState, BasicCardCsvImportOptions, Card, CardFlag, CardLineage, DeckOptions, EngramSnapshot,
    Rating,
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

    pub fn daily_limit_usage(
        &self,
        deck_id: &str,
        day_start: u64,
        day_end: u64,
        deck_options_json: &str,
    ) -> String {
        catch_json(|| {
            let options = parse_deck_options(deck_options_json)?;
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
            let options = parse_deck_options(deck_options_json)?;
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

    pub fn session_progress(&self) -> String {
        catch_json(|| {
            let progress = get_active_session_progress(&self.state);
            Ok(ok_with("progress", &progress))
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

    pub fn search_cards(&self, query: &str, now: u64) -> String {
        catch_json(|| match search_core_cards(&self.state, query, now) {
            Ok(results) => Ok(ok_with("results", &results)),
            Err(error) => Ok(error_json_with_token(&error.message, &error.token)),
        })
    }

    pub fn export_cards_csv(&self, deck_id: &str) -> String {
        catch_json(|| {
            let cards: Vec<Card> = self
                .state
                .cards
                .iter()
                .filter(|card| card.deck_id == deck_id)
                .cloned()
                .collect();
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
            let cards: Vec<Card> = self
                .state
                .cards
                .iter()
                .filter(|card| card.deck_id == deck_id)
                .cloned()
                .collect();
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

fn parse_deck_options(deck_options_json: &str) -> Result<DeckOptions, String> {
    if deck_options_json.trim().is_empty() {
        return Ok(DeckOptions::default());
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
    fn review_history_reports_rating_summary_for_range() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [
                {"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000},
                {"id":"other","name":"Spanish","description":"Words","createdAt":1700000000000}
            ],
            "noteTypes": [],
            "notes": [],
            "cards": [
                {"id":"card","deckId":"deck","front":"letter-a","back":"a","createdAt":1700000000000},
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
        assert_eq!(value["history"]["totalReviews"], 2);
        assert_eq!(value["history"]["correctReviews"], 1);
        assert_eq!(value["history"]["uniqueCards"], 1);
        assert_eq!(value["history"]["ratingCounts"]["again"], 1);
        assert_eq!(value["history"]["ratingCounts"]["good"], 1);
        assert_eq!(value["history"]["firstReviewedAt"], NOW + 10);
        assert_eq!(value["history"]["lastReviewedAt"], NOW + 20);
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
    fn export_and_parse_cards_csv() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [
                {"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000},
                {"id":"other-deck","name":"Spanish","description":"Words","createdAt":1700000000000}
            ],
            "noteTypes": [],
            "notes": [],
            "cards": [
                {"id":"card","deckId":"deck","front":"letter, \"a\"","back":"line one\nline two","createdAt":1700000000000},
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
        assert!(!csv.contains("other-deck"));

        let parsed: Value = serde_json::from_str(&session.parse_cards_csv(csv)).unwrap();
        assert_eq!(parsed["ok"], true);
        assert_eq!(parsed["cards"][0]["id"], "card");
        assert_eq!(parsed["cards"][0]["front"], "letter, \"a\"");

        let error: Value =
            serde_json::from_str(&session.parse_cards_csv("front,back\nx,y\n")).unwrap();
        assert_eq!(error["ok"], false);
        assert_eq!(error["row"], 1);
    }

    #[test]
    fn export_anki_basic_tsv_uses_anki_text_headers() {
        let mut session = EngramSession::new();
        let snapshot = r#"{
            "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
            "noteTypes": [],
            "notes": [],
            "cards": [
                {"id":"card","deckId":"deck","front":"letter\t\"a\"","back":"line one\nline two","createdAt":1700000000000}
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
