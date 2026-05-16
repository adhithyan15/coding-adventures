//! # datetime-core — Excel/Lotus/R date and time functions.
//!
//! Date math is the part of a spreadsheet's function catalog that lives
//! and dies on the *invariants* you choose at the bottom. This crate
//! picks those invariants once, in this file, and every public function
//! is implemented in terms of them:
//!
//! 1. The wire format for "an instant in time" is the
//!    [`wall_clock::Instant`] — Unix-epoch f64 seconds. Reading a clock
//!    is dependency-injected; no function in this crate touches
//!    `std::time::SystemTime` directly.
//!
//! 2. The wire format for "a calendar day with no time-of-day" is
//!    [`Date`] — i32 days since `1970-01-01`. This is the smallest
//!    representation that survives every Excel quirk and every R/POSIXct
//!    edge case. Negative values represent dates before the Unix epoch.
//!
//! 3. The civil ↔ serial conversion uses Howard Hinnant's algorithm
//!    (see [`civil_from_days`] / [`days_from_civil`]), which is exact
//!    for all Gregorian dates and tolerates dates before year 1, unlike
//!    Excel's date model.
//!
//! 4. Day-count conventions for [`yearfrac`] follow the bond-market
//!    standard set used by Excel: 30/360-US (default), Actual/Actual,
//!    Actual/360, Actual/365, 30/360-European. Each is documented at
//!    its function.
//!
//! ## Excel parity, not Excel mimicry
//!
//! Where Excel has documented bugs that we cannot productively
//! reproduce (the famous 1900 leap-year bug — Excel believes
//! `1900-02-29` exists), we choose the *correct* behavior and document
//! the divergence inline. The Excel 1900-epoch conversion helpers
//! [`from_excel_serial_1900`] / [`to_excel_serial_1900`] account for
//! the bug at the boundary so files that already trust Excel's
//! mis-numbering round-trip.
//!
//! ## Portability bar
//!
//! Per `backend-crate-catalog.md` §1: `forbid(unsafe_code)`, no
//! `#[cfg(target_os)]`, no I/O, no globals, no hidden clocks
//! (every `NOW`/`TODAY` takes a `&dyn Clock`). WASM-friendly via
//! `default-features = false` on the `wall-clock` dep.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use wall_clock::{Clock, Instant};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors raised by `datetime-core`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum DateError {
    /// A construction received a calendar component out of its
    /// permitted range (e.g. month 13, day 32, hour 25).
    InvalidComponent {
        /// The component name: "year" / "month" / "day" / "hour" /
        /// "minute" / "second".
        component: &'static str,
        /// The offending value, formatted as a string for diagnostic.
        value: String,
        /// Permitted range, formatted as a string for diagnostic.
        range: &'static str,
    },
    /// Arithmetic produced a date outside the i32 day-count range
    /// (well over 5 million years in either direction; only surfaces
    /// on adversarial input).
    Overflow {
        /// The function that triggered the overflow.
        function: &'static str,
    },
    /// A function received parameters whose pairing is invalid
    /// (e.g. `yearfrac` with `start > end` and `basis = 1` — actually
    /// this one is fine; example: `datedif` with an unknown unit).
    BadParameter {
        /// The parameter name.
        name: &'static str,
        /// The value, stringified.
        value: String,
    },
    /// A date string could not be parsed by `datevalue` /
    /// `timevalue`.
    ParseError {
        /// The function that attempted the parse.
        function: &'static str,
        /// The input that failed.
        input: String,
    },
}

impl core::fmt::Display for DateError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DateError::InvalidComponent {
                component,
                value,
                range,
            } => write!(
                f,
                "invalid {component}: {value} (must be in {range})"
            ),
            DateError::Overflow { function } => {
                write!(f, "{function}: date arithmetic overflowed i32")
            }
            DateError::BadParameter { name, value } => {
                write!(f, "bad parameter {name}={value}")
            }
            DateError::ParseError { function, input } => {
                write!(f, "{function}: could not parse '{input}'")
            }
        }
    }
}

impl std::error::Error for DateError {}

// ---------------------------------------------------------------------------
// Date — i32 days since 1970-01-01 (Unix epoch)
// ---------------------------------------------------------------------------

/// A calendar date with no time component, stored as days since the
/// Unix epoch (1970-01-01). Negative values represent earlier dates.
///
/// Range: roughly ±5.8 million years from the epoch — well beyond
/// any input a spreadsheet will ever see.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date(pub i32);

impl Date {
    /// 1970-01-01.
    pub const EPOCH: Date = Date(0);

