# mosaic_flux_flutter

Strict-Flux runtime for Mosaic UI's Flutter emitter.

Implements the architecture specified in `code/specs/UI33-rewrite-unified-architecture.md`. Mirrors the API surface of `mosaic-flux-react / html / webcomponent / swiftui / compose` in idiomatic Dart.

## Pure Dart core, Flutter widget deferred

v0.1.0 ships **pure Dart** — no Flutter SDK dependency — so the package builds without Flutter installed and can be used by non-Flutter Dart consumers (CLI tools, server-side Dart, Dart Frog handlers, etc.).

The `MosaicBuilder` widget (a `StatefulWidget` that rebuilds on store changes) was originally planned for v0.1.0 but is **deferred to v0.2.0**. Flutter consumers can use the imperative `addListener()` or `subscribe()` APIs inside their own `StatefulWidget` to trigger `setState` manually.

## API surface

| Surface | Detail |
|---|---|
| `MosaicAction<S>` abstract class | Command Pattern; subclass and override `apply()` |
| `MosaicStore<S>` class | Synchronous dispatcher; fine-grained subscriptions |
| `Middleware<S>` typedef | `void Function(MosaicAction<S>, S, S)` |
| `composeMiddleware` + `loggerMiddleware` | Throw-isolated composition |
| `createSelector1` / `createSelector2` / `createSelector3` | Memoised derived state |
| `devToolsMiddleware` | UI33-rewrite §8 stub (stdout in v0.1.0) |
| `addListener` (on store) | Bulk change notification — Flutter ChangeNotifier-compatible |

## Quick start

```dart
import 'package:mosaic_flux_flutter/mosaic_flux.dart';

class CounterState {
  final int count;
  const CounterState({this.count = 0});
  CounterState copyWith({int? count}) => CounterState(count: count ?? this.count);
}

class Increment extends MosaicAction<CounterState> {
  @override
  CounterState apply(CounterState state) =>
      state.copyWith(count: state.count + 1);
}

void main() {
  final store = MosaicStore<CounterState>(initialState: CounterState());

  // Fine-grained subscription
  final unsub = store.subscribe<int>(
    (s) => s.count,
    (newCount) => print('count is now $newCount'),
    equality: (a, b) => a == b,
  );

  store.dispatch(Increment());   // prints "count is now 1"
  store.dispatch(Increment());   // prints "count is now 2"

  unsub();
}
```

## Flutter integration (manual until v0.2.0)

```dart
class CounterDisplay extends StatefulWidget {
  final MosaicStore<CounterState> store;
  const CounterDisplay({required this.store});
  @override
  State<CounterDisplay> createState() => _CounterDisplayState();
}

class _CounterDisplayState extends State<CounterDisplay> {
  late final void Function() _unsub;

  @override
  void initState() {
    super.initState();
    _unsub = widget.store.addListener(() => setState(() {}));
  }

  @override
  void dispose() {
    _unsub();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Text('Count: ${widget.store.state.count}');
  }
}
```

## Status

v0.1.0. Initial release. **31 tests passing** via `dart test`.

## Deferred to v0.2.0

- `MosaicBuilder` widget — declarative Flutter integration.
- TCP socket DevTools transport on `localhost:9229`.
- Time-travel replay support on the runtime side.
- ChangeNotifier mixin (so MosaicStore can drop into existing Provider / get_it trees without an adapter).
