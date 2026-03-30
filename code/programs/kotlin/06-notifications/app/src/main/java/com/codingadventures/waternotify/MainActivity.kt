package com.codingadventures.waternotify

import android.Manifest
import android.os.Build
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.activity.viewModels
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp

/**
 * MainActivity — the app's single Activity and entry point.
 *
 * Responsibilities:
 *   1. Request POST_NOTIFICATIONS permission (Android 13+ only)
 *   2. Create the notification channel (idempotent; safe to call every launch)
 *   3. Schedule all 8 daily alarms (idempotent; replaces any existing alarms)
 *   4. Display the WaterScreen composable
 *
 * ═══════════════════════════════════════════════════════════════════
 * ACTIVITY vs. COMPOSABLE — WHERE DOES LOGIC LIVE?
 * ═══════════════════════════════════════════════════════════════════
 * In Jetpack Compose, the Activity is a thin wrapper:
 *   - Activity lifecycle: onCreate, onResume, onDestroy
 *   - Android framework interactions: permissions, intents, window setup
 *   - Everything else lives in Composables and ViewModels
 *
 * MainActivity handles permission requests here (in onCreate) because:
 *   - ActivityResultContracts.RequestPermission is an Activity API
 *   - Compose's `rememberLauncherForActivityResult` ties the launcher
 *     to the composition lifecycle, which starts inside setContent{}
 *
 * ═══════════════════════════════════════════════════════════════════
 * POST_NOTIFICATIONS PERMISSION (Android 13 / API 33)
 * ═══════════════════════════════════════════════════════════════════
 * Before Android 13, notification permission was granted automatically at
 * install time — no prompt needed. Android 13 introduced the explicit
 * POST_NOTIFICATIONS runtime permission to give users finer control.
 *
 * The request flow:
 *   1. App launches → check if permission already granted
 *   2. If not granted on API 33+ → launch the system permission dialog
 *   3. User grants → alarms are already scheduled, notifications will show
 *   4. User denies → show a rationale explaining why notifications help
 *                    (we don't force the request again — respect user choice)
 *
 * On API < 33 (Android 12 and below), POST_NOTIFICATIONS doesn't exist.
 * We skip the request entirely — the permission is always implicitly granted.
 *
 * ═══════════════════════════════════════════════════════════════════
 * EDGE-TO-EDGE DISPLAY
 * ═══════════════════════════════════════════════════════════════════
 * enableEdgeToEdge() makes the app draw behind the status bar and
 * navigation bar (the UI extends to the screen edges). Compose's
 * Scaffold handles the insets, so our content doesn't overlap system UI.
 * This is the recommended approach since Android 14 (API 34).
 *
 * ═══════════════════════════════════════════════════════════════════
 * viewModels() DELEGATE
 * ═══════════════════════════════════════════════════════════════════
 * `by viewModels()` is a Kotlin property delegate. It:
 *   - Creates the ViewModel the first time it's accessed
 *   - Returns the same instance on every access (within this Activity's scope)
 *   - Automatically survives configuration changes (screen rotation)
 *   - Clears the ViewModel when the Activity is permanently finished
 */
class MainActivity : ComponentActivity() {

    private val viewModel: WaterViewModel by viewModels()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        // ── Setup: channel + alarms ──────────────────────────────────────────
        // Both calls are idempotent (safe to call every time onCreate runs).
        // Creating a channel that already exists is a no-op.
        // Scheduling alarms that already exist replaces them (FLAG_UPDATE_CURRENT).
        NotificationHelper.createChannel(this)
        NotificationHelper.scheduleAll(this)

