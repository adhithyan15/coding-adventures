package com.codingadventures.waterwear

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch

/**
 * WaterViewModel — the bridge between WaterRepository (data) and WaterScreen (UI).
 *
 * VIEWMODEL RESPONSIBILITIES:
 *   - Holds UI state that survives configuration changes (screen rotation on phones;
 *     on WearOS this matters when the watch ambient/active state changes).
 *   - Exposes state as StateFlow<T>, which Compose can observe via collectAsState().
 *   - Handles user actions (logDrink) by calling the repository.
 *   - Does NOT know about Composables, context, or views.
 *
 * WHY AndroidViewModel (not plain ViewModel)?
 *   AndroidViewModel receives the Application object at construction. We need it
 *   to pass to WaterRepository for database access. Using Application (not Activity)
 *   context prevents memory leaks — Activities can be destroyed and recreated,
 *   but Application lives for the app's entire lifetime.
 *
 * STATEFLOW vs FLOW:
 *   Flow is cold (lazy) — it starts computing when someone collects it, and stops
 *   when the collector cancels. Compose's collectAsState() needs a StateFlow —
 *   a hot stream with:
 *     1. An initial value (so Compose can render immediately on first frame).
 *     2. A current value accessible synchronously via .value.
 *
 *   stateIn() converts a cold Flow into a hot StateFlow within the ViewModel's scope:
 *     - SharingStarted.Eagerly: start collecting from the repository immediately
 *       on ViewModel creation (not waiting for the first UI subscriber).
 *     - initialValue = 0: show "0 ml" while the first DB query is in flight.
 */
class WaterViewModel(app: Application) : AndroidViewModel(app) {

    private val repo = WaterRepository(app)

    /**
     * Today's total water consumed in ml — a live stream from the database.
     *
     * Every time logDrink() inserts a row, Room emits a new sum through the
     * Flow pipeline: Room → DAO → Repository → stateIn() → todayTotalMl.
     * Compose re-renders WaterScreen automatically whenever the value changes.
     */
    val todayTotalMl: StateFlow<Int> = repo.todayTotalMl
        .stateIn(
            scope = viewModelScope,
            started = SharingStarted.Eagerly,
            initialValue = 0
        )

    /**
     * Records a 250ml glass of water.
     *
     * viewModelScope.launch { } starts a coroutine that lives as long as the
     * ViewModel. If the user taps the button and immediately leaves the screen,
     * the coroutine keeps running until the insert completes — no data loss.
     *
     * The repository switches to Dispatchers.IO internally, so we never block
     * the main thread from this function.
     */
    fun logDrink() {
        viewModelScope.launch { repo.logDrink() }
    }
}
