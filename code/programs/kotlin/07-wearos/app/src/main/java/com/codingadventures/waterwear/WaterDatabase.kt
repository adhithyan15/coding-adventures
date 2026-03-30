package com.codingadventures.waterwear

import android.content.Context
import androidx.room.Database
import androidx.room.Room
import androidx.room.RoomDatabase

/**
 * WaterDatabase — the Room database singleton.
 *
 * ROOM DATABASE ANATOMY:
 *   @Database declares the schema: which tables exist (entities) and the
 *   schema version. Room compares the version number to the on-disk database;
 *   if they differ, it runs your Migration objects or (if none provided) throws.
 *
 * WHY ABSTRACT?
 *   Room generates the concrete implementation (WaterDatabase_Impl.kt) at
 *   compile time via KSP. We declare the abstract class; Room fills it in.
 *   The abstract fun waterDao() is Room's entry point for the DAO.
 *
 * SINGLETON PATTERN:
 *   SQLite is a file-based database. Opening multiple connections to the same
 *   file concurrently can cause lock contention and data corruption. The singleton
 *   ensures the entire app shares exactly one open connection.
 *
 *   @Volatile ensures the INSTANCE field is immediately visible to all threads.
 *   Without @Volatile, a CPU cache might serve a stale null to another thread
 *   even after INSTANCE is set, causing two instances to be created.
 *
 *   The synchronized(this) block prevents two coroutines from both seeing
 *   INSTANCE == null and both creating a new database simultaneously.
 *
 * exportSchema = false:
 *   When true, Room writes a JSON schema file to app/schemas/ at build time
 *   (useful for migration testing). We disable it to keep the build output clean
 *   for this learning project.
 */
@Database(entities = [WaterEntry::class], version = 1, exportSchema = false)
abstract class WaterDatabase : RoomDatabase() {

    /** Returns the DAO for water_entries. Room implements this at compile time. */
    abstract fun waterDao(): WaterDao

    companion object {
        // @Volatile: writes to this field are immediately visible to all threads,
        // bypassing CPU caches. Critical for a multi-threaded singleton.
        @Volatile
        private var INSTANCE: WaterDatabase? = null

        /**
         * Returns the singleton database, creating it if necessary.
         *
         * The double-checked locking pattern:
         *   1. First check: fast path — if INSTANCE is non-null, return it without
         *      acquiring the lock (locks are expensive on the main thread).
         *   2. synchronized block: only one thread enters at a time.
         *   3. Second check: another thread might have created the instance while
         *      we waited for the lock. Check again before creating.
         */
        fun getInstance(context: Context): WaterDatabase =
            INSTANCE ?: synchronized(this) {
                INSTANCE ?: Room.databaseBuilder(
                    context.applicationContext,   // application context: avoids Activity leaks
                    WaterDatabase::class.java,
                    "water.db"                    // filename in the app's private data directory
                ).build().also { INSTANCE = it }
            }
    }
}
