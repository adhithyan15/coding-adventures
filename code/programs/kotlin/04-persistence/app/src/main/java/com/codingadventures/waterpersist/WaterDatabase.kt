package com.codingadventures.waterpersist

import android.content.Context
import androidx.room.Database
import androidx.room.Room
import androidx.room.RoomDatabase

/**
 * The Room database. Singleton pattern ensures only one connection is open
 * at a time across the app's lifetime.
 */
@Database(entities = [WaterEntry::class], version = 1, exportSchema = false)
abstract class WaterDatabase : RoomDatabase() {
    abstract fun waterDao(): WaterDao

    companion object {
        @Volatile private var INSTANCE: WaterDatabase? = null

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
