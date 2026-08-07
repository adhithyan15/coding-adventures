use std::collections::{HashMap, HashSet};

use crate::model::{
    AppState, Card, CardProgress, CardState, DailyStudyLimitUsage, Deck, DeckOptions, DeckStats,
    ExternalSourceRecord, ExternalSourceTarget, Note,
};
use crate::sm2::{INITIAL_EASE_FACTOR, ONE_DAY_MS};

pub const SESSION_SIZE: usize = 20;
pub const MAX_NEW_PER_SESSION: usize = 7;
const ANKI_TYPE_NEW: i64 = 0;
const ANKI_QUEUE_SCHED_BURIED: i64 = -3;
const ANKI_QUEUE_USER_BURIED: i64 = -2;
const ANKI_QUEUE_SUSPENDED: i64 = -1;
const ANKI_QUEUE_NEW: i64 = 0;
const ANKI_QUEUE_LEARN: i64 = 1;
const ANKI_QUEUE_REVIEW: i64 = 2;
const ANKI_QUEUE_DAY_LEARN: i64 = 3;
const ANKI_QUEUE_PREVIEW_REPEAT: i64 = 4;

pub fn build_session_queue(
    all_cards: &[Card],
    all_progress: &[CardProgress],
    deck_id: &str,
    now: u64,
) -> Vec<Card> {
    let deck_ids = HashSet::from([deck_id]);
    build_session_queue_with_limits(
        all_cards,
        all_progress,
        &deck_ids,
        now,
        SESSION_SIZE,
        MAX_NEW_PER_SESSION,
        SESSION_SIZE,
        None,
        None,
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
    let deck_ids = HashSet::from([deck_id]);

    build_session_queue_with_limits(
        all_cards,
        all_progress,
        &deck_ids,
        now,
        session_size,
        max_new,
        max_reviews,
        None,
        None,
    )
}

pub fn build_session_queue_for_state_with_options(
    state: &AppState,
    deck_id: &str,
    now: u64,
    options: &DeckOptions,
) -> Vec<Card> {
    let max_new = options.new_cards_per_day as usize;
    let max_reviews = options.reviews_per_day as usize;
    let session_size = (max_new + max_reviews).max(1);
    let deck_ids = deck_ids_in_scope(state, deck_id);
    let new_card_positions = imported_new_card_positions(state);
    let imported_schedules = imported_anki_card_schedules(state);

    build_session_queue_with_limits(
        &state.cards,
        &state.card_progress,
        &deck_ids,
        now,
        session_size,
        max_new,
        max_reviews,
        Some(&new_card_positions),
        Some(&imported_schedules),
    )
}

pub fn deck_options_for_state(state: &AppState, deck_id: &str) -> DeckOptions {
    state
        .deck_options
        .iter()
        .find(|preset| preset.deck_id == deck_id)
        .map(|preset| preset.options.clone())
        .unwrap_or_default()
}

pub fn cards_in_deck_scope(state: &AppState, deck_id: &str) -> Vec<Card> {
    let deck_ids = deck_ids_in_scope(state, deck_id);
    state
        .cards
        .iter()
        .filter(|card| deck_ids.contains(card.deck_id.as_str()))
        .cloned()
        .collect()
}

pub fn notes_in_deck_scope(state: &AppState, deck_id: &str) -> Vec<Note> {
    let deck_ids = deck_ids_in_scope(state, deck_id);
    state
        .notes
        .iter()
        .filter(|note| deck_ids.contains(note.deck_id.as_str()))
        .cloned()
        .collect()
}

pub fn get_daily_study_limit_usage(
    state: &AppState,
    deck_id: &str,
    day_start: u64,
    day_end: u64,
    options: &DeckOptions,
) -> DailyStudyLimitUsage {
    let deck_ids = deck_ids_in_scope(state, deck_id);
    let deck_card_ids: HashSet<&str> = state
        .cards
        .iter()
        .filter(|card| deck_ids.contains(card.deck_id.as_str()))
        .map(|card| card.id.as_str())
        .collect();
    let mut new_card_ids: HashSet<&str> = HashSet::new();
    let mut review_cards_seen = 0;
    let imported_learning_review_ids = imported_anki_learning_review_ids(state);

    for review in &state.reviews {
        if review.reviewed_at < day_start || review.reviewed_at >= day_end {
            continue;
        }
        if !deck_card_ids.contains(review.card_id.as_str()) {
            continue;
        }

        if review.previous_progress.is_none()
            || imported_learning_review_ids.contains(review.id.as_str())
        {
            new_card_ids.insert(review.card_id.as_str());
        } else {
            review_cards_seen += 1;
        }
    }

    let new_cards_seen = new_card_ids.len();
    DailyStudyLimitUsage {
        deck_id: deck_id.to_string(),
        day_start,
        day_end,
        new_cards_seen,
        review_cards_seen,
        remaining_new_cards: (options.new_cards_per_day as usize).saturating_sub(new_cards_seen),
        remaining_reviews: (options.reviews_per_day as usize).saturating_sub(review_cards_seen),
    }
}

fn imported_anki_learning_review_ids(state: &AppState) -> HashSet<&str> {
    state
        .external_sources
        .iter()
        .filter(|source| source.target == ExternalSourceTarget::Review)
        .filter(|source| {
            source
                .data
                .get("kind")
                .and_then(|kind| kind.parse::<i64>().ok())
                == Some(0)
        })
        .map(|source| source.target_id.as_str())
        .collect()
}

pub fn build_session_queue_with_daily_limits(
    state: &AppState,
    deck_id: &str,
    now: u64,
    day_start: u64,
    day_end: u64,
    options: &DeckOptions,
) -> Vec<Card> {
    let usage = get_daily_study_limit_usage(state, deck_id, day_start, day_end, options);
    let session_size = (usage.remaining_new_cards + usage.remaining_reviews).max(1);
    let deck_ids = deck_ids_in_scope(state, deck_id);
    let new_card_positions = imported_new_card_positions(state);
    let imported_schedules = imported_anki_card_schedules(state);

    build_session_queue_with_limits(
        &state.cards,
        &state.card_progress,
        &deck_ids,
        now,
        session_size,
        usage.remaining_new_cards,
        usage.remaining_reviews,
        Some(&new_card_positions),
        Some(&imported_schedules),
    )
}

// Session-queue construction genuinely depends on many independent inputs
// (cards, progress, deck filter, limits, positions, schedules); bundling them
// into a params struct would add churn without improving clarity.
#[allow(clippy::too_many_arguments)]
fn build_session_queue_with_limits(
    all_cards: &[Card],
    all_progress: &[CardProgress],
    deck_ids: &HashSet<&str>,
    now: u64,
    session_size: usize,
    max_new: usize,
    max_reviews: usize,
    new_card_positions: Option<&HashMap<&str, i64>>,
    imported_schedules: Option<&HashMap<&str, ImportedAnkiSchedule>>,
) -> Vec<Card> {
    let progress_by_card: HashMap<&str, &CardProgress> = all_progress
        .iter()
        .map(|progress| (progress.card_id.as_str(), progress))
        .collect();

    let mut due_cards: Vec<Card> = all_cards
        .iter()
        .filter(|card| deck_ids.contains(card.deck_id.as_str()))
        .filter(|card| {
            let progress = progress_by_card.get(card.id.as_str()).copied();
            let imported = imported_schedules.and_then(|schedules| schedules.get(card.id.as_str()));
            effective_is_reviewable(progress, imported, now)
        })
        .cloned()
        .collect();

    due_cards.sort_by_key(|card| {
        let progress = progress_by_card.get(card.id.as_str()).copied();
        let imported = imported_schedules.and_then(|schedules| schedules.get(card.id.as_str()));
        effective_next_due_at(progress, imported).unwrap_or(u64::MAX)
    });

    let mut new_cards: Vec<(usize, Card)> = all_cards
        .iter()
        .enumerate()
        .filter(|(_, card)| deck_ids.contains(card.deck_id.as_str()))
        .filter(|(_, card)| {
            let progress = progress_by_card.get(card.id.as_str()).copied();
            let imported = imported_schedules.and_then(|schedules| schedules.get(card.id.as_str()));
            effective_is_new(progress, imported)
        })
        .map(|(index, card)| (index, card.clone()))
        .collect();
    new_cards.sort_by_key(|(index, card)| {
        (
            new_card_positions
                .and_then(|positions| positions.get(card.id.as_str()).copied())
                .unwrap_or(i64::MAX),
            *index,
        )
    });
    let new_cards: Vec<Card> = new_cards
        .into_iter()
        .take(max_new)
        .map(|(_, card)| card)
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

fn effective_is_reviewable(
    progress: Option<&CardProgress>,
    imported: Option<&ImportedAnkiSchedule>,
    now: u64,
) -> bool {
    if progress.is_some_and(|progress| !is_new_progress_overlay(progress)) {
        return progress.is_some_and(|progress| is_reviewable(progress, now));
    }

    imported.is_some_and(|schedule| schedule.is_reviewable(now))
}

fn effective_next_due_at(
    progress: Option<&CardProgress>,
    imported: Option<&ImportedAnkiSchedule>,
) -> Option<u64> {
    if let Some(progress) = progress.filter(|progress| !is_new_progress_overlay(progress)) {
        return Some(progress.next_due_at);
    }

    imported.and_then(|schedule| schedule.due_at)
}

fn effective_is_new(
    progress: Option<&CardProgress>,
    imported: Option<&ImportedAnkiSchedule>,
) -> bool {
    if progress.is_some_and(|progress| !is_new_progress_overlay(progress)) {
        return false;
    }

    imported.is_none_or(ImportedAnkiSchedule::is_new)
}

fn imported_new_card_positions(state: &AppState) -> HashMap<&str, i64> {
    state
        .external_sources
        .iter()
        .filter(|source| source.target == ExternalSourceTarget::Card)
        .filter_map(|source| {
            imported_new_card_position(source).map(|position| (source.target_id.as_str(), position))
        })
        .collect()
}

fn imported_new_card_position(source: &ExternalSourceRecord) -> Option<i64> {
    if source_data_i64(source, "kind").is_some_and(|kind| kind != 0)
        || source_data_i64(source, "queue").is_some_and(|queue| queue != 0)
    {
        return None;
    }

    source_data_i64(source, "due")
}

fn source_data_i64(source: &ExternalSourceRecord, key: &str) -> Option<i64> {
    source.data.get(key)?.parse().ok()
}

fn collection_created_at_days(state: &AppState) -> Option<i64> {
    state
        .external_sources
        .iter()
        .find(|source| source.target == ExternalSourceTarget::Collection)
        .and_then(|source| source_data_i64(source, "createdAtDays"))
}

fn imported_anki_card_schedules(state: &AppState) -> HashMap<&str, ImportedAnkiSchedule> {
    let collection_created_at_days = collection_created_at_days(state);
    state
        .external_sources
        .iter()
        .filter(|source| source.target == ExternalSourceTarget::Card)
        .filter(|source| source.source == "anki-v11")
        .filter_map(|source| {
            ImportedAnkiSchedule::from_source(source, collection_created_at_days)
                .map(|schedule| (source.target_id.as_str(), schedule))
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ImportedAnkiSchedule {
    kind: i64,
    queue: i64,
    interval: u32,
    ease_factor: f64,
    due_at: Option<u64>,
}

impl ImportedAnkiSchedule {
    fn from_source(
        source: &ExternalSourceRecord,
        collection_created_at_days: Option<i64>,
    ) -> Option<Self> {
        let kind = source_data_i64(source, "kind")?;
        let queue = source_data_i64(source, "queue")?;
        let due = source_data_i64(source, "originalDue")
            .filter(|due| *due != 0)
            .or_else(|| source_data_i64(source, "due"));
        let due_at = imported_anki_due_at(queue, due, collection_created_at_days);
        let interval = source_data_i64(source, "interval")
            .and_then(|interval| u32::try_from(interval.max(0)).ok())
            .unwrap_or_default();
        let ease_factor = source_data_i64(source, "factor")
            .filter(|factor| *factor > 0)
            .map(|factor| factor as f64 / 1000.0)
            .unwrap_or(INITIAL_EASE_FACTOR);

        Some(Self {
            kind,
            queue,
            interval,
            ease_factor,
            due_at,
        })
    }

    fn is_new(&self) -> bool {
        self.kind == ANKI_TYPE_NEW && self.queue == ANKI_QUEUE_NEW
    }

    fn is_suspended(&self) -> bool {
        self.queue == ANKI_QUEUE_SUSPENDED
    }

    fn is_currently_buried(&self, now: u64) -> bool {
        matches!(self.queue, ANKI_QUEUE_USER_BURIED | ANKI_QUEUE_SCHED_BURIED)
            && self.due_at.is_none_or(|due_at| due_at > now)
    }

    fn is_reviewable(&self, now: u64) -> bool {
        if self.is_new() || self.is_suspended() || self.is_currently_buried(now) {
            return false;
        }

        matches!(
            self.queue,
            ANKI_QUEUE_LEARN
                | ANKI_QUEUE_REVIEW
                | ANKI_QUEUE_DAY_LEARN
                | ANKI_QUEUE_PREVIEW_REPEAT
                | ANKI_QUEUE_USER_BURIED
                | ANKI_QUEUE_SCHED_BURIED
        ) && self.due_at.is_some_and(|due_at| due_at <= now)
    }
}

fn imported_anki_due_at(
    queue: i64,
    due: Option<i64>,
    collection_created_at_days: Option<i64>,
) -> Option<u64> {
    let due = due?;
    match queue {
        ANKI_QUEUE_LEARN | ANKI_QUEUE_PREVIEW_REPEAT => u64::try_from(due).ok()?.checked_mul(1000),
        ANKI_QUEUE_REVIEW
        | ANKI_QUEUE_DAY_LEARN
        | ANKI_QUEUE_USER_BURIED
        | ANKI_QUEUE_SCHED_BURIED
        | ANKI_QUEUE_SUSPENDED => {
            let day = collection_created_at_days?.saturating_add(due);
            u64::try_from(day).ok()?.checked_mul(ONE_DAY_MS)
        }
        _ => None,
    }
}

pub(crate) fn deck_ids_in_scope<'a>(state: &'a AppState, deck_id: &'a str) -> HashSet<&'a str> {
    let mut deck_ids = HashSet::from([deck_id]);
    let Some(selected_deck) = state.decks.iter().find(|deck| deck.id == deck_id) else {
        return deck_ids;
    };
    let Some(descendant_prefix) = deck_descendant_prefix(selected_deck) else {
        return deck_ids;
    };

    for deck in &state.decks {
        if deck.id != selected_deck.id && deck.name.starts_with(descendant_prefix.as_str()) {
            deck_ids.insert(deck.id.as_str());
        }
    }
    deck_ids
}

fn deck_descendant_prefix(deck: &Deck) -> Option<String> {
    (!deck.name.is_empty()).then(|| format!("{}::", deck.name))
}

pub(crate) fn is_reviewable(progress: &CardProgress, now: u64) -> bool {
    if is_new_progress_overlay(progress) {
        return false;
    }
    if is_suspended(progress) || is_currently_buried(progress, now) {
        return false;
    }

    matches!(
        progress.state,
        CardState::Learning | CardState::Review | CardState::Relearning | CardState::Buried
    ) && progress.next_due_at <= now
}

pub(crate) fn is_new_progress_overlay(progress: &CardProgress) -> bool {
    progress.state == CardState::Review
        && progress.interval == 0
        && progress.learning_step_index.is_none()
        && progress.buried_until.is_none()
        && progress.suspended_at.is_none()
        && progress.times_seen == 0
        && progress.times_correct == 0
        && progress.times_incorrect == 0
}

fn is_suspended(progress: &CardProgress) -> bool {
    progress.suspended_at.is_some() || progress.state == CardState::Suspended
}

fn is_currently_buried(progress: &CardProgress, now: u64) -> bool {
    if progress
        .buried_until
        .is_some_and(|buried_until| buried_until > now)
    {
        return true;
    }

    progress.state == CardState::Buried
        && progress
            .buried_until.is_none_or(|buried_until| buried_until > now)
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
    let deck_ids = HashSet::from([deck_id]);
    get_deck_stats_for_deck_ids(all_cards, all_progress, &deck_ids, now, None)
}

pub fn get_deck_stats_for_state(state: &AppState, deck_id: &str, now: u64) -> DeckStats {
    let deck_ids = deck_ids_in_scope(state, deck_id);
    let imported_schedules = imported_anki_card_schedules(state);
    get_deck_stats_for_deck_ids(
        &state.cards,
        &state.card_progress,
        &deck_ids,
        now,
        Some(&imported_schedules),
    )
}

fn get_deck_stats_for_deck_ids(
    all_cards: &[Card],
    all_progress: &[CardProgress],
    deck_ids: &HashSet<&str>,
    now: u64,
    imported_schedules: Option<&HashMap<&str, ImportedAnkiSchedule>>,
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
        suspended_count: 0,
        buried_count: 0,
        average_ease_factor: 0.0,
    };
    let mut ease_sum = 0.0;
    let mut ease_count = 0_u32;

    for card in all_cards
        .iter()
        .filter(|card| deck_ids.contains(card.deck_id.as_str()))
    {
        stats.total += 1;
        let progress = progress_by_card.get(card.id.as_str()).copied();
        let imported = imported_schedules.and_then(|schedules| schedules.get(card.id.as_str()));
        match progress {
            Some(progress) => {
                if is_new_progress_overlay(progress)
                    && imported.is_none_or(|schedule| schedule.is_new())
                {
                    stats.new_count += 1;
                    continue;
                }
                if is_new_progress_overlay(progress) {
                    let Some(imported) = imported else {
                        stats.new_count += 1;
                        continue;
                    };
                    record_imported_schedule_stats(
                        imported,
                        now,
                        &mut stats,
                        &mut ease_sum,
                        &mut ease_count,
                    );
                    continue;
                }
                if is_suspended(progress) {
                    stats.suspended_count += 1;
                }
                if is_currently_buried(progress, now) {
                    stats.buried_count += 1;
                }
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
                if let Some(imported) = imported {
                    record_imported_schedule_stats(
                        imported,
                        now,
                        &mut stats,
                        &mut ease_sum,
                        &mut ease_count,
                    );
                    continue;
                }
                stats.new_count += 1;
            }
        }
    }

    if ease_count > 0 {
        stats.average_ease_factor = ease_sum / f64::from(ease_count);
    }

    stats
}

fn record_imported_schedule_stats(
    schedule: &ImportedAnkiSchedule,
    now: u64,
    stats: &mut DeckStats,
    ease_sum: &mut f64,
    ease_count: &mut u32,
) {
    if schedule.is_new() {
        stats.new_count += 1;
        return;
    }

    if schedule.is_suspended() {
        stats.suspended_count += 1;
    }
    if schedule.is_currently_buried(now) {
        stats.buried_count += 1;
    }
    if schedule.interval > 21 {
        stats.mastered_count += 1;
    } else {
        stats.learning_count += 1;
    }
    if schedule.is_reviewable(now) {
        stats.due_count += 1;
    }
    *ease_sum += schedule.ease_factor;
    *ease_count += 1;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Rating, Review};
    use std::collections::BTreeMap;

    const NOW: u64 = 1_700_000_000_000;

    fn card(id: &str, deck_id: &str, created_at: u64) -> Card {
        Card {
            id: id.to_string(),
            deck_id: deck_id.to_string(),
            front: format!("front {id}"),
            back: format!("back {id}"),
            created_at,
            lineage: None,
        }
    }

    fn deck(id: &str, name: &str) -> Deck {
        Deck {
            id: id.to_string(),
            name: name.to_string(),
            description: String::new(),
            created_at: NOW,
        }
    }

    fn note(id: &str, deck_id: &str) -> Note {
        Note {
            id: id.to_string(),
            note_type_id: "basic".to_string(),
            deck_id: deck_id.to_string(),
            fields: Vec::new(),
            tags: Vec::new(),
            created_at: NOW,
            updated_at: NOW,
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
            fsrs_stability: None,
            fsrs_difficulty: None,
            flag: None,
            marked_at: None,
        }
    }

    fn metadata_overlay(card_id: &str) -> CardProgress {
        CardProgress {
            card_id: card_id.to_string(),
            state: CardState::Review,
            interval: 0,
            ease_factor: 2.5,
            next_due_at: NOW,
            learning_step_index: None,
            buried_until: None,
            suspended_at: None,
            times_seen: 0,
            times_correct: 0,
            times_incorrect: 0,
            last_seen_at: NOW,
            fsrs_stability: None,
            fsrs_difficulty: None,
            flag: Some(crate::model::CardFlag::Red),
            marked_at: Some(NOW),
        }
    }

    fn review(
        id: &str,
        card_id: &str,
        reviewed_at: u64,
        previous_progress: Option<CardProgress>,
    ) -> Review {
        Review {
            id: id.to_string(),
            session_id: "session".to_string(),
            card_id: card_id.to_string(),
            rating: Rating::Good,
            reviewed_at,
            answer_time_ms: None,
            leech_event: None,
            previous_progress,
            resulting_progress: None,
            previous_active_session: None,
            sibling_progress_snapshots: Vec::new(),
        }
    }

    fn anki_card_source(card_id: &str, kind: i64, queue: i64, due: i64) -> ExternalSourceRecord {
        ExternalSourceRecord {
            target: ExternalSourceTarget::Card,
            target_id: card_id.to_string(),
            source: "anki-v11".to_string(),
            original_id: Some(card_id.to_string()),
            data: BTreeMap::from([
                ("kind".to_string(), kind.to_string()),
                ("queue".to_string(), queue.to_string()),
                ("due".to_string(), due.to_string()),
            ]),
        }
    }

    fn anki_card_source_with_metrics(
        card_id: &str,
        kind: i64,
        queue: i64,
        due: i64,
        interval: i64,
    ) -> ExternalSourceRecord {
        let mut source = anki_card_source(card_id, kind, queue, due);
        source
            .data
            .insert("interval".to_string(), interval.to_string());
        source.data.insert("factor".to_string(), "2500".to_string());
        source
    }

    fn anki_collection_source(created_at_days: i64) -> ExternalSourceRecord {
        ExternalSourceRecord {
            target: ExternalSourceTarget::Collection,
            target_id: "collection".to_string(),
            source: "anki-v11".to_string(),
            original_id: Some("1".to_string()),
            data: BTreeMap::from([("createdAtDays".to_string(), created_at_days.to_string())]),
        }
    }

    fn anki_review_source(review_id: &str, kind: i64) -> ExternalSourceRecord {
        ExternalSourceRecord {
            target: ExternalSourceTarget::Review,
            target_id: review_id.to_string(),
            source: "anki-v11".to_string(),
            original_id: Some(review_id.to_string()),
            data: BTreeMap::from([("kind".to_string(), kind.to_string())]),
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
    fn metadata_only_progress_overlay_still_queues_as_new() {
        let cards = vec![card("flagged-new", "deck", 1), card("plain-new", "deck", 2)];
        let queue = build_session_queue(&cards, &[metadata_overlay("flagged-new")], "deck", NOW);
        let ids: Vec<_> = queue.iter().map(|card| card.id.as_str()).collect();

        assert_eq!(ids, vec!["flagged-new", "plain-new"]);

        let stats = get_deck_stats(&cards, &[metadata_overlay("flagged-new")], "deck", NOW);
        assert_eq!(stats.new_count, 2);
        assert_eq!(stats.due_count, 0);
        assert_eq!(stats.learning_count, 0);
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
            card("suspended", "deck", 4),
            card("buried", "deck", 5),
            card("expired-buried", "deck", 6),
        ];
        let mut suspended = progress("suspended", NOW - 1, 3);
        suspended.suspended_at = Some(NOW - 10);
        let mut buried = progress("buried", NOW - 1, 3);
        buried.buried_until = Some(NOW + 1000);
        let mut expired_buried = progress("expired-buried", NOW - 1, 3);
        expired_buried.state = CardState::Buried;
        expired_buried.buried_until = Some(NOW - 1);
        let progress = vec![
            progress("learning", NOW - 1, 3),
            progress("mastered", NOW + 1000, 22),
            suspended,
            buried,
            expired_buried,
        ];

        let stats = get_deck_stats(&cards, &progress, "deck", NOW);

        assert_eq!(stats.total, 6);
        assert_eq!(stats.new_count, 1);
        assert_eq!(stats.learning_count, 4);
        assert_eq!(stats.mastered_count, 1);
        assert_eq!(stats.due_count, 2);
        assert_eq!(stats.suspended_count, 1);
        assert_eq!(stats.buried_count, 1);
        assert_eq!(stats.average_ease_factor, 2.5);
    }

    #[test]
    fn state_queues_stats_and_daily_limits_include_child_decks() {
        let state = AppState {
            decks: vec![
                deck("parent", "Tamil"),
                deck("child", "Tamil::Verbs"),
                deck("sibling", "Spanish"),
            ],
            cards: vec![
                card("parent-due", "parent", 1),
                card("child-due", "child", 2),
                card("child-new", "child", 3),
                card("sibling-due", "sibling", 4),
            ],
            notes: vec![
                note("parent-note", "parent"),
                note("child-note", "child"),
                note("sibling-note", "sibling"),
            ],
            card_progress: vec![
                progress("parent-due", NOW - 100, 3),
                progress("child-due", NOW - 50, 3),
                progress("sibling-due", NOW - 200, 3),
            ],
            reviews: vec![
                review("new-child", "child-new", NOW + 10, None),
                review(
                    "review-child",
                    "child-due",
                    NOW + 20,
                    Some(progress("child-due", NOW - 50, 3)),
                ),
                review(
                    "review-sibling",
                    "sibling-due",
                    NOW + 30,
                    Some(progress("sibling-due", NOW - 200, 3)),
                ),
            ],
            ..AppState::default()
        };
        let options = DeckOptions {
            new_cards_per_day: 2,
            reviews_per_day: 3,
            ..DeckOptions::default()
        };

        let queue = build_session_queue_for_state_with_options(&state, "parent", NOW, &options);
        let ids: Vec<_> = queue.iter().map(|card| card.id.as_str()).collect();
        assert_eq!(ids, vec!["parent-due", "child-due", "child-new"]);

        let scoped_card_ids: Vec<_> = cards_in_deck_scope(&state, "parent")
            .into_iter()
            .map(|card| card.id)
            .collect();
        assert_eq!(
            scoped_card_ids,
            vec![
                "parent-due".to_string(),
                "child-due".to_string(),
                "child-new".to_string()
            ]
        );
        let scoped_note_ids: Vec<_> = notes_in_deck_scope(&state, "parent")
            .into_iter()
            .map(|note| note.id)
            .collect();
        assert_eq!(
            scoped_note_ids,
            vec!["parent-note".to_string(), "child-note".to_string()]
        );

        let stats = get_deck_stats_for_state(&state, "parent", NOW);
        assert_eq!(stats.total, 3);
        assert_eq!(stats.new_count, 1);
        assert_eq!(stats.due_count, 2);

        let usage = get_daily_study_limit_usage(&state, "parent", NOW, NOW + 100, &options);
        assert_eq!(usage.new_cards_seen, 1);
        assert_eq!(usage.review_cards_seen, 1);
        assert_eq!(usage.remaining_new_cards, 1);
        assert_eq!(usage.remaining_reviews, 2);
    }

    #[test]
    fn state_queue_sorts_imported_new_cards_by_anki_position() {
        let state = AppState {
            decks: vec![deck("deck", "Tamil")],
            cards: vec![
                card("native-new", "deck", 1),
                card("anki-late", "deck", 2),
                card("due", "deck", 3),
                card("anki-early", "deck", 4),
                card("review-source", "deck", 5),
            ],
            card_progress: vec![progress("due", NOW - 100, 3)],
            external_sources: vec![
                anki_card_source("anki-late", 0, 0, 200),
                anki_card_source("anki-early", 0, 0, 25),
                anki_card_source("review-source", 2, 2, 5),
            ],
            ..AppState::default()
        };
        let options = DeckOptions {
            new_cards_per_day: 3,
            reviews_per_day: 1,
            ..DeckOptions::default()
        };

        let queue = build_session_queue_for_state_with_options(&state, "deck", NOW, &options);
        let ids: Vec<_> = queue.iter().map(|card| card.id.as_str()).collect();

        assert_eq!(ids, vec!["due", "anki-early", "anki-late", "native-new"]);
    }

    #[test]
    fn state_queue_and_stats_use_imported_anki_schedules_without_native_progress() {
        let today = 200_i64;
        let created_at_days = (NOW / ONE_DAY_MS) as i64 - today;
        let now_secs = (NOW / 1000) as i64;
        let state = AppState {
            decks: vec![deck("deck", "Tamil")],
            cards: vec![
                card("native-new", "deck", 1),
                card("review-due", "deck", 2),
                card("review-future", "deck", 3),
                card("learning-due", "deck", 4),
                card("suspended", "deck", 5),
                card("buried", "deck", 6),
            ],
            external_sources: vec![
                anki_collection_source(created_at_days),
                anki_card_source_with_metrics("review-due", 2, 2, today, 7),
                anki_card_source_with_metrics("review-future", 2, 2, today + 3, 30),
                anki_card_source_with_metrics("learning-due", 1, 1, now_secs - 60, 0),
                anki_card_source_with_metrics("suspended", 2, -1, today, 0),
                anki_card_source_with_metrics("buried", 2, -2, today + 1, 0),
            ],
            ..AppState::default()
        };
        let options = DeckOptions {
            new_cards_per_day: 1,
            reviews_per_day: 4,
            ..DeckOptions::default()
        };

        let queue = build_session_queue_for_state_with_options(&state, "deck", NOW, &options);
        let ids: Vec<_> = queue.iter().map(|card| card.id.as_str()).collect();
        assert_eq!(ids, vec!["review-due", "learning-due", "native-new"]);

        let stats = get_deck_stats_for_state(&state, "deck", NOW);
        assert_eq!(stats.total, 6);
        assert_eq!(stats.new_count, 1);
        assert_eq!(stats.learning_count, 4);
        assert_eq!(stats.mastered_count, 1);
        assert_eq!(stats.due_count, 2);
        assert_eq!(stats.suspended_count, 1);
        assert_eq!(stats.buried_count, 1);
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

    #[test]
    fn daily_limit_usage_counts_deck_reviews_in_day_window() {
        let state = AppState {
            cards: vec![card("new-today", "deck", 1), card("reviewed", "deck", 2)],
            reviews: vec![
                review("new", "new-today", NOW + 10, None),
                review(
                    "review",
                    "reviewed",
                    NOW + 20,
                    Some(progress("reviewed", NOW - 100, 3)),
                ),
                review("before", "new-today", NOW - 1, None),
                review("other-deck", "other", NOW + 30, None),
            ],
            ..AppState::default()
        };
        let options = DeckOptions {
            new_cards_per_day: 3,
            reviews_per_day: 2,
            ..DeckOptions::default()
        };

        let usage = get_daily_study_limit_usage(&state, "deck", NOW, NOW + 100, &options);

        assert_eq!(usage.deck_id, "deck");
        assert_eq!(usage.new_cards_seen, 1);
        assert_eq!(usage.review_cards_seen, 1);
        assert_eq!(usage.remaining_new_cards, 2);
        assert_eq!(usage.remaining_reviews, 1);
    }

    #[test]
    fn daily_limit_usage_counts_imported_anki_learning_revlogs_as_new() {
        let state = AppState {
            cards: vec![
                card("imported-learning", "deck", 1),
                card("imported-review", "deck", 2),
            ],
            reviews: vec![
                review(
                    "learn-row",
                    "imported-learning",
                    NOW + 10,
                    Some(progress("imported-learning", NOW - 100, 0)),
                ),
                review(
                    "review-row",
                    "imported-review",
                    NOW + 20,
                    Some(progress("imported-review", NOW - 100, 3)),
                ),
            ],
            external_sources: vec![
                anki_review_source("learn-row", 0),
                anki_review_source("review-row", 1),
            ],
            ..AppState::default()
        };
        let options = DeckOptions {
            new_cards_per_day: 3,
            reviews_per_day: 2,
            ..DeckOptions::default()
        };

        let usage = get_daily_study_limit_usage(&state, "deck", NOW, NOW + 100, &options);

        assert_eq!(usage.new_cards_seen, 1);
        assert_eq!(usage.review_cards_seen, 1);
        assert_eq!(usage.remaining_new_cards, 2);
        assert_eq!(usage.remaining_reviews, 1);
    }

    #[test]
    fn daily_limit_queue_subtracts_reviews_already_seen_today() {
        let mut new_already_seen = progress("new-1", NOW + 60_000, 0);
        new_already_seen.times_seen = 1;
        let state = AppState {
            cards: vec![
                card("due-1", "deck", 1),
                card("reviewed-today", "deck", 2),
                card("new-1", "deck", 3),
                card("new-2", "deck", 4),
                card("new-3", "deck", 5),
            ],
            card_progress: vec![
                progress("due-1", NOW - 100, 3),
                progress("reviewed-today", NOW + 60_000, 3),
                new_already_seen,
            ],
            reviews: vec![
                review("new", "new-1", NOW + 10, None),
                review(
                    "review",
                    "reviewed-today",
                    NOW + 20,
                    Some(progress("reviewed-today", NOW - 100, 3)),
                ),
            ],
            ..AppState::default()
        };
        let options = DeckOptions {
            new_cards_per_day: 2,
            reviews_per_day: 2,
            ..DeckOptions::default()
        };

        let queue =
            build_session_queue_with_daily_limits(&state, "deck", NOW, NOW, NOW + 100, &options);
        let ids: Vec<_> = queue.iter().map(|card| card.id.as_str()).collect();

        assert_eq!(ids, vec!["due-1", "new-2"]);
    }
}
