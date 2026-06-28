use std::collections::HashMap;

use crate::model::{Card, CardProgress, CardState, DeckStats};
use crate::scheduler::DeckOptions;

pub const SESSION_SIZE: usize = 20;
pub const MAX_NEW_PER_SESSION: usize = 7;

pub fn build_session_queue(
    all_cards: &[Card],
    all_progress: &[CardProgress],
    deck_id: &str,
    now: u64,
) -> Vec<Card> {
    build_session_queue_with_limits(
        all_cards,
        all_progress,
        deck_id,
        now,
        SESSION_SIZE,
        MAX_NEW_PER_SESSION,
        SESSION_SIZE,
    )
}

pub fn build_session_queue_with_options(
    all_cards: &[Card],
    all_progress: &[CardProgress],
    deck_id: &str,
    now: u64,
    options: &DeckOptions,
) -> Vec<Card> {
    let max_new = options.new_cards_per_day as usize;
    let max_reviews = options.reviews_per_day as usize;
    let session_size = (max_new + max_reviews).max(1);

    build_session_queue_with_limits(
        all_cards,
        all_progress,
        deck_id,
        now,
        session_size,
        max_new,
        max_reviews,
    )
}

fn build_session_queue_with_limits(
    all_cards: &[Card],
    all_progress: &[CardProgress],
    deck_id: &str,
    now: u64,
    session_size: usize,
    max_new: usize,
    max_reviews: usize,
) -> Vec<Card> {
    let progress_by_card: HashMap<&str, &CardProgress> = all_progress
        .iter()
        .map(|progress| (progress.card_id.as_str(), progress))
        .collect();

    let mut due_cards: Vec<Card> = all_cards
        .iter()
        .filter(|card| card.deck_id == deck_id)
        .filter(|card| {
            progress_by_card
                .get(card.id.as_str())
                .is_some_and(|progress| is_reviewable(progress, now))
        })
        .cloned()
        .collect();

    due_cards.sort_by_key(|card| {
        progress_by_card
            .get(card.id.as_str())
            .map(|progress| progress.next_due_at)
            .unwrap_or(u64::MAX)
    });

    let new_cards: Vec<Card> = all_cards
        .iter()
        .filter(|card| card.deck_id == deck_id)
        .filter(|card| !progress_by_card.contains_key(card.id.as_str()))
        .take(max_new)
        .cloned()
        .collect();

    let remaining_session_slots = session_size.saturating_sub(new_cards.len());
    let review_slots = due_cards
        .len()
        .min(max_reviews)
        .min(remaining_session_slots);
    due_cards
        .into_iter()
        .take(review_slots)
        .chain(new_cards)
        .collect()
}

pub(crate) fn is_reviewable(progress: &CardProgress, now: u64) -> bool {
    if progress.suspended_at.is_some() || progress.state == CardState::Suspended {
        return false;
    }
    if progress.state == CardState::Buried {
        match progress.buried_until {
            Some(buried_until) if buried_until <= now => {}
            _ => return false,
        }
    } else if progress
        .buried_until
        .is_some_and(|buried_until| buried_until > now)
    {
        return false;
    }

    matches!(
        progress.state,
        CardState::Learning | CardState::Review | CardState::Relearning | CardState::Buried
    ) && progress.next_due_at <= now
}

pub fn is_deck_caught_up(
    all_cards: &[Card],
    all_progress: &[CardProgress],
    deck_id: &str,
    now: u64,
) -> bool {
    build_session_queue(all_cards, all_progress, deck_id, now).is_empty()
}

