// WaterPersistTests.swift
//
// Unit tests for WaterEntry persistence and daily filtering.
//
// All tests use an in-memory ModelContainer so they run fast and leave
// no files on disk. `isStoredInMemoryOnly: true` is the key flag —
// it gives us a real SwiftData stack (same code paths as production)
// without touching the filesystem.

import XCTest
import SwiftData
@testable import WaterPersist

final class WaterPersistTests: XCTestCase {

    // ── Helpers ───────────────────────────────────────────────────────────

    /// Creates a fresh in-memory ModelContainer for each test.
    ///
    /// Using a new container per test ensures tests don't share state —
    /// a fundamental rule of unit testing. Without this, one test's inserts
    /// would pollute the next test's reads.
    private func makeContainer() throws -> ModelContainer {
        let config = ModelConfiguration(isStoredInMemoryOnly: true)
        return try ModelContainer(for: WaterEntry.self, configurations: config)
    }

    /// Inserts a WaterEntry with a specific timestamp into the context.
    private func insert(amountMl: Int = 250,
                        timestamp: Date = Date(),
                        into context: ModelContext) {
        let entry = WaterEntry(amountMl: amountMl)
        entry.timestamp = timestamp
        context.insert(entry)
    }

    // ── Tests ─────────────────────────────────────────────────────────────

    /// A drink logged now should appear in today's entries.
    func testInsertAppearsInTodayEntries() throws {
        let container = try makeContainer()
        let context   = ModelContext(container)

        insert(into: context)

        let all       = try context.fetch(FetchDescriptor<WaterEntry>())
        let today     = all.filter { $0.timestamp >= startOfToday() }

        XCTAssertEqual(today.count, 1, "One entry logged today should be fetchable")
    }

    /// An entry from yesterday should NOT appear in today's total.
    func testYesterdayEntryExcluded() throws {
        let container = try makeContainer()
        let context   = ModelContext(container)

        // Insert an entry timestamped 25 hours ago (safely yesterday)
        let yesterday = Date().addingTimeInterval(-25 * 3600)
        insert(timestamp: yesterday, into: context)

        let all   = try context.fetch(FetchDescriptor<WaterEntry>())
        let today = all.filter { $0.timestamp >= startOfToday() }

        XCTAssertEqual(today.count, 0, "Yesterday's entry must not count toward today")
    }

    /// Three 250ml drinks should sum to 750ml.
    func testTotalSumIsCorrect() throws {
        let container = try makeContainer()
        let context   = ModelContext(container)

        insert(amountMl: 250, into: context)
        insert(amountMl: 250, into: context)
        insert(amountMl: 250, into: context)

        let all   = try context.fetch(FetchDescriptor<WaterEntry>())
        let today = all.filter { $0.timestamp >= startOfToday() }
        let total = today.reduce(0) { $0 + $1.amountMl }

        XCTAssertEqual(total, 750, "Three 250ml drinks should total 750ml")
    }

    /// With no entries, today's total should be 0 — not a crash or nil.
    func testEmptyDayReturnsZero() throws {
        let container = try makeContainer()
        let context   = ModelContext(container)

        let all   = try context.fetch(FetchDescriptor<WaterEntry>())
        let today = all.filter { $0.timestamp >= startOfToday() }
        let total = today.reduce(0) { $0 + $1.amountMl }

        XCTAssertEqual(total, 0, "No entries should give a total of 0, not a crash")
    }
}
