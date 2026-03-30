// ContentView.swift
//
// The main screen. Clean and minimal — self-care apps should get out of
// the user's way, not demand their attention.
//
// Layout (top to bottom):
//   1. GlassesView  — 8 drop icons, filled left-to-right as drinks are logged
//   2. Progress label — "3 of 8 glasses" + "750 ml of 2,000 ml"
//   3. Log a Drink button
//   4. Next reminder label — "Next reminder in 1h 23m"
//   5. Notification denied banner (only if permission was refused)
//
// DATA FLOW:
//   @Query fetches all WaterEntry records from SwiftData.
//   todayEntries filters to current calendar day.
//   todayTotalMl sums amountMl across todayEntries.
//   filledGlassCount divides by glassSize (integer division — no partial fill).
//   These are all computed properties — no mutable state except the DB.

import SwiftUI
import SwiftData

struct ContentView: View {

    // ── Constants ─────────────────────────────────────────────────────────

    /// One standard glass = 250 ml (IOM DRI definition).
    /// All glass ↔ ml conversions go through this single constant.
    private let glassSize   = 250
    private let goalGlasses = 8                      // 8 × 250 = 2,000 ml

    // ── Data ─────────────────────────────────────────────────────────────

    @Query private var allEntries: [WaterEntry]
    @Environment(\.modelContext) private var context

    private let notifications = NotificationManager.shared

    // ── Derived ───────────────────────────────────────────────────────────

    private var todayEntries: [WaterEntry] {
        let midnight = Calendar.current.startOfDay(for: Date())
        return allEntries.filter { $0.timestamp >= midnight }
    }

    private var todayTotalMl: Int {
        // Always recompute from the journal — never a stored counter.
        // This makes the total safe across timezone changes and DST.
        todayEntries.reduce(0) { $0 + $1.amountMl }
    }

    private var filledGlassCount: Int {
        // Integer division: 750 / 250 = 3 (not 3.0 or 3.2).
        // A glass is either logged or not — no partial fills in this stage.
        min(todayTotalMl / glassSize, goalGlasses)
    }

    private var goalMl: Int { goalGlasses * glassSize }
    private var goalMet: Bool { filledGlassCount >= goalGlasses }

    // ── View ──────────────────────────────────────────────────────────────

    var body: some View {
        VStack(spacing: 28) {
            Spacer()

            // Glasses grid — the visual centrepiece of the screen
            GlassesView(filledCount: filledGlassCount, total: goalGlasses)
                .padding(.horizontal)

            // Progress labels
            VStack(spacing: 4) {
                Text(goalMet ? "Goal met!" : "\(filledGlassCount) of \(goalGlasses) glasses")
                    .font(.title2.weight(.semibold))
                    .foregroundStyle(goalMet ? .green : .primary)
                    .contentTransition(.numericText())
                    .animation(.spring(duration: 0.4), value: filledGlassCount)

                // Secondary ml label: keeps the user aware of the underlying
                // unit that will sync to Apple Health in a future stage.
                Text("\(todayTotalMl) ml of \(goalMl) ml")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
                    .contentTransition(.numericText())
                    .animation(.spring(duration: 0.4), value: todayTotalMl)
            }

            // Action button
            Button(action: logDrink) {
                VStack(spacing: 2) {
                    Text(goalMet ? "Log Another" : "Log a Drink")
                        .font(.headline)
                    Text("+1 glass  (+\(glassSize) ml)")
                        .font(.caption).opacity(0.8)
                }
                .frame(maxWidth: .infinity)
                .padding()
            }
            .buttonStyle(.borderedProminent)
            .tint(goalMet ? .green : .blue)
            .padding(.horizontal)

            // Next reminder time — passive, small, not nagging
            nextReminderLabel

            // Notification denied banner — only shown when permission refused
            if notifications.authStatus == .denied {
                notificationDeniedBanner
            }

            Spacer()
        }
        .animation(.default, value: goalMet)
        .animation(.default, value: notifications.authStatus)
    }

    // ── Next reminder label ───────────────────────────────────────────────

    @ViewBuilder
    private var nextReminderLabel: some View {
        if let next = notifications.nextReminderTime {
            HStack(spacing: 4) {
                Image(systemName: "bell")
                Text("Next reminder ")
                + Text(next, style: .relative)
                    .foregroundStyle(.secondary)
            }
            .font(.caption2)
            .foregroundStyle(.secondary)
        }
    }

    // ── Notification denied banner ────────────────────────────────────────
    //
    // Shown only when the user explicitly denied notification permission.
    // Tapping it opens the iOS Settings app directly to this app's page.
    // It is not an alert — it lives inline and is easy to dismiss by scrolling
    // past it. We do not interrupt the user with an alert sheet.

    @ViewBuilder
    private var notificationDeniedBanner: some View {
        Button {
            if let url = URL(string: UIApplication.openSettingsURLString) {
                UIApplication.shared.open(url)
            }
        } label: {
            HStack(spacing: 8) {
                Image(systemName: "bell.slash")
                VStack(alignment: .leading, spacing: 2) {
                    Text("Reminders disabled")
                        .font(.caption.weight(.semibold))
                    Text("Tap to open Settings and enable notifications")
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                }
                Spacer()
                Image(systemName: "chevron.right")
                    .font(.caption2)
                    .foregroundStyle(.tertiary)
            }
            .padding(12)
            .background(.orange.opacity(0.12), in: RoundedRectangle(cornerRadius: 10))
            .foregroundStyle(.orange)
        }
        .padding(.horizontal)
    }

    // ── Actions ───────────────────────────────────────────────────────────

    private func logDrink() {
        let entry = WaterEntry(amountMl: glassSize)
        context.insert(entry)
        // Update "next reminder" label in case the user just logged their
        // last drink for the day (the 21:00 banner says "stop here").
        notifications.updateNextReminderTime()
    }
}
