//! JSON facade over `engram-core`.
//!
//! This crate is deliberately not the raw `extern "C"` WASM ABI. It is the
//! testable contract layer that WASM, C-ABI, Electron, HTML, Qt, XAML, and
//! SwiftUI bindings can all share.

#![forbid(unsafe_code)]

use std::panic::{catch_unwind, AssertUnwindSafe};

use engram_core::{
    build_session_queue_with_daily_limits, build_session_queue_with_options,
    create_engram_snapshot, deck_options_for_state, export_cards_anki_basic_tsv, export_cards_csv,
    export_notes_anki_tsv, generate_cards_for_note, get_active_session_progress,
    get_daily_study_limit_usage, get_deck_stats, import_anki_basic_tsv, import_anki_notes_tsv,
    import_basic_cards_csv, import_cards_csv, materialize_generated_card, reduce,
    restore_engram_snapshot, search_cards as search_core_cards, summarize_review_history,
    AnkiBasicTsvExportOptions, AnkiNoteTsvImportOptions, AppState, BasicCardCsvImportOptions, Card,
    CardFlag, CardLineage, DeckOptions, EngramSnapshot, MediaAssetRecord, Rating,
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

    pub fn state(&self) -> &AppState {
        &self.state
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
            let options = deck_options_for_state(&self.state, deck_id);
            let queue = build_session_queue_with_options(
                &self.state.cards,
                &self.state.card_progress,
                deck_id,
                now,
                &options,
            );
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
            let stats = get_deck_stats(&self.state.cards, &self.state.card_progress, deck_id, now);
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

    pub fn session_progress(&self) -> String {
        catch_json(|| {
            let progress = get_active_session_progress(&self.state);
            Ok(ok_with("progress", &progress))
        })
    }

    pub fn engram_app_props(&self, deck_id: &str, now: u64) -> String {
        catch_json(|| {
            let props = engram_app_props_for_state(&self.state, deck_id, now);
            Ok(ok_with("props", &props))
        })
    }

    pub fn handle_engram_app_event(&mut self, event: &str, deck_id: &str, now: u64) -> String {
        catch_json(|| {
            let event = parse_engram_app_event(event)?;
            match event {
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
                    self.state = reduce(
                        &self.state,
                        engram_core::EngramCommand::SuspendCard {
                            card_id,
                            suspended_at: now,
                        },
                    );
                }
                EngramAppEvent::ToggleMark => {
                    let card_id = current_active_card_id(&self.state, "mark")?;
                    let is_marked = self
                        .state
                        .card_progress
                        .iter()
                        .find(|progress| progress.card_id == card_id)
                        .and_then(|progress| progress.marked_at)
                        .is_some();
                    self.state = if is_marked {
                        reduce(
                            &self.state,
                            engram_core::EngramCommand::UnmarkCard { card_id },
                        )
                    } else {
                        reduce(
                            &self.state,
                            engram_core::EngramCommand::MarkCard {
                                card_id,
                                marked_at: now,
                            },
                        )
                    };
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
                    let session_id = active_session.session_id;
                    let card_id = card.id.clone();
                    let review_id = format!(
                        "engram-app::{}::{}::{now}::{}",
                        session_id,
                        card_id,
                        rating_label(rating)
                    );
                    self.state = reduce(
                        &self.state,
                        engram_core::EngramCommand::RateCard {
                            review_id,
                            session_id,
                            card_id,
                            rating,
                            reviewed_at: now,
                        },
                    );
                    self.state = reduce(&self.state, engram_core::EngramCommand::AdvanceSession);
                }
            }

            let props = engram_app_props_for_state(&self.state, deck_id, now);
            Ok(json!({
                "ok": true,
                "event": event.canonical_name(),
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
            let notes: Vec<_> = self
                .state
                .notes
                .iter()
                .filter(|note| note.note_type_id == note_type_id && note.deck_id == deck_id)
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
                &export_notes_anki_tsv(note_type, &notes, &options),
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
}

fn engram_app_props_for_state(state: &AppState, deck_id: &str, now: u64) -> Value {
    let selected_deck_id = selected_deck_id(state, deck_id);
    let deck = state.decks.iter().find(|deck| deck.id == selected_deck_id);
    let deck_name = deck
        .map(|deck| deck.name.clone())
        .filter(|name| !name.is_empty())
        .or_else(|| (!selected_deck_id.is_empty()).then(|| selected_deck_id.clone()))
        .unwrap_or_else(|| "Deck".to_string());
    let stats = get_deck_stats(
        &state.cards,
        &state.card_progress,
        selected_deck_id.as_str(),
        now,
    );
    let progress = get_active_session_progress(state);
    let active_card = state
        .active_session
        .as_ref()
        .and_then(|active| active.queue.get(active.current_index));
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

    json!({
        "app-title": "Engram",
        "deck-name": deck_name,
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
        "answer-visible": state.active_session.as_ref().is_some_and(|active| active.revealed),
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
    })
}

fn selected_deck_id(state: &AppState, deck_id: &str) -> String {
    if !deck_id.is_empty() {
        return deck_id.to_string();
    }
    state
        .active_session
        .as_ref()
        .map(|active| active.deck_id.clone())
        .or_else(|| state.decks.first().map(|deck| deck.id.clone()))
        .unwrap_or_default()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EngramAppEvent {
    Reveal,
    Undo,
    BuryCard,
    BurySiblings,
    SuspendCard,
    ToggleMark,
    Rate(Rating),
}

impl EngramAppEvent {
    fn canonical_name(self) -> &'static str {
        match self {
            Self::Reveal => "onReveal",
            Self::Undo => "onUndo",
            Self::BuryCard => "onBuryCard",
            Self::BurySiblings => "onBurySiblings",
            Self::SuspendCard => "onSuspendCard",
            Self::ToggleMark => "onToggleMark",
            Self::Rate(Rating::Again) => "onAgain",
            Self::Rate(Rating::Hard) => "onHard",
            Self::Rate(Rating::Good) => "onGood",
            Self::Rate(Rating::Easy) => "onEasy",
        }
    }
}

fn parse_engram_app_event(event: &str) -> Result<EngramAppEvent, String> {
    let lowered = event.trim().to_ascii_lowercase();
    match lowered.strip_prefix("on").unwrap_or(&lowered) {
        "reveal" => Ok(EngramAppEvent::Reveal),
        "undo" => Ok(EngramAppEvent::Undo),
        "burycard" | "bury-card" | "bury_card" => Ok(EngramAppEvent::BuryCard),
        "burysiblings" | "bury-siblings" | "bury_siblings" | "burynote" | "bury-note"
        | "bury_note" => Ok(EngramAppEvent::BurySiblings),
        "suspendcard" | "suspend-card" | "suspend_card" | "suspend" => {
            Ok(EngramAppEvent::SuspendCard)
        }
        "togglemark" | "toggle-mark" | "toggle_mark" | "mark" => Ok(EngramAppEvent::ToggleMark),
        "again" => Ok(EngramAppEvent::Rate(Rating::Again)),
        "hard" => Ok(EngramAppEvent::Rate(Rating::Hard)),
        "good" => Ok(EngramAppEvent::Rate(Rating::Good)),
        "easy" => Ok(EngramAppEvent::Rate(Rating::Easy)),
        _ => Err(format!("unknown Engram app event: {event}")),
    }
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
    DeleteDeck {
        deck_id: String,
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
            Self::DeleteDeck { deck_id } => engram_core::EngramCommand::DeleteDeck { deck_id },
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

        let value: Value = serde_json::from_str(&session.engram_app_props("deck", NOW)).unwrap();

        assert_eq!(value["ok"], true);
        assert_eq!(value["props"]["app-title"], "Engram");
        assert_eq!(value["props"]["deck-name"], "Tamil");
        assert_eq!(value["props"]["deck-total-value"], "2");
        assert_eq!(value["props"]["deck-new-value"], "2");
        assert_eq!(value["props"]["prompt"], "letter-a");
        assert_eq!(value["props"]["answer"], "a");
        assert_eq!(value["props"]["answer-visible"], true);
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
    fn export_anki_notes_tsv_uses_note_fields_and_tags() {
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
                    {"fieldId": "front", "value": "letter\t\"a\""},
                    {"fieldId": "back", "value": "line one\nline two"}
                ],
                "tags": ["tamil", "script"],
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
        let exported: Value = serde_json::from_str(&session.export_anki_notes_tsv(
            "basic",
            "deck",
            "Tamil::Script",
            "",
            false,
        ))
        .unwrap();

        assert_eq!(exported["ok"], true);
        let tsv = exported["tsv"].as_str().unwrap();
        assert!(tsv.starts_with(
            "#separator:tab\n#html:false\n#notetype:Basic\n#deck:Tamil::Script\n#columns:Front\tBack\tTags\n"
        ));
        assert!(tsv.contains("\"letter\t\"\"a\"\"\"\t\"line one\nline two\"\ttamil script\n"));
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
