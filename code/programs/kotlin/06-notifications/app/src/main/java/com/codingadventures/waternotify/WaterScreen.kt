package com.codingadventures.waternotify

import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

/**
 * WaterScreen.kt — the main screen of WaterNotify.
 *
 * This Composable displays:
 *   1. The GlassesRow — 8 glass icons (💧 filled / 🫙 empty) showing visual progress
 *   2. "X of 8 glasses" count label
 *   3. "Y ml of 2000 ml" numeric label
 *   4. A "Log a Drink (+250 ml)" button
 *   5. A small status line noting notifications are scheduled
 *
 * ═══════════════════════════════════════════════════════════════════
 * COMPOSE RECOMPOSITION MODEL
 * ═══════════════════════════════════════════════════════════════════
 * Compose uses a declarative model: you describe what the UI *should look like*
 * given the current state, and Compose figures out what changed and redraws
 * only the affected parts.
 *
 * WaterScreen receives [viewModel] as a parameter. The ViewModel exposes
 * `todayTotalMl: StateFlow<Int>`. We read it via `collectAsState()`, which:
 *   1. Subscribes to the StateFlow inside a Compose snapshot
 *   2. Triggers recomposition whenever a new value is emitted
 *   3. Unsubscribes automatically when WaterScreen leaves the composition
 *
 * When the user taps "Log a Drink":
 *   viewModel.logDrink() → Room INSERT → Flow emits new total → recomposition
 *
 * The glass icons, the labels, and the button colour all update in a single
 * recomposition frame — typically <16ms, invisible to the user.
 *
 * ═══════════════════════════════════════════════════════════════════
 * SCAFFOLD — MATERIAL DESIGN LAYOUT SKELETON
 * ═══════════════════════════════════════════════════════════════════
 * Scaffold provides Material Design's standard page structure:
 *   - topBar (not used here)
 *   - bottomBar (not used here)
 *   - content (our actual UI) — receives padding insets automatically
 *
 * The `padding` parameter inside Scaffold's content lambda contains the system
 * insets (status bar height, navigation bar height). Applying it to our Column
 * prevents content from appearing behind the status bar on edge-to-edge displays.
 */
@Composable
fun WaterScreen(viewModel: WaterViewModel) {
    // collectAsState() subscribes to the StateFlow and returns a State<Int>.
    // `by` delegates property access to the State so `totalMl` reads as Int
    // directly rather than `totalMl.value`.
    val totalMl by viewModel.todayTotalMl.collectAsState()

    val goalMl = 2000
    val filledGlasses = (totalMl / ML_PER_GLASS).coerceAtMost(TOTAL_GLASSES)
    val goalMet = totalMl >= goalMl

    // Goal-met colour: a celebratory green when the user hits 2000 ml.
    // Using a hardcoded colour here is fine for a learning project; in a production
    // app you'd define this in a MaterialTheme ColorScheme.
    val achievementColor = Color(0xFF4CAF50) // Material Green 500

    Scaffold { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(horizontal = 24.dp, vertical = 32.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(16.dp, Alignment.CenterVertically)
        ) {

            // ── Title ──────────────────────────────────────────────────────────
            Text(
                text = "💧 WaterNotify",
                fontSize = 28.sp,
                fontWeight = FontWeight.Bold,
                color = MaterialTheme.colorScheme.primary
            )

            Spacer(Modifier.height(8.dp))

            // ── Glasses row ────────────────────────────────────────────────────
            // The visual representation: 8 glasses, filled left-to-right.
            GlassesRow(totalMl = totalMl)

            // ── Glasses count label ────────────────────────────────────────────
            // e.g. "3 of 8 glasses"
            Text(
                text = "$filledGlasses of $TOTAL_GLASSES glasses",
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.SemiBold,
                color = if (goalMet) achievementColor else MaterialTheme.colorScheme.onSurface
            )

            // ── Millilitre label ───────────────────────────────────────────────
            // e.g. "750 ml of 2000 ml"
            Text(
                text = "$totalMl ml of $goalMl ml",
                style = MaterialTheme.typography.bodyMedium,
                color = Color.Gray
            )

            // ── Goal met banner ────────────────────────────────────────────────
            // Only visible when the user has reached their goal.
            if (goalMet) {
                Text(
                    text = "🎉 Daily goal reached!",
                    style = MaterialTheme.typography.bodyLarge,
                    fontWeight = FontWeight.Bold,
                    color = achievementColor
                )
            }

            Spacer(Modifier.height(8.dp))

            // ── Log drink button ───────────────────────────────────────────────
            // Full-width button, 56 dp tall — Material Design's recommended touch
            // target height for primary actions.
            Button(
                onClick = { viewModel.logDrink() },
                modifier = Modifier
                    .fillMaxWidth()
                    .height(56.dp),
                colors = ButtonDefaults.buttonColors(
                    containerColor = if (goalMet) achievementColor else MaterialTheme.colorScheme.primary
                )
            ) {
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Text(
                        text = if (goalMet) "Log Another" else "Log a Drink",
                        fontWeight = FontWeight.Bold
                    )
                    Text(
                        text = "+250 ml",
                        style = MaterialTheme.typography.bodySmall
                    )
                }
            }

            // ── Status line ────────────────────────────────────────────────────
            Text(
                text = "🔔 8 daily reminders scheduled",
                style = MaterialTheme.typography.labelSmall,
                color = Color.Gray
            )
        }
    }
}
