use crate::model::{CardProgress, CardState, DeckOptions, Rating};
use crate::sm2::{INITIAL_EASE_FACTOR, MAX_EASE_FACTOR, MIN_EASE_FACTOR, ONE_DAY_MS};

pub const ONE_MINUTE_MS: u64 = 60 * 1000;

pub fn schedule_review(
    existing: Option<&CardProgress>,
    card_id: impl Into<String>,
    rating: Rating,
    options: &DeckOptions,
    now: u64,
) -> CardProgress {
    match existing {
        Some(progress) => schedule_existing(progress, rating, options, now),
        None => schedule_new(card_id.into(), rating, options, now),
    }
}

fn schedule_new(card_id: String, rating: Rating, options: &DeckOptions, now: u64) -> CardProgress {
    let base = CardProgress {
        card_id,
        state: CardState::Learning,
        interval: 0,
        ease_factor: finite_positive(options.initial_ease_factor, INITIAL_EASE_FACTOR),
        next_due_at: now,
        learning_step_index: Some(0),
        buried_until: None,
        suspended_at: None,
        times_seen: 0,
        times_correct: 0,
        times_incorrect: 0,
        last_seen_at: now,
        flag: None,
        marked_at: None,
    };

    schedule_learning(&base, rating, options, now, false)
}

fn schedule_existing(
    progress: &CardProgress,
    rating: Rating,
    options: &DeckOptions,
    now: u64,
) -> CardProgress {
    match progress.state {
        CardState::Learning => schedule_learning(progress, rating, options, now, false),
        CardState::Relearning => schedule_learning(progress, rating, options, now, true),
        CardState::Review => schedule_review_card(progress, rating, options, now),
        CardState::Suspended | CardState::Buried => progress.clone(),
    }
}

fn schedule_learning(
    progress: &CardProgress,
    rating: Rating,
    options: &DeckOptions,
    now: u64,
    relearning: bool,
) -> CardProgress {
    let steps = if relearning {
        &options.relearning_steps_minutes
    } else {
        &options.learning_steps_minutes
    };

    let current_step = progress.learning_step_index.unwrap_or(0);
    let is_correct = rating != Rating::Again;

    let mut next = progress.clone();
    next.times_seen += 1;
    next.times_correct += u32::from(is_correct);
    next.times_incorrect += u32::from(!is_correct);
    next.last_seen_at = now;
    next.buried_until = None;
    next.suspended_at = None;

    match rating {
        Rating::Again => {
            next.state = if relearning {
                CardState::Relearning
            } else {
                CardState::Learning
            };
            next.learning_step_index = Some(0);
            next.interval = 0;
            next.ease_factor = (next.ease_factor - 0.20).max(MIN_EASE_FACTOR);
            next.next_due_at = due_after_step(steps, 0, now);
        }
        Rating::Hard => {
            next.state = if relearning {
                CardState::Relearning
            } else {
                CardState::Learning
            };
            next.learning_step_index = Some(current_step);
            next.interval = 0;
            next.ease_factor = (next.ease_factor - 0.15).max(MIN_EASE_FACTOR);
            next.next_due_at = due_after_step(steps, current_step, now);
        }
        Rating::Good => {
            let next_step = current_step + 1;
            if (next_step as usize) < steps.len() {
                next.state = if relearning {
                    CardState::Relearning
                } else {
                    CardState::Learning
                };
                next.learning_step_index = Some(next_step);
                next.interval = 0;
                next.next_due_at = due_after_step(steps, next_step, now);
            } else {
                graduate(&mut next, options.graduating_interval_days, options, now);
            }
        }
        Rating::Easy => {
            graduate(&mut next, options.easy_interval_days, options, now);
        }
    }

    next
}

