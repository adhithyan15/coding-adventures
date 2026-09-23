//! The calendar day an entry belongs to.
//!
//! ## Why a day, not an instant
//!
//! A journal entry is *about* a day. Write at 00:30 about the evening you just had
//! and the entry belongs to yesterday, whatever the clock says. The TypeScript
//! Journal made the same call (`createdAt: "YYYY-MM-DD"`), and Day One lets you
//! move an entry to any date. So an entry carries a civil [`Date`] for "which day",
//! and separate millisecond instants for "when was this written / last edited".
//!
//! ## Representation
//!
//! `Date` is **days since 1970-01-01**, an `i32` — exactly `datetime_core::Date`
//! and `task_core::Date`, so no date arithmetic is reinvented here: every civil
//! calculation delegates to `datetime-core`.
//!
//! ```text
//!   1969-12-31   1970-01-01   1970-01-02        2026-09-23
//!      -1            0            1      …        20719
//! ```
//!
//! Its **wire form** is the ISO string `"YYYY-MM-DD"`, not the integer. That keeps
//! the stored JSON readable, and it is byte-for-byte what the TypeScript app
//! already stores, so its entries import with no conversion.

/// A civil calendar date, stored as days since the Unix epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date(pub i32);

impl Date {
    /// Construct from a civil year/month/day, or `None` if no such day exists
    /// (month 13, 30 February, …).
    pub fn from_ymd(year: i32, month: u32, day: u32) -> Option<Date> {
        datetime_core::Date::from_ymd(year, month, day)
            .ok()
            .map(|d| Date(d.0))
    }

    /// Decompose into `(year, month, day)`.
    pub fn to_ymd(self) -> (i32, u8, u8) {
        datetime_core::Date(self.0).to_ymd()
    }

    /// The year component.
    pub fn year(self) -> i32 {
        self.to_ymd().0
    }

    /// This date shifted by `days` (negative moves backward).
    pub fn add_days(self, days: i32) -> Date {
        Date(datetime_core::Date(self.0).add_days(days).0)
    }

    /// Parse the ISO form `YYYY-MM-DD`.
    ///
    /// Strict on purpose: exactly four year digits, two month digits, two day
    /// digits, hyphen separators, and a day that exists. Strictness here means a
    /// malformed date in an import file is reported rather than silently shifted —
    /// `2026-02-30` is an error, not 2 March.
    pub fn parse_iso(s: &str) -> Option<Date> {
        let b = s.as_bytes();
        if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
            return None;
        }
        let digits = |range: core::ops::Range<usize>| -> Option<u32> {
            let mut v = 0u32;
            for &c in &b[range] {
                if !c.is_ascii_digit() {
                    return None;
                }
                v = v * 10 + u32::from(c - b'0');
            }
            Some(v)
        };
        let year = digits(0..4)?;
        let month = digits(5..7)?;
        let day = digits(8..10)?;
        // `year` is at most 9999, so the cast cannot overflow.
        Date::from_ymd(year as i32, month, day)
    }

    /// Format as ISO `YYYY-MM-DD` (years outside 0..=9999 are written as-is with a
    /// sign, which `parse_iso` will refuse — such dates cannot be created from the
    /// wire in the first place).
    pub fn to_iso(self) -> String {
        let (y, m, d) = self.to_ymd();
        format!("{y:04}-{m:02}-{d:02}")
    }
}

impl core::fmt::Display for Date {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.to_iso())
    }
}

/// Days in `month` of `year` — delegated, so leap years are decided in one place.
pub(crate) fn days_in_month(year: i32, month: u8) -> u8 {
    datetime_core::days_in_month(year, month)
}

#[cfg(feature = "serde")]
impl serde::Serialize for Date {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_iso())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Date {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = <String as serde::Deserialize>::deserialize(deserializer)?;
        Date::parse_iso(&s).ok_or_else(|| {
            serde::de::Error::custom(format!("invalid date {s:?}, expected YYYY-MM-DD"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_is_day_zero() {
        assert_eq!(Date::from_ymd(1970, 1, 1), Some(Date(0)));
        assert_eq!(Date(-1).to_ymd(), (1969, 12, 31));
    }

    #[test]
    fn iso_round_trips() {
        let d = Date::from_ymd(2026, 9, 3).unwrap();
        assert_eq!(d.to_iso(), "2026-09-03");
        assert_eq!(Date::parse_iso("2026-09-03"), Some(d));
        assert_eq!(d.to_string(), "2026-09-03");
    }

    #[test]
    fn parse_rejects_malformed_and_impossible_dates() {
        for bad in [
            "",
            "2026-9-03",
            "2026/09/03",
            "2026-09-3x",
            "20260-09-03",
            "2026-13-01",
            "2026-02-30",
            "2026-00-10",
            "+026-09-03",
            "２０２６-09-03",
        ] {
            assert_eq!(Date::parse_iso(bad), None, "{bad:?} should not parse");
        }
    }

    #[test]
    fn leap_day_exists_only_in_leap_years() {
        assert!(Date::parse_iso("2024-02-29").is_some());
        assert!(Date::parse_iso("2025-02-29").is_none());
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2100, 2), 28);
    }

    #[test]
    fn add_days_crosses_month_and_year() {
        let d = Date::parse_iso("2025-12-31").unwrap();
        assert_eq!(d.add_days(1).to_iso(), "2026-01-01");
        assert_eq!(d.add_days(-31).to_iso(), "2025-11-30");
        assert_eq!(d.year(), 2025);
    }
}
