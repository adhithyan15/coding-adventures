// ContentView.swift
//
// The main screen of the WaterPersist app.
//
// Identical layout to Stage 02 (water counter) but backed by SwiftData
// instead of @State. The key difference: close the app, reopen it, and
// the counter is exactly where you left it.
//
// The "Saved locally ✓" badge at the bottom makes persistence visible
// to the user — and to the developer testing it for the first time.

import SwiftUI
import SwiftData

struct ContentView: View {

    // ── Data ─────────────────────────────────────────────────────────────

    /// All WaterEntry records from the local SQLite database.
    ///
    /// @Query is a SwiftData property wrapper that:
    /// 1. Fetches all matching records on first render
    /// 2. Subscribes to the database for changes
    /// 3. Re-renders the view automatically when records are inserted/deleted
    ///
    /// There is no manual "refresh" needed. SwiftData does it automatically.
    @Query private var allEntries: [WaterEntry]

    /// The model context is the gateway to read/write the database.
    /// It's injected automatically from the ModelContainer in WaterPersistApp.
    @Environment(\.modelContext) private var context

    // ── Constants ─────────────────────────────────────────────────────────

    private let goalMl   = 2_000   // daily goal: 2 litres
    private let servingMl = 250    // one tap = one glass

    // ── Derived state ─────────────────────────────────────────────────────

    /// Entries logged since midnight today.
    ///
    /// Computed fresh on every render so the "today" boundary updates
    /// correctly even if the app was open when midnight passed.
    private var todayEntries: [WaterEntry] {
        allEntries.filter { $0.timestamp >= startOfToday() }
    }

    private var todayTotalMl: Int {
        todayEntries.reduce(0) { $0 + $1.amountMl }
    }

    private var progressFraction: Double {
        min(Double(todayTotalMl) / Double(goalMl), 1.0)
    }

    private var goalMet: Bool { todayTotalMl >= goalMl }

    // ── View ──────────────────────────────────────────────────────────────

    var body: some View {
        VStack(spacing: 24) {
            Spacer()

            // Water drop icon
            Image(systemName: "drop.fill")
                .font(.system(size: 60))
                .foregroundStyle(goalMet ? .green : .blue)

            // Today's total
            VStack(spacing: 4) {
                Text("\(todayTotalMl) ml")
                    .font(.system(size: 48, weight: .bold, design: .rounded))
                    .foregroundStyle(goalMet ? .green : .primary)
                    .contentTransition(.numericText())
                    .animation(.spring(duration: 0.4), value: todayTotalMl)

                Text("today")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            }

            // Progress bar
            VStack(spacing: 6) {
                ProgressView(value: progressFraction)
                    .tint(goalMet ? .green : .blue)
                    .animation(.easeInOut(duration: 0.3), value: progressFraction)

                Text("\(todayTotalMl) of \(goalMl) ml")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            .padding(.horizontal)

            // Log button
            Button(action: logDrink) {
                VStack(spacing: 2) {
                    Text(goalMet ? "Log Another" : "Log a Drink")
                        .font(.headline)
                    Text("+\(servingMl) ml")
                        .font(.caption)
                        .opacity(0.8)
                }
                .frame(maxWidth: .infinity)
                .padding()
            }
            .buttonStyle(.borderedProminent)
            .tint(goalMet ? .green : .blue)
            .padding(.horizontal)

            // Persistence badge — makes it obvious the data is saved locally
            Label("Saved locally", systemImage: "internaldrive.fill")
                .font(.caption2)
                .foregroundStyle(.secondary)

            Spacer()
        }
        .animation(.default, value: goalMet)
    }

    // ── Actions ───────────────────────────────────────────────────────────

    /// Inserts a new WaterEntry into the local SwiftData database.
    ///
    /// SwiftData autosaves — no explicit save() call is needed.
    /// The @Query above will pick up the new entry and re-render the view
    /// on the very next run loop tick.
    ///
    /// To verify persistence: log a drink, kill the app (swipe up in app
    /// switcher or use `xcrun simctl terminate`), reopen it. The counter
    /// should show the same value it had before the kill.
    private func logDrink() {
        let entry = WaterEntry(amountMl: servingMl)
        context.insert(entry)
    }
}
