# in-memory-data-store

Composable JVM in-memory data store facade

The package exposes a RESP-facing `DataStoreManager` on top of the shared
engine and protocol layers. It can be used in pure in-memory mode or with an
append-only file (AOF) path for command replay across restarts.

```java
DataStoreManager manager = new DataStoreManager();
DataStoreManager durable = new DataStoreManager(Path.of("appendonly.aof"));
```

## Development

```bash
bash BUILD
```