    /// Construct from year-month-day. Validates the components.
    pub fn from_ymd(year: i32, month: u32, day: u32) -> Result<Self, DateError> {
        if !(1..=12).contains(&month) {
            return Err(DateError::InvalidComponent {
                component: "month",
                value: month.to_string(),
                range: "1..=12",
            });
        }
        let dim = days_in_month(year, month as u8);
        if day < 1 || day > dim as u32 {
            return Err(DateError::InvalidComponent {
                component: "day",
                value: day.to_string(),
                range: "valid for the given year and month",
            });
        }
        Ok(Date(days_from_civil(year, month as u8, day as u8)))
    }

    /// Decompose into year-month-day.
    pub fn to_ymd(self) -> (i32, u8, u8) {
        civil_from_days(self.0)
    }

    /// Convenience accessors.
    pub fn year(self) -> i32 {
        self.to_ymd().0
    }

    /// Month (1-12).
    pub fn month(self) -> u8 {
        self.to_ymd().1
    }

    /// Day of month (1-31).
    pub fn day(self) -> u8 {
        self.to_ymd().2
    }

    /// Days from `self` to `end` (positive if `end` is after `self`).
    pub fn days_until(self, end: Date) -> i32 {
        end.0.wrapping_sub(self.0)
    }

    /// Add a count of days. Saturating in case of overflow.
    pub fn add_days(self, days: i32) -> Date {
        Date(self.0.saturating_add(days))
    }

    /// Day of the week as ISO 8601 (1 = Monday, ... 7 = Sunday).
    pub fn iso_weekday(self) -> u8 {
        // 1970-01-01 was a Thursday (ISO 4). Add days then mod 7.
        let raw = (self.0.rem_euclid(7) + 3).rem_euclid(7); // 0=Mon..6=Sun
        (raw + 1) as u8
    }
}

// ---------------------------------------------------------------------------
// Civil ↔ days — Howard Hinnant's algorithm
//
// https://howardhinnant.github.io/date_algorithms.html
//
// Domain: all proleptic Gregorian dates. Exact and fast (no
// division-by-zero, no loops, branchless after the leap-year test).
// ---------------------------------------------------------------------------

/// Convert (year, month, day) to days since 1970-01-01.
pub fn days_from_civil(y: i32, m: u8, d: u8) -> i32 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u32; // [0, 399]
    let m_u = m as u32;
    let d_u = d as u32;
    let doy = (153 * (if m_u > 2 { m_u - 3 } else { m_u + 9 }) + 2) / 5 + d_u - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    (era * 146097 + doe as i32) - 719468
}

/// Convert days since 1970-01-01 to (year, month, day).
pub fn civil_from_days(z: i32) -> (i32, u8, u8) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let y = (yoe as i32) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let y = if m <= 2 { y + 1 } else { y };
    (y, m as u8, d as u8)
}

/// Whether `y` is a leap year in the proleptic Gregorian calendar.
pub fn is_leap_year(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
}

/// Number of days in (year, month). `month` must be in 1..=12.
pub fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0, // Caller is responsible for validating.
    }
}

/// Days in the given year. 365 or 366.
pub fn days_in_year(year: i32) -> u16 {
    if is_leap_year(year) {
        366
    } else {
        365
    }
}

// ---------------------------------------------------------------------------
// Excel serial conversion
// ---------------------------------------------------------------------------

/// Convert from Excel 1900-based serial number to our `Date`.
///
/// Excel has a notorious leap-year bug: it treats 1900 as a leap year
/// (it isn't — see `is_leap_year(1900)` returning `false`). This means
/// every Excel serial ≥ 60 is one greater than the "true" days-from-
/// 1900-01-01 count. This function applies the correction so the
/// returned `Date` is the calendar date Excel *displays*, not what a
/// strict mathematical conversion would produce.
///
/// Excel serial 1 = 1900-01-01 (per Excel's display).
/// Excel serial 60 = 1900-02-29 (a bogus date) — we map it to
/// 1900-02-28 here so subsequent arithmetic stays sane.
pub fn from_excel_serial_1900(serial: f64) -> Result<Date, DateError> {
    let truncated = serial.trunc() as i64;
    if !(0..=2_958_465).contains(&truncated) {
        return Err(DateError::BadParameter {
            name: "serial",
            value: serial.to_string(),
        });
    }
    let s = truncated as i32;
    // Excel serial 1 = 1900-01-01. Pre-bug counting: subtract 1 then
    // add to days_from_civil(1900, 1, 1). Bug correction: serials >= 60
    // need an extra -1.
    let base = days_from_civil(1900, 1, 1) - 1;
    let raw = base + s;
    let adjusted = if s >= 60 { raw - 1 } else { raw };
    Ok(Date(adjusted))
}

