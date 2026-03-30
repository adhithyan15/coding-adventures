# Mobile 04 — Persistence

## Overview

The fourth app in the mobile learning series. Extends `02-water-counter` with
local persistence so the water counter survives app restarts. Implemented on
both iOS (SwiftData) and Android (Room).

When the user logs a drink, the entry is saved to the local database. When the
app is reopened, today's total is restored. At midnight, the counter resets
automatically for the new day.

No Watch sync, no cloud, no HealthKit — just the phone, the database, and the
counter. Those layers are added in later stages.

---

## Learning Goals

**iOS (SwiftData)**
- Define a `@Model` class and a `ModelContainer`
- Insert records with `modelContext.insert(_:)`
- Query records with `@Query` and predicate-based filtering
- Understand SwiftData's automatic persistence (no manual save needed)
- Filter by date to compute today's total

**Android (Room)**
- Define an `@Entity` data class
- Write a `@Dao` interface with `@Insert` and `@Query`
- Build a `@Database` class and create it with `Room.databaseBuilder`
- Expose data via a `ViewModel` + `StateFlow`
- Use `collectAsState()` to bind database-backed state to Compose UI

---

## Project Structure

### iOS

```
code/programs/swift/04-persistence/
├── project.yml
├── Sources/
│   └── WaterPersist/
│       ├── WaterPersistApp.swift       ← sets up ModelContainer
│       ├── ContentView.swift           ← reads @Query, calls insert
│       ├── WaterEntry.swift            ← @Model definition
│       └── DayFilter.swift             ← date predicate helpers
├── README.md
└── CHANGELOG.md
```

### Android

```
code/programs/kotlin/04-persistence/
├── app/
│   └── src/main/
│       ├── java/com/codingadventures/waterpersist/
│       │   ├── MainActivity.kt
│       │   ├── WaterEntry.kt           ← @Entity
│       │   ├── WaterDao.kt             ← @Dao
│       │   ├── WaterDatabase.kt        ← @Database
│       │   ├── WaterRepository.kt      ← wraps Dao, runs on IO dispatcher
│       │   └── WaterViewModel.kt       ← exposes StateFlow<Int> todayTotal
│       └── res/
├── build.gradle.kts
├── README.md
└── CHANGELOG.md
```

---

## Data Model

### Water Entry

A single log entry representing one drink.

```
id          UUID / Long (auto-generated)
timestamp   Date / Long (epoch millis)
amount_ml   Int          (always 250 for this stage)
```

### Daily Total (derived, never stored)

```
today_total_ml  =  SUM(amount_ml) WHERE date(timestamp) = today
```

The daily total is computed by querying all entries where `timestamp` falls
within today's calendar day (midnight to midnight in the user's local
timezone). It is never cached as a separate row — always derived on read.

### Midnight Reset

The counter displays `today_total_ml`. Because it is derived from entries
filtered by today's date, it automatically shows 0 on a new calendar day
with no cron job, no background task, and no stored "last reset date".

---

## iOS Implementation

### Model

```swift
// WaterEntry.swift
import SwiftData
import Foundation

/// A single logged drink.
///
/// SwiftData persists this automatically — no manual save() call needed.
/// The @Model macro generates the underlying SQLite schema.
@Model
final class WaterEntry {
    var id: UUID
    var timestamp: Date
    var amountMl: Int

    init(amountMl: Int = 250) {
        self.id = UUID()
        self.timestamp = Date()
        self.amountMl = amountMl
    }
}
```

### Container Setup

```swift
// WaterPersistApp.swift
@main
struct WaterPersistApp: App {
    var body: some Scene {
        WindowGroup {
            ContentView()
        }
        // SwiftData creates the SQLite store automatically.
        // Pass the model type; SwiftData handles migrations.
        .modelContainer(for: WaterEntry.self)
    }
}
```

### Querying Today's Entries

```swift
// ContentView.swift
struct ContentView: View {
    // @Query fetches all entries and keeps the view in sync.
    // We filter to today in a computed property.
    @Query private var allEntries: [WaterEntry]
    @Environment(\.modelContext) private var context

    private var todayEntries: [WaterEntry] {
        let startOfDay = Calendar.current.startOfDay(for: Date())
        return allEntries.filter { $0.timestamp >= startOfDay }
    }

    private var todayTotalMl: Int {
        todayEntries.reduce(0) { $0 + $1.amountMl }
    }

    var body: some View {
        // ... UI
    }

    func logDrink() {
        let entry = WaterEntry(amountMl: 250)
        context.insert(entry)
        // No save() needed — SwiftData autosaves on the next run loop tick
    }
}
```

---

## Android Implementation

### Entity

