package com.codingadventures.waterpersist

import android.content.Context
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.withContext
import java.util.Calendar

/**
 * Wraps WaterDao and handles threading. All database writes run on the
 * IO dispatcher so they never block the main (UI) thread.
 */
class WaterRepository(context: Context) {
    private val dao = WaterDatabase.getInstance(context).waterDao()

    /** Today's total as a live Flow — emits a new value on every insert. */
    val todayTotalMl: Flow<Int> = dao.todayTotalMl(startOfDayMs())

    /** Inserts a 250ml drink entry on the IO dispatcher. */
    suspend fun logDrink() = withContext(Dispatchers.IO) {
        dao.insert(WaterEntry())
    }

    /** Midnight of the current calendar day in epoch milliseconds. */
    private fun startOfDayMs(): Long {
        val cal = Calendar.getInstance()
        cal.set(Calendar.HOUR_OF_DAY, 0)
        cal.set(Calendar.MINUTE, 0)
        cal.set(Calendar.SECOND, 0)
        cal.set(Calendar.MILLISECOND, 0)
        return cal.timeInMillis
    }
}
