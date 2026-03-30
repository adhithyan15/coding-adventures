package com.codingadventures.waterwear

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
// ── WEAROS IMPORTS ───────────────────────────────────────────────────────────
// These are from androidx.wear.compose.material3, NOT androidx.compose.material3.
//
// In Wear Compose Material3 alpha25, the scaffold system is split into two:
//   - AppScaffold: app-level container — holds TimeText across all screens
//   - ScreenScaffold: per-screen container — handles scroll indicators, etc.
//
// For a single-screen app with no scrolling, we can use just AppScaffold
// with the content directly inside. This is the recommended approach.
//
// NOTE: ButtonDefaults in alpha25 provides ButtonDefaults.Height (the button
// height Dp value) rather than ButtonDefaults.DefaultButtonSize from older APIs.
// The button height is 52dp; we use that constant directly.
import androidx.wear.compose.material3.AppScaffold
import androidx.wear.compose.material3.Button
import androidx.wear.compose.material3.MaterialTheme
import androidx.wear.compose.material3.Text
import androidx.wear.compose.material3.TimeText
import androidx.wear.compose.material3.TimeTextScope

// Standard button size for a WearOS Button (from ButtonDefaults.Height = 52dp).
// Defined here as a named constant so it's easy to find and adjust.
private val WEAR_BUTTON_SIZE = 52.dp

/**
 * WaterScreen — the single Composable screen for WaterWear.
 *
 * WEAROS UI DESIGN PRINCIPLES:
 *
 *   Round screen (393dp diameter on Galaxy Watch 6 / Pixel Watch 3):
 *     The usable area is a circle, not a rectangle. Content near the corners
 *     is clipped. We use fillMaxSize() with Center alignment to place all
 *     content in the safe central zone.
 *
 *   TimeText at the top:
 *     WearOS convention places the current time at the top edge of every screen
 *     (curved along the circle). TimeText() renders this automatically.
 *     It is placed in AppScaffold's timeText slot.
 *
 *   Large tap targets:
 *     Fingers are ~7mm wide. Apple and Google both recommend ≥48dp tap targets.
 *     52dp is the standard WearOS button height — ergonomically sized for a watch.
 *
 *   Information hierarchy:
 *     1. Total ml (large) — the most important number
 *     2. Glasses count (small, grey) — secondary metric
 *     3. Tap button (centred) — the primary action
 *
 * ALPHA25 SCAFFOLD ARCHITECTURE:
 *   Wear Compose Material3 alpha25 introduces AppScaffold + ScreenScaffold:
 *     - AppScaffold: app-level wrapper, manages the TimeText slot
 *     - ScreenScaffold: per-screen wrapper, manages scroll indicators
 *   For a single non-scrolling screen, AppScaffold alone is sufficient.
 *
 * COMPOSE BASICS (for newcomers):
 *   @Composable: this function describes UI, not a class or widget tree.
 *   Compose re-calls this function (recomposition) whenever `totalMl` changes.
 *   The old UI is discarded and a new one is calculated — very fast because
 *   Compose skips unchanged parts of the tree (smart recomposition).
 *
 * COLLECTASSTATE:
 *   viewModel.todayTotalMl is a StateFlow<Int>. collectAsState() subscribes
 *   to it and returns a Compose State<Int>. Whenever a new value arrives
 *   (triggered by a Room insert), Compose schedules a recomposition of
 *   WaterScreen with the new value — the UI stays live automatically.
 */
@Composable
fun WaterScreen(viewModel: WaterViewModel) {
    // Collect the StateFlow as Compose state.
    // `by` destructures the State<Int> so `totalMl` is an Int, not State<Int>.
    val totalMl by viewModel.todayTotalMl.collectAsState()

    // Daily goal: 2,000ml (8 × 250ml glasses).
    // The goal met condition drives the green colour change — instant visual feedback.
    val goalMl = 2000
    val glassSize = 250

    // coerceIn(0, 8): clamp the glass count so it never shows "9 / 8"
    // if the user drinks more than the goal.
    val filledGlasses = (totalMl / glassSize).coerceIn(0, 8)
    val goalMet = totalMl >= goalMl

    // ── AppScaffold — the WearOS app container ────────────────────────────────
    // AppScaffold from Wear Compose Material3 alpha25 provides:
    //   timeText: the slot for TimeText() at the top edge of the round screen
    //   content: the main area rendered inside the scaffold
    //
    // IMPORTANT: This is androidx.wear.compose.material3.AppScaffold, NOT
    // androidx.compose.material3.Scaffold. They have different APIs.
    //
    // AppScaffold handles WearOS-specific framing: it positions TimeText
    // along the curved top edge and provides system chrome management.
    AppScaffold(
        timeText = {
            // TimeText renders the current time along the top arc of the screen.
            // In alpha25, TimeText requires a content lambda (TimeTextScope DSL).
            // The `time()` call inside the lambda renders the default formatted
            // system time. WearOS updates the clock every minute automatically.
            TimeText {
                time()
            }
        }
    ) {
        // Box with fillMaxSize so the Column can center itself in the full circle.
        Box(
            modifier = Modifier.fillMaxSize(),
            contentAlignment = Alignment.Center
        ) {
            // Column: arrange children vertically in the centre of the screen.
            Column(
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.Center
            ) {
                // ── Total ml ──────────────────────────────────────────────────────
                // Large text: the primary metric the user cares about.
                // Turns green when the daily goal is met — positive reinforcement.
                //
                // MaterialTheme.typography.titleMedium:
                //   Wear typography is separate from phone typography.
                //   titleMedium maps to approximately 20sp on a 393dp round screen —
                //   readable at a glance from a raised wrist position.
                Text(
                    text = "$totalMl ml",
                    style = MaterialTheme.typography.titleMedium,
                    color = if (goalMet) Color(0xFF4CAF50) else MaterialTheme.colorScheme.onSurface
                )

                // ── Glass count ───────────────────────────────────────────────────
                // Secondary metric: shows progress toward the 8-glass goal.
                // Grey text signals it is less important than the ml total.
                Text(
                    text = "$filledGlasses / 8 glasses",
                    style = MaterialTheme.typography.bodySmall,
                    color = Color.Gray
                )

                Spacer(Modifier.height(8.dp))

                // ── Tap button ────────────────────────────────────────────────────
                // The primary action: log one 250ml glass.
                //
                // WEAR_BUTTON_SIZE = 52.dp — matches ButtonDefaults.Height in the
                // Wear Compose spec. Sized for fingertip taps on a small round screen.
                //
                // The water drop emoji is universally understood without a label.
                // On WearOS, text labels on buttons are often omitted to save space.
                Button(
                    onClick = { viewModel.logDrink() },
                    modifier = Modifier.size(WEAR_BUTTON_SIZE)
                ) {
                    Text("\uD83D\uDCA7")  // 💧 water drop emoji (Unicode: U+1F4A7)
                }
            }
        }
    }
}