/// Convert from our `Date` to Excel 1900-based serial number,
/// reproducing Excel's leap-year bug so the round-trip is exact.
pub fn to_excel_serial_1900(date: Date) -> Result<f64, DateError> {
    let base = days_from_civil(1900, 1, 1) - 1;
    let s = date.0 - base;
    // For dates on or after 1900-03-01, Excel adds the phantom day.
    let mar_1_1900 = days_from_civil(1900, 3, 1);
    let adjusted = if date.0 >= mar_1_1900 { s + 1 } else { s };
    if !(0..=2_958_465).contains(&adjusted) {
        return Err(DateError::Overflow {
            function: "to_excel_serial_1900",
        });
    }
    Ok(adjusted as f64)
}

/// Excel "1904 date system" — used in macOS Excel, doesn't have the
/// leap-year bug. Serial 0 = 1904-01-01.
pub fn from_excel_serial_1904(serial: f64) -> Result<Date, DateError> {
    let truncated = serial.trunc() as i64;
    if !(0..=2_957_003).contains(&truncated) {
        return Err(DateError::BadParameter {
            name: "serial",
            value: serial.to_string(),
        });
    }
    Ok(Date(days_from_civil(1904, 1, 1) + truncated as i32))
}

/// Inverse of [`from_excel_serial_1904`].
pub fn to_excel_serial_1904(date: Date) -> Result<f64, DateError> {
    let s = date.0 - days_from_civil(1904, 1, 1);
    if s < 0 {
        return Err(DateError::Overflow {
            function: "to_excel_serial_1904",
        });
    }
    Ok(s as f64)
}

// ---------------------------------------------------------------------------
// Time-of-day helpers — Time is a fraction of a day in [0.0, 1.0)
// ---------------------------------------------------------------------------

/// Construct a time-of-day fraction from hour-minute-second.
///
/// Excel parity: `TIME(h, m, s)` returns a value in `[0.0, 1.0)`
/// representing the fraction of a 24-hour day. Hour overflow wraps
/// modulo 24 (matches Excel: `TIME(25, 0, 0) = TIME(1, 0, 0)`).
pub fn time(hour: i32, minute: i32, second: i32) -> Result<f64, DateError> {
    let total = hour as i64 * 3600 + minute as i64 * 60 + second as i64;
    let total = total.rem_euclid(86400);
    Ok(total as f64 / 86400.0)
}

/// Hour component of a time-of-day fraction (0..24).
pub fn hour_of(time_frac: f64) -> u8 {
    let seconds = (time_frac.rem_euclid(1.0) * 86400.0).round() as u32;
    ((seconds / 3600) % 24) as u8
}

/// Minute component (0..60).
pub fn minute_of(time_frac: f64) -> u8 {
    let seconds = (time_frac.rem_euclid(1.0) * 86400.0).round() as u32;
    ((seconds / 60) % 60) as u8
}

/// Second component (0..60).
pub fn second_of(time_frac: f64) -> u8 {
    let seconds = (time_frac.rem_euclid(1.0) * 86400.0).round() as u32;
    (seconds % 60) as u8
}

// ---------------------------------------------------------------------------
// Clock-injected: now() and today()
// ---------------------------------------------------------------------------

/// Excel `NOW()` — current instant. Reads through the injected clock
/// so tests can pin a known time.
pub fn now(clock: &dyn Clock) -> Instant {
    clock.now()
}

/// Excel `TODAY()` — current calendar date. Reads through the
/// injected clock and projects to a `Date`.
pub fn today(clock: &dyn Clock) -> Date {
    Date(date_part_of(clock.now()))
}

/// Days-since-epoch part of an `Instant`. Used by `today()` and by
/// downstream `YEAR/MONTH/DAY` extractors that take an `Instant`.
pub fn date_part_of(instant: Instant) -> i32 {
    (instant.seconds_since_epoch / 86400.0).floor() as i32
}

/// Time-of-day fraction of an `Instant` in `[0.0, 1.0)`.
pub fn time_part_of(instant: Instant) -> f64 {
    let secs = instant.seconds_since_epoch;
    let day = secs / 86400.0;
    day - day.floor()
}

// ---------------------------------------------------------------------------
// Excel-named extractors over Date
// ---------------------------------------------------------------------------

/// Excel `YEAR(date)`.
pub fn year(date: Date) -> i32 {
    date.year()
}

/// Excel `MONTH(date)`.
pub fn month(date: Date) -> u8 {
    date.month()
}

/// Excel `DAY(date)`.
pub fn day(date: Date) -> u8 {
    date.day()
}

/// Excel `HOUR(serial)` — extracts the hour from a time fraction.
pub fn hour(time_frac: f64) -> u8 {
    hour_of(time_frac)
}

/// Excel `MINUTE(serial)`.
pub fn minute(time_frac: f64) -> u8 {
    minute_of(time_frac)
}

/// Excel `SECOND(serial)`.
pub fn second(time_frac: f64) -> u8 {
    second_of(time_frac)
}

