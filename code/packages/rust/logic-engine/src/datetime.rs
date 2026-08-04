//! # Date arithmetic — deadlines and durations (ADJ constraints, track A3).
//!
//! Adjudication is full of date questions: *is the claim within 365 days of
//! purchase? what is the deadline 30 days after notice?* The model extracts the
//! dates (`date(2025, 1, 15)`); the **engine** does the calendar arithmetic on
//! the CPU, deterministically, so the answer is auditable and never
//! wrong-by-off-by-one. A date is a *point in time*, not a magnitude, so it gets
//! the [`Dimension::Date`](crate::Dimension) dimension and its arithmetic lives
//! here, not in the generic [`Dimension::combine`](crate::Dimension::combine):
//!
//! - `days_between(a, b)` → a [`Duration("days")`](crate::Dimension::Duration)
//!   (so a deadline predicate `elapsed <= 365` fires over it like any value).
//! - `date_add(date, days)` → a new date (`Date + Duration → Date`).
//! - `before(a, b)` / `after(a, b)` → a boolean ordering.
//!
//! ## Why the calendar math is inlined (not `datetime-core`)
//!
//! The repo's `datetime-core` is the right calendar library, but it depends on
//! `numeric-tower` (big-integer/rational), `r-vector`, and `wall-clock` — a
//! heavy chain to pull into the *core* reasoning engine for what is ~25 lines of
//! pure, branch-free integer math. We therefore inline Howard Hinnant's
//! public-domain `days_from_civil` / `civil_from_days` (the same algorithm
//! `datetime-core` uses), keeping `logic-engine` lean and dependency-free. The
//! algorithm is exact for all proleptic-Gregorian dates.
//! <http://howardhinnant.github.io/date_algorithms.html>

use crate::dimension::{Dimension, Dimensioned};
use logic_core::{Number, Term};

/// The largest absolute day-ordinal we support — the ordinal of a date in the
/// `±1_000_000`-year window `read_date` enforces, rounded up. Used to bound
/// arithmetic so the internal multiplications cannot overflow i64.
pub(crate) const MAX_ORDINAL: i64 = 366_000_000;

/// Days since the Unix epoch (1970-01-01) for a proleptic-Gregorian date.
/// Howard Hinnant's algorithm; exact for any `y`/`m`/`d`. **Internal**: callers
/// must pass a year within the `read_date` bound (`±1_000_000`); the public
/// surface (`days_between`, `date_add`, …) enforces that, so this stays
/// `pub(crate)` rather than exposing an unbounded-`i64` overflow hazard.
pub(crate) fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400; // [0, 399]
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146097 + doe - 719468
}

/// The inverse of [`days_from_civil`]: `(year, month, day)` for a day count
/// since the Unix epoch. **Internal** (`pub(crate)`): callers must pass an
/// ordinal within `±MAX_ORDINAL` so the multiplications cannot overflow.
pub(crate) fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Parse a `date(y, m, d)` term into validated `(year, month, day)`. Rejects an
/// out-of-range month (`1..=12`) or day (`1..=days_in_month`) so a malformed
/// `date(2025, 13, 40)` is a clean `None`, not a nonsense ordinal.
pub fn read_date(value: &Term) -> Option<(i64, i64, i64)> {
    let (functor, args) = match value {
        Term::Compound { functor, args } if functor == "date" => (functor, args),
        _ => return None,
    };
    let _ = functor;
    if args.len() != 3 {
        return None;
    }
    let y = read_int(&args[0])?;
    let m = read_int(&args[1])?;
    let d = read_int(&args[2])?;
    // Bound the year to a generous-but-safe range so the ordinal arithmetic
    // (`era * 146097`, …) cannot overflow i64 on an adversarial input like
    // `date(9223372036854775807, 1, 1)`. ±1,000,000 covers every real
    // adjudication date with a ~6-order-of-magnitude margin before overflow.
    if !(-1_000_000..=1_000_000).contains(&y) {
        return None;
    }
    if !(1..=12).contains(&m) {
        return None;
    }
    if d < 1 || d > days_in_month(y, m) {
        return None;
    }
    Some((y, m, d))
}

/// The day-ordinal (days since epoch) of a `date(y, m, d)` term, if valid.
pub fn date_ordinal(value: &Term) -> Option<i64> {
    let (y, m, d) = read_date(value)?;
    Some(days_from_civil(y, m, d))
}

