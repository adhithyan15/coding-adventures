package com.codingadventures.waterwear

import android.content.Context
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.withContext
import java.util.Calendar

/**
 * WaterRepository — the data layer between ViewModel and the Room DAO.
 *
 * WHY A REPOSITORY?
 *   The ViewModel should not know whether data comes from SQLite, a network API,
 *   or a cache. The repository abstracts that decision. Today we only have Room;
 *   tomorrow we could add a Wear Data Layer sync to a paired phone app without
 *   changing the ViewModel.
 *
 * THREADING MODEL:
 *   UI work → main thread (Dispatchers.Main, Compose recompositions)
 *   DB work → IO thread pool (Dispatchers.IO, designed for blocking I/O)
 *
 *   The repository enforces the threading rule: logDrink() switches to IO before
 *   calling the DAO's suspend insert(). The ViewModel never needs to think about
 *   which dispatcher to use.
 *
 * FLOW vs LIVEDATA:
 *   We use Flow (kotlinx.coroutines) rather than LiveData (androidx.lifecycle).
 *   Flow integrates better with coroutines and is lifecycle-independent —
 *   the ViewModel converts it to a StateFlow that Compose observes.
 */
class WaterRepository(context: Context) {
    private val dao = WaterDatabase.getInstance(context).waterDao()

    /**
     * Live total ml consumed today.
     *
     * startOfDayMs() recomputes midnight of today in epoch milliseconds.
     * The Flow emits a new Int every time a row is inserted that matches
     * the WHERE clause. Compose re-renders automatically.
     */
    val todayTotalMl: Flow<Int> = dao.todayTotalMl(startOfDayMs())

    /**
     * Inserts a 250ml drink on the IO dispatcher.
     *
     * `suspend` means the caller must be inside a coroutine (the ViewModel's
     * viewModelScope provides this). `withContext(Dispatchers.IO)` switches
     * to the IO thread pool for the duration of the insert, then returns to
     * the caller's dispatcher (typically Main).
     */
    suspend fun logDrink() = withContext(Dispatchers.IO) {
        dao.insert(WaterEntry())
    }

    /**
     * Returns epoch milliseconds at midnight (00:00:00.000) of the current day.
     *
     * Calendar.getInstance() initialises to the current local date and time.
     * We zero out hour, minute, second, and millisecond to get today's midnight.
     *
     * WHY NOT USE LocalDate?
     *   java.time.LocalDate is available on API 26+, and we target minSdk 30,
     *   so it would work. However, Calendar is simpler for this single purpose
     *   and avoids an extra import. Both approaches are correct.
     *
     * EXAMPLE:
     *   If now is 2026-03-29 14:35:00 UTC+9 (Tokyo),
     *   startOfDayMs() returns the epoch ms for 2026-03-29 00:00:00 UTC+9.
     *   The query then includes everything from midnight onward — today only.
     */
    private fun startOfDayMs(): Long {
        val cal = Calendar.getInstance()
        cal.set(Calendar.HOUR_OF_DAY, 0)
        cal.set(Calendar.MINUTE, 0)
        cal.set(Calendar.SECOND, 0)
        cal.set(Calendar.MILLISECOND, 0)
        return cal.timeInMillis
    }
}