/// Excel `WEEKDAY(date, return_type)` — day of week.
///
/// `return_type`:
/// - 1 (default): 1 = Sunday … 7 = Saturday
/// - 2: 1 = Monday … 7 = Sunday
/// - 3: 0 = Monday … 6 = Sunday
/// - 11: 1 = Monday … 7 = Sunday (same as 2)
/// - 12: 1 = Tuesday … 7 = Monday
/// - 13: 1 = Wednesday … 7 = Tuesday
/// - 14: 1 = Thursday … 7 = Wednesday
/// - 15: 1 = Friday … 7 = Thursday
/// - 16: 1 = Saturday … 7 = Friday
/// - 17: 1 = Sunday … 7 = Saturday (same as 1)
pub fn weekday(date: Date, return_type: u8) -> Result<u8, DateError> {
    let iso = date.iso_weekday(); // 1=Mon..7=Sun
    let sunday_based = (iso % 7) + 1; // 1=Sun..7=Sat
    match return_type {
        1 | 17 => Ok(sunday_based),
        2 | 11 => Ok(iso),
        3 => Ok(iso - 1),
        12..=16 => {
            // Anchor: iso - offset, rotated into 1..=7.
            let anchor = return_type - 11; // 1=Tue, 2=Wed, ... 5=Sat
            let shifted = ((iso as i32 - anchor as i32).rem_euclid(7) + 1) as u8;
            Ok(shifted)
        }
        _ => Err(DateError::BadParameter {
            name: "return_type",
            value: return_type.to_string(),
        }),
    }
}

/// Excel `ISOWEEKNUM(date)` — ISO-8601 week number (1..53).
pub fn isoweeknum(date: Date) -> u8 {
    // ISO weeks: week containing Thursday is in that year.
    let (y, _, _) = date.to_ymd();
    let thursday_of_this_week = date.add_days(4 - date.iso_weekday() as i32);
    let (year_of_thursday, _, _) = thursday_of_this_week.to_ymd();
    // Days from Jan 4 (always in week 1) of `year_of_thursday` to our date.
    let jan_4 = days_from_civil(year_of_thursday, 1, 4);
    let week_start_of_jan_4 = jan_4 - (Date(jan_4).iso_weekday() as i32 - 1);
    let week_number = ((date.0 - week_start_of_jan_4) / 7) + 1;
    // Suppress unused: we computed `y` only to compare for documentation.
    let _ = y;
    week_number as u8
}

// ---------------------------------------------------------------------------
// Date arithmetic: EDATE, EOMONTH, DATEDIF, DAYS
// ---------------------------------------------------------------------------

/// Excel `DAYS(end, start)` — days between two dates. Positive if
/// `end` is after `start`.
pub fn days(end: Date, start: Date) -> i32 {
    end.0.wrapping_sub(start.0)
}

/// Excel `DAYS360(start, end, method)` — days between two dates
/// assuming each month has 30 days.
///
/// `method`:
/// - `false` (US/NASD): if start is end of February, treat as end of
///   month; pre-snapping adjustments per the SIA bond standard.
/// - `true` (European): if start or end is day 31, replace with 30.
pub fn days360(start: Date, end: Date, method: bool) -> i32 {
    let (sy, sm, mut sd) = start.to_ymd();
    let (ey, em, mut ed) = end.to_ymd();

    if method {
        // European method: simply clamp 31 → 30.
        if sd == 31 {
            sd = 30;
        }
        if ed == 31 {
            ed = 30;
        }
    } else {
        // US (NASD) method:
        let last_feb_start = sm == 2 && sd == days_in_month(sy, 2);
        let last_feb_end = em == 2 && ed == days_in_month(ey, 2);
        if last_feb_start && last_feb_end {
            ed = 30;
        }
        if last_feb_start {
            sd = 30;
        }
        if ed == 31 && sd >= 30 {
            ed = 30;
        }
        if sd == 31 {
            sd = 30;
        }
    }

    ((ey - sy) * 360) + ((em as i32 - sm as i32) * 30) + (ed as i32 - sd as i32)
}

/// Excel `EDATE(start, months)` — add a number of months. Clamps
/// the day-of-month if the target month is shorter.
pub fn edate(start: Date, months: i32) -> Result<Date, DateError> {
    let (y, m, d) = start.to_ymd();
    let total_months = y * 12 + (m as i32 - 1) + months;
    let new_year = total_months.div_euclid(12);
    let new_month = (total_months.rem_euclid(12) + 1) as u8;
    let max_day = days_in_month(new_year, new_month);
    let new_day = d.min(max_day);
    Date::from_ymd(new_year, new_month as u32, new_day as u32)
}

