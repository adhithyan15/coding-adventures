//! Canonical scalar semantics for picker-backed HTML input values.

use layout_controls::ControlKind;

const MILLIS_PER_SECOND: i64 = 1_000;
const MILLIS_PER_DAY: i64 = 86_400_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedValue {
    pub scalar: i64,
    pub normalized: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TypedValueConstraints {
    pub minimum: Option<i64>,
    pub maximum: Option<i64>,
    pub step: i64,
    pub validates_step: bool,
}

pub fn parse_typed_value(kind: ControlKind, value: &str) -> Option<TypedValue> {
    let scalar = match kind {
        ControlKind::Date => parse_date(value)?,
        ControlKind::Month => parse_month(value)?,
        ControlKind::Week => parse_week(value)?,
        ControlKind::Time => parse_time(value)?,
        ControlKind::DateTimeLocal => parse_datetime_local(value)?,
        _ => return None,
    };
    Some(TypedValue {
        scalar,
        normalized: format_typed_value(kind, scalar)?,
    })
}

pub fn normalize_color(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    (bytes.len() == 7 && bytes[0] == b'#' && bytes[1..].iter().all(u8::is_ascii_hexdigit))
        .then(|| value.to_ascii_lowercase())
}

pub fn parse_typed_step(kind: ControlKind, value: &str) -> Option<i64> {
    let value = value.trim().parse::<f64>().ok()?;
    if !value.is_finite() || value <= 0.0 {
        return None;
    }
    let scale = match kind {
        ControlKind::Time | ControlKind::DateTimeLocal => MILLIS_PER_SECOND as f64,
        ControlKind::Date | ControlKind::Month | ControlKind::Week => 1.0,
        _ => return None,
    };
    let scaled = value * scale;
    if !scaled.is_finite()
        || scaled > i64::MAX as f64
        || scaled.fract().abs() > f64::EPSILON * scaled.abs().max(1.0) * 8.0
    {
        return None;
    }
    Some(scaled as i64)
}

pub fn typed_constraints(
    kind: ControlKind,
    min: Option<&str>,
    max: Option<&str>,
    step: Option<&str>,
) -> TypedValueConstraints {
    let minimum = min.and_then(|value| parse_typed_value(kind, value).map(|value| value.scalar));
    let maximum = max.and_then(|value| parse_typed_value(kind, value).map(|value| value.scalar));
    let authored_step = step.map(str::trim);
    let validates_step = authored_step != Some("any");
    let default_step = match kind {
        ControlKind::Time | ControlKind::DateTimeLocal => 60 * MILLIS_PER_SECOND,
        _ => 1,
    };
    let step = authored_step
        .and_then(|value| parse_typed_step(kind, value))
        .unwrap_or(default_step);
    TypedValueConstraints {
        minimum,
        maximum,
        step,
        validates_step,
    }
}

pub fn step_typed_value(
    kind: ControlKind,
    value: &str,
    constraints: TypedValueConstraints,
    forward: bool,
) -> Option<String> {
    let base = i128::from(constraints.minimum.unwrap_or(0));
    let step = i128::from(constraints.step);
    let current = parse_typed_value(kind, value).map(|value| i128::from(value.scalar));
    let mut next = match current {
        Some(current) if constraints.validates_step && (current - base) % step != 0 => {
            let offset = current - base;
            let quotient = offset.div_euclid(step);
            base + if forward {
                (quotient + 1) * step
            } else {
                quotient * step
            }
        }
        Some(current) => current + if forward { step } else { -step },
        None if forward => i128::from(constraints.minimum.unwrap_or(0)),
        None => i128::from(constraints.maximum.unwrap_or(0)),
    };
    if let Some(minimum) = constraints.minimum {
        next = next.max(i128::from(minimum));
    }
    if let Some(maximum) = constraints.maximum {
        next = next.min(i128::from(maximum));
    }
    format_typed_value(kind, i64::try_from(next).ok()?)
}

pub fn format_typed_value(kind: ControlKind, scalar: i64) -> Option<String> {
    match kind {
        ControlKind::Date => format_date(scalar),
        ControlKind::Month => format_month(scalar),
        ControlKind::Week => format_week(scalar),
        ControlKind::Time => format_time(scalar),
        ControlKind::DateTimeLocal => {
            let days = scalar.div_euclid(MILLIS_PER_DAY);
            let millis = scalar.rem_euclid(MILLIS_PER_DAY);
            Some(format!("{}T{}", format_date(days)?, format_time(millis)?))
        }
        _ => None,
    }
}

fn parse_date(value: &str) -> Option<i64> {
    let mut parts = value.split('-');
    let year = parse_fixed(parts.next()?, 4)? as i32;
    let month = parse_fixed(parts.next()?, 2)?;
    let day = parse_fixed(parts.next()?, 2)?;
    if parts.next().is_some() || year == 0 || day == 0 || day > days_in_month(year, month)? {
        return None;
    }
    Some(days_from_civil(year, month, day))
}

fn parse_month(value: &str) -> Option<i64> {
    let (year, month) = value.split_once('-')?;
    let year = parse_fixed(year, 4)? as i64;
    let month = parse_fixed(month, 2)? as i64;
    (year > 0 && (1..=12).contains(&month)).then_some((year - 1970) * 12 + month - 1)
}

fn parse_week(value: &str) -> Option<i64> {
    let (year, week) = value.split_once("-W")?;
    let year = parse_fixed(year, 4)? as i32;
    let week = parse_fixed(week, 2)?;
    if year == 0 || week == 0 || week > iso_weeks_in_year(year) {
        return None;
    }
    Some((iso_week_start(year, week) - iso_week_start(1970, 1)) / 7)
}

fn parse_time(value: &str) -> Option<i64> {
    let mut parts = value.split(':');
    let hour = parse_fixed(parts.next()?, 2)? as i64;
    let minute = parse_fixed(parts.next()?, 2)? as i64;
    let seconds = parts.next();
    if parts.next().is_some() || hour > 23 || minute > 59 {
        return None;
    }
    let (second, millis) = match seconds {
        None => (0, 0),
        Some(value) => {
            let (seconds, fraction) = value
                .split_once('.')
                .map_or((value, None), |parts| (parts.0, Some(parts.1)));
            let second = parse_fixed(seconds, 2)? as i64;
            if second > 59 {
                return None;
            }
            let millis = match fraction {
                None => 0,
                Some(value)
                    if (1..=3).contains(&value.len())
                        && value.bytes().all(|b| b.is_ascii_digit()) =>
                {
                    value.parse::<i64>().ok()? * 10_i64.pow((3 - value.len()) as u32)
                }
                Some(_) => return None,
            };
            (second, millis)
        }
    };
    Some(((hour * 60 + minute) * 60 + second) * MILLIS_PER_SECOND + millis)
}

fn parse_datetime_local(value: &str) -> Option<i64> {
    let (date, time) = value.split_once('T')?;
    Some(parse_date(date)? * MILLIS_PER_DAY + parse_time(time)?)
}

fn format_date(days: i64) -> Option<String> {
    if !(-719_162..=2_932_896).contains(&days) {
        return None;
    }
    let (year, month, day) = civil_from_days(days);
    (1..=9999)
        .contains(&year)
        .then(|| format!("{year:04}-{month:02}-{day:02}"))
}

fn format_month(months: i64) -> Option<String> {
    let year = 1970 + months.div_euclid(12);
    let month = months.rem_euclid(12) + 1;
    (1..=9999)
        .contains(&year)
        .then(|| format!("{year:04}-{month:02}"))
}

fn format_week(weeks: i64) -> Option<String> {
    let monday = iso_week_start(1970, 1).checked_add(weeks.checked_mul(7)?)?;
    let thursday = monday.checked_add(3)?;
    format_date(thursday)?;
    let (year, _, _) = civil_from_days(thursday);
    if !(1..=9999).contains(&year) {
        return None;
    }
    let week = (monday - iso_week_start(year as i32, 1)) / 7 + 1;
    Some(format!("{year:04}-W{week:02}"))
}

fn format_time(millis: i64) -> Option<String> {
    if !(0..MILLIS_PER_DAY).contains(&millis) {
        return None;
    }
    let hour = millis / 3_600_000;
    let minute = millis / 60_000 % 60;
    let second = millis / 1_000 % 60;
    let fraction = millis % 1_000;
    if second == 0 && fraction == 0 {
        Some(format!("{hour:02}:{minute:02}"))
    } else if fraction == 0 {
        Some(format!("{hour:02}:{minute:02}:{second:02}"))
    } else {
        Some(format!(
            "{hour:02}:{minute:02}:{second:02}.{}",
            format!("{fraction:03}").trim_end_matches('0')
        ))
    }
}

fn parse_fixed(value: &str, width: usize) -> Option<u32> {
    (value.len() == width && value.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| value.parse().ok())?
}

fn days_in_month(year: i32, month: u32) -> Option<u32> {
    Some(match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => return None,
    })
}

fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn iso_weeks_in_year(year: i32) -> u32 {
    let jan_first = (days_from_civil(year, 1, 1) + 3).rem_euclid(7);
    if jan_first == 3 || (jan_first == 2 && is_leap_year(year)) {
        53
    } else {
        52
    }
}

fn iso_week_start(year: i32, week: u32) -> i64 {
    let jan_fourth = days_from_civil(year, 1, 4);
    jan_fourth - (jan_fourth + 3).rem_euclid(7) + (week as i64 - 1) * 7
}

// Civil-date conversion algorithms use 1970-01-01 as day zero.
fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = year as i64 - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let shifted_month = month as i64 + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * shifted_month + 2) / 5 + day as i64 - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = shifted_month + if shifted_month < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_normalizes_every_temporal_family() {
        for (kind, authored, normalized) in [
            (ControlKind::Date, "2024-02-29", "2024-02-29"),
            (ControlKind::Month, "2024-07", "2024-07"),
            (ControlKind::Week, "2020-W53", "2020-W53"),
            (ControlKind::Time, "09:30:05.120", "09:30:05.12"),
            (
                ControlKind::DateTimeLocal,
                "2024-02-29T09:30:05.120",
                "2024-02-29T09:30:05.12",
            ),
        ] {
            assert_eq!(
                parse_typed_value(kind, authored).unwrap().normalized,
                normalized
            );
        }
        assert!(parse_typed_value(ControlKind::Date, "2023-02-29").is_none());
        assert!(parse_typed_value(ControlKind::Week, "2021-W53").is_none());
        assert!(parse_typed_value(ControlKind::Time, "24:00").is_none());
    }

    #[test]
    fn steps_temporal_values_with_bounds_and_alignment() {
        let constraints = typed_constraints(
            ControlKind::Date,
            Some("2024-01-01"),
            Some("2024-01-09"),
            Some("2"),
        );
        assert_eq!(
            step_typed_value(ControlKind::Date, "2024-01-02", constraints, true).as_deref(),
            Some("2024-01-03")
        );
        assert_eq!(
            step_typed_value(ControlKind::Date, "2024-01-09", constraints, true).as_deref(),
            Some("2024-01-09")
        );
        let time = typed_constraints(ControlKind::Time, None, None, Some("0.5"));
        assert_eq!(
            step_typed_value(ControlKind::Time, "12:00", time, true).as_deref(),
            Some("12:00:00.5")
        );
        assert_eq!(parse_typed_step(ControlKind::Time, "0.001"), Some(1));
        assert_eq!(parse_typed_step(ControlKind::Date, "2"), Some(2));
        assert!(parse_typed_step(ControlKind::Date, "0.5").is_none());
        assert!(parse_typed_step(ControlKind::Time, "0.0001").is_none());
    }

    #[test]
    fn temporal_scalars_round_trip_across_calendar_boundaries() {
        for days in (-25_567..=47_847).step_by(37) {
            let formatted = format_typed_value(ControlKind::Date, days).unwrap();
            assert_eq!(parse_typed_value(ControlKind::Date, &formatted).unwrap().scalar, days);
        }
        for months in (-840..=1_560).step_by(11) {
            let formatted = format_typed_value(ControlKind::Month, months).unwrap();
            assert_eq!(
                parse_typed_value(ControlKind::Month, &formatted)
                    .unwrap()
                    .scalar,
                months
            );
        }
        for weeks in (-3_650..=6_780).step_by(13) {
            let formatted = format_typed_value(ControlKind::Week, weeks).unwrap();
            assert_eq!(
                parse_typed_value(ControlKind::Week, &formatted)
                    .unwrap()
                    .scalar,
                weeks
            );
        }
        assert_eq!(
            parse_typed_value(ControlKind::Week, "1970-W01")
                .unwrap()
                .scalar,
            0
        );
    }

    #[test]
    fn normalizes_simple_colors() {
        assert_eq!(normalize_color("#A0b1C2").as_deref(), Some("#a0b1c2"));
        assert!(normalize_color("red").is_none());
        assert!(normalize_color("#abcd").is_none());
    }
}
