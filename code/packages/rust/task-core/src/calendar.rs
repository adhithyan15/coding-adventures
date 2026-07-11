//! The working-time engine.
//!
//! A project schedule does not run in wall-clock time: "one working day" of effort
//! skips weekends and holidays, and an eight-hour task started at 15:00 on a
//! nine-to-five Friday does not finish until Monday. Everything the CPM scheduler
//! does — placing a task, honouring lag, measuring slack — is expressed in **working
//! minutes** resolved against a [`Calendar`]. This module is that resolver.
//!
//! ## Instants
//!
//! We work in **instants**: integer minutes since the Unix epoch, computed as
//! `date_days × 1440 + minute_of_day`. Instants make interval arithmetic clean and
//! half-open — a task occupies `[start, finish)` and "finish-to-start" is simply
//! `successor.start ≥ predecessor.finish`, with none of the inclusive-date
//! off-by-one bookkeeping that day-granularity scheduling suffers from.
//!
//! ## Calendar resolution
//!
//! For a given date we resolve the applicable [`DaySchedule`] as: a matching
//! [`CalendarException`] (a holiday or overtime day) if one exists, otherwise the
//! weekly pattern entry for that weekday. If the referenced calendar is *absent*
//! entirely we fall back to treating all time as working (24/7) — a safe default
//! that never hangs the walk. (Base-calendar inheritance and resource calendars are
//! layered in a follow-up; today the scheduler resolves task-or-project calendars.)

use crate::ids::CalendarId;
use crate::model::{Calendar, DaySchedule, ProjectState};
use crate::primitives::{Date, Duration};

/// Minutes in a day.
const MINUTES_PER_DAY: i64 = 1440;

/// Safety bound on how many days a working-time walk may traverse before giving up
/// (≈50 years). Prevents an infinite loop if a calendar defines no working time.
const MAX_DAYS_WALK: i64 = 366 * 50;

/// An instant: integer minutes since the Unix epoch.
pub type Instant = i64;

/// Compose an instant from a date and a minute-of-day offset.
pub fn instant_of(date: Date, minute_of_day: i64) -> Instant {
    date.0 as i64 * MINUTES_PER_DAY + minute_of_day
}

/// The calendar date an instant falls on (floor division, so it is correct for
/// pre-epoch negative instants too).
pub fn date_of(inst: Instant) -> Date {
    Date(inst.div_euclid(MINUTES_PER_DAY) as i32)
}

/// The minute-of-day (0..1440) of an instant.
pub fn minute_of_day(inst: Instant) -> i64 {
    inst.rem_euclid(MINUTES_PER_DAY)
}

/// The calendar to use, falling back to the project calendar when `cal` is missing.
/// Returns `None` only when neither exists — the caller then treats time as 24/7.
fn resolve_calendar<'a>(project: &'a ProjectState, cal: &CalendarId) -> Option<&'a Calendar> {
    project
        .calendars
        .get(cal)
        .or_else(|| project.calendars.get(&project.project_calendar))
}

/// The [`DaySchedule`] that applies to `date` on `cal`: an exception if one matches,
/// otherwise the weekday's entry in the weekly pattern.
fn day_schedule(cal: &Calendar, date: Date) -> &DaySchedule {
    for ex in &cal.exceptions {
        if ex.date == date {
            return &ex.schedule;
        }
    }
    // `weekday()` is ISO: Monday = 1 … Sunday = 7; the pattern is indexed from 0.
    let idx = (date.weekday() - 1) as usize;
    &cal.work_week[idx]
}

/// Whether `date` is a working day on the calendar named by `cal`.
pub fn is_working_day(project: &ProjectState, cal: &CalendarId, date: Date) -> bool {
    match resolve_calendar(project, cal) {
        None => true, // no calendar ⇒ every day works
        Some(c) => {
            let sched = day_schedule(c, date);
            sched.working && !sched.intervals.is_empty()
        }
    }
}

