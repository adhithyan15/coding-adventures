// GlassesView.swift
//
// Displays the daily hydration goal as a row of glass icons.
//
// PURE DISPLAY COMPONENT:
//   GlassesView has no SwiftData dependency. It takes two integers and
//   renders icons. This separation makes it:
//     - independently testable (no ModelContainer needed)
//     - reusable (future watchOS or widget extension can use it)
//     - readable (all display logic in one small file)
//
// ICON CHOICE:
//   SF Symbols "drop.fill" (filled) and "drop" (empty) maintain visual
//   continuity with Stages 04 and 05. No custom image assets needed.
//
// RESPONSIVE LAYOUT:
//   LazyVGrid with adaptive columns automatically wraps on narrow devices
//   (iPhone SE: 375pt wide → 2 rows of 4) without explicit breakpoints.

import SwiftUI

struct GlassesView: View {

    /// Number of filled (logged) glasses. Caller ensures 0 ≤ filledCount ≤ total.
    let filledCount: Int

    /// Total glasses in the goal. Default 8 (= 2,000 ml at 250 ml/glass).
    let total: Int

    /// Tracks which glass was most recently filled for the pop animation.
    @State private var animatedIndex: Int? = nil

    var body: some View {
        // Guard against degenerate inputs. total < 0 would crash the range
        // in ForEach; total == 0 produces an empty view (safe but unusual).
        if total > 0 {
            glassGrid
        }
    }

    // Extracted so the guard above keeps `body` readable.
    @ViewBuilder
    private var glassGrid: some View {
        // Adaptive columns: each icon is at least 30pt wide.
        // On a 390pt-wide screen (iPhone 14): 8 icons fit in one row.
        // On a 375pt-wide screen (iPhone SE): 8 icons still fit, just snugger.
        let columns = [GridItem(.adaptive(minimum: 30), spacing: 8)]

        LazyVGrid(columns: columns, spacing: 8) {
            ForEach(0..<total, id: \.self) { index in
                let isFilled = index < filledCount

                Image(systemName: isFilled ? "drop.fill" : "drop")
                    .font(.system(size: 28))
                    .foregroundStyle(iconColor(filled: isFilled))
                    .scaleEffect(animatedIndex == index ? 1.25 : 1.0)
                    .animation(
                        .spring(response: 0.3, dampingFraction: 0.5),
                        value: animatedIndex
                    )
            }
        }
        // Trigger the pop animation whenever filledCount increases.
        .onChange(of: filledCount) { oldValue, newValue in
            // newValue >= 1 guard prevents underflow on newValue - 1 when
            // newValue is 0. Without this, 0 - 1 traps in debug and wraps
            // to Int.max in release, corrupting animatedIndex.
            guard newValue > oldValue, newValue >= 1, newValue - 1 < total else { return }
            let newlyFilled = newValue - 1
            animatedIndex = newlyFilled
            // Reset after the spring settles (~0.6s)
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.6) {
                animatedIndex = nil
            }
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────

    private func iconColor(filled: Bool) -> Color {
        guard filled else { return .secondary }
        // All filled → celebrate with green; otherwise the standard blue.
        return filledCount >= total ? .green : .blue
    }
}

// ── Preview ───────────────────────────────────────────────────────────────────

#Preview("3 of 8") {
    GlassesView(filledCount: 3, total: 8)
        .padding()
}

#Preview("Goal met") {
    GlassesView(filledCount: 8, total: 8)
        .padding()
}

#Preview("Empty") {
    GlassesView(filledCount: 0, total: 8)
        .padding()
}
