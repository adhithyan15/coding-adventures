use crate::model::{
    ActiveSessionState, AppState, Card, CardProgress, Deck, Rating, Review, Session, SessionStatus,
};
use crate::scheduler::{schedule_review, DeckOptions};
use crate::sm2::INITIAL_EASE_FACTOR;

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
    RateCardWithOptions {
        review_id: String,
        session_id: String,
        card_id: String,
        rating: Rating,
        reviewed_at: u64,
        deck_options: DeckOptions,
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
        EngramCommand::DeleteDeck { deck_id } => {
            let card_ids: Vec<String> = state
                .cards
                .iter()
                .filter(|card| card.deck_id == deck_id)
                .map(|card| card.id.clone())
                .collect();
            let session_ids: Vec<String> = state
                .sessions
                .iter()
                .filter(|session| session.deck_id == deck_id)
                .map(|session| session.id.clone())
                .collect();

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
                active_session: state
                    .active_session
                    .as_ref()
                    .filter(|session| session.deck_id != deck_id)
                    .cloned(),
            }
        }
        EngramCommand::CreateCard {
            id,
            deck_id,
            front,
            back,
            created_at,
        } => {
            let mut next = state.clone();
            next.cards.push(Card {
                id,
                deck_id,
                front,
                back,
                created_at,
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
        } => reduce_rate_card(
            state,
            review_id,
            session_id,
            card_id,
            rating,
            reviewed_at,
            &DeckOptions::default(),
        ),
        EngramCommand::RateCardWithOptions {
            review_id,
            session_id,
            card_id,
            rating,
            reviewed_at,
            deck_options,
        } => reduce_rate_card(
            state,
            review_id,
            session_id,
            card_id,
            rating,
            reviewed_at,
            &deck_options,
        ),
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

fn reduce_rate_card(
    state: &AppState,
    review_id: String,
    session_id: String,
    card_id: String,
    rating: Rating,
    reviewed_at: u64,
    deck_options: &DeckOptions,
) -> AppState {
    if state.active_session.is_none() {
        return state.clone();
    }

    let existing = state
        .card_progress
        .iter()
        .find(|progress| progress.card_id == card_id);
    let new_progress =
        schedule_review(existing, card_id.clone(), rating, deck_options, reviewed_at);

    let mut next = state.clone();
    upsert_progress(&mut next.card_progress, new_progress);
    next.reviews.push(Review {
        id: review_id,
        session_id: session_id.clone(),
        card_id,
        rating,
        reviewed_at,
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
    next
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
    use crate::model::CardState;
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
        }
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
            }],
            active_session: Some(ActiveSessionState {
                session_id: "session".to_string(),
                deck_id: "deck".to_string(),
                queue: vec![card("card")],
                current_index: 0,
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
