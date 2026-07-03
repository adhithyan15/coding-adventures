# mosaic-flux-compose

Strict-Flux runtime for Mosaic UI's Jetpack Compose emitter.

Implements the architecture specified in `code/specs/UI33-rewrite-unified-architecture.md`:

- **`MosaicAction<S>` interface** — the Command Pattern. Each action is a class (typically a `data class`) carrying its payload as constructor parameters + an `apply(state) → state` method.
- **`MosaicStore<S>` class** — state container + dispatcher. Exposes state via `kotlinx.coroutines.flow.StateFlow<S>` for fine-grained `collectAsState()` integration with Compose.
- **`Middleware<S>`** typealias + `composeMiddleware` (with throw isolation) + `loggerMiddleware`.
- **`createSelector`** — memoised derived-state combinator (1-, 2-, 3-input variants).
- **`devToolsMiddleware`** — UI33-rewrite §8 protocol stub (logs to stdout in v0.1.0; TCP socket transport to localhost:9229 deferred to v0.2.0).

## Quick start

```kotlin
import org.mosaic.flux.*

// 1. State + actions (in real Mosaic projects auto-generated from .mil)
data class CounterState(val count: Int = 0)

data class Increment(val by: Int = 1) : MosaicAction<CounterState> {
    override fun apply(state: CounterState): CounterState =
        state.copy(count = state.count + by)
}

// 2. Store
val store = MosaicStore<CounterState>(CounterState())

// 3a. Compose integration — collectAsState on the StateFlow
@Composable
fun Counter() {
    val state by store.stateFlow.collectAsState()
    Column {
        Text("Count: ${state.count}")
        Button(onClick = { store.dispatch(Increment()) }) {
            Text("Increment")
        }
    }
}

// 3b. Imperative subscription (non-Compose hosts)
val unsubscribe = store.subscribe({ it.count }, { a, b -> a == b }) { newCount ->
    println("count is now $newCount")
}
store.dispatch(Increment())     // prints "count is now 1"
unsubscribe()
```

## Build

Uses Gradle 8.14+ with the Kotlin JVM plugin. JVM 17 minimum.

```bash
gradle test
```

### macOS HFS+ note

The repo's required `BUILD` file (lowercase command, uppercase filename) collides with Gradle's default `build/` output directory on case-insensitive filesystems. This package redirects Gradle's output to `.gradle-out/` via `layout.buildDirectory.set(file(".gradle-out"))` in `build.gradle.kts` to avoid the conflict.

## Status

v0.1.0. Initial release.

- 27 JUnit5 tests, all passing
- Tested on JDK 21 with Kotlin 2.0
- Zero external dependencies beyond kotlinx-coroutines-core (for `StateFlow`)

## Deferred to v0.2.0

- TCP socket DevTools transport on `localhost:9229`
- Compose-specific helpers (state-driven Modifier composition, etc.)
- Time-travel replay support on the runtime side