pub fn get_deck_stats(
    all_cards: &[Card],
    all_progress: &[CardProgress],
    deck_id: &str,
    now: u64,
) -> DeckStats {
    let progress_by_card: HashMap<&str, &CardProgress> = all_progress
        .iter()
        .map(|progress| (progress.card_id.as_str(), progress))
        .collect();

    let mut stats = DeckStats {
        total: 0,
        new_count: 0,
        learning_count: 0,
        mastered_count: 0,
        due_count: 0,
        average_ease_factor: 0.0,
    };
    let mut ease_sum = 0.0;
    let mut ease_count = 0;

    for card in all_cards.iter().filter(|card| card.deck_id == deck_id) {
        stats.total += 1;
        match progress_by_card.get(card.id.as_str()) {
            Some(progress) => {
                if progress.interval > 21 {
                    stats.mastered_count += 1;
                } else {
                    stats.learning_count += 1;
                }
                if is_reviewable(progress, now) {
                    stats.due_count += 1;
                }
                ease_sum += progress.ease_factor;
                ease_count += 1;
            }
            None => {
                stats.new_count += 1;
            }
        }
    }

    if ease_count > 0 {
        stats.average_ease_factor = ease_sum / f64::from(ease_count);
    }

    stats
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_700_000_000_000;

    fn card(id: &str, deck_id: &str, created_at: u64) -> Card {
        Card {
            id: id.to_string(),
            deck_id: deck_id.to_string(),
            front: format!("front {id}"),
            back: format!("back {id}"),
            created_at,
        }
    }

    fn progress(card_id: &str, next_due_at: u64, interval: u32) -> CardProgress {
        CardProgress {
            card_id: card_id.to_string(),
            state: CardState::Review,
            interval,
            ease_factor: 2.5,
            next_due_at,
            learning_step_index: None,
            buried_until: None,
            suspended_at: None,
            times_seen: 1,
            times_correct: 1,
            times_incorrect: 0,
            last_seen_at: NOW - 10,
            flag: None,
            marked_at: None,
        }
    }

    #[test]
    fn session_queue_returns_due_cards_before_new_cards() {
        let cards = vec![
            card("new-1", "deck", 1),
            card("due-late", "deck", 2),
            card("due-early", "deck", 3),
        ];
        let progress = vec![
            progress("due-late", NOW - 1, 3),
            progress("due-early", NOW - 100, 3),
        ];

        let queue = build_session_queue(&cards, &progress, "deck", NOW);
        let ids: Vec<_> = queue.iter().map(|card| card.id.as_str()).collect();

        assert_eq!(ids, vec!["due-early", "due-late", "new-1"]);
    }

    #[test]
    fn caught_up_when_no_new_or_due_cards_exist() {
        let cards = vec![card("scheduled", "deck", 1)];
        let progress = vec![progress("scheduled", NOW + 1000, 30)];

        assert!(is_deck_caught_up(&cards, &progress, "deck", NOW));
    }

    #[test]
    fn deck_stats_count_learning_mastered_due_and_new() {
        let cards = vec![
            card("new", "deck", 1),
            card("learning", "deck", 2),
            card("mastered", "deck", 3),
        ];
        let progress = vec![
            progress("learning", NOW - 1, 3),
            progress("mastered", NOW + 1000, 22),
        ];

        let stats = get_deck_stats(&cards, &progress, "deck", NOW);

        assert_eq!(stats.total, 3);
        assert_eq!(stats.new_count, 1);
        assert_eq!(stats.learning_count, 1);
        assert_eq!(stats.mastered_count, 1);
        assert_eq!(stats.due_count, 1);
        assert_eq!(stats.average_ease_factor, 2.5);
    }

    #[test]
    fn session_queue_excludes_suspended_and_buried_cards() {
        let cards = vec![
            card("due", "deck", 1),
            card("suspended", "deck", 2),
            card("buried", "deck", 3),
        ];
        let mut due = progress("due", NOW - 1, 3);
        let mut suspended = progress("suspended", NOW - 1, 3);
        let mut buried = progress("buried", NOW - 1, 3);
        due.state = CardState::Review;
        suspended.state = CardState::Suspended;
        suspended.suspended_at = Some(NOW - 100);
        buried.state = CardState::Buried;
        buried.buried_until = Some(NOW + 1000);

        let queue = build_session_queue(&cards, &[due, suspended, buried], "deck", NOW);
        let ids: Vec<_> = queue.iter().map(|card| card.id.as_str()).collect();

        assert_eq!(ids, vec!["due"]);
    }

    #[test]
    fn expired_buried_cards_rejoin_the_due_queue() {
        let cards = vec![card("buried", "deck", 1)];
        let mut buried = progress("buried", NOW - 1, 3);
        buried.state = CardState::Buried;
        buried.buried_until = Some(NOW - 1);

        let queue = build_session_queue(&cards, &[buried], "deck", NOW);
        let ids: Vec<_> = queue.iter().map(|card| card.id.as_str()).collect();

        assert_eq!(ids, vec!["buried"]);
    }

    #[test]
    fn deck_options_limit_new_and_review_cards() {
        let cards = vec![
            card("due-1", "deck", 1),
            card("due-2", "deck", 2),
            card("new-1", "deck", 3),
            card("new-2", "deck", 4),
        ];
        let progress = vec![
            progress("due-1", NOW - 100, 3),
            progress("due-2", NOW - 50, 3),
        ];
        let options = DeckOptions {
            new_cards_per_day: 1,
            reviews_per_day: 1,
            ..DeckOptions::default()
        };

        let queue = build_session_queue_with_options(&cards, &progress, "deck", NOW, &options);
        let ids: Vec<_> = queue.iter().map(|card| card.id.as_str()).collect();

        assert_eq!(ids, vec!["due-1", "new-1"]);
    }
}
