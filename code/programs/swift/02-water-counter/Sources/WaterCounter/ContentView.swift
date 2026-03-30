// ContentView.swift
// A single screen with a water intake counter and a log button.
//
// This app introduces the most important concept in SwiftUI: @State.
//
// In SwiftUI, views are value types (structs) — they are created and
// destroyed constantly as the UI updates. @State is how you tell
// SwiftUI "this value belongs to this view and should survive re-draws".
// When a @State variable changes, SwiftUI automatically re-renders the
// parts of the view that depend on it. You never call "reload" or
// "update" manually — you just change the value and the UI follows.
//
//   @State var total = 0
//   Button("Log") { total += 250 }   ← change the value
//   Text("\(total) ml")               ← UI updates automatically

import SwiftUI

struct ContentView: View {

    // @State marks this as view-owned mutable state.
    // The `private` is a convention — state should not be set from outside.
    @State private var totalMl: Int = 0

    // Constants — these will become user settings in a later stage.
    let goalMl    = 2_000
    let servingMl = 250

    // Progress as a fraction between 0 and 1, capped at 1.
    // This drives the progress bar below.
    var progress: Double {
        min(Double(totalMl) / Double(goalMl), 1.0)
    }

    var goalReached: Bool {
        totalMl >= goalMl
    }

    var body: some View {
        VStack(spacing: 24) {

            Spacer()

            // ── Icon ──────────────────────────────────────────────────
            Image(systemName: goalReached ? "drop.fill" : "drop")
                .font(.system(size: 72))
                .foregroundStyle(goalReached ? .blue : .blue.opacity(0.4))
                // .animation tells SwiftUI to interpolate between the two
                // opacity values smoothly rather than jumping instantly.
                .animation(.easeInOut(duration: 0.3), value: goalReached)

            // ── Counter ───────────────────────────────────────────────
            VStack(spacing: 4) {
                // Format the total with a thousands separator for readability.
                Text("\(totalMl.formatted()) ml")
                    .font(.system(size: 52, weight: .bold, design: .rounded))
                    .contentTransition(.numericText())   // animates digit changes

                Text("of \(goalMl.formatted()) ml daily goal")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            }

            // ── Progress bar ──────────────────────────────────────────
            // ProgressView renders as a filled bar when given a `value`.
            ProgressView(value: progress)
                .tint(goalReached ? .green : .blue)
                .scaleEffect(x: 1, y: 2)   // taller bar
                .padding(.horizontal, 32)
                .animation(.easeInOut, value: progress)

            // ── Goal reached message ──────────────────────────────────
            if goalReached {
                Text("Daily goal reached!")
                    .font(.headline)
                    .foregroundStyle(.green)
                    .transition(.scale.combined(with: .opacity))
            }

            Spacer()

            // ── Log button ────────────────────────────────────────────
            // Button takes a label (the view) and an action (a closure).
            // The action runs on the main thread — safe to mutate @State.
            Button {
                withAnimation(.spring(response: 0.3, dampingFraction: 0.6)) {
                    totalMl += servingMl
                }
            } label: {
                Label("Log a Drink", systemImage: "plus.circle.fill")
                    .font(.title3.bold())
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 18)
            }
            .buttonStyle(.borderedProminent)
            .padding(.horizontal, 32)
            .disabled(goalReached)  // grey out once goal is met

            // ── Reset ─────────────────────────────────────────────────
            Button("Reset") {
                withAnimation {
                    totalMl = 0
                }
            }
            .font(.footnote)
            .foregroundStyle(.secondary)
            .padding(.bottom, 32)
        }
    }
}

#Preview {
    ContentView()
}