/// The first working instant at or after `start`. Returns `start` unchanged if it is
/// already within a working interval. Returns `None` only if no working time is found
/// within [`MAX_DAYS_WALK`] days (a calendar with no working time at all).
pub fn next_working(project: &ProjectState, cal: &CalendarId, start: Instant) -> Option<Instant> {
    let Some(c) = resolve_calendar(project, cal) else {
        return Some(start); // 24/7
    };
    let mut date = date_of(start);
    let mut from_min = minute_of_day(start);
    for _ in 0..MAX_DAYS_WALK {
        let sched = day_schedule(c, date);
        if sched.working {
            for iv in &sched.intervals {
                let end = iv.end_min as i64;
                if end > from_min {
                    let s = from_min.max(iv.start_min as i64);
                    return Some(instant_of(date, s));
                }
            }
        }
        // Nothing left today ⇒ jump to the start of the next day.
        date = date.add_days(1);
        from_min = 0;
    }
    None
}

/// Advance `start` by a [`Duration`], returning the finish instant.
///
/// - An **elapsed** duration is raw wall-clock: `start + minutes`, ignoring the
///   calendar (e.g. "wait 24h for paint to dry").
/// - A **working** duration consumes `minutes` of working time, snapping the start
///   forward into working time first and skipping every non-working interval.
/// - A **zero** duration returns `start` unchanged (a milestone is a point in time).
pub fn add_working(
    project: &ProjectState,
    cal: &CalendarId,
    start: Instant,
    dur: Duration,
) -> Instant {
    if dur.elapsed {
        return start.saturating_add(dur.working_minutes);
    }
    if dur.working_minutes <= 0 {
        return start;
    }
    let Some(c) = resolve_calendar(project, cal) else {
        return start.saturating_add(dur.working_minutes); // 24/7
    };

    let mut remaining = dur.working_minutes;
    // Snap into working time before consuming.
    let snapped = match next_working(project, cal, start) {
        Some(i) => i,
        None => return start.saturating_add(dur.working_minutes), // degenerate calendar
    };
    let mut date = date_of(snapped);
    let mut from_min = minute_of_day(snapped);

    for _ in 0..MAX_DAYS_WALK {
        let sched = day_schedule(c, date);
        if sched.working {
            for iv in &sched.intervals {
                let end = iv.end_min as i64;
                let seg_start = from_min.max(iv.start_min as i64);
                if end > seg_start {
                    let avail = end - seg_start;
                    if remaining <= avail {
                        return instant_of(date, seg_start + remaining);
                    }
                    remaining -= avail;
                }
            }
        }
        date = date.add_days(1);
        from_min = 0;
    }
    // Degenerate calendar: best effort.
    instant_of(date, 0)
}

/// Count the working minutes in the half-open interval `[a, b)`. Zero if `b <= a`.
/// Used for slack (`total_slack = working_between(early_start, late_start)`).
pub fn working_between(project: &ProjectState, cal: &CalendarId, a: Instant, b: Instant) -> i64 {
    if b <= a {
        return 0;
    }
    let Some(c) = resolve_calendar(project, cal) else {
        return b - a; // 24/7
    };
    let mut total = 0;
    let mut date = date_of(a);
    let last_date = date_of(b - 1);
    let mut guard = 0;
    while date <= last_date && guard < MAX_DAYS_WALK {
        let sched = day_schedule(c, date);
        if sched.working {
            let day_start = instant_of(date, 0);
            for iv in &sched.intervals {
                let iv_lo = day_start + iv.start_min as i64;
                let iv_hi = day_start + iv.end_min as i64;
                let lo = iv_lo.max(a);
                let hi = iv_hi.min(b);
                if hi > lo {
                    total += hi - lo;
                }
            }
        }
        date = date.add_days(1);
        guard += 1;
    }
    total
}