fn schedule_review_card(
    progress: &CardProgress,
    rating: Rating,
    options: &DeckOptions,
    now: u64,
) -> CardProgress {
    let mut next = progress.clone();
    let is_correct = rating != Rating::Again;
    next.times_seen += 1;
    next.times_correct += u32::from(is_correct);
    next.times_incorrect += u32::from(!is_correct);
    next.last_seen_at = now;
    next.buried_until = None;
    next.suspended_at = None;

    match rating {
        Rating::Again => {
            next.state = CardState::Relearning;
            next.learning_step_index = Some(0);
            next.ease_factor = (next.ease_factor - 0.20).max(MIN_EASE_FACTOR);
            next.interval = capped_interval_days(
                (progress.interval as f64)
                    * finite_positive(options.lapse_interval_multiplier, 0.0),
                options,
            );
            next.next_due_at = due_after_step(&options.relearning_steps_minutes, 0, now);
        }
        Rating::Hard => {
            next.state = CardState::Review;
            next.learning_step_index = None;
            next.ease_factor = (next.ease_factor - 0.15).max(MIN_EASE_FACTOR);
            next.interval = capped_interval_days(
                (progress.interval as f64)
                    * finite_positive(options.hard_interval_multiplier, 1.2)
                    * finite_positive(options.review_interval_modifier, 1.0),
                options,
            );
            next.next_due_at = now + u64::from(next.interval) * ONE_DAY_MS;
        }
        Rating::Good => {
            next.state = CardState::Review;
            next.learning_step_index = None;
            next.interval = capped_interval_days(
                (progress.interval as f64)
                    * progress.ease_factor
                    * finite_positive(options.review_interval_modifier, 1.0),
                options,
            );
            next.next_due_at = now + u64::from(next.interval) * ONE_DAY_MS;
        }
        Rating::Easy => {
            next.state = CardState::Review;
            next.learning_step_index = None;
            next.ease_factor = (next.ease_factor + 0.15).min(MAX_EASE_FACTOR);
            next.interval = capped_interval_days(
                (progress.interval as f64)
                    * progress.ease_factor
                    * finite_positive(options.easy_bonus_multiplier, 1.3)
                    * finite_positive(options.review_interval_modifier, 1.0),
                options,
            );
            next.next_due_at = now + u64::from(next.interval) * ONE_DAY_MS;
        }
    }

    next
}

fn graduate(progress: &mut CardProgress, base_interval_days: u32, options: &DeckOptions, now: u64) {
    let interval = capped_interval_days(base_interval_days as f64, options);
    progress.state = CardState::Review;
    progress.learning_step_index = None;
    progress.interval = interval;
    progress.next_due_at = now + u64::from(interval) * ONE_DAY_MS;
}

fn capped_interval_days(days: f64, options: &DeckOptions) -> u32 {
    let max_interval = options.maximum_interval_days.max(1);
    if !days.is_finite() {
        return 1;
    }
    days.round().max(1.0).min(max_interval as f64) as u32
}

fn finite_positive(value: f64, fallback: f64) -> f64 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback
    }
}