        setContent {
            MaterialTheme {
                // ── Permission request ───────────────────────────────────────
                // rememberLauncherForActivityResult ties the launcher to the
                // composable's lifecycle. When the permission dialog returns,
                // the lambda fires on the main thread.
                val permissionLauncher = rememberLauncherForActivityResult(
                    ActivityResultContracts.RequestPermission()
                ) { isGranted ->
                    // We note the result but take no action — alarms are
                    // already scheduled. If granted, future alarms will fire
                    // notifications. If denied, they'll silently skip (checked
                    // in NotificationReceiver.onReceive).
                    // Logging only in debug builds to avoid log spam in production.
                    android.util.Log.d(
                        "WaterNotify",
                        if (isGranted) "POST_NOTIFICATIONS granted" else "POST_NOTIFICATIONS denied"
                    )
                }

                // Permission state: have we already been granted or denied?
                // We use `remember` so the check only runs on first composition,
                // not on every recomposition.
                var permissionGranted by remember { mutableStateOf(false) }
                var permissionChecked by remember { mutableStateOf(false) }

                // LaunchedEffect(Unit) runs once when this composable enters the
                // composition (equivalent to onResume for a one-shot side effect).
                // On API 33+, we check if permission is granted and request if not.
                LaunchedEffect(Unit) {
                    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                        val checkResult = checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS)
                        if (checkResult == android.content.pm.PackageManager.PERMISSION_GRANTED) {
                            permissionGranted = true
                        } else {
                            // Launch the system dialog. The result comes back to
                            // the lambda in rememberLauncherForActivityResult above.
                            permissionLauncher.launch(Manifest.permission.POST_NOTIFICATIONS)
                        }
                    } else {
                        // API < 33: permission is implicitly granted.
                        permissionGranted = true
                    }
                    permissionChecked = true
                }

                if (!permissionChecked) {
                    // Brief loading state while we check permission on first composition.
                    // In practice this is <1ms, but the state machine is explicit.
                    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                        CircularProgressIndicator()
                    }
                } else if (!permissionGranted && Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                    // Rationale screen: shown if the user denied the permission.
                    // We don't re-request (Google Play policy limits re-requests).
                    // Instead we explain the benefit and let them change it in Settings.
                    PermissionRationaleScreen(
                        onOpenSettings = {
                            // Open the app's notification settings directly.
                            val settingsIntent = android.content.Intent(
                                android.provider.Settings.ACTION_APP_NOTIFICATION_SETTINGS
                            ).apply {
                                putExtra(
                                    android.provider.Settings.EXTRA_APP_PACKAGE,
                                    packageName
                                )
                            }
                            startActivity(settingsIntent)
                        },
                        onContinueAnyway = {
                            // Let them use the app without notifications.
                            permissionGranted = true
                        }
                    )
                } else {
                    // Normal app screen — permission granted or API < 33.
                    WaterScreen(viewModel)
                }
            }
        }
    }
}

/**
 * PermissionRationaleScreen — shown if the user denied POST_NOTIFICATIONS.
 *
 * A good permission rationale:
 *   1. Explains WHY the app needs the permission (in plain language)
 *   2. Describes what the user misses without it
 *   3. Provides a path to grant it later (Settings button)
 *   4. Doesn't block — "Continue Anyway" lets them use the app without it
 *
 * This follows Google's UX guidelines for permission rationale dialogs.
 */
@Composable
private fun PermissionRationaleScreen(
    onOpenSettings: () -> Unit,
    onContinueAnyway: () -> Unit
) {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center
    ) {
        Text("🔔", style = MaterialTheme.typography.displayMedium)
        Spacer(Modifier.height(16.dp))
        Text(
            text = "Enable Notifications",
            style = MaterialTheme.typography.headlineSmall
        )
        Spacer(Modifier.height(12.dp))
        Text(
            text = "WaterNotify sends 8 gentle reminders throughout the day to help you stay hydrated. " +
                   "Without notification permission, you won't receive these reminders.",
            style = MaterialTheme.typography.bodyMedium,
            textAlign = TextAlign.Center
        )
        Spacer(Modifier.height(32.dp))
        Button(
            onClick = onOpenSettings,
            modifier = Modifier.fillMaxWidth()
        ) {
            Text("Open Notification Settings")
        }
        Spacer(Modifier.height(12.dp))
        TextButton(onClick = onContinueAnyway) {
            Text("Continue Without Notifications")
        }
    }
}
