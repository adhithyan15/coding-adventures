package com.codingadventures.waternotify

import androidx.room.Dao
import androidx.room.Insert
import androidx.room.Query
import kotlinx.coroutines.flow.Flow

/**
 * WaterDao — Data Access Object for the water_entries table.
 *
 * A DAO is the bridge between your Kotlin code and the SQLite database.
 * You declare the *intent* of each operation (insert this, query that),
 * and Room's KSP annotation processor writes the actual SQL implementation
 * at compile time — you never write JDBC or cursor code yourself.
 *
 * WHY AN INTERFACE?
 * ─────────────────
 * Room generates a class that *implements* this interface. By declaring it
 * as an interface we rely entirely on Room's generated code rather than
 * writing our own implementation, which Room then optimises for SQLite.
 *
 * FLOW<T> — REACTIVE DATABASE QUERIES:
 * ──────────────────────────────────────
 * The `todayTotalMl` query returns Flow<Int>. This is a Kotlin coroutines
 * reactive stream. Room wraps the query in a SQLite content observer:
 * every time the `water_entries` table changes (INSERT, UPDATE, DELETE),
 * Room re-runs the query and emits the new total into the Flow.
 *
 * The Compose UI collects this Flow via `collectAsState()`, so the screen
 * automatically redraws with the new total the instant a drink is logged —
 * no polling, no manual "refresh" button.
 *
 * This is the Android equivalent of iOS's @Query + @Published pattern or
 * SwiftUI's ObservableObject: data flows from the database to the UI
 * automatically.
 */
@Dao
interface WaterDao {

    /**
     * Inserts a new drink record. `suspend` means it must be called from
     * a coroutine — Room runs the INSERT on the caller's dispatcher (we use IO).
     */
    @Insert
    suspend fun insert(entry: WaterEntry)

    /**
     * Returns today's total water intake in ml as a live reactive stream.
     *
     * COALESCE(SUM(...), 0) is a SQL trick: SUM() returns NULL when there are
     * no rows (empty table), but COALESCE converts NULL → 0 so the Flow always
     * emits an Int, never null.
     *
     * The :startOfDayMs parameter is injected by Room using SQLite's `?`
     * binding, which prevents SQL injection (Room quotes the value correctly).
     *
     * We pass startOfDayMs from the repository so only today's entries count
     * toward the goal — yesterday's drinks don't carry over.
     */
    @Query("SELECT COALESCE(SUM(amountMl), 0) FROM water_entries WHERE timestampMs >= :startOfDayMs")
    fun todayTotalMl(startOfDayMs: Long): Flow<Int>
}
