package com.codingadventures.waterpersist

import androidx.room.Entity
import androidx.room.PrimaryKey

/**
 * A single logged drink, stored as one row in the water_entries table.
 *
 * Room generates the SQLite schema from this class at compile time via KSP.
 * The @Entity annotation marks it as a table; @PrimaryKey(autoGenerate=true)
 * lets SQLite assign a unique id automatically.
 */
@Entity(tableName = "water_entries")
data class WaterEntry(
    @PrimaryKey(autoGenerate = true) val id: Long = 0,
    val timestampMs: Long = System.currentTimeMillis(),
    val amountMl: Int = 250
)
