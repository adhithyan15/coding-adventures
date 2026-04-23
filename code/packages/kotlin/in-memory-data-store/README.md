# in-memory-data-store

Composable JVM in-memory data store facade

The package exposes a RESP-facing `DataStoreManager` on top of the shared
engine and protocol layers. It can run purely in memory or write an
append-only file (AOF) that replays on startup.

```kotlin
val manager = DataStoreManager()
val durable = DataStoreManager(Path.of("appendonly.aof"))
```

## Development

```bash
bash BUILD
```
