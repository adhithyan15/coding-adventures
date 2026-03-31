package com.codingadventures.waternotify

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch

/**
 * WaterViewModel — the UI-facing data holder for the water tracking screen.
 *
 * VIEWMODEL ROLE IN ANDROID ARCHITECTURE:
 * ─────────────────────────────────────────
 * The ViewModel sits between the UI (Composables) and the data layer
 * (Repository + Room). It:
 *   1. Survives configuration changes (screen rotation, dark/light mode switch)
 *      — Activities and Composables are destroyed and recreated, but the
 *        ViewModel is NOT destroyed. This prevents data loss on rotation.
 *   2. Exposes only what the UI needs (StateFlow<Int>, fun logDrink())
 *      rather than leaking repository internals.
 *   3. Owns the coroutine scope (viewModelScope) — launched coroutines are
 *      automatically cancelled when the ViewModel is cleared (user leaves the
 *      app), preventing memory leaks.
 *
 * WHY AndroidViewModel INSTEAD OF ViewModel?
 * ───────────────────────────────────────────
 * AndroidViewModel receives the Application object, which we pass to the
 * Repository. We need it to build the Room database (needs a Context) and
 * to schedule alarms (needs Context for getSystemService). Using Application
 * (not Activity) is safe — it's never recreated, so no leak risk.
 *
 * FLOW → STATEFLOW CONVERSION:
 * ─────────────────────────────
 * Room returns Flow<Int> — a cold stream that only emits when collected.
 * Compose needs StateFlow<Int> — a hot stream that always holds the latest
 * value so new collectors get it immediately (no waiting for the next DB event).
 *
 * stateIn() converts cold → hot:
 *   scope          = viewModelScope   (the flow lives as long as the ViewModel)
 *   started        = SharingStarted.Eagerly  (start collecting immediately,
 *                                    not lazily — so today's total loads
 *                                    even before the UI subscribes)
 *   initialValue   = 0               (shown while the DB query is running)
 *
 * Compose's `collectAsState()` then reads this StateFlow and recomposes
 * the UI automatically on every emission.
 */
class WaterViewModel(app: Application) : AndroidViewModel(app) {
    private val repo = WaterRepository(app)

    /**
     * Today's total water intake in ml.
     * Compose observes this with `collectAsState()` — any insert triggers a recomposition.
     */
    val todayTotalMl: StateFlow<Int> = repo.todayTotalMl
        .stateIn(viewModelScope, SharingStarted.Eagerly, initialValue = 0)

    /**
     * Log a 250 ml drink. Launched in viewModelScope so it:
     *   - Runs on the IO thread (via the repository's withContext(Dispatchers.IO))
     *   - Is cancelled automatically if the user leaves the app mid-insert
     *     (though a SQLite INSERT is so fast this almost never matters)
     */
    fun logDrink() {
        viewModelScope.launch { repo.logDrink() }
    }
}
