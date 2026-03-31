// ContentView.swift  (iPhone)
//
// Clean, minimal water counter. The UI shows only what the user needs:
// total today, progress toward goal, and a single action button.
//
// JOURNAL ARCHITECTURE — why the total can never be wrong after sync:
//
//   Every drink is stored as an immutable record (UUID + timestamp + amountMl).
//   The displayed total is ALWAYS recomputed from those records:
//
//       todayTotal = sum(entries where timestamp >= midnight)
//
//   This means no matter what order Watch entries arrive — offline,
//   out of order, or delivered twice — the total stays correct.
//   The UUID deduplication ensures each physical drink counts once.
//
//   Offline scenario (works automatically):
//     Phone logs 2 drinks (500 ml) while Watch is out of range.
//     Watch logs 3 drinks (750 ml) while offline.
//     On reconnect → all 5 entries merge → total = 1250 ml. ✓
//
//   Future: every stored entry maps directly to an HKQuantitySample for
//   Apple Health export — no data conversion needed.

import SwiftUI
import SwiftData

struct ContentView: View {

    // ── Data ─────────────────────────────────────────────────────────────

    // @Query fetches ALL entries so we can deduplicate by UUID during sync,
    // even for entries from previous days.
    @Query private var allEntries: [WaterEntry]
    @Environment(\.modelContext) private var context

    private let connectivity = ConnectivityManager.shared
    private let goalMl       = 2_000
    private let servingMl    = 250

    // ── Derived ───────────────────────────────────────────────────────────

    private var todayEntries: [WaterEntry] {
        let midnight = Calendar.current.startOfDay(for: Date())
        return allEntries.filter { $0.timestamp >= midnight }
    }

    private var todayTotalMl: Int {
        // Always sum the journal — never a stored running total.
        // This is the design decision that makes offline sync correct.
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

            Image(systemName: "drop.fill")
                .font(.system(size: 60))
                .foregroundStyle(goalMet ? .green : .blue)

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

            VStack(spacing: 6) {
                ProgressView(value: progressFraction)
                    .tint(goalMet ? .green : .blue)
                    .animation(.easeInOut(duration: 0.3), value: progressFraction)

                Text("\(todayTotalMl) of \(goalMl) ml")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            .padding(.horizontal)

            Button(action: logDrink) {
                VStack(spacing: 2) {
                    Text(goalMet ? "Log Another" : "Log a Drink")
                        .font(.headline)
                    Text("+\(servingMl) ml")
                        .font(.caption).opacity(0.8)
                }
                .frame(maxWidth: .infinity)
                .padding()
            }
            .buttonStyle(.borderedProminent)
            .tint(goalMet ? .green : .blue)
            .padding(.horizontal)

            syncStatusBadge

            Spacer()
        }
        .onAppear { wireConnectivity() }
        .animation(.default, value: goalMet)
    }

    // ── Sync status badge ─────────────────────────────────────────────────

    @ViewBuilder
    private var syncStatusBadge: some View {
        HStack(spacing: 4) {
            Image(systemName: connectivity.isWatchReachable
                  ? "applewatch.radiowaves.left.and.right"
                  : "applewatch")
            Text(connectivity.isWatchReachable
                 ? "Watch connected"
                 : "Watch not reachable — syncs when back in range")
        }
        .font(.caption2)
        .foregroundStyle(connectivity.isWatchReachable ? .green : .secondary)
        .multilineTextAlignment(.center)
        .padding(.horizontal)
    }

    // ── Actions ───────────────────────────────────────────────────────────

    private func wireConnectivity() {
        connectivity.onReceiveEntry = { payload in
            insertIfAbsent(payload)
        }
    }

    /// Log a drink locally and enqueue it for the Watch.
    private func logDrink() {
        let entry = WaterEntry(amountMl: servingMl)
        context.insert(entry)
        connectivity.send(entry)       // queued, guaranteed delivery
    }

    /// Insert only if this UUID isn't already in the local store.
    ///
    /// Idempotent: safe to call multiple times with the same payload.
    /// The entry preserves the ORIGINAL timestamp from the Watch so the
    /// journal reflects when the user actually drank, not when it synced.
    private func insertIfAbsent(_ payload: SyncPayload) {
        let alreadyExists = allEntries.contains { $0.id == payload.id }
        guard !alreadyExists else { return }

        let entry       = WaterEntry(amountMl: payload.amountMl)
        entry.id        = payload.id
        entry.timestamp = payload.timestamp   // preserve original drink time
        context.insert(entry)
    }
}
