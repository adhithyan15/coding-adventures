package com.codingadventures.waternotify

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

/**
 * GlassesRow.kt — the "glasses" visualisation for daily water intake.
 *
 * This file contains a single Composable that renders a horizontal row of
 * 8 glass icons. Each glass is either:
 *   💧  (water droplet) — filled (drunk)
 *   🫙  (jar)           — empty (not yet drunk)
 *
 * The number of filled glasses is derived from the total ml: every 250 ml
 * fills one glass (so 2000 ml = 8 glasses fully filled).
 *
 * ═══════════════════════════════════════════════════════════════════
 * WHY LAZYROW?
 * ═══════════════════════════════════════════════════════════════════
 * LazyRow is Compose's equivalent of a horizontal RecyclerView. "Lazy" means
 * items are only composed when they enter the visible viewport — for 8 items
 * this doesn't matter (we could use Row directly), but LazyRow is the idiomatic
 * Compose approach for horizontally scrollable lists.
 *
 * For a fixed, short list like this, a regular Row with forEach would also work.
 * We use LazyRow to demonstrate the API and because it handles spacing
 * uniformly via contentPadding and horizontalArrangement.
 *
 * ═══════════════════════════════════════════════════════════════════
 * EMOJI ICONS — WHY NOT VECTOR DRAWABLES?
 * ═══════════════════════════════════════════════════════════════════
 * Unicode emoji are rendered by the system font (NotoColorEmoji on Android).
 * They're:
 *   - Zero-dependency (no drawable assets to maintain)
 *   - Expressive and universally recognisable
 *   - Automatically high-DPI (font rendering, not pixel bitmaps)
 *
 * The downside is Android renders emoji slightly differently across OS versions.
 * For a production app you'd use custom vector drawables. For this learning
 * project, emoji perfectly convey the concept with minimal code.
 *
 * TOTAL_GLASSES = 8:
 * Each glass = 250 ml × 8 = 2000 ml daily goal.
 * This matches the iOS app's glasses view and the World Health Organization's
 * commonly cited "8 glasses a day" recommendation.
 */

/** How many glasses represent the full daily goal. Matches 8 × 250 ml = 2000 ml. */
const val TOTAL_GLASSES = 8

/** Ml per glass. One standard drinking glass ≈ 250 ml (8 fl oz). */
const val ML_PER_GLASS = 250

/**
 * A horizontal row of [TOTAL_GLASSES] glass icons.
 *
 * @param totalMl  The user's water intake so far today (from Room via ViewModel).
 *                 We derive how many glasses are filled: floor(totalMl / ML_PER_GLASS),
 *                 clamped to [0, TOTAL_GLASSES].
 */
@Composable
fun GlassesRow(totalMl: Int) {
    // How many complete glasses has the user drunk?
    // Integer division naturally floors: 750 ml / 250 = 3 glasses.
    // coerceAtMost(TOTAL_GLASSES) prevents showing 9+ glasses if the user
    // logs more than 2000 ml (encouraged but doesn't overflow the UI).
    val filledCount = (totalMl / ML_PER_GLASS).coerceAtMost(TOTAL_GLASSES)

    // Build a list of booleans: true = filled, false = empty.
    // indices 0..<filledCount are filled, the rest empty.
    // This list drives the LazyRow items below.
    val glasses: List<Boolean> = List(TOTAL_GLASSES) { index -> index < filledCount }

    LazyRow(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(4.dp),
        contentPadding = PaddingValues(horizontal = 8.dp),
        // Centre items vertically within the row
        verticalAlignment = Alignment.CenterVertically
    ) {
        itemsIndexed(glasses) { _, isFilled ->
            // Each glass is rendered as a Text composable showing the emoji.
            // Using Text for emoji is the simplest approach — Compose renders
            // Text with the system font which includes all Unicode emoji.
            Text(
                text = if (isFilled) "💧" else "🫙",
                fontSize = 32.sp,
                // Filled glasses use the primary theme colour for a11y; empty
                // glasses use a dimmed grey to visually indicate "remaining".
                // (Color has no effect on emoji rendering but affects any text
                //  fallback if the emoji glyph is unavailable.)
                color = if (isFilled) MaterialTheme.colorScheme.primary else Color.Gray
            )
        }
    }
}