/// The inverse of [`add_working`]: the instant `start` such that consuming `dur`
/// working minutes forward from it lands exactly on `end`. Equivalently, walk
/// *backward* from `end`, consuming working time. Used by the backward pass
/// (`late_start = sub_working(late_finish, duration)`), by finish-anchored
/// constraints (`MustFinishOn`), and by finish-to-finish / start-to-finish links.
pub fn sub_working(
    project: &ProjectState,
    cal: &CalendarId,
    end: Instant,
    dur: Duration,
) -> Instant {
    if dur.elapsed {
        return end.saturating_sub(dur.working_minutes);
    }
    if dur.working_minutes <= 0 {
        return end;
    }
    let Some(c) = resolve_calendar(project, cal) else {
        return end.saturating_sub(dur.working_minutes); // 24/7
    };

    let mut remaining = dur.working_minutes;
    let mut date = date_of(end);
    // Upper bound (exclusive) within the current day; the whole day on earlier days.
    let mut to_min = minute_of_day(end);
    for _ in 0..MAX_DAYS_WALK {
        let sched = day_schedule(c, date);
        if sched.working {
            // Consume intervals from latest to earliest.
            for iv in sched.intervals.iter().rev() {
                let lo = iv.start_min as i64;
                let hi = (iv.end_min as i64).min(to_min);
                if hi > lo {
                    let avail = hi - lo;
                    if remaining <= avail {
                        return instant_of(date, hi - remaining);
                    }
                    remaining -= avail;
                }
            }
        }
        date = date.add_days(-1);
        to_min = MINUTES_PER_DAY;
    }
    // Degenerate calendar: best effort.
    instant_of(date, MINUTES_PER_DAY)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{CalendarId, ProjectId};

    fn project_with_standard_calendar() -> (ProjectState, CalendarId) {
        let p = ProjectState::empty(ProjectId::from_raw("p1"));
        let cal = p.project_calendar.clone();
        (p, cal)
    }

    #[test]
    fn instant_round_trips_through_date_and_minute() {
        let d = Date::from_ymd(2026, 7, 10).unwrap();
        let inst = instant_of(d, 9 * 60);
        assert_eq!(date_of(inst), d);
        assert_eq!(minute_of_day(inst), 9 * 60);
    }

    #[test]
    fn standard_calendar_marks_weekends_non_working() {
        let (p, cal) = project_with_standard_calendar();
        // 2026-07-10 is Friday, 07-11 Saturday, 07-13 Monday.
        assert!(is_working_day(
            &p,
            &cal,
            Date::from_ymd(2026, 7, 10).unwrap()
        ));
        assert!(!is_working_day(
            &p,
            &cal,
            Date::from_ymd(2026, 7, 11).unwrap()
        ));
        assert!(!is_working_day(
            &p,
            &cal,
            Date::from_ymd(2026, 7, 12).unwrap()
        ));
        assert!(is_working_day(
            &p,
            &cal,
            Date::from_ymd(2026, 7, 13).unwrap()
        ));
    }

    #[test]
    fn next_working_snaps_out_of_the_weekend_and_off_hours() {
        let (p, cal) = project_with_standard_calendar();
        // Saturday 10:00 → Monday 09:00.
        let sat = instant_of(Date::from_ymd(2026, 7, 11).unwrap(), 10 * 60);
        let got = next_working(&p, &cal, sat).unwrap();
        assert_eq!(date_of(got), Date::from_ymd(2026, 7, 13).unwrap());
        assert_eq!(minute_of_day(got), 9 * 60);

        // Friday 07:00 (before the workday) → Friday 09:00.
        let early = instant_of(Date::from_ymd(2026, 7, 10).unwrap(), 7 * 60);
        let got = next_working(&p, &cal, early).unwrap();
        assert_eq!(minute_of_day(got), 9 * 60);

        // Friday 18:00 (after the workday) → Monday 09:00.
        let late = instant_of(Date::from_ymd(2026, 7, 10).unwrap(), 18 * 60);
        let got = next_working(&p, &cal, late).unwrap();
        assert_eq!(date_of(got), Date::from_ymd(2026, 7, 13).unwrap());
    }

    #[test]
    fn add_working_consumes_a_full_eight_hour_day() {
        let (p, cal) = project_with_standard_calendar();
        // A one-day (8h) task starting Friday 09:00 finishes Friday 17:00.
        let fri_9 = instant_of(Date::from_ymd(2026, 7, 10).unwrap(), 9 * 60);
        let finish = add_working(&p, &cal, fri_9, Duration::minutes(8 * 60));
        assert_eq!(date_of(finish), Date::from_ymd(2026, 7, 10).unwrap());
        assert_eq!(minute_of_day(finish), 17 * 60);
    }

    #[test]
    fn add_working_spills_across_the_weekend() {
        let (p, cal) = project_with_standard_calendar();
        // 12 working hours from Friday 09:00: 8h fills Friday, the remaining 4h lands
        // Monday 09:00–13:00.
        let fri_9 = instant_of(Date::from_ymd(2026, 7, 10).unwrap(), 9 * 60);
        let finish = add_working(&p, &cal, fri_9, Duration::minutes(12 * 60));
        assert_eq!(date_of(finish), Date::from_ymd(2026, 7, 13).unwrap());
        assert_eq!(minute_of_day(finish), 13 * 60);
    }

    #[test]
    fn elapsed_duration_ignores_the_calendar() {
        let (p, cal) = project_with_standard_calendar();
        let fri_9 = instant_of(Date::from_ymd(2026, 7, 10).unwrap(), 9 * 60);
        let finish = add_working(
            &p,
            &cal,
            fri_9,
            Duration {
                working_minutes: MINUTES_PER_DAY,
                elapsed: true,
            },
        );
        // Exactly 24h later, weekend or not.
        assert_eq!(finish, fri_9 + MINUTES_PER_DAY);
    }

    #[test]
    fn add_working_saturates_instead_of_overflowing() {
        // A hostile project (e.g. from untrusted JSON) with an enormous duration must
        // not panic (debug) or wrap to a negative finish (release) — it saturates.
        let (p, cal) = project_with_standard_calendar();
        let start = instant_of(Date::from_ymd(2026, 7, 10).unwrap(), 9 * 60);
        let huge = Duration {
            working_minutes: i64::MAX,
            elapsed: true,
        };
        assert_eq!(add_working(&p, &cal, start, huge), i64::MAX);
    }

    #[test]
    fn sub_working_inverts_add_working_across_the_weekend() {
        let (p, cal) = project_with_standard_calendar();
        let fri_9 = instant_of(Date::from_ymd(2026, 7, 10).unwrap(), 9 * 60);
        // 12 working hours forward from Friday 09:00 → Monday 13:00.
        let mon_13 = add_working(&p, &cal, fri_9, Duration::minutes(12 * 60));
        // …and back again lands exactly on Friday 09:00.
        assert_eq!(
            sub_working(&p, &cal, mon_13, Duration::minutes(12 * 60)),
            fri_9
        );
        // A single 8h day: Friday 17:00 back 8h → Friday 09:00.
        let fri_17 = instant_of(Date::from_ymd(2026, 7, 10).unwrap(), 17 * 60);
        assert_eq!(
            sub_working(&p, &cal, fri_17, Duration::minutes(8 * 60)),
            fri_9
        );
    }

    #[test]
    fn zero_duration_is_a_point() {
        let (p, cal) = project_with_standard_calendar();
        let fri_9 = instant_of(Date::from_ymd(2026, 7, 10).unwrap(), 9 * 60);
        assert_eq!(add_working(&p, &cal, fri_9, Duration::zero()), fri_9);
    }

    #[test]
    fn working_between_counts_only_working_minutes() {
        let (p, cal) = project_with_standard_calendar();
        // Friday 09:00 to Monday 13:00 = 8h (Fri) + 4h (Mon) = 12h of working time,
        // the weekend contributing nothing.
        let a = instant_of(Date::from_ymd(2026, 7, 10).unwrap(), 9 * 60);
        let b = instant_of(Date::from_ymd(2026, 7, 13).unwrap(), 13 * 60);
        assert_eq!(working_between(&p, &cal, a, b), 12 * 60);
        assert_eq!(
            working_between(&p, &cal, b, a),
            0,
            "reversed interval is empty"
        );
    }

    #[test]
    fn holiday_exception_makes_a_weekday_non_working() {
        let mut p = ProjectState::empty(ProjectId::from_raw("p1"));
        let cal_id = p.project_calendar.clone();
        let holiday = Date::from_ymd(2026, 7, 10).unwrap(); // a Friday
        p.calendars
            .get_mut(&cal_id)
            .unwrap()
            .exceptions
            .push(crate::model::CalendarException {
                date: holiday,
                schedule: DaySchedule {
                    working: false,
                    intervals: Vec::new(),
                },
            });
        assert!(!is_working_day(&p, &cal_id, holiday));
        // A task that would have started Friday 09:00 now snaps to Monday.
        let fri_9 = instant_of(holiday, 9 * 60);
        let got = next_working(&p, &cal_id, fri_9).unwrap();
        assert_eq!(date_of(got), Date::from_ymd(2026, 7, 13).unwrap());
    }
}
