// ContentView.swift
//
// The one and only screen in the WaterWatch app.
//
// Watchos UI philosophy: a single screen, one primary action, readable
// at a glance. The user raises their wrist, sees their progress, taps
// once (or presses the Action Button), lowers their wrist. Done.
//
// Layout:
//   Water drop icon  — immediate visual identity
//   Total ml today   — the number the user cares about
//   Goal progress    — context for that number
//   Log a Drink btn  — full-width, easy to tap even mid-activity

import SwiftUI
import SwiftData
import WatchKit

struct ContentView: View {

    // ── Data ─────────────────────────────────────────────────────────────

    /// Fetches every WaterEntry from the local SwiftData store.
    ///
    /// @Query keeps this in sync automatically: when `logDrink()` inserts
    /// a new entry, SwiftData re-runs the query and SwiftUI redraws the
    /// view. No manual refresh, no NotificationCenter, no Combine needed.
    @Query private var allEntries: [WaterEntry]

    /// The model context is how we write to the database.
    /// Injected automatically from the ModelContainer in WaterWatchApp.
    @Environment(\.modelContext) private var context

    // ── Constants ─────────────────────────────────────────────────────────

    /// Daily water goal in millilitres.
    /// 2,000ml (2 litres) is the commonly cited daily minimum for adults.
    /// A proper settings screen will let users customise this in Foveo.
    private let goalMl = 2_000

    /// Volume added per drink tap.
    /// 250ml ≈ one standard glass of water.
    private let servingMl = 250

    // ── Derived state ─────────────────────────────────────────────────────

    /// All entries logged since midnight today (in the user's local timezone).
    ///
    /// Why compute this here rather than using a @Query predicate?
    /// A @Query predicate with a date is computed once at view init time.
    /// A computed property re-evaluates on every render, which means it
    /// correctly handles the edge case where the app is open at midnight —
    /// the "today" boundary updates on the next button tap or view refresh.
    private var todayEntries: [WaterEntry] {
        let startOfDay = Calendar.current.startOfDay(for: Date())
        return allEntries.filter { $0.timestamp >= startOfDay }
    }

    /// Sum of all drinks logged today, in ml.
    private var todayTotalMl: Int {
        todayEntries.reduce(0) { $0 + $1.amountMl }
    }

    /// Progress as a fraction from 0.0 to 1.0 (capped at 1.0).
    private var progressFraction: Double {
        min(Double(todayTotalMl) / Double(goalMl), 1.0)
    }

    // ── View ──────────────────────────────────────────────────────────────

    var body: some View {
        VStack(spacing: 6) {

            // Water drop icon — SF Symbols scale with Dynamic Type
            Image(systemName: "drop.fill")
                .font(.title2)
                .foregroundStyle(.blue)

            // Today's total — the primary number
            Text("\(todayTotalMl) ml")
                .font(.title3.bold())
                .foregroundStyle(todayTotalMl >= goalMl ? .green : .primary)

            // Goal context
            Text("of \(goalMl) ml")
                .font(.caption2)
                .foregroundStyle(.secondary)

            // Thin progress bar
            ProgressView(value: progressFraction)
                .tint(todayTotalMl >= goalMl ? .green : .blue)
                .padding(.horizontal, 4)

            // Primary action — full width for maximum tap area
            Button(action: logDrink) {
                Text("Log a Drink")
                    .frame(maxWidth: .infinity)
            }
            .buttonStyle(.borderedProminent)
            .tint(.blue)
        }
        .padding(.horizontal, 8)
    }

    // ── Actions ───────────────────────────────────────────────────────────

    /// Logs a 250ml drink: inserts a new WaterEntry and plays a haptic pulse.
    ///
    /// SwiftData autosaves — no explicit save() call is needed. The @Query
    /// above will automatically pick up the new entry and re-render the view.
    ///
    /// The haptic pulse is important: on a Watch, tactile feedback confirms
    /// the action even when the user's eyes aren't on the screen. Especially
    /// critical for the Action Button flow (screen may be off).
    private func logDrink() {
        let entry = WaterEntry(amountMl: servingMl)
        context.insert(entry)
        WKInterfaceDevice.current().play(.success)
    }
}
