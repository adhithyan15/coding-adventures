use crate::model::{AppState, SessionProgress};

pub fn get_active_session_progress(state: &AppState) -> Option<SessionProgress> {
    let active = state.active_session.as_ref()?;
    let total_cards = active.queue.len();
    let current_index = active.current_index.min(total_cards);
    let current_position = if total_cards == 0 {
        0
    } else {
        (current_index + 1).min(total_cards)
    };
    let remaining_cards = total_cards.saturating_sub(current_index);
    let session_stats = state
        .sessions
        .iter()
        .find(|session| session.id == active.session_id);

    Some(SessionProgress {
        session_id: active.session_id.clone(),
        deck_id: active.deck_id.clone(),
        total_cards,
        current_index,
        current_position,
        remaining_cards,
        cards_reviewed: session_stats.map_or(0, |session| session.cards_reviewed),
        cards_correct: session_stats.map_or(0, |session| session.cards_correct),
        revealed: active.revealed,
        completed: remaining_cards == 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ActiveSessionState, Card, Session, SessionStatus};

    const NOW: u64 = 1_700_000_000_000;

    fn card(id: &str) -> Card {
        Card {
            id: id.to_string(),
            deck_id: "deck".to_string(),
            front: format!("front-{id}"),
            back: format!("back-{id}"),
            created_at: NOW,
            lineage: None,
        }
    }

    fn active_state(queue: Vec<Card>, current_index: usize, revealed: bool) -> AppState {
        AppState {
            sessions: vec![Session {
                id: "session".to_string(),
                deck_id: "deck".to_string(),
                status: SessionStatus::Active,
                started_at: NOW,
                ended_at: None,
                cards_reviewed: 2,
                cards_correct: 1,
            }],
            active_session: Some(ActiveSessionState {
                session_id: "session".to_string(),
                deck_id: "deck".to_string(),
                queue,
                current_index,
                revealed,
            }),
            ..AppState::default()
        }
    }

    #[test]
    fn no_active_session_has_no_progress() {
        assert_eq!(get_active_session_progress(&AppState::default()), None);
    }

    #[test]
    fn active_session_progress_counts_cards_and_stats() {
        let state = active_state(vec![card("a"), card("b"), card("c")], 1, true);
        let progress = get_active_session_progress(&state).unwrap();

        assert_eq!(progress.session_id, "session");
        assert_eq!(progress.deck_id, "deck");
        assert_eq!(progress.total_cards, 3);
        assert_eq!(progress.current_index, 1);
        assert_eq!(progress.current_position, 2);
        assert_eq!(progress.remaining_cards, 2);
        assert_eq!(progress.cards_reviewed, 2);
        assert_eq!(progress.cards_correct, 1);
        assert!(progress.revealed);
        assert!(!progress.completed);
    }

    #[test]
    fn progress_clamps_past_end_cursor_as_completed() {
        let state = active_state(vec![card("a"), card("b")], 5, false);
        let progress = get_active_session_progress(&state).unwrap();

        assert_eq!(progress.current_index, 2);
        assert_eq!(progress.current_position, 2);
        assert_eq!(progress.remaining_cards, 0);
        assert!(progress.completed);
    }

    #[test]
    fn missing_session_stats_fall_back_to_zero() {
        let mut state = active_state(vec![card("a")], 0, false);
        state.sessions.clear();
        let progress = get_active_session_progress(&state).unwrap();

        assert_eq!(progress.cards_reviewed, 0);
        assert_eq!(progress.cards_correct, 0);
    }
}
