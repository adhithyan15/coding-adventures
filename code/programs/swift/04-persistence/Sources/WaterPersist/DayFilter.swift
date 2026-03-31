// DayFilter.swift
//
// Date helpers for filtering "today's" water entries.
//
// Why a separate file? The midnight boundary calculation is used in both
// ContentView and the test suite. Keeping it here avoids duplication and
// makes the intent explicit — this is the single source of truth for
// what "today" means in this app.

import Foundation

/// Returns the start of the current calendar day (midnight) in the
/// user's local timezone.
///
/// Example: if it's 14:32 on March 30, this returns March 30 at 00:00:00.
///
/// Used to filter water entries: any entry with `timestamp >= startOfToday()`
/// is counted toward today's total. Entries from yesterday or earlier are
/// silently excluded without being deleted — they become the history log.
func startOfToday() -> Date {
    Calendar.current.startOfDay(for: Date())
}

/// Returns the start of the calendar day containing `date`.
///
/// Used in tests to create entries for specific days without relying
/// on the current system time.
func startOfDay(for date: Date) -> Date {
    Calendar.current.startOfDay(for: date)
}

/// Returns a date that is `days` calendar days before today.
///
/// Passing -1 returns yesterday, -2 returns two days ago, etc.
/// Used in tests to verify that old entries are correctly excluded.
func daysAgo(_ days: Int) -> Date {
    Calendar.current.date(byAdding: .day, value: -days, to: Date()) ?? Date()
}
