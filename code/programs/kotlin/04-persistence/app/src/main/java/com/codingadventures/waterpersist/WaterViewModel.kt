package com.codingadventures.waterpersist

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch

/**
 * Exposes today's water total as a StateFlow so Compose can observe it.
 * AndroidViewModel gives us the Application context without leaking Activity.
 */
class WaterViewModel(app: Application) : AndroidViewModel(app) {
    private val repo = WaterRepository(app)

    /** Today's total ml — recomposes the UI automatically on every insert. */
    val todayTotalMl: StateFlow<Int> = repo.todayTotalMl
        .stateIn(viewModelScope, SharingStarted.Eagerly, initialValue = 0)

    /** Logs a 250ml drink. Runs on IO dispatcher via the repository. */
    fun logDrink() {
        viewModelScope.launch { repo.logDrink() }
    }
}