```kotlin
// WaterEntry.kt
import androidx.room.Entity
import androidx.room.PrimaryKey

/**
 * A single logged drink, stored as one row in the water_entries table.
 *
 * Room generates the SQLite schema from this class. The @Entity annotation
 * marks it as a database table; @PrimaryKey(autoGenerate = true) lets
 * SQLite assign a unique id to each row automatically.
 */
@Entity(tableName = "water_entries")
data class WaterEntry(
    @PrimaryKey(autoGenerate = true) val id: Long = 0,
    val timestampMs: Long = System.currentTimeMillis(),
    val amountMl: Int = 250
)
```

### DAO

```kotlin
// WaterDao.kt
import androidx.room.Dao
import androidx.room.Insert
import androidx.room.Query
import kotlinx.coroutines.flow.Flow

/**
 * Data Access Object for water entries.
 *
 * Room generates the implementation at compile time. @Query methods that
 * return Flow<T> are automatically re-executed when the underlying table
 * changes — the UI observes the Flow and recomposes whenever data changes.
 */
@Dao
interface WaterDao {

    @Insert
    suspend fun insert(entry: WaterEntry)

    /**
     * Returns today's total in ml.
     *
     * The WHERE clause compares timestamps to midnight today (epoch ms).
     * Room re-emits whenever the table changes, so the UI stays live.
     */
    @Query("""
        SELECT COALESCE(SUM(amountMl), 0)
        FROM water_entries
        WHERE timestampMs >= :startOfDayMs
    """)
    fun todayTotalMl(startOfDayMs: Long): Flow<Int>
}
```

### Database

```kotlin
// WaterDatabase.kt
import android.content.Context
import androidx.room.Database
import androidx.room.Room
import androidx.room.RoomDatabase

@Database(entities = [WaterEntry::class], version = 1)
abstract class WaterDatabase : RoomDatabase() {
    abstract fun waterDao(): WaterDao

    companion object {
        @Volatile private var INSTANCE: WaterDatabase? = null

        fun getInstance(context: Context): WaterDatabase =
            INSTANCE ?: synchronized(this) {
                Room.databaseBuilder(
                    context.applicationContext,
                    WaterDatabase::class.java,
                    "water.db"
                ).build().also { INSTANCE = it }
            }
    }
}
```

### ViewModel

```kotlin
// WaterViewModel.kt
import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch
import java.util.Calendar

class WaterViewModel(app: Application) : AndroidViewModel(app) {
    private val dao = WaterDatabase.getInstance(app).waterDao()

    private fun startOfDayMs(): Long {
        val cal = Calendar.getInstance()
        cal.set(Calendar.HOUR_OF_DAY, 0)
        cal.set(Calendar.MINUTE, 0)
        cal.set(Calendar.SECOND, 0)
        cal.set(Calendar.MILLISECOND, 0)
        return cal.timeInMillis
    }

    // StateFlow backed by the Room query — recomposes Compose UI automatically
    val todayTotalMl = dao
        .todayTotalMl(startOfDayMs())
        .stateIn(viewModelScope, SharingStarted.Eagerly, 0)

    fun logDrink() {
        viewModelScope.launch {
            dao.insert(WaterEntry())
        }
    }
}
```

---

## UI

Both apps show the same UI as `02-water-counter`. The only change visible to
the user is that:

1. Closing and reopening the app restores the counter to today's logged total
2. A small "Saved locally" badge replaces the previous ephemeral state note

---

## Midnight Reset Behaviour

The filter `timestamp >= startOfDay` is evaluated fresh each time the view
appears. Edge case: the app is open when midnight passes.

| Scenario | Behaviour |
|----------|-----------|
| App reopened after midnight | `startOfDay` recomputed — new day, counter shows 0 |
| App open at midnight | Counter does not reset live — shows yesterday's count until app is foregrounded or button tapped |

The live-midnight-reset is a known limitation accepted for this stage. Proper
handling (timer-based day boundary detection) is out of scope.

---

## Testing

Both platforms must have unit tests covering:

| Test | What it verifies |
|------|-----------------|
| Insert + query | Logged entry is returned by today's query |
| Yesterday's entry excluded | Entry from 25 hours ago is not counted today |
| Total sum | Three 250ml entries sum to 750ml |
| Empty day | No entries returns 0, not nil/crash |

iOS: XCTest with an in-memory `ModelContainer` (`isStoredInMemoryOnly: true`)
Android: JUnit4 with an in-memory Room database (`allowMainThreadQueries()`)

---

## GitHub Actions CI

Extends existing `build-swift.yml` and `build-android.yml` workflows.
No new workflow files needed — new paths are auto-detected by the existing
build tool's path-prefix matching.

---

## Out of Scope for This Stage

- Watch sync (Stage 03 — parallel track)
- iCloud / CloudKit sync (Foveo Stage 06)
- HealthKit (Foveo Stage 08)
- History screen / calendar view (Foveo Stage 02)
- Plant mascot (Foveo Stage 04)
- Notifications (Foveo Stage 03)
