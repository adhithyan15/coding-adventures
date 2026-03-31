package com.codingadventures.waternotify

import androidx.room.Entity
import androidx.room.PrimaryKey

/**
 * WaterEntry — a single logged drink, stored as one row in the water_entries table.
 *
 * This is identical to the model in stage 04 (WaterPersist). Reusing the same
 * schema means users who upgrade from stage 04 would keep their drink history,
 * and the Room migration path would be straightforward (no columns changed).
 *
 * ROOM ANNOTATIONS EXPLAINED:
 * ────────────────────────────
 * @Entity(tableName = "water_entries")
 *   Tells Room to create a SQLite table with this exact name. Room reads this
 *   annotation at compile time (via KSP) and generates the CREATE TABLE SQL.
 *
 * @PrimaryKey(autoGenerate = true)
 *   SQLite's ROWID alias: the database assigns a unique integer automatically
 *   on each INSERT, counting up from 1. We never set `id` manually.
 *
 * DATA CLASS ADVANTAGES:
 * ──────────────────────
 * Kotlin's `data class` gives us equals(), hashCode(), toString(), and copy()
 * for free. Room requires entities to be data classes (or have a no-arg
 * constructor) so it can create instances when reading rows back from SQLite.
 *
 * DEFAULT VALUES:
 * ───────────────
 * `timestampMs`: defaults to "now" so the call site (WaterRepository.logDrink)
 *   just does `WaterEntry()` without specifying a time.
 * `amountMl = 250`: a standard glass is ~250 ml (8 fl oz). This is hardcoded
 *   for simplicity; a real app might let users pick their glass size.
 */
@Entity(tableName = "water_entries")
data class WaterEntry(
    @PrimaryKey(autoGenerate = true) val id: Long = 0,
    val timestampMs: Long = System.currentTimeMillis(),
    val amountMl: Int = 250
)
