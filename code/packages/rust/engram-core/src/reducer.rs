use std::collections::BTreeMap;

use crate::model::{
    ActiveSessionState, AppState, Card, CardFlag, CardProgress, CardProgressSnapshot, CardState,
    Deck, DeckOptions, DeckOptionsPreset, ExternalSourceRecord, ExternalSourceTarget, LeechAction,
    LeechEvent, MediaAssetRecord, Note, NoteType, Rating, Review, Session, SessionStatus,
};
use crate::queue::is_new_progress_overlay;
use crate::scheduler::schedule_review;
use crate::sm2::{INITIAL_EASE_FACTOR, ONE_DAY_MS};
use crate::template::{
    generate_cards_for_note, materialize_generated_card, rename_note_type_field,
};

#[derive(Clone, Debug, PartialEq)]
pub enum EngramCommand {
    LoadState(AppState),
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
    RenameNoteTypeField {
        note_type_id: String,
        field_id: String,
        name: String,
        updated_at: u64,
    },
    UpsertNoteType {
        note_type: NoteType,
        materialize_cards_at: Option<u64>,
    },
    DeleteNoteType {
        note_type_id: String,
    },
    UpsertNote {
        note: Note,
        materialize_cards_at: Option<u64>,
    },
    DeleteNote {
        note_id: String,
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
        lineage: Option<crate::model::CardLineage>,
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
    },
    RateCardAndBurySiblings {
        review_id: String,
        session_id: String,
        card_id: String,
        rating: Rating,
        reviewed_at: u64,
        buried_until: u64,
    },
    RateCardWithOptions {
        review_id: String,
        session_id: String,
        card_id: String,
        rating: Rating,
        reviewed_at: u64,
        deck_options: DeckOptions,
    },
    RateCardWithOptionsAndBurySiblings {
        review_id: String,
        session_id: String,
        card_id: String,
        rating: Rating,
        reviewed_at: u64,
        deck_options: DeckOptions,
        buried_until: u64,
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

pub fn reduce(state: &AppState, command: EngramCommand) -> AppState {
    match command {
        EngramCommand::LoadState(next) => AppState {
            active_session: None,
            ..next
        },
        EngramCommand::CreateDeck {
            id,
            name,
            description,
            created_at,
        } => {
            let mut next = state.clone();
            next.decks.push(Deck {
                id,
                name,
                description,
                created_at,
            });
            next
        }
        EngramCommand::UpdateDeck {
            deck_id,
            name,
            description,
        } => {
            let mut next = state.clone();
            for deck in &mut next.decks {
                if deck.id == deck_id {
                    deck.name = name;
                    deck.description = description;
                    break;
                }
            }
            next
        }
        EngramCommand::SetDeckOptions { deck_id, options } => {
            let mut next = state.clone();
            match next
                .deck_options
                .iter_mut()
                .find(|preset| preset.deck_id == deck_id)
            {
                Some(preset) => preset.options = options,
                None => next
                    .deck_options
                    .push(DeckOptionsPreset { deck_id, options }),
            }
            next
        }
        EngramCommand::DeleteDeck { deck_id } => {
            let card_ids: Vec<String> = state
                .cards
                .iter()
                .filter(|card| card.deck_id == deck_id)
                .map(|card| card.id.clone())
                .collect();
            let note_ids: Vec<String> = state
                .notes
                .iter()
                .filter(|note| note.deck_id == deck_id)
                .map(|note| note.id.clone())
                .collect();
            let session_ids: Vec<String> = state
                .sessions
                .iter()
                .filter(|session| session.deck_id == deck_id)
                .map(|session| session.id.clone())
                .collect();
            let mut external_sources = without_external_source_target(
                &state.external_sources,
                ExternalSourceTarget::Deck,
                &deck_id,
            );
            for card_id in &card_ids {
                external_sources = without_external_source_target(
                    &external_sources,
                    ExternalSourceTarget::Card,
                    card_id,
                );
            }
            for note_id in &note_ids {
                external_sources = without_external_source_target(
                    &external_sources,
                    ExternalSourceTarget::Note,
                    note_id,
                );
            }
            for session_id in &session_ids {
                external_sources = without_external_source_target(
                    &external_sources,
                    ExternalSourceTarget::Session,
                    session_id,
                );
            }

            AppState {
                decks: state
                    .decks
                    .iter()
                    .filter(|deck| deck.id != deck_id)
                    .cloned()
                    .collect(),
                note_types: state.note_types.clone(),
                notes: state
                    .notes
                    .iter()
                    .filter(|note| note.deck_id != deck_id)
                    .cloned()
                    .collect(),
                cards: state
                    .cards
                    .iter()
                    .filter(|card| card.deck_id != deck_id)
                    .cloned()
                    .collect(),
                card_progress: state
                    .card_progress
                    .iter()
                    .filter(|progress| !card_ids.contains(&progress.card_id))
                    .cloned()
                    .collect(),
                sessions: state
                    .sessions
                    .iter()
                    .filter(|session| session.deck_id != deck_id)
                    .cloned()
                    .collect(),
                reviews: state
                    .reviews
                    .iter()
                    .filter(|review| !session_ids.contains(&review.session_id))
                    .cloned()
                    .collect(),
                deck_options: state
                    .deck_options
                    .iter()
                    .filter(|preset| preset.deck_id != deck_id)
                    .cloned()
                    .collect(),
                external_sources,
                media_assets: state.media_assets.clone(),
                active_session: state
                    .active_session
                    .as_ref()
                    .filter(|session| session.deck_id != deck_id)
                    .cloned(),
            }
        }
        EngramCommand::RenameNoteTypeField {
            note_type_id,
            field_id,
            name,
            updated_at,
        } => {
            let mut next = state.clone();
            for note_type in &mut next.note_types {
                if note_type.id == note_type_id {
                    *note_type = rename_note_type_field(note_type, &field_id, &name, updated_at);
                    break;
                }
            }
            next
        }
        EngramCommand::UpsertNoteType {
            note_type,
            materialize_cards_at,
        } => {
            let note_type_id = note_type.id.clone();
            let mut next = state.clone();
            match next
                .note_types
                .iter_mut()
                .find(|existing| existing.id == note_type_id)
            {
                Some(existing) => *existing = note_type,
                None => next.note_types.push(note_type),
            }
            if let Some(created_at) = materialize_cards_at {
                let note_ids = next
                    .notes
                    .iter()
                    .filter(|note| note.note_type_id == note_type_id)
                    .map(|note| note.id.clone())
                    .collect::<Vec<_>>();
                for note_id in note_ids {
                    next = sync_generated_cards_for_note(&next, &note_id, created_at);
                }
            }
            next
        }
        EngramCommand::DeleteNoteType { note_type_id } => {
            delete_note_type_and_related_notes(state, &note_type_id)
        }
        EngramCommand::UpsertNote {
            note,
            materialize_cards_at,
        } => {
            let mut next = state.clone();
            match next
                .notes
                .iter_mut()
                .find(|existing| existing.id == note.id)
            {
                Some(existing) => *existing = note.clone(),
                None => next.notes.push(note.clone()),
            }
            if let Some(created_at) = materialize_cards_at {
                next = sync_generated_cards_for_note(&next, &note.id, created_at);
            }
            next
        }
        EngramCommand::DeleteNote { note_id } => delete_note_and_generated_cards(state, &note_id),
        EngramCommand::UpsertMediaAsset { asset } => {
            let mut next = state.clone();
            match next
                .media_assets
                .iter_mut()
                .find(|existing| existing.id == asset.id)
            {
                Some(existing) => *existing = asset,
                None => next.media_assets.push(asset),
            }
            next
        }
        EngramCommand::DeleteMediaAsset { asset_id } => AppState {
            media_assets: state
                .media_assets
                .iter()
                .filter(|asset| asset.id != asset_id)
                .cloned()
                .collect(),
            ..state.clone()
        },
        EngramCommand::DeleteMediaAssets { asset_ids } => AppState {
            media_assets: state
                .media_assets
                .iter()
                .filter(|asset| !asset_ids.contains(&asset.id))
                .cloned()
                .collect(),
            ..state.clone()
        },
        EngramCommand::CreateCard {
            id,
            deck_id,
            front,
            back,
            created_at,
            lineage,
        } => {
            let mut next = state.clone();
            next.cards.push(Card {
                id,
                deck_id,
                front,
                back,
                created_at,
                lineage,
            });
            next
        }
        EngramCommand::UpdateCard {
            card_id,
            front,
            back,
        } => {
            let mut next = state.clone();
            for card in &mut next.cards {
                if card.id == card_id {
                    card.front = front;
                    card.back = back;
                    break;
                }
            }
            next
        }
        EngramCommand::DeleteCard { card_id } => AppState {
            cards: state
                .cards
                .iter()
                .filter(|card| card.id != card_id)
                .cloned()
                .collect(),
            card_progress: state
                .card_progress
                .iter()
                .filter(|progress| progress.card_id != card_id)
                .cloned()
                .collect(),
            external_sources: without_external_source_target(
                &state.external_sources,
                ExternalSourceTarget::Card,
                &card_id,
            ),
            active_session: remove_card_from_active_session(state.active_session.clone(), &card_id),
            ..state.clone()
        },
        EngramCommand::SuspendCard {
            card_id,
            suspended_at,
        } => {
            let mut next = state.clone();
            let progress =
                ensure_progress_overlay(&mut next.card_progress, card_id.clone(), suspended_at);
            progress.suspended_at = Some(suspended_at);
            next.active_session = remove_card_from_active_session(next.active_session, &card_id);
            next
        }
        EngramCommand::UnsuspendCard { card_id } => {
            let mut next = state.clone();
            if let Some(index) = next
                .card_progress
                .iter()
                .position(|progress| progress.card_id == card_id)
            {
                let progress = &mut next.card_progress[index];
                progress.suspended_at = None;
                if progress.state == crate::model::CardState::Suspended {
                    progress.state = crate::model::CardState::Review;
                }
                remove_clear_overlay(&mut next.card_progress, index);
            }
            next
        }
        EngramCommand::BuryCard {
            card_id,
            buried_at,
            buried_until,
        } => {
            let mut next = state.clone();
            let progress =
                ensure_progress_overlay(&mut next.card_progress, card_id.clone(), buried_at);
            progress.buried_until = Some(buried_until);
            next.active_session = remove_card_from_active_session(next.active_session, &card_id);
            next
        }
        EngramCommand::BuryCardSiblings {
            card_id,
            buried_at,
            buried_until,
        } => bury_card_siblings(state, &card_id, buried_at, buried_until),
        EngramCommand::UnburyCard { card_id } => {
            let mut next = state.clone();
            if let Some(index) = next
                .card_progress
                .iter()
                .position(|progress| progress.card_id == card_id)
            {
                let progress = &mut next.card_progress[index];
                progress.buried_until = None;
                if progress.state == crate::model::CardState::Buried {
                    progress.state = crate::model::CardState::Review;
                }
                remove_clear_overlay(&mut next.card_progress, index);
            }
            next
        }
        EngramCommand::SetCardFlag {
            card_id,
            flag,
            flagged_at,
        } => {
            let mut next = state.clone();
            match flag {
                Some(flag) => {
                    let progress =
                        ensure_progress_overlay(&mut next.card_progress, card_id, flagged_at);
                    progress.flag = Some(flag);
                }
                None => {
                    if let Some(index) = next
                        .card_progress
                        .iter()
                        .position(|progress| progress.card_id == card_id)
                    {
                        next.card_progress[index].flag = None;
                        remove_clear_overlay(&mut next.card_progress, index);
                    }
                }
            }
            next
        }
        EngramCommand::MarkCard { card_id, marked_at } => {
            let mut next = state.clone();
            let progress = ensure_progress_overlay(&mut next.card_progress, card_id, marked_at);
            progress.marked_at = Some(marked_at);
            next
        }
        EngramCommand::UnmarkCard { card_id } => {
            let mut next = state.clone();
            if let Some(index) = next
                .card_progress
                .iter()
                .position(|progress| progress.card_id == card_id)
            {
                next.card_progress[index].marked_at = None;
                remove_clear_overlay(&mut next.card_progress, index);
            }
            next
        }
        EngramCommand::StartSession {
            session_id,
            deck_id,
            queue,
            started_at,
        } => {
            let mut next = state.clone();
            next.sessions.push(Session {
                id: session_id.clone(),
                deck_id: deck_id.clone(),
                status: SessionStatus::Active,
                started_at,
                ended_at: None,
                cards_reviewed: 0,
                cards_correct: 0,
            });
            next.active_session = Some(ActiveSessionState {
                session_id,
                deck_id,
                queue,
                current_index: 0,
                current_card_started_at: Some(started_at),
                revealed: false,
            });
            next
        }
        EngramCommand::RevealCurrentCard => {
            let mut next = state.clone();
            if let Some(active_session) = &mut next.active_session {
                active_session.revealed = true;
            }
            next
        }
        EngramCommand::RateCard {
            review_id,
            session_id,
            card_id,
            rating,
            reviewed_at,
        } => {
            let deck_options = deck_options_for_card(state, &card_id);
            let sibling_bury = SiblingBuryRule::from_deck_options(
                state,
                &card_id,
                &deck_options,
                reviewed_at.saturating_add(ONE_DAY_MS),
                reviewed_at,
            );
            reduce_rate_card(
                state,
                review_id,
                session_id,
                card_id,
                rating,
                reviewed_at,
                &deck_options,
                sibling_bury,
            )
        }
        EngramCommand::RateCardAndBurySiblings {
            review_id,
            session_id,
            card_id,
            rating,
            reviewed_at,
            buried_until,
        } => {
            let deck_options = deck_options_for_card(state, &card_id);
            reduce_rate_card(
                state,
                review_id,
                session_id,
                card_id,
                rating,
                reviewed_at,
                &deck_options,
                SiblingBuryRule::All(buried_until),
            )
        }
        EngramCommand::RateCardWithOptions {
            review_id,
            session_id,
            card_id,
            rating,
            reviewed_at,
            deck_options,
        } => {
            let sibling_bury = SiblingBuryRule::from_deck_options(
                state,
                &card_id,
                &deck_options,
                reviewed_at.saturating_add(ONE_DAY_MS),
                reviewed_at,
            );
            reduce_rate_card(
                state,
                review_id,
                session_id,
                card_id,
                rating,
                reviewed_at,
                &deck_options,
                sibling_bury,
            )
        }
        EngramCommand::RateCardWithOptionsAndBurySiblings {
            review_id,
            session_id,
            card_id,
            rating,
            reviewed_at,
            deck_options,
            buried_until,
        } => reduce_rate_card(
            state,
            review_id,
            session_id,
            card_id,
            rating,
            reviewed_at,
            &deck_options,
            SiblingBuryRule::All(buried_until),
        ),
        EngramCommand::UndoLastReview { session_id } => undo_last_review(state, &session_id),
        EngramCommand::AdvanceSession => {
            let mut next = state.clone();
            if let Some(active_session) = &mut next.active_session {
                active_session.current_index += 1;
                active_session.revealed = false;
            }
            next
        }
        EngramCommand::CompleteSession {
            session_id,
            ended_at,
        } => {
            let mut next = state.clone();
            for session in &mut next.sessions {
                if session.id == session_id {
                    session.status = SessionStatus::Completed;
                    session.ended_at = Some(ended_at);
                    break;
                }
            }
            next.active_session = None;
            next
        }
    }
}

fn without_external_source_target(
    sources: &[ExternalSourceRecord],
    target: ExternalSourceTarget,
    target_id: &str,
) -> Vec<ExternalSourceRecord> {
    let mut next = Vec::with_capacity(sources.len());
    let mut tombstones = Vec::new();
    for source in sources {
        if source.target == target && source.target_id == target_id {
            if let Some(tombstone) = deleted_external_source_record(source) {
                tombstones.push(tombstone);
            }
        } else {
            next.push(source.clone());
        }
    }
    next.extend(tombstones);
    next
}

fn deleted_external_source_record(source: &ExternalSourceRecord) -> Option<ExternalSourceRecord> {
    let deleted_target = match source.target {
        ExternalSourceTarget::Deck => "deck",
        ExternalSourceTarget::Note => "note",
        ExternalSourceTarget::Card => "card",
        _ => return None,
    };
    let original_id = source.original_id.clone()?;
    let mut data = BTreeMap::new();
    data.insert("deletedTarget".to_string(), deleted_target.to_string());
    if let Some(update_sequence_number) = source.data.get("updateSequenceNumber") {
        data.insert(
            "updateSequenceNumber".to_string(),
            update_sequence_number.clone(),
        );
    }

    Some(ExternalSourceRecord {
        target: ExternalSourceTarget::Deleted,
        target_id: format!(
            "deleted:{}:{}:{}",
            source.source, deleted_target, original_id
        ),
        source: source.source.clone(),
        original_id: Some(original_id),
        data,
    })
}

fn sync_generated_cards_for_note(state: &AppState, note_id: &str, created_at: u64) -> AppState {
    let Some(note) = state.notes.iter().find(|note| note.id == note_id) else {
        return state.clone();
    };
    let Some(note_type) = state
        .note_types
        .iter()
        .find(|note_type| note_type.id == note.note_type_id)
    else {
        return state.clone();
    };

    let existing_generated = state
        .cards
        .iter()
        .filter(|card| generated_card_belongs_to_note(card, note_id))
        .cloned()
        .collect::<Vec<_>>();
    let mut generated_cards = generate_cards_for_note(note_type, note)
        .iter()
        .map(|generated| {
            let mut card = materialize_generated_card(generated, created_at);
            if let Some(existing) = existing_generated
                .iter()
                .find(|existing| existing.id == card.id)
            {
                card.created_at = existing.created_at;
            }
            card
        })
        .collect::<Vec<_>>();
    let generated_ids = generated_cards
        .iter()
        .map(|card| card.id.clone())
        .collect::<Vec<_>>();
    let removed_card_ids = existing_generated
        .iter()
        .filter(|card| !generated_ids.contains(&card.id))
        .map(|card| card.id.clone())
        .collect::<Vec<_>>();

    let mut next = state.clone();
    next.cards = state
        .cards
        .iter()
        .filter(|card| !generated_card_belongs_to_note(card, note_id))
        .cloned()
        .collect();
    next.cards.append(&mut generated_cards);
    prune_removed_cards(&mut next, &removed_card_ids);
    next
}

fn delete_note_and_generated_cards(state: &AppState, note_id: &str) -> AppState {
    let removed_card_ids = state
        .cards
        .iter()
        .filter(|card| generated_card_belongs_to_note(card, note_id))
        .map(|card| card.id.clone())
        .collect::<Vec<_>>();

    let mut next = state.clone();
    next.notes = state
        .notes
        .iter()
        .filter(|note| note.id != note_id)
        .cloned()
        .collect();
    next.cards = state
        .cards
        .iter()
        .filter(|card| !removed_card_ids.contains(&card.id))
        .cloned()
        .collect();
    next.external_sources =
        without_external_source_target(&next.external_sources, ExternalSourceTarget::Note, note_id);
    prune_removed_cards(&mut next, &removed_card_ids);
    next
}

fn delete_note_type_and_related_notes(state: &AppState, note_type_id: &str) -> AppState {
    let note_ids = state
        .notes
        .iter()
        .filter(|note| note.note_type_id == note_type_id)
        .map(|note| note.id.clone())
        .collect::<Vec<_>>();

    let mut next = state.clone();
    next.note_types = state
        .note_types
        .iter()
        .filter(|note_type| note_type.id != note_type_id)
        .cloned()
        .collect();
    next.external_sources = without_external_source_target(
        &next.external_sources,
        ExternalSourceTarget::NoteType,
        note_type_id,
    );
    for note_id in note_ids {
        next = delete_note_and_generated_cards(&next, &note_id);
    }
    next
}

fn generated_card_belongs_to_note(card: &Card, note_id: &str) -> bool {
    card.lineage
        .as_ref()
        .is_some_and(|lineage| lineage.note_id == note_id)
}

fn prune_removed_cards(state: &mut AppState, removed_card_ids: &[String]) {
    if removed_card_ids.is_empty() {
        return;
    }

    state
        .card_progress
        .retain(|progress| !removed_card_ids.contains(&progress.card_id));
    let mut external_sources = state.external_sources.clone();
    for card_id in removed_card_ids {
        external_sources =
            without_external_source_target(&external_sources, ExternalSourceTarget::Card, card_id);
    }
    state.external_sources = external_sources;
    for card_id in removed_card_ids {
        state.active_session =
            remove_card_from_active_session(state.active_session.clone(), card_id);
    }
}

fn deck_options_for_card(state: &AppState, card_id: &str) -> DeckOptions {
    state
        .cards
        .iter()
        .find(|card| card.id == card_id)
        .and_then(|card| {
            state
                .deck_options
                .iter()
                .find(|preset| preset.deck_id == card.deck_id)
        })
        .map(|preset| preset.options.clone())
        .unwrap_or_default()
}

fn bury_card_siblings(
    state: &AppState,
    card_id: &str,
    buried_at: u64,
    buried_until: u64,
) -> AppState {
    bury_card_siblings_with_snapshots(state, card_id, buried_at, buried_until).0
}

fn bury_card_siblings_with_snapshots(
    state: &AppState,
    card_id: &str,
    buried_at: u64,
    buried_until: u64,
) -> (AppState, Vec<CardProgressSnapshot>) {
    bury_card_siblings_matching_with_snapshots(state, card_id, buried_at, buried_until, |_| true)
}

fn bury_card_siblings_matching_with_snapshots(
    state: &AppState,
    card_id: &str,
    buried_at: u64,
    buried_until: u64,
    should_bury: impl Fn(&Card) -> bool,
) -> (AppState, Vec<CardProgressSnapshot>) {
    let Some(note_id) = state
        .cards
        .iter()
        .find(|card| card.id == card_id)
        .and_then(|card| card.lineage.as_ref())
        .map(|lineage| lineage.note_id.clone())
    else {
        return (state.clone(), Vec::new());
    };

    let sibling_ids: Vec<String> = state
        .cards
        .iter()
        .filter(|card| card.id != card_id)
        .filter(|card| {
            card.lineage
                .as_ref()
                .is_some_and(|lineage| lineage.note_id == note_id)
        })
        .filter(|card| should_bury(card))
        .map(|card| card.id.clone())
        .collect();

    if sibling_ids.is_empty() {
        return (state.clone(), Vec::new());
    }

    let mut next = state.clone();
    let mut snapshots = Vec::new();
    for sibling_id in &sibling_ids {
        let previous_progress = next
            .card_progress
            .iter()
            .find(|progress| progress.card_id == *sibling_id)
            .cloned();
        let progress =
            ensure_progress_overlay(&mut next.card_progress, sibling_id.clone(), buried_at);
        progress.buried_until = Some(buried_until);
        snapshots.push(CardProgressSnapshot {
            card_id: sibling_id.clone(),
            previous_progress,
            resulting_progress: Some(progress.clone()),
        });
    }
    for sibling_id in sibling_ids {
        next.active_session = remove_card_from_active_session(next.active_session, &sibling_id);
    }
    (next, snapshots)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum BuryCardKind {
    IntradayLearning,
    InterdayLearning,
    Review,
    New,
}

#[derive(Clone, Copy, Debug)]
enum SiblingBuryRule<'a> {
    None,
    All(u64),
    DeckOptions {
        options: &'a DeckOptions,
        until: u64,
        reviewed_kind: BuryCardKind,
    },
}

impl<'a> SiblingBuryRule<'a> {
    fn from_deck_options(
        state: &AppState,
        reviewed_card_id: &str,
        options: &'a DeckOptions,
        until: u64,
        reviewed_at: u64,
    ) -> Self {
        if options.bury_new_siblings
            || options.bury_review_siblings
            || options.bury_interday_learning_siblings
        {
            let Some(reviewed_kind) = card_bury_kind(state, reviewed_card_id, reviewed_at) else {
                return Self::None;
            };
            Self::DeckOptions {
                options,
                until,
                reviewed_kind,
            }
        } else {
            Self::None
        }
    }

    fn captures_previous_session(self) -> bool {
        !matches!(self, Self::None)
    }
}

fn should_bury_sibling_for_deck_options(
    state: &AppState,
    sibling_id: &str,
    reviewed_at: u64,
    options: &DeckOptions,
    reviewed_kind: BuryCardKind,
) -> bool {
    let Some(sibling_kind) = card_bury_kind(state, sibling_id, reviewed_at) else {
        return false;
    };

    if sibling_kind < reviewed_kind {
        return false;
    }

    match sibling_kind {
        BuryCardKind::New => options.bury_new_siblings,
        BuryCardKind::Review => options.bury_review_siblings,
        BuryCardKind::InterdayLearning => options.bury_interday_learning_siblings,
        BuryCardKind::IntradayLearning => false,
    }
}

fn card_bury_kind(state: &AppState, card_id: &str, now: u64) -> Option<BuryCardKind> {
    let Some(progress) = state
        .card_progress
        .iter()
        .find(|progress| progress.card_id == card_id)
    else {
        return Some(BuryCardKind::New);
    };

    if progress.suspended_at.is_some()
        || progress.state == CardState::Suspended
        || progress
            .buried_until
            .is_some_and(|buried_until| buried_until > now)
        || (progress.state == CardState::Buried
            && !progress
                .buried_until
                .is_some_and(|buried_until| buried_until <= now))
    {
        return None;
    }

    if is_new_progress_overlay(progress) {
        return Some(BuryCardKind::New);
    }

    match progress.state {
        CardState::Learning | CardState::Relearning => {
            if progress.next_due_at >= now.saturating_add(ONE_DAY_MS) {
                Some(BuryCardKind::InterdayLearning)
            } else {
                Some(BuryCardKind::IntradayLearning)
            }
        }
        CardState::Review | CardState::Buried => Some(BuryCardKind::Review),
        CardState::Suspended => None,
    }
}

fn reduce_rate_card(
    state: &AppState,
    review_id: String,
    session_id: String,
    card_id: String,
    rating: Rating,
    reviewed_at: u64,
    deck_options: &DeckOptions,
    sibling_bury: SiblingBuryRule<'_>,
) -> AppState {
    if state.active_session.is_none() {
        return state.clone();
    }

    let reschedules_card = review_reschedules_card(state, &session_id, &card_id);
    let existing = state
        .card_progress
        .iter()
        .find(|progress| progress.card_id == card_id)
        .cloned();
    let mut next = state.clone();
    let reviewed_card_id = card_id.clone();
    let (leech_event, resulting_progress) = if reschedules_card {
        let mut new_progress = schedule_review(
            existing.as_ref(),
            card_id.clone(),
            rating,
            deck_options,
            reviewed_at,
        );
        let leech_event = apply_leech_handling(
            &mut next,
            &reviewed_card_id,
            existing.as_ref(),
            &mut new_progress,
            rating,
            deck_options,
            reviewed_at,
        );
        upsert_progress(&mut next.card_progress, new_progress.clone());
        (leech_event, Some(new_progress))
    } else {
        (None, existing.clone())
    };
    let answer_time_ms = answer_time_ms_for_review(
        state.active_session.as_ref(),
        &session_id,
        &reviewed_card_id,
        reviewed_at,
    );
    next.reviews.push(Review {
        id: review_id,
        session_id: session_id.clone(),
        card_id,
        rating,
        reviewed_at,
        answer_time_ms,
        leech_event,
        previous_progress: existing,
        resulting_progress,
        previous_active_session: if sibling_bury.captures_previous_session() {
            state.active_session.clone()
        } else {
            None
        },
        sibling_progress_snapshots: Vec::new(),
    });
    for session in &mut next.sessions {
        if session.id == session_id {
            session.cards_reviewed += 1;
            if rating != Rating::Again {
                session.cards_correct += 1;
            }
            break;
        }
    }
    if reschedules_card {
        match sibling_bury {
            SiblingBuryRule::None => {}
            SiblingBuryRule::All(buried_until) => {
                let (mut buried, snapshots) = bury_card_siblings_with_snapshots(
                    &next,
                    &reviewed_card_id,
                    reviewed_at,
                    buried_until,
                );
                if let Some(review) = buried.reviews.last_mut() {
                    review.sibling_progress_snapshots = snapshots;
                }
                next = buried;
            }
            SiblingBuryRule::DeckOptions {
                options,
                until,
                reviewed_kind,
            } => {
                let (mut buried, snapshots) = bury_card_siblings_matching_with_snapshots(
                    &next,
                    &reviewed_card_id,
                    reviewed_at,
                    until,
                    |card| {
                        should_bury_sibling_for_deck_options(
                            state,
                            &card.id,
                            reviewed_at,
                            options,
                            reviewed_kind,
                        )
                    },
                );
                if let Some(review) = buried.reviews.last_mut() {
                    review.sibling_progress_snapshots = snapshots;
                }
                next = buried;
            }
        }
    }
    if let Some(active_session) = &mut next.active_session {
        if active_session.session_id == session_id {
            active_session.current_card_started_at = Some(reviewed_at);
        }
    }
    next
}

fn review_reschedules_card(state: &AppState, session_id: &str, card_id: &str) -> bool {
    let review_deck_id = state
        .active_session
        .as_ref()
        .filter(|active| active.session_id == session_id)
        .map(|active| active.deck_id.as_str())
        .or_else(|| {
            state
                .sessions
                .iter()
                .find(|session| session.id == session_id)
                .map(|session| session.deck_id.as_str())
        })
        .or_else(|| {
            state
                .cards
                .iter()
                .find(|card| card.id == card_id)
                .map(|card| card.deck_id.as_str())
        });
    let Some(review_deck_id) = review_deck_id else {
        return true;
    };

    !state
        .external_sources
        .iter()
        .filter(|source| {
            source.target == ExternalSourceTarget::Deck && source.target_id == review_deck_id
        })
        .any(|source| is_non_rescheduling_filtered_deck_source(source))
}

fn is_non_rescheduling_filtered_deck_source(source: &ExternalSourceRecord) -> bool {
    source_i64_from_data(source, "dyn").is_some_and(|dyn_value| dyn_value != 0)
        && source_bool_from_data(source, "resched") == Some(false)
}

fn source_i64_from_data(source: &ExternalSourceRecord, key: &str) -> Option<i64> {
    source
        .data
        .get(key)
        .and_then(|value| value.trim().parse::<i64>().ok())
}

fn source_bool_from_data(source: &ExternalSourceRecord, key: &str) -> Option<bool> {
    let value = source.data.get(key)?.trim();
    if value.eq_ignore_ascii_case("true") || value == "1" {
        Some(true)
    } else if value.eq_ignore_ascii_case("false") || value == "0" {
        Some(false)
    } else {
        None
    }
}

fn apply_leech_handling(
    state: &mut AppState,
    card_id: &str,
    existing: Option<&CardProgress>,
    new_progress: &mut CardProgress,
    rating: Rating,
    deck_options: &DeckOptions,
    reviewed_at: u64,
) -> Option<LeechEvent> {
    let existing = existing?;
    if rating != Rating::Again || existing.state != CardState::Review {
        return None;
    }
    if !leech_threshold_met(new_progress.times_incorrect, deck_options.leech_threshold) {
        return None;
    }

    let note_id = state
        .cards
        .iter()
        .find(|card| card.id == card_id)
        .and_then(|card| card.lineage.as_ref())
        .map(|lineage| lineage.note_id.clone());
    let previous_note_tags = note_id.as_ref().and_then(|note_id| {
        let note = state.notes.iter_mut().find(|note| note.id == *note_id)?;
        let previous = note.tags.clone();
        if !note
            .tags
            .iter()
            .any(|tag| tag.eq_ignore_ascii_case("leech"))
        {
            note.tags.push("leech".to_string());
        }
        Some(previous)
    });

    if deck_options.leech_action == LeechAction::Suspend {
        new_progress.state = CardState::Suspended;
        new_progress.learning_step_index = None;
        new_progress.buried_until = None;
        new_progress.suspended_at = Some(reviewed_at);
    }

    Some(LeechEvent {
        action: deck_options.leech_action,
        note_id,
        previous_note_tags,
    })
}

fn leech_threshold_met(lapses: u32, threshold: u32) -> bool {
    if threshold == 0 || lapses < threshold {
        return false;
    }
    let half_threshold = ((threshold as f64) / 2.0).ceil().max(1.0) as u32;
    (lapses - threshold) % half_threshold == 0
}

fn answer_time_ms_for_review(
    active_session: Option<&ActiveSessionState>,
    session_id: &str,
    card_id: &str,
    reviewed_at: u64,
) -> Option<u32> {
    let active_session = active_session?;
    if active_session.session_id != session_id {
        return None;
    }
    let active_card = active_session.queue.get(active_session.current_index)?;
    if active_card.id != card_id {
        return None;
    }
    let elapsed = reviewed_at.saturating_sub(active_session.current_card_started_at?);
    if elapsed == 0 {
        return None;
    }
    Some(u32::try_from(elapsed).unwrap_or(u32::MAX))
}

fn undo_last_review(state: &AppState, session_id: &str) -> AppState {
    let Some(review_index) = state
        .reviews
        .iter()
        .rposition(|review| review.session_id == session_id)
    else {
        return state.clone();
    };

    let review = state.reviews[review_index].clone();
    if review.resulting_progress.is_none() {
        return state.clone();
    }

    let mut next = state.clone();
    next.reviews.remove(review_index);

    match review.previous_progress.clone() {
        Some(previous_progress) => upsert_progress(&mut next.card_progress, previous_progress),
        None => next
            .card_progress
            .retain(|progress| progress.card_id != review.card_id),
    }
    for snapshot in &review.sibling_progress_snapshots {
        match snapshot.previous_progress.clone() {
            Some(previous_progress) => upsert_progress(&mut next.card_progress, previous_progress),
            None => next
                .card_progress
                .retain(|progress| progress.card_id != snapshot.card_id),
        }
    }
    if let Some(leech_event) = review.leech_event.clone() {
        if let (Some(note_id), Some(previous_note_tags)) =
            (leech_event.note_id, leech_event.previous_note_tags)
        {
            if let Some(note) = next.notes.iter_mut().find(|note| note.id == note_id) {
                note.tags = previous_note_tags;
            }
        }
    }

    for session in &mut next.sessions {
        if session.id == session_id {
            session.cards_reviewed = session.cards_reviewed.saturating_sub(1);
            if review.rating != Rating::Again {
                session.cards_correct = session.cards_correct.saturating_sub(1);
            }
            break;
        }
    }

    if let Some(previous_active_session) = review.previous_active_session {
        next.active_session = Some(previous_active_session);
    } else {
        restore_active_session_to_reviewed_card(&mut next, session_id, &review.card_id);
    }
    next
}

fn restore_active_session_to_reviewed_card(state: &mut AppState, session_id: &str, card_id: &str) {
    let Some(active_session) = &mut state.active_session else {
        return;
    };
    if active_session.session_id != session_id {
        return;
    }

    if let Some(index) = active_session
        .queue
        .iter()
        .position(|card| card.id == card_id)
    {
        active_session.current_index = index;
        active_session.revealed = true;
        return;
    }

    if let Some(card) = state.cards.iter().find(|card| card.id == card_id).cloned() {
        let insert_at = active_session
            .current_index
            .saturating_sub(1)
            .min(active_session.queue.len());
        active_session.queue.insert(insert_at, card);
        active_session.current_index = insert_at;
        active_session.revealed = true;
    }
}

fn ensure_progress_overlay(
    progress: &mut Vec<CardProgress>,
    card_id: String,
    created_at: u64,
) -> &mut CardProgress {
    if let Some(index) = progress
        .iter()
        .position(|progress| progress.card_id == card_id)
    {
        return &mut progress[index];
    }

    progress.push(CardProgress {
        card_id,
        state: crate::model::CardState::Review,
        interval: 0,
        ease_factor: INITIAL_EASE_FACTOR,
        next_due_at: created_at,
        learning_step_index: None,
        buried_until: None,
        suspended_at: None,
        times_seen: 0,
        times_correct: 0,
        times_incorrect: 0,
        last_seen_at: created_at,
        flag: None,
        marked_at: None,
    });
    progress.last_mut().expect("just pushed overlay progress")
}

fn remove_clear_overlay(progress: &mut Vec<CardProgress>, index: usize) {
    let should_remove = {
        let item = &progress[index];
        item.times_seen == 0
            && item.times_correct == 0
            && item.times_incorrect == 0
            && item.interval == 0
            && item.learning_step_index.is_none()
            && item.buried_until.is_none()
            && item.suspended_at.is_none()
            && item.flag.is_none()
            && item.marked_at.is_none()
    };

    if should_remove {
        progress.remove(index);
    }
}

fn remove_card_from_active_session(
    active_session: Option<ActiveSessionState>,
    card_id: &str,
) -> Option<ActiveSessionState> {
    active_session.map(|mut session| {
        let old_index = session.current_index;
        if let Some(removed_index) = session.queue.iter().position(|card| card.id == card_id) {
            session.queue.remove(removed_index);
            if session.queue.is_empty() {
                session.current_index = 0;
            } else if old_index > removed_index {
                session.current_index = old_index - 1;
            } else if old_index >= session.queue.len() {
                session.current_index = session.queue.len() - 1;
            }

            if removed_index <= old_index {
                session.revealed = false;
            }
        }
        session
    })
}

fn upsert_progress(progress: &mut Vec<CardProgress>, new_progress: CardProgress) {
    match progress
        .iter_mut()
        .find(|existing| existing.card_id == new_progress.card_id)
    {
        Some(existing) => *existing = new_progress,
        None => progress.push(new_progress),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        CardLineage, CardState, CardTemplate, FieldDef, LeechAction, Note, NoteFieldValue,
        NoteType, TemplateRequirementMode,
    };
    use crate::queue::build_session_queue;
    use crate::scheduler::ONE_MINUTE_MS;
    use crate::sm2::ONE_DAY_MS;

    const NOW: u64 = 1_700_000_000_000;

    fn card(id: &str) -> Card {
        Card {
            id: id.to_string(),
            deck_id: "deck".to_string(),
            front: "front".to_string(),
            back: "back".to_string(),
            created_at: NOW,
            lineage: None,
        }
    }

    fn card_with_note(id: &str, note_id: &str, ordinal: u32) -> Card {
        let mut card = card(id);
        card.lineage = Some(CardLineage {
            note_id: note_id.to_string(),
            note_type_id: "basic-and-reversed".to_string(),
            template_id: id.to_string(),
            ordinal,
            cloze_ordinal: None,
        });
        card
    }

    fn progress(card_id: &str) -> CardProgress {
        CardProgress {
            card_id: card_id.to_string(),
            state: CardState::Review,
            interval: 1,
            ease_factor: 2.5,
            next_due_at: NOW - 1,
            learning_step_index: None,
            buried_until: None,
            suspended_at: None,
            times_seen: 1,
            times_correct: 1,
            times_incorrect: 0,
            last_seen_at: NOW - ONE_DAY_MS,
            flag: None,
            marked_at: None,
        }
    }

    fn basic_note_type() -> NoteType {
        NoteType {
            id: "basic".to_string(),
            name: "Basic".to_string(),
            fields: vec![
                FieldDef {
                    id: "front".to_string(),
                    name: "Front".to_string(),
                    required: true,
                    ordinal: 0,
                },
                FieldDef {
                    id: "back".to_string(),
                    name: "Back".to_string(),
                    required: true,
                    ordinal: 1,
                },
            ],
            templates: vec![CardTemplate {
                id: "forward".to_string(),
                name: "Forward".to_string(),
                front_template: "{{Front}}".to_string(),
                back_template: "{{Back}}".to_string(),
                deck_id: None,
                required_field_names: vec!["Front".to_string(), "Back".to_string()],
                requirement_mode: TemplateRequirementMode::All,
                ordinal: 0,
            }],
            stylesheet: None,
            created_at: NOW,
            updated_at: NOW,
        }
    }

    fn basic_note() -> Note {
        Note {
            id: "note".to_string(),
            note_type_id: "basic".to_string(),
            deck_id: "deck".to_string(),
            fields: vec![
                NoteFieldValue {
                    field_id: "front".to_string(),
                    value: "amma".to_string(),
                },
                NoteFieldValue {
                    field_id: "back".to_string(),
                    value: "mother".to_string(),
                },
            ],
            tags: vec!["tamil".to_string()],
            created_at: NOW,
            updated_at: NOW,
        }
    }

    #[test]
    fn create_deck_uses_host_supplied_id_and_time() {
        let next = reduce(
            &AppState::default(),
            EngramCommand::CreateDeck {
                id: "deck".to_string(),
                name: "Tamil".to_string(),
                description: "Letters and words".to_string(),
                created_at: NOW,
            },
        );

        assert_eq!(next.decks[0].id, "deck");
        assert_eq!(next.decks[0].created_at, NOW);
    }

    #[test]
    fn media_asset_commands_add_replace_and_prune_shared_state() {
        let audio = MediaAssetRecord {
            id: "media:audio".to_string(),
            archive_name: "0".to_string(),
            filename: Some("audio/hola.mp3".to_string()),
            data: b"mp3".to_vec(),
        };
        let replaced_audio = MediaAssetRecord {
            id: "media:audio".to_string(),
            archive_name: "0".to_string(),
            filename: Some("audio/hola-v2.mp3".to_string()),
            data: b"mp3-v2".to_vec(),
        };
        let image = MediaAssetRecord {
            id: "media:image".to_string(),
            archive_name: "1".to_string(),
            filename: Some("images/card.png".to_string()),
            data: b"png".to_vec(),
        };

        let mut state = reduce(
            &AppState::default(),
            EngramCommand::UpsertMediaAsset { asset: audio },
        );
        state = reduce(
            &state,
            EngramCommand::UpsertMediaAsset {
                asset: image.clone(),
            },
        );
        state = reduce(
            &state,
            EngramCommand::UpsertMediaAsset {
                asset: replaced_audio,
            },
        );

        assert_eq!(state.media_assets.len(), 2);
        assert_eq!(state.media_assets[0].id, "media:audio");
        assert_eq!(
            state.media_assets[0].filename.as_deref(),
            Some("audio/hola-v2.mp3")
        );
        assert_eq!(state.media_assets[0].data, b"mp3-v2");
        assert_eq!(state.media_assets[1], image);

        state = reduce(
            &state,
            EngramCommand::DeleteMediaAssets {
                asset_ids: vec!["media:image".to_string(), "missing".to_string()],
            },
        );
        assert_eq!(state.media_assets.len(), 1);
        assert_eq!(state.media_assets[0].id, "media:audio");

        let state = reduce(
            &state,
            EngramCommand::DeleteMediaAsset {
                asset_id: "media:audio".to_string(),
            },
        );
        assert!(state.media_assets.is_empty());
    }

    #[test]
    fn rename_note_type_field_command_migrates_templates() {
        let note_type = NoteType {
            id: "basic".to_string(),
            name: "Basic".to_string(),
            fields: vec![
                FieldDef {
                    id: "front".to_string(),
                    name: "Front".to_string(),
                    required: true,
                    ordinal: 0,
                },
                FieldDef {
                    id: "back".to_string(),
                    name: "Back".to_string(),
                    required: true,
                    ordinal: 1,
                },
            ],
            templates: vec![CardTemplate {
                id: "forward".to_string(),
                name: "Forward".to_string(),
                front_template: "{{Front}}".to_string(),
                back_template: "{{Back}}".to_string(),
                deck_id: None,
                required_field_names: vec!["Front".to_string(), "Back".to_string()],
                requirement_mode: TemplateRequirementMode::All,
                ordinal: 0,
            }],
            stylesheet: None,
            created_at: NOW,
            updated_at: NOW,
        };
        let note = Note {
            id: "note".to_string(),
            note_type_id: "basic".to_string(),
            deck_id: "deck".to_string(),
            fields: vec![
                NoteFieldValue {
                    field_id: "front".to_string(),
                    value: "hola".to_string(),
                },
                NoteFieldValue {
                    field_id: "back".to_string(),
                    value: "hello".to_string(),
                },
            ],
            tags: Vec::new(),
            created_at: NOW,
            updated_at: NOW,
        };
        let state = AppState {
            note_types: vec![note_type],
            notes: vec![note.clone()],
            ..AppState::default()
        };

        let next = reduce(
            &state,
            EngramCommand::RenameNoteTypeField {
                note_type_id: "basic".to_string(),
                field_id: "front".to_string(),
                name: "Prompt".to_string(),
                updated_at: NOW + 1,
            },
        );
        let generated = generate_cards_for_note(&next.note_types[0], &note);

        assert_eq!(next.note_types[0].fields[0].name, "Prompt");
        assert_eq!(next.note_types[0].templates[0].front_template, "{{Prompt}}");
        assert_eq!(
            next.note_types[0].templates[0].required_field_names,
            vec!["Prompt", "Back"]
        );
        assert_eq!(next.note_types[0].updated_at, NOW + 1);
        assert_eq!(generated[0].id, "note::forward");
        assert_eq!(generated[0].front, "hola");
    }

    #[test]
    fn upsert_note_type_can_sync_existing_notes() {
        let note_type = basic_note_type();
        let state = AppState {
            notes: vec![basic_note()],
            ..AppState::default()
        };

        let mut next = reduce(
            &state,
            EngramCommand::UpsertNoteType {
                note_type: note_type.clone(),
                materialize_cards_at: Some(NOW + 1),
            },
        );

        assert_eq!(next.note_types, vec![note_type]);
        assert_eq!(next.cards.len(), 1);
        assert_eq!(next.cards[0].id, "note::forward");
        assert_eq!(next.cards[0].front, "amma");
        assert_eq!(next.cards[0].created_at, NOW + 1);
        assert_eq!(
            next.cards[0]
                .lineage
                .as_ref()
                .map(|lineage| lineage.template_id.as_str()),
            Some("forward")
        );

        next.card_progress.push(progress("note::forward"));
        let mut changed_note_type = basic_note_type();
        changed_note_type.templates[0].front_template = "{{Back}}".to_string();
        changed_note_type.updated_at = NOW + 2;
        next = reduce(
            &next,
            EngramCommand::UpsertNoteType {
                note_type: changed_note_type,
                materialize_cards_at: Some(NOW + 3),
            },
        );

        assert_eq!(next.note_types[0].updated_at, NOW + 2);
        assert_eq!(next.cards.len(), 1);
        assert_eq!(next.cards[0].id, "note::forward");
        assert_eq!(next.cards[0].front, "mother");
        assert_eq!(next.cards[0].created_at, NOW + 1);
        assert_eq!(next.card_progress.len(), 1);
        assert_eq!(next.card_progress[0].card_id, "note::forward");
    }

    #[test]
    fn upsert_note_can_sync_generated_cards_and_preserve_progress() {
        let note_type = NoteType {
            id: "basic".to_string(),
            name: "Basic".to_string(),
            fields: vec![
                FieldDef {
                    id: "front".to_string(),
                    name: "Front".to_string(),
                    required: true,
                    ordinal: 0,
                },
                FieldDef {
                    id: "back".to_string(),
                    name: "Back".to_string(),
                    required: true,
                    ordinal: 1,
                },
            ],
            templates: vec![
                CardTemplate {
                    id: "forward".to_string(),
                    name: "Forward".to_string(),
                    front_template: "{{Front}}".to_string(),
                    back_template: "{{Back}}".to_string(),
                    deck_id: None,
                    required_field_names: vec!["Front".to_string(), "Back".to_string()],
                    requirement_mode: TemplateRequirementMode::All,
                    ordinal: 0,
                },
                CardTemplate {
                    id: "reverse".to_string(),
                    name: "Reverse".to_string(),
                    front_template: "{{Back}}".to_string(),
                    back_template: "{{Front}}".to_string(),
                    deck_id: None,
                    required_field_names: vec!["Front".to_string(), "Back".to_string()],
                    requirement_mode: TemplateRequirementMode::All,
                    ordinal: 1,
                },
            ],
            stylesheet: None,
            created_at: NOW,
            updated_at: NOW,
        };
        let note = Note {
            id: "note".to_string(),
            note_type_id: "basic".to_string(),
            deck_id: "deck".to_string(),
            fields: vec![
                NoteFieldValue {
                    field_id: "front".to_string(),
                    value: "hola".to_string(),
                },
                NoteFieldValue {
                    field_id: "back".to_string(),
                    value: "hello".to_string(),
                },
            ],
            tags: vec!["spanish".to_string()],
            created_at: NOW,
            updated_at: NOW,
        };
        let mut state = AppState {
            note_types: vec![note_type],
            ..AppState::default()
        };

        state = reduce(
            &state,
            EngramCommand::UpsertNote {
                note: note.clone(),
                materialize_cards_at: Some(NOW + 1),
            },
        );

        assert_eq!(state.notes, vec![note.clone()]);
        assert_eq!(state.cards.len(), 2);
        assert_eq!(state.cards[0].id, "note::forward");
        assert_eq!(state.cards[0].front, "hola");
        assert_eq!(state.cards[0].created_at, NOW + 1);

        state.card_progress.push(progress("note::forward"));
        let mut edited = note;
        edited.fields[0].value = "hola!".to_string();
        edited.updated_at = NOW + 2;
        state = reduce(
            &state,
            EngramCommand::UpsertNote {
                note: edited,
                materialize_cards_at: Some(NOW + 3),
            },
        );

        assert_eq!(state.notes[0].updated_at, NOW + 2);
        assert_eq!(state.cards.len(), 2);
        assert_eq!(state.cards[0].id, "note::forward");
        assert_eq!(state.cards[0].front, "hola!");
        assert_eq!(state.cards[0].created_at, NOW + 1);
        assert_eq!(state.card_progress.len(), 1);
        assert_eq!(state.card_progress[0].card_id, "note::forward");
    }

    #[test]
    fn delete_note_cascades_generated_cards_without_touching_manual_cards() {
        let generated = card_with_note("note::forward", "note", 0);
        let manual = card("manual");
        let state = AppState {
            notes: vec![Note {
                id: "note".to_string(),
                note_type_id: "basic".to_string(),
                deck_id: "deck".to_string(),
                fields: Vec::new(),
                tags: Vec::new(),
                created_at: NOW,
                updated_at: NOW,
            }],
            cards: vec![generated.clone(), manual.clone()],
            card_progress: vec![progress("note::forward"), progress("manual")],
            external_sources: vec![
                ExternalSourceRecord {
                    source: "anki".to_string(),
                    target: ExternalSourceTarget::Note,
                    target_id: "note".to_string(),
                    original_id: Some("1000".to_string()),
                    data: Default::default(),
                },
                ExternalSourceRecord {
                    source: "anki".to_string(),
                    target: ExternalSourceTarget::Card,
                    target_id: "note::forward".to_string(),
                    original_id: Some("2000".to_string()),
                    data: Default::default(),
                },
            ],
            active_session: Some(ActiveSessionState {
                session_id: "session".to_string(),
                deck_id: "deck".to_string(),
                queue: vec![generated, manual],
                current_index: 0,
                current_card_started_at: None,
                revealed: true,
            }),
            ..AppState::default()
        };

        let next = reduce(
            &state,
            EngramCommand::DeleteNote {
                note_id: "note".to_string(),
            },
        );

        assert!(next.notes.is_empty());
        assert_eq!(next.cards, vec![card("manual")]);
        assert_eq!(next.card_progress.len(), 1);
        assert_eq!(next.card_progress[0].card_id, "manual");
        assert_eq!(next.external_sources.len(), 2);
        assert!(next.external_sources.iter().any(|source| {
            source.target == ExternalSourceTarget::Deleted
                && source.original_id.as_deref() == Some("1000")
                && source.data.get("deletedTarget").map(String::as_str) == Some("note")
        }));
        assert!(next.external_sources.iter().any(|source| {
            source.target == ExternalSourceTarget::Deleted
                && source.original_id.as_deref() == Some("2000")
                && source.data.get("deletedTarget").map(String::as_str) == Some("card")
        }));
        let active = next.active_session.unwrap();
        assert_eq!(active.queue, vec![card("manual")]);
        assert_eq!(active.current_index, 0);
        assert!(!active.revealed);
    }

    #[test]
    fn delete_note_type_cascades_notes_and_generated_cards() {
        let generated = card_with_note("note::forward", "note", 0);
        let manual = card("manual");
        let state = AppState {
            note_types: vec![basic_note_type()],
            notes: vec![basic_note()],
            cards: vec![generated.clone(), manual.clone()],
            card_progress: vec![progress("note::forward"), progress("manual")],
            external_sources: vec![
                ExternalSourceRecord {
                    source: "anki".to_string(),
                    target: ExternalSourceTarget::NoteType,
                    target_id: "basic".to_string(),
                    original_id: Some("model".to_string()),
                    data: Default::default(),
                },
                ExternalSourceRecord {
                    source: "anki".to_string(),
                    target: ExternalSourceTarget::Note,
                    target_id: "note".to_string(),
                    original_id: Some("note".to_string()),
                    data: Default::default(),
                },
                ExternalSourceRecord {
                    source: "anki".to_string(),
                    target: ExternalSourceTarget::Card,
                    target_id: "note::forward".to_string(),
                    original_id: Some("card".to_string()),
                    data: Default::default(),
                },
            ],
            active_session: Some(ActiveSessionState {
                session_id: "session".to_string(),
                deck_id: "deck".to_string(),
                queue: vec![generated, manual],
                current_index: 0,
                current_card_started_at: None,
                revealed: true,
            }),
            ..AppState::default()
        };

        let next = reduce(
            &state,
            EngramCommand::DeleteNoteType {
                note_type_id: "basic".to_string(),
            },
        );

        assert!(next.note_types.is_empty());
        assert!(next.notes.is_empty());
        assert_eq!(next.cards, vec![card("manual")]);
        assert_eq!(next.card_progress.len(), 1);
        assert_eq!(next.card_progress[0].card_id, "manual");
        assert_eq!(next.external_sources.len(), 2);
        assert!(next.external_sources.iter().any(|source| {
            source.target == ExternalSourceTarget::Deleted
                && source.original_id.as_deref() == Some("note")
                && source.data.get("deletedTarget").map(String::as_str) == Some("note")
        }));
        assert!(next.external_sources.iter().any(|source| {
            source.target == ExternalSourceTarget::Deleted
                && source.original_id.as_deref() == Some("card")
                && source.data.get("deletedTarget").map(String::as_str) == Some("card")
        }));
        let active = next.active_session.unwrap();
        assert_eq!(active.queue, vec![card("manual")]);
        assert_eq!(active.current_index, 0);
        assert!(!active.revealed);
    }

    #[test]
    fn rate_card_creates_progress_review_and_session_stats() {
        let mut state = AppState::default();
        state.cards.push(card("card"));
        state = reduce(
            &state,
            EngramCommand::StartSession {
                session_id: "session".to_string(),
                deck_id: "deck".to_string(),
                queue: vec![card("card")],
                started_at: NOW,
            },
        );

        let next = reduce(
            &state,
            EngramCommand::RateCard {
                review_id: "review".to_string(),
                session_id: "session".to_string(),
                card_id: "card".to_string(),
                rating: Rating::Good,
                reviewed_at: NOW,
            },
        );

        assert_eq!(next.card_progress.len(), 1);
        assert_eq!(next.card_progress[0].times_seen, 1);
        assert_eq!(next.card_progress[0].state, CardState::Learning);
        assert_eq!(next.card_progress[0].learning_step_index, Some(1));
        assert_eq!(next.card_progress[0].next_due_at, NOW + 10 * ONE_MINUTE_MS);
        assert_eq!(next.reviews.len(), 1);
        assert_eq!(next.sessions[0].cards_reviewed, 1);
        assert_eq!(next.sessions[0].cards_correct, 1);
    }

    #[test]
    fn rate_card_records_answer_time_from_active_session_card_start() {
        let mut state = AppState::default();
        state.cards.push(card("card"));
        state = reduce(
            &state,
            EngramCommand::StartSession {
                session_id: "session".to_string(),
                deck_id: "deck".to_string(),
                queue: vec![card("card")],
                started_at: NOW,
            },
        );

        let next = reduce(
            &state,
            EngramCommand::RateCard {
                review_id: "review".to_string(),
                session_id: "session".to_string(),
                card_id: "card".to_string(),
                rating: Rating::Good,
                reviewed_at: NOW + 4_200,
            },
        );

        assert_eq!(next.reviews[0].answer_time_ms, Some(4_200));
        assert_eq!(
            next.active_session
                .as_ref()
                .unwrap()
                .current_card_started_at,
            Some(NOW + 4_200)
        );
    }

    #[test]
    fn rate_card_marks_and_suspends_leech_reviews_with_undo_snapshot() {
        let review_card = card_with_note("card", "note", 0);
        let mut previous = progress("card");
        previous.times_seen = 12;
        previous.times_correct = 5;
        previous.times_incorrect = 7;
        let mut state = AppState {
            notes: vec![basic_note()],
            cards: vec![review_card.clone()],
            card_progress: vec![previous.clone()],
            ..AppState::default()
        };
        state = reduce(
            &state,
            EngramCommand::StartSession {
                session_id: "session".to_string(),
                deck_id: "deck".to_string(),
                queue: vec![review_card],
                started_at: NOW,
            },
        );

        let next = reduce(
            &state,
            EngramCommand::RateCardWithOptions {
                review_id: "review".to_string(),
                session_id: "session".to_string(),
                card_id: "card".to_string(),
                rating: Rating::Again,
                reviewed_at: NOW,
                deck_options: DeckOptions {
                    leech_threshold: 8,
                    leech_action: LeechAction::Suspend,
                    ..DeckOptions::default()
                },
            },
        );

        assert_eq!(next.card_progress[0].state, CardState::Suspended);
        assert_eq!(next.card_progress[0].times_incorrect, 8);
        assert_eq!(next.card_progress[0].suspended_at, Some(NOW));
        assert_eq!(next.notes[0].tags, vec!["tamil", "leech"]);
        let leech = next.reviews[0].leech_event.as_ref().unwrap();
        assert_eq!(leech.action, LeechAction::Suspend);
        assert_eq!(leech.note_id.as_deref(), Some("note"));
        assert_eq!(leech.previous_note_tags, Some(vec!["tamil".to_string()]));

        let undone = reduce(
            &next,
            EngramCommand::UndoLastReview {
                session_id: "session".to_string(),
            },
        );

        assert_eq!(undone.card_progress[0], previous);
        assert_eq!(undone.notes[0].tags, vec!["tamil"]);
    }

    #[test]
    fn rate_card_repeats_leech_events_without_suspending_for_tag_only_decks() {
        let review_card = card_with_note("card", "note", 0);
        let mut previous = progress("card");
        previous.times_seen = 20;
        previous.times_correct = 9;
        previous.times_incorrect = 11;
        let mut note = basic_note();
        note.tags.push("leech".to_string());
        let mut state = AppState {
            notes: vec![note],
            cards: vec![review_card.clone()],
            card_progress: vec![previous],
            ..AppState::default()
        };
        state = reduce(
            &state,
            EngramCommand::StartSession {
                session_id: "session".to_string(),
                deck_id: "deck".to_string(),
                queue: vec![review_card],
                started_at: NOW,
            },
        );

        let next = reduce(
            &state,
            EngramCommand::RateCardWithOptions {
                review_id: "review".to_string(),
                session_id: "session".to_string(),
                card_id: "card".to_string(),
                rating: Rating::Again,
                reviewed_at: NOW,
                deck_options: DeckOptions {
                    leech_threshold: 8,
                    leech_action: LeechAction::TagOnly,
                    ..DeckOptions::default()
                },
            },
        );

        assert_eq!(next.card_progress[0].state, CardState::Relearning);
        assert_eq!(next.card_progress[0].times_incorrect, 12);
        assert_eq!(next.card_progress[0].suspended_at, None);
        assert_eq!(next.notes[0].tags, vec!["tamil", "leech"]);
        let leech = next.reviews[0].leech_event.as_ref().unwrap();
        assert_eq!(leech.action, LeechAction::TagOnly);
        assert_eq!(
            leech.previous_note_tags,
            Some(vec!["tamil".to_string(), "leech".to_string()])
        );
    }

    #[test]
    fn filtered_deck_review_without_reschedule_preserves_card_progress() {
        let mut review_card = card_with_note("card", "note", 0);
        review_card.deck_id = "filtered".to_string();
        let mut sibling = card_with_note("sibling", "note", 1);
        sibling.deck_id = "filtered".to_string();
        let mut previous = progress("card");
        previous.times_seen = 12;
        previous.times_correct = 5;
        previous.times_incorrect = 7;
        let previous_sibling = progress("sibling");
        let mut state = AppState {
            decks: vec![Deck {
                id: "filtered".to_string(),
                name: "Preview".to_string(),
                description: String::new(),
                created_at: NOW,
            }],
            notes: vec![basic_note()],
            cards: vec![review_card.clone(), sibling.clone()],
            card_progress: vec![previous.clone(), previous_sibling.clone()],
            external_sources: vec![ExternalSourceRecord {
                target: ExternalSourceTarget::Deck,
                target_id: "filtered".to_string(),
                source: "anki".to_string(),
                original_id: Some("3".to_string()),
                data: BTreeMap::from([
                    ("dyn".to_string(), "1".to_string()),
                    ("resched".to_string(), "false".to_string()),
                ]),
            }],
            ..AppState::default()
        };
        state = reduce(
            &state,
            EngramCommand::StartSession {
                session_id: "session".to_string(),
                deck_id: "filtered".to_string(),
                queue: vec![review_card, sibling],
                started_at: NOW,
            },
        );

        let next = reduce(
            &state,
            EngramCommand::RateCardWithOptions {
                review_id: "preview-review".to_string(),
                session_id: "session".to_string(),
                card_id: "card".to_string(),
                rating: Rating::Again,
                reviewed_at: NOW,
                deck_options: DeckOptions {
                    leech_threshold: 8,
                    leech_action: LeechAction::Suspend,
                    ..DeckOptions::default()
                },
            },
        );

        assert_eq!(
            next.card_progress
                .iter()
                .find(|progress| progress.card_id == "card"),
            Some(&previous)
        );
        assert_eq!(
            next.card_progress
                .iter()
                .find(|progress| progress.card_id == "sibling"),
            Some(&previous_sibling)
        );
        assert_eq!(next.notes[0].tags, vec!["tamil"]);
        assert_eq!(next.sessions[0].cards_reviewed, 1);
        assert_eq!(next.sessions[0].cards_correct, 0);
        assert_eq!(next.reviews[0].previous_progress, Some(previous.clone()));
        assert_eq!(next.reviews[0].resulting_progress, Some(previous.clone()));
        assert_eq!(next.reviews[0].leech_event, None);
        assert!(next.reviews[0].sibling_progress_snapshots.is_empty());

        let undone = reduce(
            &next,
            EngramCommand::UndoLastReview {
                session_id: "session".to_string(),
            },
        );

        assert_eq!(undone.card_progress, vec![previous, previous_sibling]);
        assert!(undone.reviews.is_empty());
        assert_eq!(undone.sessions[0].cards_reviewed, 0);
        assert_eq!(undone.sessions[0].cards_correct, 0);
    }

    #[test]
    fn rate_card_with_options_honors_custom_learning_steps() {
        let mut state = AppState::default();
        state.cards.push(card("card"));
        state = reduce(
            &state,
            EngramCommand::StartSession {
                session_id: "session".to_string(),
                deck_id: "deck".to_string(),
                queue: vec![card("card")],
                started_at: NOW,
            },
        );
        let deck_options = DeckOptions {
            learning_steps_minutes: vec![3, 30],
            ..DeckOptions::default()
        };

        let next = reduce(
            &state,
            EngramCommand::RateCardWithOptions {
                review_id: "review".to_string(),
                session_id: "session".to_string(),
                card_id: "card".to_string(),
                rating: Rating::Good,
                reviewed_at: NOW,
                deck_options,
            },
        );

        assert_eq!(next.card_progress[0].state, CardState::Learning);
        assert_eq!(next.card_progress[0].learning_step_index, Some(1));
        assert_eq!(next.card_progress[0].next_due_at, NOW + 30 * ONE_MINUTE_MS);
    }

    #[test]
    fn set_deck_options_inserts_and_replaces_presets() {
        let state = AppState::default();
        let initial_options = DeckOptions {
            learning_steps_minutes: vec![3, 30],
            ..DeckOptions::default()
        };

        let inserted = reduce(
            &state,
            EngramCommand::SetDeckOptions {
                deck_id: "deck".to_string(),
                options: initial_options,
            },
        );

        assert_eq!(inserted.deck_options.len(), 1);
        assert_eq!(inserted.deck_options[0].deck_id, "deck");
        assert_eq!(
            inserted.deck_options[0].options.learning_steps_minutes,
            vec![3, 30]
        );

        let replacement_options = DeckOptions {
            maximum_interval_days: 90,
            review_interval_modifier: 0.75,
            ..DeckOptions::default()
        };
        let replaced = reduce(
            &inserted,
            EngramCommand::SetDeckOptions {
                deck_id: "deck".to_string(),
                options: replacement_options,
            },
        );

        assert_eq!(replaced.deck_options.len(), 1);
        assert_eq!(replaced.deck_options[0].options.maximum_interval_days, 90);
        assert_eq!(
            replaced.deck_options[0].options.review_interval_modifier,
            0.75
        );
        assert_eq!(
            replaced.deck_options[0].options.learning_steps_minutes,
            DeckOptions::default().learning_steps_minutes
        );
    }

    #[test]
    fn rate_card_uses_stored_deck_options() {
        let mut state = AppState::default();
        state.cards.push(card("card"));
        state.deck_options.push(crate::model::DeckOptionsPreset {
            deck_id: "deck".to_string(),
            options: DeckOptions {
                learning_steps_minutes: vec![4, 40],
                ..DeckOptions::default()
            },
        });
        state = reduce(
            &state,
            EngramCommand::StartSession {
                session_id: "session".to_string(),
                deck_id: "deck".to_string(),
                queue: vec![card("card")],
                started_at: NOW,
            },
        );

        let next = reduce(
            &state,
            EngramCommand::RateCard {
                review_id: "review".to_string(),
                session_id: "session".to_string(),
                card_id: "card".to_string(),
                rating: Rating::Good,
                reviewed_at: NOW,
            },
        );

        assert_eq!(next.card_progress[0].state, CardState::Learning);
        assert_eq!(next.card_progress[0].learning_step_index, Some(1));
        assert_eq!(next.card_progress[0].next_due_at, NOW + 40 * ONE_MINUTE_MS);
    }

    #[test]
    fn rate_card_and_bury_siblings_can_be_undone_atomically() {
        let target = card_with_note("note::forward", "note", 0);
        let sibling = card_with_note("note::reverse", "note", 1);
        let unrelated = card_with_note("other::forward", "other", 0);
        let mut state = AppState {
            cards: vec![target.clone(), sibling.clone(), unrelated.clone()],
            ..AppState::default()
        };
        state = reduce(
            &state,
            EngramCommand::StartSession {
                session_id: "session".to_string(),
                deck_id: "deck".to_string(),
                queue: vec![target.clone(), sibling.clone(), unrelated.clone()],
                started_at: NOW,
            },
        );
        state = reduce(&state, EngramCommand::RevealCurrentCard);

        let reviewed = reduce(
            &state,
            EngramCommand::RateCardAndBurySiblings {
                review_id: "review".to_string(),
                session_id: "session".to_string(),
                card_id: target.id.clone(),
                rating: Rating::Good,
                reviewed_at: NOW,
                buried_until: NOW + ONE_DAY_MS,
            },
        );

        assert_eq!(reviewed.card_progress.len(), 2);
        assert_eq!(
            reviewed
                .card_progress
                .iter()
                .find(|progress| progress.card_id == sibling.id)
                .and_then(|progress| progress.buried_until),
            Some(NOW + ONE_DAY_MS)
        );
        assert_eq!(reviewed.reviews[0].sibling_progress_snapshots.len(), 1);
        assert_eq!(
            reviewed.reviews[0].sibling_progress_snapshots[0].card_id,
            sibling.id
        );
        assert!(reviewed.reviews[0].sibling_progress_snapshots[0]
            .previous_progress
            .is_none());
        let active_ids: Vec<_> = reviewed
            .active_session
            .as_ref()
            .unwrap()
            .queue
            .iter()
            .map(|card| card.id.as_str())
            .collect();
        assert_eq!(active_ids, vec!["note::forward", "other::forward"]);

        let advanced = reduce(&reviewed, EngramCommand::AdvanceSession);
        let undone = reduce(
            &advanced,
            EngramCommand::UndoLastReview {
                session_id: "session".to_string(),
            },
        );

        assert!(undone.card_progress.is_empty());
        assert!(undone.reviews.is_empty());
        assert_eq!(undone.sessions[0].cards_reviewed, 0);
        assert_eq!(undone.sessions[0].cards_correct, 0);
        let active = undone.active_session.as_ref().unwrap();
        assert_eq!(active.current_index, 0);
        assert!(active.revealed);
        let restored_ids: Vec<_> = active.queue.iter().map(|card| card.id.as_str()).collect();
        assert_eq!(
            restored_ids,
            vec!["note::forward", "note::reverse", "other::forward"]
        );
    }

    #[test]
    fn rate_card_uses_deck_sibling_bury_options_by_card_kind() {
        let target = card_with_note("note::target-review", "note", 0);
        let review_sibling = card_with_note("note::review", "note", 1);
        let new_sibling = card_with_note("note::new", "note", 2);
        let interday_sibling = card_with_note("note::interday", "note", 3);
        let intraday_sibling = card_with_note("note::intraday", "note", 4);
        let unrelated = card_with_note("other::forward", "other", 0);

        let mut interday_progress = progress(&interday_sibling.id);
        interday_progress.state = CardState::Learning;
        interday_progress.learning_step_index = Some(0);
        interday_progress.next_due_at = NOW + ONE_DAY_MS;

        let mut intraday_progress = progress(&intraday_sibling.id);
        intraday_progress.state = CardState::Learning;
        intraday_progress.learning_step_index = Some(0);
        intraday_progress.next_due_at = NOW + ONE_MINUTE_MS;

        let mut target_progress = progress(&target.id);
        target_progress.state = CardState::Learning;
        target_progress.learning_step_index = Some(0);
        target_progress.next_due_at = NOW + ONE_DAY_MS;

        let mut state = AppState {
            cards: vec![
                target.clone(),
                review_sibling.clone(),
                new_sibling.clone(),
                interday_sibling.clone(),
                intraday_sibling.clone(),
                unrelated.clone(),
            ],
            card_progress: vec![
                target_progress,
                progress(&review_sibling.id),
                interday_progress,
                intraday_progress,
            ],
            deck_options: vec![DeckOptionsPreset {
                deck_id: "deck".to_string(),
                options: DeckOptions {
                    bury_new_siblings: true,
                    bury_review_siblings: false,
                    bury_interday_learning_siblings: true,
                    ..DeckOptions::default()
                },
            }],
            ..AppState::default()
        };
        state = reduce(
            &state,
            EngramCommand::StartSession {
                session_id: "session".to_string(),
                deck_id: "deck".to_string(),
                queue: state.cards.clone(),
                started_at: NOW,
            },
        );

        let reviewed = reduce(
            &state,
            EngramCommand::RateCard {
                review_id: "review".to_string(),
                session_id: "session".to_string(),
                card_id: target.id.clone(),
                rating: Rating::Good,
                reviewed_at: NOW,
            },
        );

        let mut buried_ids: Vec<_> = reviewed
            .card_progress
            .iter()
            .filter(|progress| progress.buried_until == Some(NOW + ONE_DAY_MS))
            .map(|progress| progress.card_id.as_str())
            .collect();
        buried_ids.sort_unstable();
        assert_eq!(buried_ids, vec!["note::interday", "note::new"]);
        assert_eq!(
            reviewed.reviews[0]
                .sibling_progress_snapshots
                .iter()
                .map(|snapshot| snapshot.card_id.as_str())
                .collect::<Vec<_>>(),
            vec!["note::new", "note::interday"]
        );
        let active_ids: Vec<_> = reviewed
            .active_session
            .as_ref()
            .unwrap()
            .queue
            .iter()
            .map(|card| card.id.as_str())
            .collect();
        assert_eq!(
            active_ids,
            vec![
                "note::target-review",
                "note::review",
                "note::intraday",
                "other::forward"
            ]
        );
    }

    #[test]
    fn undo_last_review_removes_first_review_progress_and_restores_session_cursor() {
        let mut state = AppState::default();
        state.cards.push(card("card"));
        state.cards.push(card("other"));
        state = reduce(
            &state,
            EngramCommand::StartSession {
                session_id: "session".to_string(),
                deck_id: "deck".to_string(),
                queue: vec![card("card"), card("other")],
                started_at: NOW,
            },
        );
        state = reduce(&state, EngramCommand::RevealCurrentCard);
        state = reduce(
            &state,
            EngramCommand::RateCard {
                review_id: "review".to_string(),
                session_id: "session".to_string(),
                card_id: "card".to_string(),
                rating: Rating::Good,
                reviewed_at: NOW,
            },
        );
        state = reduce(&state, EngramCommand::AdvanceSession);

        let undone = reduce(
            &state,
            EngramCommand::UndoLastReview {
                session_id: "session".to_string(),
            },
        );

        assert!(undone.card_progress.is_empty());
        assert!(undone.reviews.is_empty());
        assert_eq!(undone.sessions[0].cards_reviewed, 0);
        assert_eq!(undone.sessions[0].cards_correct, 0);
        let active = undone.active_session.as_ref().unwrap();
        assert_eq!(active.current_index, 0);
        assert!(active.revealed);
    }

    #[test]
    fn undo_last_review_restores_existing_progress_snapshot() {
        let mut state = AppState::default();
        state.cards.push(card("card"));
        state.card_progress.push(progress("card"));
        state = reduce(
            &state,
            EngramCommand::StartSession {
                session_id: "session".to_string(),
                deck_id: "deck".to_string(),
                queue: vec![card("card")],
                started_at: NOW,
            },
        );
        let previous = state.card_progress[0].clone();
        state = reduce(
            &state,
            EngramCommand::RateCard {
                review_id: "review".to_string(),
                session_id: "session".to_string(),
                card_id: "card".to_string(),
                rating: Rating::Easy,
                reviewed_at: NOW,
            },
        );
        assert_ne!(state.card_progress[0], previous);
        assert_eq!(state.reviews[0].previous_progress, Some(previous.clone()));
        assert_eq!(
            state.reviews[0].resulting_progress,
            Some(state.card_progress[0].clone())
        );

        let undone = reduce(
            &state,
            EngramCommand::UndoLastReview {
                session_id: "session".to_string(),
            },
        );

        assert_eq!(undone.card_progress[0], previous);
        assert!(undone.reviews.is_empty());
        assert_eq!(undone.sessions[0].cards_reviewed, 0);
        assert_eq!(undone.sessions[0].cards_correct, 0);
    }

    #[test]
    fn undo_last_review_without_progress_snapshot_is_noop() {
        let mut state = AppState::default();
        state.cards.push(card("card"));
        state.card_progress.push(progress("card"));
        state.sessions.push(Session {
            id: "session".to_string(),
            deck_id: "deck".to_string(),
            status: SessionStatus::Active,
            started_at: NOW,
            ended_at: None,
            cards_reviewed: 1,
            cards_correct: 1,
        });
        state.reviews.push(Review {
            id: "legacy-review".to_string(),
            session_id: "session".to_string(),
            card_id: "card".to_string(),
            rating: Rating::Good,
            reviewed_at: NOW,
            answer_time_ms: None,
            leech_event: None,
            previous_progress: None,
            resulting_progress: None,
            previous_active_session: None,
            sibling_progress_snapshots: Vec::new(),
        });

        let undone = reduce(
            &state,
            EngramCommand::UndoLastReview {
                session_id: "session".to_string(),
            },
        );

        assert_eq!(undone, state);
    }

    #[test]
    fn suspend_new_card_creates_reversible_overlay_and_removes_it_from_session() {
        let mut state = AppState::default();
        state.cards.push(card("card"));
        state.cards.push(card("other"));
        state = reduce(
            &state,
            EngramCommand::StartSession {
                session_id: "session".to_string(),
                deck_id: "deck".to_string(),
                queue: vec![card("card"), card("other")],
                started_at: NOW,
            },
        );

        let suspended = reduce(
            &state,
            EngramCommand::SuspendCard {
                card_id: "card".to_string(),
                suspended_at: NOW,
            },
        );

        assert_eq!(suspended.card_progress.len(), 1);
        assert_eq!(suspended.card_progress[0].card_id, "card");
        assert_eq!(suspended.card_progress[0].suspended_at, Some(NOW));
        let active = suspended.active_session.as_ref().unwrap();
        assert_eq!(active.queue.len(), 1);
        assert_eq!(active.queue[0].id, "other");

        let queue = build_session_queue(
            &suspended.cards,
            &suspended.card_progress,
            "deck",
            NOW + ONE_DAY_MS,
        );
        let ids: Vec<_> = queue.iter().map(|card| card.id.as_str()).collect();
        assert_eq!(ids, vec!["other"]);

        let unsuspended = reduce(
            &suspended,
            EngramCommand::UnsuspendCard {
                card_id: "card".to_string(),
            },
        );

        assert!(unsuspended.card_progress.is_empty());
        let queue = build_session_queue(
            &unsuspended.cards,
            &unsuspended.card_progress,
            "deck",
            NOW + ONE_DAY_MS,
        );
        let ids: Vec<_> = queue.iter().map(|card| card.id.as_str()).collect();
        assert_eq!(ids, vec!["card", "other"]);
    }

    #[test]
    fn bury_existing_review_card_preserves_progress_and_can_be_unburied() {
        let mut state = AppState::default();
        state.cards.push(card("card"));
        state.card_progress.push(progress("card"));

        let buried = reduce(
            &state,
            EngramCommand::BuryCard {
                card_id: "card".to_string(),
                buried_at: NOW,
                buried_until: NOW + ONE_DAY_MS,
            },
        );

        assert_eq!(buried.card_progress[0].times_seen, 1);
        assert_eq!(buried.card_progress[0].buried_until, Some(NOW + ONE_DAY_MS));
        assert!(build_session_queue(&buried.cards, &buried.card_progress, "deck", NOW).is_empty());

        let unburied = reduce(
            &buried,
            EngramCommand::UnburyCard {
                card_id: "card".to_string(),
            },
        );

        assert_eq!(unburied.card_progress[0].times_seen, 1);
        assert_eq!(unburied.card_progress[0].buried_until, None);
        let queue = build_session_queue(&unburied.cards, &unburied.card_progress, "deck", NOW);
        assert_eq!(queue[0].id, "card");
    }

    #[test]
    fn bury_card_siblings_hides_same_note_cards_until_boundary() {
        let target = card_with_note("note::forward", "note", 0);
        let sibling = card_with_note("note::reverse", "note", 1);
        let unrelated = card_with_note("other::forward", "other", 0);
        let mut state = AppState {
            cards: vec![target.clone(), sibling.clone(), unrelated.clone()],
            ..AppState::default()
        };
        state = reduce(
            &state,
            EngramCommand::StartSession {
                session_id: "session".to_string(),
                deck_id: "deck".to_string(),
                queue: vec![target.clone(), sibling.clone(), unrelated.clone()],
                started_at: NOW,
            },
        );

        let buried = reduce(
            &state,
            EngramCommand::BuryCardSiblings {
                card_id: target.id.clone(),
                buried_at: NOW,
                buried_until: NOW + ONE_DAY_MS,
            },
        );

        assert_eq!(buried.card_progress.len(), 1);
        assert_eq!(buried.card_progress[0].card_id, sibling.id);
        assert_eq!(buried.card_progress[0].buried_until, Some(NOW + ONE_DAY_MS));
        let active_ids: Vec<_> = buried
            .active_session
            .as_ref()
            .unwrap()
            .queue
            .iter()
            .map(|card| card.id.as_str())
            .collect();
        assert_eq!(active_ids, vec!["note::forward", "other::forward"]);

        let hidden_queue = build_session_queue(&buried.cards, &buried.card_progress, "deck", NOW);
        let hidden_ids: Vec<_> = hidden_queue.iter().map(|card| card.id.as_str()).collect();
        assert_eq!(hidden_ids, vec!["note::forward", "other::forward"]);

        let restored_queue = build_session_queue(
            &buried.cards,
            &buried.card_progress,
            "deck",
            NOW + ONE_DAY_MS,
        );
        let restored_ids: Vec<_> = restored_queue.iter().map(|card| card.id.as_str()).collect();
        assert_eq!(
            restored_ids,
            vec!["note::reverse", "note::forward", "other::forward"]
        );
    }

    #[test]
    fn bury_card_siblings_without_lineage_is_noop() {
        let state = AppState {
            cards: vec![card("card"), card("other")],
            ..AppState::default()
        };

        let buried = reduce(
            &state,
            EngramCommand::BuryCardSiblings {
                card_id: "card".to_string(),
                buried_at: NOW,
                buried_until: NOW + ONE_DAY_MS,
            },
        );

        assert_eq!(buried, state);
    }

    #[test]
    fn flag_and_mark_new_card_create_reversible_overlay() {
        let mut state = AppState::default();
        state.cards.push(card("card"));

        let flagged = reduce(
            &state,
            EngramCommand::SetCardFlag {
                card_id: "card".to_string(),
                flag: Some(CardFlag::Purple),
                flagged_at: NOW,
            },
        );

        assert_eq!(flagged.card_progress.len(), 1);
        assert_eq!(flagged.card_progress[0].flag, Some(CardFlag::Purple));
        assert_eq!(flagged.card_progress[0].marked_at, None);

        let marked = reduce(
            &flagged,
            EngramCommand::MarkCard {
                card_id: "card".to_string(),
                marked_at: NOW + 1,
            },
        );

        assert_eq!(marked.card_progress.len(), 1);
        assert_eq!(marked.card_progress[0].flag, Some(CardFlag::Purple));
        assert_eq!(marked.card_progress[0].marked_at, Some(NOW + 1));

        let unflagged = reduce(
            &marked,
            EngramCommand::SetCardFlag {
                card_id: "card".to_string(),
                flag: None,
                flagged_at: NOW + 2,
            },
        );

        assert_eq!(unflagged.card_progress.len(), 1);
        assert_eq!(unflagged.card_progress[0].flag, None);
        assert_eq!(unflagged.card_progress[0].marked_at, Some(NOW + 1));

        let unmarked = reduce(
            &unflagged,
            EngramCommand::UnmarkCard {
                card_id: "card".to_string(),
            },
        );

        assert!(unmarked.card_progress.is_empty());
    }

    #[test]
    fn rate_card_preserves_flag_and_mark_metadata() {
        let mut state = AppState::default();
        state.cards.push(card("card"));
        state.card_progress.push(progress("card"));
        state.card_progress[0].flag = Some(CardFlag::Blue);
        state.card_progress[0].marked_at = Some(NOW - 5);
        state = reduce(
            &state,
            EngramCommand::StartSession {
                session_id: "session".to_string(),
                deck_id: "deck".to_string(),
                queue: vec![card("card")],
                started_at: NOW,
            },
        );

        let reviewed = reduce(
            &state,
            EngramCommand::RateCard {
                review_id: "review".to_string(),
                session_id: "session".to_string(),
                card_id: "card".to_string(),
                rating: Rating::Good,
                reviewed_at: NOW,
            },
        );

        assert_eq!(reviewed.card_progress[0].flag, Some(CardFlag::Blue));
        assert_eq!(reviewed.card_progress[0].marked_at, Some(NOW - 5));
        assert_eq!(
            reviewed.reviews[0]
                .previous_progress
                .as_ref()
                .and_then(|progress| progress.flag),
            Some(CardFlag::Blue)
        );
    }

    #[test]
    fn delete_deck_cascades_related_records() {
        let state = AppState {
            decks: vec![Deck {
                id: "deck".to_string(),
                name: "Deck".to_string(),
                description: String::new(),
                created_at: NOW,
            }],
            note_types: Vec::new(),
            notes: Vec::new(),
            cards: vec![card("card")],
            card_progress: vec![progress("card")],
            sessions: vec![Session {
                id: "session".to_string(),
                deck_id: "deck".to_string(),
                status: SessionStatus::Active,
                started_at: NOW,
                ended_at: None,
                cards_reviewed: 1,
                cards_correct: 1,
            }],
            reviews: vec![Review {
                id: "review".to_string(),
                session_id: "session".to_string(),
                card_id: "card".to_string(),
                rating: Rating::Good,
                reviewed_at: NOW,
                answer_time_ms: None,
                leech_event: None,
                previous_progress: None,
                resulting_progress: Some(progress("card")),
                previous_active_session: None,
                sibling_progress_snapshots: Vec::new(),
            }],
            deck_options: Vec::new(),
            external_sources: Vec::new(),
            media_assets: Vec::new(),
            active_session: Some(ActiveSessionState {
                session_id: "session".to_string(),
                deck_id: "deck".to_string(),
                queue: vec![card("card")],
                current_index: 0,
                current_card_started_at: None,
                revealed: false,
            }),
        };

        let next = reduce(
            &state,
            EngramCommand::DeleteDeck {
                deck_id: "deck".to_string(),
            },
        );

        assert!(next.decks.is_empty());
        assert!(next.cards.is_empty());
        assert!(next.card_progress.is_empty());
        assert!(next.sessions.is_empty());
        assert!(next.reviews.is_empty());
        assert!(next.active_session.is_none());
    }
}
