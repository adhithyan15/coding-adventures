use crate::model::{
    ActiveSessionState, AppState, Card, CardProgress, Deck, Rating, Review, Session, SessionStatus,
};
use crate::sm2::{create_initial_progress, update_card_progress};

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
            ..state.clone()
        },
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
        } => {
            if state.active_session.is_none() {
                return state.clone();
            }

            let existing = state
                .card_progress
                .iter()
                .find(|progress| progress.card_id == card_id);
            let new_progress = match existing {
                Some(progress) => update_card_progress(progress, rating, reviewed_at),
                None => create_initial_progress(card_id.clone(), rating, reviewed_at),
            };

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
        assert_eq!(next.card_progress[0].next_due_at, NOW + 3 * ONE_DAY_MS);
        assert_eq!(next.reviews.len(), 1);
        assert_eq!(next.sessions[0].cards_reviewed, 1);
        assert_eq!(next.sessions[0].cards_correct, 1);
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
            card_progress: vec![CardProgress {
                card_id: "card".to_string(),
                interval: 1,
                ease_factor: 2.5,
                next_due_at: NOW,
                times_seen: 1,
                times_correct: 1,
                times_incorrect: 0,
                last_seen_at: NOW,
            }],
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
