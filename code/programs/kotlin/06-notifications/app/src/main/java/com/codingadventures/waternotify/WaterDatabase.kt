package com.codingadventures.waternotify

import android.content.Context
import androidx.room.Database
import androidx.room.Room
import androidx.room.RoomDatabase

/**
 * WaterDatabase — the Room database singleton.
 *
 * Room is an abstraction layer over SQLite. You describe your schema with
 * Kotlin annotations and Room generates the boilerplate SQL (CREATE TABLE,
 * INSERT, SELECT) at compile time via KSP. At runtime, Room manages the
 * SQLiteOpenHelper lifecycle, handles migrations, and serialises concurrent
 * access safely.
 *
 * SINGLETON PATTERN:
 * ──────────────────
 * Opening a SQLite database is expensive (file I/O, journal setup). We use
 * a classic double-checked locking singleton to ensure only ONE WaterDatabase
 * instance exists for the entire app lifetime:
 *
 *   Thread A and Thread B both call getInstance() simultaneously.
 *   Both see INSTANCE == null and enter synchronized{}.
 *   Only ONE enters the synchronized block at a time (mutex).
 *   The first creates the database; the second sees INSTANCE != null and
 *   returns it without creating a duplicate.
 *
 * @Volatile ensures INSTANCE writes are visible to all threads immediately
 * (no CPU cache staleness). Without it, Thread B might see a stale null even
 * after Thread A set it.
 *
 * WHY `context.applicationContext`?
 * ──────────────────────────────────
 * If we stored an Activity context, the database would hold a reference to
 * the Activity, preventing garbage collection (memory leak). The application
 * context lives as long as the app itself — safe to store in a static field.
 *
 * DATABASE VERSION & MIGRATIONS:
 * ───────────────────────────────
 * version = 1 means this is the initial schema. If we later add a column
 * (e.g., `drinkType: String`), we'd bump to version = 2 and provide a
 * Migration(1, 2) object with the ALTER TABLE SQL. Room would run it on
 * users' devices to upgrade their existing databases without data loss.
 *
 * exportSchema = false skips generating a JSON schema export file. In a
 * production app you'd set this to true and commit the schema files to
 * git so you can track schema history and write tested migration paths.
 */
@Database(entities = [WaterEntry::class], version = 1, exportSchema = false)
abstract class WaterDatabase : RoomDatabase() {

    /** Room generates the concrete implementation of WaterDao at compile time. */
    abstract fun waterDao(): WaterDao

    companion object {
        @Volatile
        private var INSTANCE: WaterDatabase? = null

        fun getInstance(context: Context): WaterDatabase =
            INSTANCE ?: synchronized(this) {
                INSTANCE ?: Room.databaseBuilder(
                    context.applicationContext,
                    WaterDatabase::class.java,
                    "water.db"
                ).build().also { INSTANCE = it }
            }
    }
}