/// Excel `EOMONTH(start, months)` — last day of the month after
/// adding `months`.
pub fn eomonth(start: Date, months: i32) -> Result<Date, DateError> {
    let (y, m, _) = start.to_ymd();
    let total_months = y * 12 + (m as i32 - 1) + months;
    let new_year = total_months.div_euclid(12);
    let new_month = (total_months.rem_euclid(12) + 1) as u8;
    let last_day = days_in_month(new_year, new_month);
    Date::from_ymd(new_year, new_month as u32, last_day as u32)
}

/// Excel `DATEDIF(start, end, unit)` — difference between two dates
/// in the requested unit.
///
/// Supported units: `"Y"` (whole years), `"M"` (whole months),
/// `"D"` (days — same as `DAYS`), `"YM"` (whole months excluding
/// whole years), `"YD"` (days excluding whole years), `"MD"` (days
/// excluding whole months — note Excel's MD is known to be buggy for
/// some date ranges; we follow the spec, not the bug).
pub fn datedif(start: Date, end: Date, unit: &str) -> Result<i32, DateError> {
    if end < start {
        return Err(DateError::BadParameter {
            name: "end",
            value: format!("end < start ({end:?} < {start:?})"),
        });
    }
    let (sy, sm, sd) = start.to_ymd();
    let (ey, em, ed) = end.to_ymd();

    let result = match unit {
        "D" | "d" => days(end, start),
        "Y" | "y" => {
            let mut diff = ey - sy;
            // Walk back if the end MM-DD is before the start MM-DD.
            if (em, ed) < (sm, sd) {
                diff -= 1;
            }
            diff
        }
        "M" | "m" => {
            let mut diff = (ey - sy) * 12 + (em as i32 - sm as i32);
            if ed < sd {
                diff -= 1;
            }
            diff
        }
        "YM" | "ym" => {
            let mut diff = em as i32 - sm as i32;
            if ed < sd {
                diff -= 1;
            }
            ((diff % 12) + 12) % 12
        }
        "YD" | "yd" => {
            // Days from "(start_month, start_day) of end_year" or
            // "(start_month, start_day) of end_year - 1" up to end.
            let anchor_year = if (em, ed) >= (sm, sd) { ey } else { ey - 1 };
            let max_day = days_in_month(anchor_year, sm);
            let anchor_day = sd.min(max_day);
            let anchor = Date::from_ymd(anchor_year, sm as u32, anchor_day as u32)?;
            days(end, anchor)
        }
        "MD" | "md" => {
            // Days from "start_day in end_month-end_year" up to end_day.
            // Clamp if start_day exceeds days in that month.
            let max_day = days_in_month(ey, em);
            let anchor_day = sd.min(max_day);
            let anchor_year;
            let anchor_month;
            if ed < sd {
                if em == 1 {
                    anchor_year = ey - 1;
                    anchor_month = 12;
                } else {
                    anchor_year = ey;
                    anchor_month = em - 1;
                }
                let max_day = days_in_month(anchor_year, anchor_month);
                let anchor_day = sd.min(max_day);
                let anchor =
                    Date::from_ymd(anchor_year, anchor_month as u32, anchor_day as u32)?;
                days(end, anchor)
            } else {
                let anchor =
                    Date::from_ymd(ey, em as u32, anchor_day as u32)?;
                days(end, anchor)
            }
        }
        other => {
            return Err(DateError::BadParameter {
                name: "unit",
                value: other.to_string(),
            })
        }
    };

    Ok(result)
}

// ---------------------------------------------------------------------------
// YEARFRAC — fractional years between two dates, by day-count basis
// ---------------------------------------------------------------------------

/// Day-count convention for [`yearfrac`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[non_exhaustive]
pub enum DayCount {
    /// 0: 30/360 US (NASD).
    Us30360 = 0,
    /// 1: Actual/Actual.
    ActualActual = 1,
    /// 2: Actual/360.
    Actual360 = 2,
    /// 3: Actual/365.
    Actual365 = 3,
    /// 4: 30/360 European.
    European30360 = 4,
}

impl DayCount {
    /// Convert from Excel's integer basis (0..=4).
    pub fn from_basis(basis: u8) -> Result<Self, DateError> {
        match basis {
            0 => Ok(DayCount::Us30360),
            1 => Ok(DayCount::ActualActual),
            2 => Ok(DayCount::Actual360),
            3 => Ok(DayCount::Actual365),
            4 => Ok(DayCount::European30360),
            other => Err(DateError::BadParameter {
                name: "basis",
                value: other.to_string(),
            }),
        }
    }
}

