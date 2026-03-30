package com.codingadventures.waterwear

import androidx.room.Dao
import androidx.room.Insert
import androidx.room.Query
import kotlinx.coroutines.flow.Flow

/**
 * WaterDao — Data Access Object for the water_entries table.
 *
 * DAO PATTERN:
 *   The DAO separates the "what data do I need" (this interface) from
 *   "how is the data stored" (the database). Tests can provide a fake DAO
 *   without involving SQLite at all. The ViewModel never touches SQL directly —
 *   it goes through the repository which goes through the DAO.
 *
 * ROOM CODE GENERATION:
 *   Room reads this interface at compile time (via KSP) and generates
 *   WaterDao_Impl.kt containing the actual SQLite calls. We never write
 *   PreparedStatement or Cursor code — Room handles all of that.
 *
 * COROUTINES + FLOW:
 *   - `suspend fun insert()` runs inside a coroutine (called from Dispatchers.IO)
 *   - `fun todayTotalMl()` returns Flow<Int>: a hot stream that emits a new value
 *     whenever the underlying table changes. The ViewModel converts this Flow into
 *     a StateFlow that Compose can observe.
 *
 *   Flow is like a pipe: data flows from SQLite → DAO → Repository → ViewModel
 *   → Compose. Any insert() immediately pushes a new total through the pipe.
 */
@Dao
interface WaterDao {

    /**
     * Insert a new drink entry.
     *
     * `suspend` means this function must be called from a coroutine.
     * Room executes the INSERT on whatever dispatcher the coroutine runs on;
     * the repository ensures this is Dispatchers.IO (background thread).
     */
    @Insert
    suspend fun insert(entry: WaterEntry)

    /**
     * Live total water consumed today (in ml).
     *
     * SQL walkthrough:
     *   SELECT COALESCE(SUM(amountMl), 0)   — sum all amountMl values;
     *                                           COALESCE returns 0 if no rows match
     *   FROM water_entries
     *   WHERE timestampMs >= :startOfDayMs   — only today's entries (since midnight)
     *
     * The `:startOfDayMs` is a named parameter — Room substitutes the actual value
     * at runtime, preventing SQL injection attacks.
     *
     * Returning Flow<Int> (not suspend) means Room KEEPS OBSERVING the table
     * and re-runs this query on every insert. The UI updates automatically.
     */
    @Query("SELECT COALESCE(SUM(amountMl), 0) FROM water_entries WHERE timestampMs >= :startOfDayMs")
    fun todayTotalMl(startOfDayMs: Long): Flow<Int>
}
