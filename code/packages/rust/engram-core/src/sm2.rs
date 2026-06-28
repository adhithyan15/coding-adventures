use crate::model::{CardProgress, CardState, Rating};

pub const INITIAL_EASE_FACTOR: f64 = 2.5;
pub const MIN_EASE_FACTOR: f64 = 1.3;
pub const MAX_EASE_FACTOR: f64 = 4.0;
pub const ONE_DAY_MS: u64 = 24 * 60 * 60 * 1000;

pub fn create_initial_progress(
    card_id: impl Into<String>,
    rating: Rating,
    now: u64,
) -> CardProgress {
    let initial = CardProgress {
        card_id: card_id.into(),
        state: CardState::Review,
        interval: 1,
        ease_factor: INITIAL_EASE_FACTOR,
        next_due_at: now + ONE_DAY_MS,
        learning_step_index: None,
        buried_until: None,
        suspended_at: None,
        times_seen: 0,
        times_correct: 0,
        times_incorrect: 0,
        last_seen_at: now,
    };
    update_card_progress(&initial, rating, now)
}

pub fn update_card_progress(progress: &CardProgress, rating: Rating, now: u64) -> CardProgress {
    let mut interval = progress.interval;
    let mut ease_factor = progress.ease_factor;

    match rating {
        Rating::Again => {
            interval = 1;
            ease_factor = (ease_factor - 0.20).max(MIN_EASE_FACTOR);
        }
        Rating::Hard => {
            interval = ((interval as f64) * 1.2).round().max(1.0) as u32;
            ease_factor = (ease_factor - 0.15).max(MIN_EASE_FACTOR);
        }
        Rating::Good => {
            interval = ((interval as f64) * ease_factor).round().max(1.0) as u32;
        }
        Rating::Easy => {
            interval = ((interval as f64) * ease_factor * 1.3).round().max(1.0) as u32;
            ease_factor = (ease_factor + 0.15).min(MAX_EASE_FACTOR);
        }
    }

    let is_correct = rating != Rating::Again;

    CardProgress {
        card_id: progress.card_id.clone(),
        state: progress.state,
        interval,
        ease_factor,
        next_due_at: now + u64::from(interval) * ONE_DAY_MS,
        learning_step_index: progress.learning_step_index,
        buried_until: progress.buried_until,
        suspended_at: progress.suspended_at,
        times_seen: progress.times_seen + 1,
        times_correct: progress.times_correct + u32::from(is_correct),
        times_incorrect: progress.times_incorrect + u32::from(!is_correct),
        last_seen_at: now,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_700_000_000_000;

    fn progress() -> CardProgress {
        CardProgress {
            card_id: "card-1".to_string(),
            state: CardState::Review,
            interval: 1,
            ease_factor: INITIAL_EASE_FACTOR,
            next_due_at: 0,
            learning_step_index: None,
            buried_until: None,
            suspended_at: None,
            times_seen: 0,
            times_correct: 0,
            times_incorrect: 0,
            last_seen_at: 0,
        }
    }

    #[test]
    fn initial_progress_applies_first_rating() {
        let next = create_initial_progress("card-42", Rating::Good, NOW);

        assert_eq!(next.card_id, "card-42");
        assert_eq!(next.times_seen, 1);
        assert_eq!(next.times_correct, 1);
        assert_eq!(next.times_incorrect, 0);
        assert_eq!(next.last_seen_at, NOW);
    }

    #[test]
    fn again_resets_interval_and_decreases_ease() {
        let mut current = progress();
        current.interval = 10;

        let next = update_card_progress(&current, Rating::Again, NOW);

        assert_eq!(next.interval, 1);
        assert_eq!(next.ease_factor, 2.3);
        assert_eq!(next.next_due_at, NOW + ONE_DAY_MS);
        assert_eq!(next.times_incorrect, 1);
    }

    #[test]
    fn hard_grows_slowly_and_decreases_ease() {
        let mut current = progress();
        current.interval = 5;

        let next = update_card_progress(&current, Rating::Hard, NOW);

        assert_eq!(next.interval, 6);
        assert!((next.ease_factor - 2.35).abs() < f64::EPSILON);
        assert_eq!(next.times_correct, 1);
    }

    #[test]
    fn good_uses_ease_factor_without_changing_it() {
        let mut current = progress();
        current.interval = 4;

        let next = update_card_progress(&current, Rating::Good, NOW);

        assert_eq!(next.interval, 10);
        assert_eq!(next.ease_factor, INITIAL_EASE_FACTOR);
    }

    #[test]
    fn easy_adds_bonus_and_increases_ease() {
        let mut current = progress();
        current.interval = 4;

        let next = update_card_progress(&current, Rating::Easy, NOW);

        assert_eq!(next.interval, 13);
        assert!((next.ease_factor - 2.65).abs() < f64::EPSILON);
    }
}
