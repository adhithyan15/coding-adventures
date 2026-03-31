package com.codingadventures.waterwear

// Room annotation — marks this data class as a SQLite table definition.
// Room's KSP processor reads @Entity at compile time and generates the
// CREATE TABLE statement and all required insert/query boilerplate.
import androidx.room.Entity
import androidx.room.PrimaryKey

/**
 * WaterEntry — a single logged drink event, stored as one row in SQLite.
 *
 * DATA CLASS BASICS (for newcomers):
 *   A Kotlin data class automatically provides:
 *   - equals() / hashCode() based on all properties
 *   - toString() for debugging
 *   - copy() for immutable updates (e.g., entry.copy(amountMl = 500))
 *
 * ROOM BASICS:
 *   Room maps this class to the `water_entries` table.
 *   Each property maps to one column. The column name equals the property name
 *   by default (e.g., `timestampMs` → a column named "timestampMs").
 *
 * WHY NOT USE UUID AS PRIMARY KEY?
 *   UUIDs (strings) require 16+ bytes per row and slower index lookups.
 *   An auto-incrementing Long is 8 bytes and produces sequential IDs that
 *   the B-tree index handles optimally. For 8 entries/day over 10 years,
 *   we'd generate ~29,000 rows — well within Long's 9.2 × 10^18 limit.
 *
 * SCHEMA VERSION:
 *   WaterDatabase is declared at version = 1. If you add a new column in
 *   the future, increment the version and provide a Migration object.
 *   Room will refuse to open a database whose schema doesn't match,
 *   protecting you from silent data corruption.
 */
@Entity(tableName = "water_entries")
data class WaterEntry(
    // @PrimaryKey(autoGenerate = true): SQLite assigns a unique integer ID
    // automatically. The default value of 0 is replaced by SQLite on insert.
    @PrimaryKey(autoGenerate = true) val id: Long = 0,

    // System.currentTimeMillis() returns epoch milliseconds (ms since 1970-01-01).
    // We store this as a Long rather than a LocalDateTime to avoid type converters,
    // since SQLite has no native datetime type.
    val timestampMs: Long = System.currentTimeMillis(),

    // Fixed at 250ml per glass — standard serving size.
    // Making this a column (rather than a constant) allows future flexibility:
    // e.g., a user could log a bottle (500ml) without a schema change.
    val amountMl: Int = 250
)