/// Excel `YEARFRAC(start, end, basis)` — fractional years between
/// two dates, using the specified day-count convention.
pub fn yearfrac(start: Date, end: Date, basis: DayCount) -> Result<f64, DateError> {
    // Swap if necessary so `start <= end`. Excel returns a positive
    // value in either direction.
    let (a, b) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };

    match basis {
        DayCount::Us30360 => {
            let d360 = days360(a, b, false);
            Ok(d360 as f64 / 360.0)
        }
        DayCount::European30360 => {
            let d360 = days360(a, b, true);
            Ok(d360 as f64 / 360.0)
        }
        DayCount::Actual360 => Ok(days(b, a) as f64 / 360.0),
        DayCount::Actual365 => Ok(days(b, a) as f64 / 365.0),
        DayCount::ActualActual => {
            // ISDA Actual/Actual: weighted by days in each year crossed.
            let (sy, _, _) = a.to_ymd();
            let (ey, _, _) = b.to_ymd();
            if sy == ey {
                let denom = days_in_year(sy) as f64;
                Ok(days(b, a) as f64 / denom)
            } else {
                let mut total = 0.0;
                // Days from `a` to end of its year.
                let start_year_end = Date::from_ymd(sy + 1, 1, 1)?;
                total += days(start_year_end, a) as f64 / days_in_year(sy) as f64;
                // Whole years in between.
                for y in (sy + 1)..ey {
                    total += 1.0;
                    let _ = days_in_year(y); // (constant 1.0 weight per whole year)
                }
                // Days from start of end year to `b`.
                let end_year_start = Date::from_ymd(ey, 1, 1)?;
                total += days(b, end_year_start) as f64 / days_in_year(ey) as f64;
                Ok(total)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use wall_clock::{FixedClock, Instant};

    #[test]
    fn epoch_is_thursday() {
        // 1970-01-01 was a Thursday — ISO weekday 4.
        assert_eq!(Date::EPOCH.iso_weekday(), 4);
    }

    #[test]
    fn from_ymd_round_trips() {
        let d = Date::from_ymd(2024, 2, 29).unwrap();
        assert_eq!(d.to_ymd(), (2024, 2, 29));
        let d = Date::from_ymd(1900, 1, 1).unwrap();
        assert_eq!(d.to_ymd(), (1900, 1, 1));
        let d = Date::from_ymd(2000, 12, 31).unwrap();
        assert_eq!(d.to_ymd(), (2000, 12, 31));
        let d = Date::from_ymd(-44, 3, 15).unwrap(); // Ides of March, 44 BCE
        assert_eq!(d.to_ymd(), (-44, 3, 15));
    }

    #[test]
    fn from_ymd_rejects_invalid() {
        assert!(matches!(
            Date::from_ymd(2023, 0, 1),
            Err(DateError::InvalidComponent { component: "month", .. })
        ));
        assert!(matches!(
            Date::from_ymd(2023, 13, 1),
            Err(DateError::InvalidComponent { component: "month", .. })
        ));
        assert!(matches!(
            Date::from_ymd(2023, 2, 29),
            Err(DateError::InvalidComponent { component: "day", .. })
        ));
        // 2024 is a leap year, 2023 isn't.
        Date::from_ymd(2024, 2, 29).unwrap();
        Date::from_ymd(2023, 2, 28).unwrap();
    }

    #[test]
    fn leap_year_rules() {
        assert!(is_leap_year(2000));
        assert!(!is_leap_year(1900)); // Century but not 400-divisible.
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(2023));
        assert!(is_leap_year(2400));
        assert!(!is_leap_year(2500));
    }

    #[test]
    fn weekday_known_dates() {
        // 2024-01-01 was a Monday.
        let d = Date::from_ymd(2024, 1, 1).unwrap();
        assert_eq!(d.iso_weekday(), 1);
        assert_eq!(weekday(d, 1).unwrap(), 2); // 1=Sun, so Mon=2
        assert_eq!(weekday(d, 2).unwrap(), 1);
        assert_eq!(weekday(d, 3).unwrap(), 0);
    }

    #[test]
    fn weekday_rejects_bad_return_type() {
        let d = Date::from_ymd(2024, 1, 1).unwrap();
        assert!(weekday(d, 0).is_err());
        assert!(weekday(d, 4).is_err());
        assert!(weekday(d, 10).is_err());
        assert!(weekday(d, 18).is_err());
    }

    #[test]
    fn isoweeknum_thursday_rule() {
        // 2020-12-31 was a Thursday — ISO week 53.
        let d = Date::from_ymd(2020, 12, 31).unwrap();
        assert_eq!(isoweeknum(d), 53);
        // 2021-01-01 was a Friday — still in ISO 2020 week 53.
        let d = Date::from_ymd(2021, 1, 1).unwrap();
        assert_eq!(isoweeknum(d), 53);
        // 2021-01-04 was a Monday — ISO 2021 week 1.
        let d = Date::from_ymd(2021, 1, 4).unwrap();
        assert_eq!(isoweeknum(d), 1);
    }

    #[test]
    fn days_in_month_table() {
        assert_eq!(days_in_month(2023, 1), 31);
        assert_eq!(days_in_month(2023, 2), 28);
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2023, 4), 30);
        assert_eq!(days_in_month(2023, 12), 31);
    }

    #[test]
    fn time_constructor_wraps_and_extracts() {
        let t = time(9, 30, 15).unwrap();
        assert_eq!(hour_of(t), 9);
        assert_eq!(minute_of(t), 30);
        assert_eq!(second_of(t), 15);

        // Wraps at 24h.
        let t = time(25, 0, 0).unwrap();
        assert_eq!(hour_of(t), 1);

        // Midnight.
        let t = time(0, 0, 0).unwrap();
        assert_eq!(t, 0.0);
    }

    #[test]
    fn now_today_clock_injection() {
        // 2024-06-15 18:30:00 UTC — pick an arbitrary instant.
        let instant = Instant::from_secs(1_718_476_200.0);
        let clock = FixedClock::new(instant);
        assert_eq!(now(&clock), instant);
        let today_date = today(&clock);
        let (y, m, _) = today_date.to_ymd();
        assert_eq!(y, 2024);
        assert_eq!(m, 6);
    }

    #[test]
    fn excel_serial_1900_round_trips() {
        // Excel serial 1 = 1900-01-01.
        let d = from_excel_serial_1900(1.0).unwrap();
        assert_eq!(d.to_ymd(), (1900, 1, 1));
        // Excel serial 60 = 1900-02-29 (the phantom day) — we map to 02-28.
        let d = from_excel_serial_1900(60.0).unwrap();
        assert_eq!(d.to_ymd(), (1900, 2, 28));
        // Excel serial 61 = 1900-03-01.
        let d = from_excel_serial_1900(61.0).unwrap();
        assert_eq!(d.to_ymd(), (1900, 3, 1));
        // Round-trip a modern date.
        let original = Date::from_ymd(2024, 6, 15).unwrap();
        let serial = to_excel_serial_1900(original).unwrap();
        let back = from_excel_serial_1900(serial).unwrap();
        assert_eq!(original, back);
    }

    #[test]
    fn excel_serial_1904_round_trips() {
        let d = from_excel_serial_1904(0.0).unwrap();
        assert_eq!(d.to_ymd(), (1904, 1, 1));
        let original = Date::from_ymd(2024, 6, 15).unwrap();
        let serial = to_excel_serial_1904(original).unwrap();
        let back = from_excel_serial_1904(serial).unwrap();
        assert_eq!(original, back);
    }

    #[test]
    fn days_distance() {
        let a = Date::from_ymd(2024, 1, 1).unwrap();
        let b = Date::from_ymd(2024, 12, 31).unwrap();
        assert_eq!(days(b, a), 365); // 2024 is a leap year.
    }

    #[test]
    fn days360_us_and_european() {
        let a = Date::from_ymd(2024, 1, 1).unwrap();
        let b = Date::from_ymd(2024, 12, 31).unwrap();
        // US/NASD: per Excel's DAYS360 default. 2024-12-31 is the last
        // day of month, start day = 1 < 30, so the SIA tweak does not
        // promote the end day; counted as month 12 day 31 minus month
        // 1 day 1 = 11*30 + 30 = 360. Excel returns 360 here too.
        let us = days360(a, b, false);
        assert_eq!(us, 360);
        // European: clamps end day 31 → 30, giving 11*30 + 29 = 359.
        let eu = days360(a, b, true);
        assert_eq!(eu, 359);
        // Tighter regression: a 30 + 31 month boundary, mid-year.
        let a = Date::from_ymd(2024, 1, 30).unwrap();
        let b = Date::from_ymd(2024, 3, 31).unwrap();
        // US: end day 31, start day = 30, so end → 30. (3-1)*30 + (30-30) = 60.
        assert_eq!(days360(a, b, false), 60);
        // European: end day 31 → 30. 60.
        assert_eq!(days360(a, b, true), 60);
    }

    #[test]
    fn edate_clamps_day_of_month() {
        // 2024-01-31 + 1 month → 2024-02-29 (leap-year clamp).
        let start = Date::from_ymd(2024, 1, 31).unwrap();
        assert_eq!(edate(start, 1).unwrap().to_ymd(), (2024, 2, 29));
        // 2023-01-31 + 1 month → 2023-02-28.
        let start = Date::from_ymd(2023, 1, 31).unwrap();
        assert_eq!(edate(start, 1).unwrap().to_ymd(), (2023, 2, 28));
        // Negative months.
        let start = Date::from_ymd(2024, 3, 31).unwrap();
        assert_eq!(edate(start, -1).unwrap().to_ymd(), (2024, 2, 29));
    }

    #[test]
    fn eomonth_returns_last_day() {
        let start = Date::from_ymd(2024, 1, 15).unwrap();
        assert_eq!(eomonth(start, 0).unwrap().to_ymd(), (2024, 1, 31));
        assert_eq!(eomonth(start, 1).unwrap().to_ymd(), (2024, 2, 29));
        assert_eq!(eomonth(start, -1).unwrap().to_ymd(), (2023, 12, 31));
    }

    #[test]
    fn datedif_units() {
        let a = Date::from_ymd(2020, 3, 15).unwrap();
        let b = Date::from_ymd(2024, 7, 10).unwrap();
        assert_eq!(datedif(a, b, "Y").unwrap(), 4);
        assert_eq!(datedif(a, b, "M").unwrap(), 4 * 12 + 3); // 51
        assert_eq!(datedif(a, b, "D").unwrap(), days(b, a));
    }

    #[test]
    fn datedif_rejects_reversed_dates() {
        let a = Date::from_ymd(2024, 1, 1).unwrap();
        let b = Date::from_ymd(2023, 1, 1).unwrap();
        assert!(matches!(
            datedif(a, b, "Y"),
            Err(DateError::BadParameter { .. })
        ));
    }

    #[test]
    fn datedif_rejects_unknown_unit() {
        let a = Date::from_ymd(2024, 1, 1).unwrap();
        let b = Date::from_ymd(2024, 1, 2).unwrap();
        assert!(matches!(
            datedif(a, b, "X"),
            Err(DateError::BadParameter { name: "unit", .. })
        ));
    }

    #[test]
    fn yearfrac_basis_0_30_360_us() {
        let a = Date::from_ymd(2024, 1, 1).unwrap();
        let b = Date::from_ymd(2025, 1, 1).unwrap();
        let frac = yearfrac(a, b, DayCount::Us30360).unwrap();
        // 30/360 makes a year exactly 1.0.
        assert!((frac - 1.0).abs() < 1e-9);
    }

    #[test]
    fn yearfrac_basis_3_actual_365() {
        let a = Date::from_ymd(2024, 1, 1).unwrap();
        let b = Date::from_ymd(2025, 1, 1).unwrap();
        let frac = yearfrac(a, b, DayCount::Actual365).unwrap();
        // 2024 has 366 days, so 366/365 = 1.00274...
        assert!((frac - 366.0 / 365.0).abs() < 1e-9);
    }

    #[test]
    fn yearfrac_basis_2_actual_360() {
        let a = Date::from_ymd(2024, 1, 1).unwrap();
        let b = Date::from_ymd(2025, 1, 1).unwrap();
        let frac = yearfrac(a, b, DayCount::Actual360).unwrap();
        assert!((frac - 366.0 / 360.0).abs() < 1e-9);
    }

    #[test]
    fn yearfrac_basis_1_actual_actual_within_year() {
        let a = Date::from_ymd(2024, 1, 1).unwrap();
        let b = Date::from_ymd(2024, 12, 31).unwrap();
        let frac = yearfrac(a, b, DayCount::ActualActual).unwrap();
        // 365 days of 366 in 2024.
        assert!((frac - 365.0 / 366.0).abs() < 1e-9);
    }

    #[test]
    fn yearfrac_basis_1_actual_actual_across_years() {
        let a = Date::from_ymd(2023, 7, 1).unwrap();
        let b = Date::from_ymd(2024, 7, 1).unwrap();
        let frac = yearfrac(a, b, DayCount::ActualActual).unwrap();
        // Half of 2023 + half of 2024 ≈ 1.0.
        assert!((frac - 1.0).abs() < 0.01);
    }

    #[test]
    fn yearfrac_swaps_if_start_after_end() {
        let a = Date::from_ymd(2024, 1, 1).unwrap();
        let b = Date::from_ymd(2025, 1, 1).unwrap();
        let forward = yearfrac(a, b, DayCount::Us30360).unwrap();
        let backward = yearfrac(b, a, DayCount::Us30360).unwrap();
        assert_eq!(forward, backward);
    }

    #[test]
    fn daycount_from_basis_rejects_invalid() {
        assert!(DayCount::from_basis(0).is_ok());
        assert!(DayCount::from_basis(4).is_ok());
        assert!(DayCount::from_basis(5).is_err());
        assert!(DayCount::from_basis(99).is_err());
    }

    #[test]
    fn date_part_of_and_time_part_of() {
        // 1970-01-02T06:00:00Z = epoch + 1 day + 6 hours
        let instant = Instant::from_secs(86400.0 + 6.0 * 3600.0);
        assert_eq!(date_part_of(instant), 1);
        let frac = time_part_of(instant);
        assert!((frac - 0.25).abs() < 1e-9); // 6 hours = 1/4 day
    }
}
