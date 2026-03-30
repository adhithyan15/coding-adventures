package com.codingadventures.waternotify

import android.content.Context
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.withContext
import java.util.Calendar

/**
 * WaterRepository — the data layer between the ViewModel and Room.
 *
 * The repository pattern is the standard Android architecture recommendation.
 * It separates "how data is stored/retrieved" from "what the UI needs".
 * This lets you swap the data source (e.g., from local DB to network API)
 * without changing the ViewModel or the UI.
 *
 * THREADING MODEL:
 * ────────────────
 * Android's main thread handles all UI drawing and touch events. If you block
 * the main thread (e.g., with a slow database write), the UI freezes and the
 * system may show an ANR (Application Not Responding) dialog.
 *
 * Room DAO `suspend` functions run on whatever coroutine dispatcher the caller
 * uses. We explicitly switch to Dispatchers.IO for writes:
 *
 *   suspend fun logDrink() = withContext(Dispatchers.IO) { ... }
 *
 * Dispatchers.IO is a thread pool optimised for blocking I/O (disk, network).
 * withContext() suspends the calling coroutine, runs the block on the IO pool,
 * then resumes on the caller's dispatcher — all without blocking any thread.
 *
 * FLOW (READ):
 * ────────────
 * Room's Flow<Int> for todayTotalMl is *already* observed on a background
 * thread by Room's internal machinery. We just expose it directly; the
 * ViewModel converts it to StateFlow for Compose.
 */
class WaterRepository(context: Context) {
    private val dao = WaterDatabase.getInstance(context).waterDao()

    /**
     * Live stream of today's total water intake.
     * Emits a new Int every time a drink is logged (or deleted).
     */
    val todayTotalMl: Flow<Int> = dao.todayTotalMl(startOfDayMs())

    /**
     * Logs a 250 ml drink. Suspends until the INSERT completes on the IO thread.
     */
    suspend fun logDrink() = withContext(Dispatchers.IO) {
        dao.insert(WaterEntry())
    }

    /**
     * Returns the epoch milliseconds for midnight (00:00:00.000) today.
     *
     * Calendar.getInstance() creates a Calendar set to "right now". We then
     * zero out the time components, leaving just the date. Converting to
     * timeInMillis gives us the Unix timestamp of today's midnight.
     *
     * Example: if it's 2025-03-29 14:30:00 UTC+5:30, this returns the
     * millisecond value for 2025-03-29 00:00:00 in the device's local timezone.
     * This is intentional — we want "today in the user's timezone".
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