/// Number of days in a month, accounting for leap years.
pub fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// Proleptic-Gregorian leap-year test.
pub fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Read a `duration(n, unit)` term as a whole number of **days**. Supports
/// `days` and `weeks` (× 7); other units (months/years are irregular) return
/// `None` for now. A bare `duration(n)` is assumed to be days.
pub fn read_duration_days(value: &Term) -> Option<i64> {
    let (functor, args) = match value {
        Term::Compound { functor, args } if functor == "duration" => (functor, args),
        _ => return None,
    };
    let _ = functor;
    let n = read_int(args.first()?)?;
    let days = match args.get(1) {
        None => n,
        Some(Term::Atom(u)) | Some(Term::Str(u)) => match u.as_str() {
            "days" | "day" => n,
            // `checked_mul` so a huge `n` can't overflow the weeks→days scaling.
            "weeks" | "week" => n.checked_mul(7)?,
            _ => return None,
        },
        _ => return None,
    };
    // Bound the duration to the supported date window so it can be added to a
    // date without overflow (mirrors the `read_date` year bound).
    if days.abs() > MAX_ORDINAL {
        return None;
    }
    Some(days)
}

/// Days elapsed from `a` to `b` (`b − a`), as a `Duration("days")` dimensioned
/// value. Negative if `b` precedes `a`. This is the deadline primitive: a
/// predicate `elapsed <= 365` fires over the result.
pub fn days_between(a: &Term, b: &Term) -> Option<Dimensioned> {
    let oa = date_ordinal(a)?;
    let ob = date_ordinal(b)?;
    Some(Dimensioned::new((ob - oa) as f64, Dimension::Duration("days".into())))
}

/// `Date + Duration → Date`: the `(year, month, day)` that is `days` after
/// `date`. Returns `None` if the result would fall outside the supported date
/// window (the `±MAX_ORDINAL` ordinal range), which also makes the addition
/// overflow-safe regardless of how large `days` is.
pub fn date_add(date: &Term, days: i64) -> Option<(i64, i64, i64)> {
    let ord = date_ordinal(date)?;
    let total = ord.checked_add(days)?;
    if total.abs() > MAX_ORDINAL {
        return None;
    }
    Some(civil_from_days(total))
}

/// `true` iff `a` is strictly before `b`.
pub fn before(a: &Term, b: &Term) -> Option<bool> {
    Some(date_ordinal(a)? < date_ordinal(b)?)
}

/// `true` iff `a` is strictly after `b`.
pub fn after(a: &Term, b: &Term) -> Option<bool> {
    Some(date_ordinal(a)? > date_ordinal(b)?)
}

