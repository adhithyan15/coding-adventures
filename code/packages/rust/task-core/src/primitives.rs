//! The units the model measures in: calendar dates, working-time durations, effort,
//! and money.
//!
//! ## Why durations are integers of *working* minutes
//!
//! A project schedule does not run in wall-clock time. "3 days" of work spans a
//! weekend as five, not three, calendar days; an 8-hour task on a half-day Friday
//! finishes Monday. So the model stores durations and effort as integer **minutes of
//! working time** and resolves them to real dates against a [`crate::Calendar`]
//! during scheduling. Integers keep the arithmetic exact and the JSON simple.

/// A calendar date, stored as **days since the Unix epoch (1970-01-01)** in UTC —
/// exactly `datetime_core::Date`'s representation, so all civil-date arithmetic is a
/// direct delegation to that crate (no date math is reinvented here).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Date(pub i32);

impl Date {
    /// Construct from a civil year/month/day, or `None` if the date is invalid.
    pub fn from_ymd(year: i32, month: u32, day: u32) -> Option<Date> {
        datetime_core::Date::from_ymd(year, month, day)
            .ok()
            .map(|d| Date(d.0))
    }

    /// Decompose into `(year, month, day)`.
    pub fn to_ymd(self) -> (i32, u8, u8) {
        datetime_core::Date(self.0).to_ymd()
    }

    /// ISO weekday: Monday = 1 … Sunday = 7.
    pub fn weekday(self) -> u8 {
        datetime_core::Date(self.0).iso_weekday()
    }

    /// This date shifted by `days` (negative moves backward).
    pub fn add_days(self, days: i32) -> Date {
        Date(datetime_core::Date(self.0).add_days(days).0)
    }

    /// Number of days from `self` to `end` (negative if `end` precedes `self`).
    pub fn days_until(self, end: Date) -> i32 {
        datetime_core::Date(self.0).days_until(datetime_core::Date(end.0))
    }
}

/// A span of scheduled time.
///
/// `working_minutes` is measured against a calendar unless `elapsed` is set, in
/// which case it is raw wall-clock (e.g. "wait 24h for concrete to cure" ignores
/// weekends). A milestone is a task whose `duration` is zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Duration {
    /// Minutes of time this span covers.
    pub working_minutes: i64,
    /// When true, the span ignores the calendar and is measured in wall-clock time.
    pub elapsed: bool,
}

impl Duration {
    /// A working-time duration of `minutes`.
    pub fn minutes(minutes: i64) -> Duration {
        Duration {
            working_minutes: minutes,
            elapsed: false,
        }
    }
    /// Zero duration — the definition of a milestone.
    pub fn zero() -> Duration {
        Duration::minutes(0)
    }
    /// True if this is a zero-length span.
    pub fn is_zero(self) -> bool {
        self.working_minutes == 0
    }
}

/// Total effort required, in person-minutes. Distinct from [`Duration`]: five people
/// working one day is `Work` = 5 days but `Duration` = 1 day (`Work = Duration ×
/// Units`, the scheduling triangle).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Work {
    /// Effort in person-minutes.
    pub minutes: i64,
}

impl Work {
    /// `minutes` of effort.
    pub fn minutes(minutes: i64) -> Work {
        Work { minutes }
    }
    /// Zero effort.
    pub fn zero() -> Work {
        Work { minutes: 0 }
    }
}

/// An exact monetary amount: integer minor units (e.g. cents) plus an ISO-4217
/// currency code. Integers avoid floating-point rounding in cost rollups.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Money {
    /// Amount in minor units (cents for USD/EUR, etc.).
    pub minor_units: i64,
    /// ISO-4217 currency code, e.g. "USD".
    pub currency: String,
}

impl Money {
    /// Zero in the given `currency`.
    pub fn zero(currency: impl Into<String>) -> Money {
        Money {
            minor_units: 0,
            currency: currency.into(),
        }
    }
}
