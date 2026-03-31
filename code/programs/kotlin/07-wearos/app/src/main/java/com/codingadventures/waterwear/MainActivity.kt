package com.codingadventures.waterwear

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.wear.compose.material3.MaterialTheme

/**
 * MainActivity — the entry point of WaterWear.
 *
 * WHAT THIS CLASS DOES:
 *   1. Inherits from ComponentActivity (not AppCompatActivity) because we use
 *      Jetpack Compose for all UI. Compose does not need the legacy AppCompat
 *      theme infrastructure.
 *   2. Calls setContent { } to hand control over to Compose.
 *   3. Provides a WearOS MaterialTheme wrapper so all child Composables get
 *      the correct Wear typography, colour scheme, and shapes.
 *   4. Performs one-time setup: creates the notification channel and schedules
 *      all 8 daily alarms.
 *
 * WHAT THIS CLASS DOES NOT DO:
 *   - No UI layout code — that lives in WaterScreen.kt.
 *   - No database access — that lives in WaterRepository.kt.
 *   - No state management — that lives in WaterViewModel.kt.
 *   Keeping MainActivity thin makes each file focused and testable independently.
 *
 * WEAROS ACTIVITY LIFECYCLE:
 *   WearOS activities have the same lifecycle as Android phone activities
 *   (onCreate → onStart → onResume → onPause → onStop → onDestroy).
 *   Additionally, WearOS can enter AMBIENT mode (low-power display) while
 *   the activity is still visible. We don't handle ambient mode here because
 *   Wear Compose handles it automatically via the watch face framework.
 *
 * VIEWMODEL CREATION:
 *   viewModel<WaterViewModel>() is a Compose function that:
 *   1. Creates a WaterViewModel on the first call using the default factory
 *      (which uses the Application context because WaterViewModel is AndroidViewModel).
 *   2. Returns the SAME instance on every recomposition — no data loss.
 *   3. Destroys the ViewModel when the Activity is permanently destroyed (back press).
 */
class MainActivity : ComponentActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // ── One-time notification setup ───────────────────────────────────────
        // These are idempotent: calling them multiple times is safe.
        // The channel must exist before any notification can be shown.
        // Alarms are rescheduled here to survive app updates (which can sometimes
        // clear scheduled alarms, similar to a reboot).
        NotificationHelper.createChannel(this)
        NotificationHelper.scheduleAll(this)

        // ── Compose UI root ───────────────────────────────────────────────────
        // setContent { } replaces the traditional XML layout system with a
        // Compose tree. Everything inside this lambda is Composable.
        setContent {
            // MaterialTheme from androidx.wear.compose.material3 — NOT the phone version.
            // It applies Wear-specific: typography (larger, higher contrast for watches),
            // colour scheme (dark-by-default for AMOLED watch screens),
            // and shape system (rounded components that suit round watches).
            MaterialTheme {
                // viewModel() creates or retrieves the existing WaterViewModel.
                // The ViewModel survives configuration changes and ambient transitions.
                val vm = viewModel<WaterViewModel>()

                // Render the single screen. WaterScreen handles all user interaction.
                WaterScreen(viewModel = vm)
            }
        }
    }
}