fn read_int(t: &Term) -> Option<i64> {
    match t {
        Term::Num(Number::Int(i)) => Some(*i),
        // A whole-valued float is accepted (the decomposer may emit 2025.0).
        Term::Num(Number::Float(x)) if x.fract() == 0.0 && x.is_finite() => Some(*x as i64),
        // A whole-valued exact decimal (NX-2) is accepted on the same integral gate — a date field
        // written as `2025` lowers to `Int`, but one written `2025.0` lowers to `Exact`.
        Term::Num(Number::Exact(d)) => {
            let x = d.to_f64();
            if x.fract() == 0.0 && x.is_finite() {
                Some(x as i64)
            } else {
                None
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use logic_core::{atom, compound, int};

    fn date(y: i64, m: i64, d: i64) -> Term {
        compound("date", vec![int(y), int(m), int(d)])
    }

    #[test]
    fn civil_ordinal_round_trips() {
        for (y, m, d) in [(1970, 1, 1), (2000, 2, 29), (2025, 1, 15), (1999, 12, 31), (2024, 6, 11)] {
            let ord = days_from_civil(y, m, d);
            assert_eq!(civil_from_days(ord), (y, m, d), "round trip {y}-{m}-{d}");
        }
        // The epoch is day 0.
        assert_eq!(days_from_civil(1970, 1, 1), 0);
    }

    #[test]
    fn days_between_counts_calendar_days() {
        // 2025-01-15 to 2026-02-01: spans a non-leap year.
        let d = days_between(&date(2025, 1, 15), &date(2026, 2, 1)).unwrap();
        assert_eq!(d.dim, Dimension::Duration("days".into()));
        assert_eq!(d.magnitude, 382.0);
        // symmetric negative.
        assert_eq!(
            days_between(&date(2026, 2, 1), &date(2025, 1, 15)).unwrap().magnitude,
            -382.0
        );
        // a leap day is counted.
        assert_eq!(
            days_between(&date(2024, 2, 28), &date(2024, 3, 1)).unwrap().magnitude,
            2.0
        );
    }

    #[test]
    fn date_add_crosses_month_and_year_boundaries() {
        assert_eq!(date_add(&date(2025, 1, 15), 365).unwrap(), (2026, 1, 15));
        assert_eq!(date_add(&date(2025, 1, 31), 1).unwrap(), (2025, 2, 1));
        assert_eq!(date_add(&date(2024, 2, 28), 1).unwrap(), (2024, 2, 29)); // leap
        assert_eq!(date_add(&date(2025, 1, 1), -1).unwrap(), (2024, 12, 31));
    }

    #[test]
    fn before_after_order_dates() {
        assert_eq!(before(&date(2025, 1, 1), &date(2025, 1, 2)), Some(true));
        assert_eq!(after(&date(2025, 1, 2), &date(2025, 1, 1)), Some(true));
        assert_eq!(before(&date(2025, 1, 1), &date(2025, 1, 1)), Some(false));
    }

    #[test]
    fn deadline_predicate_use_case() {
        // "within 365 days of purchase" — the adjudication payoff.
        let elapsed = days_between(&date(2025, 1, 15), &date(2025, 6, 11)).unwrap();
        assert!(elapsed.magnitude <= 365.0);
        let stale = days_between(&date(2024, 1, 1), &date(2025, 6, 11)).unwrap();
        assert!(stale.magnitude > 365.0);
    }

    #[test]
    fn invalid_dates_are_rejected() {
        assert!(read_date(&date(2025, 13, 1)).is_none()); // month 13
        assert!(read_date(&date(2025, 2, 30)).is_none()); // Feb 30
        assert!(read_date(&date(2023, 2, 29)).is_none()); // not a leap year
        assert!(read_date(&date(2024, 2, 29)).is_some()); // leap year ok
        assert!(read_date(&compound("date", vec![int(2025), int(1)])).is_none()); // arity
        assert!(read_date(&atom("nope")).is_none());
        // an adversarial huge year is rejected (would overflow the ordinal math).
        assert!(read_date(&date(i64::MAX, 1, 1)).is_none());
    }

    #[test]
    fn duration_reads_days_and_weeks() {
        assert_eq!(read_duration_days(&compound("duration", vec![int(30), atom("days")])), Some(30));
        assert_eq!(read_duration_days(&compound("duration", vec![int(2), atom("weeks")])), Some(14));
        assert_eq!(read_duration_days(&compound("duration", vec![int(5)])), Some(5));
        // an irregular unit is not convertible to days here.
        assert_eq!(read_duration_days(&compound("duration", vec![int(3), atom("months")])), None);
        // overflow-prone / out-of-window durations are rejected, not overflowed.
        assert_eq!(read_duration_days(&compound("duration", vec![int(i64::MAX), atom("weeks")])), None);
        assert_eq!(read_duration_days(&compound("duration", vec![int(i64::MAX)])), None);
    }

    #[test]
    fn date_add_rejects_out_of_window_offsets() {
        // A huge offset returns None instead of overflowing the ordinal math.
        assert_eq!(date_add(&date(2025, 1, 1), i64::MAX), None);
        assert_eq!(date_add(&date(2025, 1, 1), -i64::MAX), None);
        // a sane offset still works.
        assert_eq!(date_add(&date(2025, 1, 1), 365), Some((2026, 1, 1)));
    }

    #[test]
    fn dates_do_not_go_through_the_generic_dimension_algebra() {
        // Adding two Date dimensions is rejected — use days_between instead.
        assert!(Dimension::combine(crate::DimOp::Sub, &Dimension::Date, &Dimension::Date).is_err());
        assert!(Dimension::combine(crate::DimOp::Add, &Dimension::Date, &Dimension::Duration("days".into())).is_err());
    }
}
