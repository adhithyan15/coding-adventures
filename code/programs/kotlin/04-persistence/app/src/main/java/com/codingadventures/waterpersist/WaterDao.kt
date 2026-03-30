package com.codingadventures.waterpersist

import androidx.room.Dao
import androidx.room.Insert
import androidx.room.Query
import kotlinx.coroutines.flow.Flow

/**
 * Data Access Object for water entries.
 *
 * Room generates the SQL implementation at compile time. Methods returning
 * Flow<T> are re-executed automatically whenever the table changes, so the
 * UI stays live without any manual refresh logic.
 */
@Dao
interface WaterDao {
    @Insert
    suspend fun insert(entry: WaterEntry)

    /**
     * Returns today's total water intake in ml.
     * COALESCE ensures we get 0 (not null) when no entries exist today.
     */
    @Query("SELECT COALESCE(SUM(amountMl), 0) FROM water_entries WHERE timestampMs >= :startOfDayMs")
    fun todayTotalMl(startOfDayMs: Long): Flow<Int>

    // Used in tests only
    @Query("SELECT * FROM water_entries")
    suspend fun getAll(): List<WaterEntry>
}