fn due_after_step(steps: &[u32], index: u32, now: u64) -> u64 {
    let minutes = steps.get(index as usize).copied().unwrap_or(0);
    now + u64::from(minutes) * ONE_MINUTE_MS
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_700_000_000_000;

    fn options() -> DeckOptions {
        DeckOptions {
            learning_steps_minutes: vec![1, 10],
            relearning_steps_minutes: vec![5],
            graduating_interval_days: 1,
            easy_interval_days: 4,
            ..DeckOptions::default()
        }
    }

    fn review_progress() -> CardProgress {
        CardProgress {
            card_id: "card".to_string(),
            state: CardState::Review,
            interval: 10,
            ease_factor: 2.5,
            next_due_at: NOW,
            learning_step_index: None,
            buried_until: None,
            suspended_at: None,
            times_seen: 3,
            times_correct: 3,
            times_incorrect: 0,
            last_seen_at: NOW - ONE_DAY_MS,
            flag: None,
            marked_at: None,
        }
    }

    #[test]
    fn new_good_card_advances_to_next_learning_step() {
        let next = schedule_review(None, "card", Rating::Good, &options(), NOW);

        assert_eq!(next.state, CardState::Learning);
        assert_eq!(next.learning_step_index, Some(1));
        assert_eq!(next.next_due_at, NOW + 10 * ONE_MINUTE_MS);
        assert_eq!(next.times_seen, 1);
        assert_eq!(next.times_correct, 1);
    }

    #[test]
    fn learning_good_on_final_step_graduates_to_review() {
        let mut current = schedule_review(None, "card", Rating::Good, &options(), NOW);
        current.next_due_at = NOW;

        let next = schedule_review(Some(&current), "card", Rating::Good, &options(), NOW);

        assert_eq!(next.state, CardState::Review);
        assert_eq!(next.learning_step_index, None);
        assert_eq!(next.interval, 1);
        assert_eq!(next.next_due_at, NOW + ONE_DAY_MS);
    }

    #[test]
    fn new_easy_card_graduates_with_easy_interval() {
        let next = schedule_review(None, "card", Rating::Easy, &options(), NOW);

        assert_eq!(next.state, CardState::Review);
        assert_eq!(next.interval, 4);
        assert_eq!(next.next_due_at, NOW + 4 * ONE_DAY_MS);
    }

    #[test]
    fn new_cards_start_with_deck_initial_ease_factor() {
        let options = DeckOptions {
            initial_ease_factor: 2.8,
            ..options()
        };

        let next = schedule_review(None, "card", Rating::Good, &options, NOW);

        assert_eq!(next.state, CardState::Learning);
        assert_eq!(next.ease_factor, 2.8);
    }

    #[test]
    fn failed_review_enters_relearning() {
        let current = review_progress();
        let next = schedule_review(Some(&current), "card", Rating::Again, &options(), NOW);

        assert_eq!(next.state, CardState::Relearning);
        assert_eq!(next.learning_step_index, Some(0));
        assert_eq!(next.next_due_at, NOW + 5 * ONE_MINUTE_MS);
        assert_eq!(next.times_incorrect, 1);
    }

    #[test]
    fn review_interval_modifier_scales_good_review() {
        let current = review_progress();
        let options = DeckOptions {
            review_interval_modifier: 0.5,
            ..options()
        };

        let next = schedule_review(Some(&current), "card", Rating::Good, &options, NOW);

        assert_eq!(next.state, CardState::Review);
        assert_eq!(next.interval, 13);
        assert_eq!(next.next_due_at, NOW + 13 * ONE_DAY_MS);
    }

    #[test]
    fn maximum_interval_caps_review_and_graduation_intervals() {
        let mut current = review_progress();
        current.interval = 100;
        let options = DeckOptions {
            maximum_interval_days: 30,
            easy_interval_days: 100,
            ..options()
        };

        let reviewed = schedule_review(Some(&current), "card", Rating::Good, &options, NOW);
        let graduated = schedule_review(None, "card", Rating::Easy, &options, NOW);

        assert_eq!(reviewed.interval, 30);
        assert_eq!(reviewed.next_due_at, NOW + 30 * ONE_DAY_MS);
        assert_eq!(graduated.interval, 30);
        assert_eq!(graduated.next_due_at, NOW + 30 * ONE_DAY_MS);
    }

    #[test]
    fn hard_and_easy_review_multipliers_are_configurable() {
        let current = review_progress();
        let options = DeckOptions {
            hard_interval_multiplier: 1.5,
            easy_bonus_multiplier: 2.0,
            ..options()
        };

        let hard = schedule_review(Some(&current), "card", Rating::Hard, &options, NOW);
        let easy = schedule_review(Some(&current), "card", Rating::Easy, &options, NOW);

        assert_eq!(hard.interval, 15);
        assert_eq!(hard.next_due_at, NOW + 15 * ONE_DAY_MS);
        assert!((hard.ease_factor - 2.35).abs() < f64::EPSILON);
        assert_eq!(easy.interval, 50);
        assert_eq!(easy.next_due_at, NOW + 50 * ONE_DAY_MS);
        assert!((easy.ease_factor - 2.65).abs() < f64::EPSILON);
    }

    #[test]
    fn suspended_cards_are_not_rescheduled() {
        let mut current = review_progress();
        current.state = CardState::Suspended;
        current.suspended_at = Some(NOW - 1);

        let next = schedule_review(Some(&current), "card", Rating::Good, &options(), NOW);

        assert_eq!(next, current);
    }
}
