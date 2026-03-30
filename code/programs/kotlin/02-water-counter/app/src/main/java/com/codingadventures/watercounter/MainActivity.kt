// MainActivity.kt
// A water intake counter — the Android counterpart to the Swift version.
//
// This app introduces Compose state with `remember` and `mutableStateOf`.
//
// In Compose, UI functions (@Composable) re-run whenever their inputs
// change. `remember` keeps a value alive across re-compositions (like
// @State in SwiftUI). `mutableStateOf` wraps a value so Compose knows
// to re-run any composable that reads it when it changes.
//
//   var total by remember { mutableStateOf(0) }
//   Button(onClick = { total += 250 }) { }   ← change the value
//   Text("$total ml")                         ← UI re-runs automatically
//
// The `by` keyword uses Kotlin property delegation — it unwraps the
// State<Int> so you can write `total` instead of `total.value`.

package com.codingadventures.watercounter

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.WaterDrop
import androidx.compose.material.icons.outlined.WaterDrop
import androidx.compose.material3.Button
import androidx.compose.material3.Icon
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            WaterCounterTheme {
                Scaffold(modifier = Modifier.fillMaxSize()) { innerPadding ->
                    WaterCounterScreen(modifier = Modifier.padding(innerPadding))
                }
            }
        }
    }
}

@Composable
fun WaterCounterScreen(modifier: Modifier = Modifier) {

    // `remember` keeps this value alive across recompositions.
    // `mutableIntStateOf` is an optimised version of `mutableStateOf`
    // for Int — avoids boxing to Integer.
    // `by` delegates so we write `totalMl` not `totalMl.value`.
    var totalMl by remember { mutableIntStateOf(0) }

    val goalMl    = 2_000
    val servingMl = 250
    val progress  = (totalMl.toFloat() / goalMl).coerceIn(0f, 1f)
    val goalReached = totalMl >= goalMl

    // animateFloatAsState smoothly interpolates to the new progress value
    // each time it changes — equivalent to SwiftUI's .animation modifier.
    val animatedProgress by animateFloatAsState(
        targetValue = progress,
        animationSpec = spring(),
        label = "progress"
    )

    val iconTint by animateColorAsState(
        targetValue = if (goalReached)
            MaterialTheme.colorScheme.primary
        else
            MaterialTheme.colorScheme.primary.copy(alpha = 0.35f),
        label = "iconTint"
    )

    Column(
        modifier = modifier.fillMaxSize().padding(horizontal = 32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center
    ) {

        Spacer(Modifier.weight(1f))

        // ── Icon ──────────────────────────────────────────────────────
        Icon(
            imageVector = if (goalReached) Icons.Filled.WaterDrop
                          else Icons.Outlined.WaterDrop,
            contentDescription = null,
            tint = iconTint,
            modifier = Modifier.size(80.dp)
        )

        Spacer(Modifier.height(24.dp))

        // ── Counter ───────────────────────────────────────────────────
        // AnimatedContent swaps between composables with a transition.
        // slideInVertically/slideOutVertically makes digits roll up,
        // like a mechanical counter — equivalent to .numericText() in SwiftUI.
        AnimatedContent(
            targetState = totalMl,
            transitionSpec = {
                slideInVertically { it } togetherWith slideOutVertically { -it }
            },
            label = "counter"
        ) { displayMl ->
            Text(
                text = "%,d ml".format(displayMl),
                fontSize = 52.sp,
                fontWeight = FontWeight.Bold
            )
        }

        Text(
            text = "of %,d ml daily goal".format(goalMl),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )

        Spacer(Modifier.height(24.dp))

        // ── Progress bar ──────────────────────────────────────────────
        LinearProgressIndicator(
            progress = { animatedProgress },
            modifier = Modifier.fillMaxWidth().height(8.dp),
            color = if (goalReached) MaterialTheme.colorScheme.tertiary
                    else MaterialTheme.colorScheme.primary
        )

        // ── Goal reached message ──────────────────────────────────────
        if (goalReached) {
            Spacer(Modifier.height(12.dp))
            Text(
                text = "Daily goal reached!",
                style = MaterialTheme.typography.titleMedium,
                color = MaterialTheme.colorScheme.tertiary
            )
        }

        Spacer(Modifier.weight(1f))

        // ── Log button ────────────────────────────────────────────────
        Button(
            onClick = { totalMl += servingMl },
            enabled = !goalReached,
            modifier = Modifier.fillMaxWidth().height(56.dp)
        ) {
            Text(
                text = "Log a Drink  +${servingMl}ml",
                style = MaterialTheme.typography.titleMedium
            )
        }

        // ── Reset ─────────────────────────────────────────────────────
        TextButton(onClick = { totalMl = 0 }) {
            Text("Reset", color = MaterialTheme.colorScheme.onSurfaceVariant)
        }

        Spacer(Modifier.height(16.dp))
    }
}

@Preview(showBackground = true)
@Composable
fun WaterCounterScreenPreview() {
    WaterCounterTheme {
        WaterCounterScreen()
    }
}
