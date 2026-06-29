use std::collections::HashSet;

use crate::model::{AppState, Rating, RatingCounts, ReviewHistorySummary};

pub fn summarize_review_history(
    state: &AppState,
    deck_id: &str,
    reviewed_after: u64,
    reviewed_before: u64,
) -> ReviewHistorySummary {
    let mut summary = ReviewHistorySummary {
        deck_id: deck_id.to_string(),
        reviewed_after,
        reviewed_before,
        total_reviews: 0,
        correct_reviews: 0,
        unique_cards: 0,
        rating_counts: RatingCounts::default(),
        first_reviewed_at: None,
        last_reviewed_at: None,
    };

    let deck_card_ids: HashSet<&str> = state
        .cards
        .iter()
        .filter(|card| card.deck_id == deck_id)
        .map(|card| card.id.as_str())
        .collect();
    let mut reviewed_card_ids: HashSet<&str> = HashSet::new();

    for review in &state.reviews {
        if review.reviewed_at < reviewed_after || review.reviewed_at >= reviewed_before {
            continue;
        }
        if !deck_card_ids.contains(review.card_id.as_str()) {
            continue;
        }

        summary.total_reviews += 1;
        reviewed_card_ids.insert(review.card_id.as_str());
        summary.first_reviewed_at = match summary.first_reviewed_at {
            Some(existing) => Some(existing.min(review.reviewed_at)),
            None => Some(review.reviewed_at),
        };
        summary.last_reviewed_at = match summary.last_reviewed_at {
            Some(existing) => Some(existing.max(review.reviewed_at)),
            None => Some(review.reviewed_at),
        };

        match review.rating {
            Rating::Again => summary.rating_counts.again += 1,
            Rating::Hard => {
                summary.rating_counts.hard += 1;
                summary.correct_reviews += 1;
            }
            Rating::Good => {
                summary.rating_counts.good += 1;
                summary.correct_reviews += 1;
            }
            Rating::Easy => {
                summary.rating_counts.easy += 1;
                summary.correct_reviews += 1;
            }
        }
    }

    summary.unique_cards = reviewed_card_ids.len();
    summary
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Card, Review};

    const NOW: u64 = 1_700_000_000_000;

    fn card(id: &str, deck_id: &str) -> Card {
        Card {
            id: id.to_string(),
            deck_id: deck_id.to_string(),
            front: format!("front-{id}"),
            back: format!("back-{id}"),
            created_at: NOW,
            lineage: None,
        }
    }

    fn review(id: &str, card_id: &str, rating: Rating, reviewed_at: u64) -> Review {
        Review {
            id: id.to_string(),
            session_id: "session".to_string(),
            card_id: card_id.to_string(),
            rating,
            reviewed_at,
            answer_time_ms: None,
            previous_progress: None,
            resulting_progress: None,
            previous_active_session: None,
            sibling_progress_snapshots: Vec::new(),
        }
    }

    #[test]
    fn summary_counts_reviews_for_deck_and_range() {
        let state = AppState {
            cards: vec![card("a", "deck"), card("b", "deck"), card("x", "other")],
            reviews: vec![
                review("r1", "a", Rating::Again, NOW + 10),
                review("r2", "a", Rating::Hard, NOW + 20),
                review("r3", "b", Rating::Good, NOW + 30),
                review("r4", "b", Rating::Easy, NOW + 40),
                review("before", "a", Rating::Good, NOW - 1),
                review("other-deck", "x", Rating::Easy, NOW + 50),
            ],
            ..AppState::default()
        };

        let summary = summarize_review_history(&state, "deck", NOW, NOW + 50);

        assert_eq!(summary.deck_id, "deck");
        assert_eq!(summary.total_reviews, 4);
        assert_eq!(summary.correct_reviews, 3);
        assert_eq!(summary.unique_cards, 2);
        assert_eq!(summary.rating_counts.again, 1);
        assert_eq!(summary.rating_counts.hard, 1);
        assert_eq!(summary.rating_counts.good, 1);
        assert_eq!(summary.rating_counts.easy, 1);
        assert_eq!(summary.first_reviewed_at, Some(NOW + 10));
        assert_eq!(summary.last_reviewed_at, Some(NOW + 40));
    }

    #[test]
    fn empty_range_returns_zero_summary() {
        let state = AppState {
            cards: vec![card("a", "deck")],
            reviews: vec![review("r1", "a", Rating::Good, NOW + 10)],
            ..AppState::default()
        };

        let summary = summarize_review_history(&state, "deck", NOW + 20, NOW + 30);

        assert_eq!(summary.total_reviews, 0);
        assert_eq!(summary.correct_reviews, 0);
        assert_eq!(summary.unique_cards, 0);
        assert_eq!(summary.first_reviewed_at, None);
        assert_eq!(summary.last_reviewed_at, None);
    }
}
