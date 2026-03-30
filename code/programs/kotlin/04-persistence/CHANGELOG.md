# Changelog

## [1.0.0] — 2026-03-30

### Added
- `WaterEntry` `@Entity` — autoGenerate PK, timestampMs (Long), amountMl (Int, default 250)
- `WaterDao` — `suspend insert()` + `COALESCE(SUM) WHERE timestampMs >= :startOfDayMs` returning `Flow<Int>`
- `WaterDatabase` — `@Database` singleton with double-checked locking
- `WaterRepository` — wraps DAO, `logDrink()` on `Dispatchers.IO`, `startOfDayMs()` via Calendar
- `WaterViewModel` — `StateFlow<Int>` via `stateIn(Eagerly, initialValue=0)`
- `MainActivity` — Compose UI matching 02-water-counter layout, "Saved locally" badge
- Room 2.6.1 + KSP 2.1.0-1.0.29 dependencies
- README.md and CHANGELOG.md
