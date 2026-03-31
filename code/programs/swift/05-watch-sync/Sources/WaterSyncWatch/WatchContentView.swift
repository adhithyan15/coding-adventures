// WatchContentView.swift  (Watch)
//
// Minimal water counter for Apple Watch.
// Logging a drink stores it locally AND sends it to the iPhone.
// Receiving an entry from iPhone inserts it if not already present.
//
// The UI shows only what matters on a glance-sized screen:
// total, progress bar, and one action button.
//
// Journal architecture (same as iPhone):
//   Every drink = UUID + timestamp + amountMl stored in SwiftData.
//   Total = sum of today's records. Offline entries sync as union merge.
//   Future: each entry maps to HKQuantitySample for Apple Health export.
//
// Haptic feedback confirms every log — important when the screen
// may be off (Action Button flow).

import SwiftUI
import SwiftData
import WatchKit

struct WatchContentView: View {

    @Query private var allEntries: [WaterEntry]
    @Environment(\.modelContext) private var context

    private let connectivity = WatchConnectivityManager.shared
    private let goalMl       = 2_000
    private let servingMl    = 250

    private var todayEntries: [WaterEntry] {
        let midnight = Calendar.current.startOfDay(for: Date())
        return allEntries.filter { $0.timestamp >= midnight }
    }

    private var todayTotalMl: Int {
        todayEntries.reduce(0) { $0 + $1.amountMl }
    }

    private var progressFraction: Double {
        min(Double(todayTotalMl) / Double(goalMl), 1.0)
    }

    private var goalMet: Bool { todayTotalMl >= goalMl }

    var body: some View {
        VStack(spacing: 6) {

            Image(systemName: "drop.fill")
                .font(.title2)
                .foregroundStyle(goalMet ? .green : .blue)

            Text("\(todayTotalMl) ml")
                .font(.title3.bold())
                .foregroundStyle(goalMet ? .green : .primary)
                .contentTransition(.numericText())
                .animation(.spring(duration: 0.3), value: todayTotalMl)

            Text("of \(goalMl) ml")
                .font(.caption2)
                .foregroundStyle(.secondary)

            ProgressView(value: progressFraction)
                .tint(goalMet ? .green : .blue)
                .padding(.horizontal, 4)

            Button(action: logDrink) {
                Text("Log a Drink")
                    .frame(maxWidth: .infinity)
            }
            .buttonStyle(.borderedProminent)
            .tint(goalMet ? .green : .blue)

            // Phone reachability indicator
            Image(systemName: connectivity.isPhoneReachable
                  ? "iphone.radiowaves.left.and.right"
                  : "iphone.slash")
                .font(.caption2)
                .foregroundStyle(connectivity.isPhoneReachable ? .green : .secondary)
        }
        .padding(.horizontal, 8)
        .onAppear { wireConnectivity() }
    }

    // ── Actions ───────────────────────────────────────────────────────────

    private func wireConnectivity() {
        connectivity.onReceiveEntry = { payload in
            insertIfAbsent(payload)
        }
    }

    private func logDrink() {
        let entry = WaterEntry(amountMl: servingMl)
        context.insert(entry)
        connectivity.send(entry)
        WKInterfaceDevice.current().play(.success)
    }

    private func insertIfAbsent(_ payload: SyncPayload) {
        let alreadyExists = allEntries.contains { $0.id == payload.id }
        guard !alreadyExists else { return }

        let entry       = WaterEntry(amountMl: payload.amountMl)
        entry.id        = payload.id
        entry.timestamp = payload.timestamp   // preserve original drink time
        context.insert(entry)
    }
}
